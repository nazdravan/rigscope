//! The processor on Windows: topology and caches from the `GetLogicalProcessorInformationEx`
//! buffer, microcode from the registry, and an estimated live clock from PDH samples.

use crate::system::Firmware;
use crate::{WinSource, field_err, field_of};
use rigscope_model::{CacheKind, CacheLevel, CoreFreq, CpuInfo, Field, clean_text};

pub const MAX_PROCESSORS_BUF: usize = 1024 * 1024;
const MAX_LOGICAL: usize = 4096;
const REL_CORE: u32 = 0;
const REL_CACHE: u32 = 2;
const REL_PACKAGE: u32 = 3;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Topology {
    pub packages: u32,
    pub cores: u32,
    /// Logical CPU numbers (group × 64 + bit), sorted.
    pub logical: Vec<u32>,
    pub caches: Vec<CacheLevel>,
}

fn u16_at(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        b.get(o..o.checked_add(2)?)?.try_into().ok()?,
    ))
}
fn u32_at(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        b.get(o..o.checked_add(4)?)?.try_into().ok()?,
    ))
}
fn u64_at(b: &[u8], o: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        b.get(o..o.checked_add(8)?)?.try_into().ok()?,
    ))
}

/// Logical CPUs in `count` GROUP_AFFINITY entries starting at `at` within `rec`.
fn cpus_in(rec: &[u8], at: usize, count: u16) -> Vec<u32> {
    let mut out = Vec::new();
    for i in 0..usize::from(count) {
        let Some(base) = i.checked_mul(16).and_then(|o| o.checked_add(at)) else {
            break;
        };
        let (Some(mask), Some(group)) = (
            u64_at(rec, base),
            base.checked_add(8).and_then(|o| u16_at(rec, o)),
        ) else {
            break;
        };
        for bit in 0..64u32 {
            if mask & (1u64 << bit) != 0 {
                out.push(u32::from(group) * 64 + bit);
            }
        }
    }
    out
}

/// Parses a `GetLogicalProcessorInformationEx(RelationAll)` buffer. `None` if empty or too big.
pub fn parse_processors(buf: &[u8]) -> Option<Topology> {
    if buf.is_empty() || buf.len() > MAX_PROCESSORS_BUF {
        return None;
    }
    let mut t = Topology::default();
    // (level, kind, size) -> instances
    let mut caches: Vec<(u8, CacheKind, u64, u32)> = Vec::new();
    let mut pos = 0usize;
    while let (Some(rel), Some(size)) = (
        u32_at(buf, pos),
        pos.checked_add(4).and_then(|o| u32_at(buf, o)),
    ) {
        let size = usize::try_from(size).ok()?;
        if size < 8 {
            break;
        }
        let end = pos.checked_add(size)?;
        let Some(rec) = buf.get(pos..end) else { break };
        match rel {
            REL_CORE => {
                t.cores = t.cores.saturating_add(1);
                if let Some(n) = u16_at(rec, 30) {
                    t.logical.extend(cpus_in(rec, 32, n));
                }
            }
            REL_PACKAGE => t.packages = t.packages.saturating_add(1),
            REL_CACHE => {
                let kind = match u32_at(rec, 16) {
                    Some(0) => Some(CacheKind::Unified),
                    Some(1) => Some(CacheKind::Instruction),
                    Some(2) => Some(CacheKind::Data),
                    _ => None, // Trace caches are not shown
                };
                if let (Some(level), Some(kind), Some(sz)) =
                    (rec.get(8).copied(), kind, u32_at(rec, 12))
                {
                    let sz = u64::from(sz);
                    match caches
                        .iter_mut()
                        .find(|c| c.0 == level && c.1 == kind && c.2 == sz)
                    {
                        Some(c) => c.3 = c.3.saturating_add(1),
                        None => caches.push((level, kind, sz, 1)),
                    }
                }
            }
            _ => {}
        }
        if t.logical.len() > MAX_LOGICAL {
            t.logical.truncate(MAX_LOGICAL);
            break;
        }
        pos = end;
    }
    t.logical.sort_unstable();
    t.logical.dedup();
    t.caches = caches
        .into_iter()
        .map(|(level, kind, size_bytes, instances)| CacheLevel {
            level,
            kind,
            size_bytes,
            instances,
        })
        .collect();
    t.caches
        .sort_by_key(|c| (c.level, c.kind as u8, c.size_bytes));
    Some(t)
}

