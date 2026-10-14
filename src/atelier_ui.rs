//! Design 2: an independent, light workshop with a catalogue and a large canvas.
use crate::{Studio, design::Product, theme};
use eframe::egui::{self, Color32, FontId, Rect, RichText, Sense, Stroke, Vec2};

const CATALOGUE: [(Product, &str, &str); 6] = [
    (
        Product::Bin,
        "Kasten",
        "Fächer, Trennwände und Beschriftung",
    ),
    (
        Product::Baseplate,
        "Grundplatte",
        "Die Basis für dein Ordnungssystem",
    ),
    (
        Product::Insert,
        "Werkzeugeinsatz",
        "Aussparungen nach deinen Maßen",
    ),
    (
        Product::DeskBox,
        "Schubladenbox",
        "Stauraum für deinen Schreibtisch",
    ),
    (
        Product::Clip,
        "Verbindungsclip",
        "Grundplatten miteinander verbinden",
    ),
    (
        Product::LabelInsert,
        "Einschubschild",
        "Ein eigenes Schild für jedes Fach",
    ),
];

impl Studio {
    pub fn atelier_rail(&mut self, ctx: &egui::Context) {
        let p = theme::palette(ctx);
        egui::SidePanel::left("atelier-navigation")
            .exact_width(90.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(p.rail).inner_margin(8))
            .show(ctx, |ui| {
                ui.add_space(16.0);
                for (index, label, selected) in [
                    (0, "Start", self.show_home),
                    (1, "Entwurf", !self.show_home),
                    (2, "Projekte", false),
                    (3, "Drucker", false),
                    (4, "Hilfe", false),
                ] {
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(74.0, 76.0), Sense::click());
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
                    });
                    let colour = if selected {
                        Color32::from_rgb(219, 245, 237)
                    } else {
                        p.rail_text
                    };
                    if selected || response.hovered() {
                        ui.painter().rect_filled(rect.shrink(2.0), 9, p.rail_active);
                    }
                    let icon = Rect::from_center_size(
                        rect.center_top() + Vec2::new(0.0, 25.0),
                        Vec2::splat(24.0),
                    );
                    nav_icon(ui.painter(), icon, index, colour);
                    ui.painter().text(
                        rect.center_bottom() - Vec2::new(0.0, 15.0),
                        egui::Align2::CENTER_CENTER,
                        label,
                        FontId::proportional(11.0),
                        colour,
                    );
                    if response.on_hover_text(label).clicked() {
                        match index {
                            0 => self.show_home = true,
                            1 => self.show_home = false,
                            2 => self.show_library(false),
                            3 => self.printer_panel.open(),
                            _ => self.help_visible = true,
                        }
                    }
                    ui.add_space(6.0);
                }
                let mode_rect = Rect::from_min_size(
                    egui::pos2(ui.max_rect().left(), ui.max_rect().bottom() - 149.0),
                    Vec2::new(74.0, 141.0),
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(mode_rect), |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("DARSTELLUNG").size(8.0).color(p.rail_text));
                        for (mode, label) in [
                            (theme::Mode::System, "System"),
                            (theme::Mode::Light, "Hell"),
                            (theme::Mode::Dark, "Dunkel"),
                        ] {
                            let selected = self.appearance.mode == mode;
                            if ui
                                .add_sized(
                                    [70.0, 29.0],
                                    egui::Button::new(
                                        RichText::new(label).size(12.0).color(if selected {
                                            p.on_primary
                                        } else {
                                            p.rail_text
                                        }),
                                    )
                                    .fill(if selected { p.primary } else { p.rail })
                                    .stroke(Stroke::new(1.0_f32, p.rail_active))
                                    .corner_radius(6),
                                )
                                .on_hover_text(match mode {
                                    theme::Mode::System => {
                                        "Hellen oder dunklen Modus des Betriebssystems übernehmen"
                                    }
                                    theme::Mode::Light => "Hellmodus",
                                    theme::Mode::Dark => "Dunkelmodus",
                                })
                                .clicked()
                                && let Err(error) = self.appearance.switch(ctx, mode)
                            {
                                self.operation_error = Some(format!("{error:#}"));
                            }
                        }
                    });
                });
            });
    }

    pub fn atelier_home(&mut self, ctx: &egui::Context) {
        let p = theme::palette(ctx);
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.paper).inner_margin(28))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label(
                        RichText::new("DEINE WERKSTATT")
                            .size(11.0)
                            .color(p.accent)
                            .strong(),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Was möchtest du bauen?")
                            .size(32.0)
                            .strong()
                            .color(p.text),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(
                            "Wähle eine Form. Maße und Details legst du im nächsten Schritt fest.",
                        )
                        .color(p.muted),
                    );
                    ui.add_space(28.0);
                    let available = ui.available_width();
                    let list_width = (available * 0.34).clamp(280.0, 360.0);
                    let gap = 24.0;
                    let preview_width = (available - list_width - gap).max(330.0);
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        ui.allocate_ui_with_layout(
                            Vec2::new(list_width, 475.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.set_width(list_width);
                                for (i, &(product, name, subtitle)) in CATALOGUE.iter().enumerate()
                                {
                                    let selected = self.home_product == product;
                                    let (rect, response) = ui.allocate_exact_size(
                                        Vec2::new(list_width, 64.0),
                                        Sense::click(),
                                    );
                                    response.widget_info(|| {
                                        egui::WidgetInfo::labeled(
                                            egui::WidgetType::Button,
                                            ui.is_enabled(),
                                            name,
                                        )
                                    });
                                    let fill = if selected {
                                        p.selection
                                    } else if response.hovered() {
                                        p.hover
                                    } else {
                                        p.panel
                                    };
                                    ui.painter().rect_filled(rect, 8, fill);
                                    if selected {
                                        ui.painter().rect_filled(
                                            Rect::from_min_size(
                                                rect.min + Vec2::new(0.0, 12.0),
                                                Vec2::new(3.0, 40.0),
                                            ),
                                            2,
                                            p.accent,
                                        );
                                    }
                                    ui.painter().text(
                                        rect.left_center() + Vec2::new(20.0, 0.0),
                                        egui::Align2::CENTER_CENTER,
                                        format!("{:02}", i + 1),
                                        FontId::monospace(12.0),
                                        p.accent,
                                    );
                                    ui.painter().text(
                                        rect.min + Vec2::new(43.0, 15.0),
                                        egui::Align2::LEFT_TOP,
                                        name,
                                        FontId::proportional(16.0),
                                        p.text,
                                    );
                                    ui.painter().text(
                                        rect.min + Vec2::new(43.0, 38.0),
                                        egui::Align2::LEFT_TOP,
                                        subtitle,
                                        FontId::proportional(10.5),
                                        p.muted,
                                    );
                                    if response.on_hover_text(subtitle).clicked() {
                                        self.home_product = product;
                                    }
                                    ui.add_space(5.0);
                                }
                                ui.add_space(15.0);
                                ui.horizontal(|ui| {
                                    if ui.button("Projektdatei öffnen …").clicked() {
                                        self.open();
                                    }
                                    self.example_menu(ui);
                                });
                            },
                        );
                        egui::Frame::new()
                            .fill(p.canvas)
                            .corner_radius(16)
                            .inner_margin(24)
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    let content_width = preview_width - 48.0;
                                    ui.set_width(content_width);
                                    let (_, name, subtitle) = CATALOGUE
                                        .iter()
                                        .find(|(p, _, _)| *p == self.home_product)
                                        .unwrap();
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new("MODELLVORSCHAU")
                                                .size(10.0)
                                                .color(p.muted)
                                                .strong(),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    RichText::new("42 mm Raster")
                                                        .size(11.0)
                                                        .color(p.accent),
                                                );
                                            },
                                        );
                                    });
                                    let (preview_rect, _) = ui.allocate_exact_size(
                                        Vec2::new(
                                            content_width,
                                            (ctx.content_rect().height() - 630.0)
                                                .clamp(175.0, 268.0),
                                        ),
                                        Sense::hover(),
                                    );
                                    self.thumbnails.show(ui, preview_rect, self.home_product);
                                    ui.label(
                                        RichText::new(*name).size(29.0).strong().color(p.text),
                                    );
                                    ui.label(RichText::new(*subtitle).color(p.muted));
                                    ui.add_space(18.0);
                                    if ui
                                        .add_sized(
                                            [content_width, 44.0],
                                            egui::Button::new(
                                                RichText::new(format!("{name} erstellen   →"))
                                                    .color(p.on_primary)
                                                    .strong(),
                                            )
                                            .fill(p.primary)
                                            .corner_radius(8),
                                        )
                                        .clicked()
                                    {
                                        self.new_reference_model(self.home_product);
                                    }
                                });
                            });
                    });
                });
            });
    }
}

