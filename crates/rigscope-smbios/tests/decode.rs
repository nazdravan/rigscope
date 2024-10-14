#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use rigscope_model::{Field, FormFactor, MemType, SlotState, VoltageMv};
use rigscope_smbios::testutil::*;
use rigscope_smbios::*;

fn ddr4(locator: &str, size_mib: u16, speed: u16, configured: u16) -> Dimm<'_> {
    Dimm {
        size_mib,
        form_factor: 0x09,
        mem_type: 0x1A,
        speed,
        configured,
        locator,
        bank: "BANK 0",
        manufacturer: "Kingston",
        serial: "1A2B3C4D",
        part: "KF432C16BB/8  ",
        ranks: 1,
        voltage_mv: 1200,
    }
}

fn decode_table(structs: &[Vec<u8>]) -> Decoded {
    let t = table(structs);
    decode(&entry_point_3(t.len() as u32), &t).expect("decode")
}

#[test]
fn parses_sm3_entry_point() {
    let e = parse_entry_point(&entry_point_3(1234)).expect("entry");
    assert_eq!(
        e,
        EntryPoint {
            major: 3,
            minor: 3,
            table_len: 1234
        }
    );
}

#[test]
fn parses_sm2_entry_point() {
    let mut e = vec![0u8; 0x1F];
    e[0..4].copy_from_slice(b"_SM_");
    e[0x05] = 0x1F;
    e[0x06] = 2;
    e[0x07] = 8;
    e[0x10..0x15].copy_from_slice(b"_DMI_");
    e[0x16..0x18].copy_from_slice(&900u16.to_le_bytes());
    assert_eq!(
        parse_entry_point(&e),
        Ok(EntryPoint {
            major: 2,
            minor: 8,
            table_len: 900
        })
    );
}

#[test]
fn rejects_bad_anchor() {
    assert_eq!(
        parse_entry_point(b"_XX3_aaaaaaaaaaaaaaaaaaaaaa"),
        Err(SmbiosError::BadEntryPoint)
    );
    assert_eq!(parse_entry_point(b""), Err(SmbiosError::BadEntryPoint));
}

#[test]
fn ddr4_two_slots_one_empty() {
    let d = decode_table(&[
        memory_array(0x1000, 64 * GIB, 2),
        memory_device(0x1100, 0x1000, &ddr4("DIMM A1", 8192, 3200, 2666)),
        memory_device(0x1101, 0x1000, &ddr4("DIMM B1", 0, 0, 0)),
    ]);
    assert_eq!(d.memory.arrays.len(), 1);
    let a = &d.memory.arrays[0];
    assert_eq!(a.max_capacity_bytes, Field::Known(64 * GIB));
    assert_eq!(a.slot_count, Field::Known(2));
    assert_eq!(a.ecc, Field::Known("None".into()));
    assert_eq!(a.slots.len(), 2);
    assert_eq!(a.slots[0].locator, Field::Known("DIMM A1".into()));
    assert_eq!(a.slots[0].bank, Field::Known("BANK 0".into()));
    let SlotState::Installed(m) = &a.slots[0].module else {
        panic!("slot 0 not installed")
    };
    assert_eq!(m.size_bytes, Field::Known(8 * GIB));
    assert_eq!(m.mem_type, Field::Known(MemType::Ddr4));
    assert_eq!(m.form_factor, Field::Known(FormFactor::Dimm));
    assert_eq!(m.manufacturer, Field::Known("Kingston".into()));
    assert_eq!(m.part_number, Field::Known("KF432C16BB/8".into()));
    assert_eq!(m.serial, Field::Known("1A2B3C4D".into()));
    assert_eq!(m.rated_mts, Field::Known(3200));
    assert_eq!(m.configured_mts, Field::Known(2666));
    assert_eq!(m.ranks, Field::Known(1));
    assert_eq!(
        m.voltage_mv,
        Field::Known(VoltageMv {
            min: Some(1200),
            max: Some(1200),
            configured: Some(1200)
        })
    );
    assert_eq!(m.data_width_bits, Field::Known(64));
    assert_eq!(a.slots[1].module, SlotState::Empty);
}

#[test]
fn extended_size_and_speed() {
    let mut dev = memory_device(0x1100, 0x1000, &ddr4("DIMM A1", 0x7FFF, 0xFFFF, 0xFFFF));
    // Extended size (MiB) at 0x1C, extended speed at 0x54, extended configured speed at 0x58.
    dev[0x1C..0x20].copy_from_slice(&65536u32.to_le_bytes());
    dev[0x54..0x58].copy_from_slice(&6400u32.to_le_bytes());
    dev[0x58..0x5C].copy_from_slice(&5600u32.to_le_bytes());
    let d = decode_table(&[memory_array(0x1000, 128 * GIB, 1), dev]);
    let SlotState::Installed(m) = &d.memory.arrays[0].slots[0].module else {
        panic!()
    };
    assert_eq!(m.size_bytes, Field::Known(64 * GIB));
    assert_eq!(m.rated_mts, Field::Known(6400));
    assert_eq!(m.configured_mts, Field::Known(5600));
}

