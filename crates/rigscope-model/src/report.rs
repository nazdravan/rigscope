//! Plain-text report (English), for pasting into forums or chats.

use crate::{compat_summary, CacheKind, DmiSource, Field, SlotState, Snapshot};

/// Binary size: `8 GiB`, `7.7 GiB`, `512 KiB`.
pub fn format_bytes(n: u64) -> String {
    const UNITS: [(&str, u64); 4] = [
        ("TiB", 1 << 40),
        ("GiB", 1 << 30),
        ("MiB", 1 << 20),
        ("KiB", 1 << 10),
    ];
    for (unit, size) in UNITS {
        if n >= size {
            return if n % size == 0 {
                format!("{} {unit}", n / size)
            } else {
                format!("{:.1} {unit}", n as f64 / size as f64)
            };
        }
    }
    format!("{n} B")
}

/// Keeps the last 4 characters: `"ABCD1234"` → `"****1234"`; 4 or fewer → `"****"`.
pub fn mask(serial: &str) -> String {
    let chars: Vec<char> = serial.chars().collect();
    if chars.len() <= 4 {
        return "****".to_string();
    }
    let keep = chars.len() - 4;
    let tail: String = chars.iter().skip(keep).collect();
    format!("{}{tail}", "*".repeat(keep))
}

/// Copy of the snapshot with every serial number masked.
pub fn masked(s: &Snapshot) -> Snapshot {
    let mut out = s.clone();
    for slot in out
        .memory
        .arrays
        .iter_mut()
        .flat_map(|a| a.slots.iter_mut())
    {
        if let SlotState::Installed(m) = &mut slot.module {
            if let Field::Known(serial) = &m.serial {
                m.serial = Field::Known(mask(serial));
            }
        }
    }
    out
}

fn show<T>(f: &Field<T>, fmt: impl Fn(&T) -> String) -> String {
    match f {
        Field::Known(v) => fmt(v),
        Field::Unknown => "unknown".into(),
        Field::NeedsRoot => "needs root".into(),
        Field::Unavailable(reason) => reason.clone(),
    }
}

fn text(f: &Field<String>) -> String {
    show(f, Clone::clone)
}

