use crate::fmt;
use crate::i18n::{tr, Key};
use crate::widgets::{card, kv_grid};
use rigscope_engine::App;
use rigscope_model::CacheKind;

pub fn show(ui: &mut egui::Ui, app: &App) {
    let lang = app.session.language;
    let cx = crate::fmt::Ctx::of(app);
    let cpu = &app.session.snapshot.cpu;

    card(ui, tr(lang, Key::Processor), |ui| {
        let cores = fmt::field(cx, &cpu.physical_cores, |n| n.to_string());
        let threads = fmt::field(cx, &cpu.logical_threads, |n| n.to_string());
        kv_grid(
            ui,
            "cpu-summary",
            &[
                (tr(lang, Key::Model), fmt::text(cx, &cpu.model)),
                (tr(lang, Key::Vendor), fmt::text(cx, &cpu.vendor)),
                (
                    tr(lang, Key::Socket),
                    rigscope_model::cpu::socket_label(fmt::text(cx, &cpu.socket), &cpu.sockets),
                ),
                (tr(lang, Key::CoresThreads), format!("{cores} / {threads}")),
                (
                    tr(lang, Key::BaseClock),
                    fmt::field(cx, &cpu.base_mhz, |v| fmt::mhz(*v)),
                ),
                (
                    tr(lang, Key::MaxClock),
                    fmt::field(cx, &cpu.max_mhz, |v| fmt::mhz(*v)),
                ),
                (tr(lang, Key::Microcode), fmt::text(cx, &cpu.microcode)),
            ],
        );
    });

    if !cpu.caches.is_empty() {
        card(ui, tr(lang, Key::Cache), |ui| {
            let rows: Vec<(String, String)> = cpu
                .caches
                .iter()
                .map(|c| {
                    let kind = match c.kind {
                        CacheKind::Data => "d",
                        CacheKind::Instruction => "i",
                        CacheKind::Unified => "",
                    };
                    let per = if c.instances > 1 {
                        format!("{} × {}", c.instances, fmt::bytes(c.size_bytes))
                    } else {
                        fmt::bytes(c.size_bytes)
                    };
                    (format!("L{}{kind}", c.level), per)
                })
                .collect();
            let rows: Vec<(&str, String)> =
                rows.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
            kv_grid(ui, "cpu-cache", &rows);
        });
    }

    if !cpu.cores.is_empty() {
        card(ui, tr(lang, Key::CurrentFrequency), |ui| {
            if cpu.cores.iter().any(|c| c.from_cpuinfo_fallback) {
                ui.label(tr(lang, Key::FromCpuinfo));
            }
            if cpu.cores.iter().any(|c| c.estimated) {
                ui.label(tr(lang, Key::EstimatedFrequency));
            }
            let cols = 4;
            egui::Grid::new("cpu-cores")
                .striped(true)
                .spacing([28.0, 6.0])
                .show(ui, |ui| {
                    for (i, core) in cpu.cores.iter().enumerate() {
                        ui.label(egui::RichText::new(format!("cpu{}", core.cpu)).strong());
                        ui.label(fmt::field(cx, &core.cur_mhz, |v| fmt::mhz(*v)));
                        if (i + 1) % cols == 0 {
                            ui.end_row();
                        }
                    }
                });
        });
    }

    if !cpu.flags.is_empty() {
        card(ui, tr(lang, Key::Flags), |ui| {
            egui::CollapsingHeader::new(crate::i18n::flag_count(lang, cpu.flags.len())).show(
                ui,
                |ui| {
                    ui.label(cpu.flags.join(" "));
                },
            );
        });
    }
}
