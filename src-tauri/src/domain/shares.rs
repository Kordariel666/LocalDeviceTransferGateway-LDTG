use super::settings::AppSettings;
use super::types::ShareValidation;
use same_file::Handle as FileHandle;
use std::{
    fs, io,
    path::{Component, Path, PathBuf},
    sync::Arc,
};
use unicode_normalization::UnicodeNormalization;

const MAX_WINDOWS_FILENAME_UTF16: usize = 240;
const MAX_WINDOWS_PATH_UTF16: usize = 32_000;
const MIN_UPLOAD_FILENAME_UTF16: usize = 16;

#[derive(Debug, Clone)]
pub struct ShareRoots {
    pub download: Option<PathBuf>,
    pub upload: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct RootAnchor {
    path: PathBuf,
    handle: Arc<FileHandle>,
}

impl RootAnchor {
    pub fn open(path: &Path) -> Result<Self, String> {
        let path = fs::canonicalize(path)
            .map_err(|_| "Der Freigabeordner konnte nicht kanonisch verankert werden.")?;
        #[cfg(windows)]
        let directory = {
            use std::os::windows::fs::OpenOptionsExt;
            use windows_sys::Win32::Storage::FileSystem::{
                FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
            };
            let mut options = fs::OpenOptions::new();
            options
                .read(true)
                .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
                .custom_flags(FILE_FLAG_BACKUP_SEMANTICS);
            options
                .open(&path)
                .map_err(|_| "Der Freigabeordner konnte nicht sicher geöffnet werden.")?
        };
        #[cfg(windows)]
        let handle = FileHandle::from_file(directory);
        #[cfg(not(windows))]
        let handle = FileHandle::from_path(&path);
        Ok(Self {
            path,
            handle: Arc::new(
                handle.map_err(|_| "Der Freigabeordner konnte nicht sicher verankert werden.")?,
            ),
        })
    }

    pub fn is_current(&self) -> bool {
        FileHandle::from_path(&self.path)
            .map(|current| current == *self.handle)
            .unwrap_or(false)
    }

    pub fn validate_open_file(&self, file: &fs::File, path: &Path) -> Result<PathBuf, String> {
        if !self.is_current() {
            return Err("Die Freigabe wurde während des Betriebs verändert.".into());
        }
        let resolved = final_path_from_file(file, path)?;
        if !path_is_within(&resolved, &self.path) {
            return Err("Die geöffnete Datei liegt nicht mehr in der bestätigten Freigabe.".into());
        }
        validate_resolved_policy(&self.path, &resolved)?;
        if !self.is_current() {
            return Err("Die Freigabe wurde während des Betriebs verändert.".into());
        }
        Ok(resolved)
    }

    pub fn open_existing(&self, path: &Path) -> Result<(PathBuf, fs::File, fs::Metadata), String> {
        let mut options = fs::OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            use windows_sys::Win32::Storage::FileSystem::{
                FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE,
                FILE_SHARE_READ, FILE_SHARE_WRITE,
            };
            options
                .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
                .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
        }
        let file = options
            .open(path)
            .map_err(|_| "Die Datei konnte nicht sicher geöffnet werden.")?;
        let metadata = file
            .metadata()
            .map_err(|_| "Die Dateiinformationen konnten nicht sicher gelesen werden.")?;
        if is_reparse_point(&metadata) {
            return Err("Verknüpfungen innerhalb der Freigabe sind nicht erlaubt.".into());
        }
        let resolved = self.validate_open_file(&file, path)?;
        Ok((resolved, file, metadata))
    }
}

pub fn broad_share_warning(settings: &AppSettings) -> Option<String> {
    let profile = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .and_then(|path| fs::canonicalize(path).ok());
    let profile = profile.as_deref()?;
    [
        settings
            .download_share
            .enabled
            .then_some(settings.download_share.path.as_str()),
        settings
            .upload_share
            .enabled
            .then_some(settings.upload_share.path.as_str()),
    ]
    .into_iter()
    .flatten()
    .filter_map(|path| fs::canonicalize(path).ok())
    .find(|path| path == profile)
    .map(|path| path.display().to_string())
}

pub fn prepare_roots(download: Option<&str>, upload: Option<&str>) -> Result<ShareRoots, String> {
    let download = download
        .map(|value| validate_root(Path::new(value), false))
        .transpose()?;
    let upload = upload
        .map(|value| validate_root(Path::new(value), true))
        .transpose()?;
    if download
        .as_deref()
        .zip(upload.as_deref())
        .is_some_and(|(download, upload)| roots_overlap(download, upload))
    {
        return Err(
            "Downloadfreigabe und Upload-Eingang müssen vollständig getrennte Ordner sein.".into(),
        );
    }
    Ok(ShareRoots { download, upload })
}

