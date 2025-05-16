use crate::fmt;
use crate::i18n::{slots_free, tr, Key};
use crate::widgets::{card, command_button, kv_grid};
use crate::Actions;
use rigscope_engine::{App, Language};
use rigscope_model::{compat_summary, CompatLine, DmiSource, MemoryModule, MemorySlot, SlotState};
use serde_json::json;

pub fn show(ui: &mut egui::Ui, app: &App, actions: &mut Actions) {
    let lang = app.session.language;
    let cx = crate::fmt::Ctx::of(app);
    let snap = &app.session.snapshot;

    card(ui, tr(lang, Key::Memory), |ui| {
        let mut rows = vec![(
            tr(lang, Key::InstalledMemory),
            fmt::field(cx, &snap.memory.os_total_bytes, |b| fmt::bytes(*b)),
        )];
        for array in &snap.memory.arrays {
            rows.push((
                tr(lang, Key::MaxCapacity),
                fmt::field(cx, &array.max_capacity_bytes, |b| fmt::bytes(*b)),
            ));
            rows.push((
                tr(lang, Key::Ecc),
                fmt::field(cx, &array.ecc, |v| crate::i18n::ecc(lang, v)),
            ));
        }
        kv_grid(ui, "mem-summary", &rows);

        if let Some(c) = compat_summary(&snap.memory) {
            ui.add_space(8.0);
            compat_row(ui, app, actions, lang, &c);
        }
    });

    match snap.dmi_source {
        DmiSource::FullTable => {}
        DmiSource::NoTable | DmiSource::Firmware => {
            card(ui, tr(lang, Key::SlotDetails), |ui| {
                ui.label(tr(lang, Key::NoDmi));
            });
        }
        DmiSource::SysfsIdOnly => {
            card(ui, tr(lang, Key::SlotDetails), |ui| {
                ui.label(tr(lang, Key::NeedsRootBanner));
                command_button(
                    ui,
                    app,
                    actions,
                    "snapshot.elevate",
                    tr(lang, Key::ReadElevated),
                    json!({}),
                );
            });
        }
        DmiSource::None => {
            card(ui, tr(lang, Key::SlotDetails), |ui| {
                ui.label(tr(lang, Key::NoDmi));
            });
        }
    }

    let slots: Vec<&MemorySlot> = snap
        .memory
        .arrays
        .iter()
        .flat_map(|a| a.slots.iter())
        .collect();
    if !slots.is_empty() {
        card(ui, tr(lang, Key::Slots), |ui| {
            slot_strip(ui, cx, &slots);
            ui.add_space(10.0);
            egui::ScrollArea::horizontal().show(ui, |ui| slot_table(ui, cx, &slots));
        });
    }
}

fn compat_row(ui: &mut egui::Ui, app: &App, actions: &mut Actions, lang: Language, c: &CompatLine) {
    let mut parts: Vec<String> = Vec::new();
    let kind = [
        c.mem_type.map(|t| t.label()),
        c.form_factor.map(|f| f.label()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ");
    if !kind.is_empty() {
        parts.push(kind);
    }
    if let Some(m) = c.max_mts {
        parts.push(format!("{} {}", tr(lang, Key::UpTo), fmt::mts(m)));
    }
    if let Some(b) = c.max_per_slot_bytes_estimated {
        parts.push(format!(
            "max. {} {} ({})",
            fmt::bytes(b),
            tr(lang, Key::MaxPerSlot),
            tr(lang, Key::Estimated)
        ));
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(
            egui::RichText::new(format!(
                "{}: {}",
                tr(lang, Key::Compatible),
                parts.join(", ")
            ))
            .strong(),
        );
        command_button(
            ui,
            app,
            actions,
            "memory.copy_compat",
            tr(lang, Key::Copy),
            json!({}),
        );
    });
    if c.mixed_types {
        ui.label(tr(lang, Key::MixedTypes));
    }
    if let (Some(free), Some(total)) = (c.free_slots, c.total_slots) {
        ui.label(slots_free(lang, free, total));
    }
}

fn slot_title(lang: Language, slot: &MemorySlot, n: usize) -> String {
    match slot.locator.known() {
        Some(l) => l.clone(),
        None => format!("{} {}", tr(lang, Key::Slot), n + 1),
    }
}

/// One rectangle per slot: filled = module installed, outline = empty.
fn slot_strip(ui: &mut egui::Ui, cx: fmt::Ctx, slots: &[&MemorySlot]) {
    let lang = cx.lang;
    ui.horizontal_wrapped(|ui| {
        for (n, slot) in slots.iter().enumerate() {
            ui.vertical(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(120.0, 34.0), egui::Sense::hover());
                let visuals = ui.visuals();
                let accent = visuals.selection.bg_fill;
                let stroke = egui::Stroke::new(1.5, accent);
                match &slot.module {
                    SlotState::Installed(m) => {
                        ui.painter()
                            .rect(rect, 4.0, accent, stroke, egui::StrokeKind::Inside);
                        let text = fmt::field(cx, &m.size_bytes, |b| fmt::bytes(*b));
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            text,
                            egui::FontId::proportional(14.0),
                            visuals.selection.stroke.color,
                        );
                    }
                    SlotState::Empty | SlotState::Unknown => {
                        ui.painter()
                            .rect_stroke(rect, 4.0, stroke, egui::StrokeKind::Inside);
                        let key = if slot.module == SlotState::Empty {
                            Key::Empty
                        } else {
                            Key::Unknown
                        };
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            tr(lang, key),
                            egui::FontId::proportional(14.0),
                            visuals.text_color(),
                        );
                    }
                }
                ui.label(slot_title(lang, slot, n));
            });
            ui.add_space(8.0);
        }
    });
}

