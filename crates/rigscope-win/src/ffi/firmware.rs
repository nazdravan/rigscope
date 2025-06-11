#![allow(unsafe_code)]

use super::{from_error, last_error};
use crate::WinError;
use windows::Win32::System::SystemInformation::{
    GetLogicalProcessorInformationEx, GetSystemFirmwareTable, RelationAll, FIRMWARE_TABLE_PROVIDER,
    SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
};

const RSMB: u32 = u32::from_be_bytes(*b"RSMB");
const MAX: usize = 1024 * 1024 + 8;

/// The raw SMBIOS table: the 8-byte `RawSMBIOSData` header followed by the structures.
pub fn smbios_raw() -> Result<Vec<u8>, WinError> {
    // SAFETY: a `None` buffer only asks for the size.
    let need = unsafe { GetSystemFirmwareTable(FIRMWARE_TABLE_PROVIDER(RSMB), 0, None) } as usize;
    if need == 0 {
        return Err(last_error("GetSystemFirmwareTable"));
    }
    if need > MAX {
        return Err(WinError::Failed {
            api: "GetSystemFirmwareTable".into(),
            code: 0,
        });
    }
    let mut buf = vec![0u8; need];
    // SAFETY: `buf` is `need` bytes long, as the slice tells the function.
    let got = unsafe { GetSystemFirmwareTable(FIRMWARE_TABLE_PROVIDER(RSMB), 0, Some(&mut buf)) }
        as usize;
    if got == 0 || got > buf.len() {
        return Err(last_error("GetSystemFirmwareTable"));
    }
    buf.truncate(got);
    Ok(buf)
}

/// The `SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX` records for `RelationAll`, as bytes.
pub fn processors_raw() -> Result<Vec<u8>, WinError> {
    let mut len = 0u32;
    // SAFETY: size query with no buffer; fails with ERROR_INSUFFICIENT_BUFFER and sets `len`.
    let _ = unsafe { GetLogicalProcessorInformationEx(RelationAll, None, &mut len) };
    let n = len as usize;
    if n == 0 {
        return Err(last_error("GetLogicalProcessorInformationEx"));
    }
    if n > MAX {
        return Err(WinError::Failed {
            api: "GetLogicalProcessorInformationEx".into(),
            code: 0,
        });
    }
    // u64 storage keeps the records 8-byte aligned, as the API expects.
    let mut store = vec![0u64; n.div_ceil(8)];
    // SAFETY: `store` has at least `len` bytes, suitably aligned; `len` is passed in and out.
    unsafe {
        GetLogicalProcessorInformationEx(
            RelationAll,
            Some(
                store
                    .as_mut_ptr()
                    .cast::<SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX>(),
            ),
            &mut len,
        )
    }
    .map_err(|e| from_error("GetLogicalProcessorInformationEx", &e))?;
    let bytes: Vec<u8> = store
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .take(len as usize)
        .collect();
    Ok(bytes)
}