fn validate_configured_root(
    enabled: bool,
    path: &str,
    writable: bool,
    missing_error: &str,
) -> Result<Option<PathBuf>, String> {
    if !enabled {
        return Ok(None);
    }
    if path.trim().is_empty() {
        return Err(missing_error.into());
    }
    validate_root(Path::new(path), writable).map(Some)
}

pub fn validate_share_settings(settings: &AppSettings) -> ShareValidation {
    let download = validate_configured_root(
        settings.download_share.enabled,
        &settings.download_share.path,
        false,
        "Für Downloads fehlt ein Ordner.",
    );
    let upload = validate_configured_root(
        settings.upload_share.enabled,
        &settings.upload_share.path,
        true,
        "Für Uploads fehlt ein Eingangsordner.",
    );
    let overlap_error = match (&download, &upload) {
        (Ok(Some(download)), Ok(Some(upload))) if roots_overlap(download, upload) => Some(
            "Downloadfreigabe und Upload-Eingang müssen vollständig getrennte Ordner sein.".into(),
        ),
        _ => None,
    };
    ShareValidation {
        download_error: download.err(),
        upload_error: upload.err(),
        overlap_error,
    }
}

fn roots_overlap(left: &Path, right: &Path) -> bool {
    left == right || left.starts_with(right) || right.starts_with(left)
}

fn validate_root(path: &Path, writable: bool) -> Result<PathBuf, String> {
    if !path.is_dir() {
        return Err(format!("Der Ordner existiert nicht: {}", path.display()));
    }
    let selected_metadata =
        fs::symlink_metadata(path).map_err(|error| format!("Ordner nicht erreichbar: {error}"))?;
    if selected_metadata.file_type().is_symlink() || is_reparse_point(&selected_metadata) {
        return Err(
            "Verknüpfungen und Reparse-Points dürfen nicht als Freigabe verwendet werden.".into(),
        );
    }
    let canonical =
        fs::canonicalize(path).map_err(|error| format!("Ordner nicht erreichbar: {error}"))?;
    if canonical.parent().is_none() {
        return Err("Ein vollständiges Laufwerk darf nicht freigegeben werden.".into());
    }
    fs::read_dir(&canonical)
        .map_err(|_| "Der Freigabeordner kann nicht gelesen werden.".to_string())?;
    let protected_roots = ["WINDIR", "ProgramFiles", "ProgramFiles(x86)"]
        .into_iter()
        .filter_map(std::env::var_os)
        .filter_map(|value| fs::canonicalize(value).ok());
    if protected_roots
        .into_iter()
        .any(|protected| canonical == protected || canonical.starts_with(&protected))
    {
        return Err("Windows- und Programmverzeichnisse dürfen nicht freigegeben werden.".into());
    }
    if writable {
        if remote_or_unknown_volume(&canonical) {
            return Err(
                "Netzwerk-, UNC- und nicht sicher bestimmbare Laufwerke dürfen nicht als Upload-Eingang verwendet werden."
                    .into(),
            );
        }
        let executable_search_path = executable_search_roots()
            .into_iter()
            .any(|candidate| candidate == canonical);
        let powershell_module_path = powershell_module_roots()
            .into_iter()
            .any(|candidate| canonical == candidate || canonical.starts_with(candidate));
        let windows_autoload_path = path_is_within_any(&canonical, windows_autoload_roots().iter());
        let office_autoload_path = path_is_within_any(&canonical, office_autoload_roots().iter());
        if executable_search_path
            || powershell_module_path
            || windows_autoload_path
            || office_autoload_path
        {
            return Err("Programm-, Arbeits-, PATH-, PowerShell-Modul-, Windows-Autostart- und Office-Autoload-Verzeichnisse dürfen nicht als Upload-Eingang verwendet werden.".into());
        }
        let probe = canonical.join(format!(".dmdc-write-test-{}", uuid::Uuid::new_v4()));
        fs::write(&probe, b"test")
            .map_err(|_| "Der Uploadordner ist nicht beschreibbar.".to_string())?;
        fs::remove_file(probe)
            .map_err(|_| "Die Schreibprüfung konnte nicht bereinigt werden.".to_string())?;
    }
    Ok(canonical)
}

fn path_is_within_any<'a>(path: &Path, mut roots: impl Iterator<Item = &'a PathBuf>) -> bool {
    roots.any(|root| path == root || path.starts_with(root))
}

