use crate::domain::types::FirewallStatus;
use tauri::AppHandle;

#[cfg(windows)]
use std::{ffi::OsStr, os::windows::ffi::OsStrExt, path::PathBuf};
#[cfg(windows)]
use windows::{
    core::{Interface, BSTR},
    Win32::{
        Foundation::{RPC_E_CHANGED_MODE, VARIANT_TRUE},
        NetworkManagement::WindowsFirewall::{
            INetFwPolicy2, INetFwRule, INetFwRule2, INetFwRules, NetFwPolicy2, NetFwRule,
            NET_FW_ACTION_ALLOW, NET_FW_EDGE_TRAVERSAL_TYPE_DENY, NET_FW_IP_PROTOCOL_TCP,
            NET_FW_PROFILE2_ALL, NET_FW_RULE_DIR_IN,
        },
        System::{
            Com::{
                CoCreateInstance, CoInitializeEx, CoUninitialize, IDispatch, CLSCTX_INPROC_SERVER,
                COINIT_MULTITHREADED,
            },
            Ole::IEnumVARIANT,
            Variant::VARIANT,
        },
    },
};
#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, ERROR_CANCELLED, WAIT_FAILED},
    Globalization::{CompareStringOrdinal, CSTR_EQUAL},
    System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE},
    UI::{
        Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW},
        WindowsAndMessaging::SW_HIDE,
    },
};

const RULE_NAME: &str = "LDTG Local Transfer";
const LEGACY_RULE_NAME: &str = "DMDC Local Transfer";
#[cfg(windows)]
const INTERNAL_CONFIGURE_ARG: &str = "--ldtg-internal-firewall-configure";

#[cfg(windows)]
pub(crate) struct ComApartment {
    must_uninitialize: bool,
}

#[cfg(windows)]
impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.must_uninitialize {
            unsafe { CoUninitialize() };
        }
    }
}

#[cfg(windows)]
pub(crate) fn initialize_com() -> Result<ComApartment, String> {
    let result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    if result.is_ok() {
        Ok(ComApartment {
            must_uninitialize: true,
        })
    } else if result == RPC_E_CHANGED_MODE {
        Ok(ComApartment {
            must_uninitialize: false,
        })
    } else {
        Err(format!(
            "Die Windows-COM-Laufzeit konnte nicht initialisiert werden (0x{:08X}).",
            result.0 as u32
        ))
    }
}

#[cfg(windows)]
fn executable_path() -> Result<PathBuf, String> {
    std::env::current_exe()
        .map_err(|error| format!("Der Programmpfad konnte nicht ermittelt werden: {error}"))
}

