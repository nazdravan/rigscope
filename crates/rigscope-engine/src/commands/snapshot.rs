use super::always;
use crate::{CommandSpec, EngineError, Session};
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
    ]
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
