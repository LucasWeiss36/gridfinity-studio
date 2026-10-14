use crate::{Studio, design::Product, model, printer, printer_profiles::PROFILES, theme};
use eframe::egui::{self, Color32, RichText};

pub struct Panel {
    pub settings: printer::Settings,
    draft: Option<printer::Settings>,
    error: Option<String>,
    model_key: String,
    dimensions: Vec<Option<[f64; 3]>>,
}

impl Panel {
    pub fn new() -> Self {
        let (settings, error) = match printer::Settings::load() {
            Ok(settings) => (settings, None),
            Err(error) => (printer::Settings::default(), Some(format!("{error:#}"))),
        };
        Self {
            settings,
            draft: None,
            error,
            model_key: String::new(),
            dimensions: vec![],
        }
    }

    pub fn refresh(&mut self, key: &str, parts: &[model::Part]) {
        if self.model_key != key {
            self.model_key = key.to_owned();
            self.dimensions = parts
                .iter()
                .map(|part| model::print_dimensions(part).ok())
                .collect();
        }
    }

    pub fn open(&mut self) {
        self.draft = Some(self.settings.clone());
    }

    pub fn is_open(&self) -> bool {
        self.draft.is_some()
    }

    #[cfg(test)]
    pub fn for_test() -> Self {
        Self {
            settings: printer::Settings::default(),
            draft: None,
            error: None,
            model_key: String::new(),
            dimensions: Vec::new(),
        }
    }
}

impl Studio {
    fn printer_sidebar(&self, ui: &mut egui::Ui, settings: &printer::Settings) {
        let colors = theme::palette(ui.ctx());
        ui.label(RichText::new("Bauraumprüfung für diesen Entwurf").strong());
        let Some(usable) = settings.usable_size() else {
            ui.label(
                RichText::new("Wähle einen Drucker oder gib deine Druckbettgröße ein.")
                    .small()
                    .color(colors.muted),
            );
            return;
        };
        ui.label(settings.name());
        ui.label(
            RichText::new(format!("Nutzbar: {} mm", dimensions(usable)))
                .small()
                .color(colors.muted),
        );
        if let Some(grid) = settings.grid_capacity() {
            ui.label(RichText::new(format!("Grundplatte: bis {} × {} Rasterfelder", grid[0], grid[1])).small().color(colors.muted))
                .on_hover_text("Rechteckige Fläche mit dem eingestellten Rand. Die App prüft anschließend auch die tatsächliche Höhe und jedes einzelne Druckteil.");
        }
        if !self.fresh() {
            ui.label(
                RichText::new("Bauraumprüfung wartet auf das Modell …")
                    .small()
                    .color(colors.muted),
            );
            return;
        }
        let mut too_large = 0;
        let mut rotated = 0;
        let mut unknown = false;
        for size in &self.printer_panel.dimensions {
            match size.map(|size| printer::fit(size, usable)) {
                Some(printer::Fit::Fits) => {}
                Some(printer::Fit::Rotate90) => rotated += 1,
                Some(printer::Fit::TooLarge) => too_large += 1,
                None => unknown = true,
            }
        }
        if too_large > 0 {
            ui.colored_label(
                colors.warning,
                if too_large == 1 {
                    "Ein Druckteil überschreitet den Bauraum.".to_owned()
                } else {
                    format!("{too_large} Druckteile überschreiten den Bauraum.")
                },
            );
        } else if unknown {
            ui.colored_label(colors.warning, "Druckmaße konnten nicht geprüft werden.");
        } else if rotated > 0 {
            ui.colored_label(colors.warning, "Passt mit 90°-Drehung auf dem Druckbett.");
        } else {
            ui.colored_label(
                colors.success,
                "Die Druckteile passen einzeln in den Bauraum.",
            );
        }
        if matches!(self.cfg.product, Product::Baseplate) && too_large > 0 {
            ui.label(RichText::new("Raster verkleinern oder mehrere kleinere Platten mit Clip-Aufnahmen erstellen.").small().color(colors.muted));
        }
    }

