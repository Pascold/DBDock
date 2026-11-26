//! The grid of database cards.

use egui::{Color32, CornerRadius, RichText, Stroke, Vec2};

use super::theme::{engine_color, engine_monogram, status_color};
use crate::app::{Action, DbDockApp};
use crate::docker::status::EngineStatus;
use crate::docker::worker::OpKind;
use crate::engines::{EngineId, Field};
use crate::i18n::{t, tf};

const MIN_CARD_WIDTH: f32 = 300.0;
/// Inner margins and stroke of the card frame.
const CARD_FRAME: f32 = 26.0;
const CARD_SPACING: f32 = 12.0;

pub fn status_text(status: EngineStatus) -> &'static str {
    match status {
        EngineStatus::NotCreated => t("status.not_created"),
        EngineStatus::Stopped => t("status.stopped"),
        EngineStatus::Starting => t("status.starting"),
        EngineStatus::Healthy => t("status.healthy"),
        EngineStatus::Running => t("status.running"),
        EngineStatus::Error => t("status.error"),
        EngineStatus::LicenseRequired => t("status.license_required"),
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
                            tf("card.status_failed", &[("error", &first_line(error))]),
                        );
                        ui.add_space(6.0);
                    }
                    let available = ui.available_width();
                    let columns = ((available + CARD_SPACING) / (MIN_CARD_WIDTH + CARD_SPACING))
                        .floor()
                        .max(1.0) as usize;
                    // Stretch the cards so the columns fill the width.
                    let card_width = ((available - CARD_SPACING * (columns - 1) as f32)
                        / columns as f32)
                        .floor();
                    for row in EngineId::ALL.chunks(columns) {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = CARD_SPACING;
                            for &id in row {
                                self.card(ui, id, card_width, actions);
                            }
                        });
                        ui.add_space(CARD_SPACING);
                    }
                });
        });
    }

    fn card(&self, ui: &mut egui::Ui, id: EngineId, width: f32, actions: &mut Vec<Action>) {
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
                ui.set_width(width - CARD_FRAME);
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
                            details.push_str(&format!(
                                "  ·  {}",
                                tf("card.port", &[("port", &settings.port)])
                            ));
                        }
                        ui.label(RichText::new(details).weak());
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        let mut enabled = settings.enabled;
                        let toggle = ui
                            .add_enabled(!busy, egui::Checkbox::without_text(&mut enabled))
                            .on_hover_text(t("card.enabled"));
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
                            .on_hover_text(t("card.show_output"))
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
                    if id == EngineId::InterBase
                        && (!self.config.interbase_licensed
                            || status == EngineStatus::LicenseRequired)
                    {
                        if ui
                            .add_enabled(!busy, egui::Button::new(t("card.setup_license")))
                            .clicked()
                        {
                            actions.push(Action::OpenInterBaseWizard);
                        }
                        return;
                    }
                    if id == EngineId::Sqlite {
                        // No server: "start" creates the file once.
                        let can_create = can_act && !status.is_created();
                        if ui
                            .add_enabled(can_create, egui::Button::new(t("card.create_file")))
                            .clicked()
                        {
                            actions.push(Action::Op(OpKind::Start, vec![id]));
                        }
                        if ui.button(t("common.open_folder")).clicked() {
                            actions.push(Action::OpenFolder(self.paths.sqlite_dir()));
                        }
                        return;
                    }
                    if ui
                        .add_enabled(can_act && !up, egui::Button::new(t("card.start")))
                        .clicked()
                    {
                        actions.push(Action::Op(OpKind::Start, vec![id]));
                    }
                    if ui
                        .add_enabled(can_act && up, egui::Button::new(t("card.stop")))
                        .clicked()
                    {
                        actions.push(Action::Op(OpKind::Stop, vec![id]));
                    }
                    if ui
                        .add_enabled(
                            can_act && status.is_created(),
                            egui::Button::new(t("card.restart")),
                        )
                        .clicked()
                    {
                        actions.push(Action::Op(OpKind::Restart, vec![id]));
                    }
                });
                if engine.amd64_only() && cfg!(target_arch = "aarch64") {
                    ui.colored_label(super::theme::WARNING_COLOR, emulation_warning());
                }
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_enabled(!busy, egui::Button::new(t("card.settings")))
                        .clicked()
                    {
                        actions.push(Action::OpenSettings(id));
                    }
                    if ui.button(t("card.connection")).clicked() {
                        actions.push(Action::OpenConnection(id));
                    }
                    if ui
                        .button(t("common.copy"))
                        .on_hover_text(t("card.copy_hint"))
                        .clicked()
                    {
                        actions.push(Action::CopyConnection(id));
                    }
                    if ui
                        .add_enabled(status.is_created(), egui::Button::new(t("card.logs")))
                        .clicked()
                    {
                        actions.push(Action::OpenLogs(id));
                    }
                    if ui
                        .add_enabled(
                            !busy && status.is_created(),
                            egui::Button::new(t("card.reset")),
                        )
                        .on_hover_text(t("card.reset_hint"))
                        .clicked()
                    {
                        actions.push(Action::ConfirmReset(id));
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

/// Shown on x86-64-only engines when running on an ARM computer.
fn emulation_warning() -> &'static str {
    if cfg!(target_os = "macos") {
        t("card.emulation_mac")
    } else {
        t("card.emulation_other")
    }
}
