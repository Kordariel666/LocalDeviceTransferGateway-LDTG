use super::*;

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
        transfers.push(item);
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
        self.emit("transfer-updated", &serde_json::json!({ "id": id }));
    }

    pub async fn update_transfer(&self, id: &str, bytes: u64, state: Option<TransferState>) {
        if let Some(item) = self
            .transfers
            .lock()
            .await
            .iter_mut()
            .find(|item| item.id == id)
        {
            item.transferred_bytes = bytes;
            if let Some(state) = state {
                item.state = state;
            }
            item.updated_at = Utc::now().to_rfc3339();
        }
        self.touch();
        self.emit("transfer-updated", &serde_json::json!({ "id": id }));
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

    pub fn status(&self) -> ServiceStatus {
        let now = Instant::now();
        let sessions = self
            .sessions
            .try_lock()
            .map(|items| {
                items
                    .values()
                    .filter(|session| !session.expired_at(now))
                    .map(|session| SessionInfo {
                        id: session.id.clone(),
                        address: session.address.to_string(),
                        user_agent: session.user_agent.clone(),
                        created_at: session.created_at.clone(),
                        last_activity: session.last_activity.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let transfers = self
            .transfers
            .try_lock()
            .map(|items| items.clone())
            .unwrap_or_default();
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
