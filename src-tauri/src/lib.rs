pub mod domain;
mod platform;
mod service;

use crate::domain::{
    network,
    settings::{self, AppSettings, TrustedNetwork},
    shares,
    types::{
        AppSnapshot, CommandError, CommandErrorCode, CommandErrorContext, FirewallStatus,
        ServiceStatus, ShareValidation,
    },
};
use crate::service::ServiceHandle;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex as StdMutex,
    },
    time::{Duration, Instant},
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State, WindowEvent,
};
use tokio::sync::Mutex;
use tracing_subscriber::prelude::*;

struct RuntimeData {
    settings: AppSettings,
    configuration_warning: Option<String>,
    service: Option<ServiceHandle>,
    last_error: Option<String>,
}

struct FirewallCache {
    checked_at: Option<Instant>,
    port: u16,
    value: FirewallStatus,
}

struct NetworkCache {
    checked_at: Option<Instant>,
    value: Vec<network::NetworkInterfaceInfo>,
}

struct NetworkApproval {
    token: String,
    interface: network::NetworkInterfaceInfo,
    expires_at: Instant,
}

struct BroadShareApproval {
    token: String,
    path: String,
    expires_at: Instant,
}

fn internal_command_error<T>(
    code: CommandErrorCode,
    message: &'static str,
    operation: &'static str,
    _detail: T,
) -> CommandError {
    tracing::warn!(error_code = ?code, operation, "Tauri-Befehl fehlgeschlagen; internes Detail wird nicht protokolliert");
    CommandError::new(code, message)
}

pub struct AppState {
    settings_path: PathBuf,
    runtime: Mutex<RuntimeData>,
    lifecycle_transition: Mutex<()>,
    running: AtomicBool,
    unsaved_changes: AtomicBool,
    firewall: StdMutex<FirewallCache>,
    networks: StdMutex<NetworkCache>,
    firewall_check: Mutex<()>,
    network_check: Mutex<()>,
    network_approval: StdMutex<Option<NetworkApproval>>,
    broad_share_approval: StdMutex<Option<BroadShareApproval>>,
}

impl AppState {
    fn new(settings_path: PathBuf, loaded: settings::LoadedSettings) -> Self {
        let port = loaded
            .settings
            .runtime_settings()
            .map(|settings| settings.port)
            .unwrap_or(settings::DEFAULT_PORT);
        Self {
            settings_path,
            runtime: Mutex::new(RuntimeData {
                settings: loaded.settings,
                configuration_warning: loaded.warning,
                service: None,
                last_error: None,
            }),
            lifecycle_transition: Mutex::new(()),
            running: AtomicBool::new(false),
            unsaved_changes: AtomicBool::new(false),
            firewall: StdMutex::new(FirewallCache {
                checked_at: None,
                port,
                value: FirewallStatus::default(),
            }),
            networks: StdMutex::new(NetworkCache {
                checked_at: None,
                value: Vec::new(),
            }),
            firewall_check: Mutex::new(()),
            network_check: Mutex::new(()),
            network_approval: StdMutex::new(None),
            broad_share_approval: StdMutex::new(None),
        }
    }
}

async fn cached_networks(state: &AppState) -> Vec<network::NetworkInterfaceInfo> {
    if let Ok(cache) = state.networks.lock() {
        if cache
            .checked_at
            .is_some_and(|checked| checked.elapsed() < Duration::from_secs(8))
        {
            return cache.value.clone();
        }
    }
    let _check = state.network_check.lock().await;
    if let Ok(cache) = state.networks.lock() {
        if cache
            .checked_at
            .is_some_and(|checked| checked.elapsed() < Duration::from_secs(8))
        {
            return cache.value.clone();
        }
    }
    let checked = tauri::async_runtime::spawn_blocking(network::list_interfaces)
        .await
        .unwrap_or_default();
    if let Ok(mut cache) = state.networks.lock() {
        cache.checked_at = Some(Instant::now());
        cache.value = checked.clone();
    }
    checked
}

async fn service_state(
    state: &AppState,
) -> Option<std::sync::Arc<service::state::TransferServiceState>> {
    state
        .runtime
        .lock()
        .await
        .service
        .as_ref()
        .map(|service| service.state.clone())
}

