//! Application state and the glue between the UI and the Docker worker.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::compose::{self, ComposeContext};
use crate::config::validate::{self, Severity};
use crate::config::{AppConfig, Language, Theme};
use crate::docker::status::{engine_status, ContainerState, EngineStatus};
use crate::docker::worker::{Docker, Event, OpId, OpKind};
use crate::docker::DockerCheck;
use crate::engines::EngineId;
use crate::i18n::{self, t, tf};
use crate::paths::AppPaths;
use crate::ui::settings::{issue_text, SettingsDraft, SettingsOutcome};
use crate::ui::toasts::Toasts;
use crate::ui::windows::LogsWindow;
use crate::update::{self, Release};

/// How many finished operations are remembered for the output window.
const KEPT_OPERATIONS: usize = 20;
/// How often DBDock asks GitHub for a new release while it stays open.
const UPDATE_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

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
    /// Operation to start on the same engines once this one succeeds.
    pub then: Option<OpKind>,
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
    OpenSettings(EngineId),
    OpenConnection(EngineId),
    OpenLogs(EngineId),
    /// Run the user's scripts on the existing database.
    RunScripts(EngineId),
    OpenSnapshots(EngineId),
    SaveSnapshot(EngineId),
    /// Engine and snapshot name.
    RestoreSnapshot(EngineId, String),
    DeleteSnapshot(EngineId, String),
    /// Copy text to the clipboard.
    Copy(String),
    /// Copy the main connection string of an engine.
    CopyConnection(EngineId),
    /// Ask before deleting data.
    ConfirmReset(EngineId),
    ResetConfirmed(EngineId),
    OpenFolder(PathBuf),
    ExportCompose,
    OpenInterBaseWizard,
    /// Run a command in a new terminal window.
    OpenTerminal(String),
    InterBaseLicensed,
    RecheckDocker,
    SetTheme(Theme),
    SetLanguage(Language),
    SetCheckUpdates(bool),
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
    pub(crate) settings: Option<SettingsDraft>,
    pub(crate) connection_window: Option<EngineId>,
    pub(crate) logs_window: Option<LogsWindow>,
    pub(crate) snapshots_window: Option<crate::ui::snapshots::SnapshotsWindow>,
    pub(crate) confirm_reset: Option<EngineId>,
    pub(crate) interbase_wizard: bool,
    /// Kept alive: on X11 the clipboard content disappears with its owner.
    clipboard: Option<arboard::Clipboard>,
    /// Result of the background update check, received once.
    update_rx: Option<std::sync::mpsc::Receiver<Option<Release>>>,
    /// When the last update check started; repeated every
    /// [`UPDATE_INTERVAL`] while DBDock stays open.
    update_checked: Option<Instant>,
    /// A newer release, if one exists.
    pub(crate) update: Option<Release>,
}