#[cfg(windows)]
fn wide_null(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn windows_equal_ignore_case(left: &str, right: &str) -> bool {
    let left = OsStr::new(left).encode_wide().collect::<Vec<_>>();
    let right = OsStr::new(right).encode_wide().collect::<Vec<_>>();
    unsafe {
        CompareStringOrdinal(
            left.as_ptr(),
            left.len() as i32,
            right.as_ptr(),
            right.len() as i32,
            1,
        ) == CSTR_EQUAL
    }
}

#[cfg(windows)]
fn is_product_rule_name(name: &str) -> bool {
    windows_equal_ignore_case(name, RULE_NAME) || windows_equal_ignore_case(name, LEGACY_RULE_NAME)
}

#[cfg(windows)]
fn windows_error(context: &str, error: windows::core::Error) -> String {
    format!(
        "{context} (Windows-Fehler 0x{:08X}).",
        error.code().0 as u32
    )
}

#[cfg(windows)]
fn with_firewall_rules<T>(
    operation: impl FnOnce(&INetFwRules) -> Result<T, String>,
) -> Result<T, String> {
    let _apartment = initialize_com()?;
    let policy: INetFwPolicy2 = unsafe {
        CoCreateInstance(&NetFwPolicy2, None, CLSCTX_INPROC_SERVER).map_err(|error| {
            windows_error(
                "Die Windows-Firewallrichtlinie konnte nicht geöffnet werden",
                error,
            )
        })?
    };
    let rules = unsafe { policy.Rules() }.map_err(|error| {
        windows_error(
            "Die Windows-Firewallregeln konnten nicht gelesen werden",
            error,
        )
    })?;
    operation(&rules)
}

#[cfg(windows)]
#[derive(Debug, Clone)]
struct FirewallRuleSnapshot {
    name: String,
    program: String,
    local_ports: String,
    remote_addresses: String,
    protocol: i32,
    profiles: i32,
    direction_in: bool,
    action_allow: bool,
    enabled: bool,
    edge_traversal_options: i32,
}

#[cfg(windows)]
impl FirewallRuleSnapshot {
    fn matches(&self, program: &str, port: u16) -> bool {
        windows_equal_ignore_case(&self.name, RULE_NAME)
            && windows_equal_ignore_case(&self.program, program)
            && self.local_ports == port.to_string()
            && windows_equal_ignore_case(&self.remote_addresses, "LocalSubnet")
            && self.protocol == NET_FW_IP_PROTOCOL_TCP.0
            && self.profiles == NET_FW_PROFILE2_ALL.0
            && self.direction_in
            && self.action_allow
            && self.enabled
            && self.edge_traversal_options == NET_FW_EDGE_TRAVERSAL_TYPE_DENY.0
    }
}

#[cfg(windows)]
fn product_rule_snapshots(rules: &INetFwRules) -> Result<Vec<FirewallRuleSnapshot>, String> {
    let unknown = unsafe { rules._NewEnum() }.map_err(|error| {
        windows_error(
            "Die Windows-Firewallregeln konnten nicht aufgezählt werden",
            error,
        )
    })?;
    let enumerator: IEnumVARIANT = unknown.cast().map_err(|error| {
        windows_error("Die Windows-Firewallauflistung ist nicht verfügbar", error)
    })?;
    let mut result = Vec::new();
    loop {
        let mut value = VARIANT::default();
        let mut fetched = 0_u32;
        let next = unsafe { enumerator.Next(std::slice::from_mut(&mut value), &mut fetched) };
        if next.is_err() {
            return Err(format!(
                "Eine Windows-Firewallregel konnte nicht gelesen werden (0x{:08X}).",
                next.0 as u32
            ));
        }
        if fetched == 0 {
            break;
        }
        let dispatch = IDispatch::try_from(&value).map_err(|error| {
            windows_error(
                "Eine Windows-Firewallregel hatte ein unerwartetes Format",
                error,
            )
        })?;
        let rule: INetFwRule = dispatch.cast().map_err(|error| {
            windows_error(
                "Eine Windows-Firewallregel konnte nicht geöffnet werden",
                error,
            )
        })?;
        let name = unsafe { rule.Name() }
            .map_err(|error| windows_error("Der Firewallregelname fehlt", error))?
            .to_string();
        if !is_product_rule_name(&name) {
            continue;
        }
        let rule2: INetFwRule2 = rule.cast().map_err(|error| {
            windows_error(
                "Die erweiterten Eigenschaften einer Windows-Firewallregel fehlen",
                error,
            )
        })?;
        result.push(FirewallRuleSnapshot {
            name,
            program: unsafe { rule.ApplicationName() }
                .map_err(|error| windows_error("Der Firewall-Programmpfad fehlt", error))?
                .to_string(),
            local_ports: unsafe { rule.LocalPorts() }
                .map_err(|error| windows_error("Der lokale Firewallport fehlt", error))?
                .to_string(),
            remote_addresses: unsafe { rule.RemoteAddresses() }
                .map_err(|error| windows_error("Der Firewall-Netzwerkbereich fehlt", error))?
                .to_string(),
            protocol: unsafe { rule.Protocol() }
                .map_err(|error| windows_error("Das Firewallprotokoll fehlt", error))?,
            profiles: unsafe { rule.Profiles() }
                .map_err(|error| windows_error("Das Firewallprofil fehlt", error))?,
            direction_in: unsafe { rule.Direction() }
                .map_err(|error| windows_error("Die Firewallrichtung fehlt", error))?
                == NET_FW_RULE_DIR_IN,
            action_allow: unsafe { rule.Action() }
                .map_err(|error| windows_error("Die Firewallaktion fehlt", error))?
                == NET_FW_ACTION_ALLOW,
            enabled: unsafe { rule.Enabled() }
                .map_err(|error| windows_error("Der Firewallstatus fehlt", error))?
                .0
                != 0,
            edge_traversal_options: unsafe { rule2.EdgeTraversalOptions() }
                .map_err(|error| windows_error("Der Edge-Traversal-Status fehlt", error))?,
        });
    }
    Ok(result)
}

#[cfg(windows)]
fn remove_product_rules(rules: &INetFwRules) -> Result<(), String> {
    for _ in 0..64 {
        let product_rules = product_rule_snapshots(rules)?;
        let Some(rule) = product_rules.first() else {
            return Ok(());
        };
        unsafe { rules.Remove(&BSTR::from(rule.name.as_str())) }.map_err(|error| {
            windows_error(
                "Eine vorhandene LDTG-Firewallregel konnte nicht entfernt werden",
                error,
            )
        })?;
    }
    Err("Zu viele gleichnamige LDTG-Firewallregeln wurden gefunden.".into())
}

#[cfg(windows)]
fn configure_firewall_native(port: u16) -> Result<(), String> {
    if !(1024..=65535).contains(&port) {
        return Err("Der Port muss zwischen 1024 und 65535 liegen.".into());
    }
    let program = executable_path()?;
    if !program.is_file() {
        return Err("Der angegebene LDTG-Programmpfad existiert nicht.".into());
    }
    let program = program.to_string_lossy().into_owned();
    with_firewall_rules(|rules| {
        remove_product_rules(rules)?;
        let rule: INetFwRule2 = unsafe {
            CoCreateInstance(&NetFwRule, None, CLSCTX_INPROC_SERVER).map_err(|error| {
                windows_error(
                    "Eine neue Windows-Firewallregel konnte nicht erzeugt werden",
                    error,
                )
            })?
        };
        unsafe {
            rule.SetName(&BSTR::from(RULE_NAME))
                .and_then(|_| {
                    rule.SetDescription(&BSTR::from(
                        "Lokaler LDTG-Dateitransfer im eigenen Subnetz",
                    ))
                })
                .and_then(|_| rule.SetApplicationName(&BSTR::from(program.as_str())))
                .and_then(|_| rule.SetProtocol(NET_FW_IP_PROTOCOL_TCP.0))
                .and_then(|_| rule.SetLocalPorts(&BSTR::from(port.to_string())))
                .and_then(|_| rule.SetRemoteAddresses(&BSTR::from("LocalSubnet")))
                .and_then(|_| rule.SetDirection(NET_FW_RULE_DIR_IN))
                .and_then(|_| rule.SetProfiles(NET_FW_PROFILE2_ALL.0))
                .and_then(|_| rule.SetEdgeTraversalOptions(NET_FW_EDGE_TRAVERSAL_TYPE_DENY.0))
                .and_then(|_| rule.SetAction(NET_FW_ACTION_ALLOW))
                .and_then(|_| rule.SetEnabled(VARIANT_TRUE))
                .and_then(|_| rules.Add(&rule))
        }
        .map_err(|error| {
            windows_error(
                "Die Windows-Firewallregel konnte nicht eingerichtet werden",
                error,
            )
        })?;
        let created = product_rule_snapshots(rules)?;
        if created.len() == 1 && created[0].matches(&program, port) {
            Ok(())
        } else {
            Err("Die Windows-Firewallregel entspricht nach dem Anlegen nicht dem erwarteten Sicherheitsumfang.".into())
        }
    })
}

#[cfg(windows)]
fn cleanup_firewall_native() -> Result<(), String> {
    with_firewall_rules(|rules| {
        remove_product_rules(rules)?;
        if product_rule_snapshots(rules)?.is_empty() {
            Ok(())
        } else {
            Err("Die LDTG-Firewallregeln konnten nicht vollständig entfernt werden.".into())
        }
    })
}

#[cfg(windows)]
fn run_elevated_self(arguments: &str) -> Result<(), String> {
    let executable = executable_path()?;
    let verb = wide_null(OsStr::new("runas"));
    let executable = wide_null(executable.as_os_str());
    let arguments = wide_null(OsStr::new(arguments));
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
            "Windows konnte die Firewallaktion nicht ausführen (Fehlercode {exit_code})."
        ))
    }
}

