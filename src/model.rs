use crate::{
    design::{CutShape, Design, LabelArea, Product, Wall},
    lettering,
};
use anyhow::{Context, Result, ensure};
use manifold_csg::Manifold;
use serde::Serialize;
use std::{collections::HashMap, fs, io::Write, path::Path};
#[path = "model/push_open.rs"]
mod push_open;

#[derive(Clone, Copy)]
pub enum Motion {
    Fixed,
    Drawer(u32),
    Spring(u32),
    Follower,
}

#[derive(Clone)]
pub struct Part {
    pub name: String,
    pub vertices: Vec<[f32; 3]>,
    pub faces: Vec<[usize; 3]>,
    pub placement: [f32; 3],
    pub print_rotation: [f64; 3],
    pub colour: [u8; 3],
    pub report: Report,
    pub motion: Motion,
}
impl Part {
    pub fn assembly_vertex(&self, mut v: [f32; 3], opening: f32, depth: u32) -> [f32; 3] {
        let travel = |row: u32| opening * depth as f32 * 32.0 * (1.0 + (row + 1) as f32 * 0.08);
        match self.motion {
            Motion::Fixed => {}
            Motion::Follower => v[0] += 0.6 * ((59.0 - v[1]) / 42.0).clamp(0.0, 1.0),
            Motion::Drawer(row) => v[1] -= travel(row),
            // Schematic compression only. STL always uses the undeformed spring.
            Motion::Spring(row) => {
                let compression = (10.0 - travel(row)).max(0.0);
                v[1] += ((48.0 - v[1]) / 48.0).clamp(0.0, 1.0) * compression;
            }
        }
        std::array::from_fn(|axis| v[axis] + self.placement[axis])
    }
}
#[derive(Clone, Serialize)]
pub struct Report {
    pub dimensions: [f64; 3],
    pub triangles: usize,
    pub volume_cm3: f64,
    pub closed: bool,
    pub boundary_edges: usize,
    pub nonmanifold_edges: usize,
    pub components: usize,
}

fn outline(w: f64, d: f64, r: f64) -> Vec<[f64; 2]> {
    let r = r.max(0.1).min(w.min(d) / 2.0 - 0.01);
    let mut p = Vec::new();
    for (cx, cy, angle) in [
        (w / 2.0 - r, d / 2.0 - r, 0.0),
        (-w / 2.0 + r, d / 2.0 - r, 90.0),
        (-w / 2.0 + r, -d / 2.0 + r, 180.0),
        (w / 2.0 - r, -d / 2.0 + r, 270.0),
    ] {
        for i in 0..=12 {
            let a = (angle + i as f64 * 90.0 / 12.0).to_radians();
            p.push([cx + r * a.cos(), cy + r * a.sin()]);
        }
    }
    p
}
fn loft(profiles: &[(f64, f64, f64, f64)]) -> Result<Manifold> {
    let mut v = Vec::new();
    let mut t = Vec::<u64>::new();
    let n = 52;
    for (z, w, d, r) in profiles {
        ensure!(*w > 0.2 && *d > 0.2, "Profil ist zu klein.");
        for [x, y] in outline(*w, *d, *r) {
            v.extend([x, y, *z]);
        }
    }
    for i in 1..n - 1 {
        t.extend([0, (i + 1) as u64, i as u64]);
    }
    let top = (profiles.len() - 1) * n;
    for i in 1..n - 1 {
        t.extend([top as u64, (top + i) as u64, (top + i + 1) as u64]);
    }
    for level in 0..profiles.len() - 1 {
        for i in 0..n {
            let a = (level * n + i) as u64;
            let b = (level * n + (i + 1) % n) as u64;
            let c = b + n as u64;
            let d = a + n as u64;
            t.extend([a, b, c, a, c, d]);
        }
    }
    Manifold::from_mesh_f64(&v, 3, &t).context("Profil konnte nicht geschlossen werden")
}
fn rounded(w: f64, d: f64, h: f64, r: f64) -> Result<Manifold> {
    loft(&[(0.0, w, d, r), (h, w, d, r)])
}
fn block(w: f64, d: f64, h: f64, x: f64, y: f64, z: f64) -> Manifold {
    Manifold::cube(w, d, h, false).translate(x - w / 2.0, y - d / 2.0, z)
}

fn foot() -> Result<Manifold> {
    loft(&[
        (0.0, 35.6, 35.6, 0.8),
        (0.8, 37.2, 37.2, 1.6),
        (2.6, 37.2, 37.2, 1.6),
        (4.75, 41.5, 41.5, 3.75),
    ])
}
fn feet(cfg: &Design) -> Result<Manifold> {
    let base = if cfg.reference_profile {
        loft(&[
            (0.0, 35.48, 35.48, 0.8),
            (0.8, 37.08, 37.08, 1.6),
            (2.6, 37.08, 37.08, 1.6),
            (4.75, 41.38, 41.38, 3.75),
        ])?
    } else {
        foot()?
    };
    let mut parts = Vec::new();
    for ix in 0..cfg.grid_x {
        for iy in 0..cfg.grid_y {
            let mut one = base.clone();
            let mut holes = Vec::new();
            for x in [-13.0, 13.0] {
                for y in [-13.0, 13.0] {
                    let corner = (if x < 0.0 {
                        ix == 0
                    } else {
                        ix == cfg.grid_x - 1
                    }) && (if y < 0.0 {
                        iy == 0
                    } else {
                        iy == cfg.grid_y - 1
                    });
                    if cfg.magnets && (!cfg.magnet_corners_only || corner) {
                        holes.push(
                            Manifold::cylinder(
                                cfg.magnet_depth + 0.03,
                                cfg.magnet_diameter / 2.0,
                                cfg.magnet_diameter / 2.0,
                                48,
                                false,
                            )
                            .translate(x, y, -0.03),
                        );
                    }
                    if cfg.screws && (!cfg.screw_corners_only || corner) {
                        holes.push(
                            Manifold::cylinder(4.1, 1.5, 1.5, 32, false).translate(x, y, -0.03),
                        );
                    }
                }
            }
            if !holes.is_empty() {
                one = &one - &Manifold::batch_union(&holes);
            }
            parts.push(one.translate(
                (ix as f64 - (cfg.grid_x - 1) as f64 / 2.0) * 42.0,
                (iy as f64 - (cfg.grid_y - 1) as f64 / 2.0) * 42.0,
                0.0,
            ));
        }
    }
    Ok(Manifold::batch_union(&parts))
}
fn reference_body(cfg: &Design, insert: bool) -> Result<Manifold> {
    let (w, d) = (cfg.width(), cfg.length());
    let h = cfg.height();
    let top = h + 4.853579;
    let body_top = if cfg.lip { h + 2.72 } else { h + 4.85 };
    let outer = rounded(w, d, body_top - 4.73, 3.75)?.translate(0.0, 0.0, 4.73);
    let cavity_bottom = if insert { h - 1.4 } else { cfg.floor_z() };
    let cavity = rounded(
        w - 2.0 * cfg.wall,
        d - 2.0 * cfg.wall,
        body_top + 1.0,
        (3.75 - cfg.wall / 2.0).max(0.9375),
    )?
    .translate(0.0, 0.0, cavity_bottom);
    let mut solid = &(&outer + &feet(cfg)?) - &cavity;
    if cfg.lip {
        let apex = h + 4.995;
        let cap_center = apex - 0.1 * (1.0 + 2.0_f64.sqrt());
        let lip_bottom = if cfg.thin_lip { h - 0.705 } else { h - 3.905 };
        let mut outside = vec![(lip_bottom, w, d, 3.75), (cap_center, w, d, 3.75)];
        let inset = if cfg.thin_lip { 1.95 } else { 2.15 };
        let mut inside = if cfg.thin_lip {
            vec![
                (h - 0.705, w, d, 3.75),
                (
                    apex - inset - 1.8,
                    w - 2.0 * inset,
                    d - 2.0 * inset,
                    3.75 - inset,
                ),
                (apex - inset, w - 2.0 * inset, d - 2.0 * inset, 3.75 - inset),
            ]
        } else {
            vec![
                (h - 3.905, w, d, 3.75),
                (h - 0.955, w - 5.9, d - 5.9, 0.8),
                (h + 0.245, w - 5.9, d - 5.9, 0.8),
                (h + 1.045, w - 4.3, d - 4.3, 1.6),
                (h + 2.845, w - 4.3, d - 4.3, 1.6),
            ]
        };
        inside.push((
            cap_center,
            w - 2.0 * (apex - cap_center),
            d - 2.0 * (apex - cap_center),
            3.75 - (apex - cap_center),
        ));
        for i in 1..=16 {
            let z = cap_center + (top - 0.001 - cap_center) * i as f64 / 16.0;
            let half = (0.01 - (z - cap_center).powi(2)).max(0.0).sqrt();
            let outer_inset = 0.1 - half;
            let inner_inset = if z <= cap_center + 0.1 / 2.0_f64.sqrt() {
                apex - z
            } else {
                0.1 + half
            };
            outside.push((
                z,
                w - 2.0 * outer_inset,
                d - 2.0 * outer_inset,
                3.75 - outer_inset,
            ));
            inside.push((
                z,
                w - 2.0 * inner_inset,
                d - 2.0 * inner_inset,
                3.75 - inner_inset,
            ));
        }
        let last = *inside.last().unwrap();
        inside.push((top + 0.1, last.1, last.2, last.3));
        let lip = &loft(&outside)? - &loft(&inside)?;
        solid = &solid + &lip;
    }
    Ok(solid)
}
fn pocket(cfg: &Design, insert: bool) -> Result<Manifold> {
    let w = cfg.width();
    let d = cfg.length();
    let h = cfg.height();
    let ri = (3.75 - cfg.wall).max(0.8);
    let mut profiles = if insert {
        vec![(h - 1.4, w - 2.0 * cfg.wall, d - 2.0 * cfg.wall, ri)]
    } else {
        vec![
            (
                cfg.floor_z(),
                w - 2.0 * cfg.wall - 2.0,
                d - 2.0 * cfg.wall - 2.0,
                ri,
            ),
            (
                cfg.floor_z() + 1.0,
                w - 2.0 * cfg.wall,
                d - 2.0 * cfg.wall,
                ri,
            ),
        ]
    };
    if cfg.lip {
        if !insert && h - 3.8 > cfg.floor_z() + 1.0 {
            profiles.push((h - 3.8, w - 2.0 * cfg.wall, d - 2.0 * cfg.wall, ri));
        }
        profiles.extend([
            (h, w - 5.2, d - 5.2, 1.15),
            (h + 0.7, w - 3.8, d - 3.8, 1.85),
            (h + 2.5, w - 3.8, d - 3.8, 1.85),
            (h + 4.4, w - 0.3, d - 0.3, 3.6),
            (h + 5.0, w - 0.3, d - 0.3, 3.6),
        ]);
    } else {
        profiles.push((h + 0.5, w - 2.0 * cfg.wall, d - 2.0 * cfg.wall, ri));
    }
    loft(&profiles)
}
fn label(
    solid: Manifold,
    text: &str,
    w: f64,
    d: f64,
    h: f64,
    floor: f64,
    engraved: bool,
) -> Result<Manifold> {
    if text.trim().is_empty() {
        return Ok(solid);
    }
    let text = lettering::text(text);
    let n = text.chars().count() as f64;
    let cell = 0.85_f64
        .min(w * 0.84 / (n * 6.0))
        .min((h - floor - 1.5) / 7.0);
    ensure!(
        cell >= 0.4,
        "Beschriftung ist für diese Box zu lang. Text kürzen oder Box vergrößern."
    );
    let width = (n * 6.0 - 1.0) * cell;
    let base = floor + 0.7;
    let mut strokes = Vec::new();
    for (i, c) in text.chars().enumerate() {
        for (row, bits) in lettering::glyph(c).iter().enumerate() {
            for col in 0..5 {
                if bits & (1 << (4 - col)) != 0 {
                    let x = -width / 2.0 + (i as f64 * 6.0 + col as f64 + 0.5) * cell;
                    let z = base + (6 - row) as f64 * cell;
                    strokes.push(block(
                        cell + 0.01,
                        if engraved { 0.55 } else { 0.5 },
                        cell + 0.01,
                        x,
                        -d / 2.0 + if engraved { 0.22 } else { -0.15 },
                        z,
                    ));
                }
            }
        }
    }
    if strokes.is_empty() {
        return Ok(solid);
    }
    let text = Manifold::batch_union(&strokes);
    Ok(if engraved {
        &solid - &text
    } else {
        &solid + &text
    })
}
pub fn tool_dimensions(path: &Path) -> Result<[f64; 2]> {
    ensure!(
        fs::metadata(path)?.len() <= 40_000_000,
        "STL ist größer als 40 MB."
    );
    let mesh = stl_io::read_stl(&mut fs::File::open(path)?)?;
    ensure!(
        !mesh.vertices.is_empty() && mesh.faces.len() <= 500_000,
        "STL hat keine Geometrie oder zu viele Dreiecke."
    );
    let mut low = [f64::INFINITY; 2];
    let mut high = [f64::NEG_INFINITY; 2];
    for v in &mesh.vertices {
        for i in 0..2 {
            let p = v[i] as f64;
            ensure!(p.is_finite(), "STL enthält ungültige Koordinaten.");
            low[i] = low[i].min(p);
            high[i] = high[i].max(p);
        }
    }
    Ok(std::array::from_fn(|i| high[i] - low[i]))
}
fn stl_outline(c: &crate::design::Cutout) -> Result<manifold_csg::CrossSection> {
    let path = c.stl_path.as_ref().context("STL-Datei fehlt")?;
    ensure!(
        fs::metadata(path)?.len() <= 40_000_000,
        "STL ist größer als 40 MB."
    );
    let mesh =
        stl_io::read_stl(&mut fs::File::open(path)?).context("STL konnte nicht gelesen werden")?;
    ensure!(
        !mesh.faces.is_empty() && mesh.faces.len() <= 500_000,
        "STL hat zu viele oder keine Dreiecke."
    );
    let v: Vec<f64> = mesh
        .vertices
        .iter()
        .flat_map(|v| [v[0] as f64, v[1] as f64, v[2] as f64])
        .collect();
    ensure!(
        v.iter().all(|x| x.is_finite()),
        "STL enthält ungültige Koordinaten."
    );
    let t: Vec<u64> = mesh
        .faces
        .iter()
        .flat_map(|f| f.vertices.map(|i| i as u64))
        .collect();
    ensure!(
        t.iter().all(|i| *i < mesh.vertices.len() as u64),
        "STL enthält ungültige Indizes."
    );
    let solid = Manifold::from_mesh_f64(&v, 3, &t)
        .context("Werkzeug-STL muss eine geschlossene Oberfläche haben")?;
    let b = solid.bounding_box().context("STL ist leer")?;
    let low = b.min();
    let high = b.max();
    ensure!(
        high[0] - low[0] > 0.01 && high[1] - low[1] > 0.01,
        "STL-Kontur ist zu schmal."
    );
    let contours = solid
        .translate(-(high[0] + low[0]) / 2.0, -(high[1] + low[1]) / 2.0, 0.0)
        .scale(
            (c.width + c.clearance) / (high[0] - low[0]),
            (c.length + c.clearance) / (high[1] - low[1]),
            1.0,
        )
        .project();
    Ok(manifold_csg::CrossSection::from_polygons(&contours))
}
/// Ends are measured on a divider's centreline. At a T-junction the two side
/// edges meet the host's centre plane, rather than extending by the divider's
/// own thickness and creating a ridge on the host's opposite face.
pub(crate) fn divider_footprint<'a>(
    wall: &Wall,
    others: impl Iterator<Item = &'a Wall>,
) -> [[f64; 2]; 4] {
    let ends = [[wall.x1, wall.y1], [wall.x2, wall.y2]];
    let length = (wall.x2 - wall.x1).hypot(wall.y2 - wall.y1);
    // The plan also calls this while editing, before model validation. An
    // incomplete zero-length gesture has no area and must not produce NaNs.
    if !length.is_finite() || length <= 1e-9 || !wall.thickness.is_finite() || wall.thickness <= 0.0
    {
        let point = if ends[0].iter().all(|value| value.is_finite()) {
            ends[0]
        } else {
            [0.0; 2]
        };
        return [point; 4];
    }
    let direction = [(wall.x2 - wall.x1) / length, (wall.y2 - wall.y1) / length];
    let side = [-direction[1], direction[0]];
    let half = wall.thickness / 2.0;
    let mut caps = ends.map(|p| {
        [-1.0, 1.0].map(|sign| std::array::from_fn(|axis| p[axis] + sign * side[axis] * half))
    });
    let mut nearest = [f64::INFINITY; 2];
    for host in others {
        let host_start = [host.x1, host.y1];
        let host_length = (host.x2 - host.x1).hypot(host.y2 - host.y1);
        if !host_length.is_finite()
            || host_length <= 1e-9
            || !host.thickness.is_finite()
            || host.thickness <= 0.0
        {
            continue;
        }
        let tangent = [
            (host.x2 - host.x1) / host_length,
            (host.y2 - host.y1) / host_length,
        ];
        let normal = [-tangent[1], tangent[0]];
        let incidence = direction[0] * normal[0] + direction[1] * normal[1];
        if incidence.abs() < 1e-6 {
            continue;
        }
        let distance =
            |p: [f64; 2]| (p[0] - host_start[0]) * normal[0] + (p[1] - host_start[1]) * normal[1];
        let host_half = host.thickness / 2.0;
        let cap_reach = half * (side[0] * normal[0] + side[1] * normal[1]).abs();
        for end in 0..2 {
            let end_distance = distance(ends[end]);
            // Only repair a real end contact. Do not close an intentional gap,
            // shorten a crossing, or mistake nearly parallel walls for a T.
            if end_distance.abs() > host_half + 1e-8
                || end_distance.abs() >= nearest[end]
                || distance(ends[1 - end]).abs() <= host_half + cap_reach + 1e-8
            {
                continue;
            }
            let intersection: [f64; 2] = std::array::from_fn(|axis| {
                ends[end][axis] - direction[axis] * end_distance / incidence
            });
            let along = (intersection[0] - host_start[0]) * tangent[0]
                + (intersection[1] - host_start[1]) * tangent[1];
            // Both side edges must land inside the host. End-to-end corners
            // retain their explicit coordinates instead of acquiring a bevel.
            let half_join = half / incidence.abs();
            if along <= half_join + 1e-8 || along >= host_length - half_join - 1e-8 {
                continue;
            }
            caps[end] = [-1.0, 1.0].map(|sign| {
                let p = std::array::from_fn(|axis| ends[end][axis] + sign * side[axis] * half);
                let offset = distance(p) / incidence;
                std::array::from_fn(|axis| p[axis] - direction[axis] * offset)
            });
            nearest[end] = end_distance.abs();
        }
    }
    // Very short dividers between oblique hosts can have overlapping bevels.
    // In that case keep the explicit square ends rather than inverting a face.
    if (0..2).any(|side| {
        (0..2)
            .map(|axis| (caps[1][side][axis] - caps[0][side][axis]) * direction[axis])
            .sum::<f64>()
            <= 1e-6
    }) {
        caps = ends.map(|p| {
            [-1.0, 1.0].map(|sign| std::array::from_fn(|axis| p[axis] + sign * side[axis] * half))
        });
    }
    [caps[0][0], caps[1][0], caps[1][1], caps[0][1]]
}

