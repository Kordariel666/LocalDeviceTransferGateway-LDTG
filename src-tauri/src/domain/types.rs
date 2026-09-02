use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub id: String,
    pub address: String,
    pub user_agent: String,
    pub created_at: String,
    pub last_activity: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum TransferDirection {
    Upload,
    Download,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum TransferState {
    Active,
    Complete,
    Cancelled,
    Failed,
    Expired,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TransferInfo {
    pub id: String,
    pub direction: TransferDirection,
    pub name: String,
    #[ts(type = "number")]
    pub transferred_bytes: u64,
    #[ts(type = "number")]
    pub total_bytes: u64,
    pub state: TransferState,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum ServiceState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
}

impl ServiceState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stopped => "stopped",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Stopping => "stopping",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatus {
    pub state: ServiceState,
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
            state: if error.is_some() {
                ServiceState::Error
            } else {
                ServiceState::Stopped
            },
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
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

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub app_version: String,
    pub settings: super::settings::AppSettings,
    pub configuration_warning: Option<String>,
    pub service: ServiceStatus,
    pub networks: Vec<super::network::NetworkInterfaceInfo>,
    pub firewall: FirewallStatus,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
pub struct ShareValidation {
    pub download_error: Option<String>,
    pub upload_error: Option<String>,
    pub overlap_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionResponse {
    pub service_id: String,
    pub csrf_token: String,
    pub download_enabled: bool,
    pub upload_enabled: bool,
    #[ts(type = "number | null")]
    pub max_upload_bytes: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(rename = "DownloadEntryKind")]
pub enum DirectoryEntryKind {
    Directory,
    File,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "DownloadEntry")]
pub struct DirectoryEntry {
    pub name: String,
    pub path: String,
    pub kind: DirectoryEntryKind,
    #[ts(type = "number")]
    pub size: u64,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryResponse {
    pub path: String,
    pub query: String,
    pub entries: Vec<DirectoryEntry>,
    pub next_cursor: Option<String>,
    #[ts(type = "number | null")]
    pub next_page: Option<u64>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "UploadCreated")]
pub struct UploadResponse {
    pub upload_id: String,
    #[ts(type = "number")]
    pub offset: u64,
    #[ts(type = "number")]
    pub total_bytes: u64,
    pub chunk_size: usize,
    pub service_id: String,
    #[ts(type = "number")]
    pub last_modified: u64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CompleteResponse {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "ApiError")]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommandErrorCode {
    ActiveTransfers,
    BroadShare,
    DiagnosticsExportFailed,
    FirewallConfigurationFailed,
    NetworkUnavailable,
    NetworkUntrusted,
    ServiceAlreadyRunning,
    ServiceNotRunning,
    ServiceStartFailed,
    SessionNotFound,
    SettingsInvalid,
    SettingsSaveFailed,
    SharePreparationFailed,
    ShareValidationFailed,
    UnsavedChanges,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CommandErrorContext {
    NetworkApproval { token: String, network_name: String },
    BroadShareApproval { token: String, path: String },
    ActiveTransfers { count: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: CommandErrorCode,
    pub message: String,
    pub context: Option<CommandErrorContext>,
}

impl CommandError {
    pub fn new(code: CommandErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            context: None,
        }
    }

    pub fn with_context(
        code: CommandErrorCode,
        message: impl Into<String>,
        context: CommandErrorContext,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            context: Some(context),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_error_serializes_as_stable_tagged_object() {
        let error = CommandError::with_context(
            CommandErrorCode::NetworkUntrusted,
            "Dieses Netzwerk ist noch nicht als vertrauenswürdig bestätigt.",
            CommandErrorContext::NetworkApproval {
                token: "approval-token".into(),
                network_name: "WLAN".into(),
            },
        );

        assert_eq!(
            serde_json::to_value(error).unwrap(),
            serde_json::json!({
                "code": "NETWORK_UNTRUSTED",
                "message": "Dieses Netzwerk ist noch nicht als vertrauenswürdig bestätigt.",
                "context": {
                    "kind": "networkApproval",
                    "token": "approval-token",
                    "networkName": "WLAN"
                }
            })
        );
    }
}
