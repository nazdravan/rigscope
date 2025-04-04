use crate::Field;
use serde::{Deserialize, Serialize};

/// Block devices that are real disks (not loop, zram, device-mapper…).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StorageInfo {
    pub disks: Vec<Disk>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Disk {
    /// Kernel name: `nvme0n1`, `sda`, `mmcblk0`, `vda`.
    pub name: String,
    pub model: Field<String>,
    pub vendor: Field<String>,
    pub serial: Field<String>,
    pub firmware: Field<String>,
    pub size_bytes: Field<u64>,
    pub kind: Field<DiskKind>,
    pub interface: Field<Interface>,
    pub removable: bool,
    /// SMART health (needs root, read by the elevated helper through smartctl).
    #[serde(default)]
    pub health: Field<DiskHealth>,
}

/// What rigscope takes from `smartctl --json -a`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskHealth {
    /// The drive's own overall verdict.
    pub passed: Option<bool>,
    pub temperature_c: Option<i32>,
    pub power_on_hours: Option<u64>,
    pub power_cycles: Option<u64>,
    /// NVMe: share of the rated endurance used (can exceed 100).
    pub percent_used: Option<u32>,
    /// NVMe: spare capacity left, percent.
    pub available_spare: Option<u32>,
    pub media_errors: Option<u64>,
    pub written_bytes: Option<u64>,
    /// ATA attribute 5.
    pub reallocated_sectors: Option<u64>,
    /// ATA attribute 197.
    pub pending_sectors: Option<u64>,
    /// ATA attribute 198.
    pub uncorrectable_sectors: Option<u64>,
}

impl DiskHealth {
    /// Problems worth a warning: failed verdict, worn out, or bad sectors / media errors.
    pub fn warnings(&self) -> Vec<&'static str> {
        let some = |v: Option<u64>| v.is_some_and(|n| n > 0);
        let mut out = Vec::new();
        if self.passed == Some(false) {
            out.push("drive reports SMART failure");
        }
        if self.percent_used.is_some_and(|p| p >= 90) {
            out.push("rated endurance nearly used up");
        }
        if some(self.reallocated_sectors) {
            out.push("reallocated sectors");
        }
        if some(self.pending_sectors) {
            out.push("sectors pending reallocation");
        }
        if some(self.uncorrectable_sectors) {
            out.push("uncorrectable sectors");
        }
        if some(self.media_errors) {
            out.push("media errors");
        }
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiskKind {
    NvmeSsd,
    Ssd,
    Hdd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Interface {
    Nvme,
    Sata,
    Usb,
    Scsi,
    Mmc,
    Virtio,
}

impl DiskKind {
    pub fn label(self) -> &'static str {
        match self {
            DiskKind::NvmeSsd => "NVMe SSD",
            DiskKind::Ssd => "SSD",
            DiskKind::Hdd => "HDD",
        }
    }
}

impl Interface {
    pub fn label(self) -> &'static str {
        match self {
            Interface::Nvme => "NVMe",
            Interface::Sata => "SATA",
            Interface::Usb => "USB",
            Interface::Scsi => "SCSI/SAS",
            Interface::Mmc => "MMC/SD",
            Interface::Virtio => "virtio",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthy_drive_has_no_warnings() {
        let h = DiskHealth {
            passed: Some(true),
            percent_used: Some(3),
            reallocated_sectors: Some(0),
            ..Default::default()
        };
        assert!(h.warnings().is_empty());
    }

    #[test]
    fn warnings_for_failing_worn_and_bad_sectors() {
        let h = DiskHealth {
            passed: Some(false),
            percent_used: Some(98),
            reallocated_sectors: Some(8),
            pending_sectors: Some(1),
            media_errors: Some(2),
            ..Default::default()
        };
        assert_eq!(
            h.warnings(),
            vec![
                "drive reports SMART failure",
                "rated endurance nearly used up",
                "reallocated sectors",
                "sectors pending reallocation",
                "media errors"
            ]
        );
    }

    #[test]
    fn labels() {
        assert_eq!(DiskKind::NvmeSsd.label(), "NVMe SSD");
        assert_eq!(DiskKind::Ssd.label(), "SSD");
        assert_eq!(DiskKind::Hdd.label(), "HDD");
        assert_eq!(Interface::Sata.label(), "SATA");
        assert_eq!(Interface::Usb.label(), "USB");
        assert_eq!(Interface::Mmc.label(), "MMC/SD");
        assert_eq!(Interface::Virtio.label(), "virtio");
    }
}
