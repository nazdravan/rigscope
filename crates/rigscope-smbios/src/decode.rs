use crate::bytes::{u8_at, u16_at, u32_at, u64_at};
use crate::table::{Structure, structures};
use crate::{SmbiosError, parse_entry_point};
use rigscope_model::{
    BoardInfo, Field, FormFactor, MemType, MemoryArray, MemoryInfo, MemoryModule, MemorySlot,
    SlotState, VoltageMv,
};

/// Everything rigscope takes from the SMBIOS table.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Decoded {
    pub memory: MemoryInfo,
    pub board: BoardInfo,
    pub cpu_socket: Option<String>,
    pub cpu_max_mhz: Option<u32>,
    pub cpu_cores: Option<u32>,
    pub cpu_threads: Option<u32>,
    /// Populated processor sockets.
    pub cpu_sockets: Option<u32>,
}

const KIND_BIOS: u8 = 0;
const KIND_BOARD: u8 = 2;
const KIND_PROCESSOR: u8 = 4;
const KIND_MEMORY_ARRAY: u8 = 16;
const KIND_MEMORY_DEVICE: u8 = 17;
/// Type 16 "use" value for system memory.
const USE_SYSTEM_MEMORY: u8 = 0x03;

/// Decodes the entry point and table. Malformed structures are skipped.
pub fn decode(entry: &[u8], table: &[u8]) -> Result<Decoded, SmbiosError> {
    let ep = parse_entry_point(entry)?;
    // The 2.x entry gives the exact length; 3.x gives a maximum. Never read past either.
    let limit = usize::try_from(ep.table_len).unwrap_or(usize::MAX);
    let table = if limit > 0 {
        table.get(..limit).unwrap_or(table)
    } else {
        table
    };
    decode_table(table)
}

/// Decodes a bare structure table (no entry point: Windows hands the table over directly).
pub fn decode_table(table: &[u8]) -> Result<Decoded, SmbiosError> {
    let structs = structures(table)?;

    let mut out = Decoded::default();
    let mut array_handles: Vec<u16> = Vec::new();
    let mut processor_seen = false;

    for s in &structs {
        match s.kind {
            KIND_BIOS => {
                out.board.bios_vendor = Field::from_option(s.string_at(0x04));
                out.board.bios_version = Field::from_option(s.string_at(0x05));
                out.board.bios_date = Field::from_option(s.string_at(0x08));
            }
            KIND_BOARD => {
                out.board.vendor = Field::from_option(s.string_at(0x04));
                out.board.product = Field::from_option(s.string_at(0x05));
                out.board.version = Field::from_option(s.string_at(0x06));
            }
            // Status byte 0x18 bit 6: socket populated. Skip empty sockets.
            KIND_PROCESSOR if u8_at(s.formatted, 0x18).is_some_and(|st| st & 0x40 != 0) => {
                decode_processor(s, &mut out, processor_seen);
                processor_seen = true;
            }
            KIND_MEMORY_ARRAY if u8_at(s.formatted, 0x05) == Some(USE_SYSTEM_MEMORY) => {
                array_handles.push(s.handle);
                out.memory.arrays.push(decode_array(s));
            }
            _ => {}
        }
    }

    for s in structs.iter().filter(|s| s.kind == KIND_MEMORY_DEVICE) {
        let Some(array_handle) = u16_at(s.formatted, 0x04) else {
            continue;
        };
        let Some(idx) = array_handles.iter().position(|h| *h == array_handle) else {
            continue;
        };
        if let Some(array) = out.memory.arrays.get_mut(idx) {
            array.slots.push(decode_device(s));
        }
    }
    Ok(out)
}

/// Populated sockets add up: cores and threads are summed, the socket name and max clock are
/// taken from the first one.
fn decode_processor(s: &Structure<'_>, out: &mut Decoded, more: bool) {
    let add = |acc: Option<u32>, v: Option<u32>| match (acc, v) {
        (Some(a), Some(b)) => Some(a.saturating_add(b)),
        (a, b) => a.or(b),
    };
    if !more {
        out.cpu_socket = s.string_at(0x04);
        out.cpu_max_mhz = u16_at(s.formatted, 0x14).filter(|v| *v != 0).map(u32::from);
    }
    out.cpu_cores = add(out.cpu_cores, count(s, 0x23, 0x2A));
    out.cpu_threads = add(out.cpu_threads, count(s, 0x25, 0x2E));
    out.cpu_sockets = Some(out.cpu_sockets.unwrap_or(0).saturating_add(1));
}

/// Core/thread count: byte field, or the 2-byte field when the byte is 0xFF. 0 = unknown.
fn count(s: &Structure<'_>, byte_off: usize, word_off: usize) -> Option<u32> {
    match u8_at(s.formatted, byte_off)? {
        0 => None,
        0xFF => u16_at(s.formatted, word_off)
            .filter(|v| *v != 0 && *v != 0xFFFF)
            .map(u32::from),
        n => Some(u32::from(n)),
    }
}

