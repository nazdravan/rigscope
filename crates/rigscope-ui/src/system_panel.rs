use crate::fmt;
use crate::i18n::{tr, Key};
use crate::widgets::{card, kv_grid};
use rigscope_engine::App;

pub fn show(ui: &mut egui::Ui, app: &App) {
    let lang = app.session.language;
    let s = &app.session.snapshot;
    card(ui, tr(lang, Key::System), |ui| {
        kv_grid(
            ui,
            "sys",
            &[
                (tr(lang, Key::Os), fmt::text(lang, &s.system.os_name)),
                (tr(lang, Key::Kernel), fmt::text(lang, &s.system.kernel)),
                (tr(lang, Key::Hostname), fmt::text(lang, &s.system.hostname)),
                (
                    tr(lang, Key::Uptime),
                    fmt::field(lang, &s.system.uptime_s, |v| fmt::uptime(*v)),
                ),
            ],
        );
    });
    card(ui, tr(lang, Key::Board), |ui| {
        kv_grid(
            ui,
            "board",
            &[
                (
                    tr(lang, Key::Manufacturer),
                    fmt::text(lang, &s.board.vendor),
                ),
                (tr(lang, Key::Model), fmt::text(lang, &s.board.product)),
                (tr(lang, Key::Version), fmt::text(lang, &s.board.version)),
            ],
        );
    });
    card(ui, tr(lang, Key::Bios), |ui| {
        kv_grid(
            ui,
            "bios",
            &[
                (tr(lang, Key::Vendor), fmt::text(lang, &s.board.bios_vendor)),
                (
                    tr(lang, Key::Version),
                    fmt::text(lang, &s.board.bios_version),
                ),
                (tr(lang, Key::Date), fmt::text(lang, &s.board.bios_date)),
            ],
        );
    });
}