fn windows_autoload_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = [
        std::env::var_os("APPDATA").map(PathBuf::from),
        std::env::var_os("PROGRAMDATA").map(PathBuf::from),
    ]
    .into_iter()
    .flatten()
    .map(|root| {
        root.join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("Startup")
    })
    .filter_map(|path| fs::canonicalize(path).ok())
    .collect();
    roots.extend(effective_startup_roots());
    roots.sort();
    roots.dedup();
    roots
}

#[cfg(windows)]
fn effective_startup_roots() -> Vec<PathBuf> {
    use std::{ffi::OsString, os::windows::ffi::OsStringExt, ptr};
    use windows_sys::{
        core::GUID,
        Win32::UI::Shell::{
            FOLDERID_CommonStartup, FOLDERID_Startup, SHGetKnownFolderPath, KF_FLAG_DEFAULT,
        },
    };

    #[link(name = "ole32")]
    unsafe extern "system" {
        fn CoTaskMemFree(value: *const core::ffi::c_void);
    }

    fn resolve(id: &GUID) -> Option<PathBuf> {
        let mut raw = ptr::null_mut();
        let result =
            unsafe { SHGetKnownFolderPath(id, KF_FLAG_DEFAULT as u32, ptr::null_mut(), &mut raw) };
        if result < 0 || raw.is_null() {
            return None;
        }
        let mut length = 0;
        unsafe {
            while *raw.add(length) != 0 {
                length += 1;
            }
        }
        let value = unsafe { OsString::from_wide(std::slice::from_raw_parts(raw, length)) };
        unsafe { CoTaskMemFree(raw.cast()) };
        fs::canonicalize(PathBuf::from(value)).ok()
    }

    [resolve(&FOLDERID_Startup), resolve(&FOLDERID_CommonStartup)]
        .into_iter()
        .flatten()
        .collect()
}

#[cfg(not(windows))]
fn effective_startup_roots() -> Vec<PathBuf> {
    Vec::new()
}

fn office_autoload_roots() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(app_data) = std::env::var_os("APPDATA").map(PathBuf::from) {
        candidates.push(app_data.join("Microsoft").join("Word").join("STARTUP"));
        candidates.push(app_data.join("Microsoft").join("Excel").join("XLSTART"));
    }
    for program_files in ["ProgramFiles", "ProgramFiles(x86)"] {
        let Some(program_files) = std::env::var_os(program_files).map(PathBuf::from) else {
            continue;
        };
        for version in ["Office16", "Office15", "Office14", "Office12"] {
            for base in [
                program_files
                    .join("Microsoft Office")
                    .join("root")
                    .join(version),
                program_files.join("Microsoft Office").join(version),
            ] {
                candidates.push(base.join("XLSTART"));
                candidates.push(base.join("STARTUP"));
            }
        }
    }
    candidates
        .into_iter()
        .filter_map(|path| fs::canonicalize(path).ok())
        .collect()
}

#[cfg(windows)]
fn remote_or_unknown_volume(path: &Path) -> bool {
    use std::{iter, os::windows::ffi::OsStrExt};
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetVolumePathNameW};

    const DRIVE_REMOVABLE: u32 = 2;
    const DRIVE_FIXED: u32 = 3;
    const DRIVE_RAMDISK: u32 = 6;

    let source: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect();
    let mut volume = vec![0_u16; 32_768];
    let found =
        unsafe { GetVolumePathNameW(source.as_ptr(), volume.as_mut_ptr(), volume.len() as u32) };
    if found == 0 {
        return true;
    }
    let drive_type = unsafe { GetDriveTypeW(volume.as_ptr()) };
    !matches!(drive_type, DRIVE_FIXED | DRIVE_REMOVABLE | DRIVE_RAMDISK)
}

#[cfg(not(windows))]
fn remote_or_unknown_volume(_: &Path) -> bool {
    false
}

fn executable_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(executable) = std::env::current_exe() {
        if let Some(parent) = executable.parent() {
            roots.push(parent.to_path_buf());
        }
    }
    if let Ok(current) = std::env::current_dir() {
        roots.push(current);
    }
    if let Some(path) = std::env::var_os("PATH") {
        roots.extend(std::env::split_paths(&path));
    }
    roots
        .into_iter()
        .filter_map(|path| fs::canonicalize(path).ok())
        .collect()
}

