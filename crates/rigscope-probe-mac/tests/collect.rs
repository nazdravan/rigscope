#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use rigscope_model::{
    Collector, DiskKind, DmiSource, Field, Interface, MemType, NetKind, Platform, SlotState,
};
use rigscope_probe_mac::{MacCollector, MemMac};
use std::path::PathBuf;

fn fixture(name: &str) -> MemMac {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name);
    MemMac::load(&dir).unwrap()
}

#[test]
fn imac_snapshot_shows_two_ddr4_modules() {
    let boot = 1_793_730_004; // kern.boottime of the fixture
    let s = MacCollector::new(fixture("mac-imac")).collect(None, boot + 3600);
    assert_eq!(s.platform, Platform::MacOs);
    assert_eq!(s.dmi_source, DmiSource::Firmware);
    assert_eq!(s.memory.os_total_bytes, Field::Known(16 << 30));
    let array = &s.memory.arrays[0];
    assert_eq!(array.slot_count, Field::Known(2));
    assert_eq!(array.ecc, Field::Known("None".into()));
    for slot in &array.slots {
        let SlotState::Installed(m) = &slot.module else {
            panic!("slot is not installed: {slot:?}");
        };
        assert_eq!(m.size_bytes, Field::Known(8 << 30));
        assert_eq!(m.mem_type, Field::Known(MemType::Ddr4));
        assert_eq!(m.configured_mts, Field::Known(2133));
        assert_eq!(m.rated_mts, Field::Unknown, "macOS reports one speed only");
    }
    assert_eq!(s.board.product, Field::Known("iMac18,2".into()));
    assert_eq!(s.board.vendor, Field::Known("Apple".into()));
    assert_eq!(
        s.cpu.model,
        Field::Known("Intel(R) Core(TM) i5-7400 CPU @ 3.00GHz".into())
    );
    assert_eq!(s.cpu.physical_cores, Field::Known(4));
    assert_eq!(s.cpu.base_mhz, Field::Known(3000));
    assert_eq!(s.system.hostname, Field::Known("CAPTURED-MAC".into()));
    assert_eq!(s.system.uptime_s, Field::Known(3600));
    assert_eq!(
        s.system.os_name,
        Field::Known("macOS 13.7.8 (Ventura)".into())
    );
}

#[test]
fn fixture_serials_are_masked() {
    let text = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/mac-imac/macsource.json"),
    )
    .unwrap();
    assert!(!text.contains("C02WH"));
    assert!(text.contains("\"serial_number\": \"XXXXXXXXXXXX\""));
}

#[test]
fn an_empty_source_degrades_to_unknown() {
    let s = MacCollector::new(MemMac::new()).collect(None, 1);
    assert_eq!(s.platform, Platform::MacOs);
    assert_eq!(s.dmi_source, DmiSource::None);
    assert_eq!(s.memory.os_total_bytes, Field::Unknown);
    assert_eq!(s.board.product, Field::Unknown);
    assert_eq!(s.cpu.model, Field::Unknown);
}

#[test]
fn apple_silicon_memory_has_no_slots_but_a_total() {
    let m = MemMac::new()
        .with_profiler(
            "SPMemoryDataType",
            r#"{"SPMemoryDataType":[{"SPMemoryDataType":"16 GB","dimm_manufacturer":"Apple","dimm_type":"LPDDR5"}]}"#,
        )
        .with_sysctl("hw.memsize", "17179869184");
    let s = MacCollector::new(m).collect(None, 1);
    assert!(s.memory.arrays.is_empty());
    assert_eq!(s.dmi_source, DmiSource::Firmware, "the profiler answered");
    assert_eq!(s.memory.os_total_bytes, Field::Known(16 << 30));
}

#[test]
fn an_unreadable_profiler_is_said_not_hidden() {
    let m = MemMac::new().with_sysctl("hw.memsize", "17179869184");
    let s = MacCollector::new(m).collect(None, 1);
    assert!(matches!(
        s.memory.arrays[0].max_capacity_bytes,
        Field::Unavailable(_)
    ));
}

#[test]
fn a_saved_fixture_loads_back() {
    let dir = std::env::temp_dir().join(format!("rigscope-mac-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let m = fixture("mac-imac");
    m.save(&dir).unwrap();
    assert!(MemMac::is_fixture(&dir));
    assert_eq!(MemMac::load(&dir).unwrap(), m);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_bad_fixture_is_an_error_not_a_panic() {
    let dir = std::env::temp_dir().join(format!("rigscope-mac-bad-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("macsource.json"), "not json").unwrap();
    assert!(MemMac::load(&dir).is_err());
    std::fs::write(
        dir.join("macsource.json"),
        r#"{"format":9,"profiler":{},"sysctl":{}}"#,
    )
    .unwrap();
    assert!(MemMac::load(&dir).unwrap_err().contains("unknown format"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_boot_time_in_the_future_gives_no_uptime() {
    let s = MacCollector::new(fixture("mac-imac")).collect(None, 1);
    assert_eq!(s.system.uptime_s, Field::Unknown);
}

#[test]
fn imac_storage_graphics_and_network() {
    let s = MacCollector::new(fixture("mac-imac")).collect(None, 1);

    assert_eq!(s.storage.disks.len(), 1);
    let d = &s.storage.disks[0];
    assert_eq!(d.name, "disk0");
    assert_eq!(d.model, Field::Known("TOSHIBA THNSNJ1T02CSX".into()));
    assert_eq!(d.kind, Field::Known(DiskKind::Ssd));
    assert_eq!(d.interface, Field::Known(Interface::Sata));
    assert_eq!(d.size_bytes, Field::Known(1_024_209_543_168));
    assert_eq!(
        d.serial,
        Field::Known("XXXXXXXXXXXX".into()),
        "masked in the fixture"
    );
    assert_eq!(d.health.known().and_then(|h| h.passed), Some(true));

    assert_eq!(s.gpu.gpus.len(), 1);
    let g = &s.gpu.gpus[0];
    assert_eq!(g.vendor_name, Field::Known("AMD".into()));
    assert_eq!(g.device_name, Field::Known("Radeon Pro 555".into()));
    assert_eq!(g.vendor_id, Field::Known(0x1002));
    assert_eq!(g.device_id, Field::Known(0x67ef));
    assert_eq!(g.vram_bytes, Field::Known(2 << 30));
    assert_eq!(g.pcie_link, Field::Known("PCIe x8".into()));
    assert!(g.boot_vga);

    let names: Vec<&str> = s.net.interfaces.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(
        names,
        ["en0", "en1"],
        "bridge0 and the USB modem en4 are hidden"
    );
    assert_eq!(s.net.hidden_virtual, 2);
    let eth = &s.net.interfaces[0];
    assert_eq!(eth.kind, Field::Known(NetKind::Ethernet));
    assert_eq!(eth.vendor_name, Field::Known("Broadcom".into()));
    assert_eq!(eth.device_name, Field::Known("57766-A1".into()));
    assert_eq!(s.net.interfaces[1].kind, Field::Known(NetKind::WiFi));

    assert!(
        s.power.batteries.is_empty(),
        "battery is not read on macOS yet"
    );
}

#[test]
fn an_empty_source_has_no_disks_cards_or_adapters() {
    let s = MacCollector::new(MemMac::new()).collect(None, 1);
    assert!(s.storage.disks.is_empty());
    assert!(s.gpu.gpus.is_empty());
    assert!(s.net.interfaces.is_empty());
}
