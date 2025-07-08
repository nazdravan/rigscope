use serde::{Deserialize, Serialize};

/// Why a Windows read failed. Mapped to `Field` states by `rigscope-probe-win`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WinError {
    NotAvailable,
    AccessDenied,
    Timeout,
    Failed { api: String, code: u32 },
}

impl WinError {
    /// From a Win32 error code: 5 = ERROR_ACCESS_DENIED, 2/50/1168 = not found/not supported.
    pub fn from_code(api: &str, code: u32) -> WinError {
        match code {
            5 => WinError::AccessDenied,
            2 | 50 | 1168 => WinError::NotAvailable,
            _ => WinError::Failed {
                api: api.to_string(),
                code,
            },
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsVersionRaw {
    pub product_name: Option<String>,
    pub display_version: Option<String>,
    pub current_build: Option<u32>,
    pub ubr: Option<u32>,
    pub edition_id: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryStatusRaw {
    pub total_phys: u64,
    pub avail_phys: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CpuRegistryRaw {
    pub name: Option<String>,
    pub vendor: Option<String>,
    /// `~MHz`: the nominal (base) clock.
    pub mhz: Option<u32>,
    /// `Update Revision` (REG_BINARY, 8 bytes).
    pub update_revision: Option<Vec<u8>>,
}

/// One `MSFT_PhysicalDisk` row. `bus_type`/`media_type` are the raw WMI codes.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicalDiskRaw {
    pub number: u32,
    pub friendly_name: Option<String>,
    pub serial: Option<String>,
    pub firmware: Option<String>,
    pub size: Option<u64>,
    pub bus_type: Option<u16>,
    pub media_type: Option<u16>,
}

/// `\\.\PhysicalDriveN` read without WMI: the raw `STORAGE_DEVICE_DESCRIPTOR` and the size.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskIoctlRaw {
    pub number: u32,
    pub descriptor: Vec<u8>,
    pub length: Option<u64>,
}

/// The 512-byte ATA SMART attribute and threshold sectors.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtaSmartRaw {
    pub attributes: Vec<u8>,
    pub thresholds: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DxgiAdapterRaw {
    pub description: String,
    pub vendor_id: u32,
    pub device_id: u32,
    pub dedicated_video_memory: u64,
    pub shared_system_memory: u64,
    /// `(HighPart as u32 as u64) << 32 | LowPart`.
    pub luid: u64,
    pub software: bool,
}

/// A present display-class PCI device (SetupAPI), for the link and the driver version.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PciDisplayRaw {
    pub vendor_id: u16,
    pub device_id: u16,
    pub driver_version: Option<String>,
    /// `DEVPKEY_PciDevice_CurrentLinkSpeed`: 1 = 2.5, 2 = 5, 3 = 8, 4 = 16, 5 = 32, 6 = 64 GT/s.
    pub link_speed: Option<u32>,
    pub link_width: Option<u32>,
    pub location: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetAdapterRaw {
    pub name: String,
    pub description: Option<String>,
    /// IANA ifType: 6 = Ethernet, 71 = IEEE 802.11.
    pub if_type: u32,
    pub up: bool,
    pub speed_bps: Option<u64>,
    pub mac: Option<String>,
    pub mtu: Option<u32>,
    /// `MIB_IF_ROW2.InterfaceAndOperStatusFlags.HardwareInterface`.
    pub hardware: bool,
    /// `MIB_IF_ROW2.InterfaceAndOperStatusFlags.FilterInterface`: an NDIS filter (WFP, QoS,
    /// Native Wi-Fi, ...) stacked on another adapter. Absent in captures made before it existed.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub filter: bool,
}

/// One battery from `IOCTL_BATTERY_QUERY_*`. Capacities are mWh unless `relative`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatteryRaw {
    pub name: Option<String>,
    pub manufacturer: Option<String>,
    pub serial: Option<String>,
    pub chemistry: Option<String>,
    /// `BATTERY_CAPACITY_RELATIVE`: capacities are not in mWh.
    pub relative: bool,
    pub designed: Option<u32>,
    pub full: Option<u32>,
    pub now: Option<u32>,
    pub cycles: Option<u32>,
    pub power_state: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThermalZoneRaw {
    pub instance: String,
    pub tenths_kelvin: u32,
}
