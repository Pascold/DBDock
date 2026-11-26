//! Settings window of one engine.

use egui::RichText;

use super::theme::{ERROR_COLOR, WARNING_COLOR};
use crate::config::validate::{self, Issue, Problem, Severity};
use crate::config::{AppConfig, EngineSettings};
use crate::docker::status::EngineStatus;
use crate::engines::{EngineId, Field};
use crate::i18n::{t, tf};

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
        // Existing data may not open in another version (always so for a
        // different PostgreSQL major or a downgrade). SQLite keeps a plain
        // file that any version reads.
        let version_changed =
            id != EngineId::Sqlite && self.settings.version != self.original.version;

        let mut outcome = SettingsOutcome::Open;
        let mut open = true;
        egui::Window::new(tf("settings.title", &[("engine", &id.display_name())]))
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
                ui.label(RichText::new(t("settings.recreate_note")).weak());
                if credentials_changed && status.is_created() {
                    ui.add_space(4.0);
                    ui.colored_label(WARNING_COLOR, t("settings.reset_warning"));
                }
                if version_changed && status.is_created() {
                    ui.add_space(4.0);
                    ui.colored_label(WARNING_COLOR, t("settings.version_warning"));
                }
                ui.label(RichText::new(t("settings.plain_text")).weak().small());

                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    if ui.button(t("settings.restore_defaults")).clicked() {
                        let enabled = self.settings.enabled;
                        self.settings = engine.default_settings();
                        self.settings.enabled = enabled;
                        self.port_text = self.settings.port.to_string();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let can_save = changed && !has_errors;
                        if status.is_created() {
                            if (credentials_changed || version_changed)
                                && ui
                                    .add_enabled(
                                        can_save,
                                        egui::Button::new(t("settings.apply_reset")),
                                    )
                                    .on_hover_text(t("settings.apply_reset_hint"))
                                    .clicked()
                            {
                                outcome = SettingsOutcome::SaveAndReset;
                            }
                            if ui
                                .add_enabled(
                                    can_save,
                                    egui::Button::new(t("settings.apply_restart")),
                                )
                                .clicked()
                            {
                                outcome = SettingsOutcome::SaveAndRecreate;
                            }
                        }
                        if ui
                            .add_enabled(can_save, egui::Button::new(t("settings.save")))
                            .clicked()
                        {
                            outcome = SettingsOutcome::Save;
                        }
                        if ui.button(t("common.cancel")).clicked() {
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
                            .selected_text(t("settings.suggestions"))
                            .show_ui(ui, |ui| {
                                for &tag in id.engine().version_suggestions() {
                                    if id.engine().is_eol(tag) {
                                        let label = format!("{tag}   {}", t("settings.eol"));
                                        ui.selectable_value(
                                            &mut self.settings.version,
                                            tag.to_owned(),
                                            label,
                                        )
                                        .on_hover_text(t("settings.eol_hint"));
                                    } else {
                                        ui.selectable_value(
                                            &mut self.settings.version,
                                            tag.to_owned(),
                                            tag,
                                        );
                                    }
                                }
                            });
                        if id.engine().is_eol(&self.settings.version) {
                            ui.label(RichText::new(t("settings.eol")).color(WARNING_COLOR))
                                .on_hover_text(t("settings.eol_hint"));
                        }
                    });
                    ui.label(
                        RichText::new(tf(
                            "settings.image",
                            &[("image", &id.engine().image(&self.settings))],
                        ))
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
                    ui.checkbox(&mut self.settings.seed, t("settings.seed_label"));
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
        let label = if *show {
            t("settings.hide")
        } else {
            t("settings.show")
        };
        if ui.button(label).clicked() {
            *show = !*show;
        }
    });
}

fn field_label(id: EngineId, field: Field) -> String {
    match field {
        Field::Version => t("settings.field.version").into(),
        Field::Port => t("settings.field.port").into(),
        Field::Database if id == EngineId::Sqlite => t("settings.field.file_name").into(),
        Field::Database => t("settings.field.database").into(),
        Field::User => t("settings.field.user").into(),
        Field::Password => t("settings.field.password").into(),
        Field::AdminPassword => tf(
            "settings.field.admin_password",
            &[("user", &id.engine().admin_user().unwrap_or("admin"))],
        ),
        Field::Seed => t("settings.field.seed").into(),
    }
}

/// Human readable description of a validation issue.
pub fn issue_text(issue: &Issue) -> String {
    match issue.problem {
        Problem::Empty => t("issue.empty").into(),
        Problem::InvalidPort => t("issue.invalid_port").into(),
        Problem::PortConflict(other) => {
            tf("issue.port_conflict", &[("engine", &other.display_name())])
        }
        Problem::PortInUse => t("issue.port_in_use").into(),
        Problem::WeakMssqlPassword => t("issue.weak_mssql_password").into(),
        Problem::UnsupportedCharacters => t("issue.unsupported_characters").into(),
        Problem::InvalidIdentifier => t("issue.invalid_identifier").into(),
        Problem::InvalidFileName => t("issue.invalid_file_name").into(),
        Problem::InvalidTag => t("issue.invalid_tag").into(),
        Problem::ReservedUser => t("issue.reserved_user").into(),
    }
}
