//! Screen shown while Docker is unavailable.

use egui::RichText;

use super::theme::ERROR_COLOR;
use crate::app::Action;
use crate::docker::DockerCheck;

pub const DOCKER_DESKTOP_URL: &str = "https://www.docker.com/products/docker-desktop/";
const DOCKER_ENGINE_URL: &str = "https://docs.docker.com/engine/install/";

/// `check` is `None` while the check is still running.
pub fn show(ui: &mut egui::Ui, check: Option<&DockerCheck>, actions: &mut Vec<Action>) {
    egui::CentralPanel::default().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.15);
            let Some(check) = check else {
                ui.spinner();
                ui.label("Checking Docker…");
                return;
            };

            ui.label(
                RichText::new("Docker is not available")
                    .size(24.0)
                    .color(ERROR_COLOR),
            );
            ui.add_space(12.0);
            ui.set_max_width(560.0);
            match check {
                DockerCheck::NotInstalled => {
                    ui.label(
                        "DBDock runs the databases in Docker containers, but the docker \
                         command was not found. Install Docker Desktop (Windows, macOS) or \
                         Docker Engine with the Compose plugin (Linux), then check again.",
                    );
                }
                DockerCheck::DaemonDown(details) => {
                    ui.label(
                        "The docker command was found, but the Docker engine does not answer. \
                         Start Docker Desktop and wait until it reports that the engine is \
                         running. On Linux, make sure the docker service runs and your user may \
                         use it.",
                    );
                    ui.add_space(8.0);
                    ui.label(RichText::new(details).monospace().weak());
                }
                DockerCheck::ComposeMissing => {
                    ui.label(
                        "Docker works, but the Compose plugin (docker compose) is missing. \
                         Docker Desktop includes it; on Linux install the docker-compose-plugin \
                         package.",
                    );
                }
                DockerCheck::Ready { .. } => {}
            }
            ui.add_space(16.0);
            ui.hyperlink_to("Download Docker Desktop", DOCKER_DESKTOP_URL);
            ui.hyperlink_to("Install Docker Engine on Linux", DOCKER_ENGINE_URL);
            ui.add_space(16.0);
            if ui.button("Check again").clicked() {
                actions.push(Action::RecheckDocker);
            }
        });
    });
}