    pub fn printer_export_info(&mut self, ui: &mut egui::Ui) {
        let colors = theme::palette(ui.ctx());
        ui.separator();
        let Some(usable) = self.printer_panel.settings.usable_size() else {
            ui.label(
                RichText::new("Kein Drucker ausgewählt – Bauraum noch nicht geprüft.")
                    .small()
                    .color(colors.muted),
            );
            if ui.button("Drucker auswählen …").clicked() {
                self.printer_panel.open();
            }
            ui.separator();
            return;
        };
        ui.label(RichText::new(self.printer_panel.settings.name()).strong());
        ui.label(
            RichText::new(format!("Nutzbarer Bauraum: {} mm", dimensions(usable)))
                .small()
                .color(colors.muted),
        );
        if self.fresh() {
            for (index, part) in self.parts.iter().enumerate() {
                let selected = index == self.active;
                if let Some(Some(size)) = self.printer_panel.dimensions.get(index) {
                    let (label, color) = fit_text(printer::fit(*size, usable), colors);
                    let title = format!("{} · {} mm", part.name, dimensions(*size));
                    ui.label(if selected {
                        RichText::new(title).strong()
                    } else {
                        RichText::new(title)
                    });
                    ui.label(RichText::new(label).small().color(color));
                } else {
                    ui.colored_label(
                        colors.warning,
                        format!("{}: Druckmaße nicht verfügbar", part.name),
                    );
                }
            }
        } else {
            ui.label("Bauraumprüfung wartet auf die neue Geometrie …");
        }
        ui.label(RichText::new("Jedes Teil wird einzeln geprüft. Nötige Drehungen oder Teilungen im Slicer vornehmen.").small().color(colors.muted));
        ui.separator();
    }

    pub fn printer_dialog(&mut self, ctx: &egui::Context) {
        let colors = theme::palette(ctx);
        let Some(mut draft) = self.printer_panel.draft.take() else {
            return;
        };
        let mut open = true;
        let mut cancel = false;
        let mut apply = false;
        let mut saved = false;
        egui::Window::new("Drucker & Bauraum")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(510.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_max_width(510.0);
                egui::ScrollArea::vertical().max_height((ctx.content_rect().height() - 140.0).max(200.0)).show(ui, |ui| {
                    ui.label("Drucker auswählen oder eigene Bauraummaße eingeben.");
                    ui.add_space(8.0);
                    let mut mode = match draft.profile_id.as_deref() { None => 0, Some("custom") => 2, _ => 1 };
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut mode, 0, "Kein Drucker");
                        ui.selectable_value(&mut mode, 1, "Druckerprofil");
                        ui.selectable_value(&mut mode, 2, "Benutzerdefiniert");
                    });
                    match mode {
                        0 => draft.profile_id = None,
                        1 => {
                            if draft.profile_id.as_deref().is_none_or(|id| id == "custom") {
                                draft.profile_id = PROFILES.first().map(|profile| profile.id.to_owned());
                            }
                            profile_controls(ui, &mut draft);
                        },
                        _ => {
                            if draft.profile_id.as_deref() != Some("custom") {
                                if let Some(size) = draft.volume() { draft.custom_size = size; }
                                draft.profile_id = Some("custom".into());
                            }
                            ui.add_space(8.0);
                            egui::Grid::new("custom-printer-size").spacing([18.0, 8.0]).show(ui, |ui| {
                                for (label, size) in ["Breite X", "Tiefe Y", "Druckhöhe Z"].into_iter().zip(&mut draft.custom_size) {
                                    ui.label(label);
                                    ui.add(mm_edit(size, 1.0..=2000.0));
                                    ui.end_row();
                                }
                            });
                        },
                    }
                    if draft.profile_id.is_some() {
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label("Freier Rand je Seite");
                            ui.add(mm_edit(&mut draft.margin, 0.0..=50.0));
                        });
                        ui.label(RichText::new("Abstand zum Bettrand, z. B. für einen Brim.").small().color(colors.muted));
                        if let Some(size) = draft.usable_size() {
                            ui.add_space(8.0);
                            ui.label(RichText::new(format!("Nutzbarer Bauraum: {} mm", dimensions(size))).strong());
                            if let Some(grid) = draft.grid_capacity() {
                                ui.label(format!("Grundplatte: maximal {} × {} Rasterfelder à 42 mm", grid[0], grid[1]));
                                if grid.iter().any(|n| *n > 8) {
                                    ui.label(RichText::new("Die App unterstützt derzeit bis zu 8 × 8 Felder pro Modell.").small().color(colors.muted));
                                }
                            }
                        }
                        ui.add_space(8.0);
                        ui.label(RichText::new("Sperrflächen und Platz für Stützen im Slicer prüfen.").small().color(colors.muted))
                            .on_hover_text("Verglichen werden die rechteckigen Abmessungen jedes einzelnen Druckteils. Wischbereiche, Spültürme und druckerspezifische Sperrflächen sind darin nicht enthalten.");
                    }
                    let validation = draft.validate();
                    if let Err(error) = &validation { ui.colored_label(colors.warning, error.to_string()); }
                    if let Some(error) = &self.printer_panel.error { ui.colored_label(colors.error, error); }
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        apply = ui.add_enabled(validation.is_ok(), egui::Button::new(RichText::new("Übernehmen").color(colors.on_primary)).fill(colors.primary)).clicked();
                        cancel = ui.button("Abbrechen").clicked();
                    });
                    if apply {
                        match draft.save() {
                            Ok(()) => {
                                self.printer_panel.settings = draft.clone();
                                self.printer_panel.error = None;
                                self.printer_bed_view = true;
                                self.show_build_plate = true;
                                self.view_plan = false;
                                self.reset_view();
                                self.message = "Druckerauswahl gespeichert.".into();
                                saved = true;
                            }
                            Err(error) => {
                                self.printer_panel.error = Some(format!(
                                    "Druckerauswahl konnte nicht gespeichert werden: {error:#}"
                                ));
                            }
                        }
                    }
                    if !self.show_home && !saved && validation.is_ok() {
                        ui.add_space(12.0);
                        ui.separator();
                        self.printer_sidebar(ui, &draft);
                    }
                });
            });
        if !saved && open && !cancel && !ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.printer_panel.draft = Some(draft);
        }
    }
}

