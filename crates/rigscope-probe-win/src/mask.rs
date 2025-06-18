//! Masking of raw data before it is written to a fixture (plain Rust, tested on every OS).

/// Overwrites the serial string inside a `STORAGE_DEVICE_DESCRIPTOR` with `*` (same length,
/// spaces kept). `SerialNumberOffset` is the `u32` at byte 24; 0 means there is no serial.
/// Never panics: a bad offset or a missing terminator stops at the end of the buffer.
pub fn mask_descriptor_serial(d: &mut [u8]) {
    let off = d
        .get(24..28)
        .and_then(|b| b.try_into().ok())
        .map(u32::from_le_bytes)
        .unwrap_or(0);
    let Ok(off) = usize::try_from(off) else {
        return;
    };
    if off == 0 {
        return;
    }
    let Some(serial) = d.get_mut(off..) else {
        return;
    };
    for b in serial.iter_mut().take_while(|b| **b != 0) {
        if *b != b' ' {
            *b = b'*';
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 48-byte descriptor: vendor "VND" at 28 (fake), serial "AB123" at 36.
    fn descriptor(serial_off: u32) -> Vec<u8> {
        let mut d = vec![0u8; 48];
        d[0] = 1;
        d[12..16].copy_from_slice(&28u32.to_le_bytes());
        d[24..28].copy_from_slice(&serial_off.to_le_bytes());
        d[28..31].copy_from_slice(b"VND");
        d[36..41].copy_from_slice(b"AB123");
        d
    }

    #[test]
    fn serial_becomes_stars_and_the_rest_stays() {
        let mut d = descriptor(36);
        let before = d.clone();
        mask_descriptor_serial(&mut d);
        assert_eq!(&d[36..41], b"*****");
        assert_eq!(d[41], 0, "the terminator stays");
        assert_eq!(&d[..36], &before[..36]);
        assert_eq!(&d[41..], &before[41..]);
    }

    #[test]
    fn no_serial_offset_or_a_bad_one_changes_nothing() {
        for off in [0, 1000, u32::MAX] {
            let mut d = descriptor(off);
            let before = d.clone();
            mask_descriptor_serial(&mut d);
            assert_eq!(d, before);
        }
        let mut short = vec![1u8; 10];
        mask_descriptor_serial(&mut short);
        assert_eq!(short, vec![1u8; 10]);
    }
}
