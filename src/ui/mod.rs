//! Views, cards, windows and styling.

pub mod cards;
pub mod docker_missing;
pub mod interbase;
pub mod settings;
pub mod theme;
pub mod toasts;
pub mod windows;

use egui::RichText;

use crate::app::{engine_names, kind_text, Action, DbDockApp};
use crate::config::{Language, Theme};
use crate::docker::DockerCheck;
use crate::i18n::{self, t, tf};
use theme::{ERROR_COLOR, OK_COLOR};

impl DbDockApp {
    pub(crate) fn top_bar(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        egui::Panel::top("top-bar").show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("DBDock").strong().size(20.0));
                ui.separator();

                let ready = self
                    .docker_check
                    .as_ref()
                    .is_some_and(DockerCheck::is_ready);
                if ui
                    .add_enabled(ready, egui::Button::new(t("top.start_all")))
                    .clicked()
                {
                    actions.push(Action::StartAllEnabled);
                }
                if ui
                    .add_enabled(ready, egui::Button::new(t("top.stop_all")))
                    .clicked()
                {
                    actions.push(Action::StopAll);
                }
                ui.separator();
                if ui
                    .button(t("top.open_data_folder"))
                    .on_hover_text(self.paths.base().display().to_string())
                    .clicked()
                {
                    actions.push(Action::OpenFolder(self.paths.base().to_path_buf()));
                }
                if ui
                    .button(t("top.export_compose"))
                    .on_hover_text(t("top.export_compose_hint"))
                    .clicked()
                {
                    actions.push(Action::ExportCompose);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (label, next) = match self.config.theme {
                        Theme::Dark => (t("top.theme_light"), Theme::Light),
                        Theme::Light => (t("top.theme_dark"), Theme::Dark),
                    };
                    if ui.button(label).clicked() {
                        actions.push(Action::SetTheme(next));
                    }
                    language_switch(ui, actions);
                    ui.separator();
                    docker_indicator(ui, self.docker_check.as_ref());
                });
            });
            ui.add_space(6.0);
        });
    }

    pub(crate) fn output_window_ui(&mut self, ctx: &egui::Context) {
        let Some(op_id) = self.output_window else {
            return;
        };
        let Some(op) = self.operation(op_id) else {
            self.output_window = None;
            return;
        };
        let title = format!("{} {}", kind_text(op.kind), engine_names(&op.engines));
        let mut open = true;
        egui::Window::new(t("output.title"))
            .id(egui::Id::unique("op-output"))
            .open(&mut open)
            .default_width(640.0)
            .default_height(360.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(title).strong());
                    match &op.result {
                        None => {
                            ui.spinner();
                        }
                        Some(Ok(())) => {
                            ui.colored_label(OK_COLOR, t("output.done"));
                        }
                        Some(Err(_)) => {
                            ui.colored_label(ERROR_COLOR, t("output.failed"));
                        }
                    }
                    if ui.button(t("common.copy")).clicked() {
                        ui.ctx().copy_text(op.output.join("\n"));
                    }
                });
                ui.separator();
                egui::ScrollArea::both()
                    .stick_to_bottom(true)
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        for line in &op.output {
                            ui.label(RichText::new(line).monospace());
                        }
                        if let Some(Err(e)) = &op.result {
                            ui.colored_label(ERROR_COLOR, RichText::new(e).monospace());
                        }
                    });
            });
        if !open {
            self.output_window = None;
        }
    }
}

/// EN / PL toggle.
fn language_switch(ui: &mut egui::Ui, actions: &mut Vec<Action>) {
    let current = i18n::language();
    for (language, label) in [(Language::Pl, "PL"), (Language::En, "EN")] {
        if ui
            .selectable_label(current == language, label)
            .on_hover_text(t("top.language"))
            .clicked()
            && current != language
        {
            actions.push(Action::SetLanguage(language));
        }
    }
}

fn docker_indicator(ui: &mut egui::Ui, check: Option<&DockerCheck>) {
    match check {
        None => {
            ui.spinner();
            ui.label("Docker");
        }
        Some(DockerCheck::Ready {
            server_version,
            compose_version,
        }) => {
            ui.label(RichText::new("Docker").color(OK_COLOR))
                .on_hover_text(tf(
                    "top.docker_ok",
                    &[("docker", server_version), ("compose", compose_version)],
                ));
            dot(ui, OK_COLOR);
        }
        Some(_) => {
            ui.label(RichText::new("Docker").color(ERROR_COLOR))
                .on_hover_text(t("top.docker_missing"));
            dot(ui, ERROR_COLOR);
        }
    }
}

/// Small filled circle used as a status light (drawn, so it does not depend
/// on emoji fonts).
pub(crate) fn dot(ui: &mut egui::Ui, color: egui::Color32) {
    let size = ui.text_style_height(&egui::TextStyle::Body);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size * 0.7, size), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), size * 0.3, color);
}
