use super::*;

#[derive(Clone)]
pub(super) struct UploadChunkPermit {
    pub(super) _lease: Arc<UploadChunkLease>,
}

pub(super) struct UploadByteReservation {
    state: Arc<TransferServiceState>,
    bytes: u64,
    committed: bool,
}

pub(super) struct UploadObjectReservation {
    state: Arc<TransferServiceState>,
    committed: bool,
}

impl UploadObjectReservation {
    fn new(state: Arc<TransferServiceState>) -> ApiResult<Self> {
        state.reserve_upload_object().map_err(|_| {
            ApiFailure::new(
                StatusCode::INSUFFICIENT_STORAGE,
                "INBOX_FILE_LIMIT",
                "Der Upload-Eingang hat die konfigurierte Dateianzahl erreicht.",
            )
        })?;
        Ok(Self {
            state,
            committed: false,
        })
    }

    fn commit(&mut self) {
        self.committed = true;
    }
}

impl Drop for UploadObjectReservation {
    fn drop(&mut self) {
        if !self.committed {
            self.state.release_upload(0);
        }
    }
}

impl UploadByteReservation {
    fn new(state: Arc<TransferServiceState>, bytes: u64) -> ApiResult<Self> {
        state.reserve_upload_bytes(bytes).map_err(|_| {
            ApiFailure::new(
                StatusCode::INSUFFICIENT_STORAGE,
                "INBOX_BYTE_LIMIT",
                "Der Upload-Eingang hat das konfigurierte Speicherlimit erreicht.",
            )
        })?;
        Ok(Self {
            state,
            bytes,
            committed: false,
        })
    }

    fn commit(&mut self) {
        self.committed = true;
    }
}

