//! Design-2-only appearance. Model files and printer preferences are independent.
use anyhow::{Context, Result};
use eframe::egui::{self, Color32, Stroke};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Light,
    Dark,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Appearance {
    pub mode: Mode,
    #[serde(skip)]
    path: Option<PathBuf>,
}

impl Appearance {
    pub fn load() -> (Self, Option<String>) {
        let path = std::env::var_os("GRIDFINITY_DESIGN2_APPEARANCE_FILE")
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("LOCALAPPDATA")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| {
                        std::env::var_os("USERPROFILE")
                            .map(PathBuf::from)
                            .unwrap_or_default()
                            .join("AppData")
                            .join("Local")
                    })
                    .join("GridfinityStudioDesign2")
                    .join("appearance.json")
            });
        let (mut value, error) = match Self::read(&path) {
            Ok(value) => (value, None),
            Err(error) => (
                Self::default(),
                Some(format!(
                    "Darstellung konnte nicht geladen werden: {error:#}"
                )),
            ),
        };
        value.path = Some(path);
        (value, error)
    }

    fn read(path: &Path) -> Result<Self> {
        match fs::read(path) {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error.into()),
        }
    }

    pub fn switch(&mut self, ctx: &egui::Context, mode: Mode) -> Result<()> {
        self.mode = mode;
        apply(ctx, mode);
        if let Some(path) = &self.path {
            self.save_to(path)
                .context("Darstellung konnte nicht gespeichert werden")?;
        }
        Ok(())
    }

    fn save_to(&self, path: &Path) -> Result<()> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let staged = parent.join(format!(".appearance-{}-{stamp}.tmp", std::process::id()));
        let result = (|| -> Result<()> {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&staged)?;
            file.write_all(&serde_json::to_vec_pretty(self)?)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&staged, path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&staged);
        }
        result
    }
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub panel: Color32,
    pub canvas: Color32,
    pub paper: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub primary: Color32,
    pub on_primary: Color32,
    pub soft: Color32,
    pub border: Color32,
    pub hover: Color32,
    pub selection: Color32,
    pub warning: Color32,
    pub error: Color32,
    pub success: Color32,
    pub selected: Color32,
    pub grid: Color32,
    pub rail: Color32,
    pub rail_text: Color32,
    pub rail_active: Color32,
}

pub fn palette(ctx: &egui::Context) -> Palette {
    for_dark(ctx.style().visuals.dark_mode)
}

pub fn for_dark(dark: bool) -> Palette {
    if dark {
        Palette {
            panel: Color32::from_rgb(31, 42, 47),
            canvas: Color32::from_rgb(25, 33, 38),
            paper: Color32::from_rgb(22, 30, 35),
            text: Color32::from_rgb(229, 238, 235),
            muted: Color32::from_rgb(162, 182, 180),
            accent: Color32::from_rgb(112, 217, 197),
            primary: Color32::from_rgb(31, 108, 97),
            on_primary: Color32::WHITE,
            soft: Color32::from_rgb(42, 56, 61),
            border: Color32::from_rgb(66, 86, 91),
            hover: Color32::from_rgb(48, 72, 73),
            selection: Color32::from_rgb(40, 82, 77),
            warning: Color32::from_rgb(241, 189, 113),
            error: Color32::from_rgb(255, 143, 133),
            success: Color32::from_rgb(126, 216, 173),
            selected: Color32::from_rgb(243, 174, 110),
            grid: Color32::from_rgb(53, 75, 78),
            rail: Color32::from_rgb(18, 28, 33),
            rail_text: Color32::from_rgb(173, 197, 193),
            rail_active: Color32::from_rgb(41, 71, 69),
        }
    } else {
        Palette {
            panel: Color32::from_rgb(250, 252, 251),
            canvas: Color32::from_rgb(235, 240, 239),
            paper: Color32::from_rgb(244, 247, 246),
            text: Color32::from_rgb(35, 55, 58),
            muted: Color32::from_rgb(87, 110, 110),
            accent: Color32::from_rgb(20, 111, 104),
            primary: Color32::from_rgb(25, 103, 95),
            on_primary: Color32::WHITE,
            soft: Color32::from_rgb(229, 237, 235),
            border: Color32::from_rgb(200, 215, 212),
            hover: Color32::from_rgb(215, 232, 226),
            selection: Color32::from_rgb(212, 234, 226),
            warning: Color32::from_rgb(141, 82, 21),
            error: Color32::from_rgb(166, 55, 48),
            success: Color32::from_rgb(28, 113, 76),
            selected: Color32::from_rgb(175, 89, 42),
            grid: Color32::from_rgb(191, 210, 205),
            rail: Color32::from_rgb(30, 56, 58),
            rail_text: Color32::from_rgb(181, 205, 198),
            rail_active: Color32::from_rgb(44, 81, 78),
        }
    }
}

