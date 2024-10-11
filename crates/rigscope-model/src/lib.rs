//! rigscope data model: a [`Snapshot`] describes one machine at one moment.
//!
//! Plain data + serde, no IO. Missing values are explicit ([`Field`]), never empty strings.

pub mod compat;
pub mod cpu;
pub mod field;
pub mod memory;
pub mod system;
pub mod text;

pub use compat::{compat_summary, CompatLine};
pub use cpu::{CacheKind, CacheLevel, CoreFreq, CpuInfo};
pub use field::Field;
pub use memory::{
    FormFactor, MemType, MemoryArray, MemoryInfo, MemoryModule, MemorySlot, SlotState, VoltageMv,
};
pub use system::{BoardInfo, DmiSource, Snapshot, SystemInfo, SCHEMA_VERSION};
pub use text::clean_text;
