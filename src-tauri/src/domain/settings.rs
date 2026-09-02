use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

pub const DEFAULT_PORT: u16 = 8765;
pub const DEFAULT_MAX_UPLOAD: u64 = 20 * 1024 * 1024 * 1024;
pub const DEFAULT_MAX_INBOX_BYTES: u64 = 100 * 1024 * 1024 * 1024;
pub const DEFAULT_MAX_INBOX_FILES: u32 = 10_000;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShareSettings {
    pub enabled: bool,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub version: u32,
    pub ui_version: String,
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
            version: 1,
            ui_version: env!("CARGO_PKG_VERSION").into(),
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

pub fn load(path: &Path) -> LoadedSettings {
    match fs::read_to_string(path) {
        Ok(value) => match serde_json::from_str(&value) {
            Ok(settings) => LoadedSettings {
                settings,
                warning: None,
            },
            Err(error) => LoadedSettings {
                settings: AppSettings::default(),
                warning: Some(format!(
                    "Die gespeicherten Einstellungen sind beschädigt ({error}). Sichere Standardwerte sind aktiv; die vorhandene settings.json wurde unverändert behalten."
                )),
            },
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

pub fn save(path: &Path, settings: &AppSettings) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Ungültiger Konfigurationspfad.".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let data = serde_json::to_vec_pretty(settings).map_err(|error| error.to_string())?;
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
        .map(|_| ())
        .map_err(|error| error.error.to_string())
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
        save(&path, &expected).unwrap();
        let loaded = load(&path);
        assert!(loaded.warning.is_none());
        assert_eq!(loaded.settings.version, 1);
        assert_eq!(loaded.settings.ui_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(loaded.settings.port, 9000);
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
    fn unreadable_settings_path_is_reported_without_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("settings.json");
        fs::create_dir(&path).unwrap();
        let loaded = load(&path);
        assert!(loaded.warning.is_some());
        assert!(path.is_dir());
    }
}
