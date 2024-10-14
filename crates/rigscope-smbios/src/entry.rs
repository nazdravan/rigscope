use crate::bytes::{u16_at, u32_at, u8_at};
use crate::SmbiosError;

/// The parts of the entry point the decoder needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryPoint {
    pub major: u8,
    pub minor: u8,
    /// Table length (2.x) or maximum table size (3.x).
    pub table_len: u32,
}

/// Parses a 64-bit `_SM3_` (3.x) or 32-bit `_SM_` (2.x) entry point.
///
/// The checksum is not enforced: the kernel already located the table, and a bad checksum
/// from sloppy firmware should not hide otherwise readable data.
pub fn parse_entry_point(bytes: &[u8]) -> Result<EntryPoint, SmbiosError> {
    let bad = SmbiosError::BadEntryPoint;
    if bytes.starts_with(b"_SM3_") {
        Ok(EntryPoint {
            major: u8_at(bytes, 0x07).ok_or(bad.clone())?,
            minor: u8_at(bytes, 0x08).ok_or(bad.clone())?,
            table_len: u32_at(bytes, 0x0C).ok_or(bad)?,
        })
    } else if bytes.starts_with(b"_SM_") {
        Ok(EntryPoint {
            major: u8_at(bytes, 0x06).ok_or(bad.clone())?,
            minor: u8_at(bytes, 0x07).ok_or(bad.clone())?,
            table_len: u32::from(u16_at(bytes, 0x16).ok_or(bad)?),
        })
    } else {
        Err(bad)
    }
}
