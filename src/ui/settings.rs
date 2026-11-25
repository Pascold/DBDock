//! Settings window of one engine.

use egui::RichText;

use super::theme::{ERROR_COLOR, WARNING_COLOR};
use crate::config::validate::{self, Issue, Problem, Severity};
use crate::config::{AppConfig, EngineSettings};
use crate::docker::status::EngineStatus;
use crate::engines::{EngineId, Field};

/// Unsaved edits of one engine.
pub struct SettingsDraft {
    pub engine: EngineId,
    pub settings: EngineSettings,
    /// Settings as they were when the window opened.
    original: EngineSettings,
    port_text: String,
    show_password: bool,
    show_admin_password: bool,
    /// Cached result of the last port probe: (port, free).
    port_probe: Option<(u16, bool)>,
}

/// What the user decided in the settings window this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsOutcome {
    Open,
    Cancel,
    /// Save without touching the container.
    Save,
    /// Save and recreate the container.
    SaveAndRecreate,
    /// Save, delete the data and start fresh.
    SaveAndReset,
}

impl SettingsDraft {
    pub fn new(engine: EngineId, settings: &EngineSettings) -> Self {
        Self {
            engine,
            settings: settings.clone(),
            original: settings.clone(),
            port_text: settings.port.to_string(),
            show_password: false,
            show_admin_password: false,
            port_probe: None,
        }
    }

    fn port_free(&mut self, status: EngineStatus) -> bool {
        let port = self.settings.port;
        // While our own container publishes the saved port, it is busy by
        // definition, so only other ports are probed.
        if status.is_up() && port == self.original.port {
            return true;
        }
        match self.port_probe {
            Some((probed, free)) if probed == port => free,
            _ => {
                let free = validate::is_port_free(port);
                self.port_probe = Some((port, free));
                free
            }
        }
    }

    /// Draws the window. `status` is the current engine status.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        config: &AppConfig,
        status: EngineStatus,
    ) -> SettingsOutcome {
        let id = self.engine;
        let engine = id.engine();
        let free = self.port_free(status);
        let issues = validate::validate_engine(config, id, &self.settings, |_| free);
        let has_errors = issues.iter().any(|i| i.severity() == Severity::Error);
        let changed = self.settings != self.original;
        let credentials_changed = self.settings.credentials_differ(&self.original);

        let mut outcome = SettingsOutcome::Open;
        let mut open = true;
        egui::Window::new(format!("{} settings", id.display_name()))
            .id(egui::Id::unique(("settings", id.key())))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(460.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                egui::Grid::new("settings-grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        for &field in engine.fields() {
                            self.field_row(ui, field, &issues);
                        }
                    });

                ui.add_space(8.0);
                ui.label(
                    RichText::new(
                        "Changes take effect when the container is recreated. Database name, \
                         user and passwords are applied only when the data is created for the \
                         first time.",
                    )
                    .weak(),
                );
                if credentials_changed && status.is_created() {
                    ui.add_space(4.0);
                    ui.colored_label(
                        WARNING_COLOR,
                        "The data already exists, so the new login settings only take effect \
                         after resetting the data (all data in this database is deleted).",
                    );
                }
                ui.label(
                    RichText::new(
                        "Passwords are stored in plain text in config.toml: use test passwords \
                         only.",
                    )
                    .weak()
                    .small(),
                );

                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Restore defaults").clicked() {
                        let enabled = self.settings.enabled;
                        self.settings = engine.default_settings();
                        self.settings.enabled = enabled;
                        self.port_text = self.settings.port.to_string();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let can_save = changed && !has_errors;
                        if status.is_created() {
                            if credentials_changed
                                && ui
                                    .add_enabled(
                                        can_save,
                                        egui::Button::new("Apply and reset data"),
                                    )
                                    .on_hover_text("Deletes all data of this database")
                                    .clicked()
                            {
                                outcome = SettingsOutcome::SaveAndReset;
                            }
                            if ui
                                .add_enabled(can_save, egui::Button::new("Apply and restart"))
                                .clicked()
                            {
                                outcome = SettingsOutcome::SaveAndRecreate;
                            }
                        }
                        if ui
                            .add_enabled(can_save, egui::Button::new("Save"))
                            .clicked()
                        {
                            outcome = SettingsOutcome::Save;
                        }
                        if ui.button("Cancel").clicked() {
                            outcome = SettingsOutcome::Cancel;
                        }
                    });
                });
            });
        if !open {
            outcome = SettingsOutcome::Cancel;
        }
        outcome
    }

    fn field_row(&mut self, ui: &mut egui::Ui, field: Field, issues: &[Issue]) {
        let id = self.engine;
        ui.label(field_label(id, field));
        ui.vertical(|ui| {
            match field {
                Field::Version => {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.settings.version)
                                .desired_width(160.0),
                        );
                        egui::ComboBox::from_id_salt("version-suggestions")
                            .selected_text("Suggestions")
                            .show_ui(ui, |ui| {
                                for &tag in id.engine().version_suggestions() {
                                    ui.selectable_value(
                                        &mut self.settings.version,
                                        tag.to_owned(),
                                        tag,
                                    );
                                }
                            });
                    });
                    ui.label(
                        RichText::new(format!("Image: {}", id.engine().image(&self.settings)))
                            .weak()
                            .small(),
                    );
                }
                Field::Port => {
                    let response =
                        ui.add(egui::TextEdit::singleline(&mut self.port_text).desired_width(80.0));
                    if response.changed() {
                        self.settings.port = self.port_text.trim().parse().unwrap_or(0);
                    }
                }
                Field::Database => {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.settings.database)
                            .desired_width(220.0),
                    );
                }
                Field::User => {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.settings.user).desired_width(220.0),
                    );
                }
                Field::Password => {
                    password_input(ui, &mut self.settings.password, &mut self.show_password);
                }
                Field::AdminPassword => {
                    password_input(
                        ui,
                        &mut self.settings.admin_password,
                        &mut self.show_admin_password,
                    );
                }
                Field::Seed => {
                    ui.checkbox(
                        &mut self.settings.seed,
                        "Create tables klienci and zamowienia with sample rows",
                    );
                }
            }
            for issue in issues.iter().filter(|i| i.field == field) {
                let color = match issue.severity() {
                    Severity::Error => ERROR_COLOR,
                    Severity::Warning => WARNING_COLOR,
                };
                ui.colored_label(color, issue_text(issue));
            }
        });
        ui.end_row();
    }
}

