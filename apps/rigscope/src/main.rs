//! rigscope — hardware configuration viewer.

fn main() -> anyhow::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native("rigscope", options, Box::new(|_cc| Ok(Box::new(Shell))))
        .map_err(|e| anyhow::anyhow!("{e}"))
}

struct Shell;

impl eframe::App for Shell {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        ui.label("rigscope");
    }
}
