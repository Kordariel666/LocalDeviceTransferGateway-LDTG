use super::*;
use crate::domain::{
    network::NetworkInterfaceInfo,
    settings::{AppSettings, ShareSettings},
    shares::ShareRoots,
};
use crate::service::state::UploadIoTestGate;
use axum::body::to_bytes;
use std::{io::Write, net::Ipv4Addr, sync::Condvar};
use tower::ServiceExt;

fn test_state(root: &Path) -> Arc<TransferServiceState> {
    // Production roots are canonicalized by `prepare_roots`. Mirror that
    // invariant here as Windows may otherwise compare a `\\?\` path with
    // the non-canonical temporary-directory spelling.
    let root = fs::canonicalize(root).unwrap();
    let settings = AppSettings {
        download_share: ShareSettings {
            enabled: true,
            path: root.display().to_string(),
        },
        upload_share: ShareSettings {
            enabled: true,
            path: root.display().to_string(),
        },
        ..AppSettings::default()
    };
    let interface = NetworkInterfaceInfo {
        id: "lan|192.168.10.2".into(),
        name: "lan".into(),
        profile_name: "Testnetz".into(),
        address: Ipv4Addr::new(192, 168, 10, 2),
        prefix_length: 24,
        network_id: "lan-test".into(),
        category: "Privat".into(),
        profile_resolved: true,
        preferred: true,
        netmask: Ipv4Addr::new(255, 255, 255, 0),
    };
    let state = Arc::new(
        TransferServiceState::new(
            settings,
            interface,
            ShareRoots {
                download: Some(root.clone()),
                upload: Some(root),
            },
            None,
        )
        .unwrap(),
    );
    *state.access_code.write().unwrap() = "12345678".into();
    state
}

fn request(method: Method, uri: &str, body: Body) -> Request<Body> {
    request_from(method, uri, body, Ipv4Addr::new(192, 168, 10, 50))
}

fn request_from(method: Method, uri: &str, body: Body, address: Ipv4Addr) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method.clone())
        .uri(uri)
        .header(header::HOST, "192.168.10.2:8765");
    if matches!(
        method,
        Method::POST | Method::PATCH | Method::DELETE | Method::PUT
    ) {
        builder = builder.header(header::ORIGIN, "http://192.168.10.2:8765");
    }
    let mut request = builder.body(body).unwrap();
    request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::from((address, 54000))));
    request
}

async fn json(response: Response) -> serde_json::Value {
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

async fn session_headers_for(state: &TransferServiceState, address: Ipv4Addr) -> (String, String) {
    let session = state
        .create_session(IpAddr::V4(address), "Testbrowser".into())
        .await
        .unwrap();
    (format!("dmdc_session={}", session.token), session.csrf)
}

async fn session_headers(state: &TransferServiceState) -> (String, String) {
    session_headers_for(state, Ipv4Addr::new(192, 168, 10, 50)).await
}

#[test]
fn parses_download_ranges() {
    let mut headers = HeaderMap::new();
    headers.insert(header::RANGE, HeaderValue::from_static("bytes=10-19"));
    assert_eq!(parse_range(&headers, 100).unwrap(), (10, 19, true));
    headers.insert(header::RANGE, HeaderValue::from_static("bytes=-7"));
    assert_eq!(parse_range(&headers, 100).unwrap(), (93, 99, true));
    headers.insert(header::RANGE, HeaderValue::from_static("bytes=100-"));
    assert!(parse_range(&headers, 100).is_err());
}

#[test]
fn sorts_names_naturally_without_case() {
    assert_eq!(
        natural_compare("Datei 2.txt", "datei 10.txt"),
        Ordering::Less
    );
    assert_eq!(natural_compare("BILD.JPG", "bild.jpg"), Ordering::Equal);
}

#[tokio::test]
async fn authenticates_and_sets_http_only_cookie() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path());
    let app = router(state.clone());
    let mut auth = request(
        Method::POST,
        "/api/v1/auth",
        Body::from(r#"{"code":"12345678","deviceName":"  Marias iPhone  "}"#),
    );
    auth.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    auth.headers_mut().insert(
        header::USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 Version/18.0 Mobile/15E148 Safari/604.1",
        ),
    );
    let response = app.oneshot(auth).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Strict"));
    assert!(!cookie.contains("Secure"));
    let sessions = state.status().await.sessions;
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].device_name.as_deref(), Some("Marias iPhone"));
    assert_eq!(sessions[0].client_name, "Safari auf iPhone");
    assert_eq!(sessions[0].address, "192.168.10.50");
    assert!(!sessions[0].created_at.is_empty());
    assert!(!sessions[0].last_activity.is_empty());
}

#[tokio::test]
async fn rejects_unsafe_session_device_names() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path());
    let app = router(state.clone());
    let mut auth = request(
        Method::POST,
        "/api/v1/auth",
        Body::from("{\"code\":\"12345678\",\"deviceName\":\"Handy\\u202ePC\"}"),
    );
    auth.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    let response = app.oneshot(auth).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json(response).await["code"], "DEVICE_NAME_INVALID");
    assert!(state.status().await.sessions.is_empty());
}

