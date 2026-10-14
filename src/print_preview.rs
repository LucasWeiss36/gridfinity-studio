//! Print-oriented model and finite, dimensionally accurate printer-bed display.
//! Display geometry stays below Z=0 and is never added to an exported part.
use crate::{model, printer};
use anyhow::{Context, Result, ensure};

pub(crate) const TOO_LARGE: [u8; 3] = [181, 66, 51];
pub(crate) const BED: [u8; 3] = [216, 226, 224];
pub(crate) const MARGIN: [u8; 3] = [225, 216, 195];
pub(crate) const GRID: [u8; 3] = [177, 194, 189];
pub(crate) const OUTLINE: [u8; 3] = [69, 109, 92];
pub(crate) const USABLE: [u8; 3] = [153, 115, 52];

pub struct Preview {
    pub vertices: Vec<[f32; 3]>,
    pub faces: Vec<([usize; 3], [u8; 3])>,
    pub low: [f32; 3],
    pub high: [f32; 3],
    pub fit: printer::Fit,
    pub rotated: bool,
    /// Actual dimensions after the optional in-plane 90-degree rotation.
    pub dimensions: [f64; 3],
}

pub fn build(part: &model::Part, settings: &printer::Settings, show_bed: bool) -> Result<Preview> {
    settings.validate()?;
    let volume = settings
        .volume()
        .context("Für die Druckbettansicht zuerst einen Drucker auswählen.")?;
    let usable = settings
        .usable_size()
        .context("Die Druckbettmaße sind ungültig.")?;
    let mut vertices = model::print_vertices(part)?;
    ensure!(
        part.faces.iter().flatten().all(|i| *i < vertices.len()),
        "Das Druckteil enthält ungültige Flächen."
    );
    let (low, high) = bounds(&vertices);
    let mut dimensions = std::array::from_fn(|i| f64::from(high[i]) - f64::from(low[i]));
    let fit = printer::fit(dimensions, usable);
    let rotated = fit == printer::Fit::Rotate90;
    let centre: [f64; 2] = std::array::from_fn(|i| (f64::from(low[i]) + f64::from(high[i])) / 2.0);
    for vertex in &mut vertices {
        let x = (f64::from(vertex[0]) - centre[0]) as f32;
        let y = (f64::from(vertex[1]) - centre[1]) as f32;
        [vertex[0], vertex[1]] = if rotated { [-y, x] } else { [x, y] };
    }
    if rotated {
        dimensions.swap(0, 1);
    }
    let colour = if fit == printer::Fit::TooLarge {
        TOO_LARGE
    } else {
        part.colour
    };
    let mut preview = Preview {
        vertices,
        faces: part.faces.iter().map(|face| (*face, colour)).collect(),
        low,
        high,
        fit,
        rotated,
        dimensions,
    };
    if show_bed {
        preview.add_bed(volume, settings.margin);
    }
    ensure!(
        preview.vertices.iter().flatten().all(|n| n.is_finite()),
        "Die Druckbettvorschau enthält ungültige Koordinaten."
    );
    (preview.low, preview.high) = bounds(&preview.vertices);
    Ok(preview)
}

fn bounds(vertices: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut low = [f32::INFINITY; 3];
    let mut high = [f32::NEG_INFINITY; 3];
    for vertex in vertices {
        for axis in 0..3 {
            low[axis] = low[axis].min(vertex[axis]);
            high[axis] = high[axis].max(vertex[axis]);
        }
    }
    (low, high)
}

impl Preview {
    fn rectangle(&mut self, low: [f32; 2], high: [f32; 2], z: f32, colour: [u8; 3]) {
        if low[0] >= high[0] || low[1] >= high[1] {
            return;
        }
        let start = self.vertices.len();
        self.vertices.extend([
            [low[0], low[1], z],
            [high[0], low[1], z],
            [high[0], high[1], z],
            [low[0], high[1], z],
        ]);
        self.faces.extend([
            ([start, start + 1, start + 2], colour),
            ([start, start + 2, start + 3], colour),
        ]);
    }

    /// Inset strips keep every corner and line within the actual bed footprint.
    fn border(&mut self, half: [f32; 2], width: f32, z: f32, colour: [u8; 3]) {
        let [x, y] = half;
        let width = width.min(x).min(y);
        self.rectangle([-x, -y], [x, -y + width], z, colour);
        self.rectangle([-x, y - width], [x, y], z, colour);
        self.rectangle([-x, -y + width], [-x + width, y - width], z, colour);
        self.rectangle([x - width, -y + width], [x, y - width], z, colour);
    }

