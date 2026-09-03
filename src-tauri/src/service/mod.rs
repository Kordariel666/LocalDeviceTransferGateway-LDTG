pub mod api;
pub mod state;

use crate::domain::{
    network::{self, same_network_identity, same_subnet, NetworkInterfaceInfo},
    settings::RuntimeSettings,
    shares::ShareRoots,
};
use axum::{body::Body, extract::ConnectInfo, http::Request, Extension, Router};
use hyper::body::Incoming;
use hyper_util::{
    rt::{TokioExecutor, TokioIo, TokioTimer},
    server::conn::auto::Builder as ConnectionBuilder,
    service::TowerToHyperService,
};
use state::{TransferServiceState, DOWNLOAD_MAX_DURATION};
use std::{
    collections::HashMap,
    future::Future,
    io,
    net::{IpAddr, SocketAddr},
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex as StdMutex, Weak,
    },
    task::{Context, Poll},
    time::Duration,
};
use tauri::AppHandle;
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::{TcpListener, TcpStream},
    sync::{oneshot, watch, Mutex, OwnedSemaphorePermit, Semaphore},
    task::{JoinHandle, JoinSet},
    time::{Instant as TokioInstant, Sleep},
};
use tower::ServiceExt;

const MAX_CONNECTIONS_GLOBAL: usize = 96;
const MAX_CONNECTIONS_PER_PEER: usize = 12;
const MAX_ANONYMOUS_CONNECTIONS: usize = 24;
const MAX_AUTHENTICATED_CONNECTIONS: usize = 64;
const CONNECTION_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const HEADER_READ_TIMEOUT: Duration = Duration::from_secs(15);
const ANONYMOUS_CONNECTION_LIFETIME: Duration = Duration::from_secs(30);

struct ConnectionPermits {
    _global: OwnedSemaphorePermit,
    _peer: OwnedSemaphorePermit,
}

pub(crate) struct ConnectionSecurity {
    authenticated: AtomicBool,
    anonymous: StdMutex<Option<OwnedSemaphorePermit>>,
    authenticated_permit: StdMutex<Option<OwnedSemaphorePermit>>,
    authenticated_slots: Arc<Semaphore>,
}

impl ConnectionSecurity {
    fn new(anonymous: OwnedSemaphorePermit, authenticated_slots: Arc<Semaphore>) -> Self {
        Self {
            authenticated: AtomicBool::new(false),
            anonymous: StdMutex::new(Some(anonymous)),
            authenticated_permit: StdMutex::new(None),
            authenticated_slots,
        }
    }

    pub(crate) fn mark_authenticated(&self) -> bool {
        if self.is_authenticated() {
            return true;
        }
        let mut authenticated_permit = self
            .authenticated_permit
            .lock()
            .expect("connection security lock poisoned");
        if authenticated_permit.is_some() {
            return true;
        }
        let Ok(permit) = self.authenticated_slots.clone().try_acquire_owned() else {
            return false;
        };
        *authenticated_permit = Some(permit);
        self.authenticated.store(true, Ordering::Release);
        self.anonymous
            .lock()
            .expect("connection security lock poisoned")
            .take();
        true
    }

    fn is_authenticated(&self) -> bool {
        self.authenticated.load(Ordering::Acquire)
    }
}

struct ConnectionLimiter {
    global: Arc<Semaphore>,
    anonymous: Arc<Semaphore>,
    authenticated: Arc<Semaphore>,
    per_peer: usize,
    peers: Mutex<HashMap<String, Weak<Semaphore>>>,
}

impl ConnectionLimiter {
    fn new(global: usize, per_peer: usize, anonymous: usize) -> Self {
        Self {
            global: Arc::new(Semaphore::new(global)),
            anonymous: Arc::new(Semaphore::new(anonymous)),
            authenticated: Arc::new(Semaphore::new(MAX_AUTHENTICATED_CONNECTIONS.min(global))),
            per_peer,
            peers: Mutex::new(HashMap::new()),
        }
    }

    async fn try_admit(
        &self,
        address: IpAddr,
    ) -> Option<(ConnectionPermits, Arc<ConnectionSecurity>)> {
        self.try_admit_peer(network::peer_fairness_key(address))
            .await
    }

