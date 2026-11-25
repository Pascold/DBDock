//! Screen shown while Docker is unavailable.

use egui::RichText;

use super::theme::ERROR_COLOR;
use crate::app::Action;
use crate::docker::DockerCheck;
use crate::i18n::t;

pub const DOCKER_DESKTOP_URL: &str = "https://www.docker.com/products/docker-desktop/";
const DOCKER_ENGINE_URL: &str = "https://docs.docker.com/engine/install/";

/// `check` is `None` while the check is still running.
pub fn show(ui: &mut egui::Ui, check: Option<&DockerCheck>, actions: &mut Vec<Action>) {
    egui::CentralPanel::default().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.15);
            let Some(check) = check else {
                ui.spinner();
                ui.label(t("docker.checking"));
                return;
            };

            ui.label(
                RichText::new(t("docker.unavailable"))
                    .size(24.0)
                    .color(ERROR_COLOR),
            );
            ui.add_space(12.0);
            ui.set_max_width(560.0);
            match check {
                DockerCheck::NotInstalled => {
                    ui.label(t("docker.not_installed"));
                }
                DockerCheck::DaemonDown(details) => {
                    ui.label(t("docker.daemon_down"));
                    ui.add_space(8.0);
                    ui.label(RichText::new(details).monospace().weak());
                }
                DockerCheck::ComposeMissing => {
                    ui.label(t("docker.compose_missing"));
                }
                DockerCheck::Ready { .. } => {}
            }
            ui.add_space(16.0);
            ui.hyperlink_to(t("docker.download_desktop"), DOCKER_DESKTOP_URL);
            ui.hyperlink_to(t("docker.install_engine"), DOCKER_ENGINE_URL);
            ui.add_space(16.0);
            if ui.button(t("docker.check_again")).clicked() {
                actions.push(Action::RecheckDocker);
            }
        });
    });
}
