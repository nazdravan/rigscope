//! Window layout, menus, shortcuts and the eframe entry point.

use crate::i18n::{Key, command_label, tr};
use crate::{
    Actions, cpu_panel, gpu_panel, memory_panel, net_panel, power_panel, sensors_panel,
    storage_panel, system_panel,
};
use rigscope_engine::{App, Language, SectionId};
use serde_json::json;
use std::time::Duration;

/// Seconds between live CPU-frequency updates.
pub const LIVE_INTERVAL: Duration = Duration::from_secs(1);

/// How often the UI checks whether the privileged helper finished.
pub const ELEVATE_POLL_INTERVAL: Duration = Duration::from_millis(200);

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
pub fn params_for(id: &str) -> Option<serde_json::Value> {
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
    draw_with(ui, app, &params_for);
}

/// [`draw`], returning whether Help → "Check for updates" was clicked.
pub fn draw_updates(ui: &mut egui::Ui, app: &mut App) -> bool {
    draw_picked(ui, app, &params_for)
}

/// Asks for a command's parameters (file dialogs); `None` = cancelled.
pub type ParamsPicker<'a> = &'a dyn Fn(&str) -> Option<serde_json::Value>;

/// [`draw`] with an injectable parameter picker (tests replace the file dialogs).
pub fn draw_with(ui: &mut egui::Ui, app: &mut App, pick: ParamsPicker<'_>) {
    draw_picked(ui, app, pick);
}

fn draw_picked(ui: &mut egui::Ui, app: &mut App, pick: ParamsPicker<'_>) -> bool {
    let mut actions = Actions::default();
    let mut check_updates = false;
    let lang = app.session.language;

    // The privileged helper runs in the background; collect its result without blocking.
    if app.session.elevating {
        actions.push("snapshot.elevate_poll", json!({}));
        ui.ctx().request_repaint_after(ELEVATE_POLL_INTERVAL);
    }

    for spec in app.specs() {
        if let Some(sc) = spec.shortcut.and_then(parse_shortcut) {
            if ui.input_mut(|i| i.consume_shortcut(&sc)) && app.is_enabled(spec.id).is_ok() {
                if let Some(params) = pick(spec.id) {
                    actions.push(spec.id, params);
                }
            }
        }
    }

    egui::Panel::top("menu").show(ui, |ui| {
        menu_bar(ui, app, &mut actions, lang, pick, &mut check_updates)
    });
    egui::Panel::bottom("status").show(ui, |ui| status_bar(ui, app, &mut actions, lang));
    // The status line is overwritten by the next action; this banner stays while the data is
    // not this machine's.
    if let Some(path) = &app.session.opened {
        egui::Panel::top("opened").show(ui, |ui| {
            ui.label(egui::RichText::new(crate::i18n::opened_snapshot(lang, path)).strong());
        });
    }
    egui::Panel::left("sections")
        .resizable(false)
        .exact_size(170.0)
        .show(ui, |ui| {
            ui.add_space(8.0);
            for (id, key) in [
                (SectionId::Memory, Key::Memory),
                (SectionId::Cpu, Key::Processor),
                (SectionId::Storage, Key::Storage),
                (SectionId::Gpu, Key::Graphics),
                (SectionId::Network, Key::Network),
                (SectionId::Battery, Key::Battery),
                (SectionId::Sensors, Key::Sensors),
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
                SectionId::Storage => storage_panel::show(ui, app, &mut actions),
                SectionId::Gpu => gpu_panel::show(ui, app),
                SectionId::Network => net_panel::show(ui, app),
                SectionId::Battery => power_panel::show(ui, app),
                SectionId::Sensors => sensors_panel::show(ui, app),
                SectionId::System => system_panel::show(ui, app),
            });
    });

    for (id, params) in actions.0 {
        // Errors are already recorded in `session.status` by the engine.
        let _ = app.run(&id, params);
    }
    check_updates
}

fn menu_bar(
    ui: &mut egui::Ui,
    app: &App,
    actions: &mut Actions,
    lang: Language,
    pick: ParamsPicker<'_>,
    check_updates: &mut bool,
) {
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
                        ui.close();
                        if let Some(params) = pick(spec.id) {
                            actions.push(spec.id, params);
                        }
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
        // Not an engine command: the updater lives in `RigscopeUi`, outside the document.
        ui.menu_button(tr(lang, Key::Help), |ui| {
            if ui
                .button(app_update::texts(update_lang(lang)).check_button)
                .clicked()
            {
                *check_updates = true;
                ui.close();
            }
        });
    });
}

/// The updater's language for the UI language.
fn update_lang(lang: Language) -> app_update::Lang {
    match lang {
        Language::En => app_update::Lang::En,
        Language::Ro => app_update::Lang::Ro,
    }
}

/// What the updater needs to find, download and install a new release from GitHub.
pub const RIGSCOPE: app_update::AppInfo = app_update::AppInfo {
    name: "rigscope",
    display_name: "rigscope",
    repo: "nazdravan/rigscope",
    version: env!("CARGO_PKG_VERSION"),
    windows_exe: "rigscope.exe",
    // There is no .msi: on Windows only the portable zip updates in place.
    msi_registry_key: r"Software\nazdravan\rigscope",
    deb_package: "rigscope",
    mac_bundle: "rigscope.app",
};

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
            let text = crate::i18n::status(lang, status);
            // Long messages (helper errors) are cut at the window edge; the tooltip has them all.
            ui.add(egui::Label::new(&text).truncate())
                .on_hover_text(&text);
        }
    });
}

/// eframe application: owns the engine [`App`] and drives live updates.
pub struct RigscopeUi {
    pub app: App,
    /// Shown as a banner when the data is a sample, not this machine.
    pub demo: bool,
    last_tick: Option<std::time::Instant>,
    updater: app_update::Updater,
}

impl RigscopeUi {
    /// `storage` (eframe persistence) restores the language chosen last time.
    pub fn new(mut app: App, demo: bool, storage: Option<&dyn eframe::Storage>) -> Self {
        if let Some(lang) = storage.and_then(|s| s.get_string(LANGUAGE_KEY)) {
            // Unknown stored values are rejected by the command and simply ignored.
            let _ = app.run("view.language", json!({ "id": lang }));
        }
        RigscopeUi {
            app,
            demo,
            last_tick: None,
            updater: app_update::Updater::new(RIGSCOPE),
        }
    }
}

/// Persistence key for the UI language.
const LANGUAGE_KEY: &str = "language";

impl eframe::App for RigscopeUi {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let id = match self.app.session.language {
            Language::En => "en",
            Language::Ro => "ro",
        };
        storage.set_string(LANGUAGE_KEY, id.to_string());
    }

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
        let ctx = ui.ctx().clone();
        if draw_updates(ui, &mut self.app) {
            self.updater.check(&ctx);
        }
        let lang = update_lang(self.app.session.language);
        if let Some(app_update::UpdaterEvent::RestartRequested) = self.updater.show(&ctx, lang) {
            match self.updater.restart() {
                Ok(()) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                Err(e) => {
                    let text = app_update::texts(lang);
                    self.app.session.status = Some(text.error(&e).unwrap_or_else(|| e.to_string()));
                }
            }
        }
    }
}
