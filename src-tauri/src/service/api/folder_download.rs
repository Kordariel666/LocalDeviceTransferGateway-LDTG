//! Bounded ZIP32 writer: STORE, UTF-8 names and signed data descriptors.
//! Format: PKWARE APPNOTE 6.3.10 sections 4.3 and 4.4. No file/ZIP buffering.
use super::*;
use crate::domain::shares::{is_reparse_point, RootAnchor};
use hyper::body::Body as HttpBody;
use std::io;

const MAX_ARCHIVE_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_ENTRIES: usize = 10_000;
const MAX_DEPTH: usize = 32;
const MAX_PATH_BYTES: usize = 1024;
const MAX_NAME_BYTES: usize = 4 * 1024 * 1024;
const MAX_SCANNED: usize = 20_000;
const SCAN_TIMEOUT: Duration = Duration::from_secs(30);
const BUFFER_SIZE: usize = 256 * 1024;

#[derive(Clone, Copy)]
struct Limits {
    bytes: u64,
    entries: usize,
    depth: usize,
    names: usize,
    scanned: usize,
}
const LIMITS: Limits = Limits {
    bytes: MAX_ARCHIVE_BYTES,
    entries: MAX_ENTRIES,
    depth: MAX_DEPTH,
    names: MAX_NAME_BYTES,
    scanned: MAX_SCANNED,
};

struct Entry {
    relative: String,
    name: String,
    size: u32,
    directory: bool,
    crc: u32,
    offset: u32,
}
struct Manifest {
    entries: Vec<Entry>,
    size: u64,
}

fn unavailable() -> ApiFailure {
    ApiFailure::new(
        StatusCode::NOT_FOUND,
        "PATH_UNAVAILABLE",
        "Der Ordnerinhalt ist nicht sicher verfügbar.",
    )
}
fn limit() -> ApiFailure {
    ApiFailure::new(
        StatusCode::PAYLOAD_TOO_LARGE,
        "ARCHIVE_LIMIT",
        "Der Ordner überschreitet ein ZIP-Limit (Größe, Einträge, Tiefe oder Pfadlänge).",
    )
}