impl Drop for UploadByteReservation {
    fn drop(&mut self) {
        if !self.committed {
            self.state.release_upload_bytes(self.bytes);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UploadChunkIoError {
    Cancelled,
    Failed,
}

pub(super) fn upload_io_busy() -> ApiFailure {
    ApiFailure::new(
        StatusCode::TOO_MANY_REQUESTS,
        "UPLOAD_IO_BUSY",
        "DMDC verarbeitet bereits die maximale Zahl blockierender Uploadvorgänge.",
    )
}

#[cfg(windows)]
pub(super) fn write_all_at(
    file: &fs::File,
    mut data: &[u8],
    mut offset: u64,
) -> std::io::Result<()> {
    use std::os::windows::fs::FileExt;

    while !data.is_empty() {
        let written = file.seek_write(data, offset)?;
        if written == 0 {
            return Err(std::io::ErrorKind::WriteZero.into());
        }
        data = &data[written..];
        offset = offset.saturating_add(written as u64);
    }
    Ok(())
}

#[cfg(unix)]
pub(super) fn write_all_at(
    file: &fs::File,
    mut data: &[u8],
    mut offset: u64,
) -> std::io::Result<()> {
    use std::os::unix::fs::FileExt;

    while !data.is_empty() {
        let written = file.write_at(data, offset)?;
        if written == 0 {
            return Err(std::io::ErrorKind::WriteZero.into());
        }
        data = &data[written..];
        offset = offset.saturating_add(written as u64);
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
pub(super) fn write_all_at(file: &fs::File, data: &[u8], offset: u64) -> std::io::Result<()> {
    use std::io::{Seek, Write};

    let mut file = file.try_clone()?;
    file.seek(std::io::SeekFrom::Start(offset))?;
    file.write_all(data)
}

#[cfg(test)]
pub(super) fn wait_for_upload_io_test_gate(state: &TransferServiceState) {
    let gate = state
        .upload_io_test_gate
        .lock()
        .expect("upload I/O test gate lock poisoned")
        .clone();
    if let Some(gate) = gate {
        let _ = gate.started.send(());
        let (released, wake) = &*gate.release;
        let mut released = released.lock().expect("upload I/O test gate poisoned");
        while !*released {
            released = wake.wait(released).expect("upload I/O test gate poisoned");
        }
    }
}

pub(super) fn write_upload_chunk_blocking(
    state: &TransferServiceState,
    file: &fs::File,
    partial_path: &Path,
    offset: u64,
    body: &[u8],
    cancel_signal: &AtomicBool,
) -> Result<(), UploadChunkIoError> {
    #[cfg(test)]
    wait_for_upload_io_test_gate(state);

    let result = (|| -> std::io::Result<()> {
        if cancel_signal.load(AtomicOrdering::Acquire) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "upload cancelled",
            ));
        }
        if !state.roots_are_current() {
            return Err(std::io::Error::other("upload root changed"));
        }
        let anchor = state
            .partial_dir_anchor
            .as_ref()
            .ok_or_else(|| std::io::Error::other("partial root unavailable"))?;
        anchor
            .validate_open_file(file, partial_path)
            .map_err(std::io::Error::other)?;
        let upload_root = state
            .roots
            .upload
            .as_ref()
            .ok_or_else(|| std::io::Error::other("upload root unavailable"))?;
        if fs2::available_space(upload_root)? < DISK_RESERVE.saturating_add(body.len() as u64) {
            return Err(std::io::Error::other("disk reserve reached"));
        }
        write_all_at(file, body, offset)?;
        file.sync_data()?;
        if cancel_signal.load(AtomicOrdering::Acquire) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "upload cancelled",
            ));
        }
        Ok(())
    })();

    match result {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = file.set_len(offset);
            if error.kind() == std::io::ErrorKind::Interrupted {
                Err(UploadChunkIoError::Cancelled)
            } else {
                Err(UploadChunkIoError::Failed)
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CreateUploadRequest {
    name: String,
    size: u64,
    last_modified: u64,
    #[serde(default)]
    client_token: String,
}

pub(super) async fn create_upload(
    State(state): State<Arc<TransferServiceState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    payload: Result<Json<CreateUploadRequest>, JsonRejection>,
) -> ApiResult<Json<UploadResponse>> {
    let Json(payload) = payload.map_err(|error| {
        ApiFailure::new(
            error.status(),
            "UPLOAD_BODY_INVALID",
            "Die Uploadinformationen sind ungültig.",
        )
    })?;
    let session = authorized(&state, &headers, client.ip(), true).await?;
    let root = state.roots.upload.as_ref().cloned().ok_or_else(|| {
        ApiFailure::new(
            StatusCode::NOT_FOUND,
            "UPLOAD_DISABLED",
            "Der Upload-Eingang ist nicht aktiv.",
        )
    })?;
    if state
        .settings
        .max_upload_bytes
        .is_some_and(|limit| payload.size > limit)
    {
        return Err(ApiFailure::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "FILE_TOO_LARGE",
            "Die Datei überschreitet das konfigurierte Uploadlimit.",
        ));
    }
    let name = safe_file_name_for_root(&root, &payload.name)
        .map_err(|message| ApiFailure::new(StatusCode::BAD_REQUEST, "NAME_INVALID", message))?;
    let client_token = if payload.client_token.is_empty() {
        Uuid::new_v4().to_string()
    } else if payload.client_token.len() == 32
        && payload
            .client_token
            .bytes()
            .all(|value| value.is_ascii_hexdigit())
    {
        payload.client_token.clone()
    } else {
        return Err(ApiFailure::new(
            StatusCode::BAD_REQUEST,
            "UPLOAD_TOKEN_INVALID",
            "Die Wiederaufnahme-ID des Uploads ist ungültig.",
        ));
    };
    if let Some(completed) = state
        .completed_upload_by_token(&client_token, &name, payload.size, payload.last_modified)
        .await
    {
        return Ok(Json(UploadResponse {
            upload_id: completed.upload_id,
            offset: completed.total_bytes,
            total_bytes: completed.total_bytes,
            chunk_size: CHUNK_SIZE,
            service_id: state.service_id.clone(),
            last_modified: completed.last_modified,
        }));
    }
    let reservation_lock = state.upload_fs_lock.clone().lock_owned().await;
    let io_permit = state
        .begin_upload_io(client.ip())
        .await
        .ok_or_else(upload_io_busy)?;
    let task_state = state.clone();
    let create = tokio::spawn(async move {
        let _reservation_lock = reservation_lock;
        ensure_session_active(&task_state, &session).await?;
        let active_uploads: Vec<_> = task_state.uploads.lock().await.values().cloned().collect();
        if active_uploads.len() >= 64 {
            return Err(ApiFailure::new(
                StatusCode::TOO_MANY_REQUESTS,
                "TOO_MANY_UPLOADS",
                "Es sind bereits zu viele unvollständige Uploads vorhanden.",
            ));
        }
        let mut address_uploads = 0_usize;
        for upload in &active_uploads {
            let upload = upload.lock().await;
            if !upload.cancelled
                && upload.owner_session == session.id
                && upload.client_token == client_token
                && upload.name == name
                && upload.declared_size == payload.size
                && upload.last_modified == payload.last_modified
            {
                return Ok(UploadResponse {
                    upload_id: upload.id.clone(),
                    offset: upload.offset,
                    total_bytes: upload.declared_size,
                    chunk_size: CHUNK_SIZE,
                    service_id: task_state.service_id.clone(),
                    last_modified: upload.last_modified,
                });
            }
            if !upload.cancelled && upload.owner_address == session.address {
                address_uploads += 1;
            }
        }
        if address_uploads >= MAX_UPLOADS_PER_ADDRESS {
            return Err(ApiFailure::new(
                StatusCode::TOO_MANY_REQUESTS,
                "UPLOAD_CLIENT_LIMIT",
                "Von dieser Geräteadresse sind bereits zu viele unvollständige Uploads vorhanden.",
            ));
        }

        let id = Uuid::new_v4().to_string();
        let partial_dir = task_state.partial_dir.as_ref().cloned().ok_or_else(|| {
            ApiFailure::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "PARTIAL_DIR_FAILED",
                "Temporärer Uploadordner ist nicht verfügbar.",
            )
        })?;
        let partial_path = partial_dir.join(format!("{id}.part"));
        let blocking_state = task_state.clone();
        let blocking_root = root.clone();
        let blocking_path = partial_path.clone();
        let (prepared, _io_permit) = tokio::task::spawn_blocking(move || {
            let result = (|| -> ApiResult<(Arc<fs::File>, UploadObjectReservation)> {
                if !blocking_state.roots_are_current() {
                    return Err(ApiFailure::new(
                        StatusCode::CONFLICT,
                        "UPLOAD_ROOT_CHANGED",
                        "Der Upload-Eingang wurde während des Betriebs verändert.",
                    ));
                }
                blocking_state.refresh_inbox_usage().map_err(|_| {
                    ApiFailure::new(
                        StatusCode::INSUFFICIENT_STORAGE,
                        "INBOX_USAGE_UNKNOWN",
                        "Die Belegung des Upload-Eingangs konnte nicht sicher bestimmt werden.",
                    )
                })?;
                let available = fs2::available_space(&blocking_root).map_err(|_| {
                    ApiFailure::new(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "SPACE_UNKNOWN",
                        "Freier Speicher konnte nicht geprüft werden.",
                    )
                })?;
                if available < DISK_RESERVE {
                    return Err(ApiFailure::new(
                        StatusCode::INSUFFICIENT_STORAGE,
                        "DISK_FULL",
                        "Die Sicherheitsreserve des Datenträgers ist erreicht.",
                    ));
                }
                let reservation = UploadObjectReservation::new(blocking_state.clone())?;
                let file = create_upload_partial(&blocking_path).map_err(|_| {
                    ApiFailure::new(
                        StatusCode::CONFLICT,
                        "UPLOAD_COLLISION",
                        "Upload konnte nicht angelegt werden.",
                    )
                })?;
                Ok((Arc::new(file), reservation))
            })();
            (result, io_permit)
        })
        .await
        .map_err(|_| {
            ApiFailure::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "UPLOAD_IO_FAILED",
                "Der Upload konnte nicht sicher vorbereitet werden.",
            )
        })?;
        let (partial_file, mut object_reservation) = prepared?;
        if let Err(error) = ensure_session_active(&task_state, &session).await {
            let cleanup_file = partial_file.clone();
            let cleanup_path = partial_path.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let _io_permit = _io_permit;
                delete_open_upload(&cleanup_file, &cleanup_path)
            })
            .await;
            return Err(error);
        }

        let transfer_id = Uuid::new_v4().to_string();
        let transfer_name = name.clone();
        let record = UploadRecord {
            id: id.clone(),
            owner_session: session.id,
            owner_address: session.address,
            name,
            declared_size: payload.size,
            offset: 0,
            last_modified: payload.last_modified,
            client_token,
            created_at: Instant::now(),
            last_activity: Instant::now(),
            cancelled: false,
            finalizing: false,
            cancel_signal: Arc::new(AtomicBool::new(false)),
            chunk_slots: Arc::new(tokio::sync::Semaphore::new(1)),
            partial_path,
            partial_file,
            transfer_id: transfer_id.clone(),
        };
        task_state
            .uploads
            .lock()
            .await
            .insert(id.clone(), Arc::new(Mutex::new(record)));
        object_reservation.commit();
        task_state
            .record_transfer_with_id(
                &transfer_id,
                TransferDirection::Upload,
                &transfer_name,
                payload.size,
            )
            .await;
        Ok(UploadResponse {
            upload_id: id,
            offset: 0,
            total_bytes: payload.size,
            chunk_size: CHUNK_SIZE,
            service_id: task_state.service_id.clone(),
            last_modified: payload.last_modified,
        })
    });
    create
        .await
        .map_err(|_| {
            ApiFailure::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "UPLOAD_IO_FAILED",
                "Der Upload konnte nicht sicher vorbereitet werden.",
            )
        })?
        .map(Json)
}

