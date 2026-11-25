//! Application state and the glue between the UI and the Docker worker.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::compose::{self, ComposeContext};
use crate::config::validate::{self, Severity};
use crate::config::{AppConfig, Theme};
use crate::docker::status::{engine_status, ContainerState, EngineStatus};
use crate::docker::worker::{Docker, Event, OpId, OpKind};
use crate::docker::DockerCheck;
use crate::engines::EngineId;
use crate::paths::AppPaths;
use crate::ui::toasts::Toasts;

/// How many finished operations are remembered for the output window.
const KEPT_OPERATIONS: usize = 20;

/// A Docker operation started from the UI.
pub struct Operation {
    pub id: OpId,
    pub kind: OpKind,
    /// Empty means "all engines" (Stop all).
    pub engines: Vec<EngineId>,
    pub output: Vec<String>,
    /// `None` while running.
    pub result: Option<Result<(), String>>,
    /// Engine to disable once the operation succeeds.
    pub disable_after: Option<EngineId>,
}

impl Operation {
    pub fn is_running(&self) -> bool {
        self.result.is_none()
    }

    pub fn affects(&self, id: EngineId) -> bool {
        self.engines.is_empty() || self.engines.contains(&id)
    }
}

/// Something the user asked for while the UI was drawn. Applied after the
/// frame's UI code so drawing only needs shared access to the state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Op(OpKind, Vec<EngineId>),
    StartAllEnabled,
    StopAll,
    SetEnabled(EngineId, bool),
    ShowOutput(OpId),
    RecheckDocker,
    SetTheme(Theme),
}

pub struct DbDockApp {
    pub(crate) paths: AppPaths,
    pub(crate) config: AppConfig,
    pub(crate) docker: Docker,
    /// `None` while the check runs.
    pub(crate) docker_check: Option<DockerCheck>,
    pub(crate) containers: Vec<ContainerState>,
    pub(crate) statuses: BTreeMap<EngineId, EngineStatus>,
    /// Set when the last status refresh failed.
    pub(crate) status_error: Option<String>,
    pub(crate) operations: Vec<Operation>,
    pub(crate) toasts: Toasts,
    /// Operation shown in the output window.
    pub(crate) output_window: Option<OpId>,
}

