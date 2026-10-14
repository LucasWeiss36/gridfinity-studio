use crate::{
    Studio,
    library::{self, Entry, Library},
};
use eframe::egui::{self, RichText};
use std::path::{Path, PathBuf};

pub struct Browser {
    pub visible: bool,
    library: Option<Library>,
    dir: PathBuf,
    entries: Vec<Entry>,
    selected: Option<PathBuf>,
    save_mode: bool,
    name: String,
    folder_name: String,
    rename_name: Option<String>,
    delete_entry: Option<Entry>,
    error: Option<String>,
    overwrite: bool,
    pending_open: Option<PathBuf>,
    confirm_switch: bool,
}
impl Browser {
    pub fn new() -> Self {
        // A separate root allows repeatable QA without touching real projects.
        let root = std::env::var_os("GRIDFINITY_DESIGN2_LIBRARY_DIR")
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(library::default_root);
        Self::from_root(root)
    }
    #[cfg(test)]
    pub fn for_test(root: PathBuf) -> Self {
        Self::from_root(root)
    }
    fn from_root(root: PathBuf) -> Self {
        let result = Library::new(root.clone());
        let mut browser = Self {
            visible: false,
            dir: root,
            entries: vec![],
            selected: None,
            save_mode: false,
            name: String::new(),
            folder_name: String::new(),
            rename_name: None,
            delete_entry: None,
            error: None,
            overwrite: false,
            pending_open: None,
            confirm_switch: false,
            library: None,
        };
        match result {
            Ok(lib) => {
                browser.dir = lib.root.clone();
                browser.library = Some(lib);
                browser.refresh();
            }
            Err(error) => {
                browser.error = Some(format!(
                    "Projektablage konnte nicht geöffnet werden: {error:#}"
                ))
            }
        }
        browser
    }
    fn refresh(&mut self) {
        if let Some(lib) = &self.library {
            match lib.entries(&self.dir) {
                Ok(entries) => {
                    self.entries = entries;
                    if self.selected.as_ref().is_some_and(|selected| {
                        !self.entries.iter().any(|entry| &entry.path == selected)
                    }) {
                        self.selected = None;
                        self.rename_name = None;
                    }
                    self.error = None;
                }
                Err(error) => {
                    self.entries.clear();
                    self.selected = None;
                    self.rename_name = None;
                    self.error = Some(format!("{error:#}"));
                }
            }
        }
    }
    fn navigate(&mut self, path: PathBuf) {
        self.dir = path;
        self.selected = None;
        self.rename_name = None;
        self.delete_entry = None;
        self.refresh();
    }
    fn cancel_switch(&mut self) {
        self.pending_open = None;
        self.confirm_switch = false;
    }
    fn close(&mut self) {
        self.visible = false;
        self.cancel_switch();
        self.overwrite = false;
        self.delete_entry = None;
        self.rename_name = None;
    }
    fn open_failed(&mut self, error: String) {
        self.visible = true;
        self.save_mode = false;
        self.cancel_switch();
        self.overwrite = false;
        self.error = Some(format!("Entwurf konnte nicht geöffnet werden: {error}"));
    }
    fn root(&self) -> PathBuf {
        self.library
            .as_ref()
            .map(|lib| lib.root.clone())
            .unwrap_or_else(|| self.dir.clone())
    }
}