async fn stop_runtime(state: &AppState, force: bool) -> Result<(), CommandError> {
    let _transition = state.lifecycle_transition.lock().await;
    let service = {
        let mut runtime = state.runtime.lock().await;
        let Some(service) = runtime.service.as_ref() else {
            state.running.store(false, Ordering::Relaxed);
            return Ok(());
        };
        let active_transfers = service.state.active_transfers().await;
        if !force && active_transfers > 0 {
            return Err(CommandError::with_context(
                CommandErrorCode::ActiveTransfers,
                "Mindestens eine Übertragung ist noch aktiv.",
                CommandErrorContext::ActiveTransfers {
                    count: active_transfers,
                },
            ));
        }
        runtime.service.take().expect("service was checked")
    };
    state.running.store(false, Ordering::Relaxed);
    service.stop().await;
    Ok(())
}

async fn cached_firewall(app: AppHandle, state: &AppState, port: u16) -> FirewallStatus {
    if let Ok(cache) = state.firewall.lock() {
        if cache.port == port
            && cache
                .checked_at
                .is_some_and(|checked| checked.elapsed() < Duration::from_secs(8))
        {
            return cache.value.clone();
        }
    }
    let _check = state.firewall_check.lock().await;
    if let Ok(cache) = state.firewall.lock() {
        if cache.port == port
            && cache
                .checked_at
                .is_some_and(|checked| checked.elapsed() < Duration::from_secs(8))
        {
            return cache.value.clone();
        }
    }
    let app_for_check = app.clone();
    let checked = tauri::async_runtime::spawn_blocking(move || {
        platform::firewall_status(&app_for_check, port)
    })
    .await
    .unwrap_or_else(|error| FirewallStatus {
        detail: format!("Firewallstatus konnte nicht geprüft werden: {error}"),
        ..Default::default()
    });
    if let Ok(mut cache) = state.firewall.lock() {
        cache.checked_at = Some(Instant::now());
        cache.port = port;
        cache.value = checked.clone();
    }
    checked
}

async fn current_service_status(state: &AppState) -> ServiceStatus {
    let _transition = state.lifecycle_transition.lock().await;
    let finished = state
        .runtime
        .lock()
        .await
        .service
        .as_ref()
        .is_some_and(ServiceHandle::is_finished);
    if finished {
        let handle = state.runtime.lock().await.service.take();
        if let Some(handle) = handle {
            let reason = handle.finish().await;
            if reason.as_deref() != Some("AUTO_STOP") {
                state.runtime.lock().await.last_error = reason;
            }
        }
        state.running.store(false, Ordering::Relaxed);
    }

    let (service, last_error) = {
        let runtime = state.runtime.lock().await;
        (
            runtime.service.as_ref().map(|handle| handle.state.clone()),
            runtime.last_error.clone(),
        )
    };
    match service {
        Some(service) => service.status().await,
        None => ServiceStatus::stopped(last_error),
    }
}

#[tauri::command]
async fn get_service_status(state: State<'_, AppState>) -> Result<ServiceStatus, CommandError> {
    Ok(current_service_status(&state).await)
}

#[tauri::command]
async fn clear_transfer_history(state: State<'_, AppState>) -> Result<ServiceStatus, CommandError> {
    let service = service_state(&state).await.ok_or_else(|| {
        CommandError::new(
            CommandErrorCode::ServiceNotRunning,
            "Der Dienst läuft nicht.",
        )
    })?;
    service.clear_transfer_history().await;
    Ok(service.status().await)
}

