//! Masking of raw data before it is written to a fixture (plain Rust, tested on every OS).

use crate::MemWin;

/// The host name a captured fixture carries.
pub const CAPTURED_NAME: &str = "CAPTURED-PC";

/// What `--capture-fixture` writes: `w` with every SMBIOS string the decoder does not read
/// (serials, asset tags, OEM strings) blanked and the UUID zeroed
/// (`rigscope_smbios::mask_serials`), the disk serials (WMI and inside the IOCTL descriptors),
/// the battery serials and the MACs masked, and the host name replaced by [`CAPTURED_NAME`].
/// Errors stay as they are.
pub fn mask_fixture(mut w: MemWin) -> MemWin {
    use rigscope_model::{mask, mask_mac};
    if let Ok(b) = w.smbios.as_mut() {
        if let Some(t) = b.get_mut(rigscope_smbios::RSMB_HEADER_LEN..) {
            rigscope_smbios::mask_serials(t);
        }
    }
    if let Ok(name) = w.computer_name.as_mut() {
        *name = CAPTURED_NAME.to_string();
    }
    if let Ok(v) = w.disks_wmi.as_mut() {
        for d in v.iter_mut() {
            d.serial = d.serial.as_deref().map(mask);
        }
    }
    if let Ok(v) = w.disks_ioctl.as_mut() {
        for d in v.iter_mut() {
            mask_descriptor_serial(&mut d.descriptor);
        }
    }
    if let Ok(v) = w.net.as_mut() {
        for n in v.iter_mut() {
            n.mac = n.mac.as_deref().map(mask_mac);
        }
    }
    if let Ok(v) = w.batteries.as_mut() {
        for b in v.iter_mut() {
            b.serial = b.serial.as_deref().map(mask);
        }
    }
    w
}

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

    #[test]
    fn mask_fixture_leaves_no_original_identifier() {
        let mut w = crate::samples::laptop();
        // The sample has no IOCTL list; give it one with a serial in the descriptor.
        w.disks_ioctl = Ok(vec![rigscope_win::DiskIoctlRaw {
            number: 0,
            descriptor: descriptor(36),
            length: Some(1 << 30),
        }]);
        let before = w.clone();
        let m = mask_fixture(w);

        assert_eq!(m.computer_name.as_deref(), Ok(CAPTURED_NAME));
        let disk = |w: &MemWin| w.disks_wmi.as_ref().unwrap()[0].serial.clone().unwrap();
        assert_ne!(disk(&m), disk(&before));
        assert!(!disk(&m).contains("S641NX0T"), "{}", disk(&m));
        assert_eq!(
            &m.disks_ioctl.as_ref().unwrap()[0].descriptor[36..41],
            b"*****"
        );
        let bat = |w: &MemWin| w.batteries.as_ref().unwrap()[0].serial.clone().unwrap();
        assert_ne!(bat(&m), bat(&before));
        let mac = |w: &MemWin| w.net.as_ref().unwrap()[0].mac.clone().unwrap();
        assert_ne!(mac(&m), mac(&before));
        assert!(mac(&m).starts_with("**:**:**:**:"), "{}", mac(&m));

        let smbios = m.smbios.as_ref().unwrap();
        assert!(!String::from_utf8_lossy(smbios).contains("00000001"));
        let decoded = rigscope_smbios::decode_rsmb(smbios).unwrap();
        assert_eq!(decoded.memory.arrays[0].slots.len(), 2, "still decodes");
        // Everything that is not an identifier is unchanged.
        assert_eq!(m.os_version, before.os_version);
        assert_eq!(m.dxgi, before.dxgi);
        assert_eq!(m.nvme, before.nvme);
    }
}
