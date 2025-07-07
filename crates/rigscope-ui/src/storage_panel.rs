use crate::fmt;
use crate::i18n::{tr, Key};
use crate::widgets::{card, command_button, kv_grid};
use crate::Actions;
use rigscope_engine::App;
use rigscope_model::{Field, Platform};
use serde_json::json;

pub fn show(ui: &mut egui::Ui, app: &App, actions: &mut Actions) {
    let lang = app.session.language;
    let cx = crate::fmt::Ctx::of(app);
    let disks = &app.session.snapshot.storage.disks;
    if disks.iter().any(|d| d.health == Field::NeedsElevation) {
        let windows = app.session.snapshot.platform == Platform::Windows;
        let key = if windows {
            Key::SmartNeedsAdmin
        } else {
            Key::SmartNeedsRoot
        };
        card(ui, tr(lang, Key::DiskHealth), |ui| {
            ui.label(tr(lang, key));
            // Windows has no elevated helper: the user restarts rigscope as administrator.
            if !windows {
                command_button(
                    ui,
                    app,
                    actions,
                    "snapshot.elevate",
                    tr(lang, Key::ReadElevated),
                    json!({}),
                );
            }
        });
    }
    if disks.is_empty() {
        card(ui, tr(lang, Key::Storage), |ui| {
            ui.label(tr(lang, Key::NoDisks));
        });
        return;
    }
    for d in disks {
        let title = match d.model.known() {
            Some(model) => format!("{model}  ({})", d.name),
            None => d.name.clone(),
        };
        card(ui, &title, |ui| {
            let removable = if d.removable { Key::Yes } else { Key::No };
            kv_grid(
                ui,
                &format!("disk-{}", d.name),
                &[
                    (
                        tr(lang, Key::DiskSize),
                        fmt::field(cx, &d.size_bytes, |v| rigscope_model::format_disk_size(*v)),
                    ),
                    (
                        tr(lang, Key::DiskKind),
                        fmt::field(cx, &d.kind, |k| k.label().to_string()),
                    ),
                    (
                        tr(lang, Key::Interface),
                        fmt::field(cx, &d.interface, |i| i.label().to_string()),
                    ),
                    (tr(lang, Key::Manufacturer), fmt::text(cx, &d.vendor)),
                    (tr(lang, Key::Model), fmt::text(cx, &d.model)),
                    (tr(lang, Key::Serial), fmt::text(cx, &d.serial)),
                    (tr(lang, Key::Firmware), fmt::text(cx, &d.firmware)),
                    (tr(lang, Key::Removable), tr(lang, removable).to_string()),
                    (
                        tr(lang, Key::DiskHealth),
                        fmt::field(cx, &d.health, |h| crate::i18n::health_line(lang, h)),
                    ),
                ],
            );
            if let Field::Known(h) = &d.health {
                for w in h.warnings() {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        format!(
                            "{}: {}",
                            tr(lang, Key::Warning),
                            crate::i18n::warning(lang, w)
                        ),
                    );
                }
            }
        });
    }
}