#[test]
fn extended_max_capacity() {
    let d = decode_table(&[memory_array(0x1000, 4096 * GIB, 8)]);
    assert_eq!(
        d.memory.arrays[0].max_capacity_bytes,
        Field::Known(4096 * GIB)
    );
}

#[test]
fn size_kib_granularity() {
    let d = decode_table(&[
        memory_array(0x1000, GIB, 1),
        memory_device(0x1100, 0x1000, &ddr4("A", 0x8000 | 512, 1600, 1600)),
    ]);
    let SlotState::Installed(m) = &d.memory.arrays[0].slots[0].module else {
        panic!()
    };
    assert_eq!(m.size_bytes, Field::Known(512 * 1024));
}

#[test]
fn unknown_size_is_installed_with_unknown_size() {
    let d = decode_table(&[
        memory_array(0x1000, GIB, 1),
        memory_device(0x1100, 0x1000, &ddr4("A", 0xFFFF, 0, 0)),
    ]);
    let SlotState::Installed(m) = &d.memory.arrays[0].slots[0].module else {
        panic!()
    };
    assert_eq!(m.size_bytes, Field::Unknown);
    assert_eq!(m.rated_mts, Field::Unknown);
}

#[test]
fn ddr5_sodimm_codes() {
    let mut dimm = ddr4("A", 16384, 4800, 4800);
    dimm.mem_type = 0x22;
    dimm.form_factor = 0x0D;
    let d = decode_table(&[
        memory_array(0x1000, 64 * GIB, 1),
        memory_device(0x1100, 0x1000, &dimm),
    ]);
    let SlotState::Installed(m) = &d.memory.arrays[0].slots[0].module else {
        panic!()
    };
    assert_eq!(m.mem_type, Field::Known(MemType::Ddr5));
    assert_eq!(m.form_factor, Field::Known(FormFactor::SoDimm));
}

#[test]
fn placeholder_strings_are_unknown() {
    let mut dimm = ddr4("A", 8192, 3200, 3200);
    dimm.manufacturer = "To Be Filled By O.E.M.";
    dimm.serial = "";
    let d = decode_table(&[
        memory_array(0x1000, 64 * GIB, 1),
        memory_device(0x1100, 0x1000, &dimm),
    ]);
    let SlotState::Installed(m) = &d.memory.arrays[0].slots[0].module else {
        panic!()
    };
    assert_eq!(m.manufacturer, Field::Unknown);
    assert_eq!(m.serial, Field::Unknown);
}

#[test]
fn truncated_structure_keeps_rest() {
    // Valid framing, but a type 17 too short to hold its size field.
    let short = StructBuilder::new(17, 0x2000, 0x06)
        .u16(0x04, 0x1000)
        .build();
    let d = decode_table(&[
        memory_array(0x1000, 64 * GIB, 2),
        short,
        memory_device(0x1100, 0x1000, &ddr4("A", 8192, 3200, 3200)),
    ]);
    let slots = &d.memory.arrays[0].slots;
    assert_eq!(slots.len(), 2);
    assert_eq!(slots[0].module, SlotState::Unknown);
    assert!(matches!(slots[1].module, SlotState::Installed(_)));
}

#[test]
fn header_length_below_4_stops_decoding() {
    // Where the next structure starts is unknowable; keep what came before (like dmidecode).
    let bad = StructBuilder::new(17, 0x2000, 0x5C).raw_len(2).build();
    let d = decode_table(&[
        memory_array(0x1000, 64 * GIB, 1),
        bad,
        memory_device(0x1100, 0x1000, &ddr4("A", 8192, 3200, 3200)),
    ]);
    assert_eq!(d.memory.arrays.len(), 1);
    assert!(d.memory.arrays[0].slots.is_empty());
}

#[test]
fn bad_string_index_is_none() {
    let dev = StructBuilder::new(17, 0x1100, 0x5C)
        .u16(0x04, 0x1000)
        .u16(0x0C, 8192)
        .string(0x10, "DIMM A1")
        .u8(0x17, 9)
        .build();
    let d = decode_table(&[memory_array(0x1000, 64 * GIB, 1), dev]);
    let SlotState::Installed(m) = &d.memory.arrays[0].slots[0].module else {
        panic!()
    };
    assert_eq!(m.manufacturer, Field::Unknown);
    assert_eq!(
        d.memory.arrays[0].slots[0].locator,
        Field::Known("DIMM A1".into())
    );
}

