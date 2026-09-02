use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub id: String,
    pub address: String,
    pub user_agent: String,
    pub created_at: String,
    pub last_activity: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferInfo {
    pub id: String,
    pub direction: String,
    pub name: String,
    pub transferred_bytes: u64,
    pub total_bytes: u64,
    pub state: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatus {
    pub state: String,
    pub service_id: Option<String>,
    pub url: Option<String>,
    pub access_code: Option<String>,
    pub started_at: Option<String>,
    pub active_transfers: usize,
    pub sessions: Vec<SessionInfo>,
    pub transfers: Vec<TransferInfo>,
    pub error: Option<String>,
}

impl ServiceStatus {
    pub fn stopped(error: Option<String>) -> Self {
        Self {
            state: if error.is_some() { "error" } else { "stopped" }.into(),
            service_id: None,
            url: None,
            access_code: None,
            started_at: None,
            active_transfers: 0,
            sessions: vec![],
            transfers: vec![],
            error,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirewallStatus {
    pub configured: bool,
    pub program_path: Option<String>,
    pub port: Option<u16>,
    pub detail: String,
}

impl Default for FirewallStatus {
    fn default() -> Self {
        Self {
            configured: false,
            program_path: None,
            port: None,
            detail: "Firewallstatus wurde noch nicht geprüft.".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub app_version: String,
    pub settings: super::settings::AppSettings,
    pub configuration_warning: Option<String>,
    pub service: ServiceStatus,
    pub networks: Vec<super::network::NetworkInterfaceInfo>,
    pub firewall: FirewallStatus,
}
