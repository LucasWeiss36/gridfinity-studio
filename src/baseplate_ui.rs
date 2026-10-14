//! Baseplate mounting presets; existing uncommon combinations remain untouched.
use crate::{design::Design, model};
use eframe::egui::{self, RichText};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Preset {
    None,
    CornerMagnets,
    CellMagnets,
    OuterScrews,
    CellMagnetsAndScrews,
    CornerMagnetsAndScrews,
}

impl Preset {
    const ALL: [Self; 6] = [
        Self::None,
        Self::CornerMagnets,
        Self::CellMagnets,
        Self::OuterScrews,
        Self::CellMagnetsAndScrews,
        Self::CornerMagnetsAndScrews,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::None => "Ohne Befestigung",
            Self::CornerMagnets => "4 äußere Magnete",
            Self::CellMagnets => "Magnete je Rasterfeld",
            Self::OuterScrews => "4 äußere Schrauben",
            Self::CellMagnetsAndScrews => "Magnete je Feld + 4 Schrauben",
            Self::CornerMagnetsAndScrews => "4 Magnete + 4 Schrauben",
        }
    }

    fn current(cfg: &Design) -> Option<Self> {
        match (cfg.magnets, cfg.screws) {
            (false, false) => Some(Self::None),
            (true, false) if cfg.magnet_corners_only => Some(Self::CornerMagnets),
            (true, false) => Some(Self::CellMagnets),
            (false, true) if cfg.screw_corners_only => Some(Self::OuterScrews),
            (true, true) if cfg.screw_corners_only && cfg.magnet_corners_only => {
                Some(Self::CornerMagnetsAndScrews)
            }
            (true, true) if cfg.screw_corners_only => Some(Self::CellMagnetsAndScrews),
            _ => None,
        }
    }

    fn apply(self, cfg: &mut Design) {
        let (magnets, magnet_corners, screws) = match self {
            Self::None => (false, false, false),
            Self::CornerMagnets => (true, true, false),
            Self::CellMagnets => (true, false, false),
            Self::OuterScrews => (false, false, true),
            Self::CellMagnetsAndScrews => (true, false, true),
            Self::CornerMagnetsAndScrews => (true, true, true),
        };
        cfg.magnets = magnets;
        cfg.magnet_corners_only = magnet_corners;
        cfg.screws = screws;
        cfg.screw_corners_only = screws;
    }
}

fn counts(cfg: &Design) -> (u32, u32) {
    let per_cell = cfg.grid_x.saturating_mul(cfg.grid_y).saturating_mul(4);
    let count = |enabled: bool, corners: bool| {
        if !enabled {
            0
        } else if corners {
            4
        } else {
            per_cell
        }
    };
    (
        count(cfg.magnets, cfg.magnet_corners_only),
        count(cfg.screws, cfg.screw_corners_only),
    )
}

pub fn controls(ui: &mut egui::Ui, cfg: &mut Design) {
    ui.scope(|ui| {
        ui.set_max_width(ui.available_width().min(244.0));
        ui.checkbox(&mut cfg.baseplate_open, "Offene Rasterfelder (ohne Boden)");
        ui.add_space(5.0);
        ui.label("Befestigung");
        let current = Preset::current(cfg);
        egui::ComboBox::from_id_salt("baseplate-mounting-preset")
            .width(ui.available_width())
            .wrap()
            .selected_text(current.map_or("Eigene Kombination", Preset::label))
            .show_ui(ui, |ui| {
                ui.set_max_width(230.0);
                for preset in Preset::ALL {
                    if ui
                        .add(
                            egui::Button::new(preset.label())
                                .selected(current == Some(preset))
                                .wrap(),
                        )
                        .clicked()
                    {
                        preset.apply(cfg);
                        ui.close();
                    }
                }
            });

        let (magnets, screws) = counts(cfg);
        let count_text = match (magnets, screws) {
            (0, 0) => None,
            (0, screws) => Some(format!("{screws} Schraublöcher")),
            (magnets, 0) => Some(format!("{magnets} Magnetaufnahmen")),
            (magnets, screws) => Some(format!("{magnets} Magnetaufnahmen · {screws} Schraublöcher")),
        };
        if let Some(text) = count_text {
            ui.label(RichText::new(text).strong());
        }
        if cfg.magnets {
            note(
                ui,
                if cfg.magnet_corners_only {
                    "Magnete an den äußeren Ecken."
                } else {
                    "Vier Magnete je Rasterfeld."
                },
            );
            compact_slider(ui, "Magnet-Ø", &mut cfg.magnet_diameter, 5.0..=8.0);
            compact_slider(ui, "Magnettiefe", &mut cfg.magnet_depth, 1.0..=3.0);
        }
        if cfg.screws {
            note(ui, "Schraublöcher: Ø 3,2 mm, Senkung Ø 6,4 mm.");
        }
        if cfg.magnets && cfg.screws {
            note(ui, "Schraublöcher versetzt neben den Magneten.");
        }
        if cfg.magnets || cfg.screws {
            let original_floor = if cfg.baseplate_open { 0.0 } else { 1.5 };
            let extra = model::baseplate_seat_height(cfg) - original_floor;
            let extra = format!("{extra:.2}").replace('.', ",");
            note(
                ui,
                &format!("Zusätzliche Höhe: {extra} mm."),
            );
        }
        ui.add_space(5.0);
        ui.checkbox(&mut cfg.baseplate_clips, "Clip-Aufnahmen");
        if cfg.baseplate_clips {
            note(ui, "Clips separat erstellen und drucken. Verbundene Platten brauchen dieselbe Auflagehöhe.");
        }
    });
}