#[tokio::test]
async fn rejects_wrong_host_and_outside_subnet() {
    let temp = tempfile::tempdir().unwrap();
    let app = router(test_state(temp.path()));
    let mut wrong_host = request(Method::GET, "/", Body::empty());
    wrong_host
        .headers_mut()
        .insert(header::HOST, HeaderValue::from_static("example.test"));
    assert_eq!(
        app.clone().oneshot(wrong_host).await.unwrap().status(),
        StatusCode::MISDIRECTED_REQUEST
    );

    let mut outside = request(Method::GET, "/", Body::empty());
    outside
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::from((
            Ipv4Addr::new(192, 168, 11, 5),
            54000,
        ))));
    assert_eq!(
        app.oneshot(outside).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn enforces_csrf_for_writes() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path());
    let (cookie, _) = session_headers(&state).await;
    let app = router(state);
    let mut logout = request(Method::POST, "/api/v1/logout", Body::empty());
    logout
        .headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    let response = app.oneshot(logout).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(json(response).await["code"], "CSRF_INVALID");
}

#[tokio::test]
async fn locks_ip_on_the_tenth_wrong_code() {
    let temp = tempfile::tempdir().unwrap();
    let app = router(test_state(temp.path()));
    for _ in 0..9 {
        let mut auth = request(
            Method::POST,
            "/api/v1/auth",
            Body::from(r#"{"code":"00000000"}"#),
        );
        auth.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        assert_eq!(
            app.clone().oneshot(auth).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
    }
    let mut tenth = request(
        Method::POST,
        "/api/v1/auth",
        Body::from(r#"{"code":"00000000"}"#),
    );
    tenth.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    assert_eq!(
        app.clone().oneshot(tenth).await.unwrap().status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    let mut blocked = request(
        Method::POST,
        "/api/v1/auth",
        Body::from(r#"{"code":"12345678"}"#),
    );
    blocked.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    assert_eq!(
        app.oneshot(blocked).await.unwrap().status(),
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[tokio::test]
async fn serializes_concurrent_failed_code_attempts() {
    let temp = tempfile::tempdir().unwrap();
    let app = router(test_state(temp.path()));
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..20 {
        let app = app.clone();
        tasks.spawn(async move {
            let mut auth = request(
                Method::POST,
                "/api/v1/auth",
                Body::from(r#"{"code":"00000000"}"#),
            );
            auth.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            app.oneshot(auth).await.unwrap().status()
        });
    }
    let mut unauthorized = 0;
    let mut blocked = 0;
    while let Some(result) = tasks.join_next().await {
        match result.unwrap() {
            StatusCode::UNAUTHORIZED => unauthorized += 1,
            StatusCode::TOO_MANY_REQUESTS => blocked += 1,
            status => panic!("unerwarteter Status: {status}"),
        }
    }
    assert_eq!(unauthorized, 9);
    assert_eq!(blocked, 11);
}

#[tokio::test]
async fn distributed_failures_block_without_rotating_the_service_code() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path());
    let app = router(state.clone());
    for index in 1..50 {
        let mut auth = request_from(
            Method::POST,
            "/api/v1/auth",
            Body::from(r#"{"code":"00000000"}"#),
            Ipv4Addr::new(192, 168, 10, index),
        );
        auth.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        assert_eq!(
            app.clone().oneshot(auth).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
    }
    let mut threshold = request_from(
        Method::POST,
        "/api/v1/auth",
        Body::from(r#"{"code":"00000000"}"#),
        Ipv4Addr::new(192, 168, 10, 50),
    );
    threshold.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    let response = app.clone().oneshot(threshold).await.unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(json(response).await["code"], "SERVICE_CODE_BLOCKED");
    let unchanged = state.access_code.read().unwrap().clone();
    assert_eq!(unchanged, "12345678");

    let mut recovery = request_from(
        Method::POST,
        "/api/v1/auth",
        Body::from(format!(r#"{{"code":"{unchanged}"}}"#)),
        Ipv4Addr::new(192, 168, 10, 51),
    );
    recovery.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    assert_eq!(
        app.oneshot(recovery).await.unwrap().status(),
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[tokio::test]
async fn rejects_patch_before_polling_an_unauthenticated_body() {
    let temp = tempfile::tempdir().unwrap();
    let body = Body::from_stream(async_stream::stream! {
        if std::hint::black_box(true) {
            panic!("unauthenticated PATCH body must not be polled");
        }
        yield Ok::<Bytes, std::io::Error>(Bytes::from_static(b"blocked"));
    });
    let patch = request(Method::PATCH, "/api/v1/uploads/not-owned", body);
    let response = router(test_state(temp.path()))
        .oneshot(patch)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(json(response).await["code"], "AUTH_REQUIRED");
}

#[tokio::test]
async fn rejects_unknown_owned_patch_before_polling_its_authenticated_body() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path());
    let (cookie, csrf) = session_headers(&state).await;
    let body = Body::from_stream(async_stream::stream! {
        if std::hint::black_box(true) {
            panic!("unknown authenticated PATCH body must not be polled");
        }
        yield Ok::<Bytes, std::io::Error>(Bytes::from_static(b"blocked"));
    });
    let mut patch = request(Method::PATCH, "/api/v1/uploads/not-owned", body);
    patch
        .headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    patch
        .headers_mut()
        .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
    let response = router(state).oneshot(patch).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(json(response).await["code"], "UPLOAD_NOT_FOUND");
}

#[tokio::test]
async fn rejects_foreign_patch_before_polling_its_authenticated_body() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path());
    let address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
    let owner = state.create_session(address, "owner".into()).await.unwrap();
    let attacker = state
        .create_session(address, "attacker".into())
        .await
        .unwrap();
    let id = Uuid::new_v4().to_string();
    let partial_path = state
        .partial_dir
        .as_ref()
        .unwrap()
        .join(format!("{id}.part"));
    let partial_file = create_upload_partial(&partial_path).unwrap();
    state.uploads.lock().await.insert(
        id.clone(),
        Arc::new(Mutex::new(UploadRecord {
            id: id.clone(),
            owner_session: owner.id,
            owner_address: address,
            name: "foreign.bin".into(),
            declared_size: 1,
            offset: 0,
            last_modified: 1,
            client_token: "foreign-token".into(),
            created_at: Instant::now(),
            last_activity: Instant::now(),
            cancelled: false,
            finalizing: false,
            cancel_signal: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            chunk_slots: Arc::new(tokio::sync::Semaphore::new(1)),
            partial_path,
            partial_file: Arc::new(partial_file),
            transfer_id: "transfer".into(),
        })),
    );
    let body = Body::from_stream(async_stream::stream! {
        if std::hint::black_box(true) {
            panic!("foreign authenticated PATCH body must not be polled");
        }
        yield Ok::<Bytes, std::io::Error>(Bytes::from_static(b"blocked"));
    });
    let mut patch = request(Method::PATCH, &format!("/api/v1/uploads/{id}"), body);
    patch.headers_mut().insert(
        header::COOKIE,
        HeaderValue::from_str(&format!("dmdc_session={}", attacker.token)).unwrap(),
    );
    patch.headers_mut().insert(
        "x-dmdc-csrf",
        HeaderValue::from_str(&attacker.csrf).unwrap(),
    );
    let response = router(state).oneshot(patch).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(json(response).await["code"], "UPLOAD_OWNER_MISMATCH");
}

#[tokio::test]
async fn rejects_parallel_patch_before_polling_a_second_body_for_the_same_upload() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path());
    let address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
    let session = state.create_session(address, "owner".into()).await.unwrap();
    let id = Uuid::new_v4().to_string();
    let partial_path = state
        .partial_dir
        .as_ref()
        .unwrap()
        .join(format!("{id}.part"));
    let partial_file = create_upload_partial(&partial_path).unwrap();
    let record = Arc::new(Mutex::new(UploadRecord {
        id: id.clone(),
        owner_session: session.id,
        owner_address: address,
        name: "parallel.bin".into(),
        declared_size: 1,
        offset: 0,
        last_modified: 1,
        client_token: "parallel-token".into(),
        created_at: Instant::now(),
        last_activity: Instant::now(),
        cancelled: false,
        finalizing: false,
        cancel_signal: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        chunk_slots: Arc::new(tokio::sync::Semaphore::new(1)),
        partial_path,
        partial_file: Arc::new(partial_file),
        transfer_id: "transfer".into(),
    }));
    state
        .uploads
        .lock()
        .await
        .insert(id.clone(), record.clone());
    let active_chunk = {
        let record = record.lock().await;
        state.begin_upload_chunk(&record).unwrap()
    };
    let body = Body::from_stream(async_stream::stream! {
        if std::hint::black_box(true) {
            panic!("parallel PATCH body must not be polled");
        }
        yield Ok::<Bytes, std::io::Error>(Bytes::from_static(b"blocked"));
    });
    let mut patch = request(Method::PATCH, &format!("/api/v1/uploads/{id}"), body);
    patch.headers_mut().insert(
        header::COOKIE,
        HeaderValue::from_str(&format!("dmdc_session={}", session.token)).unwrap(),
    );
    patch
        .headers_mut()
        .insert("x-dmdc-csrf", HeaderValue::from_str(&session.csrf).unwrap());
    patch
        .headers_mut()
        .insert("upload-offset", HeaderValue::from_static("0"));

    let response = router(state).oneshot(patch).await.unwrap();
    drop(active_chunk);
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(json(response).await["code"], "UPLOAD_CHUNK_BUSY");
}

#[tokio::test]
async fn blocked_upload_io_does_not_delay_service_cleanup() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path());
    let address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
    let session = state.create_session(address, "owner".into()).await.unwrap();
    let id = Uuid::new_v4().to_string();
    let partial_path = state
        .partial_dir
        .as_ref()
        .unwrap()
        .join(format!("{id}.part"));
    let partial_file = create_upload_partial(&partial_path).unwrap();
    state.reserve_upload_object().unwrap();
    state.uploads.lock().await.insert(
        id.clone(),
        Arc::new(Mutex::new(UploadRecord {
            id: id.clone(),
            owner_session: session.id,
            owner_address: address,
            name: "blocked.bin".into(),
            declared_size: 1,
            offset: 0,
            last_modified: 1,
            client_token: "blocked-token".into(),
            created_at: Instant::now(),
            last_activity: Instant::now(),
            cancelled: false,
            finalizing: false,
            cancel_signal: Arc::new(AtomicBool::new(false)),
            chunk_slots: Arc::new(tokio::sync::Semaphore::new(1)),
            partial_path,
            partial_file: Arc::new(partial_file),
            transfer_id: "blocked-transfer".into(),
        })),
    );
    let release = Arc::new((std::sync::Mutex::new(false), Condvar::new()));
    let (started, started_rx) = std::sync::mpsc::channel();
    *state.upload_io_test_gate.lock().unwrap() = Some(UploadIoTestGate {
        started,
        release: release.clone(),
    });

    let mut patch = request(
        Method::PATCH,
        &format!("/api/v1/uploads/{id}"),
        Body::from("x"),
    );
    patch.headers_mut().insert(
        header::COOKIE,
        HeaderValue::from_str(&format!("dmdc_session={}", session.token)).unwrap(),
    );
    patch
        .headers_mut()
        .insert("x-dmdc-csrf", HeaderValue::from_str(&session.csrf).unwrap());
    patch
        .headers_mut()
        .insert("upload-offset", HeaderValue::from_static("0"));
    let app = router(state.clone());
    let chunk = tokio::spawn(async move { app.oneshot(patch).await.unwrap() });
    tokio::task::spawn_blocking(move || started_rx.recv_timeout(Duration::from_secs(1)))
        .await
        .unwrap()
        .expect("blocking upload I/O must start");

    tokio::time::timeout(Duration::from_secs(1), state.cleanup_partials())
        .await
        .expect("service cleanup must not wait for blocked upload I/O");
    let (released, wake) = &*release;
    *released.lock().unwrap() = true;
    wake.notify_all();
    let response = tokio::time::timeout(Duration::from_secs(1), chunk)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(state.active_upload_bytes_for_test(), 0);
}

#[tokio::test]
async fn dropped_chunk_waiter_keeps_permits_and_commits_durable_progress() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path());
    let address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
    let session = state.create_session(address, "owner".into()).await.unwrap();
    let id = Uuid::new_v4().to_string();
    let partial_path = state
        .partial_dir
        .as_ref()
        .unwrap()
        .join(format!("{id}.part"));
    let partial_file = create_upload_partial(&partial_path).unwrap();
    let transfer_id = state
        .record_transfer(TransferDirection::Upload, "detached.bin", 1)
        .await;
    state.reserve_upload_object().unwrap();
    let record = Arc::new(Mutex::new(UploadRecord {
        id: id.clone(),
        owner_session: session.id,
        owner_address: address,
        name: "detached.bin".into(),
        declared_size: 1,
        offset: 0,
        last_modified: 1,
        client_token: "detached-token".into(),
        created_at: Instant::now(),
        last_activity: Instant::now(),
        cancelled: false,
        finalizing: false,
        cancel_signal: Arc::new(AtomicBool::new(false)),
        chunk_slots: Arc::new(tokio::sync::Semaphore::new(1)),
        partial_path: partial_path.clone(),
        partial_file: Arc::new(partial_file),
        transfer_id: transfer_id.clone(),
    }));
    state
        .uploads
        .lock()
        .await
        .insert(id.clone(), record.clone());
    let release = Arc::new((std::sync::Mutex::new(false), Condvar::new()));
    let (started, started_rx) = std::sync::mpsc::channel();
    *state.upload_io_test_gate.lock().unwrap() = Some(UploadIoTestGate {
        started,
        release: release.clone(),
    });

    let mut patch = request(
        Method::PATCH,
        &format!("/api/v1/uploads/{id}"),
        Body::from("x"),
    );
    patch.headers_mut().insert(
        header::COOKIE,
        HeaderValue::from_str(&format!("dmdc_session={}", session.token)).unwrap(),
    );
    patch
        .headers_mut()
        .insert("x-dmdc-csrf", HeaderValue::from_str(&session.csrf).unwrap());
    patch
        .headers_mut()
        .insert("upload-offset", HeaderValue::from_static("0"));
    let app = router(state.clone());
    let waiter = tokio::spawn(async move { app.oneshot(patch).await });
    tokio::task::spawn_blocking(move || started_rx.recv_timeout(Duration::from_secs(1)))
        .await
        .unwrap()
        .expect("blocking upload I/O must start");
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    {
        let record = record.lock().await;
        assert!(matches!(state.begin_upload_chunk(&record), Err("upload")));
    }

    let (released, wake) = &*release;
    *released.lock().unwrap() = true;
    wake.notify_all();
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if record.lock().await.offset == 1 {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("detached blocking upload must publish its durable offset");
    assert_eq!(state.active_upload_bytes_for_test(), 1);
    assert_eq!(
        record.lock().await.partial_file.metadata().unwrap().len(),
        1
    );
    let transfer = state
        .transfers
        .lock()
        .await
        .iter()
        .find(|item| item.id == transfer_id)
        .cloned()
        .unwrap();
    assert_eq!(transfer.transferred_bytes, 1);
    let record = record.lock().await;
    assert!(state.begin_upload_chunk(&record).is_ok());
}

#[tokio::test]
async fn failed_chunk_write_preserves_offset_budget_and_transfer_state() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path());
    let address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
    let session = state.create_session(address, "owner".into()).await.unwrap();
    let id = Uuid::new_v4().to_string();
    let partial_path = state
        .partial_dir
        .as_ref()
        .unwrap()
        .join(format!("{id}.part"));
    drop(create_upload_partial(&partial_path).unwrap());
    let partial_file = fs::OpenOptions::new()
        .read(true)
        .open(&partial_path)
        .unwrap();
    let transfer_id = state
        .record_transfer(TransferDirection::Upload, "readonly.bin", 1)
        .await;
    state.reserve_upload_object().unwrap();
    let record = Arc::new(Mutex::new(UploadRecord {
        id: id.clone(),
        owner_session: session.id,
        owner_address: address,
        name: "readonly.bin".into(),
        declared_size: 1,
        offset: 0,
        last_modified: 1,
        client_token: "readonly-token".into(),
        created_at: Instant::now(),
        last_activity: Instant::now(),
        cancelled: false,
        finalizing: false,
        cancel_signal: Arc::new(AtomicBool::new(false)),
        chunk_slots: Arc::new(tokio::sync::Semaphore::new(1)),
        partial_path: partial_path.clone(),
        partial_file: Arc::new(partial_file),
        transfer_id: transfer_id.clone(),
    }));
    state
        .uploads
        .lock()
        .await
        .insert(id.clone(), record.clone());

    let mut patch = request(
        Method::PATCH,
        &format!("/api/v1/uploads/{id}"),
        Body::from("x"),
    );
    patch.headers_mut().insert(
        header::COOKIE,
        HeaderValue::from_str(&format!("dmdc_session={}", session.token)).unwrap(),
    );
    patch
        .headers_mut()
        .insert("x-dmdc-csrf", HeaderValue::from_str(&session.csrf).unwrap());
    patch
        .headers_mut()
        .insert("upload-offset", HeaderValue::from_static("0"));
    let response = router(state.clone()).oneshot(patch).await.unwrap();
    assert_eq!(response.status(), StatusCode::INSUFFICIENT_STORAGE);
    assert_eq!(json(response).await["code"], "PARTIAL_WRITE_FAILED");
    assert_eq!(record.lock().await.offset, 0);
    assert_eq!(state.active_upload_bytes_for_test(), 0);
    let transfer = state
        .transfers
        .lock()
        .await
        .iter()
        .find(|item| item.id == transfer_id)
        .cloned()
        .unwrap();
    assert_eq!(transfer.transferred_bytes, 0);
    assert_eq!(transfer.state, TransferState::Active);
    assert_eq!(fs::metadata(partial_path).unwrap().len(), 0);
}

#[tokio::test]
async fn directory_page_stops_after_a_bounded_number_of_hidden_entries() {
    let temp = tempfile::tempdir().unwrap();
    for index in 0..(DIRECTORY_SCAN_LIMIT + 40) {
        fs::write(temp.path().join(format!(".hidden-{index:04}")), b"x").unwrap();
    }
    let state = test_state(temp.path());
    let (cookie, _) = session_headers(&state).await;
    let app = router(state);
    let mut list = request(Method::GET, "/api/v1/downloads?path=", Body::empty());
    list.headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    let response = json(app.oneshot(list).await.unwrap()).await;
    assert!(response["entries"].as_array().unwrap().is_empty());
    assert!(response["nextCursor"].as_str().is_some());
}

#[tokio::test]
async fn directory_cursor_is_bound_to_its_session() {
    let temp = tempfile::tempdir().unwrap();
    for index in 0..(DIRECTORY_SCAN_LIMIT + 1) {
        fs::write(temp.path().join(format!(".hidden-{index:04}")), b"x").unwrap();
    }
    let state = test_state(temp.path());
    let (first_cookie, _) = session_headers(&state).await;
    let (second_cookie, _) = session_headers(&state).await;
    let app = router(state);
    let mut first = request(Method::GET, "/api/v1/downloads?path=", Body::empty());
    first.headers_mut().insert(
        header::COOKIE,
        HeaderValue::from_str(&first_cookie).unwrap(),
    );
    let first = json(app.clone().oneshot(first).await.unwrap()).await;
    let cursor = first["nextCursor"].as_str().unwrap();

    let mut foreign = request(
        Method::GET,
        &format!("/api/v1/downloads?path=&cursor={cursor}&page=1"),
        Body::empty(),
    );
    foreign.headers_mut().insert(
        header::COOKIE,
        HeaderValue::from_str(&second_cookie).unwrap(),
    );
    let response = app.oneshot(foreign).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json(response).await["code"], "CURSOR_INVALID");
}

#[tokio::test]
async fn directory_cursor_replays_lost_intermediate_and_final_pages_without_skipping() {
    let temp = tempfile::tempdir().unwrap();
    for index in 0..(DIRECTORY_PAGE_SIZE * 2 + 10) {
        fs::write(temp.path().join(format!("file-{index:04}.txt")), b"x").unwrap();
    }
    let state = test_state(temp.path());
    let (cookie, _) = session_headers(&state).await;
    let app = router(state);

    let mut first = request(Method::GET, "/api/v1/downloads?path=", Body::empty());
    first
        .headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    let first = json(app.clone().oneshot(first).await.unwrap()).await;
    let cursor = first["nextCursor"].as_str().unwrap();
    let page = first["nextPage"].as_u64().unwrap();

    let page_uri = format!("/api/v1/downloads?path=&cursor={cursor}&page={page}");
    let mut next = request(Method::GET, &page_uri, Body::empty());
    next.headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    let next = json(app.clone().oneshot(next).await.unwrap()).await;
    assert_eq!(next["nextCursor"], cursor);
    assert_eq!(
        next["entries"].as_array().unwrap().len(),
        DIRECTORY_PAGE_SIZE
    );

    let mut retry = request(Method::GET, &page_uri, Body::empty());
    retry
        .headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    let retry = json(app.clone().oneshot(retry).await.unwrap()).await;
    assert_eq!(
        retry, next,
        "a retry must replay the exact intermediate page"
    );

    let final_page = next["nextPage"].as_u64().unwrap();
    let final_uri = format!("/api/v1/downloads?path=&cursor={cursor}&page={final_page}");
    let mut final_request = request(Method::GET, &final_uri, Body::empty());
    final_request
        .headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    let final_page = json(app.clone().oneshot(final_request).await.unwrap()).await;
    assert!(final_page["nextCursor"].is_null());
    assert_eq!(final_page["entries"].as_array().unwrap().len(), 10);

    let mut final_retry = request(Method::GET, &final_uri, Body::empty());
    final_retry
        .headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    let final_retry = json(app.oneshot(final_retry).await.unwrap()).await;
    assert_eq!(
        final_retry, final_page,
        "a retry must replay the exact final page"
    );
}

#[tokio::test]
async fn streams_ranges_as_forced_attachments() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("probe.txt"), b"0123456789").unwrap();
    let state = test_state(temp.path());
    let (cookie, _) = session_headers(&state).await;
    let app = router(state);
    let mut download = request(
        Method::GET,
        "/api/v1/download?path=probe.txt",
        Body::empty(),
    );
    download
        .headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    download
        .headers_mut()
        .insert(header::RANGE, HeaderValue::from_static("bytes=2-5"));
    let response = app.clone().oneshot(download).await.unwrap();
    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    assert!(response
        .headers()
        .get(header::CONTENT_DISPOSITION)
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("attachment"));
    assert_eq!(
        response.headers().get(header::CONTENT_RANGE).unwrap(),
        "bytes 2-5/10"
    );
    assert_eq!(
        &to_bytes(response.into_body(), 1024).await.unwrap()[..],
        b"2345"
    );

    let mut head = request(
        Method::HEAD,
        "/api/v1/download?path=probe.txt",
        Body::empty(),
    );
    head.headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    head.headers_mut()
        .insert(header::RANGE, HeaderValue::from_static("bytes=7-"));
    let response = app.oneshot(head).await.unwrap();
    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(response.headers().get(header::CONTENT_LENGTH).unwrap(), "3");
}