fn bin(cfg: &Design, insert: bool) -> Result<Manifold> {
    let w = cfg.width();
    let d = cfg.length();
    let h = cfg.height() + if cfg.lip { 4.4 } else { 0.0 };
    let body = rounded(w, d, h - 4.73, 3.75)?.translate(0.0, 0.0, 4.73);
    let mut solid = if cfg.reference_profile {
        reference_body(cfg, insert)?
    } else {
        &(&body + &feet(cfg)?) - &pocket(cfg, insert)?
    };
    if !insert && !cfg.walls.is_empty() {
        let mut walls = Vec::new();
        for (index, wall) in cfg.walls.iter().enumerate() {
            let mut footprint = divider_footprint(
                wall,
                cfg.walls
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != index)
                    .map(|(_, wall)| wall),
            );
            // A small overlap with the outer shell prevents coplanar contact
            // fragments after STL vertex welding. Accept sub-0.025 mm rounding
            // in older saved coordinates; larger gaps, free ends and internal
            // joints stay exact.
            let direction = [wall.x2 - wall.x1, wall.y2 - wall.y1];
            let length = direction[0].hypot(direction[1]);
            let limits = [w / 2.0 - cfg.wall, d / 2.0 - cfg.wall];
            for (point, corners, sign) in [
                ([wall.x1, wall.y1], [0, 3], -1.0),
                ([wall.x2, wall.y2], [1, 2], 1.0),
            ] {
                if (0..2).any(|axis| (point[axis].abs() - limits[axis]).abs() <= 0.025) {
                    for corner in corners {
                        for (axis, direction) in direction.iter().enumerate() {
                            footprint[corner][axis] += sign * direction / length * 0.1;
                        }
                    }
                }
            }
            walls.push(prism(&footprint, wall.height + 0.1).translate(
                0.0,
                0.0,
                cfg.floor_z() - 0.1,
            ));
        }
        // The floor and side-wall contacts form a single body. Thick or
        // diagonal dividers must not extend past the rounded outer footprint.
        let envelope = rounded(w, d, cfg.height(), 3.75)?;
        solid = &solid + &Manifold::batch_union(&walls).intersection(&envelope);
    }
    if insert && !cfg.cutouts.is_empty() {
        let mut cuts = Vec::new();
        let top = cfg.height() - 1.4;
        for c in &cfg.cutouts {
            let w = c.width + c.clearance;
            let d = c.length + c.clearance;
            let cut = match c.shape {
                CutShape::Circle => Manifold::cylinder(c.depth + 0.1, w / 2.0, w / 2.0, 64, false),
                CutShape::Rectangle => rounded(w, d, c.depth + 0.1, 2.0)?,
                CutShape::Hexagon => Manifold::cylinder(
                    c.depth + 0.1,
                    w / 3.0_f64.sqrt(),
                    w / 3.0_f64.sqrt(),
                    6,
                    false,
                ),
                CutShape::Stl => stl_outline(c)?.extrude(c.depth + 0.1),
            };
            cuts.push(
                cut.rotate(0.0, 0.0, c.angle)
                    .translate(c.x, c.y, top - c.depth),
            );
        }
        solid = &solid - &Manifold::batch_union(&cuts);
    }
    if cfg.label_area == LabelArea::Recess && cfg.product != Product::LabelInsert {
        let center_z = 5.75 + cfg.label_height / 2.0;
        let recess = rounded(
            cfg.label_width,
            cfg.label_height,
            cfg.label_depth + 0.05,
            0.8,
        )?
        .rotate(90.0, 0.0, 0.0)
        .translate(0.0, -d / 2.0 + cfg.label_depth, center_z);
        return Ok(&solid - &recess);
    }
    if cfg.label_area == LabelArea::SlideIn {
        // Reconcile coplanar CSG face provenance before welding STL vertices;
        // sub-micron fragments at the frame junctions are not printable detail.
        return Ok((&solid + &label_holder(cfg)).as_original().simplify(0.0001));
    }
    label(
        solid,
        &cfg.label,
        w,
        d,
        cfg.height(),
        cfg.floor_z(),
        cfg.engraved,
    )
}

