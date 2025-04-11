use crate::{DmiDump, ReadError, Source};
use rigscope_model::{
    BoardInfo, CacheKind, CacheLevel, CoreFreq, CpuInfo, DmiSource, Field, MemoryInfo, Snapshot,
    SystemInfo,
};
use rigscope_procfs as pf;
use rigscope_smbios::Decoded;

/// Paths read by the probe.
pub mod paths {
    pub const CPUINFO: &str = "/proc/cpuinfo";
    pub const MEMINFO: &str = "/proc/meminfo";
    pub const UPTIME: &str = "/proc/uptime";
    pub const OSRELEASE_KERNEL: &str = "/proc/sys/kernel/osrelease";
    pub const HOSTNAME: &str = "/proc/sys/kernel/hostname";
    pub const OS_RELEASE: &str = "/etc/os-release";
    pub const OS_RELEASE_FALLBACK: &str = "/usr/lib/os-release";
    pub const CPU_PRESENT: &str = "/sys/devices/system/cpu/present";
    pub const CPU_DIR: &str = "/sys/devices/system/cpu";
    pub const DMI_ID_DIR: &str = "/sys/class/dmi/id";
    pub const DMI_ENTRY: &str = "/sys/firmware/dmi/tables/smbios_entry_point";
    pub const DMI_TABLE: &str = "/sys/firmware/dmi/tables/DMI";
}

const NO_DMI: &str = "no DMI table (virtual machine or container?)";

fn text(src: &dyn Source, path: &str) -> Option<String> {
    src.read(path)
        .ok()
        .map(|b| String::from_utf8_lossy(&b).into_owned())
}

/// What we know about the DMI table after trying to read it.
enum Dmi {
    Full(Box<Decoded>),
    Denied,
    Missing,
}

fn read_dmi(src: &dyn Source, dump: Option<&DmiDump>) -> Dmi {
    if let Some(d) = dump {
        if let Ok(decoded) = rigscope_smbios::decode(&d.entry, &d.table) {
            return Dmi::Full(Box::new(decoded));
        }
    }
    let entry = src.read(paths::DMI_ENTRY);
    let table = src.read(paths::DMI_TABLE);
    match (entry, table) {
        (Ok(e), Ok(t)) => {
            rigscope_smbios::decode(&e, &t).map_or(Dmi::Missing, |d| Dmi::Full(Box::new(d)))
        }
        (Err(ReadError::PermissionDenied), _) | (_, Err(ReadError::PermissionDenied)) => {
            Dmi::Denied
        }
        _ => Dmi::Missing,
    }
}

/// Builds a snapshot. `dmi` (from the elevated helper) takes precedence over reading the
/// DMI table through `src`.
/// Fills disk health from the helper's smartctl output, matched by disk name.
fn with_smart(
    mut storage: rigscope_model::StorageInfo,
    dump: Option<&DmiDump>,
) -> rigscope_model::StorageInfo {
    let Some(dump) = dump else { return storage };
    for disk in &mut storage.disks {
        let Some((_, json)) = dump.smart.iter().find(|(n, _)| *n == disk.name) else {
            continue;
        };
        disk.health = match crate::parse_smartctl_report(json) {
            Ok(r) => {
                // sysfs values win; smartctl fills what the kernel does not expose.
                let fill = |f: &mut Field<String>, v: Option<String>| {
                    if f.known().is_none() {
                        if let Some(v) = v {
                            *f = Field::Known(v);
                        }
                    }
                };
                fill(&mut disk.serial, r.serial);
                fill(&mut disk.model, r.model);
                fill(&mut disk.firmware, r.firmware);
                Field::Known(r.health)
            }
            Err(crate::SmartError::DeviceError(msg)) => Field::Unavailable(msg),
            Err(crate::SmartError::NotJson) => {
                Field::Unavailable("smartctl output could not be read".into())
            }
        };
    }
    storage
}