// Reject ambiguous extraction names instead of renaming and creating collisions.
fn safe_component(name: &str) -> bool {
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end_matches(' ')
        .to_ascii_uppercase();
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.ends_with(['.', ' '])
        && !name.chars().any(|c| {
            c.is_control()
                || matches!(c, '/' | '\\' | ':' | '<' | '>' | '"' | '|' | '?' | '*')
                || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
        && !matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        )
        && !stem
            .strip_prefix("COM")
            .or_else(|| stem.strip_prefix("LPT"))
            .is_some_and(|s| {
                matches!(
                    s,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
}

struct ScanCancellation(Arc<AtomicBool>);
impl Drop for ScanCancellation {
    fn drop(&mut self) {
        self.0.store(true, AtomicOrdering::Release);
    }
}

fn scan(
    root: &Path,
    anchor: &RootAnchor,
    relative: &str,
    cancelled: &AtomicBool,
    deadline: Instant,
    limits: Limits,
) -> ApiResult<Manifest> {
    let selected = safe_existing(root, relative, Some(true)).map_err(|_| unavailable())?;
    let (selected, _handle, _) = anchor.open_existing(&selected).map_err(|_| unavailable())?;
    let top = selected
        .file_name()
        .map_or(Some("Freigabe"), |s| s.to_str())
        .filter(|s| safe_component(s))
        .ok_or_else(unavailable)?
        .to_owned();
    let selected_relative = selected
        .strip_prefix(root)
        .map_err(|_| unavailable())?
        .to_path_buf();
    if selected_relative.as_os_str().len() > MAX_PATH_BYTES {
        return Err(limit());
    }
    let mut stack = vec![(selected_relative, format!("{top}/"), 0)];
    let mut manifest = Manifest {
        entries: Vec::new(),
        size: 22,
    };
    let mut scanned = 0;
    let mut names = top.len() + 1;
    while let Some((relative_path, name, depth)) = stack.pop() {
        if cancelled.load(AtomicOrdering::Acquire) || Instant::now() >= deadline {
            return Err(ApiFailure::new(
                StatusCode::REQUEST_TIMEOUT,
                "ARCHIVE_SCAN_CANCELLED",
                "Die ZIP-Prüfung wurde abgebrochen oder hat zu lange gedauert.",
            ));
        }
        if depth > limits.depth
            || manifest.entries.len() >= limits.entries
            || name.len() > MAX_PATH_BYTES
        {
            return Err(limit());
        }
        let relative = relative_url_path(root, &root.join(&relative_path));
        let checked = safe_existing(root, &relative, None).map_err(|_| unavailable())?;
        let (resolved, _file, metadata) =
            anchor.open_existing(&checked).map_err(|_| unavailable())?;
        if !metadata.is_file() && !metadata.is_dir() {
            return Err(unavailable());
        }
        let directory = metadata.is_dir();
        let size = if directory { 0 } else { metadata.len() };
        manifest.size = manifest
            .size
            .checked_add(76 + 2 * name.len() as u64 + size + if directory { 0 } else { 16 })
            .ok_or_else(limit)?;
        if manifest.size > limits.bytes || size > u32::MAX as u64 || names > limits.names {
            return Err(limit());
        }
        manifest.entries.push(Entry {
            relative,
            name: name.clone(),
            size: size as u32,
            directory,
            crc: 0,
            offset: 0,
        });
        if directory {
            let iterator = fs::read_dir(&resolved).map_err(|_| unavailable())?;
            // Validate the opened directory identity again before using its iterator.
            anchor
                .validate_open_file(&_file, &resolved)
                .map_err(|_| unavailable())?;
            for item in iterator {
                scanned += 1;
                if scanned > limits.scanned {
                    return Err(limit());
                }
                if cancelled.load(AtomicOrdering::Acquire) || Instant::now() >= deadline {
                    return Err(ApiFailure::new(
                        StatusCode::REQUEST_TIMEOUT,
                        "ARCHIVE_SCAN_CANCELLED",
                        "Die ZIP-Prüfung wurde abgebrochen oder hat zu lange gedauert.",
                    ));
                }
                let item = item.map_err(|_| unavailable())?;
                let metadata = fs::symlink_metadata(item.path()).map_err(|_| unavailable())?;
                if metadata.file_type().is_symlink()
                    || is_reparse_point(&metadata)
                    || is_hidden_or_managed(&item.path(), &metadata)
                {
                    continue;
                }
                let component = item.file_name();
                let component = component
                    .to_str()
                    .filter(|s| safe_component(s))
                    .ok_or_else(unavailable)?;
                let child_name = format!(
                    "{name}{component}{}",
                    if metadata.is_dir() { "/" } else { "" }
                );
                names += child_name.len();
                if child_name.len() > MAX_PATH_BYTES
                    || names > limits.names
                    || stack.len() + manifest.entries.len() >= limits.entries
                {
                    return Err(limit());
                }
                stack.push((relative_path.join(component), child_name, depth + 1));
            }
        }
    }
    Ok(manifest)
}

fn u16le(out: &mut Vec<u8>, n: u16) {
    out.extend_from_slice(&n.to_le_bytes());
}
fn u32le(out: &mut Vec<u8>, n: u32) {
    out.extend_from_slice(&n.to_le_bytes());
}
fn header(entry: &Entry, central: bool) -> Bytes {
    let mut out = Vec::with_capacity(46 + entry.name.len());
    u32le(&mut out, if central { 0x02014b50 } else { 0x04034b50 });
    if central {
        u16le(&mut out, 20);
    }
    u16le(&mut out, 20);
    u16le(&mut out, 0x0800 | if entry.directory { 0 } else { 8 });
    u16le(&mut out, 0); // STORE
    u16le(&mut out, 0); // 1980-01-01, no source metadata promised
    u16le(&mut out, 33);
    u32le(&mut out, if central { entry.crc } else { 0 });
    u32le(&mut out, if central { entry.size } else { 0 });
    u32le(&mut out, if central { entry.size } else { 0 });
    u16le(&mut out, entry.name.len() as u16);
    u16le(&mut out, 0);
    if central {
        u16le(&mut out, 0);
        u16le(&mut out, 0);
        u16le(&mut out, 0);
        u32le(&mut out, if entry.directory { 0x10 } else { 0 });
        u32le(&mut out, entry.offset);
    }
    out.extend_from_slice(entry.name.as_bytes());
    Bytes::from(out)
}

fn archive_body(
    state: Arc<TransferServiceState>,
    address: IpAddr,
    root: PathBuf,
    mut manifest: Manifest,
) -> Body {
    let stream = async_stream::try_stream! {
        let mut offset = 0_u32;
        let mut buffer = vec![0_u8; BUFFER_SIZE];
        for entry in &mut manifest.entries {
            entry.offset = offset;
            let local = header(entry, false);
            offset += local.len() as u32;
            yield local;
            if !entry.directory {
                let (_, mut file, metadata) = safe_download_target(&state, address, root.clone(), entry.relative.clone()).await.map_err(|_| io::Error::other("ZIP source unavailable"))?;
                if !metadata.is_file() || metadata.len() != u64::from(entry.size) { Err(io::Error::other("ZIP source changed"))?; }
                let mut remaining = entry.size as usize;
                let mut crc = crc32fast::Hasher::new();
                while remaining > 0 {
                    let requested = remaining.min(buffer.len());
                    let count = file.read(&mut buffer[..requested]).await?;
                    if count == 0 { Err(io::Error::new(io::ErrorKind::UnexpectedEof, "ZIP source shortened"))?; }
                    remaining -= count;
                    crc.update(&buffer[..count]);
                    offset += count as u32;
                    yield Bytes::copy_from_slice(&buffer[..count]);
                }
                if file.read(&mut buffer[..1]).await? != 0 { Err(io::Error::other("ZIP source grew"))?; }
                entry.crc = crc.finalize();
                let mut descriptor = Vec::with_capacity(16);
                u32le(&mut descriptor, 0x08074b50); u32le(&mut descriptor, entry.crc);
                u32le(&mut descriptor, entry.size); u32le(&mut descriptor, entry.size);
                offset += 16;
                yield Bytes::from(descriptor);
            }
        }
        let central_start = offset;
        for entry in &manifest.entries {
            let central = header(entry, true);
            offset += central.len() as u32;
            yield central;
        }
        let mut end = Vec::with_capacity(22);
        u32le(&mut end, 0x06054b50); u16le(&mut end, 0); u16le(&mut end, 0);
        u16le(&mut end, manifest.entries.len() as u16); u16le(&mut end, manifest.entries.len() as u16);
        u32le(&mut end, offset - central_start); u32le(&mut end, central_start); u16le(&mut end, 0);
        yield Bytes::from(end);
    };
    fn typed(
        stream: impl futures_core::Stream<Item = Result<Bytes, io::Error>> + Send + 'static,
    ) -> Body {
        Body::from_stream(stream)
    }
    typed(stream)
}

pub(super) async fn download_folder(
    State(state): State<Arc<TransferServiceState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    query: Result<Query<FileQuery>, QueryRejection>,
) -> ApiResult<Response> {
    let Query(query) = query.map_err(|_| {
        ApiFailure::new(
            StatusCode::BAD_REQUEST,
            "QUERY_INVALID",
            "Die Ordneranfrage ist ungültig.",
        )
    })?;
    let session = authorized(&state, &headers, client.ip(), false).await?;
    if headers.contains_key(header::RANGE) {
        return Err(ApiFailure::new(
            StatusCode::RANGE_NOT_SATISFIABLE,
            "ARCHIVE_RANGE_UNSUPPORTED",
            "ZIP-Downloads müssen vollständig neu gestartet werden.",
        ));
    }
    let root = state.roots.download.clone().ok_or_else(|| {
        ApiFailure::new(
            StatusCode::NOT_FOUND,
            "DOWNLOAD_DISABLED",
            "Die Downloadfreigabe ist nicht aktiv.",
        )
    })?;
    let lease = state
        .begin_download(&session.id, session.address, "Ordner.zip", 0)
        .await
        .map_err(|_| {
            ApiFailure::new(
                StatusCode::TOO_MANY_REQUESTS,
                "DOWNLOAD_LIMIT",
                "Es laufen bereits zu viele Downloads.",
            )
        })?;
    let mut guard = DownloadGuard {
        state: state.clone(),
        lease: Some(lease),
        bytes: 0,
        complete: false,
    };
    let permit = state
        .begin_filesystem_lookup(client.ip())
        .await
        .ok_or_else(|| {
            ApiFailure::new(
                StatusCode::TOO_MANY_REQUESTS,
                "FILESYSTEM_LIMIT",
                "Es werden bereits zu viele Dateipfade geprüft.",
            )
        })?;
    let anchor = state.download_root_anchor.clone().ok_or_else(unavailable)?;
    let cancelled = ScanCancellation(Arc::new(AtomicBool::new(false)));
    let scan_cancel = cancelled.0.clone();
    let scan_root = root.clone();
    let mut job = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        scan(
            &scan_root,
            &anchor,
            &query.path,
            &scan_cancel,
            Instant::now() + SCAN_TIMEOUT,
            LIMITS,
        )
    });
    let manifest = tokio::select! {
        result = &mut job => result.map_err(|_| unavailable())??,
        _ = guard.lease.as_mut().unwrap().cancel.changed() => return Err(unavailable()),
        _ = tokio::time::sleep(SCAN_TIMEOUT) => return Err(ApiFailure::new(StatusCode::REQUEST_TIMEOUT, "ARCHIVE_SCAN_CANCELLED", "Die ZIP-Prüfung hat zu lange gedauert.")),
    };
    drop(cancelled);
    ensure_session_active(&state, &session).await?;
    let name = format!("{}.zip", manifest.entries[0].name.trim_end_matches('/'));
    let size = manifest.size;
    if let Some(transfer) = state
        .transfers
        .lock()
        .await
        .iter_mut()
        .find(|item| item.id == guard.lease.as_ref().unwrap().id)
    {
        transfer.name.clone_from(&name);
        transfer.total_bytes = size;
    }
    let mut body = archive_body(state.clone(), client.ip(), root, manifest);
    let stream = async_stream::stream! {
        loop {
            let lease = guard.lease.as_mut().unwrap();
            let remaining = lease.remaining();
            let next = if *lease.cancel.borrow() || lease.expired() { None } else {
                tokio::select! {
                    frame = std::future::poll_fn(|cx| std::pin::Pin::new(&mut body).poll_frame(cx)) => frame,
                    _ = lease.cancel.changed() => None,
                    _ = tokio::time::sleep(remaining) => None,
                }
            };
            match next {
                Some(Ok(frame)) => if let Ok(bytes) = frame.into_data() {
                    guard.bytes += bytes.len() as u64;
                    state.update_transfer(&guard.lease.as_ref().unwrap().id, guard.bytes, None).await;
                    if guard.bytes == size {
                        state.finish_download(&guard.lease.as_ref().unwrap().id, guard.bytes, TransferState::Complete).await;
                        guard.mark_complete();
                        yield Ok::<Bytes, io::Error>(bytes);
                        return;
                    }
                    yield Ok::<Bytes, io::Error>(bytes);
                },
                result => {
                    let lease = guard.lease.as_ref().unwrap();
                    let final_state = if *lease.cancel.borrow() { TransferState::Cancelled } else if lease.expired() { TransferState::Expired } else if result.is_none() && guard.bytes == size { TransferState::Complete } else { TransferState::Failed };
                    let successful = final_state == TransferState::Complete;
                    state.finish_download(&lease.id, guard.bytes, final_state).await;
                    guard.mark_complete();
                    if !successful { yield Err(io::Error::other("ZIP download interrupted")); }
                    break;
                }
            }
        }
    };
    let mut response = Response::new(Body::from_stream(stream));
    download_headers(&mut response, &name, size, 0, size - 1, false);
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/zip"),
    );
    response
        .headers_mut()
        .insert(header::ACCEPT_RANGES, HeaderValue::from_static("none"));
    Ok(response)
}

#[cfg(test)]
mod tests;
