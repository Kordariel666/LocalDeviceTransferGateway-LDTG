use super::*;

#[derive(Deserialize)]
pub(super) struct FileQuery {
    path: String,
}

pub(super) fn parse_range(headers: &HeaderMap, size: u64) -> ApiResult<(u64, u64, bool)> {
    let Some(value) = headers.get(header::RANGE) else {
        return Ok((0, size.saturating_sub(1), false));
    };
    let value = value.to_str().map_err(|_| {
        ApiFailure::new(
            StatusCode::RANGE_NOT_SATISFIABLE,
            "RANGE_INVALID",
            "Ungültiger Downloadbereich.",
        )
    })?;
    let value = value.strip_prefix("bytes=").ok_or_else(|| {
        ApiFailure::new(
            StatusCode::RANGE_NOT_SATISFIABLE,
            "RANGE_INVALID",
            "Nur Byte-Bereiche werden unterstützt.",
        )
    })?;
    if value.contains(',') {
        return Err(ApiFailure::new(
            StatusCode::RANGE_NOT_SATISFIABLE,
            "MULTI_RANGE_UNSUPPORTED",
            "Mehrere Downloadbereiche werden nicht unterstützt.",
        ));
    }
    let (left, right) = value.split_once('-').ok_or_else(|| {
        ApiFailure::new(
            StatusCode::RANGE_NOT_SATISFIABLE,
            "RANGE_INVALID",
            "Ungültiger Downloadbereich.",
        )
    })?;
    let (start, end) = if left.is_empty() {
        let suffix = right.parse::<u64>().map_err(|_| {
            ApiFailure::new(
                StatusCode::RANGE_NOT_SATISFIABLE,
                "RANGE_INVALID",
                "Ungültiger Downloadbereich.",
            )
        })?;
        (size.saturating_sub(suffix), size.saturating_sub(1))
    } else {
        let start = left.parse::<u64>().map_err(|_| {
            ApiFailure::new(
                StatusCode::RANGE_NOT_SATISFIABLE,
                "RANGE_INVALID",
                "Ungültiger Downloadbereich.",
            )
        })?;
        let end = if right.is_empty() {
            size.saturating_sub(1)
        } else {
            right.parse::<u64>().map_err(|_| {
                ApiFailure::new(
                    StatusCode::RANGE_NOT_SATISFIABLE,
                    "RANGE_INVALID",
                    "Ungültiger Downloadbereich.",
                )
            })?
        };
        (start, end.min(size.saturating_sub(1)))
    };
    if size == 0 || start >= size || end < start {
        return Err(ApiFailure::new(
            StatusCode::RANGE_NOT_SATISFIABLE,
            "RANGE_INVALID",
            "Der angeforderte Bereich liegt außerhalb der Datei.",
        ));
    }
    Ok((start, end, true))
}

pub(super) fn download_headers(
    response: &mut Response,
    file_name: &str,
    size: u64,
    start: u64,
    end: u64,
    partial: bool,
) {
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    let content_length = if size == 0 {
        0
    } else {
        end.saturating_sub(start) + 1
    };
    headers.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&content_length.to_string()).unwrap(),
    );
    let encoded = utf8_percent_encode(file_name, NON_ALPHANUMERIC).to_string();
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename*=UTF-8''{encoded}"))
            .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
    );
    if partial {
        headers.insert(
            header::CONTENT_RANGE,
            HeaderValue::from_str(&format!("bytes {start}-{end}/{size}")).unwrap(),
        );
    }
}

pub(super) struct DownloadGuard {
    state: Arc<TransferServiceState>,
    lease: Option<DownloadLease>,
    bytes: u64,
    complete: bool,
}
impl DownloadGuard {
    fn mark_complete(&mut self) {
        self.complete = true;
    }
}
impl Drop for DownloadGuard {
    fn drop(&mut self) {
        if !self.complete {
            let state = self.state.clone();
            let lease = self.lease.take();
            let id = lease.as_ref().map(|lease| lease.id.clone());
            let bytes = self.bytes;
            if let Some(id) = id {
                tokio::spawn(async move {
                    state
                        .finish_download(&id, bytes, TransferState::Cancelled)
                        .await;
                    drop(lease);
                });
            }
        }
    }
}