#[cfg(windows)]
fn powershell_module_roots() -> Vec<PathBuf> {
    std::env::var_os("PSModulePath")
        .map(|value| {
            std::env::split_paths(&value)
                .filter_map(|path| fs::canonicalize(path).ok())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(not(windows))]
fn powershell_module_roots() -> Vec<PathBuf> {
    Vec::new()
}

pub fn safe_existing(
    root: &Path,
    relative: &str,
    expect_directory: Option<bool>,
) -> Result<PathBuf, String> {
    let relative_path = Path::new(relative);
    if relative_path.components().any(|part| {
        matches!(
            part,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err("Ungültiger Dateipfad.".into());
    }
    let mut current = root.to_path_buf();
    for component in relative_path.components() {
        if let Component::Normal(value) = component {
            if value.to_string_lossy().contains(':') {
                return Err("Alternate Data Streams sind nicht erlaubt.".into());
            }
            current.push(value);
            let metadata = fs::symlink_metadata(&current)
                .map_err(|_| "Datei oder Ordner wurde nicht gefunden.".to_string())?;
            if metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
                return Err("Verknüpfungen innerhalb der Freigabe sind nicht erlaubt.".into());
            }
            if is_hidden_or_managed(&current, &metadata) {
                return Err("Versteckte und Systemdateien sind nicht freigegeben.".into());
            }
        }
    }
    let canonical = fs::canonicalize(&current)
        .map_err(|_| "Datei oder Ordner wurde nicht gefunden.".to_string())?;
    if canonical != root && !canonical.starts_with(root) {
        return Err("Zugriff außerhalb der Freigabe wurde blockiert.".into());
    }
    validate_resolved_policy(root, &canonical)?;
    if let Some(directory) = expect_directory {
        if directory != canonical.is_dir() {
            return Err(if directory {
                "Ordner erwartet."
            } else {
                "Datei erwartet."
            }
            .into());
        }
    }
    Ok(canonical)
}

fn path_is_within(path: &Path, root: &Path) -> bool {
    #[cfg(windows)]
    {
        let path = path.to_string_lossy().replace('/', "\\").to_lowercase();
        let root = root.to_string_lossy().replace('/', "\\").to_lowercase();
        path == root
            || path
                .strip_prefix(&root)
                .is_some_and(|suffix| suffix.starts_with('\\'))
    }
    #[cfg(not(windows))]
    {
        path == root || path.starts_with(root)
    }
}

fn validate_resolved_policy(root: &Path, resolved: &Path) -> Result<(), String> {
    let relative = resolved
        .strip_prefix(root)
        .map_err(|_| "Zugriff außerhalb der Freigabe wurde blockiert.")?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        if let Component::Normal(value) = component {
            current.push(value);
            let metadata = fs::symlink_metadata(&current)
                .map_err(|_| "Datei oder Ordner wurde nicht gefunden.".to_string())?;
            if metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
                return Err("Verknüpfungen innerhalb der Freigabe sind nicht erlaubt.".into());
            }
            if is_hidden_or_managed(&current, &metadata) {
                return Err("Versteckte und Systemdateien sind nicht freigegeben.".into());
            }
        }
    }
    Ok(())
}

#[cfg(windows)]
fn final_path_from_file(file: &fs::File, _: &Path) -> Result<PathBuf, String> {
    use std::{
        ffi::OsString,
        os::windows::{ffi::OsStringExt, io::AsRawHandle},
    };
    use windows_sys::Win32::Storage::FileSystem::{
        GetFinalPathNameByHandleW, FILE_NAME_NORMALIZED, VOLUME_NAME_DOS,
    };

    let handle = file.as_raw_handle() as windows_sys::Win32::Foundation::HANDLE;
    let required = unsafe {
        GetFinalPathNameByHandleW(
            handle,
            std::ptr::null_mut(),
            0,
            FILE_NAME_NORMALIZED | VOLUME_NAME_DOS,
        )
    };
    if required == 0 {
        return Err("Der endgültige Dateipfad konnte nicht geprüft werden.".into());
    }
    let mut value = vec![0_u16; required as usize + 1];
    let written = unsafe {
        GetFinalPathNameByHandleW(
            handle,
            value.as_mut_ptr(),
            value.len() as u32,
            FILE_NAME_NORMALIZED | VOLUME_NAME_DOS,
        )
    };
    if written == 0 || written as usize >= value.len() {
        return Err("Der endgültige Dateipfad konnte nicht geprüft werden.".into());
    }
    Ok(PathBuf::from(OsString::from_wide(
        &value[..written as usize],
    )))
}

#[cfg(not(windows))]
fn final_path_from_file(file: &fs::File, path: &Path) -> Result<PathBuf, String> {
    let opened = FileHandle::from_file(
        file.try_clone()
            .map_err(|_| "Dateiidentität konnte nicht geprüft werden.")?,
    )
    .map_err(|_| "Dateiidentität konnte nicht geprüft werden.")?;
    let current =
        FileHandle::from_path(path).map_err(|_| "Dateiidentität konnte nicht geprüft werden.")?;
    if opened != current {
        return Err("Die Datei wurde während der Prüfung ersetzt.".into());
    }
    fs::canonicalize(path)
        .map_err(|_| "Der endgültige Dateipfad konnte nicht geprüft werden.".into())
}

#[cfg(windows)]
pub(crate) fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x400 != 0
}
#[cfg(not(windows))]
pub(crate) fn is_reparse_point(_: &fs::Metadata) -> bool {
    false
}

