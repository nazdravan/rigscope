//! Temperatures and fans from `/sys/class/hwmon` (no root).

use crate::sysfs::text;
use crate::Source;
use rigscope_model::{Fan, Field, SensorChip, SensorInfo, Temperature};

pub const HWMON_DIR: &str = "/sys/class/hwmon";
pub const MAX_CHIPS: usize = 64;
pub const MAX_CHANNELS: usize = 64;
/// Readings outside this range (m°C) come from broken or unconnected sensors.
const TEMP_RANGE: std::ops::RangeInclusive<i32> = -60_000..=250_000;

fn temp(src: &dyn Source, path: &str) -> Option<i32> {
    text(src, path)?
        .parse::<i32>()
        .ok()
        .filter(|v| TEMP_RANGE.contains(v))
}

fn rpm(src: &dyn Source, path: &str) -> Option<u32> {
    text(src, path)?.parse::<u32>().ok()
}

/// Channel numbers N of files named `{prefix}N_input`, sorted.
fn channels(files: &[String], prefix: &str) -> Vec<u32> {
    let mut out: Vec<u32> = files
        .iter()
        .filter_map(|f| f.strip_prefix(prefix)?.strip_suffix("_input")?.parse().ok())
        .collect();
    out.sort_unstable();
    out.dedup();
    out.truncate(MAX_CHANNELS);
    out
}

pub fn read_sensors(src: &dyn Source) -> SensorInfo {
    let Ok(names) = src.list(HWMON_DIR) else {
        return SensorInfo::default();
    };
    let chips = names
        .iter()
        .filter(|n| n.starts_with("hwmon"))
        .take(MAX_CHIPS)
        .filter_map(|hwmon| read_chip(src, hwmon))
        .collect();
    SensorInfo { chips }
}

fn read_chip(src: &dyn Source, hwmon: &str) -> Option<SensorChip> {
    let dir = format!("{HWMON_DIR}/{hwmon}");
    let files = src.list(&dir).ok()?;
    let name = text(src, &format!("{dir}/name")).unwrap_or_else(|| hwmon.to_string());
    let label =
        |kind: &str, n: u32| text(src, &format!("{dir}/{kind}{n}_label")).filter(|l| !l.is_empty());
    let temps = channels(&files, "temp")
        .into_iter()
        .map(|n| Temperature {
            index: n,
            label: label("temp", n).unwrap_or_else(|| format!("temp{n}")),
            millicelsius: Field::from_option(temp(src, &format!("{dir}/temp{n}_input"))),
            max_mc: Field::from_option(temp(src, &format!("{dir}/temp{n}_max"))),
            crit_mc: Field::from_option(temp(src, &format!("{dir}/temp{n}_crit"))),
        })
        .collect::<Vec<_>>();
    let fans = channels(&files, "fan")
        .into_iter()
        .filter_map(|n| {
            let value = rpm(src, &format!("{dir}/fan{n}_input"))?;
            let named = label("fan", n);
            // An unlabelled header reading 0 RPM is almost always nothing plugged in.
            if value == 0 && named.is_none() {
                return None;
            }
            Some(Fan {
                index: n,
                label: named.unwrap_or_else(|| format!("fan{n}")),
                rpm: Field::Known(value),
            })
        })
        .collect::<Vec<_>>();
    if temps.is_empty() && fans.is_empty() {
        return None;
    }
    Some(SensorChip {
        hwmon: hwmon.to_string(),
        name,
        temps,
        fans,
    })
}

/// Re-reads only the current values (for the live view).
pub fn refresh_sensors(src: &dyn Source, sensors: &mut SensorInfo) {
    for chip in &mut sensors.chips {
        let dir = format!("{HWMON_DIR}/{}", chip.hwmon);
        for t in &mut chip.temps {
            t.millicelsius = Field::from_option(temp(src, &format!("{dir}/temp{}_input", t.index)));
        }
        for f in &mut chip.fans {
            f.rpm = Field::from_option(rpm(src, &format!("{dir}/fan{}_input", f.index)));
        }
    }
}
