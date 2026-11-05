#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use rigscope_model::*;
use rigscope_probe::*;
use std::path::PathBuf;

fn fixture(name: &str) -> FixtureSource {
    FixtureSource::new(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(name),
    )
}

#[test]
fn desktop_nvidia_card_skips_connectors() {
    let s = build_snapshot(&fixture("desktop-ddr4"), None, 0);
    assert_eq!(
        s.gpu.gpus.len(),
        1,
        "card0-HDMI-A-1 is an output, not a GPU"
    );
    let g = &s.gpu.gpus[0];
    assert_eq!(g.card, "card0");
    assert_eq!(g.pci_address, Field::Known("0000:01:00.0".into()));
    assert_eq!(g.vendor_id, Field::Known(0x10de));
    assert_eq!(g.device_id, Field::Known(0x2484));
    assert_eq!(g.vendor_name, Field::Known("NVIDIA Corporation".into()));
    assert_eq!(
        g.device_name,
        Field::Known("GA104 [GeForce RTX 3070]".into())
    );
    assert_eq!(g.subsystem_name, Field::Known("TUF RTX 3070 OC".into()));
    assert_eq!(g.driver, Field::Known("nvidia".into()));
    assert_eq!(g.pcie_link, Field::Known("16.0 GT/s PCIe x16".into()));
    assert!(g.boot_vga);
    assert!(
        matches!(g.vram_bytes, Field::Unavailable(_)),
        "proprietary nvidia driver does not expose VRAM in sysfs"
    );
}

#[test]
fn laptop_amd_igpu_reports_vram() {
    let s = build_snapshot(&fixture("laptop-ddr5"), None, 0);
    let g = &s.gpu.gpus[0];
    assert_eq!(g.card, "card1");
    assert_eq!(g.driver, Field::Known("amdgpu".into()));
    assert_eq!(g.vram_bytes, Field::Known(512 * 1024 * 1024));
    assert_eq!(g.subsystem_name, Field::Known("Radeon 780M".into()));
}

#[test]
fn vm_without_drm_has_no_gpus() {
    assert!(
        build_snapshot(&fixture("minimal-vm"), None, 0)
            .gpu
            .gpus
            .is_empty()
    );
}

#[test]
fn missing_pci_ids_explains_and_keeps_ids() {
    let mut m = MemSource::default()
        .with("/sys/class/drm/card0/device/vendor", "0x8086\n")
        .with("/sys/class/drm/card0/device/device", "0x46a6\n");
    m.links.insert(
        "/sys/class/drm/card0/device/driver".into(),
        "/sys/bus/pci/drivers/i915".into(),
    );
    let s = build_snapshot(&m, None, 0);
    let g = &s.gpu.gpus[0];
    assert_eq!(g.pci_id().as_deref(), Some("8086:46a6"));
    assert!(
        matches!(g.device_name, Field::Unavailable(_)),
        "pci.ids missing → say so"
    );
    assert_eq!(g.driver, Field::Known("i915".into()));
    assert!(
        matches!(g.vram_bytes, Field::Unavailable(_)),
        "integrated Intel uses system memory"
    );
}

#[test]
fn garbage_ids_do_not_panic() {
    let m = MemSource::default()
        .with("/sys/class/drm/card7/device/vendor", "nvidia?\n")
        .with("/sys/class/drm/card7/device/mem_info_vram_total", "lots\n");
    let s = build_snapshot(&m, None, 0);
    assert_eq!(s.gpu.gpus[0].vendor_id, Field::Unknown);
    assert_eq!(s.gpu.gpus[0].vram_bytes, Field::Unknown);
}

/// As on an ASUS X510UNR: Intel UHD 620 on the root bus, a board id pci.ids does not list.
#[test]
fn integrated_intel_names_the_board_maker_and_has_no_link() {
    let ids = "8086  Intel Corporation\n\t5917  UHD Graphics 620\n1043  ASUSTeK Computer Inc.\n";
    let mut m = MemSource::default()
        .with("/usr/share/hwdata/pci.ids", ids)
        .with("/sys/class/drm/card1/device/vendor", "0x8086\n")
        .with("/sys/class/drm/card1/device/device", "0x5917\n")
        .with("/sys/class/drm/card1/device/subsystem_vendor", "0x1043\n")
        .with("/sys/class/drm/card1/device/subsystem_device", "0x163e\n")
        .with(
            "/sys/class/drm/card1/device/current_link_speed",
            "Unknown\n",
        )
        .with("/sys/class/drm/card1/device/current_link_width", "0\n");
    m.links.insert(
        "/sys/class/drm/card1/device".into(),
        "/sys/devices/pci0000:00/0000:00:02.0".into(),
    );
    let s = build_snapshot(&m, None, 0);
    let g = &s.gpu.gpus[0];
    assert_eq!(g.pci_address, Field::Known("0000:00:02.0".into()));
    assert_eq!(
        g.subsystem_name,
        Field::Known("ASUSTeK Computer Inc. (1043:163e)".into())
    );
    assert_eq!(g.pcie_link, Field::Unavailable(NO_PCIE_LINK.into()));
}
