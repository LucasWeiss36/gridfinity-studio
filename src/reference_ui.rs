use crate::{
    PlanTool, Selection, Studio,
    design::{Design, LabelArea, Product},
    theme, wall_editor,
};
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};

impl Studio {
    pub fn reference_header(&mut self, ctx: &egui::Context) {
        let palette = theme::palette(ctx);
        // TopBottomPanel clips to its declared height even if its content makes
        // the following panels start lower. Budget both text rows, the menu row,
        // their gap and the frame margins to avoid an unpainted strip below it.
        let title_height = ctx
            .fonts_mut(|fonts| {
                fonts.row_height(&egui::FontId::proportional(10.0))
                    + 2.0
                    + fonts.row_height(&egui::FontId::proportional(23.0))
            })
            .ceil()
            .max(39.0);
        let menu_height = ctx.style().spacing.interact_size.y.max(28.0);
        let header_height = (title_height + 2.0 + menu_height + 12.0).ceil();
        egui::TopBottomPanel::top("atelier-header")
            .exact_height(header_height)
            .frame(
                egui::Frame::new()
                    .fill(palette.panel)
                    .inner_margin(egui::Margin::symmetric(22, 6)),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 2.0);
                ui.spacing_mut().button_padding = egui::vec2(12.0, 7.0);
                ui.horizontal(|ui| {
                    let title_width = if self.show_home {
                        ui.available_width() - 105.0
                    } else {
                        ui.available_width() - 475.0
                    };
                    ui.allocate_ui_with_layout(
                        egui::vec2(title_width.max(110.0), title_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.label(
                                RichText::new("GRIDFINITY  /  DESIGN 02")
                                    .size(10.0)
                                    .color(palette.muted),
                            );
                            let title = if self.show_home {
                                "Deine Werkstatt"
                            } else {
                                &self.cfg.title
                            };
                            ui.add(
                                egui::Label::new(
                                    RichText::new(title).size(23.0).strong().color(palette.text),
                                )
                                .truncate(),
                            );
                        },
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !self.show_home {
                            if ui
                                .add_enabled(
                                    self.fresh(),
                                    egui::Button::new(
                                        RichText::new("STL exportieren  ↗")
                                            .color(palette.on_primary)
                                            .strong(),
                                    )
                                    .fill(palette.primary)
                                    .corner_radius(8)
                                    .min_size(egui::vec2(145.0, 35.0)),
                                )
                                .clicked()
                            {
                                self.export_dialog = true;
                            }
                            if ui
                                .add(
                                    egui::Button::new("Speichern").min_size(egui::vec2(90.0, 35.0)),
                                )
                                .clicked()
                            {
                                self.save();
                            }
                            if ui
                                .add(
                                    egui::Button::new("Eigenschaften")
                                        .selected(self.sidebar_visible)
                                        .min_size(egui::vec2(100.0, 35.0)),
                                )
                                .on_hover_text("Eigenschaften ein- oder ausblenden")
                                .clicked()
                            {
                                self.sidebar_visible = !self.sidebar_visible;
                            }
                        }
                        if ui
                            .add(egui::Button::new("Projekte").min_size(egui::vec2(80.0, 35.0)))
                            .clicked()
                        {
                            self.show_library(false);
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().button_padding = egui::vec2(8.0, 3.0);
                    ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::NONE;
                    ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
                    if !self.show_home {
                        ui.menu_button("Datei", |ui| {
                            ui.menu_button("Neu", |ui| {
                                for (p, name) in products() {
                                    if ui.button(name).on_hover_text(p.label()).clicked() {
                                        self.new_reference_model(p);
                                        ui.close();
                                    }
                                }
                            });
                            if ui.button("Projektdatei öffnen …").clicked() {
                                self.open();
                                ui.close();
                            }
                            if ui.button("Projekt speichern …").clicked() {
                                self.save();
                                ui.close();
                            }
                            if ui.button("In Projektablage speichern …").clicked() {
                                self.show_library(true);
                                ui.close();
                            }
                            if ui.button("Als Projektdatei speichern …").clicked() {
                                self.save_file_as();
                                ui.close();
                            }
                            ui.separator();
                            self.example_menu(ui);
                            if ui.button("Modelltyp auswählen").clicked() {
                                self.show_home = true;
                                ui.close();
                            }
                        });
                        ui.menu_button("Bearbeiten", |ui| {
                            if ui
                                .add_enabled(
                                    !self.wall_editor.undo.is_empty(),
                                    egui::Button::new("Rückgängig   Strg+Z"),
                                )
                                .clicked()
                            {
                                self.wall_editor.undo(&mut self.cfg);
                                self.selection = None;
                                ui.close();
                            }
                            if ui
                                .add_enabled(
                                    !self.wall_editor.redo.is_empty(),
                                    egui::Button::new("Wiederholen   Strg+Shift+Z"),
                                )
                                .clicked()
                            {
                                self.wall_editor.redo(&mut self.cfg);
                                self.selection = None;
                                ui.close();
                            }
                            if self.cfg.product == Product::Bin
                                && ui.button("Keine Trennwände").clicked()
                            {
                                self.wall_editor.clear(&mut self.cfg);
                                self.selection = None;
                                ui.close();
                            }
                        });
                        ui.menu_button("Werkzeuge", |ui| {
                            if self.cfg.product == Product::Bin {
                                if ui
                                    .add_enabled(
                                        self.cfg.walls.len() < 40,
                                        egui::Button::new("Wand zeichnen"),
                                    )
                                    .clicked()
                                {
                                    self.wall_editor.mode = wall_editor::Mode::Free;
                                    self.view_plan = true;
                                    self.tool = PlanTool::Wall;
                                    self.wall_start = None;
                                    ui.close();
                                }
                                if ui
                                    .add_enabled(
                                        self.cfg.walls.len() < 40,
                                        egui::Button::new("Waagerechte Wand"),
                                    )
                                    .clicked()
                                {
                                    self.wall_editor.mode = wall_editor::Mode::Free;
                                    self.add_wall(true);
                                    ui.close();
                                }
                                if ui
                                    .add_enabled(
                                        self.cfg.walls.len() < 40,
                                        egui::Button::new("Senkrechte Wand"),
                                    )
                                    .clicked()
                                {
                                    self.wall_editor.mode = wall_editor::Mode::Free;
                                    self.add_wall(false);
                                    ui.close();
                                }
                            }
                            if self.cfg.product == Product::Insert
                                && ui
                                    .add_enabled(
                                        self.cfg.cutouts.len() < 40,
                                        egui::Button::new("Aussparung hinzufügen"),
                                    )
                                    .clicked()
                            {
                                self.add_cutout();
                                ui.close();
                            }
                            if ui.button("3D-Ansicht zurücksetzen").clicked() {
                                self.reset_view();
                                ui.close();
                            }
                        });
                        ui.menu_button("Ansicht", |ui| {
                            ui.checkbox(&mut self.sidebar_visible, "Seitenleiste");
                            ui.checkbox(
                                &mut self.show_build_plate,
                                if self.printer_bed_view
                                    && self.printer_panel.settings.usable_size().is_some()
                                {
                                    "Druckbett anzeigen"
                                } else {
                                    "Raster anzeigen"
                                },
                            );
                            ui.checkbox(&mut self.show_measurements, "Maße anzeigen");
                            ui.checkbox(&mut self.show_plan_grid, "Hilfsraster");
                            if ui.button("Draufsicht / Grundriss").clicked() {
                                self.view_plan = true;
                                ui.close();
                            }
                            if ui.button("3D-Ansicht").clicked() {
                                self.view_plan = false;
                                ui.close();
                            }
                        });
                        ui.menu_button("Hilfe", |ui| {
                            if ui.button("Bedienung & Tastenkürzel").clicked() {
                                self.help_visible = true;
                                ui.close();
                            }
                        });
                    } else {
                        ui.label(
                            RichText::new("Entwerfen. Anpassen. Drucken.")
                                .size(12.0)
                                .color(palette.muted),
                        );
                    }
                });
            });
        if !self.dialog_active() && ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::K)) {
            self.show_home = !self.show_home;
        }
    }
    pub(crate) fn new_reference_model(&mut self, product: Product) {
        let mut cfg = Design {
            product,
            title: product.label().into(),
            ..Design::default()
        };
        if product == Product::LabelInsert {
            cfg.label_area = LabelArea::SlideIn;
        }
        self.request_replace(cfg, None);
    }
    pub(crate) fn example_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Beispiele", |ui| {
            for (name, json) in [
                (
                    "Fachbox",
                    include_str!("../examples/compartments.gridfinity.json"),
                ),
                (
                    "Werkzeugeinsatz",
                    include_str!("../examples/tools.gridfinity.json"),
                ),
                (
                    "Schubladenbox",
                    include_str!("../examples/deskbox.gridfinity.json"),
                ),
                (
                    "Drucköffnung – Federprototyp",
                    include_str!("../examples/deskbox-push-open.gridfinity.json"),
                ),
                (
                    "Grundplatte mit Clip-Aufnahmen",
                    include_str!("../examples/baseplate-clips.gridfinity.json"),
                ),
                (
                    "Grundplatte mit Magneten und Schrauben",
                    include_str!("../examples/baseplate-magnets.gridfinity.json"),
                ),
                (
                    "Verbindungsclip",
                    include_str!("../examples/clip.gridfinity.json"),
                ),
                (
                    "Box mit Etikettenfeld",
                    include_str!("../examples/label-recess.gridfinity.json"),
                ),
                (
                    "Box mit Schildaufnahme",
                    include_str!("../examples/label-slide-in-box.gridfinity.json"),
                ),
                (
                    "Einschubschild einzeln",
                    include_str!("../examples/label-insert.gridfinity.json"),
                ),
            ] {
                if ui.button(name).clicked() {
                    self.load_example(json);
                    ui.close();
                }
            }
        });
    }
    pub fn reference_controls(&mut self, ctx: &egui::Context) {
        if !self.sidebar_visible {
            return;
        }
        let palette = theme::palette(ctx);
        let old_max = wall_editor::bounds(&self.cfg);
        let old_wall_height = self.cfg.height() - self.cfg.floor_z();
        let old_thickness = self.wall_editor.thickness;
        let old_key = self.key();
        let has_details = !matches!(self.cfg.product, Product::Baseplate | Product::Clip);
        egui::SidePanel::right("atelier-properties")
            .exact_width(340.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(palette.panel).inner_margin(18))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 10.0);
                ui.spacing_mut().button_padding = egui::vec2(9.0, 6.0);
                ui.label(RichText::new("EIGENSCHAFTEN").size(10.0).color(palette.muted));
                ui.label(RichText::new(self.cfg.product.label()).size(21.0).strong().color(palette.text));
                ui.add_space(6.0);
                egui::ScrollArea::vertical().id_salt("atelier-properties-content").show(ui, |ui| {
                    section(ui, "Abmessungen", true, |ui| {
                        if !matches!(self.cfg.product, Product::Clip | Product::LabelInsert) {
                        ui.horizontal(|ui| {
                            field_label(ui, "Rastergröße");
                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), 24.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(rect, 6, palette.soft);
                            ui.painter().text(
                                rect.right_center() - egui::vec2(8.0, 0.0),
                                egui::Align2::RIGHT_CENTER, "42 mm",
                                egui::FontId::proportional(12.0), ui.visuals().text_color(),
                            );
                            response.on_hover_text("Gridfinity-Standard: 42 mm je Rastereinheit");
                        });
                        units(ui, "X Einheiten", &mut self.cfg.grid_x, 1..=8);
                        units(ui, "Y Einheiten", &mut self.cfg.grid_y, 1..=8);
                        if matches!(self.cfg.product, Product::Bin | Product::Insert) {
                            units(ui, "Z Einheiten", &mut self.cfg.height_u, 2..=15);
                        }
                        }
                        let measurement_width = ui.available_width();
                        egui::Frame::new().stroke(Stroke::new(1.0_f32, palette.border))
                            .corner_radius(7).inner_margin(7).show(ui, |ui| {
                                ui.set_width((measurement_width - 16.0).max(0.0));
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Maße").small().color(palette.muted));
                                    if ui.add(egui::Button::new(RichText::new(if self.show_inches {
                                        "Millimeter anzeigen"
                                    } else { "Zoll anzeigen" }).small().color(palette.accent)).frame(false)).clicked() {
                                        self.show_inches = !self.show_inches;
                                    }
                                });
                                if let Some(part) = self.parts.get(self.active).filter(|_| self.fresh()) {
                                let dimensions = if self.printer_bed_view && !self.view_plan {
                                    if let Some(usable) = self.printer_panel.settings.usable_size() {
                                        let mut size = crate::model::print_dimensions(part).unwrap_or(part.report.dimensions);
                                        if crate::printer::fit(size, usable) == crate::printer::Fit::Rotate90 { size.swap(0, 1); }
                                        size
                                    } else { part.report.dimensions }
                                } else { part.report.dimensions };
                                let factor = if self.show_inches { 1.0 / 25.4 } else { 1.0 };
                                let unit = if self.show_inches { "in" } else { "mm" };
                                ui.horizontal(|ui| {
                                    for (axis, value) in ["X", "Y", "Z"].into_iter().zip(dimensions) {
                                        ui.label(RichText::new(format!("{axis} {} {unit}", decimal(value * factor, 1))).small().color(palette.muted));
                                    }
                                });
                                } else {
                                    ui.label(RichText::new(if self.error.is_some() { "Maße nicht verfügbar" } else { "Maße werden berechnet …" }).small().color(palette.muted));
                                }
                            });
                    });
                    section_separator(ui);
                    section(ui, "Ausführung", true, |ui| {
                        match self.cfg.product {
                            Product::Bin | Product::Insert => {
                                feature_mode(ui, "Sockelmagnete", &mut self.cfg.magnets, &mut self.cfg.magnet_corners_only);
                                feature_mode(ui, "Schraublöcher", &mut self.cfg.screws, &mut self.cfg.screw_corners_only);
                                lip_mode(ui, &mut self.cfg);
                                slider(ui, "Außenwandstärke", &mut self.cfg.wall, 1.0..=4.0, " mm");
                                if self.cfg.product == Product::Bin {
                                    slider(ui, if self.wall_editor.mode == wall_editor::Mode::Grid { "Wandstärke" } else { "Neue Wandstärke" }, &mut self.wall_editor.thickness, 0.8..=4.0, " mm");
                                }
                                section(ui, "Boden & Magnetmaße", false, |ui| {
                                    slider(ui, "Bodenstärke", &mut self.cfg.floor, 1.2..=5.0, " mm");
                                    if self.cfg.magnets {
                                        slider(ui, "Magnet-Ø", &mut self.cfg.magnet_diameter, 5.0..=8.0, " mm");
                                        slider(ui, "Magnettiefe", &mut self.cfg.magnet_depth, 1.0..=3.0, " mm");
                                    }
                                    ui.label(RichText::new(format!("Stapelhöhe {} mm", decimal(self.cfg.stacking_pitch(), 2))).small().color(palette.muted));
                                });
                            }
                            Product::Baseplate => {
                                crate::baseplate_ui::controls(ui, &mut self.cfg);
                            }
                            Product::Clip => {
                                slider(ui, "Clip-Toleranz", &mut self.cfg.clip_clearance, -0.1..=0.2, " mm");
                                ui.label(RichText::new("Für Grundplatten mit Clip-Aufnahmen. Separat drucken und von oben aufdrücken.").small().color(palette.muted));
                            }
                            Product::LabelInsert => {
                                ui.label(RichText::new("Nutzfläche und Einschubspiel von der passenden Box übernehmen.").small().color(palette.muted));
                            }
                            Product::DeskBox => {
                                units(ui, "Schubladen", &mut self.cfg.drawer_count, 1..=5);
                                units(ui, "Höhe je Schublade (U)", &mut self.cfg.drawer_height_u, 2..=10);
                                slider(ui, "Spiel je Seite", &mut self.cfg.drawer_clearance, 0.2..=1.0, " mm");
                                ui.checkbox(&mut self.cfg.top_grid, "Gridfinity-Raster oben");
                                ui.separator();
                                ui.checkbox(&mut self.cfg.printed_push_open, "Drucköffnung mit gedruckter Feder");
                                if self.cfg.printed_push_open {
                                    ui.colored_label(palette.warning, "Prototyp – Testdruck erforderlich");
                                    slider(ui, "Federstärke", &mut self.cfg.spring_thickness, 0.8..=1.2, " mm");
                                    ui.label(RichText::new("Vollständig gedruckt. Zwei Federn und ein Rastfinger je Schublade separat exportieren.").small().color(palette.muted));
                                    ui.label(RichText::new("Gehäuse 70 mm tiefer · mindestens 2 Felder breit und 3 U hoch.").small().color(palette.muted));
                                    ui.label(RichText::new("Montage und Testablauf: README, Abschnitt Drucköffnung.").small().color(palette.muted));
                                }
                            }
                        }
                    });
                    if matches!(self.cfg.product, Product::Bin | Product::Insert) {
                        section_separator(ui);
                        let count = if self.cfg.product == Product::Insert { self.cfg.cutouts.len() } else { self.cfg.walls.len() };
                        let element_state = egui::collapsing_header::CollapsingState::load_with_default_open(
                            ui.ctx(), ui.make_persistent_id("reference-elements"), true,
                        ).show_header(ui, |ui| {
                            ui.label(RichText::new(format!("Elemente ({count})")).strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.menu_button(RichText::new("Hinzufügen +").color(palette.accent), |ui| {
                                    if self.cfg.product == Product::Bin {
                                        for (label, horizontal) in [("Waagerechte Wand", true), ("Senkrechte Wand", false)] {
                                            if ui.add_enabled(self.cfg.walls.len() < 40, egui::Button::new(label)).clicked() {
                                                self.wall_editor.mode = wall_editor::Mode::Free;
                                                self.add_wall(horizontal); ui.close();
                                            }
                                        }
                                        if ui.add_enabled(self.cfg.walls.len() < 40, egui::Button::new("Wand frei zeichnen")).clicked() {
                                            self.wall_editor.mode = wall_editor::Mode::Free;
                                            self.tool = PlanTool::Wall; self.view_plan = true; self.wall_start = None; ui.close();
                                        }
                                    } else if ui.add_enabled(self.cfg.cutouts.len() < 40, egui::Button::new("Aussparung")).clicked() { self.add_cutout(); ui.close(); }
                                });
                            });
                        });
                        element_state.body_unindented(|ui| {
                            if self.cfg.product == Product::Bin {
                                if self.cfg.walls.is_empty() {
                                    ui.label(RichText::new("Ein Fach ohne Trennwände").small().color(palette.muted));
                                } else {
                                    for i in 0..self.cfg.walls.len() {
                                        let w = &self.cfg.walls[i];
                                        let length = (w.x2 - w.x1).hypot(w.y2 - w.y1);
                                        let selected = matches!(self.selection, Some(Selection::Wall(j)) if i == j);
                                        ui.horizontal(|ui| {
                                            if ui.selectable_label(selected, format!("Wand {}", i + 1)).clicked() {
                                                self.selection = Some(Selection::Wall(i)); self.view_plan = true; self.tool = PlanTool::Select;
                                            }
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.label(RichText::new(format!("{} × {} mm", decimal(length, 1), decimal(w.height, 1))).small().color(palette.muted));
                                            });
                                        });
                                    }
                                    if ui.add(egui::Button::new(RichText::new("Keine Trennwände").small().color(palette.accent)).frame(false)).clicked() {
                                        self.wall_editor.clear(&mut self.cfg); self.selection = None; self.tool = PlanTool::Select;
                                    }
                                }
                                section(ui, "Fachaufteilung", false, |ui| { self.wall_controls(ui); });
                            } else {
                                self.cutout_controls(ui);
                            }
                        });
                    }
                    if has_details {
                        section_separator(ui);
                        section(ui, "Beschriftung", true, |ui| { self.label_controls(ui); });
                    }
                });
            });
        if self.cfg.product == Product::Bin {
            let max = wall_editor::bounds(&self.cfg);
            if old_max != max || old_thickness != self.wall_editor.thickness {
                if self.wall_editor.mode == wall_editor::Mode::Grid {
                    self.wall_editor.grid(&mut self.cfg);
                } else {
                    self.wall_editor.checkpoint(&self.cfg.walls);
                    for w in &mut self.cfg.walls {
                        w.x1 *= max[0] / old_max[0];
                        w.x2 *= max[0] / old_max[0];
                        w.y1 *= max[1] / old_max[1];
                        w.y2 *= max[1] / old_max[1];
                    }
                }
            }
            if old_max != max || old_wall_height != self.cfg.height() - self.cfg.floor_z() {
                wall_editor::fit_walls(&mut self.cfg);
            }
        }
        if self.key() != old_key {
            clamp_label_dimensions(&mut self.cfg);
        }
    }
    fn label_controls(&mut self, ui: &mut egui::Ui) {
        let palette = theme::palette(ui.ctx());
        let standalone = self.cfg.product == Product::LabelInsert;
        let box_product = matches!(self.cfg.product, Product::Bin | Product::Insert);
        if box_product {
            let old_area = self.cfg.label_area;
            ui.horizontal(|ui| {
                field_label(ui, "Ausführung");
                egui::ComboBox::from_id_salt("label-area-mode")
                    .selected_text(match self.cfg.label_area {
                        LabelArea::None => "Direkter Text",
                        LabelArea::Recess => "Etikettenfeld",
                        LabelArea::SlideIn => "Einschubschild",
                    })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.cfg.label_area,
                            LabelArea::None,
                            "Direkter Text",
                        );
                        ui.selectable_value(
                            &mut self.cfg.label_area,
                            LabelArea::Recess,
                            "Etikettenfeld",
                        );
                        ui.selectable_value(
                            &mut self.cfg.label_area,
                            LabelArea::SlideIn,
                            "Einschubschild",
                        );
                    });
            });
            if old_area != LabelArea::SlideIn && self.cfg.label_area == LabelArea::SlideIn {
                self.assembly = true;
                self.active = 0;
            }
            if old_area != self.cfg.label_area {
                clamp_label_dimensions(&mut self.cfg);
            }
        }
        let recess = box_product && self.cfg.label_area == LabelArea::Recess;
        let slide_in = standalone || (box_product && self.cfg.label_area == LabelArea::SlideIn);
        if recess || slide_in {
            let max_width = self.cfg.label_width_limit();
            let max_height = self.cfg.label_height_limit();
            if slide_in {
                ui.label(RichText::new("Nutzfläche des Schilds").strong());
            }
            slider(
                ui,
                if slide_in { "Nutzbreite" } else { "Feldbreite" },
                &mut self.cfg.label_width,
                10.0..=max_width,
                " mm",
            );
            slider(
                ui,
                if slide_in { "Nutzhöhe" } else { "Feldhöhe" },
                &mut self.cfg.label_height,
                3.0..=max_height,
                " mm",
            );
            if recess {
                let max_depth = self.cfg.label_depth_limit();
                slider(
                    ui,
                    "Vertiefung",
                    &mut self.cfg.label_depth,
                    0.1..=max_depth,
                    " mm",
                );
                ui.label(
                    RichText::new("Leeres Feld an der Vorderseite für Klebeetiketten.")
                        .small()
                        .color(palette.muted),
                );
            } else {
                slider(
                    ui,
                    "Einschubspiel",
                    &mut self.cfg.label_insert_clearance,
                    0.15..=0.6,
                    " mm",
                );
                ui.label(
                    RichText::new("Schild: 1 mm Rand je Seite, 1,2 mm dick.")
                        .small()
                        .color(palette.muted),
                );
                if !standalone {
                    ui.label(RichText::new("Schild separat exportieren und drucken, dann von oben in die Frontführung schieben.").small().color(palette.muted));
                    ui.label(
                        RichText::new(format!(
                            "Überstand vor der Box: {} mm.",
                            decimal(2.0 + 2.0 * self.cfg.label_insert_clearance, 2)
                        ))
                        .small()
                        .color(palette.muted),
                    );
                }
                ui.label(
                    RichText::new("Passung testdrucken. Mehr Spiel erleichtert das Einschieben.")
                        .small()
                        .color(palette.muted),
                );
            }
        }
        if !recess {
            ui.add(
                egui::TextEdit::singleline(&mut self.cfg.label)
                    .hint_text(if slide_in {
                        "Leer lassen für Klebeetikett"
                    } else {
                        "Text · leer lassen für keine Beschriftung"
                    })
                    .char_limit(32)
                    .desired_width(f32::INFINITY),
            );
            ui.checkbox(&mut self.cfg.engraved, "Text vertiefen");
        }
    }
    pub fn reference_dialogs(&mut self, ctx: &egui::Context) {
        let palette = theme::palette(ctx);
        let dialog_height = (ctx.content_rect().height() - 100.0).max(80.0);
        let mut help = self.help_visible;
        egui::Window::new("Bedienung")
            .open(&mut help)
            .collapsible(false)
            .resizable(false)
            .default_width(520.0)
            .max_width((ctx.content_rect().width() - 48.0).clamp(200.0, 600.0))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("help-dialog-content")
                    .max_height(dialog_height)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                ui.heading("Wände und Fächer");
                ui.label("Keine Trennwände: ein offenes Fach.");
                ui.label("Gleichmäßige Fächer: Reihen und Spalten einstellen.");
                ui.label("Eigene Wände: zeichnen oder ganze Wände verschieben.");
                ui.label("Strg halten: waagerecht oder senkrecht fixieren.");
                ui.label("Runde Griffe ziehen: Endpunkte ändern.");
                ui.label("Wanddetails: Höhe, Stärke und freie Fachmaße einstellen.");
                ui.label("Strg+Z / Strg+Shift+Z: Rückgängig / Wiederholen.");
                ui.label("Escape: Zeichnen abbrechen.");
                ui.separator();
                ui.label("Clips sind ein eigener Modelltyp. Grundplatten mit Clip-Aufnahmen erzeugen ausschließlich die Platte.");
                ui.label("Beschriftung: Direkter Text, vertieftes Etikettenfeld oder eine untere Schildaufnahme mit separatem Einschubschild.");
                ui.label("Einschubschild von oben in die Frontführung schieben. Im Export das gewünschte Druckteil wählen; Montagevorschau unter „Montiert anzeigen“.");
                ui.label("Gehäuse und Schubladen lassen sich als einzelne Druckteile exportieren.");
                    });
            });
        self.help_visible = help;
        let mut export = self.export_dialog;
        egui::Window::new("Export")
            .open(&mut export)
            .collapsible(false)
            .resizable(false)
            .default_width(400.0)
            .max_width((ctx.content_rect().width() - 48.0).max(200.0))
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("export-dialog-content")
                    .max_height(dialog_height)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.heading("STL exportieren");
                        ui.label("Format: STL · Maße in Millimetern");
                        ui.add_space(12.0);
                        if self.parts.len() > 1 {
                            ui.horizontal(|ui| {
                                ui.label("Druckteil");
                                egui::ComboBox::from_id_salt("export-part")
                                    .selected_text(
                                        self.parts
                                            .get(self.active)
                                            .map_or("Teil auswählen", |p| p.name.as_str()),
                                    )
                                    .show_ui(ui, |ui| {
                                        for (index, part) in self.parts.iter().enumerate() {
                                            if ui
                                                .selectable_value(
                                                    &mut self.active,
                                                    index,
                                                    &part.name,
                                                )
                                                .changed()
                                            {
                                                self.assembly = false;
                                            }
                                        }
                                    });
                            });
                        } else if let Some(part) = self.parts.get(self.active) {
                            ui.label(format!("Druckteil: {}", part.name));
                        }
                        self.printer_export_info(ui);
                        if ui
                            .add_enabled(
                                self.fresh(),
                                egui::Button::new(
                                    RichText::new("Ausgewähltes Teil exportieren …")
                                        .color(palette.on_primary),
                                )
                                .fill(palette.primary),
                            )
                            .clicked()
                        {
                            self.export(false);
                            self.export_dialog = false;
                        }
                        if self.parts.len() > 1
                            && ui
                                .add_enabled(
                                    self.fresh(),
                                    egui::Button::new("Alle Teile separat exportieren …"),
                                )
                                .clicked()
                        {
                            self.export(true);
                            self.export_dialog = false;
                        }
                    });
            });
        if !export {
            self.export_dialog = false;
        }
    }
    pub fn home_screen(&mut self, ctx: &egui::Context) {
        self.atelier_home(ctx);
    }
}

