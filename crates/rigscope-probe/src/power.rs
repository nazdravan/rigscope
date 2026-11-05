//! Batteries and the mains adapter from `/sys/class/power_supply` (no root).

use crate::Source;
use crate::sysfs::text;
use rigscope_model::{Battery, Field, PowerInfo, clean_text};

pub const POWER_DIR: &str = "/sys/class/power_supply";
pub const MAX_SUPPLIES: usize = 32;

fn num(src: &dyn Source, path: &str) -> Option<u64> {
    text(src, path)?.parse::<u64>().ok()
}

pub fn read_power(src: &dyn Source) -> PowerInfo {
    let Ok(names) = src.list(POWER_DIR) else {
        return PowerInfo::default();
    };
    let mut out = PowerInfo::default();
    for name in names.iter().take(MAX_SUPPLIES) {
        let base = format!("{POWER_DIR}/{name}");
        let read = |f: &str| text(src, &format!("{base}/{f}"));
        match read("type").as_deref() {
            Some("Mains") => {
                if let Some(online) = read("online") {
                    let now = online == "1";
                    // Several adapters: any one online means on mains.
                    out.ac_online =
                        Field::Known(now || out.ac_online.known().copied().unwrap_or(false));
                }
            }
            // `scope=Device`: a mouse or headset battery, not the machine's.
            Some("Battery") if read("scope").as_deref() != Some("Device") => {
                out.batteries.push(read_battery(src, name, &base))
            }
            _ => {}
        }
    }
    out
}

/// Energy in mWh from `energy_*` (µWh), or from `charge_*` (µAh) × design voltage (µV).
fn energy_mwh(src: &dyn Source, base: &str, which: &str, voltage_uv: Option<u64>) -> Option<u64> {
    if let Some(uwh) = num(src, &format!("{base}/energy_{which}")) {
        return Some(uwh / 1000);
    }
    let uah = num(src, &format!("{base}/charge_{which}"))?;
    let uv = voltage_uv?;
    // µAh × µV = 1e-12 Wh → mWh = µAh × µV / 1e9.
    u64::try_from(u128::from(uah) * u128::from(uv) / 1_000_000_000).ok()
}

fn read_battery(src: &dyn Source, name: &str, base: &str) -> Battery {
    let read = |f: &str| {
        text(src, &format!("{base}/{f}"))
            .as_deref()
            .and_then(clean_text)
    };
    let voltage = num(src, &format!("{base}/voltage_min_design"));
    Battery {
        name: name.to_string(),
        manufacturer: Field::from_option(read("manufacturer")),
        model: Field::from_option(read("model_name")),
        serial: Field::from_option(read("serial_number")),
        technology: Field::from_option(read("technology")),
        status: Field::from_option(read("status")),
        charge_percent: Field::from_option(
            num(src, &format!("{base}/capacity"))
                .and_then(|c| u8::try_from(c).ok())
                .filter(|c| *c <= 100),
        ),
        design_mwh: Field::from_option(energy_mwh(src, base, "full_design", voltage)),
        full_mwh: Field::from_option(energy_mwh(src, base, "full", voltage)),
        now_mwh: Field::from_option(energy_mwh(src, base, "now", voltage)),
        cycles: Field::from_option(
            num(src, &format!("{base}/cycle_count"))
                .and_then(|c| u32::try_from(c).ok())
                .filter(|c| *c > 0),
        ),
    }
}
