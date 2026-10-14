//! Native GPU viewport with a real depth buffer. No sorting approximations.
use eframe::{egui, egui_wgpu::RenderState};
use std::collections::HashMap;
use wgpu::util::DeviceExt;

const SAMPLES: u32 = 4;
const CREASE_COS: f32 = 0.939_692_6; // Twenty degrees; omit mesh triangulation and round tessellation.

// A display-only material palette. Semantic roles (including an oversized
// print's red warning) and the model/export data remain unchanged.
fn display_colour(colour: [u8; 3], dark: bool) -> [u8; 3] {
    match (colour, dark) {
        ([45, 167, 216], false) => [60, 129, 117],
        ([45, 167, 216], true) => [78, 164, 147],
        ([121, 151, 192], false) => [129, 154, 139],
        ([121, 151, 192], true) => [140, 164, 154],
        ([118, 210, 224], _) => [174, 201, 178],
        ([242, 187, 76], false) => [187, 135, 65],
        ([242, 187, 76], true) => [212, 167, 94],
        ([133, 213, 153], false) => [100, 150, 107],
        ([133, 213, 153], true) => [143, 193, 155],
        (crate::print_preview::TOO_LARGE, true) => [240, 125, 110],
        (other, _) => other,
    }
}

fn display_ground_colour(colour: [u8; 3], dark: bool) -> [u8; 3] {
    match (colour, dark) {
        ([147, 167, 150], false) => [162, 181, 176],
        ([215, 222, 209], false) => [214, 225, 222],
        ([147, 167, 150], true) => [61, 84, 89],
        ([215, 222, 209], true) => [38, 51, 56],
        (crate::print_preview::BED, true) => [38, 51, 57],
        (crate::print_preview::MARGIN, true) => [66, 58, 44],
        (crate::print_preview::GRID, true) => [60, 79, 85],
        (crate::print_preview::OUTLINE, true) => [82, 172, 151],
        (crate::print_preview::USABLE, true) => [180, 143, 82],
        (other, _) => other,
    }
}

struct Edge {
    points: [[f32; 3]; 2],
    normals: Vec<[f32; 3]>,
    crease: bool,
}

struct Targets {
    _colour: wgpu::Texture,
    view: wgpu::TextureView,
    _multisample: wgpu::Texture,
    multisample_view: wgpu::TextureView,
    _depth: wgpu::Texture,
    depth_view: wgpu::TextureView,
}