pub fn build_snapshot(src: &dyn Source, dmi: Option<&DmiDump>, now_unix: u64) -> Snapshot {
    let dump = dmi;
    let dmi = read_dmi(src, dmi);
    let sysfs_board = read_sysfs_board(src);
    let has_sysfs_id = sysfs_board.is_some();

    let dmi_source = match (&dmi, has_sysfs_id) {
        (Dmi::Full(_), _) => DmiSource::FullTable,
        (Dmi::Denied, _) => DmiSource::SysfsIdOnly,
        (Dmi::Missing, true) => DmiSource::NoTable,
        (Dmi::Missing, false) => DmiSource::None,
    };

    let board = match (&dmi, sysfs_board) {
        (Dmi::Full(d), sys) => merge_board(d.board.clone(), sys.unwrap_or_default()),
        (_, Some(sys)) => sys,
        (_, None) => unavailable_board(),
    };

    let memory = MemoryInfo {
        os_total_bytes: Field::from_option(
            text(src, paths::MEMINFO)
                .as_deref()
                .and_then(pf::parse_meminfo_total),
        ),
        arrays: match &dmi {
            Dmi::Full(d) => d.memory.arrays.clone(),
            _ => Vec::new(),
        },
    };

    Snapshot {
        taken_at_unix: now_unix,
        system: read_system(src),
        board,
        memory,
        cpu: read_cpu(src, &dmi),
        storage: with_smart(crate::read_storage(src), dump),
        gpu: crate::read_gpus(src),
        net: crate::read_net(src),
        power: crate::read_power(src),
        sensors: crate::read_sensors(src),
        dmi_source,
        ..Snapshot::default()
    }
}

fn read_system(src: &dyn Source) -> SystemInfo {
    let os = text(src, paths::OS_RELEASE).or_else(|| text(src, paths::OS_RELEASE_FALLBACK));
    SystemInfo {
        os_name: Field::from_option(os.as_deref().and_then(pf::parse_os_release)),
        kernel: Field::from_option(
            text(src, paths::OSRELEASE_KERNEL)
                .as_deref()
                .and_then(rigscope_model::clean_text),
        ),
        hostname: Field::from_option(
            text(src, paths::HOSTNAME)
                .as_deref()
                .and_then(rigscope_model::clean_text),
        ),
        uptime_s: Field::from_option(
            text(src, paths::UPTIME)
                .as_deref()
                .and_then(pf::parse_uptime),
        ),
    }
}

fn read_sysfs_board(src: &dyn Source) -> Option<BoardInfo> {
    let mut any = false;
    let mut get = |name: &str| {
        let path = format!("{}/{name}", paths::DMI_ID_DIR);
        match src.read(&path) {
            Ok(b) => {
                any = true;
                Field::from_option(pf::clean_dmi_value(&String::from_utf8_lossy(&b)))
            }
            Err(ReadError::PermissionDenied) => {
                any = true;
                Field::NeedsRoot
            }
            Err(_) => Field::Unknown,
        }
    };
    let board = BoardInfo {
        vendor: get("board_vendor"),
        product: get("board_name"),
        version: get("board_version"),
        bios_vendor: get("bios_vendor"),
        bios_version: get("bios_version"),
        bios_date: get("bios_date"),
    };
    any.then_some(board)
}

/// SMBIOS values win; sysfs fills fields the table left unknown.
fn merge_board(table: BoardInfo, sys: BoardInfo) -> BoardInfo {
    let pick = |a: Field<String>, b: Field<String>| if a.known().is_some() { a } else { b };
    BoardInfo {
        vendor: pick(table.vendor, sys.vendor),
        product: pick(table.product, sys.product),
        version: pick(table.version, sys.version),
        bios_vendor: pick(table.bios_vendor, sys.bios_vendor),
        bios_version: pick(table.bios_version, sys.bios_version),
        bios_date: pick(table.bios_date, sys.bios_date),
    }
}

fn unavailable_board() -> BoardInfo {
    let u = || Field::Unavailable(NO_DMI.to_string());
    BoardInfo {
        vendor: u(),
        product: u(),
        version: u(),
        bios_vendor: u(),
        bios_version: u(),
        bios_date: u(),
    }
}

fn freq_path(cpu: u32, file: &str) -> String {
    format!("{}/cpu{cpu}/cpufreq/{file}", paths::CPU_DIR)
}

fn mhz(src: &dyn Source, path: &str) -> Option<u32> {
    text(src, path).as_deref().and_then(pf::parse_khz)
}

