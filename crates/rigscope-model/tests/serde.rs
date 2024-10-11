use rigscope_model::*;

#[test]
fn snapshot_json_roundtrip() {
    let mut s = Snapshot::default();
    s.memory.os_total_bytes = Field::Known(17_179_869_184);
    s.memory.arrays.push(MemoryArray {
        slot_count: Field::Known(2),
        slots: vec![MemorySlot {
            locator: Field::Known("DIMM A1".into()),
            bank: Field::NeedsRoot,
            module: SlotState::Empty,
        }],
        ..Default::default()
    });
    s.cpu.model = Field::Unavailable("no cpuinfo".into());
    let json = serde_json::to_string(&s).expect("ser");
    let back: Snapshot = serde_json::from_str(&json).expect("de");
    assert_eq!(back, s);
}
