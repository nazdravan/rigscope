//! SMART data read straight from the drive on Windows (the Linux side parses smartctl JSON).
//! Same `DiskHealth` model, same rules: absent values stay `None`.

use rigscope_model::DiskHealth;

const NVME_DATA_UNIT: u64 = 512_000;
const LOG_LEN: usize = 512;

/// Reads a little-endian u128 counter at `off` as a u64. A non-zero high half means the
/// value does not fit in u64, which only a corrupt or unknown log produces, so it reads as
/// absent (`None`) rather than as an invented number.
fn u128_as_u64(b: &[u8], off: usize) -> Option<u64> {
    let lo = u64::from_le_bytes(b.get(off..off + 8)?.try_into().ok()?);
    let hi = u64::from_le_bytes(b.get(off + 8..off + 16)?.try_into().ok()?);
    (hi == 0).then_some(lo)
}

pub fn parse_nvme_health(log: &[u8]) -> Option<DiskHealth> {
    if log.len() != LOG_LEN {
        return None;
    }
    let temp_k = u16::from_le_bytes(log.get(1..3)?.try_into().ok()?);
    Some(DiskHealth {
        passed: Some(*log.first()? == 0),
        temperature_c: (temp_k > 0).then(|| i32::from(temp_k) - 273),
        available_spare: log.get(3).map(|v| u32::from(*v)),
        percent_used: log.get(5).map(|v| u32::from(*v)),
        written_bytes: u128_as_u64(log, 48).and_then(|u| u.checked_mul(NVME_DATA_UNIT)),
        power_cycles: u128_as_u64(log, 112),
        power_on_hours: u128_as_u64(log, 128),
        media_errors: u128_as_u64(log, 160),
        ..DiskHealth::default()
    })
}

struct Attr {
    id: u8,
    prefail: bool,
    current: u8,
    raw: u64,
}

fn attrs(sector: &[u8]) -> Vec<Attr> {
    (0..30)
        .filter_map(|i| {
            let e = sector.get(2 + i * 12..2 + i * 12 + 12)?;
            let id = *e.first()?;
            if id == 0 {
                return None;
            }
            let mut raw = [0u8; 8];
            raw.get_mut(..6)?.copy_from_slice(e.get(5..11)?);
            Some(Attr {
                id,
                prefail: e.get(1).is_some_and(|f| f & 1 != 0),
                current: *e.get(3)?,
                raw: u64::from_le_bytes(raw),
            })
        })
        .collect()
}

pub fn parse_ata_smart(attributes: &[u8], thresholds: &[u8]) -> Option<DiskHealth> {
    if attributes.len() < LOG_LEN || thresholds.len() < LOG_LEN {
        return None;
    }
    let list = attrs(attributes);
    // An all-zero (or failed) read lists no attribute: that is no data, not a healthy disk.
    if list.is_empty() {
        return None;
    }
    let threshold = |id: u8| -> Option<u8> {
        (0..30).find_map(|i| {
            let e = thresholds.get(2 + i * 12..2 + i * 12 + 2)?;
            (e.first() == Some(&id))
                .then(|| e.get(1).copied())
                .flatten()
        })
    };
    let raw = |id: u8| list.iter().find(|a| a.id == id).map(|a| a.raw);
    let failing = list
        .iter()
        .any(|a| a.prefail && threshold(a.id).is_some_and(|t| t != 0 && a.current <= t));
    Some(DiskHealth {
        passed: failing.then_some(false),
        // 194 (or 190): the current temperature is the low byte of the raw value;
        // 0 means the drive does not report it.
        temperature_c: [194, 190]
            .into_iter()
            .filter_map(raw)
            .map(|r| (r & 0xFF) as u8)
            .find(|t| *t != 0)
            .map(i32::from),
        // 9: low 32 bits (some vendors pack minutes in the upper bytes).
        power_on_hours: raw(9).map(|r| r & 0xFFFF_FFFF),
        power_cycles: raw(12),
        reallocated_sectors: raw(5),
        pending_sectors: raw(197),
        uncorrectable_sectors: raw(198),
        ..DiskHealth::default()
    })
}

