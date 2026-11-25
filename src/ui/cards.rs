//! The grid of database cards.

use egui::{Color32, CornerRadius, RichText, Stroke, Vec2};

use super::theme::{engine_color, engine_monogram, status_color};
use crate::app::{Action, DbDockApp};
use crate::docker::status::EngineStatus;
use crate::docker::worker::OpKind;
use crate::engines::{EngineId, Field};

const CARD_WIDTH: f32 = 300.0;
const CARD_SPACING: f32 = 12.0;

pub fn status_text(status: EngineStatus) -> &'static str {
    match status {
        EngineStatus::NotCreated => "Not created",
        EngineStatus::Stopped => "Stopped",
        EngineStatus::Starting => "Starting",
        EngineStatus::Healthy => "Healthy",
        EngineStatus::Running => "Running",
        EngineStatus::Error => "Error",
    }
}

impl DbDockApp {
    pub(crate) fn cards(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    if let Some(error) = &self.status_error {
                        ui.colored_label(
                            super::theme::WARNING_COLOR,
                            format!("Status refresh failed: {}", first_line(error)),
                        );
                        ui.add_space(6.0);
                    }
                    let columns = ((ui.available_width() + CARD_SPACING)
                        / (CARD_WIDTH + CARD_SPACING))
                        .floor()
                        .max(1.0) as usize;
                    for row in EngineId::ALL.chunks(columns) {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = CARD_SPACING;
                            for &id in row {
                                self.card(ui, id, actions);
                            }
                        });
                        ui.add_space(CARD_SPACING);
                    }
                });
        });
    }

    fn card(&self, ui: &mut egui::Ui, id: EngineId, actions: &mut Vec<Action>) {
        let engine = id.engine();
        let settings = self.config.engine(id);
        let status = self.status(id);
        let running_op = self.running_op(id);
        let busy = running_op.is_some();
        let dark = ui.visuals().dark_mode;
        let accent = engine_color(id);

        let frame = egui::Frame::group(ui.style())
            .fill(ui.visuals().faint_bg_color)
            .stroke(Stroke::new(1.0, accent.gamma_multiply(0.6)))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(12.0);
        frame.show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_width(CARD_WIDTH - 24.0);
                if !settings.enabled {
                    ui.multiply_opacity(0.6);
                }

                // Header: badge, name, enabled switch.
                ui.horizontal(|ui| {
                    badge(ui, id);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(id.display_name()).strong().size(16.0));
                        let mut details = settings.version.clone();
                        if engine.uses(Field::Port) {
                            details.push_str(&format!("  ·  port {}", settings.port));
                        }
                        ui.label(RichText::new(details).weak());
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        let mut enabled = settings.enabled;
                        let toggle = ui
                            .add_enabled(!busy, egui::Checkbox::without_text(&mut enabled))
                            .on_hover_text("Enabled");
                        if toggle.changed() {
                            actions.push(Action::SetEnabled(id, enabled));
                        }
                    });
                });
                ui.add_space(6.0);

                // Status line.
                ui.horizontal(|ui| {
                    if let Some(op) = running_op {
                        ui.spinner();
                        let label = format!("{}…", crate::app::kind_text(op.kind));
                        if ui
                            .link(label)
                            .on_hover_text("Show the command output")
                            .clicked()
                        {
                            actions.push(Action::ShowOutput(op.id));
                        }
                    } else {
                        let color = status_color(status, dark);
                        super::dot(ui, color);
                        ui.label(RichText::new(status_text(status)).color(color).strong());
                    }
                });
                ui.add_space(6.0);

                // Main actions.
                ui.horizontal_wrapped(|ui| {
                    let can_act = settings.enabled && !busy;
                    let up = status.is_up();
                    if ui
                        .add_enabled(can_act && !up, egui::Button::new("Start"))
                        .clicked()
                    {
                        actions.push(Action::Op(OpKind::Start, vec![id]));
                    }
                    if ui
                        .add_enabled(can_act && up, egui::Button::new("Stop"))
                        .clicked()
                    {
                        actions.push(Action::Op(OpKind::Stop, vec![id]));
                    }
                    let can_restart = can_act && status.is_created() && id != EngineId::Sqlite;
                    if ui
                        .add_enabled(can_restart, egui::Button::new("Restart"))
                        .clicked()
                    {
                        actions.push(Action::Op(OpKind::Restart, vec![id]));
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_enabled(!busy, egui::Button::new("Settings"))
                        .clicked()
                    {
                        actions.push(Action::OpenSettings(id));
                    }
                });
            })
        });
    }
}

/// Coloured rounded square with the engine monogram.
fn badge(ui: &mut egui::Ui, id: EngineId) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(36.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(8), engine_color(id));
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        engine_monogram(id),
        egui::FontId::proportional(15.0),
        Color32::WHITE,
    );
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or(text)
}
