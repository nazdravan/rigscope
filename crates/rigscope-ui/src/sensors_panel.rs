use crate::fmt;
use crate::i18n::{tr, Key};
use crate::widgets::card;
use rigscope_engine::App;
use rigscope_model::{chip_kind, format_celsius};

pub fn show(ui: &mut egui::Ui, app: &App) {
    let lang = app.session.language;
    let chips = &app.session.snapshot.sensors.chips;
    if chips.is_empty() {
        card(ui, tr(lang, Key::Sensors), |ui| {
            ui.label(tr(lang, Key::NoSensors));
        });
        return;
    }
    for chip in chips {
        let title = match chip_kind(&chip.name) {
            Some(kind) => format!("{kind}  ({})", chip.name),
            None => chip.name.clone(),
        };
        card(ui, &title, |ui| {
            egui::Grid::new(format!("sensors-{}", chip.hwmon))
                .num_columns(3)
                .striped(true)
                .spacing([24.0, 6.0])
                .show(ui, |ui| {
                    for t in &chip.temps {
                        ui.label(egui::RichText::new(&t.label).strong());
                        ui.label(fmt::field(lang, &t.millicelsius, |v| format_celsius(*v)));
                        let mut limits = Vec::new();
                        if let Some(m) = t.max_mc.known() {
                            limits.push(format!("max {}", format_celsius(*m)));
                        }
                        if let Some(c) = t.crit_mc.known() {
                            limits.push(format!("crit {}", format_celsius(*c)));
                        }
                        ui.label(limits.join(", "));
                        ui.end_row();
                    }
                    for f in &chip.fans {
                        ui.label(egui::RichText::new(&f.label).strong());
                        ui.label(fmt::field(lang, &f.rpm, |r| format!("{r} RPM")));
                        ui.label("");
                        ui.end_row();
                    }
                });
        });
    }
}
