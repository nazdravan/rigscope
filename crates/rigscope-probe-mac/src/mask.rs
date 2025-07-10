//! Masking of a capture before it is written to a fixture (plain Rust, tested on every OS).

use crate::MemMac;
use serde_json::Value;

/// Keys whose string values identify one machine: serial numbers and UUIDs.
const SECRET_KEYS: &[&str] = &[
    "serial_number",
    "dimm_serial_number",
    "platform_UUID",
    "provisioning_UDID",
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

/// What `--capture-fixture` writes: `m` with serial numbers and UUIDs masked and the host name
/// replaced.
pub fn mask_fixture(mut m: MemMac) -> MemMac {
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
