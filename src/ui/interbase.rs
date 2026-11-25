//! First-run wizard for InterBase (license registration).

use egui::{Align2, RichText};

use super::theme::WARNING_COLOR;
use crate::app::{Action, DbDockApp};
use crate::docker::{compose_args, display_command};
use crate::engines::interbase::{database_path, ISQL};
use crate::engines::EngineId;

pub const LICENSE_URL: &str = "https://www.embarcadero.com/products/interbase/start-for-free";

impl DbDockApp {
    /// Command that runs the container interactively so the license manager
    /// can ask for the serial number.
    pub(crate) fn interbase_register_command(&self) -> String {
        display_command(&compose_args(
            &self.paths.compose_file(),
            &["run", "--rm", "interbase"],
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
        egui::Window::new("InterBase setup")
            .id(egui::Id::unique("interbase-wizard"))
            .open(&mut open)
            .collapsible(false)
            .default_width(620.0)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(
                    "InterBase is commercial software. The official Docker image needs a \
                     license, and the free Developer Edition license is enough for testing.",
                );
                ui.add_space(8.0);

                ui.label(RichText::new("1. Get a free Developer Edition license").strong());
                ui.label(
                    "Register on the Embarcadero website. The serial number arrives by e-mail; \
                     keep your Embarcadero account (EDN) login at hand as well.",
                );
                ui.hyperlink_to("InterBase Developer Edition", LICENSE_URL);
                ui.add_space(8.0);

                ui.label(RichText::new("2. Register the license (once)").strong());
                ui.label(
                    "Run this command in a terminal. The license manager starts and asks for the \
                     serial number and your account. The license is stored in the \
                     dbdock_interbase-license volume and survives resets.",
                );
                command_box(ui, &register, actions);
                ui.horizontal(|ui| {
                    if ui.button("Open a terminal with this command").clicked() {
                        actions.push(Action::OpenTerminal(register.clone()));
                    }
                });
                if cfg!(target_arch = "aarch64") {
                    ui.colored_label(
                        WARNING_COLOR,
                        "The image is x86-64 only and runs under emulation on this computer.",
                    );
                }
                ui.add_space(8.0);

                ui.label(RichText::new("3. Start InterBase").strong());
                if ui.button("License registered – start normally").clicked() {
                    actions.push(Action::InterBaseLicensed);
                }
                ui.add_space(8.0);

                ui.label(RichText::new("4. Create a database").strong());
                ui.label("InterBase starts without a database. Open isql in the container:");
                command_box(ui, &isql, actions);
                ui.label("and create one (stored in the persistent /opt/interbase volume):");
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
        if ui.small_button("Copy").clicked() {
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
