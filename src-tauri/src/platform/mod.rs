use crate::domain::types::FirewallStatus;
use tauri::AppHandle;

#[cfg(windows)]
use base64::{engine::general_purpose::STANDARD, Engine};
#[cfg(windows)]
use std::{
    ffi::OsStr,
    os::windows::{ffi::OsStrExt, process::CommandExt},
    path::PathBuf,
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};
#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, ERROR_CANCELLED, WAIT_FAILED},
    System::Threading::{GetExitCodeProcess, WaitForSingleObject, CREATE_NO_WINDOW, INFINITE},
    UI::{
        Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW},
        WindowsAndMessaging::SW_HIDE,
    },
};

const RULE_NAME: &str = "DMDC Local Transfer";
#[cfg(windows)]
const POWERSHELL_TIMEOUT: Duration = Duration::from_secs(15);

#[cfg(windows)]
fn executable_path() -> Result<PathBuf, String> {
    std::env::current_exe()
        .map_err(|error| format!("Der Programmpfad konnte nicht ermittelt werden: {error}"))
}

#[cfg(windows)]
pub(crate) fn powershell_path() -> Result<PathBuf, String> {
    use std::{ffi::OsString, os::windows::ffi::OsStringExt};
    use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;

    let mut buffer = vec![0_u16; 32_768];
    let length = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) } as usize;
    if length == 0 || length >= buffer.len() {
        return Err(
            "Das Windows-Systemverzeichnis konnte nicht über die Windows-API ermittelt werden."
                .into(),
        );
    }
    let system_directory = PathBuf::from(OsString::from_wide(&buffer[..length]));
    let canonical_root = std::fs::canonicalize(&system_directory)
        .map_err(|error| format!("Das Windows-Systemverzeichnis ist nicht erreichbar: {error}"))?;
    let candidate = canonical_root
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe");
    let canonical = std::fs::canonicalize(&candidate).map_err(|error| {
        format!("Windows PowerShell wurde nicht am erwarteten Systempfad gefunden: {error}")
    })?;
    if !canonical.starts_with(&canonical_root) || !canonical.is_file() {
        return Err(
            "Windows PowerShell liegt nicht innerhalb des erwarteten Systemverzeichnisses.".into(),
        );
    }
    Ok(canonical)
}

#[cfg(windows)]
pub(crate) fn hidden_powershell_command() -> Result<Command, String> {
    let mut command = Command::new(powershell_path()?);
    command.creation_flags(CREATE_NO_WINDOW);
    Ok(command)
}

#[cfg(windows)]
fn powershell_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(windows)]
fn encoded_command(script: &str) -> String {
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    STANDARD.encode(bytes)
}

#[cfg(windows)]
fn prepared_powershell_script(script: &str) -> String {
    format!(
        "$ErrorActionPreference = 'Stop'\n$ProgressPreference = 'SilentlyContinue'\n[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)\n$OutputEncoding = [Console]::OutputEncoding\n{script}"
    )
}

#[cfg(windows)]
pub(crate) fn run_encoded(script: &str) -> Result<std::process::Output, String> {
    let script = prepared_powershell_script(script);
    let child = hidden_powershell_command()?
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-EncodedCommand",
            &encoded_command(&script),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Windows PowerShell konnte nicht gestartet werden: {error}"))?;
    wait_with_output_timeout(child, POWERSHELL_TIMEOUT)
}

#[cfg(windows)]
fn wait_with_output_timeout(mut child: Child, timeout: Duration) -> Result<Output, String> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return child.wait_with_output().map_err(|error| {
                    format!("Windows PowerShell-Ausgabe konnte nicht gelesen werden: {error}")
                })
            }
            Ok(None) if started.elapsed() < timeout => {
                thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Windows PowerShell hat das Zeitlimit überschritten.".into());
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "Windows PowerShell-Status konnte nicht gelesen werden: {error}"
                ));
            }
        }
    }
}