pub(super) async fn safe_download_target(
    state: &TransferServiceState,
    address: IpAddr,
    root: PathBuf,
    relative_path: String,
) -> ApiResult<(PathBuf, tokio::fs::File, fs::Metadata)> {
    let anchor = state.download_root_anchor.clone().ok_or_else(|| {
        ApiFailure::new(
            StatusCode::NOT_FOUND,
            "PATH_UNAVAILABLE",
            "Datei ist nicht verfügbar.",
        )
    })?;
    let permit = state
        .begin_filesystem_lookup(address)
        .await
        .ok_or_else(|| {
            ApiFailure::new(
                StatusCode::TOO_MANY_REQUESTS,
                "FILESYSTEM_LIMIT",
                "Es werden bereits zu viele Dateipfade geprüft.",
            )
        })?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let path = safe_existing(&root, &relative_path, Some(false)).map_err(|_| {
            ApiFailure::new(
                StatusCode::NOT_FOUND,
                "PATH_UNAVAILABLE",
                "Datei ist nicht verfügbar.",
            )
        })?;
        let file = fs::File::open(&path).map_err(|_| {
            ApiFailure::new(
                StatusCode::NOT_FOUND,
                "PATH_UNAVAILABLE",
                "Datei ist nicht verfügbar.",
            )
        })?;
        let resolved = anchor.validate_open_file(&file, &path).map_err(|_| {
            ApiFailure::new(
                StatusCode::NOT_FOUND,
                "PATH_UNAVAILABLE",
                "Datei ist nicht verfügbar.",
            )
        })?;
        let metadata = file.metadata().map_err(|_| {
            ApiFailure::new(
                StatusCode::NOT_FOUND,
                "PATH_UNAVAILABLE",
                "Datei ist nicht verfügbar.",
            )
        })?;
        Ok((resolved, tokio::fs::File::from_std(file), metadata))
    })
    .await
    .map_err(|_| {
        ApiFailure::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "FILESYSTEM_TASK_FAILED",
            "Der Dateipfad konnte nicht sicher geprüft werden.",
        )
    })?
}

