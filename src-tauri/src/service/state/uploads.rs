use super::*;

impl TransferServiceState {
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

    #[cfg(test)]
    pub fn active_upload_bytes_for_test(&self) -> u64 {
        self.inbox_usage
            .lock()
            .expect("inbox usage lock poisoned")
            .active_bytes
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

    pub fn schedule_upload_delete(
        &self,
        file: Arc<fs::File>,
        path: PathBuf,
        chunk_slots: Arc<Semaphore>,
    ) {
        let upload_io_slots = self.upload_io_slots.clone();
        tokio::spawn(async move {
            let Ok(chunk_permit) = chunk_slots.acquire_owned().await else {
                return;
            };
            let Ok(io_permit) = upload_io_slots.acquire_owned().await else {
                return;
            };
            let _ = tokio::task::spawn_blocking(move || {
                let _chunk_permit = chunk_permit;
                let _io_permit = io_permit;
                delete_open_upload(&file, &path)
            })
            .await;
        });
    }

    pub(super) async fn cancel_uploads(&self, owner: Option<&str>) {
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
            let file = record.partial_file.clone();
            let chunk_slots = record.chunk_slots.clone();
            let transfer_id = record.transfer_id.clone();
            let offset = record.offset;
            drop(record);
            self.schedule_upload_delete(file, path, chunk_slots);
            self.release_upload(offset);
            self.update_transfer(&transfer_id, offset, Some(TransferState::Cancelled))
                .await;
        }
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
            let file = record.partial_file.clone();
            let chunk_slots = record.chunk_slots.clone();
            let transfer_id = record.transfer_id.clone();
            let offset = record.offset;
            drop(record);
            self.schedule_upload_delete(file, path, chunk_slots);
            self.release_upload(offset);
            self.update_transfer(&transfer_id, offset, Some(TransferState::Expired))
                .await;
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
            self.update_transfer(&id, 0, Some(TransferState::Cancelled))
                .await;
        }
        if let Some(partial_dir) = self.partial_dir.clone() {
            let _ = tokio::task::spawn_blocking(move || cleanup_owned_partials(&partial_dir)).await;
        }
    }
}