pub(super) async fn owned_upload(
    state: &TransferServiceState,
    id: &str,
    session: &SessionRecord,
) -> ApiResult<Arc<Mutex<UploadRecord>>> {
    let record = state.uploads.lock().await.get(id).cloned().ok_or_else(|| {
        ApiFailure::new(
            StatusCode::NOT_FOUND,
            "UPLOAD_NOT_FOUND",
            "Teilübertragung wurde nicht gefunden.",
        )
    })?;
    let record_guard = record.lock().await;
    if record_guard.cancelled {
        return Err(ApiFailure::new(
            StatusCode::NOT_FOUND,
            "UPLOAD_NOT_FOUND",
            "Teilübertragung wurde nicht gefunden.",
        ));
    }
    if record_guard.owner_session != session.id {
        return Err(ApiFailure::new(
            StatusCode::FORBIDDEN,
            "UPLOAD_OWNER_MISMATCH",
            "Diese Teilübertragung gehört zu einer anderen Sitzung.",
        ));
    }
    drop(record_guard);
    Ok(record)
}

pub(super) async fn upload_status(
    State(state): State<Arc<TransferServiceState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    id: Result<AxumPath<String>, PathRejection>,
) -> ApiResult<Json<UploadResponse>> {
    let AxumPath(id) = id.map_err(|error| {
        ApiFailure::new(
            error.status(),
            "UPLOAD_ID_INVALID",
            "Die Upload-ID ist ungültig.",
        )
    })?;
    let session = authorized(&state, &headers, client.ip(), false).await?;
    if let Some(completed) = state.completed_upload(&id, session.address).await {
        return Ok(Json(UploadResponse {
            upload_id: completed.upload_id,
            offset: completed.total_bytes,
            total_bytes: completed.total_bytes,
            chunk_size: CHUNK_SIZE,
            service_id: state.service_id.clone(),
            last_modified: completed.last_modified,
        }));
    }
    let record = owned_upload(&state, &id, &session).await?;
    let record = record.lock().await;
    Ok(Json(UploadResponse {
        upload_id: record.id.clone(),
        offset: record.offset,
        total_bytes: record.declared_size,
        chunk_size: CHUNK_SIZE,
        service_id: state.service_id.clone(),
        last_modified: record.last_modified,
    }))
}

