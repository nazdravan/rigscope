//! Bounds-checked little-endian reads.

pub fn u8_at(b: &[u8], off: usize) -> Option<u8> {
    b.get(off).copied()
}

pub fn u16_at(b: &[u8], off: usize) -> Option<u16> {
    let s = b.get(off..off.checked_add(2)?)?;
    Some(u16::from_le_bytes([*s.first()?, *s.get(1)?]))
}

pub fn u32_at(b: &[u8], off: usize) -> Option<u32> {
    let s = b.get(off..off.checked_add(4)?)?;
    let mut a = [0u8; 4];
    a.copy_from_slice(s);
    Some(u32::from_le_bytes(a))
}

pub fn u64_at(b: &[u8], off: usize) -> Option<u64> {
    let s = b.get(off..off.checked_add(8)?)?;
    let mut a = [0u8; 8];
    a.copy_from_slice(s);
    Some(u64::from_le_bytes(a))
}
