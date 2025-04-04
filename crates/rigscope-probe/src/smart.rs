//! Parsing of `smartctl --json -a` output (smartmontools). Untrusted input: never panics.

use rigscope_model::DiskHealth;

/// Largest smartctl JSON accepted per disk.
pub const MAX_SMART_JSON: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmartError {
    NotJson,
    /// smartctl ran but could not open or identify the device.
    DeviceError(String),
}

use serde_json::Value;

/// smartctl exit-status bits 0 and 1: bad command line / device could not be opened.
const FATAL_EXIT_BITS: u64 = 0b11;
/// NVMe "data unit" = 1000 × 512 bytes.
const NVME_DATA_UNIT: u64 = 512_000;

fn u64_at(v: &Value, path: &[&str]) -> Option<u64> {
    path.iter().try_fold(v, |v, k| v.get(*k))?.as_u64()
}

fn ata_raw(v: &Value, id: u64) -> Option<u64> {
    v.get("ata_smart_attributes")?
        .get("table")?
        .as_array()?
        .iter()
        .find(|a| a.get("id").and_then(Value::as_u64) == Some(id))?
        .get("raw")?
        .get("value")?
        .as_u64()
}

pub fn parse_smartctl(json: &[u8]) -> Result<DiskHealth, SmartError> {
    if json.len() > MAX_SMART_JSON {
        return Err(SmartError::NotJson);
    }
    let v: Value = serde_json::from_slice(json).map_err(|_| SmartError::NotJson)?;
    if !v.is_object() {
        return Err(SmartError::NotJson);
    }
    // The helper reports its own problems (smartctl missing, timeout) in this key.
    if let Some(msg) = v.get("rigscope_error").and_then(Value::as_str) {
        return Err(SmartError::DeviceError(msg.chars().take(300).collect()));
    }
    let exit = u64_at(&v, &["smartctl", "exit_status"]).unwrap_or(0);
    if exit & FATAL_EXIT_BITS != 0 {
        let msg = v
            .get("smartctl")
            .and_then(|s| s.get("messages"))
            .and_then(Value::as_array)
            .and_then(|m| {
                m.iter()
                    .find_map(|m| m.get("string").and_then(Value::as_str))
            })
            .unwrap_or("smartctl could not read the drive")
            .chars()
            .take(300)
            .collect();
        return Err(SmartError::DeviceError(msg));
    }
    let nvme = |k: &str| u64_at(&v, &["nvme_smart_health_information_log", k]);
    let small = |n: Option<u64>| n.and_then(|n| u32::try_from(n).ok());
    Ok(DiskHealth {
        passed: v
            .get("smart_status")
            .and_then(|s| s.get("passed"))
            .and_then(Value::as_bool),
        temperature_c: v
            .get("temperature")
            .and_then(|t| t.get("current"))
            .and_then(Value::as_i64)
            .and_then(|t| i32::try_from(t).ok()),
        power_on_hours: u64_at(&v, &["power_on_time", "hours"]),
        power_cycles: u64_at(&v, &["power_cycle_count"]),
        percent_used: small(nvme("percentage_used")),
        available_spare: small(nvme("available_spare")),
        media_errors: nvme("media_errors"),
        written_bytes: nvme("data_units_written").and_then(|u| u.checked_mul(NVME_DATA_UNIT)),
        reallocated_sectors: ata_raw(&v, 5),
        pending_sectors: ata_raw(&v, 197),
        uncorrectable_sectors: ata_raw(&v, 198),
    })
}