pub(super) async fn upload_chunk(
    State(state): State<Arc<TransferServiceState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    id: Result<AxumPath<String>, PathRejection>,
    Extension(_chunk_permit): Extension<UploadChunkPermit>,
    body: Result<Bytes, BytesRejection>,
) -> ApiResult<Response> {
    let AxumPath(id) = id.map_err(|error| {
        ApiFailure::new(
            error.status(),
            "UPLOAD_ID_INVALID",
            "Die Upload-ID ist ungültig.",
        )
    })?;
    let session = authorized(&state, &headers, client.ip(), true).await?;
    let body = body.map_err(|error| {
        ApiFailure::new(
            error.status(),
            "CHUNK_BODY_INVALID",
            "Der Uploadblock konnte nicht gelesen werden.",
        )
    })?;
    if body.is_empty() {
        return Err(ApiFailure::new(
            StatusCode::BAD_REQUEST,
            "EMPTY_CHUNK",
            "Ein Uploadblock darf nicht leer sein.",
        ));
    }
    if body.len() > CHUNK_SIZE {
        return Err(ApiFailure::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "CHUNK_TOO_LARGE",
            "Der Uploadblock ist größer als erlaubt.",
        ));
    }
    let supplied_offset = headers
        .get("upload-offset")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| {
            ApiFailure::new(
                StatusCode::BAD_REQUEST,
                "OFFSET_MISSING",
                "Upload-Offset fehlt.",
            )
        })?;
    let record = owned_upload(&state, &id, &session).await?;
    ensure_session_active(&state, &session).await?;
    let (partial_file, partial_path, cancel_signal, transfer_id, next_offset) = {
        let record = record.lock().await;
        if record.cancelled {
            return Err(ApiFailure::new(
                StatusCode::NOT_FOUND,
                "UPLOAD_NOT_FOUND",
                "Teilübertragung wurde nicht gefunden.",
            ));
        }
        if record.finalizing {
            return Err(ApiFailure::new(
                StatusCode::CONFLICT,
                "UPLOAD_FINALIZING",
                "Die Datei wird bereits endgültig übernommen.",
            ));
        }
        if supplied_offset != record.offset {
            return Err(ApiFailure::new(
                StatusCode::CONFLICT,
                "OFFSET_MISMATCH",
                format!("Erwarteter Offset: {}", record.offset),
            ));
        }
        if record.offset.saturating_add(body.len() as u64) > record.declared_size {
            return Err(ApiFailure::new(
                StatusCode::BAD_REQUEST,
                "SIZE_EXCEEDED",
                "Der Upload überschreitet die angekündigte Dateigröße.",
            ));
        }
        let remaining = record.declared_size.saturating_sub(record.offset);
        let required = remaining.min(CHUNK_SIZE as u64) as usize;
        if body.len() != required {
            return Err(ApiFailure::new(
                StatusCode::BAD_REQUEST,
                "CHUNK_SIZE_INVALID",
                format!("Für diesen Uploadblock werden genau {required} Bytes erwartet."),
            ));
        }
        (
            record.partial_file.clone(),
            record.partial_path.clone(),
            record.cancel_signal.clone(),
            record.transfer_id.clone(),
            record.offset.saturating_add(body.len() as u64),
        )
    };
    let io_permit = state
        .begin_upload_io(client.ip())
        .await
        .ok_or_else(upload_io_busy)?;
    let reservation = UploadByteReservation::new(state.clone(), body.len() as u64)?;
    let task_state = state.clone();
    let task_record = record.clone();
    let write = tokio::spawn(async move {
        let mut reservation = reservation;
        let blocking_state = task_state.clone();
        let (write_result, _io_permit, _chunk_permit) = tokio::task::spawn_blocking(move || {
            let result = write_upload_chunk_blocking(
                &blocking_state,
                &partial_file,
                &partial_path,
                supplied_offset,
                &body,
                &cancel_signal,
            );
            (result, io_permit, _chunk_permit)
        })
        .await
        .map_err(|_| {
            ApiFailure::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "UPLOAD_IO_FAILED",
                "Der Uploadblock konnte nicht sicher verarbeitet werden.",
            )
        })?;
        match write_result {
            Ok(()) => {}
            Err(UploadChunkIoError::Cancelled) => {
                return Err(ApiFailure::new(
                    StatusCode::NOT_FOUND,
                    "UPLOAD_NOT_FOUND",
                    "Teilübertragung wurde abgebrochen.",
                ));
            }
            Err(UploadChunkIoError::Failed) => {
                return Err(ApiFailure::new(
                    StatusCode::INSUFFICIENT_STORAGE,
                    "PARTIAL_WRITE_FAILED",
                    "Uploadblock konnte nicht sicher gespeichert werden.",
                ));
            }
        }

        let still_active = task_state
            .uploads
            .lock()
            .await
            .get(&id)
            .is_some_and(|current| Arc::ptr_eq(current, &task_record));
        let mut current = task_record.lock().await;
        if !still_active || current.cancelled {
            return Err(ApiFailure::new(
                StatusCode::NOT_FOUND,
                "UPLOAD_NOT_FOUND",
                "Teilübertragung wurde abgebrochen.",
            ));
        }
        if current.offset != supplied_offset {
            return Err(ApiFailure::new(
                StatusCode::CONFLICT,
                "OFFSET_MISMATCH",
                format!("Erwarteter Offset: {}", current.offset),
            ));
        }
        current.offset = next_offset;
        current.last_activity = Instant::now();
        reservation.commit();
        drop(current);
        task_state
            .update_transfer(&transfer_id, next_offset, None)
            .await;
        Ok(next_offset)
    });
    let next_offset = write.await.map_err(|_| {
        ApiFailure::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "UPLOAD_IO_FAILED",
            "Der Uploadblock konnte nicht sicher verarbeitet werden.",
        )
    })??;
    let mut response = StatusCode::OK.into_response();
    response.headers_mut().insert(
        "upload-offset",
        HeaderValue::from_str(&next_offset.to_string()).unwrap(),
    );
    Ok(response)
}

