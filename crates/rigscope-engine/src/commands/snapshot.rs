use super::always;
use crate::{CommandSpec, EngineError, Session};
use rigscope_model::{DmiSource, Field, Platform, Snapshot};
use serde_json::{json, Value};

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            id: "snapshot.refresh",
            label_key: "cmd.refresh",
            menu: "View",
            shortcut: Some("F5"),
            params_doc: "{}",
            enabled: always,
            run: refresh,
        },
        CommandSpec {
            id: "snapshot.undo",
            label_key: "cmd.undo",
            menu: "Edit",
            shortcut: Some("Ctrl+Z"),
            params_doc: "{}",
            enabled: |s| {
                if s.history.is_empty() {
                    Err("nothing to undo".into())
                } else {
                    Ok(())
                }
            },
            run: undo,
        },
        CommandSpec {
            id: "snapshot.elevate",
            label_key: "cmd.elevate",
            menu: "File",
            shortcut: None,
            params_doc:
                "{} — runs the privileged helper (pkexec) to read the SMBIOS table and SMART",
            enabled: |s| {
                if !s.env.can_elevate() {
                    Err(if s.snapshot.platform == Platform::Windows {
                        "on Windows rigscope runs as administrator from the start; there is no separate elevated read".into()
                    } else {
                        "elevated reading needs Linux with pkexec".into()
                    })
                } else if s.elevating {
                    Err("already waiting for authentication".into())
                } else if s.snapshot.dmi_source == DmiSource::SysfsIdOnly || needs_root_for_smart(s)
                {
                    Ok(())
                } else if matches!(
                    s.snapshot.dmi_source,
                    DmiSource::FullTable | DmiSource::Firmware
                ) {
                    Err("the full DMI table is already loaded".into())
                } else {
                    Err("this machine has no DMI table".into())
                }
            },
            run: elevate,
        },
        CommandSpec {
            id: "snapshot.elevate_poll",
            label_key: "cmd.elevate_poll",
            menu: "",
            shortcut: None,
            params_doc: "{} — collects the helper result; the UI calls it while `elevating`",
            enabled: always,
            run: elevate_poll,
        },
    ]
}

fn elevate(s: &mut Session, _: &Value) -> Result<Value, EngineError> {
    s.env.start_elevate()?;
    s.elevating = true;
    s.status = Some("waiting for authentication…".into());
    Ok(json!({ "pending": true }))
}

fn elevate_poll(s: &mut Session, _: &Value) -> Result<Value, EngineError> {
    if !s.elevating {
        return Ok(json!({ "pending": false }));
    }
    let Some(result) = s.env.poll_elevate() else {
        return Ok(json!({ "pending": true }));
    };
    s.elevating = false;
    let raw = result?;
    let dump =
        rigscope_model::decode_dmi_dump(&raw).map_err(|e| EngineError::Probe(e.to_string()))?;
    // A dump is useful if its DMI table decodes or it carries SMART data.
    if let Err(e) = rigscope_smbios::decode(&dump.entry, &dump.table) {
        if dump.smart.is_empty() {
            return Err(EngineError::Probe(e.to_string()));
        }
    }
    let now = s.env.now_unix();
    let next = s.env.collector().collect(Some(&dump), now);
    s.dmi_dump = Some(dump);
    s.replace_snapshot(next);
    s.status = elevation_status(&s.snapshot);
    Ok(json!(null))
}

fn refresh(s: &mut Session, _: &Value) -> Result<Value, EngineError> {
    let now = s.env.now_unix();
    let next = s.env.collector().collect(s.dmi_dump.as_ref(), now);
    s.replace_snapshot(next);
    s.opened = None;
    s.status = elevation_status(&s.snapshot);
    Ok(json!(null))
}

fn undo(s: &mut Session, _: &Value) -> Result<Value, EngineError> {
    if let Some(prev) = s.history.pop() {
        s.snapshot = std::sync::Arc::unwrap_or_clone(prev);
        if s.snapshot.dmi_source != DmiSource::FullTable {
            s.dmi_dump = None;
        }
    }
    Ok(json!(null))
}

/// On Windows a disk that needs elevation means rigscope runs without administrator rights.
pub(crate) fn elevation_status(s: &Snapshot) -> Option<String> {
    (s.platform == Platform::Windows
        && s.storage
            .disks
            .iter()
            .any(|d| d.health == Field::NeedsElevation))
    .then(|| "running without administrator rights".to_string())
}

fn needs_root_for_smart(s: &Session) -> bool {
    s.snapshot
        .storage
        .disks
        .iter()
        .any(|d| d.health == Field::NeedsElevation)
}