#[tokio::test]
async fn dropping_unpolled_download_releases_active_registration() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("probe.bin"), vec![7_u8; 1024]).unwrap();
    let state = test_state(temp.path());
    let (cookie, _) = session_headers(&state).await;
    let app = router(state.clone());
    let mut download = request(
        Method::GET,
        "/api/v1/download?path=probe.bin",
        Body::empty(),
    );
    download
        .headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    let response = app.oneshot(download).await.unwrap();
    assert_eq!(state.active_transfers().await, 1);
    drop(response);
    for _ in 0..20 {
        if state.active_transfers().await == 0 {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert_eq!(state.active_transfers().await, 0);
}

#[tokio::test]
async fn upload_commit_remains_service_owned_after_the_waiter_is_dropped() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path());
    let id = Uuid::new_v4().to_string();
    let partial_path = state
        .partial_dir
        .as_ref()
        .unwrap()
        .join(format!("{id}.part"));
    let mut partial_file = create_upload_partial(&partial_path).unwrap();
    partial_file.write_all(b"x").unwrap();
    let transfer_id = state
        .record_transfer(TransferDirection::Upload, "late.txt", 1)
        .await;
    let record = Arc::new(Mutex::new(UploadRecord {
        id: id.clone(),
        owner_session: "session".into(),
        owner_address: IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50)),
        name: "late.txt".into(),
        declared_size: 1,
        offset: 1,
        last_modified: 1,
        client_token: "late-token".into(),
        created_at: Instant::now(),
        last_activity: Instant::now(),
        cancelled: false,
        finalizing: true,
        cancel_signal: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        chunk_slots: Arc::new(tokio::sync::Semaphore::new(1)),
        partial_path,
        partial_file: Arc::new(partial_file),
        transfer_id,
    }));
    state
        .uploads
        .lock()
        .await
        .insert(id.clone(), record.clone());
    let chunk_lease = {
        let record = record.lock().await;
        state.begin_upload_chunk(&record).unwrap()
    };
    let io_permit = state
        .begin_upload_io(IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50)))
        .await
        .unwrap();
    let task_state = state.clone();
    let task_id = id.clone();
    let commit = tokio::spawn(async move {
        finalize_upload_owned(task_state, task_id, record, chunk_lease, io_permit).await
    });
    drop(commit);

    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let published = fs::read_dir(temp.path()).unwrap().flatten().any(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                name.starts_with("late (") && name.ends_with(").txt")
            });
            if published && !state.uploads.lock().await.contains_key(&id) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("detaching the HTTP waiter must not detach the commit result");
}

