//! `\\.\PhysicalDriveN` IOCTLs through `Device`. No `unsafe` here.

use super::Device;
use crate::layout::{
    ata_data_from_response, ata_smart_request, device_descriptor_query, nvme_health_query,
    ATA_READ_ATTRIBUTES, ATA_READ_THRESHOLDS, ATA_SMART_OUT_LEN, IOCTL_DISK_GET_LENGTH_INFO,
    IOCTL_STORAGE_QUERY_PROPERTY, NVME_HEALTH_OUT_LEN, SMART_RCV_DRIVE_DATA,
};
use crate::{AtaSmartRaw, DiskIoctlRaw, WinError};

/// PhysicalDrive numbers are not contiguous after hot-unplug; probe a fixed range.
const PROBE: u32 = 32;
/// `STORAGE_DEVICE_DESCRIPTOR` with its strings; real ones are a few hundred bytes.
const DESCRIPTOR_LEN: usize = 1024;
/// `GET_LENGTH_INFORMATION`: one i64.
const LENGTH_LEN: usize = 8;

fn path(n: u32) -> String {
    format!(r"\\.\PhysicalDrive{n}")
}

pub fn physical_disks_ioctl() -> Result<Vec<DiskIoctlRaw>, WinError> {
    let mut out = Vec::new();
    for n in 0..PROBE {
        let Ok(dev) = Device::open(&path(n), false) else {
            continue;
        };
        let Ok(descriptor) = dev.ioctl(
            IOCTL_STORAGE_QUERY_PROPERTY,
            &device_descriptor_query(),
            DESCRIPTOR_LEN,
        ) else {
            continue;
        };
        // Needs read access, which a non-elevated process does not get on a physical drive.
        let length = Device::open(&path(n), true)
            .ok()
            .and_then(|d| d.ioctl(IOCTL_DISK_GET_LENGTH_INFO, &[], LENGTH_LEN).ok())
            .and_then(|b| Some(u64::from_le_bytes(b.get(..8)?.try_into().ok()?)));
        out.push(DiskIoctlRaw {
            number: n,
            descriptor,
            length,
        });
    }
    Ok(out)
}

/// The raw `STORAGE_PROTOCOL_DATA_DESCRIPTOR` holding the NVMe SMART/health log.
pub fn nvme_health_raw(disk: u32) -> Result<Vec<u8>, WinError> {
    Device::open(&path(disk), true)?.ioctl(
        IOCTL_STORAGE_QUERY_PROPERTY,
        &nvme_health_query(),
        NVME_HEALTH_OUT_LEN,
    )
}

pub fn ata_smart_raw(disk: u32) -> Result<AtaSmartRaw, WinError> {
    let dev = Device::open(&path(disk), true)?;
    let drive = u8::try_from(disk).unwrap_or(0);
    let read = |feature| -> Result<Vec<u8>, WinError> {
        let out = dev.ioctl(
            SMART_RCV_DRIVE_DATA,
            &ata_smart_request(drive, feature),
            ATA_SMART_OUT_LEN,
        )?;
        ata_data_from_response(&out)
            .map(<[u8]>::to_vec)
            .ok_or(WinError::NotAvailable)
    };
    Ok(AtaSmartRaw {
        attributes: read(ATA_READ_ATTRIBUTES)?,
        thresholds: read(ATA_READ_THRESHOLDS)?,
    })
}