    fn add_bed(&mut self, volume: [f64; 3], margin: f64) {
        let [x, y] = [volume[0] as f32 / 2.0, volume[1] as f32 / 2.0];
        let margin = margin as f32;
        let [ux, uy] = [x - margin, y - margin];
        self.rectangle([-x, -y], [x, y], -0.4, BED);
        if margin > 0.0 {
            self.border([x, y], margin, -0.35, MARGIN);
        }
        // Integer multiples of 10 mm, anchored at the centre of the bed.
        // Every strip is clipped so it can never enlarge the physical footprint.
        for index in (-(ux / 10.0).floor() as i32)..=((ux / 10.0).floor() as i32) {
            let at = index as f32 * 10.0;
            self.rectangle(
                [(at - 0.06).max(-ux), -uy],
                [(at + 0.06).min(ux), uy],
                -0.3,
                GRID,
            );
        }
        for index in (-(uy / 10.0).floor() as i32)..=((uy / 10.0).floor() as i32) {
            let at = index as f32 * 10.0;
            self.rectangle(
                [-ux, (at - 0.06).max(-uy)],
                [ux, (at + 0.06).min(uy)],
                -0.29,
                GRID,
            );
        }
        if margin > 0.0 {
            self.border([ux, uy], 0.3, -0.2, USABLE);
        }
        self.border([x, y], 0.5, -0.1, OUTLINE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design::{Design, Product};

    fn printer(size: [f64; 3], margin: f64) -> printer::Settings {
        printer::Settings {
            profile_id: Some("custom".into()),
            custom_size: size,
            margin,
        }
    }

    fn block(size: [f32; 3]) -> model::Part {
        // Deliberately off-centre and above Z=0 to exercise STL placement.
        let origin = [7.0, -19.0, 4.0];
        let vertices = (0..8)
            .map(|corner| {
                std::array::from_fn(|axis| {
                    origin[axis]
                        + if corner & (1 << axis) != 0 {
                            size[axis]
                        } else {
                            0.0
                        }
                })
            })
            .collect();
        model::Part {
            name: "Druckbettprüfung".into(),
            vertices,
            faces: vec![
                [0, 2, 3],
                [0, 3, 1],
                [4, 5, 7],
                [4, 7, 6],
                [0, 1, 5],
                [0, 5, 4],
                [2, 6, 7],
                [2, 7, 3],
                [0, 4, 6],
                [0, 6, 2],
                [1, 3, 7],
                [1, 7, 5],
            ],
            placement: [300.0, -200.0, 100.0],
            print_rotation: [0.0; 3],
            colour: [45, 167, 216],
            report: model::Report {
                dimensions: size.map(f64::from),
                triangles: 12,
                volume_cm3: size.iter().map(|v| f64::from(*v)).product::<f64>() / 1000.0,
                closed: true,
                boundary_edges: 0,
                nonmanifold_edges: 0,
                components: 1,
            },
            motion: model::Motion::Fixed,
        }
    }

    #[test]
    fn rectangular_bed_and_margin_are_finite_and_keep_the_exact_footprint() {
        let part = block([40.0, 50.0, 20.0]);
        let preview = build(&part, &printer([240.0, 180.0, 250.0], 5.0), true).unwrap();
        assert_eq!(preview.low, [-120.0, -90.0, -0.4]);
        assert_eq!(preview.high, [120.0, 90.0, 20.0]);
        assert_eq!(preview.fit, printer::Fit::Fits);
        assert_eq!(preview.dimensions, [40.0, 50.0, 20.0]);
        let bed = &preview.vertices[part.vertices.len()..];
        assert!(bed.iter().flatten().all(|v| v.is_finite()));
        assert!(
            bed.iter()
                .all(|v| v[0].abs() <= 120.0 && v[1].abs() <= 90.0 && v[2] < 0.0)
        );
        assert!(preview.faces.iter().any(|(_, colour)| *colour == MARGIN));
        let usable_corners: Vec<_> = preview
            .faces
            .iter()
            .filter(|(_, colour)| *colour == USABLE)
            .flat_map(|(face, _)| face.iter().map(|i| preview.vertices[*i]))
            .collect();
        let (low, high) = bounds(&usable_corners);
        assert_eq!([low[0], low[1]], [-115.0, -85.0]);
        assert_eq!([high[0], high[1]], [115.0, 85.0]);
    }

    #[test]
    fn margin_and_height_failures_colour_the_part_and_do_not_hide_overhang() {
        let part = block([98.0, 78.0, 20.0]);
        let no_margin = build(&part, &printer([100.0, 80.0, 200.0], 0.0), true).unwrap();
        assert_eq!(no_margin.fit, printer::Fit::Fits);
        let margin = build(&part, &printer([100.0, 80.0, 200.0], 2.0), true).unwrap();
        assert_eq!(margin.fit, printer::Fit::TooLarge);
        assert!(
            margin.faces[..part.faces.len()]
                .iter()
                .all(|(_, c)| *c == TOO_LARGE)
        );
        let height = build(&part, &printer([100.0, 80.0, 19.0], 0.0), true).unwrap();
        assert_eq!(height.fit, printer::Fit::TooLarge);
        assert!(!height.rotated);
        let overhang = build(
            &block([180.0, 120.0, 20.0]),
            &printer([160.0, 140.0, 30.0], 0.0),
            true,
        )
        .unwrap();
        assert_eq!(overhang.fit, printer::Fit::TooLarge);
        assert_eq!(overhang.low, [-90.0, -70.0, -0.4]);
        assert_eq!(overhang.high, [90.0, 70.0, 20.0]);
    }

    #[test]
    fn optional_rotation_fits_rectangular_bed_and_hiding_bed_preserves_model() {
        let part = block([120.0, 180.0, 3.0]);
        let settings = printer([200.0, 140.0, 20.0], 5.0);
        let preview = build(&part, &settings, true).unwrap();
        assert_eq!(preview.fit, printer::Fit::Rotate90);
        assert!(preview.rotated);
        assert_eq!(preview.dimensions, [180.0, 120.0, 3.0]);
        let hidden = build(&part, &settings, false).unwrap();
        assert_eq!(hidden.low, [-90.0, -60.0, 0.0]);
        assert_eq!(hidden.high, [90.0, 60.0, 3.0]);
        assert_eq!(hidden.vertices, preview.vertices[..part.vertices.len()]);
        assert_eq!(hidden.faces.len(), part.faces.len());
        assert_eq!(hidden.vertices[0], [90.0, -60.0, 0.0]);
    }

    #[test]
    fn label_and_push_open_housing_use_exact_stl_orientation_without_mutating_export() {
        let settings = printer([500.0; 3], 0.0);
        for product in [Product::LabelInsert, Product::DeskBox] {
            let cfg = Design {
                product,
                printed_push_open: true,
                drawer_count: 1,
                ..Default::default()
            };
            let parts = model::generate(&cfg).unwrap();
            let part = &parts[0];
            let original_vertices = part.vertices.clone();
            let original_rotation = part.print_rotation;
            assert_eq!(original_rotation, [-90.0, 0.0, 0.0]);
            let expected = model::print_vertices(part).unwrap();
            let (low, high) = bounds(&expected);
            let preview = build(part, &settings, false).unwrap();
            assert!(!preview.rotated);
            assert_eq!(preview.dimensions, model::print_dimensions(part).unwrap());
            for (actual, expected) in preview.vertices.iter().zip(expected) {
                assert_eq!(actual[2], expected[2]);
                for axis in 0..2 {
                    let centre = (f64::from(low[axis]) + f64::from(high[axis])) / 2.0;
                    assert_eq!(actual[axis], (f64::from(expected[axis]) - centre) as f32);
                }
            }
            assert_eq!(part.vertices, original_vertices);
            assert_eq!(part.print_rotation, original_rotation);
        }
        let part = block([120.0, 180.0, 3.0]);
        let path = std::env::temp_dir().join(format!(
            "gridfinity-preview-export-{}.stl",
            std::process::id()
        ));
        model::export_stl(&part, &path).unwrap();
        let original = std::fs::read(&path).unwrap();
        build(&part, &printer([200.0, 140.0, 20.0], 5.0), true).unwrap();
        model::export_stl(&part, &path).unwrap();
        let after = std::fs::read(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(original, after);
    }

    #[test]
    fn invalid_settings_or_mesh_do_not_produce_a_preview() {
        let mut part = block([10.0; 3]);
        assert!(build(&part, &printer::Settings::default(), true).is_err());
        assert!(build(&part, &printer([20.0; 3], 10.0), true).is_err());
        part.faces[0][0] = usize::MAX;
        assert!(build(&part, &printer([20.0; 3], 0.0), true).is_err());
        part.vertices[0][0] = f32::NAN;
        assert!(build(&part, &printer([20.0; 3], 0.0), true).is_err());
    }
}