pub fn is_hidden_or_managed(path: &Path, metadata: &fs::Metadata) -> bool {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == ".dmdc" || name.starts_with('.'))
    {
        return true;
    }
    hidden_attributes(metadata)
}

#[cfg(windows)]
fn hidden_attributes(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & (0x2 | 0x4) != 0
}
#[cfg(not(windows))]
fn hidden_attributes(_: &fs::Metadata) -> bool {
    false
}

fn utf16_len(value: &str) -> usize {
    value.encode_utf16().count()
}

fn truncate_utf16(value: &str, limit: usize) -> String {
    let mut used = 0;
    value
        .chars()
        .take_while(|character| {
            let width = character.len_utf16();
            if used + width > limit {
                false
            } else {
                used += width;
                true
            }
        })
        .collect()
}

#[cfg(windows)]
fn path_utf16_len(path: &Path) -> usize {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str().encode_wide().count()
}

#[cfg(not(windows))]
fn path_utf16_len(path: &Path) -> usize {
    path.to_string_lossy().encode_utf16().count()
}

fn filename_limit_for_root(root: &Path) -> Result<usize, String> {
    let available = MAX_WINDOWS_PATH_UTF16.saturating_sub(path_utf16_len(root) + 1);
    let limit = available.min(MAX_WINDOWS_FILENAME_UTF16);
    if limit < MIN_UPLOAD_FILENAME_UTF16 {
        return Err("Der Uploadordner ist für einen sicheren Windows-Zielpfad zu lang.".into());
    }
    Ok(limit)
}

fn sanitize_file_name(raw: &str, limit: usize) -> Result<String, String> {
    let normalized: String = raw.nfc().collect();
    let mut value: String = normalized
        .chars()
        .map(|character| {
            if character.is_control()
                || is_invisible_format(character)
                || matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
            {
                '_'
            } else {
                character
            }
        })
        .collect();
    value = value.trim().trim_end_matches([' ', '.']).to_string();
    if value.is_empty() || value == "." || value == ".." {
        return Err("Ungültiger Dateiname.".into());
    }
    let stem = Path::new(&value)
        .file_stem()
        .and_then(|item| item.to_str())
        .unwrap_or("")
        .to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit());
    if reserved {
        value.insert(0, '_');
    }
    value = truncate_utf16(&value, limit);
    value = value.trim_end_matches([' ', '.']).to_string();
    if value.is_empty() {
        return Err("Der Dateiname ist für den Zielpfad zu lang.".into());
    }
    Ok(value)
}

fn is_invisible_format(character: char) -> bool {
    matches!(
        character,
        '\u{061c}'
            | '\u{200b}'..='\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2060}'..='\u{206f}'
            | '\u{feff}'
    )
}

pub fn create_upload_partial(path: &Path) -> io::Result<fs::File> {
    let mut options = fs::OpenOptions::new();
    options.read(true).write(true).create_new(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::{
            Foundation::{GENERIC_READ, GENERIC_WRITE},
            Storage::FileSystem::DELETE,
        };
        options
            .access_mode(GENERIC_READ | GENERIC_WRITE | DELETE)
            .share_mode(0);
    }
    options.open(path)
}

pub fn delete_open_upload(file: &fs::File, _path: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            FileDispositionInfo, SetFileInformationByHandle, FILE_DISPOSITION_INFO,
        };
        let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
        let changed = unsafe {
            SetFileInformationByHandle(
                file.as_raw_handle() as windows_sys::Win32::Foundation::HANDLE,
                FileDispositionInfo,
                (&disposition as *const FILE_DISPOSITION_INFO).cast(),
                std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
            )
        };
        if changed == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        fs::remove_file(_path)
    }
}