fn read_cpu(src: &dyn Source, dmi: &Dmi) -> CpuInfo {
    let info = text(src, paths::CPUINFO)
        .map(|t| pf::parse_cpuinfo(&t))
        .unwrap_or_default();
    let present = text(src, paths::CPU_PRESENT)
        .map(|t| pf::parse_cpu_list(&t))
        .unwrap_or_default();
    let cpus: Vec<u32> = if present.is_empty() {
        (0..info.logical).take(pf::MAX_CPUS).collect()
    } else {
        present
    };

    let cores: Vec<CoreFreq> = cpus
        .iter()
        .map(|&cpu| {
            let cur = mhz(src, &freq_path(cpu, "scaling_cur_freq"));
            let fallback = info
                .mhz_per_cpu
                .iter()
                .find(|(n, _)| *n == cpu)
                .map(|(_, m)| *m);
            CoreFreq {
                cpu,
                cur_mhz: Field::from_option(cur.or(fallback)),
                min_mhz: Field::from_option(mhz(src, &freq_path(cpu, "cpuinfo_min_freq"))),
                max_mhz: Field::from_option(mhz(src, &freq_path(cpu, "cpuinfo_max_freq"))),
                from_cpuinfo_fallback: cur.is_none() && fallback.is_some(),
            }
        })
        .collect();

    let cpufreq_max = cores
        .iter()
        .filter_map(|c| c.max_mhz.known().copied())
        .max();
    let base = cpus
        .first()
        .and_then(|&c| mhz(src, &freq_path(c, "base_frequency")));

    let (socket, smbios_max, smbios_cores, smbios_sockets) = match dmi {
        Dmi::Full(d) => (
            Field::from_option(d.cpu_socket.clone()),
            d.cpu_max_mhz,
            d.cpu_cores,
            d.cpu_sockets,
        ),
        Dmi::Denied => (Field::NeedsRoot, None, None, None),
        Dmi::Missing => (Field::Unavailable(NO_DMI.to_string()), None, None, None),
    };

    let logical = if info.logical > 0 {
        Some(info.logical)
    } else {
        u32::try_from(cpus.len()).ok().filter(|n| *n > 0)
    };

    CpuInfo {
        model: Field::from_option(info.model),
        vendor: Field::from_option(info.vendor),
        socket,
        physical_cores: Field::from_option(info.physical_cores.or(smbios_cores)),
        sockets: Field::from_option(info.packages.or(smbios_sockets)),
        logical_threads: Field::from_option(logical),
        base_mhz: Field::from_option(base),
        max_mhz: Field::from_option(cpufreq_max.or(smbios_max)),
        caches: read_caches(src, &cpus),
        cores,
        microcode: Field::from_option(info.microcode),
        flags: info.flags,
    }
}

fn read_caches(src: &dyn Source, cpus: &[u32]) -> Vec<CacheLevel> {
    // (level, kind, size) → distinct shared_cpu_list values. Size is part of the key because
    // hybrid CPUs have different cache sizes on P- and E-cores at the same level.
    let mut groups: Vec<(u8, CacheKind, u64, Vec<String>)> = Vec::new();
    for &cpu in cpus {
        let dir = format!("{}/cpu{cpu}/cache", paths::CPU_DIR);
        let Ok(entries) = src.list(&dir) else {
            continue;
        };
        for idx in entries.iter().filter(|e| e.starts_with("index")) {
            let read = |f: &str| text(src, &format!("{dir}/{idx}/{f}"));
            let level = read("level").and_then(|t| t.trim().parse::<u8>().ok());
            let kind = match read("type").as_deref().map(str::trim) {
                Some("Data") => Some(CacheKind::Data),
                Some("Instruction") => Some(CacheKind::Instruction),
                Some("Unified") => Some(CacheKind::Unified),
                _ => None,
            };
            let size = read("size").as_deref().and_then(pf::parse_cache_size);
            let (Some(level), Some(kind), Some(size)) = (level, kind, size) else {
                continue;
            };
            let shared = read("shared_cpu_list")
                .map(|t| t.trim().to_string())
                .unwrap_or_else(|| cpu.to_string());
            match groups
                .iter_mut()
                .find(|g| g.0 == level && g.1 == kind && g.2 == size)
            {
                Some(g) => {
                    if !g.3.contains(&shared) {
                        g.3.push(shared);
                    }
                }
                None => groups.push((level, kind, size, vec![shared])),
            }
        }
    }
    let mut out: Vec<CacheLevel> = groups
        .into_iter()
        .map(|(level, kind, size_bytes, shared)| CacheLevel {
            level,
            kind,
            size_bytes,
            instances: u32::try_from(shared.len()).unwrap_or(u32::MAX),
        })
        .collect();
    out.sort_by_key(|c| (c.level, c.kind as u8, c.size_bytes));
    out
}

/// Re-reads only the current per-core frequency.
pub fn refresh_live(src: &dyn Source, cpu: &mut CpuInfo) {
    let mut fallback: Option<Vec<(u32, u32)>> = None;
    for core in &mut cpu.cores {
        if let Some(cur) = mhz(src, &freq_path(core.cpu, "scaling_cur_freq")) {
            core.cur_mhz = Field::Known(cur);
            core.from_cpuinfo_fallback = false;
        } else if core.from_cpuinfo_fallback {
            let table = fallback.get_or_insert_with(|| {
                text(src, paths::CPUINFO)
                    .map(|t| pf::parse_cpuinfo(&t).mhz_per_cpu)
                    .unwrap_or_default()
            });
            if let Some((_, m)) = table.iter().find(|(n, _)| *n == core.cpu) {
                core.cur_mhz = Field::Known(*m);
            }
        }
    }
}
