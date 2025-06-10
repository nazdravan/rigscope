#![allow(unsafe_code)]

use super::from_error;
use crate::{DxgiAdapterRaw, WinError};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE, DXGI_ERROR_NOT_FOUND,
};

const MAX: u32 = 16;

pub fn adapters_dxgi() -> Result<Vec<DxgiAdapterRaw>, WinError> {
    // SAFETY: plain factory creation with no arguments; the returned COM interface is
    // reference-counted by the wrapper and released on drop.
    let f: IDXGIFactory1 =
        unsafe { CreateDXGIFactory1() }.map_err(|e| from_error("CreateDXGIFactory1", &e))?;
    let mut out = Vec::new();
    for i in 0..MAX {
        // SAFETY: `f` is a valid factory; an index past the end returns DXGI_ERROR_NOT_FOUND.
        let a = match unsafe { f.EnumAdapters1(i) } {
            Ok(a) => a,
            Err(e) if e.code() == DXGI_ERROR_NOT_FOUND => break,
            Err(e) => return Err(from_error("EnumAdapters1", &e)),
        };
        // SAFETY: `a` is a valid adapter; GetDesc1 fills a plain struct it returns by value.
        let d = unsafe { a.GetDesc1() }.map_err(|e| from_error("GetDesc1", &e))?;
        let len = d
            .Description
            .iter()
            .position(|c| *c == 0)
            .unwrap_or(d.Description.len());
        out.push(DxgiAdapterRaw {
            description: String::from_utf16_lossy(d.Description.get(..len).unwrap_or(&[])),
            vendor_id: d.VendorId,
            device_id: d.DeviceId,
            dedicated_video_memory: d.DedicatedVideoMemory as u64,
            shared_system_memory: d.SharedSystemMemory as u64,
            luid: (u64::from(d.AdapterLuid.HighPart as u32) << 32)
                | u64::from(d.AdapterLuid.LowPart),
            software: d.Flags & (DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) != 0,
        });
    }
    Ok(out)
}