impl DbDockApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = cc.egui_ctx.clone();
        let mut toasts = Toasts::default();

        let paths = AppPaths::detect().unwrap_or_else(|| {
            toasts.error(t("app.no_config_dir"));
            AppPaths::new("DBDock")
        });
        let config = load_config(&paths, &mut toasts);
        i18n::set_language(config.language.unwrap_or_else(i18n::system_language));
        let update_rx = config
            .check_updates
            .then(|| spawn_update_check(&cc.egui_ctx));

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
            settings: None,
            connection_window: None,
            logs_window: None,
            snapshots_window: None,
            confirm_reset: None,
            interbase_wizard: false,
            clipboard: None,
            update_checked: update_rx.as_ref().map(|_| Instant::now()),
            update_rx,
            update: None,
        };
        crate::ui::theme::apply_style(&cc.egui_ctx);
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
            self.toasts.error(tf("app.save_failed", &[("error", &e)]));
        }
    }

    /// Writes the compose file and its helper files.
    fn start_update_check(&mut self, ctx: &egui::Context) {
        self.update_rx = Some(spawn_update_check(ctx));
        self.update_checked = Some(Instant::now());
    }

    /// Checks for a new release again once a day while DBDock stays open.
    fn recheck_updates(&mut self, ctx: &egui::Context) {
        if !self.config.check_updates || self.update_rx.is_some() {
            return;
        }
        match self.update_checked.map(|at| at.elapsed()) {
            Some(elapsed) if elapsed >= UPDATE_INTERVAL => self.start_update_check(ctx),
            // Wake up for the next check even if nothing else repaints.
            Some(elapsed) => ctx.request_repaint_after(UPDATE_INTERVAL - elapsed),
            None => {}
        }
    }

    fn write_compose(&mut self) -> Option<PathBuf> {
        let ctx = ComposeContext::for_host(&self.paths);
        match compose::write_files(&self.config, &ctx) {
            Ok(path) => Some(path),
            Err(e) => {
                self.toasts
                    .error(tf("app.compose_failed", &[("error", &e)]));
                None
            }
        }
    }

    // -----------------------------------------------------------------------
    // Events from the Docker worker
    // -----------------------------------------------------------------------

    pub(crate) fn process_events(&mut self) {
        if let Some(rx) = &self.update_rx {
            match rx.try_recv() {
                Ok(release) => {
                    // A failed later check keeps the notice already shown.
                    if release.is_some() {
                        self.update = release;
                    }
                    self.update_rx = None;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => self.update_rx = None,
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }
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
                Event::Logs { engine, result } => {
                    if let Some(logs) = self.logs_window.as_mut().filter(|l| l.engine == engine) {
                        logs.text = Some(result);
                        logs.loading = false;
                        logs.fetched_at = Some(std::time::Instant::now());
                    }
                }
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
        let then = op.then.take();
        let engines = op.engines.clone();
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
                if let Some(next) = then {
                    self.start_op(next, engines, None);
                } else {
                    self.toasts
                        .success(tf(done_key(kind), &[("names", &names)]));
                }
            }
            Err(e) => self.toasts.error_with_output(
                tf("op.failed", &[("names", &names), ("error", &last_line(&e))]),
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
        if matches!(kind, OpKind::SaveSnapshot | OpKind::RestoreSnapshot) {
            self.refresh_snapshots();
        }
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
                // InterBase is enabled by its wizard before the license exists.
                let licensed = self.config.interbase_licensed;
                let engines: Vec<_> = compose::enabled(&self.config)
                    .filter(|&id| id != EngineId::InterBase || licensed)
                    .collect();
                if engines.is_empty() {
                    self.toasts.info(t("app.none_enabled"));
                } else {
                    self.start_op(OpKind::Start, engines, None);
                }
            }
            Action::StopAll => {
                self.start_op(OpKind::Stop, Vec::new(), None);
            }
            Action::SetEnabled(id, enabled) => self.set_enabled(id, enabled),
            Action::ShowOutput(op) => self.output_window = Some(op),
            Action::OpenConnection(id) => self.connection_window = Some(id),
            Action::OpenLogs(id) => self.logs_window = Some(LogsWindow::new(id)),
            Action::RunScripts(id) => self.run_scripts(id),
            Action::OpenSnapshots(id) => {
                self.snapshots_window =
                    Some(crate::ui::snapshots::SnapshotsWindow::new(id, &self.paths));
            }
            Action::SaveSnapshot(id) => self.save_snapshot(id),
            Action::RestoreSnapshot(id, name) => self.restore_snapshot(id, &name),
            Action::DeleteSnapshot(id, name) => self.delete_snapshot(id, &name),
            Action::Copy(text) => self.copy(ctx, text),
            Action::CopyConnection(id) => {
                let info = id.engine().connection(self.config.engine(id), &self.paths);
                if let Some((_, value)) = info.strings.into_iter().next() {
                    self.copy(ctx, value);
                }
            }
            Action::ConfirmReset(id) => self.confirm_reset = Some(id),
            Action::ResetConfirmed(id) => {
                self.reset(id);
            }
            Action::OpenFolder(path) => self.open_folder(&path),
            Action::ExportCompose => self.export_compose(),
            Action::OpenInterBaseWizard => self.apply_interbase_wizard(),
            Action::OpenTerminal(command) => {
                if let Err(e) = crate::terminal::open_with_command(&command) {
                    self.toasts
                        .error(tf("app.terminal_failed", &[("error", &e)]));
                }
            }
            Action::InterBaseLicensed => {
                self.config.interbase_licensed = true;
                self.save_config();
                self.interbase_wizard = false;
                self.start_op(OpKind::Start, vec![EngineId::InterBase], None);
            }
            Action::OpenSettings(id) => {
                self.settings = Some(SettingsDraft::new(id, self.config.engine(id)));
            }
            Action::RecheckDocker => {
                self.docker_check = None;
                self.docker.check();
            }
            Action::SetCheckUpdates(enabled) => {
                self.config.check_updates = enabled;
                self.save_config();
                if !enabled {
                    self.update = None;
                    self.update_rx = None;
                } else if self.update.is_none() && self.update_rx.is_none() {
                    self.start_update_check(ctx);
                }
            }
            Action::SetLanguage(language) => {
                self.config.language = Some(language);
                i18n::set_language(language);
                self.save_config();
            }
            Action::SetTheme(theme) => {
                self.config.theme = theme;
                self.apply_theme(ctx);
                self.save_config();
            }
        }
    }

    /// Enables InterBase (its service must be in the compose file for the
    /// registration command) and opens the license wizard.
    fn apply_interbase_wizard(&mut self) {
        if !self.config.engine(EngineId::InterBase).enabled {
            self.config.engine_mut(EngineId::InterBase).enabled = true;
            self.save_config();
        }
        self.write_compose();
        self.interbase_wizard = true;
    }

    fn set_enabled(&mut self, id: EngineId, enabled: bool) {
        if !enabled && self.status(id).is_created() {
            // Remove the containers while the service is still in the
            // compose file; the data volume is kept.
            self.start_op(OpKind::Remove, vec![id], Some(id));
            return;
        }
        if enabled && id == EngineId::InterBase && !self.config.interbase_licensed {
            // InterBase cannot start before its license is registered.
            self.apply_interbase_wizard();
            return;
        }
        self.config.engine_mut(id).enabled = enabled;
        self.save_config();
    }

    fn settings_window_ui(&mut self, ctx: &egui::Context) {
        let Some(mut draft) = self.settings.take() else {
            return;
        };
        let id = draft.engine;
        let status = self.status(id);
        let outcome = draft.show(ctx, &self.config, status);
        if outcome == SettingsOutcome::Open {
            self.settings = Some(draft);
            return;
        }
        if outcome == SettingsOutcome::Cancel {
            return;
        }

        // The enabled switch lives on the card; keep its current value.
        let mut settings = draft.settings;
        settings.enabled = self.config.engine(id).enabled;
        *self.config.engine_mut(id) = settings;
        self.save_config();
        match outcome {
            SettingsOutcome::SaveAndRecreate => {
                self.start_op(OpKind::Recreate, vec![id], None);
            }
            SettingsOutcome::SaveAndReset => self.reset_and_start(id),
            _ => self
                .toasts
                .success(tf("app.settings_saved", &[("engine", &id.display_name())])),
        }
    }

    fn copy(&mut self, ctx: &egui::Context, text: String) {
        if self.clipboard.is_none() {
            self.clipboard = arboard::Clipboard::new().ok();
        }
        let copied = self
            .clipboard
            .as_mut()
            .is_some_and(|c| c.set_text(text.clone()).is_ok());
        if !copied {
            // Fall back to the clipboard integration of the window backend.
            ctx.copy_text(text);
        }
        self.toasts.info(t("app.copied"));
    }

    fn open_folder(&mut self, path: &std::path::Path) {
        if let Err(e) = std::fs::create_dir_all(path) {
            self.toasts.error(tf(
                "app.create_dir_failed",
                &[("path", &path.display()), ("error", &e)],
            ));
            return;
        }
        if let Err(e) = open::that_detached(path) {
            self.toasts.error(tf(
                "app.open_failed",
                &[("path", &path.display()), ("error", &e)],
            ));
        }
    }

    /// Copies the generated compose file to the Downloads folder so it can be
    /// used without DBDock.
    fn export_compose(&mut self) {
        let Some(file) = self.write_compose() else {
            return;
        };
        let target_dir = directories::UserDirs::new()
            .and_then(|d| d.download_dir().map(std::path::Path::to_path_buf))
            .unwrap_or_else(|| self.paths.base().to_path_buf());
        let target = target_dir.join("dbdock-docker-compose.yml");
        match std::fs::copy(&file, &target) {
            Ok(_) => {
                self.toasts.success(tf(
                    "app.exported",
                    &[
                        ("path", &target.display()),
                        ("seeds", &self.paths.seed_dir().display()),
                    ],
                ));
                self.open_folder(&target_dir);
            }
            Err(e) => self.toasts.error(tf(
                "app.export_failed",
                &[("path", &target.display()), ("error", &e)],
            )),
        }
    }

    /// Deletes the data of `id` and starts it again with the current settings.
    pub(crate) fn reset_and_start(&mut self, id: EngineId) {
        if let Some(op) = self.reset(id) {
            if let Some(op) = self.operations.iter_mut().find(|o| o.id == op) {
                op.then = Some(OpKind::Start);
            }
        }
    }

    /// Removes the container and data of `id`.
    pub(crate) fn reset(&mut self, id: EngineId) -> Option<OpId> {
        if id == EngineId::Sqlite {
            let file = crate::engines::sqlite::database_file(self.config.engine(id), &self.paths);
            if let Err(e) = std::fs::remove_file(&file) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    self.toasts.error(tf(
                        "app.delete_failed",
                        &[("path", &file.display()), ("error", &e)],
                    ));
                    return None;
                }
            }
        }
        self.start_op(OpKind::Reset, vec![id], None)
    }

    /// Validates, writes the compose file and starts the operation.
    pub(crate) fn start_op(
        &mut self,
        kind: OpKind,
        engines: Vec<EngineId>,
        disable_after: Option<EngineId>,
    ) -> Option<OpId> {
        self.begin_op(kind, engines, disable_after, None)
    }

    /// Starts an operation whose Docker invocations the caller prepared
    /// (see [`OpKind::is_planned`]).
    fn start_planned(
        &mut self,
        kind: OpKind,
        id: EngineId,
        commands: Vec<Vec<String>>,
    ) -> Option<OpId> {
        self.begin_op(kind, vec![id], None, Some(commands))
    }

    fn begin_op(
        &mut self,
        kind: OpKind,
        engines: Vec<EngineId>,
        disable_after: Option<EngineId>,
        plan: Option<Vec<Vec<String>>>,
    ) -> Option<OpId> {
        if !self
            .docker_check
            .as_ref()
            .is_some_and(DockerCheck::is_ready)
        {
            self.toasts.error(t("app.docker_unavailable"));
            return None;
        }
        if let Some(busy) = engines.iter().find(|&&id| self.running_op(id).is_some()) {
            self.toasts
                .info(tf("app.busy", &[("engine", &busy.display_name())]));
            return None;
        }
        if matches!(kind, OpKind::Start | OpKind::Recreate | OpKind::Restart) {
            let errors: Vec<String> = validate::validate_enabled(&self.config)
                .into_iter()
                .filter(|(id, issue)| engines.contains(id) && issue.severity() == Severity::Error)
                .map(|(id, issue)| format!("{}: {}", id.display_name(), issue_text(&issue)))
                .collect();
            if !errors.is_empty() {
                self.toasts
                    .error(tf("app.fix_settings", &[("issues", &errors.join("\n"))]));
                return None;
            }
        }
        // Also refreshes the init folders that "Run now" reads.
        let file = self.write_compose()?;
        let id = match plan {
            Some(commands) => self.docker.run_commands(commands),
            None => self.docker.run_op(kind, &file, &engines),
        };
        self.operations.push(Operation {
            id,
            kind,
            engines,
            output: Vec::new(),
            result: None,
            disable_after,
            then: None,
        });
        Some(id)
    }

    // -----------------------------------------------------------------------
    // Scripts and snapshots
    // -----------------------------------------------------------------------

    fn run_scripts(&mut self, id: EngineId) {
        let settings = self.config.engine(id);
        let Some(args) = id.engine().run_scripts_args(settings) else {
            return;
        };
        if crate::compose::user_scripts(&self.paths, id).is_empty() {
            self.toasts.info(tf(
                "app.no_scripts",
                &[("path", &self.paths.scripts_dir(id).display())],
            ));
            return;
        }
        // SQLite needs its file, servers need to run.
        let ready = match id {
            EngineId::Sqlite => self.status(id) == EngineStatus::Healthy,
            _ => self.status(id).is_up(),
        };
        if !ready {
            self.toasts.info(tf(
                "app.scripts_not_running",
                &[("engine", &id.display_name())],
            ));
            return;
        }
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let command = crate::docker::compose_args(&self.paths.compose_file(), &args);
        if let Some(op) = self.start_planned(OpKind::RunScripts, id, vec![command]) {
            // The output of the scripts is what the user wants to see.
            self.output_window = Some(op);
        }
    }

    /// Engines with a server that runs now; snapshots stop and restart it.
    fn server_up(&self, id: EngineId) -> bool {
        id != EngineId::Sqlite && self.status(id).is_up()
    }

    fn save_snapshot(&mut self, id: EngineId) {
        let settings = self.config.engine(id).clone();
        let snapshot = match crate::snapshots::prepare(&self.paths, id, &settings) {
            Ok(snapshot) => snapshot,
            Err(e) => {
                self.toasts.error(tf(
                    "app.create_dir_failed",
                    &[
                        ("path", &self.paths.snapshots_dir(id).display()),
                        ("error", &e),
                    ],
                ));
                return;
            }
        };
        let owner = ComposeContext::for_host(&self.paths).owner;
        let commands = crate::snapshots::save_commands(
            &self.paths,
            id,
            &settings,
            &snapshot,
            self.server_up(id),
            owner,
        );
        self.start_planned(OpKind::SaveSnapshot, id, commands);
        self.refresh_snapshots();
    }

    fn restore_snapshot(&mut self, id: EngineId, name: &str) {
        let settings = self.config.engine(id).clone();
        let Some(snapshot) = crate::snapshots::list(&self.paths, id)
            .into_iter()
            .find(|s| s.name == name)
        else {
            self.refresh_snapshots();
            return;
        };
        if !snapshot.complete || !snapshot.fits(id, &settings) {
            return;
        }
        let owner = ComposeContext::for_host(&self.paths).owner;
        let commands = crate::snapshots::restore_commands(
            &self.paths,
            id,
            &settings,
            &snapshot,
            self.server_up(id),
            owner,
        );
        self.start_planned(OpKind::RestoreSnapshot, id, commands);
    }

    fn delete_snapshot(&mut self, id: EngineId, name: &str) {
        if let Some(snapshot) = crate::snapshots::list(&self.paths, id)
            .into_iter()
            .find(|s| s.name == name)
        {
            if let Err(e) = crate::snapshots::delete(&snapshot) {
                self.toasts.error(tf(
                    "app.delete_failed",
                    &[("path", &snapshot.dir.display()), ("error", &e)],
                ));
            }
        }
        self.refresh_snapshots();
    }
}

