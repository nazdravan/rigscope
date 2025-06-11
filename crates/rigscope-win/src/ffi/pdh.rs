#![allow(unsafe_code)]

use crate::WinError;
use windows::core::{w, PCWSTR};
use windows::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
    PdhOpenQueryW, PDH_CSTATUS_INVALID_DATA, PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_VALID_DATA,
    PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_INVALID_DATA,
    PDH_MORE_DATA,
};

/// `\Processor Information(*)\% Processor Performance`, kept open between ticks.
pub struct Pdh {
    query: PDH_HQUERY,
    counter: PDH_HCOUNTER,
}

fn fail(api: &str, code: u32) -> WinError {
    WinError::Failed {
        api: api.into(),
        code,
    }
}

impl Pdh {
    pub fn new() -> Result<Pdh, WinError> {
        let mut query = PDH_HQUERY::default();
        // SAFETY: a null data source means real-time data; `query` is a local out-parameter.
        let r = unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut query) };
        if r != 0 {
            return Err(fail("PdhOpenQueryW", r));
        }
        let mut counter = PDH_HCOUNTER::default();
        // SAFETY: valid query; the path is a static wide string.
        let r = unsafe {
            PdhAddEnglishCounterW(
                query,
                w!("\\Processor Information(*)\\% Processor Performance"),
                0,
                &mut counter,
            )
        };
        if r != 0 {
            // SAFETY: closes the query opened above, once.
            let _ = unsafe { PdhCloseQuery(query) };
            return Err(fail("PdhAddEnglishCounterW", r));
        }
        Ok(Pdh { query, counter })
    }

    /// `(logical cpu, %)`. The first call returns an empty list (no rate yet).
    pub fn sample(&mut self) -> Result<Vec<(u32, f64)>, WinError> {
        // SAFETY: valid query handle owned by self.
        let r = unsafe { PdhCollectQueryData(self.query) };
        if r != 0 {
            return Err(fail("PdhCollectQueryData", r));
        }
        let (mut size, mut count) = (0u32, 0u32);
        // SAFETY: size query with no buffer; `size`/`count` are local out-parameters.
        let r = unsafe {
            PdhGetFormattedCounterArrayW(self.counter, PDH_FMT_DOUBLE, &mut size, &mut count, None)
        };
        match r {
            PDH_MORE_DATA => {}
            // First sample of a rate counter: no values yet. ERROR_SUCCESS on a size query
            // means zero bytes are needed, i.e. no instances: nothing to read either.
            PDH_CSTATUS_INVALID_DATA | PDH_INVALID_DATA | 0 => return Ok(Vec::new()),
            _ => return Err(fail("PdhGetFormattedCounterArrayW", r)),
        }
        if size as usize > 4 * 1024 * 1024 {
            return Err(fail("PdhGetFormattedCounterArrayW", r));
        }
        let item = std::mem::size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>();
        let mut store = vec![0u64; (size as usize).div_ceil(8)];
        // SAFETY: `store` has `size` bytes, aligned for the item array; PDH fills `count` items
        // followed by their name strings inside the same buffer.
        let r = unsafe {
            PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut size,
                &mut count,
                Some(store.as_mut_ptr().cast()),
            )
        };
        match r {
            0 => {}
            // First sample of a rate counter: the size query still says PDH_MORE_DATA, but
            // the array call reports invalid data (seen on Windows 10 19045). No values yet.
            PDH_CSTATUS_INVALID_DATA | PDH_INVALID_DATA => return Ok(Vec::new()),
            _ => return Err(fail("PdhGetFormattedCounterArrayW", r)),
        }
        let n = (count as usize).min(store.len() * 8 / item);
        // SAFETY: PDH wrote `count` PDH_FMT_COUNTERVALUE_ITEM_W at the start of `store`, and
        // `n` never exceeds what fits in it; `store` outlives `items`.
        let items = unsafe {
            std::slice::from_raw_parts(store.as_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>(), n)
        };
        // The names live after the items, inside `store`; a pointer outside it is skipped.
        let lo = store.as_ptr() as usize;
        let hi = lo + store.len() * 8;
        let mut out = Vec::new();
        for it in items {
            let at = it.szName.0 as usize;
            if it.szName.is_null() || at < lo || at >= hi {
                continue;
            }
            // SAFETY: szName is non-null and points inside `store` (checked above), where PDH
            // wrote the NUL-terminated instance names; `store` is alive.
            let name = unsafe { it.szName.to_string() }.unwrap_or_default();
            if let Some(cpu) = crate::layout::pdh_cpu(&name) {
                // SAFETY: PDH_FMT_DOUBLE was requested, so the union holds doubleValue.
                let v = unsafe { it.FmtValue.Anonymous.doubleValue };
                if matches!(
                    it.FmtValue.CStatus,
                    PDH_CSTATUS_VALID_DATA | PDH_CSTATUS_NEW_DATA
                ) {
                    out.push((cpu, v));
                }
            }
        }
        out.sort_by_key(|(c, _)| *c);
        Ok(out)
    }
}

impl Drop for Pdh {
    fn drop(&mut self) {
        // SAFETY: closes the query owned by self (and its counter), once.
        let _ = unsafe { PdhCloseQuery(self.query) };
    }
}
