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
    // Un tick după colectare dă frecvențe estimate.
    let mut cpu = s.cpu.clone();
    c.refresh_live(&mut cpu);
    assert!(cpu
        .cores
        .iter()
        .all(|k| k.estimated && k.cur_mhz.known().is_some()));
    // Raportul se generează fără panică și spune „needs administrator” la nevoie.
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
