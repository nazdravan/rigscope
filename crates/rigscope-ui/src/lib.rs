//! egui front-end for rigscope. Widgets only queue commands for `App::run`; they never touch
//! the snapshot themselves.

mod cpu_panel;
pub mod fmt;
mod gpu_panel;
pub mod i18n;
mod memory_panel;
mod net_panel;
mod power_panel;
mod sensors_panel;
mod shell;
mod storage_panel;
mod system_panel;
pub mod theme;
mod widgets;

pub use shell::{ParamsPicker, RigscopeUi, draw, draw_with, params_for};

/// Commands requested by widgets during a frame, run after drawing.
#[derive(Default)]
pub(crate) struct Actions(pub Vec<(String, serde_json::Value)>);

impl Actions {
    pub fn push(&mut self, id: &str, params: serde_json::Value) {
        self.0.push((id.to_string(), params));
    }
}

/// Markdown table of the keyboard shortcuts, generated from the command registry.
pub fn shortcut_table(lang: rigscope_engine::Language) -> String {
    let (key, action) = match lang {
        rigscope_engine::Language::En => ("Shortcut", "Action"),
        rigscope_engine::Language::Ro => ("Scurtătură", "Acțiune"),
    };
    let mut rows: Vec<(String, String)> = rigscope_engine::builtin_commands()
        .iter()
        .filter_map(|c| {
            Some((
                c.shortcut?.to_string(),
                i18n::command_label(lang, c.label_key),
            ))
        })
        .collect();
    rows.sort();
    let mut out = format!("| {key} | {action} |\n|---|---|\n");
    for (k, a) in rows {
        out.push_str(&format!("| `{k}` | {a} |\n"));
    }
    out
}
