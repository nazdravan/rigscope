//! Window layout, menus, shortcuts and the eframe entry point.

use crate::i18n::{command_label, tr, Key};
use crate::{cpu_panel, memory_panel, system_panel, Actions};
use rigscope_engine::{App, Language, SectionId};
use serde_json::json;
use std::time::Duration;

/// Seconds between live CPU-frequency updates.
pub const LIVE_INTERVAL: Duration = Duration::from_secs(1);

/// Parses a spec shortcut such as `"Ctrl+Shift+C"` or `"F5"`.
pub fn parse_shortcut(s: &str) -> Option<egui::KeyboardShortcut> {
    let mut mods = egui::Modifiers::NONE;
    let mut key = None;
    for part in s.split('+') {
        match part {
            "Ctrl" => mods |= egui::Modifiers::COMMAND,
            "Shift" => mods |= egui::Modifiers::SHIFT,
            "Alt" => mods |= egui::Modifiers::ALT,
            k => key = egui::Key::from_name(k),
        }
    }
    key.map(|k| egui::KeyboardShortcut::new(mods, k))
}

/// Parameters for a command triggered from a menu or shortcut. Commands that take a file path
/// ask for it with a native dialog; `None` means the user cancelled.
fn params_for(id: &str) -> Option<serde_json::Value> {
    let json_filter = |d: rfd::FileDialog| d.add_filter("rigscope snapshot", &["json"]);
    match id {
        "report.export_text" => rfd::FileDialog::new()
            .add_filter("Text", &["txt"])
            .set_file_name("rigscope-report.txt")
            .save_file()
            .map(|p| json!({ "path": p.display().to_string() })),
        "snapshot.save" => json_filter(rfd::FileDialog::new())
            .set_file_name("rigscope-snapshot.json")
            .save_file()
            .map(|p| json!({ "path": p.display().to_string() })),
        "snapshot.open" => json_filter(rfd::FileDialog::new())
            .pick_file()
            .map(|p| json!({ "path": p.display().to_string() })),
        _ => Some(json!({})),
    }
}

/// Draws the whole window into `ui` and runs the commands the user triggered.
pub fn draw(ui: &mut egui::Ui, app: &mut App) {
    let mut actions = Actions::default();
    let lang = app.session.language;

    for spec in app.specs() {
        if let Some(sc) = spec.shortcut.and_then(parse_shortcut) {
            if ui.input_mut(|i| i.consume_shortcut(&sc)) && app.is_enabled(spec.id).is_ok() {
                if let Some(params) = params_for(spec.id) {
                    actions.push(spec.id, params);
                }
            }
        }
    }

    egui::Panel::top("menu").show(ui, |ui| menu_bar(ui, app, &mut actions, lang));
    egui::Panel::bottom("status").show(ui, |ui| status_bar(ui, app, &mut actions, lang));
    egui::Panel::left("sections")
        .resizable(false)
        .exact_size(170.0)
        .show(ui, |ui| {
            ui.add_space(8.0);
            for (id, key) in [
                (SectionId::Memory, Key::Memory),
                (SectionId::Cpu, Key::Processor),
                (SectionId::System, Key::System),
            ] {
                let selected = app.session.section == id;
                let resp = ui.add_sized(
                    [ui.available_width(), 28.0],
                    egui::Button::selectable(selected, tr(lang, key)),
                );
                if resp.clicked() && !selected {
                    actions.push("view.section", json!({ "id": id }));
                }
            }
        });
    egui::CentralPanel::default().show(ui, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| match app.session.section {
                SectionId::Memory => memory_panel::show(ui, app, &mut actions),
                SectionId::Cpu => cpu_panel::show(ui, app),
                SectionId::System => system_panel::show(ui, app),
            });
    });

    for (id, params) in actions.0 {
        // Errors are already recorded in `session.status` by the engine.
        let _ = app.run(&id, params);
    }
}

fn menu_bar(ui: &mut egui::Ui, app: &App, actions: &mut Actions, lang: Language) {
    egui::MenuBar::new().ui(ui, |ui| {
        for (menu, key) in [
            ("File", Key::File),
            ("Edit", Key::Edit),
            ("View", Key::View),
        ] {
            ui.menu_button(tr(lang, key), |ui| {
                for spec in app.specs().iter().filter(|s| s.menu == menu) {
                    let mut button = egui::Button::new(command_label(lang, spec.label_key));
                    if let Some(sc) = spec.shortcut {
                        button = button.shortcut_text(sc);
                    }
                    if ui
                        .add_enabled(app.is_enabled(spec.id).is_ok(), button)
                        .clicked()
                    {
                        actions.push(spec.id, json!({}));
                        ui.close();
                    }
                }
                if menu == "View" {
                    ui.separator();
                    ui.menu_button(tr(lang, Key::LanguageMenu), |ui| {
                        for (l, name) in [(Language::En, "English"), (Language::Ro, "Română")] {
                            if ui.selectable_label(lang == l, name).clicked() {
                                actions.push("view.language", json!({ "id": l }));
                                ui.close();
                            }
                        }
                    });
                }
            });
        }
    });
}

fn status_bar(ui: &mut egui::Ui, app: &App, actions: &mut Actions, lang: Language) {
    ui.horizontal(|ui| {
        let mut live = app.session.live;
        if ui.checkbox(&mut live, tr(lang, Key::Live)).changed() {
            actions.push("live.toggle", json!({}));
        }
        if ui.button(tr(lang, Key::Refresh)).clicked() {
            actions.push("snapshot.refresh", json!({}));
        }
        if let Some(status) = &app.session.status {
            ui.separator();
            ui.label(status.as_str());
        }
    });
}

/// eframe application: owns the engine [`App`] and drives live updates.
pub struct RigscopeUi {
    pub app: App,
    /// Shown as a banner when the data is a sample, not this machine.
    pub demo: bool,
    last_tick: Option<std::time::Instant>,
}

impl RigscopeUi {
    pub fn new(app: App, demo: bool) -> Self {
        RigscopeUi {
            app,
            demo,
            last_tick: None,
        }
    }
}

impl eframe::App for RigscopeUi {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.app.session.live {
            let now = std::time::Instant::now();
            if self
                .last_tick
                .is_none_or(|t| now.duration_since(t) >= LIVE_INTERVAL)
            {
                self.last_tick = Some(now);
                let _ = self.app.run("live.tick", json!({}));
            }
            ctx.request_repaint_after(LIVE_INTERVAL);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.demo {
            let lang = self.app.session.language;
            egui::Panel::top("demo").show(ui, |ui| {
                ui.label(egui::RichText::new(tr(lang, Key::DemoMode)).strong());
            });
        }
        draw(ui, &mut self.app);
    }
}
