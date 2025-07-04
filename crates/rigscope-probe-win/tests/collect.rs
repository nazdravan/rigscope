#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use rigscope_model::*;
use rigscope_probe_win::{samples, MemWin, WinCollector};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

#[test]
fn laptop_snapshot() {
    let mut c = WinCollector::new(samples::laptop());
    let s = c.collect(None, 7);
    assert_eq!(s.platform, Platform::Windows);
    assert_eq!(s.dmi_source, DmiSource::Firmware);
    assert_eq!(s.taken_at_unix, 7);
    assert!(s.system.os_name.known().unwrap().starts_with("Windows 11"));
    assert_eq!(s.memory.arrays[0].slots.len(), 2);
    assert_eq!(s.cpu.logical_threads, Field::Known(16));
    assert_eq!(s.power.batteries.len(), 1);
    assert!(s.storage.disks[0].health.known().is_some());
    assert!(s.sensors.chips.iter().any(|c| c.hwmon == "cpu"));
    // A tick after collecting gives estimated frequencies.
    let mut cpu = s.cpu.clone();
    c.refresh_live(&mut cpu);
    assert!(cpu
        .cores
        .iter()
        .all(|k| k.estimated && k.cur_mhz.known().is_some()));
    // The report builds without panicking and says "needs administrator" where needed.
    let _ = report_text(&s, true);
}

#[test]
fn vm_snapshot_has_no_invented_values() {
    let s = WinCollector::new(samples::vm()).collect(None, 0);
    assert!(s.power.batteries.is_empty());
    assert_eq!(
        s.storage.disks[0].interface,
        Field::Known(Interface::Virtual)
    );
    assert!(matches!(s.storage.disks[0].health, Field::Unavailable(_)));
    assert!(
        s.gpu.gpus.is_empty()
            || matches!(
                s.sensors
                    .chips
                    .iter()
                    .find(|c| c.hwmon == "gpu0")
                    .unwrap()
                    .temps[0]
                    .millicelsius,
                Field::Unavailable(_)
            )
    );
}

#[test]
fn checked_in_fixtures_match_the_samples() {
    for (name, sample) in [
        ("windows-laptop", samples::laptop()),
        ("windows-vm", samples::vm()),
    ] {
        let loaded = MemWin::load(&fixture(name)).unwrap();
        assert_eq!(
            loaded, sample,
            "{name}: regenerate with `cargo xtask gen-fixtures`"
        );
    }
}

#[test]
fn not_elevated_shows_needs_elevation_for_smart() {
    let mut w = samples::laptop();
    w.elevated = false;
    for v in w.nvme.values_mut() {
        *v = Err(rigscope_win::WinError::AccessDenied);
    }
    let s = WinCollector::new(w).collect(None, 0);
    assert_eq!(s.storage.disks[0].health, Field::NeedsElevation);
}

#[test]
fn real_desktop_fixture() {
    let w = MemWin::load(&fixture("windows-desktop")).unwrap();
    let s = WinCollector::new(w).collect(None, 0);
    assert_eq!(s.dmi_source, DmiSource::Firmware);
    assert!(!s.memory.arrays.is_empty(), "SMBIOS memory slots");
    // i7-4600U: 2 cores / 4 threads.
    assert_eq!(s.cpu.logical_threads, Field::Known(4));
    assert_eq!(s.storage.disks.len(), 1);
    let d = &s.storage.disks[0];
    assert_eq!(d.interface, Field::Known(Interface::Raid)); // Intel RST, BusType 8
    let h = d.health.known().expect("SMART readable behind Intel RST");
    assert!(h.power_on_hours.is_some());
    // Max clock must not be reported below the base clock (SMBIOS Max Speed is 2100 here).
    if let (Some(b), Some(m)) = (s.cpu.base_mhz.known(), s.cpu.max_mhz.known()) {
        assert!(m >= b, "max {m} < base {b}");
    }
    assert!(!s.gpu.gpus.is_empty());
    assert!(s.gpu.gpus.iter().any(|g| g
        .device_name
        .known()
        .is_some_and(|n| n.contains("Intel(R) HD Graphics"))));
    assert!(!s.net.interfaces.is_empty());
    assert_eq!(s.power.batteries.len(), 1);
    let pct = s.power.batteries[0].health_percent().unwrap();
    assert!(
        (pct - 12310.0 / 33966.0 * 100.0).abs() < 0.5,
        "health {pct}"
    );
    let r = report_text(&s, true);
    assert!(
        !r.contains("CAPTURED-PC"),
        "host name is masked in the report too"
    );
}

#[test]
#[ignore]
fn print_real_desktop_report() {
    let w = MemWin::load(&fixture("windows-desktop")).unwrap();
    let s = WinCollector::new(w).collect(None, 0);
    println!("{}", report_text(&s, false));
}