pub fn apply(ctx: &egui::Context, mode: Mode) {
    let dark = mode == Mode::Dark;
    ctx.set_theme(if dark {
        egui::ThemePreference::Dark
    } else {
        egui::ThemePreference::Light
    });
    let p = for_dark(dark);
    let mut v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.panel_fill = p.panel;
    v.window_fill = p.panel;
    v.window_stroke = Stroke::new(1.0_f32, p.border);
    v.extreme_bg_color = p.canvas;
    v.override_text_color = Some(p.text);
    v.widgets.noninteractive.bg_fill = p.panel;
    v.widgets.noninteractive.weak_bg_fill = p.panel;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text);
    v.widgets.inactive.bg_fill = p.soft;
    v.widgets.inactive.weak_bg_fill = p.soft;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.border);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.text);
    v.widgets.hovered.bg_fill = p.hover;
    v.widgets.hovered.weak_bg_fill = p.hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.accent);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, p.text);
    v.widgets.active.bg_fill = p.selection;
    v.widgets.active.weak_bg_fill = p.selection;
    v.widgets.active.fg_stroke = Stroke::new(1.0_f32, p.accent);
    v.selection.bg_fill = p.selection;
    v.selection.stroke = Stroke::new(1.0_f32, p.accent);
    v.faint_bg_color = p.soft;
    v.hyperlink_color = p.accent;
    v.warn_fg_color = p.warning;
    v.error_fg_color = p.error;
    ctx.set_visuals(v);
    ctx.style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 7.0);
        s.spacing.button_padding = egui::vec2(12.0, 7.0);
        s.spacing.interact_size.y = 28.0;
        s.text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(13.0));
        s.text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(13.0));
        s.text_styles
            .insert(egui::TextStyle::Small, egui::FontId::proportional(11.0));
    });
    ctx.request_repaint();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn switching_themes_preserves_control_sizes_and_uses_matching_surfaces() {
        let ctx = egui::Context::default();
        for mode in [Mode::Light, Mode::Dark, Mode::Light] {
            apply(&ctx, mode);
            assert_eq!(ctx.style().visuals.dark_mode, mode == Mode::Dark);
            assert_eq!(ctx.style().spacing.button_padding, egui::vec2(12.0, 7.0));
            assert_eq!(ctx.style().spacing.interact_size.y, 28.0);
            assert_eq!(ctx.style().visuals.extreme_bg_color, palette(&ctx).canvas);
        }
    }
    #[test]
    fn appearance_roundtrip_preserves_choice_and_rejects_invalid_mode() {
        let root = std::env::temp_dir().join(format!(
            "gridfinity-theme-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = root.join("appearance.json");
        assert_eq!(Appearance::read(&path).unwrap().mode, Mode::Light);
        for mode in [Mode::Dark, Mode::Light] {
            Appearance { mode, path: None }.save_to(&path).unwrap();
            assert_eq!(Appearance::read(&path).unwrap().mode, mode);
        }
        fs::write(&path, br#"{"mode":"unknown"}"#).unwrap();
        assert!(Appearance::read(&path).is_err());
        fs::remove_file(path).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn theme_text_has_readable_contrast_on_both_surfaces() {
        fn luminance(c: Color32) -> f64 {
            let rgb = [c.r(), c.g(), c.b()].map(|v| {
                let v = f64::from(v) / 255.0;
                if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            });
            rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722
        }
        for dark in [false, true] {
            let p = for_dark(dark);
            for (fg, bg) in [
                (p.text, p.panel),
                (p.muted, p.panel),
                (p.muted, p.canvas),
                (p.accent, p.panel),
                (p.error, p.panel),
                (p.on_primary, p.primary),
                (p.rail_text, p.rail),
            ] {
                let a = luminance(fg);
                let b = luminance(bg);
                assert!(
                    (a.max(b) + 0.05) / (a.min(b) + 0.05) >= 4.5,
                    "{fg:?} on {bg:?}"
                );
            }
        }
    }
}
