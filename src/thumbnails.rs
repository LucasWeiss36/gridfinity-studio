//! Cached home-screen previews built by the same geometry generator as exported parts.
use crate::{
    design::{CutShape, Cutout, Design, Product},
    model, viewport,
};
use eframe::{egui, egui_wgpu::RenderState};
use std::{sync::mpsc, time::Duration};

const PRODUCTS: [Product; 6] = [
    Product::Bin,
    Product::Baseplate,
    Product::Clip,
    Product::Insert,
    Product::DeskBox,
    Product::LabelInsert,
];

struct Mesh {
    vertices: Vec<[f32; 3]>,
    faces: Vec<([usize; 3], [u8; 3])>,
    middle: [f32; 3],
    projected_size: [f32; 2],
    extent: f32,
    yaw: f32,
    pitch: f32,
}

struct Entry {
    product: Product,
    mesh: Option<Result<Mesh, String>>,
    viewport: Option<viewport::Viewport>,
    texture: Option<egui::TextureId>,
    rendered_size: [u32; 2],
    rendered_background: Option<egui::Color32>,
}

type Generated = (Product, Result<Mesh, String>);

pub struct Thumbnails {
    state: Option<RenderState>,
    entries: Vec<Entry>,
    receiver: Option<mpsc::Receiver<Generated>>,
}

impl Thumbnails {
    pub fn new(state: Option<RenderState>, ctx: egui::Context) -> Self {
        let (sender, receiver) = mpsc::channel();
        // Fixed in-memory presets only. Geometry runs once off the UI thread;
        // GPU resources are created and rendered on the UI thread when visible.
        std::thread::spawn(move || {
            for product in PRODUCTS {
                if sender.send((product, make_mesh(product))).is_err() {
                    return;
                }
                ctx.request_repaint();
            }
        });
        Self {
            state,
            entries: PRODUCTS
                .into_iter()
                .map(|product| Entry {
                    product,
                    mesh: None,
                    viewport: None,
                    texture: None,
                    rendered_size: [0; 2],
                    rendered_background: None,
                })
                .collect(),
            receiver: Some(receiver),
        }
    }

