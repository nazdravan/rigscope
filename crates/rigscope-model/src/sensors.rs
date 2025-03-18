use crate::Field;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SensorInfo {
    pub chips: Vec<SensorChip>,
}

/// One hwmon device (`hwmon0`), e.g. `coretemp`, `nvme`, `nct6798`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SensorChip {
    pub hwmon: String,
    pub name: String,
    pub temps: Vec<Temperature>,
    pub fans: Vec<Fan>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Temperature {
    /// `tempN`.
    pub index: u32,
    /// `Package id 0`, `Composite`, or `temp1` when the driver gives no label.
    pub label: String,
    pub millicelsius: Field<i32>,
    pub max_mc: Field<i32>,
    pub crit_mc: Field<i32>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Fan {
    /// `fanN`.
    pub index: u32,
    pub label: String,
    pub rpm: Field<u32>,
}

/// What a hwmon driver measures, for display: `coretemp` → `CPU (Intel)`.
pub fn chip_kind(name: &str) -> Option<&'static str> {
    Some(match name {
        "coretemp" => "CPU (Intel)",
        "k10temp" | "zenpower" => "CPU (AMD)",
        "nvme" => "NVMe disk",
        "drivetemp" => "SATA disk",
        "amdgpu" => "GPU (AMD)",
        "nouveau" => "GPU (NVIDIA, nouveau)",
        "acpitz" => "ACPI thermal zone",
        "iwlwifi_1" | "mt7921_phy0" => "Wi-Fi",
        n if n.starts_with("nct") || n.starts_with("it8") || n.starts_with("f71") => {
            "Motherboard (Super I/O)"
        }
        _ => return None,
    })
}

/// `45125` m°C → `45.1 °C`.
pub fn format_celsius(mc: i32) -> String {
    format!("{:.1} °C", f64::from(mc) / 1000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds() {
        assert_eq!(chip_kind("coretemp"), Some("CPU (Intel)"));
        assert_eq!(chip_kind("k10temp"), Some("CPU (AMD)"));
        assert_eq!(chip_kind("nvme"), Some("NVMe disk"));
        assert_eq!(chip_kind("amdgpu"), Some("GPU (AMD)"));
        assert_eq!(chip_kind("nct6798"), Some("Motherboard (Super I/O)"));
        assert_eq!(chip_kind("it8686"), Some("Motherboard (Super I/O)"));
        assert_eq!(chip_kind("acpitz"), Some("ACPI thermal zone"));
        assert_eq!(chip_kind("mystery"), None);
    }

    #[test]
    fn celsius() {
        assert_eq!(format_celsius(45_125), "45.1 °C");
        assert_eq!(format_celsius(-5_000), "-5.0 °C");
        assert_eq!(format_celsius(100_000), "100.0 °C");
    }
}