#[tauri::command]
async fn get_app_snapshot(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AppSnapshot, CommandError> {
    let service = current_service_status(&state).await;
    let (settings, configuration_warning) = {
        let runtime = state.runtime.lock().await;
        (
            runtime.settings.clone(),
            runtime.configuration_warning.clone(),
        )
    };
    let port = settings
        .runtime_settings()
        .map_err(|message| CommandError::new(CommandErrorCode::SettingsInvalid, message))?
        .port;
    let firewall = cached_firewall(app, &state, port).await;
    let networks = cached_networks(&state).await;
    Ok(AppSnapshot {
        app_version: env!("CARGO_PKG_VERSION").into(),
        settings,
        configuration_warning,
        service,
        networks,
        firewall,
    })
}

#[tauri::command]
async fn save_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, CommandError> {
    let settings = settings::normalize_for_save(settings)
        .map_err(|message| CommandError::new(CommandErrorCode::SettingsInvalid, message))?;
    let _transition = state.lifecycle_transition.lock().await;
    let mut runtime = state.runtime.lock().await;
    if runtime.service.is_some() {
        return Err(CommandError::new(
            CommandErrorCode::ServiceAlreadyRunning,
            "Einstellungen können nur bei gestopptem Dienst geändert werden.",
        ));
    }
    if runtime.configuration_warning.is_some() {
        settings::backup_for_recovery(&state.settings_path).map_err(|error| {
            internal_command_error(
                CommandErrorCode::SettingsSaveFailed,
                "Die Einstellungen konnten nicht gespeichert werden.",
                "settings recovery backup",
                error,
            )
        })?;
    }
    let settings = settings::save(&state.settings_path, &settings).map_err(|error| {
        internal_command_error(
            CommandErrorCode::SettingsSaveFailed,
            "Die Einstellungen konnten nicht gespeichert werden.",
            "settings save",
            error,
        )
    })?;
    runtime.settings = settings.clone();
    runtime.configuration_warning = None;
    state.unsaved_changes.store(false, Ordering::Release);
    Ok(settings)
}

#[tauri::command]
async fn forget_trusted_network(
    state: State<'_, AppState>,
    network_id: Option<String>,
) -> Result<AppSettings, CommandError> {
    let _transition = state.lifecycle_transition.lock().await;
    let mut runtime = state.runtime.lock().await;
    if runtime.service.is_some() {
        return Err(CommandError::new(
            CommandErrorCode::ServiceAlreadyRunning,
            "Vertrauenswürdige Netzwerke können nur bei gestopptem Dienst geändert werden.",
        ));
    }
    let mut updated = runtime.settings.clone();
    match network_id {
        Some(id) => updated.trusted_networks.retain(|network| network.id != id),
        None => updated.trusted_networks.clear(),
    }
    if updated == runtime.settings {
        return Ok(updated);
    }
    let saved = settings::save(&state.settings_path, &updated).map_err(|error| {
        internal_command_error(
            CommandErrorCode::SettingsSaveFailed,
            "Die Vertrauensliste konnte nicht gespeichert werden.",
            "trusted network removal",
            error,
        )
    })?;
    runtime.settings = saved.clone();
    Ok(saved)
}

#[tauri::command]
fn set_unsaved_changes(state: State<'_, AppState>, dirty: bool) {
    state.unsaved_changes.store(dirty, Ordering::Release);
}

#[tauri::command]
async fn validate_share_settings(settings: AppSettings) -> Result<ShareValidation, CommandError> {
    tauri::async_runtime::spawn_blocking(move || shares::validate_share_settings(&settings))
        .await
        .map_err(|error| {
            internal_command_error(
                CommandErrorCode::ShareValidationFailed,
                "Die Freigaben konnten nicht geprüft werden.",
                "share validation task",
                error,
            )
        })
}

#[tauri::command]
async fn start_service(
    app: AppHandle,
    state: State<'_, AppState>,
    network_approval: Option<String>,
    broad_share_approval: Option<String>,
) -> Result<(), CommandError> {
    let _transition = state.lifecycle_transition.lock().await;
    let settings = {
        let runtime = state.runtime.lock().await;
        if runtime.service.is_some() {
            return Err(CommandError::new(
                CommandErrorCode::ServiceAlreadyRunning,
                "Der Dienst läuft bereits.",
            ));
        }
        runtime.settings.clone()
    };
    settings
        .validate_for_start()
        .map_err(|message| CommandError::new(CommandErrorCode::SettingsInvalid, message))?;
    let runtime_settings = settings
        .runtime_settings()
        .map_err(|message| CommandError::new(CommandErrorCode::SettingsInvalid, message))?;
    let preferred_adapter = runtime_settings.preferred_adapter_id.clone();
    let interface = tauri::async_runtime::spawn_blocking(move || {
        network::select_interface(preferred_adapter.as_deref())
    })
    .await
    .map_err(|error| {
        internal_command_error(
            CommandErrorCode::NetworkUnavailable,
            "Keine geeignete private Netzwerkschnittstelle ist verfügbar.",
            "network selection task",
            error,
        )
    })?
    .map_err(|error| {
        internal_command_error(
            CommandErrorCode::NetworkUnavailable,
            "Keine geeignete private Netzwerkschnittstelle ist verfügbar.",
            "network selection",
            error,
        )
    })?;
    if !settings
        .trusted_networks
        .iter()
        .any(|network| network.id == interface.network_id)
    {
        let accepted = network_approval.as_deref().is_some_and(|token| {
            state
                .network_approval
                .lock()
                .ok()
                .and_then(|mut approval| approval.take())
                .is_some_and(|approval| {
                    approval.token == token
                        && approval.expires_at > Instant::now()
                        && network::same_network_identity(&approval.interface, &interface)
                })
        });
        if !accepted {
            let token = uuid::Uuid::new_v4().to_string();
            if let Ok(mut approval) = state.network_approval.lock() {
                *approval = Some(NetworkApproval {
                    token: token.clone(),
                    interface: interface.clone(),
                    expires_at: Instant::now() + Duration::from_secs(2 * 60),
                });
            }
            return Err(CommandError::with_context(
                CommandErrorCode::NetworkUntrusted,
                "Dieses Netzwerk ist noch nicht als vertrauenswürdig bestätigt.",
                CommandErrorContext::NetworkApproval {
                    token,
                    network_name: interface.name.clone(),
                },
            ));
        }
    }
    let share_settings = runtime_settings.clone();
    if let Some(path) = shares::broad_share_warning(&share_settings) {
        let accepted = broad_share_approval.as_deref().is_some_and(|token| {
            state
                .broad_share_approval
                .lock()
                .ok()
                .and_then(|mut approval| approval.take())
                .is_some_and(|approval| {
                    approval.token == token
                        && approval.path == path
                        && approval.expires_at > Instant::now()
                })
        });
        if !accepted {
            let token = uuid::Uuid::new_v4().to_string();
            if let Ok(mut approval) = state.broad_share_approval.lock() {
                *approval = Some(BroadShareApproval {
                    token: token.clone(),
                    path: path.clone(),
                    expires_at: Instant::now() + Duration::from_secs(2 * 60),
                });
            }
            return Err(CommandError::with_context(
                CommandErrorCode::BroadShare,
                "Eine sehr breit gewählte Freigabe muss ausdrücklich bestätigt werden.",
                CommandErrorContext::BroadShareApproval { token, path },
            ));
        }
    }
    let roots = tauri::async_runtime::spawn_blocking(move || {
        shares::prepare_roots(
            share_settings
                .download_share
                .enabled
                .then_some(share_settings.download_share.path.as_str()),
            share_settings
                .upload_share
                .enabled
                .then_some(share_settings.upload_share.path.as_str()),
        )
    })
    .await
    .map_err(|error| {
        internal_command_error(
            CommandErrorCode::SharePreparationFailed,
            "Die Freigabeordner konnten nicht sicher vorbereitet werden.",
            "share preparation task",
            error,
        )
    })?
    .map_err(|error| {
        internal_command_error(
            CommandErrorCode::SharePreparationFailed,
            "Die Freigabeordner konnten nicht sicher vorbereitet werden.",
            "share preparation",
            error,
        )
    })?;
    let mut persisted = settings.clone();
    let trusted_network = TrustedNetwork {
        id: interface.network_id.clone(),
        name: interface.profile_name.clone(),
        category: interface.category.clone(),
        last_used_at: Some(chrono::Utc::now().to_rfc3339()),
    };
    persisted.remember_trusted_network(trusted_network);
    persisted = settings::save(&state.settings_path, &persisted).map_err(|error| {
        internal_command_error(
            CommandErrorCode::SettingsSaveFailed,
            "Das bestätigte Netzwerk konnte nicht gespeichert werden.",
            "trusted network save",
            error,
        )
    })?;
    state.runtime.lock().await.settings = persisted.clone();
    let handle = service::start(runtime_settings, interface, roots, Some(app.clone()))
        .await
        .map_err(|error| {
            internal_command_error(
                CommandErrorCode::ServiceStartFailed,
                "Der Dienst konnte nicht gestartet werden.",
                "service start",
                error,
            )
        })?;
    {
        let mut runtime = state.runtime.lock().await;
        runtime.service = Some(handle);
        runtime.last_error = None;
    }
    state.running.store(true, Ordering::Relaxed);
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some("LDTG – Dienst läuft"));
    }
    let _ = app.emit(
        "service-status-changed",
        serde_json::json!({ "state": "running" }),
    );
    Ok(())
}