fn decode_array(s: &Structure<'_>) -> MemoryArray {
    let max = match u32_at(s.formatted, 0x07) {
        Some(0x8000_0000) => u64_at(s.formatted, 0x0F).filter(|v| *v != 0),
        Some(0) | None => None,
        Some(kib) => Some(u64::from(kib) * 1024),
    };
    let ecc = match u8_at(s.formatted, 0x06) {
        Some(0x03) => Some("None"),
        Some(0x04) => Some("Parity"),
        Some(0x05) => Some("Single-bit ECC"),
        Some(0x06) => Some("Multi-bit ECC"),
        Some(0x07) => Some("CRC"),
        _ => None,
    };
    MemoryArray {
        max_capacity_bytes: Field::from_option(max),
        slot_count: Field::from_option(u16_at(s.formatted, 0x0D)),
        ecc: Field::from_option(ecc.map(str::to_string)),
        slots: Vec::new(),
    }
}

fn decode_device(s: &Structure<'_>) -> MemorySlot {
    let f = s.formatted;
    let module = match u16_at(f, 0x0C) {
        None => SlotState::Unknown,
        Some(0) => SlotState::Empty,
        Some(raw) => SlotState::Installed(Box::new(decode_module(s, raw))),
    };
    MemorySlot {
        locator: Field::from_option(s.string_at(0x10)),
        bank: Field::from_option(s.string_at(0x11)),
        module,
    }
}

fn decode_module(s: &Structure<'_>, raw_size: u16) -> MemoryModule {
    let f = s.formatted;
    const MIB: u64 = 1024 * 1024;
    let size = match raw_size {
        0xFFFF => None,
        0x7FFF => u32_at(f, 0x1C)
            .map(|mib| u64::from(mib & 0x7FFF_FFFF) * MIB)
            .filter(|v| *v != 0),
        v if v & 0x8000 != 0 => Some(u64::from(v & 0x7FFF) * 1024),
        v => Some(u64::from(v) * MIB),
    };
    let width = |off| u16_at(f, off).filter(|w| *w != 0 && *w != 0xFFFF);
    let voltage = VoltageMv {
        min: u16_at(f, 0x22).filter(|v| *v != 0),
        max: u16_at(f, 0x24).filter(|v| *v != 0),
        configured: u16_at(f, 0x26).filter(|v| *v != 0),
    };
    let has_voltage =
        voltage.min.is_some() || voltage.max.is_some() || voltage.configured.is_some();
    MemoryModule {
        size_bytes: Field::from_option(size),
        mem_type: Field::from_option(u8_at(f, 0x12).and_then(mem_type)),
        form_factor: Field::from_option(u8_at(f, 0x0E).and_then(form_factor)),
        manufacturer: Field::from_option(s.string_at(0x17)),
        part_number: Field::from_option(s.string_at(0x1A)),
        serial: Field::from_option(s.string_at(0x18)),
        rated_mts: Field::from_option(speed(f, 0x15, 0x54)),
        configured_mts: Field::from_option(speed(f, 0x20, 0x58)),
        ranks: Field::from_option(u8_at(f, 0x1B).map(|a| a & 0x0F).filter(|r| *r != 0)),
        voltage_mv: if has_voltage {
            Field::Known(voltage)
        } else {
            Field::Unknown
        },
        data_width_bits: Field::from_option(width(0x0A)),
        total_width_bits: Field::from_option(width(0x08)),
    }
}

/// Speed in MT/s: 0 = unknown, 0xFFFF = use the 32-bit extended field.
fn speed(f: &[u8], off: usize, ext_off: usize) -> Option<u32> {
    match u16_at(f, off)? {
        0 => None,
        0xFFFF => u32_at(f, ext_off)
            .map(|v| v & 0x7FFF_FFFF)
            .filter(|v| *v != 0),
        v => Some(u32::from(v)),
    }
}

fn mem_type(code: u8) -> Option<MemType> {
    match code {
        0x00..=0x02 => None, // reserved / other / unknown
        0x18 => Some(MemType::Ddr3),
        0x1A => Some(MemType::Ddr4),
        0x1D => Some(MemType::Lpddr3),
        0x1E => Some(MemType::Lpddr4),
        0x22 => Some(MemType::Ddr5),
        0x23 => Some(MemType::Lpddr5),
        other => Some(MemType::Other(other)),
    }
}

fn form_factor(code: u8) -> Option<FormFactor> {
    match code {
        0x00..=0x02 => None,
        0x09 => Some(FormFactor::Dimm),
        0x0D => Some(FormFactor::SoDimm),
        other => Some(FormFactor::Other(other)),
    }
}
