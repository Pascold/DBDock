// Hide the console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("DBDock")
            .with_inner_size([960.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native("DBDock", options, Box::new(|_cc| Ok(Box::new(DbDockApp))))
}

struct DbDockApp;

impl eframe::App for DbDockApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("DBDock");
        });
    }
}