/// `Update Revision` (8 bytes): Intel keeps the revision in the high dword, AMD in the low one.
pub fn microcode(rev: &[u8]) -> Option<String> {
    let hi = u32_at(rev, 4)?;
    let lo = u32_at(rev, 0)?;
    let v = if hi != 0 { hi } else { lo };
    (v != 0).then(|| format!("{v:#x}"))
}

pub(crate) fn read_cpu(src: &mut dyn WinSource, fw: &Firmware) -> CpuInfo {
    let topo = src.processors().ok().and_then(|b| parse_processors(&b));
    let reg = src.cpu_registry();
    let d = fw.decoded.as_ref();
    let logical = topo.as_ref().map(|t| t.logical.clone()).unwrap_or_default();
    let base = reg.as_ref().ok().and_then(|r| r.mhz).filter(|m| *m > 0);
    let mut cpu = CpuInfo {
        model: field_of(&reg, |r| {
            Field::from_option(r.name.as_deref().and_then(clean_text))
        }),
        vendor: field_of(&reg, |r| {
            Field::from_option(r.vendor.as_deref().and_then(clean_text))
        }),
        socket: match d {
            Some(d) => Field::from_option(d.cpu_socket.clone()),
            None => fw.missing.clone(),
        },
        physical_cores: Field::from_option(
            topo.as_ref()
                .map(|t| t.cores)
                .filter(|n| *n > 0)
                .or(d.and_then(|d| d.cpu_cores)),
        ),
        sockets: Field::from_option(
            topo.as_ref()
                .map(|t| t.packages)
                .filter(|n| *n > 0)
                .or(d.and_then(|d| d.cpu_sockets)),
        ),
        logical_threads: Field::from_option(u32::try_from(logical.len()).ok().filter(|n| *n > 0)),
        base_mhz: Field::from_option(base),
        // SMBIOS "Max Speed" can sit below the measured base (Dell reports 2100 for a 2.7 GHz part).
        max_mhz: Field::from_option(
            d.and_then(|d| d.cpu_max_mhz)
                .filter(|m| base.is_none_or(|b| *m >= b)),
        ),
        cores: logical
            .iter()
            .map(|&c| CoreFreq {
                cpu: c,
                ..CoreFreq::default()
            })
            .collect(),
        caches: topo.map(|t| t.caches).unwrap_or_default(),
        microcode: field_of(&reg, |r| {
            Field::from_option(r.update_revision.as_deref().and_then(microcode))
        }),
        flags: src.cpu_features().unwrap_or_default(),
    };
    // Primes PDH: the first sample has no rate yet, so the values stay Unknown.
    refresh_cpu(src, &mut cpu);
    cpu
}

