use crate::{PlanTool, Studio, design::Design, wall_editor};
use eframe::egui;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

pub enum Action {
    Replace {
        cfg: Box<Design>,
        path: Option<PathBuf>,
    },
    Close,
}

#[derive(PartialEq)]
pub enum Stage {
    Confirm,
    Saving,
    Ready,
}

pub struct Pending {
    action: Action,
    stage: Stage,
}

impl Pending {
    fn new(action: Action) -> Self {
        Self {
            action,
            stage: Stage::Confirm,
        }
    }
    fn saved(&mut self) {
        if self.stage == Stage::Saving {
            self.stage = Stage::Ready;
        }
    }
}

impl Studio {
    pub fn dialog_active(&self) -> bool {
        self.library_browser.visible
            || self.export_dialog
            || self.help_visible
            || self.printer_panel.is_open()
            || self.pending_transition.is_some()
            || self.operation_error.is_some()
    }

    pub fn request_replace(&mut self, cfg: Design, path: Option<PathBuf>) {
        if self.key() != self.saved_key {
            self.pending_transition = Some(Pending::new(Action::Replace {
                cfg: Box::new(cfg),
                path,
            }));
        } else {
            self.apply_replace(cfg, path);
        }
    }

    pub(crate) fn apply_replace(&mut self, cfg: Design, path: Option<PathBuf>) {
        self.cfg = cfg;
        self.saved_path = path;
        self.saved_key = self.key();
        self.show_home = false;
        self.wall_editor = wall_editor::Editor::from_design(&self.cfg);
        self.wall_drag = None;
        self.dimension_edit = None;
        self.tool = PlanTool::Select;
        self.view_plan = false;
        self.selection = None;
        self.wall_start = None;
        self.active = 0;
        self.assembly = true;
        self.error = None;
        self.operation_error = None;
        self.parts.clear();
        self.built.clear();
        self.observed.clear();
        self.changed = Instant::now() - Duration::from_secs(1);
        self.reset_view();
    }

    pub fn handle_close_request(&mut self, ctx: &egui::Context) {
        if !self.allow_close
            && ctx.input(|input| input.viewport().close_requested())
            && (self.key() != self.saved_key || self.pending_transition.is_some())
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if self.pending_transition.is_none() {
                self.pending_transition = Some(Pending::new(Action::Close));
            }
        }
    }

    pub fn pending_save_finished(&mut self) {
        if let Some(pending) = &mut self.pending_transition {
            pending.saved();
        }
    }

    pub fn transition_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut pending) = self.pending_transition.take() else {
            return;
        };
        if pending.stage == Stage::Saving {
            // Closing the save browser is cancellation, never implicit approval.
            if self.library_browser.visible {
                self.pending_transition = Some(pending);
            }
            return;
        }
        if pending.stage == Stage::Confirm {
            let mut save = false;
            let mut discard = false;
            let mut cancel = false;
            let modal = egui::Modal::new(egui::Id::new("unsaved-current-design")).show(ctx, |ui| {
                ui.set_max_width(470.0);
                ui.heading("Änderungen speichern?");
                ui.label(format!(
                    "„{}“ enthält ungespeicherte Änderungen.",
                    self.cfg.title
                ));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    save = ui.button("Speichern …").clicked();
                    discard = ui
                        .button(if matches!(pending.action, Action::Close) {
                            "Ohne Speichern schließen"
                        } else {
                            "Änderungen verwerfen"
                        })
                        .clicked();
                    cancel = ui.button("Abbrechen").clicked();
                });
            });
            if cancel || modal.should_close() {
                return;
            }
            if save {
                pending.stage = Stage::Saving;
                self.pending_transition = Some(pending);
                self.show_library(true);
                return;
            }
            if !discard {
                self.pending_transition = Some(pending);
                return;
            }
        }
        match pending.action {
            Action::Replace { cfg, path } => self.apply_replace(*cfg, path),
            Action::Close => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    pub fn operation_error_dialog(&mut self, ctx: &egui::Context) {
        let Some(error) = self.operation_error.clone() else {
            return;
        };
        let mut close = false;
        let response = egui::Modal::new(egui::Id::new("operation-error")).show(ctx, |ui| {
            ui.set_max_width(500.0);
            ui.heading("Vorgang fehlgeschlagen");
            ui.add(egui::Label::new(error).wrap());
            ui.add_space(8.0);
            close = ui.button("OK").clicked();
        });
        if close || response.should_close() {
            self.operation_error = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pending_action_resumes_only_after_an_explicit_successful_save() {
        let mut pending = Pending::new(Action::Close);
        pending.saved();
        assert!(pending.stage == Stage::Confirm);
        pending.stage = Stage::Saving;
        pending.saved();
        assert!(pending.stage == Stage::Ready);
    }
}
