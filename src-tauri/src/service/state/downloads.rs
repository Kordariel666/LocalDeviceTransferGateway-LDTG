use super::*;

impl TransferServiceState {
    pub(super) async fn cancel_downloads(&self, owner: Option<&str>) {
        let downloads = self.downloads.lock().await;
        for download in downloads.values() {
            if owner.is_none_or(|owner| owner == download.owner_session) {
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
        self.record_transfer_with_id(&id, owner_session, TransferDirection::Download, name, total)
            .await;
        Ok(DownloadLease {
            id,
            cancel: receiver,
            started_at: Instant::now(),
            _permit: permit,
        })
    }

    pub async fn finish_download(&self, id: &str, bytes: u64, state: TransferState) {
        self.downloads.lock().await.remove(id);
        self.update_transfer(id, bytes, Some(state)).await;
    }
}
