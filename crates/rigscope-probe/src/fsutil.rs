//! Size-capped filesystem helpers shared by the real and fixture sources.

use crate::{MAX_FILE_LEN, ReadError};
use std::io::Read;

/// Most directory entries listed.
pub const MAX_DIR_ENTRIES: usize = 8192;

pub(crate) fn read_capped(path: &std::path::Path) -> Result<Vec<u8>, ReadError> {
    let file = std::fs::File::open(path).map_err(map_io)?;
    let mut buf = Vec::new();
    // sysfs files report size 4096 regardless of content, so read up to the cap + 1.
    file.take(MAX_FILE_LEN as u64 + 1)
        .read_to_end(&mut buf)
        .map_err(map_io)?;
    if buf.len() > MAX_FILE_LEN {
        return Err(ReadError::TooLarge);
    }
    Ok(buf)
}

pub(crate) fn list_dir(path: &std::path::Path) -> Result<Vec<String>, ReadError> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(path)
        .map_err(map_io)?
        .take(MAX_DIR_ENTRIES)
    {
        let entry = entry.map_err(map_io)?;
        out.push(entry.file_name().to_string_lossy().into_owned());
    }
    out.sort();
    Ok(out)
}

pub(crate) fn map_io(e: std::io::Error) -> ReadError {
    match e.kind() {
        std::io::ErrorKind::NotFound => ReadError::NotFound,
        std::io::ErrorKind::PermissionDenied => ReadError::PermissionDenied,
        _ => ReadError::Io(e.to_string()),
    }
}
