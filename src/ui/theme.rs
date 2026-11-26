//! Colours of engines and statuses.

use egui::Color32;

use crate::docker::status::EngineStatus;
use crate::engines::EngineId;

/// Brand-inspired accent colour of an engine.
pub fn engine_color(id: EngineId) -> Color32 {
    match id {
        EngineId::Postgres => Color32::from_rgb(0x33, 0x67, 0x91),
        EngineId::MariaDb => Color32::from_rgb(0xC0, 0x76, 0x5A),
        EngineId::MySql => Color32::from_rgb(0x00, 0x75, 0x8F),
        EngineId::MsSql => Color32::from_rgb(0xCC, 0x29, 0x27),
        EngineId::Firebird => Color32::from_rgb(0xF2, 0x6B, 0x1D),
        EngineId::InterBase => Color32::from_rgb(0x8E, 0x44, 0xAD),
        EngineId::Sqlite => Color32::from_rgb(0x0F, 0x80, 0xCC),
    }
}

/// Short monogram drawn in the engine badge.
pub fn engine_monogram(id: EngineId) -> &'static str {
    match id {
        EngineId::Postgres => "PG",
        EngineId::MariaDb => "Ma",
        EngineId::MySql => "My",
        EngineId::MsSql => "MS",
        EngineId::Firebird => "FB",
        EngineId::InterBase => "IB",
        EngineId::Sqlite => "SL",
    }
}

pub fn status_color(status: EngineStatus, dark: bool) -> Color32 {
    match (status, dark) {
        (EngineStatus::NotCreated, _) => Color32::from_rgb(0x8A, 0x8F, 0x98),
        (EngineStatus::Stopped, true) => Color32::from_rgb(0x9A, 0xA5, 0xB8),
        (EngineStatus::Stopped, false) => Color32::from_rgb(0x5A, 0x65, 0x78),
        (EngineStatus::Starting, true) => Color32::from_rgb(0xF5, 0xB7, 0x31),
        (EngineStatus::Starting, false) => Color32::from_rgb(0xB0, 0x76, 0x00),
        (EngineStatus::Healthy, true) => Color32::from_rgb(0x4C, 0xC9, 0x6A),
        (EngineStatus::Healthy, false) => Color32::from_rgb(0x1E, 0x8E, 0x3E),
        (EngineStatus::Running, true) => Color32::from_rgb(0x4F, 0xC3, 0xC9),
        (EngineStatus::Running, false) => Color32::from_rgb(0x0B, 0x7A, 0x80),
        (EngineStatus::Error, true) => Color32::from_rgb(0xF0, 0x5D, 0x5D),
        (EngineStatus::Error, false) => Color32::from_rgb(0xC6, 0x28, 0x28),
        (EngineStatus::LicenseRequired, true) => Color32::from_rgb(0xF2, 0x8C, 0x38),
        (EngineStatus::LicenseRequired, false) => Color32::from_rgb(0xB3, 0x50, 0x00),
    }
}

pub const OK_COLOR: Color32 = Color32::from_rgb(0x3D, 0xB8, 0x5C);
pub const ERROR_COLOR: Color32 = Color32::from_rgb(0xE0, 0x4F, 0x4F);
pub const WARNING_COLOR: Color32 = Color32::from_rgb(0xE6, 0xA2, 0x17);

/// Spacing and shapes shared by the light and dark themes.
pub fn apply_style(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.window_margin = egui::Margin::same(14);
        style.visuals.window_corner_radius = egui::CornerRadius::same(10);
        style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
        style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
        style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);
    });
}
