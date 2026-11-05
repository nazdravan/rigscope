//! Builds a [`Snapshot`](rigscope_model::Snapshot) on Windows from a [`WinSource`].
//! Every function here is safe Rust over raw data, so it is tested on Linux and macOS too.

mod collector;
mod cpu;
mod errors;
mod gpu;
mod mask;
mod net;
mod power;
#[cfg(windows)]
mod real;
mod sensors;
mod smart;
mod source;
mod storage;
mod system;

pub use collector::WinCollector;
pub use cpu::{MAX_PROCESSORS_BUF, Topology, microcode, parse_processors};
pub use errors::{field_err, field_of};
pub use gpu::MAX_GPUS;
pub use mask::{CAPTURED_NAME, mask_descriptor_serial, mask_fixture};
pub use net::MAX_INTERFACES;
pub use power::MAX_SUPPLIES;
#[cfg(windows)]
pub use real::{RealWin, capture};
pub use sensors::NO_CPU_TEMP;
pub use smart::{parse_ata_smart, parse_nvme_health};
pub use source::{CAPTURE_LOG, MemWin, R, WinSource};
pub use storage::{MAX_DISKS, NO_SMART_RAID, NO_SMART_USB, interface_of};

#[cfg(any(test, feature = "testutil"))]
pub mod samples;
