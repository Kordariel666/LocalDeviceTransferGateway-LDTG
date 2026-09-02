use super::*;

impl TransferServiceState {
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

    pub(super) async fn remove_directory_listings(&self, owner_session: Option<&str>) {
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
}