#[cfg(windows)]
#[derive(Debug, PartialEq, Eq)]
enum InternalFirewallCommand {
    Configure(u16),
}

#[cfg(windows)]
fn parse_internal_firewall_command(
    arguments: impl IntoIterator<Item = std::ffi::OsString>,
) -> Option<Result<InternalFirewallCommand, ()>> {
    let mut arguments = arguments.into_iter();
    let mode = arguments.next()?;
    if mode == INTERNAL_CONFIGURE_ARG {
        let Some(port) = arguments.next() else {
            return Some(Err(()));
        };
        if arguments.next().is_some() {
            return Some(Err(()));
        }
        let Some(port) = port.to_str().and_then(|value| value.parse::<u16>().ok()) else {
            return Some(Err(()));
        };
        return Some(if (1024..=65535).contains(&port) {
            Ok(InternalFirewallCommand::Configure(port))
        } else {
            Err(())
        });
    }
    None
}

#[cfg(windows)]
pub(crate) fn run_internal_firewall_command() -> Option<i32> {
    match parse_internal_firewall_command(std::env::args_os().skip(1))? {
        Ok(InternalFirewallCommand::Configure(port)) => {
            Some(if configure_firewall_native(port).is_ok() {
                0
            } else {
                1
            })
        }
        Err(()) => Some(2),
    }
}

#[cfg(not(windows))]
pub(crate) fn run_internal_firewall_command() -> Option<i32> {
    None
}

#[cfg(windows)]
pub(crate) fn run_firewall_cleanup_helper() -> i32 {
    if cleanup_firewall_native().is_ok() {
        0
    } else {
        1
    }
}

#[cfg(not(windows))]
pub(crate) fn run_firewall_cleanup_helper() -> i32 {
    1
}

