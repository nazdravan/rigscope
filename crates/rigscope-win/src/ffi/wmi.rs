//! WMI queries through the `wmi` crate. No `unsafe` here.

use crate::{PhysicalDiskRaw, ThermalZoneRaw, WinError};
use serde::Deserialize;
use std::sync::mpsc;
use std::time::Duration;

pub const WMI_TIMEOUT: Duration = Duration::from_secs(5);

/// `WBEM_E_ACCESS_DENIED`.
const WBEM_E_ACCESS_DENIED: u32 = 0x8004_1003;
/// `WBEM_E_NOT_SUPPORTED`: the class has no provider data on this machine.
const WBEM_E_NOT_SUPPORTED: u32 = 0x8004_100C;

/// Runs `f` on its own thread (WMI needs a COM apartment per thread) and gives up after
/// WMI_TIMEOUT; a hung WMI service then costs one parked thread, not a frozen UI.
fn with_timeout<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, WinError> + Send + 'static,
) -> Result<T, WinError> {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("rigscope-wmi".into())
        .spawn(move || {
            // The receiver may have given up already; nothing to do then.
            let _ = tx.send(f());
        })
        .map_err(|_| WinError::Failed {
            api: "thread".into(),
            code: 0,
        })?;
    match rx.recv_timeout(WMI_TIMEOUT) {
        Ok(r) => r,
        Err(mpsc::RecvTimeoutError::Timeout) => Err(WinError::Timeout),
        // The worker ended without sending (it panicked).
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(WinError::Failed {
            api: "WMI thread".into(),
            code: 0,
        }),
    }
}

fn wmi_err(e: ::wmi::WMIError) -> WinError {
    match e {
        ::wmi::WMIError::HResultError { hres } if hres as u32 == WBEM_E_ACCESS_DENIED => {
            WinError::AccessDenied
        }
        ::wmi::WMIError::HResultError { hres } => WinError::Failed {
            api: "WMI".into(),
            code: hres as u32,
        },
        e => WinError::Failed {
            api: format!("WMI: {e}"),
            code: 0,
        },
    }
}

#[derive(Deserialize)]
#[serde(rename = "MSFT_PhysicalDisk", rename_all = "PascalCase")]
struct MsftPhysicalDisk {
    device_id: Option<String>,
    friendly_name: Option<String>,
    serial_number: Option<String>,
    firmware_version: Option<String>,
    size: Option<u64>,
    bus_type: Option<u16>,
    media_type: Option<u16>,
}

pub fn physical_disks_wmi() -> Result<Vec<PhysicalDiskRaw>, WinError> {
    with_timeout(|| {
        let con = ::wmi::WMIConnection::with_namespace_path(r"ROOT\Microsoft\Windows\Storage")
            .map_err(wmi_err)?;
        let rows: Vec<MsftPhysicalDisk> = con.query().map_err(wmi_err)?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                Some(PhysicalDiskRaw {
                    number: r.device_id.as_deref()?.trim().parse().ok()?,
                    friendly_name: r.friendly_name,
                    serial: r.serial_number.map(|s| s.trim().to_string()),
                    firmware: r.firmware_version,
                    size: r.size,
                    bus_type: r.bus_type,
                    media_type: r.media_type,
                })
            })
            .collect())
    })
}

#[derive(Deserialize)]
#[serde(rename = "MSAcpi_ThermalZoneTemperature", rename_all = "PascalCase")]
struct ThermalZone {
    instance_name: String,
    current_temperature: u32,
}

pub fn acpi_thermal_zones() -> Result<Vec<ThermalZoneRaw>, WinError> {
    with_timeout(|| {
        let con = ::wmi::WMIConnection::with_namespace_path(r"ROOT\WMI").map_err(wmi_err)?;
        let rows: Vec<ThermalZone> = con.query().map_err(|e| match e {
            // Not supported on this machine (no ACPI thermal zones): an absent value, not a failure.
            ::wmi::WMIError::HResultError { hres } if hres as u32 == WBEM_E_NOT_SUPPORTED => {
                WinError::NotAvailable
            }
            e => wmi_err(e),
        })?;
        Ok(rows
            .into_iter()
            .map(|z| ThermalZoneRaw {
                instance: z.instance_name,
                tenths_kelvin: z.current_temperature,
            })
            .collect())
    })
}
