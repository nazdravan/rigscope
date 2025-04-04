#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use rigscope_probe::*;

fn sample(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/../../fixtures/smart/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

#[test]
fn nvme_health_log() {
    let h = parse_smartctl(&sample("nvme-healthy")).unwrap();
    assert_eq!(h.passed, Some(true));
    assert_eq!(h.temperature_c, Some(39));
    assert_eq!(h.power_on_hours, Some(4321));
    assert_eq!(h.power_cycles, Some(512));
    assert_eq!(h.percent_used, Some(2));
    assert_eq!(h.available_spare, Some(100));
    assert_eq!(h.media_errors, Some(0));
    // NVMe data units are 1000 × 512 bytes.
    assert_eq!(h.written_bytes, Some(19_531_250 * 512_000));
    assert!(h.warnings().is_empty());
}

#[test]
fn ata_attributes() {
    let h = parse_smartctl(&sample("sata-hdd-reallocated")).unwrap();
    assert_eq!(h.passed, Some(true));
    assert_eq!(h.reallocated_sectors, Some(8));
    assert_eq!(h.pending_sectors, Some(0));
    assert_eq!(h.uncorrectable_sectors, Some(0));
    assert_eq!(h.power_on_hours, Some(32_170));
    assert_eq!(h.temperature_c, Some(37));
    assert_eq!(h.warnings(), vec!["reallocated sectors"]);
}

#[test]
fn device_open_failure_is_an_error_with_message() {
    assert_eq!(
        parse_smartctl(&sample("usb-open-failed")),
        Err(SmartError::DeviceError(
            "/dev/sdb: Unknown USB bridge [0x0781:0x5583 (0x100)]".into()
        ))
    );
}

#[test]
fn garbage_is_not_json() {
    assert_eq!(parse_smartctl(b"\xff\xfe"), Err(SmartError::NotJson));
    assert_eq!(parse_smartctl(b"[1,2]"), Err(SmartError::NotJson));
}
