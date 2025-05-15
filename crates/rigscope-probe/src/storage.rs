//! Disks from `/sys/block` (no root needed except for SATA serial numbers).

use crate::{ReadError, Source};
use rigscope_model::{clean_text, Disk, DiskKind, Field, Interface, StorageInfo};

pub const BLOCK_DIR: &str = "/sys/block";
pub const NO_SMART: &str = "this kind of device has no SMART data";
/// Most disks listed.
pub const MAX_DISKS: usize = 256;
const SECTOR: u64 = 512;
/// Kernel names of block devices that are not disks.
const VIRTUAL_PREFIXES: &[&str] = &["loop", "ram", "zram", "dm-", "md", "nbd", "sr", "fd"];

fn text(src: &dyn Source, path: &str) -> Option<String> {
    src.read(path)
        .ok()
        .map(|b| String::from_utf8_lossy(&b).trim().to_string())
}

/// Bus of a disk from its kernel name and canonical sysfs path.
pub fn interface_for(name: &str, canonical: &str) -> Option<Interface> {
    if name.starts_with("nvme") {
        Some(Interface::Nvme)
    } else if name.starts_with("mmcblk") {
        Some(Interface::Mmc)
    } else if canonical.contains("/virtio") {
        Some(Interface::Virtio)
    } else if canonical.contains("/usb") {
        Some(Interface::Usb)
    } else if canonical.contains("/ata") {
        Some(Interface::Sata)
    } else if name.starts_with("sd") {
        Some(Interface::Scsi)
    } else {
        None
    }
}

/// SSD/HDD from the `rotational` flag, only where the flag is trustworthy.
pub fn kind_for(interface: Option<Interface>, rotational: Option<&str>) -> Option<DiskKind> {
    match interface {
        Some(Interface::Nvme) => Some(DiskKind::NvmeSsd),
        // USB bridges, virtual disks and MMC report the flag unreliably.
        Some(
            Interface::Usb
            | Interface::Virtio
            | Interface::Mmc
            | Interface::Virtual
            | Interface::Raid,
        )
        | None => None,
        Some(Interface::Sata | Interface::Scsi) => match rotational {
            Some("0") => Some(DiskKind::Ssd),
            Some("1") => Some(DiskKind::Hdd),
            _ => None,
        },
    }
}

/// Makers whose model strings start with their name (compared without case), as the kernel
/// reports them for SATA (`vendor` = "ATA") and NVMe (no `vendor` file at all).
const MODEL_MAKERS: &[(&str, &str)] = &[
    ("samsung", "Samsung"),
    ("spcc", "Silicon Power"),
    ("kingston", "Kingston"),
    ("crucial", "Crucial"),
    ("micron", "Micron"),
    ("wdc", "Western Digital"),
    ("wd", "Western Digital"),
    ("seagate", "Seagate"),
    ("toshiba", "Toshiba"),
    ("kioxia", "Kioxia"),
    ("sandisk", "SanDisk"),
    ("intel", "Intel"),
    ("hgst", "HGST"),
    ("hitachi", "Hitachi"),
    ("adata", "ADATA"),
    ("transcend", "Transcend"),
    ("corsair", "Corsair"),
    ("pny", "PNY"),
    ("lexar", "Lexar"),
    ("patriot", "Patriot"),
    ("apacer", "Apacer"),
    ("netac", "Netac"),
    ("plextor", "Plextor"),
    ("liteon", "Lite-On"),
    ("lite-on", "Lite-On"),
    ("sk", "SK hynix"),
    ("hynix", "SK hynix"),
    ("maxtor", "Maxtor"),
    ("ocz", "OCZ"),
    ("teamgroup", "Team Group"),
];

/// The maker named by the first word of a disk model, or by a well-known part-number prefix
/// (`ST1000DM003` Seagate, `CT500MX500SSD1` Crucial, `WDS500G2B0A` Western Digital).
pub fn maker_from_model(model: &str) -> Option<&'static str> {
    let first = model.split_whitespace().next()?;
    let lower = first.to_ascii_lowercase();
    if let Some((_, maker)) = MODEL_MAKERS.iter().find(|(word, _)| *word == lower) {
        return Some(maker);
    }
    let digit_after = |prefix: &str| {
        lower
            .strip_prefix(prefix)
            .and_then(|rest| rest.chars().next())
            .is_some_and(|c| c.is_ascii_digit())
    };
    if digit_after("st") {
        Some("Seagate")
    } else if digit_after("ct") {
        Some("Crucial")
    } else if lower.starts_with("wds") || digit_after("wd") {
        Some("Western Digital")
    } else {
        None
    }
}

fn vendor(raw: Option<String>) -> Option<String> {
    let v = clean_text(&raw?)?;
    // libata always says "ATA"; virtio gives a PCI id. Neither is the maker.
    if v == "ATA" || v.starts_with("0x") {
        None
    } else {
        Some(v)
    }
}