#[tauri::command]
async fn stop_service(
    app: AppHandle,
    state: State<'_, AppState>,
    force: bool,
) -> Result<(), CommandError> {
    stop_runtime(&state, force).await?;
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some("LDTG – Dienst gestoppt"));
    }
    let _ = app.emit(
        "service-status-changed",
        serde_json::json!({ "state": "stopped" }),
    );
    Ok(())
}

#[tauri::command]
async fn rotate_access_code(state: State<'_, AppState>) -> Result<String, CommandError> {
    service_state(&state)
        .await
        .map(|service| service.rotate_code())
        .ok_or_else(|| {
            CommandError::new(
                CommandErrorCode::ServiceNotRunning,
                "Der Dienst läuft nicht.",
            )
        })
}

#[tauri::command]
async fn revoke_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), CommandError> {
    let service = service_state(&state).await.ok_or_else(|| {
        CommandError::new(
            CommandErrorCode::ServiceNotRunning,
            "Der Dienst läuft nicht.",
        )
    })?;
    if service.revoke_session(&session_id).await {
        Ok(())
    } else {
        Err(CommandError::new(
            CommandErrorCode::SessionNotFound,
            "Die Sitzung wurde nicht gefunden.",
        ))
    }
}

#[tauri::command]
async fn revoke_all_sessions(state: State<'_, AppState>) -> Result<(), CommandError> {
    let service = service_state(&state).await.ok_or_else(|| {
        CommandError::new(
            CommandErrorCode::ServiceNotRunning,
            "Der Dienst läuft nicht.",
        )
    })?;
    service.revoke_all().await;
    Ok(())
}

