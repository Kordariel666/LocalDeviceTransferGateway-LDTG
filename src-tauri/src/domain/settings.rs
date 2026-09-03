use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::HashSet,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};
use ts_rs::TS;

pub const DEFAULT_PORT: u16 = 8765;
pub const DEFAULT_MAX_UPLOAD: u64 = 20 * 1024 * 1024 * 1024;
pub const DEFAULT_MAX_INBOX_BYTES: u64 = 100 * 1024 * 1024 * 1024;
pub const DEFAULT_MAX_INBOX_FILES: u32 = 10_000;
pub const CURRENT_SETTINGS_VERSION: u32 = 4;
pub const DEFAULT_PROFILE_ID: &str = "00000000-0000-4000-8000-000000000001";
const MAX_TRUSTED_NETWORKS: usize = 256;
const MAX_PROFILES: usize = 32;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
pub struct ShareSettings {
    pub enabled: bool,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct LimitSettings {
    #[ts(type = "number | null")]
    pub max_upload_bytes: Option<u64>,
    #[ts(type = "number")]
    pub max_inbox_bytes: u64,
    pub max_inbox_files: u32,
    pub idle_timeout_minutes: Option<u32>,
}

impl Default for LimitSettings {
    fn default() -> Self {
        Self {
            max_upload_bytes: Some(DEFAULT_MAX_UPLOAD),
            max_inbox_bytes: DEFAULT_MAX_INBOX_BYTES,
            max_inbox_files: DEFAULT_MAX_INBOX_FILES,
            idle_timeout_minutes: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct NetworkSettings {
    pub preferred_adapter_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct ProfileOverrides {
    pub network: Option<NetworkSettings>,
    pub port: Option<u16>,
    pub limits: Option<LimitSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct ShareProfile {
    pub id: String,
    pub name: String,
    pub download_share: ShareSettings,
    pub upload_share: ShareSettings,
    pub overrides: ProfileOverrides,
}

impl Default for ShareProfile {
    fn default() -> Self {
        Self {
            id: DEFAULT_PROFILE_ID.into(),
            name: "Standard".into(),
            download_share: ShareSettings::default(),
            upload_share: ShareSettings::default(),
            overrides: ProfileOverrides::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
pub struct TrustedNetwork {
    pub id: String,
    pub name: String,
    pub category: String,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub version: u32,
    pub profiles: Vec<ShareProfile>,
    pub active_profile_id: String,
    pub preferred_adapter_id: Option<String>,
    pub port: u16,
    #[ts(type = "number | null")]
    pub max_upload_bytes: Option<u64>,
    #[ts(type = "number")]
    pub max_inbox_bytes: u64,
    pub max_inbox_files: u32,
    pub idle_timeout_minutes: Option<u32>,
    pub trusted_networks: Vec<TrustedNetwork>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            version: CURRENT_SETTINGS_VERSION,
            profiles: vec![ShareProfile::default()],
            active_profile_id: DEFAULT_PROFILE_ID.into(),
            preferred_adapter_id: None,
            port: DEFAULT_PORT,
            max_upload_bytes: Some(DEFAULT_MAX_UPLOAD),
            max_inbox_bytes: DEFAULT_MAX_INBOX_BYTES,
            max_inbox_files: DEFAULT_MAX_INBOX_FILES,
            idle_timeout_minutes: None,
            trusted_networks: vec![],
        }
    }
}

impl AppSettings {
    pub fn active_profile(&self) -> Result<&ShareProfile, String> {
        self.profiles
            .iter()
            .find(|profile| profile.id == self.active_profile_id)
            .ok_or_else(|| "Das aktive Freigabeprofil existiert nicht.".into())
    }

    pub fn runtime_settings(&self) -> Result<RuntimeSettings, String> {
        let profile = self.active_profile()?;
        let limits = profile.overrides.limits.as_ref();
        Ok(RuntimeSettings {
            download_share: profile.download_share.clone(),
            upload_share: profile.upload_share.clone(),
            preferred_adapter_id: profile
                .overrides
                .network
                .as_ref()
                .map(|settings| settings.preferred_adapter_id.clone())
                .unwrap_or_else(|| self.preferred_adapter_id.clone()),
            port: profile.overrides.port.unwrap_or(self.port),
            max_upload_bytes: limits
                .map(|settings| settings.max_upload_bytes)
                .unwrap_or(self.max_upload_bytes),
            max_inbox_bytes: limits
                .map(|settings| settings.max_inbox_bytes)
                .unwrap_or(self.max_inbox_bytes),
            max_inbox_files: limits
                .map(|settings| settings.max_inbox_files)
                .unwrap_or(self.max_inbox_files),
            idle_timeout_minutes: limits
                .map(|settings| settings.idle_timeout_minutes)
                .unwrap_or(self.idle_timeout_minutes),
        })
    }

    pub fn remember_trusted_network(&mut self, network: TrustedNetwork) {
        self.trusted_networks
            .retain(|existing| existing.id != network.id);
        self.trusted_networks.push(network);
        if self.trusted_networks.len() > MAX_TRUSTED_NETWORKS {
            self.trusted_networks.remove(0);
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_port(self.port)?;
        validate_limits(
            self.max_upload_bytes,
            self.max_inbox_bytes,
            self.max_inbox_files,
        )?;
        if self.profiles.is_empty() || self.profiles.len() > MAX_PROFILES {
            return Err("Es muss zwischen einem und 32 Freigabeprofilen geben.".into());
        }
        let mut profile_ids = HashSet::new();
        let mut profile_names = HashSet::new();
        for profile in &self.profiles {
            if uuid::Uuid::parse_str(&profile.id).is_err() || !profile_ids.insert(&profile.id) {
                return Err("Eine Profil-ID ist ungültig oder mehrfach gespeichert.".into());
            }
            let name = profile.name.trim();
            if name.is_empty()
                || name.chars().count() > 64
                || name.chars().any(profile_name_character_is_unsafe)
            {
                return Err("Ein Profilname ist ungültig.".into());
            }
            if !profile_names.insert(name.to_lowercase()) {
                return Err("Ein Profilname ist mehrfach gespeichert.".into());
            }
            if let Some(port) = profile.overrides.port {
                validate_port(port)?;
            }
            if let Some(limits) = &profile.overrides.limits {
                validate_limits(
                    limits.max_upload_bytes,
                    limits.max_inbox_bytes,
                    limits.max_inbox_files,
                )?;
            }
            if profile
                .overrides
                .network
                .as_ref()
                .and_then(|network| network.preferred_adapter_id.as_deref())
                .is_some_and(|id| id.trim().is_empty() || id.len() > 1_024)
            {
                return Err("Eine Netzwerkschnittstellen-ID im Profil ist ungültig.".into());
            }
        }
        self.active_profile()?;
        if self.trusted_networks.len() > MAX_TRUSTED_NETWORKS {
            return Err("Es sind zu viele vertrauenswürdige Netzwerke gespeichert.".into());
        }
        let mut network_ids = HashSet::new();
        for network in &self.trusted_networks {
            if network.id.trim().is_empty() || network.id.len() > 1_024 {
                return Err("Eine gespeicherte Netzwerk-ID ist ungültig.".into());
            }
            if !network_ids.insert(network.id.as_str()) {
                return Err("Eine Netzwerk-ID ist mehrfach gespeichert.".into());
            }
            if network.name.trim().is_empty() || network.name.len() > 512 {
                return Err("Ein gespeicherter Netzwerkname ist ungültig.".into());
            }
            if network.category.trim().is_empty() || network.category.len() > 128 {
                return Err("Eine gespeicherte Netzwerkkategorie ist ungültig.".into());
            }
            if network
                .last_used_at
                .as_deref()
                .is_some_and(|value| chrono::DateTime::parse_from_rfc3339(value).is_err())
            {
                return Err("Der Zeitpunkt der letzten Netzwerkverwendung ist ungültig.".into());
            }
        }
        Ok(())
    }

    pub fn validate_for_start(&self) -> Result<(), String> {
        self.validate()?;
        let runtime = self.runtime_settings()?;
        if !runtime.download_share.enabled && !runtime.upload_share.enabled {
            return Err("Mindestens eine Freigabe muss aktiviert sein.".into());
        }
        if runtime.download_share.enabled && runtime.download_share.path.trim().is_empty() {
            return Err("Für Downloads fehlt ein Ordner.".into());
        }
        if runtime.upload_share.enabled && runtime.upload_share.path.trim().is_empty() {
            return Err("Für Uploads fehlt ein Eingangsordner.".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSettings {
    pub download_share: ShareSettings,
    pub upload_share: ShareSettings,
    pub preferred_adapter_id: Option<String>,
    pub port: u16,
    pub max_upload_bytes: Option<u64>,
    pub max_inbox_bytes: u64,
    pub max_inbox_files: u32,
    pub idle_timeout_minutes: Option<u32>,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        AppSettings::default()
            .runtime_settings()
            .expect("default settings contain the default profile")
    }
}

fn validate_port(port: u16) -> Result<(), String> {
    if !(1024..=65535).contains(&port) {
        return Err("Der Port muss zwischen 1024 und 65535 liegen.".into());
    }
    Ok(())
}

fn validate_limits(
    max_upload_bytes: Option<u64>,
    max_inbox_bytes: u64,
    max_inbox_files: u32,
) -> Result<(), String> {
    if matches!(max_upload_bytes, Some(0)) {
        return Err("Das Uploadlimit muss größer als null sein.".into());
    }
    if max_inbox_bytes == 0 {
        return Err("Das Gesamtlimit des Upload-Eingangs muss größer als null sein.".into());
    }
    if max_inbox_files == 0 {
        return Err("Das Dateilimit des Upload-Eingangs muss größer als null sein.".into());
    }
    if max_upload_bytes.is_some_and(|limit| limit > max_inbox_bytes) {
        return Err("Das Uploadlimit pro Datei darf das Gesamtlimit des Upload-Eingangs nicht überschreiten.".into());
    }
    Ok(())
}

fn profile_name_character_is_unsafe(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            '\u{061c}'
                | '\u{200e}'
                | '\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}'
        )
}

pub struct LoadedSettings {
    pub settings: AppSettings,
    pub warning: Option<String>,
}

fn version_from(object: &Map<String, Value>) -> Result<u32, String> {
    let Some(value) = object.get("version") else {
        return Ok(0);
    };
    let version = value
        .as_u64()
        .ok_or_else(|| "Das Feld version muss eine nichtnegative ganze Zahl sein.".to_string())?;
    u32::try_from(version).map_err(|_| "Die Konfigurationsversion ist zu groß.".to_string())
}

fn insert_missing(object: &mut Map<String, Value>, key: &str, value: Value) {
    object.entry(key.to_string()).or_insert(value);
}

fn migrate_v0_to_v1(object: &mut Map<String, Value>) {
    insert_missing(
        object,
        "downloadShare",
        serde_json::json!({ "enabled": false, "path": "" }),
    );
    insert_missing(
        object,
        "uploadShare",
        serde_json::json!({ "enabled": false, "path": "" }),
    );
    insert_missing(object, "preferredAdapterId", Value::Null);
    insert_missing(object, "port", Value::from(DEFAULT_PORT));
    insert_missing(object, "maxUploadBytes", Value::from(DEFAULT_MAX_UPLOAD));
    insert_missing(
        object,
        "maxInboxBytes",
        Value::from(DEFAULT_MAX_INBOX_BYTES),
    );
    insert_missing(
        object,
        "maxInboxFiles",
        Value::from(DEFAULT_MAX_INBOX_FILES),
    );
    insert_missing(object, "idleTimeoutMinutes", Value::Null);
    insert_missing(object, "trustedNetworks", Value::Array(vec![]));
    object.insert("version".into(), Value::from(1));
}

fn migrate_v1_to_v2(object: &mut Map<String, Value>) {
    object.remove("uiVersion");
    object.insert("version".into(), Value::from(2));
}

fn migrate_v2_to_v3(object: &mut Map<String, Value>) -> Result<(), String> {
    let legacy = object
        .remove("trustedNetworks")
        .unwrap_or_else(|| Value::Array(vec![]));
    let values = legacy
        .as_array()
        .ok_or_else(|| "Das Feld trustedNetworks muss eine Liste sein.".to_string())?;
    let mut seen = HashSet::new();
    let mut migrated = Vec::new();
    for value in values {
        let id = value.as_str().ok_or_else(|| {
            "Version 2 erwartet Netzwerk-IDs als Text in trustedNetworks.".to_string()
        })?;
        if id.trim().is_empty() || !seen.insert(id) {
            continue;
        }
        migrated.push(serde_json::json!({
            "id": id,
            "name": format!("Migriertes Netzwerk {}", migrated.len() + 1),
            "category": "Unbekannt",
            "lastUsedAt": null
        }));
    }
    object.insert("trustedNetworks".into(), Value::Array(migrated));
    object.insert("version".into(), Value::from(3));
    Ok(())
}

fn migrate_v3_to_v4(object: &mut Map<String, Value>) {
    let download_share = object
        .remove("downloadShare")
        .unwrap_or_else(|| serde_json::json!({ "enabled": false, "path": "" }));
    let upload_share = object
        .remove("uploadShare")
        .unwrap_or_else(|| serde_json::json!({ "enabled": false, "path": "" }));
    object.insert(
        "profiles".into(),
        serde_json::json!([{
            "id": DEFAULT_PROFILE_ID,
            "name": "Standard",
            "downloadShare": download_share,
            "uploadShare": upload_share,
            "overrides": {}
        }]),
    );
    object.insert(
        "activeProfileId".into(),
        Value::String(DEFAULT_PROFILE_ID.into()),
    );
    object.insert("version".into(), Value::from(4));
}

fn migrate_value(mut value: Value) -> Result<AppSettings, String> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| "Die Wurzel der Einstellungsdatei muss ein JSON-Objekt sein.".to_string())?;
    let mut version = version_from(object)?;
    if version > CURRENT_SETTINGS_VERSION {
        return Err(format!(
            "Konfigurationsschema {version} ist neuer als das unterstützte Schema {CURRENT_SETTINGS_VERSION}."
        ));
    }
    while version < CURRENT_SETTINGS_VERSION {
        version = match version {
            0 => {
                migrate_v0_to_v1(object);
                1
            }
            1 => {
                migrate_v1_to_v2(object);
                2
            }
            2 => {
                migrate_v2_to_v3(object)?;
                3
            }
            3 => {
                migrate_v3_to_v4(object);
                4
            }
            _ => {
                return Err(format!(
                    "Konfigurationsschema {version} kann nicht migriert werden."
                ))
            }
        };
    }
    let settings: AppSettings = serde_json::from_value(value)
        .map_err(|error| format!("Die Einstellungsfelder sind ungültig: {error}"))?;
    settings
        .validate()
        .map_err(|error| format!("Die Einstellungen sind semantisch ungültig: {error}"))?;
    Ok(settings)
}

fn unusable_settings(reason: impl std::fmt::Display) -> LoadedSettings {
    LoadedSettings {
        settings: AppSettings::default(),
        warning: Some(format!(
            "Die gespeicherten Einstellungen sind nicht verwendbar ({reason}). Sichere Standardwerte sind aktiv; die vorhandene settings.json wurde unverändert behalten."
        )),
    }
}

pub fn load(path: &Path) -> LoadedSettings {
    match fs::read_to_string(path) {
        Ok(value) => match serde_json::from_str(&value)
            .map_err(|error| error.to_string())
            .and_then(migrate_value)
        {
            Ok(settings) => LoadedSettings {
                settings,
                warning: None,
            },
            Err(error) => unusable_settings(error),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => LoadedSettings {
            settings: AppSettings::default(),
            warning: None,
        },
        Err(error) => LoadedSettings {
            settings: AppSettings::default(),
            warning: Some(format!(
                "Die gespeicherten Einstellungen konnten nicht gelesen werden ({error}). Sichere Standardwerte sind aktiv; die vorhandene settings.json wurde unverändert behalten."
            )),
        },
    }
}

pub fn normalize_for_save(mut settings: AppSettings) -> Result<AppSettings, String> {
    if settings.version > CURRENT_SETTINGS_VERSION {
        return Err(format!(
            "Konfigurationsschema {} ist neuer als das unterstützte Schema {CURRENT_SETTINGS_VERSION}.",
            settings.version
        ));
    }
    settings.version = CURRENT_SETTINGS_VERSION;
    settings.validate()?;
    Ok(settings)
}

pub fn save(path: &Path, settings: &AppSettings) -> Result<AppSettings, String> {
    let settings = normalize_for_save(settings.clone())?;
    let parent = path
        .parent()
        .ok_or_else(|| "Ungültiger Konfigurationspfad.".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let data = serde_json::to_vec_pretty(&settings).map_err(|error| error.to_string())?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
    temporary
        .write_all(&data)
        .map_err(|error| error.to_string())?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|error| error.to_string())?;
    temporary
        .persist(path)
        .map_err(|error| error.error.to_string())?;
    Ok(settings)
}

pub fn backup_for_recovery(path: &Path) -> Result<Option<PathBuf>, String> {
    let mut source = match fs::File::open(path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "Die beschädigten Einstellungen konnten vor dem Ersetzen nicht gesichert werden: {error}"
            ))
        }
    };
    let parent = path
        .parent()
        .ok_or_else(|| "Ungültiger Konfigurationspfad.".to_string())?;
    for index in 1..=100 {
        let candidate = parent.join(format!("settings.recovery-{index}.json"));
        let mut target = match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(target) => target,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "Die Wiederherstellungskopie der Einstellungen konnte nicht angelegt werden: {error}"
                ))
            }
        };
        if let Err(error) = io::copy(&mut source, &mut target).and_then(|_| target.sync_all()) {
            drop(target);
            let _ = fs::remove_file(&candidate);
            return Err(format!(
                "Die Wiederherstellungskopie der Einstellungen konnte nicht geschrieben werden: {error}"
            ));
        }
        return Ok(Some(candidate));
    }
    Err("Es existieren bereits zu viele Wiederherstellungskopien der Einstellungen.".into())
}

pub fn settings_path(config_dir: PathBuf) -> PathBuf {
    config_dir.join("settings.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_loads_versioned_settings() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("settings.json");
        let expected = AppSettings {
            port: 9000,
            ..Default::default()
        };
        let saved = save(&path, &expected).unwrap();
        let loaded = load(&path);
        assert!(loaded.warning.is_none());
        assert_eq!(saved.version, CURRENT_SETTINGS_VERSION);
        assert_eq!(loaded.settings.version, CURRENT_SETTINGS_VERSION);
        assert_eq!(loaded.settings.port, 9000);
        assert!(!fs::read_to_string(path).unwrap().contains("uiVersion"));
    }

    #[test]
    fn migrates_unversioned_settings_with_missing_fields() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("settings.json");
        fs::write(&path, r#"{"port":9123}"#).unwrap();

        let loaded = load(&path);

        assert!(loaded.warning.is_none());
        assert_eq!(loaded.settings.version, CURRENT_SETTINGS_VERSION);
        assert_eq!(loaded.settings.port, 9123);
        assert_eq!(loaded.settings.max_inbox_bytes, DEFAULT_MAX_INBOX_BYTES);
        assert_eq!(loaded.settings.max_inbox_files, DEFAULT_MAX_INBOX_FILES);
    }

    #[test]
    fn migrates_v1_stepwise_and_idempotently() {
        let v1 = serde_json::json!({
            "version": 1,
            "uiVersion": "0.1.0",
            "port": 9124
        });

        let migrated = migrate_value(v1).unwrap();
        let migrated_again = migrate_value(serde_json::to_value(&migrated).unwrap()).unwrap();

        assert_eq!(migrated, migrated_again);
        assert_eq!(migrated.version, CURRENT_SETTINGS_VERSION);
        assert_eq!(migrated.port, 9124);
        assert!(serde_json::to_value(migrated)
            .unwrap()
            .get("uiVersion")
            .is_none());
    }

    #[test]
    fn migrates_v2_network_ids_without_losing_stable_identity() {
        let v2 = serde_json::json!({
            "version": 2,
            "trustedNetworks": ["{guid}|192.168.1.0/24", "{guid}|192.168.1.0/24", "{other}|10.0.0.0/24"]
        });

        let migrated = migrate_value(v2).unwrap();
        let migrated_again = migrate_value(serde_json::to_value(&migrated).unwrap()).unwrap();

        assert_eq!(migrated, migrated_again);
        assert_eq!(migrated.version, CURRENT_SETTINGS_VERSION);
        assert_eq!(migrated.trusted_networks.len(), 2);
        assert_eq!(migrated.trusted_networks[0].id, "{guid}|192.168.1.0/24");
        assert_eq!(migrated.trusted_networks[0].name, "Migriertes Netzwerk 1");
        assert_eq!(migrated.trusted_networks[0].category, "Unbekannt");
        assert_eq!(migrated.trusted_networks[0].last_used_at, None);
    }

    #[test]
    fn remembers_network_metadata_by_stable_id_without_duplicates() {
        let mut settings = AppSettings::default();
        settings.remember_trusted_network(TrustedNetwork {
            id: "stable-network-id".into(),
            name: "Altes Profil".into(),
            category: "Öffentlich".into(),
            last_used_at: Some("2026-09-01T10:00:00Z".into()),
        });
        settings.remember_trusted_network(TrustedNetwork {
            id: "stable-network-id".into(),
            name: "Heimnetz".into(),
            category: "Privat".into(),
            last_used_at: Some("2026-09-03T10:00:00Z".into()),
        });

        assert_eq!(settings.trusted_networks.len(), 1);
        assert_eq!(settings.trusted_networks[0].name, "Heimnetz");
        assert_eq!(settings.trusted_networks[0].category, "Privat");
        assert_eq!(
            settings.trusted_networks[0].last_used_at.as_deref(),
            Some("2026-09-03T10:00:00Z")
        );
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn rejects_future_schema_without_replacing_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("settings.json");
        let original = br#"{"version":999,"port":9123}"#;
        fs::write(&path, original).unwrap();

        let loaded = load(&path);

        assert!(loaded.warning.unwrap().contains("Schema 4"));
        assert_eq!(loaded.settings, AppSettings::default());
        assert_eq!(fs::read(path).unwrap(), original);
    }

    #[test]
    fn save_rejects_future_schema() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("settings.json");
        let settings = AppSettings {
            version: CURRENT_SETTINGS_VERSION + 1,
            ..Default::default()
        };

        assert!(save(&path, &settings).is_err());
        assert!(!path.exists());
    }

