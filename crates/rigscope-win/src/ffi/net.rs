#![allow(unsafe_code)]

use crate::{NetAdapterRaw, WinError};
use windows::core::PWSTR;
use windows::Win32::NetworkManagement::IpHelper::{
    GetAdaptersAddresses, GetIfEntry2, GAA_FLAG_INCLUDE_ALL_INTERFACES, GAA_FLAG_SKIP_ANYCAST,
    GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST, IP_ADAPTER_ADDRESSES_LH, MIB_IF_ROW2,
};
use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows::Win32::Networking::WinSock::AF_UNSPEC;

/// A bound on the list walk. GAA_FLAG_INCLUDE_ALL_INTERFACES also lists every filter
/// (WFP, QoS, ...) interface: a Windows 10 laptop with Hyper-V showed 66, so 64 truncated.
/// Filter rows are flagged (`NetAdapterRaw::filter`) and dropped by the probe.
const MAX: usize = 256;
const MAX_BYTES: u32 = 1024 * 1024;
const ERROR_BUFFER_OVERFLOW: u32 = 111;
const ERROR_NO_DATA: u32 = 232;

/// A wide string inside the adapter list, or None when the pointer is null.
///
/// # Safety
/// `p` is null or points at a NUL-terminated UTF-16 string that outlives the call.
unsafe fn wstr(p: PWSTR) -> Option<String> {
    if p.is_null() {
        return None;
    }
    // SAFETY: non-null, NUL-terminated and live, per this function's contract.
    unsafe { p.to_string() }.ok()
}

pub fn net_adapters() -> Result<Vec<NetAdapterRaw>, WinError> {
    let flags = GAA_FLAG_SKIP_ANYCAST
        | GAA_FLAG_SKIP_MULTICAST
        | GAA_FLAG_SKIP_DNS_SERVER
        | GAA_FLAG_INCLUDE_ALL_INTERFACES;
    let mut size = 32 * 1024u32;
    let mut store: Vec<u64>;
    let mut tries = 0;
    loop {
        tries += 1;
        store = vec![0u64; (size as usize).div_ceil(8)];
        // SAFETY: `store` has at least `size` bytes, 8-aligned, for the linked list the API
        // writes; `size` is a live in/out parameter.
        let r = unsafe {
            GetAdaptersAddresses(
                u32::from(AF_UNSPEC.0),
                flags,
                None,
                Some(store.as_mut_ptr().cast()),
                &mut size,
            )
        };
        match r {
            0 => break,
            ERROR_NO_DATA => return Ok(Vec::new()),
            // `size` now holds the size needed; the list may grow between calls.
            ERROR_BUFFER_OVERFLOW if size <= MAX_BYTES && tries < 4 => continue,
            code => return Err(WinError::from_code("GetAdaptersAddresses", code)),
        }
    }
    let mut out = Vec::new();
    let mut p = store.as_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
    while !p.is_null() && out.len() < MAX {
        // SAFETY: `p` is the list head at the start of `store` or a `Next` link the API wrote;
        // all records live inside `store`, which is alive and not written to while borrowed.
        let a = unsafe { &*p };
        // SAFETY: FriendlyName/Description are null or NUL-terminated strings inside `store`.
        let name = unsafe { wstr(a.FriendlyName) }.unwrap_or_default();
        // SAFETY: as above.
        let description = unsafe { wstr(a.Description) };
        let mac = (a.PhysicalAddressLength == 6).then(|| {
            a.PhysicalAddress
                .get(..6)
                .unwrap_or(&[])
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(":")
        });
        let mut row = MIB_IF_ROW2 {
            InterfaceLuid: a.Luid,
            ..Default::default()
        };
        // SAFETY: `row` is a live MIB_IF_ROW2 with the LUID set, as the API requires.
        let found = unsafe { GetIfEntry2(&mut row) }.is_ok();
        // InterfaceAndOperStatusFlags (netioapi.h, LSB first): bit 0 HardwareInterface,
        // bit 1 FilterInterface. windows 0.62 exposes it as an opaque `_bitfield: u8`.
        let bits = row.InterfaceAndOperStatusFlags._bitfield;
        let hardware = found && bits & 0b01 != 0;
        let filter = found && bits & 0b10 != 0;
        out.push(NetAdapterRaw {
            name,
            description,
            if_type: a.IfType,
            up: a.OperStatus == IfOperStatusUp,
            speed_bps: (a.TransmitLinkSpeed != u64::MAX && a.TransmitLinkSpeed != 0)
                .then_some(a.TransmitLinkSpeed),
            mac,
            mtu: (a.Mtu != u32::MAX).then_some(a.Mtu),
            hardware,
            filter,
        });
        p = a.Next;
    }
    Ok(out)
}
