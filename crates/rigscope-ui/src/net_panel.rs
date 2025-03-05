use crate::fmt;
use crate::i18n::{hidden_virtual, tr, Key};
use crate::widgets::{card, kv_grid};
use rigscope_engine::App;

pub fn show(ui: &mut egui::Ui, app: &App) {
    let lang = app.session.language;
    let net = &app.session.snapshot.net;
    if net.interfaces.is_empty() {
        card(ui, tr(lang, Key::Network), |ui| {
            ui.label(tr(lang, Key::NoInterfaces));
        });
    }
    for n in &net.interfaces {
        let title = match n.device_name.known() {
            Some(d) => format!("{}  ({d})", n.name),
            None => n.name.clone(),
        };
        card(ui, &title, |ui| {
            kv_grid(
                ui,
                &format!("net-{}", n.name),
                &[
                    (
                        tr(lang, Key::NetKind),
                        fmt::field(lang, &n.kind, |k| k.label().to_string()),
                    ),
                    (tr(lang, Key::Vendor), fmt::text(lang, &n.vendor_name)),
                    (tr(lang, Key::Model), fmt::text(lang, &n.device_name)),
                    (tr(lang, Key::Driver), fmt::text(lang, &n.driver)),
                    (tr(lang, Key::Mac), fmt::text(lang, &n.mac)),
                    (
                        tr(lang, Key::State),
                        fmt::field(lang, &n.state, |v| crate::i18n::kernel_value(lang, v)),
                    ),
                    (
                        tr(lang, Key::Speed),
                        fmt::field(lang, &n.speed_mbps, |v| rigscope_model::format_speed(*v)),
                    ),
                    (
                        tr(lang, Key::Duplex),
                        fmt::field(lang, &n.duplex, |v| crate::i18n::kernel_value(lang, v)),
                    ),
                    (
                        tr(lang, Key::Mtu),
                        fmt::field(lang, &n.mtu, |v| v.to_string()),
                    ),
                ],
            );
        });
    }
    if net.hidden_virtual > 0 {
        ui.label(hidden_virtual(lang, net.hidden_virtual));
    }
}
