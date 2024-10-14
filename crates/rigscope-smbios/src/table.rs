use crate::bytes::{u16_at, u8_at};
use crate::{SmbiosError, MAX_STRINGS, MAX_STRUCTURES, MAX_TABLE_LEN};

/// One SMBIOS structure: header + formatted area, plus its string set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Structure<'a> {
    pub kind: u8,
    pub handle: u16,
    /// Formatted area including the 4-byte header.
    pub formatted: &'a [u8],
    pub strings: Vec<&'a [u8]>,
}

/// Splits a table into structures, stopping at type 127, at a header that cannot be framed
/// (length < 4 or past the end), or at [`MAX_STRUCTURES`].
pub fn structures(table: &[u8]) -> Result<Vec<Structure<'_>>, SmbiosError> {
    if table.len() > MAX_TABLE_LEN {
        return Err(SmbiosError::TooLarge);
    }
    let mut out = Vec::new();
    let mut off = 0usize;
    while out.len() < MAX_STRUCTURES {
        let (Some(kind), Some(len), Some(handle)) = (
            u8_at(table, off),
            u8_at(table, off + 1),
            u16_at(table, off + 2),
        ) else {
            break;
        };
        let len = usize::from(len);
        if len < 4 {
            break;
        }
        let Some(formatted) = table.get(off..off + len) else {
            break;
        };
        let Some(rest) = table.get(off + len..) else {
            break;
        };
        // String set: NUL-terminated strings, ended by an extra NUL ("\0\0" when empty).
        let Some(end) = rest.windows(2).position(|w| w == [0, 0]) else {
            break;
        };
        let strings: Vec<&[u8]> = rest
            .get(..end)
            .unwrap_or_default()
            .split(|b| *b == 0)
            .filter(|s| !s.is_empty())
            .take(MAX_STRINGS)
            .collect();
        out.push(Structure {
            kind,
            handle,
            formatted,
            strings,
        });
        if kind == 127 {
            break;
        }
        off = off + len + end + 2;
    }
    Ok(out)
}

impl Structure<'_> {
    /// The string referenced by the 1-based index stored at `off` (0 = no string).
    pub fn string_at(&self, off: usize) -> Option<String> {
        let idx = usize::from(u8_at(self.formatted, off)?);
        let raw = self.strings.get(idx.checked_sub(1)?)?;
        let raw = raw.get(..crate::MAX_STRING_LEN).unwrap_or(raw);
        rigscope_model::clean_text(&String::from_utf8_lossy(raw))
    }
}
