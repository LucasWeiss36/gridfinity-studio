use crate::{PlanTool, Product, Selection, Studio, measurement, theme, wall_editor};
use eframe::egui::{self, Pos2, Stroke, Vec2};
use wall_editor::Dimension;

pub struct DimensionEdit {
    index: usize,
    kind: Dimension,
    design_key: String,
    text: String,
    error: Option<String>,
    focus: bool,
}

pub struct PlanMeasure {
    index: usize,
    kind: Dimension,
    a: Pos2,
    b: Pos2,
    value: f64,
    name: &'static str,
}

impl PlanMeasure {
    fn label(&self) -> String {
        format!("{} mm", editable_number(self.value))
    }
    fn centre(&self) -> Pos2 {
        self.a + (self.b - self.a) * 0.5
    }
    pub fn label_rect(&self, painter: &egui::Painter) -> egui::Rect {
        let text = painter.layout_no_wrap(
            self.label(),
            egui::FontId::proportional(12.0),
            theme::palette(painter.ctx()).accent,
        );
        egui::Rect::from_center_size(self.centre(), text.size() + Vec2::splat(10.0))
    }
}

impl Studio {
    pub fn validate_dimension_edit(&mut self) {
        if self.dimension_edit.as_ref().is_some_and(|edit| {
            !self.view_plan
                || self.cfg.product != Product::Bin
                || !self.show_measurements
                || !matches!(self.tool, PlanTool::Select)
                || !matches!(self.selection, Some(Selection::Wall(i)) if i == edit.index)
                || edit.design_key != self.key()
        }) {
            self.dimension_edit = None;
        }
    }

    pub fn plan_measures(
        &self,
        to_screen: &impl Fn([f64; 2]) -> Pos2,
        max: [f64; 2],
    ) -> Vec<PlanMeasure> {
        let mut result = Vec::new();
        if self.cfg.product != Product::Bin
            || !self.show_measurements
            || !matches!(self.tool, PlanTool::Select)
        {
            return result;
        }
        let Some(Selection::Wall(index)) = self.selection else {
            return result;
        };
        let Some(wall) = self.cfg.walls.get(index) else {
            return result;
        };
        let a = to_screen([wall.x1, wall.y1]);
        let b = to_screen([wall.x2, wall.y2]);
        let offset = (b - a).rot90().normalized() * 24.0;
        result.push(PlanMeasure {
            index,
            kind: Dimension::Length,
            a: a + offset,
            b: b + offset,
            value: (wall.x2 - wall.x1).hypot(wall.y2 - wall.y1),
            name: "Wandlänge",
        });
        if let Some(spans) = wall_editor::spans(&self.cfg.walls, index, max) {
            for (kind, low, high, value, name) in [
                (
                    Dimension::LowGap,
                    spans.low,
                    spans.wall_low,
                    spans.widths()[0],
                    if spans.axis == 0 {
                        "Fachmaß links"
                    } else {
                        "Fachmaß unten"
                    },
                ),
                (
                    Dimension::HighGap,
                    spans.wall_high,
                    spans.high,
                    spans.widths()[1],
                    if spans.axis == 0 {
                        "Fachmaß rechts"
                    } else {
                        "Fachmaß oben"
                    },
                ),
            ] {
                let mut a = spans.point;
                let mut b = a;
                a[spans.axis] = low;
                b[spans.axis] = high;
                result.push(PlanMeasure {
                    index,
                    kind,
                    a: to_screen(a),
                    b: to_screen(b),
                    value,
                    name,
                });
            }
        }
        result
    }