#[tauri::command]
async fn configure_firewall(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<FirewallStatus, CommandError> {
    let _transition = state.lifecycle_transition.lock().await;
    let port = {
        let runtime = state.runtime.lock().await;
        if runtime.service.is_some() {
            return Err(CommandError::new(
                CommandErrorCode::ServiceAlreadyRunning,
                "Die Firewallregel kann nur bei gestopptem Dienst geändert werden.",
            ));
        }
        runtime
            .settings
            .runtime_settings()
            .map_err(|message| CommandError::new(CommandErrorCode::SettingsInvalid, message))?
            .port
    };
    if !(1024..=65535).contains(&port) {
        return Err(CommandError::new(
            CommandErrorCode::SettingsInvalid,
            "Der Port muss zwischen 1024 und 65535 liegen.",
        ));
    }
    let _check = state.firewall_check.lock().await;
    let app_for_task = app.clone();
    let checked = tauri::async_runtime::spawn_blocking(move || {
        platform::configure_firewall(&app_for_task, port)?;
        let mut checked = platform::firewall_status(&app_for_task, port);
        for _ in 0..2 {
            if checked.configured {
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
            checked = platform::firewall_status(&app_for_task, port);
        }
        Ok::<FirewallStatus, String>(checked)
    })
    .await
    .map_err(|error| {
        internal_command_error(
            CommandErrorCode::FirewallConfigurationFailed,
            "Die Firewallregel konnte nicht eingerichtet werden.",
            "firewall task",
            error,
        )
    })?
    .map_err(|error| {
        internal_command_error(
            CommandErrorCode::FirewallConfigurationFailed,
            "Die Firewallregel konnte nicht eingerichtet werden.",
            "firewall configuration",
            error,
        )
    })?;
    if let Ok(mut cache) = state.firewall.lock() {
        cache.checked_at = Some(Instant::now());
        cache.port = port;
        cache.value = checked.clone();
    }
    if checked.configured {
        Ok(checked)
    } else {
        Err(internal_command_error(
            CommandErrorCode::FirewallConfigurationFailed,
            "Die Firewallregel wurde angelegt, konnte danach aber nicht bestätigt werden.",
            "firewall verification",
            checked.detail,
        ))
    }
}

#[tauri::command]
async fn export_diagnostics(
    app: AppHandle,
    state: State<'_, AppState>,
    destination: String,
) -> Result<(), CommandError> {
    let (settings, service) = {
        let runtime = state.runtime.lock().await;
        (
            runtime.settings.clone(),
            runtime.service.as_ref().map(|handle| handle.state.clone()),
        )
    };
    let service = match service {
        Some(service) => Some(service.status().await),
        None => None,
    };
    let networks = cached_networks(&state).await;
    let effective = settings
        .runtime_settings()
        .map_err(|message| CommandError::new(CommandErrorCode::SettingsInvalid, message))?;
    let firewall = cached_firewall(app, &state, effective.port).await;
    let report = serde_json::json!({
        "createdAt": chrono::Utc::now().to_rfc3339(),
        "appVersion": env!("CARGO_PKG_VERSION"),
        "configSchema": settings::CURRENT_SETTINGS_VERSION,
        "platform": std::env::consts::OS,
        "settings": {
            "profileCount": settings.profiles.len(),
            "downloadEnabled": effective.download_share.enabled,
            "uploadEnabled": effective.upload_share.enabled,
            "sharesUseSamePath": effective.download_share.path == effective.upload_share.path,
            "preferredAdapterId": effective.preferred_adapter_id,
            "port": effective.port,
            "maxUploadBytes": effective.max_upload_bytes,
            "idleTimeoutMinutes": effective.idle_timeout_minutes,
            "trustedNetworkCount": settings.trusted_networks.len(),
        },
        "service": {
            "state": service.as_ref().map(|value| value.state.as_str()).unwrap_or("stopped"),
            "activeTransfers": service.as_ref().map(|value| value.active_transfers).unwrap_or(0),
            "sessionCount": service.as_ref().map(|value| value.sessions.len()).unwrap_or(0),
        },
        "networks": networks,
        "firewall": firewall,
        "privacy": "Keine Dateiliste, Dateiinhalte, Zugangscodes oder Sitzungstoken enthalten.",
    });
    let bytes = serde_json::to_vec_pretty(&report).map_err(|error| {
        internal_command_error(
            CommandErrorCode::DiagnosticsExportFailed,
            "Die Diagnose konnte nicht gespeichert werden.",
            "diagnostics serialization",
            error,
        )
    })?;
    tauri::async_runtime::spawn_blocking(move || std::fs::write(destination, bytes))
        .await
        .map_err(|error| {
            internal_command_error(
                CommandErrorCode::DiagnosticsExportFailed,
                "Die Diagnose konnte nicht gespeichert werden.",
                "diagnostics task",
                error,
            )
        })?
        .map_err(|error| {
            internal_command_error(
                CommandErrorCode::DiagnosticsExportFailed,
                "Die Diagnose konnte nicht gespeichert werden.",
                "diagnostics write",
                error,
            )
        })
}

#[tauri::command]
async fn quit_app(
    app: AppHandle,
    state: State<'_, AppState>,
    force: bool,
    discard_unsaved: bool,
) -> Result<(), CommandError> {
    if state.unsaved_changes.load(Ordering::Acquire) && !discard_unsaved {
        return Err(CommandError::new(
            CommandErrorCode::UnsavedChanges,
            "Es gibt ungespeicherte Änderungen.",
        ));
    }
    stop_runtime(&state, force).await?;
    app.exit(0);
    Ok(())
}

fn install_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Öffnen", true, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop", "Dienst stoppen", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "LDTG beenden", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &stop, &quit])?;
    let mut tray = TrayIconBuilder::with_id("main-tray");
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.tooltip("LDTG – Local Device Transfer Gateway")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "stop" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
                let _ = app.emit("stop-requested", ());
            }
            "quit" => {
                let state = app.state::<AppState>();
                if state.running.load(Ordering::Relaxed)
                    || state.unsaved_changes.load(Ordering::Acquire)
                {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                    let _ = app.emit("quit-requested", ());
                } else {
                    app.exit(0);
                }
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;
    Ok(())
}

