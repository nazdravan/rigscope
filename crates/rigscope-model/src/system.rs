use crate::{CpuInfo, Field, GpuInfo, MemoryInfo, NetInfo, PowerInfo, SensorInfo, StorageInfo};
use serde::{Deserialize, Serialize};

/// Bumped when the JSON shape changes incompatibly.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub schema: u32,
    pub taken_at_unix: u64,
    pub system: SystemInfo,
    pub board: BoardInfo,
    pub memory: MemoryInfo,
    pub cpu: CpuInfo,
    /// Added after schema 1 shipped; older files load with no disks.
    #[serde(default)]
    pub storage: StorageInfo,
    #[serde(default)]
    pub gpu: GpuInfo,
    #[serde(default)]
    pub net: NetInfo,
    #[serde(default)]
    pub power: PowerInfo,
    #[serde(default)]
    pub sensors: SensorInfo,
    pub dmi_source: DmiSource,
}

impl Default for Snapshot {
    fn default() -> Self {
        Snapshot {
            schema: SCHEMA_VERSION,
            taken_at_unix: 0,
            system: SystemInfo::default(),
            board: BoardInfo::default(),
            memory: MemoryInfo::default(),
            cpu: CpuInfo::default(),
            storage: StorageInfo::default(),
            gpu: GpuInfo::default(),
            net: NetInfo::default(),
            power: PowerInfo::default(),
            sensors: SensorInfo::default(),
            dmi_source: DmiSource::None,
        }
    }
}

/// How much DMI/SMBIOS data the snapshot contains.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DmiSource {
    /// No DMI data at all (VM/container or non-Linux).
    None,
    /// Only the world-readable `/sys/class/dmi/id/*` fields.
    SysfsIdOnly,
    /// The `/sys/class/dmi/id` fields exist but the SMBIOS table is missing or unreadable as
    /// SMBIOS, so elevating cannot help.
    NoTable,
    /// The full SMBIOS table (root or elevated helper).
    FullTable,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os_name: Field<String>,
    pub kernel: Field<String>,
    pub hostname: Field<String>,
    pub uptime_s: Field<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BoardInfo {
    pub vendor: Field<String>,
    pub product: Field<String>,
    pub version: Field<String>,
    pub bios_vendor: Field<String>,
    pub bios_version: Field<String>,
    pub bios_date: Field<String>,
}