#[cfg(windows)]
pub fn firewall_status(_app: &AppHandle, expected_port: u16) -> FirewallStatus {
    let program = match executable_path() {
        Ok(value) => value.to_string_lossy().into_owned(),
        Err(error) => {
            return FirewallStatus {
                detail: error,
                ..Default::default()
            }
        }
    };
    let rules = match with_firewall_rules(product_rule_snapshots) {
        Ok(value) => value,
        Err(error) => {
            return FirewallStatus {
                detail: format!("Firewallstatus konnte nicht geprüft werden: {error}"),
                ..Default::default()
            }
        }
    };
    if rules.is_empty() {
        return FirewallStatus {
            detail: "Keine passende Windows-Firewallregel gefunden.".into(),
            ..Default::default()
        };
    }
    let first = &rules[0];
    let configured = rules.len() == 1 && first.matches(&program, expected_port);
    FirewallStatus {
        configured,
        program_path: Some(first.program.clone()),
        port: first.local_ports.parse().ok(),
        detail: if configured {
            format!("Firewallregel ist für LDTG und TCP-Port {expected_port} eingerichtet.")
        } else {
            "Die vorhandenen Firewallregeln passen nicht vollständig zu Programmpfad, Port und Sicherheitsumfang.".into()
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
    if !(1024..=65535).contains(&port) {
        return Err("Der Port muss zwischen 1024 und 65535 liegen.".into());
    }
    run_elevated_self(&format!("{INTERNAL_CONFIGURE_ARG} {port}"))
}

#[cfg(not(windows))]
pub fn configure_firewall(_app: &AppHandle, _port: u16) -> Result<(), String> {
    Err("Die Firewallintegration ist in v1 nur unter Windows verfügbar.".into())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    fn valid_rule() -> FirewallRuleSnapshot {
        FirewallRuleSnapshot {
            name: RULE_NAME.into(),
            program: r"C:\Program Files\LDTG\ldtg.exe".into(),
            local_ports: "8765".into(),
            remote_addresses: "LocalSubnet".into(),
            protocol: NET_FW_IP_PROTOCOL_TCP.0,
            profiles: NET_FW_PROFILE2_ALL.0,
            direction_in: true,
            action_allow: true,
            enabled: true,
            edge_traversal_options: NET_FW_EDGE_TRAVERSAL_TYPE_DENY.0,
        }
    }

    #[test]
    fn firewall_rule_matcher_requires_the_complete_security_scope() {
        let expected_program = r"c:\program files\ldtg\LDTG.EXE";
        assert!(valid_rule().matches(expected_program, 8765));

        let mutations: [fn(&mut FirewallRuleSnapshot); 10] = [
            |rule| rule.name = LEGACY_RULE_NAME.into(),
            |rule| rule.program = r"C:\Other\ldtg.exe".into(),
            |rule| rule.local_ports = "8766".into(),
            |rule| rule.remote_addresses = "Any".into(),
            |rule| rule.protocol = 17,
            |rule| rule.profiles = 2,
            |rule| rule.direction_in = false,
            |rule| rule.action_allow = false,
            |rule| rule.enabled = false,
            |rule| rule.edge_traversal_options = 3,
        ];
        for mutate in mutations {
            let mut rule = valid_rule();
            mutate(&mut rule);
            assert!(!rule.matches(expected_program, 8765));
        }
    }

    #[test]
    fn internal_firewall_modes_are_fixed_constants() {
        assert_eq!(INTERNAL_CONFIGURE_ARG, "--ldtg-internal-firewall-configure");
        assert!(!INTERNAL_CONFIGURE_ARG.contains(' '));
        assert!(is_product_rule_name(RULE_NAME));
        assert!(is_product_rule_name(LEGACY_RULE_NAME));
        assert!(!is_product_rule_name("LDTG Local Transfer Backup"));

        let args = |values: &[&str]| {
            values
                .iter()
                .map(std::ffi::OsString::from)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            parse_internal_firewall_command(args(&[INTERNAL_CONFIGURE_ARG, "8765"])),
            Some(Ok(InternalFirewallCommand::Configure(8765)))
        );
        assert_eq!(
            parse_internal_firewall_command(args(&[INTERNAL_CONFIGURE_ARG, "80"])),
            Some(Err(()))
        );
        assert_eq!(
            parse_internal_firewall_command(args(&[
                INTERNAL_CONFIGURE_ARG,
                "8765",
                r"C:\attacker.exe",
            ])),
            Some(Err(()))
        );
        assert_eq!(
            parse_internal_firewall_command(args(&["--arbitrary-command"])),
            None
        );
    }

    #[test]
    fn native_firewall_query_reads_the_local_policy_without_elevation() {
        let rules = with_firewall_rules(product_rule_snapshots)
            .expect("the supported Windows host must allow read-only firewall inspection");
        assert!(rules.iter().all(|rule| is_product_rule_name(&rule.name)));
    }
}
