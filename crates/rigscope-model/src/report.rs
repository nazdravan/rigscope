//! Plain-text report (English), for pasting into forums or chats.

use crate::{compat_summary, CacheKind, DmiSource, Field, Platform, SlotState, Snapshot};

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

/// Decimal size as disks are sold: `500.1 GB`, `2000 GB`.
pub fn format_decimal_gb(n: u64) -> String {
    let gb = n as f64 / 1e9;
    if (gb - gb.round()).abs() < 0.05 {
        format!("{} GB", gb.round() as u64)
    } else {
        format!("{gb:.1} GB")
    }
}

/// `465.8 GiB (500.1 GB)`: binary as the OS counts, decimal as on the box.
pub fn format_disk_size(n: u64) -> String {
    format!("{} ({})", format_bytes(n), format_decimal_gb(n))
}

/// Bytes written to a disk, decimal like drive specs: `10.0 TB`, `512 GB`.
pub fn format_written(n: u64) -> String {
    if n >= 1_000_000_000_000 {
        format!("{:.1} TB", n as f64 / 1e12)
    } else {
        format_decimal_gb(n)
    }
}

/// The SMART facts that are present, in a fixed order.
pub fn health_summary(h: &crate::DiskHealth) -> Vec<String> {
    let mut out = Vec::new();
    match h.passed {
        Some(true) => out.push("SMART passed".to_string()),
        Some(false) => out.push("SMART FAILED".to_string()),
        None => {}
    }
    if let Some(t) = h.temperature_c {
        out.push(format!("{t} °C"));
    }
    if let Some(v) = h.power_on_hours {
        out.push(format!("{v} h powered on"));
    }
    if let Some(v) = h.power_cycles {
        out.push(format!("{v} power cycles"));
    }
    if let Some(v) = h.percent_used {
        out.push(format!("{v} % endurance used"));
    }
    if let Some(v) = h.available_spare {
        out.push(format!("{v} % spare"));
    }
    if let Some(v) = h.written_bytes {
        out.push(format!("{} written", format_written(v)));
    }
    for (v, what) in [
        (h.reallocated_sectors, "reallocated sectors"),
        (h.pending_sectors, "pending sectors"),
        (h.uncorrectable_sectors, "uncorrectable sectors"),
        (h.media_errors, "media errors"),
    ] {
        if let Some(n) = v.filter(|n| *n > 0) {
            out.push(format!("{n} {what}"));
        }
    }
    out
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

/// Keeps the last two octets: `a8:a1:59:12:34:56` → `**:**:**:**:34:56`.
pub fn mask_mac(mac: &str) -> String {
    let parts: Vec<&str> = mac.split(':').collect();
    let keep = parts.len().saturating_sub(2);
    parts
        .iter()
        .enumerate()
        .map(|(i, p)| if i < keep { "**" } else { p })
        .collect::<Vec<_>>()
        .join(":")
}

/// Copy of the snapshot with every serial number masked.
pub fn masked(s: &Snapshot) -> Snapshot {
    let mut out = s.clone();
    if let Field::Known(host) = &out.system.hostname {
        out.system.hostname = Field::Known(mask(host));
    }
    for iface in &mut out.net.interfaces {
        if let Field::Known(mac) = &iface.mac {
            iface.mac = Field::Known(mask_mac(mac));
        }
    }
    for battery in &mut out.power.batteries {
        if let Field::Known(serial) = &battery.serial {
            battery.serial = Field::Known(mask(serial));
        }
    }
    for disk in &mut out.storage.disks {
        if let Field::Known(serial) = &disk.serial {
            disk.serial = Field::Known(mask(serial));
        }
    }
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

fn show<T>(p: Platform, f: &Field<T>, fmt: impl Fn(&T) -> String) -> String {
    match f {
        Field::Known(v) => fmt(v),
        Field::Unknown => "unknown".into(),
        Field::NeedsElevation => match p {
            Platform::Windows => "needs administrator".into(),
            Platform::Linux | Platform::Other => "needs root".into(),
        },
        Field::Unavailable(reason) => reason.clone(),
    }
}

fn text(p: Platform, f: &Field<String>) -> String {
    show(p, f, Clone::clone)
}

pub fn report_text(s: &Snapshot, mask_serials: bool) -> String {
    let s = if mask_serials { masked(s) } else { s.clone() };
    let p = s.platform;
    let mut out: Vec<String> = vec![format!(
        "rigscope report (snapshot taken at unix time {})",
        s.taken_at_unix
    )];

    out.push("\n== System ==".into());
    out.push(format!("OS: {}", text(p, &s.system.os_name)));
    out.push(format!("Kernel: {}", text(p, &s.system.kernel)));
    out.push(format!("Hostname: {}", text(p, &s.system.hostname)));

    let b = &s.board;
    out.push("\n== Motherboard ==".into());
    out.push(format!(
        "Board: {} {} (version {})",
        text(p, &b.vendor),
        text(p, &b.product),
        text(p, &b.version)
    ));
    out.push(format!(
        "BIOS: {} {} ({})",
        text(p, &b.bios_vendor),
        text(p, &b.bios_version),
        text(p, &b.bios_date)
    ));

    out.push("\n== Memory ==".into());
    out.push(format!(
        "Installed (seen by the OS): {}",
        show(p, &s.memory.os_total_bytes, |v| format_bytes(*v))
    ));
    for a in &s.memory.arrays {
        out.push(format!(
            "Board maximum: {}, slots: {}, ECC: {}",
            show(p, &a.max_capacity_bytes, |v| format_bytes(*v)),
            show(p, &a.slot_count, |v| v.to_string()),
            text(p, &a.ecc)
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
            text(p, &slot.locator),
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
                show(p, &m.size_bytes, |v| format_bytes(*v)),
                show(p, &m.mem_type, |t| t.label()),
                show(p, &m.form_factor, |f| f.label()),
                show(p, &m.rated_mts, |v| format!("{v} MT/s")),
                show(p, &m.configured_mts, |v| format!("{v} MT/s")),
                text(p, &m.manufacturer),
                text(p, &m.part_number),
                text(p, &m.serial),
                show(p, &m.ranks, |r| r.to_string()),
                show(p, &m.voltage_mv, |v| {
                    match v.configured.or(v.max).or(v.min) {
                        Some(mv) => format!("{:.2} V", f64::from(mv) / 1000.0),
                        None => "voltage unknown".into(),
                    }
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
            DmiSource::NoTable => "Slot details: this machine has no DMI table".into(),
            DmiSource::Firmware => "Slot details: the firmware table lists no memory slots".into(),
            _ => "Slot details: no DMI table".into(),
        });
    }

    let c = &s.cpu;
    out.push("\n== Storage ==".into());
    if s.storage.disks.is_empty() {
        out.push("No disks found".into());
    }
    for d in &s.storage.disks {
        out.push(format!(
            "{}: {}, {}, {}, {}, serial {}, firmware {}{}",
            d.name,
            text(p, &d.model),
            show(p, &d.size_bytes, |v| format_disk_size(*v)),
            show(p, &d.kind, |k| k.label().to_string()),
            show(p, &d.interface, |i| i.label().to_string()),
            text(p, &d.serial),
            text(p, &d.firmware),
            if d.removable { ", removable" } else { "" }
        ));
        match &d.health {
            Field::Known(h) => {
                out.push(format!("  health: {}", health_summary(h).join(", ")));
                for w in h.warnings() {
                    out.push(format!("  WARNING: {w}"));
                }
            }
            other => out.push(format!(
                "  health: {}",
                show(p, other, |_: &crate::DiskHealth| String::new())
            )),
        }
    }

    out.push("\n== Graphics ==".into());
    if s.gpu.gpus.is_empty() {
        out.push("No graphics cards found".into());
    }
    for g in &s.gpu.gpus {
        let name = [g.vendor_name.known(), g.device_name.known()]
            .into_iter()
            .flatten()
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        let name = if name.is_empty() {
            "unknown card".to_string()
        } else {
            name
        };
        let id = g.pci_id().map(|i| format!(" [{i}]")).unwrap_or_default();
        out.push(format!(
            "{}: {name}{id}, driver {}, VRAM {}{}",
            g.card,
            text(p, &g.driver),
            show(p, &g.vram_bytes, |v| format_bytes(*v)),
            if g.boot_vga { ", primary" } else { "" }
        ));
    }

    out.push("\n== Network ==".into());
    if s.net.interfaces.is_empty() {
        out.push("No physical network interfaces found".into());
    }
    for n in &s.net.interfaces {
        let mut parts = vec![show(p, &n.kind, |k| k.label().to_string())];
        if let Some(d) = n.device_name.known() {
            parts.push(d.clone());
        }
        if let Some(sp) = n.speed_mbps.known() {
            parts.push(crate::format_speed(*sp));
        }
        parts.push(format!("driver {}", text(p, &n.driver)));
        parts.push(format!("MAC {}", text(p, &n.mac)));
        out.push(format!("{}: {}", n.name, parts.join(", ")));
    }
    if s.net.hidden_virtual > 0 {
        out.push(format!(
            "({} virtual interfaces not listed)",
            s.net.hidden_virtual
        ));
    }

    out.push("\n== Battery ==".into());
    if let Field::Known(on) = s.power.ac_online {
        out.push(format!("On mains power: {}", if on { "yes" } else { "no" }));
    }
    if s.power.batteries.is_empty() {
        out.push("No battery".into());
    }
    for b in &s.power.batteries {
        let health = match (b.health_percent(), b.full_mwh.known(), b.design_mwh.known()) {
            (Some(h), Some(f), Some(d)) => {
                format!(
                    "health {h:.1} % ({:.1} of {}, derived)",
                    *f as f64 / 1000.0,
                    crate::format_mwh(*d)
                )
            }
            _ => "health unknown".into(),
        };
        out.push(format!(
            "{}: {} {} ({}), {}, {}, {health}, {} cycles, serial {}",
            b.name,
            text(p, &b.manufacturer),
            text(p, &b.model),
            text(p, &b.technology),
            show(p, &b.charge_percent, |c| format!("{c} %")),
            text(p, &b.status),
            show(p, &b.cycles, |c| c.to_string()),
            text(p, &b.serial),
        ));
    }

    out.push("\n== Sensors ==".into());
    if s.sensors.chips.is_empty() {
        out.push("No sensors found".into());
    }
    for chip in &s.sensors.chips {
        let mut parts: Vec<String> = chip
            .temps
            .iter()
            .map(|t| {
                let mut limits = Vec::new();
                if let Some(m) = t.max_mc.known() {
                    limits.push(format!("max {}", crate::format_celsius(*m)));
                }
                if let Some(c) = t.crit_mc.known() {
                    limits.push(format!("critical {}", crate::format_celsius(*c)));
                }
                let limits = if limits.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", limits.join(", "))
                };
                format!(
                    "{} {}{limits}",
                    t.label,
                    show(p, &t.millicelsius, |v| crate::format_celsius(*v))
                )
            })
            .collect();
        parts.extend(
            chip.fans
                .iter()
                .map(|f| format!("{} {}", f.label, show(p, &f.rpm, |r| format!("{r} RPM")))),
        );
        let kind = crate::chip_kind(&chip.name)
            .map(|k| format!(" ({k})"))
            .unwrap_or_default();
        out.push(format!("{}{kind}: {}", chip.name, parts.join("; ")));
    }

    out.push("\n== Processor ==".into());
    out.push(format!("Model: {}", text(p, &c.model)));
    out.push(format!("Vendor: {}", text(p, &c.vendor)));
    out.push(format!(
        "Socket: {}",
        crate::cpu::socket_label(text(p, &c.socket), &c.sockets)
    ));
    out.push(format!(
        "Cores / threads: {} / {}",
        show(p, &c.physical_cores, |v| v.to_string()),
        show(p, &c.logical_threads, |v| v.to_string())
    ));
    out.push(format!(
        "Base clock: {}",
        show(p, &c.base_mhz, |v| format!("{v} MHz"))
    ));
    out.push(format!(
        "Max clock: {}",
        show(p, &c.max_mhz, |v| format!("{v} MHz"))
    ));
    out.push(format!("Microcode: {}", text(p, &c.microcode)));
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
        s.cpu.socket = Field::NeedsElevation;
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

    fn with_disk() -> Snapshot {
        let mut s = sample();
        s.storage.disks.push(Disk {
            name: "nvme0n1".into(),
            model: Field::Known("Samsung SSD 970 EVO Plus 500GB".into()),
            serial: Field::Known("S4EVNX0N123456A".into()),
            size_bytes: Field::Known(500_107_862_016),
            kind: Field::Known(DiskKind::NvmeSsd),
            interface: Field::Known(Interface::Nvme),
            ..Default::default()
        });
        s
    }

    #[test]
    fn report_lists_disks_and_masks_their_serials() {
        let r = report_text(&with_disk(), true);
        assert!(r.contains("== Storage =="), "{r}");
        assert!(
            r.contains(
                "nvme0n1: Samsung SSD 970 EVO Plus 500GB, 465.8 GiB (500.1 GB), NVMe SSD, NVMe"
            ),
            "{r}"
        );
        assert!(r.contains("serial ***********456A"), "{r}");
        assert!(!r.contains("S4EVNX0N123456A"));
    }

    #[test]
    fn report_lists_gpus() {
        let mut s = Snapshot::default();
        s.gpu.gpus.push(Gpu {
            card: "card0".into(),
            vendor_id: Field::Known(0x10de),
            device_id: Field::Known(0x2484),
            vendor_name: Field::Known("NVIDIA Corporation".into()),
            device_name: Field::Known("GA104 [GeForce RTX 3070]".into()),
            driver: Field::Known("nvidia".into()),
            vram_bytes: Field::Unavailable("not exposed by the nvidia driver".into()),
            boot_vga: true,
            ..Default::default()
        });
        let r = report_text(&s, true);
        assert!(r.contains("== Graphics =="), "{r}");
        assert!(r.contains("card0: NVIDIA Corporation GA104 [GeForce RTX 3070] [10de:2484], driver nvidia, VRAM not exposed by the nvidia driver, primary"), "{r}");
    }

    #[test]
    fn report_lists_network_and_masks_mac() {
        let mut s = Snapshot::default();
        s.net.hidden_virtual = 2;
        s.net.interfaces.push(NetInterface {
            name: "enp4s0".into(),
            kind: Field::Known(NetKind::Ethernet),
            mac: Field::Known("a8:a1:59:12:34:56".into()),
            speed_mbps: Field::Known(1000),
            driver: Field::Known("igb".into()),
            device_name: Field::Known("I211 Gigabit Network Connection".into()),
            ..Default::default()
        });
        let r = report_text(&s, true);
        assert!(r.contains("== Network =="), "{r}");
        assert!(r.contains("enp4s0: Ethernet, I211 Gigabit Network Connection, 1 Gb/s, driver igb, MAC **:**:**:**:34:56"), "{r}");
        assert!(r.contains("(2 virtual interfaces not listed)"), "{r}");
        assert!(!report_text(&s, true).contains("a8:a1"));
        assert!(report_text(&s, false).contains("a8:a1:59:12:34:56"));
    }

    #[test]
    fn report_lists_battery_with_health() {
        let mut s = Snapshot::default();
        s.power.ac_online = Field::Known(false);
        s.power.batteries.push(Battery {
            name: "BAT0".into(),
            manufacturer: Field::Known("SMP".into()),
            model: Field::Known("5B10W51867".into()),
            serial: Field::Known("41234567".into()),
            technology: Field::Known("Li-poly".into()),
            status: Field::Known("Discharging".into()),
            charge_percent: Field::Known(76),
            design_mwh: Field::Known(57_000),
            full_mwh: Field::Known(52_440),
            cycles: Field::Known(87),
            ..Default::default()
        });
        let r = report_text(&s, true);
        assert!(r.contains("== Battery =="), "{r}");
        assert!(r.contains("On mains power: no"), "{r}");
        assert!(r.contains("BAT0: SMP 5B10W51867 (Li-poly), 76 %, Discharging, health 92.0 % (52.4 of 57.0 Wh, derived), 87 cycles, serial ****4567"), "{r}");
    }

    #[test]
    fn report_shows_socket_count() {
        let mut s = Snapshot::default();
        s.cpu.socket = Field::Known("LGA4677".into());
        s.cpu.sockets = Field::Known(2);
        assert!(report_text(&s, true).contains("Socket: 2 × LGA4677"));
        s.cpu.sockets = Field::Known(1);
        assert!(report_text(&s, true).contains("Socket: LGA4677\n"));
    }

    #[test]
    fn masked_report_hides_hostname() {
        let mut s = Snapshot::default();
        s.system.hostname = Field::Known("nicu-desktop".into());
        assert!(!report_text(&s, true).contains("nicu-desktop"));
        assert!(report_text(&s, false).contains("nicu-desktop"));
    }

    #[test]
    fn report_lists_sensors() {
        let mut s = Snapshot::default();
        s.sensors.chips.push(SensorChip {
            hwmon: "hwmon0".into(),
            name: "coretemp".into(),
            temps: vec![Temperature {
                index: 1,
                label: "Package id 0".into(),
                millicelsius: Field::Known(45_000),
                max_mc: Field::Known(80_000),
                crit_mc: Field::Known(100_000),
            }],
            fans: vec![Fan {
                index: 1,
                label: "fan1".into(),
                rpm: Field::Known(1180),
            }],
        });
        let r = report_text(&s, true);
        assert!(r.contains("== Sensors =="), "{r}");
        assert!(r.contains("coretemp (CPU (Intel)): Package id 0 45.0 °C (max 80.0 °C, critical 100.0 °C); fan1 1180 RPM"), "{r}");
    }

    #[test]
    fn report_shows_disk_health_and_warnings() {
        let mut s = with_disk();
        s.storage.disks[0].health = Field::Known(DiskHealth {
            passed: Some(true),
            temperature_c: Some(39),
            power_on_hours: Some(4321),
            power_cycles: Some(512),
            percent_used: Some(2),
            available_spare: Some(100),
            written_bytes: Some(10_000_000_000_000),
            reallocated_sectors: Some(8),
            ..Default::default()
        });
        let r = report_text(&s, true);
        assert!(r.contains("  health: SMART passed, 39 °C, 4321 h powered on, 512 power cycles, 2 % endurance used, 100 % spare, 10.0 TB written, 8 reallocated sectors"), "{r}");
        assert!(r.contains("  WARNING: reallocated sectors"), "{r}");
    }

    #[test]
    fn decimal_tb() {
        assert_eq!(format_written(10_000_000_000_000), "10.0 TB");
        assert_eq!(format_written(512_000_000_000), "512 GB");
    }

    #[test]
    fn decimal_gb() {
        assert_eq!(format_decimal_gb(500_107_862_016), "500.1 GB");
        assert_eq!(format_decimal_gb(2_000_000_000_000), "2000 GB");
        assert_eq!(format_decimal_gb(30_765_219_840), "30.8 GB");
    }

    #[test]
    fn empty_snapshot_report() {
        let r = report_text(&Snapshot::default(), true);
        assert!(r.contains("== Processor =="));
    }
}
