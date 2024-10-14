#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use proptest::prelude::*;
use rigscope_smbios::testutil::*;
use rigscope_smbios::*;

fn valid_table() -> Vec<u8> {
    table(&[
        memory_array(0x1000, 64 * GIB, 2),
        memory_device(
            0x1100,
            0x1000,
            &Dimm {
                size_mib: 8192,
                form_factor: 0x09,
                mem_type: 0x1A,
                speed: 3200,
                configured: 3200,
                locator: "DIMM A1",
                bank: "BANK 0",
                manufacturer: "Acme",
                serial: "1234",
                part: "PART",
                ranks: 2,
                voltage_mv: 1200,
            },
        ),
        StructBuilder::new(4, 4, 0x30)
            .string(0x04, "AM4")
            .u8(0x23, 8)
            .build(),
    ])
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn decode_never_panics(entry in proptest::collection::vec(any::<u8>(), 0..64),
                           table in proptest::collection::vec(any::<u8>(), 0..8192)) {
        let _ = decode(&entry, &table);
        let _ = structures(&table);
        let _ = parse_entry_point(&entry);
    }

    #[test]
    fn decode_mutated_valid_table_never_panics(edits in proptest::collection::vec((any::<usize>(), any::<u8>()), 1..32),
                                               cut in any::<usize>()) {
        let mut t = valid_table();
        let len = t.len();
        for (pos, val) in edits {
            t[pos % len] = val;
        }
        t.truncate(cut % (len + 1));
        let _ = decode(&entry_point_3(len as u32), &t);
    }
}
