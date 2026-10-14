use crate::design::{Design, Wall};

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Mode {
    #[default]
    None,
    Grid,
    Free,
}

pub struct Editor {
    pub mode: Mode,
    pub rows: usize,
    pub cols: usize,
    pub thickness: f64,
    pub height: f64,
    pub undo: Vec<Vec<Wall>>,
    pub redo: Vec<Vec<Wall>>,
}
impl Default for Editor {
    fn default() -> Self {
        Self {
            mode: Mode::None,
            rows: 2,
            cols: 2,
            thickness: 0.8,
            height: 10.45,
            undo: vec![],
            redo: vec![],
        }
    }
}
impl Editor {
    pub fn from_design(cfg: &Design) -> Self {
        // Standalone parts intentionally allow unused legacy box fields. They
        // do not have an editable wall layout, so never interpret those fields.
        if cfg.product != crate::design::Product::Bin {
            return Self::default();
        }
        let mut editor = Self {
            mode: if cfg.walls.is_empty() {
                Mode::None
            } else {
                Mode::Free
            },
            ..Self::default()
        };
        if let Some(first) = cfg.walls.first() {
            let vertical = cfg.walls.iter().filter(|w| w.x1 == w.x2).count();
            let horizontal = cfg.walls.iter().filter(|w| w.y1 == w.y2).count();
            if vertical + horizontal == cfg.walls.len() && vertical < 8 && horizontal < 8 {
                editor.cols = vertical + 1;
                editor.rows = horizontal + 1;
                editor.thickness = first.thickness;
                editor.height = first.height;
                let mut candidate = cfg.clone();
                let mut grid = Self {
                    rows: editor.rows,
                    cols: editor.cols,
                    thickness: editor.thickness,
                    height: editor.height,
                    ..Self::default()
                };
                grid.grid(&mut candidate);
                if candidate.walls.iter().zip(&cfg.walls).all(|(a, b)| {
                    [
                        a.x1 - b.x1,
                        a.y1 - b.y1,
                        a.x2 - b.x2,
                        a.y2 - b.y2,
                        a.thickness - b.thickness,
                        a.height - b.height,
                    ]
                    .iter()
                    .all(|v| v.abs() < 1e-6)
                }) {
                    editor.mode = Mode::Grid;
                }
            }
        }
        editor
    }
    pub fn checkpoint(&mut self, walls: &[Wall]) {
        if self.undo.last().is_none_or(|last| last != walls) {
            if self.undo.len() == 60 {
                self.undo.remove(0);
            }
            self.undo.push(walls.to_vec());
        }
        self.redo.clear();
    }
    pub fn clear(&mut self, cfg: &mut Design) {
        if !cfg.walls.is_empty() {
            self.checkpoint(&cfg.walls);
            cfg.walls.clear();
        }
        self.mode = Mode::None;
    }
    pub fn grid(&mut self, cfg: &mut Design) {
        let max = bounds(cfg);
        let height = self.height.clamp(2.0, cfg.height() - cfg.floor_z());
        let mut walls = vec![];
        for i in 1..self.cols {
            // Divide the free interior after subtracting every divider's
            // thickness. Equal centre spacing makes edge compartments wider.
            let clear = (2.0 * max[0] - (self.cols - 1) as f64 * self.thickness) / self.cols as f64;
            let x = -max[0] + i as f64 * clear + (i as f64 - 0.5) * self.thickness;
            walls.push(Wall {
                x1: x,
                y1: -max[1],
                x2: x,
                y2: max[1],
                thickness: self.thickness,
                height,
            });
        }
        for i in 1..self.rows {
            let clear = (2.0 * max[1] - (self.rows - 1) as f64 * self.thickness) / self.rows as f64;
            let y = -max[1] + i as f64 * clear + (i as f64 - 0.5) * self.thickness;
            walls.push(Wall {
                x1: -max[0],
                y1: y,
                x2: max[0],
                y2: y,
                thickness: self.thickness,
                height,
            });
        }
        if walls != cfg.walls {
            self.checkpoint(&cfg.walls);
            cfg.walls = walls;
        }
        self.mode = Mode::Grid;
    }
    pub fn undo(&mut self, cfg: &mut Design) {
        if let Some(walls) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut cfg.walls, walls));
            self.infer(cfg);
        }
    }
    pub fn redo(&mut self, cfg: &mut Design) {
        if let Some(walls) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut cfg.walls, walls));
            self.infer(cfg);
        }
    }
    fn infer(&mut self, cfg: &mut Design) {
        fit_walls(cfg);
        let restored = Self::from_design(cfg);
        self.mode = restored.mode;
        self.rows = restored.rows;
        self.cols = restored.cols;
        self.thickness = restored.thickness;
        self.height = restored.height;
    }
}
pub fn bounds(cfg: &Design) -> [f64; 2] {
    [cfg.width() / 2.0 - cfg.wall, cfg.length() / 2.0 - cfg.wall]
}
pub fn fit_walls(cfg: &mut Design) {
    let max = bounds(cfg);
    let height = cfg.height() - cfg.floor_z();
    for wall in &mut cfg.walls {
        wall.x1 = wall.x1.clamp(-max[0], max[0]);
        wall.x2 = wall.x2.clamp(-max[0], max[0]);
        wall.y1 = wall.y1.clamp(-max[1], max[1]);
        wall.y2 = wall.y2.clamp(-max[1], max[1]);
        wall.height = wall.height.min(height);
        if (wall.x2 - wall.x1).hypot(wall.y2 - wall.y1) < 1.0 {
            wall.x2 = wall.x1 + if wall.x1 + 1.0 <= max[0] { 1.0 } else { -1.0 };
        }
    }
}
pub fn snap(point: [f64; 2], max: [f64; 2], step: f64) -> [f64; 2] {
    std::array::from_fn(|i| {
        if (point[i].abs() - max[i]).abs() < step.max(1.5) {
            point[i].signum() * max[i]
        } else {
            ((point[i] / step).round() * step).clamp(-max[i], max[i])
        }
    })
}
/// Snap a drawn or edited endpoint to a nearby existing wall centre line.
/// The fixed 1.5 mm reach is measured before grid rounding, so a coarse grid
/// cannot create an unintended connection. An optional anchor constrains the
/// result to the dominant horizontal/vertical axis, including diagonal hosts.
pub fn snap_endpoint(
    point: [f64; 2],
    max: [f64; 2],
    step: f64,
    walls: &[Wall],
    excluded: Option<usize>,
    axis_anchor: Option<[f64; 2]>,
) -> [f64; 2] {
    const REACH: f64 = 1.5;
    const EPS: f64 = 1e-9;
    let mut fallback = snap(point, max, step);
    let locked = axis_anchor.map(|anchor| {
        let fixed = usize::from((point[0] - anchor[0]).abs() >= (point[1] - anchor[1]).abs());
        fallback[fixed] = anchor[fixed];
        (fixed, anchor[fixed])
    });
    let mut probe = point;
    if let Some((fixed, value)) = locked {
        probe[fixed] = value;
    }
    let distance = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]);
    let inside = |p: [f64; 2]| (0..2).all(|i| p[i].is_finite() && p[i].abs() <= max[i] + EPS);
    let mut best = REACH + EPS;
    let mut result = fallback;
    for (index, wall) in walls.iter().enumerate() {
        if Some(index) == excluded {
            continue;
        }
        let a = [wall.x1, wall.y1];
        let delta = [wall.x2 - wall.x1, wall.y2 - wall.y1];
        let length_sq = delta[0] * delta[0] + delta[1] * delta[1];
        if !length_sq.is_finite() || length_sq < EPS * EPS {
            continue;
        }
        let project = |p: [f64; 2]| {
            let t =
                (((p[0] - a[0]) * delta[0] + (p[1] - a[1]) * delta[1]) / length_sq).clamp(0.0, 1.0);
            [a[0] + t * delta[0], a[1] + t * delta[1]]
        };
        let candidate = if let Some((fixed, value)) = locked {
            if delta[fixed].abs() < EPS {
                if (a[fixed] - value).abs() > EPS {
                    continue;
                }
                let mut p = project(probe);
                p[fixed] = value;
                p
            } else {
                let t = (value - a[fixed]) / delta[fixed];
                if !(0.0..=1.0).contains(&t) {
                    continue;
                }
                let mut p = [a[0] + t * delta[0], a[1] + t * delta[1]];
                p[fixed] = value;
                p
            }
        } else {
            project(probe)
        };
        let reach = distance(probe, candidate);
        if reach >= best || !inside(candidate) {
            continue;
        }
        best = reach;
        result = candidate;
        if locked.is_none() {
            // Keep grid alignment along the host where possible; proximity to
            // that host is still determined only from the unrounded cursor.
            let aligned = project(fallback);
            if inside(aligned) && distance(probe, aligned) <= REACH {
                result = aligned;
            }
        }
    }
    result
}
pub fn move_wall(original: &Wall, delta: [f64; 2], max: [f64; 2]) -> Wall {
    let dx = delta[0].clamp(
        -max[0] - original.x1.min(original.x2),
        max[0] - original.x1.max(original.x2),
    );
    let dy = delta[1].clamp(
        -max[1] - original.y1.min(original.y2),
        max[1] - original.y1.max(original.y2),
    );
    Wall {
        x1: original.x1 + dx,
        x2: original.x2 + dx,
        y1: original.y1 + dy,
        y2: original.y2 + dy,
        ..original.clone()
    }
}
pub fn endpoint(original: &Wall, end: bool, point: [f64; 2]) -> Wall {
    let mut wall = original.clone();
    if end {
        wall.x2 = point[0];
        wall.y2 = point[1];
    } else {
        wall.x1 = point[0];
        wall.y1 = point[1];
    }
    if (wall.x2 - wall.x1).hypot(wall.y2 - wall.y1) < 1.0 {
        original.clone()
    } else {
        wall
    }
}
/// Ortho mode, choosing the dominant axis relative to the fixed anchor.
pub fn axis_lock(anchor: [f64; 2], point: [f64; 2]) -> [f64; 2] {
    if (point[0] - anchor[0]).abs() >= (point[1] - anchor[1]).abs() {
        [point[0], anchor[1]]
    } else {
        [anchor[0], point[1]]
    }
}
pub struct Spans {
    pub axis: usize,
    pub point: [f64; 2],
    pub low: f64,
    pub high: f64,
    pub wall_low: f64,
    pub wall_high: f64,
}
impl Spans {
    pub fn widths(&self) -> [f64; 2] {
        [
            (self.wall_low - self.low).max(0.0),
            (self.high - self.wall_high).max(0.0),
        ]
    }
}
/// Clear space at a cross-section, including the thickness of adjacent walls.
pub fn spans(walls: &[Wall], index: usize, max: [f64; 2]) -> Option<Spans> {
    let w = walls.get(index)?;
    let axis = if (w.x2 - w.x1).abs() < 1e-6 {
        0
    } else if (w.y2 - w.y1).abs() < 1e-6 {
        1
    } else {
        return None;
    };
    let other = 1 - axis;
    let point = [w.x1 + (w.x2 - w.x1) * 0.35, w.y1 + (w.y2 - w.y1) * 0.35];
    let mut low = -max[axis];
    let mut high = max[axis];
    for (i, wall) in walls.iter().enumerate().filter(|(i, _)| *i != index) {
        let _ = i;
        let a = [wall.x1, wall.y1];
        let b = [wall.x2, wall.y2];
        let delta = b[other] - a[other];
        if delta.abs() < 1e-6 {
            continue;
        }
        let t = (point[other] - a[other]) / delta;
        if !(0.0..=1.0).contains(&t) {
            continue;
        }
        let center = a[axis] + t * (b[axis] - a[axis]);
        let half = wall.thickness / 2.0 * ((b[axis] - a[axis]).hypot(delta) / delta.abs());
        if center < point[axis] {
            low = low.max(center + half);
        } else {
            high = high.min(center - half);
        }
    }
    Some(Spans {
        axis,
        point,
        low,
        high,
        wall_low: point[axis] - w.thickness / 2.0,
        wall_high: point[axis] + w.thickness / 2.0,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dimension {
    Length,
    LowGap,
    HighGap,
}

/// Apply an exact plan dimension without silently moving through a neighboring wall.
/// Length edits keep the first endpoint and direction; gap edits translate the wall.
pub fn set_dimension(
    walls: &[Wall],
    index: usize,
    max: [f64; 2],
    kind: Dimension,
    value: f64,
) -> Result<Wall, String> {
    const EPS: f64 = 1e-7;
    let wall = walls
        .get(index)
        .ok_or_else(|| "Die Wand ist nicht mehr vorhanden.".to_string())?;
    if !value.is_finite() {
        return Err("Bitte eine endliche Zahl in Millimetern eingeben.".into());
    }
    if max.iter().any(|v| !v.is_finite() || *v <= 0.0)
        || [wall.x1, wall.y1, wall.x2, wall.y2, wall.thickness]
            .iter()
            .any(|v| !v.is_finite())
        || wall.thickness <= 0.0
    {
        return Err("Die Wandmaße sind ungültig.".into());
    }
    let range_error = |min: f64, limit: f64| {
        let min = format!("{min:.2}").replace('.', ",");
        let limit = format!("{limit:.2}").replace('.', ",");
        format!("Erlaubt sind {min} bis {limit} mm.")
    };
    let inside = |point: [f64; 2]| (0..2).all(|i| point[i].abs() <= max[i] + EPS);
    if !inside([wall.x1, wall.y1]) || !inside([wall.x2, wall.y2]) {
        return Err("Die Wand liegt außerhalb des Innenraums.".into());
    }
    match kind {
        Dimension::Length => {
            let start = [wall.x1, wall.y1];
            let delta = [wall.x2 - wall.x1, wall.y2 - wall.y1];
            let length = delta[0].hypot(delta[1]);
            if length < EPS {
                return Err("Die Wand hat keine eindeutige Richtung.".into());
            }
            let direction = [delta[0] / length, delta[1] / length];
            let mut limit = f64::INFINITY;
            for i in 0..2 {
                if direction[i] > 0.0 {
                    limit = limit.min((max[i] - start[i]) / direction[i]);
                } else if direction[i] < 0.0 {
                    limit = limit.min((-max[i] - start[i]) / direction[i]);
                }
            }
            if limit < 1.0 - EPS || value < 1.0 - EPS || value > limit + EPS {
                return Err(range_error(1.0, limit.max(0.0)));
            }
            // The epsilon only compensates for floating-point boundary arithmetic.
            let requested = value.max(1.0).min(limit);
            let mut end: [f64; 2] = std::array::from_fn(|i| {
                (start[i] + direction[i] * requested).clamp(-max[i], max[i])
            });
            // Avoid a sub-1 mm result caused solely by cancellation at a nonzero anchor.
            if (end[0] - start[0]).hypot(end[1] - start[1]) < 1.0 {
                let corrected = (requested + EPS / 100.0).min(limit);
                end = std::array::from_fn(|i| {
                    (start[i] + direction[i] * corrected).clamp(-max[i], max[i])
                });
            }
            if (end[0] - start[0]).hypot(end[1] - start[1]) < 1.0 {
                return Err("Für diese Richtung ist mindestens 1 mm Platz nötig.".into());
            }
            Ok(Wall {
                x2: end[0],
                y2: end[1],
                ..wall.clone()
            })
        }
        Dimension::LowGap | Dimension::HighGap => {
            let spaces = spans(walls, index, max).ok_or_else(|| {
                "Fachmaße benötigen eine waagerechte oder senkrechte Wand.".to_string()
            })?;
            let limit = spaces.high - spaces.low - wall.thickness;
            if !limit.is_finite() || limit < -EPS {
                return Err("Zwischen den benachbarten Wänden ist kein freier Platz.".into());
            }
            if value < -EPS || value > limit + EPS {
                return Err(range_error(0.0, limit.max(0.0)));
            }
            let requested = value.clamp(0.0, limit.max(0.0));
            let center = match kind {
                Dimension::LowGap => spaces.low + wall.thickness / 2.0 + requested,
                Dimension::HighGap => spaces.high - wall.thickness / 2.0 - requested,
                Dimension::Length => unreachable!(),
            };
            let mut delta = [0.0, 0.0];
            delta[spaces.axis] = center - spaces.point[spaces.axis];
            let start = [wall.x1 + delta[0], wall.y1 + delta[1]];
            let end = [wall.x2 + delta[0], wall.y2 + delta[1]];
            if !inside(start) || !inside(end) {
                return Err(
                    "Das Fachmaß würde die Wand außerhalb des Innenraums verschieben.".into(),
                );
            }
            Ok(move_wall(wall, delta, max))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn host(start: [f64; 2], end: [f64; 2]) -> Wall {
        Wall {
            x1: start[0],
            y1: start[1],
            x2: end[0],
            y2: end[1],
            thickness: 0.8,
            height: 10.0,
        }
    }

    #[test]
    fn endpoint_snaps_to_non_grid_wall_centres_from_either_side() {
        let walls = [host([-30.0, 3.35], [30.0, 3.35])];
        for y in [2.0, 3.6, 4.7] {
            let snapped = snap_endpoint([10.2, y], [40.0, 40.0], 1.0, &walls, None, None);
            assert!((snapped[0] - 10.0).abs() < 1e-10);
            assert_eq!(snapped[1], 3.35);
        }
    }

    #[test]
    fn endpoint_snap_proximity_is_independent_of_coarse_grid() {
        let walls = [host([-30.0, 10.2], [30.0, 10.2])];
        // The rounded cursor is near the host, but the real cursor is not.
        assert_eq!(
            snap_endpoint([4.0, 7.5], [40.0, 40.0], 10.0, &walls, None, None),
            [0.0, 10.0]
        );
        // A genuinely nearby host remains reachable even with a coarse grid.
        assert_eq!(
            snap_endpoint([4.0, 10.0], [40.0, 40.0], 10.0, &walls, None, None),
            [4.0, 10.2]
        );
    }

    #[test]
    fn constrained_endpoint_uses_exact_intersection_and_preserves_anchor() {
        let diagonal = [host([8.0, -5.0], [18.0, 5.0])];
        assert_eq!(
            snap_endpoint(
                [12.2, 0.6],
                [40.0, 40.0],
                1.0,
                &diagonal,
                None,
                Some([0.0, 0.0]),
            ),
            [13.0, 0.0]
        );
        let horizontal = [host([-30.0, 20.43], [30.0, 20.43])];
        assert_eq!(
            snap_endpoint(
                [2.5, 20.0],
                [40.0, 40.0],
                1.0,
                &horizontal,
                None,
                Some([2.37, 3.71]),
            ),
            [2.37, 20.43]
        );
        // Grid rounding must not switch the originally dominant axis.
        assert_eq!(
            snap_endpoint([0.6, 0.4], [40.0, 40.0], 1.0, &[], None, Some([0.49, 0.1]),),
            [0.49, 0.0]
        );
    }

    #[test]
    fn constrained_endpoint_rejects_distant_or_parallel_intersections() {
        for wall in [
            host([-30.0, 0.5], [30.0, 0.5]),
            host([-20.0, -2.0], [20.0, 2.0]),
        ] {
            assert_eq!(
                snap_endpoint(
                    [10.2, 0.4],
                    [40.0, 40.0],
                    1.0,
                    &[wall],
                    None,
                    Some([0.0, 0.0]),
                ),
                [10.0, 0.0]
            );
        }
    }

    #[test]
    fn endpoint_snap_excludes_self_and_respects_finite_segments() {
        let walls = [host([0.0, -5.0], [0.0, 5.0])];
        assert_eq!(
            snap_endpoint([0.6, 2.0], [40.0, 40.0], 1.0, &walls, Some(0), None),
            [1.0, 2.0]
        );
        assert_eq!(
            snap_endpoint([0.6, 2.0], [40.0, 40.0], 1.0, &walls, None, None),
            [0.0, 2.0]
        );
        assert_eq!(
            snap_endpoint([0.0, 7.0], [40.0, 40.0], 1.0, &walls, None, None),
            [0.0, 7.0]
        );
        assert_eq!(
            snap_endpoint([0.0, 6.3], [40.0, 40.0], 1.0, &walls, None, None),
            [0.0, 5.0]
        );
        let outside = [host([40.5, -10.0], [40.5, 10.0])];
        assert_eq!(
            snap_endpoint([40.0, 0.0], [40.0, 40.0], 1.0, &outside, None, None),
            [40.0, 0.0]
        );
    }

    #[test]
    fn ortho_keeps_anchor_on_the_other_axis() {
        assert_eq!(axis_lock([3.0, 5.0], [25.0, 10.0]), [25.0, 5.0]);
        assert_eq!(axis_lock([3.0, 5.0], [-4.0, -30.0]), [3.0, -30.0]);
    }
    #[test]
    fn clear_widths_include_neighbor_and_selected_wall_thickness() {
        let walls = vec![
            Wall {
                x1: -10.0,
                x2: -10.0,
                y1: -40.0,
                y2: 40.0,
                thickness: 2.0,
                height: 10.0,
            },
            Wall {
                x1: 20.0,
                x2: 20.0,
                y1: -40.0,
                y2: 40.0,
                thickness: 4.0,
                height: 10.0,
            },
        ];
        let spaces = spans(&walls, 0, [40.0, 40.0]).unwrap();
        assert_eq!(spaces.widths(), [29.0, 27.0]);
        let mut rotated = walls;
        for w in &mut rotated {
            std::mem::swap(&mut w.x1, &mut w.y1);
            std::mem::swap(&mut w.x2, &mut w.y2);
        }
        let spaces = spans(&rotated, 0, [40.0, 40.0]).unwrap();
        assert_eq!(spaces.axis, 1);
        assert_eq!(spaces.widths(), [29.0, 27.0]);
    }
    #[test]
    fn both_editable_gaps_move_wall_exactly_between_neighbors() {
        let original = vec![
            Wall {
                x1: -20.0,
                x2: -20.0,
                y1: -40.0,
                y2: 40.0,
                thickness: 4.0,
                height: 10.0,
            },
            Wall {
                x1: -10.0,
                x2: -10.0,
                y1: -40.0,
                y2: 40.0,
                thickness: 2.0,
                height: 10.0,
            },
            Wall {
                x1: 20.0,
                x2: 20.0,
                y1: -40.0,
                y2: 40.0,
                thickness: 4.0,
                height: 10.0,
            },
        ];
        for rotated in [false, true] {
            let mut walls = original.clone();
            if rotated {
                for wall in &mut walls {
                    std::mem::swap(&mut wall.x1, &mut wall.y1);
                    std::mem::swap(&mut wall.x2, &mut wall.y2);
                }
            }
            for (kind, requested) in [(Dimension::LowGap, 12.5), (Dimension::HighGap, 23.25)] {
                let before = walls[1].clone();
                walls[1] = set_dimension(&walls, 1, [40.0, 40.0], kind, requested).unwrap();
                let edited = &walls[1];
                let widths = spans(&walls, 1, [40.0, 40.0]).unwrap().widths();
                let side = usize::from(kind == Dimension::HighGap);
                assert!((widths[side] - requested).abs() < 1e-10);
                assert!((widths[0] + widths[1] - 34.0).abs() < 1e-10);
                assert_eq!(edited.x2 - edited.x1, before.x2 - before.x1);
                assert_eq!(edited.y2 - edited.y1, before.y2 - before.y1);
                assert_eq!(
                    (edited.thickness, edited.height),
                    (before.thickness, before.height)
                );
                let cfg = Design {
                    walls: walls.clone(),
                    ..Design::default()
                };
                cfg.validate().unwrap();
            }
            for kind in [Dimension::LowGap, Dimension::HighGap] {
                assert!(set_dimension(&walls, 1, [40.0, 40.0], kind, 34.1).is_err());
                for boundary in [0.0, 34.0] {
                    let moved = set_dimension(&walls, 1, [40.0, 40.0], kind, boundary).unwrap();
                    let mut candidate = walls.clone();
                    candidate[1] = moved;
                    let widths = spans(&candidate, 1, [40.0, 40.0]).unwrap().widths();
                    assert!(
                        (widths[usize::from(kind == Dimension::HighGap)] - boundary).abs() < 1e-10
                    );
                }
            }
        }
    }
    #[test]
    fn editable_diagonal_length_keeps_anchor_and_direction() {
        let wall = Wall {
            x1: -10.0,
            y1: -5.0,
            x2: -4.0,
            y2: 3.0,
            thickness: 1.2,
            height: 8.0,
        };
        let walls = [wall.clone()];
        let resized = set_dimension(&walls, 0, [40.0, 40.0], Dimension::Length, 25.0).unwrap();
        assert_eq!((resized.x1, resized.y1), (-10.0, -5.0));
        assert!((resized.x2 - 5.0).abs() < 1e-12);
        assert!((resized.y2 - 15.0).abs() < 1e-12);
        assert!(((resized.x2 - resized.x1).hypot(resized.y2 - resized.y1) - 25.0).abs() < 1e-12);
        assert_eq!(
            (resized.thickness, resized.height),
            (wall.thickness, wall.height)
        );
        Design {
            walls: vec![resized],
            ..Design::default()
        }
        .validate()
        .unwrap();
        let boundary = set_dimension(&walls, 0, [40.0, 40.0], Dimension::Length, 56.25).unwrap();
        assert_eq!(boundary.y2, 40.0);
        assert!(set_dimension(&walls, 0, [40.0, 40.0], Dimension::Length, 56.251).is_err());
        assert!(set_dimension(&walls, 0, [40.0, 40.0], Dimension::Length, 0.999).is_err());
        for kind in [Dimension::LowGap, Dimension::HighGap] {
            assert!(set_dimension(&walls, 0, [40.0, 40.0], kind, 5.0).is_err());
        }
    }
    #[test]
    fn dimensions_reject_invalid_numbers_and_tolerate_rounding_at_bounds() {
        let wall = Wall {
            x1: 0.0,
            y1: -40.0,
            x2: 0.0,
            y2: 40.0,
            thickness: 2.0,
            height: 10.0,
        };
        let walls = [wall];
        for kind in [Dimension::Length, Dimension::LowGap, Dimension::HighGap] {
            for value in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                assert!(set_dimension(&walls, 0, [40.0, 40.0], kind, value).is_err());
            }
            assert!(set_dimension(&walls, 1, [40.0, 40.0], kind, 10.0).is_err());
        }
        let resized =
            set_dimension(&walls, 0, [40.0, 40.0], Dimension::Length, 80.0 + 5e-8).unwrap();
        assert_eq!(resized.y2, 40.0);
        let mut cfg = Design::default();
        for kind in [Dimension::LowGap, Dimension::HighGap] {
            for value in [-5e-8, 78.0 + 5e-8] {
                let edited = set_dimension(&walls, 0, [40.0, 40.0], kind, value).unwrap();
                cfg.walls = vec![edited];
                cfg.validate().unwrap();
                let widths = spans(&cfg.walls, 0, [40.0, 40.0]).unwrap().widths();
                assert!(
                    (widths[usize::from(kind == Dimension::HighGap)] - value.clamp(0.0, 78.0))
                        .abs()
                        < 1e-10
                );
            }
        }
    }
    #[test]
    fn none_and_undo_restore_actual_print_walls() {
        let mut cfg = Design::default();
        let mut editor = Editor::default();
        editor.grid(&mut cfg);
        let grid = cfg.walls.clone();
        assert_eq!(grid.len(), 2);
        editor.clear(&mut cfg);
        assert!(cfg.walls.is_empty());
        editor.undo(&mut cfg);
        assert_eq!(cfg.walls, grid);
        editor.redo(&mut cfg);
        assert!(cfg.walls.is_empty());
    }
    #[test]
    fn moving_stays_inside_and_keeps_length() {
        let wall = Wall {
            x1: -10.0,
            y1: -20.0,
            x2: 10.0,
            y2: 20.0,
            thickness: 1.2,
            height: 8.0,
        };
        let moved = move_wall(&wall, [100.0, -100.0], [30.0, 30.0]);
        assert_eq!(
            [moved.x1, moved.x2, moved.y1, moved.y2],
            [10.0, 30.0, -30.0, 10.0]
        );
        assert_eq!(moved.x2 - moved.x1, wall.x2 - wall.x1);
    }
    #[test]
    fn endpoint_cannot_collapse_wall() {
        let w = Wall {
            x1: 0.0,
            y1: 0.0,
            x2: 10.0,
            y2: 0.0,
            thickness: 1.2,
            height: 8.0,
        };
        assert_eq!(endpoint(&w, true, [0.1, 0.0]), w);
    }
    #[test]
    fn grid_remains_valid_on_small_bin() {
        let mut cfg = Design {
            grid_x: 1,
            grid_y: 1,
            height_u: 2,
            ..Default::default()
        };
        let mut editor = Editor {
            rows: 8,
            cols: 8,
            ..Default::default()
        };
        editor.grid(&mut cfg);
        cfg.validate().unwrap();
        assert_eq!(cfg.walls.len(), 14);
        assert_eq!(snap([18.6, 0.2], [19.15, 19.15], 1.0), [19.15, 0.0]);
    }
    #[test]
    fn undo_after_shrinking_container_remains_printable() {
        let mut cfg = Design {
            grid_x: 8,
            grid_y: 8,
            height_u: 15,
            ..Default::default()
        };
        let mut editor = Editor::default();
        editor.grid(&mut cfg);
        editor.clear(&mut cfg);
        cfg.grid_x = 1;
        cfg.grid_y = 1;
        cfg.height_u = 2;
        editor.undo(&mut cfg);
        cfg.validate().unwrap();
    }
    #[test]
    fn standalone_parts_do_not_reconstruct_unused_invalid_box_layouts() {
        for product in [
            crate::design::Product::Clip,
            crate::design::Product::LabelInsert,
        ] {
            let cfg = Design {
                product,
                grid_x: 0,
                grid_y: 0,
                height_u: 0,
                walls: vec![Wall {
                    x1: 0.0,
                    y1: -10.0,
                    x2: 0.0,
                    y2: 10.0,
                    thickness: 1.2,
                    height: 8.0,
                }],
                ..Default::default()
            };
            cfg.validate().unwrap();
            assert!(Editor::from_design(&cfg).mode == Mode::None);
        }
    }
    #[test]
    fn uniform_grid_has_equal_clear_compartments_in_both_axes() {
        for thickness in [0.8, 1.2, 4.0] {
            let mut cfg = Design {
                grid_x: 2,
                grid_y: 1,
                ..Default::default()
            };
            let mut editor = Editor {
                cols: 3,
                rows: 4,
                thickness,
                ..Default::default()
            };
            editor.grid(&mut cfg);
            cfg.validate().unwrap();
            let max = bounds(&cfg);
            for index in 0..cfg.walls.len() {
                let spaces = spans(&cfg.walls, index, max).unwrap();
                let count = if spaces.axis == 0 {
                    editor.cols
                } else {
                    editor.rows
                } as f64;
                let clear = (2.0 * max[spaces.axis] - (count - 1.0) * thickness) / count;
                for width in spaces.widths() {
                    assert!(
                        (width - clear).abs() < 1e-10,
                        "axis {}, thickness {thickness}: {width} != {clear}",
                        spaces.axis
                    );
                }
            }
        }
    }
    #[test]
    fn saved_uniform_layout_is_recognized_after_reload() {
        let mut cfg = Design::default();
        let mut editor = Editor {
            cols: 3,
            rows: 2,
            ..Default::default()
        };
        editor.grid(&mut cfg);
        let reloaded: Design = serde_json::from_str(&serde_json::to_string(&cfg).unwrap()).unwrap();
        let restored = Editor::from_design(&reloaded);
        assert!(restored.mode == Mode::Grid);
        assert_eq!((restored.cols, restored.rows), (3, 2));
        cfg.walls[0].x1 += 3.0;
        assert!(Editor::from_design(&cfg).mode == Mode::Free);
    }
}
