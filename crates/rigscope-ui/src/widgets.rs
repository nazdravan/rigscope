//! Small shared widgets.

use crate::Actions;
use rigscope_engine::App;

/// A button bound to a command: disabled (with the reason as tooltip) when the command can't run.
pub fn command_button(
    ui: &mut egui::Ui,
    app: &App,
    actions: &mut Actions,
    id: &str,
    label: &str,
    params: serde_json::Value,
) {
    let enabled = app.is_enabled(id);
    let resp = ui.add_enabled(enabled.is_ok(), egui::Button::new(label));
    let resp = match &enabled {
        Err(reason) => resp.on_disabled_hover_text(reason.as_str()),
        Ok(()) => resp,
    };
    if resp.clicked() {
        actions.push(id, params);
    }
}

/// Two-column key/value grid.
pub fn kv_grid(ui: &mut egui::Ui, id: &str, rows: &[(&str, String)]) {
    egui::Grid::new(id)
        .num_columns(2)
        .striped(true)
        .spacing([24.0, 6.0])
        .show(ui, |ui| {
            for (k, v) in rows {
                ui.label(egui::RichText::new(*k).strong());
                ui.label(v.as_str());
                ui.end_row();
            }
        });
}

/// A framed group with a heading.
pub fn card(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style())
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.heading(title);
            ui.add_space(6.0);
            add(ui);
        });
    ui.add_space(10.0);
}
