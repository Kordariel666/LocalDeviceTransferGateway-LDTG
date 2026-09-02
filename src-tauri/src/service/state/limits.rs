use super::*;

impl TransferServiceState {
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

    pub async fn begin_upload_io(&self, address: IpAddr) -> Option<UploadIoPermit> {
        let global = self.upload_io_slots.clone().try_acquire_owned().ok()?;
        let address_slots = {
            let mut slots = self.upload_io_address_slots.lock().await;
            slots.retain(|_, slots| slots.strong_count() > 0);
            if let Some(existing) = slots.get(&address).and_then(Weak::upgrade) {
                existing
            } else {
                let created = Arc::new(Semaphore::new(MAX_UPLOAD_IO_PER_ADDRESS));
                slots.insert(address, Arc::downgrade(&created));
                created
            }
        };
        let address = address_slots.try_acquire_owned().ok()?;
        Some(UploadIoPermit {
            _global: global,
            _address: address,
        })
    }
}
