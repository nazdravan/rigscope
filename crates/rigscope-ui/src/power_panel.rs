use crate::fmt;
use crate::i18n::{Key, tr};
use crate::widgets::{card, kv_grid};
use rigscope_engine::App;
use rigscope_model::format_mwh;

pub fn show(ui: &mut egui::Ui, app: &App) {
    let lang = app.session.language;
    let cx = crate::fmt::Ctx::of(app);
    let power = &app.session.snapshot.power;
    if power.batteries.is_empty() {
        card(ui, tr(lang, Key::Battery), |ui| {
            ui.label(tr(lang, Key::NoBattery));
        });
        return;
    }
    for b in &power.batteries {
        let title = match b.model.known() {
            Some(m) => format!("{}  ({m})", b.name),
            None => b.name.clone(),
        };
        card(ui, &title, |ui| {
            let health = match b.health_percent() {
                Some(h) => format!("{h:.1} % ({})", tr(lang, Key::Derived)),
                None => tr(lang, Key::Unknown).to_string(),
            };
            let on_mains = fmt::field(cx, &power.ac_online, |on| {
                tr(lang, if *on { Key::Yes } else { Key::No }).to_string()
            });
            kv_grid(
                ui,
                &format!("bat-{}", b.name),
                &[
                    (tr(lang, Key::Health), health),
                    (
                        tr(lang, Key::FullCapacity),
                        fmt::field(cx, &b.full_mwh, |v| format_mwh(*v)),
                    ),
                    (
                        tr(lang, Key::DesignCapacity),
                        fmt::field(cx, &b.design_mwh, |v| format_mwh(*v)),
                    ),
                    (
                        tr(lang, Key::CurrentEnergy),
                        fmt::field(cx, &b.now_mwh, |v| format_mwh(*v)),
                    ),
                    (
                        tr(lang, Key::Charge),
                        fmt::field(cx, &b.charge_percent, |c| format!("{c} %")),
                    ),
                    (
                        tr(lang, Key::Status),
                        fmt::field(cx, &b.status, |v| crate::i18n::kernel_value(lang, v)),
                    ),
                    (tr(lang, Key::OnMains), on_mains),
                    (
                        tr(lang, Key::Cycles),
                        fmt::field(cx, &b.cycles, |c| c.to_string()),
                    ),
                    (tr(lang, Key::Technology), fmt::text(cx, &b.technology)),
                    (tr(lang, Key::Manufacturer), fmt::text(cx, &b.manufacturer)),
                    (tr(lang, Key::Model), fmt::text(cx, &b.model)),
                    (tr(lang, Key::Serial), fmt::text(cx, &b.serial)),
                ],
            );
        });
    }
}
