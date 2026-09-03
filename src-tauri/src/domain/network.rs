use if_addrs::{get_if_addrs, IfAddr};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, UdpSocket},
};
use ts_rs::TS;

#[cfg(windows)]
use windows::Win32::{
    Networking::NetworkListManager::{
        INetworkListManager, NetworkListManager, NLM_NETWORK_CATEGORY,
        NLM_NETWORK_CATEGORY_DOMAIN_AUTHENTICATED, NLM_NETWORK_CATEGORY_PRIVATE,
        NLM_NETWORK_CATEGORY_PUBLIC,
    },
    System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER},
};

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
    let Ok(_apartment) = crate::platform::initialize_com() else {
        return HashMap::new();
    };
    let Ok(manager): Result<INetworkListManager, _> =
        (unsafe { CoCreateInstance(&NetworkListManager, None, CLSCTX_INPROC_SERVER) })
    else {
        return HashMap::new();
    };
    let Ok(connections) = (unsafe { manager.GetNetworkConnections() }) else {
        return HashMap::new();
    };
    let mut profiles = HashMap::new();
    loop {
        let mut item = [None];
        let mut fetched = 0_u32;
        let next = unsafe { connections.Next(&mut item, Some(&mut fetched)) };
        if next.is_err() || fetched == 0 {
            break;
        }
        let Some(connection) = item[0].take() else {
            continue;
        };
        if !unsafe { connection.IsConnected() }.is_ok_and(|connected| connected.0 != 0) {
            continue;
        }
        let Ok(adapter_id) = (unsafe { connection.GetAdapterId() }) else {
            continue;
        };
        let Ok(network) = (unsafe { connection.GetNetwork() }) else {
            continue;
        };
        let Ok(network_id) = (unsafe { network.GetNetworkId() }) else {
            continue;
        };
        let Ok(name) = (unsafe { network.GetName() }) else {
            continue;
        };
        let Ok(category) = (unsafe { network.GetCategory() }) else {
            continue;
        };
        profiles.insert(
            normalize_adapter_id(&format!("{adapter_id:?}")),
            (
                name.to_string(),
                network_category_name(category).into(),
                format!("{network_id:?}"),
            ),
        );
    }
    profiles
}

#[cfg(windows)]
fn normalize_adapter_id(value: &str) -> String {
    value
        .trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .to_ascii_lowercase()
}

#[cfg(windows)]
fn network_category_name(category: NLM_NETWORK_CATEGORY) -> &'static str {
    if category == NLM_NETWORK_CATEGORY_PUBLIC {
        "Öffentlich"
    } else if category == NLM_NETWORK_CATEGORY_PRIVATE {
        "Privat"
    } else if category == NLM_NETWORK_CATEGORY_DOMAIN_AUTHENTICATED {
        "Domänennetzwerk"
    } else {
        "Unbekannt"
    }
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
                #[cfg(windows)]
                let profile_key = normalize_adapter_id(&interface.adapter_name);
                #[cfg(not(windows))]
                let profile_key = interface.name.clone();
                let profile_resolved = !cfg!(windows) || profiles.contains_key(&profile_key);
                let (mut profile_name, category, profile_guid) =
                    profiles.get(&profile_key).cloned().unwrap_or_else(|| {
                        (
                            interface.name.clone(),
                            "Netzwerkprofil unbekannt".into(),
                            "unresolved".into(),
                        )
                    });
                if profile_name.is_empty() {
                    profile_name = interface.name.clone();
                }
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

pub fn trusted_network_matches(
    approved_id: &str,
    approved_category: &str,
    current: &NetworkInterfaceInfo,
) -> bool {
    current.profile_resolved
        && approved_category != "Unbekannt"
        && approved_id == current.network_id
        && approved_category == current.category
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

    #[test]
    fn persisted_trust_requires_the_approved_known_category() {
        let current = NetworkInterfaceInfo {
            id: "lan|192.168.1.20".into(),
            name: "lan".into(),
            profile_name: "Heimnetz".into(),
            address: Ipv4Addr::new(192, 168, 1, 20),
            prefix_length: 24,
            network_id: "{guid}|192.168.1.0/24".into(),
            category: "Privat".into(),
            profile_resolved: true,
            preferred: true,
            netmask: Ipv4Addr::new(255, 255, 255, 0),
        };

        assert!(trusted_network_matches(
            &current.network_id,
            "Privat",
            &current
        ));
        assert!(!trusted_network_matches(
            &current.network_id,
            "Öffentlich",
            &current
        ));
        assert!(!trusted_network_matches(
            &current.network_id,
            "Unbekannt",
            &current
        ));
        assert!(!trusted_network_matches(
            "{other}|192.168.1.0/24",
            "Privat",
            &current
        ));
    }

    #[cfg(windows)]
    #[test]
    fn native_windows_profile_values_preserve_identity_and_category() {
        assert_eq!(normalize_adapter_id("{ABC-123}"), "abc-123");
        assert_eq!(
            network_category_name(NLM_NETWORK_CATEGORY_PUBLIC),
            "Öffentlich"
        );
        assert_eq!(
            network_category_name(NLM_NETWORK_CATEGORY_PRIVATE),
            "Privat"
        );
        assert_eq!(
            network_category_name(NLM_NETWORK_CATEGORY_DOMAIN_AUTHENTICATED),
            "Domänennetzwerk"
        );
    }
}
