#![allow(unsafe_code)]

use crate::WinError;
use windows::Wdk::Graphics::Direct3D::{
    D3DKMT_ADAPTER_PERFDATA, D3DKMT_CLOSEADAPTER, D3DKMT_OPENADAPTERFROMLUID,
    D3DKMT_QUERYADAPTERINFO, D3DKMTCloseAdapter, D3DKMTOpenAdapterFromLuid, D3DKMTQueryAdapterInfo,
    KMTQAITYPE_ADAPTERPERFDATA,
};
use windows::Win32::Foundation::LUID;

/// GPU temperature through the WDDM 2.4+ kernel thunk (what Task Manager shows).
pub struct GpuPerf;

impl GpuPerf {
    /// Deci-°C; `None` when the driver reports 0 (no sensor exposed).
    pub fn temperature(luid: u64) -> Result<Option<u32>, WinError> {
        let mut open = D3DKMT_OPENADAPTERFROMLUID {
            AdapterLuid: LUID {
                LowPart: luid as u32,
                HighPart: (luid >> 32) as u32 as i32,
            },
            ..Default::default()
        };
        // SAFETY: `open` is a valid, live in/out struct; on success it holds an adapter
        // handle, which is closed below.
        let st = unsafe { D3DKMTOpenAdapterFromLuid(&mut open) };
        if st.is_err() {
            return Err(WinError::NotAvailable);
        }
        let mut perf = D3DKMT_ADAPTER_PERFDATA::default();
        let mut q = D3DKMT_QUERYADAPTERINFO {
            hAdapter: open.hAdapter,
            Type: KMTQAITYPE_ADAPTERPERFDATA,
            pPrivateDriverData: (&raw mut perf).cast(),
            PrivateDriverDataSize: std::mem::size_of::<D3DKMT_ADAPTER_PERFDATA>() as u32,
        };
        // SAFETY: `q` points at `perf`, of the stated size, and both live through the call.
        let st = unsafe { D3DKMTQueryAdapterInfo(&mut q) };
        let close = D3DKMT_CLOSEADAPTER {
            hAdapter: open.hAdapter,
        };
        // SAFETY: closes the handle opened above, exactly once.
        let _ = unsafe { D3DKMTCloseAdapter(&close) };
        if st.is_err() {
            // Older WDDM: the query type is unknown.
            return Err(WinError::NotAvailable);
        }
        Ok((perf.Temperature != 0).then_some(perf.Temperature))
    }
}
