//! egui front-end for rigscope. Widgets only queue commands for `App::run`; they never touch
//! the snapshot themselves.

mod cpu_panel;
pub mod fmt;
pub mod i18n;
mod memory_panel;
mod shell;
mod storage_panel;
mod system_panel;
mod widgets;

pub use shell::{draw, draw_with, params_for, ParamsPicker, RigscopeUi};

/// Commands requested by widgets during a frame, run after drawing.
#[derive(Default)]
pub(crate) struct Actions(pub Vec<(String, serde_json::Value)>);

impl Actions {
    pub fn push(&mut self, id: &str, params: serde_json::Value) {
        self.0.push((id.to_string(), params));
    }
}
