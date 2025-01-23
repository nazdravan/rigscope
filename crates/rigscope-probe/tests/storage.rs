#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use rigscope_model::*;
use rigscope_probe::*;
use std::path::PathBuf;

const SECTOR: u64 = 512;

fn fixture(name: &str) -> FixtureSource {
    FixtureSource::new(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(name),
    )
}

fn disk<'a>(s: &'a Snapshot, name: &str) -> &'a Disk {
    s.storage
        .disks
        .iter()
        .find(|d| d.name == name)
        .unwrap_or_else(|| panic!("no disk {name}"))
}

#[test]
fn fixture_canonical_follows_symlinks_inside_root() {
    let src = fixture("desktop-ddr4");
    assert_eq!(
        src.canonical("/sys/block/sda").unwrap(),
        "/sys/devices/pci0000:00/0000:00:17.0/ata1/host0/target0:0:0/0:0:0:0/block/sda"
    );
    assert_eq!(src.canonical("/sys/block/nope"), Err(ReadError::NotFound));
    assert_eq!(src.canonical("/../../Cargo.toml"), Err(ReadError::NotFound));
}

#[test]
fn mem_source_canonical_uses_links() {
    let mut m = MemSource::default().with("/sys/block/sda/size", "1\n");
    m.links.insert(
        "/sys/block/sda".into(),
        "/sys/devices/x/usb1/block/sda".into(),
    );
    assert_eq!(
        m.canonical("/sys/block/sda").unwrap(),
        "/sys/devices/x/usb1/block/sda"
    );
    assert_eq!(
        m.canonical("/sys/block/sda/size").unwrap(),
        "/sys/block/sda/size"
    );
    assert_eq!(m.canonical("/nope"), Err(ReadError::NotFound));
}

#[test]
fn desktop_has_nvme_and_sata_hdd_and_skips_loop() {
    let s = build_snapshot(&fixture("desktop-ddr4"), None, 0);
    let names: Vec<&str> = s.storage.disks.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, vec!["nvme0n1", "sda"]);

    let n = disk(&s, "nvme0n1");
    assert_eq!(
        n.model,
        Field::Known("Samsung SSD 970 EVO Plus 500GB".into())
    );
    assert_eq!(n.serial, Field::Known("S4EVNX0N123456A".into()));
    assert_eq!(n.firmware, Field::Known("2B2QEXM7".into()));
    assert_eq!(n.size_bytes, Field::Known(976_773_168 * SECTOR));
    assert_eq!(n.kind, Field::Known(DiskKind::NvmeSsd));
    assert_eq!(n.interface, Field::Known(Interface::Nvme));
    assert!(!n.removable);

    let h = disk(&s, "sda");
    assert_eq!(h.model, Field::Known("WDC WD10EZEX-08W".into()));
    assert_eq!(
        h.vendor,
        Field::Unknown,
        "libata reports the vendor as 'ATA'"
    );
    assert_eq!(h.firmware, Field::Known("1A01".into()));
    assert_eq!(h.kind, Field::Known(DiskKind::Hdd));
    assert_eq!(h.interface, Field::Known(Interface::Sata));
    assert!(
        matches!(h.serial, Field::Unavailable(_)),
        "SATA serial needs root"
    );
}

#[test]
fn laptop_usb_stick_is_removable_with_unknown_kind() {
    let s = build_snapshot(&fixture("laptop-ddr5"), None, 0);
    let u = disk(&s, "sdb");
    assert_eq!(u.interface, Field::Known(Interface::Usb));
    assert_eq!(u.vendor, Field::Known("SanDisk".into()));
    assert_eq!(u.model, Field::Known("Ultra Fit".into()));
    assert!(u.removable);
    assert_eq!(
        u.kind,
        Field::Unknown,
        "USB bridges report 'rotational' unreliably"
    );
}

#[test]
fn vm_virtio_disk() {
    let s = build_snapshot(&fixture("minimal-vm"), None, 0);
    let v = disk(&s, "vda");
    assert_eq!(v.interface, Field::Known(Interface::Virtio));
    assert_eq!(v.size_bytes, Field::Known(20 * 1024 * 1024 * 1024));
    assert_eq!(v.model, Field::Unknown);
}

#[test]
fn garbage_sysfs_values_do_not_panic() {
    let mut m = MemSource::default()
        .with("/sys/block/sdz/size", "lots\n")
        .with("/sys/block/sdz/queue/rotational", "maybe\n")
        .with("/sys/block/sdz/device/model", vec![0xFF, 0xFE]);
    m.links.insert("/sys/block/sdz".into(), "/weird".into());
    let s = build_snapshot(&m, None, 0);
    let d = disk(&s, "sdz");
    assert_eq!(d.size_bytes, Field::Unknown);
    assert_eq!(d.kind, Field::Unknown);
}