fn password_input(ui: &mut egui::Ui, value: &mut String, show: &mut bool) {
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(value)
                .password(!*show)
                .desired_width(180.0),
        );
        let label = if *show { "Hide" } else { "Show" };
        if ui.button(label).clicked() {
            *show = !*show;
        }
    });
}

fn field_label(id: EngineId, field: Field) -> String {
    match field {
        Field::Version => "Version (image tag)".into(),
        Field::Port => "Host port".into(),
        Field::Database if id == EngineId::Sqlite => "File name".into(),
        Field::Database => "Database".into(),
        Field::User => "User".into(),
        Field::Password => "Password".into(),
        Field::AdminPassword => {
            format!("{} password", id.engine().admin_user().unwrap_or("Admin"))
        }
        Field::Seed => "Sample data".into(),
    }
}

/// Human readable description of a validation issue.
pub fn issue_text(issue: &Issue) -> String {
    match issue.problem {
        Problem::Empty => "Required.".into(),
        Problem::InvalidPort => "Enter a port between 1 and 65535.".into(),
        Problem::PortConflict(other) => {
            format!("Port already used by {}.", other.display_name())
        }
        Problem::PortInUse => "Another program listens on this port.".into(),
        Problem::WeakMssqlPassword => "SQL Server needs at least 8 characters from three of: \
                                       upper case, lower case, digits, symbols."
            .into(),
        Problem::UnsupportedCharacters => {
            "Use ASCII letters, digits and symbols except ' \" \\ ` ; and spaces.".into()
        }
        Problem::InvalidIdentifier => {
            "Use letters, digits and _ (not starting with a digit).".into()
        }
        Problem::InvalidFileName => "Use letters, digits, '.', '-' and '_' only.".into(),
        Problem::InvalidTag => "Not a valid image tag.".into(),
        Problem::ReservedUser => "root is created automatically; choose another user.".into(),
    }
}