impl Studio {
    pub fn show_library(&mut self, save: bool) {
        self.dimension_edit = None;
        self.library_browser.cancel_switch();
        self.library_browser.visible = true;
        self.library_browser.save_mode = save;
        self.library_browser.overwrite = false;
        self.library_browser.rename_name = None;
        self.library_browser.delete_entry = None;
        if save {
            self.library_browser.name = self.cfg.title.clone();
            if let Some(path) = self
                .saved_path
                .as_ref()
                .map(|path| path.canonicalize().unwrap_or_else(|_| path.clone()))
                && path.starts_with(self.library_browser.root())
                && let Some(parent) = path.parent()
            {
                self.library_browser.dir = parent.to_path_buf();
                if let Some(name) = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .and_then(library::design_name)
                {
                    self.library_browser.name = name.into();
                }
            }
        }
        self.library_browser.refresh();
    }
    fn open_library_design(&mut self, path: PathBuf) {
        let result = self
            .library_browser
            .library
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Projektablage ist nicht verfügbar."))
            .and_then(|lib| lib.load_design(&path));
        match result {
            Ok(cfg) => {
                self.apply_replace(cfg, Some(path));
                self.library_browser.visible = false;
                self.library_browser.pending_open = None;
                self.library_browser.confirm_switch = false;
                self.message = format!("„{}“ geöffnet.", self.cfg.title);
            }
            Err(error) => {
                // A failed open after "save first" must stay visible, even
                // though the successful save just closed this window.
                self.library_browser.open_failed(format!("{error:#}"));
            }
        }
    }
    fn request_library_open(&mut self, path: PathBuf) {
        if self.key() != self.saved_key {
            self.library_browser.pending_open = Some(path);
            self.library_browser.confirm_switch = true;
        } else {
            self.open_library_design(path);
        }
    }
    fn save_library_design(&mut self, overwrite: bool) {
        self.library_browser.name = self.library_browser.name.trim().to_owned();
        let browser = &self.library_browser;
        let mut cfg = self.cfg.clone();
        cfg.title = browser.name.trim().into();
        let result = browser
            .library
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Projektablage ist nicht verfügbar."))
            .and_then(|lib| lib.save_design(&browser.dir, &browser.name, &cfg, overwrite));
        match result {
            Ok(path) => {
                self.cfg = cfg;
                self.saved_path = Some(path.clone());
                self.saved_key = self.key();
                self.pending_save_finished();
                self.message = format!("„{}“ gespeichert.", self.cfg.title);
                self.library_browser.visible = false;
                self.library_browser.overwrite = false;
                self.library_browser.refresh();
                if let Some(next) = self.library_browser.pending_open.take() {
                    self.open_library_design(next);
                }
            }
            Err(error) => {
                self.library_browser.error = Some(format!("{error:#}"));
                self.library_browser.overwrite = false;
            }
        }
    }
    pub fn library_dialogs(&mut self, ctx: &egui::Context) {
        let colors = crate::theme::palette(ctx);
        if !self.library_browser.visible {
            return;
        }
        let mut visible = true;
        let mut open_entry = None;
        let mut save = false;
        let mut rename = false;
        let mut cancel_rename = false;
        let browser = &mut self.library_browser;
        egui::Window::new(if browser.save_mode {
            "Entwurf speichern"
        } else {
            "Meine Projekte"
        })
        .id(egui::Id::new("project-library"))
        .open(&mut visible)
        .collapsible(false)
        .resizable(true)
        .default_width(780.0)
        .min_width(610.0)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            let root = browser.root();
            let mut navigate = None;
            ui.horizontal_wrapped(|ui| {
                if ui.button("Alle Projekte").clicked() {
                    navigate = Some(root.clone());
                }
                if let Ok(relative) = browser.dir.strip_prefix(&root) {
                    let mut path = root.clone();
                    for component in relative.components() {
                        path.push(component.as_os_str());
                        ui.label("/");
                        if ui.button(component.as_os_str().to_string_lossy()).clicked() {
                            navigate = Some(path.clone());
                        }
                    }
                }
                if ui.small_button("Aktualisieren").clicked() {
                    browser.refresh();
                }
            });
            if let Some(path) = navigate {
                browser.navigate(path);
            }
            let at_root = browser.dir == root;
            ui.separator();
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut browser.folder_name)
                        .id_salt("new-library-folder")
                        .desired_width(230.0)
                        .char_limit(120)
                        .hint_text(if at_root { "Projektname" } else { "Ordnername" }),
                );
                if ui
                    .add_enabled(
                        browser.library.is_some() && !browser.folder_name.trim().is_empty(),
                        egui::Button::new(if at_root {
                            "Projekt anlegen"
                        } else {
                            "Ordner anlegen"
                        }),
                    )
                    .clicked()
                    && let Some(lib) = &browser.library
                {
                    match lib.create_folder(&browser.dir, browser.folder_name.trim()) {
                        Ok(path) => {
                            browser.folder_name.clear();
                            browser.navigate(path);
                        }
                        Err(error) => browser.error = Some(format!("{error:#}")),
                    }
                }
            });
            ui.add_space(8.0);
            let mut selected = None;
            let mut enter = None;
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                egui::ScrollArea::vertical()
                    .id_salt("project-entries")
                    .min_scrolled_height(210.0)
                    .max_height(280.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if browser.entries.is_empty() && browser.error.is_none() {
                            ui.add_space(24.0);
                            ui.vertical_centered(|ui| {
                                ui.heading(if browser.dir == root {
                                    "Noch keine Projekte"
                                } else {
                                    "Ordner ist leer"
                                });
                                ui.label(if browser.dir == root {
                                    "Lege oben ein Projekt an."
                                } else {
                                    "Speichere einen Entwurf oder lege einen Unterordner an."
                                });
                            });
                        }
                        for entry in &browser.entries {
                            ui.horizontal(|ui| {
                                let response = entry_row(
                                    ui,
                                    entry,
                                    browser.selected.as_ref() == Some(&entry.path),
                                );
                                if response.clicked() {
                                    selected = Some(entry.clone());
                                }
                                if response.double_clicked() {
                                    enter = Some(entry.clone());
                                }
                                ui.label(
                                    RichText::new(if entry.is_dir {
                                        if browser.dir == root {
                                            "Projekt"
                                        } else {
                                            "Ordner"
                                        }
                                    } else {
                                        "Entwurf"
                                    })
                                    .small()
                                    .color(colors.muted),
                                );
                            });
                        }
                    });
            });
            if let Some(entry) = selected {
                browser.selected = Some(entry.path);
                browser.rename_name = None;
                if browser.save_mode && !entry.is_dir {
                    browser.name = entry.name;
                }
            }
            ui.horizontal(|ui| {
                let entry = browser
                    .entries
                    .iter()
                    .find(|e| Some(&e.path) == browser.selected.as_ref())
                    .cloned();
                let open_label =
                    if browser.save_mode && entry.as_ref().is_some_and(|entry| !entry.is_dir) {
                        "Name übernehmen"
                    } else {
                        "Öffnen"
                    };
                if ui
                    .add_enabled(entry.is_some(), egui::Button::new(open_label))
                    .clicked()
                {
                    enter = entry.clone();
                }
                if ui
                    .add_enabled(entry.is_some(), egui::Button::new("Umbenennen"))
                    .clicked()
                    && let Some(entry) = &entry
                {
                    browser.rename_name = Some(entry.name.clone());
                }
                if ui
                    .add_enabled(entry.is_some(), egui::Button::new("Löschen …"))
                    .clicked()
                    && let Some(entry) = entry
                {
                    browser.delete_entry = Some(entry);
                    browser.rename_name = None;
                }
                if browser.save_mode && ui.button("Projekte ansehen").clicked() {
                    browser.save_mode = false;
                    browser.cancel_switch();
                }
            });
            if let Some(entry) = enter {
                if entry.is_dir {
                    browser.navigate(entry.path);
                } else if !browser.save_mode {
                    open_entry = Some(entry.path);
                } else {
                    browser.name = entry.name;
                }
            }
            if let Some(name) = &mut browser.rename_name {
                ui.horizontal(|ui| {
                    ui.label("Neuer Name");
                    ui.add(
                        egui::TextEdit::singleline(name)
                            .desired_width(240.0)
                            .char_limit(120),
                    );
                    if ui.button("Übernehmen").clicked() {
                        rename = true;
                    }
                    if ui.button("Abbrechen").clicked() {
                        cancel_rename = true;
                    }
                });
            }
            if browser.save_mode {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Entwurfsname");
                    ui.add(
                        egui::TextEdit::singleline(&mut browser.name)
                            .id_salt("saved-design-name")
                            .desired_width(300.0)
                            .char_limit(120),
                    );
                    if ui
                        .add_enabled(
                            browser.library.is_some()
                                && browser.dir != root
                                && !browser.name.trim().is_empty(),
                            egui::Button::new(
                                RichText::new("Hier speichern").color(colors.on_primary),
                            )
                            .fill(colors.primary),
                        )
                        .clicked()
                    {
                        save = true;
                    }
                });
                if browser.dir == root {
                    ui.label("Wähle zuerst ein Projekt oder lege eines an.");
                }
            } else if !self.show_home && ui.button("Aktuellen Entwurf hier speichern …").clicked()
            {
                browser.name = self.cfg.title.clone();
                browser.save_mode = true;
            }
            if let Some(error) = &browser.error {
                ui.colored_label(colors.error, error);
            }
            ui.separator();
            let root_text = root.to_string_lossy();
            let root_text = root_text.strip_prefix(r"\\?\").unwrap_or(&root_text);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!("Ablageort: {root_text}"))
                        .small()
                        .color(colors.muted),
                );
                if ui.small_button("Im Explorer öffnen").clicked()
                    && let Err(error) = std::process::Command::new("explorer.exe")
                        .arg(root_text)
                        .spawn()
                {
                    browser.error = Some(format!("Ordner konnte nicht geöffnet werden: {error}"));
                }
            });
        });
        browser.visible = visible;
        if !visible {
            browser.close();
            return;
        }
        if browser.delete_entry.is_some() {
            self.library_delete_dialog(ctx);
            return;
        }
        if cancel_rename {
            browser.rename_name = None;
            rename = false;
        }
        if rename
            && let (Some(lib), Some(path), Some(name)) = (
                &browser.library,
                browser.selected.clone(),
                browser.rename_name.clone(),
            )
        {
            // Capture the association while the old path still exists, including
            // a normal Windows path returned by the standalone file dialog.
            let saved_relative = self
                .saved_path
                .as_ref()
                .and_then(|saved| relative_path(saved, &path));
            match lib.rename(&path, name.trim()) {
                Ok(new_path) => {
                    if let Some(rest) = saved_relative {
                        self.saved_path = Some(if rest.as_os_str().is_empty() {
                            new_path.clone()
                        } else {
                            new_path.join(rest)
                        });
                    }
                    if let Some(pending) = &browser.pending_open
                        && let Ok(rest) = pending.strip_prefix(&path)
                    {
                        browser.pending_open = Some(if rest.as_os_str().is_empty() {
                            new_path.clone()
                        } else {
                            new_path.join(rest)
                        });
                    }
                    browser.rename_name = None;
                    browser.refresh();
                    browser.selected = Some(new_path);
                }
                Err(error) => browser.error = Some(format!("{error:#}")),
            }
        }
        if let Some(path) = open_entry {
            self.request_library_open(path);
        }
        if save {
            self.library_browser.name = self.library_browser.name.trim().to_owned();
            let exists = self.library_browser.entries.iter().any(|entry| {
                !entry.is_dir
                    && entry.name.to_lowercase() == self.library_browser.name.trim().to_lowercase()
            });
            if exists {
                self.library_browser.overwrite = true;
            } else {
                self.save_library_design(false);
            }
        }
        if self.library_browser.overwrite {
            let mut replace = false;
            let mut cancel = false;
            let modal = egui::Modal::new(egui::Id::new("replace-library-design")).show(ctx, |ui| {
                ui.heading("Gespeicherten Entwurf ersetzen?");
                ui.label(format!(
                    "„{}“ gibt es in diesem Ordner bereits.",
                    self.library_browser.name
                ));
                ui.horizontal(|ui| {
                    replace = ui.button("Entwurf ersetzen").clicked();
                    cancel = ui.button("Abbrechen").clicked();
                });
            });
            if cancel || modal.should_close() {
                self.library_browser.overwrite = false;
            } else if replace {
                self.save_library_design(true);
            }
        }
        if self.library_browser.confirm_switch {
            let mut save_first = false;
            let mut discard = false;
            let mut cancel = false;
            let modal = egui::Modal::new(egui::Id::new("unsaved-library-design")).show(ctx, |ui| {
                ui.heading("Aktuellen Entwurf speichern?");
                ui.label("Dein Entwurf enthält ungespeicherte Änderungen.");
                ui.horizontal(|ui| {
                    save_first = ui.button("Erst speichern").clicked();
                    discard = ui.button("Ohne Speichern öffnen").clicked();
                    cancel = ui.button("Abbrechen").clicked();
                });
            });
            if cancel || modal.should_close() {
                self.library_browser.cancel_switch();
            } else if save_first {
                let pending = self.library_browser.pending_open.take();
                self.show_library(true);
                self.library_browser.pending_open = pending;
            } else if discard && let Some(path) = self.library_browser.pending_open.take() {
                self.library_browser.confirm_switch = false;
                self.open_library_design(path);
            }
        }
    }

    fn library_delete_dialog(&mut self, ctx: &egui::Context) {
        let colors = crate::theme::palette(ctx);
        let Some(entry) = self.library_browser.delete_entry.clone() else {
            return;
        };
        let kind = if !entry.is_dir {
            "Entwurf"
        } else if entry.path.parent() == Some(self.library_browser.root().as_path()) {
            "Projekt"
        } else {
            "Ordner"
        };
        // Resolve before deletion: a design opened through the file dialog may
        // use C:\… while library entries use Windows' canonical \\?\C:\… paths.
        let active_affected = saved_design_is_within(&self.saved_path, &entry.path);
        let mut confirm = false;
        let mut cancel = false;
        let modal = egui::Modal::new(egui::Id::new("delete-library-entry")).show(ctx, |ui| {
            ui.set_max_width(460.0);
            ui.heading(format!("{kind} wirklich löschen?"));
            ui.label(RichText::new(format!("„{}“", entry.name)).strong());
            ui.add_space(8.0);
            ui.label(if entry.is_dir {
                "Dieser Ordner und sämtliche Unterordner, Entwürfe und anderen Dateien darin werden endgültig gelöscht."
            } else {
                "Diese gespeicherte Entwurfsdatei wird endgültig gelöscht."
            });
            ui.label("Die Dateien werden nicht in den Windows-Papierkorb verschoben.");
            if active_affected {
                ui.add_space(8.0);
                ui.label("Der offene Entwurf bleibt im Editor. Du kannst ihn erneut speichern.");
            }
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                cancel = ui.button("Abbrechen").clicked();
                confirm = ui.add(egui::Button::new(RichText::new("Endgültig löschen").color(colors.error))
                    .fill(colors.soft)
                    .stroke(egui::Stroke::new(1.0_f32, colors.error))).clicked();
            });
        });
        if cancel || modal.should_close() {
            self.library_browser.delete_entry = None;
            return;
        }
        if !confirm {
            return;
        }

        let result = self
            .library_browser
            .library
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Projektablage ist nicht verfügbar."))
            .and_then(|library| library.delete(&entry));
        let detached = detach_deleted_design(
            &mut self.saved_path,
            &mut self.saved_key,
            active_affected,
            result.is_ok(),
        );
        let browser = &mut self.library_browser;
        browser.delete_entry = None;
        browser.rename_name = None;
        browser.selected = None;
        if browser.pending_open.as_ref().is_some_and(|path| {
            path.starts_with(&entry.path)
                && (result.is_ok() || matches!(path.try_exists(), Ok(false)))
        }) {
            browser.pending_open = None;
            browser.confirm_switch = false;
        }
        // The list must reflect even a partially completed deletion. Preserve the
        // actual failure after refresh, since refresh clears earlier errors.
        browser.refresh();
        match result {
            Ok(()) => {
                self.message = format!("„{}“ wurde gelöscht.", entry.name);
            }
            Err(error) => {
                browser.error = Some(format!("Löschen nicht abgeschlossen: {error:#}"));
                self.message =
                    "Löschen nicht abgeschlossen. Die Projektliste wurde aktualisiert.".into();
            }
        }
        if detached {
            self.message.push_str(
                " Der offene Entwurf ist weiterhin im Editor und muss neu gespeichert werden.",
            );
        }
    }
}

