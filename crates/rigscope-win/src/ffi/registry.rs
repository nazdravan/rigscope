#![allow(unsafe_code)]

use super::wide;
use windows::Win32::System::Registry::{
    HKEY_LOCAL_MACHINE, REG_ROUTINE_FLAGS, REG_VALUE_TYPE, RRF_RT_REG_BINARY, RRF_RT_REG_DWORD,
    RRF_RT_REG_SZ, RegGetValueW,
};
use windows::core::PCWSTR;

/// One HKLM value of the type `flags` allows, as raw bytes (at most 64 KiB).
fn get(subkey: &str, value: &str, flags: REG_ROUTINE_FLAGS) -> Option<Vec<u8>> {
    let (k, v) = (wide(subkey), wide(value));
    let mut size = 0u32;
    let mut kind = REG_VALUE_TYPE::default();
    // SAFETY: size query; key/value are NUL-terminated buffers alive for the call, and the
    // out-pointers point at locals of this frame.
    let r = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(k.as_ptr()),
            PCWSTR(v.as_ptr()),
            flags,
            Some(&raw mut kind),
            None,
            Some(&raw mut size),
        )
    };
    if r.is_err() || size == 0 || size > 64 * 1024 {
        return None;
    }
    let mut buf = vec![0u8; size as usize];
    // SAFETY: `buf` holds `size` bytes, passed as the in/out size.
    let r = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(k.as_ptr()),
            PCWSTR(v.as_ptr()),
            flags,
            Some(&raw mut kind),
            Some(buf.as_mut_ptr().cast()),
            Some(&raw mut size),
        )
    };
    if r.is_err() {
        return None;
    }
    buf.truncate(size as usize);
    Some(buf)
}

/// A REG_SZ value under HKLM, trimmed; empty → None.
pub fn registry_string(subkey: &str, value: &str) -> Option<String> {
    crate::layout::utf16z(&get(subkey, value, RRF_RT_REG_SZ)?)
}

/// A REG_DWORD value under HKLM.
pub fn registry_u32(subkey: &str, value: &str) -> Option<u32> {
    let b = get(subkey, value, RRF_RT_REG_DWORD)?;
    Some(u32::from_le_bytes(b.get(..4)?.try_into().ok()?))
}

/// A REG_BINARY value under HKLM.
pub fn registry_bytes(subkey: &str, value: &str) -> Option<Vec<u8>> {
    get(subkey, value, RRF_RT_REG_BINARY)
}
