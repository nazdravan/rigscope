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

fn chip<'a>(s: &'a Snapshot, name: &str) -> &'a SensorChip {
    s.sensors
        .chips
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("no chip {name}"))
}

#[test]
fn desktop_cpu_disk_and_board_sensors() {
    let s = build_snapshot(&fixture("desktop-ddr4"), None, 0);
    let cpu = chip(&s, "coretemp");
    assert_eq!(cpu.temps.len(), 5);
    assert_eq!(cpu.temps[0].label, "Package id 0");
    assert_eq!(cpu.temps[0].millicelsius, Field::Known(45_000));
    assert_eq!(cpu.temps[0].max_mc, Field::Known(80_000));
    assert_eq!(cpu.temps[0].crit_mc, Field::Known(100_000));
    assert_eq!(cpu.temps[4].label, "Core 3");

    assert_eq!(chip(&s, "nvme").temps[0].label, "Composite");

    let board = chip(&s, "nct6798");
    assert_eq!(board.temps[0].max_mc, Field::Unknown);
    let fans: Vec<(&str, Field<u32>)> = board
        .fans
        .iter()
        .map(|f| (f.label.as_str(), f.rpm.clone()))
        .collect();
    assert_eq!(
        fans,
        vec![("fan1", Field::Known(1180)), ("CPU_FAN", Field::Known(0))],
        "unlabelled 0 RPM header hidden"
    );
}

#[test]
fn laptop_unlabelled_temp_gets_its_index() {
    let s = build_snapshot(&fixture("laptop-ddr5"), None, 0);
    assert_eq!(chip(&s, "acpitz").temps[0].label, "temp1");
    assert_eq!(
        chip(&s, "k10temp").temps[0].millicelsius,
        Field::Known(52_125)
    );
}

#[test]
fn vm_has_no_sensors() {
    assert!(
        build_snapshot(&fixture("minimal-vm"), None, 0)
            .sensors
            .chips
            .is_empty()
    );
}

#[test]
fn refresh_sensors_updates_readings() {
    let path = "/sys/class/hwmon/hwmon0/temp1_input";
    let src = MemSource::default()
        .with("/sys/class/hwmon/hwmon0/name", "coretemp\n")
        .with(path, "40000\n");
    let mut s = build_snapshot(&src, None, 0);
    let src = src.with(path, "61500\n");
    refresh_sensors(&src, &mut s.sensors);
    assert_eq!(
        s.sensors.chips[0].temps[0].millicelsius,
        Field::Known(61_500)
    );
}

#[test]
fn absurd_readings_are_unknown() {
    let src = MemSource::default()
        .with("/sys/class/hwmon/hwmon9/name", "weird\n")
        .with("/sys/class/hwmon/hwmon9/temp1_input", "999999999\n")
        .with("/sys/class/hwmon/hwmon9/temp2_input", "hot\n")
        .with("/sys/class/hwmon/hwmon9/fan1_input", "-3\n");
    let s = build_snapshot(&src, None, 0);
    let c = &s.sensors.chips[0];
    assert_eq!(
        c.temps[0].millicelsius,
        Field::Unknown,
        "over 250 °C is a broken sensor"
    );
    assert_eq!(c.temps[1].millicelsius, Field::Unknown);
    assert!(c.fans.is_empty());
}
