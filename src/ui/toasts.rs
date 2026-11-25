//! Small notifications in the bottom-right corner.

use std::time::{Duration, Instant};

use egui::{Align2, Color32, RichText};

use super::theme::{ERROR_COLOR, OK_COLOR};
use crate::app::Action;
use crate::docker::worker::OpId;
use crate::i18n::t;

const MAX_TOASTS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Info,
    Success,
    Error,
}

struct Toast {
    kind: Kind,
    text: String,
    created: Instant,
    /// Offers a "Show output" button for a failed operation.
    output: Option<OpId>,
}

impl Toast {
    fn lifetime(&self) -> Duration {
        match self.kind {
            Kind::Error => Duration::from_secs(15),
            _ => Duration::from_secs(5),
        }
    }
}

#[derive(Default)]
pub struct Toasts {
    toasts: Vec<Toast>,
}

impl Toasts {
    pub fn info(&mut self, text: impl Into<String>) {
        self.push(Kind::Info, text.into(), None);
    }

    pub fn success(&mut self, text: impl Into<String>) {
        self.push(Kind::Success, text.into(), None);
    }

    pub fn error(&mut self, text: impl Into<String>) {
        self.push(Kind::Error, text.into(), None);
    }

    pub fn error_with_output(&mut self, text: impl Into<String>, op: OpId) {
        self.push(Kind::Error, text.into(), Some(op));
    }

    fn push(&mut self, kind: Kind, text: String, output: Option<OpId>) {
        self.toasts.push(Toast {
            kind,
            text,
            created: Instant::now(),
            output,
        });
        if self.toasts.len() > MAX_TOASTS {
            self.toasts.remove(0);
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let now = Instant::now();
        self.toasts.retain(|t| now - t.created < t.lifetime());
        if self.toasts.is_empty() {
            return;
        }
        // Wake up again when the oldest toast expires.
        if let Some(next) = self
            .toasts
            .iter()
            .map(|t| t.lifetime() - (now - t.created))
            .min()
        {
            ctx.request_repaint_after(next);
        }

        let mut close = None;
        egui::Area::new(egui::Id::unique("dbdock-toasts"))
            .anchor(Align2::RIGHT_BOTTOM, [-12.0, -12.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.set_max_width(380.0);
                for (index, toast) in self.toasts.iter().enumerate() {
                    let accent = match toast.kind {
                        Kind::Info => ui.visuals().hyperlink_color,
                        Kind::Success => OK_COLOR,
                        Kind::Error => ERROR_COLOR,
                    };
                    egui::Frame::popup(ui.style())
                        .stroke(egui::Stroke::new(1.0, accent))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                super::dot(ui, accent);
                                ui.add(egui::Label::new(&toast.text).wrap());
                            });
                            ui.horizontal(|ui| {
                                if let Some(op) = toast.output {
                                    if ui.small_button(t("toast.show_output")).clicked() {
                                        actions.push(Action::ShowOutput(op));
                                        close = Some(index);
                                    }
                                }
                                if ui
                                    .small_button(
                                        RichText::new(t("toast.dismiss")).color(Color32::GRAY),
                                    )
                                    .clicked()
                                {
                                    close = Some(index);
                                }
                            });
                        });
                    ui.add_space(6.0);
                }
            });
        if let Some(index) = close {
            self.toasts.remove(index);
        }
    }
}
