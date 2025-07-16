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
pub use gpu::{parse_displays, parse_hex_u16, GpuEntry, MAX_GPUS};
pub use mask::{mask_fixture, mask_text, CAPTURED_NAME};
pub use net::{is_virtual, parse_ethernet, parse_network, EthernetCard, NetEntry, MAX_INTERFACES};
pub use parse::{
    decode_hex_text, manufacturer, parse_boottime, parse_hardware, parse_memory, parse_mts,
    parse_size, parse_type, Dimm, HardwareProfile, MemoryProfile, MAX_SLOTS,
};
pub use real::{capture, RealMac, DATA_TYPES, SYSCTLS};
pub use source::{MacError, MacSource, MemMac, FIXTURE_FILE, MAX_FIXTURE_LEN, R};
pub use storage::{parse_disks, DiskEntry, MAX_DISKS};
