//! Panic-free parsers for the Linux `/proc` and `/sys` text files rigscope reads.
//!
//! All functions take text already read (and size-capped) by the caller; none do IO.

mod cpuinfo;
mod values;

pub use cpuinfo::{parse_cpuinfo, CpuinfoSummary};
pub use values::{
    clean_dmi_value, parse_cache_size, parse_cpu_list, parse_khz, parse_meminfo_total,
    parse_os_release, parse_uptime, MAX_CPUS,
};