    async fn try_admit_peer(
        &self,
        peer: String,
    ) -> Option<(ConnectionPermits, Arc<ConnectionSecurity>)> {
        let global = self.global.clone().try_acquire_owned().ok()?;
        let anonymous = self.anonymous.clone().try_acquire_owned().ok()?;
        let peer_slots = {
            let mut peers = self.peers.lock().await;
            peers.retain(|_, slots| slots.strong_count() > 0);
            if let Some(existing) = peers.get(&peer).and_then(Weak::upgrade) {
                existing
            } else {
                let created = Arc::new(Semaphore::new(self.per_peer));
                peers.insert(peer, Arc::downgrade(&created));
                created
            }
        };
        let peer = peer_slots.try_acquire_owned().ok()?;
        Some((
            ConnectionPermits {
                _global: global,
                _peer: peer,
            },
            Arc::new(ConnectionSecurity::new(
                anonymous,
                self.authenticated.clone(),
            )),
        ))
    }
}

struct AcceptedIo {
    stream: TcpStream,
    read_deadline: Pin<Box<Sleep>>,
    write_deadline: Pin<Box<Sleep>>,
    absolute_deadline: Pin<Box<Sleep>>,
    anonymous_deadline: Pin<Box<Sleep>>,
    timeout: Duration,
    security: Arc<ConnectionSecurity>,
    _permits: ConnectionPermits,
}

impl AcceptedIo {
    fn new(
        stream: TcpStream,
        permits: ConnectionPermits,
        timeout: Duration,
        max_lifetime: Duration,
        security: Arc<ConnectionSecurity>,
    ) -> Self {
        let deadline = TokioInstant::now() + timeout;
        Self {
            stream,
            read_deadline: Box::pin(tokio::time::sleep_until(deadline)),
            write_deadline: Box::pin(tokio::time::sleep_until(deadline)),
            absolute_deadline: Box::pin(tokio::time::sleep(max_lifetime)),
            anonymous_deadline: Box::pin(tokio::time::sleep(ANONYMOUS_CONNECTION_LIFETIME)),
            timeout,
            security,
            _permits: permits,
        }
    }

    fn reset_deadlines(&mut self) {
        let deadline = TokioInstant::now() + self.timeout;
        self.read_deadline.as_mut().reset(deadline);
        self.write_deadline.as_mut().reset(deadline);
    }

    fn timeout_error() -> io::Error {
        io::Error::new(io::ErrorKind::TimedOut, "LDTG connection became idle")
    }

    fn lifetime_error() -> io::Error {
        io::Error::new(
            io::ErrorKind::TimedOut,
            "LDTG connection exceeded its absolute lifetime",
        )
    }

    fn anonymous_lifetime_expired(&mut self, context: &mut Context<'_>) -> bool {
        !self.security.is_authenticated()
            && self.anonymous_deadline.as_mut().poll(context).is_ready()
    }
}

impl AsyncRead for AcceptedIo {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.absolute_deadline.as_mut().poll(context).is_ready() {
            return Poll::Ready(Err(Self::lifetime_error()));
        }
        if self.anonymous_lifetime_expired(context) {
            return Poll::Ready(Err(Self::lifetime_error()));
        }
        let before = buffer.filled().len();
        let result = Pin::new(&mut self.stream).poll_read(context, buffer);
        match result {
            Poll::Ready(Ok(())) if buffer.filled().len() > before => {
                self.reset_deadlines();
                Poll::Ready(Ok(()))
            }
            Poll::Pending if self.read_deadline.as_mut().poll(context).is_ready() => {
                Poll::Ready(Err(Self::timeout_error()))
            }
            result => result,
        }
    }
}

impl AsyncWrite for AcceptedIo {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        if self.absolute_deadline.as_mut().poll(context).is_ready() {
            return Poll::Ready(Err(Self::lifetime_error()));
        }
        if self.anonymous_lifetime_expired(context) {
            return Poll::Ready(Err(Self::lifetime_error()));
        }
        let result = Pin::new(&mut self.stream).poll_write(context, buffer);
        match result {
            Poll::Ready(Ok(written)) if written > 0 => {
                self.reset_deadlines();
                Poll::Ready(Ok(written))
            }
            Poll::Pending if self.write_deadline.as_mut().poll(context).is_ready() => {
                Poll::Ready(Err(Self::timeout_error()))
            }
            result => result,
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        if self.absolute_deadline.as_mut().poll(context).is_ready() {
            return Poll::Ready(Err(Self::lifetime_error()));
        }
        if self.anonymous_lifetime_expired(context) {
            return Poll::Ready(Err(Self::lifetime_error()));
        }
        Pin::new(&mut self.stream).poll_flush(context)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        Pin::new(&mut self.stream).poll_shutdown(context)
    }
}