/// Per-core clock = % processor performance × base clock (what Task Manager shows), marked
/// estimated. Without a base clock or a sample the cores stay as they are.
pub(crate) fn refresh_cpu(src: &mut dyn WinSource, cpu: &mut CpuInfo) {
    let Some(base) = cpu.base_mhz.known().copied() else {
        return;
    };
    let samples = match src.cpu_performance() {
        Ok(s) => s,
        Err(e) => {
            for c in &mut cpu.cores {
                c.cur_mhz = field_err(&e);
            }
            return;
        }
    };
    for core in &mut cpu.cores {
        if let Some((_, pct)) = samples.iter().find(|(n, _)| *n == core.cpu) {
            if pct.is_finite() && *pct >= 0.0 {
                let mhz = (f64::from(base) * pct / 100.0).round();
                if mhz <= f64::from(u32::MAX) {
                    core.cur_mhz = Field::Known(mhz as u32);
                    core.estimated = true;
                }
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::indexing_slicing,
        clippy::field_reassign_with_default
    )]
    use super::*;
    use crate::{MemWin, system::read_firmware};
    use rigscope_model::{CacheKind, Field};
    use rigscope_win::{CpuRegistryRaw, WinError};

    /// One processor-core record with the given logical-CPU bitmask in group 0.
    pub(crate) fn core(mask: u64, smt: bool) -> Vec<u8> {
        let mut r = vec![0u8; 48];
        r[0..4].copy_from_slice(&0u32.to_le_bytes());
        r[4..8].copy_from_slice(&48u32.to_le_bytes());
        r[8] = u8::from(smt);
        r[30..32].copy_from_slice(&1u16.to_le_bytes());
        r[32..40].copy_from_slice(&mask.to_le_bytes());
        r
    }

    pub(crate) fn package(mask: u64) -> Vec<u8> {
        let mut r = core(mask, false);
        r[0..4].copy_from_slice(&3u32.to_le_bytes());
        r
    }

    pub(crate) fn cache(level: u8, kind: u32, size: u32, mask: u64) -> Vec<u8> {
        let mut r = vec![0u8; 56];
        r[0..4].copy_from_slice(&2u32.to_le_bytes());
        r[4..8].copy_from_slice(&56u32.to_le_bytes());
        r[8] = level;
        r[12..16].copy_from_slice(&size.to_le_bytes());
        r[16..20].copy_from_slice(&kind.to_le_bytes());
        r[38..40].copy_from_slice(&1u16.to_le_bytes());
        r[40..48].copy_from_slice(&mask.to_le_bytes());
        r
    }

    /// 4 cores / 8 threads, 1 package: L1d 48K and L1i 32K per core, L2 1.25M per core, L3 12M shared.
    pub(crate) fn quad_core() -> Vec<u8> {
        let mut b = Vec::new();
        for c in 0..4u64 {
            let m = 0b11 << (2 * c);
            b.extend(core(m, true));
            b.extend(cache(1, 2, 48 << 10, m));
            b.extend(cache(1, 1, 32 << 10, m));
            b.extend(cache(2, 0, 1280 << 10, m));
        }
        b.extend(cache(3, 0, 12 << 20, 0xFF));
        b.extend(package(0xFF));
        b
    }

    #[test]
    fn topology_and_caches() {
        let t = parse_processors(&quad_core()).unwrap();
        assert_eq!((t.packages, t.cores), (1, 4));
        assert_eq!(t.logical, (0..8).collect::<Vec<_>>());
        let l1d = t
            .caches
            .iter()
            .find(|c| c.level == 1 && c.kind == CacheKind::Data)
            .unwrap();
        assert_eq!((l1d.size_bytes, l1d.instances), (48 << 10, 4));
        let l3 = t.caches.iter().find(|c| c.level == 3).unwrap();
        assert_eq!((l3.size_bytes, l3.instances), (12 << 20, 1));
    }

    #[test]
    fn hybrid_caches_group_by_size() {
        let mut b = quad_core();
        // Two E-cores with a 2 MiB L2, which must be a separate group from the 1.25 MiB ones.
        b.extend(core(1 << 8, false));
        b.extend(cache(2, 0, 2 << 20, 1 << 8));
        b.extend(core(1 << 9, false));
        b.extend(cache(2, 0, 2 << 20, 1 << 9));
        let t = parse_processors(&b).unwrap();
        let l2: Vec<_> = t.caches.iter().filter(|c| c.level == 2).collect();
        assert_eq!(l2.len(), 2);
        assert_eq!(t.cores, 6);
        assert_eq!(t.logical.len(), 10);
    }

    #[test]
    fn garbage_never_panics_and_bad_size_stops() {
        assert_eq!(parse_processors(&[]), None);
        let mut r = core(1, false);
        r[4..8].copy_from_slice(&0u32.to_le_bytes()); // Size 0 would loop forever
        assert_eq!(parse_processors(&r).map(|t| t.cores), Some(0));
        assert_eq!(parse_processors(&vec![0u8; MAX_PROCESSORS_BUF + 1]), None);
        // A huge Size must not overflow the cursor.
        let mut r = core(1, false);
        r[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(parse_processors(&r).map(|t| t.cores), Some(0));
    }

    #[test]
    fn trace_caches_are_skipped() {
        let t = parse_processors(&cache(1, 3, 12 << 10, 1)).unwrap();
        assert!(t.caches.is_empty());
    }

    #[test]
    fn microcode_from_update_revision() {
        // Intel: the revision is in the high 4 bytes.
        assert_eq!(
            microcode(&[0, 0, 0, 0, 0xF4, 0, 0, 0]).as_deref(),
            Some("0xf4")
        );
        // AMD (the high 4 bytes are zero): the low part.
        assert_eq!(
            microcode(&[0x3E, 0x10, 0x20, 0x0A, 0, 0, 0, 0]).as_deref(),
            Some("0xa20103e")
        );
        assert_eq!(microcode(&[0; 8]), None);
        assert_eq!(microcode(&[1, 2]), None);
    }

    #[test]
    fn cpu_info_and_estimated_live_frequency() {
        let mut w = MemWin::default();
        w.processors = Ok(quad_core());
        w.cpu_registry = Ok(CpuRegistryRaw {
            name: Some("12th Gen Intel(R) Core(TM) i5-12400F  ".into()),
            vendor: Some("GenuineIntel".into()),
            mhz: Some(2500),
            update_revision: Some(vec![0, 0, 0, 0, 0x2C, 0x01, 0, 0]),
        });
        w.cpu_features = Ok(vec!["avx2".into(), "sse4.2".into()]);
        w.perf = vec![Ok(Vec::new()), Ok((0..8).map(|c| (c, 160.0)).collect())];
        let fw = read_firmware(&w);
        let mut cpu = read_cpu(&mut w, &fw);
        assert_eq!(
            cpu.model,
            Field::Known("12th Gen Intel(R) Core(TM) i5-12400F".into())
        );
        assert_eq!(cpu.physical_cores, Field::Known(4));
        assert_eq!(cpu.logical_threads, Field::Known(8));
        assert_eq!(cpu.sockets, Field::Known(1));
        assert_eq!(cpu.base_mhz, Field::Known(2500));
        assert_eq!(cpu.microcode, Field::Known("0x12c".into()));
        assert_eq!(cpu.flags, vec!["avx2".to_string(), "sse4.2".into()]);
        assert_eq!(cpu.cores.len(), 8);
        // The first PDH sample has no data: Unknown, not an invented value.
        assert_eq!(cpu.cores[0].cur_mhz, Field::Unknown);
        refresh_cpu(&mut w, &mut cpu);
        assert_eq!(cpu.cores[0].cur_mhz, Field::Known(4000));
        assert!(cpu.cores[0].estimated);
    }

    #[test]
    fn cpu_without_registry_access() {
        let mut w = MemWin::default();
        w.cpu_registry = Err(WinError::AccessDenied);
        let fw = read_firmware(&w);
        let cpu = read_cpu(&mut w, &fw);
        assert_eq!(cpu.model, Field::NeedsElevation);
        assert_eq!(cpu.physical_cores, Field::Unknown);
    }

    proptest::proptest! {
        #[test]
        fn parse_processors_never_panics(b in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..2048)) {
            let _ = parse_processors(&b);
        }
    }
}
