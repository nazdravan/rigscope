use crate::fmt;
use crate::i18n::{Key, hidden_virtual, tr};
use crate::widgets::{card, kv_grid};
use rigscope_engine::App;

pub fn show(ui: &mut egui::Ui, app: &App) {
    let lang = app.session.language;
    let cx = crate::fmt::Ctx::of(app);
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
                        fmt::field(cx, &n.kind, |k| k.label().to_string()),
                    ),
                    (tr(lang, Key::Vendor), fmt::text(cx, &n.vendor_name)),
                    (tr(lang, Key::Model), fmt::text(cx, &n.device_name)),
                    (tr(lang, Key::Driver), fmt::text(cx, &n.driver)),
                    (tr(lang, Key::Mac), fmt::text(cx, &n.mac)),
                    (
                        tr(lang, Key::State),
                        fmt::field(cx, &n.state, |v| crate::i18n::kernel_value(lang, v)),
                    ),
                    (
                        tr(lang, Key::Speed),
                        fmt::field(cx, &n.speed_mbps, |v| rigscope_model::format_speed(*v)),
                    ),
                    (
                        tr(lang, Key::Duplex),
                        fmt::field(cx, &n.duplex, |v| crate::i18n::kernel_value(lang, v)),
                    ),
                    (
                        tr(lang, Key::Mtu),
                        fmt::field(cx, &n.mtu, |v| v.to_string()),
                    ),
                ],
            );
        });
    }
    if net.hidden_virtual > 0 {
        ui.label(hidden_virtual(lang, net.hidden_virtual));
    }
}