fn label_holder(cfg: &Design) -> Manifold {
    let front = -cfg.length() / 2.0;
    let clearance = cfg.label_insert_clearance;
    let depth = 2.0 + 2.0 * clearance;
    let outer_y = front - depth;
    let outer_width = cfg.label_width + 2.0 + 2.0 * clearance + 1.6;
    let bottom = cfg.label_insert_bottom() - 0.8;
    let rail_height = cfg.label_height + 0.5;
    // A 45-degree ramp grows from the unchanged foot's top edge. It supports
    // the projecting lower stop without a sudden horizontal overhang.
    let support = prism(
        &[
            [front + 1.0, 4.75],
            [front, 4.75],
            [outer_y, bottom + 0.02],
            [front + 1.0, bottom + 0.02],
        ],
        outer_width,
    )
    .rotate(90.0, 0.0, 90.0)
    .translate(-outer_width / 2.0, 0.0, 0.0);
    let mut pieces = vec![
        support,
        block(
            outer_width,
            depth + 1.0,
            0.8,
            0.0,
            (outer_y + front + 1.0) / 2.0,
            bottom,
        ),
    ];
    for direction in [-1.0, 1.0] {
        let side_x = direction * ((cfg.label_width + 2.0) / 2.0 + clearance + 0.4);
        pieces.push(block(
            0.8,
            depth + 1.0,
            rail_height + 0.02,
            side_x,
            (outer_y + front + 1.0) / 2.0,
            cfg.label_insert_bottom() - 0.02,
        ));
        let retainer_width = 1.0 + clearance + 0.8;
        let lip_x = direction * (cfg.label_width / 2.0 + retainer_width / 2.0);
        pieces.push(block(
            retainer_width,
            0.8,
            rail_height + 0.02,
            lip_x,
            outer_y + 0.4,
            cfg.label_insert_bottom() - 0.02,
        ));
    }
    Manifold::batch_union(&pieces)
}

fn label_insert(cfg: &Design) -> Result<Manifold> {
    let mut solid = block(
        cfg.label_width + 2.0,
        1.2,
        cfg.label_height + 2.0,
        0.0,
        0.0,
        0.0,
    );
    if !cfg.label.trim().is_empty() {
        let text = lettering::text(&cfg.label);
        let count = text.chars().count() as f64;
        let cell = 0.9_f64
            .min(cfg.label_width / (count * 6.0))
            .min(cfg.label_height / 8.0);
        ensure!(
            cell >= 0.4,
            "Text ist für das Einschubschild zu lang. Text kürzen oder Schild verbreitern."
        );
        let text_width = (count * 6.0 - 1.0) * cell;
        let base = 1.0 + (cfg.label_height - 7.0 * cell) / 2.0;
        let surface = -0.6;
        let mut strokes = Vec::new();
        for (i, c) in text.chars().enumerate() {
            for (row, bits) in lettering::glyph(c).iter().enumerate() {
                for col in 0..5 {
                    if bits & (1 << (4 - col)) != 0 {
                        strokes.push(
                            manifold_csg::CrossSection::square(cell + 0.01, cell + 0.01, false)
                                .translate(
                                    -text_width / 2.0 + (i as f64 * 6.0 + col as f64) * cell
                                        - 0.005,
                                    base + (6 - row) as f64 * cell,
                                ),
                        );
                    }
                }
            }
        }
        if !strokes.is_empty() {
            // Merge the glyph outlines in 2D before extruding; overlapping 3D
            // pixel boxes create tiny coplanar slivers in engraved text.
            let text_solid = manifold_csg::CrossSection::batch_union(&strokes)
                .extrude(0.46)
                .rotate(90.0, 0.0, 0.0)
                .translate(0.0, surface + if cfg.engraved { 0.38 } else { 0.08 }, 0.0);
            solid = if cfg.engraved {
                &solid - &text_solid
            } else {
                &solid + &text_solid
            };
        }
    }
    Ok(solid)
}

fn label_insert_placement(cfg: &Design) -> [f64; 3] {
    [
        0.0,
        -cfg.length() / 2.0 - cfg.label_insert_clearance - 0.6,
        cfg.label_insert_bottom(),
    ]
}
fn label_insert_part(cfg: &Design, mounted: bool) -> Result<Part> {
    let placement = if mounted {
        label_insert_placement(cfg).map(|v| v as f32)
    } else {
        [0.0; 3]
    };
    let mut result = part(
        "Einschubschild",
        label_insert(cfg)?,
        placement,
        [118, 210, 224],
    )?;
    ensure!(
        result.report.components == 1,
        "Das Einschubschild ist nicht zusammenhängend."
    );
    result.print_rotation = [-90.0, 0.0, 0.0];
    Ok(result)
}
fn sockets(nx: u32, ny: u32, z: f64) -> Result<Manifold> {
    let one = loft(&[
        (0.0, 36.1, 36.1, 1.05),
        (0.8, 37.7, 37.7, 1.85),
        (2.6, 37.7, 37.7, 1.85),
        (4.75, 42.0, 42.0, 4.0),
        (5.5, 42.0, 42.0, 4.0),
    ])?;
    let mut cuts = Vec::new();
    for x in 0..nx {
        for y in 0..ny {
            cuts.push(one.translate(
                (x as f64 - (nx - 1) as f64 / 2.0) * 42.0,
                (y as f64 - (ny - 1) as f64 / 2.0) * 42.0,
                z,
            ));
        }
    }
    Ok(Manifold::batch_union(&cuts))
}
// Dimensions measured from the reference generator's 42 mm connection system.
// Rust reconstructs the cross-sections locally; no website code is executed.
const PLATE_HEIGHT: f64 = 4.65;
const CLIP_LENGTH: f64 = 19.6;
const CLIP_SLOT_LENGTH: f64 = 20.0;

fn prism(profile: &[[f64; 2]], length: f64) -> Manifold {
    manifold_csg::CrossSection::from_polygons_with_fill_rule(
        &[profile.to_vec()],
        manifold_csg::FillRule::EvenOdd,
    )
    .extrude(length)
}
fn connector_clip(tolerance: f64) -> Manifold {
    // An inverted U with two hooked legs and a rounded 45 degree cap.
    // Positive tolerance increases the inner space; outside stays 4.3 mm wide.
    let mut profile = vec![
        [-2.15, -2.15],
        [-2.15, -3.8],
        [-1.05 - tolerance, -3.8],
        [-1.05 - tolerance, -3.55 - tolerance],
        [-1.45 - tolerance, -3.017 - tolerance],
        [-1.45 - tolerance, -2.45 + tolerance],
        [1.45 + tolerance, -2.45 + tolerance],
        [1.45 + tolerance, -3.017 - tolerance],
        [1.05 + tolerance, -3.55 - tolerance],
        [1.05 + tolerance, -3.8],
        [2.15, -3.8],
        [2.15, -2.15],
    ];
    // Radius 0.3 mm at the cap. The reference fillets the long apex edge.
    let radius = 0.3;
    for i in 0..=16 {
        let angle = (45.0 + i as f64 * 90.0 / 16.0).to_radians();
        profile.push([
            radius * angle.cos(),
            -radius * 2.0_f64.sqrt() + radius * angle.sin(),
        ]);
    }
    prism(&profile, CLIP_LENGTH)
}
fn connector_cut() -> Manifold {
    // Coordinates: inward distance from the plate edge, height above its bed.
    // Leaves a 2 mm rail with a 0.3 mm ramped hook for the clip's legs.
    prism(
        &[
            [-0.01, 6.0],
            [3.0, 6.0],
            [3.0, 0.701],
            [1.0, 0.701],
            [1.0, 1.1],
            [1.3, 1.5],
            [1.3, 2.0],
            [-0.01, 2.0],
        ],
        CLIP_SLOT_LENGTH,
    )
    .rotate(90.0, 0.0, 0.0)
}
fn baseplate_frame(cfg: &Design) -> Result<Manifold> {
    let (nx, ny) = (cfg.grid_x, cfg.grid_y);
    let (w, d) = (nx as f64 * 42.0, ny as f64 * 42.0);
    let floor = if cfg.baseplate_open { 0.0 } else { 1.5 };
    let outer = rounded(w, d, PLATE_HEIGHT + floor, 4.0)?;
    let opening = loft(&[
        (-0.1, w - 5.7, d - 5.7, 1.15),
        (0.0, w - 5.7, d - 5.7, 1.15),
        (0.7, w - 4.3, d - 4.3, 1.85),
        (2.5, w - 4.3, d - 4.3, 1.85),
        (PLATE_HEIGHT, w, d, 4.0),
        (PLATE_HEIGHT + 0.1, w, d, 4.0),
    ])?;
    let mut solid = &outer - &opening.translate(0.0, 0.0, floor);
    // Straight internal rails match the reference's swept grid: only the
    // perimeter has rounded corners, not all four corners of every cell.
    let rail_profile = [
        [0.0, PLATE_HEIGHT],
        [2.15, 2.5],
        [2.15, 0.7],
        [2.85, 0.0],
        [-2.85, 0.0],
        [-2.15, 0.7],
        [-2.15, 2.5],
    ];
    let mut rails = Vec::new();
    for x in 1..nx {
        rails.push(prism(&rail_profile, d).rotate(90.0, 0.0, 0.0).translate(
            x as f64 * 42.0 - w / 2.0,
            d / 2.0,
            floor,
        ));
    }
    for y in 1..ny {
        rails.push(prism(&rail_profile, w).rotate(90.0, 0.0, 90.0).translate(
            -w / 2.0,
            y as f64 * 42.0 - d / 2.0,
            floor,
        ));
    }
    if !rails.is_empty() {
        solid = &solid + &Manifold::batch_union(&rails);
    }
    // The closed alternative has a continuous 1.5 mm floor.
    if !cfg.baseplate_open {
        solid = &solid + &rounded(w, d, floor, 4.0)?;
    }
    if cfg.baseplate_clips {
        let socket = connector_cut();
        let mut connections = Vec::new();
        for y in 0..ny {
            let cy = (y as f64 - (ny - 1) as f64 / 2.0) * 42.0;
            connections.push(socket.translate(-w / 2.0, cy + CLIP_SLOT_LENGTH / 2.0, floor));
            connections.push(socket.rotate(0.0, 0.0, 180.0).translate(
                w / 2.0,
                cy - CLIP_SLOT_LENGTH / 2.0,
                floor,
            ));
        }
        for x in 0..nx {
            let cx = (x as f64 - (nx - 1) as f64 / 2.0) * 42.0;
            connections.push(socket.rotate(0.0, 0.0, 90.0).translate(
                cx - CLIP_SLOT_LENGTH / 2.0,
                -d / 2.0,
                floor,
            ));
            connections.push(socket.rotate(0.0, 0.0, -90.0).translate(
                cx + CLIP_SLOT_LENGTH / 2.0,
                d / 2.0,
                floor,
            ));
        }
        solid = &solid - &Manifold::batch_union(&connections);
    }
    // Trim the ideal knife-edge apex by one micron. This prevents zero-width
    // touching faces after STL float conversion without changing print fits.
    Ok(solid.intersection(&block(
        w + 2.0,
        d + 2.0,
        PLATE_HEIGHT + floor - 0.001,
        0.0,
        0.0,
        0.0,
    )))
}

// Height of the bottom of a seated bin above the print bed. Mounts belong
// below this plane; the unchanged grid profile and its clip slots sit above.
pub fn baseplate_seat_height(cfg: &Design) -> f64 {
    let original_floor: f64 = if cfg.baseplate_open { 0.0 } else { 1.5 };
    let magnet_support = if cfg.magnets {
        cfg.magnet_depth + 1.0
    } else {
        0.0
    };
    let screw_support: f64 = if cfg.screws { 2.6 } else { 0.0 };
    original_floor.max(magnet_support).max(screw_support)
}

