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
pub use gpu::{read_gpus, MAX_GPUS, NO_PCIE_LINK, NO_PCI_IDS, NVIDIA_NO_VRAM, SHARED_MEMORY};
pub use hwmon::{read_sensors, refresh_sensors, MAX_CHIPS};
pub use net::{read_net, MAX_INTERFACES, WIFI_NO_DUPLEX, WIFI_NO_SPEED};
pub use power::{read_power, MAX_SUPPLIES};
pub use rigscope_model::{decode_dmi_dump, encode_dmi_dump, DmiDump, DumpError, MAX_DMI_PART};
pub use smart::{parse_smartctl, parse_smartctl_report, SmartError, SmartReport, MAX_SMART_JSON};
pub use source::{MemSource, ReadError, Source, MAX_FILE_LEN};
pub use storage::{interface_for, kind_for, read_storage, MAX_DISKS, NO_SMART};