#[tokio::test]
async fn uploads_in_order_without_overwriting() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("foto.jpg"), b"vorhanden").unwrap();
    let state = test_state(temp.path());
    let (cookie, csrf) = session_headers(&state).await;
    let app = router(state);

    let mut create = request(
        Method::POST,
        "/api/v1/uploads",
        Body::from(r#"{"name":"foto.jpg","size":4,"lastModified":1}"#),
    );
    create.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    create
        .headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    create
        .headers_mut()
        .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
    let created = json(app.clone().oneshot(create).await.unwrap()).await;
    let id = created["uploadId"].as_str().unwrap();

    let mut empty_chunk = request(
        Method::PATCH,
        &format!("/api/v1/uploads/{id}"),
        Body::empty(),
    );
    empty_chunk
        .headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    empty_chunk
        .headers_mut()
        .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
    empty_chunk
        .headers_mut()
        .insert("upload-offset", HeaderValue::from_static("0"));
    let response = app.clone().oneshot(empty_chunk).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json(response).await["code"], "EMPTY_CHUNK");

    let mut wrong_offset = request(
        Method::PATCH,
        &format!("/api/v1/uploads/{id}"),
        Body::from("test"),
    );
    wrong_offset
        .headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    wrong_offset
        .headers_mut()
        .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
    wrong_offset
        .headers_mut()
        .insert("upload-offset", HeaderValue::from_static("1"));
    assert_eq!(
        app.clone().oneshot(wrong_offset).await.unwrap().status(),
        StatusCode::CONFLICT
    );

    let mut chunk = request(
        Method::PATCH,
        &format!("/api/v1/uploads/{id}"),
        Body::from("test"),
    );
    chunk
        .headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    chunk
        .headers_mut()
        .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
    chunk
        .headers_mut()
        .insert("upload-offset", HeaderValue::from_static("0"));
    assert_eq!(
        app.clone().oneshot(chunk).await.unwrap().status(),
        StatusCode::OK
    );

    let mut complete = request(
        Method::POST,
        &format!("/api/v1/uploads/{id}/complete"),
        Body::empty(),
    );
    complete
        .headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    complete
        .headers_mut()
        .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
    let completed = app.oneshot(complete).await.unwrap();
    assert_eq!(completed.status(), StatusCode::OK);
    let completed = json(completed).await;
    let saved_name = completed["name"].as_str().unwrap();
    assert_eq!(
        fs::read(temp.path().join("foto.jpg")).unwrap(),
        b"vorhanden"
    );
    assert_ne!(saved_name, "foto.jpg");
    assert_ne!(saved_name, "foto (2).jpg");
    assert_eq!(fs::read(temp.path().join(saved_name)).unwrap(), b"test");
}

