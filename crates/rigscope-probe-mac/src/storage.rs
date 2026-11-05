//! Disks from `SPSerialATADataType` and `SPNVMeDataType`. macOS gives the drive's own SMART
//! verdict ("Verified" / "Failing"), not the attributes, so only the verdict is shown.

use crate::MacSource;
use rigscope_model::{Disk, DiskHealth, DiskKind, Field, Interface, StorageInfo, clean_text};
use serde_json::Value;

/// Most disks read from the profiler output.
pub const MAX_DISKS: usize = 64;

/// One disk as the profiler lists it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiskEntry {
    pub bsd_name: Option<String>,
    pub model: Option<String>,
    pub revision: Option<String>,
    pub serial: Option<String>,
    pub size_bytes: Option<u64>,
    /// `Verified`, `Failing`, `Not Supported`, as printed.
    pub smart: Option<String>,
    /// `Solid State` / `Rotational`, SATA only.
    pub medium: Option<String>,
    pub removable: bool,
}

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key)?.as_str().and_then(clean_text)
}

/// Disks under `root[data_type][*]._items[*]`.
pub fn parse_disks(json: &str, data_type: &str) -> Vec<DiskEntry> {
    let Ok(root) = serde_json::from_str::<Value>(json) else {
        return Vec::new();
    };
    let Some(groups) = root.get(data_type).and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for group in groups {
        let Some(items) = group.get("_items").and_then(Value::as_array) else {
            continue;
        };
        for item in items {
            if out.len() >= MAX_DISKS {
                return out;
            }
            out.push(DiskEntry {
                bsd_name: text(item, "bsd_name"),
                model: text(item, "device_model").or_else(|| text(item, "_name")),
                revision: text(item, "device_revision"),
                serial: text(item, "device_serial"),
                size_bytes: item.get("size_in_bytes").and_then(Value::as_u64),
                smart: text(item, "smart_status"),
                medium: text(item, "spsata_medium_type"),
                removable: text(item, "removable_media").as_deref() == Some("yes")
                    || text(item, "detachable_drive").as_deref() == Some("yes"),
            });
        }
    }
    out
}

const NO_SMART: &str = "this drive does not report SMART to macOS";

fn health(smart: Option<&str>) -> Field<DiskHealth> {
    match smart.map(str::to_ascii_lowercase).as_deref() {
        Some("verified") => Field::Known(DiskHealth {
            passed: Some(true),
            ..DiskHealth::default()
        }),
        Some("failing") => Field::Known(DiskHealth {
            passed: Some(false),
            ..DiskHealth::default()
        }),
        Some(_) => Field::Unavailable(NO_SMART.into()),
        None => Field::Unknown,
    }
}

fn disk(e: &DiskEntry, nvme: bool) -> Disk {
    let kind = if nvme {
        Field::Known(DiskKind::NvmeSsd)
    } else {
        match e.medium.as_deref() {
            Some("Solid State") => Field::Known(DiskKind::Ssd),
            Some("Rotational") => Field::Known(DiskKind::Hdd),
            _ => Field::Unknown,
        }
    };
    Disk {
        name: e.bsd_name.clone().unwrap_or_else(|| "disk".into()),
        model: Field::from_option(e.model.clone()),
        vendor: Field::Unknown,
        serial: Field::from_option(e.serial.clone()),
        firmware: Field::from_option(e.revision.clone()),
        size_bytes: Field::from_option(e.size_bytes),
        kind,
        interface: Field::Known(if nvme {
            Interface::Nvme
        } else {
            Interface::Sata
        }),
        removable: e.removable,
        health: health(e.smart.as_deref()),
    }
}

pub(crate) fn read_storage(src: &dyn MacSource) -> StorageInfo {
    let mut disks = Vec::new();
    for (data_type, nvme) in [("SPNVMeDataType", true), ("SPSerialATADataType", false)] {
        if let Ok(json) = src.profiler(data_type) {
            disks.extend(parse_disks(&json, data_type).iter().map(|e| disk(e, nvme)));
        }
    }
    disks.truncate(MAX_DISKS);
    StorageInfo { disks }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SATA: &str = r#"{"SPSerialATADataType":[{"_items":[{"_name":"TOSHIBA THNSNJ1T02CSX","bsd_name":"disk0","device_model":"TOSHIBA THNSNJ1T02CSX   ","device_revision":"JXPS4101","device_serial":"157S102STGUW","removable_media":"no","detachable_drive":"no","size_in_bytes":1024209543168,"smart_status":"Verified","spsata_medium_type":"Solid State"}]}]}"#;

    #[test]
    fn a_sata_ssd() {
        let e = parse_disks(SATA, "SPSerialATADataType");
        assert_eq!(e.len(), 1);
        let d = disk(&e[0], false);
        assert_eq!(d.name, "disk0");
        assert_eq!(d.model, Field::Known("TOSHIBA THNSNJ1T02CSX".into()));
        assert_eq!(d.kind, Field::Known(DiskKind::Ssd));
        assert_eq!(d.interface, Field::Known(Interface::Sata));
        assert_eq!(d.size_bytes, Field::Known(1_024_209_543_168));
        assert_eq!(
            d.health,
            Field::Known(DiskHealth {
                passed: Some(true),
                ..DiskHealth::default()
            })
        );
        assert!(!d.removable);
    }

    #[test]
    fn smart_verdicts() {
        assert_eq!(health(Some("Failing")).known().unwrap().passed, Some(false));
        assert!(matches!(
            health(Some("Not Supported")),
            Field::Unavailable(_)
        ));
        assert_eq!(health(None), Field::Unknown);
    }

    #[test]
    fn an_nvme_drive_is_an_nvme_ssd() {
        let j = r#"{"SPNVMeDataType":[{"_items":[{"_name":"APPLE SSD AP0512M","bsd_name":"disk0","size_in_bytes":500277792768,"smart_status":"Verified"}]}]}"#;
        let e = parse_disks(j, "SPNVMeDataType");
        let d = disk(&e[0], true);
        assert_eq!(d.kind, Field::Known(DiskKind::NvmeSsd));
        assert_eq!(d.interface, Field::Known(Interface::Nvme));
    }

    #[test]
    fn empty_and_garbage_give_no_disks() {
        assert!(parse_disks(r#"{"SPNVMeDataType":[]}"#, "SPNVMeDataType").is_empty());
        assert!(parse_disks("nope", "SPNVMeDataType").is_empty());
        assert!(parse_disks("{}", "SPNVMeDataType").is_empty());
    }

    #[test]
    fn disks_are_capped() {
        let items: Vec<String> = (0..200)
            .map(|i| format!(r#"{{"bsd_name":"disk{i}"}}"#))
            .collect();
        let j = format!(r#"{{"X":[{{"_items":[{}]}}]}}"#, items.join(","));
        assert_eq!(parse_disks(&j, "X").len(), MAX_DISKS);
    }
}
