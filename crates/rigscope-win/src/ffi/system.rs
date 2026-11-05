#![allow(unsafe_code)]

use super::{from_error, registry_bytes, registry_string, registry_u32};
use crate::{CpuRegistryRaw, MemoryStatusRaw, OsVersionRaw, WinError};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::System::SystemInformation::{
    ComputerNamePhysicalDnsHostname, GetComputerNameExW, GetTickCount64, GlobalMemoryStatusEx,
    MEMORYSTATUSEX,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::core::PWSTR;

const CURRENT_VERSION: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
const CPU0: &str = r"HARDWARE\DESCRIPTION\System\CentralProcessor\0";

/// Installed and available physical memory.
pub fn memory_status() -> Result<MemoryStatusRaw, WinError> {
    let mut m = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    // SAFETY: `m` is a properly sized MEMORYSTATUSEX with dwLength set.
    unsafe { GlobalMemoryStatusEx(&mut m) }.map_err(|e| from_error("GlobalMemoryStatusEx", &e))?;
    Ok(MemoryStatusRaw {
        total_phys: m.ullTotalPhys,
        avail_phys: m.ullAvailPhys,
    })
}

/// Milliseconds since boot.
pub fn uptime_ms() -> u64 {
    // SAFETY: no arguments, no side effects.
    unsafe { GetTickCount64() }
}

/// The physical DNS host name.
pub fn computer_name() -> Result<String, WinError> {
    let mut buf = [0u16; 256];
    let mut len = buf.len() as u32;
    // SAFETY: `buf` has `len` UTF-16 units; the function writes at most that many.
    unsafe {
        GetComputerNameExW(
            ComputerNamePhysicalDnsHostname,
            Some(PWSTR(buf.as_mut_ptr())),
            &mut len,
        )
    }
    .map_err(|e| from_error("GetComputerNameExW", &e))?;
    Ok(String::from_utf16_lossy(
        buf.get(..len as usize).unwrap_or(&[]),
    ))
}

/// The OS version from `CurrentVersion`; an unreadable build means not available.
pub fn os_version() -> Result<OsVersionRaw, WinError> {
    let build = registry_string(CURRENT_VERSION, "CurrentBuild").and_then(|b| b.parse().ok());
    if build.is_none() {
        return Err(WinError::NotAvailable);
    }
    Ok(OsVersionRaw {
        product_name: registry_string(CURRENT_VERSION, "ProductName"),
        display_version: registry_string(CURRENT_VERSION, "DisplayVersion"),
        current_build: build,
        ubr: registry_u32(CURRENT_VERSION, "UBR"),
        edition_id: registry_string(CURRENT_VERSION, "EditionID"),
    })
}

/// CPU 0's registry description; neither name nor clock means not available.
pub fn cpu_registry() -> Result<CpuRegistryRaw, WinError> {
    let r = CpuRegistryRaw {
        name: registry_string(CPU0, "ProcessorNameString"),
        vendor: registry_string(CPU0, "VendorIdentifier"),
        mhz: registry_u32(CPU0, "~MHz"),
        update_revision: registry_bytes(CPU0, "Update Revision"),
    };
    if r.name.is_none() && r.mhz.is_none() {
        return Err(WinError::NotAvailable);
    }
    Ok(r)
}

/// Instruction-set extensions the CPU and OS support (std's runtime detection: safe).
pub fn cpu_features() -> Vec<String> {
    #[cfg(target_arch = "x86_64")]
    {
        let mut v = Vec::new();
        macro_rules! f {
            ($($name:tt),*) => {
                $( if std::arch::is_x86_feature_detected!($name) { v.push($name.to_string()); } )*
            };
        }
        f!(
            "sse2",
            "sse3",
            "ssse3",
            "sse4.1",
            "sse4.2",
            "popcnt",
            "aes",
            "pclmulqdq",
            "avx",
            "avx2",
            "fma",
            "bmi1",
            "bmi2",
            "lzcnt",
            "f16c",
            "sha",
            "rdrand",
            "rdseed",
            "adx",
            "avx512f",
            "avx512bw",
            "avx512vl",
            "avx512vnni",
            "vaes",
            "vpclmulqdq",
            "gfni"
        );
        v
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        Vec::new()
    }
}

/// Whether this process runs with an elevated token; any failure reads as not elevated.
pub fn is_elevated() -> bool {
    let mut token = HANDLE::default();
    // SAFETY: the pseudo-handle of the current process; `token` receives an owned handle.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }.is_err() {
        return false;
    }
    let mut e = TOKEN_ELEVATION::default();
    let mut len = 0u32;
    // SAFETY: `e` is a TOKEN_ELEVATION of the size passed; `len` is a local out-parameter.
    let ok = unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            Some((&raw mut e).cast()),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut len,
        )
    }
    .is_ok();
    // SAFETY: closes the token opened above, once.
    let _ = unsafe { CloseHandle(token) };
    ok && e.TokenIsElevated != 0
}

/// AC line status: online, offline, or unknown (255).
pub fn ac_online() -> Result<Option<bool>, WinError> {
    let mut s = SYSTEM_POWER_STATUS::default();
    // SAFETY: `s` is a valid out-parameter.
    unsafe { GetSystemPowerStatus(&mut s) }.map_err(|e| from_error("GetSystemPowerStatus", &e))?;
    Ok(match s.ACLineStatus {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    })
}
