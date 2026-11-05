use crate::{WinSource, field_err};
use rigscope_model::{Battery, Field, PowerInfo, clean_text};
use rigscope_win::layout::BATTERY_UNKNOWN;

pub const MAX_SUPPLIES: usize = 8;
const RELATIVE: &str = "the battery reports relative capacity, not mWh";
/// `BATTERY_STATUS.PowerState` bits.
const ON_LINE: u32 = 1;
const DISCHARGING: u32 = 2;
const CHARGING: u32 = 4;

pub(crate) fn read_power(src: &dyn WinSource) -> PowerInfo {
    let batteries = match src.batteries() {
        Ok(v) => v,
        Err(e) => {
            return PowerInfo {
                batteries: Vec::new(),
                ac_online: field_err(&e),
            };
        }
    };
    let batteries = batteries
        .iter()
        .take(MAX_SUPPLIES)
        .enumerate()
        .map(|(i, b)| {
            let mwh = |v: Option<u32>| -> Field<u64> {
                if b.relative {
                    return Field::Unavailable(RELATIVE.into());
                }
                Field::from_option(
                    v.filter(|v| *v != BATTERY_UNKNOWN && *v != 0)
                        .map(u64::from),
                )
            };
            let text = |s: &Option<String>| Field::from_option(s.as_deref().and_then(clean_text));
            let charge = match (b.now, b.full) {
                // Rounded to the nearest percent, as Windows shows it (43000/52440 → 82 %).
                (Some(n), Some(f)) if f > 0 && f != BATTERY_UNKNOWN && n != BATTERY_UNKNOWN => {
                    u8::try_from(((u64::from(n) * 100 + u64::from(f) / 2) / u64::from(f)).min(100))
                        .ok()
                }
                _ => None,
            };
            Battery {
                name: format!("BAT{i}"),
                manufacturer: text(&b.manufacturer),
                model: text(&b.name),
                serial: text(&b.serial),
                technology: text(&b.chemistry),
                status: Field::from_option(b.power_state.map(|s| {
                    if s & CHARGING != 0 {
                        "Charging"
                    } else if s & DISCHARGING != 0 {
                        "Discharging"
                    } else if s & ON_LINE != 0 {
                        "Not charging"
                    } else {
                        "Unknown"
                    }
                    .to_string()
                })),
                charge_percent: Field::from_option(charge),
                design_mwh: mwh(b.designed),
                full_mwh: mwh(b.full),
                now_mwh: mwh(b.now),
                cycles: Field::from_option(b.cycles.filter(|c| *c != 0)),
            }
        })
        .collect();
    PowerInfo {
        batteries,
        ac_online: match src.ac_online() {
            Ok(v) => Field::from_option(v),
            Err(e) => field_err(&e),
        },
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::indexing_slicing,
        clippy::field_reassign_with_default
    )]
    use super::*;
    use crate::MemWin;
    use rigscope_win::BatteryRaw;

    pub(crate) fn bat() -> BatteryRaw {
        BatteryRaw {
            name: Some("5B10W51867".into()),
            manufacturer: Some("SMP".into()),
            serial: Some("41234567".into()),
            chemistry: Some("LiP".into()),
            relative: false,
            designed: Some(57_000),
            full: Some(52_440),
            now: Some(43_000),
            cycles: Some(87),
            power_state: Some(2),
        }
    }

    #[test]
    fn battery_fields_and_health() {
        let mut w = MemWin::default();
        w.batteries = Ok(vec![bat()]);
        w.ac_online = Ok(Some(false));
        let p = read_power(&w);
        let b = &p.batteries[0];
        assert_eq!(p.ac_online, Field::Known(false));
        assert_eq!(b.name, "BAT0");
        assert_eq!(b.design_mwh, Field::Known(57_000));
        assert_eq!(b.health_percent(), Some(92.0));
        assert_eq!(b.cycles, Field::Known(87));
        assert_eq!(b.status, Field::Known("Discharging".into()));
        assert_eq!(b.charge_percent, Field::Known(82)); // 43000 / 52440
    }

    #[test]
    fn unknown_values_stay_unknown() {
        let mut w = MemWin::default();
        w.batteries = Ok(vec![BatteryRaw {
            cycles: Some(0),
            designed: Some(0xFFFF_FFFF),
            relative: false,
            ..bat()
        }]);
        let b = &read_power(&w).batteries[0];
        assert_eq!(b.cycles, Field::Unknown);
        assert_eq!(b.design_mwh, Field::Unknown);
        w.batteries = Ok(vec![BatteryRaw {
            relative: true,
            ..bat()
        }]);
        let b = &read_power(&w).batteries[0];
        assert!(matches!(b.design_mwh, Field::Unavailable(_)));
    }
}