fn nav_icon(painter: &egui::Painter, rect: Rect, index: usize, colour: Color32) {
    let p = theme::palette(painter.ctx());
    let stroke = Stroke::new(1.6_f32, colour);
    let c = rect.center();
    match index {
        0 => {
            for x in [0.0, 13.0] {
                for y in [0.0, 13.0] {
                    painter.rect_stroke(
                        Rect::from_min_size(rect.min + Vec2::new(x, y), Vec2::splat(9.0)),
                        2,
                        stroke,
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }
        1 => {
            painter.rect_stroke(rect.shrink(2.0), 3, stroke, egui::StrokeKind::Inside);
            painter.line_segment(
                [
                    rect.left_center() + Vec2::new(4.0, 0.0),
                    rect.right_center() - Vec2::new(4.0, 0.0),
                ],
                stroke,
            );
            painter.line_segment([c + Vec2::new(0.0, -8.0), c + Vec2::new(0.0, 8.0)], stroke);
        }
        2 => {
            painter.rect_stroke(
                Rect::from_min_max(rect.min + Vec2::new(0.0, 6.0), rect.max),
                3,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.line_segment(
                [
                    rect.min + Vec2::new(1.0, 6.0),
                    rect.min + Vec2::new(1.0, 2.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    rect.min + Vec2::new(1.0, 2.0),
                    rect.min + Vec2::new(11.0, 2.0),
                ],
                stroke,
            );
        }
        3 => {
            painter.rect_stroke(
                Rect::from_center_size(c, Vec2::new(24.0, 12.0)),
                3,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.rect_stroke(
                Rect::from_center_size(c - Vec2::new(0.0, 7.0), Vec2::new(14.0, 9.0)),
                1,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.rect_filled(
                Rect::from_center_size(c + Vec2::new(0.0, 7.0), Vec2::new(14.0, 9.0)),
                1,
                p.rail,
            );
            painter.rect_stroke(
                Rect::from_center_size(c + Vec2::new(0.0, 7.0), Vec2::new(14.0, 9.0)),
                1,
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        _ => {
            painter.circle_stroke(c, 11.0, stroke);
            painter.text(
                c,
                egui::Align2::CENTER_CENTER,
                "?",
                FontId::proportional(17.0),
                colour,
            );
        }
    }
}
