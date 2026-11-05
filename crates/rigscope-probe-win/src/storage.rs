use crate::{WinSource, field_err, parse_ata_smart, parse_nvme_health};
use rigscope_model::{Disk, DiskHealth, DiskKind, Field, Interface, StorageInfo, clean_text};
use rigscope_win::layout::{nvme_log_from_response, parse_device_descriptor};
use rigscope_win::{PhysicalDiskRaw, WinError};

pub const MAX_DISKS: usize = rigscope_model::MAX_DUMP_DISKS;
pub const NO_SMART_USB: &str = "the USB bridge does not pass SMART through";
pub const NO_SMART_RAID: &str = "behind a RAID controller";
const NO_SMART_VIRTUAL: &str = "virtual disk (no SMART)";

pub fn interface_of(bus: u16) -> Option<Interface> {
    Some(match bus {
        3 | 11 => Interface::Sata,
        17 => Interface::Nvme,
        7 => Interface::Usb,
        1 | 10 => Interface::Scsi,
        12 | 13 => Interface::Mmc,
        14..=16 => Interface::Virtual,
        8 => Interface::Raid,
        _ => return None,
    })
}

fn kind_of(bus: Option<u16>, media: Option<u16>) -> Field<DiskKind> {
    match (bus, media) {
        (Some(17), _) => Field::Known(DiskKind::NvmeSsd),
        (_, Some(4)) => Field::Known(DiskKind::Ssd),
        (_, Some(3)) => Field::Known(DiskKind::Hdd),
        _ => Field::Unknown,
    }
}

fn health(src: &dyn WinSource, n: u32, iface: Option<Interface>) -> Field<DiskHealth> {
    let from_err = |e: WinError| match e {
        WinError::AccessDenied => Field::NeedsElevation,
        e => field_err(&e),
    };
    match iface {
        Some(Interface::Nvme) => match src.nvme_health_log(n) {
            Ok(out) => nvme_log_from_response(&out)
                .and_then(parse_nvme_health)
                .map_or(
                    Field::Unavailable("unreadable NVMe health log".into()),
                    Field::Known,
                ),
            Err(e) => from_err(e),
        },
        Some(Interface::Usb) => match src.ata_smart(n) {
            Ok(a) => parse_ata_smart(&a.attributes, &a.thresholds).map_or(
                Field::Unavailable("unreadable SMART data".into()),
                Field::Known,
            ),
            Err(WinError::AccessDenied) => Field::NeedsElevation,
            Err(_) => Field::Unavailable(NO_SMART_USB.into()),
        },
        // Intel RST and similar report BusType RAID but often pass ATA SMART through.
        Some(Interface::Raid) => match src.ata_smart(n) {
            Ok(a) => parse_ata_smart(&a.attributes, &a.thresholds)
                .map_or(Field::Unavailable(NO_SMART_RAID.into()), Field::Known),
            Err(WinError::AccessDenied) => Field::NeedsElevation,
            Err(_) => Field::Unavailable(NO_SMART_RAID.into()),
        },
        Some(Interface::Virtual) => Field::Unavailable(NO_SMART_VIRTUAL.into()),
        _ => match src.ata_smart(n) {
            Ok(a) => parse_ata_smart(&a.attributes, &a.thresholds).map_or(
                Field::Unavailable("unreadable SMART data".into()),
                Field::Known,
            ),
            Err(e) => from_err(e),
        },
    }
}

fn disk_from(src: &dyn WinSource, r: &PhysicalDiskRaw) -> Disk {
    let iface = r.bus_type.and_then(interface_of);
    let text = |s: &Option<String>| Field::from_option(s.as_deref().and_then(clean_text));
    Disk {
        name: format!("PhysicalDrive{}", r.number),
        model: text(&r.friendly_name),
        vendor: Field::Unknown,
        serial: text(&r.serial),
        firmware: text(&r.firmware),
        size_bytes: Field::from_option(r.size.filter(|s| *s > 0)),
        kind: kind_of(r.bus_type, r.media_type),
        interface: Field::from_option(iface),
        removable: matches!(iface, Some(Interface::Usb) | Some(Interface::Mmc)),
        health: health(src, r.number, iface),
    }
}