fn detach_deleted_design(
    saved_path: &mut Option<PathBuf>,
    saved_key: &mut String,
    affected: bool,
    completed: bool,
) -> bool {
    if affected
        && saved_path
            .as_ref()
            .is_some_and(|path| completed || matches!(path.try_exists(), Ok(false)))
    {
        *saved_path = None;
        saved_key.clear();
        true
    } else {
        false
    }
}

fn saved_design_is_within(saved_path: &Option<PathBuf>, deleted: &Path) -> bool {
    saved_path
        .as_ref()
        .is_some_and(|path| relative_path(path, deleted).is_some())
}

fn relative_path(path: &Path, parent: &Path) -> Option<PathBuf> {
    path.strip_prefix(parent)
        .ok()
        .map(Path::to_owned)
        .or_else(|| {
            path.canonicalize()
                .ok()?
                .strip_prefix(parent)
                .ok()
                .map(Path::to_owned)
        })
}

fn entry_row(ui: &mut egui::Ui, entry: &Entry, selected: bool) -> egui::Response {
    let colors = crate::theme::palette(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width() - 105.0, 32.0),
        egui::Sense::click(),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &entry.name));
    let painter = ui.painter().with_clip_rect(rect);
    if selected || response.hovered() {
        painter.rect_filled(
            rect,
            4,
            if selected {
                colors.selection
            } else {
                colors.hover
            },
        );
    }
    let colour = if entry.is_dir {
        colors.accent
    } else {
        colors.text
    };
    let icon = egui::Rect::from_center_size(
        rect.left_center() + egui::vec2(17.0, 0.0),
        egui::vec2(16.0, 12.0),
    );
    painter.rect_stroke(
        icon,
        2,
        egui::Stroke::new(1.3_f32, colour),
        egui::StrokeKind::Inside,
    );
    if entry.is_dir {
        painter.rect_filled(
            egui::Rect::from_min_size(icon.min - egui::vec2(0.0, 3.0), egui::vec2(8.0, 4.0)),
            1,
            colour,
        );
    } else {
        for dy in [-2.0, 2.0] {
            painter.line_segment(
                [
                    icon.center() + egui::vec2(-4.0, dy),
                    icon.center() + egui::vec2(4.0, dy),
                ],
                egui::Stroke::new(1.0_f32, colour),
            );
        }
    }
    painter.text(
        rect.left_center() + egui::vec2(34.0, 0.0),
        egui::Align2::LEFT_CENTER,
        &entry.name,
        egui::FontId::proportional(13.0),
        colour,
    );
    response.on_hover_text(&entry.name)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Sandbox(PathBuf);
    impl Sandbox {
        fn new() -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "gridfinity-browser-test-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path.canonicalize().unwrap())
        }
    }
    impl Drop for Sandbox {
        fn drop(&mut self) {
            if self.0.parent() == std::env::temp_dir().canonicalize().ok().as_deref()
                && self
                    .0
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("gridfinity-browser-test-")
            {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }

    #[test]
    fn closing_the_browser_cancels_all_pending_actions_without_touching_files() {
        let sandbox = Sandbox::new();
        let mut browser = Browser::for_test(sandbox.0.clone());
        let file = browser
            .library
            .as_ref()
            .unwrap()
            .save_design(
                &browser.root(),
                "Box",
                &crate::design::Design::default(),
                false,
            )
            .unwrap();
        let bytes = std::fs::read(&file).unwrap();
        browser.refresh();
        browser.visible = true;
        browser.save_mode = true;
        browser.overwrite = true;
        browser.pending_open = Some(file.clone());
        browser.confirm_switch = true;
        browser.delete_entry = Some(browser.entries[0].clone());
        browser.rename_name = Some("Neu".into());
        browser.close();
        assert!(!browser.visible && !browser.overwrite && !browser.confirm_switch);
        assert!(browser.pending_open.is_none());
        assert!(browser.delete_entry.is_none());
        assert!(browser.rename_name.is_none());
        assert_eq!(std::fs::read(file).unwrap(), bytes);
    }

    #[test]
    fn failed_open_after_saving_reopens_the_browser_and_clears_the_switch() {
        let sandbox = Sandbox::new();
        let mut browser = Browser::for_test(sandbox.0.clone());
        browser.visible = false;
        browser.save_mode = true;
        browser.pending_open = Some(sandbox.0.join("Fehlt.gridfinity.json"));
        browser.open_failed("Datei nicht gefunden.".into());
        assert!(browser.visible);
        assert!(!browser.save_mode && !browser.confirm_switch);
        assert!(browser.pending_open.is_none());
        assert!(
            browser
                .error
                .as_ref()
                .unwrap()
                .contains("Datei nicht gefunden.")
        );
    }

    #[test]
    fn refreshing_removes_a_disappeared_selection_and_rename_action() {
        let sandbox = Sandbox::new();
        let mut browser = Browser::for_test(sandbox.0.clone());
        let file = browser
            .library
            .as_ref()
            .unwrap()
            .save_design(
                &browser.root(),
                "Box",
                &crate::design::Design::default(),
                false,
            )
            .unwrap();
        browser.refresh();
        browser.selected = Some(file.clone());
        browser.rename_name = Some("Neu".into());
        std::fs::remove_file(file).unwrap();
        browser.refresh();
        assert!(browser.entries.is_empty());
        assert!(browser.selected.is_none() && browser.rename_name.is_none());
    }

    #[test]
    fn deleted_open_design_is_preserved_in_memory_but_marked_unsaved() {
        let root = PathBuf::from("Projekte");
        let folder = root.join("Schrank");
        for deleted in [folder.clone(), folder.join("Kabel/Box.gridfinity.json")] {
            let mut saved = Some(folder.join("Kabel/Box.gridfinity.json"));
            let mut key = "gespeicherter Entwurf".to_owned();
            let affected = saved_design_is_within(&saved, &deleted);
            assert!(detach_deleted_design(&mut saved, &mut key, affected, true));
            assert!(saved.is_none());
            assert!(key.is_empty());
        }
        let original = root.join("Schrank 2/Box.gridfinity.json");
        let mut saved = Some(original.clone());
        let mut key = "gespeichert".to_owned();
        let affected = saved_design_is_within(&saved, &folder);
        assert!(!detach_deleted_design(&mut saved, &mut key, affected, true));
        assert_eq!(saved, Some(original));
        assert_eq!(key, "gespeichert");
    }

    #[test]
    fn failed_delete_detaches_only_when_the_saved_file_is_actually_missing() {
        let existing = std::env::current_exe().unwrap();
        let mut saved = Some(existing.clone());
        let mut key = "gespeichert".to_owned();
        let affected = saved_design_is_within(&saved, &existing);
        assert!(!detach_deleted_design(
            &mut saved, &mut key, affected, false
        ));
        assert_eq!(saved, Some(existing));
        assert_eq!(key, "gespeichert");

        let missing = std::env::temp_dir().join(format!(
            "gridfinity-missing-test-{}-{}.gridfinity.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert!(!missing.exists());
        saved = Some(missing.clone());
        let affected = saved_design_is_within(&saved, &missing);
        assert!(detach_deleted_design(&mut saved, &mut key, affected, false));
        assert!(saved.is_none());
        assert!(key.is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn file_dialog_paths_match_canonical_library_paths_before_deletion() {
        let canonical = std::env::current_exe().unwrap().canonicalize().unwrap();
        let plain = PathBuf::from(canonical.to_string_lossy().strip_prefix(r"\\?\").unwrap());
        assert!(plain.is_file());
        assert!(!plain.starts_with(&canonical));
        let mut saved = Some(plain.clone());
        let mut key = "gespeichert".to_owned();
        let affected = saved_design_is_within(&saved, &canonical);
        assert!(affected);
        assert!(saved_design_is_within(&saved, canonical.parent().unwrap()));
        assert_eq!(
            relative_path(&plain, canonical.parent().unwrap()),
            Some(PathBuf::from(canonical.file_name().unwrap()))
        );
        // A cancelled confirmation does not invoke detachment or mutate paths.
        assert_eq!(saved, Some(plain));
        assert_eq!(key, "gespeichert");
        // A complete deletion still uses the previously resolved association.
        // This test only changes in-memory state; it never removes the test binary.
        assert!(detach_deleted_design(&mut saved, &mut key, affected, true));
        assert!(saved.is_none());
        assert!(key.is_empty());
    }
}
