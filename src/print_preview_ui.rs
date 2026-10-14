use crate::{Studio, print_preview::Preview, printer::Fit, theme};
use eframe::egui::{self, Color32, Pos2, RichText, Vec2};

fn mm(value: f64) -> String {
    format!("{value:.1}").replace('.', ",")
}

impl Studio {
    pub fn print_preview_overlay(
        &self,
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        rect: egui::Rect,
        preview: &Preview,
        middle: [f32; 3],
        scale: f32,
    ) {
        let settings = &self.printer_panel.settings;
        let Some([x, y, z]) = settings.volume() else {
            return;
        };
        let palette = theme::palette(ui.ctx());
        let fresh = self.fresh();
        let (status, colour) = if !fresh {
            ("Vorschau wird aktualisiert …".to_owned(), palette.muted)
        } else {
            match preview.fit {
                Fit::Fits => (
                    "Dieses Teil passt in den Bauraum.".to_owned(),
                    palette.success,
                ),
                Fit::Rotate90 => (
                    "Passt um 90° gedreht – diese Drehung im Slicer übernehmen.".to_owned(),
                    palette.warning,
                ),
                Fit::TooLarge if preview.dimensions[2] > z + 0.0001 => (
                    format!(
                        "Zu hoch: {} mm · maximal {} mm",
                        mm(preview.dimensions[2]),
                        mm(z)
                    ),
                    palette.error,
                ),
                Fit::TooLarge => (
                    "Das Teil ragt über die nutzbare Druckfläche hinaus.".to_owned(),
                    palette.error,
                ),
            }
        };
        ui.scope_builder(
            egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                rect.min + Vec2::new(18.0, 54.0),
                Vec2::new((rect.width() - 36.0).min(420.0), 110.0),
            )),
            |ui| {
                egui::Frame::new()
                    .fill(palette.paper)
                    .stroke(egui::Stroke::new(1.0_f32, palette.border))
                    .inner_margin(12)
                    .corner_radius(10)
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} · {} × {} mm",
                                settings.name(),
                                mm(x),
                                mm(y)
                            ))
                            .color(palette.accent)
                            .strong(),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Max. Höhe {} mm · Rand {} mm je Seite",
                                mm(z),
                                mm(settings.margin)
                            ))
                            .small()
                            .color(palette.muted),
                        );
                        ui.label(RichText::new(status).small().color(colour));
                        if self.parts.len() > 1 {
                            ui.label(
                                RichText::new("Weitere Druckteile in der Teileauswahl anzeigen.")
                                    .small()
                                    .color(palette.muted),
                            );
                        }
                    });
            },
        );
        if !self.show_build_plate {
            return;
        }
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let project = |point: [f32; 3]| {
            let [x, y, z] = std::array::from_fn(|axis| point[axis] - middle[axis]);
            rect.center()
                + Vec2::new(cy * x - sy * y, -(sp * sy * x + sp * cy * y + cp * z)) * scale
        };
        for (point, text, offset) in [
            (
                [0.0, -y as f32 / 2.0, 0.0],
                format!("X · {} mm", mm(x)),
                Vec2::new(0.0, 14.0),
            ),
            (
                [x as f32 / 2.0, 0.0, 0.0],
                format!("Y · {} mm", mm(y)),
                Vec2::new(40.0, 0.0),
            ),
        ] {
            let position: Pos2 = project(point) + offset;
            let galley =
                painter.layout_no_wrap(text, egui::FontId::proportional(12.0), palette.text);
            let label = egui::Rect::from_center_size(position, galley.size());
            // Hide an annotation outside the canvas rather than painting over controls.
            if rect.shrink2(Vec2::new(8.0, 65.0)).contains_rect(label) {
                painter.rect_filled(label.expand2(Vec2::new(5.0, 3.0)), 3, palette.paper);
                painter.galley(label.min, galley, Color32::WHITE);
            }
        }
        debug_assert_eq!(preview.rotated, preview.fit == Fit::Rotate90);
    }
}
