#![allow(unsafe_code)]

use super::{from_error, wide};
use crate::WinError;
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::IO::DeviceIoControl;
use windows::core::PCWSTR;

/// An open device handle, closed on drop.
pub struct Device(HANDLE);

impl Device {
    /// `read = false` opens with no access rights (enough for IOCTL_STORAGE_QUERY_PROPERTY);
    /// `read = true` asks for read/write, which SMART and battery IOCTLs need.
    pub fn open(path: &str, read: bool) -> Result<Device, WinError> {
        let p = wide(path);
        let access = if read {
            (GENERIC_READ | GENERIC_WRITE).0
        } else {
            0
        };
        // SAFETY: `p` is a NUL-terminated UTF-16 buffer that outlives the call; the other
        // arguments are plain values. The returned handle is owned by `Device`.
        let h = unsafe {
            CreateFileW(
                PCWSTR(p.as_ptr()),
                access,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        }
        .map_err(|e| from_error("CreateFileW", &e))?;
        Ok(Device(h))
    }

    /// One DeviceIoControl round trip; the output is truncated to the bytes returned.
    pub fn ioctl(&self, code: u32, input: &[u8], out_len: usize) -> Result<Vec<u8>, WinError> {
        let mut out = vec![0u8; out_len];
        let mut got = 0u32;
        let in_len = u32::try_from(input.len()).map_err(|_| WinError::NotAvailable)?;
        let o_len = u32::try_from(out.len()).map_err(|_| WinError::NotAvailable)?;
        // SAFETY: input/output pointers and lengths describe live buffers owned by this frame;
        // the call is synchronous (no OVERLAPPED), so nothing outlives it.
        unsafe {
            DeviceIoControl(
                self.0,
                code,
                (!input.is_empty()).then_some(input.as_ptr().cast()),
                in_len,
                (!out.is_empty()).then_some(out.as_mut_ptr().cast()),
                o_len,
                Some(&raw mut got),
                None,
            )
        }
        .map_err(|e| from_error("DeviceIoControl", &e))?;
        out.truncate(got as usize);
        Ok(out)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        // SAFETY: the handle came from CreateFileW and is closed exactly once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}
