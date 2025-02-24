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
fn laptop_battery_from_energy_files() {
    let s = build_snapshot(&fixture("laptop-ddr5"), None, 0);
    assert_eq!(s.power.ac_online, Field::Known(false));
    let b = &s.power.batteries[0];
    assert_eq!(b.name, "BAT0");
    assert_eq!(b.manufacturer, Field::Known("SMP".into()));
    assert_eq!(b.model, Field::Known("5B10W51867".into()));
    assert_eq!(b.serial, Field::Known("412".into()));
    assert_eq!(b.technology, Field::Known("Li-poly".into()));
    assert_eq!(b.status, Field::Known("Discharging".into()));
    assert_eq!(b.charge_percent, Field::Known(76));
    assert_eq!(b.cycles, Field::Known(87));
    assert_eq!(b.design_mwh, Field::Known(57_000));
    assert_eq!(b.full_mwh, Field::Known(52_440));
    assert_eq!(b.now_mwh, Field::Known(39_854));
    assert_eq!(b.health_percent(), Some(92.0));
}

#[test]
fn desktop_has_no_battery() {
    let s = build_snapshot(&fixture("desktop-ddr4"), None, 0);
    assert!(s.power.batteries.is_empty());
    assert_eq!(s.power.ac_online, Field::Unknown);
}

#[test]
fn charge_files_are_converted_with_design_voltage() {
    let m = MemSource::default()
        .with("/sys/class/power_supply/BAT1/type", "Battery\n")
        .with(
            "/sys/class/power_supply/BAT1/charge_full_design",
            "4000000\n",
        )
        .with("/sys/class/power_supply/BAT1/charge_full", "3600000\n")
        .with(
            "/sys/class/power_supply/BAT1/voltage_min_design",
            "11400000\n",
        )
        .with("/sys/class/power_supply/BAT1/cycle_count", "0\n");
    let s = build_snapshot(&m, None, 0);
    let b = &s.power.batteries[0];
    assert_eq!(b.design_mwh, Field::Known(45_600));
    assert_eq!(b.full_mwh, Field::Known(41_040));
    assert_eq!(
        b.cycles,
        Field::Unknown,
        "many firmwares write 0 when they do not count cycles"
    );
}

#[test]
fn peripherals_with_batteries_are_skipped() {
    let m = MemSource::default()
        .with("/sys/class/power_supply/hidpp_battery_0/type", "Battery\n")
        .with("/sys/class/power_supply/hidpp_battery_0/scope", "Device\n");
    assert!(
        build_snapshot(&m, None, 0).power.batteries.is_empty(),
        "a wireless mouse is not the laptop battery"
    );
}