fn note(ui: &mut egui::Ui, text: &str) {
    let colors = crate::theme::palette(ui.ctx());
    ui.add(egui::Label::new(RichText::new(text).small().color(colors.muted)).wrap());
}

fn compact_slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
) {
    ui.label(label);
    ui.scope(|ui| {
        // Leave room for the directly editable numeric value and its unit.
        ui.spacing_mut().slider_width = (ui.available_width() - 86.0).max(50.0);
        ui.add(
            egui::Slider::new(value, range)
                .clamping(egui::SliderClamping::Edits)
                .suffix(" mm")
                .max_decimals(2)
                .step_by(0.1),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_select_only_the_intended_mounts_and_keep_other_design_data() {
        let original = Design {
            grid_x: 3,
            grid_y: 2,
            baseplate_clips: true,
            magnet_diameter: 7.2,
            magnet_depth: 2.8,
            ..Design::default()
        };
        let expected = [(0, 0), (4, 0), (24, 0), (0, 4), (24, 4), (4, 4)];
        for (preset, expected) in Preset::ALL.into_iter().zip(expected) {
            let mut cfg = original.clone();
            preset.apply(&mut cfg);
            assert_eq!(Preset::current(&cfg), Some(preset));
            assert_eq!(counts(&cfg), expected);
            assert_eq!(cfg.magnet_diameter, original.magnet_diameter);
            assert_eq!(cfg.magnet_depth, original.magnet_depth);
            assert_eq!(cfg.baseplate_open, original.baseplate_open);
            assert_eq!(cfg.baseplate_clips, original.baseplate_clips);
            assert_eq!(cfg.grid_x, original.grid_x);
            assert_eq!(cfg.grid_y, original.grid_y);
            assert_eq!(cfg.title, original.title);
        }
    }

    #[test]
    fn existing_dense_combinations_are_recognized_without_changing_them() {
        for magnets in [false, true] {
            for magnet_corners_only in [false, true] {
                let cfg = Design {
                    grid_x: 3,
                    grid_y: 2,
                    magnets,
                    magnet_corners_only,
                    screws: true,
                    screw_corners_only: false,
                    ..Design::default()
                };
                let before = serde_json::to_vec(&cfg).unwrap();
                assert_eq!(Preset::current(&cfg), None);
                assert_eq!(counts(&cfg).1, 24);
                assert_eq!(serde_json::to_vec(&cfg).unwrap(), before);
            }
        }
    }

    #[test]
    fn drawing_controls_does_not_round_existing_magnet_dimensions() {
        let mut cfg = Design {
            magnets: true,
            screws: true,
            magnet_diameter: 6.25,
            magnet_depth: 2.35,
            ..Design::default()
        };
        let before = serde_json::to_vec(&cfg).unwrap();
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| controls(ui, &mut cfg));
        });
        assert_eq!(serde_json::to_vec(&cfg).unwrap(), before);
    }

    #[test]
    fn a_single_cell_has_four_mounts_for_both_outer_and_per_cell_modes() {
        for preset in Preset::ALL {
            let mut cfg = Design {
                grid_x: 1,
                grid_y: 1,
                ..Design::default()
            };
            preset.apply(&mut cfg);
            let (magnets, screws) = counts(&cfg);
            assert_eq!(magnets, if cfg.magnets { 4 } else { 0 });
            assert_eq!(screws, if cfg.screws { 4 } else { 0 });
        }
    }

    #[test]
    fn selecting_no_mounts_disables_both_features_from_any_existing_state() {
        let mut cfg = Design {
            magnets: true,
            screws: true,
            magnet_corners_only: true,
            screw_corners_only: false,
            ..Design::default()
        };
        Preset::None.apply(&mut cfg);
        assert!(!cfg.magnets && !cfg.screws);
        assert_eq!(counts(&cfg), (0, 0));
        // Disabled feature flags from older files do not create a false custom state.
        cfg.magnet_corners_only = true;
        cfg.screw_corners_only = true;
        assert_eq!(Preset::current(&cfg), Some(Preset::None));
    }
}