pub fn report_text(s: &Snapshot, mask_serials: bool) -> String {
    let s = if mask_serials { masked(s) } else { s.clone() };
    let mut out: Vec<String> = vec![format!(
        "rigscope report (snapshot taken at unix time {})",
        s.taken_at_unix
    )];

    out.push("\n== System ==".into());
    out.push(format!("OS: {}", text(&s.system.os_name)));
    out.push(format!("Kernel: {}", text(&s.system.kernel)));
    out.push(format!("Hostname: {}", text(&s.system.hostname)));

    let b = &s.board;
    out.push("\n== Motherboard ==".into());
    out.push(format!(
        "Board: {} {} (version {})",
        text(&b.vendor),
        text(&b.product),
        text(&b.version)
    ));
    out.push(format!(
        "BIOS: {} {} ({})",
        text(&b.bios_vendor),
        text(&b.bios_version),
        text(&b.bios_date)
    ));

    out.push("\n== Memory ==".into());
    out.push(format!(
        "Installed (seen by the OS): {}",
        show(&s.memory.os_total_bytes, |v| format_bytes(*v))
    ));
    for a in &s.memory.arrays {
        out.push(format!(
            "Board maximum: {}, slots: {}, ECC: {}",
            show(&a.max_capacity_bytes, |v| format_bytes(*v)),
            show(&a.slot_count, |v| v.to_string()),
            text(&a.ecc)
        ));
    }
    if let Some(c) = compat_summary(&s.memory) {
        out.push(format!(
            "Compatible: {} (per-slot size is an estimate)",
            c.to_search_text()
        ));
    }
    for slot in s.memory.arrays.iter().flat_map(|a| a.slots.iter()) {
        let head = format!(
            "Slot {} ({})",
            text(&slot.locator),
            match &slot.bank {
                Field::Known(b) => b.clone(),
                _ => "unknown bank".to_string(),
            }
        );
        let body = match &slot.module {
            SlotState::Empty => "empty".to_string(),
            SlotState::Unknown => "unknown".to_string(),
            SlotState::Installed(m) => format!(
                "{} {} {}, rated {}, running at {}, {} {}, serial {}, ranks {}, {}",
                show(&m.size_bytes, |v| format_bytes(*v)),
                show(&m.mem_type, |t| t.label()),
                show(&m.form_factor, |f| f.label()),
                show(&m.rated_mts, |v| format!("{v} MT/s")),
                show(&m.configured_mts, |v| format!("{v} MT/s")),
                text(&m.manufacturer),
                text(&m.part_number),
                text(&m.serial),
                show(&m.ranks, |r| r.to_string()),
                show(&m.voltage_mv, |v| match v.configured.or(v.max).or(v.min) {
                    Some(mv) => format!("{:.2} V", f64::from(mv) / 1000.0),
                    None => "voltage unknown".into(),
                }),
            ),
        };
        out.push(format!("{head}: {body}"));
    }
    if s.memory.arrays.is_empty() {
        out.push(match s.dmi_source {
            DmiSource::SysfsIdOnly => {
                "Slot details: need root (run rigscope with elevated rights)".into()
            }
            _ => "Slot details: no DMI table".into(),
        });
    }

    let c = &s.cpu;
    out.push("\n== Processor ==".into());
    out.push(format!("Model: {}", text(&c.model)));
    out.push(format!("Vendor: {}", text(&c.vendor)));
    out.push(format!("Socket: {}", text(&c.socket)));
    out.push(format!(
        "Cores / threads: {} / {}",
        show(&c.physical_cores, |v| v.to_string()),
        show(&c.logical_threads, |v| v.to_string())
    ));
    out.push(format!(
        "Base clock: {}",
        show(&c.base_mhz, |v| format!("{v} MHz"))
    ));
    out.push(format!(
        "Max clock: {}",
        show(&c.max_mhz, |v| format!("{v} MHz"))
    ));
    out.push(format!("Microcode: {}", text(&c.microcode)));
    if !c.caches.is_empty() {
        let caches: Vec<String> = c
            .caches
            .iter()
            .map(|k| {
                let kind = match k.kind {
                    CacheKind::Data => "d",
                    CacheKind::Instruction => "i",
                    CacheKind::Unified => "",
                };
                format!(
                    "L{}{kind} {} × {}",
                    k.level,
                    k.instances,
                    format_bytes(k.size_bytes)
                )
            })
            .collect();
        out.push(format!("Cache: {}", caches.join(", ")));
    }
    if !c.flags.is_empty() {
        out.push(format!("Flags: {}", c.flags.join(" ")));
    }
    out.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    const GIB: u64 = 1 << 30;

    fn sample() -> Snapshot {
        let mut s = Snapshot::default();
        s.system.os_name = Field::Known("Ubuntu 24.04.1 LTS".into());
        s.memory.os_total_bytes = Field::Known(8 * GIB);
        s.memory.arrays.push(MemoryArray {
            max_capacity_bytes: Field::Known(64 * GIB),
            slot_count: Field::Known(2),
            ecc: Field::Known("None".into()),
            slots: vec![
                MemorySlot {
                    locator: Field::Known("DIMM A1".into()),
                    bank: Field::Known("BANK 0".into()),
                    module: SlotState::Installed(Box::new(MemoryModule {
                        size_bytes: Field::Known(8 * GIB),
                        mem_type: Field::Known(MemType::Ddr4),
                        form_factor: Field::Known(FormFactor::Dimm),
                        manufacturer: Field::Known("Kingston".into()),
                        part_number: Field::Known("KHX2666C16/8G".into()),
                        serial: Field::Known("4A1B2C3D".into()),
                        rated_mts: Field::Known(2666),
                        configured_mts: Field::Known(2400),
                        ..Default::default()
                    })),
                },
                MemorySlot {
                    locator: Field::Known("DIMM B1".into()),
                    module: SlotState::Empty,
                    ..Default::default()
                },
            ],
        });
        s.cpu.model = Field::Known("Intel(R) Core(TM) i5-7500 CPU @ 3.40GHz".into());
        s.cpu.socket = Field::NeedsRoot;
        s
    }

    #[test]
    fn bytes_formatting() {
        assert_eq!(format_bytes(8 * GIB), "8 GiB");
        assert_eq!(format_bytes(8_233_177_088), "7.7 GiB");
        assert_eq!(format_bytes(512 * 1024), "512 KiB");
        assert_eq!(format_bytes(12), "12 B");
    }

    #[test]
    fn mask_keeps_last_four() {
        assert_eq!(mask("ABCD1234"), "****1234");
        assert_eq!(mask("1234"), "****");
        assert_eq!(mask(""), "****");
        assert_eq!(mask("ăîșțâXYZW"), "*****XYZW");
    }

    #[test]
    fn report_contains_compat_and_slots() {
        let r = report_text(&sample(), false);
        assert!(r.contains("== Memory =="), "{r}");
        assert!(r.contains("Compatible: DDR4 DIMM 2666 MT/s 32GB"), "{r}");
        assert!(r.contains("DIMM A1"), "{r}");
        assert!(r.contains("KHX2666C16/8G"), "{r}");
        assert!(r.contains("DIMM B1 (unknown bank): empty"), "{r}");
        assert!(r.contains("Socket: needs root"), "{r}");
        assert!(r.contains("Ubuntu 24.04.1 LTS"), "{r}");
        assert!(r.contains("4A1B2C3D"));
    }

    #[test]
    fn report_masks_serials() {
        let r = report_text(&sample(), true);
        assert!(!r.contains("4A1B2C3D"), "{r}");
        assert!(r.contains("****2C3D"), "{r}");
    }

    #[test]
    fn masked_snapshot_hides_serials() {
        let m = masked(&sample());
        let SlotState::Installed(module) = &m.memory.arrays[0].slots[0].module else {
            panic!()
        };
        assert_eq!(module.serial, Field::Known("****2C3D".into()));
    }

    #[test]
    fn empty_snapshot_report() {
        let r = report_text(&Snapshot::default(), true);
        assert!(r.contains("== Processor =="));
    }
}