pub struct Viewport {
    state: RenderState,
    pipeline: wgpu::RenderPipeline,
    edge_pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    bind: wgpu::BindGroup,
    targets: Targets,
    texture_id: egui::TextureId,
    size: [u32; 2],
    vertices: wgpu::Buffer,
    count: u32,
    edges: Vec<Edge>,
    edge_vertices: wgpu::Buffer,
    edge_count: u32,
    edge_camera: [u32; 2],
    key: String,
    dark_theme: bool,
}
fn bytes(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_ne_bytes()).collect()
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}
fn normalise(n: [f32; 3]) -> [f32; 3] {
    let length = dot(n, n).sqrt().max(1e-20);
    n.map(|v| v / length)
}
fn mesh_data(
    vertices: &[[f32; 3]],
    faces: &[([usize; 3], [u8; 3])],
    dark: bool,
) -> (Vec<f32>, Vec<Edge>) {
    let mut adjacent = vec![Vec::<[f32; 3]>::new(); vertices.len()];
    let mut edge_normals = HashMap::<[usize; 2], Vec<[f32; 3]>>::new();
    let normals: Vec<_> = faces
        .iter()
        .map(|(indices, _)| {
            let [a, b, c] = indices.map(|i| vertices[i]);
            // The grid is display-only geometry below the build plane. A zero normal
            // marks its colours as unlit, and it must never receive model outlines.
            if a[2] < 0.0 && b[2] < 0.0 && c[2] < 0.0 {
                return [0.0; 3];
            }
            let u: [f32; 3] = std::array::from_fn(|i| b[i] - a[i]);
            let v: [f32; 3] = std::array::from_fn(|i| c[i] - a[i]);
            let normal = normalise([
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ]);
            for &index in indices {
                adjacent[index].push(normal);
            }
            for [a, b] in [
                [indices[0], indices[1]],
                [indices[1], indices[2]],
                [indices[2], indices[0]],
            ] {
                edge_normals
                    .entry([a.min(b), a.max(b)])
                    .or_default()
                    .push(normal);
            }
            normal
        })
        .collect();
    let mut data = Vec::with_capacity(faces.len() * 27);
    for ((face, colour), normal) in faces.iter().zip(normals) {
        let colour = if dot(normal, normal) > 0.01 {
            display_colour(*colour, dark)
        } else {
            display_ground_colour(*colour, dark)
        };
        for &index in face {
            let mut smooth = [0.0; 3];
            for n in &adjacent[index] {
                if dot(*n, normal) > CREASE_COS {
                    for i in 0..3 {
                        smooth[i] += n[i];
                    }
                }
            }
            data.extend(vertices[index]);
            data.extend(normalise(smooth));
            data.extend(colour.map(|v| v as f32 / 255.0));
        }
    }
    let edges = edge_normals
        .into_iter()
        .map(|(indices, normals)| {
            let crease = normals.len() != 2 || dot(normals[0], normals[1]) < CREASE_COS;
            Edge {
                points: indices.map(|i| vertices[i]),
                normals,
                crease,
            }
        })
        .collect();
    (data, edges)
}
fn edge_visible(edge: &Edge, view: [f32; 3]) -> bool {
    edge.crease
        || (edge.normals.iter().any(|n| dot(*n, view) > 1e-5)
            && edge.normals.iter().any(|n| dot(*n, view) < -1e-5))
}
fn render_size(requested: [u32; 2]) -> [u32; 2] {
    let requested = requested.map(|n| n.max(1) as f64);
    let longest = requested[0].max(requested[1]);
    let shortest = requested[0].min(requested[1]);
    // Resize both dimensions together: clamping them independently stretches
    // the displayed model on ultrawide and high-DPI windows. Prefer at least
    // 32 pixels per side, but the 4096-pixel upper bound takes precedence for
    // exceptionally narrow views; even a one-pixel texture is valid for WGPU.
    let factor = (32.0 / shortest).max(1.0).min(4096.0 / longest);
    requested.map(|n| (n * factor).round().clamp(1.0, 4096.0) as u32)
}
fn targets(device: &wgpu::Device, size: [u32; 2]) -> Targets {
    let colour = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Gridfinity colour"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = colour.create_view(&Default::default());
    let multisample = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Gridfinity antialiased colour"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: SAMPLES,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let multisample_view = multisample.create_view(&Default::default());
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Gridfinity depth"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: SAMPLES,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let depth_view = depth.create_view(&Default::default());
    Targets {
        _colour: colour,
        view,
        _multisample: multisample,
        multisample_view,
        _depth: depth,
        depth_view,
    }
}
impl Viewport {
    pub fn new(state: RenderState) -> Self {
        let device = &state.device;
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera"),
            size: 128,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera binding"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Gridfinity preview shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("preview.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Preview layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let mut pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            label: Some("Gridfinity solid preview"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 36,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 12,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 24,
                            shader_location: 2,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                // Polygon offset keeps an overlaid CAD line visible across its
                // MSAA footprint, including steep walls meeting a flat floor.
                bias: wgpu::DepthBiasState {
                    constant: 1,
                    slope_scale: 1.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState {
                count: SAMPLES,
                ..Default::default()
            },
            multiview: None,
            cache: None,
        };
        let pipeline = device.create_render_pipeline(&pipeline_descriptor);
        pipeline_descriptor.label = Some("Gridfinity visible feature edges");
        pipeline_descriptor.vertex.entry_point = Some("vs_edge");
        pipeline_descriptor.fragment.as_mut().unwrap().entry_point = Some("fs_edge");
        pipeline_descriptor.primitive.topology = wgpu::PrimitiveTopology::LineList;
        pipeline_descriptor.primitive.cull_mode = None;
        let edge_depth = pipeline_descriptor.depth_stencil.as_mut().unwrap();
        edge_depth.depth_write_enabled = false;
        edge_depth.depth_compare = wgpu::CompareFunction::LessEqual;
        edge_depth.bias = Default::default();
        let edge_pipeline = device.create_render_pipeline(&pipeline_descriptor);
        let size = [32, 32];
        let targets = targets(device, size);
        let texture_id = state.renderer.write().register_native_texture(
            device,
            &targets.view,
            wgpu::FilterMode::Linear,
        );
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Empty preview"),
            contents: &[0; 36],
            usage: wgpu::BufferUsages::VERTEX,
        });
        let edge_vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Empty feature edges"),
            contents: &[0; 36],
            usage: wgpu::BufferUsages::VERTEX,
        });
        Self {
            state,
            pipeline,
            edge_pipeline,
            uniform,
            bind,
            targets,
            texture_id,
            size,
            vertices,
            count: 0,
            edges: Vec::new(),
            edge_vertices,
            edge_count: 0,
            edge_camera: [u32::MAX; 2],
            key: String::new(),
            dark_theme: false,
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        key: &str,
        vertices: &[[f32; 3]],
        faces: &[([usize; 3], [u8; 3])],
        size: [u32; 2],
        middle: [f32; 3],
        extent: f32,
        yaw: f32,
        pitch: f32,
        zoom: f32,
        background: egui::Color32,
    ) -> egui::TextureId {
        let size = render_size(size);
        let dark =
            u16::from(background.r()) + u16::from(background.g()) + u16::from(background.b()) < 384;
        let device = &self.state.device;
        if self.size != size {
            let targets = targets(device, size);
            self.state
                .renderer
                .write()
                .update_egui_texture_from_wgpu_texture(
                    device,
                    &targets.view,
                    wgpu::FilterMode::Linear,
                    self.texture_id,
                );
            self.targets = targets;
            self.size = size;
        }
        if self.key != key || self.dark_theme != dark {
            let (data, edges) = mesh_data(vertices, faces, dark);
            self.edges = edges;
            self.edge_camera = [u32::MAX; 2];
            if !data.is_empty() {
                self.vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Gridfinity model vertices"),
                    contents: &bytes(&data),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            }
            self.count = (faces.len() * 3) as u32;
            self.key = key.into();
            self.dark_theme = dark;
        }
        let (sy, cy) = yaw.sin_cos();
        let (sp, cp) = pitch.sin_cos();
        let rotation = [
            [cy, -sy, 0.0],
            [sp * sy, sp * cy, cp],
            [-cp * sy, -cp * cy, sp],
        ];
        let camera_key = [yaw.to_bits(), pitch.to_bits()];
        if self.edge_camera != camera_key {
            let mut data = Vec::new();
            for edge in self
                .edges
                .iter()
                .filter(|edge| edge_visible(edge, rotation[2]))
            {
                for point in edge.points {
                    data.extend(point);
                    data.extend([0.0; 6]);
                }
            }
            self.edge_count = (data.len() / 9) as u32;
            if !data.is_empty() {
                self.edge_vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Model creases and visible silhouettes"),
                    contents: &bytes(&data),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            }
            self.edge_camera = camera_key;
        }
        let scale = size[0].min(size[1]) as f32 * 0.7 / extent * zoom;
        let factor = [
            2.0 * scale / size[0] as f32,
            2.0 * scale / size[1] as f32,
            -1.0 / (32.0 * extent),
        ];
        let mut matrix = [0.0_f32; 16];
        let mut normal_matrix = [0.0_f32; 16];
        for row in 0..3 {
            for col in 0..3 {
                matrix[col * 4 + row] = rotation[row][col] * factor[row];
                normal_matrix[col * 4 + row] = rotation[row][col];
            }
            matrix[12 + row] = -rotation[row]
                .iter()
                .zip(middle)
                .map(|(r, m)| r * m)
                .sum::<f32>()
                * factor[row]
                + if row == 2 { 0.5 } else { 0.0 };
        }
        matrix[15] = 1.0;
        normal_matrix[15] = 1.0;
        let mut uniforms = matrix.to_vec();
        uniforms.extend(normal_matrix);
        self.state
            .queue
            .write_buffer(&self.uniform, 0, &bytes(&uniforms));
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Gridfinity preview pass"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Solid geometry"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.targets.multisample_view,
                    depth_slice: None,
                    resolve_target: Some(&self.targets.view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(background.r()) / 255.0,
                            g: f64::from(background.g()) / 255.0,
                            b: f64::from(background.b()) / 255.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.targets.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind, &[]);
            pass.set_vertex_buffer(0, self.vertices.slice(..));
            pass.draw(0..self.count, 0..1);
            pass.set_pipeline(&self.edge_pipeline);
            pass.set_vertex_buffer(0, self.edge_vertices.slice(..));
            pass.draw(0..self.edge_count, 0..1);
        }
        self.state.queue.submit([encoder.finish()]);
        self.texture_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ultrawide_render_target_preserves_the_requested_aspect_ratio() {
        assert_eq!(render_size([5120, 1440]), [4096, 1152]);
        assert_eq!(render_size([1440, 5120]), [1152, 4096]);
        assert_eq!(render_size([1920, 1080]), [1920, 1080]);
        assert_eq!(render_size([24, 12]), [64, 32]);
        assert_eq!(render_size([0, 0]), [32, 32]);
    }

    #[test]
    fn planar_mesh_diagonal_is_not_a_visible_feature() {
        let vertices = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let (_, edges) = mesh_data(
            &vertices,
            &[([0, 1, 2], [0; 3]), ([0, 2, 3], [0; 3])],
            false,
        );
        assert_eq!(edges.iter().filter(|e| e.crease).count(), 4);
        let diagonal = edges.iter().find(|e| !e.crease).unwrap();
        assert!(!edge_visible(diagonal, [0.2, 0.4, 0.8]));
        assert!(!edge_visible(diagonal, [0.0, 1.0, 0.0]));
    }

    #[test]
    fn rounded_surface_edge_is_drawn_only_at_the_silhouette() {
        let angle = 10_f32.to_radians();
        let edge = Edge {
            points: [[0.0; 3], [1.0, 0.0, 0.0]],
            normals: vec![[0.0, 0.0, 1.0], [0.0, -angle.sin(), angle.cos()]],
            crease: false,
        };
        assert!(!edge_visible(&edge, [0.0, 0.0, 1.0]));
        assert!(edge_visible(&edge, [0.0, 1.0, 0.08]));
    }

    #[test]
    fn ground_grid_keeps_its_colour_without_outlines() {
        let (data, edges) = mesh_data(
            &[[0.0, 0.0, -0.03], [1.0, 0.0, -0.03], [0.0, 1.0, -0.03]],
            &[([0, 1, 2], [36, 48, 59])],
            false,
        );
        assert!(edges.is_empty());
        assert!(data.as_chunks::<9>().0.iter().all(|v| v[3..6] == [0.0; 3]));
    }
}
