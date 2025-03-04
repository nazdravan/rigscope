#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use rigscope_model::*;
use rigscope_probe::*;
use std::path::PathBuf;

const GIB: u64 = 1024 * 1024 * 1024;

fn fixture(name: &str) -> FixtureSource {
    FixtureSource::new(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(name),
    )
}

#[test]
fn fixture_source_reads_and_lists() {
    let src = fixture("desktop-ddr4");
    assert!(String::from_utf8(src.read("/proc/cpuinfo").unwrap())
        .unwrap()
        .contains("i5-7500"));
    assert_eq!(src.read("/nope"), Err(ReadError::NotFound));
    let cache = src.list("/sys/devices/system/cpu/cpu0/cache").unwrap();
    assert_eq!(cache, vec!["index0", "index1", "index2", "index3"]);
}

#[test]
fn fixture_source_rejects_path_escape() {
    let src = fixture("desktop-ddr4");
    assert_eq!(src.read("/../../Cargo.toml"), Err(ReadError::NotFound));
}

#[test]
fn probe_on_minimal_vm_fixture() {
    let s = build_snapshot(&fixture("minimal-vm"), None, 42);
    assert_eq!(s.taken_at_unix, 42);
    assert_eq!(s.dmi_source, DmiSource::None);
    assert!(s.memory.arrays.is_empty());
    assert_eq!(s.memory.os_total_bytes, Field::Known(4_028_264 * 1024));
    assert_eq!(
        s.system.os_name,
        Field::Known("Debian GNU/Linux 12 (bookworm)".into())
    );
    assert_eq!(s.system.kernel, Field::Known("6.1.0-25-amd64".into()));
    assert_eq!(s.system.hostname, Field::Known("vm-test".into()));
    assert_eq!(s.system.uptime_s, Field::Known(3600));
    assert_eq!(
        s.cpu.model,
        Field::Known("Intel Xeon Processor (Skylake, IBRS)".into())
    );
    assert_eq!(s.cpu.logical_threads, Field::Known(2));
    assert_eq!(s.cpu.cores.len(), 2);
    assert!(s.cpu.cores.iter().all(|c| c.from_cpuinfo_fallback));
    assert_eq!(s.cpu.cores[0].cur_mhz, Field::Known(2095));
    assert!(matches!(s.board.vendor, Field::Unavailable(_)));
    assert!(matches!(s.cpu.socket, Field::Unavailable(_)));
}

#[test]
fn probe_desktop_ddr4() {
    let s = build_snapshot(&fixture("desktop-ddr4"), None, 0);
    assert_eq!(s.dmi_source, DmiSource::FullTable);
    let a = &s.memory.arrays[0];
    assert_eq!(a.slots.len(), 2);
    assert_eq!(a.slots[1].module, SlotState::Empty);
    let c = compat_summary(&s.memory).unwrap();
    assert_eq!(c.max_mts, Some(2666));
    assert_eq!(c.max_per_slot_bytes_estimated, Some(32 * GIB));
    assert_eq!(c.free_slots, Some(1));

    assert_eq!(
        s.cpu.model,
        Field::Known("Intel(R) Core(TM) i5-7500 CPU @ 3.40GHz".into())
    );
    assert_eq!(s.cpu.vendor, Field::Known("GenuineIntel".into()));
    assert_eq!(s.cpu.socket, Field::Known("LGA1151".into()));
    assert_eq!(s.cpu.physical_cores, Field::Known(4));
    assert_eq!(s.cpu.logical_threads, Field::Known(4));
    assert_eq!(s.cpu.base_mhz, Field::Known(3400));
    assert_eq!(s.cpu.max_mhz, Field::Known(3800));
    assert_eq!(s.cpu.microcode, Field::Known("0xf4".into()));
    assert!(s.cpu.flags.contains(&"avx2".to_string()));
    assert_eq!(s.cpu.cores.len(), 4);
    assert_eq!(s.cpu.cores[1].cur_mhz, Field::Known(3100));
    assert_eq!(s.cpu.cores[1].min_mhz, Field::Known(800));
    assert!(!s.cpu.cores[1].from_cpuinfo_fallback);

    let l3 = s.cpu.caches.iter().find(|c| c.level == 3).unwrap();
    assert_eq!((l3.size_bytes, l3.instances), (6144 * 1024, 1));
    let l1d = s
        .cpu
        .caches
        .iter()
        .find(|c| c.level == 1 && c.kind == CacheKind::Data)
        .unwrap();
    assert_eq!((l1d.size_bytes, l1d.instances), (32 * 1024, 4));
    assert_eq!(s.cpu.caches.len(), 4);

    assert_eq!(
        s.board.vendor,
        Field::Known("Gigabyte Technology Co., Ltd.".into())
    );
    assert_eq!(s.board.bios_version, Field::Known("F10".into()));
}

fn denied_source() -> MemSource {
    let mut m = MemSource::default()
        .with(paths::CPUINFO, "processor\t: 0\nmodel name\t: Test CPU\n\n")
        .with("/sys/class/dmi/id/board_vendor", "Acme\n")
        .with(paths::MEMINFO, "MemTotal: 1024 kB\n");
    m.denied = vec![paths::DMI_ENTRY.to_string(), paths::DMI_TABLE.to_string()];
    m
}

#[test]
fn probe_permission_denied_dmi_is_needs_root() {
    let s = build_snapshot(&denied_source(), None, 0);
    assert_eq!(s.dmi_source, DmiSource::SysfsIdOnly);
    assert!(s.memory.arrays.is_empty());
    assert_eq!(s.cpu.socket, Field::NeedsRoot);
    assert_eq!(s.board.vendor, Field::Known("Acme".into()));
    assert_eq!(s.board.bios_version, Field::Unknown);
}

