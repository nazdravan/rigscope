//! Masking of a capture before it is written to a fixture (plain Rust, tested on every OS).

use crate::MemMac;
use serde_json::Value;

/// Keys whose string values identify one machine: serial numbers and UUIDs.
const SECRET_KEYS: &[&str] = &[
    "serial_number",
    "dimm_serial_number",
    "platform_UUID",
    "provisioning_UDID",
    "device_serial",
    "MAC Address",
    "spethernet_mac_address",
];

/// Overwrites every letter and digit of a secret with `X`, keeping the length and the
/// punctuation (so `0x37781227` stays a `0x` plus eight characters).
pub fn mask_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        out.push(if c.is_ascii_alphanumeric() { 'X' } else { c });
    }
    // Keep a `0x` prefix readable.
    if s.starts_with("0x") || s.starts_with("0X") {
        out.replace_range(..2, &s.chars().take(2).collect::<String>());
    }
    out
}

fn mask_value(v: &mut Value) {
    match v {
        Value::Object(map) => {
            // Volume names, mount points and UUIDs say what the owner keeps on the disk.
            map.remove("volumes");
            // A display's EDID, serial number and product ids: keep only that one is attached.
            if let Some(Value::Array(displays)) = map.get_mut("spdisplays_ndrvs") {
                for d in displays.iter_mut() {
                    *d = serde_json::json!({ "_name": "display" });
                }
            }
            for (k, val) in map.iter_mut() {
                if SECRET_KEYS.contains(&k.as_str()) {
                    if let Value::String(s) = val {
                        *s = mask_text(s);
                        continue;
                    }
                }
                mask_value(val);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(mask_value),
        _ => {}
    }
}

/// The host name a captured fixture carries.
pub const CAPTURED_NAME: &str = "CAPTURED-MAC";

/// `SPNetworkDataType` lists every service with its addresses, DNS servers and the names of USB
/// modems. Keep only the Ethernet and Wi-Fi adapters, with their interface and MAC.
fn slim_network(v: &mut Value) {
    let Some(Value::Array(items)) = v.get_mut("SPNetworkDataType") else {
        return;
    };
    let slim: Vec<Value> = items
        .iter()
        .filter_map(|n| {
            let hardware = n.get("hardware")?.as_str()?;
            let label = match hardware {
                "Ethernet" => "Ethernet",
                "AirPort" => "Wi-Fi",
                _ => return None,
            };
            let mut out = serde_json::json!({
                "_name": label,
                "hardware": hardware,
                "interface": n.get("interface")?.clone(),
            });
            if let Some(mac) = n.get("Ethernet").and_then(|e| e.get("MAC Address")) {
                if let Some(obj) = out.as_object_mut() {
                    obj.insert(
                        "Ethernet".into(),
                        serde_json::json!({ "MAC Address": mac.clone() }),
                    );
                }
            }
            Some(out)
        })
        .collect();
    *items = slim;
}

/// What `--capture-fixture` writes: `m` with serial numbers and UUIDs masked and the host name
/// replaced.
pub fn mask_fixture(mut m: MemMac) -> MemMac {
    if let Some(n) = m.profiler.get_mut("SPNetworkDataType") {
        slim_network(n);
    }
    m.profiler.values_mut().for_each(mask_value);
    if let Some(h) = m.sysctl.get_mut("kern.hostname") {
        *h = CAPTURED_NAME.to_string();
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serials_and_uuids_are_masked_everywhere() {
        let m = MemMac::new()
            .with_profiler(
                "SPHardwareDataType",
                r#"{"SPHardwareDataType":[{"serial_number":"C02WH23VJ1G5","platform_UUID":"483F5EC7-76C9-58B9-92D8-67DFC6279A00","machine_model":"iMac18,2"}]}"#,
            )
            .with_profiler(
                "SPMemoryDataType",
                r#"{"SPMemoryDataType":[{"_items":[{"dimm_serial_number":"0x37781227","dimm_size":"8 GB"}]}]}"#,
            );
        let text = serde_json::to_string(&mask_fixture(m)).unwrap();
        assert!(!text.contains("C02WH23VJ1G5"), "{text}");
        assert!(!text.contains("483F5EC7"), "{text}");
        assert!(!text.contains("37781227"), "{text}");
        assert!(text.contains("XXXXXXXXXXXX"));
        assert!(text.contains("0xXXXXXXXX"));
        assert!(text.contains("iMac18,2"));
        assert!(text.contains("8 GB"));
    }

    #[test]
    fn disk_volumes_serials_and_display_ids_are_removed() {
        let m = MemMac::new()
            .with_profiler(
                "SPSerialATADataType",
                r#"{"SPSerialATADataType":[{"_items":[{"device_serial":"157S102STGUW","bsd_name":"disk0","volumes":[{"_name":"Alice Photos","volume_uuid":"U"}]}]}]}"#,
            )
            .with_profiler(
                "SPDisplaysDataType",
                r#"{"SPDisplaysDataType":[{"sppci_model":"Radeon","spdisplays_ndrvs":[{"_spdisplays_display-serial-number":"1be22532","_IODisplayEDID":"{length = 384}"}]}]}"#,
            );
        let text = serde_json::to_string(&mask_fixture(m)).unwrap();
        for secret in [
            "157S102STGUW",
            "Alice Photos",
            "1be22532",
            "EDID",
            "volume_uuid",
        ] {
            assert!(!text.contains(secret), "{secret}: {text}");
        }
        assert!(text.contains("disk0"));
        assert!(text.contains(r#""spdisplays_ndrvs":[{"_name":"display"}]"#));
    }

    #[test]
    fn the_network_capture_keeps_only_adapters_and_macs() {
        let m = MemMac::new().with_profiler(
            "SPNetworkDataType",
            r#"{"SPNetworkDataType":[
              {"_name":"Arduino Leonardo","hardware":"Modem","interface":"usbmodem1"},
              {"_name":"My Home Wi-Fi Adapter","hardware":"AirPort","interface":"en1",
               "ip_address":["192.168.1.23"],"dhcp":{"dhcp_routers":"192.168.1.1"},
               "Ethernet":{"MAC Address":"24:f6:77:15:a6:be","MediaSubType":"autoselect"}}]}"#,
        );
        let text = serde_json::to_string(&mask_fixture(m)).unwrap();
        for secret in ["Arduino", "192.168", "My Home", "24:f6:77", "dhcp"] {
            assert!(!text.contains(secret), "{secret}: {text}");
        }
        assert!(text.contains("XX:XX:XX:XX:XX:XX"));
        assert!(text.contains(r#""interface":"en1""#));
        assert!(text.contains(r#""_name":"Wi-Fi""#));
    }

    #[test]
    fn the_host_name_is_replaced() {
        let m = MemMac::new().with_sysctl("kern.hostname", "alice-imac.local");
        assert_eq!(mask_fixture(m).sysctl["kern.hostname"], CAPTURED_NAME);
    }

    #[test]
    fn masking_keeps_length_and_punctuation() {
        assert_eq!(mask_text("AB-12"), "XX-XX");
        assert_eq!(mask_text("0xAB"), "0xXX");
        assert_eq!(mask_text(""), "");
    }
}
