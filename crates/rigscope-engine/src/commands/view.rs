use super::always;
use crate::{CommandSpec, EngineError, SectionId, Session};
use serde_json::{json, Value};

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            id: "view.section",
            label_key: "cmd.section",
            menu: "",
            shortcut: None,
            params_doc: r#"{"id": "memory" | "cpu" | "system"}"#,
            enabled: always,
            run: section,
        },
        CommandSpec {
            id: "live.toggle",
            label_key: "cmd.live",
            menu: "View",
            shortcut: Some("Ctrl+L"),
            params_doc: "{}",
            enabled: always,
            run: |s, _| {
                s.live = !s.live;
                Ok(json!({ "live": s.live }))
            },
        },
        CommandSpec {
            id: "live.tick",
            label_key: "cmd.tick",
            menu: "",
            shortcut: None,
            params_doc: "{} — re-reads current CPU frequencies when live updates are on",
            enabled: always,
            run: tick,
        },
    ]
}

fn section(s: &mut Session, params: &Value) -> Result<Value, EngineError> {
    let id = params
        .get("id")
        .cloned()
        .ok_or_else(|| EngineError::BadParams("missing \"id\"".into()))?;
    let section: SectionId =
        serde_json::from_value(id).map_err(|e| EngineError::BadParams(format!("id: {e}")))?;
    s.section = section;
    Ok(json!(null))
}

fn tick(s: &mut Session, _: &Value) -> Result<Value, EngineError> {
    if s.live {
        rigscope_probe::refresh_live(s.env.source(), &mut s.snapshot.cpu);
    }
    Ok(json!(null))
}
