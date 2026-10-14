//! Development-only CAD preview: `cargo run --release --example render_review -- input.stl output.bmp`.
//! Reads actual exported triangles; no UI automation, image synthesis, or added dependencies.
//! `--project project.json output.bmp` renders generated parts in their actual assembled positions.
use std::{collections::HashMap, env, fs::File, io::Write, path::Path};

#[allow(dead_code)]
#[path = "../src/design.rs"]
mod design;
#[allow(dead_code)]
#[path = "../src/lettering.rs"]
mod lettering;
#[allow(dead_code)]
#[path = "../src/model.rs"]
mod model;

const WIDTH: usize = 900;
const HEIGHT: usize = 650;
type V3 = [f64; 3];

fn dot(a: V3, b: V3) -> f64 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}
fn normal(a: V3, b: V3, c: V3) -> V3 {
    let u: V3 = std::array::from_fn(|i| b[i] - a[i]);
    let v: V3 = std::array::from_fn(|i| c[i] - a[i]);
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let length = dot(n, n).sqrt().max(1e-20);
    n.map(|n| n / length)
}
fn orient(a: V3, b: V3, p: V3) -> f64 {
    (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
}

fn write_bmp(path: &Path, pixels: &[[u8; 3]]) -> std::io::Result<()> {
    let stride = (WIDTH * 3 + 3) & !3;
    let mut header = [0_u8; 54];
    header[..2].copy_from_slice(b"BM");
    header[2..6].copy_from_slice(&((54 + stride * HEIGHT) as u32).to_le_bytes());
    header[10..14].copy_from_slice(&54_u32.to_le_bytes());
    header[14..18].copy_from_slice(&40_u32.to_le_bytes());
    header[18..22].copy_from_slice(&(WIDTH as i32).to_le_bytes());
    header[22..26].copy_from_slice(&(HEIGHT as i32).to_le_bytes());
    header[26..28].copy_from_slice(&1_u16.to_le_bytes());
    header[28..30].copy_from_slice(&24_u16.to_le_bytes());
    let mut file = File::create(path)?;
    file.write_all(&header)?;
    let mut row = vec![0; stride];
    for y in (0..HEIGHT).rev() {
        for x in 0..WIDTH {
            let [r, g, b] = pixels[y * WIDTH + x];
            row[x * 3..x * 3 + 3].copy_from_slice(&[b, g, r]);
        }
        file.write_all(&row)?;
    }
    file.flush()
}

fn main() -> anyhow::Result<()> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    anyhow::ensure!(
        args.len() >= 2,
        "Aufruf: render_review input.stl [weitere.stl] output.bmp ODER --project project.json output.bmp"
    );
    let mut vertices = Vec::<V3>::new();
    let mut faces = Vec::<([usize; 3], usize)>::new();
    let palette = [
        [45.0, 161.0, 207.0],
        [73.0, 196.0, 200.0],
        [239.0, 177.0, 87.0],
    ];
    if args[0] == "--project" {
        anyhow::ensure!(
            args.len() == 3,
            "Projektmodus: --project project.json output.bmp"
        );
        let cfg: design::Design = serde_json::from_reader(File::open(&args[1])?)?;
        for (index, part) in model::generate(&cfg)?.into_iter().enumerate() {
            let offset = vertices.len();
            vertices.extend(part.vertices.iter().map(|v| {
                std::array::from_fn(|axis| f64::from(v[axis]) + f64::from(part.placement[axis]))
            }));
            faces.extend(part.faces.iter().map(|f| (f.map(|i| i + offset), index)));
            let [w, d, h] = part.report.dimensions;
            println!(
                "{}: {:.3} × {:.3} × {:.3} mm, {} Dreiecke, Montageposition {:?}",
                part.name,
                w,
                d,
                h,
                part.faces.len(),
                part.placement
            );
        }
    } else {
        for (part, path) in args[..args.len() - 1].iter().enumerate() {
            let mesh = stl_io::read_stl(&mut File::open(path)?)?;
            let offset = vertices.len();
            let mut low = [f64::INFINITY; 3];
            let mut high = [f64::NEG_INFINITY; 3];
            for v in mesh.vertices {
                let p = [f64::from(v[0]), f64::from(v[1]), f64::from(v[2])];
                anyhow::ensure!(
                    p.iter().all(|n| n.is_finite()),
                    "STL enthält ungültige Koordinaten"
                );
                for i in 0..3 {
                    low[i] = low[i].min(p[i]);
                    high[i] = high[i].max(p[i]);
                }
                vertices.push(p);
            }
            println!(
                "{}: {:.3} × {:.3} × {:.3} mm, {} Dreiecke",
                Path::new(path).display(),
                high[0] - low[0],
                high[1] - low[1],
                high[2] - low[2],
                mesh.faces.len()
            );
            faces.extend(
                mesh.faces
                    .iter()
                    .map(|f| (f.vertices.map(|i| i + offset), part)),
            );
        }
    }
    anyhow::ensure!(!faces.is_empty(), "Keine Dreiecke in der STL");
    let (sy, cy) = (-0.40_f64).sin_cos();
    let (sp, cp) = 0.70_f64.sin_cos();
    let rows = [
        [cy, -sy, 0.0],
        [sp * sy, sp * cy, cp],
        [-cp * sy, -cp * cy, sp],
    ];
    let mut projected: Vec<V3> = vertices
        .iter()
        .map(|&v| [dot(rows[0], v), -dot(rows[1], v), dot(rows[2], v)])
        .collect();
    let low: V3 =
        std::array::from_fn(|i| projected.iter().map(|p| p[i]).fold(f64::INFINITY, f64::min));
    let high: V3 = std::array::from_fn(|i| {
        projected
            .iter()
            .map(|p| p[i])
            .fold(f64::NEG_INFINITY, f64::max)
    });
    let scale = ((WIDTH as f64 - 120.0) / (high[0] - low[0]).max(0.01))
        .min((HEIGHT as f64 - 100.0) / (high[1] - low[1]).max(0.01));
    for p in &mut projected {
        p[0] = (p[0] - (low[0] + high[0]) / 2.0) * scale + WIDTH as f64 / 2.0;
        p[1] = (p[1] - (low[1] + high[1]) / 2.0) * scale + HEIGHT as f64 / 2.0;
    }
    let mut pixels = vec![[239, 244, 247]; WIDTH * HEIGHT];
    let mut depth = vec![f64::NEG_INFINITY; WIDTH * HEIGHT];
    let mut edges = HashMap::<[usize; 2], Vec<V3>>::new();
    for &(indices, part) in &faces {
        let n = normal(
            vertices[indices[0]],
            vertices[indices[1]],
            vertices[indices[2]],
        );
        for (a, b) in [
            (indices[0], indices[1]),
            (indices[1], indices[2]),
            (indices[2], indices[0]),
        ] {
            edges.entry([a.min(b), a.max(b)]).or_default().push(n);
        }
        let [a, b, c] = indices.map(|i| projected[i]);
        let area = orient(a, b, c);
        if area.abs() < 1e-10 {
            continue;
        }
        let from_x = a[0].min(b[0]).min(c[0]).floor().max(0.0) as usize;
        let to_x = a[0].max(b[0]).max(c[0]).ceil().min((WIDTH - 1) as f64) as usize;
        let from_y = a[1].min(b[1]).min(c[1]).floor().max(0.0) as usize;
        let to_y = a[1].max(b[1]).max(c[1]).ceil().min((HEIGHT - 1) as f64) as usize;
        let camera_normal = [dot(rows[0], n), dot(rows[1], n), dot(rows[2], n)];
        let brightness =
            (0.72 + 0.24 * dot(camera_normal, [-0.35, 0.6, 0.72]).max(0.0) + 0.08 * n[2].max(0.0))
                .min(1.0);
        let colour = palette[part % palette.len()]
            .map(|v| ((v + (255.0 - v) * 0.12 * n[2].max(0.0)) * brightness).round() as u8);
        for y in from_y..=to_y {
            for x in from_x..=to_x {
                let p = [x as f64 + 0.5, y as f64 + 0.5, 0.0];
                let weights = [
                    orient(b, c, p) / area,
                    orient(c, a, p) / area,
                    orient(a, b, p) / area,
                ];
                if weights.iter().all(|&w| w >= -1e-9) {
                    let z = weights[0] * a[2] + weights[1] * b[2] + weights[2] * c[2];
                    let i = y * WIDTH + x;
                    if z > depth[i] {
                        depth[i] = z;
                        pixels[i] = colour;
                    }
                }
            }
        }
    }
    // Geometric creases and silhouettes, depth tested against actual filled triangles.
    for (indices, normals) in edges {
        let crease = normals.len() != 2 || dot(normals[0], normals[1]) < 0.94;
        let silhouette = normals.iter().any(|&n| dot(n, rows[2]) > 1e-5)
            && normals.iter().any(|&n| dot(n, rows[2]) < -1e-5);
        if !crease && !silhouette {
            continue;
        }
        let [a, b] = indices.map(|i| projected[i]);
        let steps = (a[0] - b[0]).abs().max((a[1] - b[1]).abs()).ceil().max(1.0) as usize;
        for step in 0..=steps {
            let t = step as f64 / steps as f64;
            let p: V3 = std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t);
            let (x, y) = (p[0].floor() as isize, p[1].floor() as isize);
            if x < 0 || y < 0 || x >= WIDTH as isize || y >= HEIGHT as isize {
                continue;
            }
            let i = y as usize * WIDTH + x as usize;
            if p[2] + 1.2 / scale >= depth[i] {
                pixels[i] = [28, 68, 85];
            }
        }
    }
    let output = Path::new(args.last().unwrap());
    write_bmp(output, &pixels)?;
    println!(
        "CAD-Vorschau: {} ({}×{} px, orthografisch)",
        output.display(),
        WIDTH,
        HEIGHT
    );
    Ok(())
}
