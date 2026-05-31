use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv6Addr};

use sysinfo::{MacAddr, Networks};

use crate::types::NetworkInfo;

pub fn collect_network_info() -> NetworkInfo {
    let networks = Networks::new_with_refreshed_list();
    let mut private_ips = BTreeSet::new();
    let mut mac_addresses = BTreeSet::new();
    let mut saw_wifi = false;
    let mut saw_ethernet = false;

    for (interface_name, network) in &networks {
        let lower_name = interface_name.to_ascii_lowercase();
        saw_wifi |= lower_name.contains("wi-fi")
            || lower_name.contains("wifi")
            || lower_name.contains("wlan");
        saw_ethernet |= lower_name.contains("ethernet")
            || lower_name.starts_with("eth")
            || lower_name.starts_with("en");

        let mac = network.mac_address();
        if !mac.is_unspecified() {
            mac_addresses.insert(format_mac(mac));
        }

        for ip_network in network.ip_networks() {
            if is_private_ip(ip_network.addr) {
                private_ips.insert(ip_network.addr.to_string());
            }
        }
    }

    NetworkInfo {
        network_type: network_type(saw_wifi, saw_ethernet),
        private_ips: private_ips.into_iter().collect(),
        mac_addresses: mac_addresses.into_iter().collect(),
        gateway_ip: None,
        ssid: None,
        bssid_hash: None,
        public_ip_seen_by_server: None,
    }
}

fn network_type(saw_wifi: bool, saw_ethernet: bool) -> String {
    match (saw_wifi, saw_ethernet) {
        (true, _) => "wifi".to_string(),
        (false, true) => "ethernet".to_string(),
        (false, false) => "unknown".to_string(),
    }
}

fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_private() && !ip.is_loopback(),
        IpAddr::V6(ip) => is_unique_local_ipv6(ip) && !ip.is_loopback(),
    }
}

fn is_unique_local_ipv6(ip: Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xfe00) == 0xfc00
}

fn format_mac(mac: MacAddr) -> String {
    mac.to_string().to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;

    #[test]
    fn is_private_ip_should_accept_rfc1918() {
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 10, 5))));
    }

    #[test]
    fn is_private_ip_should_reject_loopback() {
        assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
    }
}