#[tokio::test]
async fn completes_zero_byte_upload() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path());
    let (cookie, csrf) = session_headers(&state).await;
    let app = router(state);
    let mut create = request(
        Method::POST,
        "/api/v1/uploads",
        Body::from(r#"{"name":"leer.txt","size":0,"lastModified":1}"#),
    );
    create.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    create
        .headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    create
        .headers_mut()
        .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
    let created = json(app.clone().oneshot(create).await.unwrap()).await;
    let id = created["uploadId"].as_str().unwrap();

    let mut complete = request(
        Method::POST,
        &format!("/api/v1/uploads/{id}/complete"),
        Body::empty(),
    );
    complete
        .headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    complete
        .headers_mut()
        .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
    let completed = app.oneshot(complete).await.unwrap();
    assert_eq!(completed.status(), StatusCode::OK);
    let completed = json(completed).await;
    let saved_name = completed["name"].as_str().unwrap();
    assert_ne!(saved_name, "leer.txt");
    assert_eq!(fs::metadata(temp.path().join(saved_name)).unwrap().len(), 0);
}

#[tokio::test]
async fn completion_receipt_and_client_token_make_retries_idempotent() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path());
    let (cookie, csrf) = session_headers(&state).await;
    let app = router(state.clone());
    let create_body = r#"{"name":"retry.bin","size":4,"lastModified":7,"clientToken":"0123456789abcdef0123456789abcdef"}"#;

    let mut create = request(Method::POST, "/api/v1/uploads", Body::from(create_body));
    create.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    create
        .headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    create
        .headers_mut()
        .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
    let created = json(app.clone().oneshot(create).await.unwrap()).await;
    let id = created["uploadId"].as_str().unwrap().to_string();

    let mut chunk = request(
        Method::PATCH,
        &format!("/api/v1/uploads/{id}"),
        Body::from("data"),
    );
    chunk
        .headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    chunk
        .headers_mut()
        .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
    chunk
        .headers_mut()
        .insert("upload-offset", HeaderValue::from_static("0"));
    assert_eq!(
        app.clone().oneshot(chunk).await.unwrap().status(),
        StatusCode::OK
    );

    let complete_request = || {
        let mut request = request(
            Method::POST,
            &format!("/api/v1/uploads/{id}/complete"),
            Body::empty(),
        );
        request
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        request
            .headers_mut()
            .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
        request
    };
    let first = json(app.clone().oneshot(complete_request()).await.unwrap()).await;
    let replay = json(app.clone().oneshot(complete_request()).await.unwrap()).await;
    assert_eq!(first["name"], replay["name"]);

    let mut recreate = request(Method::POST, "/api/v1/uploads", Body::from(create_body));
    recreate.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    recreate
        .headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    recreate
        .headers_mut()
        .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
    let recreated = json(app.clone().oneshot(recreate).await.unwrap()).await;
    assert_eq!(recreated["uploadId"], id);
    assert_eq!(recreated["offset"], 4);

    let changed_address = Ipv4Addr::new(192, 168, 10, 51);
    let (changed_cookie, changed_csrf) = session_headers_for(&state, changed_address).await;
    let mut changed_recreate = request_from(
        Method::POST,
        "/api/v1/uploads",
        Body::from(create_body),
        changed_address,
    );
    changed_recreate.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    changed_recreate.headers_mut().insert(
        header::COOKIE,
        HeaderValue::from_str(&changed_cookie).unwrap(),
    );
    changed_recreate
        .headers_mut()
        .insert("x-dmdc-csrf", HeaderValue::from_str(&changed_csrf).unwrap());
    let changed_recreated = json(app.oneshot(changed_recreate).await.unwrap()).await;
    assert_eq!(changed_recreated["uploadId"], id);
    assert_eq!(changed_recreated["offset"], 4);
    assert_eq!(
        fs::read_dir(temp.path())
            .unwrap()
            .flatten()
            .filter(|entry| entry.file_name() != ".dmdc")
            .count(),
        1
    );
}

