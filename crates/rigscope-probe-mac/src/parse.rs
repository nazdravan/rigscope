//! Pure parsers for what `system_profiler` prints. Never panic: anything unexpected is `None`.

use rigscope_model::{clean_text, FormFactor, MemType};
use serde_json::Value;

/// Most memory slots read from the profiler output.
pub const MAX_SLOTS: usize = 64;

/// One memory module (or empty slot) as `SPMemoryDataType` lists it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Dimm {
    pub locator: Option<String>,
    pub manufacturer: Option<String>,
    pub part_number: Option<String>,
    pub serial: Option<String>,
    pub size_bytes: Option<u64>,
    pub speed_mts: Option<u32>,
    pub mem_type: Option<MemType>,
    pub form_factor: Option<FormFactor>,
    /// The slot holds no module.
    pub empty: bool,
}

/// What `SPMemoryDataType` says about the whole memory.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemoryProfile {
    pub dimms: Vec<Dimm>,
    /// `ecc_disabled` / `ecc_enabled`, when printed.
    pub ecc: Option<String>,
}

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key)?.as_str().and_then(clean_text)
}

/// `"8 GB"` -> bytes. macOS prints GiB as "GB" for memory modules.
pub fn parse_size(s: &str) -> Option<u64> {
    let mut it = s.split_whitespace();
    let n: u64 = it.next()?.parse().ok()?;
    let unit: u64 = match it.next()?.to_ascii_uppercase().as_str() {
        "KB" => 1 << 10,
        "MB" => 1 << 20,
        "GB" => 1 << 30,
        "TB" => 1 << 40,
        _ => return None,
    };
    n.checked_mul(unit)
}

/// `"2133 MHz"` -> 2133 (the profiler's MHz is the module's MT/s).
pub fn parse_mts(s: &str) -> Option<u32> {
    let mut it = s.split_whitespace();
    let n: u32 = it.next()?.parse().ok()?;
    it.next()?.eq_ignore_ascii_case("MHz").then_some(n)
}

/// `"DDR4 SO-DIMM"` -> (type, form factor); either part may be missing.
pub fn parse_type(s: &str) -> (Option<MemType>, Option<FormFactor>) {
    let upper = s.to_ascii_uppercase();
    let word = upper.split_whitespace().next().unwrap_or("");
    let mem_type = match word {
        "DDR3" => Some(MemType::Ddr3),
        "DDR4" => Some(MemType::Ddr4),
        "DDR5" => Some(MemType::Ddr5),
        "LPDDR3" => Some(MemType::Lpddr3),
        "LPDDR4" | "LPDDR4X" => Some(MemType::Lpddr4),
        "LPDDR5" | "LPDDR5X" => Some(MemType::Lpddr5),
        _ => None,
    };
    let form = if upper.contains("SO-DIMM") {
        Some(FormFactor::SoDimm)
    } else if upper.contains("DIMM") {
        Some(FormFactor::Dimm)
    } else {
        None
    };
    (mem_type, form)
}

/// `"0x484D41"` -> `"HMA"` when the hex decodes to printable ASCII; any other text is kept.
pub fn decode_hex_text(s: &str) -> Option<String> {
    let t = s.trim();
    let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) else {
        return clean_text(t);
    };
    if hex.is_empty() || hex.len() % 2 != 0 || hex.len() > 128 || !hex.is_ascii() {
        return clean_text(t);
    }
    let mut out = String::new();
    for pair in hex.as_bytes().chunks(2) {
        let Some(b) = std::str::from_utf8(pair)
            .ok()
            .and_then(|p| u8::from_str_radix(p, 16).ok())
        else {
            return clean_text(t);
        };
        if !(0x20..0x7F).contains(&b) && b != 0 {
            return clean_text(t);
        }
        out.push(b as char);
    }
    clean_text(&out)
}

