use crate::domain::{
    network::NetworkInterfaceInfo,
    settings::AppSettings,
    shares::{delete_open_upload, is_reparse_point, RootAnchor, ShareRoots},
    types::{ServiceStatus, SessionInfo, TransferInfo},
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use rand::{rngs::OsRng, Rng, RngCore};
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    net::IpAddr,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicI64, Ordering},
        Arc, Mutex as StdMutex, RwLock, Weak,
    },
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;
use tauri::{AppHandle, Emitter};
use tokio::sync::{watch, Mutex, OwnedSemaphorePermit, Semaphore};
use uuid::Uuid;

#[cfg(test)]
use crate::domain::shares::create_upload_partial;

pub const CHUNK_SIZE: usize = 8 * 1024 * 1024;
pub const DISK_RESERVE: u64 = 1024 * 1024 * 1024;
pub const MAX_DOWNLOADS_GLOBAL: usize = 12;
pub const MAX_DOWNLOADS_PER_SESSION: usize = 3;
pub const MAX_DOWNLOADS_PER_ADDRESS: usize = 4;
pub const MAX_UPLOAD_CHUNKS_ACTIVE: usize = 8;
pub const MAX_UPLOADS_PER_ADDRESS: usize = 4;
pub const MAX_SESSIONS_GLOBAL: usize = 128;
pub const MAX_SESSIONS_PER_ADDRESS: usize = 4;
pub const MAX_REQUESTS_GLOBAL: usize = 64;
pub const MAX_REQUESTS_PER_ADDRESS: usize = 8;
pub const MAX_ANONYMOUS_REQUESTS_GLOBAL: usize = 16;
pub const MAX_AUTHENTICATED_REQUESTS_GLOBAL: usize = 48;
pub const MAX_DIRECTORY_LISTINGS: usize = 64;
pub const MAX_DIRECTORY_LISTINGS_PER_SESSION: usize = 4;
pub const MAX_DIRECTORY_LISTINGS_PER_ADDRESS: usize = 8;
pub const MAX_DIRECTORY_LISTING_RECORDS: usize = MAX_DIRECTORY_LISTINGS + MAX_SESSIONS_GLOBAL;
pub const MAX_DIRECTORY_LISTINGS_ACTIVE: usize = 4;
pub const MAX_DIRECTORY_LISTINGS_ACTIVE_PER_ADDRESS: usize = 2;
pub const MAX_DIRECTORY_LISTINGS_ACTIVE_PER_SESSION: usize = 1;
pub const MAX_FILESYSTEM_LOOKUPS_ACTIVE: usize = 4;
pub const MAX_FILESYSTEM_LOOKUPS_PER_ADDRESS: usize = 2;
pub const MAX_AUTH_ATTEMPT_RECORDS: usize = 1_024;
pub const ACCESS_CODE_DIGITS: usize = 8;
pub const UPLOAD_IDLE_TIMEOUT: Duration = Duration::from_secs(30 * 60);
pub const UPLOAD_MAX_LIFETIME: Duration = Duration::from_secs(24 * 60 * 60);
pub const DOWNLOAD_MAX_DURATION: Duration = Duration::from_secs(6 * 60 * 60);
pub const SESSION_IDLE_TIMEOUT: Duration = Duration::from_secs(6 * 60 * 60 + 15 * 60);
pub const SESSION_MAX_LIFETIME: Duration = Duration::from_secs(24 * 60 * 60);

const AUTH_FAILURES_PER_ADDRESS: u8 = 10;
const AUTH_FAILURES_GLOBAL: u16 = 50;
const AUTH_WINDOW: Duration = Duration::from_secs(5 * 60);
const AUTH_BLOCK_DURATION: Duration = Duration::from_secs(5 * 60);
const AUTH_ATTEMPT_TTL: Duration = Duration::from_secs(10 * 60);
pub(crate) const DIRECTORY_CURSOR_TTL: Duration = Duration::from_secs(2 * 60);

const PARTIAL_DIR_NAME: &str = ".dmdc";
const PARTIAL_MARKER_NAME: &str = ".owner-v1";
const PARTIAL_MARKER: &[u8] = b"DMDC_UPLOAD_PARTIALS_V1\n";

#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub id: String,
    pub token: String,
    pub csrf: String,
    pub address: IpAddr,
    pub user_agent: String,
    pub created_at: String,
    pub last_activity: String,
    created_at_instant: Instant,
    last_activity_instant: Instant,
}

impl SessionRecord {
    fn expired_at(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.last_activity_instant) >= SESSION_IDLE_TIMEOUT
            || now.saturating_duration_since(self.created_at_instant) >= SESSION_MAX_LIFETIME
    }
}

#[derive(Debug)]
pub struct AttemptRecord {
    pub failures: u8,
    pub blocked_until: Option<Instant>,
    pub last_seen: Instant,
}

#[derive(Debug)]
pub struct AuthAttemptState {
    pub attempts: HashMap<IpAddr, AttemptRecord>,
    pub global_failures: u16,
    pub global_window_started: Instant,
    pub global_blocked_until: Option<Instant>,
}