#[cfg(windows)]
fn wide_null(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn run_elevated_encoded(script: &str) -> Result<(), String> {
    let executable = powershell_path()?;
    let prepared = prepared_powershell_script(script);
    let arguments = format!(
        "-NoLogo -NoProfile -NonInteractive -WindowStyle Hidden -EncodedCommand {}",
        encoded_command(&prepared)
    );
    let verb = wide_null(OsStr::new("runas"));
    let executable = wide_null(executable.as_os_str());
    let arguments = wide_null(OsStr::new(&arguments));
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: verb.as_ptr(),
        lpFile: executable.as_ptr(),
        lpParameters: arguments.as_ptr(),
        nShow: SW_HIDE,
        ..Default::default()
    };
    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        let error = unsafe { GetLastError() };
        if error == ERROR_CANCELLED {
            return Err("Die Administratorabfrage wurde abgebrochen.".into());
        }
        return Err(format!(
            "Die Administratorabfrage konnte nicht geöffnet werden (Windows-Fehler {error})."
        ));
    }
    if info.hProcess.is_null() {
        return Err("Windows hat keinen Prozess für die Administratoraktion zurückgegeben.".into());
    }
    let wait_result = unsafe { WaitForSingleObject(info.hProcess, INFINITE) };
    if wait_result == WAIT_FAILED {
        unsafe { CloseHandle(info.hProcess) };
        return Err("Auf die Administratoraktion konnte nicht gewartet werden.".into());
    }
    let mut exit_code = 1_u32;
    let exit_result = unsafe { GetExitCodeProcess(info.hProcess, &mut exit_code) };
    unsafe { CloseHandle(info.hProcess) };
    if exit_result == 0 {
        return Err("Der Status der Administratoraktion konnte nicht gelesen werden.".into());
    }
    if exit_code == 0 {
        Ok(())
    } else {
        Err(format!(
            "Windows konnte die Firewallregel nicht einrichten (Fehlercode {exit_code})."
        ))
    }
}

#[cfg(windows)]
fn parse_firewall_status(stdout: &[u8]) -> Result<FirewallStatus, String> {
    let text = std::str::from_utf8(stdout)
        .map_err(|_| "Die Windows-Firewall hat keine gültige UTF-8-Antwort geliefert.".to_string())?
        .trim_start_matches('\u{feff}')
        .trim();
    serde_json::from_str(text)
        .map_err(|error| format!("Die Antwort der Windows-Firewall war ungültig: {error}"))
}

#[cfg(windows)]
pub fn firewall_status(_app: &AppHandle, expected_port: u16) -> FirewallStatus {
    let program = match executable_path() {
        Ok(value) => value,
        Err(error) => {
            return FirewallStatus {
                detail: error,
                ..Default::default()
            }
        }
    };
    let script = format!(
        r#"
try {{
Import-Module (Join-Path $PSHOME 'Modules\NetSecurity\NetSecurity.psd1') -Force -ErrorAction Stop
$ruleName = {rule_name}
$programPath = {program_path}
$expectedPort = {expected_port}
$rule = Get-NetFirewallRule -DisplayName $ruleName -ErrorAction SilentlyContinue | Select-Object -First 1
if ($null -eq $rule) {{
    @{{ configured = $false; programPath = $null; port = $null; detail = 'Keine passende Windows-Firewallregel gefunden.' }} | ConvertTo-Json -Compress
    exit 0
}}
$appFilter = $rule | Get-NetFirewallApplicationFilter
$portFilter = $rule | Get-NetFirewallPortFilter
$addressFilter = $rule | Get-NetFirewallAddressFilter
$programMatches = [String]::Equals([string]$appFilter.Program, $programPath, [StringComparison]::OrdinalIgnoreCase)
$portMatches = [String]::Equals([string]$portFilter.LocalPort, [string]$expectedPort, [StringComparison]::OrdinalIgnoreCase)
$subnetMatches = [String]::Equals([string]$addressFilter.RemoteAddress, 'LocalSubnet', [StringComparison]::OrdinalIgnoreCase)
$protocolMatches = [String]::Equals([string]$portFilter.Protocol, 'TCP', [StringComparison]::OrdinalIgnoreCase) -or [string]$portFilter.Protocol -eq '6'
$profileMatches = [String]::Equals([string]$rule.Profile, 'Any', [StringComparison]::OrdinalIgnoreCase)
$edgeTraversalBlocked = [String]::Equals([string]$rule.EdgeTraversalPolicy, 'Block', [StringComparison]::OrdinalIgnoreCase)
$configured = $rule.Enabled -eq 'True' -and $rule.Direction -eq 'Inbound' -and $rule.Action -eq 'Allow' -and $programMatches -and $portMatches -and $subnetMatches -and $protocolMatches -and $profileMatches -and $edgeTraversalBlocked
$detail = if ($configured) {{ "Firewallregel ist für DMDC und TCP-Port $expectedPort eingerichtet." }} else {{ 'Die vorhandene Firewallregel passt nicht vollständig zu Programmpfad, Port und Sicherheitsumfang.' }}
@{{ configured = [bool]$configured; programPath = [string]$appFilter.Program; port = if ($portFilter.LocalPort -as [int]) {{ [int]$portFilter.LocalPort }} else {{ $null }}; detail = $detail }} | ConvertTo-Json -Compress
}} catch {{
    [Console]::Error.WriteLine($_.Exception.Message)
    exit 1
}}
"#,
        rule_name = powershell_literal(RULE_NAME),
        program_path = powershell_literal(&program.to_string_lossy()),
    );
    match run_encoded(&script) {
        Ok(output) if output.status.success() => match parse_firewall_status(&output.stdout) {
            Ok(status) => status,
            Err(error) => FirewallStatus {
                detail: error,
                ..Default::default()
            },
        },
        Ok(output) => FirewallStatus {
            detail: {
                let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
                if error.is_empty() {
                    "Die Windows-Firewallprüfung ist fehlgeschlagen.".into()
                } else {
                    error
                }
            },
            ..Default::default()
        },
        Err(error) => FirewallStatus {
            detail: format!("Firewallstatus konnte nicht geprüft werden: {error}"),
            ..Default::default()
        },
    }
}

