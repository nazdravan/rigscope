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
            id: "view.language",
            label_key: "cmd.language",
            menu: "",
            shortcut: None,
            params_doc: r#"{"id": "en" | "ro"}"#,
            enabled: always,
            run: language,
        },
        CommandSpec {
            id: "live.toggle",
            label_key: "cmd.live",
            menu: "View",
            shortcut: Some("Ctrl+L"),
            params_doc: "{}",
            enabled: |s| match &s.opened {
                Some(path) => Err(format!(
                    "showing {path}; refresh to return to this computer"
                )),
                None => Ok(()),
            },
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

fn language(s: &mut Session, params: &Value) -> Result<Value, EngineError> {
    let id = params
        .get("id")
        .cloned()
        .ok_or_else(|| EngineError::BadParams("missing \"id\"".into()))?;
    s.language =
        serde_json::from_value(id).map_err(|e| EngineError::BadParams(format!("id: {e}")))?;
    Ok(json!(null))
}

fn tick(s: &mut Session, _: &Value) -> Result<Value, EngineError> {
    // An opened snapshot is another moment or machine: never mix in local frequencies.
    if s.live && s.opened.is_none() {
        let c = s.env.collector();
        c.refresh_live(&mut s.snapshot.cpu);
        c.refresh_sensors(&mut s.snapshot.sensors);
    }
    Ok(json!(null))
}
