use crate::fmt;
use crate::i18n::{tr, Key};
use crate::widgets::{card, kv_grid};
use rigscope_engine::App;
use rigscope_model::format_mwh;

pub fn show(ui: &mut egui::Ui, app: &App) {
    let lang = app.session.language;
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
            let on_mains = fmt::field(lang, &power.ac_online, |on| {
                tr(lang, if *on { Key::Yes } else { Key::No }).to_string()
            });
            kv_grid(
                ui,
                &format!("bat-{}", b.name),
                &[
                    (tr(lang, Key::Health), health),
                    (
                        tr(lang, Key::FullCapacity),
                        fmt::field(lang, &b.full_mwh, |v| format_mwh(*v)),
                    ),
                    (
                        tr(lang, Key::DesignCapacity),
                        fmt::field(lang, &b.design_mwh, |v| format_mwh(*v)),
                    ),
                    (
                        tr(lang, Key::CurrentEnergy),
                        fmt::field(lang, &b.now_mwh, |v| format_mwh(*v)),
                    ),
                    (
                        tr(lang, Key::Charge),
                        fmt::field(lang, &b.charge_percent, |c| format!("{c} %")),
                    ),
                    (tr(lang, Key::Status), fmt::text(lang, &b.status)),
                    (tr(lang, Key::OnMains), on_mains),
                    (
                        tr(lang, Key::Cycles),
                        fmt::field(lang, &b.cycles, |c| c.to_string()),
                    ),
                    (tr(lang, Key::Technology), fmt::text(lang, &b.technology)),
                    (
                        tr(lang, Key::Manufacturer),
                        fmt::text(lang, &b.manufacturer),
                    ),
                    (tr(lang, Key::Model), fmt::text(lang, &b.model)),
                    (tr(lang, Key::Serial), fmt::text(lang, &b.serial)),
                ],
            );
        });
    }
}
