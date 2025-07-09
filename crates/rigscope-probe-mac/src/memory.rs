use crate::parse::{parse_memory, Dimm};
use crate::MacSource;
use rigscope_model::{Field, MemoryArray, MemoryInfo, MemoryModule, MemorySlot, SlotState};

const NO_PROFILER: &str = "system_profiler gave no memory data";

/// The memory section, and whether `system_profiler` answered (a machine without slots still
/// answers, with no modules).
pub(crate) fn read_memory(src: &dyn MacSource) -> (MemoryInfo, bool) {
    let os_total_bytes = match src
        .sysctl("hw.memsize")
        .ok()
        .and_then(|s| s.trim().parse().ok())
    {
        Some(n) => Field::Known(n),
        None => Field::Unknown,
    };
    let profile = src
        .profiler("SPMemoryDataType")
        .ok()
        .and_then(|j| parse_memory(&j));
    let arrays = match profile {
        // No slots (Apple silicon: the memory is part of the chip): no array to show.
        Some(p) if p.dimms.is_empty() => Vec::new(),
        Some(p) => vec![MemoryArray {
            max_capacity_bytes: Field::Unknown,
            slot_count: Field::Known(u16::try_from(p.dimms.len()).unwrap_or(u16::MAX)),
            ecc: match p.ecc.as_deref() {
                Some("ecc_disabled") => Field::Known("None".into()),
                Some("ecc_enabled") => Field::Known("ECC".into()),
                _ => Field::Unknown,
            },
            slots: p.dimms.iter().map(slot).collect(),
        }],
        None => {
            return (
                MemoryInfo {
                    os_total_bytes,
                    arrays: vec![MemoryArray {
                        max_capacity_bytes: Field::Unavailable(NO_PROFILER.into()),
                        ..MemoryArray::default()
                    }],
                },
                false,
            )
        }
    };
    (
        MemoryInfo {
            os_total_bytes,
            arrays,
        },
        true,
    )
}

fn slot(d: &Dimm) -> MemorySlot {
    let module = if d.empty {
        SlotState::Empty
    } else {
        SlotState::Installed(Box::new(MemoryModule {
            size_bytes: Field::from_option(d.size_bytes),
            mem_type: Field::from_option(d.mem_type),
            form_factor: Field::from_option(d.form_factor),
            manufacturer: Field::from_option(d.manufacturer.clone()),
            part_number: Field::from_option(d.part_number.clone()),
            serial: Field::from_option(d.serial.clone()),
            // macOS reports only the speed the module runs at.
            rated_mts: Field::Unknown,
            configured_mts: Field::from_option(d.speed_mts),
            ranks: Field::Unknown,
            voltage_mv: Field::Unknown,
            data_width_bits: Field::Unknown,
            total_width_bits: Field::Unknown,
        }))
    };
    MemorySlot {
        locator: Field::from_option(d.locator.clone()),
        bank: Field::Unknown,
        module,
    }
}
