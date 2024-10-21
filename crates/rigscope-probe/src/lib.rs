//! Collects a [`Snapshot`](rigscope_model::Snapshot) from a [`Source`] of Linux system files.

mod build;
mod dmi_dump;
mod error;
mod fixture_source;
#[cfg(target_os = "linux")]
mod fs_source;
mod fsutil;
mod source;

pub use build::{build_snapshot, paths, refresh_live};
pub use dmi_dump::{decode_dmi_dump, encode_dmi_dump, DmiDump, MAX_DMI_PART};
pub use error::ProbeError;
pub use fixture_source::FixtureSource;
#[cfg(target_os = "linux")]
pub use fs_source::FsSource;
pub use source::{MemSource, ReadError, Source, MAX_FILE_LEN};