impl DbDockApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = cc.egui_ctx.clone();
        let mut toasts = Toasts::default();

        let paths = AppPaths::detect().unwrap_or_else(|| {
            toasts.error("Cannot determine the configuration folder; using ./DBDock.");
            AppPaths::new("DBDock")
        });
        let config = load_config(&paths, &mut toasts);

        let docker = Docker::new(move || ctx.request_repaint());
        docker.check();

        let app = Self {
            paths,
            config,
            docker,
            docker_check: None,
            containers: Vec::new(),
            statuses: BTreeMap::new(),
            status_error: None,
            operations: Vec::new(),
            toasts,
            output_window: None,
        };
        app.apply_theme(&cc.egui_ctx);
        app
    }

    pub(crate) fn status(&self, id: EngineId) -> EngineStatus {
        self.statuses
            .get(&id)
            .copied()
            .unwrap_or(EngineStatus::NotCreated)
    }

    /// The running operation affecting `id`, if any.
    pub(crate) fn running_op(&self, id: EngineId) -> Option<&Operation> {
        self.operations
            .iter()
            .rev()
            .find(|op| op.is_running() && op.affects(id))
    }

    pub(crate) fn operation(&self, id: OpId) -> Option<&Operation> {
        self.operations.iter().find(|op| op.id == id)
    }

    pub(crate) fn any_running(&self) -> bool {
        self.operations.iter().any(Operation::is_running)
    }

    fn apply_theme(&self, ctx: &egui::Context) {
        ctx.set_theme(match self.config.theme {
            Theme::Dark => egui::Theme::Dark,
            Theme::Light => egui::Theme::Light,
        });
    }

    pub(crate) fn save_config(&mut self) {
        if let Err(e) = self.config.save(&self.paths.config_file()) {
            self.toasts.error(format!("Cannot save settings: {e}"));
        }
    }

    /// Writes the compose file and its helper files.
    fn write_compose(&mut self) -> Option<PathBuf> {
        let ctx = ComposeContext::for_host(&self.paths);
        match compose::write_files(&self.config, &ctx) {
            Ok(path) => Some(path),
            Err(e) => {
                self.toasts
                    .error(format!("Cannot write docker-compose.yml: {e}"));
                None
            }
        }
    }

    // -----------------------------------------------------------------------
    // Events from the Docker worker
    // -----------------------------------------------------------------------

    pub(crate) fn process_events(&mut self) {
        for event in self.docker.drain() {
            match event {
                Event::Checked(check) => self.on_docker_checked(check),
                Event::Status(Ok(containers)) => {
                    self.containers = containers;
                    self.status_error = None;
                    self.update_statuses();
                }
                Event::Status(Err(e)) => self.status_error = Some(e),
                Event::Output { op, line } => {
                    if let Some(op) = self.operations.iter_mut().find(|o| o.id == op) {
                        op.output.push(line);
                    }
                }
                Event::Finished { op, result } => self.on_finished(op, result),
                Event::Logs { .. } => {}
            }
        }
    }

    fn on_docker_checked(&mut self, check: DockerCheck) {
        if check.is_ready() {
            if let Some(file) = self.write_compose() {
                self.docker.start_polling(file);
            }
        }
        self.docker_check = Some(check);
    }

    fn update_statuses(&mut self) {
        let sqlite_file = crate::engines::sqlite::database_file(
            self.config.engine(EngineId::Sqlite),
            &self.paths,
        );
        let sqlite_exists = sqlite_file.is_file();
        self.statuses = EngineId::ALL
            .into_iter()
            .map(|id| (id, engine_status(id, &self.containers, sqlite_exists)))
            .collect();
    }

    fn on_finished(&mut self, id: OpId, result: Result<(), String>) {
        let Some(index) = self.operations.iter().position(|o| o.id == id) else {
            return;
        };
        let op = &mut self.operations[index];
        op.result = Some(result.clone());
        let disable = op.disable_after.take();
        let names = engine_names(&op.engines);
        let kind = op.kind;

        match result {
            Ok(()) => {
                if let Some(engine) = disable {
                    self.config.engine_mut(engine).enabled = false;
                    self.save_config();
                    if !self.any_running() {
                        self.write_compose();
                    }
                }
                self.toasts.success(format!("{}: {names}", done_text(kind)));
            }
            Err(e) => self.toasts.error_with_output(
                format!("{} {names} failed: {}", kind_text(kind), last_line(&e)),
                id,
            ),
        }
        // Forget old finished operations.
        let finished = self.operations.iter().filter(|o| !o.is_running()).count();
        if finished > KEPT_OPERATIONS {
            if let Some(oldest) = self.operations.iter().position(|o| !o.is_running()) {
                self.operations.remove(oldest);
            }
        }
        self.update_statuses();
    }

    // -----------------------------------------------------------------------
    // Actions
    // -----------------------------------------------------------------------

    pub(crate) fn apply(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::Op(kind, engines) => {
                self.start_op(kind, engines, None);
            }
            Action::StartAllEnabled => {
                let engines: Vec<_> = compose::enabled(&self.config).collect();
                if engines.is_empty() {
                    self.toasts.info("No database is enabled.");
                } else {
                    self.start_op(OpKind::Start, engines, None);
                }
            }
            Action::StopAll => {
                self.start_op(OpKind::Stop, Vec::new(), None);
            }
            Action::SetEnabled(id, enabled) => self.set_enabled(id, enabled),
            Action::ShowOutput(op) => self.output_window = Some(op),
            Action::RecheckDocker => {
                self.docker_check = None;
                self.docker.check();
            }
            Action::SetTheme(theme) => {
                self.config.theme = theme;
                self.apply_theme(ctx);
                self.save_config();
            }
        }
    }

    fn set_enabled(&mut self, id: EngineId, enabled: bool) {
        if !enabled && self.status(id).is_created() {
            // Remove the containers while the service is still in the
            // compose file; the data volume is kept.
            self.start_op(OpKind::Remove, vec![id], Some(id));
            return;
        }
        self.config.engine_mut(id).enabled = enabled;
        self.save_config();
    }

    /// Validates, writes the compose file and starts the operation.
    pub(crate) fn start_op(
        &mut self,
        kind: OpKind,
        engines: Vec<EngineId>,
        disable_after: Option<EngineId>,
    ) -> Option<OpId> {
        if !self
            .docker_check
            .as_ref()
            .is_some_and(DockerCheck::is_ready)
        {
            self.toasts.error("Docker is not available.");
            return None;
        }
        if let Some(busy) = engines.iter().find(|&&id| self.running_op(id).is_some()) {
            self.toasts.info(format!(
                "{} is busy, wait for the current operation.",
                busy.display_name()
            ));
            return None;
        }
        if matches!(kind, OpKind::Start | OpKind::Recreate | OpKind::Restart) {
            let errors: Vec<String> = validate::validate_enabled(&self.config)
                .into_iter()
                .filter(|(id, issue)| engines.contains(id) && issue.severity() == Severity::Error)
                .map(|(id, issue)| format!("{}: {:?}", id.display_name(), issue.problem))
                .collect();
            if !errors.is_empty() {
                self.toasts
                    .error(format!("Fix the settings first.\n{}", errors.join("\n")));
                return None;
            }
        }
        let file = self.write_compose()?;
        let id = self.docker.run_op(kind, &file, &engines);
        self.operations.push(Operation {
            id,
            kind,
            engines,
            output: Vec::new(),
            result: None,
            disable_after,
        });
        Some(id)
    }
}

