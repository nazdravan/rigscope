//! Builder for synthetic SMBIOS structures and tables (tests and fixtures only).

/// Builds one structure: formatted area of a fixed length, then the string set.
pub struct StructBuilder {
    formatted: Vec<u8>,
    strings: Vec<String>,
}

impl StructBuilder {
    /// `len` is the formatted-area length including the 4-byte header.
    pub fn new(kind: u8, handle: u16, len: u8) -> Self {
        let mut formatted = vec![0u8; usize::from(len.max(4))];
        if let Some(b) = formatted.get_mut(0) {
            *b = kind;
        }
        if let Some(b) = formatted.get_mut(1) {
            *b = len;
        }
        let h = handle.to_le_bytes();
        if let Some(s) = formatted.get_mut(2..4) {
            s.copy_from_slice(&h);
        }
        StructBuilder {
            formatted,
            strings: Vec::new(),
        }
    }

    fn put(mut self, off: usize, bytes: &[u8]) -> Self {
        if let Some(s) = self.formatted.get_mut(off..off + bytes.len()) {
            s.copy_from_slice(bytes);
        }
        self
    }

    pub fn u8(self, off: usize, v: u8) -> Self {
        self.put(off, &[v])
    }
    pub fn u16(self, off: usize, v: u16) -> Self {
        self.put(off, &v.to_le_bytes())
    }
    pub fn u32(self, off: usize, v: u32) -> Self {
        self.put(off, &v.to_le_bytes())
    }
    pub fn u64(self, off: usize, v: u64) -> Self {
        self.put(off, &v.to_le_bytes())
    }

    /// Appends `s` to the string set and stores its 1-based index at `off`.
    pub fn string(mut self, off: usize, s: &str) -> Self {
        self.strings.push(s.to_string());
        let idx = u8::try_from(self.strings.len()).unwrap_or(0);
        self.u8(off, idx)
    }

    /// Overrides the length byte without resizing (to build malformed structures).
    pub fn raw_len(mut self, len: u8) -> Self {
        if let Some(b) = self.formatted.get_mut(1) {
            *b = len;
        }
        self
    }

    pub fn build(self) -> Vec<u8> {
        let mut out = self.formatted;
        if self.strings.is_empty() {
            out.extend_from_slice(&[0, 0]);
        } else {
            for s in &self.strings {
                out.extend_from_slice(s.as_bytes());
                out.push(0);
            }
            out.push(0);
        }
        out
    }
}

/// Concatenates structures and appends the end-of-table marker (type 127).
pub fn table(structs: &[Vec<u8>]) -> Vec<u8> {
    let mut out: Vec<u8> = structs.concat();
    out.extend(StructBuilder::new(127, 0xFFFF, 4).build());
    out
}

/// A 64-bit SMBIOS 3.x entry point (`_SM3_`) for version 3.3 and the given table length.
pub fn entry_point_3(table_len: u32) -> Vec<u8> {
    let mut e = vec![0u8; 0x18];
    if let Some(s) = e.get_mut(0..5) {
        s.copy_from_slice(b"_SM3_");
    }
    let set = |e: &mut Vec<u8>, off: usize, bytes: &[u8]| {
        if let Some(s) = e.get_mut(off..off + bytes.len()) {
            s.copy_from_slice(bytes);
        }
    };
    set(&mut e, 0x06, &[0x18, 3, 3, 0, 1]);
    set(&mut e, 0x0C, &table_len.to_le_bytes());
    let sum = e.iter().fold(0u8, |a, b| a.wrapping_add(*b));
    set(&mut e, 0x05, &[0u8.wrapping_sub(sum)]);
    e
}

pub const GIB: u64 = 1024 * 1024 * 1024;

/// Type 16: system memory array with `max_bytes` capacity and `slots` devices.
pub fn memory_array(handle: u16, max_bytes: u64, slots: u16) -> Vec<u8> {
    let kib = max_bytes / 1024;
    let b = StructBuilder::new(16, handle, 0x17)
        .u8(0x04, 0x03)
        .u8(0x05, 0x03)
        .u8(0x06, 0x03);
    let b = if kib >= 0x8000_0000 {
        b.u32(0x07, 0x8000_0000).u64(0x0F, max_bytes)
    } else {
        b.u32(0x07, u32::try_from(kib).unwrap_or(0))
    };
    b.u16(0x0B, 0xFFFE).u16(0x0D, slots).build()
}

/// Parameters of a type 17 device for [`memory_device`].
pub struct Dimm<'a> {
    pub size_mib: u16,
    pub form_factor: u8,
    pub mem_type: u8,
    pub speed: u16,
    pub configured: u16,
    pub locator: &'a str,
    pub bank: &'a str,
    pub manufacturer: &'a str,
    pub serial: &'a str,
    pub part: &'a str,
    pub ranks: u8,
    pub voltage_mv: u16,
}

/// Type 17 (SMBIOS 3.3 length 0x5C). `size_mib == 0` builds an empty slot.
pub fn memory_device(handle: u16, array: u16, d: &Dimm<'_>) -> Vec<u8> {
    StructBuilder::new(17, handle, 0x5C)
        .u16(0x04, array)
        .u16(0x06, 0xFFFE)
        .u16(0x08, if d.size_mib == 0 { 0xFFFF } else { 64 })
        .u16(0x0A, if d.size_mib == 0 { 0xFFFF } else { 64 })
        .u16(0x0C, d.size_mib)
        .u8(0x0E, d.form_factor)
        .string(0x10, d.locator)
        .string(0x11, d.bank)
        .u8(0x12, d.mem_type)
        .u16(0x15, d.speed)
        .string(0x17, d.manufacturer)
        .string(0x18, d.serial)
        .string(0x1A, d.part)
        .u8(0x1B, d.ranks)
        .u16(0x20, d.configured)
        .u16(0x22, d.voltage_mv)
        .u16(0x24, d.voltage_mv)
        .u16(0x26, d.voltage_mv)
        .build()
}
