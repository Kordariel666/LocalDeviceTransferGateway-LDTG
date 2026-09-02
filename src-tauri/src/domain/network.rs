use if_addrs::{get_if_addrs, IfAddr};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, UdpSocket},
};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInterfaceInfo {
    pub id: String,
    pub name: String,
    pub profile_name: String,
    pub address: Ipv4Addr,
    pub prefix_length: u8,
    pub network_id: String,
    pub category: String,
    pub profile_resolved: bool,
    pub preferred: bool,
    #[serde(skip, default = "unspecified_ipv4")]
    #[ts(skip)]
    pub netmask: Ipv4Addr,
}

fn unspecified_ipv4() -> Ipv4Addr {
    Ipv4Addr::UNSPECIFIED
}

fn prefix_length(mask: Ipv4Addr) -> u8 {
    u32::from(mask).count_ones() as u8
}

fn preferred_ip() -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect((Ipv4Addr::new(192, 0, 2, 1), 9)).ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(ip) => Some(ip),
        _ => None,
    }
}

fn virtual_adapter(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    [
        "vpn",
        "virtual",
        "hyper-v",
        "vmware",
        "virtualbox",
        "wsl",
        "tailscale",
        "zerotier",
        "loopback",
        "tunnel",
    ]
    .iter()
    .any(|marker| name.contains(marker))
}

#[cfg(windows)]
pub fn peer_fairness_key(address: IpAddr) -> String {
    use std::{ptr, slice};
    use windows_sys::Win32::{
        NetworkManagement::IpHelper::{FreeMibTable, GetIpNetTable2, MIB_IPNET_ROW2},
        Networking::WinSock::AF_INET,
    };

    let IpAddr::V4(address) = address else {
        return format!("ip:{address}");
    };
    let mut table = ptr::null_mut();
    if unsafe { GetIpNetTable2(AF_INET, &mut table) } != 0 || table.is_null() {
        return format!("ip:{address}");
    }
    struct TableGuard(*mut core::ffi::c_void);
    impl Drop for TableGuard {
        fn drop(&mut self) {
            unsafe { FreeMibTable(self.0) };
        }
    }
    let _guard = TableGuard(table.cast());
    let rows = unsafe {
        slice::from_raw_parts(
            ptr::addr_of!((*table).Table).cast::<MIB_IPNET_ROW2>(),
            (*table).NumEntries as usize,
        )
    };
    let expected = address.octets();
    rows.iter()
        .find_map(|row| {
            let ipv4 = unsafe { row.Address.Ipv4 };
            let bytes = unsafe { ipv4.sin_addr.S_un.S_un_b };
            if [bytes.s_b1, bytes.s_b2, bytes.s_b3, bytes.s_b4] != expected {
                return None;
            }
            let length = (row.PhysicalAddressLength as usize).min(row.PhysicalAddress.len());
            (length > 0).then(|| {
                let physical = row.PhysicalAddress[..length]
                    .iter()
                    .map(|value| format!("{value:02x}"))
                    .collect::<String>();
                format!("neighbor:{}:{physical}", row.InterfaceIndex)
            })
        })
        .unwrap_or_else(|| format!("ip:{address}"))
}

#[cfg(not(windows))]
pub fn peer_fairness_key(address: IpAddr) -> String {
    format!("ip:{address}")
}

#[cfg(windows)]
fn windows_profiles() -> HashMap<String, (String, String, String)> {
    let script = r#"
$profiles = Get-NetConnectionProfile -ErrorAction Stop | ForEach-Object {
  [pscustomobject]@{
    InterfaceAlias = [string]$_.InterfaceAlias
    Name = [string]$_.Name
    NetworkCategory = [string]$_.NetworkCategory
    NetworkGuid = [string]$_.InstanceID
  }
}
@($profiles) | ConvertTo-Json -Compress
"#;
    let Ok(output) = crate::platform::run_encoded(script) else {
        return HashMap::new();
    };
    if !output.status.success() {
        return HashMap::new();
    }
    parse_windows_profiles(&output.stdout)
}

#[cfg(windows)]
fn parse_windows_profiles(bytes: &[u8]) -> HashMap<String, (String, String, String)> {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return HashMap::new();
    };
    let values = value.as_array().cloned().unwrap_or_else(|| vec![value]);
    values
        .into_iter()
        .filter_map(|item| {
            let alias = item.get("InterfaceAlias")?.as_str()?.to_string();
            let name = item
                .get("Name")
                .and_then(|value| value.as_str())
                .filter(|value| !value.is_empty())
                .unwrap_or(&alias)
                .to_string();
            let category = match item.get("NetworkCategory").and_then(|value| value.as_str()) {
                Some("Public") => "Öffentlich",
                Some("Private") => "Privat",
                Some("DomainAuthenticated") => "Domänennetzwerk",
                Some(value) => value,
                None => "Unbekannt",
            }
            .to_string();
            let network_guid = item.get("NetworkGuid")?.as_str()?.to_string();
            (!alias.is_empty() && !network_guid.is_empty())
                .then_some((alias, (name, category, network_guid)))
        })
        .collect()
}

#[cfg(not(windows))]
fn windows_profiles() -> HashMap<String, (String, String, String)> {
    HashMap::new()
}