/// `sysctl -n kern.boottime` prints `{ sec = 1730000000, usec = 123456 } Mon Nov  4 ...`;
/// returns `sec`.
pub fn parse_boottime(s: &str) -> Option<u64> {
    let rest = s.split("sec =").nth(1)?;
    let digits: String = rest
        .trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

/// JEDEC manufacturer codes as macOS prints them (continuation count in the high byte, then the
/// code). Only codes that were checked are listed; anything else is shown as printed.
const JEDEC: &[(&str, &str)] = &[
    ("0x802C", "Micron"),
    ("0x80AD", "SK Hynix"),
    ("0x80CE", "Samsung"),
    ("0x859B", "Crucial"),
    ("0x9801", "Kingston"),
];

/// A manufacturer: a JEDEC code is named when known, text is kept as it is.
pub fn manufacturer(s: &str) -> Option<String> {
    let t = s.trim();
    if t.starts_with("0x") || t.starts_with("0X") {
        let upper = format!("0x{}", t.get(2..).unwrap_or("").to_ascii_uppercase());
        return JEDEC
            .iter()
            .find(|(code, _)| *code == upper)
            .map(|(_, name)| (*name).to_string())
            .or_else(|| clean_text(t));
    }
    clean_text(t)
}

/// Reads `SPMemoryDataType` JSON. Machines without slots (Apple silicon) give no DIMMs.
pub fn parse_memory(json: &str) -> Option<MemoryProfile> {
    let root: Value = serde_json::from_str(json).ok()?;
    let top = root.get("SPMemoryDataType")?.as_array()?;
    let mut profile = MemoryProfile::default();
    for group in top {
        if profile.ecc.is_none() {
            profile.ecc = text(group, "global_ecc_state");
        }
        let Some(items) = group.get("_items").and_then(Value::as_array) else {
            continue;
        };
        for item in items
            .iter()
            .take(MAX_SLOTS.saturating_sub(profile.dimms.len()))
        {
            profile.dimms.push(parse_dimm(item));
        }
    }
    Some(profile)
}

/// A string field as printed, without `clean_text` (which would drop "Empty").
fn raw<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key)?.as_str().map(str::trim)
}

fn parse_dimm(item: &Value) -> Dimm {
    let size = raw(item, "dimm_size");
    let size_bytes = size.and_then(parse_size);
    let empty = raw(item, "dimm_status").is_some_and(|s| s.eq_ignore_ascii_case("empty"))
        || size.is_some_and(|s| s.eq_ignore_ascii_case("empty"));
    let (mem_type, form_factor) = raw(item, "dimm_type").map_or((None, None), parse_type);
    Dimm {
        locator: text(item, "_name"),
        manufacturer: text(item, "dimm_manufacturer").and_then(|m| manufacturer(&m)),
        part_number: text(item, "dimm_part_number").and_then(|p| decode_hex_text(&p)),
        serial: text(item, "dimm_serial_number"),
        size_bytes,
        speed_mts: text(item, "dimm_speed").as_deref().and_then(parse_mts),
        mem_type,
        form_factor,
        empty,
    }
}

/// The few `SPHardwareDataType` fields rigscope shows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HardwareProfile {
    pub model: Option<String>,
    pub name: Option<String>,
    pub cpu: Option<String>,
    pub boot_rom: Option<String>,
}

