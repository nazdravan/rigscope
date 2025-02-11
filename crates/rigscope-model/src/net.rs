use crate::Field;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NetInfo {
    /// Physical interfaces (Ethernet, Wi-Fi). Loopback is never listed.
    pub interfaces: Vec<NetInterface>,
    /// Virtual interfaces left out (bridges, veth, tun, docker0…).
    pub hidden_virtual: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NetInterface {
    pub name: String,
    pub kind: Field<NetKind>,
    pub mac: Field<String>,
    /// `up`, `down`, `dormant`… as the kernel reports `operstate`.
    pub state: Field<String>,
    pub speed_mbps: Field<u32>,
    pub duplex: Field<String>,
    pub mtu: Field<u32>,
    pub driver: Field<String>,
    pub vendor_name: Field<String>,
    pub device_name: Field<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetKind {
    Ethernet,
    WiFi,
}

impl NetKind {
    pub fn label(self) -> &'static str {
        match self {
            NetKind::Ethernet => "Ethernet",
            NetKind::WiFi => "Wi-Fi",
        }
    }
}

/// `1000 Mb/s` → `1 Gb/s`; `2500` → `2.5 Gb/s`; `100` → `100 Mb/s`.
pub fn format_speed(mbps: u32) -> String {
    if mbps < 1000 {
        format!("{mbps} Mb/s")
    } else if mbps % 1000 == 0 {
        format!("{} Gb/s", mbps / 1000)
    } else {
        format!("{:.1} Gb/s", f64::from(mbps) / 1000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speeds() {
        assert_eq!(format_speed(100), "100 Mb/s");
        assert_eq!(format_speed(1000), "1 Gb/s");
        assert_eq!(format_speed(2500), "2.5 Gb/s");
        assert_eq!(format_speed(10000), "10 Gb/s");
    }
}