    pub fn edit_plan_measures(
        &mut self,
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        max: [f64; 2],
        measures: &[PlanMeasure],
    ) {
        let palette = theme::palette(painter.ctx());
        for measure in measures {
            measurement(painter, measure.a, measure.b, measure.label());
            let rect = measure.label_rect(painter);
            let active = self
                .dimension_edit
                .as_ref()
                .is_some_and(|edit| edit.index == measure.index && edit.kind == measure.kind);
            if active {
                continue;
            }
            let response = ui.interact(
                rect,
                ui.id()
                    .with(("plan-dimension", measure.index, measure.kind as u8)),
                egui::Sense::click(),
            );
            response.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    true,
                    format!("{}: {} ändern", measure.name, measure.label()),
                )
            });
            if response.hovered() {
                painter.rect_stroke(
                    rect,
                    3,
                    Stroke::new(1.0_f32, palette.accent),
                    egui::StrokeKind::Outside,
                );
            }
            if response.clicked() {
                self.dimension_edit = Some(DimensionEdit {
                    index: measure.index,
                    kind: measure.kind,
                    design_key: self.key(),
                    text: editable_number(measure.value),
                    error: None,
                    focus: true,
                });
                self.wall_drag = None;
                self.wall_start = None;
            }
            response.on_hover_cursor(egui::CursorIcon::Text).on_hover_text(format!(
                "{} ändern · Klicken und Millimeter eingeben\nEnter: übernehmen · Escape: abbrechen", measure.name));
        }

        let Some(mut edit) = self.dimension_edit.take() else {
            return;
        };
        let Some(measure) = measures
            .iter()
            .find(|m| m.index == edit.index && m.kind == edit.kind)
        else {
            return;
        };
        let id = ui.id().with("inline-dimension-input");
        let available = ui.clip_rect().shrink(8.0);
        let centre = Pos2::new(
            measure
                .centre()
                .x
                .clamp(available.left() + 58.0, available.right() - 58.0),
            measure
                .centre()
                .y
                .clamp(available.top() + 14.0, available.bottom() - 42.0),
        );
        let rect = egui::Rect::from_center_size(centre, Vec2::new(116.0, 26.0));
        let (enter, escape, outside) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
                i.pointer.any_pressed()
                    && i.pointer
                        .interact_pos()
                        .is_some_and(|p| !rect.expand(4.0).contains(p)),
            )
        });
        let colour = if edit.error.is_some() {
            palette.error
        } else {
            palette.accent
        };
        painter.rect_filled(rect.expand(3.0), 4, palette.paper);
        painter.rect_stroke(
            rect.expand(3.0),
            4,
            Stroke::new(1.5_f32, colour),
            egui::StrokeKind::Outside,
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("dimension-overlay")
                .max_rect(rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        let mut output = egui::TextEdit::singleline(&mut edit.text)
            .id(id)
            .desired_width(87.0)
            .font(egui::FontId::proportional(13.0))
            .char_limit(24)
            .show(&mut child);
        child.label("mm");
        if edit.focus {
            output.response.request_focus();
            output
                .state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(edit.text.chars().count()),
                )));
            output.state.store(ui.ctx(), id);
            edit.focus = false;
        } else if escape || outside || (output.response.lost_focus() && !enter) {
            output.response.surrender_focus();
            ui.ctx().request_repaint();
            return;
        } else if enter {
            let value = if edit.text == editable_number(measure.value) {
                Ok(measure.value)
            } else {
                parse_number(&edit.text)
            };
            let result = value.and_then(|value| {
                wall_editor::set_dimension(&self.cfg.walls, edit.index, max, edit.kind, value)
            });
            match result {
                Ok(wall) => {
                    if wall != self.cfg.walls[edit.index] {
                        self.wall_editor.checkpoint(&self.cfg.walls);
                        self.cfg.walls[edit.index] = wall;
                        self.wall_editor.mode = wall_editor::Mode::Free;
                    }
                    output.response.surrender_focus();
                    ui.ctx().request_repaint();
                    return;
                }
                Err(error) => {
                    edit.error = Some(error);
                    output.response.request_focus();
                    ui.ctx().request_repaint();
                }
            }
        } else if output.response.changed() {
            edit.error = None;
        }
        let hint = edit
            .error
            .as_deref()
            .unwrap_or("Enter: übernehmen · Esc: abbrechen");
        let text = painter.layout(
            hint.into(),
            egui::FontId::proportional(11.0),
            if edit.error.is_some() {
                colour
            } else {
                palette.muted
            },
            250.0,
        );
        let x = (centre.x - text.size().x / 2.0)
            .clamp(available.left(), available.right() - text.size().x);
        let pos = Pos2::new(x, rect.bottom() + 8.0);
        painter.rect_filled(
            egui::Rect::from_min_size(pos, text.size()).expand(3.0),
            3,
            palette.paper,
        );
        painter.galley(pos, text, palette.muted);
        self.dimension_edit = Some(edit);
    }
}

pub(crate) fn editable_number(value: f64) -> String {
    format!("{value:.3}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .replace('.', ",")
}
pub(crate) fn parse_number(text: &str) -> Result<f64, String> {
    let text = text.trim().strip_suffix("mm").unwrap_or(text.trim()).trim();
    text.replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| "Bitte eine Zahl eingeben, z. B. 29,1.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_dimensions_accept_decimal_comma_and_format_readably() {
        assert_eq!(parse_number(" 29,1 mm ").unwrap(), 29.1);
        assert_eq!(parse_number("51.25").unwrap(), 51.25);
        assert_eq!(editable_number(80.97999999999999), "80,98");
        assert_eq!(editable_number(29.09), "29,09");
        assert_eq!(editable_number(40.0), "40");
        for invalid in ["", "29,1,2", "NaN", "inf", "1e999", "text"] {
            assert!(parse_number(invalid).is_err());
        }
    }
}
