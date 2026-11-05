use crate::fmt;
use crate::i18n::{Key, tr};
use crate::widgets::{card, kv_grid};
use rigscope_engine::App;

pub fn show(ui: &mut egui::Ui, app: &App) {
    let lang = app.session.language;
    let cx = crate::fmt::Ctx::of(app);
    let gpus = &app.session.snapshot.gpu.gpus;
    if gpus.is_empty() {
        card(ui, tr(lang, Key::Graphics), |ui| {
            ui.label(tr(lang, Key::NoGpus));
        });
        return;
    }
    for g in gpus {
        let title = match g.device_name.known() {
            Some(name) => format!("{name}  ({})", g.card),
            None => g.card.clone(),
        };
        card(ui, &title, |ui| {
            let primary = if g.boot_vga { Key::Yes } else { Key::No };
            let id = g
                .pci_id()
                .unwrap_or_else(|| tr(lang, Key::Unknown).to_string());
            kv_grid(
                ui,
                &format!("gpu-{}", g.card),
                &[
                    (tr(lang, Key::Vendor), fmt::text(cx, &g.vendor_name)),
                    (tr(lang, Key::Model), fmt::text(cx, &g.device_name)),
                    (
                        tr(lang, Key::BoardVariant),
                        fmt::text(cx, &g.subsystem_name),
                    ),
                    (tr(lang, Key::PciId), id),
                    (tr(lang, Key::PciAddress), fmt::text(cx, &g.pci_address)),
                    (tr(lang, Key::Driver), fmt::text(cx, &g.driver)),
                    (
                        tr(lang, Key::Vram),
                        fmt::field(cx, &g.vram_bytes, |v| fmt::bytes(*v)),
                    ),
                    (tr(lang, Key::PcieLink), fmt::text(cx, &g.pcie_link)),
                    (tr(lang, Key::PrimaryDisplay), tr(lang, primary).to_string()),
                ],
            );
        });
    }
}