impl eframe::App for DbDockApp {
    /// Only the window size and position are persisted (by eframe, in
    /// `window.ron`); the UI state always starts fresh.
    fn persist_egui_memory(&self) -> bool {
        false
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.process_events();
        let ctx = ui.ctx().clone();
        self.recheck_updates(&ctx);
        let mut actions = Vec::new();

        self.top_bar(ui, &mut actions);
        self.footer(ui, &mut actions);
        match &self.docker_check {
            Some(check) if check.is_ready() => self.cards(ui, &mut actions),
            check => crate::ui::docker_missing::show(ui, check.as_ref(), &mut actions),
        }
        self.settings_window_ui(&ctx);
        self.connection_window_ui(&ctx, &mut actions);
        self.logs_window_ui(&ctx);
        self.snapshots_window_ui(&ctx, &mut actions);
        self.reset_confirm_ui(&ctx, &mut actions);
        self.interbase_wizard_ui(&ctx, &mut actions);
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
                tf(
                    "app.config_broken_backup",
                    &[("error", &e), ("path", &backup.display())],
                )
            } else {
                tf("app.config_broken", &[("error", &e)])
            });
            AppConfig::default()
        }
    }
}

pub(crate) fn engine_names(engines: &[EngineId]) -> String {
    if engines.is_empty() {
        return t("app.all_databases").to_owned();
    }
    engines
        .iter()
        .map(|id| id.display_name())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Present-participle label of a running operation ("Starting").
pub(crate) fn kind_text(kind: OpKind) -> &'static str {
    t(match kind {
        OpKind::Start => "op.running.start",
        OpKind::Stop => "op.running.stop",
        OpKind::Restart => "op.running.restart",
        OpKind::Recreate => "op.running.recreate",
        OpKind::Reset => "op.running.reset",
        OpKind::Remove => "op.running.remove",
        OpKind::RunScripts => "op.running.run_scripts",
        OpKind::SaveSnapshot => "op.running.save_snapshot",
        OpKind::RestoreSnapshot => "op.running.restore_snapshot",
    })
}

/// Translation key of the success message (with `{names}`).
fn done_key(kind: OpKind) -> &'static str {
    match kind {
        OpKind::Start => "op.done.start",
        OpKind::Stop => "op.done.stop",
        OpKind::Restart => "op.done.restart",
        OpKind::Recreate => "op.done.recreate",
        OpKind::Reset => "op.done.reset",
        OpKind::Remove => "op.done.remove",
        OpKind::RunScripts => "op.done.run_scripts",
        OpKind::SaveSnapshot => "op.done.save_snapshot",
        OpKind::RestoreSnapshot => "op.done.restore_snapshot",
    }
}

fn last_line(text: &str) -> &str {
    text.lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or(text)
        .trim()
}

/// Checks GitHub for a newer release on a background thread.
fn spawn_update_check(ctx: &egui::Context) -> std::sync::mpsc::Receiver<Option<Release>> {
    let (tx, rx) = std::sync::mpsc::channel();
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let _ = tx.send(update::check());
        ctx.request_repaint();
    });
    rx
}