fn products() -> [(Product, &'static str); 6] {
    [
        (Product::Bin, "Kasten"),
        (Product::Baseplate, "Grundplatte"),
        (Product::Clip, "Clips"),
        (Product::Insert, "Werkzeugeinsatz"),
        (Product::DeskBox, "Schubladenbox"),
        (Product::LabelInsert, "Einschubschild"),
    ]
}
fn decimal(value: f64, places: usize) -> String {
    format!("{value:.places$}").replace('.', ",")
}
fn clamp_label_dimensions(cfg: &mut Design) {
    let box_product = matches!(cfg.product, Product::Bin | Product::Insert);
    if cfg.product == Product::LabelInsert || (box_product && cfg.label_area != LabelArea::None) {
        cfg.label_width = cfg.label_width.clamp(10.0, cfg.label_width_limit());
        cfg.label_height = cfg.label_height.clamp(3.0, cfg.label_height_limit());
        if box_product && cfg.label_area == LabelArea::Recess {
            cfg.label_depth = cfg.label_depth.clamp(0.1, cfg.label_depth_limit());
        } else {
            cfg.label_insert_clearance = cfg.label_insert_clearance.clamp(0.15, 0.6);
        }
    }
}
fn lip_mode(ui: &mut egui::Ui, cfg: &mut Design) {
    ui.horizontal(|ui| {
        field_label(ui, "Stapelrand");
        segmented(ui, |ui| {
            if segment(ui, cfg.lip && !cfg.thin_lip, "Standard").clicked() {
                cfg.lip = true;
                cfg.thin_lip = false;
            }
            if cfg.reference_profile && segment(ui, cfg.lip && cfg.thin_lip, "Dünn").clicked() {
                cfg.lip = true;
                cfg.thin_lip = true;
            }
            if segment(ui, !cfg.lip, "Keine").clicked() {
                cfg.lip = false;
            }
        });
    });
}
fn feature_mode(ui: &mut egui::Ui, label: &str, enabled: &mut bool, corners: &mut bool) {
    ui.horizontal(|ui| {
        field_label(ui, label);
        segmented(ui, |ui| {
            for (name, on, only_corners) in [
                ("Keine", false, false),
                ("Ecken", true, true),
                ("Alle", true, false),
            ] {
                if segment(
                    ui,
                    *enabled == on && (!on || *corners == only_corners),
                    name,
                )
                .clicked()
                {
                    *enabled = on;
                    *corners = only_corners;
                }
            }
        });
    });
}
fn section(ui: &mut egui::Ui, title: &str, open: bool, add_body: impl FnOnce(&mut egui::Ui)) {
    let palette = theme::palette(ui.ctx());
    egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        ui.make_persistent_id(title),
        open,
    )
    .show_header(ui, |ui| {
        ui.label(RichText::new(title).size(14.0).strong().color(palette.text));
    })
    .body_unindented(|ui| {
        ui.add_space(7.0);
        add_body(ui);
    });
}
fn section_separator(ui: &mut egui::Ui) {
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);
}
fn field_label(ui: &mut egui::Ui, label: &str) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(126.0, 30.0), egui::Sense::hover());
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child
        .add(egui::Label::new(label).truncate())
        .on_hover_text(label);
}
fn segmented(ui: &mut egui::Ui, add_buttons: impl FnOnce(&mut egui::Ui)) {
    let palette = theme::palette(ui.ctx());
    egui::Frame::new()
        .fill(palette.soft)
        .corner_radius(6)
        .inner_margin(2)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.spacing_mut().button_padding = egui::vec2(4.0, 3.0);
            ui.horizontal(add_buttons);
        });
}
fn segment(ui: &mut egui::Ui, selected: bool, text: &str) -> egui::Response {
    let palette = theme::palette(ui.ctx());
    ui.add(
        egui::Button::new(RichText::new(text).size(12.0).color(if selected {
            palette.accent
        } else {
            palette.muted
        }))
        .frame(true)
        .fill(if selected {
            palette.selection
        } else {
            Color32::TRANSPARENT
        })
        .stroke(Stroke::NONE)
        .corner_radius(3),
    )
}
fn units(ui: &mut egui::Ui, label: &str, value: &mut u32, range: std::ops::RangeInclusive<u32>) {
    let mut n = *value as f64;
    slider(
        ui,
        label,
        &mut n,
        *range.start() as f64..=*range.end() as f64,
        "",
    );
    *value = n.round() as u32;
}
fn slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    suffix: &str,
) {
    let palette = theme::palette(ui.ctx());
    ui.push_id(label, |ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.horizontal(|ui| {
                ui.label(RichText::new(label).size(12.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(
                        egui::DragValue::new(value)
                            .range(range.clone())
                            .clamp_existing_to_range(false)
                            .speed(if suffix.is_empty() { 1.0 } else { 0.1 })
                            .max_decimals(if suffix.is_empty() { 0 } else { 2 })
                            .suffix(suffix),
                    )
                    .on_hover_text("Klicken zum Eingeben · ziehen zum Ändern");
                });
            });
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), 19.0),
                egui::Sense::click_and_drag(),
            );
            let track = rect.shrink2(egui::vec2(7.0, 6.5));
            if (response.clicked() || response.dragged())
                && let Some(p) = response.interact_pointer_pos()
            {
                let ratio = ((p.x - track.left()) / track.width()).clamp(0.0, 1.0) as f64;
                *value = *range.start() + ratio * (*range.end() - *range.start());
            }
            let span = *range.end() - *range.start();
            let fraction = if span > f64::EPSILON {
                ((*value - *range.start()) / span).clamp(0.0, 1.0) as f32
            } else {
                0.0
            };
            let x = track.left() + track.width() * fraction;
            ui.painter().rect_filled(track, 3, palette.border);
            ui.painter().rect_filled(
                egui::Rect::from_min_max(
                    track.min,
                    egui::pos2(x.max(track.left() + 0.5), track.bottom()),
                ),
                3,
                palette.accent,
            );
            let center = egui::pos2(x, track.center().y);
            ui.painter().circle_filled(center, 7.0, palette.panel);
            ui.painter()
                .circle_stroke(center, 7.0, Stroke::new(2.0_f32, palette.accent));
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measurements_use_decimal_commas_with_the_requested_precision() {
        assert_eq!(decimal(84.0, 1), "84,0");
        assert_eq!(decimal(21.245, 2), "21,25");
        assert_eq!(decimal(-0.05, 2), "-0,05");
        assert_eq!(decimal(25.4 / 25.4, 1), "1,0");
    }

    #[test]
    fn showing_numeric_controls_preserves_loaded_precision_and_out_of_range_values() {
        let ctx = egui::Context::default();
        let mut dimensions = [6.25723, 9.5, -0.25];
        let before = dimensions;
        let mut count = 9;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                for (index, value) in dimensions.iter_mut().enumerate() {
                    ui.push_id(index, |ui| {
                        slider(ui, "Maß", value, 5.0..=8.0, " mm");
                    });
                }
                units(ui, "Anzahl", &mut count, 1..=8);
            });
        });
        assert_eq!(dimensions, before);
        assert_eq!(count, 9);
    }
}