async fn serve_http_connection(
    io: AcceptedIo,
    address: SocketAddr,
    router: Router,
    security: Arc<ConnectionSecurity>,
    mut shutdown: watch::Receiver<bool>,
    header_timeout: Duration,
) -> io::Result<()> {
    let tower_service = router
        .layer(Extension(ConnectInfo(address)))
        .layer(Extension(security))
        .map_request(|request: Request<Incoming>| request.map(Body::new));
    let hyper_service = TowerToHyperService::new(tower_service);
    let mut builder = ConnectionBuilder::new(TokioExecutor::new());
    builder
        .http1()
        .timer(TokioTimer::new())
        .header_read_timeout(header_timeout);
    let mut connection =
        Box::pin(builder.serve_connection_with_upgrades(TokioIo::new(io), hyper_service));
    tokio::select! {
        result = &mut connection => result.map_err(io::Error::other),
        _ = shutdown.changed() => {
            connection.as_mut().graceful_shutdown();
            connection.await.map_err(io::Error::other)
        }
    }
}

pub struct ServiceHandle {
    pub state: Arc<TransferServiceState>,
    shutdown: Option<oneshot::Sender<()>>,
    pub join: JoinHandle<()>,
}

impl ServiceHandle {
    pub fn is_finished(&self) -> bool {
        self.join.is_finished()
    }

    pub async fn stop(mut self) {
        if let Some(sender) = self.shutdown.take() {
            let _ = sender.send(());
        }
        match tokio::time::timeout(Duration::from_secs(4), &mut self.join).await {
            Ok(result) => record_join_result(&self.state, result),
            Err(_) => {
                self.join.abort();
                let _ = tokio::time::timeout(Duration::from_secs(1), &mut self.join).await;
            }
        }
        self.state.cleanup_partials().await;
    }

    pub async fn finish(mut self) -> Option<String> {
        let result = (&mut self.join).await;
        record_join_result(&self.state, result);
        self.state.cleanup_partials().await;
        self.state.stop_reason()
    }
}

fn record_join_result(state: &TransferServiceState, result: Result<(), tokio::task::JoinError>) {
    if let Err(error) = result {
        tracing::error!(%error, "HTTP service task stopped unexpectedly");
        state.set_stop_reason("Der Netzwerkdienst wurde unerwartet beendet.");
    }
}

fn record_serve_result(state: &TransferServiceState, result: io::Result<()>) {
    if let Err(error) = result {
        tracing::error!(%error, "HTTP service stopped with an error");
        state.set_stop_reason("Der Netzwerkdienst wurde wegen eines Laufzeitfehlers beendet.");
    }
}

