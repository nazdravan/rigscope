use super::always;
use crate::{CommandSpec, EngineError, Session};
use rigscope_model::{compat_summary, masked, report_text, Snapshot, SCHEMA_VERSION};
use serde_json::{json, Value};

/// Largest snapshot file `snapshot.open` accepts.
pub const MAX_SNAPSHOT_FILE: usize = 4 * 1024 * 1024;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            id: "report.copy_text",
            label_key: "cmd.copy_report",
            menu: "File",
            shortcut: Some("Ctrl+Shift+C"),
            params_doc: r#"{"mask": bool = true} — serial numbers masked unless mask=false"#,
            enabled: always,
            run: copy_text,
        },
        CommandSpec {
            id: "report.export_text",
            label_key: "cmd.export_text",
            menu: "File",
            shortcut: None,
            params_doc: r#"{"path": string, "mask": bool = true}"#,
            enabled: always,
            run: export_text,
        },
        CommandSpec {
            id: "snapshot.save",
            label_key: "cmd.save",
            menu: "File",
            shortcut: Some("Ctrl+S"),
            params_doc: r#"{"path": string, "mask": bool = true} — JSON"#,
            enabled: always,
            run: save,
        },
        CommandSpec {
            id: "snapshot.open",
            label_key: "cmd.open",
            menu: "File",
            shortcut: Some("Ctrl+O"),
            params_doc: r#"{"path": string} — JSON saved by rigscope (schema 1)"#,
            enabled: always,
            run: open,
        },
        CommandSpec {
            id: "memory.copy_compat",
            label_key: "cmd.copy_compat",
            menu: "",
            shortcut: None,
            params_doc: "{} — copies e.g. \"DDR4 SO-DIMM 3200 MT/s 32GB\"",
            enabled: |s| match compat_summary(&s.snapshot.memory) {
                Some(_) => Ok(()),
                None => Err("no memory slot data".into()),
            },
            run: copy_compat,
        },
    ]
}

fn mask_param(params: &Value) -> bool {
    params.get("mask").and_then(Value::as_bool).unwrap_or(true)
}

fn path_param(params: &Value) -> Result<String, EngineError> {
    params
        .get("path")
        .and_then(Value::as_str)
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .ok_or_else(|| EngineError::BadParams("missing \"path\"".into()))
}

fn copy_text(s: &mut Session, params: &Value) -> Result<Value, EngineError> {
    let text = report_text(&s.snapshot, mask_param(params));
    s.env.clipboard_set(&text)?;
    s.status = Some("report copied".into());
    Ok(json!(null))
}

fn export_text(s: &mut Session, params: &Value) -> Result<Value, EngineError> {
    let path = path_param(params)?;
    s.env.write_file(
        &path,
        report_text(&s.snapshot, mask_param(params)).as_bytes(),
    )?;
    s.status = Some(format!("saved {path}"));
    Ok(json!(null))
}

fn save(s: &mut Session, params: &Value) -> Result<Value, EngineError> {
    let path = path_param(params)?;
    let snap = if mask_param(params) {
        masked(&s.snapshot)
    } else {
        s.snapshot.clone()
    };
    let bytes = serde_json::to_vec_pretty(&snap).map_err(|e| EngineError::Io(e.to_string()))?;
    s.env.write_file(&path, &bytes)?;
    s.status = Some(format!("saved {path}"));
    Ok(json!(null))
}

fn open(s: &mut Session, params: &Value) -> Result<Value, EngineError> {
    let path = path_param(params)?;
    let bytes = s.env.read_file(&path)?;
    if bytes.len() > MAX_SNAPSHOT_FILE {
        return Err(EngineError::BadParams(format!("{path}: file too large")));
    }
    let snap: Snapshot = serde_json::from_slice(&bytes)
        .map_err(|e| EngineError::BadParams(format!("{path}: not a rigscope snapshot ({e})")))?;
    if snap.schema != SCHEMA_VERSION {
        return Err(EngineError::BadParams(format!(
            "{path}: unsupported snapshot version {}",
            snap.schema
        )));
    }
    s.replace_snapshot(snap);
    // The opened snapshot describes another moment (or machine): stop live updates and forget
    // this machine's elevated DMI data.
    s.live = false;
    s.dmi_dump = None;
    s.status = Some(format!("opened {path}"));
    Ok(json!(null))
}

fn copy_compat(s: &mut Session, _: &Value) -> Result<Value, EngineError> {
    let line = compat_summary(&s.snapshot.memory)
        .map(|c| c.to_search_text())
        .ok_or_else(|| EngineError::Disabled("no memory slot data".into()))?;
    s.env.clipboard_set(&line)?;
    s.status = Some(format!("copied: {line}"));
    Ok(json!(null))
}
