//! Connection, logs and reset confirmation windows.

use std::time::{Duration, Instant};

use egui::{Align2, RichText};

use super::theme::ERROR_COLOR;
use crate::app::{Action, DbDockApp};
use crate::engines::EngineId;

/// How often an open logs window refreshes itself.
const LOGS_REFRESH: Duration = Duration::from_secs(3);

/// State of the logs window.
pub struct LogsWindow {
    pub engine: EngineId,
    /// `None` until the first fetch finished.
    pub text: Option<Result<String, String>>,
    pub loading: bool,
    pub auto_refresh: bool,
    pub fetched_at: Option<Instant>,
}

impl LogsWindow {
    pub fn new(engine: EngineId) -> Self {
        Self {
            engine,
            text: None,
            loading: false,
            auto_refresh: true,
            fetched_at: None,
        }
    }

    pub fn needs_refresh(&self) -> bool {
        !self.loading
            && (self.fetched_at.is_none()
                || (self.auto_refresh
                    && self.fetched_at.is_some_and(|t| t.elapsed() >= LOGS_REFRESH)))
    }
}

impl DbDockApp {
    pub(crate) fn connection_window_ui(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let Some(id) = self.connection_window else {
            return;
        };
        let engine = id.engine();
        let info = engine.connection(self.config.engine(id), &self.paths);
        let mut open = true;
        egui::Window::new(format!("Connect to {}", id.display_name()))
            .id(egui::Id::unique(("connection", id.key())))
            .open(&mut open)
            .collapsible(false)
            .default_width(560.0)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                egui::Grid::new("connection-grid")
                    .num_columns(3)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        let mut row = |ui: &mut egui::Ui, label: &str, value: String| {
                            ui.label(label);
                            ui.label(RichText::new(&value).monospace());
                            if ui.small_button("Copy").clicked() {
                                actions.push(Action::Copy(value));
                            }
                            ui.end_row();
                        };
                        if let Some(port) = info.port {
                            row(ui, "Host", info.host.clone());
                            row(ui, "Port", port.to_string());
                            row(ui, "Database", info.database.clone());
                            row(ui, "User", info.user.clone());
                            row(ui, "Password", info.password.clone());
                        } else {
                            row(ui, "Database file", info.database.clone());
                        }
                    });
                ui.separator();
                for (label, value) in &info.strings {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(*label).strong());
                        if ui.small_button("Copy").clicked() {
                            actions.push(Action::Copy(value.clone()));
                        }
                    });
                    ui.add(
                        egui::TextEdit::multiline(&mut value.as_str())
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(1)
                            .desired_width(f32::INFINITY),
                    );
                }
                if let Some(note) = connection_note(id) {
                    ui.add_space(6.0);
                    ui.label(RichText::new(note).weak());
                }
                if id == EngineId::Sqlite {
                    ui.add_space(6.0);
                    if ui.button("Open folder").clicked() {
                        actions.push(Action::OpenFolder(self.paths.sqlite_dir()));
                    }
                }
            });
        if !open {
            self.connection_window = None;
        }
    }

    pub(crate) fn logs_window_ui(&mut self, ctx: &egui::Context) {
        let Some(logs) = &mut self.logs_window else {
            return;
        };
        let mut open = true;
        let mut refresh = false;
        egui::Window::new(format!("{} logs", logs.engine.display_name()))
            .id(egui::Id::unique("logs-window"))
            .open(&mut open)
            .default_width(760.0)
            .default_height(440.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!logs.loading, egui::Button::new("Refresh"))
                        .clicked()
                    {
                        refresh = true;
                    }
                    ui.checkbox(&mut logs.auto_refresh, "Auto refresh");
                    if let Some(Ok(text)) = &logs.text {
                        if ui.button("Copy").clicked() {
                            ui.ctx().copy_text(text.clone());
                        }
                    }
                    if logs.loading {
                        ui.spinner();
                    }
                    ui.label(
                        RichText::new(format!("last {} lines", crate::docker::worker::LOG_LINES))
                            .weak(),
                    );
                });
                ui.separator();
                egui::ScrollArea::both()
                    .stick_to_bottom(true)
                    .auto_shrink([false; 2])
                    .show(ui, |ui| match &logs.text {
                        None => {
                            ui.spinner();
                        }
                        Some(Ok(text)) if text.trim().is_empty() => {
                            ui.label(RichText::new("No log output yet.").weak());
                        }
                        Some(Ok(text)) => {
                            ui.add(egui::Label::new(RichText::new(text).monospace()).extend());
                        }
                        Some(Err(e)) => {
                            ui.colored_label(ERROR_COLOR, RichText::new(e).monospace());
                        }
                    });
            });
        if refresh {
            logs.fetched_at = None;
        }
        if !open {
            self.logs_window = None;
            return;
        }
        if logs.needs_refresh() {
            logs.loading = true;
            let engine = logs.engine;
            self.docker.logs(&self.paths.compose_file(), engine);
        } else if logs.auto_refresh {
            ctx.request_repaint_after(LOGS_REFRESH);
        }
    }

    pub(crate) fn reset_confirm_ui(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let Some(id) = self.confirm_reset else {
            return;
        };
        let mut close = false;
        let modal = egui::Modal::new(egui::Id::unique("reset-confirm")).show(ctx, |ui| {
            ui.set_max_width(420.0);
            ui.heading(format!("Reset {}?", id.display_name()));
            ui.add_space(6.0);
            ui.label(if id == EngineId::Sqlite {
                "The database file is deleted. It is created again, with the current settings, \
                 on the next start."
            } else {
                "The container and its data volume are deleted, including every table and \
                 row. The database is initialised again, with the current settings, on the \
                 next start."
            });
            if id == EngineId::InterBase {
                ui.label("The registered license is kept.");
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let delete = egui::Button::new(RichText::new("Delete data").color(ERROR_COLOR));
                if ui.add(delete).clicked() {
                    actions.push(Action::ResetConfirmed(id));
                    close = true;
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
        if close || modal.should_close() {
            self.confirm_reset = None;
        }
    }
}

fn connection_note(id: EngineId) -> Option<&'static str> {
    match id {
        EngineId::MsSql => Some(
            "SQL Server uses a self-signed certificate: enable TrustServerCertificate \
             (or -C for sqlcmd) in your client.",
        ),
        EngineId::Firebird => Some(
            "Wire encryption is enabled. Firebird 3+ clients connect with the default \
             settings; older clients may need WireCrypt = Enabled.",
        ),
        EngineId::Sqlite => {
            Some("SQLite has no server: open the file directly. It is created on the first start.")
        }
        _ => None,
    }
}
