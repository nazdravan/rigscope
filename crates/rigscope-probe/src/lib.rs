//! Collects a [`Snapshot`](rigscope_model::Snapshot) from a [`Source`] of Linux system files.

mod build;
mod collector;
mod fixture_source;
#[cfg(target_os = "linux")]
mod fs_source;
mod fsutil;
mod gpu;
mod hwmon;
mod net;
mod power;
mod smart;
mod source;
mod storage;
pub mod sysfs;

pub use build::{build_snapshot, paths, refresh_live};
pub use collector::LinuxCollector;
pub use fixture_source::FixtureSource;
#[cfg(target_os = "linux")]
pub use fs_source::FsSource;
pub use gpu::{MAX_GPUS, NO_PCI_IDS, NO_PCIE_LINK, NVIDIA_NO_VRAM, SHARED_MEMORY, read_gpus};
pub use hwmon::{MAX_CHIPS, read_sensors, refresh_sensors};
pub use net::{MAX_INTERFACES, WIFI_NO_DUPLEX, WIFI_NO_SPEED, read_net};
pub use power::{MAX_SUPPLIES, read_power};
pub use rigscope_model::{DmiDump, DumpError, MAX_DMI_PART, decode_dmi_dump, encode_dmi_dump};
pub use smart::{MAX_SMART_JSON, SmartError, SmartReport, parse_smartctl, parse_smartctl_report};
pub use source::{MAX_FILE_LEN, MemSource, ReadError, Source};
pub use storage::{MAX_DISKS, NO_SMART, interface_for, kind_for, read_storage};