fn initialize_logging(log_dir: &std::path::Path) {
    if std::fs::create_dir_all(log_dir).is_err() {
        return;
    }
    let file = tracing_appender::rolling::daily(log_dir, "ldtg.log");
    let (writer, guard) = tracing_appender::non_blocking(file);
    let _ = tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ldtg=info,tower_http=warn".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(writer),
        )
        .try_init();
    Box::leak(Box::new(guard));
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            if let Ok(log_dir) = app.path().app_log_dir() {
                initialize_logging(&log_dir);
            }
            let config_dir = app.path().app_config_dir()?;
            let settings_path = settings::settings_path(config_dir);
            let loaded = settings::load(&settings_path);
            app.manage(AppState::new(settings_path, loaded));
            install_tray(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                if state.running.load(Ordering::Relaxed) {
                    api.prevent_close();
                    let _ = window.hide();
                } else if state.unsaved_changes.load(Ordering::Acquire) {
                    api.prevent_close();
                    let _ = window.emit("quit-requested", ());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_app_snapshot,
            get_service_status,
            clear_transfer_history,
            save_settings,
            forget_trusted_network,
            set_unsaved_changes,
            validate_share_settings,
            start_service,
            stop_service,
            rotate_access_code,
            revoke_session,
            revoke_all_sessions,
            configure_firewall,
            export_diagnostics,
            quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("LDTG konnte nicht gestartet werden");
}

#[cfg(test)]
mod capability_tests {
    use super::{internal_command_error, settings, stop_runtime, AppState};
    use crate::domain::types::CommandErrorCode;
    use std::sync::Arc;
    use tokio::time::{timeout, Duration};

    #[test]
    fn desktop_capability_allows_every_used_plugin_command() {
        let capability: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/default.json"))
                .expect("desktop capability must be valid JSON");
        let permissions = capability["permissions"]
            .as_array()
            .expect("permissions must be an array");
        for required in [
            "dialog:allow-message",
            "dialog:allow-open",
            "dialog:allow-save",
            "notification:default",
        ] {
            assert!(
                permissions.iter().any(|value| value == required),
                "missing capability permission: {required}"
            );
        }
    }

    #[test]
    fn internal_command_errors_do_not_expose_raw_details() {
        let error = internal_command_error(
            CommandErrorCode::DiagnosticsExportFailed,
            "Die Diagnose konnte nicht gespeichert werden.",
            "diagnostics write",
            r"C:\private\diagnose.json: Zugriff verweigert",
        );
        let serialized = serde_json::to_string(&error).unwrap();

        assert!(serialized.contains("DIAGNOSTICS_EXPORT_FAILED"));
        assert!(serialized.contains("Die Diagnose konnte nicht gespeichert werden."));
        assert!(!serialized.contains("private"));
        assert!(!serialized.contains("Zugriff verweigert"));
    }

    #[tokio::test]
    async fn service_stop_waits_for_the_active_lifecycle_transition() {
        let temporary = tempfile::tempdir().expect("temporary settings directory");
        let state = Arc::new(AppState::new(
            temporary.path().join("settings.json"),
            settings::LoadedSettings {
                settings: Default::default(),
                warning: None,
            },
        ));
        let transition = state.lifecycle_transition.lock().await;
        let stop_state = state.clone();
        let stop = tokio::spawn(async move { stop_runtime(&stop_state, true).await });

        tokio::task::yield_now().await;
        assert!(
            !stop.is_finished(),
            "stop must not pass an active start/reap/save transition"
        );

        drop(transition);
        timeout(Duration::from_secs(1), stop)
            .await
            .expect("stop must continue after the transition finishes")
            .expect("stop task must not panic")
            .expect("stopping an already stopped service succeeds");
    }

    #[test]
    fn uninstaller_never_recursively_deletes_application_data() {
        let hook = include_str!("../windows/hooks.nsh");
        assert!(!hook.contains("RMDir /r"));
        assert!(!hook.contains("$APPDATA\\de.ldtg.desktop"));
        assert!(!hook.contains("$LOCALAPPDATA\\de.ldtg.desktop"));
    }
}