impl eframe::App for DbDockApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.process_events();
        let ctx = ui.ctx().clone();
        let mut actions = Vec::new();

        self.top_bar(ui, &mut actions);
        match &self.docker_check {
            Some(check) if check.is_ready() => self.cards(ui, &mut actions),
            check => crate::ui::docker_missing::show(ui, check.as_ref(), &mut actions),
        }
        self.output_window_ui(&ctx);
        self.toasts.show(&ctx, &mut actions);

        for action in actions {
            self.apply(&ctx, action);
        }
    }
}

/// Loads the configuration. A broken file is kept as `config.toml.broken`
/// and replaced by defaults so the app still starts.
fn load_config(paths: &AppPaths, toasts: &mut Toasts) -> AppConfig {
    let file = paths.config_file();
    match AppConfig::load(&file) {
        Ok(config) => config,
        Err(e) => {
            let backup = file.with_extension("toml.broken");
            let kept = std::fs::rename(&file, &backup).is_ok();
            toasts.error(if kept {
                format!(
                    "{e}\nDefaults are used; the old file was saved as {}.",
                    backup.display()
                )
            } else {
                format!("{e}\nDefaults are used.")
            });
            AppConfig::default()
        }
    }
}

pub(crate) fn engine_names(engines: &[EngineId]) -> String {
    if engines.is_empty() {
        return "all databases".to_owned();
    }
    engines
        .iter()
        .map(|id| id.display_name())
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn kind_text(kind: OpKind) -> &'static str {
    match kind {
        OpKind::Start => "Starting",
        OpKind::Stop => "Stopping",
        OpKind::Restart => "Restarting",
        OpKind::Recreate => "Applying settings to",
        OpKind::Reset => "Resetting",
        OpKind::Remove => "Removing",
    }
}

fn done_text(kind: OpKind) -> &'static str {
    match kind {
        OpKind::Start => "Started",
        OpKind::Stop => "Stopped",
        OpKind::Restart => "Restarted",
        OpKind::Recreate => "Settings applied",
        OpKind::Reset => "Data reset",
        OpKind::Remove => "Removed",
    }
}

fn last_line(text: &str) -> &str {
    text.lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or(text)
        .trim()
}