#[cfg(not(windows))]
pub fn firewall_status(_app: &AppHandle, _expected_port: u16) -> FirewallStatus {
    FirewallStatus {
        detail: "Die Firewallintegration ist in v1 nur unter Windows verfügbar.".into(),
        ..Default::default()
    }
}

#[cfg(windows)]
pub fn configure_firewall(_app: &AppHandle, port: u16) -> Result<(), String> {
    let program = executable_path()?;
    let elevated_script = format!(
        r#"
Import-Module (Join-Path $PSHOME 'Modules\NetSecurity\NetSecurity.psd1') -Force -ErrorAction Stop
$ruleName = {rule_name}
$programPath = {program_path}
$port = {port}
if (-not (Test-Path -LiteralPath $programPath -PathType Leaf)) {{ throw 'Der angegebene DMDC-Programmpfad existiert nicht.' }}
Get-NetFirewallRule -DisplayName $ruleName -ErrorAction SilentlyContinue | Remove-NetFirewallRule
New-NetFirewallRule -DisplayName $ruleName -Description 'Lokaler DMDC-Dateitransfer im eigenen Subnetz' -Direction Inbound -Action Allow -Enabled True -Profile Any -Program $programPath -Protocol TCP -LocalPort $port -RemoteAddress LocalSubnet -EdgeTraversalPolicy Block | Out-Null
"#,
        rule_name = powershell_literal(RULE_NAME),
        program_path = powershell_literal(&program.to_string_lossy()),
    );
    run_elevated_encoded(&elevated_script)
}

#[cfg(not(windows))]
pub fn configure_firewall(_app: &AppHandle, _port: u16) -> Result<(), String> {
    Err("Die Firewallintegration ist in v1 nur unter Windows verfügbar.".into())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn powershell_json_is_utf8_even_with_german_text() {
        let output = run_encoded(
            "@{ configured = $true; programPath = 'C:\\DMDC.exe'; port = 8765; detail = 'Regel für DMDC' } | ConvertTo-Json -Compress",
        )
        .expect("PowerShell should start");
        assert!(output.status.success());
        let parsed = parse_firewall_status(&output.stdout).expect("valid UTF-8 JSON");
        assert!(parsed.configured);
        assert_eq!(parsed.detail, "Regel für DMDC");
    }

    #[test]
    fn powershell_timeout_terminates_a_stalled_process() {
        let script = encoded_command(&prepared_powershell_script("Start-Sleep -Seconds 5"));
        let child = hidden_powershell_command()
            .unwrap()
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-EncodedCommand",
                &script,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let started = Instant::now();
        let error = wait_with_output_timeout(child, Duration::from_millis(50)).unwrap_err();
        assert!(error.contains("Zeitlimit"));
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn firewall_parser_rejects_non_utf8_output() {
        let error = parse_firewall_status(&[0x7b, 0x81, 0x7d]).unwrap_err();
        assert!(error.contains("UTF-8"));
    }
}
