use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Product {
    Bin,
    Insert,
    Baseplate,
    Clip,
    #[serde(alias = "LabelClip")]
    LabelInsert,
    DeskBox,
}
impl Product {
    pub fn label(self) -> &'static str {
        match self {
            Self::Bin => "Behälter",
            Self::Insert => "Werkzeugeinsatz",
            Self::Baseplate => "Grundplatte",
            Self::Clip => "Verbindungsclip",
            Self::LabelInsert => "Einschubschild",
            Self::DeskBox => "Tischbox mit Schubladen",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LabelArea {
    #[default]
    None,
    Recess,
    #[serde(alias = "Clip")]
    SlideIn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CutShape {
    Circle,
    Rectangle,
    Hexagon,
    Stl,
}
impl CutShape {
    pub fn label(self) -> &'static str {
        match self {
            Self::Circle => "Rund",
            Self::Rectangle => "Abgerundet",
            Self::Hexagon => "Sechskant",
            Self::Stl => "STL-Werkzeug",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Wall {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub thickness: f64,
    pub height: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cutout {
    pub name: String,
    pub shape: CutShape,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub length: f64,
    pub depth: f64,
    pub angle: f64,
    pub clearance: f64,
    pub stl_path: Option<String>,
}
impl Cutout {
    pub fn half_extents(&self) -> [f64; 2] {
        let c = self;
        let a = c.angle.to_radians();
        let bounds = if c.shape == CutShape::Circle {
            ((c.width + c.clearance) / 2.0, (c.width + c.clearance) / 2.0)
        } else if c.shape == CutShape::Hexagon {
            let radius = (c.width + c.clearance) / 3.0_f64.sqrt();
            let points: Vec<_> = (0..6)
                .map(|i| {
                    let t = a + i as f64 * std::f64::consts::PI / 3.0;
                    [radius * t.cos(), radius * t.sin()]
                })
                .collect();
            (
                points.iter().map(|p| p[0].abs()).fold(0.0, f64::max),
                points.iter().map(|p| p[1].abs()).fold(0.0, f64::max),
            )
        } else {
            (
                ((c.width + c.clearance) * a.cos().abs()
                    + (c.length + c.clearance) * a.sin().abs())
                    / 2.0,
                ((c.width + c.clearance) * a.sin().abs()
                    + (c.length + c.clearance) * a.cos().abs())
                    / 2.0,
            )
        };
        [bounds.0, bounds.1]
    }
}
impl Default for Cutout {
    fn default() -> Self {
        Self {
            name: "Werkzeug".into(),
            shape: CutShape::Circle,
            x: 0.0,
            y: 0.0,
            width: 15.0,
            length: 25.0,
            depth: 10.0,
            angle: 0.0,
            clearance: 0.3,
            stl_path: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Design {
    pub version: u32,
    pub title: String,
    pub product: Product,
    pub grid_x: u32,
    pub grid_y: u32,
    pub height_u: u32,
    #[serde(default = "legacy_wall")]
    pub wall: f64,
    #[serde(default = "legacy_floor")]
    pub floor: f64,
    pub lip: bool,
    #[serde(default)]
    pub reference_profile: bool,
    #[serde(default)]
    pub thin_lip: bool,
    pub magnets: bool,
    #[serde(default)]
    pub magnet_corners_only: bool,
    pub magnet_diameter: f64,
    pub magnet_depth: f64,
    pub screws: bool,
    #[serde(default)]
    pub screw_corners_only: bool,
    pub label: String,
    pub engraved: bool,
    pub label_area: LabelArea,
    pub label_width: f64,
    pub label_height: f64,
    pub label_depth: f64,
    #[serde(alias = "label_clip_clearance")]
    pub label_insert_clearance: f64,
    pub walls: Vec<Wall>,
    pub cutouts: Vec<Cutout>,
    pub drawer_count: u32,
    pub drawer_height_u: u32,
    pub drawer_clearance: f64,
    pub printed_push_open: bool,
    pub spring_thickness: f64,
    pub top_grid: bool,
    pub baseplate_open: bool,
    pub baseplate_clips: bool,
    pub clip_clearance: f64,
}
fn legacy_wall() -> f64 {
    1.6
}
fn legacy_floor() -> f64 {
    2.0
}
impl Default for Design {
    fn default() -> Self {
        Self {
            version: 1,
            title: "Mein Gridfinity".into(),
            product: Product::Bin,
            grid_x: 2,
            grid_y: 2,
            height_u: 3,
            wall: 1.2,
            floor: 1.21,
            lip: true,
            reference_profile: true,
            thin_lip: false,
            magnets: false,
            magnet_corners_only: false,
            magnet_diameter: 6.2,
            magnet_depth: 2.4,
            screws: false,
            screw_corners_only: false,
            label: "".into(),
            engraved: false,
            label_area: LabelArea::None,
            label_width: 30.0,
            label_height: 9.0,
            label_depth: 0.4,
            label_insert_clearance: 0.25,
            walls: vec![],
            cutouts: vec![],
            drawer_count: 2,
            drawer_height_u: 3,
            drawer_clearance: 0.4,
            printed_push_open: false,
            spring_thickness: 0.8,
            top_grid: true,
            baseplate_open: true,
            baseplate_clips: false,
            clip_clearance: 0.0,
        }
    }
}
impl Design {
    pub fn from_json(bytes: &[u8]) -> anyhow::Result<Self> {
        let value: serde_json::Value = serde_json::from_slice(bytes)?;
        anyhow::ensure!(value.is_object(), "Projekt muss ein JSON-Objekt sein.");
        let legacy_label = value.get("product").and_then(|p| p.as_str()) == Some("LabelClip")
            || value.get("label_area").and_then(|p| p.as_str()) == Some("Clip");
        let mut design: Self = serde_json::from_value(value)?;
        if legacy_label {
            let width_limit = design.label_width_limit();
            let height_limit = design.label_height_limit();
            anyhow::ensure!(
                width_limit.is_finite()
                    && width_limit >= 10.0
                    && height_limit.is_finite()
                    && height_limit >= 3.0,
                "Die Boxabmessungen sind für ein Einschubschild ungültig."
            );
            design.label_width = design.label_width.clamp(10.0, width_limit);
            design.label_height = design.label_height.clamp(3.0, height_limit);
        }
        design.validate()?;
        Ok(design)
    }
    pub fn width(&self) -> f64 {
        self.grid_x as f64 * 42.0 - if self.reference_profile { 0.62 } else { 0.5 }
    }
    pub fn length(&self) -> f64 {
        self.grid_y as f64 * 42.0 - if self.reference_profile { 0.62 } else { 0.5 }
    }
    pub fn height(&self) -> f64 {
        self.height_u as f64 * 7.0
    }
    pub fn stacking_pitch(&self) -> f64 {
        self.height() + if self.reference_profile { 0.245 } else { 0.0 }
    }
    pub fn floor_z(&self) -> f64 {
        4.75 + self.floor
    }
    pub fn box_top_height(&self) -> f64 {
        self.height()
            + if self.reference_profile {
                if self.lip { 4.852579 } else { 4.85 }
            } else if self.lip {
                4.4
            } else {
                0.0
            }
    }
    pub fn label_width_limit(&self) -> f64 {
        if self.product == Product::LabelInsert {
            300.0
        } else {
            self.width() - 8.0
        }
    }
    pub fn label_height_limit(&self) -> f64 {
        if self.product == Product::LabelInsert {
            90.0
        } else if self.label_area == LabelArea::Recess {
            self.height() - 6.75
        } else {
            (self.box_top_height() - self.label_insert_bottom() - 2.25).max(3.0)
        }
    }
    pub fn label_depth_limit(&self) -> f64 {
        (self.wall - 0.8).min(1.2)
    }
    pub fn label_insert_bottom(&self) -> f64 {
        7.55 + 2.0 * self.label_insert_clearance
    }
    pub fn validate(&self) -> anyhow::Result<()> {
        use anyhow::ensure;
        ensure!(self.version == 1, "Unbekannte Projektversion.");
        ensure!(
            self.clip_clearance.is_finite() && (-0.1..=0.2).contains(&self.clip_clearance),
            "Clip-Toleranz: -0,1–0,2 mm."
        );
        if self.product == Product::Clip {
            return Ok(());
        }
        let standalone_label = self.product == Product::LabelInsert;
        ensure!(
            standalone_label || (1..=8).contains(&self.grid_x) && (1..=8).contains(&self.grid_y),
            "Raster muss zwischen 1 und 8 liegen."
        );
        ensure!(
            standalone_label || (2..=15).contains(&self.height_u),
            "Höhe muss zwischen 2 und 15 U liegen."
        );
        ensure!(
            standalone_label || self.wall.is_finite() && (1.0..=4.0).contains(&self.wall),
            "Wandstärke: 1–4 mm."
        );
        ensure!(
            standalone_label || self.floor.is_finite() && (1.2..=5.0).contains(&self.floor),
            "Bodenstärke: 1,2–5 mm."
        );
        let blank_recess = matches!(self.product, Product::Bin | Product::Insert)
            && self.label_area == LabelArea::Recess;
        ensure!(
            blank_recess || crate::lettering::text(&self.label).chars().count() <= 32,
            "Beschriftung darf maximal 32 Zeichen enthalten."
        );
        ensure!(
            blank_recess || crate::lettering::is_supported(&self.label),
            "Beschriftung unterstützt A–Z, Ä/Ö/Ü, ß, Zahlen, Leerzeichen, Bindestriche und Unterstriche."
        );
        if self.product == Product::LabelInsert
            || (matches!(self.product, Product::Bin | Product::Insert)
                && self.label_area != LabelArea::None)
        {
            ensure!(
                self.label_width.is_finite()
                    && self.label_width >= 10.0
                    && self.label_width <= self.label_width_limit() + 1e-8,
                "Etikettbreite muss zwischen 10 und {:.2} mm liegen.",
                self.label_width_limit()
            );
            ensure!(
                self.label_height.is_finite()
                    && self.label_height >= 3.0
                    && self.label_height <= self.label_height_limit() + 1e-8,
                "Etiketthöhe muss zwischen 3 und {:.2} mm liegen.",
                self.label_height_limit()
            );
            if blank_recess {
                ensure!(
                    self.label_depth.is_finite()
                        && self.label_depth >= 0.1
                        && self.label_depth <= self.label_depth_limit() + 1e-8,
                    "Etiketttiefe muss zwischen 0,1 und {:.2} mm liegen (mindestens 0,8 mm Restwand).",
                    self.label_depth_limit()
                );
            } else {
                ensure!(
                    self.label_insert_clearance.is_finite()
                        && (0.15..=0.6).contains(&self.label_insert_clearance),
                    "Schild-Spiel muss zwischen 0,15 und 0,6 mm liegen."
                );
            }
        }
        if standalone_label {
            return Ok(());
        }
        ensure!(
            self.walls.len() <= 40 && self.cutouts.len() <= 40,
            "Maximal 40 Wände bzw. Aussparungen."
        );
        ensure!(
            self.magnet_diameter.is_finite() && (5.0..=8.0).contains(&self.magnet_diameter),
            "Magnetdurchmesser: 5–8 mm."
        );
        ensure!(
            self.magnet_depth.is_finite() && (1.0..=3.0).contains(&self.magnet_depth),
            "Magnettiefe: 1–3 mm."
        );
        ensure!(
            (1..=5).contains(&self.drawer_count) && (2..=10).contains(&self.drawer_height_u),
            "Schubladen: 1–5, Höhe: 2–10 U."
        );
        ensure!(
            self.drawer_clearance.is_finite() && (0.2..=1.0).contains(&self.drawer_clearance),
            "Schubladenspiel: 0,2–1 mm pro Seite."
        );
        if self.product == Product::DeskBox && self.printed_push_open {
            ensure!(
                self.grid_x >= 2 && self.drawer_height_u >= 3,
                "Die gedruckte Drucköffnung benötigt mindestens 2 Rasterfelder Breite und 3 U Schubladenhöhe."
            );
            ensure!(
                self.spring_thickness.is_finite() && (0.8..=1.2).contains(&self.spring_thickness),
                "Federstärke: 0,8–1,2 mm."
            );
        }
        let max_x = self.width() / 2.0 - self.wall;
        let max_y = self.length() / 2.0 - self.wall;
        for w in self.walls.iter().filter(|_| self.product == Product::Bin) {
            ensure!(
                [w.x1, w.x2, w.y1, w.y2, w.thickness, w.height]
                    .iter()
                    .all(|v| v.is_finite()),
                "Ungültige Wandwerte."
            );
            ensure!(
                w.x1.abs() <= max_x
                    && w.x2.abs() <= max_x
                    && w.y1.abs() <= max_y
                    && w.y2.abs() <= max_y,
                "Eine Wand liegt außerhalb des Innenraums."
            );
            ensure!(
                (0.8..=4.0).contains(&w.thickness)
                    && (2.0..=self.height() - self.floor_z()).contains(&w.height),
                "Wandhöhe oder -stärke außerhalb des zulässigen Bereichs."
            );
            ensure!((w.x2 - w.x1).hypot(w.y2 - w.y1) >= 1.0, "Wand ist zu kurz.");
        }
        for c in self
            .cutouts
            .iter()
            .filter(|_| self.product == Product::Insert)
        {
            ensure!(
                [c.x, c.y, c.width, c.length, c.depth, c.angle, c.clearance]
                    .iter()
                    .all(|v| v.is_finite()),
                "Ungültige Aussparungswerte."
            );
            ensure!(
                (2.0..=200.0).contains(&c.width)
                    && (2.0..=200.0).contains(&c.length)
                    && (0.0..=2.0).contains(&c.clearance),
                "Ungültige Aussparungsgröße."
            );
            ensure!(
                (-180.0..=180.0).contains(&c.angle),
                "Drehung muss zwischen -180 und 180 Grad liegen."
            );
            ensure!(c.name.chars().count() <= 40, "Aussparungsname ist zu lang.");
            let top = self.height() - 1.4;
            ensure!(
                (1.0..=top - self.floor_z()).contains(&c.depth),
                "Aussparung ist zu tief: mindestens 1,2 mm Boden und der Gridfinity-Fuß müssen erhalten bleiben."
            );
            let [rx, ry] = c.half_extents();
            ensure!(
                c.x.abs() + rx <= max_x - 1.0 && c.y.abs() + ry <= max_y - 1.0,
                "Aussparung '{}' überschreitet die Seitenwand. Größe, Drehung oder Position anpassen.",
                c.name
            );
            if c.shape == CutShape::Stl {
                ensure!(
                    c.stl_path.as_ref().is_some_and(|p| !p.is_empty()),
                    "Für die STL-Aussparung fehlt eine Datei."
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod compatibility_tests {
    use super::*;
    #[test]
    fn german_label_characters_validate_without_transliteration() {
        for label in [
            "ÄÖÜäöüßẞ".to_owned(),
            "a\u{0308}o\u{0308}u\u{0308}".to_owned(),
            "a\u{0308}".repeat(32),
        ] {
            let cfg = Design {
                label,
                ..Default::default()
            };
            assert!(cfg.validate().is_ok(), "{}", cfg.label);
        }
        for label in [
            "a\u{0308}".repeat(33),
            "é".into(),
            "★".into(),
            "e\u{0308}".into(),
            "ﬃ".into(),
        ] {
            let cfg = Design {
                label,
                ..Default::default()
            };
            assert!(cfg.validate().is_err(), "{}", cfg.label);
        }
    }
    #[test]
    fn old_projects_keep_original_dimensions_and_new_projects_roundtrip() {
        let old: Design =
            serde_json::from_str(r#"{"product":"Bin","grid_x":2,"grid_y":2}"#).unwrap();
        assert!(!old.reference_profile);
        assert_eq!((old.wall, old.floor, old.width()), (1.6, 2.0, 83.5));
        let new: Design =
            serde_json::from_str(&serde_json::to_string(&Design::default()).unwrap()).unwrap();
        assert!(new.reference_profile);
        assert_eq!((new.wall, new.floor, new.width()), (1.2, 1.21, 83.38));
    }
    #[test]
    fn old_label_settings_migrate_without_panics_or_silent_new_design_resizing() {
        let old = br#"{"product":"Bin","label_area":"Clip","label_height":20,"label_clip_clearance":0.25}"#;
        let migrated = Design::from_json(old).unwrap();
        assert_eq!(migrated.label_area, LabelArea::SlideIn);
        assert_eq!(migrated.label_height, migrated.label_height_limit());
        assert_eq!(migrated.label_insert_clearance, 0.25);
        let standalone = Design::from_json(br#"{"product":"LabelClip","label_width":80,"label_height":30,"grid_x":0,"height_u":0}"#).unwrap();
        assert_eq!(standalone.product, Product::LabelInsert);
        assert_eq!(
            (standalone.label_width, standalone.label_height),
            (80.0, 30.0)
        );
        for invalid in [
            br#"{"product":"Bin","label_area":"Clip","grid_x":0}"#.as_slice(),
            br#"{"product":"Bin","label_area":"SlideIn","label_height":30}"#.as_slice(),
            b"null".as_slice(),
            b"[]".as_slice(),
        ] {
            assert!(Design::from_json(invalid).is_err());
        }
        let small = Design {
            reference_profile: false,
            lip: false,
            height_u: 2,
            label_area: LabelArea::SlideIn,
            label_insert_clearance: 0.6,
            ..Default::default()
        };
        assert!(small.label_height_limit() >= 3.0);
    }
}