pub(super) async fn finalize_upload_owned(
    state: Arc<TransferServiceState>,
    id: String,
    record: Arc<Mutex<UploadRecord>>,
    _chunk_lease: UploadChunkLease,
    io_permit: UploadIoPermit,
) -> ApiResult<CompleteResponse> {
    let result = async {
        let (
            partial_path,
            requested_name,
            transfer_id,
            total,
            owner_address,
            last_modified,
            client_token,
            cancel_signal,
            file,
            root,
            anchor,
        ) = {
            let record = record.lock().await;
            if record.cancelled {
                return Err(ApiFailure::new(
                    StatusCode::NOT_FOUND,
                    "UPLOAD_NOT_FOUND",
                    "Teilübertragung wurde nicht gefunden.",
                ));
            }
            if record.offset != record.declared_size {
                return Err(ApiFailure::new(
                    StatusCode::CONFLICT,
                    "UPLOAD_INCOMPLETE",
                    "Die Datei ist noch nicht vollständig übertragen.",
                ));
            }
            let root = state.roots.upload.as_ref().cloned().ok_or_else(|| {
                ApiFailure::new(
                    StatusCode::NOT_FOUND,
                    "UPLOAD_DISABLED",
                    "Der Upload-Eingang ist nicht aktiv.",
                )
            })?;
            let anchor = state.upload_root_anchor.clone().ok_or_else(|| {
                ApiFailure::new(
                    StatusCode::CONFLICT,
                    "UPLOAD_ROOT_CHANGED",
                    "Der Upload-Eingang wurde während des Betriebs verändert.",
                )
            })?;
            (
                record.partial_path.clone(),
                record.name.clone(),
                record.transfer_id.clone(),
                record.declared_size,
                record.owner_address,
                record.last_modified,
                record.client_token.clone(),
                record.cancel_signal.clone(),
                record.partial_file.clone(),
                root,
                anchor,
            )
        };
        let publish_name = requested_name.clone();
        let publish_path = partial_path.clone();
        let publish_cancel = cancel_signal.clone();
        let cleanup_file = file.clone();
        let (target, io_permit) = tokio::task::spawn_blocking(move || {
            let result = (|| -> ApiResult<PathBuf> {
                let actual_size = file
                    .metadata()
                    .map_err(|_| {
                        ApiFailure::new(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "PARTIAL_STAT_FAILED",
                            "Die Größe der Teilübertragung konnte nicht geprüft werden.",
                        )
                    })?
                    .len();
                if actual_size != total {
                    return Err(ApiFailure::new(
                        StatusCode::CONFLICT,
                        "FINAL_SIZE_MISMATCH",
                        "Die gespeicherte Dateigröße stimmt nicht mit dem Upload überein.",
                    ));
                }
                if publish_cancel.load(AtomicOrdering::Acquire) {
                    return Err(ApiFailure::new(
                        StatusCode::NOT_FOUND,
                        "UPLOAD_NOT_FOUND",
                        "Teilübertragung wurde abgebrochen.",
                    ));
                }
                file.sync_all().map_err(|_| {
                    ApiFailure::new(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "FINALIZE_FAILED",
                        "Datei konnte nicht endgültig übernommen werden.",
                    )
                })?;
                if publish_cancel.load(AtomicOrdering::Acquire) {
                    return Err(ApiFailure::new(
                        StatusCode::NOT_FOUND,
                        "UPLOAD_NOT_FOUND",
                        "Teilübertragung wurde abgebrochen.",
                    ));
                }
                let target =
                    publish_open_upload(&file, &publish_path, &root, &anchor, &publish_name)
                        .map_err(|_| {
                            ApiFailure::new(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "FINALIZE_FAILED",
                                "Datei konnte nicht endgültig übernommen werden.",
                            )
                        })?;
                if publish_cancel.load(AtomicOrdering::Acquire) {
                    let _ = delete_open_upload(&file, &target);
                    return Err(ApiFailure::new(
                        StatusCode::NOT_FOUND,
                        "UPLOAD_NOT_FOUND",
                        "Teilübertragung wurde abgebrochen.",
                    ));
                }
                Ok(target)
            })();
            (result, io_permit)
        })
        .await
        .map_err(|_| {
            ApiFailure::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "FINALIZE_FAILED",
                "Datei konnte nicht endgültig übernommen werden.",
            )
        })?;
        let target = target?;
        let final_name = target
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Datei")
            .to_string();
        let _filesystem = state.upload_fs_lock.lock().await;
        if cancel_signal.load(std::sync::atomic::Ordering::Acquire) {
            drop(_filesystem);
            let cleanup_target = target.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let _io_permit = io_permit;
                delete_open_upload(&cleanup_file, &cleanup_target)
            })
            .await;
            return Err(ApiFailure::new(
                StatusCode::NOT_FOUND,
                "UPLOAD_NOT_FOUND",
                "Teilübertragung wurde abgebrochen.",
            ));
        }
        let still_active = state
            .uploads
            .lock()
            .await
            .get(&id)
            .is_some_and(|current| Arc::ptr_eq(current, &record));
        if !still_active {
            drop(_filesystem);
            let cleanup_target = target.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let _io_permit = io_permit;
                delete_open_upload(&cleanup_file, &cleanup_target)
            })
            .await;
            return Err(ApiFailure::new(
                StatusCode::NOT_FOUND,
                "UPLOAD_NOT_FOUND",
                "Teilübertragung wurde abgebrochen.",
            ));
        }
        state.uploads.lock().await.remove(&id);
        state.complete_upload_budget(total);
        drop(_filesystem);
        state
            .remember_completed_upload(CompletedUpload::new(
                id.clone(),
                owner_address,
                final_name.clone(),
                requested_name,
                total,
                last_modified,
                client_token,
            ))
            .await;
        state
            .update_transfer(&transfer_id, total, Some(TransferState::Complete))
            .await;
        Ok(CompleteResponse { name: final_name })
    }
    .await;

    if result.is_err() {
        record.lock().await.finalizing = false;
    }
    result
}

