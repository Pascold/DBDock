// Hide the console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dbdock::app::DbDockApp;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("DBDock")
            .with_inner_size([1000.0, 680.0])
            .with_min_inner_size([360.0, 420.0]),
        ..Default::default()
    };
    eframe::run_native(
        "DBDock",
        options,
        Box::new(|cc| Ok(Box::new(DbDockApp::new(cc)))),
    )
}