pub fn parse_hardware(json: &str) -> Option<HardwareProfile> {
    let root: Value = serde_json::from_str(json).ok()?;
    let h = root.get("SPHardwareDataType")?.as_array()?.first()?;
    Some(HardwareProfile {
        model: text(h, "machine_model"),
        name: text(h, "machine_name"),
        cpu: text(h, "cpu_type").or_else(|| text(h, "chip_type")),
        boot_rom: text(h, "boot_rom_version"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const IMAC_MEMORY: &str = r#"{"SPMemoryDataType":[{"_items":[
      {"_name":"BANK 0/DIMM0","dimm_manufacturer":"0x80AD","dimm_part_number":"0x484D414631475336","dimm_serial_number":"0x37781227","dimm_size":"8 GB","dimm_speed":"2133 MHz","dimm_status":"ok","dimm_type":"DDR4 SO-DIMM"},
      {"_name":"BANK 1/DIMM0","dimm_manufacturer":"0x802C","dimm_size":"8 GB","dimm_speed":"2133 MHz","dimm_status":"ok","dimm_type":"DDR4 SO-DIMM"}],
      "_name":"global_name","global_ecc_state":"ecc_disabled"}]}"#;

    #[test]
    fn imac_memory_has_two_ddr4_so_dimms() {
        let p = parse_memory(IMAC_MEMORY).unwrap();
        assert_eq!(p.dimms.len(), 2);
        assert_eq!(p.ecc.as_deref(), Some("ecc_disabled"));
        let d = &p.dimms[0];
        assert_eq!(d.locator.as_deref(), Some("BANK 0/DIMM0"));
        assert_eq!(d.manufacturer.as_deref(), Some("SK Hynix"));
        assert_eq!(d.part_number.as_deref(), Some("HMAF1GS6"));
        assert_eq!(d.size_bytes, Some(8 << 30));
        assert_eq!(d.speed_mts, Some(2133));
        assert_eq!(d.mem_type, Some(MemType::Ddr4));
        assert_eq!(d.form_factor, Some(FormFactor::SoDimm));
        assert!(!d.empty);
        assert_eq!(p.dimms[1].manufacturer.as_deref(), Some("Micron"));
    }

    #[test]
    fn an_empty_slot_is_empty() {
        let j = r#"{"SPMemoryDataType":[{"_items":[
          {"_name":"BANK 1/DIMM0","dimm_size":"Empty","dimm_status":"empty","dimm_type":"empty"}]}]}"#;
        let p = parse_memory(j).unwrap();
        assert!(p.dimms[0].empty);
        assert_eq!(p.dimms[0].size_bytes, None);
        assert_eq!(p.dimms[0].mem_type, None);
    }

    #[test]
    fn apple_silicon_has_no_slots() {
        let j = r#"{"SPMemoryDataType":[{"SPMemoryDataType":"16 GB","dimm_manufacturer":"Apple","dimm_type":"LPDDR5"}]}"#;
        assert!(parse_memory(j).unwrap().dimms.is_empty());
    }

    #[test]
    fn sizes_speeds_and_types() {
        assert_eq!(parse_size("16 GB"), Some(16 << 30));
        assert_eq!(parse_size("512 MB"), Some(512 << 20));
        assert_eq!(parse_size("Empty"), None);
        assert_eq!(parse_size("99999999999999999999 GB"), None);
        assert_eq!(parse_mts("2133 MHz"), Some(2133));
        assert_eq!(parse_mts("2133"), None);
        assert_eq!(parse_type("LPDDR4X"), (Some(MemType::Lpddr4), None));
        assert_eq!(
            parse_type("DDR3 DIMM"),
            (Some(MemType::Ddr3), Some(FormFactor::Dimm))
        );
        assert_eq!(parse_type("empty"), (None, None));
    }

    #[test]
    fn hex_text_decodes_only_printable_ascii() {
        assert_eq!(decode_hex_text("0x4142").as_deref(), Some("AB"));
        assert_eq!(decode_hex_text("0x414220202020").as_deref(), Some("AB"));
        assert_eq!(decode_hex_text("0x01FF").as_deref(), Some("0x01FF"));
        assert_eq!(decode_hex_text("0xZZ").as_deref(), Some("0xZZ"));
        assert_eq!(decode_hex_text("Kingston").as_deref(), Some("Kingston"));
        assert_eq!(decode_hex_text("0x"), Some("0x".into()));
    }

    #[test]
    fn unknown_jedec_codes_are_shown_as_printed() {
        assert_eq!(manufacturer("0x1234").as_deref(), Some("0x1234"));
        assert_eq!(manufacturer("0x80ad").as_deref(), Some("SK Hynix"));
        assert_eq!(manufacturer("Samsung").as_deref(), Some("Samsung"));
    }

    #[test]
    fn hardware_profile() {
        let j = r#"{"SPHardwareDataType":[{"machine_model":"iMac18,2","machine_name":"iMac","cpu_type":"Quad-Core Intel Core i5"}]}"#;
        let h = parse_hardware(j).unwrap();
        assert_eq!(h.model.as_deref(), Some("iMac18,2"));
        assert_eq!(h.cpu.as_deref(), Some("Quad-Core Intel Core i5"));
    }

    #[test]
    fn boottime_seconds() {
        assert_eq!(
            parse_boottime("{ sec = 1730000000, usec = 5 } Sun Oct 27 05:33:20 2024"),
            Some(1_730_000_000)
        );
        assert_eq!(parse_boottime("{ sec = , usec = 5 }"), None);
        assert_eq!(parse_boottime("garbage"), None);
    }

    #[test]
    fn garbage_is_none() {
        assert!(parse_memory("not json").is_none());
        assert!(parse_memory("{}").is_none());
        assert!(parse_hardware("[]").is_none());
    }
}