pub fn read_storage(src: &dyn Source) -> StorageInfo {
    let Ok(names) = src.list(BLOCK_DIR) else {
        return StorageInfo::default();
    };
    let disks = names
        .iter()
        .filter(|n| !VIRTUAL_PREFIXES.iter().any(|p| n.starts_with(p)))
        .take(MAX_DISKS)
        .filter_map(|name| read_disk(src, name))
        .collect();
    StorageInfo { disks }
}

fn read_disk(src: &dyn Source, name: &str) -> Option<Disk> {
    let base = format!("{BLOCK_DIR}/{name}");
    let canonical = src.canonical(&base).unwrap_or_default();
    if canonical.contains("/virtual/") {
        return None;
    }
    let dev = |f: &str| text(src, &format!("{base}/device/{f}"));
    let interface = interface_for(name, &canonical);
    let rotational = text(src, &format!("{base}/queue/rotational"));
    let size = text(src, &format!("{base}/size"))
        .and_then(|s| s.parse::<u64>().ok())
        .and_then(|sectors| sectors.checked_mul(SECTOR));
    let serial = match src.read(&format!("{base}/device/serial")) {
        Ok(b) => Field::from_option(clean_text(&String::from_utf8_lossy(&b))),
        Err(ReadError::PermissionDenied) => Field::NeedsElevation,
        Err(_)
            if matches!(
                interface,
                Some(Interface::Sata | Interface::Scsi | Interface::Usb)
            ) =>
        {
            Field::Unavailable("not exposed by the kernel without root".into())
        }
        Err(_) => Field::Unknown,
    };
    let model = dev("model").as_deref().and_then(clean_text);
    let vendor = vendor(dev("vendor")).or_else(|| {
        model
            .as_deref()
            .and_then(maker_from_model)
            .map(str::to_string)
    });
    Some(Disk {
        name: name.to_string(),
        model: Field::from_option(model),
        vendor: Field::from_option(vendor),
        serial,
        firmware: Field::from_option(
            dev("firmware_rev")
                .or_else(|| dev("rev"))
                .as_deref()
                .and_then(clean_text),
        ),
        size_bytes: Field::from_option(size),
        kind: Field::from_option(kind_for(interface, rotational.as_deref())),
        interface: Field::from_option(interface),
        removable: text(src, &format!("{base}/removable")).as_deref() == Some("1"),
        health: match interface {
            Some(Interface::Nvme | Interface::Sata | Interface::Scsi | Interface::Usb) => {
                Field::NeedsElevation
            }
            _ => Field::Unavailable(NO_SMART.to_string()),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makers_from_model_names() {
        for (model, maker) in [
            ("Samsung SSD 860", Some("Samsung")),
            ("SAMSUNG MZVL21T0HCLR-00BL2", Some("Samsung")),
            ("SPCC Solid State", Some("Silicon Power")),
            ("WDC WD10EZEX-08W", Some("Western Digital")),
            ("WD Blue SN570 1TB", Some("Western Digital")),
            ("WDS500G2B0A-00SM50", Some("Western Digital")),
            ("ST1000DM003-1CH162", Some("Seagate")),
            ("CT500MX500SSD1", Some("Crucial")),
            ("KINGSTON SA400S37240G", Some("Kingston")),
            ("SK hynix PC711", Some("SK hynix")),
            // Words that only look like a prefix are not guessed.
            ("STORAGE DEVICE", None),
            ("Ultra Fit", None),
            ("", None),
        ] {
            assert_eq!(maker_from_model(model), maker, "{model}");
        }
    }

    #[test]
    fn interfaces_from_paths() {
        assert_eq!(
            interface_for(
                "sda",
                "/sys/devices/pci0000:00/0000:00:14.0/usb3/3-1/x/block/sda"
            ),
            Some(Interface::Usb)
        );
        assert_eq!(
            interface_for(
                "sda",
                "/sys/devices/pci0000:00/0000:00:17.0/ata1/host0/x/block/sda"
            ),
            Some(Interface::Sata)
        );
        assert_eq!(
            interface_for(
                "sdc",
                "/sys/devices/pci0000:00/x/host5/target5:0:0/block/sdc"
            ),
            Some(Interface::Scsi)
        );
        assert_eq!(interface_for("mmcblk0", ""), Some(Interface::Mmc));
        assert_eq!(
            interface_for("xvda", "/sys/devices/vbd-51712/block/xvda"),
            None
        );
    }

    #[test]
    fn kinds_only_where_trustworthy() {
        assert_eq!(
            kind_for(Some(Interface::Sata), Some("0")),
            Some(DiskKind::Ssd)
        );
        assert_eq!(
            kind_for(Some(Interface::Sata), Some("1")),
            Some(DiskKind::Hdd)
        );
        assert_eq!(kind_for(Some(Interface::Usb), Some("1")), None);
        assert_eq!(kind_for(Some(Interface::Scsi), Some("x")), None);
    }
}