pub fn publish_open_upload(
    file: &fs::File,
    _source_path: &Path,
    root: &Path,
    anchor: &RootAnchor,
    name: &str,
) -> Result<PathBuf, String> {
    if !anchor.is_current() {
        return Err("Der Upload-Eingang wurde während des Betriebs verändert.".into());
    }
    #[cfg(windows)]
    {
        for _ in 0..100 {
            let target = unique_target(root, name)?;
            if !anchor.is_current() {
                return Err("Der Upload-Eingang wurde während des Betriebs verändert.".into());
            }
            match rename_open_file(file, &target) {
                Ok(()) => match anchor.validate_open_file(file, &target) {
                    Ok(resolved) => return Ok(resolved),
                    Err(error) => {
                        let _ = delete_open_upload(file, &target);
                        return Err(error);
                    }
                },
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(format!(
                        "Datei konnte nicht endgültig übernommen werden: {error}"
                    ))
                }
            }
        }
        Err("Für den Dateinamen konnte kein freier Zielname reserviert werden.".into())
    }
    #[cfg(not(windows))]
    {
        publish_new(_source_path, root, name)
    }
}

#[cfg(windows)]
fn rename_open_file(file: &fs::File, target: &Path) -> io::Result<()> {
    use std::{
        os::windows::{ffi::OsStrExt, io::AsRawHandle},
        ptr,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        FileRenameInfo, SetFileInformationByHandle, FILE_RENAME_INFO,
    };

    let name: Vec<u16> = target.as_os_str().encode_wide().collect();
    let bytes = name.len().saturating_mul(std::mem::size_of::<u16>());
    let size = std::mem::size_of::<FILE_RENAME_INFO>().saturating_add(bytes);
    let mut buffer = vec![0_u8; size];
    let info = buffer.as_mut_ptr().cast::<FILE_RENAME_INFO>();
    unsafe {
        (*info).Anonymous.ReplaceIfExists = false;
        (*info).RootDirectory = ptr::null_mut();
        (*info).FileNameLength = bytes as u32;
        ptr::copy_nonoverlapping(name.as_ptr(), (*info).FileName.as_mut_ptr(), name.len());
    }
    let changed = unsafe {
        SetFileInformationByHandle(
            file.as_raw_handle() as windows_sys::Win32::Foundation::HANDLE,
            FileRenameInfo,
            buffer.as_ptr().cast(),
            buffer.len() as u32,
        )
    };
    if changed == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
pub fn safe_file_name(raw: &str) -> Result<String, String> {
    sanitize_file_name(raw, MAX_WINDOWS_FILENAME_UTF16)
}

pub fn safe_file_name_for_root(root: &Path, raw: &str) -> Result<String, String> {
    sanitize_file_name(raw, filename_limit_for_root(root)?)
}

fn opaque_target_name(name: &str, nonce: &str, limit: usize) -> String {
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("Datei");
    let suffix = format!(" ({nonce})");
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{value}"))
        .unwrap_or_default();
    let maximum_extension = limit.saturating_sub(utf16_len(&suffix) + 1);
    let extension = truncate_utf16(&extension, maximum_extension);
    let maximum_stem = limit.saturating_sub(utf16_len(&suffix) + utf16_len(&extension));
    let mut stem = truncate_utf16(stem, maximum_stem);
    if stem.is_empty() {
        stem = "_".into();
    }
    format!("{stem}{suffix}{extension}")
}

pub fn unique_target(root: &Path, name: &str) -> Result<PathBuf, String> {
    let limit = filename_limit_for_root(root)?;
    let name = truncate_utf16(name, limit);
    let random = uuid::Uuid::new_v4().simple().to_string();
    Ok(root.join(opaque_target_name(&name, &random[..12], limit)))
}

#[cfg(any(not(windows), test))]
pub fn publish_new(partial: &Path, root: &Path, name: &str) -> Result<PathBuf, String> {
    for _ in 0..100 {
        let target = unique_target(root, name)?;
        match publish_without_replace(partial, &target) {
            Ok(()) => return Ok(target),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "Datei konnte nicht endgültig übernommen werden: {error}"
                ))
            }
        }
    }
    Err("Für den Dateinamen konnte kein freier Zielname reserviert werden.".into())
}