fn baseplate(cfg: &Design) -> Result<Manifold> {
    let frame = baseplate_frame(cfg)?;
    if !cfg.magnets && !cfg.screws {
        // Preserve all existing plates, including reference clip dimensions.
        return Ok(frame);
    }
    let (nx, ny) = (cfg.grid_x, cfg.grid_y);
    let (w, d) = (nx as f64 * 42.0, ny as f64 * 42.0);
    let original_floor = if cfg.baseplate_open { 0.0 } else { 1.5 };
    let seat = baseplate_seat_height(cfg);
    let mut supports = vec![frame.translate(0.0, 0.0, seat - original_floor)];
    if cfg.baseplate_open {
        // Extend the narrow rails to the bed, with positive overlap inside the
        // original rail footprint. Cell centres stay completely open.
        supports.push(
            &rounded(w, d, seat + 0.02, 4.0)?
                - &rounded(w - 5.5, d - 5.5, seat + 0.04, 1.25)?.translate(0.0, 0.0, -0.01),
        );
        for ix in 1..nx {
            supports.push(block(
                5.5,
                d - 1.0,
                seat + 0.02,
                ix as f64 * 42.0 - w / 2.0,
                0.0,
                0.0,
            ));
        }
        for iy in 1..ny {
            supports.push(block(
                w - 1.0,
                5.5,
                seat + 0.02,
                0.0,
                iy as f64 * 42.0 - d / 2.0,
                0.0,
            ));
        }
    } else {
        // The optional solid floor remains continuous underneath the pockets.
        supports.push(rounded(w, d, seat - original_floor + 0.02, 4.0)?);
    }
    let mut cuts = Vec::new();
    for ix in 0..nx {
        for iy in 0..ny {
            let cx = (ix as f64 - (nx - 1) as f64 / 2.0) * 42.0;
            let cy = (iy as f64 - (ny - 1) as f64 / 2.0) * 42.0;
            for sx in [-1.0, 1.0] {
                for sy in [-1.0, 1.0] {
                    let corner = (if sx < 0.0 { ix == 0 } else { ix == nx - 1 })
                        && (if sy < 0.0 { iy == 0 } else { iy == ny - 1 });
                    let magnet = cfg.magnets && (!cfg.magnet_corners_only || corner);
                    let screw = cfg.screws && (!cfg.screw_corners_only || corner);
                    for (enabled, is_magnet) in [(magnet, true), (screw, false)] {
                        if !enabled {
                            continue;
                        }
                        // Magnets keep standard 26 mm-spaced positions. On
                        // mixed plates, move screws to the four edge centres
                        // so ordinary screw heads never consume magnet floors.
                        let (px, py) = if !is_magnet && cfg.magnets {
                            if sx == sy {
                                (sx * 13.0, 0.0)
                            } else {
                                (0.0, sy * 13.0)
                            }
                        } else {
                            (sx * 13.0, sy * 13.0)
                        };
                        let (x, y) = (cx + px, cy + py);
                        if cfg.baseplate_open {
                            let radius = if is_magnet {
                                cfg.magnet_diameter / 2.0 + 1.2
                            } else {
                                4.4
                            };
                            supports.push(
                                Manifold::cylinder(seat, radius, radius, 64, false)
                                    .translate(x, y, 0.0),
                            );
                            // Short arms join the nearest grid edges; no arm
                            // spans the open centre of the cell.
                            if px != 0.0 {
                                supports.push(block(7.0, 3.0, seat, x + px.signum() * 3.5, y, 0.0));
                            }
                            if py != 0.0 {
                                supports.push(block(3.0, 7.0, seat, x, y + py.signum() * 3.5, 0.0));
                            }
                        }
                        if is_magnet {
                            cuts.push(
                                Manifold::cylinder(
                                    cfg.magnet_depth + 0.02,
                                    cfg.magnet_diameter / 2.0,
                                    cfg.magnet_diameter / 2.0,
                                    64,
                                    false,
                                )
                                .translate(
                                    x,
                                    y,
                                    seat - cfg.magnet_depth,
                                ),
                            );
                        } else {
                            cuts.push(
                                Manifold::cylinder(seat + 0.04, 1.6, 1.6, 64, false)
                                    .translate(x, y, -0.02),
                            );
                            cuts.push(Manifold::cylinder(1.62, 1.6, 3.22, 64, false).translate(
                                x,
                                y,
                                seat - 1.6,
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(
        (&Manifold::batch_union(&supports) - &Manifold::batch_union(&cuts))
            .as_original()
            .simplify(0.0001),
    )
}
fn drawer(cfg: &Design) -> Result<Manifold> {
    let w = cfg.grid_x as f64 * 42.0 + 4.0;
    let d = cfg.grid_y as f64 * 42.0 + 4.0;
    let h = cfg.drawer_height_u as f64 * 7.0;
    let outer = rounded(w, d, h, 3.0)?;
    let inner = rounded(w - 3.2, d - 3.2, h, 1.4)?.translate(0.0, 0.0, 6.5);
    let solid = &(&outer - &inner) - &sockets(cfg.grid_x, cfg.grid_y, 1.5)?;
    let handle = rounded(24.0, 8.0, 8.0, 2.0)?.translate(0.0, -d / 2.0 - 2.7, 7.0);
    let solid = label(
        &solid + &handle,
        &cfg.label,
        24.0,
        d + 13.4,
        15.0,
        7.0,
        cfg.engraved,
    )?;
    if cfg.printed_push_open {
        Ok(&solid + &push_open::cam(cfg)?)
    } else {
        Ok(solid)
    }
}
fn housing(cfg: &Design) -> Result<Manifold> {
    let drawer_w = cfg.grid_x as f64 * 42.0 + 4.0;
    let drawer_d = cfg.grid_y as f64 * 42.0 + 4.0;
    let inner_w = drawer_w + 2.0 * cfg.drawer_clearance;
    let inner_d = drawer_d + 1.0;
    let w = inner_w + 4.0;
    let d = inner_d + 4.0;
    let pitch = cfg.drawer_height_u as f64 * 7.0 + 3.0;
    let h = pitch * cfg.drawer_count as f64 + 4.0;
    let extra = if cfg.printed_push_open {
        push_open::EXTRA_DEPTH
    } else {
        0.0
    };
    let outer = rounded(w, d + extra, h, 3.0)?.translate(0.0, extra / 2.0, 0.0);
    let cavity = block(
        inner_w,
        inner_d + 4.0 + extra,
        h - 4.0,
        0.0,
        -2.0 + extra / 2.0,
        2.0,
    );
    let mut solid = &outer - &cavity;
    let mut rails = Vec::new();
    for row in 0..cfg.drawer_count {
        let z = 2.0 + row as f64 * pitch;
        for side in [-1.0, 1.0] {
            rails.push(block(
                3.0,
                drawer_d - 0.6,
                1.2,
                side * (inner_w / 2.0 - 0.9),
                0.0,
                z,
            ));
        }
    }
    solid = &solid + &Manifold::batch_union(&rails);
    if cfg.printed_push_open {
        solid = &solid + &push_open::mounts(cfg);
    }
    if cfg.top_grid {
        let top = rounded(w, d + extra, 6.5, 3.0)?.translate(0.0, extra / 2.0, h - 0.05);
        solid = &(&solid + &top) - &sockets(cfg.grid_x, cfg.grid_y, h + 1.45)?;
    }
    Ok(solid)
}

fn part(name: &str, solid: Manifold, placement: [f32; 3], colour: [u8; 3]) -> Result<Part> {
    solid.status().context("CSG-Geometrie ist ungültig")?;
    ensure!(
        !solid.is_empty() && solid.volume() > 0.001,
        "Das Modell ist leer."
    );
    let (v, n, t) = solid.to_mesh_f64();
    // Weld numerical duplicates on a 0.00001 mm lattice before converting to
    // STL floats. Coplanar CSG unions may produce points 1e-11 mm apart.
    let mut vertices = Vec::<[f32; 3]>::new();
    let mut lookup = HashMap::<[i64; 3], usize>::new();
    let remap: Vec<_> = v
        .chunks_exact(n)
        .map(|p| {
            let key = std::array::from_fn(|i| (p[i] * 100_000.0).round() as i64);
            *lookup.entry(key).or_insert_with(|| {
                let index = vertices.len();
                vertices.push(key.map(|c| (c as f64 / 100_000.0) as f32));
                index
            })
        })
        .collect();
    let faces: Vec<[usize; 3]> = t
        .as_chunks::<3>()
        .0
        .iter()
        .map(|f| f.map(|i| remap[i as usize]))
        .filter(|[a, b, c]| a != b && b != c && c != a)
        .collect();
    let mut edges: HashMap<(usize, usize), usize> = HashMap::new();
    for [a, b, c] in &faces {
        for (a, b) in [(*a, *b), (*b, *c), (*c, *a)] {
            *edges.entry((a.min(b), a.max(b))).or_default() += 1;
        }
    }
    let boundary = edges.values().filter(|n| **n == 1).count();
    let nonmanifold = edges.values().filter(|n| **n > 2).count();
    ensure!(
        boundary == 0 && nonmanifold == 0,
        "Das Modell '{name}' hat offene oder nicht-manifolde Kanten (offen: {boundary}, nicht-manifold: {nonmanifold})."
    );
    let b = solid.bounding_box().context("Keine Modellabmessungen")?;
    let dims = std::array::from_fn(|i| b.max()[i] - b.min()[i]);
    let report = Report {
        dimensions: dims,
        triangles: faces.len(),
        volume_cm3: solid.volume() / 1000.0,
        closed: true,
        boundary_edges: boundary,
        nonmanifold_edges: nonmanifold,
        components: solid.decompose().len(),
    };
    Ok(Part {
        name: name.into(),
        vertices,
        faces,
        placement,
        print_rotation: [0.0; 3],
        colour,
        report,
        motion: Motion::Fixed,
    })
}
pub fn generate(cfg: &Design) -> Result<Vec<Part>> {
    cfg.validate()?;
    match cfg.product {
        Product::Bin | Product::Insert => {
            let insert = cfg.product == Product::Insert;
            let body = bin(cfg, insert)?;
            let mut parts = vec![part(
                if insert {
                    "Werkzeugeinsatz"
                } else {
                    "Behälter"
                },
                body.clone(),
                [0.0; 3],
                [45, 167, 216],
            )?];
            if cfg.label_area == LabelArea::SlideIn {
                parts.push(label_insert_part(cfg, true)?);
            }
            Ok(parts)
        }
        Product::LabelInsert => Ok(vec![label_insert_part(cfg, false)?]),
        Product::Baseplate => Ok(vec![part(
            "Grundplatte",
            baseplate(cfg)?,
            [0.0; 3],
            [45, 167, 216],
        )?]),
        Product::Clip => Ok(vec![part(
            "Verbindungsclip",
            connector_clip(cfg.clip_clearance),
            [0.0; 3],
            [45, 167, 216],
        )?]),
        Product::DeskBox => {
            let mut parts = vec![part("Gehäuse", housing(cfg)?, [0.0; 3], [121, 151, 192])?];
            if cfg.printed_push_open {
                parts[0].print_rotation = [-90.0, 0.0, 0.0];
            }
            let pitch = cfg.drawer_height_u as f32 * 7.0 + 3.0;
            for row in 0..cfg.drawer_count {
                let mut tray = part(
                    &format!("Schublade_{}", row + 1),
                    drawer(cfg)?,
                    [0.0, 0.0, 3.2 + row as f32 * pitch],
                    [45, 167, 216],
                )?;
                tray.motion = Motion::Drawer(row);
                parts.push(tray);
                if cfg.printed_push_open {
                    parts.extend(push_open::parts(cfg, row)?);
                }
            }
            Ok(parts)
        }
    }
}
pub(crate) fn print_vertices(part: &Part) -> Result<Vec<[f32; 3]>> {
    ensure!(
        part.print_rotation.iter().all(|a| a.is_finite()),
        "Ungültige Druckausrichtung."
    );
    ensure!(
        !part.vertices.is_empty() && part.vertices.iter().flatten().all(|v| v.is_finite()),
        "Das Druckteil enthält keine gültigen Koordinaten."
    );
    let rotations = part
        .print_rotation
        .map(|angle| angle.to_radians().sin_cos());
    let mut vertices: Vec<_> = part
        .vertices
        .iter()
        .map(|v| {
            if part.print_rotation == [-90.0, 0.0, 0.0] {
                [v[0], v[2], -v[1]]
            } else if part.print_rotation == [180.0, 0.0, 0.0] {
                [v[0], -v[1], -v[2]]
            } else if part.print_rotation == [0.0, 90.0, 0.0] {
                [v[2], v[1], -v[0]]
            } else if part.print_rotation == [0.0; 3] {
                *v
            } else {
                let [mut x, mut y, mut z] = v.map(f64::from);
                let [(sx, cx), (sy, cy), (sz, cz)] = rotations;
                (y, z) = (y * cx - z * sx, y * sx + z * cx);
                (x, z) = (x * cy + z * sy, -x * sy + z * cy);
                (x, y) = (x * cz - y * sz, x * sz + y * cz);
                [x as f32, y as f32, z as f32]
            }
        })
        .collect();
    let min_z = vertices.iter().map(|v| v[2]).fold(f32::INFINITY, f32::min);
    for v in &mut vertices {
        v[2] -= min_z;
    }
    ensure!(
        vertices.iter().flatten().all(|v| v.is_finite()),
        "Ungültige Druckkoordinaten nach Ausrichtung."
    );
    Ok(vertices)
}

/// Bounding dimensions in the exact STL print orientation. Assembly placement
/// is intentionally ignored: each part is printed and checked separately.
pub fn print_dimensions(part: &Part) -> Result<[f64; 3]> {
    let vertices = print_vertices(part)?;
    let mut low = [f64::INFINITY; 3];
    let mut high = [f64::NEG_INFINITY; 3];
    for v in vertices {
        for axis in 0..3 {
            low[axis] = low[axis].min(f64::from(v[axis]));
            high[axis] = high[axis].max(f64::from(v[axis]));
        }
    }
    Ok(std::array::from_fn(|axis| high[axis] - low[axis]))
}

pub fn export_stl(part: &Part, path: &Path) -> Result<()> {
    let print_vertices = print_vertices(part)?;
    let mut file = fs::File::create(path)?;
    stl_io::write_stl(
        &mut file,
        part.faces.iter().map(|face| {
            let vertices = face.map(|i| print_vertices[i]);
            let u: [f32; 3] = std::array::from_fn(|i| vertices[1][i] - vertices[0][i]);
            let v: [f32; 3] = std::array::from_fn(|i| vertices[2][i] - vertices[0][i]);
            let normal = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            let len = normal.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-20);
            stl_io::Triangle {
                normal: stl_io::Vector::new(normal.map(|v| v / len)),
                vertices: vertices.map(stl_io::Vector::new),
            }
        }),
    )?;
    file.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn divider_footprint_handles_unfinished_editor_gestures() {
        let complete = Wall {
            x1: 0.0,
            y1: 0.0,
            x2: 20.0,
            y2: 0.0,
            thickness: 0.8,
            height: 8.0,
        };
        let incomplete = Wall {
            x2: 0.0,
            ..complete.clone()
        };
        assert_eq!(
            divider_footprint(&incomplete, [&complete].into_iter()),
            [[0.0; 2]; 4]
        );
        assert_eq!(
            divider_footprint(&complete, [&incomplete].into_iter()),
            divider_footprint(&complete, std::iter::empty())
        );
        let invalid = Wall {
            x1: f64::NAN,
            ..complete.clone()
        };
        assert!(
            divider_footprint(&invalid, [&complete].into_iter())
                .iter()
                .flatten()
                .all(|value| value.is_finite())
        );
        assert_eq!(
            divider_footprint(&complete, [&invalid].into_iter()),
            divider_footprint(&complete, std::iter::empty())
        );
    }

    #[test]
    fn divider_outer_contacts_accept_old_rounded_coordinates_but_preserve_gaps() {
        for reference_profile in [true, false] {
            for gap in [0.01, 0.024, 0.05] {
                let mut cfg = Design {
                    reference_profile,
                    ..Default::default()
                };
                let limit = cfg.width() / 2.0 - cfg.wall;
                cfg.walls.push(Wall {
                    x1: -limit + gap,
                    y1: 0.0,
                    x2: limit - gap,
                    y2: 0.0,
                    thickness: 0.8,
                    height: 8.0,
                });
                let body = bin(&cfg, false).unwrap();
                for sign in [-1.0, 1.0] {
                    let gauge = block(
                        gap * 0.8,
                        0.6,
                        2.0,
                        sign * (limit - gap / 2.0),
                        0.0,
                        cfg.floor_z() + 2.0,
                    );
                    if gap < 0.025 {
                        assert!(
                            (&gauge - &body).volume() < 1e-7,
                            "rounded outer contact has a slit"
                        );
                    } else {
                        assert!(
                            body.intersection(&gauge).volume() < 1e-7,
                            "intentional outer gap was filled"
                        );
                    }
                }
                let exported = part("Außenanschluss", body, [0.0; 3], [0; 3]).unwrap();
                assert!(exported.report.closed && exported.report.components == 1);
            }
        }
    }

    #[test]
    fn t_junctions_are_flush_with_unequal_widths_oblique_angles_and_saved_offsets() {
        for (host_angle, join_angle, branch_thickness, offset, branch_height) in [
            (0.0_f64, 90.0_f64, 0.8, 0.0, 8.0),
            (0.0, 90.0, 4.0, -0.3, 3.0),
            (0.0, 90.0, 2.4, 0.3, 12.0),
            (0.0, 45.0, 4.0, 0.0, 8.0),
            (37.0, 45.0, 4.0, -0.2, 8.0),
            (37.0, 135.0, 2.4, 0.2, 3.0),
        ] {
            let rotate = |p: [f64; 2]| {
                let (sin, cos) = host_angle.to_radians().sin_cos();
                [p[0] * cos - p[1] * sin, p[0] * sin + p[1] * cos]
            };
            let wall = |a: [f64; 2], b: [f64; 2], thickness, height| {
                let [x1, y1] = rotate(a);
                let [x2, y2] = rotate(b);
                Wall {
                    x1,
                    y1,
                    x2,
                    y2,
                    thickness,
                    height,
                }
            };
            let host = wall([-30.0, 0.0], [30.0, 0.0], 0.8, 10.0);
            let (sin, cos) = join_angle.to_radians().sin_cos();
            let branch = wall(
                [20.0 * cos, offset + 20.0 * sin],
                [0.0, offset],
                branch_thickness,
                branch_height,
            );
            for reverse in [false, true] {
                let mut cfg = Design::default();
                cfg.walls = vec![host.clone(), branch.clone()];
                if reverse {
                    for wall in &mut cfg.walls {
                        std::mem::swap(&mut wall.x1, &mut wall.x2);
                        std::mem::swap(&mut wall.y1, &mut wall.y2);
                    }
                    cfg.walls.reverse();
                }
                cfg.validate().unwrap();
                let body = bin(&cfg, false).unwrap();
                let opposite_face = block(24.0, 4.0, 13.0, 0.0, -2.401, cfg.floor_z() + 0.01)
                    .rotate(0.0, 0.0, host_angle);
                assert!(
                    body.intersection(&opposite_face).volume() < 1e-6,
                    "T protrudes: host={host_angle}, branch={join_angle}, thickness={branch_thickness}, offset={offset}, reverse={reverse}"
                );
                // The branch keeps its requested height and joins with its
                // complete width, including the acute side of an oblique T.
                let branch_direction = [-cos, -sin];
                let branch_side = [-branch_direction[1], branch_direction[0]];
                for side in [-0.45, 0.45] {
                    let x = side * branch_thickness * branch_side[0];
                    let y = side * branch_thickness * branch_side[1] + offset;
                    let along = (0.6 - y) / branch_direction[1];
                    let point = rotate([x + along * branch_direction[0], 0.6]);
                    let gauge = block(
                        0.015,
                        0.015,
                        0.01,
                        point[0],
                        point[1],
                        cfg.floor_z() + branch_height - 0.01,
                    );
                    assert!(
                        (&gauge - &body).volume() < 1e-8,
                        "T has an incomplete angled cap"
                    );
                }
                let exported = part("T-Test", body, [0.0; 3], [0; 3]).unwrap();
                assert!(exported.report.closed && exported.report.components == 1);
            }
        }
    }

    #[test]
    fn divider_crossings_free_ends_and_deliberate_gaps_keep_their_coordinates() {
        let host = Wall {
            x1: -30.0,
            y1: 0.0,
            x2: 30.0,
            y2: 0.0,
            thickness: 0.8,
            height: 10.0,
        };
        let crossing = Wall {
            x1: 0.0,
            y1: -20.0,
            x2: 0.0,
            y2: 20.0,
            thickness: 4.0,
            height: 8.0,
        };
        let mut cfg = Design {
            walls: vec![host.clone(), crossing],
            ..Default::default()
        };
        let body = bin(&cfg, false).unwrap();
        for y in [-19.9, 19.9] {
            let gauge = block(3.8, 0.1, 7.0, 0.0, y, cfg.floor_z() + 0.1);
            assert!(
                (&gauge - &body).volume() < 1e-6,
                "intentional cross was shortened"
            );
        }
        for y in [-20.1, 20.1] {
            assert!(
                body.intersection(&block(4.1, 0.1, 8.0, 0.0, y, cfg.floor_z() + 0.1))
                    .volume()
                    < 1e-6,
                "free end was extended beyond its coordinates"
            );
        }
        // A 0.2 mm intentional gap is not auto-closed by the geometric repair.
        cfg.walls[1].y1 = 0.6;
        let body = bin(&cfg, false).unwrap();
        let gap = block(3.8, 0.18, 7.0, 0.0, 0.5, cfg.floor_z() + 0.1);
        assert!(body.intersection(&gap).volume() < 1e-6);
        let exported = part("Wandtest", body, [0.0; 3], [0; 3]).unwrap();
        assert!(exported.report.closed && exported.report.components == 1);
    }

    #[test]
    fn thick_dividers_stay_inside_rounded_box_and_keep_requested_height() {
        for reference_profile in [true, false] {
            for angle in [0.0_f64, 30.0, 45.0, 90.0] {
                for height in [3.0, 12.0] {
                    let mut cfg = Design {
                        reference_profile,
                        wall: 1.0,
                        ..Default::default()
                    };
                    let max = [cfg.width() / 2.0 - cfg.wall, cfg.length() / 2.0 - cfg.wall];
                    let direction = [angle.to_radians().cos(), angle.to_radians().sin()];
                    let reach = (0..2)
                        .filter(|i| direction[*i].abs() > 1e-8)
                        .map(|i| max[i] / direction[i].abs())
                        .fold(f64::INFINITY, f64::min);
                    let end = direction.map(|d| d * reach);
                    cfg.walls.push(crate::design::Wall {
                        x1: -end[0],
                        y1: -end[1],
                        x2: end[0],
                        y2: end[1],
                        thickness: 4.0,
                        height,
                    });
                    cfg.validate().unwrap();
                    let body = bin(&cfg, false).unwrap();
                    let envelope =
                        rounded(cfg.width(), cfg.length(), cfg.box_top_height() + 1.0, 3.75)
                            .unwrap();
                    assert!(
                        (&body - &envelope).volume() < 1e-6,
                        "divider protrudes at {angle}°"
                    );
                    let top_gauge = block(0.1, 0.1, 0.01, 0.0, 0.0, cfg.floor_z() + height - 0.01);
                    assert!(
                        (&top_gauge - &body).volume() < 1e-6,
                        "divider lost its height"
                    );
                    assert!(
                        body.intersection(&top_gauge.translate(0.0, 0.0, 0.02))
                            .volume()
                            < 1e-6
                    );
                    let part = part("Behälter", body, [0.0; 3], [0; 3]).unwrap();
                    assert!(part.report.closed && part.report.components == 1);
                    assert!((part.report.dimensions[0] - cfg.width()).abs() < 1e-6);
                    assert!((part.report.dimensions[1] - cfg.length()).abs() < 1e-6);
                }
            }
        }
    }
    #[test]
    fn print_dimensions_match_export_and_ignore_assembly_placement() {
        let mut p = part(
            "Druckmaßtest",
            block(20.0, 8.0, 3.0, 4.0, -7.0, 2.0),
            [300.0, -400.0, 800.0],
            [0; 3],
        )
        .unwrap();
        for (index, rotation) in [
            [0.0; 3],
            [-90.0, 0.0, 0.0],
            [180.0, 0.0, 0.0],
            [0.0, 90.0, 0.0],
            [20.0, 35.0, 17.0],
        ]
        .into_iter()
        .enumerate()
        {
            p.print_rotation = rotation;
            let dimensions = print_dimensions(&p).unwrap();
            let path = std::env::temp_dir().join(format!(
                "gridfinity-print-dimensions-{}-{index}.stl",
                std::process::id()
            ));
            export_stl(&p, &path).unwrap();
            let mesh = stl_io::read_stl(&mut fs::File::open(&path).unwrap()).unwrap();
            fs::remove_file(path).unwrap();
            let exported: [f64; 3] = std::array::from_fn(|axis| {
                let low = mesh
                    .vertices
                    .iter()
                    .map(|v| f64::from(v[axis]))
                    .fold(f64::INFINITY, f64::min);
                let high = mesh
                    .vertices
                    .iter()
                    .map(|v| f64::from(v[axis]))
                    .fold(f64::NEG_INFINITY, f64::max);
                high - low
            });
            assert_eq!(dimensions, exported);
            p.placement = [0.0; 3];
            assert_eq!(print_dimensions(&p).unwrap(), dimensions);
        }
        p.print_rotation = [f64::NAN, 0.0, 0.0];
        assert!(print_dimensions(&p).is_err());
        p.print_rotation = [0.0; 3];
        p.vertices.clear();
        assert!(print_dimensions(&p).is_err());
    }
    #[test]
    fn print_dimensions_measure_label_flat_instead_of_mounted() {
        let cfg = Design {
            product: Product::LabelInsert,
            label_width: 40.0,
            label_height: 12.0,
            label: "KABEL".into(),
            ..Default::default()
        };
        let parts = generate(&cfg).unwrap();
        let dimensions = print_dimensions(&parts[0]).unwrap();
        assert!((dimensions[0] - 42.0).abs() < 1e-5);
        assert!((dimensions[1] - 14.0).abs() < 1e-5);
        assert!((dimensions[2] - 1.58).abs() < 1e-5);
    }
    #[test]
    fn recessed_labels_stay_blank_and_preserve_at_least_point_eight_wall() {
        for reference_profile in [true, false] {
            for product in [Product::Bin, Product::Insert] {
                let cfg = Design {
                    product,
                    reference_profile,
                    grid_x: 1,
                    grid_y: 1,
                    label_area: LabelArea::Recess,
                    label: "Unbenutzt: ★".into(),
                    ..Default::default()
                };
                let field = bin(&cfg, product == Product::Insert).unwrap();
                let mut blank = cfg.clone();
                blank.label.clear();
                assert!(
                    (field.volume() - bin(&blank, product == Product::Insert).unwrap().volume())
                        .abs()
                        < 1e-8
                );
                let mut plain = blank.clone();
                plain.label_area = LabelArea::None;
                let base = bin(&plain, product == Product::Insert).unwrap();
                assert!(
                    base.volume() - field.volume()
                        > cfg.label_width * cfg.label_height * cfg.label_depth * 0.9
                );
                let front = -cfg.length() / 2.0;
                let backing = block(
                    cfg.label_width - 2.0,
                    0.799,
                    cfg.label_height - 2.0,
                    0.0,
                    front + cfg.label_depth + 0.4,
                    6.75,
                );
                assert!((backing.volume() - field.intersection(&backing).volume()).abs() < 1e-6);
                let parts = generate(&cfg).unwrap();
                assert_eq!(parts.len(), 1);
                assert!(parts[0].report.closed && parts[0].report.components == 1);
                let too_deep = Design {
                    label_depth: 0.41,
                    ..cfg
                };
                assert!(generate(&too_deep).is_err());
            }
        }
    }
    #[test]
    fn slide_in_holders_preserve_bins_and_clear_the_full_insertion_sweep() {
        for reference_profile in [true, false] {
            for (lip, thin_lip) in [(false, false), (true, false), (true, true)] {
                for wall in [1.0, 4.0] {
                    for product in [Product::Bin, Product::Insert] {
                        let mut cfg = Design {
                            product,
                            reference_profile,
                            lip,
                            thin_lip,
                            wall,
                            grid_x: 1,
                            grid_y: 1,
                            height_u: 2,
                            label_area: LabelArea::SlideIn,
                            ..Default::default()
                        };
                        cfg.label_height = cfg.label_height.min(cfg.label_height_limit());
                        cfg.walls.push(crate::design::Wall {
                            x1: 0.0,
                            x2: 0.0,
                            y1: -cfg.length() / 2.0 + wall,
                            y2: cfg.length() / 2.0 - wall,
                            thickness: 1.2,
                            height: cfg.height() - cfg.floor_z(),
                        });
                        let body = bin(&cfg, product == Product::Insert).unwrap();
                        let mut original = cfg.clone();
                        original.label_area = LabelArea::None;
                        let original_body = bin(&original, product == Product::Insert).unwrap();
                        let difference = &body - &original_body;
                        // All extra material is outside the original interior, above the foot.
                        let interior = block(
                            cfg.width() + 2.0,
                            cfg.length() - wall,
                            cfg.box_top_height() + 1.0,
                            0.0,
                            wall / 2.0,
                            0.0,
                        );
                        assert!(difference.intersection(&interior).volume() < 1e-5);
                        let below_foot = block(
                            cfg.width() + 10.0,
                            cfg.length() + 20.0,
                            4.749,
                            0.0,
                            0.0,
                            0.0,
                        );
                        assert!(difference.intersection(&below_foot).volume() < 1e-6);
                        let card = label_insert(&cfg).unwrap();
                        let [x, y, z] = label_insert_placement(&cfg);
                        let sweep = manifold_csg::CrossSection::from_polygons(&card.project())
                            .extrude(cfg.box_top_height() + cfg.label_height + 10.0)
                            .translate(x, y, z);
                        assert!(
                            body.intersection(&sweep).volume() < 1e-6,
                            "profile={reference_profile},lip={lip},thin={thin_lip},wall={wall},product={product:?}"
                        );
                        let parts = generate(&cfg).unwrap();
                        assert_eq!(parts.len(), 2);
                        assert!(
                            parts
                                .iter()
                                .all(|p| p.report.closed && p.report.components == 1)
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn slide_in_guides_retain_sideways_forward_and_downward_at_size_limits() {
        for reference_profile in [true, false] {
            for wall in [1.0, 4.0] {
                for clearance in [0.15, 0.6] {
                    let mut cfg = Design {
                        grid_x: 1,
                        grid_y: 1,
                        wall,
                        reference_profile,
                        label_area: LabelArea::SlideIn,
                        label_insert_clearance: clearance,
                        ..Default::default()
                    };
                    cfg.label_width = cfg.label_width_limit();
                    cfg.label_height = cfg.label_height_limit();
                    let body = bin(&cfg, false).unwrap();
                    let [x, y, z] = label_insert_placement(&cfg);
                    let card = label_insert(&cfg).unwrap().translate(x, y, z);
                    assert!(body.intersection(&card).volume() < 1e-6);
                    for delta in [
                        [clearance + 0.1, 0.0, 0.0],
                        [-clearance - 0.1, 0.0, 0.0],
                        [0.0, -clearance - 0.1, 0.0],
                        [0.0, clearance + 0.1, 0.0],
                        [0.0, 0.0, -0.1],
                    ] {
                        assert!(
                            body.intersection(&card.translate(delta[0], delta[1], delta[2]))
                                .volume()
                                > 1e-4,
                            "Missing retention at delta={delta:?}"
                        );
                    }
                    assert!(
                        generate(&cfg)
                            .unwrap()
                            .iter()
                            .all(|p| p.report.closed && p.report.components == 1)
                    );
                }
            }
        }
    }
    #[test]
    fn german_labels_export_closed_in_both_text_paths() {
        for product in [Product::Bin, Product::LabelInsert] {
            for engraved in [false, true] {
                let mut cfg = Design {
                    product,
                    label: "ÄÖÜß".into(),
                    engraved,
                    ..Default::default()
                };
                let parts = generate(&cfg).unwrap();
                assert_eq!(parts.len(), 1);
                let part = &parts[0];
                assert!(part.report.closed && part.report.components == 1);
                cfg.label = "A\u{0308}O\u{0308}U\u{0308}ẞ".into();
                let decomposed = generate(&cfg).unwrap();
                assert_eq!(part.vertices, decomposed[0].vertices);
                assert_eq!(part.faces, decomposed[0].faces);
                let path = std::env::temp_dir().join(format!(
                    "gridfinity-german-label-{}-{}-{engraved}.stl",
                    std::process::id(),
                    part.name
                ));
                export_stl(part, &path).unwrap();
                let mesh = stl_io::read_stl(&mut fs::File::open(&path).unwrap()).unwrap();
                fs::remove_file(path).unwrap();
                assert!(
                    mesh.validate().is_ok(),
                    "{product:?}/{engraved}: {:?}",
                    mesh.validate()
                );
            }
        }
    }
    #[test]
    fn slide_in_labels_export_flat_back_down_with_legible_text_up() {
        for (text, engraved) in [("", false), ("KABEL", false), ("KABEL", true)] {
            let cfg = Design {
                product: Product::LabelInsert,
                label: text.into(),
                engraved,
                label_width: 40.0,
                label_height: 12.0,
                ..Default::default()
            };
            let parts = generate(&cfg).unwrap();
            assert_eq!(parts.len(), 1);
            let card = &parts[0];
            assert_eq!(card.name, "Einschubschild");
            assert_eq!(card.placement, [0.0; 3]);
            assert_eq!(card.print_rotation, [-90.0, 0.0, 0.0]);
            let path = std::env::temp_dir().join(format!(
                "gridfinity-insert-label-{}-{text}-{engraved}.stl",
                std::process::id()
            ));
            export_stl(card, &path).unwrap();
            let mesh = stl_io::read_stl(&mut fs::File::open(&path).unwrap()).unwrap();
            fs::remove_file(path).unwrap();
            assert!(
                mesh.validate().is_ok(),
                "{text}/{engraved}: {:?}",
                mesh.validate()
            );
            let lo = mesh
                .vertices
                .iter()
                .map(|v| v[2])
                .fold(f32::INFINITY, f32::min);
            let hi = mesh
                .vertices
                .iter()
                .map(|v| v[2])
                .fold(f32::NEG_INFINITY, f32::max);
            assert_eq!(lo, 0.0);
            assert!(
                (hi - if !text.is_empty() && !engraved {
                    1.58
                } else {
                    1.2
                })
                .abs()
                    < 1e-5
            );
        }
        let cfg = Design::from_json(include_bytes!(
            "../examples/label-slide-in-box.gridfinity.json"
        ))
        .unwrap();
        let parts = generate(&cfg).unwrap();
        assert_eq!(parts.len(), 2);
        let body = bin(&cfg, false).unwrap();
        let [x, y, z] = label_insert_placement(&cfg);
        let raised_sweep =
            manifold_csg::CrossSection::from_polygons(&label_insert(&cfg).unwrap().project())
                .extrude(cfg.box_top_height() + cfg.label_height + 10.0)
                .translate(x, y, z);
        assert!(body.intersection(&raised_sweep).volume() < 1e-6);
        for part in parts {
            let path = std::env::temp_dir().join(format!(
                "gridfinity-slide-example-{}-{}.stl",
                std::process::id(),
                part.name
            ));
            export_stl(&part, &path).unwrap();
            let mesh = stl_io::read_stl(&mut fs::File::open(&path).unwrap()).unwrap();
            fs::remove_file(path).unwrap();
            assert!(
                mesh.validate().is_ok(),
                "{}: {:?}",
                part.name,
                mesh.validate()
            );
        }
    }
    #[test]
    fn reference_box_variants_export_closed_and_stack() {
        for (nx, ny, height) in [(1, 1, 2), (2, 3, 3), (4, 2, 10)] {
            for (lip, thin_lip) in [(true, false), (true, true), (false, false)] {
                let cfg = Design {
                    grid_x: nx,
                    grid_y: ny,
                    height_u: height,
                    lip,
                    thin_lip,
                    magnets: true,
                    screws: true,
                    magnet_corners_only: true,
                    screw_corners_only: true,
                    ..Default::default()
                };
                let parts = generate(&cfg).unwrap();
                let p = &parts[0];
                assert!(p.report.closed && p.report.components == 1);
                let path = std::env::temp_dir().join(format!(
                    "gridfinity-reference-{}-{nx}-{ny}-{height}-{lip}-{thin_lip}.stl",
                    std::process::id()
                ));
                export_stl(p, &path).unwrap();
                let mesh = stl_io::read_stl(&mut fs::File::open(&path).unwrap()).unwrap();
                fs::remove_file(path).unwrap();
                assert!(
                    mesh.validate().is_ok(),
                    "{nx}x{ny} lip={lip} thin={thin_lip}: {:?}",
                    mesh.validate()
                );
                if lip {
                    let body = bin(&cfg, false).unwrap();
                    assert!(
                        body.intersection(&body.translate(0.0, 0.0, cfg.stacking_pitch()))
                            .volume()
                            < 1e-5
                    );
                }
            }
        }
    }
    #[test]
    fn baseplate_mounts_keep_floors_open_and_clear_feet_and_clips() {
        for (magnets, screws) in [(true, false), (false, true), (true, true)] {
            for open in [false, true] {
                for corners in [false, true] {
                    let cfg = Design {
                        product: Product::Baseplate,
                        grid_x: 3,
                        grid_y: 2,
                        magnets,
                        screws,
                        magnet_corners_only: corners,
                        screw_corners_only: corners,
                        baseplate_open: open,
                        baseplate_clips: true,
                        ..Default::default()
                    };
                    let solid = baseplate(&cfg).unwrap();
                    let seat = baseplate_seat_height(&cfg);
                    let p = part("Grundplatte", solid.clone(), [0.0; 3], [0; 3]).unwrap();
                    assert!(p.report.closed && p.report.components == 1);
                    assert!((p.report.dimensions[2] - seat - PLATE_HEIGHT + 0.001).abs() < 1e-6);
                    for cx in [-42.0, 0.0, 42.0] {
                        for cy in [-21.0, 21.0] {
                            assert!(
                                solid
                                    .intersection(&foot().unwrap().translate(cx, cy, seat))
                                    .volume()
                                    < 1e-6
                            );
                            let centre = block(12.0, 12.0, seat + 5.0, cx, cy, -0.01);
                            assert_eq!(solid.intersection(&centre).volume() < 1e-6, open);
                        }
                    }
                    // Both edge orientations, including a mixed screw pad
                    // directly below the separate clip's retaining legs.
                    let across_x = connector_clip(0.0).rotate(90.0, 0.0, 0.0).translate(
                        63.0,
                        21.0 + CLIP_LENGTH / 2.0,
                        seat + PLATE_HEIGHT,
                    );
                    let across_y = connector_clip(0.0).rotate(90.0, 0.0, 90.0).translate(
                        -CLIP_LENGTH / 2.0,
                        42.0,
                        seat + PLATE_HEIGHT,
                    );
                    assert!(solid.intersection(&across_x).volume() < 1e-6);
                    assert!(solid.intersection(&across_y).volume() < 1e-6);
                    let path = std::env::temp_dir().join(format!(
                        "gridfinity-mounts-{}-{magnets}-{screws}-{open}-{corners}.stl",
                        std::process::id()
                    ));
                    export_stl(&p, &path).unwrap();
                    let mesh = stl_io::read_stl(&mut fs::File::open(&path).unwrap()).unwrap();
                    fs::remove_file(path).unwrap();
                    assert!(
                        mesh.validate().is_ok(),
                        "{magnets}/{screws}/{open}/{corners}: {:?}",
                        mesh.validate()
                    );
                }
            }
        }
    }
    #[test]
    fn baseplate_mounts_have_sized_pockets_retain_floors_and_standard_countersinks() {
        for (diameter, depth) in [(5.0, 1.0), (6.2, 2.4), (8.0, 3.0)] {
            let cfg = Design {
                product: Product::Baseplate,
                grid_x: 1,
                grid_y: 1,
                magnets: true,
                screws: true,
                magnet_diameter: diameter,
                magnet_depth: depth,
                ..Default::default()
            };
            let plate = baseplate(&cfg).unwrap();
            let seat = baseplate_seat_height(&cfg);
            let floor_height = seat - depth;
            assert!(floor_height >= 1.0);
            for x in [-13.0, 13.0] {
                for y in [-13.0, 13.0] {
                    let pocket = Manifold::cylinder(
                        depth - 0.01,
                        diameter / 2.0 - 0.01,
                        diameter / 2.0 - 0.01,
                        64,
                        false,
                    )
                    .translate(x, y, floor_height + 0.01);
                    assert!(plate.intersection(&pocket).volume() < 1e-6);
                    let floor = Manifold::cylinder(
                        floor_height - 0.01,
                        diameter / 2.0,
                        diameter / 2.0,
                        64,
                        false,
                    )
                    .translate(x, y, 0.0);
                    assert!(
                        (&floor - &plate).volume() < 1e-6,
                        "magnet has no continuous retaining floor"
                    );
                    let wall_gauge = block(
                        0.05,
                        0.05,
                        depth - 0.1,
                        x + diameter / 2.0 + 0.05,
                        y,
                        floor_height + 0.05,
                    );
                    assert!(
                        (&wall_gauge - &plate).volume() < 1e-6,
                        "magnet pocket oversized"
                    );
                }
            }
            for (x, y) in [(-13.0, 0.0), (13.0, 0.0), (0.0, -13.0), (0.0, 13.0)] {
                let through =
                    Manifold::cylinder(seat + 0.02, 1.59, 1.59, 64, false).translate(x, y, -0.01);
                assert!(plate.intersection(&through).volume() < 1e-6);
                let head =
                    Manifold::cylinder(1.6, 1.59, 3.19, 64, false).translate(x, y, seat - 1.6);
                assert!(plate.intersection(&head).volume() < 1e-6);
                let wall_gauge = block(0.05, 0.05, 0.5, x + 1.65, y, 0.1);
                assert!(
                    (&wall_gauge - &plate).volume() < 1e-6,
                    "screw passage oversized"
                );
            }
            let p = part("Grundplatte", plate, [0.0; 3], [0; 3]).unwrap();
            assert!(p.report.closed && p.report.components == 1);
        }
    }
    #[test]
    fn baseplate_corner_mounts_select_four_global_positions_and_disabled_is_unchanged() {
        let plain = Design {
            product: Product::Baseplate,
            grid_x: 3,
            grid_y: 2,
            ..Default::default()
        };
        let original = baseplate_frame(&plain).unwrap();
        let unchanged = baseplate(&plain).unwrap();
        assert_eq!(original.to_mesh_f64(), unchanged.to_mesh_f64());
        for (magnet, screws) in [(true, false), (false, true), (true, true)] {
            let cfg = Design {
                magnets: magnet,
                screws,
                magnet_corners_only: true,
                screw_corners_only: true,
                ..plain.clone()
            };
            let plate = baseplate(&cfg).unwrap();
            let mut count = 0;
            for cx in [-42.0, 0.0, 42.0] {
                for cy in [-21.0, 21.0] {
                    for x in [-13.0_f64, 13.0] {
                        for y in [-13.0_f64, 13.0] {
                            // Sample the solid pad bottom, away from a screw's
                            // centre bore and each pad's connecting arms.
                            let gauge = block(
                                0.1,
                                0.1,
                                0.1,
                                cx + x - x.signum() * 2.7,
                                cy + y - y.signum() * 2.7,
                                0.1,
                            );
                            if plate.intersection(&gauge).volume() > 0.0009 {
                                count += 1;
                            }
                        }
                    }
                }
            }
            assert_eq!(count, 4);
            if magnet && screws {
                let mut screw_count = 0;
                for cx in [-42.0, 0.0, 42.0] {
                    for cy in [-21.0, 21.0] {
                        for (x, y) in [(-13.0, 0.0), (13.0, 0.0), (0.0, -13.0), (0.0, 13.0)] {
                            let gauge = block(
                                0.1,
                                0.1,
                                0.1,
                                cx + x + if x == 0.0 { 3.7 } else { 0.0 },
                                cy + y + if y == 0.0 { 3.7 } else { 0.0 },
                                0.1,
                            );
                            if plate.intersection(&gauge).volume() > 0.0009 {
                                screw_count += 1;
                            }
                        }
                    }
                }
                assert_eq!(
                    screw_count, 4,
                    "mixed plates must retain four separate outer screws"
                );
            }
        }
    }
    #[test]
    fn plate_exports_are_closed_for_small_large_and_solid_variants() {
        for (nx, ny) in [(1, 1), (3, 2), (8, 8)] {
            for open in [true, false] {
                let cfg = Design {
                    product: Product::Baseplate,
                    grid_x: nx,
                    grid_y: ny,
                    baseplate_open: open,
                    baseplate_clips: true,
                    ..Default::default()
                };
                let parts = generate(&cfg).unwrap();
                assert_eq!(parts.len(), 1, "Platten erzeugen keine Clips mit");
                for p in parts {
                    let path = std::env::temp_dir().join(format!(
                        "gridfinity-roundtrip-{}-{nx}-{ny}-{open}-{}.stl",
                        std::process::id(),
                        p.name
                    ));
                    export_stl(&p, &path).unwrap();
                    let mesh = stl_io::read_stl(&mut fs::File::open(&path).unwrap()).unwrap();
                    fs::remove_file(path).unwrap();
                    assert!(
                        mesh.validate().is_ok(),
                        "{nx}x{ny}, open={open}, {}: {:?}",
                        p.name,
                        mesh.validate()
                    );
                    assert_eq!(p.report.components, 1);
                }
            }
        }
    }
    #[test]
    #[ignore = "requires reference-site STL exports in reviews/reference-site"]
    fn reference_generator_interoperability() {
        fn read_solid(file: &str) -> Manifold {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("reviews/reference-site")
                .join(file);
            let mesh = stl_io::read_stl(&mut fs::File::open(path).unwrap()).unwrap();
            let vertices: Vec<_> = mesh
                .vertices
                .iter()
                .flat_map(|v| v.0.map(f64::from))
                .collect();
            let triangles: Vec<_> = mesh
                .faces
                .iter()
                .flat_map(|f| f.vertices.map(|i| i as u64))
                .collect();
            Manifold::from_mesh_f64(&vertices, 3, &triangles).unwrap()
        }
        let reference_plate = read_solid("baseplate-2x2-connectable.stl");
        let reference_clip = read_solid("connection-clip.stl");
        let reference_box = read_solid("box-2-2-3.stl");
        let our_box = bin(&Design::default(), false).unwrap();
        let bounds = reference_box.bounding_box().unwrap();
        println!(
            "uploaded box: dimensions={:?}, volume={}, components={}",
            std::array::from_fn::<_, 3, _>(|i| bounds.max()[i] - bounds.min()[i]),
            reference_box.volume(),
            reference_box.decompose().len()
        );
        let box_difference =
            (&reference_box - &our_box).volume() + (&our_box - &reference_box).volume();
        println!(
            "our reference box: volume={}, symmetric difference={box_difference}",
            our_box.volume()
        );
        for z in [1.0, 5.5, 6.0, 20.0, 21.0, 23.0, 25.8] {
            println!(
                "slice z={z}: reference contours={}, own contours={}",
                reference_box.slice_at_z(z).len(),
                our_box.slice_at_z(z).len()
            );
        }
        assert!(box_difference / reference_box.volume() < 0.001);
        for pitch in [21.0, 21.245, 21.265, 21.3] {
            println!(
                "reference stack at {pitch}: {}",
                our_box
                    .intersection(&our_box.translate(0.0, 0.0, pitch))
                    .volume()
            );
        }
        let cfg = Design {
            product: Product::Baseplate,
            baseplate_clips: true,
            ..Default::default()
        };
        let ours = baseplate(&cfg).unwrap();
        let our_clip = connector_clip(0.0);
        let clip_error =
            (&our_clip - &reference_clip).volume() + (&reference_clip - &our_clip).volume();
        let plate_error = (&ours - &reference_plate).volume() + (&reference_plate - &ours).volume();
        println!(
            "reference clip volume={}, our volume={}, symmetric difference={clip_error}",
            reference_clip.volume(),
            our_clip.volume()
        );
        println!(
            "reference plate volume={}, our volume={}, symmetric difference={plate_error}",
            reference_plate.volume(),
            ours.volume()
        );
        assert!(clip_error / reference_clip.volume() < 0.005);
        assert!(plate_error / reference_plate.volume() < 0.001);
        for (plate, clip) in [(&reference_plate, &our_clip), (&ours, &reference_clip)] {
            let mounted =
                clip.rotate(90.0, 0.0, 0.0)
                    .translate(42.0, 21.0 + CLIP_LENGTH / 2.0, PLATE_HEIGHT);
            let interference = plate.intersection(&mounted).volume();
            let next_interference = plate
                .translate(84.0, 0.0, 0.0)
                .intersection(&mounted)
                .volume();
            println!("cross fit: intersection={interference}, next={next_interference}");
            assert!(interference < 0.001 && next_interference < 0.001);
        }
    }
    #[test]
    fn open_baseplates_keep_grid_and_remove_floor() {
        for (nx, ny) in [(1, 1), (2, 3), (8, 8)] {
            let cfg = Design {
                product: Product::Baseplate,
                grid_x: nx,
                grid_y: ny,
                ..Default::default()
            };
            let open = baseplate(&cfg).unwrap();
            let closed = baseplate(&Design {
                baseplate_open: false,
                ..cfg.clone()
            })
            .unwrap();
            assert!(open.volume() < closed.volume());
            for x in 0..nx {
                for y in 0..ny {
                    let cx = (x as f64 - (nx - 1) as f64 / 2.0) * 42.0;
                    let cy = (y as f64 - (ny - 1) as f64 / 2.0) * 42.0;
                    assert!(
                        open.intersection(&block(8.0, 8.0, 8.0, cx, cy, -0.1))
                            .volume()
                            < 1e-6
                    );
                    assert!(
                        open.intersection(&foot().unwrap().translate(cx, cy, 0.0))
                            .volume()
                            < 1e-6
                    );
                }
            }
            let parts = generate(&cfg).unwrap();
            assert!(parts[0].report.closed);
            assert_eq!(parts[0].report.components, 1);
            assert!((parts[0].report.dimensions[0] - nx as f64 * 42.0).abs() < 1e-5);
        }
    }
    #[test]
    fn clips_fit_adjacent_plates_and_have_retention() {
        for clearance in [0.0, 0.1, 0.2] {
            let cfg = Design {
                product: Product::Baseplate,
                grid_x: 1,
                grid_y: 1,
                baseplate_clips: true,
                clip_clearance: clearance,
                ..Default::default()
            };
            let a = baseplate(&cfg).unwrap();
            let b = a.translate(42.0, 0.0, 0.0);
            let mounted = connector_clip(clearance).rotate(90.0, 0.0, 0.0).translate(
                21.0,
                CLIP_LENGTH / 2.0,
                PLATE_HEIGHT,
            );
            let clip = mounted.clone();
            assert!(clip.volume() > 1.0, "empty clip: {}", clip.volume());
            assert!(a.volume() > 1.0);
            assert!(a.intersection(&b).volume() < 1e-6);
            assert!(a.intersection(&clip).volume() < 1e-6);
            assert!(b.intersection(&clip).volume() < 1e-6);
            let across_y = connector_clip(clearance)
                .rotate(90.0, 0.0, 0.0)
                .rotate(0.0, 0.0, 90.0)
                .translate(-CLIP_LENGTH / 2.0, 21.0, PLATE_HEIGHT);
            assert!(a.intersection(&across_y).volume() < 1e-6);
            assert!(a.translate(0.0, 42.0, 0.0).intersection(&across_y).volume() < 1e-6);
            // Pulling straight up meets the rail's retaining hooks.
            let retention = (1..=10)
                .map(|i| {
                    a.intersection(&clip.translate(0.0, 0.0, i as f64 * 0.1))
                        .volume()
                })
                .fold(0.0, f64::max);
            assert!(
                retention > 0.001,
                "no hook engagement at tolerance {clearance}: {retention}"
            );
            let parts = generate(&cfg).unwrap();
            assert_eq!(parts.len(), 1);
            assert!(
                parts
                    .iter()
                    .all(|p| p.report.closed && p.report.components == 1)
            );
        }
        let large = Design {
            product: Product::Baseplate,
            grid_x: 3,
            grid_y: 2,
            baseplate_clips: true,
            ..Default::default()
        };
        assert!(
            generate(&large)
                .unwrap()
                .iter()
                .all(|p| p.report.components == 1)
        );
    }
    #[test]
    fn standalone_clips_export_independently_with_valid_tolerance() {
        for clearance in [-0.1, 0.0, 0.2] {
            let cfg = Design {
                product: Product::Clip,
                clip_clearance: clearance,
                baseplate_clips: false,
                ..Default::default()
            };
            let roundtrip: Design =
                serde_json::from_str(&serde_json::to_string(&cfg).unwrap()).unwrap();
            assert_eq!(roundtrip.product, Product::Clip);
            let parts = generate(&roundtrip).unwrap();
            assert_eq!(parts.len(), 1);
            let clip = &parts[0];
            assert_eq!(clip.name, "Verbindungsclip");
            assert!(clip.report.closed && clip.report.components == 1);
            assert_eq!(clip.placement, [0.0; 3]);
            assert!((clip.report.dimensions[0] - 4.3).abs() < 1e-5);
            assert!((clip.report.dimensions[2] - CLIP_LENGTH).abs() < 1e-5);
            let path = std::env::temp_dir().join(format!(
                "gridfinity-clip-{}-{clearance}.stl",
                std::process::id()
            ));
            export_stl(clip, &path).unwrap();
            let mesh = stl_io::read_stl(&mut fs::File::open(&path).unwrap()).unwrap();
            fs::remove_file(path).unwrap();
            assert!(mesh.validate().is_ok());
        }
        let invalid = Design {
            product: Product::Clip,
            clip_clearance: 0.3,
            ..Default::default()
        };
        assert!(generate(&invalid).is_err());
    }
    #[test]
    fn imported_stl_creates_closed_tool_insert() {
        let path = std::env::temp_dir().join(format!("gridfinity-tool-{}.stl", std::process::id()));
        let tool = part(
            "test-tool",
            Manifold::cube(12.0, 20.0, 8.0, false),
            [0.0; 3],
            [255; 3],
        )
        .unwrap();
        export_stl(&tool, &path).unwrap();
        assert_eq!(tool_dimensions(&path).unwrap(), [12.0, 20.0]);
        let mut cfg = Design {
            product: Product::Insert,
            ..Default::default()
        };
        cfg.cutouts.push(crate::design::Cutout {
            shape: CutShape::Stl,
            width: 12.0,
            length: 20.0,
            depth: 8.0,
            angle: 30.0,
            stl_path: Some(path.to_string_lossy().into_owned()),
            ..Default::default()
        });
        let imported = generate(&cfg).unwrap();
        let empty = generate(&Design {
            product: Product::Insert,
            ..Default::default()
        })
        .unwrap();
        fs::remove_file(path).unwrap();
        assert!(imported[0].report.closed);
        assert_eq!(imported[0].report.components, 1);
        assert!(
            (empty[0].report.volume_cm3
                - imported[0].report.volume_cm3
                - 12.3 * 20.3 * 8.0 / 1000.0)
                .abs()
                < 0.01
        );
    }
    #[test]
    fn standard_bin_dimensions_and_closed_surface() {
        let p = generate(&Design::default()).unwrap();
        assert!(p[0].report.closed);
        assert!((p[0].report.dimensions[0] - 83.38).abs() < 1e-5);
        assert!((p[0].report.dimensions[2] - 25.852579).abs() < 0.001);
        assert_eq!(p[0].report.components, 1);
    }
    #[test]
    fn stacked_bins_have_no_intersection() {
        let c = Design::default();
        let a = bin(&c, false).unwrap();
        let b = a.translate(0.0, 0.0, c.stacking_pitch());
        assert!(a.intersection(&b).volume() < 1e-5);
    }
    #[test]
    fn custom_walls_holes_and_letters_are_connected() {
        let mut c = Design {
            label: "BITS".into(),
            magnets: true,
            ..Default::default()
        };
        c.walls.push(crate::design::Wall {
            x1: -40.15,
            y1: 0.0,
            x2: 40.15,
            y2: 0.0,
            thickness: 1.2,
            height: 10.0,
        });
        assert_eq!(generate(&c).unwrap()[0].report.components, 1);
        c.product = Product::Insert;
        c.walls.clear();
        c.cutouts.push(crate::design::Cutout {
            depth: 8.0,
            ..Default::default()
        });
        assert_eq!(generate(&c).unwrap()[0].report.components, 1);
    }
    #[test]
    fn drawers_clear_housing() {
        let c = Design {
            product: Product::DeskBox,
            ..Default::default()
        };
        let h = housing(&c).unwrap();
        let d = drawer(&c).unwrap();
        for row in 0..c.drawer_count {
            let placed = d.translate(
                0.0,
                0.0,
                3.2 + row as f64 * (c.drawer_height_u as f64 * 7.0 + 3.0),
            );
            assert!(
                h.intersection(&placed).volume() < 1e-5,
                "drawer collides with frame"
            );
        }
        assert_eq!(generate(&c).unwrap().len(), 3);
    }
    #[test]
    fn baseplate_accepts_standard_foot() {
        let plate = baseplate(&Design {
            grid_x: 1,
            grid_y: 1,
            ..Default::default()
        })
        .unwrap();
        let placed = foot().unwrap();
        assert!(plate.intersection(&placed).volume() < 1e-6);
    }
    #[test]
    fn exported_stl_roundtrip_is_closed_and_has_correct_size() {
        let cfg = Design {
            product: Product::Baseplate,
            ..Default::default()
        };
        let parts = generate(&cfg).unwrap();
        let path = std::env::temp_dir().join(format!("gridfinity-test-{}.stl", std::process::id()));
        export_stl(&parts[0], &path).unwrap();
        let mesh = stl_io::read_stl(&mut fs::File::open(&path).unwrap()).unwrap();
        fs::remove_file(path).unwrap();
        assert_eq!(mesh.faces.len(), parts[0].faces.len());
        assert!(
            mesh.validate().is_ok(),
            "STL validation: {:?}",
            mesh.validate()
        );
        let low = mesh
            .vertices
            .iter()
            .map(|v| v[0])
            .fold(f32::INFINITY, f32::min);
        let high = mesh
            .vertices
            .iter()
            .map(|v| v[0])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((high - low - 84.0).abs() < 0.001);
    }
}