#[cfg(test)]
pub(crate) mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]
    use super::*;

    pub(crate) fn nvme_log(temp_k: u16, used: u8, written_units: u64, warn: u8) -> Vec<u8> {
        let mut l = vec![0u8; 512];
        l[0] = warn;
        l[1..3].copy_from_slice(&temp_k.to_le_bytes());
        l[3] = 100;
        l[5] = used;
        l[48..56].copy_from_slice(&written_units.to_le_bytes());
        l[112..120].copy_from_slice(&512u64.to_le_bytes());
        l[128..136].copy_from_slice(&4321u64.to_le_bytes());
        l
    }

    #[test]
    fn nvme_health_fields() {
        let h = parse_nvme_health(&nvme_log(312, 2, 19_531_250, 0)).unwrap();
        assert_eq!(h.passed, Some(true));
        assert_eq!(h.temperature_c, Some(39));
        assert_eq!(h.percent_used, Some(2));
        assert_eq!(h.available_spare, Some(100));
        assert_eq!(h.power_on_hours, Some(4321));
        assert_eq!(h.power_cycles, Some(512));
        assert_eq!(h.written_bytes, Some(10_000_000_000_000));
        assert_eq!(h.media_errors, Some(0));
        assert_eq!(
            parse_nvme_health(&nvme_log(312, 2, 0, 0b100))
                .unwrap()
                .passed,
            Some(false)
        );
        assert_eq!(parse_nvme_health(&[0; 100]), None);
        // 0 K means the sensor is absent, not -273 °C.
        assert_eq!(
            parse_nvme_health(&nvme_log(0, 0, 0, 0))
                .unwrap()
                .temperature_c,
            None
        );
    }

    #[test]
    fn nvme_counter_above_u64_reads_as_absent() {
        // A non-zero high half on power_on_hours (bytes 136..144 of the u128) is a corrupt log.
        let mut log = nvme_log(312, 2, 19_531_250, 0);
        log[136..144].copy_from_slice(&1u64.to_le_bytes());
        let h = parse_nvme_health(&log).unwrap();
        assert_eq!(h.power_on_hours, None);
        // The other counters are still parsed.
        assert_eq!(h.power_cycles, Some(512));
        assert_eq!(h.media_errors, Some(0));
        assert_eq!(h.written_bytes, Some(10_000_000_000_000));
    }

    pub(crate) fn ata_sectors(
        attrs: &[(u8, u16, u8, u64)],
        thr: &[(u8, u8)],
    ) -> (Vec<u8>, Vec<u8>) {
        let mut a = vec![0u8; 512];
        let mut t = vec![0u8; 512];
        for (i, (id, flags, cur, raw)) in attrs.iter().enumerate() {
            let o = 2 + i * 12;
            a[o] = *id;
            a[o + 1..o + 3].copy_from_slice(&flags.to_le_bytes());
            a[o + 3] = *cur;
            a[o + 4] = *cur;
            a[o + 5..o + 11].copy_from_slice(&raw.to_le_bytes()[..6]);
        }
        for (i, (id, th)) in thr.iter().enumerate() {
            let o = 2 + i * 12;
            t[o] = *id;
            t[o + 1] = *th;
        }
        (a, t)
    }

    #[test]
    fn ata_attributes() {
        let (a, t) = ata_sectors(
            &[
                (5, 0x33, 100, 8),
                (9, 0x32, 95, 4321),
                (12, 0x32, 99, 512),
                (194, 0x22, 61, 0x0028_0000_0027),
                (197, 0x32, 100, 1),
                (198, 0x30, 100, 0),
            ],
            &[(5, 10), (9, 0), (12, 0), (194, 0), (197, 0), (198, 0)],
        );
        let h = parse_ata_smart(&a, &t).unwrap();
        assert_eq!(h.reallocated_sectors, Some(8));
        assert_eq!(h.power_on_hours, Some(4321));
        assert_eq!(h.power_cycles, Some(512));
        assert_eq!(h.temperature_c, Some(39)); // the low byte of the raw value
        assert_eq!(h.pending_sectors, Some(1));
        assert_eq!(h.uncorrectable_sectors, Some(0));
        // Nothing at or below threshold: no "passed" claim without SMART RETURN STATUS.
        assert_eq!(h.passed, None);
    }

    #[test]
    fn ata_prefail_attribute_at_threshold_fails() {
        let (a, t) = ata_sectors(&[(5, 0x33, 10, 2000)], &[(5, 10)]);
        assert_eq!(parse_ata_smart(&a, &t).unwrap().passed, Some(false));
        assert_eq!(parse_ata_smart(&a[..100], &t), None);
    }

    #[test]
    fn ata_all_zero_sectors_are_no_data() {
        assert_eq!(parse_ata_smart(&[0; 512], &[0; 512]), None);
    }

    #[test]
    fn ata_temperature_raw_zero_is_absent() {
        let (a, t) = ata_sectors(&[(9, 0x32, 95, 4321), (194, 0x22, 100, 0)], &[]);
        let h = parse_ata_smart(&a, &t).unwrap();
        assert_eq!(h.temperature_c, None);
        assert_eq!(h.power_on_hours, Some(4321));
        // 194 reads 0, 190 has a value: use 190.
        let (a, t) = ata_sectors(&[(194, 0x22, 100, 0), (190, 0x22, 64, 36)], &[]);
        assert_eq!(parse_ata_smart(&a, &t).unwrap().temperature_c, Some(36));
    }

    proptest::proptest! {
        #[test]
        fn never_panics(a in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..600),
                        t in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..600)) {
            let _ = parse_nvme_health(&a);
            let _ = parse_ata_smart(&a, &t);
        }
    }
}
