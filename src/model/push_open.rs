//! Experimental all-printed push-push mechanism. No purchased hardware.
//! Independent, replaceable planar springs; open heart track with ramped floors
//! and drop steps; a printed flexure follower. Geometry checks are not a fatigue
//! or a functional print validation. See README before printing.
use super::*;

pub const EXTRA_DEPTH: f64 = 70.0;
const CAM_Z: f64 = 5.0;
const TRACK_RADIUS: f64 = 1.15;
const STAGES: [&[[f64; 2]]; 4] = [
    &[[0.0, 20.0], [-5.0, 14.0], [-5.0, 1.0], [-3.0, 0.0]],
    &[[-3.0, 0.0], [0.0, 6.0]],
    &[[0.0, 6.0], [3.0, 0.0]],
    &[[3.0, 0.0], [5.0, 1.0], [5.0, 14.0], [0.0, 20.0]],
];

fn rear(cfg: &Design) -> f64 {
    (cfg.grid_y as f64 * 42.0 + 4.0) / 2.0
}

fn segment(a: [f64; 2], b: [f64; 2], width: f64, height: f64, z: f64) -> Manifold {
    let length = (b[0] - a[0]).hypot(b[1] - a[1]);
    let angle = (b[1] - a[1]).atan2(b[0] - a[0]).to_degrees();
    let bar = block(length, width, height, 0.0, 0.0, z)
        .rotate(0.0, 0.0, angle)
        .translate((a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0, 0.0);
    let cap = |p: [f64; 2]| {
        Manifold::cylinder(height, width / 2.0, width / 2.0, 20, false).translate(p[0], p[1], z)
    };
    Manifold::batch_union(&[bar, cap(a), cap(b)])
}

fn ramp_cut(a: [f64; 2], b: [f64; 2], bottom_a: f64, bottom_b: f64) -> Result<Manifold> {
    let length = (b[0] - a[0]).hypot(b[1] - a[1]);
    let radius = TRACK_RADIUS;
    let mut vertices = Vec::new();
    for top in [false, true] {
        for (x, y, z) in [
            (0.0, -radius, bottom_a),
            (length, -radius, bottom_b),
            (length, radius, bottom_b),
            (0.0, radius, bottom_a),
        ] {
            vertices.extend([x, y, if top { 4.1 } else { z }]);
        }
    }
    let mut triangles = vec![0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7];
    for i in 0..4_u64 {
        let j = (i + 1) % 4;
        triangles.extend([i, j, j + 4, i, j + 4, i + 4]);
    }
    let cut = Manifold::from_mesh_f64(&vertices, 3, &triangles)?
        .rotate(0.0, 0.0, (b[1] - a[1]).atan2(b[0] - a[0]).to_degrees())
        .translate(a[0], a[1], 0.0);
    let cap = |p: [f64; 2], z: f64| {
        Manifold::cylinder(4.1 - z, radius, radius, 32, false).translate(p[0], p[1], z)
    };
    Ok(Manifold::batch_union(&[
        cut,
        cap(a, bottom_a),
        cap(b, bottom_b),
    ]))
}

/// Open-top groove. Each leg rises from 1.2 to 2.0 mm, then drops into
/// the next leg; a spring-loaded flat-ended pin can step down but meets a
/// vertical face if it tries to retrace the previous leg.
pub(super) fn cam(cfg: &Design) -> Result<Manifold> {
    let mut cuts = Vec::new();
    for points in STAGES {
        let total: f64 = points
            .windows(2)
            .map(|p| (p[1][0] - p[0][0]).hypot(p[1][1] - p[0][1]))
            .sum();
        let mut traversed = 0.0;
        for pair in points.windows(2) {
            let length = (pair[1][0] - pair[0][0]).hypot(pair[1][1] - pair[0][1]);
            cuts.push(ramp_cut(
                pair[0],
                pair[1],
                1.2 + 0.8 * traversed / total,
                1.2 + 0.8 * (traversed + length) / total,
            )?);
            traversed += length;
        }
    }
    // Funnel for removal/reinsertion of a drawer. The follower is biased left
    // on insertion. The last return leg joins the same open mouth.
    cuts.push(segment([0.0, 20.0], [0.0, 27.0], 2.3, 2.9, 1.2));
    let root = block(16.0, 10.0, 4.0, 0.0, -6.8, 0.0);
    let base = &rounded(16.0, 29.0, 4.0, 1.0)?.translate(0.0, 10.5, 0.0) + &root;
    let track = &base - &Manifold::batch_union(&cuts);
    Ok(track.translate(0.0, rear(cfg) + 11.0, CAM_Z))
}

/// Nine transverse runs joined by semicircles, extruded in Z. Print flat.
/// End pads sit outside the working beams; no support or separate fasteners.
pub(super) fn spring(thickness: f64) -> Manifold {
    let mut pieces = Vec::new();
    for row in 0..9 {
        let y = row as f64 * 6.0;
        pieces.push(segment([-10.0, y], [10.0, y], thickness, 6.0, 0.0));
        if row < 8 {
            let side = if row % 2 == 0 { 1.0 } else { -1.0 };
            let center = [side * 10.0, y + 3.0];
            for step in 0..18 {
                let point = |s: usize| {
                    let angle =
                        -std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * s as f64 / 18.0;
                    [
                        center[0] + side * 3.0 * angle.cos(),
                        center[1] + 3.0 * angle.sin(),
                    ]
                };
                pieces.push(segment(point(step), point(step + 1), thickness, 6.0, 0.0));
            }
        }
    }
    pieces.push(block(26.0, 2.0, 6.0, 0.0, -0.5, 0.0));
    pieces.push(block(26.0, 2.0, 6.0, 0.0, 48.5, 0.0));
    Manifold::batch_union(&pieces)
}

/// The 45 mm long flexure bends both sideways and vertically. The pin is
/// flat ended to engage the drop steps. Print inverted, pin facing upwards.
pub(super) fn follower() -> Manifold {
    let pad = block(10.0, 6.0, 2.4, 0.0, 62.0, 8.6);
    let beam = segment([-0.6, 17.0], [0.0, 62.0], 1.0, 1.0, 10.0);
    let pin = Manifold::cylinder(3.95, 0.8, 0.8, 32, false).translate(-0.6, 17.0, 6.2);
    Manifold::batch_union(&[pad, beam, pin])
}

pub(super) fn mounts(cfg: &Design) -> Manifold {
    let mut pieces = Vec::new();
    let back = rear(cfg);
    let pitch = cfg.drawer_height_u as f64 * 7.0 + 3.0;
    let width = cfg.grid_x as f64 * 42.0 + 4.0;
    for row in 0..cfg.drawer_count {
        let z = 3.2 + row as f64 * pitch;
        for side in [-1.0, 1.0] {
            let x = side * 27.0;
            // Rear spring pad pocket, open above and towards the front.
            pieces.push(block(28.0, 34.0, 2.0, x, back + 54.0, z));
            pieces.push(block(28.0, 2.0, 6.4, x, back + 42.2, z + 2.0));
            for edge in [-1.0, 1.0] {
                pieces.push(block(1.0, 4.0, 6.4, x + edge * 13.8, back + 40.0, z + 2.0));
                // Retain the rear pad vertically, while leaving the spring replaceable.
                pieces.push(block(3.0, 4.0, 1.2, x + edge * 12.8, back + 40.0, z + 8.3));
            }
            // Hard stop before excessive cam/spring travel. Release press 6 mm.
            pieces.push(block(
                3.8,
                2.0,
                4.0,
                side * (width / 2.0 - 0.5),
                back + 7.8,
                z + 5.0,
            ));
            // Connect stops to the outer wall; the rear edge of the tray contacts them.
            pieces.push(block(
                4.0,
                10.0,
                4.0,
                side * (width / 2.0 + cfg.drawer_clearance),
                back + 11.8,
                z + 5.0,
            ));
        }
        // Follower anchor: horizontal T-slot, slide the pad backwards from front.
        pieces.push(block(14.0, 14.0, 2.0, 0.0, back + 64.0, z + 6.6));
        pieces.push(block(14.0, 2.0, 5.0, 0.0, back + 66.3, z + 8.6));
        for side in [-1.0, 1.0] {
            pieces.push(block(1.5, 8.0, 4.0, side * 6.1, back + 62.0, z + 8.6));
            pieces.push(block(3.0, 8.0, 1.2, side * 5.4, back + 62.0, z + 11.3));
        }
        // A narrow front detent is intentionally flexible during installation.
        pieces.push(block(2.0, 0.5, 0.7, 0.0, back + 58.5, z + 8.6));
    }
    Manifold::batch_union(&pieces)
}

pub(super) fn parts(cfg: &Design, row: u32) -> Result<Vec<Part>> {
    let z = 3.2 + row as f64 * (cfg.drawer_height_u as f64 * 7.0 + 3.0);
    let mut result = Vec::new();
    for (side, suffix) in [(-1.0, "links"), (1.0, "rechts")] {
        let mut p = part(
            &format!("Feder_{}_{suffix}", row + 1),
            spring(cfg.spring_thickness),
            [
                (side * 27.0) as f32,
                (rear(cfg) - 8.5) as f32,
                (z + 2.0) as f32,
            ],
            [242, 187, 76],
        )?;
        p.motion = Motion::Spring(row);
        result.push(p);
    }
    let mut finger = part(
        &format!("Rastfinger_{}", row + 1),
        follower(),
        [0.0, rear(cfg) as f32, z as f32],
        [133, 213, 153],
    )?;
    finger.print_rotation = [180.0, 0.0, 0.0];
    finger.motion = Motion::Follower;
    result.push(finger);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_and_tall_housings_keep_all_mechanism_parts_connected() {
        for (depth, count, clearance, roof) in [(1, 1, 0.2, false), (1, 5, 1.0, true)] {
            let cfg = Design {
                product: Product::DeskBox,
                printed_push_open: true,
                grid_y: depth,
                drawer_count: count,
                drawer_clearance: clearance,
                top_grid: roof,
                ..Default::default()
            };
            let parts = generate(&cfg).unwrap();
            assert_eq!(parts.len(), 1 + 4 * count as usize);
            assert!(
                parts
                    .iter()
                    .all(|p| p.report.closed && p.report.components == 1)
            );
            assert_eq!(parts[0].print_rotation, [-90.0, 0.0, 0.0]);
        }
    }

    #[test]
    fn assembled_mounts_leave_clearance_for_separate_parts() {
        let cfg = Design {
            product: Product::DeskBox,
            printed_push_open: true,
            drawer_count: 1,
            ..Default::default()
        };
        let frame = housing(&cfg).unwrap();
        let tray = drawer(&cfg).unwrap().translate(0.0, 0.0, 3.2);
        for p in parts(&cfg, 0).unwrap() {
            let vertices: Vec<f64> = p
                .vertices
                .iter()
                .flat_map(|v| p.assembly_vertex(*v, 0.0, cfg.grid_y).map(f64::from))
                .collect();
            let faces: Vec<u64> = p.faces.iter().flatten().map(|v| *v as u64).collect();
            let assembled = Manifold::from_mesh_f64(&vertices, 3, &faces).unwrap();
            assert!(
                frame.intersection(&assembled).volume() < 0.001,
                "{} intersects housing: {}",
                p.name,
                frame.intersection(&assembled).volume()
            );
            assert!(
                tray.intersection(&assembled).volume() < 0.001,
                "{} intersects drawer: {}",
                p.name,
                tray.intersection(&assembled).volume()
            );
        }
    }

    #[test]
    fn old_projects_keep_standard_drawers_and_prototype_round_trips() {
        let mut cfg: Design = serde_json::from_str(r#"{"product":"DeskBox"}"#).unwrap();
        assert!(!cfg.printed_push_open);
        cfg.printed_push_open = true;
        let loaded: Design = serde_json::from_str(&serde_json::to_string(&cfg).unwrap()).unwrap();
        assert!(loaded.printed_push_open);
        assert_eq!(loaded.spring_thickness, 0.8);
        cfg.grid_x = 1;
        assert!(cfg.validate().is_err());
        cfg.grid_x = 2;
        cfg.spring_thickness = 1.21;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn printed_parts_are_connected_and_fit_in_the_reserved_space() {
        for strength in [0.8, 1.2] {
            let cfg = Design {
                product: Product::DeskBox,
                printed_push_open: true,
                spring_thickness: strength,
                drawer_count: 1,
                ..Default::default()
            };
            let generated = generate(&cfg).unwrap();
            assert_eq!(generated.len(), 5);
            for p in &generated {
                assert!(p.report.closed, "{}", p.name);
                assert_eq!(p.report.components, 1, "{}", p.name);
                assert!(
                    print_dimensions(p)
                        .unwrap()
                        .into_iter()
                        .all(|n| n.is_finite() && n > 0.0)
                );
            }
            assert!((generated[0].report.dimensions[1] - 163.0).abs() < 1e-3);
            let frame = housing(&cfg).unwrap();
            let tray = drawer(&cfg).unwrap();
            for displacement in [-20.0, -10.0, 0.0, 3.0, 6.7] {
                let moved = tray.translate(0.0, displacement, 3.2);
                assert!(
                    frame.intersection(&moved).volume() < 1e-5,
                    "collision at {displacement}"
                );
            }
            assert!(
                frame.intersection(&tray.translate(0.0, 7.5, 3.2)).volume() > 0.1,
                "overtravel stop is missing"
            );
        }
    }

    #[test]
    fn springs_export_relaxed_and_preview_does_not_move_the_fixed_follower() {
        let cfg = Design {
            product: Product::DeskBox,
            printed_push_open: true,
            ..Default::default()
        };
        let p = parts(&cfg, 0).unwrap();
        let spring = &p[0];
        assert!((print_dimensions(spring).unwrap()[1] - 51.0).abs() < 0.01);
        let front = [0.0, 0.0, 0.0];
        let closed = spring.assembly_vertex(front, 0.0, cfg.grid_y);
        let opened = spring.assembly_vertex(front, 1.0, cfg.grid_y);
        assert!((closed[1] - opened[1] - 10.0).abs() < 0.001);
        assert_eq!(
            spring.assembly_vertex([0.0, 48.0, 0.0], 0.0, cfg.grid_y),
            spring.assembly_vertex([0.0, 48.0, 0.0], 1.0, cfg.grid_y)
        );
        assert_eq!(
            p[2].assembly_vertex(front, 0.0, cfg.grid_y),
            p[2].assembly_vertex(front, 1.0, cfg.grid_y)
        );
    }

    #[test]
    fn heart_track_has_an_open_pin_path_and_retained_floor() {
        let cfg = Design {
            printed_push_open: true,
            ..Default::default()
        };
        let solid = cam(&cfg)
            .unwrap()
            .translate(0.0, -rear(&cfg) - 11.0, -CAM_Z);
        for stage in STAGES {
            for pair in stage.windows(2) {
                for i in 0..=12 {
                    let t = i as f64 / 12.0;
                    let p: [f64; 2] =
                        std::array::from_fn(|k| pair[0][k] + (pair[1][k] - pair[0][k]) * t);
                    let pin =
                        Manifold::cylinder(1.5, 0.8, 0.8, 24, false).translate(p[0], p[1], 2.1);
                    assert!(solid.intersection(&pin).volume() < 1e-5, "blocked at {p:?}");
                    let floor =
                        Manifold::cylinder(0.5, 0.4, 0.4, 20, false).translate(p[0], p[1], 0.1);
                    assert!(
                        solid.intersection(&floor).volume() > 0.2,
                        "missing floor at {p:?}"
                    );
                }
            }
        }
    }
}