fn profile_controls(ui: &mut egui::Ui, draft: &mut printer::Settings) {
    let colors = theme::palette(ui.ctx());
    let current = PROFILES
        .iter()
        .find(|profile| Some(profile.id) == draft.profile_id.as_deref());
    let mut brand = current.map_or("Bambu Lab", |profile| profile.brand);
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Hersteller");
        egui::ComboBox::from_id_salt("printer-brand")
            .width(300.0)
            .selected_text(brand)
            .show_ui(ui, |ui| {
                let mut brands = Vec::new();
                for profile in PROFILES {
                    if !brands.contains(&profile.brand) {
                        brands.push(profile.brand);
                    }
                }
                for name in brands {
                    ui.selectable_value(&mut brand, name, name);
                }
            });
    });
    if current.is_none_or(|profile| profile.brand != brand) {
        draft.profile_id = PROFILES
            .iter()
            .find(|profile| profile.brand == brand)
            .map(|profile| profile.id.to_owned());
    }
    ui.horizontal(|ui| {
        ui.label("Modell");
        egui::ComboBox::from_id_salt("printer-model")
            .width(340.0)
            .wrap()
            .selected_text(
                PROFILES
                    .iter()
                    .find(|profile| Some(profile.id) == draft.profile_id.as_deref())
                    .map_or("Modell wählen", |profile| profile.name),
            )
            .show_ui(ui, |ui| {
                for profile in PROFILES.iter().filter(|profile| profile.brand == brand) {
                    ui.selectable_value(
                        &mut draft.profile_id,
                        Some(profile.id.to_owned()),
                        profile.name,
                    );
                }
            });
    });
    if let Some(profile) = PROFILES
        .iter()
        .find(|profile| Some(profile.id) == draft.profile_id.as_deref())
    {
        ui.label(format!("Bauraum: {} mm", dimensions(profile.size)));
        ui.collapsing("Profilhinweise", |ui| {
            ui.label(RichText::new(profile.note).small().color(colors.muted));
            ui.hyperlink_to("Herstellerangaben öffnen", profile.source);
        });
        if ui
            .small_button("Maße als eigene Vorgabe bearbeiten")
            .clicked()
        {
            draft.custom_size = profile.size;
            draft.profile_id = Some("custom".into());
        }
    }
}

fn dimensions(size: [f64; 3]) -> String {
    size.map(|value| format!("{value:.1}").replace('.', ","))
        .join(" × ")
}

fn mm_edit(value: &mut f64, range: std::ops::RangeInclusive<f64>) -> egui::DragValue<'_> {
    egui::DragValue::new(value)
        .range(range)
        .speed(1.0)
        .suffix(" mm")
        .max_decimals(1)
        .custom_formatter(|value, _| format!("{value:.1}").replace('.', ","))
        .custom_parser(|text| text.trim().replace(',', ".").parse().ok())
}

fn fit_text(fit: printer::Fit, colors: theme::Palette) -> (&'static str, Color32) {
    match fit {
        printer::Fit::Fits => ("Passt in Drucklage in den Bauraum", colors.success),
        printer::Fit::Rotate90 => ("Passt nach 90°-Drehung auf dem Druckbett", colors.warning),
        printer::Fit::TooLarge => ("Zu groß für diesen Bauraum", colors.warning),
    }
}