impl AuthAttemptState {
    fn new(now: Instant) -> Self {
        Self {
            attempts: HashMap::new(),
            global_failures: 0,
            global_window_started: now,
            global_blocked_until: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthDecision {
    Accepted,
    Invalid,
    AddressBlocked,
    GlobalBlocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionCreateError {
    AddressLimit,
    GlobalLimit,
}

pub struct RequestPermit {
    _global: OwnedSemaphorePermit,
    _address: OwnedSemaphorePermit,
    _class: OwnedSemaphorePermit,
}

pub struct DirectoryListingPermit {
    _global: OwnedSemaphorePermit,
    _address: OwnedSemaphorePermit,
    _session: OwnedSemaphorePermit,
}

pub struct FilesystemLookupPermit {
    _global: OwnedSemaphorePermit,
    _address: OwnedSemaphorePermit,
}

pub struct UploadChunkLease {
    _global: OwnedSemaphorePermit,
    _upload: OwnedSemaphorePermit,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryEntry {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub size: u64,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DirectoryPage {
    pub sequence: u64,
    pub entries: Vec<DirectoryEntry>,
    pub exhausted: bool,
}

pub struct DirectoryListingCursor {
    pub iterator: fs::ReadDir,
    pub last_activity: Instant,
    pub exhausted: bool,
    pub next_page: u64,
    pub last_page: Option<DirectoryPage>,
}

pub struct DirectoryListing {
    pub owner_session: String,
    pub owner_address: IpAddr,
    pub path: String,
    pub filter: String,
    pub root: PathBuf,
    pub root_anchor: RootAnchor,
    pub cursor: StdMutex<DirectoryListingCursor>,
}

#[derive(Debug)]
pub struct UploadRecord {
    pub id: String,
    pub owner_session: String,
    pub owner_address: IpAddr,
    pub name: String,
    pub declared_size: u64,
    pub offset: u64,
    pub last_modified: u64,
    pub client_token: String,
    pub created_at: Instant,
    pub last_activity: Instant,
    pub cancelled: bool,
    pub finalizing: bool,
    pub cancel_signal: Arc<AtomicBool>,
    pub chunk_slots: Arc<Semaphore>,
    pub partial_path: PathBuf,
    pub partial_file: fs::File,
    pub transfer_id: String,
}

#[derive(Debug, Clone)]
pub struct CompletedUpload {
    pub upload_id: String,
    pub owner_address: IpAddr,
    pub name: String,
    pub requested_name: String,
    pub total_bytes: u64,
    pub last_modified: u64,
    pub client_token: String,
    completed_at: Instant,
}

impl CompletedUpload {
    pub fn new(
        upload_id: String,
        owner_address: IpAddr,
        name: String,
        requested_name: String,
        total_bytes: u64,
        last_modified: u64,
        client_token: String,
    ) -> Self {
        Self {
            upload_id,
            owner_address,
            name,
            requested_name,
            total_bytes,
            last_modified,
            client_token,
            completed_at: Instant::now(),
        }
    }
}

#[derive(Debug, Default)]
struct InboxUsage {
    completed_bytes: u64,
    completed_files: u64,
    active_bytes: u64,
    active_files: u64,
}

#[derive(Debug)]
struct ActiveDownload {
    owner_session: String,
    owner_address: IpAddr,
    cancel: watch::Sender<bool>,
}

pub struct DownloadLease {
    pub id: String,
    pub cancel: watch::Receiver<bool>,
    started_at: Instant,
    _permit: OwnedSemaphorePermit,
}

impl DownloadLease {
    pub fn expired(&self) -> bool {
        self.expired_at(Instant::now())
    }

    fn expired_at(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.started_at) >= DOWNLOAD_MAX_DURATION
    }

    pub fn remaining(&self) -> Duration {
        DOWNLOAD_MAX_DURATION.saturating_sub(self.started_at.elapsed())
    }
}

pub struct TransferServiceState {
    pub service_id: String,
    pub settings: AppSettings,
    pub interface: NetworkInterfaceInfo,
    pub roots: ShareRoots,
    pub partial_dir: Option<PathBuf>,
    pub download_root_anchor: Option<RootAnchor>,
    pub upload_root_anchor: Option<RootAnchor>,
    pub partial_dir_anchor: Option<RootAnchor>,
    pub access_code: RwLock<String>,
    pub stop_reason: RwLock<Option<String>>,
    pub sessions: Mutex<HashMap<String, SessionRecord>>,
    pub auth_attempts: StdMutex<AuthAttemptState>,
    pub uploads: Mutex<HashMap<String, Arc<Mutex<UploadRecord>>>>,
    pub completed_uploads: Mutex<HashMap<String, CompletedUpload>>,
    directory_listings: Mutex<HashMap<String, Arc<DirectoryListing>>>,
    downloads: Mutex<HashMap<String, ActiveDownload>>,
    download_slots: Arc<Semaphore>,
    filesystem_lookup_slots: Arc<Semaphore>,
    filesystem_lookup_address_slots: Mutex<HashMap<IpAddr, Weak<Semaphore>>>,
    listing_slots: Arc<Semaphore>,
    listing_address_slots: Mutex<HashMap<IpAddr, Weak<Semaphore>>>,
    listing_session_slots: Mutex<HashMap<String, Weak<Semaphore>>>,
    upload_chunk_slots: Arc<Semaphore>,
    request_slots: Arc<Semaphore>,
    anonymous_request_slots: Arc<Semaphore>,
    authenticated_request_slots: Arc<Semaphore>,
    request_address_slots: Mutex<HashMap<IpAddr, Weak<Semaphore>>>,
    inbox_usage: StdMutex<InboxUsage>,
    pub transfers: Mutex<Vec<TransferInfo>>,
    pub started_at: String,
    pub last_activity_unix: AtomicI64,
    pub app: Option<AppHandle>,
    pub upload_fs_lock: Arc<Mutex<()>>,
}

fn random_token(bytes: usize) -> String {
    let mut value = vec![0_u8; bytes];
    OsRng.fill_bytes(&mut value);
    URL_SAFE_NO_PAD.encode(value)
}

fn new_code() -> String {
    format!("{:08}", OsRng.gen_range(0..100_000_000_u32))
}

fn validate_owned_partial_dir(partial_dir: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(partial_dir)
        .map_err(|error| format!("Temporärer Uploadordner ist nicht erreichbar: {error}"))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
        return Err("Der reservierte .dmdc-Pfad ist kein sicherer DMDC-Ordner. Vorhandene Daten wurden nicht verändert.".into());
    }

    let marker = partial_dir.join(PARTIAL_MARKER_NAME);
    let marker_metadata = fs::symlink_metadata(&marker).map_err(|_| {
        "Im Upload-Eingang existiert bereits ein nicht von DMDC markierter .dmdc-Ordner. Vorhandene Daten wurden nicht verändert.".to_string()
    })?;
    if !marker_metadata.is_file()
        || marker_metadata.file_type().is_symlink()
        || is_reparse_point(&marker_metadata)
    {
        return Err("Die Besitzmarkierung des DMDC-Uploadordners ist ungültig. Vorhandene Daten wurden nicht verändert.".into());
    }
    let contents = fs::read(&marker).map_err(|error| {
        format!("Die Besitzmarkierung des DMDC-Uploadordners konnte nicht gelesen werden: {error}")
    })?;
    if contents != PARTIAL_MARKER {
        return Err("Die Besitzmarkierung des DMDC-Uploadordners stimmt nicht. Vorhandene Daten wurden nicht verändert.".into());
    }
    Ok(())
}

fn cleanup_owned_partials(partial_dir: &Path) -> Result<(), String> {
    // The marker identifies DMDC's directory format, but it is public and therefore
    // cannot prove ownership of an individual file. Live uploads are deleted through
    // their already-open handles; crash leftovers are deliberately preserved.
    validate_owned_partial_dir(partial_dir)
}

fn scan_inbox_usage(root: Option<&Path>) -> Result<(u64, u64), String> {
    let Some(root) = root else {
        return Ok((0, 0));
    };
    let mut bytes = 0_u64;
    let mut files = 0_u64;
    for entry in fs::read_dir(root)
        .map_err(|error| format!("Upload-Eingang konnte nicht gezählt werden: {error}"))?
    {
        let entry = entry.map_err(|error| {
            format!("Eintrag im Upload-Eingang konnte nicht geprüft werden: {error}")
        })?;
        if entry.file_name() == PARTIAL_DIR_NAME {
            continue;
        }
        files = files.saturating_add(1);
        let metadata = fs::symlink_metadata(entry.path()).map_err(|error| {
            format!("Eintrag im Upload-Eingang konnte nicht vermessen werden: {error}")
        })?;
        if metadata.is_file() {
            bytes = bytes.saturating_add(metadata.len());
        }
    }
    Ok((bytes, files))
}

fn prepare_partial_dir(root: &Path) -> Result<PathBuf, String> {
    let partial_dir = root.join(PARTIAL_DIR_NAME);
    let created = match fs::create_dir(&partial_dir) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => false,
        Err(error) => {
            return Err(format!(
                "Temporärer Uploadordner konnte nicht erstellt werden: {error}"
            ))
        }
    };

    if created {
        let marker = partial_dir.join(PARTIAL_MARKER_NAME);
        let result = (|| -> Result<(), String> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&marker)
                .map_err(|error| {
                    format!(
                        "Besitzmarkierung des Uploadordners konnte nicht erstellt werden: {error}"
                    )
                })?;
            file.write_all(PARTIAL_MARKER).map_err(|error| {
                format!(
                    "Besitzmarkierung des Uploadordners konnte nicht geschrieben werden: {error}"
                )
            })?;
            file.sync_all().map_err(|error| {
                format!(
                    "Besitzmarkierung des Uploadordners konnte nicht gespeichert werden: {error}"
                )
            })?;
            Ok(())
        })();
        if let Err(error) = result {
            let _ = fs::remove_file(marker);
            let _ = fs::remove_dir(&partial_dir);
            return Err(error);
        }
    }

    cleanup_owned_partials(&partial_dir)?;
    Ok(partial_dir)
}

impl TransferServiceState {
    pub fn new(
        settings: AppSettings,
        interface: NetworkInterfaceInfo,
        roots: ShareRoots,
        app: Option<AppHandle>,
    ) -> Result<Self, String> {
        let partial_dir = roots
            .upload
            .as_deref()
            .map(prepare_partial_dir)
            .transpose()?;
        let download_root_anchor = roots
            .download
            .as_deref()
            .map(RootAnchor::open)
            .transpose()?;
        let upload_root_anchor = roots.upload.as_deref().map(RootAnchor::open).transpose()?;
        let partial_dir_anchor = partial_dir.as_deref().map(RootAnchor::open).transpose()?;
        let (completed_bytes, completed_files) = scan_inbox_usage(roots.upload.as_deref())?;
        Ok(Self {
            service_id: Uuid::new_v4().to_string(),
            settings,
            interface,
            roots,
            partial_dir,
            download_root_anchor,
            upload_root_anchor,
            partial_dir_anchor,
            access_code: RwLock::new(new_code()),
            stop_reason: RwLock::new(None),
            sessions: Mutex::new(HashMap::new()),
            auth_attempts: StdMutex::new(AuthAttemptState::new(Instant::now())),
            uploads: Mutex::new(HashMap::new()),
            completed_uploads: Mutex::new(HashMap::new()),
            directory_listings: Mutex::new(HashMap::new()),
            downloads: Mutex::new(HashMap::new()),
            download_slots: Arc::new(Semaphore::new(MAX_DOWNLOADS_GLOBAL)),
            filesystem_lookup_slots: Arc::new(Semaphore::new(MAX_FILESYSTEM_LOOKUPS_ACTIVE)),
            filesystem_lookup_address_slots: Mutex::new(HashMap::new()),
            listing_slots: Arc::new(Semaphore::new(MAX_DIRECTORY_LISTINGS_ACTIVE)),
            listing_address_slots: Mutex::new(HashMap::new()),
            listing_session_slots: Mutex::new(HashMap::new()),
            upload_chunk_slots: Arc::new(Semaphore::new(MAX_UPLOAD_CHUNKS_ACTIVE)),
            request_slots: Arc::new(Semaphore::new(MAX_REQUESTS_GLOBAL)),
            anonymous_request_slots: Arc::new(Semaphore::new(MAX_ANONYMOUS_REQUESTS_GLOBAL)),
            authenticated_request_slots: Arc::new(Semaphore::new(
                MAX_AUTHENTICATED_REQUESTS_GLOBAL,
            )),
            request_address_slots: Mutex::new(HashMap::new()),
            inbox_usage: StdMutex::new(InboxUsage {
                completed_bytes,
                completed_files,
                ..InboxUsage::default()
            }),
            transfers: Mutex::new(vec![]),
            started_at: Utc::now().to_rfc3339(),
            last_activity_unix: AtomicI64::new(Utc::now().timestamp()),
            app,
            upload_fs_lock: Arc::new(Mutex::new(())),
        })
    }

    pub fn url(&self) -> String {
        format!("http://{}:{}/", self.interface.address, self.settings.port)
    }
    pub fn expected_host(&self) -> String {
        format!("{}:{}", self.interface.address, self.settings.port)
    }
    pub fn touch(&self) {
        self.last_activity_unix
            .store(Utc::now().timestamp(), Ordering::Relaxed);
    }

