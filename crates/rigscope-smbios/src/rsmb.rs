//! Windows `GetSystemFirmwareTable('RSMB')` buffers: an 8-byte `RawSMBIOSData` header
//! (`Used20CallingMethod`, major, minor, `DmiRevision`, `Length: u32`) followed by the table.

use crate::bytes::{u32_at, u8_at};
use crate::{decode_table, Decoded, SmbiosError, MAX_TABLE_LEN};

pub const RSMB_HEADER_LEN: usize = 8;

/// (major, minor, table) from a `GetSystemFirmwareTable('RSMB')` buffer.
pub fn split_rsmb(buf: &[u8]) -> Result<(u8, u8, &[u8]), SmbiosError> {
    let bad = SmbiosError::BadRsmb;
    let major = u8_at(buf, 1).ok_or(bad.clone())?;
    let minor = u8_at(buf, 2).ok_or(bad.clone())?;
    let len = usize::try_from(u32_at(buf, 4).ok_or(bad.clone())?).map_err(|_| bad.clone())?;
    if len > MAX_TABLE_LEN {
        return Err(bad);
    }
    let end = RSMB_HEADER_LEN.checked_add(len).ok_or(bad.clone())?;
    let table = buf.get(RSMB_HEADER_LEN..end).ok_or(bad)?;
    Ok((major, minor, table))
}

pub fn decode_rsmb(buf: &[u8]) -> Result<Decoded, SmbiosError> {
    let (_, _, table) = split_rsmb(buf)?;
    decode_table(table)
}

/// (structure type, offset of a string-index byte in the formatted area): serial numbers and
/// asset tags, overwritten with 'X'.
const SERIALS: &[(u8, usize)] = &[
    (1, 0x07),  // system serial
    (2, 0x07),  // baseboard serial
    (2, 0x08),  // baseboard asset tag
    (3, 0x07),  // chassis serial
    (3, 0x08),  // chassis asset tag
    (4, 0x20),  // processor serial
    (4, 0x21),  // processor asset tag
    (17, 0x18), // memory device serial
    (17, 0x19), // memory device asset tag
    (22, 0x07), // portable battery serial
    (39, 0x08), // power supply serial
];
/// (structure type, offset, length): binary fields zeroed. Type 1 UUID; type 22 SBDS serial.
const ZEROED: &[(u8, usize, usize)] = &[(1, 0x08, 16), (22, 0x10, 2)];