pub async fn start(
    settings: RuntimeSettings,
    interface: NetworkInterfaceInfo,
    roots: ShareRoots,
    app: Option<AppHandle>,
) -> Result<ServiceHandle, String> {
    let address = SocketAddr::from((interface.address, settings.port));
    let listener = TcpListener::bind(address).await.map_err(|error| {
        if error.kind() == std::io::ErrorKind::AddrInUse {
            format!("PORT_IN_USE|Port {} wird bereits verwendet.", settings.port)
        } else {
            format!("Der Netzwerkdienst konnte nicht gestartet werden: {error}")
        }
    })?;
    let limiter = Arc::new(ConnectionLimiter::new(
        MAX_CONNECTIONS_GLOBAL,
        MAX_CONNECTIONS_PER_PEER,
        MAX_ANONYMOUS_CONNECTIONS,
    ));
    let state = Arc::new(
        tokio::task::spawn_blocking(move || {
            TransferServiceState::new(settings, interface, roots, app)
        })
        .await
        .map_err(|error| format!("Dienstzustand konnte nicht vorbereitet werden: {error}"))??,
    );
    let router = api::router(state.clone());
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
    let monitor_state = state.clone();
    let join = tokio::spawn(async move {
        let serve_state = monitor_state.clone();
        let (connection_shutdown, _) = watch::channel(false);
        let mut connections = JoinSet::new();
        let mut network_checks = JoinSet::new();
        let mut monitor = tokio::time::interval(Duration::from_secs(15));
        monitor.tick().await;
        loop {
            while let Some(result) = connections.try_join_next() {
                if let Ok(Err(error)) = result {
                    tracing::debug!(%error, "HTTP connection closed with an error");
                }
            }
            tokio::select! {
                _ = &mut shutdown_rx => break,
                _ = monitor.tick() => {
                    monitor_state.expire_stale_sessions().await;
                    monitor_state.expire_stale_uploads().await;
                    monitor_state.expire_stale_directory_listings().await;
                    if monitor_state.should_auto_stop().await {
                        monitor_state.set_stop_reason("AUTO_STOP");
                        monitor_state.emit_auto_stop();
                        break;
                    }
                    if network_checks.is_empty() {
                        let expected = monitor_state.interface.clone();
                        let checked_state = monitor_state.clone();
                        network_checks.spawn_blocking(move || {
                            let roots_are_current = checked_state.roots_are_current();
                            let network_is_current = network::list_interfaces()
                                .into_iter()
                                .any(|item| same_network_identity(&expected, &item));
                            (roots_are_current, network_is_current)
                        });
                    }
                }
                Some(result) = network_checks.join_next(), if !network_checks.is_empty() => {
                    if matches!(result, Ok((false, _))) {
                        monitor_state.set_stop_reason("Ein freigegebener Ordner wurde ersetzt oder umgeleitet. LDTG wurde sicher gestoppt.");
                        break;
                    }
                    if !matches!(result, Ok((true, true))) {
                        monitor_state.set_stop_reason("Die ausgewählte Netzwerkverbindung wurde getrennt oder geändert. LDTG wurde sicher gestoppt.");
                        monitor_state.emit_network_lost();
                        break;
                    }
                }
                accepted = listener.accept() => {
                    match accepted {
                        Ok((stream, address)) => {
                            let IpAddr::V4(client_v4) = address.ip() else {
                                drop(stream);
                                continue;
                            };
                            if !same_subnet(client_v4, &monitor_state.interface) {
                                drop(stream);
                                continue;
                            }
                            if let Some((permits, security)) = limiter.try_admit(address.ip()).await {
                                let io = AcceptedIo::new(
                                    stream,
                                    permits,
                                    CONNECTION_IDLE_TIMEOUT,
                                    DOWNLOAD_MAX_DURATION,
                                    security.clone(),
                                );
                                let connection_router = router.clone();
                                let connection_shutdown = connection_shutdown.subscribe();
                                connections.spawn(async move {
                                    serve_http_connection(
                                        io,
                                        address,
                                        connection_router,
                                        security,
                                        connection_shutdown,
                                        HEADER_READ_TIMEOUT,
                                    ).await
                                });
                            }
                        }
                        Err(error) => {
                            tracing::error!(%error, "TCP listener stopped with an error");
                            monitor_state.set_stop_reason("Der Netzwerkdienst konnte keine Verbindungen mehr annehmen.");
                            break;
                        }
                    }
                }
                Some(result) = connections.join_next(), if !connections.is_empty() => {
                    if let Ok(Err(error)) = result {
                        tracing::debug!(%error, "HTTP connection closed with an error");
                    }
                }
            }
        }
        network_checks.abort_all();
        let _ = connection_shutdown.send(true);
        let drain = async {
            while let Some(result) = connections.join_next().await {
                if let Ok(Err(error)) = result {
                    tracing::debug!(%error, "HTTP connection closed during shutdown");
                }
            }
        };
        if tokio::time::timeout(Duration::from_secs(3), drain)
            .await
            .is_err()
        {
            connections.abort_all();
        }
        record_serve_result(&serve_state, Ok(()));
    });
    Ok(ServiceHandle {
        state,
        shutdown: Some(shutdown_tx),
        join,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{network::NetworkInterfaceInfo, shares::ShareRoots};
    use std::net::Ipv4Addr;
    use std::sync::{Condvar, Mutex as StdMutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn test_state() -> Arc<TransferServiceState> {
        Arc::new(
            TransferServiceState::new(
                RuntimeSettings::default(),
                NetworkInterfaceInfo {
                    id: "lan|192.168.10.2".into(),
                    name: "lan".into(),
                    profile_name: "Testnetz".into(),
                    address: Ipv4Addr::new(192, 168, 10, 2),
                    prefix_length: 24,
                    network_id: "lan|Testnetz|192.168.10.0/24".into(),
                    category: "Privat".into(),
                    profile_resolved: true,
                    preferred: true,
                    netmask: Ipv4Addr::new(255, 255, 255, 0),
                },
                ShareRoots {
                    download: None,
                    upload: None,
                },
                None,
            )
            .unwrap(),
        )
    }

    #[tokio::test]
    async fn connection_limiter_enforces_global_and_address_caps() {
        let limiter = ConnectionLimiter::new(2, 1, 2);
        let first_address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 10));
        let second_address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 11));
        let third_address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 12));
        let first = limiter.try_admit(first_address).await.unwrap();
        assert!(limiter.try_admit(first_address).await.is_none());
        let second = limiter.try_admit(second_address).await.unwrap();
        assert!(limiter.try_admit(third_address).await.is_none());
        drop(first);
        assert!(limiter.try_admit(third_address).await.is_some());
        drop(second);
    }

    #[tokio::test]
    async fn ip_aliases_cannot_consume_the_anonymous_connection_reserve() {
        let limiter = ConnectionLimiter::new(10, 10, 2);
        let (_, first_security) = limiter
            .try_admit(IpAddr::V4(Ipv4Addr::new(192, 168, 10, 10)))
            .await
            .unwrap();
        let second = limiter
            .try_admit(IpAddr::V4(Ipv4Addr::new(192, 168, 10, 11)))
            .await
            .unwrap();
        assert!(limiter
            .try_admit(IpAddr::V4(Ipv4Addr::new(192, 168, 10, 12)))
            .await
            .is_none());

        assert!(first_security.mark_authenticated());
        assert!(limiter
            .try_admit(IpAddr::V4(Ipv4Addr::new(192, 168, 10, 12)))
            .await
            .is_some());
        drop(second);
    }

    #[tokio::test]
    async fn link_layer_aliases_share_one_peer_limit() {
        let limiter = ConnectionLimiter::new(10, 1, 10);
        let peer = "neighbor:7:001122334455".to_string();
        let first = limiter.try_admit_peer(peer.clone()).await.unwrap();

        assert!(limiter.try_admit_peer(peer).await.is_none());
        assert_eq!(limiter.anonymous.available_permits(), 9);
        drop(first);
    }

    #[test]
    fn concurrent_authentication_uses_only_one_authenticated_slot() {
        use std::sync::Barrier;

        let anonymous_slots = Arc::new(Semaphore::new(1));
        let anonymous = anonymous_slots.clone().try_acquire_owned().unwrap();
        let authenticated_slots = Arc::new(Semaphore::new(2));
        let security = Arc::new(ConnectionSecurity::new(
            anonymous,
            authenticated_slots.clone(),
        ));
        let barrier = Arc::new(Barrier::new(3));
        let mut threads = Vec::new();
        for _ in 0..2 {
            let security = security.clone();
            let barrier = barrier.clone();
            threads.push(std::thread::spawn(move || {
                barrier.wait();
                assert!(security.mark_authenticated());
            }));
        }
        barrier.wait();
        for thread in threads {
            thread.join().unwrap();
        }

        assert_eq!(authenticated_slots.available_permits(), 1);
        assert_eq!(anonymous_slots.available_permits(), 1);
    }

    #[tokio::test]
    async fn idle_connection_io_times_out() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let client = TcpStream::connect(address);
        let accepted = listener.accept();
        let (client, accepted) = tokio::join!(client, accepted);
        let _client = client.unwrap();
        let (server, remote) = accepted.unwrap();
        let limiter = ConnectionLimiter::new(1, 1, 1);
        let (permits, security) = limiter.try_admit(remote.ip()).await.unwrap();
        let mut io = AcceptedIo::new(
            server,
            permits,
            Duration::from_millis(5),
            Duration::from_secs(1),
            security,
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
        let mut byte = [0_u8; 1];
        let error = io.read(&mut byte).await.unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    }

    #[tokio::test]
    async fn connection_lifetime_is_not_extended_by_io_progress() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let client = TcpStream::connect(address);
        let accepted = listener.accept();
        let (client, accepted) = tokio::join!(client, accepted);
        let mut client = client.unwrap();
        let (server, remote) = accepted.unwrap();
        let limiter = ConnectionLimiter::new(1, 1, 1);
        let (permits, security) = limiter.try_admit(remote.ip()).await.unwrap();
        let mut io = AcceptedIo::new(
            server,
            permits,
            Duration::from_millis(200),
            Duration::from_millis(40),
            security,
        );
        let writer = tokio::spawn(async move {
            loop {
                if client.write_all(b"x").await.is_err() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        });

        let mut bytes = 0;
        let error = loop {
            let mut byte = [0_u8; 1];
            match io.read(&mut byte).await {
                Ok(0) => panic!("client closed before the absolute lifetime elapsed"),
                Ok(read) => bytes += read,
                Err(error) => break error,
            }
        };
        writer.abort();
        let _ = writer.await;

        assert!(bytes > 1, "test must exercise progressing I/O");
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert!(error.to_string().contains("absolute lifetime"));
    }

    #[tokio::test]
    async fn absolute_header_timeout_is_not_extended_by_drip_fed_bytes() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let client = TcpStream::connect(address);
        let accepted = listener.accept();
        let (client, accepted) = tokio::join!(client, accepted);
        let mut client = client.unwrap();
        let (server, remote) = accepted.unwrap();
        let limiter = ConnectionLimiter::new(1, 1, 1);
        let (permits, security) = limiter.try_admit(remote.ip()).await.unwrap();
        let io = AcceptedIo::new(
            server,
            permits,
            Duration::from_secs(1),
            Duration::from_secs(1),
            security.clone(),
        );
        let (_shutdown, receiver) = watch::channel(false);
        let server = tokio::spawn(serve_http_connection(
            io,
            remote,
            Router::new(),
            security,
            receiver,
            Duration::from_millis(30),
        ));

        for byte in b"GET / HTTP/1.1\r" {
            if client.write_all(&[*byte]).await.is_err() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }

        let result = tokio::time::timeout(Duration::from_secs(1), server)
            .await
            .expect("the parser must close a slow header connection")
            .expect("connection task must not panic");
        assert!(result.is_err());
    }

    #[test]
    fn serve_error_sets_a_visible_stop_reason() {
        let state = test_state();
        record_serve_result(&state, Err(io::Error::other("test failure")));
        assert_eq!(
            state.stop_reason().as_deref(),
            Some("Der Netzwerkdienst wurde wegen eines Laufzeitfehlers beendet.")
        );
    }

    #[tokio::test]
    async fn join_failure_sets_a_visible_stop_reason() {
        let state = test_state();
        let handle = ServiceHandle {
            state: state.clone(),
            shutdown: None,
            join: tokio::spawn(async { panic!("simulated server panic") }),
        };
        let reason = handle.finish().await;
        assert_eq!(
            reason.as_deref(),
            Some("Der Netzwerkdienst wurde unerwartet beendet.")
        );
    }

    #[tokio::test]
    async fn stalled_blocking_monitor_work_does_not_delay_service_stop() {
        let state = test_state();
        let release = Arc::new((StdMutex::new(false), Condvar::new()));
        let worker_release = release.clone();
        let (started, started_rx) = std::sync::mpsc::channel();
        let (shutdown, shutdown_rx) = oneshot::channel();
        let join = tokio::spawn(async move {
            let mut checks = JoinSet::new();
            checks.spawn_blocking(move || {
                started.send(()).unwrap();
                let (flag, wake) = &*worker_release;
                let mut released = flag.lock().unwrap();
                while !*released {
                    released = wake.wait(released).unwrap();
                }
            });
            let _ = shutdown_rx.await;
            checks.abort_all();
        });
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if started_rx.try_recv().is_ok() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("blocking monitor work must start");
        let handle = ServiceHandle {
            state,
            shutdown: Some(shutdown),
            join,
        };
        tokio::time::timeout(Duration::from_millis(500), handle.stop())
            .await
            .expect("service stop must not await detached blocking monitor work");

        let (flag, wake) = &*release;
        *flag.lock().unwrap() = true;
        wake.notify_all();
    }

    #[tokio::test]
    async fn shutdown_wins_over_a_simultaneously_ready_failed_network_check() {
        let (shutdown, mut shutdown_rx) = oneshot::channel();
        let mut checks = JoinSet::new();
        let (finished, finished_rx) = oneshot::channel();
        checks.spawn(async move {
            let _ = finished.send(());
            false
        });
        finished_rx.await.unwrap();
        tokio::task::yield_now().await;
        shutdown.send(()).unwrap();

        let mut emitted_network_loss = false;
        tokio::select! {
            biased;
            _ = &mut shutdown_rx => {}
            Some(result) = checks.join_next() => {
                emitted_network_loss = !matches!(result, Ok(true));
            }
        }
        assert!(!emitted_network_loss);
    }
}