pub(super) async fn complete_upload(
    State(state): State<Arc<TransferServiceState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    id: Result<AxumPath<String>, PathRejection>,
) -> ApiResult<Json<CompleteResponse>> {
    let AxumPath(id) = id.map_err(|error| {
        ApiFailure::new(
            error.status(),
            "UPLOAD_ID_INVALID",
            "Die Upload-ID ist ungültig.",
        )
    })?;
    let session = authorized(&state, &headers, client.ip(), true).await?;
    if let Some(completed) = state.completed_upload(&id, session.address).await {
        return Ok(Json(CompleteResponse {
            name: completed.name,
        }));
    }
    let filesystem = state.upload_fs_lock.lock().await;
    ensure_session_active(&state, &session).await?;
    let record = owned_upload(&state, &id, &session).await?;
    let record_guard = record.lock().await;
    if record_guard.cancelled {
        return Err(ApiFailure::new(
            StatusCode::NOT_FOUND,
            "UPLOAD_NOT_FOUND",
            "Teilübertragung wurde nicht gefunden.",
        ));
    }
    if record_guard.offset != record_guard.declared_size {
        return Err(ApiFailure::new(
            StatusCode::CONFLICT,
            "UPLOAD_INCOMPLETE",
            "Die Datei ist noch nicht vollständig übertragen.",
        ));
    }
    if record_guard.finalizing {
        return Err(ApiFailure::new(
            StatusCode::CONFLICT,
            "UPLOAD_FINALIZING",
            "Die Datei wird bereits endgültig übernommen.",
        ));
    }
    let chunk_lease = state.begin_upload_chunk(&record_guard).map_err(|_| {
        ApiFailure::new(
            StatusCode::CONFLICT,
            "UPLOAD_CHUNK_BUSY",
            "Für diese Upload-ID wird noch ein Block verarbeitet.",
        )
    })?;
    drop(record_guard);
    let io_permit = state
        .begin_upload_io(client.ip())
        .await
        .ok_or_else(upload_io_busy)?;
    let mut record_guard = record.lock().await;
    record_guard.finalizing = true;
    drop(record_guard);
    drop(filesystem);
    let task_state = state.clone();
    let task_id = id.clone();
    let task_record = record.clone();
    let commit = tokio::spawn(async move {
        finalize_upload_owned(task_state, task_id, task_record, chunk_lease, io_permit).await
    });
    match commit.await {
        Ok(result) => result.map(Json),
        Err(_) => {
            record.lock().await.finalizing = false;
            Err(ApiFailure::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "FINALIZE_FAILED",
                "Datei konnte nicht endgültig übernommen werden.",
            ))
        }
    }
}

pub(super) async fn cancel_upload(
    State(state): State<Arc<TransferServiceState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    id: Result<AxumPath<String>, PathRejection>,
) -> ApiResult<StatusCode> {
    let AxumPath(id) = id.map_err(|error| {
        ApiFailure::new(
            error.status(),
            "UPLOAD_ID_INVALID",
            "Die Upload-ID ist ungültig.",
        )
    })?;
    let session = authorized(&state, &headers, client.ip(), true).await?;
    let _cancel = state.upload_fs_lock.lock().await;
    let record = owned_upload(&state, &id, &session).await?;
    let mut record = record.lock().await;
    record.cancelled = true;
    record
        .cancel_signal
        .store(true, std::sync::atomic::Ordering::Release);
    let path = record.partial_path.clone();
    let file = record.partial_file.clone();
    let chunk_slots = record.chunk_slots.clone();
    let transfer_id = record.transfer_id.clone();
    let offset = record.offset;
    drop(record);
    state.uploads.lock().await.remove(&id);
    state.schedule_upload_delete(file, path, chunk_slots);
    state.release_upload(offset);
    state
        .update_transfer(&transfer_id, offset, Some(TransferState::Cancelled))
        .await;
    Ok(StatusCode::NO_CONTENT)
}
