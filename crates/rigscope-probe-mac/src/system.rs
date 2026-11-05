use crate::MacSource;
use crate::parse::{parse_boottime, parse_hardware};
use rigscope_model::{BoardInfo, CpuInfo, Field, SystemInfo, clean_text};

fn sysctl_text(src: &dyn MacSource, name: &str) -> Field<String> {
    match src.sysctl(name) {
        Ok(s) => Field::from_option(clean_text(&s)),
        Err(_) => Field::Unknown,
    }
}

fn sysctl_u32(src: &dyn MacSource, name: &str) -> Field<u32> {
    Field::from_option(src.sysctl(name).ok().and_then(|s| s.trim().parse().ok()))
}

/// macOS product version -> marketing name; newer or unknown versions keep the number.
fn os_name(version: &str) -> String {
    let major: u32 = version
        .split('.')
        .next()
        .and_then(|m| m.parse().ok())
        .unwrap_or(0);
    let name = match major {
        11 => Some("Big Sur"),
        12 => Some("Monterey"),
        13 => Some("Ventura"),
        14 => Some("Sonoma"),
        15 => Some("Sequoia"),
        _ => None,
    };
    match name {
        Some(n) => format!("macOS {version} ({n})"),
        None => format!("macOS {version}"),
    }
}

pub(crate) fn read_system(src: &dyn MacSource, now_unix: u64) -> SystemInfo {
    SystemInfo {
        os_name: match src.sysctl("kern.osproductversion") {
            Ok(v) => Field::from_option(clean_text(&v).map(|v| os_name(&v))),
            Err(_) => Field::Unknown,
        },
        kernel: match sysctl_text(src, "kern.osrelease") {
            Field::Known(r) => Field::Known(format!("Darwin {r}")),
            other => other,
        },
        hostname: sysctl_text(src, "kern.hostname"),
        uptime_s: Field::from_option(
            src.sysctl("kern.boottime")
                .ok()
                .and_then(|s| parse_boottime(&s))
                .and_then(|boot| now_unix.checked_sub(boot)),
        ),
    }
}

pub(crate) fn read_board(src: &dyn MacSource) -> BoardInfo {
    let hw = src
        .profiler("SPHardwareDataType")
        .ok()
        .and_then(|j| parse_hardware(&j))
        .unwrap_or_default();
    BoardInfo {
        vendor: Field::Known("Apple".into()),
        product: Field::from_option(
            hw.model
                .or_else(|| sysctl_text(src, "hw.model").known().cloned()),
        ),
        version: Field::from_option(hw.name),
        bios_vendor: Field::Unknown,
        bios_version: Field::from_option(hw.boot_rom),
        bios_date: Field::Unknown,
    }
}

pub(crate) fn read_cpu(src: &dyn MacSource) -> CpuInfo {
    let hw = src
        .profiler("SPHardwareDataType")
        .ok()
        .and_then(|j| parse_hardware(&j));
    let model = sysctl_text(src, "machdep.cpu.brand_string");
    let model = match (&model, hw.and_then(|h| h.cpu)) {
        (Field::Known(_), _) => model,
        (_, Some(c)) => Field::Known(c),
        _ => model,
    };
    // `hw.cpufrequency` is in Hz and exists on Intel Macs only.
    let base_mhz = Field::from_option(
        src.sysctl("hw.cpufrequency")
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .and_then(|hz| u32::try_from(hz / 1_000_000).ok())
            .filter(|mhz| *mhz > 0),
    );
    CpuInfo {
        model,
        vendor: sysctl_text(src, "machdep.cpu.vendor"),
        physical_cores: sysctl_u32(src, "hw.physicalcpu"),
        logical_threads: sysctl_u32(src, "hw.logicalcpu"),
        sockets: sysctl_u32(src, "hw.packages"),
        base_mhz,
        ..CpuInfo::default()
    }
}
