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
