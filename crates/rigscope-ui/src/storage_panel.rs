use crate::fmt;
use crate::i18n::{tr, Key};
use crate::widgets::{card, kv_grid};
use rigscope_engine::App;

pub fn show(ui: &mut egui::Ui, app: &App) {
    let lang = app.session.language;
    let disks = &app.session.snapshot.storage.disks;
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
                        fmt::field(lang, &d.size_bytes, |v| {
                            rigscope_model::format_disk_size(*v)
                        }),
                    ),
                    (
                        tr(lang, Key::DiskKind),
                        fmt::field(lang, &d.kind, |k| k.label().to_string()),
                    ),
                    (
                        tr(lang, Key::Interface),
                        fmt::field(lang, &d.interface, |i| i.label().to_string()),
                    ),
                    (tr(lang, Key::Manufacturer), fmt::text(lang, &d.vendor)),
                    (tr(lang, Key::Model), fmt::text(lang, &d.model)),
                    (tr(lang, Key::Serial), fmt::text(lang, &d.serial)),
                    (tr(lang, Key::Firmware), fmt::text(lang, &d.firmware)),
                    (tr(lang, Key::Removable), tr(lang, removable).to_string()),
                ],
            );
        });
    }
}