/// Overwrites serial numbers, asset tags, the system UUID and the battery's SBDS serial in
/// place (see [`SERIALS`] and [`ZEROED`]: types 1, 2, 3, 4, 17, 22, 39), keeping every length
/// so the table stays decodable. A field is touched only when it lies inside the structure's
/// formatted length. Malformed tables are masked as far as they parse; never panics.
pub fn mask_serials(table: &mut [u8]) {
    let mut pos = 0usize;
    while let (Some(kind), Some(len)) = (
        u8_at(table, pos),
        pos.checked_add(1).and_then(|i| u8_at(table, i)),
    ) {
        let len = usize::from(len);
        if len < 4 || kind == 127 {
            return;
        }
        let Some(strings_start) = pos.checked_add(len) else {
            return;
        };
        // End of the string set: the first two zero bytes at or after strings_start.
        let Some(end) = table
            .get(strings_start..)
            .and_then(|r| r.windows(2).position(|w| w == [0, 0]))
            .and_then(|p| strings_start.checked_add(p))
        else {
            return;
        };
        for (_, off, n) in ZEROED.iter().filter(|(k, _, _)| *k == kind) {
            if len < off + n {
                continue;
            }
            let from = pos + off;
            if let Some(u) = table.get_mut(from..from + n) {
                u.fill(0);
            }
        }
        for (_, off) in SERIALS.iter().filter(|(k, _)| *k == kind) {
            if *off >= len {
                continue;
            }
            let idx = table.get(pos + off).copied().unwrap_or(0);
            if idx == 0 {
                continue;
            }
            // Walk to string number `idx` (1-based) and overwrite its bytes with 'X'.
            let mut s = strings_start;
            for _ in 1..idx {
                while s < end && table.get(s).is_some_and(|b| *b != 0) {
                    s += 1;
                }
                s += 1;
            }
            while s < end && table.get(s).is_some_and(|b| *b != 0) {
                if let Some(b) = table.get_mut(s) {
                    *b = b'X';
                }
                s += 1;
            }
        }
        pos = end + 2;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]
    use super::*;
    use crate::testutil::{memory_array, memory_device, table, Dimm, StructBuilder};

    fn rsmb(major: u8, minor: u8, t: &[u8]) -> Vec<u8> {
        let mut b = vec![0, major, minor, 0];
        b.extend(u32::try_from(t.len()).unwrap().to_le_bytes());
        b.extend_from_slice(t);
        b
    }

    fn sample_table() -> Vec<u8> {
        let dimm = Dimm {
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
        };
        table(&[
            memory_array(0x10, 64 << 30, 2),
            memory_device(0x11, 0x10, &dimm),
        ])
    }

    #[test]
    fn rsmb_decodes_like_the_linux_table() {
        let t = sample_table();
        let d = decode_rsmb(&rsmb(3, 4, &t)).unwrap();
        assert_eq!(d, decode_table(&t).unwrap());
        assert_eq!(d.memory.arrays.len(), 1);
    }

    #[test]
    fn rsmb_header_is_checked() {
        assert_eq!(split_rsmb(&[0, 3, 4]), Err(SmbiosError::BadRsmb));
        let t = sample_table();
        let mut b = rsmb(3, 4, &t);
        // Length claims more than the buffer holds: error, not a truncated read.
        b[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(split_rsmb(&b), Err(SmbiosError::BadRsmb));
        let ok = rsmb(2, 8, &t);
        let (major, minor, body) = split_rsmb(&ok).unwrap();
        assert_eq!((major, minor, body.len()), (2, 8, t.len()));
    }

    #[test]
    fn mask_overwrites_serials_and_uuid_only() {
        let system = StructBuilder::new(1, 0x01, 0x1B)
            .string(0x04, "LENOVO")
            .string(0x07, "PF2ABCDE")
            .u64(0x08, 0x1122_3344_5566_7788)
            .u64(0x10, 0x99AA_BBCC_DDEE_FF00)
            .build();
        let board = StructBuilder::new(2, 0x02, 0x08)
            .string(0x04, "LENOVO")
            .string(0x07, "L1HF12345")
            .build();
        let mut t = table(&[system, board]);
        let before = t.len();
        mask_serials(&mut t);
        assert_eq!(t.len(), before);
        let s = String::from_utf8_lossy(&t);
        assert!(!s.contains("PF2ABCDE") && !s.contains("L1HF12345"), "{s}");
        assert!(s.contains("XXXXXXXX") && s.contains("XXXXXXXXX"), "{s}");
        assert!(s.contains("LENOVO"));
        assert_eq!(crate::structures(&t).unwrap().len(), 3);
        assert!(t[0x08..0x08 + 16].iter().all(|b| *b == 0));
    }

    #[test]
    fn mask_covers_the_portable_battery() {
        let battery = StructBuilder::new(22, 0x16, 0x1A)
            .string(0x04, "Front")
            .string(0x05, "SMP")
            .string(0x07, "BATSER77")
            .string(0x08, "5B10W13930")
            .u16(0x10, 0xBEEF)
            .build();
        let mut t = table(&[battery]);
        mask_serials(&mut t);
        let s = String::from_utf8_lossy(&t);
        assert!(!s.contains("BATSER77") && s.contains("XXXXXXXX"), "{s}");
        assert!(s.contains("Front") && s.contains("SMP") && s.contains("5B10W13930"));
        assert_eq!(&t[0x10..0x12], &[0, 0], "SBDS serial");
        assert_eq!(crate::structures(&t).unwrap().len(), 2);
        // Too short for the SBDS word: the bytes after the formatted area are not touched.
        let short = StructBuilder::new(22, 0x16, 0x10).string(0x07, "S").build();
        let mut t = table(&[short]);
        let strings = t[0x10..].to_vec();
        mask_serials(&mut t);
        assert_eq!(t[0x10], b'X');
        assert_eq!(&t[0x11..], &strings[1..]);
    }

    #[test]
    fn mask_covers_processor_and_asset_tags() {
        let cpu = StructBuilder::new(4, 0x04, 0x2A)
            .string(0x04, "CPU0")
            .string(0x07, "Intel")
            .string(0x20, "CPUSERIAL")
            .string(0x21, "CPUASSET")
            .string(0x22, "PARTNO")
            .build();
        let board = StructBuilder::new(2, 0x02, 0x09)
            .string(0x04, "LENOVO")
            .string(0x08, "BOARDTAG")
            .build();
        let psu = StructBuilder::new(39, 0x27, 0x16)
            .string(0x05, "PSU1")
            .string(0x08, "PSUSERIAL")
            .build();
        let mut t = table(&[cpu, board, psu]);
        mask_serials(&mut t);
        let s = String::from_utf8_lossy(&t);
        for gone in ["CPUSERIAL", "CPUASSET", "BOARDTAG", "PSUSERIAL"] {
            assert!(!s.contains(gone), "{gone}: {s}");
        }
        for kept in ["CPU0", "Intel", "PARTNO", "LENOVO", "PSU1"] {
            assert!(s.contains(kept), "{kept}: {s}");
        }
        assert_eq!(crate::structures(&t).unwrap().len(), 4);
    }

    #[test]
    fn mask_handles_dimm_serial_and_malformed_input() {
        let dimm = Dimm {
            size_mib: 8192,
            form_factor: 0x09,
            mem_type: 0x1A,
            speed: 3200,
            configured: 3200,
            locator: "DIMM A1",
            bank: "BANK 0",
            manufacturer: "Acme",
            serial: "SN123456",
            part: "PART",
            ranks: 2,
            voltage_mv: 1200,
        };
        let mut t = table(&[memory_device(0x11, 0x10, &dimm)]);
        mask_serials(&mut t);
        let s = String::from_utf8_lossy(&t);
        assert!(!s.contains("SN123456") && s.contains("XXXXXXXX"), "{s}");
        assert!(s.contains("DIMM A1") && s.contains("PART"));
        // Truncated / hostile tables: no panic.
        let mut bad = vec![1u8, 0xFF, 0, 0, 7, 7, 7];
        mask_serials(&mut bad);
        let mut bad = vec![17u8, 0x20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        mask_serials(&mut bad);
        mask_serials(&mut []);
    }
}