    fn poll(&mut self) {
        loop {
            match self.receiver.as_ref().map(mpsc::Receiver::try_recv) {
                Some(Ok((product, mesh))) => {
                    if let Some(entry) = self.entries.iter_mut().find(|e| e.product == product) {
                        entry.mesh = Some(mesh);
                    }
                }
                Some(Err(mpsc::TryRecvError::Disconnected)) => {
                    self.receiver = None;
                    for entry in &mut self.entries {
                        if entry.mesh.is_none() {
                            entry.mesh =
                                Some(Err("Modellvorschau konnte nicht berechnet werden.".into()));
                        }
                    }
                    return;
                }
                _ => return,
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, rect: egui::Rect, product: Product) {
        self.poll();
        if !ui.is_rect_visible(rect) || rect.width() < 1.0 || rect.height() < 1.0 {
            return;
        }
        let Some(entry) = self.entries.iter_mut().find(|e| e.product == product) else {
            message(ui, rect, "Keine Modellvorschau verfügbar.");
            return;
        };
        let mesh = match &entry.mesh {
            Some(Ok(mesh)) => mesh,
            Some(Err(error)) => {
                message(ui, rect, "Vorschau nicht verfügbar");
                ui.interact(rect, ui.id().with("preview-error"), egui::Sense::hover())
                    .on_hover_text(error);
                return;
            }
            None => {
                ui.put(
                    egui::Rect::from_center_size(
                        rect.center() - egui::vec2(0.0, 12.0),
                        egui::vec2(20.0, 20.0),
                    ),
                    egui::Spinner::new().size(20.0),
                );
                message(
                    ui,
                    rect.translate(egui::vec2(0.0, 15.0)),
                    "Vorschau wird geladen …",
                );
                ui.ctx().request_repaint_after(Duration::from_millis(100));
                return;
            }
        };
        let Some(state) = &self.state else {
            message(ui, rect, "3D-Vorschau auf diesem Gerät nicht verfügbar.");
            return;
        };
        let ratio = ui.ctx().pixels_per_point();
        let size = [
            (rect.width() * ratio).round().max(1.0) as u32,
            (rect.height() * ratio).round().max(1.0) as u32,
        ];
        let background = crate::theme::palette(ui.ctx()).canvas;
        if entry.texture.is_none()
            || entry.rendered_size != size
            || entry.rendered_background != Some(background)
        {
            let gpu = entry
                .viewport
                .get_or_insert_with(|| viewport::Viewport::new(state.clone()));
            entry.texture = Some(gpu.render(
                "fixed-home-preset",
                &mesh.vertices,
                &mesh.faces,
                size,
                mesh.middle,
                mesh.extent,
                mesh.yaw,
                mesh.pitch,
                fit_zoom(mesh, size),
                background,
            ));
            entry.rendered_size = size;
            entry.rendered_background = Some(background);
        }
        if let Some(texture) = entry.texture {
            ui.painter().image(
                texture,
                rect,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
    }
}

fn message(ui: &egui::Ui, rect: egui::Rect, text: &str) {
    let colour = crate::theme::palette(ui.ctx()).muted;
    let text = ui.painter().layout(
        text.into(),
        egui::FontId::proportional(12.0),
        colour,
        (rect.width() - 16.0).max(1.0),
    );
    ui.painter()
        .galley(rect.center() - text.size() / 2.0, text, colour);
}

fn preset(product: Product) -> Design {
    let mut cfg = Design {
        product,
        ..Default::default()
    };
    match product {
        Product::Baseplate => {
            cfg.grid_x = 3;
            cfg.baseplate_open = true;
            cfg.baseplate_clips = false;
            cfg.magnets = false;
            cfg.screws = false;
        }
        Product::Insert => {
            cfg.height_u = 4;
            cfg.cutouts = vec![
                Cutout {
                    shape: CutShape::Rectangle,
                    x: -17.0,
                    y: 0.0,
                    width: 18.0,
                    length: 45.0,
                    depth: 14.0,
                    ..Default::default()
                },
                Cutout {
                    shape: CutShape::Circle,
                    x: 18.0,
                    y: -18.0,
                    width: 16.0,
                    depth: 12.0,
                    ..Default::default()
                },
                Cutout {
                    shape: CutShape::Hexagon,
                    x: 18.0,
                    y: 17.0,
                    width: 15.0,
                    depth: 12.0,
                    ..Default::default()
                },
            ];
        }
        Product::LabelInsert => {
            cfg.label_width = 40.0;
            cfg.label_height = 12.0;
            cfg.label = "KABEL".into();
        }
        Product::Bin | Product::Clip | Product::DeskBox => {}
    }
    cfg
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}

fn make_mesh(product: Product) -> Result<Mesh, String> {
    let parts = model::generate(&preset(product)).map_err(|e| e.to_string())?;
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    for (index, part) in parts.into_iter().enumerate() {
        if !part.report.closed || part.report.components != 1 {
            return Err("Das Vorschaumodell ist nicht geschlossen.".into());
        }
        let offset = vertices.len();
        for vertex in part.vertices {
            let mut p: [f32; 3] = std::array::from_fn(|i| vertex[i] + part.placement[i]);
            match product {
                // Rigid rotations only: show the real clip lying lengthwise,
                // with its open U section facing the camera, and the label face up.
                Product::Clip => p = [p[0], -p[2], p[1]],
                Product::LabelInsert => p = [p[0], p[2], -p[1]],
                Product::DeskBox if index > 0 => p[1] -= if index == 1 { 9.0 } else { 30.0 },
                _ => {}
            }
            vertices.push(p);
        }
        faces.extend(
            part.faces
                .into_iter()
                .map(|face| (face.map(|i| i + offset), part.colour)),
        );
    }
    if vertices.is_empty() {
        return Err("Das Vorschaumodell ist leer.".into());
    }
    let min_z = vertices.iter().map(|p| p[2]).fold(f32::INFINITY, f32::min);
    for p in &mut vertices {
        p[2] -= min_z;
    }
    let (yaw, pitch): (f32, f32) = match product {
        Product::Clip => (-0.67, 0.42),
        Product::LabelInsert => (-0.33, 0.96),
        Product::Baseplate => (-0.38, 0.94),
        Product::DeskBox => (-0.38, 0.65),
        Product::Bin | Product::Insert => (-0.38, 0.79),
    };
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let rows = [
        [cy, -sy, 0.0],
        [sp * sy, sp * cy, cp],
        [-cp * sy, -cp * cy, sp],
    ];
    let mut projected_low = [f32::INFINITY; 3];
    let mut projected_high = [f32::NEG_INFINITY; 3];
    let mut low = [f32::INFINITY; 3];
    let mut high = [f32::NEG_INFINITY; 3];
    for &p in &vertices {
        for i in 0..3 {
            let projection = dot(rows[i], p);
            projected_low[i] = projected_low[i].min(projection);
            projected_high[i] = projected_high[i].max(projection);
            low[i] = low[i].min(p[i]);
            high[i] = high[i].max(p[i]);
        }
    }
    let center: [f32; 3] = std::array::from_fn(|i| (projected_low[i] + projected_high[i]) / 2.0);
    // Invert the orthonormal camera basis, centering the projected silhouette
    // rather than the world-space bounding box (important for open drawers).
    let middle = std::array::from_fn(|axis| (0..3).map(|i| rows[i][axis] * center[i]).sum());
    Ok(Mesh {
        vertices,
        faces,
        middle,
        projected_size: [
            projected_high[0] - projected_low[0],
            projected_high[1] - projected_low[1],
        ],
        extent: (0..3).map(|i| high[i] - low[i]).fold(1.0_f32, f32::max),
        yaw,
        pitch,
    })
}

fn fit_zoom(mesh: &Mesh, size: [u32; 2]) -> f32 {
    let scale = (size[0] as f32 * 0.90 / mesh.projected_size[0].max(0.01))
        .min(size[1] as f32 * 0.88 / mesh.projected_size[1].max(0.01));
    scale * mesh.extent / (size[0].min(size[1]).max(1) as f32 * 0.7)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_home_presets_generate_closed_real_parts_and_fit_their_cards() {
        for product in PRODUCTS {
            let mesh = make_mesh(product).unwrap_or_else(|error| panic!("{product:?}: {error}"));
            assert!(!mesh.faces.is_empty());
            assert!(mesh.vertices.iter().all(|p| p[2] >= 0.0));
            for size in [[210, 156], [448, 156], [896, 312]] {
                let scale = size[0].min(size[1]) as f32 * 0.7 / mesh.extent * fit_zoom(&mesh, size);
                assert!(mesh.projected_size[0] * scale <= size[0] as f32 * 0.901);
                assert!(mesh.projected_size[1] * scale <= size[1] as f32 * 0.881);
            }
        }
    }
}
