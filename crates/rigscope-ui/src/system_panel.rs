use crate::fmt;
use crate::i18n::{Key, tr};
use crate::widgets::{card, kv_grid};
use rigscope_engine::App;

pub fn show(ui: &mut egui::Ui, app: &App) {
    let lang = app.session.language;
    let cx = crate::fmt::Ctx::of(app);
    let s = &app.session.snapshot;
    card(ui, tr(lang, Key::System), |ui| {
        kv_grid(
            ui,
            "sys",
            &[
                (tr(lang, Key::Os), fmt::text(cx, &s.system.os_name)),
                (tr(lang, Key::Kernel), fmt::text(cx, &s.system.kernel)),
                (tr(lang, Key::Hostname), fmt::text(cx, &s.system.hostname)),
                (
                    tr(lang, Key::Uptime),
                    fmt::field(cx, &s.system.uptime_s, |v| fmt::uptime(*v)),
                ),
            ],
        );
    });
    card(ui, tr(lang, Key::Board), |ui| {
        kv_grid(
            ui,
            "board",
            &[
                (tr(lang, Key::Manufacturer), fmt::text(cx, &s.board.vendor)),
                (tr(lang, Key::Model), fmt::text(cx, &s.board.product)),
                (tr(lang, Key::Version), fmt::text(cx, &s.board.version)),
            ],
        );
    });
    card(ui, tr(lang, Key::Bios), |ui| {
        kv_grid(
            ui,
            "bios",
            &[
                (tr(lang, Key::Vendor), fmt::text(cx, &s.board.bios_vendor)),
                (tr(lang, Key::Version), fmt::text(cx, &s.board.bios_version)),
                (tr(lang, Key::Date), fmt::text(cx, &s.board.bios_date)),
            ],
        );
    });
}