#[cfg(all(windows, test))]
fn publish_without_replace(partial: &Path, target: &Path) -> std::io::Result<()> {
    use std::{iter, os::windows::ffi::OsStrExt};
    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;

    let partial: Vec<u16> = partial
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect();
    let target: Vec<u16> = target
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect();
    let moved = unsafe { MoveFileExW(partial.as_ptr(), target.as_ptr(), 0) };
    if moved == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn publish_without_replace(partial: &Path, target: &Path) -> std::io::Result<()> {
    fs::hard_link(partial, target)?;
    if let Err(error) = fs::remove_file(partial) {
        let _ = fs::remove_file(target);
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sanitizes_windows_names() {
        assert_eq!(safe_file_name("../CON?.txt").unwrap(), ".._CON_.txt");
        assert_eq!(safe_file_name("Bild: 1.jpg").unwrap(), "Bild_ 1.jpg");
        assert_eq!(
            safe_file_name("rechnung\u{202e}fdp.exe").unwrap(),
            "rechnung_fdp.exe"
        );
    }

    #[test]
    fn limits_non_bmp_names_by_utf16_units() {
        let name = safe_file_name(&format!("{}.txt", "😀".repeat(200))).unwrap();
        assert!(utf16_len(&name) <= MAX_WINDOWS_FILENAME_UTF16);
    }

    #[test]
    fn opaque_suffix_stays_inside_utf16_component_limit() {
        let temp = tempfile::tempdir().unwrap();
        let name = format!("{}.txt", "😀".repeat(118));
        assert_eq!(utf16_len(&name), MAX_WINDOWS_FILENAME_UTF16);
        fs::write(temp.path().join(&name), b"vorhanden").unwrap();
        let target = unique_target(temp.path(), &name).unwrap();
        let target_name = target.file_name().unwrap().to_string_lossy();
        assert!(utf16_len(&target_name) <= MAX_WINDOWS_FILENAME_UTF16);
        assert!(target_name.ends_with(").txt"));
        let suffix = target_name
            .rsplit_once(" (")
            .unwrap()
            .1
            .strip_suffix(").txt")
            .unwrap();
        assert_eq!(suffix.len(), 12);
        assert!(suffix.bytes().all(|value| value.is_ascii_hexdigit()));
    }

    #[test]
    fn rejects_root_without_safe_total_path_budget() {
        let root = PathBuf::from("x".repeat(MAX_WINDOWS_PATH_UTF16));
        assert!(safe_file_name_for_root(&root, "datei.txt").is_err());
    }
    #[test]
    fn rejects_parent_components() {
        let temp = tempfile::tempdir().unwrap();
        assert!(safe_existing(temp.path(), "../secret", None).is_err());
    }
    #[test]
    fn rejects_alternate_data_streams() {
        let temp = tempfile::tempdir().unwrap();
        assert!(safe_existing(temp.path(), "datei.txt:stream", None).is_err());
    }

    #[test]
    fn rejects_equal_and_nested_share_roots() {
        let temp = tempfile::tempdir().unwrap();
        let inbox = temp.path().join("inbox");
        let download = temp.path().join("download");
        fs::create_dir(&inbox).unwrap();
        fs::create_dir(&download).unwrap();

        assert!(prepare_roots(
            Some(temp.path().to_str().unwrap()),
            Some(temp.path().to_str().unwrap())
        )
        .is_err());
        assert!(prepare_roots(
            Some(temp.path().to_str().unwrap()),
            Some(inbox.to_str().unwrap())
        )
        .is_err());
        assert!(prepare_roots(
            Some(inbox.to_str().unwrap()),
            Some(temp.path().to_str().unwrap())
        )
        .is_err());
        assert!(prepare_roots(
            Some(download.to_str().unwrap()),
            Some(inbox.to_str().unwrap())
        )
        .is_ok());
    }

    #[test]
    fn reports_share_errors_for_the_responsible_fields() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("missing");
        let settings = AppSettings {
            download_share: super::super::settings::ShareSettings {
                enabled: true,
                path: String::new(),
            },
            upload_share: super::super::settings::ShareSettings {
                enabled: true,
                path: missing.display().to_string(),
            },
            ..Default::default()
        };

        let validation = validate_share_settings(&settings);

        assert!(validation.download_error.unwrap().contains("Downloads"));
        assert!(validation.upload_error.unwrap().contains("existiert nicht"));
        assert!(validation.overlap_error.is_none());
    }

    #[test]
    fn reports_canonical_share_overlap_before_start() {
        let temp = tempfile::tempdir().unwrap();
        let nested = temp.path().join("nested");
        fs::create_dir(&nested).unwrap();
        let settings = AppSettings {
            download_share: super::super::settings::ShareSettings {
                enabled: true,
                path: temp.path().join(".").display().to_string(),
            },
            upload_share: super::super::settings::ShareSettings {
                enabled: true,
                path: nested.display().to_string(),
            },
            ..Default::default()
        };

        let validation = validate_share_settings(&settings);

        assert!(validation.download_error.is_none());
        assert!(validation.upload_error.is_none());
        assert!(validation
            .overlap_error
            .unwrap()
            .contains("getrennte Ordner"));
    }

    #[test]
    fn canonical_aliases_are_treated_as_the_same_share() {
        let temp = tempfile::tempdir().unwrap();
        let canonical = fs::canonicalize(temp.path()).unwrap();
        assert!(roots_overlap(
            &canonical,
            &fs::canonicalize(&canonical).unwrap()
        ));
    }

    #[cfg(windows)]
    #[test]
    fn short_name_alias_of_a_dotfile_is_rejected_after_resolution() {
        use std::{
            ffi::OsString,
            iter,
            os::windows::ffi::{OsStrExt, OsStringExt},
        };
        use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;

        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let hidden = root.join(".sensitive-configuration-file");
        fs::write(&hidden, b"secret").unwrap();
        let wide: Vec<u16> = hidden
            .as_os_str()
            .encode_wide()
            .chain(iter::once(0))
            .collect();
        let required = unsafe { GetShortPathNameW(wide.as_ptr(), std::ptr::null_mut(), 0) };
        if required == 0 {
            return;
        }
        let mut output = vec![0_u16; required as usize + 1];
        let written =
            unsafe { GetShortPathNameW(wide.as_ptr(), output.as_mut_ptr(), output.len() as u32) };
        if written == 0 {
            return;
        }
        let alias = PathBuf::from(OsString::from_wide(&output[..written as usize]));
        let alias_name = alias.file_name().unwrap().to_string_lossy();
        assert!(safe_existing(&root, &alias_name, Some(false)).is_err());
    }

    #[test]
    fn startup_roots_and_descendants_are_blocked_upload_targets() {
        let temp = tempfile::tempdir().unwrap();
        let user_startup = temp.path().join("UserStartup");
        let common_startup = temp.path().join("CommonStartup");
        let child = user_startup.join("child");
        let ordinary = temp.path().join("ordinary");
        fs::create_dir(&user_startup).unwrap();
        fs::create_dir(&common_startup).unwrap();
        fs::create_dir(&child).unwrap();
        fs::create_dir(&ordinary).unwrap();
        let protected = [
            fs::canonicalize(&user_startup).unwrap(),
            fs::canonicalize(&common_startup).unwrap(),
        ];
        let child = fs::canonicalize(&child).unwrap();
        let ordinary = fs::canonicalize(&ordinary).unwrap();
        assert!(path_is_within_any(&child, protected.iter()));
        assert!(path_is_within_any(&protected[1], protected.iter()));
        assert!(!path_is_within_any(&ordinary, protected.iter()));
    }

    #[test]
    fn publishes_without_overwriting() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("datei.txt"), b"alt").unwrap();
        let partial = temp.path().join("teil.part");
        fs::write(&partial, b"neu").unwrap();
        let target = publish_new(&partial, temp.path(), "datei.txt").unwrap();
        let target_name = target.file_name().unwrap().to_string_lossy();
        assert!(target_name.starts_with("datei ("));
        assert!(target_name.ends_with(").txt"));
        assert_ne!(target_name, "datei (2).txt");
        assert_eq!(fs::read(temp.path().join("datei.txt")).unwrap(), b"alt");
        assert_eq!(fs::read(target).unwrap(), b"neu");
    }

    #[test]
    fn publishes_an_open_upload_handle_into_the_anchored_root() {
        let temp = tempfile::tempdir().unwrap();
        let partial_dir = temp.path().join("partials");
        fs::create_dir(&partial_dir).unwrap();
        let partial = partial_dir.join("teil.part");
        let mut file = create_upload_partial(&partial).unwrap();
        use std::io::Write as _;
        file.write_all(b"neu").unwrap();
        let anchor = RootAnchor::open(temp.path()).unwrap();

        let target =
            publish_open_upload(&file, &partial, temp.path(), &anchor, "datei.txt").unwrap();

        drop(file);
        assert_eq!(fs::read(target).unwrap(), b"neu");
        assert!(!partial.exists());
    }

    #[test]
    fn target_name_is_opaque_even_when_the_base_name_is_free() {
        let temp = tempfile::tempdir().unwrap();
        let target = unique_target(temp.path(), "frei.txt").unwrap();
        let target_name = target.file_name().unwrap().to_string_lossy();
        assert_ne!(target_name, "frei.txt");
        assert!(target_name.starts_with("frei ("));
        assert!(target_name.ends_with(").txt"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_publication_fails_if_target_appears() {
        let temp = tempfile::tempdir().unwrap();
        let partial = temp.path().join("teil.part");
        let target = temp.path().join("datei.txt");
        fs::write(&partial, b"neu").unwrap();
        fs::write(&target, b"alt").unwrap();

        let error = publish_without_replace(&partial, &target).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&target).unwrap(), b"alt");
        assert_eq!(fs::read(&partial).unwrap(), b"neu");
    }
}
