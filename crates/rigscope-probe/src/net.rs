//! Network interfaces from `/sys/class/net` (no root). Virtual ones are counted, not listed.

use crate::Source;
use crate::sysfs::{hex_id, pci_ids, text};
use rigscope_model::{Field, NetInfo, NetInterface, NetKind, clean_text};
use rigscope_procfs::pci_ids as ids;

pub const NET_DIR: &str = "/sys/class/net";
pub const MAX_INTERFACES: usize = 128;
pub const WIFI_NO_SPEED: &str = "not reported for Wi-Fi";
/// Wi-Fi drivers reject reads of `duplex` (EINVAL): the notion belongs to wired links.
pub const WIFI_NO_DUPLEX: &str = "does not apply to Wi-Fi";

pub fn read_net(src: &dyn Source) -> NetInfo {
    let Ok(names) = src.list(NET_DIR) else {
        return NetInfo::default();
    };
    let table = pci_ids(src);
    let mut out = NetInfo::default();
    for name in names
        .iter()
        .filter(|n| n.as_str() != "lo")
        .take(MAX_INTERFACES)
    {
        let base = format!("{NET_DIR}/{name}");
        let canonical = src.canonical(&base).unwrap_or_default();
        if canonical.contains("/virtual/") {
            out.hidden_virtual = out.hidden_virtual.saturating_add(1);
            continue;
        }
        out.interfaces
            .push(read_interface(src, name, &base, table.as_deref()));
    }
    out
}

fn read_interface(src: &dyn Source, name: &str, base: &str, table: Option<&str>) -> NetInterface {
    let read = |f: &str| text(src, &format!("{base}/{f}"));
    let wifi = src.canonical(&format!("{base}/phy80211")).is_ok()
        || src.list(&format!("{base}/wireless")).is_ok();
    let kind = if wifi {
        NetKind::WiFi
    } else {
        NetKind::Ethernet
    };
    let speed = match read("speed").and_then(|s| s.parse::<i64>().ok()) {
        Some(v) if v > 0 => u32::try_from(v).map_or(Field::Unknown, Field::Known),
        _ if wifi => Field::Unavailable(WIFI_NO_SPEED.to_string()),
        _ => Field::Unknown,
    };
    let duplex = match read("duplex").filter(|d| d == "full" || d == "half") {
        Some(d) => Field::Known(d),
        None if wifi => Field::Unavailable(WIFI_NO_DUPLEX.to_string()),
        None => Field::Unknown,
    };
    let mac = read("address")
        .as_deref()
        .and_then(clean_text)
        .filter(|m| m != "00:00:00:00:00:00");
    let dev = format!("{base}/device");
    let (vendor_name, device_name) = match (
        hex_id(text(src, &format!("{dev}/vendor"))),
        hex_id(text(src, &format!("{dev}/device"))),
        table,
    ) {
        (Some(v), Some(d), Some(t)) => {
            let n = ids::lookup(t, v, d, None);
            (Field::from_option(n.vendor), Field::from_option(n.device))
        }
        (Some(_), Some(_), None) => {
            let u = || Field::Unavailable(crate::NO_PCI_IDS.to_string());
            (u(), u())
        }
        _ => (Field::Unknown, Field::Unknown),
    };
    NetInterface {
        name: name.to_string(),
        kind: Field::Known(kind),
        mac: Field::from_option(mac),
        state: Field::from_option(read("operstate").filter(|s| !s.is_empty() && s != "unknown")),
        speed_mbps: speed,
        duplex,
        mtu: Field::from_option(read("mtu").and_then(|m| m.parse::<u32>().ok())),
        driver: Field::from_option(crate::sysfs::driver(src, &dev)),
        vendor_name,
        device_name,
    }
}
