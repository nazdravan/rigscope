//! Network adapters from `SPNetworkDataType` (interface, kind, MAC) and `SPEthernetDataType`
//! (PCI ids and driver of wired cards). The Wi-Fi profiler (`SPAirPortDataType`) scans the air
//! for several seconds, so it is not used.

use crate::MacSource;
use crate::gpu::parse_hex_u16;
use rigscope_model::{Field, NetInfo, NetInterface, NetKind, clean_text};
use serde_json::Value;

/// Most interfaces read from the profiler output.
pub const MAX_INTERFACES: usize = 64;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NetEntry {
    pub interface: String,
    pub kind: Option<NetKind>,
    pub mac: Option<String>,
}

/// The PCI card behind an Ethernet interface.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EthernetCard {
    pub interface: String,
    pub name: Option<String>,
    pub vendor_id: Option<u16>,
    pub device_id: Option<u16>,
    pub driver: Option<String>,
}

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key)?.as_str().and_then(clean_text)
}

/// Interfaces that are not adapters of their own: bridges, tunnels, AWDL, low-latency WLAN.
pub fn is_virtual(name: &str) -> bool {
    [
        "bridge", "utun", "awdl", "llw", "lo", "gif", "stf", "ap", "vmnet", "vboxnet",
    ]
    .iter()
    .any(|p| {
        name.strip_prefix(p)
            .is_some_and(|rest| rest.chars().all(|c| c.is_ascii_digit()))
    })
}

pub fn parse_network(json: &str) -> Vec<NetEntry> {
    let Ok(root) = serde_json::from_str::<Value>(json) else {
        return Vec::new();
    };
    let Some(items) = root.get("SPNetworkDataType").and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|n| {
            let interface = text(n, "interface")?;
            let kind = match text(n, "hardware").as_deref() {
                Some("Ethernet") => Some(NetKind::Ethernet),
                Some("AirPort") => Some(NetKind::WiFi),
                _ => None,
            };
            let mac = n.get("Ethernet").and_then(|e| text(e, "MAC Address"));
            Some(NetEntry {
                interface,
                kind,
                mac,
            })
        })
        .take(MAX_INTERFACES)
        .collect()
}

pub fn parse_ethernet(json: &str) -> Vec<EthernetCard> {
    let Ok(root) = serde_json::from_str::<Value>(json) else {
        return Vec::new();
    };
    let Some(items) = root.get("SPEthernetDataType").and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|c| {
            Some(EthernetCard {
                interface: text(c, "spethernet_BSD_Device_Name")?,
                name: text(c, "_name"),
                vendor_id: text(c, "spethernet_vendor-id")
                    .as_deref()
                    .and_then(parse_hex_u16),
                device_id: text(c, "spethernet_device-id")
                    .as_deref()
                    .and_then(parse_hex_u16),
                driver: text(c, "spethernet_driver"),
            })
        })
        .take(MAX_INTERFACES)
        .collect()
}

/// The vendors of the cards Macs ship with or take; anything else shows its id.
fn pci_vendor(id: u16) -> Option<&'static str> {
    match id {
        0x14e4 => Some("Broadcom"),
        0x8086 => Some("Intel"),
        0x10ec => Some("Realtek"),
        0x106b => Some("Apple"),
        0x168c => Some("Qualcomm Atheros"),
        0x11ab => Some("Marvell"),
        _ => None,
    }
}

