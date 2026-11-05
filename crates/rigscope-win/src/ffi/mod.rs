//! Win32 calls. Each `unsafe` block states why it is sound. Allowed per file in
//! xtask/src/unsafe_scope.rs; the rest of the crate denies unsafe code.

mod d3dkmt;
mod dxgi;
mod firmware;
mod handle;
mod net;
mod pdh;
mod registry;
mod setupapi;
mod storage;
mod system;
mod wmi;

pub use d3dkmt::GpuPerf;
pub use dxgi::adapters_dxgi;

pub use self::wmi::{WMI_TIMEOUT, acpi_thermal_zones, physical_disks_wmi};
pub use firmware::{processors_raw, smbios_raw};
pub use handle::Device;
pub use net::net_adapters;
pub use pdh::Pdh;
pub use registry::{registry_bytes, registry_string, registry_u32};
pub use setupapi::{batteries, pci_display};
pub use storage::{ata_smart_raw, nvme_health_raw, physical_disks_ioctl};
pub use system::{
    ac_online, computer_name, cpu_features, cpu_registry, is_elevated, memory_status, os_version,
    uptime_ms,
};

/// The last Win32 error (`GetLastError`) as a `WinError`.
pub(crate) fn last_error(api: &str) -> crate::WinError {
    from_error(api, &windows::core::Error::from_thread())
}

/// HRESULT_FROM_WIN32: the low 16 bits carry the Win32 code.
pub(crate) fn from_error(api: &str, e: &windows::core::Error) -> crate::WinError {
    crate::WinError::from_code(api, (e.code().0 as u32) & 0xFFFF)
}

/// A NUL-terminated UTF-16 copy of `s`.
pub(crate) fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