    #[test]
    fn permits_draft_without_share_but_not_start() {
        let settings = AppSettings::default();
        assert!(settings.validate().is_ok());
        assert!(settings.validate_for_start().is_err());
    }

    #[test]
    fn missing_settings_use_defaults_without_warning() {
        let temp = tempfile::tempdir().unwrap();
        let loaded = load(&temp.path().join("missing.json"));
        assert!(loaded.warning.is_none());
        assert_eq!(loaded.settings.port, DEFAULT_PORT);
    }

    #[test]
    fn corrupt_settings_are_preserved_and_reported() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("settings.json");
        fs::write(&path, b"{not-json").unwrap();
        let loaded = load(&path);
        assert!(loaded.warning.is_some());
        assert_eq!(loaded.settings.port, DEFAULT_PORT);
        assert_eq!(fs::read(&path).unwrap(), b"{not-json");
    }

    #[test]
    fn corrupt_settings_are_backed_up_before_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("settings.json");
        fs::write(&path, b"{not-json").unwrap();
        let backup = backup_for_recovery(&path).unwrap().unwrap();
        save(&path, &AppSettings::default()).unwrap();
        assert_eq!(fs::read(backup).unwrap(), b"{not-json");
        assert!(load(&path).warning.is_none());
    }

    #[test]
    fn semantically_invalid_settings_are_backed_up_before_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("settings.json");
        let original = br#"{"version":2,"port":80}"#;
        fs::write(&path, original).unwrap();

        let loaded = load(&path);
        assert!(loaded.warning.unwrap().contains("semantisch ungültig"));
        let backup = backup_for_recovery(&path).unwrap().unwrap();
        save(&path, &loaded.settings).unwrap();

        assert_eq!(fs::read(backup).unwrap(), original);
        assert!(load(&path).warning.is_none());
    }

    #[test]
    fn syntactically_valid_settings_with_corrupt_values_are_preserved() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("settings.json");
        let original = br#"{"version":2,"port":"kaputt"}"#;
        fs::write(&path, original).unwrap();

        let loaded = load(&path);

        assert!(loaded.warning.is_some());
        assert_eq!(loaded.settings, AppSettings::default());
        assert_eq!(fs::read(path).unwrap(), original);
    }

    #[test]
    fn unreadable_settings_path_is_reported_without_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("settings.json");
        fs::create_dir(&path).unwrap();
        let loaded = load(&path);
        assert!(loaded.warning.is_some());
        assert!(path.is_dir());
    }

    #[test]
    fn migrates_v3_shares_into_a_default_profile_without_changing_values() {
        let v3 = serde_json::json!({
            "version": 3,
            "downloadShare": { "enabled": true, "path": "C:\\Freigabe" },
            "uploadShare": { "enabled": true, "path": "D:\\Eingang" },
            "preferredAdapterId": "lan-id",
            "port": 9123,
            "maxUploadBytes": 5_000,
            "maxInboxBytes": 10_000,
            "maxInboxFiles": 200,
            "idleTimeoutMinutes": 30,
            "trustedNetworks": []
        });

        let migrated = migrate_value(v3).unwrap();
        let effective = migrated.runtime_settings().unwrap();

        assert_eq!(migrated.version, CURRENT_SETTINGS_VERSION);
        assert_eq!(migrated.active_profile_id, DEFAULT_PROFILE_ID);
        assert_eq!(migrated.profiles.len(), 1);
        assert_eq!(migrated.profiles[0].name, "Standard");
        assert!(effective.download_share.enabled);
        assert_eq!(effective.download_share.path, "C:\\Freigabe");
        assert_eq!(effective.upload_share.path, "D:\\Eingang");
        assert_eq!(effective.preferred_adapter_id.as_deref(), Some("lan-id"));
        assert_eq!(effective.port, 9123);
        assert_eq!(effective.max_upload_bytes, Some(5_000));
        assert_eq!(effective.max_inbox_bytes, 10_000);
        assert_eq!(effective.max_inbox_files, 200);
        assert_eq!(effective.idle_timeout_minutes, Some(30));
    }

    #[test]
    fn resolves_profile_overrides_without_mutating_global_defaults() {
        let mut settings = AppSettings {
            preferred_adapter_id: Some("global-lan".into()),
            port: 8765,
            max_upload_bytes: Some(100),
            max_inbox_bytes: 200,
            max_inbox_files: 20,
            idle_timeout_minutes: None,
            ..Default::default()
        };
        settings.profiles[0].overrides = ProfileOverrides {
            network: Some(NetworkSettings {
                preferred_adapter_id: None,
            }),
            port: Some(9123),
            limits: Some(LimitSettings {
                max_upload_bytes: None,
                max_inbox_bytes: 400,
                max_inbox_files: 40,
                idle_timeout_minutes: Some(60),
            }),
        };

        let effective = settings.runtime_settings().unwrap();

        assert_eq!(effective.preferred_adapter_id, None);
        assert_eq!(effective.port, 9123);
        assert_eq!(effective.max_upload_bytes, None);
        assert_eq!(effective.max_inbox_bytes, 400);
        assert_eq!(effective.max_inbox_files, 40);
        assert_eq!(effective.idle_timeout_minutes, Some(60));
        assert_eq!(settings.preferred_adapter_id.as_deref(), Some("global-lan"));
        assert_eq!(settings.port, 8765);
    }

    #[test]
    fn rejects_missing_active_profile_and_duplicate_profile_names() {
        let missing = AppSettings {
            active_profile_id: uuid::Uuid::new_v4().to_string(),
            ..Default::default()
        };
        assert!(missing
            .validate()
            .unwrap_err()
            .contains("aktive Freigabeprofil"));

        let mut duplicate = AppSettings::default();
        let mut second = duplicate.profiles[0].clone();
        second.id = uuid::Uuid::new_v4().to_string();
        second.name = "standard".into();
        duplicate.profiles.push(second);
        assert!(duplicate.validate().unwrap_err().contains("Profilname"));
    }
}