/// Fields as rows, slots as columns.
fn slot_table(ui: &mut egui::Ui, cx: fmt::Ctx, slots: &[&MemorySlot]) {
    let lang = cx.lang;
    type Row = (Key, fn(fmt::Ctx, &MemoryModule) -> String);
    let rows: [Row; 11] = [
        (Key::Size, |l, m| {
            fmt::field(l, &m.size_bytes, |b| fmt::bytes(*b))
        }),
        (Key::Type, |l, m| fmt::field(l, &m.mem_type, |t| t.label())),
        (Key::FormFactor, |l, m| {
            fmt::field(l, &m.form_factor, |f| f.label())
        }),
        (Key::RatedSpeed, |l, m| {
            fmt::field(l, &m.rated_mts, |v| fmt::mts(*v))
        }),
        (Key::ConfiguredSpeed, |l, m| {
            fmt::field(l, &m.configured_mts, |v| fmt::mts(*v))
        }),
        (Key::Manufacturer, |l, m| fmt::text(l, &m.manufacturer)),
        (Key::PartNumber, |l, m| fmt::text(l, &m.part_number)),
        (Key::Serial, |l, m| fmt::text(l, &m.serial)),
        (Key::Ranks, |l, m| {
            fmt::field(l, &m.ranks, |r| r.to_string())
        }),
        (Key::Voltage, |l, m| {
            fmt::field(l, &m.voltage_mv, |v| {
                match v.configured.or(v.max).or(v.min) {
                    Some(mv) => format!("{:.2} V", f64::from(mv) / 1000.0),
                    None => tr(l.lang, Key::Unknown).to_string(),
                }
            })
        }),
        (Key::Width, |l, m| {
            fmt::field(l, &m.data_width_bits, |w| format!("{w} bit"))
        }),
    ];
    egui::Grid::new("slot-table")
        .striped(true)
        .spacing([24.0, 6.0])
        .show(ui, |ui| {
            ui.label(egui::RichText::new(tr(lang, Key::Slot)).strong());
            for (n, slot) in slots.iter().enumerate() {
                ui.label(egui::RichText::new(slot_title(lang, slot, n)).strong());
            }
            ui.end_row();
            ui.label(egui::RichText::new(tr(lang, Key::Bank)).strong());
            for slot in slots {
                ui.label(fmt::text(cx, &slot.bank));
            }
            ui.end_row();
            for (key, show) in rows {
                ui.label(egui::RichText::new(tr(lang, key)).strong());
                for slot in slots {
                    let v = match &slot.module {
                        SlotState::Installed(m) => show(cx, m),
                        SlotState::Empty => {
                            if key == Key::Size {
                                tr(lang, Key::Empty).to_string()
                            } else {
                                "—".to_string()
                            }
                        }
                        SlotState::Unknown => tr(lang, Key::Unknown).to_string(),
                    };
                    ui.label(v);
                }
                ui.end_row();
            }
        });
}
