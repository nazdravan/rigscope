use crate::WinSource;
use rigscope_model::{Field, NetInfo, NetInterface, NetKind};

pub const MAX_INTERFACES: usize = 64;
const IF_ETHERNET: u32 = 6;
const IF_WIFI: u32 = 71;
/// The same reason the Linux probe gives (crates/rigscope-probe/src/net.rs, WIFI_NO_DUPLEX).
const WIFI_NO_DUPLEX: &str = "does not apply to Wi-Fi";

pub(crate) fn read_net(src: &dyn WinSource) -> NetInfo {
    let mut out = NetInfo::default();
    for a in src.net_adapters().unwrap_or_default() {
        // An NDIS filter row is a layer of another adapter, not an interface: neither listed
        // nor counted as hidden.
        if a.filter {
            continue;
        }
        let kind = match a.if_type {
            IF_ETHERNET => Some(NetKind::Ethernet),
            IF_WIFI => Some(NetKind::WiFi),
            _ => None,
        };
        let Some(kind) = kind.filter(|_| a.hardware) else {
            out.hidden_virtual = out.hidden_virtual.saturating_add(1);
            continue;
        };
        if out.interfaces.len() == MAX_INTERFACES {
            continue;
        }
        out.interfaces.push(NetInterface {
            name: a.name.clone(),
            kind: Field::Known(kind),
            mac: Field::from_option(a.mac.clone()),
            state: Field::Known(if a.up { "up" } else { "down" }.into()),
            speed_mbps: Field::from_option(
                a.speed_bps
                    .filter(|_| a.up)
                    .and_then(|b| u32::try_from(b / 1_000_000).ok())
                    .filter(|m| *m > 0),
            ),
            duplex: match kind {
                NetKind::WiFi => Field::Unavailable(WIFI_NO_DUPLEX.into()),
                NetKind::Ethernet => Field::Unknown,
            },
            mtu: Field::from_option(a.mtu),
            driver: Field::Unknown,
            vendor_name: Field::Unknown,
            device_name: Field::from_option(a.description.clone()),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::indexing_slicing,
        clippy::field_reassign_with_default
    )]
    use super::*;
    use crate::MemWin;
    use rigscope_win::NetAdapterRaw;

    fn nic(name: &str, if_type: u32, hw: bool) -> NetAdapterRaw {
        NetAdapterRaw {
            name: name.into(),
            description: Some("Intel(R) Ethernet Connection (17) I219-V".into()),
            if_type,
            up: true,
            speed_bps: Some(1_000_000_000),
            mac: Some("a8:a1:59:12:34:56".into()),
            mtu: Some(1500),
            hardware: hw,
            filter: false,
        }
    }

    #[test]
    fn physical_shown_virtual_counted() {
        let mut w = MemWin::default();
        w.net = Ok(vec![
            nic("Ethernet", 6, true),
            nic("Wi-Fi", 71, true),
            nic("vEthernet (WSL)", 6, false),
            nic("Loopback", 24, false),
        ]);
        let n = read_net(&w);
        assert_eq!(n.interfaces.len(), 2);
        assert_eq!(n.hidden_virtual, 2);
        assert_eq!(n.interfaces[0].kind, Field::Known(NetKind::Ethernet));
        assert_eq!(n.interfaces[0].speed_mbps, Field::Known(1000));
        assert_eq!(n.interfaces[0].state, Field::Known("up".into()));
        assert_eq!(n.interfaces[1].kind, Field::Known(NetKind::WiFi));
        assert!(matches!(n.interfaces[1].duplex, Field::Unavailable(_)));
    }

    #[test]
    fn filter_rows_are_neither_listed_nor_counted() {
        let filtered = |name: &str, if_type: u32, hw: bool| NetAdapterRaw {
            filter: true,
            ..nic(name, if_type, hw)
        };
        let mut w = MemWin::default();
        w.net = Ok(vec![
            nic("Ethernet", 6, true),
            filtered(
                "Ethernet-WFP Native MAC Layer LightWeight Filter-0000",
                6,
                false,
            ),
            filtered("Ethernet-QoS Packet Scheduler-0000", 6, false),
            filtered("Wi-Fi-Native WiFi Filter Driver-0000", 71, true),
            nic("vEthernet (WSL)", 6, false),
        ]);
        let n = read_net(&w);
        assert_eq!(n.interfaces.len(), 1);
        assert_eq!(n.interfaces[0].name, "Ethernet");
        assert_eq!(
            n.hidden_virtual, 1,
            "only the WSL switch is a hidden virtual adapter"
        );
    }

    #[test]
    fn capture_without_filter_field_still_loads() {
        let old = r#"{"name":"Ethernet","description":null,"if_type":6,"up":true,
            "speed_bps":null,"mac":null,"mtu":null,"hardware":true}"#;
        let a: NetAdapterRaw = serde_json::from_str(old).unwrap();
        assert!(!a.filter);
        // And a non-filter row serializes without the field, so old fixtures stay unchanged.
        assert!(!serde_json::to_string(&a).unwrap().contains("filter"));
    }
}