pub fn list_interfaces() -> Vec<NetworkInterfaceInfo> {
    let preferred = preferred_ip();
    let profiles = windows_profiles();
    let mut result = Vec::new();
    if let Ok(interfaces) = get_if_addrs() {
        for interface in interfaces {
            if let IfAddr::V4(value) = interface.addr {
                if value.ip.is_loopback() || value.ip.is_link_local() || !value.ip.is_private() {
                    continue;
                }
                let profile_resolved = !cfg!(windows) || profiles.contains_key(&interface.name);
                let (profile_name, category, profile_guid) =
                    profiles.get(&interface.name).cloned().unwrap_or_else(|| {
                        (
                            interface.name.clone(),
                            "Netzwerkprofil unbekannt".into(),
                            "unresolved".into(),
                        )
                    });
                let prefix = prefix_length(value.netmask);
                let network_address =
                    Ipv4Addr::from(u32::from(value.ip) & u32::from(value.netmask));
                let id = format!("{}|{}", interface.name, value.ip);
                let network_id = format!("{profile_guid}|{network_address}/{prefix}");
                result.push(NetworkInterfaceInfo {
                    id,
                    name: interface.name.clone(),
                    profile_name,
                    address: value.ip,
                    prefix_length: prefix,
                    network_id,
                    category,
                    profile_resolved,
                    preferred: preferred == Some(value.ip) && !virtual_adapter(&interface.name),
                    netmask: value.netmask,
                });
            }
        }
    }
    result.sort_by_key(|item| {
        (
            !item.preferred,
            virtual_adapter(&item.name),
            item.name.to_lowercase(),
            item.address,
        )
    });
    result
}

pub fn select_interface(id: Option<&str>) -> Result<NetworkInterfaceInfo, String> {
    let interfaces = list_interfaces();
    if let Some(id) = id {
        return interfaces
            .into_iter()
            .find(|item| item.id == id && item.profile_resolved)
            .ok_or_else(|| {
                "Die ausgewählte Netzwerkschnittstelle ist nicht verfügbar oder ihr Windows-Netzwerkprofil konnte nicht sicher bestimmt werden.".into()
            });
    }
    interfaces
        .iter()
        .find(|item| item.preferred && item.profile_resolved)
        .cloned()
        .or_else(|| {
            interfaces
                .iter()
                .find(|item| item.profile_resolved && !virtual_adapter(&item.name))
                .cloned()
        })
        .or_else(|| interfaces.into_iter().find(|item| item.profile_resolved))
        .ok_or_else(|| "Keine private IPv4-Netzwerkschnittstelle mit sicher bestimmtem Netzwerkprofil gefunden.".into())
}

pub fn same_subnet(client: Ipv4Addr, interface: &NetworkInterfaceInfo) -> bool {
    u32::from(client) & u32::from(interface.netmask)
        == u32::from(interface.address) & u32::from(interface.netmask)
}

pub fn same_network_identity(
    expected: &NetworkInterfaceInfo,
    current: &NetworkInterfaceInfo,
) -> bool {
    expected.profile_resolved
        && current.profile_resolved
        && current.id == expected.id
        && current.netmask == expected.netmask
        && current.network_id == expected.network_id
        && current.category == expected.category
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn subnet_check_is_exact() {
        let interface = NetworkInterfaceInfo {
            id: "x".into(),
            name: "x".into(),
            profile_name: "Heimnetz".into(),
            address: Ipv4Addr::new(192, 168, 1, 20),
            prefix_length: 24,
            network_id: "x".into(),
            category: "x".into(),
            profile_resolved: true,
            preferred: true,
            netmask: Ipv4Addr::new(255, 255, 255, 0),
        };
        assert!(same_subnet(Ipv4Addr::new(192, 168, 1, 99), &interface));
        assert!(!same_subnet(Ipv4Addr::new(192, 168, 2, 99), &interface));
    }

    #[test]
    fn profile_change_is_not_the_same_confirmed_network() {
        let expected = NetworkInterfaceInfo {
            id: "lan|192.168.1.20".into(),
            name: "lan".into(),
            profile_name: "Heimnetz".into(),
            address: Ipv4Addr::new(192, 168, 1, 20),
            prefix_length: 24,
            network_id: "lan|Heimnetz|192.168.1.0/24".into(),
            category: "Privat".into(),
            profile_resolved: true,
            preferred: true,
            netmask: Ipv4Addr::new(255, 255, 255, 0),
        };
        let mut changed = expected.clone();
        changed.profile_name = "Gastnetz".into();
        changed.network_id = "lan|Gastnetz|192.168.1.0/24".into();

        assert!(!same_network_identity(&expected, &changed));
        assert!(same_network_identity(&expected, &expected));
    }

    #[cfg(windows)]
    #[test]
    fn windows_profile_output_preserves_identity_and_category() {
        let profiles = parse_windows_profiles(
            br#"[{"InterfaceAlias":"Ethernet","Name":"Heimnetz","NetworkCategory":"Private","NetworkGuid":"{1234}"}]"#,
        );
        assert_eq!(
            profiles.get("Ethernet"),
            Some(&("Heimnetz".into(), "Privat".into(), "{1234}".into()))
        );
    }
}
