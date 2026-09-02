use super::*;

const TRANSFER_EVENT_INTERVAL: Duration = Duration::from_millis(250);
pub(super) const TRANSFER_EVENT_BYTES: u64 = 1024 * 1024;
const SPEED_SMOOTHING_ALPHA: f64 = 0.25;

pub(super) fn smooth_transfer_speed(
    current: Option<f64>,
    added_bytes: u64,
    elapsed: Duration,
) -> Option<f64> {
    let elapsed = elapsed.as_secs_f64();
    if elapsed <= 0.0 || added_bytes == 0 {
        return current;
    }
    let measured = added_bytes as f64 / elapsed;
    Some(current.map_or(measured, |value| {
        value * (1.0 - SPEED_SMOOTHING_ALPHA) + measured * SPEED_SMOOTHING_ALPHA
    }))
}

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
        let now = Utc::now().to_rfc3339();
        let item = TransferInfo {
            id: id.into(),
            direction,
            name: name.into(),
            started_at: now.clone(),
            last_progress_at: None,
            transferred_bytes: 0,
            total_bytes: total,
            bytes_per_second: None,
            speed_sample_count: 0,
            state: TransferState::Active,
            updated_at: now,
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
                sampled_bytes: 0,
                sampled_at: Instant::now(),
                smoothed_bytes_per_second: None,
                speed_sample_count: 0,
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
        let now = Instant::now();
        let updated_at = Utc::now().to_rfc3339();
        let mut transfers = self.transfers.lock().await;
        let Some(item) = transfers.iter_mut().find(|item| item.id == id) else {
            return;
        };
        let previous_bytes = item.transferred_bytes;
        let terminal = state.is_some_and(|next| next != TransferState::Active);
        let mut notifications = self.transfer_notifications.lock().await;
        let notification = notifications
            .entry(id.into())
            .or_insert_with(|| TransferNotification {
                transferred_bytes: previous_bytes,
                emitted_at: now,
                sampled_bytes: previous_bytes,
                sampled_at: now,
                smoothed_bytes_per_second: item.bytes_per_second,
                speed_sample_count: item.speed_sample_count,
            });
        if bytes < notification.sampled_bytes {
            notification.sampled_bytes = bytes;
            notification.sampled_at = now;
            notification.smoothed_bytes_per_second = None;
            notification.speed_sample_count = 0;
        } else if bytes > notification.sampled_bytes {
            let elapsed = now.saturating_duration_since(notification.sampled_at);
            if !elapsed.is_zero() {
                notification.smoothed_bytes_per_second = smooth_transfer_speed(
                    notification.smoothed_bytes_per_second,
                    bytes - notification.sampled_bytes,
                    elapsed,
                );
                notification.speed_sample_count = notification.speed_sample_count.saturating_add(1);
            }
            notification.sampled_bytes = bytes;
            notification.sampled_at = now;
        }
        let should_emit = terminal
            || bytes.saturating_sub(notification.transferred_bytes) >= TRANSFER_EVENT_BYTES
            || now.saturating_duration_since(notification.emitted_at) >= TRANSFER_EVENT_INTERVAL;
        if should_emit {
            notification.transferred_bytes = bytes;
            notification.emitted_at = now;
        }
        item.transferred_bytes = bytes;
        if bytes > previous_bytes {
            item.last_progress_at = Some(updated_at.clone());
        }
        item.bytes_per_second = notification.smoothed_bytes_per_second;
        item.speed_sample_count = notification.speed_sample_count;
        if let Some(state) = state {
            item.state = state;
        }
        item.updated_at = updated_at;
        let updated = item.clone();
        if terminal {
            notifications.remove(id);
        }
        drop(notifications);
        drop(transfers);
        self.touch();
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