pub(crate) fn read_net(src: &dyn MacSource) -> NetInfo {
    let entries = src
        .profiler("SPNetworkDataType")
        .map(|j| parse_network(&j))
        .unwrap_or_default();
    let cards = src
        .profiler("SPEthernetDataType")
        .map(|j| parse_ethernet(&j))
        .unwrap_or_default();
    let mut interfaces = Vec::new();
    let mut hidden_virtual = 0u32;
    for e in &entries {
        // Only Ethernet and Wi-Fi adapters; modems and serial ports are not network cards here.
        let Some(kind) = e.kind else { continue };
        if is_virtual(&e.interface) {
            hidden_virtual = hidden_virtual.saturating_add(1);
            continue;
        }
        let card = cards.iter().find(|c| c.interface == e.interface);
        // A USB modem or phone shows up as "Ethernet" with neither a card nor a MAC address.
        if card.is_none() && e.mac.is_none() {
            hidden_virtual = hidden_virtual.saturating_add(1);
            continue;
        }
        let vendor = card
            .and_then(|c| c.vendor_id)
            .and_then(pci_vendor)
            .map(str::to_string);
        // "Broadcom 57766-A1" -> "57766-A1" once the vendor column says Broadcom.
        let device = card.and_then(|c| c.name.clone()).map(|n| match &vendor {
            Some(v) => n
                .strip_prefix(v.as_str())
                .map_or(n.clone(), |r| r.trim().to_string()),
            None => n,
        });
        interfaces.push(NetInterface {
            name: e.interface.clone(),
            kind: Field::Known(kind),
            mac: Field::from_option(e.mac.clone()),
            state: Field::Unknown,
            speed_mbps: Field::Unknown,
            duplex: Field::Unknown,
            mtu: Field::Unknown,
            driver: Field::from_option(card.and_then(|c| c.driver.clone())),
            vendor_name: Field::from_option(vendor),
            device_name: Field::from_option(device),
        });
    }
    NetInfo {
        interfaces,
        hidden_virtual,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NET: &str = r#"{"SPNetworkDataType":[
      {"_name":"Arduino","hardware":"Modem","interface":"usbmodem1","type":"PPP"},
      {"_name":"Ethernet","hardware":"Ethernet","interface":"en0","Ethernet":{"MAC Address":"78:7b:8a:d5:61:0c"}},
      {"_name":"Wi-Fi","hardware":"AirPort","interface":"en1","Ethernet":{"MAC Address":"24:f6:77:15:a6:be"}},
      {"_name":"Thunderbolt Bridge","hardware":"Ethernet","interface":"bridge0"}]}"#;
    const ETH: &str = r#"{"SPEthernetDataType":[{"_name":"Broadcom 57766-A1","spethernet_BSD_Device_Name":"en0","spethernet_vendor-id":"0x14e4","spethernet_device-id":"0x1686","spethernet_driver":"com.apple.iokit.AppleBCM5701Ethernet"}]}"#;

    #[test]
    fn virtual_names() {
        for n in ["bridge0", "utun3", "awdl0", "llw0", "lo0", "gif0"] {
            assert!(is_virtual(n), "{n}");
        }
        for n in ["en0", "en12", "bridgeX", "apple"] {
            assert!(!is_virtual(n), "{n}");
        }
    }

    #[test]
    fn ethernet_and_wifi_with_card_details() {
        use crate::MemMac;
        let m = MemMac::new()
            .with_profiler("SPNetworkDataType", NET)
            .with_profiler("SPEthernetDataType", ETH);
        let n = read_net(&m);
        assert_eq!(n.interfaces.len(), 2, "the modem is not a network card");
        assert_eq!(n.hidden_virtual, 1);
        let e = &n.interfaces[0];
        assert_eq!(e.name, "en0");
        assert_eq!(e.kind, Field::Known(NetKind::Ethernet));
        assert_eq!(e.vendor_name, Field::Known("Broadcom".into()));
        assert_eq!(e.device_name, Field::Known("57766-A1".into()));
        assert_eq!(e.mac, Field::Known("78:7b:8a:d5:61:0c".into()));
        let w = &n.interfaces[1];
        assert_eq!(w.kind, Field::Known(NetKind::WiFi));
        assert_eq!(w.vendor_name, Field::Unknown);
    }

    #[test]
    fn an_adapter_with_no_card_and_no_mac_is_hidden() {
        use crate::MemMac;
        let j = r#"{"SPNetworkDataType":[{"hardware":"Ethernet","interface":"en4"}]}"#;
        let n = read_net(&MemMac::new().with_profiler("SPNetworkDataType", j));
        assert!(n.interfaces.is_empty());
        assert_eq!(n.hidden_virtual, 1);
    }

    #[test]
    fn garbage_gives_nothing() {
        assert!(parse_network("x").is_empty());
        assert!(parse_ethernet("{}").is_empty());
    }
}
