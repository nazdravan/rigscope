use crate::Field;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CpuInfo {
    pub model: Field<String>,
    pub vendor: Field<String>,
    pub socket: Field<String>,
    pub physical_cores: Field<u32>,
    /// Populated sockets (packages). Added after schema 1; older files load as unknown.
    #[serde(default)]
    pub sockets: Field<u32>,
    pub logical_threads: Field<u32>,
    pub base_mhz: Field<u32>,
    pub max_mhz: Field<u32>,
    pub cores: Vec<CoreFreq>,
    pub caches: Vec<CacheLevel>,
    pub microcode: Field<String>,
    pub flags: Vec<String>,
}

/// `2 × LGA4677` for multi-socket machines, else the (already formatted) socket name.
pub fn socket_label(name: String, sockets: &Field<u32>) -> String {
    match sockets.known() {
        Some(n) if *n > 1 => format!("{n} × {name}"),
        _ => name,
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CoreFreq {
    /// Logical CPU number (`cpuN`).
    pub cpu: u32,
    pub cur_mhz: Field<u32>,
    pub min_mhz: Field<u32>,
    pub max_mhz: Field<u32>,
    /// `cur_mhz` came from `/proc/cpuinfo` because cpufreq is missing.
    pub from_cpuinfo_fallback: bool,
    /// The value is derived (Windows: % processor performance × base clock), not measured.
    #[serde(default)]
    pub estimated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheKind {
    Data,
    Instruction,
    Unified,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheLevel {
    pub level: u8,
    pub kind: CacheKind,
    /// Size of one instance.
    pub size_bytes: u64,
    /// Number of distinct instances across the package(s).
    pub instances: u32,
}
