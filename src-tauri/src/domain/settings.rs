use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

pub const DEFAULT_PORT: u16 = 8765;
pub const DEFAULT_MAX_UPLOAD: u64 = 20 * 1024 * 1024 * 1024;
pub const DEFAULT_MAX_INBOX_BYTES: u64 = 100 * 1024 * 1024 * 1024;
pub const DEFAULT_MAX_INBOX_FILES: u32 = 10_000;
pub const CURRENT_SETTINGS_VERSION: u32 = 2;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShareSettings {
    pub enabled: bool,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub version: u32,
    pub download_share: ShareSettings,
    pub upload_share: ShareSettings,
    pub preferred_adapter_id: Option<String>,
    pub port: u16,
    pub max_upload_bytes: Option<u64>,
    pub max_inbox_bytes: u64,
    pub max_inbox_files: u32,
    pub idle_timeout_minutes: Option<u32>,
    pub trusted_networks: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            version: CURRENT_SETTINGS_VERSION,
            download_share: ShareSettings::default(),
            upload_share: ShareSettings::default(),
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
    pub fn validate(&self) -> Result<(), String> {
        if !(1024..=65535).contains(&self.port) {
            return Err("Der Port muss zwischen 1024 und 65535 liegen.".into());
        }
        if matches!(self.max_upload_bytes, Some(0)) {
            return Err("Das Uploadlimit muss größer als null sein.".into());
        }
        if self.max_inbox_bytes == 0 {
            return Err("Das Gesamtlimit des Upload-Eingangs muss größer als null sein.".into());
        }
        if self.max_inbox_files == 0 {
            return Err("Das Dateilimit des Upload-Eingangs muss größer als null sein.".into());
        }
        if self
            .max_upload_bytes
            .is_some_and(|limit| limit > self.max_inbox_bytes)
        {
            return Err("Das Uploadlimit pro Datei darf das Gesamtlimit des Upload-Eingangs nicht überschreiten.".into());
        }
        Ok(())
    }

    pub fn validate_for_start(&self) -> Result<(), String> {
        self.validate()?;
        if !self.download_share.enabled && !self.upload_share.enabled {
            return Err("Mindestens eine Freigabe muss aktiviert sein.".into());
        }
        if self.download_share.enabled && self.download_share.path.trim().is_empty() {
            return Err("Für Downloads fehlt ein Ordner.".into());
        }
        if self.upload_share.enabled && self.upload_share.path.trim().is_empty() {
            return Err("Für Uploads fehlt ein Eingangsordner.".into());
        }
        Ok(())
    }
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
    fn rejects_future_schema_without_replacing_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("settings.json");
        let original = br#"{"version":999,"port":9123}"#;
        fs::write(&path, original).unwrap();

        let loaded = load(&path);

        assert!(loaded.warning.unwrap().contains("Schema 2"));
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
}