#[test]
fn explicit_dump_overrides_denied_table() {
    let (entry, table) = rigscope_smbios::testutil::fixture_tables("desktop-ddr4").unwrap();
    let s = build_snapshot(&denied_source(), Some(&DmiDump { entry, table }), 0);
    assert_eq!(s.dmi_source, DmiSource::FullTable);
    assert_eq!(s.memory.arrays[0].slots.len(), 2);
    assert_eq!(s.cpu.socket, Field::Known("LGA1151".into()));
}

#[test]
fn garbage_dmi_table_degrades_to_sysfs_only() {
    let m = MemSource::default()
        .with(paths::DMI_ENTRY, b"garbage".to_vec())
        .with(paths::DMI_TABLE, vec![0xFF; 64])
        .with("/sys/class/dmi/id/board_vendor", "Acme\n");
    let s = build_snapshot(&m, None, 0);
    assert_eq!(s.dmi_source, DmiSource::SysfsIdOnly);
    assert!(s.memory.arrays.is_empty());
}

#[test]
fn empty_source_does_not_panic() {
    let s = build_snapshot(&MemSource::default(), None, 0);
    assert_eq!(s.dmi_source, DmiSource::None);
    assert_eq!(s.cpu.model, Field::Unknown);
    assert!(s.cpu.cores.is_empty());
}

#[test]
fn refresh_live_updates_frequency() {
    let path = "/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq";
    let src = MemSource::default()
        .with(paths::CPUINFO, "processor\t: 0\n\n")
        .with(paths::CPU_PRESENT, "0\n")
        .with(path, "1000000\n");
    let mut s = build_snapshot(&src, None, 0);
    assert_eq!(s.cpu.cores[0].cur_mhz, Field::Known(1000));
    let src = src.with(path, "2500000\n");
    refresh_live(&src, &mut s.cpu);
    assert_eq!(s.cpu.cores[0].cur_mhz, Field::Known(2500));
}

#[test]
fn dual_socket_from_cpuinfo() {
    let mut cpuinfo = String::new();
    for cpu in 0..4 {
        cpuinfo += &format!(
            "processor\t: {cpu}\nmodel name\t: Xeon\nphysical id\t: {}\ncore id\t: {}\n\n",
            cpu / 2,
            cpu % 2
        );
    }
    let s = build_snapshot(&MemSource::default().with(paths::CPUINFO, cpuinfo), None, 0);
    assert_eq!(s.cpu.sockets, Field::Known(2));
    assert_eq!(s.cpu.physical_cores, Field::Known(4));
}

#[test]
fn dmi_dump_roundtrip() {
    let d = DmiDump {
        entry: vec![1, 2, 3],
        table: vec![9; 100],
    };
    assert_eq!(decode_dmi_dump(&encode_dmi_dump(&d)), Ok(d));
}

#[test]
fn decode_dmi_dump_rejects_oversize_and_garbage() {
    let mut b = b"RSDMI1".to_vec();
    b.extend(((MAX_DMI_PART + 1) as u32).to_le_bytes());
    assert_eq!(decode_dmi_dump(&b), Err(ProbeError::DumpTooLarge));
    assert_eq!(decode_dmi_dump(b"hello"), Err(ProbeError::BadDump));
    let mut short = encode_dmi_dump(&DmiDump {
        entry: vec![1; 10],
        table: vec![2; 10],
    });
    short.pop();
    assert_eq!(decode_dmi_dump(&short), Err(ProbeError::BadDump));
}

#[test]
fn fixtures_are_up_to_date() {
    for name in rigscope_smbios::testutil::DMI_FIXTURES {
        let (entry, table) = rigscope_smbios::testutil::fixture_tables(name).unwrap();
        let src = fixture(name);
        assert_eq!(
            src.read(paths::DMI_ENTRY).unwrap(),
            entry,
            "{name}: run `cargo xtask gen-fixtures`"
        );
        assert_eq!(
            src.read(paths::DMI_TABLE).unwrap(),
            table,
            "{name}: run `cargo xtask gen-fixtures`"
        );
    }
}

/// Smoke test against the real `/proc` and `/sys` (Docker in `cargo xtask linux`, CI).
#[cfg(target_os = "linux")]
#[test]
fn fs_source_smoke_on_real_linux() {
    let s = build_snapshot(&FsSource, None, 0);
    assert!(s.cpu.logical_threads.known().is_some_and(|n| *n > 0));
    assert!(!s.cpu.cores.is_empty());
    assert!(s.memory.os_total_bytes.known().is_some());
    assert!(s.system.kernel.known().is_some());
}

#[test]
fn hybrid_cpu_caches_keep_each_size() {
    // P-cores 0-1 (own 1.25 MiB L2 each), E-cores 2-5 (one shared 2 MiB L2 cluster).
    let mut m = MemSource::default().with(paths::CPU_PRESENT, "0-5\n");
    for cpu in 0..6u32 {
        let (size, shared) = if cpu < 2 {
            ("1280K", cpu.to_string())
        } else {
            ("2048K", "2-5".to_string())
        };
        let d = format!("/sys/devices/system/cpu/cpu{cpu}/cache/index2");
        m = m
            .with(&format!("{d}/level"), "2\n")
            .with(&format!("{d}/type"), "Unified\n")
            .with(&format!("{d}/size"), format!("{size}\n"))
            .with(&format!("{d}/shared_cpu_list"), format!("{shared}\n"));
    }
    let s = build_snapshot(&m, None, 0);
    let l2: Vec<(u64, u32)> = s
        .cpu
        .caches
        .iter()
        .filter(|c| c.level == 2)
        .map(|c| (c.size_bytes, c.instances))
        .collect();
    assert_eq!(l2, vec![(1280 * 1024, 2), (2048 * 1024, 1)]);
}