#[test]
fn short_type17_from_old_smbios_has_unknown_new_fields() {
    // SMBIOS 2.3 type 17 is 0x1B bytes: no configured speed, no voltages.
    let dev = StructBuilder::new(17, 0x1100, 0x1B)
        .u16(0x04, 0x1000)
        .u16(0x0C, 4096)
        .u8(0x12, 0x18)
        .u16(0x15, 1333)
        .build();
    let d = decode_table(&[memory_array(0x1000, 16 * GIB, 1), dev]);
    let SlotState::Installed(m) = &d.memory.arrays[0].slots[0].module else {
        panic!()
    };
    assert_eq!(m.mem_type, Field::Known(MemType::Ddr3));
    assert_eq!(m.rated_mts, Field::Known(1333));
    assert_eq!(m.configured_mts, Field::Unknown);
    assert_eq!(m.voltage_mv, Field::Unknown);
    assert_eq!(m.ranks, Field::Unknown);
}

#[test]
fn non_system_memory_arrays_are_ignored() {
    let flash = StructBuilder::new(16, 0x0900, 0x17)
        .u8(0x05, 0x04)
        .u32(0x07, 1024)
        .u16(0x0D, 1)
        .build();
    let d = decode_table(&[flash, memory_array(0x1000, 8 * GIB, 1)]);
    assert_eq!(d.memory.arrays.len(), 1);
    assert_eq!(d.memory.arrays[0].max_capacity_bytes, Field::Known(8 * GIB));
}

#[test]
fn bios_board_and_processor() {
    let bios = StructBuilder::new(0, 0x0000, 0x18)
        .string(0x04, "American Megatrends Inc.")
        .string(0x05, "F12")
        .string(0x08, "03/15/2024")
        .build();
    let board = StructBuilder::new(2, 0x0002, 0x0F)
        .string(0x04, "Gigabyte Technology Co., Ltd.")
        .string(0x05, "B550 AORUS ELITE")
        .string(0x06, "x.x")
        .build();
    let cpu = StructBuilder::new(4, 0x0004, 0x30)
        .string(0x04, "AM4")
        .u16(0x14, 4650)
        .u8(0x18, 0x41)
        .u8(0x23, 8)
        .u8(0x25, 16)
        .build();
    let d = decode_table(&[bios, board, cpu]);
    assert_eq!(
        d.board.bios_vendor,
        Field::Known("American Megatrends Inc.".into())
    );
    assert_eq!(d.board.bios_version, Field::Known("F12".into()));
    assert_eq!(d.board.bios_date, Field::Known("03/15/2024".into()));
    assert_eq!(
        d.board.vendor,
        Field::Known("Gigabyte Technology Co., Ltd.".into())
    );
    assert_eq!(d.board.product, Field::Known("B550 AORUS ELITE".into()));
    assert_eq!(d.board.version, Field::Known("x.x".into()));
    assert_eq!(d.cpu_socket.as_deref(), Some("AM4"));
    assert_eq!(d.cpu_max_mhz, Some(4650));
    assert_eq!(d.cpu_cores, Some(8));
    assert_eq!(d.cpu_threads, Some(16));
}

#[test]
fn processor_core_count_2() {
    let cpu = StructBuilder::new(4, 0x0004, 0x32)
        .u8(0x18, 0x41)
        .u8(0x23, 0xFF)
        .u8(0x25, 0xFF)
        .u16(0x2A, 128)
        .u16(0x2E, 256)
        .build();
    let d = decode_table(&[cpu]);
    assert_eq!(d.cpu_cores, Some(128));
    assert_eq!(d.cpu_threads, Some(256));
}

#[test]
fn stops_at_end_of_table_type_127() {
    let mut t = table(&[memory_array(0x1000, 8 * GIB, 1)]);
    t.extend(memory_array(0x2000, 8 * GIB, 1));
    let d = decode(&entry_point_3(t.len() as u32), &t).expect("decode");
    assert_eq!(d.memory.arrays.len(), 1);
}

#[test]
fn table_too_large_is_error() {
    let t = vec![0u8; MAX_TABLE_LEN + 1];
    assert_eq!(
        decode(&entry_point_3(t.len() as u32), &t),
        Err(SmbiosError::TooLarge)
    );
}

#[test]
fn structures_lists_types_and_strings() {
    let t = table(&[StructBuilder::new(2, 7, 0x0F).string(0x04, "Acme").build()]);
    let s = structures(&t).expect("structures");
    assert_eq!(s.len(), 2);
    assert_eq!(s[0].kind, 2);
    assert_eq!(s[0].handle, 7);
    assert_eq!(s[0].strings, vec![b"Acme".as_slice()]);
    assert_eq!(s[1].kind, 127);
}
