//! rigscope data model: a [`Snapshot`] describes one machine at one moment.
//!
//! Plain data + serde, no IO. Missing values are explicit ([`Field`]), never empty strings.

pub mod compat;
pub mod cpu;
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

pub use compat::{compat_summary, CompatLine};
pub use cpu::{CacheKind, CacheLevel, CoreFreq, CpuInfo};
pub use field::Field;
pub use gpu::{Gpu, GpuInfo};
pub use memory::{
    FormFactor, MemType, MemoryArray, MemoryInfo, MemoryModule, MemorySlot, SlotState, VoltageMv,
};
pub use net::{format_speed, NetInfo, NetInterface, NetKind};
pub use power::{format_mwh, Battery, PowerInfo};
pub use report::{
    format_bytes, format_decimal_gb, format_disk_size, format_written, health_summary, mask,
    mask_mac, masked, report_text,
};
pub use sensors::{chip_kind, format_celsius, Fan, SensorChip, SensorInfo, Temperature};
pub use storage::{Disk, DiskHealth, DiskKind, Interface, StorageInfo};
pub use system::{BoardInfo, DmiSource, Snapshot, SystemInfo, SCHEMA_VERSION};
pub use text::clean_text;
