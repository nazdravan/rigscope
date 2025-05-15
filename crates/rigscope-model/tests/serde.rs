use rigscope_model::*;

#[test]
fn snapshot_json_roundtrip() {
    let mut s = Snapshot::default();
    s.memory.os_total_bytes = Field::Known(17_179_869_184);
    s.memory.arrays.push(MemoryArray {
        slot_count: Field::Known(2),
        slots: vec![MemorySlot {
            locator: Field::Known("DIMM A1".into()),
            bank: Field::NeedsElevation,
            module: SlotState::Empty,
        }],
        ..Default::default()
    });
    s.cpu.model = Field::Unavailable("no cpuinfo".into());
    let json = serde_json::to_string(&s).expect("ser");
    let back: Snapshot = serde_json::from_str(&json).expect("de");
    assert_eq!(back, s);
}

#[test]
fn snapshot_without_storage_still_loads() {
    let mut v = serde_json::to_value(Snapshot::default()).expect("ser");
    v.as_object_mut().expect("object").remove("storage");
    let s: Snapshot = serde_json::from_value(v).expect("old snapshot loads");
    assert!(s.storage.disks.is_empty());
}

#[test]
fn old_needs_root_and_missing_platform_still_load() {
    // A v0.2.0 snapshot: NeedsRoot, no "platform", no "estimated".
    let json = r#"{"schema":1,"taken_at_unix":0,
        "system":{"os_name":"NeedsRoot","kernel":"Unknown","hostname":"Unknown","uptime_s":"Unknown"},
        "board":{"vendor":"Unknown","product":"Unknown","version":"Unknown","bios_vendor":"Unknown","bios_version":"Unknown","bios_date":"Unknown"},
        "memory":{"os_total_bytes":"Unknown","arrays":[]},
        "cpu":{"model":"Unknown","vendor":"Unknown","socket":"NeedsRoot","physical_cores":"Unknown","logical_threads":"Unknown","base_mhz":"Unknown","max_mhz":"Unknown",
               "cores":[{"cpu":0,"cur_mhz":{"Known":800},"min_mhz":"Unknown","max_mhz":"Unknown","from_cpuinfo_fallback":false}],
               "caches":[],"microcode":"Unknown","flags":[]},
        "dmi_source":"SysfsIdOnly"}"#;
    let s: Snapshot = serde_json::from_str(json).unwrap();
    assert_eq!(s.system.os_name, Field::NeedsElevation);
    assert_eq!(s.cpu.socket, Field::NeedsElevation);
    assert_eq!(s.platform, Platform::Linux);
    assert!(!s.cpu.cores[0].estimated);
}

#[test]
fn needs_elevation_text_depends_on_platform() {
    let mut s = Snapshot::default();
    s.cpu.socket = Field::NeedsElevation;
    assert!(report_text(&s, true).contains("Socket: needs root"));
    s.platform = Platform::Windows;
    assert!(report_text(&s, true).contains("Socket: needs administrator"));
}

#[test]
fn new_interfaces_have_labels() {
    assert_eq!(Interface::Virtual.label(), "virtual");
    assert_eq!(Interface::Raid.label(), "RAID");
}