fn from_ioctl(src: &dyn WinSource) -> Vec<PhysicalDiskRaw> {
    src.physical_disks_ioctl()
        .unwrap_or_default()
        .into_iter()
        .map(|d| {
            let p = parse_device_descriptor(&d.descriptor).unwrap_or_default();
            PhysicalDiskRaw {
                number: d.number,
                friendly_name: match (p.vendor, p.product) {
                    (Some(v), Some(m)) if !m.starts_with(&v) => Some(format!("{v} {m}")),
                    (_, m) => m,
                },
                serial: p.serial,
                firmware: p.revision,
                size: d.length,
                bus_type: p.bus_type,
                media_type: None,
            }
        })
        .collect()
}

pub(crate) fn read_storage(src: &dyn WinSource) -> StorageInfo {
    let mut raw = match src.physical_disks_wmi() {
        Ok(v) => v,
        Err(_) => from_ioctl(src),
    };
    raw.sort_by_key(|d| d.number);
    raw.truncate(MAX_DISKS);
    StorageInfo {
        disks: raw.iter().map(|r| disk_from(src, r)).collect(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::indexing_slicing,
        clippy::field_reassign_with_default
    )]
    use super::*;
    use crate::MemWin;
    use crate::smart::tests::{ata_sectors, nvme_log};
    use rigscope_model::{DiskKind, Field};
    use rigscope_win::{AtaSmartRaw, DiskIoctlRaw, PhysicalDiskRaw, WinError};

    fn nvme_response(log: &[u8]) -> Vec<u8> {
        let mut out = vec![0u8; rigscope_win::layout::NVME_HEALTH_OUT_LEN];
        out[24..28].copy_from_slice(&40u32.to_le_bytes());
        out[28..32].copy_from_slice(&512u32.to_le_bytes());
        out[48..560].copy_from_slice(log);
        out
    }

    fn disk(n: u32, bus: u16, media: u16) -> PhysicalDiskRaw {
        PhysicalDiskRaw {
            number: n,
            friendly_name: Some("Samsung SSD 970 EVO Plus 500GB".into()),
            serial: Some("S4EVNX0N123456A".into()),
            firmware: Some("2B2QEXM7".into()),
            size: Some(500_107_862_016),
            bus_type: Some(bus),
            media_type: Some(media),
        }
    }

    #[test]
    fn nvme_disk_with_health() {
        let mut w = MemWin::default();
        w.disks_wmi = Ok(vec![disk(0, 17, 4)]);
        w.nvme.insert(0, Ok(nvme_response(&nvme_log(312, 2, 0, 0))));
        let s = read_storage(&w);
        let d = &s.disks[0];
        assert_eq!(d.name, "PhysicalDrive0");
        assert_eq!(d.kind, Field::Known(DiskKind::NvmeSsd));
        assert_eq!(d.interface, Field::Known(Interface::Nvme));
        assert_eq!(d.size_bytes, Field::Known(500_107_862_016));
        assert_eq!(d.health.known().unwrap().temperature_c, Some(39));
    }

    #[test]
    fn sata_hdd_usb_virtual_raid() {
        let mut w = MemWin::default();
        let (a, t) = ata_sectors(&[(5, 0x33, 100, 0)], &[(5, 10)]);
        w.disks_wmi = Ok(vec![
            disk(0, 11, 3),
            disk(1, 7, 0),
            disk(2, 14, 0),
            disk(3, 8, 0),
        ]);
        w.ata.insert(
            0,
            Ok(AtaSmartRaw {
                attributes: a,
                thresholds: t,
            }),
        );
        w.ata.insert(
            1,
            Err(WinError::Failed {
                api: "SMART_RCV_DRIVE_DATA".into(),
                code: 1,
            }),
        );
        let s = read_storage(&w);
        assert_eq!(s.disks[0].kind, Field::Known(DiskKind::Hdd));
        assert_eq!(s.disks[0].interface, Field::Known(Interface::Sata));
        assert!(s.disks[0].health.known().is_some());
        assert_eq!(s.disks[1].interface, Field::Known(Interface::Usb));
        assert!(s.disks[1].removable);
        assert_eq!(s.disks[1].health, Field::Unavailable(NO_SMART_USB.into()));
        assert_eq!(s.disks[1].kind, Field::Unknown);
        assert_eq!(s.disks[2].interface, Field::Known(Interface::Virtual));
        assert_eq!(s.disks[3].health, Field::Unavailable(NO_SMART_RAID.into()));
    }

    #[test]
    fn not_elevated_means_needs_elevation() {
        let mut w = MemWin::default();
        w.disks_wmi = Ok(vec![disk(0, 11, 4)]);
        w.ata.insert(0, Err(WinError::AccessDenied));
        assert_eq!(read_storage(&w).disks[0].health, Field::NeedsElevation);
    }

    #[test]
    fn wmi_timeout_falls_back_to_ioctl() {
        let mut w = MemWin::default();
        w.disks_wmi = Err(WinError::Timeout);
        let mut d = vec![0u8; 36];
        d[16..20].copy_from_slice(&36u32.to_le_bytes());
        d.extend_from_slice(b"WDC WD10EZEX\0");
        d[28..32].copy_from_slice(&11u32.to_le_bytes());
        w.disks_ioctl = Ok(vec![DiskIoctlRaw {
            number: 1,
            descriptor: d,
            length: Some(1_000_204_886_016),
        }]);
        let s = read_storage(&w);
        assert_eq!(s.disks.len(), 1);
        assert_eq!(s.disks[0].name, "PhysicalDrive1");
        assert_eq!(s.disks[0].model, Field::Known("WDC WD10EZEX".into()));
        assert_eq!(s.disks[0].interface, Field::Known(Interface::Sata));
        assert_eq!(s.disks[0].kind, Field::Unknown); // without WMI there is no MediaType
    }

    #[test]
    fn storage_spaces_is_virtual() {
        let mut w = MemWin::default();
        w.disks_wmi = Ok(vec![disk(0, 16, 0)]);
        let d = &read_storage(&w).disks[0];
        assert_eq!(d.interface, Field::Known(Interface::Virtual));
        assert_eq!(d.health, Field::Unavailable(NO_SMART_VIRTUAL.into()));
    }

    #[test]
    fn usb_with_unparseable_smart_says_unreadable() {
        let mut w = MemWin::default();
        w.disks_wmi = Ok(vec![disk(0, 7, 0)]);
        w.ata.insert(
            0,
            Ok(AtaSmartRaw {
                attributes: vec![0; 10],
                thresholds: vec![0; 10],
            }),
        );
        assert_eq!(
            read_storage(&w).disks[0].health,
            Field::Unavailable("unreadable SMART data".into())
        );
    }

    #[test]
    fn raid_bus_with_readable_ata_smart_is_known() {
        let mut w = MemWin::default();
        let (a, t) = ata_sectors(&[(9, 0x32, 100, 1234)], &[(9, 0)]);
        w.disks_wmi = Ok(vec![disk(0, 8, 4)]);
        w.ata.insert(
            0,
            Ok(AtaSmartRaw {
                attributes: a,
                thresholds: t,
            }),
        );
        let d = &read_storage(&w).disks[0];
        assert_eq!(d.interface, Field::Known(Interface::Raid));
        assert!(d.health.known().is_some());
    }

    #[test]
    fn raid_bus_not_elevated_needs_elevation() {
        let mut w = MemWin::default();
        w.disks_wmi = Ok(vec![disk(0, 8, 4)]);
        w.ata.insert(0, Err(WinError::AccessDenied));
        assert_eq!(read_storage(&w).disks[0].health, Field::NeedsElevation);
    }

    #[test]
    fn raid_bus_with_unparseable_smart_stays_unavailable() {
        let mut w = MemWin::default();
        w.disks_wmi = Ok(vec![disk(0, 8, 4)]);
        w.ata.insert(
            0,
            Ok(AtaSmartRaw {
                attributes: vec![0; 10],
                thresholds: vec![0; 10],
            }),
        );
        assert_eq!(
            read_storage(&w).disks[0].health,
            Field::Unavailable(NO_SMART_RAID.into())
        );
    }
}
