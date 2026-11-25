// Hide the console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use dbdock::app::DbDockApp;

const ICON: &[u8] = include_bytes!("../assets/icon/dbdock.png");

fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("DBDock")
        .with_app_id("dbdock")
        .with_inner_size([1000.0, 700.0])
        .with_min_inner_size([400.0, 480.0]);
    // The icon is embedded and valid; should decoding ever fail the window
    // simply keeps the default icon.
    if let Ok(icon) = eframe::icon_data::from_png_bytes(ICON) {
        viewport = viewport.with_icon(Arc::new(icon));
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "DBDock",
        options,
        Box::new(|cc| Ok(Box::new(DbDockApp::new(cc)))),
    )
}
