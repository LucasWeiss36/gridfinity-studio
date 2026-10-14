#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod atelier_ui;
mod baseplate_ui;
mod design;
mod edit_transition;
mod lettering;
mod library;
mod library_ui;
mod model;
mod plan_dimensions;
mod print_preview;
mod print_preview_ui;
mod printer;
mod printer_profiles;
mod printer_ui;
mod reference_ui;
mod theme;
mod thumbnails;
#[cfg(test)]
mod ui_tests;
mod viewport;
mod wall_editor;

use design::{CutShape, Cutout, Design, Product, Wall};
use eframe::egui::{self, Color32, Pos2, RichText, Stroke, Vec2};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
enum Selection {
    Wall(usize),
    Cutout(usize),
}
enum PlanTool {
    Select,
    Wall,
}
enum WallDrag {
    Move(usize, Wall, [f64; 2]),
    End(usize, Wall, bool),
    Draw,
}
type BuildResult = (String, Result<Vec<model::Part>, String>, Duration);
struct Studio {
    root: PathBuf,
    cfg: Design,
    parts: Vec<model::Part>,
    task: Option<Receiver<BuildResult>>,
    observed: String,
    built: String,
    changed: Instant,
    error: Option<String>,
    operation_error: Option<String>,
    pending_transition: Option<edit_transition::Pending>,
    allow_close: bool,
    message: String,
    active: usize,
    assembly: bool,
    explode: f32,
    yaw: f32,
    pitch: f32,
    zoom: f32,
    pan: Vec2,
    view_plan: bool,
    selection: Option<Selection>,
    tool: PlanTool,
    wall_start: Option<[f64; 2]>,
    wall_editor: wall_editor::Editor,
    wall_drag: Option<WallDrag>,
    dimension_edit: Option<plan_dimensions::DimensionEdit>,
    snap: f64,
    show_plan_grid: bool,
    show_measurements: bool,
    show_home: bool,
    home_product: Product,
    appearance: theme::Appearance,
    sidebar_visible: bool,
    help_visible: bool,
    show_inches: bool,
    show_build_plate: bool,
    printer_bed_view: bool,
    export_dialog: bool,
    saved_path: Option<PathBuf>,
    saved_key: String,
    library_browser: library_ui::Browser,
    last_duration: Duration,
    gpu: Option<viewport::Viewport>,
    thumbnails: thumbnails::Thumbnails,
    printer_panel: printer_ui::Panel,
}
impl Studio {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (appearance, appearance_error) = theme::Appearance::load();
        theme::apply(&cc.egui_ctx, appearance.mode);
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "Inter".into(),
            egui::FontData::from_static(include_bytes!("../assets/InterVariable.ttf")).into(),
        );
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "Inter".into());
        cc.egui_ctx.set_fonts(fonts);
        Self {
            root: std::env::current_exe()
                .ok()
                .and_then(|path| path.parent().map(Path::to_path_buf))
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default()),
            cfg: Design::default(),
            parts: vec![],
            task: None,
            observed: String::new(),
            built: String::new(),
            changed: Instant::now() - Duration::from_secs(1),
            error: None,
            operation_error: appearance_error,
            pending_transition: None,
            allow_close: false,
            message: "Bereit.".into(),
            active: 0,
            assembly: true,
            explode: 0.5,
            yaw: -0.38,
            pitch: 0.84,
            zoom: 0.88,
            pan: Vec2::ZERO,
            view_plan: false,
            selection: None,
            tool: PlanTool::Select,
            wall_start: None,
            wall_editor: wall_editor::Editor::default(),
            wall_drag: None,
            dimension_edit: None,
            snap: 1.0,
            show_plan_grid: false,
            show_measurements: true,
            show_home: true,
            home_product: Product::Bin,
            appearance,
            sidebar_visible: true,
            help_visible: false,
            show_inches: false,
            show_build_plate: true,
            printer_bed_view: true,
            export_dialog: false,
            saved_path: None,
            saved_key: serde_json::to_string(&Design::default()).unwrap_or_default(),
            library_browser: library_ui::Browser::new(),
            last_duration: Duration::ZERO,
            gpu: cc.wgpu_render_state.clone().map(viewport::Viewport::new),
            thumbnails: thumbnails::Thumbnails::new(
                cc.wgpu_render_state.clone(),
                cc.egui_ctx.clone(),
            ),
            printer_panel: printer_ui::Panel::new(),
        }
    }
    fn key(&self) -> String {
        serde_json::to_string(&self.cfg).unwrap_or_default()
    }
    fn fresh(&self) -> bool {
        !self.parts.is_empty() && self.built == self.key() && self.error.is_none()
    }
    fn poll(&mut self, ctx: &egui::Context) {
        let key = self.key();
        if key != self.observed {
            self.observed = key.clone();
            self.changed = Instant::now();
            self.error = None;
        }
        if let Some(task) = &self.task {
            match task.try_recv() {
                Ok((built, result, time)) => {
                    self.task = None;
                    if built == self.key() {
                        match result {
                            Ok(parts) => {
                                self.active = self.active.min(parts.len().saturating_sub(1));
                                self.parts = parts;
                                self.built = built;
                                self.last_duration = time;
                                self.error = None;
                                self.message = "Vorschau aktuell.".into();
                            }
                            Err(error) => self.error = Some(error),
                        }
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.task = None;
                    self.error = Some("Geometrieberechnung wurde unerwartet beendet.".into());
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if self.task.is_none()
            && key != self.built
            && self.error.is_none()
            && self.changed.elapsed() > Duration::from_millis(300)
        {
            if let Err(error) = self.cfg.validate() {
                self.error = Some(error.to_string());
            } else {
                let cfg = self.cfg.clone();
                let context = ctx.clone();
                let (sender, receiver) = mpsc::channel();
                self.task = Some(receiver);
                std::thread::spawn(move || {
                    let start = Instant::now();
                    let result = std::panic::catch_unwind(|| model::generate(&cfg))
                        .map_err(|_| "Geometrieberechnung fehlgeschlagen.".into())
                        .and_then(|r| r.map_err(|e| format!("{e:#}")));
                    let _ = sender.send((key, result, start.elapsed()));
                    context.request_repaint();
                });
            }
        }
        if self.task.is_some() || self.observed != self.built {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
    fn reset_view(&mut self) {
        self.yaw = -0.38;
        self.pitch = 0.84;
        self.zoom = 0.88;
        self.pan = Vec2::ZERO;
    }
    fn save(&mut self) {
        self.show_library(true);
    }
    fn save_file_as(&mut self) {
        if let Err(error) = self.cfg.validate() {
            self.operation_error = Some(format!("Entwurf kann nicht gespeichert werden: {error}"));
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .set_file_name("mein-gridfinity.gridfinity.json")
            .set_directory(self.root.join("examples"))
            .add_filter("Gridfinity-Projekt", &["json"])
            .save_file()
        else {
            return;
        };
        match library::save_design_file(&path, &self.cfg) {
            Ok(()) => {
                self.message = format!("Projekt gespeichert: {}", path.display());
                self.saved_path = Some(path);
                self.saved_key = self.key();
                self.pending_save_finished();
            }
            Err(e) => self.operation_error = Some(format!("{e:#}")),
        }
    }
    fn load_path(&mut self, path: PathBuf) {
        let result = (|| -> anyhow::Result<Design> {
            anyhow::ensure!(
                fs::metadata(&path)?.len() < 8_000_000,
                "Projektdatei ist zu groß."
            );
            Design::from_json(&fs::read(&path)?)
        })();
        match result {
            Ok(cfg) => self.request_replace(cfg, Some(path)),
            Err(e) => {
                self.operation_error =
                    Some(format!("Projektdatei konnte nicht geöffnet werden: {e:#}"))
            }
        }
    }
    fn open(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_directory(self.root.join("examples"))
            .add_filter("Gridfinity-Projekt", &["json"])
            .pick_file()
        {
            self.load_path(path);
        }
    }
    fn load_example(&mut self, json: &str) {
        let result = Design::from_json(json.as_bytes());
        match result {
            Ok(cfg) => self.request_replace(cfg, None),
            Err(e) => {
                self.operation_error = Some(format!("Beispiel konnte nicht geöffnet werden: {e:#}"))
            }
        }
    }
    fn export(&mut self, all: bool) {
        if !self.fresh() {
            return;
        }
        if all {
            let Some(folder) = rfd::FileDialog::new()
                .set_title("Zielordner für alle Druckteile")
                .pick_folder()
            else {
                return;
            };
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            let folder = folder.join(format!("Gridfinity-{stamp}"));
            let result = (|| -> anyhow::Result<()> {
                fs::create_dir_all(&folder)?;
                for p in &self.parts {
                    model::export_stl(p, &folder.join(format!("{}.stl", p.name)))?;
                }
                fs::write(
                    folder.join("Projekt.gridfinity.json"),
                    serde_json::to_vec_pretty(&self.cfg)?,
                )?;
                fs::write(
                    folder.join("Druckteile.json"),
                    serde_json::to_vec_pretty(
                        &self
                            .parts
                            .iter()
                            .map(|p| (&p.name, &p.report))
                            .collect::<Vec<_>>(),
                    )?,
                )?;
                Ok(())
            })();
            match result {
                Ok(()) => {
                    self.message = format!("Alle Druckteile gespeichert: {}", folder.display())
                }
                Err(e) => self.operation_error = Some(format!("Export fehlgeschlagen: {e:#}")),
            }
        } else {
            let Some(p) = self.parts.get(self.active) else {
                return;
            };
            let Some(path) = rfd::FileDialog::new()
                .set_file_name(format!("{}.stl", p.name))
                .add_filter("STL", &["stl"])
                .save_file()
            else {
                return;
            };
            match model::export_stl(p, &path) {
                Ok(()) => self.message = format!("STL gespeichert: {}", path.display()),
                Err(e) => self.operation_error = Some(format!("Export fehlgeschlagen: {e:#}")),
            }
        }
    }
    fn add_cutout(&mut self) {
        if self.cfg.cutouts.len() >= 40 {
            self.operation_error =
                Some("Pro Einsatz sind höchstens 40 Aussparungen möglich.".into());
            return;
        }
        let available = (self.cfg.height() - 1.4 - self.cfg.floor_z()).max(1.0);
        let c = Cutout {
            depth: 8.0_f64.min(available),
            ..Default::default()
        };
        self.cfg.cutouts.push(c);
        self.selection = Some(Selection::Cutout(self.cfg.cutouts.len() - 1));
        self.view_plan = true;
    }
    fn wall_controls(&mut self, ui: &mut egui::Ui) {
        use wall_editor::Mode;
        if ui
            .selectable_label(
                self.wall_editor.mode == Mode::None,
                "Keine Trennwände · ein offenes Fach",
            )
            .clicked()
        {
            self.wall_editor.clear(&mut self.cfg);
            self.selection = None;
            self.wall_start = None;
            self.wall_drag = None;
            self.tool = PlanTool::Select;
        }
        if ui
            .selectable_label(self.wall_editor.mode == Mode::Grid, "Gleichmäßige Fächer")
            .clicked()
        {
            self.wall_editor.grid(&mut self.cfg);
            self.selection = None;
            self.view_plan = true;
            self.tool = PlanTool::Select;
        }
        if ui
            .selectable_label(self.wall_editor.mode == Mode::Free, "Eigene Wände")
            .clicked()
        {
            self.wall_editor.mode = Mode::Free;
            self.view_plan = true;
            self.tool = PlanTool::Select;
        }
        if self.wall_editor.mode == Mode::Grid {
            let before = (
                self.wall_editor.rows,
                self.wall_editor.cols,
                self.wall_editor.thickness,
                self.wall_editor.height,
            );
            ui.add_space(8.0);
            egui::Grid::new("division-count")
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Spalten");
                    ui.add(egui::DragValue::new(&mut self.wall_editor.cols).range(1..=8));
                    ui.end_row();
                    ui.label("Reihen");
                    ui.add(egui::DragValue::new(&mut self.wall_editor.rows).range(1..=8));
                    ui.end_row();
                });
            dimension(
                ui,
                "Trennwandstärke",
                &mut self.wall_editor.thickness,
                0.8..=4.0,
            );
            let max_height = self.cfg.height() - self.cfg.floor_z();
            dimension(
                ui,
                "Wandhöhe",
                &mut self.wall_editor.height,
                2.0..=max_height,
            );
            if before
                != (
                    self.wall_editor.rows,
                    self.wall_editor.cols,
                    self.wall_editor.thickness,
                    self.wall_editor.height,
                )
            {
                self.wall_editor.grid(&mut self.cfg);
            }
            ui.label(
                RichText::new(format!(
                    "{} gleich große Fächer",
                    self.wall_editor.rows * self.wall_editor.cols
                ))
                .color(theme::palette(ui.ctx()).accent),
            );
            ui.label(
                RichText::new(
                    "Eine Wand im Grundriss verschieben macht daraus eine eigene Aufteilung.",
                )
                .small()
                .color(theme::palette(ui.ctx()).muted),
            );
        }
        if self.wall_editor.mode == Mode::Free {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(self.cfg.walls.len() < 40, egui::Button::new("+ Waagerecht"))
                    .clicked()
                {
                    self.add_wall(true);
                }
                if ui
                    .add_enabled(self.cfg.walls.len() < 40, egui::Button::new("+ Senkrecht"))
                    .clicked()
                {
                    self.add_wall(false);
                }
            });
            if ui
                .add_enabled(
                    self.cfg.walls.len() < 40,
                    egui::Button::new("Freie Wand ziehen"),
                )
                .clicked()
            {
                self.view_plan = true;
                self.tool = PlanTool::Wall;
                self.wall_start = None;
            }
            ui.label(
                RichText::new("Wand ziehen = verschieben. Runde Griffe ziehen = Endpunkte ändern.")
                    .small()
                    .color(theme::palette(ui.ctx()).muted),
            );
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                for i in 0..self.cfg.walls.len() {
                    if ui
                        .selectable_label(
                            matches!(self.selection,Some(Selection::Wall(j)) if j==i),
                            format!("Wand {}", i + 1),
                        )
                        .clicked()
                    {
                        self.selection = Some(Selection::Wall(i));
                        self.view_plan = true;
                    }
                }
            });
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !self.wall_editor.undo.is_empty(),
                    egui::Button::new("Rückgängig"),
                )
                .clicked()
            {
                self.wall_editor.undo(&mut self.cfg);
                self.selection = None;
            }
            if ui
                .add_enabled(
                    !self.wall_editor.redo.is_empty(),
                    egui::Button::new("Wiederholen"),
                )
                .clicked()
            {
                self.wall_editor.redo(&mut self.cfg);
                self.selection = None;
            }
        });
    }
    fn wall_inspector(&mut self, ctx: &egui::Context) {
        if !self.view_plan || self.cfg.product != Product::Bin {
            return;
        }
        egui::SidePanel::left("wall-properties")
            .exact_width(235.0)
            .resizable(false)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(12.0);
                    eyebrow(ui, "AUSWAHL");
                    if let Some(Selection::Wall(i)) = self.selection
                        && i < self.cfg.walls.len()
                    {
                        ui.separator();
                        ui.label(
                            RichText::new(format!("Wand {} bearbeiten", i + 1))
                                .strong()
                                .color(theme::palette(ui.ctx()).accent),
                        );
                        let original = self.cfg.walls[i].clone();
                        let max_h = (self.cfg.height() - self.cfg.floor_z()).max(2.0);
                        let max = wall_editor::bounds(&self.cfg);
                        let w = &mut self.cfg.walls[i];
                        dimension(ui, "Stärke", &mut w.thickness, 0.8..=4.0);
                        dimension(ui, "Höhe", &mut w.height, 2.0..=max_h);
                        ui.label(
                            RichText::new("Wandhöhe ab dem Innenboden")
                                .small()
                                .color(theme::palette(ui.ctx()).muted),
                        );
                        let old_length = (w.x2 - w.x1).hypot(w.y2 - w.y1);
                        let mut length = old_length;
                        dimension(
                            ui,
                            "Länge",
                            &mut length,
                            1.0..=(2.0 * max[0]).hypot(2.0 * max[1]),
                        );
                        if (length - old_length).abs() > 1e-6 {
                            w.x2 = w.x1 + (w.x2 - w.x1) * length / old_length;
                            w.y2 = w.y1 + (w.y2 - w.y1) * length / old_length;
                        }
                        egui::CollapsingHeader::new("Exakte Position (mm)").show(ui, |ui| {
                            coords(ui, "Start", &mut w.x1, &mut w.y1);
                            coords(ui, "Ende", &mut w.x2, &mut w.y2);
                        });
                        w.x1 = w.x1.clamp(-max[0], max[0]);
                        w.x2 = w.x2.clamp(-max[0], max[0]);
                        w.y1 = w.y1.clamp(-max[1], max[1]);
                        w.y2 = w.y2.clamp(-max[1], max[1]);
                        if (w.x2 - w.x1).hypot(w.y2 - w.y1) < 1.0 {
                            *w = original.clone();
                        }
                        if *w != original {
                            let mut previous = self.cfg.walls.clone();
                            previous[i] = original;
                            self.wall_editor.checkpoint(&previous);
                            self.wall_editor.mode = wall_editor::Mode::Free;
                        }
                        if let Some(measure) = wall_editor::spans(&self.cfg.walls, i, max) {
                            ui.separator();
                            ui.label(RichText::new("Freie Fachmaße").strong());
                            let widths = measure.widths();
                            let names = if measure.axis == 0 {
                                ["Links", "Rechts"]
                            } else {
                                ["Unten", "Oben"]
                            };
                            ui.label(format!(
                                "{}: {} mm",
                                names[0],
                                plan_dimensions::editable_number(widths[0])
                            ));
                            ui.label(format!(
                                "{}: {} mm",
                                names[1],
                                plan_dimensions::editable_number(widths[1])
                            ));
                            let mut distance = widths[0];
                            let limit =
                                (measure.high - measure.low - self.cfg.walls[i].thickness).max(0.0);
                            dimension(
                                ui,
                                &format!("{} einstellen", names[0]),
                                &mut distance,
                                0.0..=limit,
                            );
                            if (distance - widths[0]).abs() > 1e-6 {
                                self.wall_editor.checkpoint(&self.cfg.walls);
                                let mut delta = [0.0; 2];
                                delta[measure.axis] = distance - widths[0];
                                self.cfg.walls[i] =
                                    wall_editor::move_wall(&self.cfg.walls[i], delta, max);
                                self.wall_editor.mode = wall_editor::Mode::Free;
                            }
                        }
                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(
                                    self.cfg.walls.len() < 40,
                                    egui::Button::new("Duplizieren"),
                                )
                                .clicked()
                            {
                                self.wall_editor.checkpoint(&self.cfg.walls);
                                let duplicate =
                                    wall_editor::move_wall(&self.cfg.walls[i], [5.0, 5.0], max);
                                self.cfg.walls.push(duplicate);
                                self.selection = Some(Selection::Wall(self.cfg.walls.len() - 1));
                                self.wall_editor.mode = wall_editor::Mode::Free;
                            }
                            if ui.button("90° drehen").clicked() {
                                self.wall_editor.checkpoint(&self.cfg.walls);
                                let w = &mut self.cfg.walls[i];
                                let cx = (w.x1 + w.x2) / 2.0;
                                let cy = (w.y1 + w.y2) / 2.0;
                                let dx = (w.x2 - w.x1) / 2.0;
                                let dy = (w.y2 - w.y1) / 2.0;
                                w.x1 = cx + dy;
                                w.y1 = cy - dx;
                                w.x2 = cx - dy;
                                w.y2 = cy + dx;
                                wall_editor::fit_walls(&mut self.cfg);
                                self.wall_editor.mode = wall_editor::Mode::Free;
                            }
                        });
                        if ui.button("Ausgewählte Wand entfernen").clicked() {
                            self.wall_editor.checkpoint(&self.cfg.walls);
                            self.cfg.walls.remove(i);
                            self.selection = None;
                        }
                    } else {
                        ui.heading("Neue Wand");
                        let max_height = self.cfg.height() - self.cfg.floor_z();
                        dimension(
                            ui,
                            "Wandhöhe",
                            &mut self.wall_editor.height,
                            2.0..=max_height,
                        );
                        dimension(ui, "Stärke", &mut self.wall_editor.thickness, 0.8..=4.0);
                        ui.label(
                            "Klicke eine Wand im Grundriss an, um ihre Eigenschaften zu ändern.",
                        );
                        ui.add_space(14.0);
                        ui.separator();
                        ui.add_space(10.0);
                        ui.label(RichText::new("Direkt bearbeiten").strong());
                        ui.label("Wand ziehen: verschieben");
                        ui.label("Runde Griffe: Endpunkte ändern");
                        ui.label("Strg halten: Achse fixieren");
                        ui.label("Escape: Zeichnen abbrechen");
                        ui.label("Strg+Z: Rückgängig");
                    }
                });
            });
    }
    fn add_wall(&mut self, horizontal: bool) {
        if self.cfg.walls.len() >= 40 {
            self.operation_error =
                Some("Pro Behälter sind höchstens 40 Trennwände möglich.".into());
            return;
        }
        let max = wall_editor::bounds(&self.cfg);
        let axis = usize::from(horizontal);
        let coordinate = (0..20)
            .map(|i| {
                if i % 2 == 0 {
                    i as f64 / 2.0 * 5.0
                } else {
                    -(i as f64 + 1.0) / 2.0 * 5.0
                }
            })
            .find(|p| {
                p.abs() < max[axis] - 1.0
                    && !self.cfg.walls.iter().any(|w| {
                        if horizontal {
                            (w.y1 - p).abs() < 1.0 && (w.y2 - p).abs() < 1.0
                        } else {
                            (w.x1 - p).abs() < 1.0 && (w.x2 - p).abs() < 1.0
                        }
                    })
            })
            .unwrap_or(0.0);
        self.wall_editor.checkpoint(&self.cfg.walls);
        self.cfg.walls.push(if horizontal {
            Wall {
                x1: -max[0],
                y1: coordinate,
                x2: max[0],
                y2: coordinate,
                thickness: self.wall_editor.thickness,
                height: self
                    .wall_editor
                    .height
                    .clamp(2.0, self.cfg.height() - self.cfg.floor_z()),
            }
        } else {
            Wall {
                x1: coordinate,
                y1: -max[1],
                x2: coordinate,
                y2: max[1],
                thickness: self.wall_editor.thickness,
                height: self
                    .wall_editor
                    .height
                    .clamp(2.0, self.cfg.height() - self.cfg.floor_z()),
            }
        });
        self.selection = Some(Selection::Wall(self.cfg.walls.len() - 1));
        self.tool = PlanTool::Select;
        self.view_plan = true;
    }
    fn cutout_controls(&mut self, ui: &mut egui::Ui) {
        if self.cfg.product == Product::Insert {
            if ui.button("+ Aussparung hinzufügen").clicked() {
                self.add_cutout();
            }
            for (i, c) in self.cfg.cutouts.iter().enumerate() {
                if ui
                    .selectable_label(
                        matches!(self.selection,Some(Selection::Cutout(j)) if j==i),
                        &c.name,
                    )
                    .clicked()
                {
                    self.selection = Some(Selection::Cutout(i));
                }
            }
            if let Some(Selection::Cutout(i)) = self.selection
                && let Some(c) = self.cfg.cutouts.get_mut(i)
            {
                ui.separator();
                ui.add(egui::TextEdit::singleline(&mut c.name).char_limit(40));
                egui::ComboBox::from_id_salt("shape")
                    .selected_text(c.shape.label())
                    .show_ui(ui, |ui| {
                        for s in [
                            CutShape::Circle,
                            CutShape::Rectangle,
                            CutShape::Hexagon,
                            CutShape::Stl,
                        ] {
                            ui.selectable_value(&mut c.shape, s, s.label());
                        }
                    });
                dimension(
                    ui,
                    if c.shape == CutShape::Circle {
                        "Durchmesser"
                    } else if c.shape == CutShape::Hexagon {
                        "Schlüsselweite"
                    } else {
                        "Breite"
                    },
                    &mut c.width,
                    2.0..=200.0,
                );
                if matches!(c.shape, CutShape::Rectangle | CutShape::Stl) {
                    dimension(ui, "Länge", &mut c.length, 2.0..=200.0);
                }
                dimension(ui, "Tiefe", &mut c.depth, 1.0..=80.0);
                dimension(ui, "Zugabe gesamt", &mut c.clearance, 0.0..=2.0);
                ui.horizontal(|ui| {
                    ui.label("Drehung");
                    ui.add(
                        egui::DragValue::new(&mut c.angle)
                            .range(-180.0..=180.0)
                            .suffix("°"),
                    );
                });
                coords(ui, "Position", &mut c.x, &mut c.y);
                if c.shape == CutShape::Stl {
                    if ui.button("Werkzeug-STL auswählen …").clicked()
                        && let Some(path) = rfd::FileDialog::new()
                            .add_filter("STL", &["stl"])
                            .pick_file()
                    {
                        match model::tool_dimensions(&path) {
                            Ok([width, length]) => {
                                c.width = width;
                                c.length = length;
                                c.stl_path = Some(path.to_string_lossy().into_owned());
                            }
                            Err(error) => {
                                self.operation_error = Some(format!(
                                    "Werkzeugdatei konnte nicht geöffnet werden: {error}"
                                ))
                            }
                        }
                    }
                    ui.label(RichText::new("Die Aussparung folgt dem Umriss der STL von oben. Breite und Länge ändern die Größe.").small().color(theme::palette(ui.ctx()).muted));
                }
                if ui.button("Aussparung entfernen").clicked() {
                    self.cfg.cutouts.remove(i);
                    self.selection = None;
                }
            }
        }
    }
    fn plan(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui
                .selectable_label(
                    matches!(self.tool, PlanTool::Select),
                    "Auswählen / Verschieben",
                )
                .clicked()
            {
                self.tool = PlanTool::Select;
                self.wall_start = None;
            }
            if self.cfg.product == Product::Bin
                && ui
                    .selectable_label(matches!(self.tool, PlanTool::Wall), "Freie Wand ziehen")
                    .clicked()
            {
                self.tool = PlanTool::Wall;
            }
            ui.label("Raster");
            ui.add(
                egui::DragValue::new(&mut self.snap)
                    .range(0.5..=7.0)
                    .speed(0.5)
                    .custom_formatter(|value, _| plan_dimensions::editable_number(value))
                    .custom_parser(|text| plan_dimensions::parse_number(text).ok())
                    .suffix(" mm"),
            );
            ui.checkbox(&mut self.show_plan_grid, "Hilfsraster");
        });
        ui.label(
            RichText::new(if matches!(self.tool, PlanTool::Wall) {
                "Drücken und ziehen. Wandenden rasten an anderen Wänden ein. Strg: Achse fixieren. Escape: abbrechen."
            } else {
                "Maß anklicken und Wert eingeben. Wände und runde Wandgriffe lassen sich ziehen."
            })
            .small()
            .color(theme::palette(ui.ctx()).muted),
        );
        let (response, painter) =
            ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
        let rect = response.rect;
        painter.rect_filled(rect, 12, theme::palette(ui.ctx()).canvas);
        let width = self.cfg.width();
        let length = self.cfg.length();
        let scale = (rect.width() - 80.0).min(rect.height() - 80.0) as f64 / width.max(length);
        let centre = rect.center();
        let to_screen = |p: [f64; 2]| {
            Pos2::new(
                centre.x + (p[0] * scale) as f32,
                centre.y - (p[1] * scale) as f32,
            )
        };
        let to_model = |p: Pos2| {
            [
                (p.x - centre.x) as f64 / scale,
                -(p.y - centre.y) as f64 / scale,
            ]
        };
        let outer = egui::Rect::from_min_max(
            to_screen([-width / 2.0, length / 2.0]),
            to_screen([width / 2.0, -length / 2.0]),
        );
        painter.rect_filled(outer, 8, theme::palette(ui.ctx()).panel);
        painter.rect_stroke(
            outer,
            8,
            Stroke::new(
                (self.cfg.wall * scale).max(2.0) as f32,
                theme::palette(ui.ctx()).accent,
            ),
            egui::StrokeKind::Inside,
        );
        for x in (1..self.cfg.grid_x).filter(|_| self.show_plan_grid) {
            let p = -(self.cfg.grid_x as f64) * 21.0 + x as f64 * 42.0;
            painter.line_segment(
                [to_screen([p, -length / 2.0]), to_screen([p, length / 2.0])],
                Stroke::new(1.0_f32, theme::palette(ui.ctx()).grid),
            );
        }
        for y in (1..self.cfg.grid_y).filter(|_| self.show_plan_grid) {
            let p = -(self.cfg.grid_y as f64) * 21.0 + y as f64 * 42.0;
            painter.line_segment(
                [to_screen([-width / 2.0, p]), to_screen([width / 2.0, p])],
                Stroke::new(1.0_f32, theme::palette(ui.ctx()).grid),
            );
        }
        if self.cfg.product == Product::Bin {
            for (i, w) in self.cfg.walls.iter().enumerate() {
                let colour = if matches!(self.selection,Some(Selection::Wall(j)) if j==i) {
                    theme::palette(ui.ctx()).selected
                } else {
                    theme::palette(ui.ctx()).accent
                };
                let footprint = model::divider_footprint(
                    w,
                    self.cfg
                        .walls
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| *j != i)
                        .map(|(_, wall)| wall),
                );
                painter.add(egui::Shape::convex_polygon(
                    footprint.into_iter().map(to_screen).collect(),
                    colour,
                    Stroke::NONE,
                ));
                // Wall names live in the element list and selection panel. Drawing
                // them at wall midpoints overlaps dimensions and crossing walls.
                if matches!(self.selection,Some(Selection::Wall(j)) if j==i) {
                    for point in [[w.x1, w.y1], [w.x2, w.y2]] {
                        painter.circle_filled(to_screen(point), 7.0, colour);
                        painter.circle_stroke(
                            to_screen(point),
                            7.0,
                            Stroke::new(2.0_f32, theme::palette(ui.ctx()).canvas),
                        );
                    }
                }
            }
        }
        if self.cfg.product == Product::Insert {
            for (i, c) in self.cfg.cutouts.iter().enumerate() {
                let centre = to_screen([c.x, c.y]);
                let colour = if matches!(self.selection,Some(Selection::Cutout(j)) if j==i) {
                    theme::palette(ui.ctx()).selected
                } else {
                    theme::palette(ui.ctx()).accent
                };
                let w = (c.width + c.clearance) * scale;
                let d = (c.length + c.clearance) * scale;
                if c.shape == CutShape::Circle {
                    painter.circle_filled(centre, (w / 2.0) as f32, theme::palette(ui.ctx()).panel);
                    painter.circle_stroke(centre, (w / 2.0) as f32, Stroke::new(2.0_f32, colour));
                } else {
                    let angle = c.angle.to_radians();
                    let raw = if c.shape == CutShape::Hexagon {
                        (0..6)
                            .map(|i| {
                                let a = i as f64 * std::f64::consts::PI / 3.0;
                                [w / 3.0_f64.sqrt() * a.cos(), w / 3.0_f64.sqrt() * a.sin()]
                            })
                            .collect::<Vec<_>>()
                    } else {
                        vec![
                            [-w / 2.0, -d / 2.0],
                            [w / 2.0, -d / 2.0],
                            [w / 2.0, d / 2.0],
                            [-w / 2.0, d / 2.0],
                        ]
                    };
                    let points = raw
                        .iter()
                        .map(|p| {
                            centre
                                + Vec2::new(
                                    (p[0] * angle.cos() - p[1] * angle.sin()) as f32,
                                    -(p[0] * angle.sin() + p[1] * angle.cos()) as f32,
                                )
                        })
                        .collect();
                    painter.add(egui::Shape::convex_polygon(
                        points,
                        theme::palette(ui.ctx()).panel,
                        Stroke::new(2.0_f32, colour),
                    ));
                }
                painter.text(
                    centre,
                    egui::Align2::CENTER_CENTER,
                    &c.name,
                    egui::FontId::proportional(13.0),
                    colour,
                );
            }
        }
        let max = [width / 2.0 - self.cfg.wall, length / 2.0 - self.cfg.wall];
        self.validate_dimension_edit();
        let measures = self.plan_measures(&to_screen, max);
        let measure_rects: Vec<_> = measures.iter().map(|m| m.label_rect(&painter)).collect();
        // Dimension labels sit above the canvas: clicking one must not deselect
        // its wall or begin moving an endpoint underneath it.
        let dimension_input = self.dimension_edit.is_some()
            || (self.wall_drag.is_none()
                && ui.input(|input| {
                    [input.pointer.interact_pos(), input.pointer.press_origin()]
                        .into_iter()
                        .flatten()
                        .any(|pos| measure_rects.iter().any(|rect| rect.contains(pos)))
                }));
        if self.cfg.product == Product::Bin
            && self.cfg.walls.is_empty()
            && !matches!(self.tool, PlanTool::Wall)
        {
            painter.text(
                centre,
                egui::Align2::CENTER_CENTER,
                "Ein offenes Fach",
                egui::FontId::proportional(24.0),
                theme::palette(ui.ctx()).muted,
            );
        }
        if !dimension_input && let Some(pos) = response.interact_pointer_pos() {
            let raw = to_model(pos);
            let mut p = wall_editor::snap(raw, max, self.snap);
            let origin = ui
                .input(|i| i.pointer.press_origin())
                .map(to_model)
                .unwrap_or(raw);
            if self.cfg.product == Product::Bin {
                if matches!(self.tool, PlanTool::Wall) {
                    if response.drag_started()
                        && outer.contains(to_screen(origin))
                        && self.cfg.walls.len() < 40
                    {
                        self.wall_start = Some(wall_editor::snap_endpoint(
                            origin,
                            max,
                            self.snap,
                            &self.cfg.walls,
                            None,
                            None,
                        ));
                        self.wall_drag = Some(WallDrag::Draw);
                    }
                    if let Some(start) = self.wall_start {
                        let axis_anchor = ui
                            .input(|i| i.modifiers.ctrl || i.modifiers.shift)
                            .then_some(start);
                        p = wall_editor::snap_endpoint(
                            raw,
                            max,
                            self.snap,
                            &self.cfg.walls,
                            None,
                            axis_anchor,
                        );
                        painter.line_segment(
                            [to_screen(start), to_screen(p)],
                            Stroke::new(4.0_f32, theme::palette(ui.ctx()).selected),
                        );
                        painter.circle_filled(
                            to_screen(start),
                            5.0,
                            theme::palette(ui.ctx()).accent,
                        );
                        if self.show_measurements {
                            let a = to_screen(start);
                            let b = to_screen(p);
                            let offset = (b - a).rot90().normalized() * 20.0;
                            measurement(
                                &painter,
                                a + offset,
                                b + offset,
                                format!("{:.1} mm", (p[0] - start[0]).hypot(p[1] - start[1])),
                            );
                        }
                        if response.drag_stopped()
                            && (p[0] - start[0]).hypot(p[1] - start[1]) >= 1.0
                        {
                            self.wall_editor.checkpoint(&self.cfg.walls);
                            self.cfg.walls.push(Wall {
                                x1: start[0],
                                y1: start[1],
                                x2: p[0],
                                y2: p[1],
                                thickness: self.wall_editor.thickness,
                                height: self
                                    .wall_editor
                                    .height
                                    .clamp(2.0, self.cfg.height() - self.cfg.floor_z()),
                            });
                            self.selection = Some(Selection::Wall(self.cfg.walls.len() - 1));
                            self.wall_editor.mode = wall_editor::Mode::Free;
                            self.tool = PlanTool::Select;
                        }
                    }
                } else {
                    if response.clicked() || response.drag_started() {
                        let mut hit = None;
                        // The selected wall's handles take priority at intersections.
                        let selected = if let Some(Selection::Wall(i)) = self.selection {
                            Some(i)
                        } else {
                            None
                        };
                        let order = selected.into_iter().chain(
                            (0..self.cfg.walls.len())
                                .rev()
                                .filter(|i| Some(*i) != selected),
                        );
                        for i in order {
                            let w = &self.cfg.walls[i];
                            for (end, point) in [(false, [w.x1, w.y1]), (true, [w.x2, w.y2])] {
                                if (origin[0] - point[0]).hypot(origin[1] - point[1]) < 9.0 / scale
                                {
                                    hit = Some((i, Some(end)));
                                    break;
                                }
                            }
                            if hit.is_some() {
                                break;
                            }
                            if distance_to_segment(origin, [w.x1, w.y1], [w.x2, w.y2])
                                < (8.0 / scale).max(w.thickness / 2.0)
                            {
                                hit = Some((i, None));
                                break;
                            }
                        }
                        self.selection = hit.map(|(i, _)| Selection::Wall(i));
                        if response.drag_started()
                            && let Some((i, end)) = hit
                        {
                            self.wall_editor.checkpoint(&self.cfg.walls);
                            self.wall_editor.mode = wall_editor::Mode::Free;
                            self.wall_drag = Some(if let Some(end) = end {
                                WallDrag::End(i, self.cfg.walls[i].clone(), end)
                            } else {
                                WallDrag::Move(i, self.cfg.walls[i].clone(), origin)
                            });
                        }
                    }
                    if response.dragged()
                        && let Some(drag) = &self.wall_drag
                    {
                        match drag {
                            WallDrag::Move(i, wall, start) => {
                                let mut delta = std::array::from_fn(|j| {
                                    ((raw[j] - start[j]) / self.snap).round() * self.snap
                                });
                                if ui.input(|i| i.modifiers.ctrl) {
                                    delta = wall_editor::axis_lock([0.0, 0.0], delta);
                                }
                                self.cfg.walls[*i] = wall_editor::move_wall(wall, delta, max);
                            }
                            WallDrag::End(i, wall, end) => {
                                let axis_anchor =
                                    if ui.input(|i| i.modifiers.ctrl || i.modifiers.shift) {
                                        Some(if *end {
                                            [wall.x1, wall.y1]
                                        } else {
                                            [wall.x2, wall.y2]
                                        })
                                    } else {
                                        None
                                    };
                                p = wall_editor::snap_endpoint(
                                    raw,
                                    max,
                                    self.snap,
                                    &self.cfg.walls,
                                    Some(*i),
                                    axis_anchor,
                                );
                                self.cfg.walls[*i] = wall_editor::endpoint(wall, *end, p);
                            }
                            WallDrag::Draw => {}
                        }
                    }
                }
            } else if self.cfg.product == Product::Insert {
                if response.clicked() || response.drag_started() {
                    self.selection = self
                        .cfg
                        .cutouts
                        .iter()
                        .enumerate()
                        .rev()
                        .find(|(_, c)| {
                            let extent = c.half_extents();
                            (origin[0] - c.x).abs() <= extent[0] + 2.0
                                && (origin[1] - c.y).abs() <= extent[1] + 2.0
                        })
                        .map(|(i, _)| Selection::Cutout(i));
                }
                if response.dragged()
                    && let Some(Selection::Cutout(i)) = self.selection
                    && let Some(c) = self.cfg.cutouts.get_mut(i)
                {
                    let extents = c.half_extents();
                    let limit = [
                        (max[0] - extents[0] - 1.0).max(0.0),
                        (max[1] - extents[1] - 1.0).max(0.0),
                    ];
                    let position = p.map_each_clamped(limit);
                    c.x = position[0];
                    c.y = position[1];
                }
            }
        }
        if response.drag_stopped() {
            self.wall_drag = None;
            self.wall_start = None;
        }
        if self.dimension_edit.is_none()
            && !self.dialog_active()
            && ui.input(|i| i.key_pressed(egui::Key::Escape))
        {
            if let Some(WallDrag::Move(i, original, _) | WallDrag::End(i, original, _)) =
                self.wall_drag.take()
                && let Some(wall) = self.cfg.walls.get_mut(i)
            {
                *wall = original;
            }
            self.wall_start = None;
            self.tool = PlanTool::Select;
        }
        if self.dimension_edit.is_none()
            && !self.dialog_active()
            // Buttons can retain keyboard focus too. Only an actual text input
            // should prevent wall shortcuts (TextEdit emits IME output).
            && ui.ctx().output(|o| o.ime.is_none())
            && self.cfg.product == Product::Bin
        {
            if ui.input(|i| i.key_pressed(egui::Key::Delete))
                && let Some(Selection::Wall(index)) = self.selection
                && index < self.cfg.walls.len()
            {
                self.wall_editor.checkpoint(&self.cfg.walls);
                self.cfg.walls.remove(index);
                self.selection = None;
                self.wall_drag = None;
            }
            let redo = ui.input_mut(|i| {
                i.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::Z,
                )
            });
            let undo = ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z));
            if redo || undo {
                if redo {
                    self.wall_editor.redo(&mut self.cfg);
                } else {
                    self.wall_editor.undo(&mut self.cfg);
                }
                self.selection = None;
                self.wall_drag = None;
            }
        }
        let measures = self.plan_measures(&to_screen, max);
        self.edit_plan_measures(ui, &painter, max, &measures);
        painter.text(
            rect.left_bottom() + Vec2::new(16.0, -16.0),
            egui::Align2::LEFT_BOTTOM,
            format!("{:.1} × {:.1} mm", width, length).replace('.', ","),
            egui::FontId::monospace(11.0),
            theme::palette(ui.ctx()).muted,
        );
    }
    fn viewport(&mut self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::drag());
        let rect = response.rect;
        let centre = rect.center();
        painter.rect_filled(rect, 0, theme::palette(ui.ctx()).canvas);
        if response.dragged() {
            let d = ui.input(|i| i.pointer.delta());
            if ui.input(|i| i.modifiers.shift)
                || response.dragged_by(egui::PointerButton::Middle)
                || response.dragged_by(egui::PointerButton::Secondary)
            {
                self.pan += d;
            } else {
                self.yaw += d.x * 0.008;
                self.pitch = (self.pitch + d.y * 0.008).clamp(-1.55, 1.55);
            }
        }
        if response.hovered() {
            self.zoom = (self.zoom * (ui.input(|i| i.smooth_scroll_delta.y) * 0.0015).exp())
                .clamp(0.3, 3.0);
        }
        if self.parts.is_empty() {
            painter.text(
                centre,
                egui::Align2::CENTER_CENTER,
                "Vorschau wird berechnet …",
                egui::FontId::proportional(24.0),
                theme::palette(ui.ctx()).muted,
            );
            return;
        }
        let bed_available = self.printer_panel.settings.usable_size().is_some();
        let print_preview = if bed_available && self.printer_bed_view {
            match print_preview::build(
                &self.parts[self.active],
                &self.printer_panel.settings,
                self.show_build_plate,
            ) {
                Ok(preview) => Some(preview),
                Err(error) => {
                    painter.text(
                        centre,
                        egui::Align2::CENTER_CENTER,
                        format!("Druckvorschau nicht verfügbar: {error}"),
                        egui::FontId::proportional(16.0),
                        theme::palette(ui.ctx()).muted,
                    );
                    return;
                }
            }
        } else {
            None
        };
        let mut vertices = Vec::<[f32; 3]>::new();
        let mut faces = Vec::<([usize; 3], [u8; 3])>::new();
        for (i, p) in self.parts.iter().enumerate() {
            if print_preview.is_some() {
                break;
            }
            if !self.assembly && i != self.active {
                continue;
            }
            let offset = vertices.len();
            vertices.extend(p.vertices.iter().map(|v| {
                if self.assembly {
                    p.assembly_vertex(*v, self.explode, self.cfg.grid_y)
                } else {
                    *v
                }
            }));
            faces.extend(p.faces.iter().map(|f| (f.map(|j| j + offset), p.colour)));
        }
        if let Some(preview) = &print_preview {
            vertices.clone_from(&preview.vertices);
            faces.clone_from(&preview.faces);
        }
        let mut low = [f32::INFINITY; 3];
        let mut high = [f32::NEG_INFINITY; 3];
        for v in &vertices {
            for j in 0..3 {
                low[j] = low[j].min(v[j]);
                high[j] = high[j].max(v[j]);
            }
        }
        if let Some(preview) = &print_preview {
            low = preview.low;
            high = preview.high;
        }
        let mut middle: [f32; 3] = std::array::from_fn(|j| (low[j] + high[j]) / 2.0);
        let extent = (0..3)
            .map(|j| high[j] - low[j])
            .fold(0.0_f32, f32::max)
            .max(1.0);
        if self.show_build_plate && print_preview.is_none() {
            let span = (extent * 6.0 / 10.0).ceil() as i32;
            let radius = span as f32 * 10.0;
            for i in -span..=span {
                let position = i as f32 * 10.0;
                let colour = if i % 5 == 0 {
                    [147, 167, 150]
                } else {
                    [215, 222, 209]
                };
                let half_width = if i % 5 == 0 { 0.09 } else { 0.045 } * extent / 84.0;
                for vertical in [false, true] {
                    let points = if vertical {
                        [
                            [position - half_width, -radius, -0.03],
                            [position + half_width, -radius, -0.03],
                            [position + half_width, radius, -0.03],
                            [position - half_width, radius, -0.03],
                        ]
                    } else {
                        [
                            [-radius, position - half_width, -0.03],
                            [radius, position - half_width, -0.03],
                            [radius, position + half_width, -0.03],
                            [-radius, position + half_width, -0.03],
                        ]
                    };
                    let n = vertices.len();
                    vertices.extend(points);
                    faces.extend([([n, n + 1, n + 2], colour), ([n, n + 2, n + 3], colour)]);
                }
            }
        }
        let scale = rect.width().min(rect.height()) * 0.7 / extent * self.zoom;
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        middle[0] -= (cy * self.pan.x - sp * sy * self.pan.y) / scale;
        middle[1] -= (-sy * self.pan.x - sp * cy * self.pan.y) / scale;
        middle[2] += cp * self.pan.y / scale;
        if let Some(gpu) = &mut self.gpu {
            let key = format!(
                "{}-{}-{}-{}-{}-{}-{:?}-{}",
                self.built,
                self.active,
                self.assembly,
                self.explode.to_bits(),
                self.show_build_plate,
                print_preview.is_some(),
                self.printer_panel.settings.volume(),
                self.printer_panel.settings.margin.to_bits()
            );
            let ratio = ui.ctx().pixels_per_point();
            let texture = gpu.render(
                &key,
                &vertices,
                &faces,
                [
                    (rect.width() * ratio) as u32,
                    (rect.height() * ratio) as u32,
                ],
                middle,
                extent,
                self.yaw,
                self.pitch,
                self.zoom,
                theme::palette(ui.ctx()).canvas,
            );
            painter.image(
                texture,
                rect,
                egui::Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }

        ui.scope_builder(
            egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                rect.min + Vec2::splat(12.0),
                Vec2::new(rect.width() - 24.0, 30.0),
            )),
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    if bed_available {
                        let before = self.printer_bed_view;
                        ui.selectable_value(&mut self.printer_bed_view, false, "Modell");
                        ui.selectable_value(&mut self.printer_bed_view, true, "Druckbett");
                        if before != self.printer_bed_view { self.reset_view(); }
                        ui.separator();
                    }
                    if matches!(self.cfg.product, Product::Bin | Product::Insert)
                        && ui.button("Grundriss bearbeiten").clicked()
                    {
                        self.view_plan = true;
                    }
                    if self.parts.len() > 1 {
                        egui::ComboBox::from_id_salt("viewport-part")
                            .selected_text(&self.parts[self.active].name)
                            .show_ui(ui, |ui| {
                                for (i, p) in self.parts.iter().enumerate() {
                                    if ui.selectable_value(&mut self.active, i, &p.name).clicked() {
                                        self.assembly = false;
                                    }
                                }
                            });
                        let assembly_label =
                            if matches!(self.cfg.product, Product::Bin | Product::Insert) {
                                "Montiert anzeigen"
                            } else {
                                "Alle Druckteile"
                            };
                        if print_preview.is_none() { ui.checkbox(&mut self.assembly, assembly_label); }
                    }
                    if self.cfg.product == Product::DeskBox && self.assembly && print_preview.is_none() {
                        ui.label("Schubladen öffnen");
                        ui.add(egui::Slider::new(&mut self.explode, 0.0..=1.0).show_value(false));
                        if self.cfg.printed_push_open {
                            ui.label(RichText::new("Mechanik schematisch").small()).on_hover_text("Die Federverformung ist eine schematische Vorschau, keine Kraft- oder Bewegungssimulation.");
                        }
                    }
                });
            },
        );
        if let Some(preview) = &print_preview {
            self.print_preview_overlay(ui, &painter, rect, preview, middle, scale);
        }
        let fit = egui::Rect::from_center_size(
            rect.center_bottom() + Vec2::new(0.0, -27.0),
            Vec2::new(156.0, 30.0),
        );
        if ui.put(fit,egui::Button::new("Passend zur Ansicht").corner_radius(6)).on_hover_text("Ansicht zurücksetzen · Ziehen: drehen · Shift+Ziehen: verschieben · Mausrad: zoomen").clicked(){self.reset_view();}
        self.orientation_gizmo(ui, &painter, rect);
    }
    fn orientation_gizmo(&mut self, ui: &mut egui::Ui, painter: &egui::Painter, rect: egui::Rect) {
        let origin = rect.right_bottom() + Vec2::new(-52.0, -51.0);
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let mut axes = [
            (
                "X",
                [cy, -sp * sy, -cp * sy],
                Color32::from_rgb(230, 71, 105),
            ),
            (
                "Y",
                [-sy, -sp * cy, -cp * cy],
                Color32::from_rgb(138, 185, 52),
            ),
            ("Z", [0.0, -cp, sp], Color32::from_rgb(57, 151, 241)),
        ];
        axes.sort_by(|a, b| a.1[2].total_cmp(&b.1[2]));
        for (name, axis, colour) in axes {
            let end = origin + Vec2::new(axis[0], axis[1]) * 30.0;
            painter.line_segment([origin, end], Stroke::new(2.0_f32, colour));
            painter.circle_filled(end, 8.0, colour);
            painter.text(
                end,
                egui::Align2::CENTER_CENTER,
                name,
                egui::FontId::proportional(10.0),
                Color32::from_rgb(12, 24, 34),
            );
            if ui
                .interact(
                    egui::Rect::from_center_size(end, Vec2::splat(18.0)),
                    ui.id().with(name),
                    egui::Sense::click(),
                )
                .on_hover_text(format!("Ansicht entlang {name}"))
                .clicked()
            {
                self.pan = Vec2::ZERO;
                match name {
                    "Z" => {
                        self.yaw = 0.0;
                        self.pitch = std::f32::consts::FRAC_PI_2;
                    }
                    "X" => {
                        self.yaw = std::f32::consts::FRAC_PI_2;
                        self.pitch = 0.0;
                    }
                    _ => {
                        self.yaw = 0.0;
                        self.pitch = 0.0;
                    }
                }
            }
        }
    }
}
impl eframe::App for Studio {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.handle_close_request(ctx);
        self.poll(ctx);
        self.printer_panel.refresh(&self.built, &self.parts);
        self.reference_header(ctx);
        self.atelier_rail(ctx);
        self.reference_dialogs(ctx);
        self.library_dialogs(ctx);
        self.printer_dialog(ctx);
        self.transition_dialog(ctx);
        self.operation_error_dialog(ctx);
        if self.show_home {
            self.dimension_edit = None;
            self.home_screen(ctx);
            return;
        }
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            if let Some(error) = &self.error {
                ui.colored_label(theme::palette(ui.ctx()).error, error);
                return;
            }
            ui.horizontal(|ui| {
                if self.task.is_some() {
                    ui.spinner();
                    ui.label("Vorschau wird berechnet …");
                } else if !self.fresh() {
                    ui.label(
                        RichText::new("Vorschau noch nicht aktuell; Export ist gesperrt.")
                            .small()
                            .color(theme::palette(ui.ctx()).muted),
                    );
                } else {
                    ui.label(
                        RichText::new(&self.message)
                            .small()
                            .color(theme::palette(ui.ctx()).muted),
                    );
                }
            });
        });
        self.reference_controls(ctx);
        self.wall_inspector(ctx);
        self.validate_dimension_edit();
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::palette(ctx).canvas))
            .show(ctx, |ui| {
                if self.view_plan && matches!(self.cfg.product, Product::Bin | Product::Insert) {
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        if ui.button("Zurück zur 3D-Ansicht").clicked() {
                            self.view_plan = false;
                        }
                        ui.label(
                            RichText::new("Grundriss bearbeiten")
                                .small()
                                .color(theme::palette(ui.ctx()).muted),
                        );
                    });
                    self.plan(ui);
                } else {
                    self.viewport(ui);
                }
            });
        if let Some(path) = ctx.input(|i| i.raw.dropped_files.iter().find_map(|f| f.path.clone()))
            && path.extension().is_some_and(|e| e == "json")
        {
            self.load_path(path);
        }
    }
}
trait ClampPoint {
    fn map_each_clamped(self, max: [f64; 2]) -> Self;
}
impl ClampPoint for [f64; 2] {
    fn map_each_clamped(self, max: [f64; 2]) -> Self {
        [
            self[0].clamp(-max[0], max[0]),
            self[1].clamp(-max[1], max[1]),
        ]
    }
}
fn eyebrow(ui: &mut egui::Ui, t: &str) {
    ui.label(
        RichText::new(t)
            .monospace()
            .size(11.0)
            .color(theme::palette(ui.ctx()).accent),
    );
}
fn measurement(painter: &egui::Painter, a: Pos2, b: Pos2, label: String) {
    let colour = theme::palette(painter.ctx()).accent;
    if a.distance(b) >= 2.0 {
        let direction = (b - a).normalized();
        let normal = direction.rot90();
        painter.line_segment([a, b], Stroke::new(1.0_f32, colour));
        for (point, sign) in [(a, 1.0), (b, -1.0)] {
            for side in [-1.0, 1.0] {
                painter.line_segment(
                    [
                        point,
                        point + direction * (sign * 7.0) + normal * (side * 3.0),
                    ],
                    Stroke::new(1.0_f32, colour),
                );
            }
        }
    }
    let text = painter.layout_no_wrap(
        label.replace('.', ","),
        egui::FontId::proportional(13.0),
        colour,
    );
    let pos = a + (b - a) * 0.5 - text.size() * 0.5;
    painter.rect_filled(
        text.rect.translate(pos.to_vec2()).expand(4.0),
        3,
        theme::palette(painter.ctx()).canvas,
    );
    painter.galley(pos, text, colour);
}
fn dimension(ui: &mut egui::Ui, name: &str, value: &mut f64, range: std::ops::RangeInclusive<f64>) {
    ui.horizontal(|ui| {
        ui.label(name);
        ui.add(
            egui::DragValue::new(value)
                .range(range)
                .clamp_existing_to_range(false)
                .speed(0.1)
                .custom_formatter(|value, _| plan_dimensions::editable_number(value))
                .custom_parser(|text| plan_dimensions::parse_number(text).ok())
                .suffix(" mm"),
        );
    });
}
fn coords(ui: &mut egui::Ui, name: &str, x: &mut f64, y: &mut f64) {
    ui.horizontal(|ui| {
        ui.label(name);
        ui.label("X");
        ui.add(
            egui::DragValue::new(x)
                .speed(0.5)
                .custom_formatter(|value, _| plan_dimensions::editable_number(value))
                .custom_parser(|text| plan_dimensions::parse_number(text).ok()),
        );
        ui.label("Y");
        ui.add(
            egui::DragValue::new(y)
                .speed(0.5)
                .custom_formatter(|value, _| plan_dimensions::editable_number(value))
                .custom_parser(|text| plan_dimensions::parse_number(text).ok()),
        );
    });
}
fn distance_to_segment(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let u = [b[0] - a[0], b[1] - a[1]];
    let t = (((p[0] - a[0]) * u[0] + (p[1] - a[1]) * u[1]) / (u[0] * u[0] + u[1] * u[1]).max(1e-9))
        .clamp(0.0, 1.0);
    (p[0] - a[0] - t * u[0]).hypot(p[1] - a[1] - t * u[1])
}
fn main() -> eframe::Result {
    let args: Vec<_> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--export") {
        let result = (|| -> anyhow::Result<()> {
            let cfg_path = args
                .get(i + 1)
                .ok_or_else(|| anyhow::anyhow!("Projektdatei fehlt"))?;
            let cfg = Design::from_json(&fs::read(cfg_path)?)?;
            let output = args
                .iter()
                .position(|a| a == "--output")
                .and_then(|i| args.get(i + 1))
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("output"));
            fs::create_dir_all(&output)?;
            let parts = model::generate(&cfg)?;
            for p in &parts {
                model::export_stl(p, &output.join(format!("{}.stl", p.name)))?;
                println!("{}: {} triangles", p.name, p.report.triangles);
            }
            fs::write(
                output.join("report.json"),
                serde_json::to_vec_pretty(
                    &parts
                        .iter()
                        .map(|p| (&p.name, &p.report))
                        .collect::<Vec<_>>(),
                )?,
            )?;
            Ok(())
        })();
        if let Err(e) = result {
            eprintln!("FEHLER: {e:#}");
            std::process::exit(1);
        }
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_icon(atelier_ui::window_icon())
            .with_inner_size([1440.0, 940.0])
            .with_min_inner_size([1100.0, 740.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Gridfinity Studio · Design 2",
        options,
        Box::new(move |cc| {
            let mut studio = Studio::new(cc);
            if let Some(i) = args.iter().position(|a| a == "--project")
                && let Some(path) = args.get(i + 1)
            {
                studio.load_path(Path::new(path).to_path_buf());
            }
            Ok(Box::new(studio))
        }),
    )
}
