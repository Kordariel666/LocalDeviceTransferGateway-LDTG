use crate::domain::{
    network::NetworkInterfaceInfo,
    settings::RuntimeSettings,
    shares::{delete_open_upload, is_reparse_point, RootAnchor, ShareRoots},
    types::{
        DirectoryEntry, ServiceState, ServiceStatus, SessionChangedEvent, SessionInfo,
        TransferChangedEvent, TransferDirection, TransferInfo, TransferState,
    },
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use rand::{rand_core::UnwrapErr, rngs::SysRng, Rng, RngExt};
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

mod cursors;
mod downloads;
mod journal;
mod limits;
mod sessions;
mod uploads;

#[cfg(test)]
use crate::domain::shares::create_upload_partial;

pub const CHUNK_SIZE: usize = 8 * 1024 * 1024;
pub const DISK_RESERVE: u64 = 1024 * 1024 * 1024;
pub const MAX_DOWNLOADS_GLOBAL: usize = 12;
pub const MAX_DOWNLOADS_PER_SESSION: usize = 3;
pub const MAX_DOWNLOADS_PER_ADDRESS: usize = 4;
pub const MAX_UPLOAD_CHUNKS_ACTIVE: usize = 8;
pub const MAX_UPLOAD_IO_ACTIVE: usize = 4;
pub const MAX_UPLOAD_IO_PER_ADDRESS: usize = 2;
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
pub const MAX_DEVICE_NAME_CHARS: usize = 64;
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
    pub device_name: Option<String>,
    pub client_name: String,
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

    fn info(&self) -> SessionInfo {
        SessionInfo {
            id: self.id.clone(),
            address: self.address.to_string(),
            device_name: self.device_name.clone(),
            client_name: self.client_name.clone(),
            created_at: self.created_at.clone(),
            last_activity: self.last_activity.clone(),
        }
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
    InvalidDeviceName,
}

fn has_forbidden_device_name_character(value: char) -> bool {
    value.is_control()
        || matches!(
            value,
            '\u{061c}'
                | '\u{200e}'
                | '\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}'
        )
}

fn normalize_device_name(value: Option<&str>) -> Result<Option<String>, SessionCreateError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > MAX_DEVICE_NAME_CHARS
        || value.len() > MAX_DEVICE_NAME_CHARS * 4
        || value.chars().any(has_forbidden_device_name_character)
    {
        return Err(SessionCreateError::InvalidDeviceName);
    }
    Ok(Some(value.to_string()))
}

fn describe_user_agent(user_agent: &str) -> String {
    let browser = if user_agent.contains("EdgA/")
        || user_agent.contains("EdgiOS/")
        || user_agent.contains("Edg/")
    {
        "Microsoft Edge"
    } else if user_agent.contains("SamsungBrowser/") {
        "Samsung Internet"
    } else if user_agent.contains("OPR/") || user_agent.contains("Opera/") {
        "Opera"
    } else if user_agent.contains("FxiOS/") || user_agent.contains("Firefox/") {
        "Firefox"
    } else if user_agent.contains("CriOS/") || user_agent.contains("Chrome/") {
        "Chrome"
    } else if user_agent.contains("Version/") && user_agent.contains("Safari/") {
        "Safari"
    } else {
        "Unbekannter Browser"
    };
    let device = if user_agent.contains("iPhone") {
        Some("iPhone")
    } else if user_agent.contains("iPad") {
        Some("iPad")
    } else if user_agent.contains("Android") {
        Some("Android")
    } else if user_agent.contains("Windows") {
        Some("Windows")
    } else if user_agent.contains("Macintosh") || user_agent.contains("Mac OS X") {
        Some("macOS")
    } else if user_agent.contains("CrOS") {
        Some("ChromeOS")
    } else if user_agent.contains("Linux") {
        Some("Linux")
    } else {
        None
    };
    match device {
        Some(device) => format!("{browser} auf {device}"),
        None => browser.to_string(),
    }
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

pub struct UploadIoPermit {
    _global: OwnedSemaphorePermit,
    _address: OwnedSemaphorePermit,
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
    pub partial_file: Arc<fs::File>,
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

#[derive(Debug)]
struct TransferNotification {
    transferred_bytes: u64,
    emitted_at: Instant,
    sampled_bytes: u64,
    sampled_at: Instant,
    smoothed_bytes_per_second: Option<f64>,
    speed_sample_count: u32,
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
    pub settings: RuntimeSettings,
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
    upload_io_slots: Arc<Semaphore>,
    upload_io_address_slots: Mutex<HashMap<IpAddr, Weak<Semaphore>>>,
    request_slots: Arc<Semaphore>,
    anonymous_request_slots: Arc<Semaphore>,
    authenticated_request_slots: Arc<Semaphore>,
    request_address_slots: Mutex<HashMap<IpAddr, Weak<Semaphore>>>,
    inbox_usage: StdMutex<InboxUsage>,
    pub transfers: Mutex<Vec<TransferInfo>>,
    transfer_notifications: Mutex<HashMap<String, TransferNotification>>,
    pub started_at: String,
    pub last_activity_unix: AtomicI64,
    pub app: Option<AppHandle>,
    pub upload_fs_lock: Arc<Mutex<()>>,
    #[cfg(test)]
    pub upload_io_test_gate: StdMutex<Option<UploadIoTestGate>>,
}

#[cfg(test)]
#[derive(Clone)]
pub struct UploadIoTestGate {
    pub started: std::sync::mpsc::Sender<()>,
    pub release: Arc<(StdMutex<bool>, std::sync::Condvar)>,
}

fn random_token(bytes: usize) -> String {
    let mut value = vec![0_u8; bytes];
    UnwrapErr(SysRng).fill_bytes(&mut value);
    URL_SAFE_NO_PAD.encode(value)
}

fn new_code() -> String {
    format!("{:08}", UnwrapErr(SysRng).random_range(0..100_000_000_u32))
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
        settings: RuntimeSettings,
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
            upload_io_slots: Arc::new(Semaphore::new(MAX_UPLOAD_IO_ACTIVE)),
            upload_io_address_slots: Mutex::new(HashMap::new()),
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
            transfer_notifications: Mutex::new(HashMap::new()),
            started_at: Utc::now().to_rfc3339(),
            last_activity_unix: AtomicI64::new(Utc::now().timestamp()),
            app,
            upload_fs_lock: Arc::new(Mutex::new(())),
            #[cfg(test)]
            upload_io_test_gate: StdMutex::new(None),
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
            .filter(|item| item.state == TransferState::Active)
            .count()
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
}

#[cfg(test)]
mod tests;
