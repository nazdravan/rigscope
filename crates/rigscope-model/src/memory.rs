use crate::Field;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MemoryInfo {
    /// Total usable memory as seen by the OS (`/proc/meminfo`).
    pub os_total_bytes: Field<u64>,
    /// Physical memory arrays from SMBIOS type 16 (system memory only).
    pub arrays: Vec<MemoryArray>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MemoryArray {
    pub max_capacity_bytes: Field<u64>,
    pub slot_count: Field<u16>,
    pub ecc: Field<String>,
    pub slots: Vec<MemorySlot>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MemorySlot {
    pub locator: Field<String>,
    pub bank: Field<String>,
    pub module: SlotState,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum SlotState {
    Empty,
    #[default]
    Unknown,
    Installed(Box<MemoryModule>),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MemoryModule {
    pub size_bytes: Field<u64>,
    pub mem_type: Field<MemType>,
    pub form_factor: Field<FormFactor>,
    pub manufacturer: Field<String>,
    pub part_number: Field<String>,
    pub serial: Field<String>,
    /// Rated (maximum) speed in MT/s.
    pub rated_mts: Field<u32>,
    /// Speed the module is configured to run at, in MT/s.
    pub configured_mts: Field<u32>,
    pub ranks: Field<u8>,
    pub voltage_mv: Field<VoltageMv>,
    pub data_width_bits: Field<u16>,
    pub total_width_bits: Field<u16>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoltageMv {
    pub min: Option<u16>,
    pub max: Option<u16>,
    pub configured: Option<u16>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemType {
    Ddr3,
    Ddr4,
    Ddr5,
    Lpddr3,
    Lpddr4,
    Lpddr5,
    Other(u8),
}

impl MemType {
    pub fn label(self) -> String {
        match self {
            MemType::Ddr3 => "DDR3".into(),
            MemType::Ddr4 => "DDR4".into(),
            MemType::Ddr5 => "DDR5".into(),
            MemType::Lpddr3 => "LPDDR3".into(),
            MemType::Lpddr4 => "LPDDR4".into(),
            MemType::Lpddr5 => "LPDDR5".into(),
            MemType::Other(code) => format!("type 0x{code:02X}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormFactor {
    Dimm,
    SoDimm,
    Other(u8),
}

impl FormFactor {
    pub fn label(self) -> String {
        match self {
            FormFactor::Dimm => "DIMM".into(),
            FormFactor::SoDimm => "SO-DIMM".into(),
            FormFactor::Other(code) => format!("form factor 0x{code:02X}"),
        }
    }
}