    pub fn roots_are_current(&self) -> bool {
        self.download_root_anchor
            .as_ref()
            .map(RootAnchor::is_current)
            .unwrap_or(true)
            && self
                .upload_root_anchor
                .as_ref()
                .map(RootAnchor::is_current)
                .unwrap_or(true)
            && self
                .partial_dir_anchor
                .as_ref()
                .map(RootAnchor::is_current)
                .unwrap_or(true)
    }

    pub fn refresh_inbox_usage(&self) -> Result<(), String> {
        let (completed_bytes, completed_files) = scan_inbox_usage(self.roots.upload.as_deref())?;
        let mut usage = self.inbox_usage.lock().expect("inbox usage lock poisoned");
        usage.completed_bytes = completed_bytes;
        usage.completed_files = completed_files;
        Ok(())
    }

    pub fn reserve_upload_object(&self) -> Result<(), &'static str> {
        let mut usage = self.inbox_usage.lock().expect("inbox usage lock poisoned");
        if usage
            .completed_files
            .saturating_add(usage.active_files)
            .saturating_add(1)
            > u64::from(self.settings.max_inbox_files)
        {
            return Err("files");
        }
        usage.active_files = usage.active_files.saturating_add(1);
        Ok(())
    }

    pub fn reserve_upload_bytes(&self, bytes: u64) -> Result<(), &'static str> {
        let mut usage = self.inbox_usage.lock().expect("inbox usage lock poisoned");
        if usage
            .completed_bytes
            .saturating_add(usage.active_bytes)
            .saturating_add(bytes)
            > self.settings.max_inbox_bytes
        {
            return Err("bytes");
        }
        usage.active_bytes = usage.active_bytes.saturating_add(bytes);
        Ok(())
    }

    pub fn release_upload_bytes(&self, bytes: u64) {
        let mut usage = self.inbox_usage.lock().expect("inbox usage lock poisoned");
        usage.active_bytes = usage.active_bytes.saturating_sub(bytes);
    }

    pub fn release_upload(&self, bytes: u64) {
        let mut usage = self.inbox_usage.lock().expect("inbox usage lock poisoned");
        usage.active_files = usage.active_files.saturating_sub(1);
        usage.active_bytes = usage.active_bytes.saturating_sub(bytes);
    }

    pub fn complete_upload_budget(&self, bytes: u64) {
        let mut usage = self.inbox_usage.lock().expect("inbox usage lock poisoned");
        usage.active_files = usage.active_files.saturating_sub(1);
        usage.active_bytes = usage.active_bytes.saturating_sub(bytes);
        usage.completed_files = usage.completed_files.saturating_add(1);
        usage.completed_bytes = usage.completed_bytes.saturating_add(bytes);
    }

    pub async fn remember_completed_upload(&self, completed: CompletedUpload) {
        const RECEIPT_TTL: Duration = Duration::from_secs(24 * 60 * 60);
        const MAX_RECEIPTS: usize = 256;
        let now = Instant::now();
        let mut receipts = self.completed_uploads.lock().await;
        receipts.retain(|_, item| now.saturating_duration_since(item.completed_at) < RECEIPT_TTL);
        if receipts.len() >= MAX_RECEIPTS {
            if let Some(oldest) = receipts
                .iter()
                .min_by_key(|(_, item)| item.completed_at)
                .map(|(id, _)| id.clone())
            {
                receipts.remove(&oldest);
            }
        }
        receipts.insert(completed.upload_id.clone(), completed);
    }

    pub async fn completed_upload(
        &self,
        id: &str,
        owner_address: IpAddr,
    ) -> Option<CompletedUpload> {
        const RECEIPT_TTL: Duration = Duration::from_secs(24 * 60 * 60);
        let now = Instant::now();
        let mut receipts = self.completed_uploads.lock().await;
        receipts.retain(|_, item| now.saturating_duration_since(item.completed_at) < RECEIPT_TTL);
        receipts
            .get(id)
            .filter(|item| item.owner_address == owner_address)
            .cloned()
    }

    pub async fn completed_upload_by_token(
        &self,
        token: &str,
        name: &str,
        total_bytes: u64,
        last_modified: u64,
    ) -> Option<CompletedUpload> {
        const RECEIPT_TTL: Duration = Duration::from_secs(24 * 60 * 60);
        let now = Instant::now();
        let mut receipts = self.completed_uploads.lock().await;
        receipts.retain(|_, item| now.saturating_duration_since(item.completed_at) < RECEIPT_TTL);
        receipts
            .values()
            .find(|item| {
                item.client_token == token
                    && item.requested_name == name
                    && item.total_bytes == total_bytes
                    && item.last_modified == last_modified
            })
            .cloned()
    }

    pub async fn should_auto_stop(&self) -> bool {
        let Some(minutes) = self.settings.idle_timeout_minutes else {
            return false;
        };
        if self.active_transfers().await > 0 {
            return false;
        }
        Utc::now().timestamp() - self.last_activity_unix.load(Ordering::Relaxed)
            >= i64::from(minutes) * 60
    }

    pub async fn active_transfers(&self) -> usize {
        self.transfers
            .lock()
            .await
            .iter()
            .filter(|item| item.state == "active")
            .count()
    }

    pub fn rotate_code(&self) -> String {
        let mut throttle = self
            .auth_attempts
            .lock()
            .expect("auth attempt lock poisoned");
        *throttle = AuthAttemptState::new(Instant::now());
        let value = new_code();
        *self.access_code.write().expect("access code lock poisoned") = value.clone();
        value
    }

    pub fn verify_access_code(&self, address: IpAddr, supplied: &str) -> AuthDecision {
        self.verify_access_code_at(address, supplied, Instant::now())
    }

    fn verify_access_code_at(&self, address: IpAddr, supplied: &str, now: Instant) -> AuthDecision {
        let mut throttle = self
            .auth_attempts
            .lock()
            .expect("auth attempt lock poisoned");
        throttle.attempts.retain(|_, record| {
            record.blocked_until.is_some_and(|until| until > now)
                || now.saturating_duration_since(record.last_seen) < AUTH_ATTEMPT_TTL
        });
        if now.saturating_duration_since(throttle.global_window_started) >= AUTH_WINDOW {
            throttle.global_failures = 0;
            throttle.global_window_started = now;
            throttle.global_blocked_until = None;
        }
        if throttle
            .global_blocked_until
            .is_some_and(|until| until > now)
        {
            return AuthDecision::GlobalBlocked;
        }
        if throttle.global_blocked_until.is_some() {
            throttle.global_blocked_until = None;
            throttle.global_failures = 0;
            throttle.global_window_started = now;
        }
        if throttle
            .attempts
            .get(&address)
            .and_then(|record| record.blocked_until)
            .is_some_and(|until| until > now)
        {
            return AuthDecision::AddressBlocked;
        }

        let expected = self
            .access_code
            .read()
            .expect("access code lock poisoned")
            .clone();
        let valid_shape = supplied.len() == ACCESS_CODE_DIGITS
            && supplied.bytes().all(|value| value.is_ascii_digit());
        let correct =
            valid_shape && supplied.as_bytes().ct_eq(expected.as_bytes()).unwrap_u8() == 1;
        if correct {
            throttle.attempts.remove(&address);
            return AuthDecision::Accepted;
        }

        throttle.global_failures = throttle.global_failures.saturating_add(1);
        if throttle.global_failures >= AUTH_FAILURES_GLOBAL {
            throttle.global_blocked_until = Some(now + AUTH_BLOCK_DURATION);
            throttle.global_window_started = now;
            return AuthDecision::GlobalBlocked;
        }

        if let Some(record) = throttle.attempts.get_mut(&address) {
            if record.blocked_until.is_some() {
                record.failures = 0;
                record.blocked_until = None;
            }
            record.failures = record.failures.saturating_add(1);
            record.last_seen = now;
            if record.failures >= AUTH_FAILURES_PER_ADDRESS {
                record.failures = 0;
                record.blocked_until = Some(now + AUTH_BLOCK_DURATION);
                return AuthDecision::AddressBlocked;
            }
        } else if throttle.attempts.len() < MAX_AUTH_ATTEMPT_RECORDS {
            throttle.attempts.insert(
                address,
                AttemptRecord {
                    failures: 1,
                    blocked_until: None,
                    last_seen: now,
                },
            );
        }
        AuthDecision::Invalid
    }

    pub async fn begin_request(
        &self,
        address: IpAddr,
        authenticated: bool,
    ) -> Option<RequestPermit> {
        let global = self.request_slots.clone().try_acquire_owned().ok()?;
        let class = if authenticated {
            self.authenticated_request_slots
                .clone()
                .try_acquire_owned()
                .ok()?
        } else {
            self.anonymous_request_slots
                .clone()
                .try_acquire_owned()
                .ok()?
        };
        let address_slots = {
            let mut slots = self.request_address_slots.lock().await;
            slots.retain(|_, slots| slots.strong_count() > 0);
            if let Some(existing) = slots.get(&address).and_then(Weak::upgrade) {
                existing
            } else {
                let created = Arc::new(Semaphore::new(MAX_REQUESTS_PER_ADDRESS));
                slots.insert(address, Arc::downgrade(&created));
                created
            }
        };
        let address = address_slots.try_acquire_owned().ok()?;
        Some(RequestPermit {
            _global: global,
            _address: address,
            _class: class,
        })
    }

    pub async fn begin_filesystem_lookup(&self, address: IpAddr) -> Option<FilesystemLookupPermit> {
        let global = self
            .filesystem_lookup_slots
            .clone()
            .try_acquire_owned()
            .ok()?;
        let address_slots = {
            let mut slots = self.filesystem_lookup_address_slots.lock().await;
            slots.retain(|_, slots| slots.strong_count() > 0);
            if let Some(existing) = slots.get(&address).and_then(Weak::upgrade) {
                existing
            } else {
                let created = Arc::new(Semaphore::new(MAX_FILESYSTEM_LOOKUPS_PER_ADDRESS));
                slots.insert(address, Arc::downgrade(&created));
                created
            }
        };
        let address = address_slots.try_acquire_owned().ok()?;
        Some(FilesystemLookupPermit {
            _global: global,
            _address: address,
        })
    }

    pub async fn begin_listing(
        &self,
        owner_session: &str,
        owner_address: IpAddr,
    ) -> Option<DirectoryListingPermit> {
        let global = self.listing_slots.clone().try_acquire_owned().ok()?;
        let address_slots = {
            let mut slots = self.listing_address_slots.lock().await;
            slots.retain(|_, slots| slots.strong_count() > 0);
            if let Some(existing) = slots.get(&owner_address).and_then(Weak::upgrade) {
                existing
            } else {
                let created = Arc::new(Semaphore::new(MAX_DIRECTORY_LISTINGS_ACTIVE_PER_ADDRESS));
                slots.insert(owner_address, Arc::downgrade(&created));
                created
            }
        };
        let address = address_slots.try_acquire_owned().ok()?;
        let session_slots = {
            let mut slots = self.listing_session_slots.lock().await;
            slots.retain(|_, slots| slots.strong_count() > 0);
            if let Some(existing) = slots.get(owner_session).and_then(Weak::upgrade) {
                existing
            } else {
                let created = Arc::new(Semaphore::new(MAX_DIRECTORY_LISTINGS_ACTIVE_PER_SESSION));
                slots.insert(owner_session.to_string(), Arc::downgrade(&created));
                created
            }
        };
        let session = session_slots.try_acquire_owned().ok()?;
        Some(DirectoryListingPermit {
            _global: global,
            _address: address,
            _session: session,
        })
    }

    pub fn begin_upload_chunk(
        &self,
        upload: &UploadRecord,
    ) -> Result<UploadChunkLease, &'static str> {
        let global = self
            .upload_chunk_slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| "global")?;
        let upload = upload
            .chunk_slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| "upload")?;
        Ok(UploadChunkLease {
            _global: global,
            _upload: upload,
        })
    }

    pub async fn create_directory_listing(
        &self,
        session: &SessionRecord,
        path: String,
        filter: String,
        root: PathBuf,
        iterator: fs::ReadDir,
    ) -> Result<(String, Arc<DirectoryListing>), &'static str> {
        let now = Instant::now();
        let sessions = self.sessions.lock().await;
        if !sessions.get(&session.token).is_some_and(|current| {
            current.id == session.id
                && current.address == session.address
                && !current.expired_at(now)
        }) {
            return Err("invalid");
        }
        let mut listings = self.directory_listings.lock().await;
        listings.retain(|_, listing| {
            listing.cursor.try_lock().map_or(true, |cursor| {
                now.saturating_duration_since(cursor.last_activity) < DIRECTORY_CURSOR_TTL
            })
        });
        listings.retain(|_, listing| {
            listing.owner_session != session.id
                || listing
                    .cursor
                    .try_lock()
                    .map_or(true, |cursor| !cursor.exhausted)
        });
        if listings.len() >= MAX_DIRECTORY_LISTING_RECORDS {
            let oldest_exhausted = listings
                .iter()
                .filter_map(|(id, listing)| {
                    listing.cursor.try_lock().ok().and_then(|cursor| {
                        cursor
                            .exhausted
                            .then_some((id.clone(), cursor.last_activity))
                    })
                })
                .min_by_key(|(_, last_activity)| *last_activity)
                .map(|(id, _)| id);
            if let Some(id) = oldest_exhausted {
                listings.remove(&id);
            }
        }
        let active = |listing: &&Arc<DirectoryListing>| {
            listing
                .cursor
                .try_lock()
                .map_or(true, |cursor| !cursor.exhausted)
        };
        if listings.values().filter(active).count() >= MAX_DIRECTORY_LISTINGS {
            return Err("capacity");
        }
        if listings
            .values()
            .filter(|listing| listing.owner_session == session.id)
            .filter(active)
            .count()
            >= MAX_DIRECTORY_LISTINGS_PER_SESSION
        {
            return Err("session");
        }
        if listings
            .values()
            .filter(|listing| listing.owner_address == session.address)
            .filter(active)
            .count()
            >= MAX_DIRECTORY_LISTINGS_PER_ADDRESS
        {
            return Err("address");
        }
        let root_anchor = self.download_root_anchor.clone().ok_or("invalid")?;
        let id = random_token(24);
        let listing = Arc::new(DirectoryListing {
            owner_session: session.id.clone(),
            owner_address: session.address,
            path,
            filter,
            root,
            root_anchor,
            cursor: StdMutex::new(DirectoryListingCursor {
                iterator,
                last_activity: now,
                exhausted: false,
                next_page: 0,
                last_page: None,
            }),
        });
        listings.insert(id.clone(), listing.clone());
        Ok((id, listing))
    }

    pub async fn directory_listing(
        &self,
        id: &str,
        owner_session: &str,
        path: &str,
        filter: &str,
    ) -> Option<Arc<DirectoryListing>> {
        let listing = self.directory_listings.lock().await.get(id).cloned()?;
        if listing.owner_session != owner_session
            || listing.path != path
            || listing.filter != filter
        {
            return None;
        }
        let fresh = match listing.cursor.try_lock() {
            Ok(cursor) => cursor.last_activity.elapsed() < DIRECTORY_CURSOR_TTL,
            Err(_) => true,
        };
        if fresh {
            Some(listing)
        } else {
            self.directory_listings.lock().await.remove(id);
            None
        }
    }

    pub async fn remove_directory_listing(&self, id: &str) {
        self.directory_listings.lock().await.remove(id);
    }

    async fn remove_directory_listings(&self, owner_session: Option<&str>) {
        self.directory_listings
            .lock()
            .await
            .retain(|_, listing| owner_session.is_some_and(|owner| listing.owner_session != owner));
    }

    pub async fn expire_stale_directory_listings(&self) {
        let now = Instant::now();
        self.directory_listings.lock().await.retain(|_, listing| {
            listing.cursor.try_lock().map_or(true, |cursor| {
                now.saturating_duration_since(cursor.last_activity) < DIRECTORY_CURSOR_TTL
            })
        });
    }

    pub fn set_stop_reason(&self, reason: impl Into<String>) {
        *self.stop_reason.write().expect("stop reason lock poisoned") = Some(reason.into());
    }

    pub fn stop_reason(&self) -> Option<String> {
        self.stop_reason
            .read()
            .expect("stop reason lock poisoned")
            .clone()
    }

    pub async fn create_session(
        &self,
        address: IpAddr,
        user_agent: String,
    ) -> Result<SessionRecord, SessionCreateError> {
        let now_instant = Instant::now();
        let now = Utc::now().to_rfc3339();
        let record = SessionRecord {
            id: Uuid::new_v4().to_string(),
            token: random_token(32),
            csrf: random_token(24),
            address,
            user_agent,
            created_at: now.clone(),
            last_activity: now,
            created_at_instant: now_instant,
            last_activity_instant: now_instant,
        };
        let (expired, result, count) = {
            let mut sessions = self.sessions.lock().await;
            let expired: Vec<_> = sessions
                .values()
                .filter(|session| session.expired_at(now_instant))
                .map(|session| session.id.clone())
                .collect();
            sessions.retain(|_, session| !session.expired_at(now_instant));
            let result = if sessions.len() >= MAX_SESSIONS_GLOBAL {
                Err(SessionCreateError::GlobalLimit)
            } else if sessions
                .values()
                .filter(|session| session.address == address)
                .count()
                >= MAX_SESSIONS_PER_ADDRESS
            {
                Err(SessionCreateError::AddressLimit)
            } else {
                sessions.insert(record.token.clone(), record.clone());
                Ok(record.clone())
            };
            (expired, result, sessions.len())
        };
        self.cleanup_expired_sessions(&expired).await;
        if !expired.is_empty() || result.is_ok() {
            self.emit("sessions-changed", &serde_json::json!({ "count": count }));
        }
        result
    }

    pub async fn authenticate(&self, token: &str, address: IpAddr) -> Option<SessionRecord> {
        let now = Instant::now();
        let (session, expired, count) = {
            let mut sessions = self.sessions.lock().await;
            let current = sessions.get(token)?;
            if current.expired_at(now) {
                let id = current.id.clone();
                sessions.remove(token);
                (None, Some(id), sessions.len())
            } else if current.address != address {
                return None;
            } else {
                let current = sessions.get_mut(token).expect("session remains present");
                current.last_activity = Utc::now().to_rfc3339();
                current.last_activity_instant = now;
                (Some(current.clone()), None, sessions.len())
            }
        };
        if let Some(expired) = expired {
            self.cleanup_expired_sessions(std::slice::from_ref(&expired))
                .await;
            self.emit("sessions-changed", &serde_json::json!({ "count": count }));
            return None;
        }
        self.touch();
        session
    }

    pub async fn session_token_is_active(&self, token: &str, address: IpAddr) -> bool {
        let now = Instant::now();
        self.sessions
            .lock()
            .await
            .get(token)
            .is_some_and(|session| session.address == address && !session.expired_at(now))
    }

    pub async fn session_is_active(&self, expected: &SessionRecord) -> bool {
        let now = Instant::now();
        let (active, expired, count) = {
            let mut sessions = self.sessions.lock().await;
            let expired = sessions
                .get(&expected.token)
                .and_then(|current| current.expired_at(now).then(|| current.id.clone()));
            if expired.is_some() {
                sessions.remove(&expected.token);
            }
            let active = sessions.get(&expected.token).is_some_and(|current| {
                current.id == expected.id && current.address == expected.address
            });
            (active, expired, sessions.len())
        };
        if let Some(expired) = expired {
            self.cleanup_expired_sessions(std::slice::from_ref(&expired))
                .await;
            self.emit("sessions-changed", &serde_json::json!({ "count": count }));
        }
        active
    }

    async fn cleanup_expired_sessions(&self, session_ids: &[String]) {
        for id in session_ids {
            self.cancel_downloads(Some(id)).await;
            self.cancel_uploads(Some(id)).await;
            self.remove_directory_listings(Some(id)).await;
        }
    }

    pub async fn expire_stale_sessions(&self) {
        let now = Instant::now();
        let (expired, count) = {
            let mut sessions = self.sessions.lock().await;
            let expired: Vec<_> = sessions
                .values()
                .filter(|session| session.expired_at(now))
                .map(|session| session.id.clone())
                .collect();
            sessions.retain(|_, session| !session.expired_at(now));
            (expired, sessions.len())
        };
        if !expired.is_empty() {
            self.cleanup_expired_sessions(&expired).await;
            self.emit("sessions-changed", &serde_json::json!({ "count": count }));
        }
    }

    pub async fn revoke_session(&self, id: &str) -> bool {
        let mut sessions = self.sessions.lock().await;
        let before = sessions.len();
        sessions.retain(|_, item| item.id != id);
        let changed = before != sessions.len();
        let count = sessions.len();
        drop(sessions);
        if changed {
            self.cancel_downloads(Some(id)).await;
            self.cancel_uploads(Some(id)).await;
            self.remove_directory_listings(Some(id)).await;
            self.emit("sessions-changed", &serde_json::json!({ "count": count }));
        }
        changed
    }

    pub async fn revoke_all(&self) {
        let revoked: Vec<_> = {
            let mut sessions = self.sessions.lock().await;
            let revoked = sessions
                .values()
                .map(|session| session.id.clone())
                .collect();
            sessions.clear();
            revoked
        };
        self.cleanup_expired_sessions(&revoked).await;
        self.emit("sessions-changed", &serde_json::json!({ "count": 0 }));
    }

    async fn cancel_uploads(&self, owner: Option<&str>) {
        let _filesystem = self.upload_fs_lock.lock().await;
        let records: Vec<_> = self
            .uploads
            .lock()
            .await
            .iter()
            .map(|(id, record)| (id.clone(), record.clone()))
            .collect();
        for (id, record) in records {
            let mut record = record.lock().await;
            if owner.is_some_and(|owner| owner != record.owner_session) {
                continue;
            }
            record.cancelled = true;
            record.cancel_signal.store(true, Ordering::Release);
            self.uploads.lock().await.remove(&id);
            let path = record.partial_path.clone();
            let transfer_id = record.transfer_id.clone();
            let offset = record.offset;
            let _ = delete_open_upload(&record.partial_file, &path);
            drop(record);
            self.release_upload(offset);
            self.update_transfer(&transfer_id, offset, Some("cancelled"))
                .await;
        }
    }

    async fn cancel_downloads(&self, owner: Option<&str>) {
        let downloads = self.downloads.lock().await;
        for download in downloads.values() {
            if owner.map_or(true, |owner| owner == download.owner_session) {
                let _ = download.cancel.send(true);
            }
        }
    }

    pub async fn begin_download(
        &self,
        owner_session: &str,
        owner_address: IpAddr,
        name: &str,
        total: u64,
    ) -> Result<DownloadLease, &'static str> {
        let permit = self
            .download_slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| "global")?;
        let mut downloads = self.downloads.lock().await;
        if downloads
            .values()
            .filter(|item| item.owner_session == owner_session)
            .count()
            >= MAX_DOWNLOADS_PER_SESSION
        {
            return Err("session");
        }
        if downloads
            .values()
            .filter(|item| item.owner_address == owner_address)
            .count()
            >= MAX_DOWNLOADS_PER_ADDRESS
        {
            return Err("address");
        }
        let id = Uuid::new_v4().to_string();
        let (cancel, receiver) = watch::channel(false);
        downloads.insert(
            id.clone(),
            ActiveDownload {
                owner_session: owner_session.into(),
                owner_address,
                cancel,
            },
        );
        self.record_transfer_with_id(&id, "download", name, total)
            .await;
        Ok(DownloadLease {
            id,
            cancel: receiver,
            started_at: Instant::now(),
            _permit: permit,
        })
    }

    pub async fn finish_download(&self, id: &str, bytes: u64, state: &str) {
        self.downloads.lock().await.remove(id);
        self.update_transfer(id, bytes, Some(state)).await;
    }

    pub async fn expire_stale_uploads(&self) {
        let _filesystem = self.upload_fs_lock.lock().await;
        let records: Vec<_> = self
            .uploads
            .lock()
            .await
            .iter()
            .map(|(id, record)| (id.clone(), record.clone()))
            .collect();
        for (id, record) in records {
            let mut record = record.lock().await;
            if record.cancelled
                || (record.last_activity.elapsed() < UPLOAD_IDLE_TIMEOUT
                    && record.created_at.elapsed() < UPLOAD_MAX_LIFETIME)
            {
                continue;
            }
            record.cancelled = true;
            record.cancel_signal.store(true, Ordering::Release);
            self.uploads.lock().await.remove(&id);
            let path = record.partial_path.clone();
            let transfer_id = record.transfer_id.clone();
            let offset = record.offset;
            let _ = delete_open_upload(&record.partial_file, &path);
            drop(record);
            self.release_upload(offset);
            self.update_transfer(&transfer_id, offset, Some("expired"))
                .await;
        }
    }

    #[cfg(test)]
    pub async fn record_transfer(&self, direction: &str, name: &str, total: u64) -> String {
        let id = Uuid::new_v4().to_string();
        self.record_transfer_with_id(&id, direction, name, total)
            .await;
        id
    }

    pub async fn record_transfer_with_id(&self, id: &str, direction: &str, name: &str, total: u64) {
        let item = TransferInfo {
            id: id.into(),
            direction: direction.into(),
            name: name.into(),
            transferred_bytes: 0,
            total_bytes: total,
            state: "active".into(),
            updated_at: Utc::now().to_rfc3339(),
        };
        let mut transfers = self.transfers.lock().await;
        transfers.push(item);
        while transfers.len() > 100 {
            let Some(index) = transfers.iter().position(|item| item.state != "active") else {
                break;
            };
            transfers.remove(index);
        }
        drop(transfers);
        self.emit("transfer-updated", &serde_json::json!({ "id": id }));
    }

    pub async fn update_transfer(&self, id: &str, bytes: u64, state: Option<&str>) {
        if let Some(item) = self
            .transfers
            .lock()
            .await
            .iter_mut()
            .find(|item| item.id == id)
        {
            item.transferred_bytes = bytes;
            if let Some(state) = state {
                item.state = state.into();
            }
            item.updated_at = Utc::now().to_rfc3339();
        }
        self.touch();
        self.emit("transfer-updated", &serde_json::json!({ "id": id }));
    }

    fn emit<T: serde::Serialize + Clone>(&self, event: &str, payload: &T) {
        if let Some(app) = &self.app {
            let _ = app.emit(event, payload.clone());
        }
    }

    pub fn emit_network_lost(&self) {
        if let Some(app) = &self.app {
            if let Some(tray) = app.tray_by_id("main-tray") {
                let _ = tray.set_tooltip(Some("DMDC – Netzwerk verloren, Dienst gestoppt"));
            }
        }
        self.emit(
            "network-changed",
            &serde_json::json!({ "available": false }),
        );
    }

    pub fn emit_auto_stop(&self) {
        if let Some(app) = &self.app {
            if let Some(tray) = app.tray_by_id("main-tray") {
                let _ = tray.set_tooltip(Some("DMDC – wegen Inaktivität gestoppt"));
            }
        }
        self.emit(
            "service-status-changed",
            &serde_json::json!({ "state": "stopped", "reason": "idle" }),
        );
    }

    pub fn status(&self) -> ServiceStatus {
        let now = Instant::now();
        let sessions = self
            .sessions
            .try_lock()
            .map(|items| {
                items
                    .values()
                    .filter(|session| !session.expired_at(now))
                    .map(|session| SessionInfo {
                        id: session.id.clone(),
                        address: session.address.to_string(),
                        user_agent: session.user_agent.clone(),
                        created_at: session.created_at.clone(),
                        last_activity: session.last_activity.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let transfers = self
            .transfers
            .try_lock()
            .map(|items| items.clone())
            .unwrap_or_default();
        let active_transfers = transfers
            .iter()
            .filter(|item| item.state == "active")
            .count();
        ServiceStatus {
            state: "running".into(),
            service_id: Some(self.service_id.clone()),
            url: Some(self.url()),
            access_code: Some(
                self.access_code
                    .read()
                    .expect("access code lock poisoned")
                    .clone(),
            ),
            started_at: Some(self.started_at.clone()),
            active_transfers,
            sessions,
            transfers,
            error: None,
        }
    }

    pub async fn cleanup_partials(&self) {
        self.cancel_downloads(None).await;
        self.cancel_uploads(None).await;
        self.remove_directory_listings(None).await;
        let drained: Vec<_> = self
            .downloads
            .lock()
            .await
            .drain()
            .map(|(id, _)| id)
            .collect();
        for id in drained {
            self.update_transfer(&id, 0, Some("cancelled")).await;
        }
        if let Some(partial_dir) = self.partial_dir.clone() {
            let _ = tokio::task::spawn_blocking(move || cleanup_owned_partials(&partial_dir)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::settings::ShareSettings;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use std::sync::Condvar;

    fn test_state(root: &Path) -> Result<TransferServiceState, String> {
        let settings = AppSettings {
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
        TransferServiceState::new(
            settings,
            interface,
            ShareRoots {
                download: Some(root.to_path_buf()),
                upload: Some(root.to_path_buf()),
            },
            None,
        )
    }

    #[test]
    fn preserves_unmanaged_dmdc_directory() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join(PARTIAL_DIR_NAME)).unwrap();
        let user_file = temp.path().join(PARTIAL_DIR_NAME).join("user.txt");
        fs::write(&user_file, b"behalten").unwrap();
        assert!(test_state(temp.path()).is_err());
        assert_eq!(fs::read(user_file).unwrap(), b"behalten");
    }

    #[test]
    fn preserves_untracked_uuid_partials_even_with_public_marker() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path()).unwrap();
        let partial_dir = state.partial_dir.as_ref().unwrap();
        let owned = partial_dir.join(format!("{}.part", Uuid::new_v4()));
        let unrelated = partial_dir.join("notes.part");
        fs::write(&owned, b"teil").unwrap();
        fs::write(&unrelated, b"behalten").unwrap();
        cleanup_owned_partials(partial_dir).unwrap();
        assert_eq!(fs::read(owned).unwrap(), b"teil");
        assert_eq!(fs::read(unrelated).unwrap(), b"behalten");
    }

    #[test]
    fn inbox_limits_include_files_that_existed_before_service_start() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("existing.bin"), b"1234").unwrap();
        let settings = AppSettings {
            upload_share: ShareSettings {
                enabled: true,
                path: temp.path().display().to_string(),
            },
            max_upload_bytes: Some(5),
            max_inbox_bytes: 5,
            max_inbox_files: 1,
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
        let state = TransferServiceState::new(
            settings,
            interface,
            ShareRoots {
                download: None,
                upload: Some(fs::canonicalize(temp.path()).unwrap()),
            },
            None,
        )
        .unwrap();

        assert_eq!(state.reserve_upload_object(), Err("files"));
        assert_eq!(state.reserve_upload_bytes(2), Err("bytes"));
        assert!(state.reserve_upload_bytes(1).is_ok());
        state.release_upload_bytes(1);
    }

    #[test]
    fn access_codes_have_eight_decimal_digits() {
        let code = new_code();
        assert_eq!(code.len(), ACCESS_CODE_DIGITS);
        assert!(code.bytes().all(|value| value.is_ascii_digit()));
    }

    #[test]
    fn global_cooldown_is_checked_before_the_access_code() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path()).unwrap();
        let now = Instant::now();
        state.auth_attempts.lock().unwrap().global_blocked_until = Some(now + AUTH_BLOCK_DURATION);
        let code = state.access_code.read().unwrap().clone();
        assert_eq!(
            state.verify_access_code_at(IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50)), &code, now,),
            AuthDecision::GlobalBlocked
        );
        assert_eq!(
            state.verify_access_code_at(
                IpAddr::V4(Ipv4Addr::new(192, 168, 10, 51)),
                "00000000",
                now,
            ),
            AuthDecision::GlobalBlocked
        );
    }

    #[test]
    fn auth_attempt_records_expire_and_remain_capacity_bounded() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path()).unwrap();
        let expired_address = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        assert_eq!(
            state.verify_access_code(expired_address, ""),
            AuthDecision::Invalid
        );
        {
            let mut throttle = state.auth_attempts.lock().unwrap();
            throttle
                .attempts
                .get_mut(&expired_address)
                .unwrap()
                .last_seen = Instant::now() - AUTH_ATTEMPT_TTL - Duration::from_secs(1);
        }
        assert_eq!(
            state.verify_access_code(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)), ""),
            AuthDecision::Invalid
        );
        assert!(!state
            .auth_attempts
            .lock()
            .unwrap()
            .attempts
            .contains_key(&expired_address));

        let now = Instant::now();
        let mut throttle = state.auth_attempts.lock().unwrap();
        throttle.attempts.clear();
        for index in 0..MAX_AUTH_ATTEMPT_RECORDS {
            throttle.attempts.insert(
                IpAddr::V6(Ipv6Addr::from(index as u128 + 1)),
                AttemptRecord {
                    failures: 1,
                    blocked_until: None,
                    last_seen: now,
                },
            );
        }
        throttle.global_failures = 0;
        throttle.global_window_started = now;
        drop(throttle);
        assert_eq!(
            state.verify_access_code(IpAddr::V6(Ipv6Addr::from(5_000_u128)), ""),
            AuthDecision::Invalid
        );
        let throttle = state.auth_attempts.lock().unwrap();
        assert_eq!(throttle.attempts.len(), MAX_AUTH_ATTEMPT_RECORDS);
        assert_eq!(throttle.global_failures, 1);
    }

    #[tokio::test]
    async fn limits_concurrent_requests_per_address() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path()).unwrap();
        let address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
        let mut permits = Vec::new();
        for _ in 0..MAX_REQUESTS_PER_ADDRESS {
            permits.push(state.begin_request(address, true).await.unwrap());
        }
        assert!(state.begin_request(address, true).await.is_none());
        drop(permits.pop());
        assert!(state.begin_request(address, true).await.is_some());
    }

    #[tokio::test]
    async fn partitions_authenticated_request_capacity() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path()).unwrap();
        let mut permits = Vec::new();
        for index in 0..MAX_AUTHENTICATED_REQUESTS_GLOBAL {
            permits.push(
                state
                    .begin_request(IpAddr::V6(Ipv6Addr::from(index as u128 + 1)), true)
                    .await
                    .unwrap(),
            );
        }
        assert!(state
            .begin_request(IpAddr::V6(Ipv6Addr::from(10_000_u128)), true)
            .await
            .is_none());
        assert!(state
            .begin_request(IpAddr::V6(Ipv6Addr::from(20_000_u128)), false)
            .await
            .is_some());
        drop(permits.pop());
        assert!(state
            .begin_request(IpAddr::V6(Ipv6Addr::from(10_000_u128)), true)
            .await
            .is_some());
    }

    #[test]
    fn limits_upload_chunks_per_id_and_globally_before_body_work() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path()).unwrap();
        let records = (0..=MAX_UPLOAD_CHUNKS_ACTIVE)
            .map(|index| UploadRecord {
                id: format!("upload-{index}"),
                owner_session: format!("session-{index}"),
                owner_address: IpAddr::V6(Ipv6Addr::from(index as u128 + 1)),
                name: format!("file-{index}.bin"),
                declared_size: 1,
                offset: 0,
                last_modified: 1,
                client_token: format!("token-{index}"),
                created_at: Instant::now(),
                last_activity: Instant::now(),
                cancelled: false,
                finalizing: false,
                cancel_signal: Arc::new(AtomicBool::new(false)),
                chunk_slots: Arc::new(Semaphore::new(1)),
                partial_path: temp.path().join(format!("upload-{index}.part")),
                partial_file: fs::File::create(temp.path().join(format!("upload-{index}.part")))
                    .unwrap(),
                transfer_id: format!("transfer-{index}"),
            })
            .collect::<Vec<_>>();

        let first = state.begin_upload_chunk(&records[0]).unwrap();
        assert!(matches!(
            state.begin_upload_chunk(&records[0]),
            Err("upload")
        ));
        let mut leases = vec![first];
        for record in &records[1..MAX_UPLOAD_CHUNKS_ACTIVE] {
            leases.push(state.begin_upload_chunk(record).unwrap());
        }
        assert!(matches!(
            state.begin_upload_chunk(&records[MAX_UPLOAD_CHUNKS_ACTIVE]),
            Err("global")
        ));
        drop(leases.pop());
        assert!(state
            .begin_upload_chunk(&records[MAX_UPLOAD_CHUNKS_ACTIVE])
            .is_ok());
    }

    #[tokio::test]
    async fn limits_parallel_directory_work() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path()).unwrap();
        let mut permits = Vec::new();
        for index in 0..MAX_DIRECTORY_LISTINGS_ACTIVE {
            permits.push(
                state
                    .begin_listing(
                        &format!("session-{index}"),
                        IpAddr::V6(Ipv6Addr::from(index as u128 + 1)),
                    )
                    .await
                    .unwrap(),
            );
        }
        assert!(state
            .begin_listing("overflow", IpAddr::V6(Ipv6Addr::from(100_u128)))
            .await
            .is_none());
        drop(permits.pop());
        assert!(state
            .begin_listing("recovered", IpAddr::V6(Ipv6Addr::from(101_u128)))
            .await
            .is_some());
    }

    #[tokio::test]
    async fn one_session_or_address_cannot_consume_all_active_listing_slots() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path()).unwrap();
        let address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
        let first = state.begin_listing("session-a", address).await.unwrap();
        assert!(state.begin_listing("session-a", address).await.is_none());
        let second = state.begin_listing("session-b", address).await.unwrap();
        assert!(state.begin_listing("session-c", address).await.is_none());
        assert!(state
            .begin_listing("session-c", IpAddr::V4(Ipv4Addr::new(192, 168, 10, 51)),)
            .await
            .is_some());
        drop((first, second));
    }

    #[tokio::test]
    async fn dropped_listing_waiters_do_not_release_running_blocking_work() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path()).unwrap();
        let release = Arc::new((StdMutex::new(false), Condvar::new()));
        let (started, started_rx) = std::sync::mpsc::channel();
        for index in 0..MAX_DIRECTORY_LISTINGS_ACTIVE {
            let permit = state
                .begin_listing(
                    &format!("session-{index}"),
                    IpAddr::V6(Ipv6Addr::from(index as u128 + 1)),
                )
                .await
                .unwrap();
            let release = release.clone();
            let started = started.clone();
            let task = tokio::task::spawn_blocking(move || {
                let _permit = permit;
                started.send(()).unwrap();
                let (flag, wake) = &*release;
                let mut released = flag.lock().unwrap();
                while !*released {
                    released = wake.wait(released).unwrap();
                }
            });
            drop(task);
        }
        for _ in 0..MAX_DIRECTORY_LISTINGS_ACTIVE {
            started_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("blocking listing task must start");
        }
        assert!(state
            .begin_listing("overflow", IpAddr::V6(Ipv6Addr::from(100_u128)))
            .await
            .is_none());

        let (flag, wake) = &*release;
        *flag.lock().unwrap() = true;
        wake.notify_all();
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if state
                    .begin_listing("recovered", IpAddr::V6(Ipv6Addr::from(101_u128)))
                    .await
                    .is_some()
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("capacity must recover after the blocking work exits");
    }

    #[tokio::test]
    async fn dropped_lookup_waiters_do_not_release_running_blocking_work() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path()).unwrap();
        let release = Arc::new((StdMutex::new(false), Condvar::new()));
        let (started, started_rx) = std::sync::mpsc::channel();
        for index in 0..MAX_FILESYSTEM_LOOKUPS_ACTIVE {
            let permit = state
                .begin_filesystem_lookup(IpAddr::V6(Ipv6Addr::from(index as u128 + 1)))
                .await
                .unwrap();
            let release = release.clone();
            let started = started.clone();
            let task = tokio::task::spawn_blocking(move || {
                let _permit = permit;
                started.send(()).unwrap();
                let (flag, wake) = &*release;
                let mut released = flag.lock().unwrap();
                while !*released {
                    released = wake.wait(released).unwrap();
                }
            });
            drop(task);
        }
        for _ in 0..MAX_FILESYSTEM_LOOKUPS_ACTIVE {
            started_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("blocking lookup task must start");
        }
        assert!(state
            .begin_filesystem_lookup(IpAddr::V6(Ipv6Addr::from(100_u128)))
            .await
            .is_none());

        let (flag, wake) = &*release;
        *flag.lock().unwrap() = true;
        wake.notify_all();
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if state
                    .begin_filesystem_lookup(IpAddr::V6(Ipv6Addr::from(101_u128)))
                    .await
                    .is_some()
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("capacity must recover after the blocking lookup exits");
    }

    #[tokio::test]
    async fn limits_directory_cursors_per_session_without_blocking_other_sessions() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path()).unwrap();
        let session_a = state
            .create_session(
                IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50)),
                "session-a".into(),
            )
            .await
            .unwrap();
        let session_b = state
            .create_session(
                IpAddr::V4(Ipv4Addr::new(192, 168, 10, 51)),
                "session-b".into(),
            )
            .await
            .unwrap();
        for index in 0..MAX_DIRECTORY_LISTINGS_PER_SESSION {
            state
                .create_directory_listing(
                    &session_a,
                    format!("path-{index}"),
                    String::new(),
                    temp.path().to_path_buf(),
                    fs::read_dir(temp.path()).unwrap(),
                )
                .await
                .unwrap();
        }
        assert!(matches!(
            state
                .create_directory_listing(
                    &session_a,
                    "overflow".into(),
                    String::new(),
                    temp.path().to_path_buf(),
                    fs::read_dir(temp.path()).unwrap(),
                )
                .await,
            Err("session")
        ));
        assert!(state
            .create_directory_listing(
                &session_b,
                "other".into(),
                String::new(),
                temp.path().to_path_buf(),
                fs::read_dir(temp.path()).unwrap(),
            )
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn limits_persistent_directory_cursors_per_address() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path()).unwrap();
        let address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
        let sessions = [
            state
                .create_session(address, "session-0".into())
                .await
                .unwrap(),
            state
                .create_session(address, "session-1".into())
                .await
                .unwrap(),
        ];
        for index in 0..MAX_DIRECTORY_LISTINGS_PER_ADDRESS {
            state
                .create_directory_listing(
                    &sessions[index / MAX_DIRECTORY_LISTINGS_PER_SESSION],
                    format!("path-{index}"),
                    String::new(),
                    temp.path().to_path_buf(),
                    fs::read_dir(temp.path()).unwrap(),
                )
                .await
                .unwrap();
        }
        let overflow = state
            .create_session(address, "session-overflow".into())
            .await
            .unwrap();
        assert!(matches!(
            state
                .create_directory_listing(
                    &overflow,
                    "overflow".into(),
                    String::new(),
                    temp.path().to_path_buf(),
                    fs::read_dir(temp.path()).unwrap(),
                )
                .await,
            Err("address")
        ));
        let other = state
            .create_session(
                IpAddr::V4(Ipv4Addr::new(192, 168, 10, 51)),
                "other-address".into(),
            )
            .await
            .unwrap();
        assert!(state
            .create_directory_listing(
                &other,
                "other".into(),
                String::new(),
                temp.path().to_path_buf(),
                fs::read_dir(temp.path()).unwrap(),
            )
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn cursor_lookup_does_not_refresh_activity_before_work_admission() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path()).unwrap();
        let session = state
            .create_session(
                IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50)),
                "session".into(),
            )
            .await
            .unwrap();
        let (id, listing) = state
            .create_directory_listing(
                &session,
                String::new(),
                String::new(),
                temp.path().to_path_buf(),
                fs::read_dir(temp.path()).unwrap(),
            )
            .await
            .unwrap();
        let before = Instant::now() - Duration::from_secs(30);
        listing.cursor.lock().unwrap().last_activity = before;
        assert!(state
            .directory_listing(&id, &session.id, "", "")
            .await
            .is_some());
        assert_eq!(listing.cursor.lock().unwrap().last_activity, before);
    }

    #[tokio::test]
    async fn session_limits_reject_new_sessions_without_evicting_existing_ones() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path()).unwrap();
        let first_address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 1));
        let first = state
            .create_session(first_address, "first".into())
            .await
            .unwrap();
        let mut lease = state
            .begin_download(&first.id, first.address, "datei", 10)
            .await
            .unwrap();

        for index in 2..=MAX_SESSIONS_GLOBAL {
            let address = IpAddr::V6(Ipv6Addr::from(index as u128));
            state
                .create_session(address, format!("client-{index}"))
                .await
                .unwrap();
        }
        assert!(matches!(
            state
                .create_session(IpAddr::V6(Ipv6Addr::LOCALHOST), "overflow".into())
                .await,
            Err(SessionCreateError::GlobalLimit)
        ));
        assert!(state.session_is_active(&first).await);
        assert!(lease.cancel.has_changed().is_ok_and(|changed| !changed));

        state.revoke_all().await;
        for index in 0..MAX_SESSIONS_PER_ADDRESS {
            state
                .create_session(first_address, format!("same-{index}"))
                .await
                .unwrap();
        }
        assert!(matches!(
            state.create_session(first_address, "too-many".into()).await,
            Err(SessionCreateError::AddressLimit)
        ));
        let _ = lease.cancel.changed().await;
    }

    #[tokio::test]
    async fn expired_sessions_are_reclaimed_atomically_and_cancel_resources() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path()).unwrap();
        let address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
        let expired = state
            .create_session(address, "expired".into())
            .await
            .unwrap();
        let mut lease = state
            .begin_download(&expired.id, address, "datei", 1)
            .await
            .unwrap();
        for index in 1..MAX_SESSIONS_PER_ADDRESS {
            state
                .create_session(address, format!("fresh-{index}"))
                .await
                .unwrap();
        }
        state
            .sessions
            .lock()
            .await
            .get_mut(&expired.token)
            .unwrap()
            .last_activity_instant = Instant::now() - SESSION_IDLE_TIMEOUT - Duration::from_secs(1);

        let replacement = state
            .create_session(address, "replacement".into())
            .await
            .expect("stale capacity must be reclaimed during admission");
        assert!(!state.session_is_active(&expired).await);
        assert!(state.session_is_active(&replacement).await);
        lease.cancel.changed().await.unwrap();
        assert!(*lease.cancel.borrow());
        assert_eq!(state.sessions.lock().await.len(), MAX_SESSIONS_PER_ADDRESS);
    }

    #[tokio::test]
    async fn authentication_rejects_idle_and_absolute_session_expiry() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path()).unwrap();
        let address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
        let idle = state.create_session(address, "idle".into()).await.unwrap();
        state
            .sessions
            .lock()
            .await
            .get_mut(&idle.token)
            .unwrap()
            .last_activity_instant = Instant::now() - SESSION_IDLE_TIMEOUT - Duration::from_secs(1);
        assert!(state.authenticate(&idle.token, address).await.is_none());

        let absolute = state
            .create_session(address, "absolute".into())
            .await
            .unwrap();
        {
            let mut sessions = state.sessions.lock().await;
            let record = sessions.get_mut(&absolute.token).unwrap();
            record.created_at_instant =
                Instant::now() - SESSION_MAX_LIFETIME - Duration::from_secs(1);
            record.last_activity_instant = Instant::now();
        }
        assert!(state.authenticate(&absolute.token, address).await.is_none());
    }

    #[tokio::test]
    async fn revoke_all_does_not_remove_resources_of_a_concurrently_new_session() {
        let temp = tempfile::tempdir().unwrap();
        let state = Arc::new(test_state(temp.path()).unwrap());
        state
            .create_session(IpAddr::V4(Ipv4Addr::new(192, 168, 10, 49)), "old".into())
            .await
            .unwrap();
        let filesystem = state.upload_fs_lock.clone().lock_owned().await;
        let revoke_state = state.clone();
        let revoke = tokio::spawn(async move { revoke_state.revoke_all().await });
        tokio::time::timeout(Duration::from_secs(1), async {
            while !state.sessions.lock().await.is_empty() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("bulk revoke must clear its initial session set");

        let fresh = state
            .create_session(IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50)), "fresh".into())
            .await
            .unwrap();
        let (cursor_id, _) = state
            .create_directory_listing(
                &fresh,
                String::new(),
                String::new(),
                temp.path().to_path_buf(),
                fs::read_dir(temp.path()).unwrap(),
            )
            .await
            .unwrap();
        drop(filesystem);
        revoke.await.unwrap();

        assert!(state.session_is_active(&fresh).await);
        assert!(state
            .directory_listing(&cursor_id, &fresh.id, "", "")
            .await
            .is_some());
    }

    #[tokio::test]
    async fn cursor_creation_is_ordered_with_concurrent_session_revocation() {
        let temp = tempfile::tempdir().unwrap();
        let state = Arc::new(test_state(temp.path()).unwrap());
        let session = state
            .create_session(
                IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50)),
                "session".into(),
            )
            .await
            .unwrap();
        let listings = state.directory_listings.lock().await;
        let create_state = state.clone();
        let create_session = session.clone();
        let root = temp.path().to_path_buf();
        let create = tokio::spawn(async move {
            create_state
                .create_directory_listing(
                    &create_session,
                    String::new(),
                    String::new(),
                    root.clone(),
                    fs::read_dir(root).unwrap(),
                )
                .await
        });
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if state.sessions.try_lock().is_err() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("cursor creation must hold the session admission lock");
        let revoke_state = state.clone();
        let session_id = session.id.clone();
        let revoke = tokio::spawn(async move { revoke_state.revoke_session(&session_id).await });
        drop(listings);

        let (cursor_id, _) = create.await.unwrap().unwrap();
        assert!(revoke.await.unwrap());
        assert!(!state.session_is_active(&session).await);
        assert!(state
            .directory_listing(&cursor_id, &session.id, "", "")
            .await
            .is_none());
    }

    #[tokio::test]
    async fn limits_downloads_per_session() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path()).unwrap();
        let address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
        let first = state
            .begin_download("session", address, "a", 1)
            .await
            .unwrap();
        let second = state
            .begin_download("session", address, "b", 1)
            .await
            .unwrap();
        let third = state
            .begin_download("session", address, "c", 1)
            .await
            .unwrap();
        assert!(matches!(
            state.begin_download("session", address, "d", 1).await,
            Err("session")
        ));
        drop((first, second, third));
    }

    #[tokio::test]
    async fn one_address_cannot_consume_all_global_download_slots() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path()).unwrap();
        let address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
        let other = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 51));
        let mut leases = Vec::new();
        for index in 0..MAX_DOWNLOADS_PER_ADDRESS {
            leases.push(
                state
                    .begin_download(&format!("session-{index}"), address, "datei", 1)
                    .await
                    .unwrap(),
            );
        }
        assert!(matches!(
            state.begin_download("overflow", address, "datei", 1).await,
            Err("address")
        ));
        leases.push(
            state
                .begin_download("other", other, "datei", 1)
                .await
                .expect("a second address retains global capacity"),
        );
    }

    #[tokio::test]
    async fn download_lease_has_an_absolute_deadline() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path()).unwrap();
        let address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
        let mut lease = state
            .begin_download("session", address, "datei", 1)
            .await
            .unwrap();
        lease.started_at = Instant::now() - DOWNLOAD_MAX_DURATION - Duration::from_secs(1);
        assert!(lease.expired());
    }

    #[tokio::test]
    async fn revoked_session_cancels_its_download() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path()).unwrap();
        let session = state
            .create_session(IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50)), "Test".into())
            .await
            .unwrap();
        let mut lease = state
            .begin_download(&session.id, session.address, "datei", 10)
            .await
            .unwrap();
        assert!(state.revoke_session(&session.id).await);
        lease.cancel.changed().await.unwrap();
        assert!(*lease.cancel.borrow());
    }

    #[tokio::test]
    async fn expires_inactive_upload_and_removes_only_its_partial() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path()).unwrap();
        let id = Uuid::new_v4().to_string();
        let partial_path = state
            .partial_dir
            .as_ref()
            .unwrap()
            .join(format!("{id}.part"));
        let mut partial_file = create_upload_partial(&partial_path).unwrap();
        partial_file.write_all(b"teil").unwrap();
        let transfer_id = state.record_transfer("upload", "datei.txt", 100).await;
        state.uploads.lock().await.insert(
            id.clone(),
            Arc::new(Mutex::new(UploadRecord {
                id: id.clone(),
                owner_session: "session".into(),
                owner_address: IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50)),
                name: "datei.txt".into(),
                declared_size: 100,
                offset: 4,
                last_modified: 1,
                client_token: "expired-token".into(),
                created_at: Instant::now(),
                last_activity: Instant::now() - UPLOAD_IDLE_TIMEOUT - Duration::from_secs(1),
                cancelled: false,
                finalizing: false,
                cancel_signal: Arc::new(AtomicBool::new(false)),
                chunk_slots: Arc::new(Semaphore::new(1)),
                partial_path: partial_path.clone(),
                partial_file,
                transfer_id: transfer_id.clone(),
            })),
        );

        state.expire_stale_uploads().await;
        assert!(!partial_path.exists());
        assert!(!state.uploads.lock().await.contains_key(&id));
        assert_eq!(
            state
                .transfers
                .lock()
                .await
                .iter()
                .find(|item| item.id == transfer_id)
                .unwrap()
                .state,
            "expired"
        );
    }

    #[tokio::test]
    async fn expires_upload_after_absolute_lifetime_despite_recent_progress() {
        let temp = tempfile::tempdir().unwrap();
        let state = test_state(temp.path()).unwrap();
        let id = Uuid::new_v4().to_string();
        let partial_path = state
            .partial_dir
            .as_ref()
            .unwrap()
            .join(format!("{id}.part"));
        let mut partial_file = create_upload_partial(&partial_path).unwrap();
        partial_file.write_all(b"x").unwrap();
        let transfer_id = state.record_transfer("upload", "langsam.txt", 100).await;
        state.uploads.lock().await.insert(
            id.clone(),
            Arc::new(Mutex::new(UploadRecord {
                id: id.clone(),
                owner_session: "session".into(),
                owner_address: IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50)),
                name: "langsam.txt".into(),
                declared_size: 100,
                offset: 1,
                last_modified: 1,
                client_token: "lifetime-token".into(),
                created_at: Instant::now() - UPLOAD_MAX_LIFETIME - Duration::from_secs(1),
                last_activity: Instant::now(),
                cancelled: false,
                finalizing: false,
                cancel_signal: Arc::new(AtomicBool::new(false)),
                chunk_slots: Arc::new(Semaphore::new(1)),
                partial_path: partial_path.clone(),
                partial_file,
                transfer_id,
            })),
        );

        state.expire_stale_uploads().await;
        assert!(!partial_path.exists());
        assert!(!state.uploads.lock().await.contains_key(&id));
    }
}
