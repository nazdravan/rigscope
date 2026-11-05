//! rigscope data model: a [`Snapshot`] describes one machine at one moment.
//!
//! Plain data + serde, no IO. Missing values are explicit ([`Field`]), never empty strings.

pub mod collector;
pub mod compat;
pub mod cpu;
pub mod dump;
pub mod field;
pub mod gpu;
pub mod memory;
pub mod net;
pub mod power;
pub mod report;
pub mod sensors;
pub mod storage;
pub mod system;
pub mod text;

pub use collector::Collector;
pub use compat::{CompatLine, compat_summary};
pub use cpu::{CacheKind, CacheLevel, CoreFreq, CpuInfo};
pub use dump::{
    DmiDump, DumpError, MAX_DMI_PART, MAX_DUMP_DISKS, decode_dmi_dump, encode_dmi_dump,
};
pub use field::Field;
pub use gpu::{Gpu, GpuInfo};
pub use memory::{
    FormFactor, MemType, MemoryArray, MemoryInfo, MemoryModule, MemorySlot, SlotState, VoltageMv,
};
pub use net::{NetInfo, NetInterface, NetKind, format_speed};
pub use power::{Battery, PowerInfo, format_mwh};
pub use report::{
    format_bytes, format_decimal_gb, format_disk_size, format_written, health_summary, mask,
    mask_mac, masked, report_text,
};
pub use sensors::{Fan, SensorChip, SensorInfo, Temperature, chip_kind, format_celsius};
pub use storage::{Disk, DiskHealth, DiskKind, Interface, StorageInfo};
pub use system::{BoardInfo, DmiSource, Platform, SCHEMA_VERSION, Snapshot, SystemInfo};
pub use text::clean_text;
