//! Builds a [`Snapshot`](rigscope_model::Snapshot) on Windows from a [`WinSource`].
//! Every function here is safe Rust over raw data, so it is tested on Linux and macOS too.

// Used by the collector added in a later task; the lint fires outside tests only.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "used by WinCollector (Task 11)")
)]
mod cpu;
mod errors;
mod smart;
mod source;
// Used by the collector added in a later task; the lint fires outside tests only.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "used by WinCollector (Task 11)")
)]
mod storage;
// The readers are used by the collector added in a later task.
#[allow(dead_code)]
mod system;

pub use cpu::{microcode, parse_processors, Topology, MAX_PROCESSORS_BUF};
pub use errors::{field_err, field_of};
pub use smart::{parse_ata_smart, parse_nvme_health};
pub use source::{MemWin, WinSource, R};
pub use storage::{interface_of, MAX_DISKS, NO_SMART_RAID, NO_SMART_USB};
