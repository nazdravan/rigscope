//! Builds a [`Snapshot`](rigscope_model::Snapshot) on macOS from a [`MacSource`]
//! (`system_profiler` JSON and `sysctl` values). Everything here is safe Rust over text, so it
//! is tested on Linux and Windows too; only `RealMac` talks to the OS, and only on macOS do the
//! tools exist.

mod collector;
mod gpu;
mod mask;
mod memory;
mod net;
mod parse;
mod real;
mod source;
mod storage;
mod system;

pub use collector::MacCollector;
pub use gpu::{GpuEntry, MAX_GPUS, parse_displays, parse_hex_u16};
pub use mask::{CAPTURED_NAME, mask_fixture, mask_text};
pub use net::{EthernetCard, MAX_INTERFACES, NetEntry, is_virtual, parse_ethernet, parse_network};
pub use parse::{
    Dimm, HardwareProfile, MAX_SLOTS, MemoryProfile, decode_hex_text, manufacturer, parse_boottime,
    parse_hardware, parse_memory, parse_mts, parse_size, parse_type,
};
pub use real::{DATA_TYPES, RealMac, SYSCTLS, capture};
pub use source::{FIXTURE_FILE, MAX_FIXTURE_LEN, MacError, MacSource, MemMac, R};
pub use storage::{DiskEntry, MAX_DISKS, parse_disks};