pub(super) async fn download(
    State(state): State<Arc<TransferServiceState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    query: Result<Query<FileQuery>, QueryRejection>,
) -> ApiResult<Response> {
    let Query(query) = query.map_err(|error| {
        ApiFailure::new(
            error.status(),
            "QUERY_INVALID",
            "Die Dateianfrage ist ungültig.",
        )
    })?;
    let session = authorized(&state, &headers, client.ip(), false).await?;
    let root = state.roots.download.as_ref().cloned().ok_or_else(|| {
        ApiFailure::new(
            StatusCode::NOT_FOUND,
            "DOWNLOAD_DISABLED",
            "Die Downloadfreigabe ist nicht aktiv.",
        )
    })?;
    let (path, mut file, metadata) =
        safe_download_target(&state, client.ip(), root, query.path).await?;
    ensure_session_active(&state, &session).await?;
    let size = metadata.len();
    if size == 0 {
        let mut response = Body::empty().into_response();
        download_headers(
            &mut response,
            path.file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("Datei"),
            0,
            0,
            0,
            false,
        );
        return Ok(response);
    }
    let (start, end, partial) = parse_range(&headers, size)?;
    let total = end - start + 1;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Datei")
        .to_string();
    let lease = state
        .begin_download(&session.id, session.address, &name, total)
        .await
        .map_err(|scope| {
            let message = match scope {
                "session" => "Diese Sitzung hat bereits zu viele gleichzeitige Downloads.",
                "address" => "Von dieser Geräteadresse laufen bereits zu viele Downloads.",
                _ => "LDTG verarbeitet bereits die maximale Zahl gleichzeitiger Downloads.",
            };
            ApiFailure::new(StatusCode::TOO_MANY_REQUESTS, "DOWNLOAD_LIMIT", message)
        })?;
    let mut guard = DownloadGuard {
        state: state.clone(),
        lease: Some(lease),
        bytes: 0,
        complete: false,
    };
    if let Err(error) = ensure_session_active(&state, &session).await {
        let id = guard
            .lease
            .as_ref()
            .expect("download lease exists")
            .id
            .clone();
        state
            .finish_download(&id, 0, TransferState::Cancelled)
            .await;
        guard.mark_complete();
        return Err(error);
    }
    if file.seek(std::io::SeekFrom::Start(start)).await.is_err() {
        let id = guard
            .lease
            .as_ref()
            .expect("download lease exists")
            .id
            .clone();
        state.finish_download(&id, 0, TransferState::Failed).await;
        guard.mark_complete();
        return Err(ApiFailure::new(
            StatusCode::BAD_REQUEST,
            "SEEK_FAILED",
            "Downloadposition ist ungültig.",
        ));
    }
    let stream_state = state.clone();
    let stream = async_stream::stream! {
        let mut remaining = total;
        let mut sent = 0_u64;
        let mut cancelled = false;
        let mut expired = false;
        let mut buffer = vec![0_u8; 256 * 1024];
        while remaining > 0 {
            let lease = guard.lease.as_mut().expect("download lease exists");
            if *lease.cancel.borrow() { cancelled = true; break; }
            if lease.expired() { expired = true; break; }
            let requested = buffer.len().min(remaining as usize);
            let lease_remaining = lease.remaining();
            let read_result = tokio::select! {
                _ = lease.cancel.changed() => { cancelled = true; Ok::<usize, std::io::Error>(0) }
                _ = tokio::time::sleep(lease_remaining) => { expired = true; Ok::<usize, std::io::Error>(0) }
                result = file.read(&mut buffer[..requested]) => result,
            };
            let count = match read_result {
                Ok(count) => count,
                Err(error) => {
                    let id = guard.lease.as_ref().expect("download lease exists").id.clone();
                    stream_state.finish_download(&id, sent, TransferState::Failed).await;
                    guard.mark_complete();
                    yield Err::<Bytes, std::io::Error>(error);
                    return;
                }
            };
            if count == 0 { break; }
            sent += count as u64; remaining -= count as u64; guard.bytes = sent;
            let id = guard.lease.as_ref().expect("download lease exists").id.clone();
            stream_state.update_transfer(&id, sent, None).await;
            yield Ok::<Bytes, std::io::Error>(Bytes::copy_from_slice(&buffer[..count]));
        }
        let final_state = if cancelled {
            TransferState::Cancelled
        } else if expired {
            TransferState::Expired
        } else if sent == total {
            TransferState::Complete
        } else {
            TransferState::Failed
        };
        let id = guard.lease.as_ref().expect("download lease exists").id.clone();
        stream_state.finish_download(&id, sent, final_state).await;
        guard.mark_complete();
    };
    let mut response = Response::new(Body::from_stream(stream));
    *response.status_mut() = if partial {
        StatusCode::PARTIAL_CONTENT
    } else {
        StatusCode::OK
    };
    download_headers(&mut response, &name, size, start, end, partial);
    Ok(response)
}

pub(super) async fn download_head(
    State(state): State<Arc<TransferServiceState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    query: Result<Query<FileQuery>, QueryRejection>,
) -> ApiResult<Response> {
    let Query(query) = query.map_err(|error| {
        ApiFailure::new(
            error.status(),
            "QUERY_INVALID",
            "Die Dateianfrage ist ungültig.",
        )
    })?;
    let session = authorized(&state, &headers, client.ip(), false).await?;
    let root = state.roots.download.as_ref().cloned().ok_or_else(|| {
        ApiFailure::new(
            StatusCode::NOT_FOUND,
            "DOWNLOAD_DISABLED",
            "Die Downloadfreigabe ist nicht aktiv.",
        )
    })?;
    let (path, _file, metadata) =
        safe_download_target(&state, client.ip(), root, query.path).await?;
    ensure_session_active(&state, &session).await?;
    let size = metadata.len();
    let mut response = Body::empty().into_response();
    let (start, end, partial) = if size == 0 {
        (0, 0, false)
    } else {
        parse_range(&headers, size)?
    };
    *response.status_mut() = if partial {
        StatusCode::PARTIAL_CONTENT
    } else {
        StatusCode::OK
    };
    download_headers(
        &mut response,
        path.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Datei"),
        size,
        start,
        end,
        partial,
    );
    Ok(response)
}
