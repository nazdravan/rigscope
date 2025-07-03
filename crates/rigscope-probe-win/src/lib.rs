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
pub use cpu::{microcode, parse_processors, Topology, MAX_PROCESSORS_BUF};
pub use errors::{field_err, field_of};
pub use gpu::MAX_GPUS;
pub use mask::{mask_descriptor_serial, mask_fixture, CAPTURED_NAME};
pub use net::MAX_INTERFACES;
pub use power::MAX_SUPPLIES;
#[cfg(windows)]
pub use real::{capture, RealWin};
pub use sensors::NO_CPU_TEMP;
pub use smart::{parse_ata_smart, parse_nvme_health};
pub use source::{MemWin, WinSource, CAPTURE_LOG, R};
pub use storage::{interface_of, MAX_DISKS, NO_SMART_RAID, NO_SMART_USB};

#[cfg(any(test, feature = "testutil"))]
pub mod samples;
