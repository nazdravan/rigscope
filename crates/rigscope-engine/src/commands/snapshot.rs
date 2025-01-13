use super::always;
use crate::{CommandSpec, EngineError, Session};
use rigscope_model::DmiSource;
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
            params_doc: "{} — runs the privileged helper (pkexec) to read the SMBIOS table",
            enabled: |s| {
                if !s.env.can_elevate() {
                    Err("elevated reading needs Linux with pkexec".into())
                } else if s.elevating {
                    Err("already waiting for authentication".into())
                } else if s.snapshot.dmi_source == DmiSource::FullTable {
                    Err("the full DMI table is already loaded".into())
                } else {
                    Ok(())
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
        rigscope_probe::decode_dmi_dump(&raw).map_err(|e| EngineError::Probe(e.to_string()))?;
    rigscope_smbios::decode(&dump.entry, &dump.table)
        .map_err(|e| EngineError::Probe(e.to_string()))?;
    let next = rigscope_probe::build_snapshot(s.env.source(), Some(&dump), s.env.now_unix());
    s.dmi_dump = Some(dump);
    s.replace_snapshot(next);
    s.status = None;
    Ok(json!(null))
}

fn refresh(s: &mut Session, _: &Value) -> Result<Value, EngineError> {
    let next =
        rigscope_probe::build_snapshot(s.env.source(), s.dmi_dump.as_ref(), s.env.now_unix());
    s.replace_snapshot(next);
    s.status = None;
    Ok(json!(null))
}

fn undo(s: &mut Session, _: &Value) -> Result<Value, EngineError> {
    if let Some(prev) = s.history.pop() {
        s.snapshot = prev;
    }
    Ok(json!(null))
}
