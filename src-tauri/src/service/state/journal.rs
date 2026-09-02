use super::*;

const TRANSFER_EVENT_INTERVAL: Duration = Duration::from_millis(250);
pub(super) const TRANSFER_EVENT_BYTES: u64 = 1024 * 1024;

impl TransferServiceState {
    #[cfg(test)]
    pub async fn record_transfer(
        &self,
        direction: TransferDirection,
        name: &str,
        total: u64,
    ) -> String {
        let id = Uuid::new_v4().to_string();
        self.record_transfer_with_id(&id, direction, name, total)
            .await;
        id
    }

    pub async fn record_transfer_with_id(
        &self,
        id: &str,
        direction: TransferDirection,
        name: &str,
        total: u64,
    ) {
        let item = TransferInfo {
            id: id.into(),
            direction,
            name: name.into(),
            transferred_bytes: 0,
            total_bytes: total,
            state: TransferState::Active,
            updated_at: Utc::now().to_rfc3339(),
        };
        let mut transfers = self.transfers.lock().await;
        transfers.push(item.clone());
        while transfers.len() > 100 {
            let Some(index) = transfers
                .iter()
                .position(|item| item.state != TransferState::Active)
            else {
                break;
            };
            transfers.remove(index);
        }
        drop(transfers);
        self.transfer_notifications.lock().await.insert(
            id.into(),
            TransferNotification {
                transferred_bytes: 0,
                emitted_at: Instant::now(),
            },
        );
        self.emit(
            "transfer-updated",
            &TransferChangedEvent {
                service_id: self.service_id.clone(),
                transfer: item,
            },
        );
    }

    pub async fn update_transfer(&self, id: &str, bytes: u64, state: Option<TransferState>) {
        let updated = {
            let mut transfers = self.transfers.lock().await;
            transfers.iter_mut().find(|item| item.id == id).map(|item| {
                item.transferred_bytes = bytes;
                if let Some(state) = state {
                    item.state = state;
                }
                item.updated_at = Utc::now().to_rfc3339();
                item.clone()
            })
        };
        self.touch();
        let Some(updated) = updated else {
            return;
        };
        let now = Instant::now();
        let terminal = updated.state != TransferState::Active;
        let should_emit = {
            let mut notifications = self.transfer_notifications.lock().await;
            let notification =
                notifications
                    .entry(id.into())
                    .or_insert_with(|| TransferNotification {
                        transferred_bytes: 0,
                        emitted_at: now,
                    });
            let due = terminal
                || bytes.saturating_sub(notification.transferred_bytes) >= TRANSFER_EVENT_BYTES
                || now.saturating_duration_since(notification.emitted_at)
                    >= TRANSFER_EVENT_INTERVAL;
            if due {
                notification.transferred_bytes = bytes;
                notification.emitted_at = now;
            }
            if terminal {
                notifications.remove(id);
            }
            due
        };
        if should_emit {
            self.emit(
                "transfer-updated",
                &TransferChangedEvent {
                    service_id: self.service_id.clone(),
                    transfer: updated,
                },
            );
        }
    }

    pub(super) fn emit<T: serde::Serialize + Clone>(&self, event: &str, payload: &T) {
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

    pub async fn status(&self) -> ServiceStatus {
        let now = Instant::now();
        let sessions = self
            .sessions
            .lock()
            .await
            .values()
            .filter(|session| !session.expired_at(now))
            .map(SessionRecord::info)
            .collect();
        let transfers = self.transfers.lock().await.clone();
        let active_transfers = transfers
            .iter()
            .filter(|item| item.state == TransferState::Active)
            .count();
        ServiceStatus {
            state: ServiceState::Running,
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
}
