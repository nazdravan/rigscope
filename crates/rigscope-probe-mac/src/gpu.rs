//! Graphics cards from `SPDisplaysDataType`.

use crate::MacSource;
use crate::parse::parse_size;
use rigscope_model::{Field, Gpu, GpuInfo, clean_text};
use serde_json::Value;

/// Most cards read from the profiler output.
pub const MAX_GPUS: usize = 16;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GpuEntry {
    pub model: Option<String>,
    /// `AMD`, `NVIDIA`, `Intel`, `Apple`.
    pub vendor: Option<String>,
    pub vendor_id: Option<u16>,
    pub device_id: Option<u16>,
    pub vram_bytes: Option<u64>,
    /// `x8`, `x16`.
    pub pcie_width: Option<String>,
    /// A display is attached to this card.
    pub has_display: bool,
}

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key)?.as_str().and_then(clean_text)
}

/// `0x67ef` -> 0x67ef.
pub fn parse_hex_u16(s: &str) -> Option<u16> {
    let t = s.trim();
    let digits = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X"))?;
    u16::from_str_radix(digits, 16).ok()
}

/// `sppci_vendor_amd` -> ("AMD", 0x1002). Apple's GPUs are not on PCI.
fn vendor(code: &str) -> (Option<String>, Option<u16>) {
    let name = code
        .trim()
        .strip_prefix("sppci_vendor_")
        .unwrap_or(code.trim());
    match name.to_ascii_lowercase().as_str() {
        "amd" => (Some("AMD".into()), Some(0x1002)),
        "nvidia" => (Some("NVIDIA".into()), Some(0x10de)),
        "intel" => (Some("Intel".into()), Some(0x8086)),
        "apple" => (Some("Apple".into()), None),
        "" => (None, None),
        _ => (clean_text(name), None),
    }
}

pub fn parse_displays(json: &str) -> Vec<GpuEntry> {
    let Ok(root) = serde_json::from_str::<Value>(json) else {
        return Vec::new();
    };
    let Some(items) = root.get("SPDisplaysDataType").and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .filter(|g| g.get("sppci_model").is_some() || g.get("spdisplays_vendor").is_some())
        .take(MAX_GPUS)
        .map(|g| {
            let (vendor_name, vendor_id) =
                text(g, "spdisplays_vendor").map_or((None, None), |v| vendor(&v));
            GpuEntry {
                model: text(g, "sppci_model"),
                vendor: vendor_name,
                vendor_id,
                device_id: text(g, "spdisplays_device-id")
                    .as_deref()
                    .and_then(parse_hex_u16),
                vram_bytes: text(g, "spdisplays_vram").as_deref().and_then(parse_size),
                pcie_width: text(g, "spdisplays_pcie_width"),
                has_display: g
                    .get("spdisplays_ndrvs")
                    .and_then(Value::as_array)
                    .is_some_and(|d| !d.is_empty()),
            }
        })
        .collect()
}

pub(crate) fn read_gpus(src: &dyn MacSource) -> GpuInfo {
    let entries = src
        .profiler("SPDisplaysDataType")
        .map(|j| parse_displays(&j))
        .unwrap_or_default();
    let mut boot_marked = false;
    let gpus = entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            // The first card with a display attached is the one the machine shows its screen on.
            let boot = e.has_display && !boot_marked;
            boot_marked |= boot;
            Gpu {
                card: format!("gpu{i}"),
                pci_address: Field::Unknown,
                vendor_id: Field::from_option(e.vendor_id),
                device_id: Field::from_option(e.device_id),
                vendor_name: Field::from_option(e.vendor.clone()),
                device_name: Field::from_option(e.model.clone()),
                subsystem_name: Field::Unknown,
                driver: Field::Unknown,
                vram_bytes: Field::from_option(e.vram_bytes),
                pcie_link: Field::from_option(e.pcie_width.as_ref().map(|w| format!("PCIe {w}"))),
                boot_vga: boot,
            }
        })
        .collect();
    GpuInfo { gpus }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AMD: &str = r#"{"SPDisplaysDataType":[{"_name":"kHW_AMDRadeonPro555Item","spdisplays_device-id":"0x67ef","spdisplays_pcie_width":"x8","spdisplays_vendor":"sppci_vendor_amd","spdisplays_vram":"2 GB","sppci_model":"Radeon Pro 555","spdisplays_ndrvs":[{"_name":"iMac"}]}]}"#;

    #[test]
    fn an_amd_card_with_a_display() {
        let e = parse_displays(AMD);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].vendor.as_deref(), Some("AMD"));
        assert_eq!(e[0].vendor_id, Some(0x1002));
        assert_eq!(e[0].device_id, Some(0x67ef));
        assert_eq!(e[0].vram_bytes, Some(2 << 30));
        assert!(e[0].has_display);
    }

    #[test]
    fn apple_gpus_have_no_pci_ids() {
        let j = r#"{"SPDisplaysDataType":[{"sppci_model":"Apple M1","spdisplays_vendor":"sppci_vendor_Apple"}]}"#;
        let e = parse_displays(j);
        assert_eq!(e[0].vendor.as_deref(), Some("Apple"));
        assert_eq!(e[0].vendor_id, None);
    }

    #[test]
    fn hex_ids() {
        assert_eq!(parse_hex_u16("0x67ef"), Some(0x67ef));
        assert_eq!(parse_hex_u16("67ef"), None);
        assert_eq!(parse_hex_u16("0x12345"), None);
        assert_eq!(parse_hex_u16(""), None);
    }

    #[test]
    fn garbage_gives_no_cards() {
        assert!(parse_displays("x").is_empty());
        assert!(parse_displays("{}").is_empty());
        assert!(parse_displays(r#"{"SPDisplaysDataType":[{"x":1}]}"#).is_empty());
    }
}
