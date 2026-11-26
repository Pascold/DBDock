//! First-run wizard for InterBase (license registration).

use egui::{Align2, RichText};

use super::theme::WARNING_COLOR;
use crate::app::{Action, DbDockApp};
use crate::docker::{compose_args, display_command};
use crate::engines::interbase::{database_path, ISQL, LICENSE_MANAGER};
use crate::engines::EngineId;
use crate::i18n::t;

pub const LICENSE_URL: &str = "https://www.embarcadero.com/products/interbase/start-for-free";

impl DbDockApp {
    /// Command that runs the container interactively so the license manager
    /// can ask for the serial number.
    pub(crate) fn interbase_register_command(&self) -> String {
        display_command(&compose_args(
            &self.paths.compose_file(),
            // The service command exits without a license, so the license
            // manager is started explicitly.
            &["run", "--rm", "interbase", LICENSE_MANAGER, "-i", "Console"],
        ))
    }

    /// Command that opens `isql` inside the running container.
    pub(crate) fn interbase_isql_command(&self) -> String {
        let password = &self.config.engine(EngineId::InterBase).admin_password;
        display_command(&compose_args(
            &self.paths.compose_file(),
            &[
                "exec",
                "interbase",
                ISQL,
                "-user",
                "SYSDBA",
                "-password",
                password,
            ],
        ))
    }

    pub(crate) fn interbase_wizard_ui(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        if !self.interbase_wizard {
            return;
        }
        let register = self.interbase_register_command();
        let isql = self.interbase_isql_command();
        let create = format!(
            "CREATE DATABASE '{}';",
            database_path(self.config.engine(EngineId::InterBase))
        );
        let mut open = true;
        egui::Window::new(t("interbase.title"))
            .id(egui::Id::unique("interbase-wizard"))
            .open(&mut open)
            .collapsible(false)
            .default_width(620.0)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(t("interbase.intro"));
                ui.add_space(8.0);

                ui.label(RichText::new(t("interbase.step1")).strong());
                ui.label(t("interbase.step1_text"));
                ui.hyperlink_to(t("interbase.license_link"), LICENSE_URL);
                ui.add_space(8.0);

                ui.label(RichText::new(t("interbase.step2")).strong());
                ui.label(t("interbase.step2_text"));
                command_box(ui, &register, actions);
                if ui.button(t("interbase.open_terminal")).clicked() {
                    actions.push(Action::OpenTerminal(register.clone()));
                }
                if cfg!(target_arch = "aarch64") {
                    ui.colored_label(WARNING_COLOR, t("interbase.emulation"));
                }
                ui.add_space(8.0);

                ui.label(RichText::new(t("interbase.step3")).strong());
                if ui.button(t("interbase.licensed")).clicked() {
                    actions.push(Action::InterBaseLicensed);
                }
                ui.add_space(8.0);

                ui.label(RichText::new(t("interbase.step4")).strong());
                ui.label(t("interbase.step4_isql"));
                command_box(ui, &isql, actions);
                ui.label(t("interbase.step4_create"));
                command_box(ui, &create, actions);
            });
        if !open {
            self.interbase_wizard = false;
        }
    }
}

/// Read-only monospace command with a copy button.
pub(crate) fn command_box(ui: &mut egui::Ui, command: &str, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        if ui.small_button(t("common.copy")).clicked() {
            actions.push(Action::Copy(command.to_owned()));
        }
        ui.add(
            egui::TextEdit::multiline(&mut &*command)
                .font(egui::TextStyle::Monospace)
                .desired_rows(1)
                .desired_width(f32::INFINITY),
        );
    });
}
