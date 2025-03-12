//! Panic-free SMBIOS (DMTF DSP0134) decoder.
//!
//! Input is untrusted bytes (`/sys/firmware/dmi/tables/{smbios_entry_point,DMI}` or a dump from
//! the privileged helper). Every read is bounds-checked; malformed data yields `Unknown` fields,
//! never a panic.

mod bytes;
mod decode;
mod entry;
mod error;
mod table;
#[cfg(any(test, feature = "testutil"))]
pub mod testutil;

pub use decode::{decode, Decoded};
pub use entry::{parse_entry_point, EntryPoint};
pub use error::SmbiosError;
pub use table::{structures, Structure};

/// Largest table accepted. SMBIOS 2.x caps at 65535 bytes; 3.x tables on big servers can be larger.
pub const MAX_TABLE_LEN: usize = 1024 * 1024;
/// Largest number of structures decoded.
pub const MAX_STRUCTURES: usize = 1024;
/// Longest string kept, in bytes.
pub const MAX_STRING_LEN: usize = 256;
/// Most strings read per structure.
pub const MAX_STRINGS: usize = 64;
