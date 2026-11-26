//! The snapshots window: save the data of a database and restore it later.

use egui::RichText;

use super::theme::WARNING_COLOR;
use crate::app::{Action, DbDockApp};
use crate::engines::EngineId;
use crate::i18n::{t, tf};
use crate::snapshots::{self, Snapshot};

pub struct SnapshotsWindow {
    pub engine: EngineId,
    /// Read from disk when the window opens and after every change.
    pub list: Vec<Snapshot>,
    /// Snapshot waiting for "Restore" or "Delete" to be confirmed.
    pub confirm: Option<(String, Confirm)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirm {
    Restore,
    Delete,
}

impl SnapshotsWindow {
    pub fn new(engine: EngineId, paths: &crate::paths::AppPaths) -> Self {
        Self {
            engine,
            list: snapshots::list(paths, engine),
            confirm: None,
        }
    }
}

impl DbDockApp {
    pub(crate) fn snapshots_window_ui(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let Some(window) = &self.snapshots_window else {
            return;
        };
        let id = window.engine;
        let settings = self.config.engine(id);
        let status = self.status(id);
        let busy = self.running_op(id).is_some();
        let mut open = true;
        let mut confirm = window.confirm.clone();

        egui::Window::new(tf("snapshots.title", &[("engine", &id.display_name())]))
            .id(egui::Id::unique("snapshots-window"))
            .open(&mut open)
            .collapsible(false)
            .default_width(560.0)
            .show(ctx, |ui| {
                ui.add(egui::Label::new(t("snapshots.intro")).wrap());
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let can_save = status.is_created() && !busy;
                    let save = ui
                        .add_enabled(can_save, egui::Button::new(t("snapshots.save")))
                        .on_hover_text(t("snapshots.save_hint"));
                    if !status.is_created() {
                        save.on_disabled_hover_text(t("snapshots.not_created"));
                    } else if save.clicked() {
                        actions.push(Action::SaveSnapshot(id));
                    }
                    if ui.button(t("common.open_folder")).clicked() {
                        actions.push(Action::OpenFolder(self.paths.snapshots_dir(id)));
                    }
                    if busy {
                        ui.spinner();
                    }
                });
                ui.separator();

                if window.list.is_empty() {
                    ui.label(RichText::new(t("snapshots.empty")).weak());
                    return;
                }
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        egui::Grid::new("snapshots-grid")
                            .num_columns(3)
                            .spacing([16.0, 6.0])
                            .striped(true)
                            .show(ui, |ui| {
                                for snapshot in &window.list {
                                    row(
                                        ui,
                                        id,
                                        snapshot,
                                        snapshot.fits(id, settings),
                                        busy,
                                        &mut confirm,
                                        actions,
                                    );
                                    ui.end_row();
                                }
                            });
                    });
            });

        if let Some(window) = &mut self.snapshots_window {
            window.confirm = confirm;
        }
        if !open {
            self.snapshots_window = None;
        }
    }

    /// Reads the snapshot list again (after saving, restoring or deleting).
    pub(crate) fn refresh_snapshots(&mut self) {
        if let Some(window) = &mut self.snapshots_window {
            window.list = snapshots::list(&self.paths, window.engine);
            window.confirm = None;
        }
    }
}

fn row(
    ui: &mut egui::Ui,
    id: EngineId,
    snapshot: &Snapshot,
    fits: bool,
    busy: bool,
    confirm: &mut Option<(String, Confirm)>,
    actions: &mut Vec<Action>,
) {
    ui.vertical(|ui| {
        ui.label(RichText::new(snapshots::display_time(snapshot.created)).strong());
        ui.label(RichText::new(&snapshot.name).small().weak());
    });
    ui.vertical(|ui| {
        if id != EngineId::Sqlite {
            ui.label(tf("snapshots.version", &[("version", &snapshot.version)]));
        }
        if !snapshot.complete {
            ui.colored_label(WARNING_COLOR, t("snapshots.incomplete"))
                .on_hover_text(t("snapshots.incomplete_hint"));
        } else if !fits {
            ui.colored_label(WARNING_COLOR, t("snapshots.other_version"))
                .on_hover_text(t("snapshots.other_version_hint"));
        }
    });
    ui.horizontal(|ui| {
        let pending = confirm
            .as_ref()
            .filter(|(name, _)| *name == snapshot.name)
            .map(|(_, what)| *what);
        match pending {
            Some(what) => {
                let (question, button) = match what {
                    Confirm::Restore => ("snapshots.confirm_restore", "snapshots.restore"),
                    Confirm::Delete => ("snapshots.confirm_delete", "snapshots.delete"),
                };
                ui.colored_label(WARNING_COLOR, t(question));
                if ui
                    .add_enabled(!busy, egui::Button::new(t(button)))
                    .clicked()
                {
                    actions.push(match what {
                        Confirm::Restore => Action::RestoreSnapshot(id, snapshot.name.clone()),
                        Confirm::Delete => Action::DeleteSnapshot(id, snapshot.name.clone()),
                    });
                    *confirm = None;
                }
                if ui.button(t("common.cancel")).clicked() {
                    *confirm = None;
                }
            }
            None => {
                let can_restore = snapshot.complete && fits && !busy;
                if ui
                    .add_enabled(can_restore, egui::Button::new(t("snapshots.restore")))
                    .on_hover_text(t("snapshots.restore_hint"))
                    .clicked()
                {
                    *confirm = Some((snapshot.name.clone(), Confirm::Restore));
                }
                if ui
                    .add_enabled(!busy, egui::Button::new(t("snapshots.delete")))
                    .clicked()
                {
                    *confirm = Some((snapshot.name.clone(), Confirm::Delete));
                }
            }
        }
    });
}