#[tokio::test]
async fn rejects_tiny_nonfinal_upload_chunks() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path());
    let (cookie, csrf) = session_headers(&state).await;
    let app = router(state);
    let mut create = request(
        Method::POST,
        "/api/v1/uploads",
        Body::from(format!(
            r#"{{"name":"many-chunks.bin","size":{},"lastModified":1}}"#,
            CHUNK_SIZE + 1
        )),
    );
    create.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    create
        .headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    create
        .headers_mut()
        .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
    let created = json(app.clone().oneshot(create).await.unwrap()).await;
    let id = created["uploadId"].as_str().unwrap();

    let mut chunk = request(
        Method::PATCH,
        &format!("/api/v1/uploads/{id}"),
        Body::from("x"),
    );
    chunk
        .headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    chunk
        .headers_mut()
        .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
    chunk
        .headers_mut()
        .insert("upload-offset", HeaderValue::from_static("0"));
    let response = app.oneshot(chunk).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json(response).await["code"], "CHUNK_SIZE_INVALID");
}

#[tokio::test]
async fn limits_incomplete_uploads_per_client_address() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path());
    let (cookie, csrf) = session_headers(&state).await;
    let app = router(state);
    for index in 0..MAX_UPLOADS_PER_ADDRESS {
        let mut create = request(
            Method::POST,
            "/api/v1/uploads",
            Body::from(format!(
                r#"{{"name":"datei-{index}.txt","size":0,"lastModified":1}}"#
            )),
        );
        create.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        create
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        create
            .headers_mut()
            .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
        assert_eq!(
            app.clone().oneshot(create).await.unwrap().status(),
            StatusCode::OK
        );
    }

    let mut rejected = request(
        Method::POST,
        "/api/v1/uploads",
        Body::from(r#"{"name":"zu-viel.txt","size":0,"lastModified":1}"#),
    );
    rejected.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    rejected
        .headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    rejected
        .headers_mut()
        .insert("x-dmdc-csrf", HeaderValue::from_str(&csrf).unwrap());
    let response = app.oneshot(rejected).await.unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(json(response).await["code"], "UPLOAD_CLIENT_LIMIT");
}
