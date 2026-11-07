//! Throughput probe: a print-sized mesh, depth, a gizmo, and picking.
//!
//! Run with `cargo run --bin bench --release`.
//!
//! What it measures:
//!
//! 1. Upload of a ~2.9M-triangle sphere into vertex and index buffers.
//! 2. `dragonfruit-mesh-core`'s BVH build over the same mesh — the crate a
//!    GPUI port would reuse for picking.
//! 3. 10,000 ray queries against that BVH, the CPU picking cost.
//! 4. 240 frames of the mesh plus an always-on-top axis gizmo, with depth
//!    testing, each frame waited on the GPU so the number is real end-to-end
//!    frame time rather than submit latency.
//!
//! The sphere is procedural: a real model has worse locality, but the triangle
//! count and the vertex-shader load are representative.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use dragonfruit_mesh_core::{bvh::Bvh, mesh::Vec3};
use glam::{Mat4, Vec3 as GVec3};
use gpui_probe::{Vertex, sphere};
use gpui::{
    App, Bounds, Context, Corners, IntoElement, Render, Window, WindowBounds, WindowOptions, div,
    gpu_canvas, prelude::*, px, rgb, size,
};
use gpui_interop::{WgpuSurface, attach};
use gpui_wgpu::{ImportedTextureExt as _, WgpuRenderer};
use wgpu::util::DeviceExt as _;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const RINGS: u32 = 1200;
const SEGMENTS: u32 = 1200;
const FRAMES: usize = 240;
const WARMUP: usize = 10;
const RAYS: usize = 10_000;

fn main() {
    gpui::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(960.), px(640.)), cx);
        let options = gpui_probe::with_wgpu_renderer(WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        });
        cx.open_window(options, |_, cx| cx.new(|_| BenchProbe::new()))
            .expect("a window");
        cx.activate(true);
    });
}

struct BenchProbe {
    state: Rc<RefCell<Option<BenchState>>>,
    status: Rc<RefCell<String>>,
}

impl BenchProbe {
    fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(None)),
            status: Rc::new(RefCell::new("waiting for the first paint".to_owned())),
        }
    }
}

impl Render for BenchProbe {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.clone();
        let status = self.status.clone();
        let text = self.status.borrow().clone();

        div()
            .size_full()
            .bg(rgb(0x0b0f14))
            .flex()
            .flex_col()
            .gap_4()
            .p_8()
            .child(
                div()
                    .text_2xl()
                    .text_color(rgb(0xffffff))
                    .child("DragonFruit · GPUI mesh benchmark"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x9aa4b2))
                    .child(text),
            )
            .child(
                div()
                    .w(px(WIDTH as f32))
                    .h(px(HEIGHT as f32))
                    .child(
                        gpu_canvas(move |gpu| {
                            let mut slot = state.borrow_mut();
                            let interop = match attach(gpu.try_device::<WgpuRenderer>()) {
                                Ok(interop) => interop,
                                Err(error) => {
                                    *status.borrow_mut() = format!("not wgpu: {error:?}");
                                    return;
                                }
                            };
                            let Some((device, queue)) = interop.adapter().wgpu() else {
                                *status.borrow_mut() = "no wgpu device".to_owned();
                                return;
                            };
                            if slot.is_none() {
                                match BenchState::new(&device, &queue) {
                                    Ok(built) => {
                                        let report = built.report.clone();
                                        eprintln!("{report}");
                                        *status.borrow_mut() = report;
                                        *slot = Some(built);
                                    }
                                    Err(error) => {
                                        let report = format!("bench: {error:#}");
                                        eprintln!("{report}");
                                        *status.borrow_mut() = report;
                                        return;
                                    }
                                }
                            }
                            let state = slot.as_mut().expect("built above");
                            if let Err(error) = state.render(&device, &queue) {
                                *status.borrow_mut() = format!("render: {error:#}");
                                return;
                            }
                            match state.surface.view().to_imported_handle() {
                                Ok(handle) => {
                                    gpu.paint_texture(handle, Corners::default(), 1.0, false);
                                    // The bench prints its report before this frame; give the
                                    // window a moment to present it, then leave. A benchmark
                                    // that waits for a user to close it cannot run unattended.
                                    if !state.exiting {
                                        state.exiting = true;
                                        std::thread::spawn(|| {
                                            std::thread::sleep(Duration::from_millis(800));
                                            std::process::exit(0);
                                        });
                                    }
                                }
                                Err(error) => {
                                    *status.borrow_mut() = format!("import: {error:#}");
                                }
                            }
                        })
                        .size_full(),
                    ),
            )
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniforms {
    mvp: [[f32; 4]; 4],
}

struct BenchState {
    surface: WgpuSurface,
    pipeline: wgpu::RenderPipeline,
    gizmo_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    depth_view: wgpu::TextureView,
    angle: f32,
    report: String,
    exiting: bool,
}

impl BenchState {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Self> {
        let started = Instant::now();
        let (vertices, indices, mesh) = sphere(RINGS, SEGMENTS);
        let generation = started.elapsed();

        let upload_started = Instant::now();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bench vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bench indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let upload = upload_started.elapsed();

        let bvh_started = Instant::now();
        let bvh = Bvh::build(&mesh);
        let bvh_build = bvh_started.elapsed();

        // Picking: rays from a shell of radius 2 toward the centre.
        let ray_started = Instant::now();
        let mut hits = 0usize;
        let mut seed = 0x1234_5678u32;
        for _ in 0..RAYS {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let a = (seed >> 8) as f32 / 16_777_216.0 * std::f32::consts::TAU;
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let b = (seed >> 8) as f32 / 16_777_216.0 * std::f32::consts::PI;
            let origin = Vec3::new(
                2.0 * b.sin() * a.cos(),
                2.0 * b.cos(),
                2.0 * b.sin() * a.sin(),
            );
            let dir = Vec3::new(-origin.x, -origin.y, -origin.z);
            if bvh.ray_nearest_within(&mesh, origin, dir, 10.0, 0.0).is_some() {
                hits += 1;
            }
        }
        let rays = ray_started.elapsed();

        let surface = WgpuSurface::new(device, WIDTH, HEIGHT)?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bench mesh"),
            source: wgpu::ShaderSource::Wgsl(MESH_SHADER.into()),
        });
        let gizmo_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bench gizmo"),
            source: wgpu::ShaderSource::Wgsl(GIZMO_SHADER.into()),
        });

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("bench uniforms"),
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
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bench"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bench uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bench uniforms"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bench mesh"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
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
                    ],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: WgpuSurface::DEFAULT_FORMAT,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let gizmo_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bench gizmo"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &gizmo_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &gizmo_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: WgpuSurface::DEFAULT_FORMAT,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bench depth"),
            size: wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());

        let mut state = Self {
            surface,
            pipeline,
            gizmo_pipeline,
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            uniform,
            bind_group,
            depth_view,
            angle: 0.0,
            report: String::new(),
            exiting: false,
        };

        // Warm up: first submissions pay pipeline and buffer residency.
        for _ in 0..WARMUP {
            state.angle += 0.01;
            state.draw(device, queue, true)?;
        }

        let mut samples = Vec::with_capacity(FRAMES);
        for _ in 0..FRAMES {
            state.angle += 0.01;
            let started = Instant::now();
            state.draw(device, queue, true)?;
            samples.push(started.elapsed());
        }

        samples.sort_unstable();
        let mean = samples.iter().sum::<Duration>() / samples.len() as u32;
        let p50 = samples[samples.len() / 2];
        let p95 = samples[samples.len() * 95 / 100];
        let fps = 1.0 / mean.as_secs_f64();

        state.report = format!(
            "mesh {} tris / {} verts · gen {:.0} ms · upload {:.0} ms (V {:.1} MB, I {:.1} MB)\n\
             bvh build {:.0} ms · {} rays {:.1} ms ({:.0} ns/ray, {} hits)\n\
             {} frames: mean {:.2} ms ({:.1} fps) · p50 {:.2} ms · p95 {:.2} ms",
            mesh.triangles.len(),
            mesh.positions.len(),
            generation.as_secs_f64() * 1000.0,
            upload.as_secs_f64() * 1000.0,
            (vertices.len() * std::mem::size_of::<Vertex>()) as f64 / 1e6,
            (indices.len() * 4) as f64 / 1e6,
            bvh_build.as_secs_f64() * 1000.0,
            RAYS,
            rays.as_secs_f64() * 1000.0,
            rays.as_secs_f64() * 1e9 / RAYS as f64,
            hits,
            FRAMES,
            mean.as_secs_f64() * 1000.0,
            fps,
            p50.as_secs_f64() * 1000.0,
            p95.as_secs_f64() * 1000.0,
        );
        Ok(state)
    }

    fn render(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<()> {
        self.angle += 0.005;
        self.draw(device, queue, false)
    }

    fn draw(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, wait: bool) -> Result<()> {
        let proj = Mat4::perspective_rh(45f32.to_radians(), WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
        let view = Mat4::look_at_rh(GVec3::new(0.0, 0.0, 3.2), GVec3::ZERO, GVec3::Y);
        let model = Mat4::from_rotation_y(self.angle);
        let mvp = proj * view * model;
        queue.write_buffer(
            &self.uniform,
            0,
            bytemuck::bytes_of(&Uniforms {
                mvp: mvp.to_cols_array_2d(),
            }),
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("bench frame"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bench frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.surface.view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.02,
                            g: 0.03,
                            b: 0.05,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_pipeline(&self.pipeline);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..self.index_count, 0, 0..1);
            pass.set_pipeline(&self.gizmo_pipeline);
            pass.draw(0..6, 0..1);
        }
        queue.submit(Some(encoder.finish()));
        if wait {
            let _ = device.poll(wgpu::PollType::wait_indefinitely());
        }
        Ok(())
    }
}

const MESH_SHADER: &str = r#"
struct Uniforms {
    mvp: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
};

@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) normal: vec3<f32>) -> VertexOut {
    var out: VertexOut;
    out.position = uniforms.mvp * vec4<f32>(position, 1.0);
    out.normal = normal;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let light = normalize(vec3<f32>(0.4, 0.8, 0.5));
    let diffuse = max(dot(normalize(in.normal), light), 0.0);
    return vec4<f32>(vec3<f32>(0.35, 0.55, 0.85) * (0.25 + 0.75 * diffuse), 1.0);
}
"#;

const GIZMO_SHADER: &str = r#"
struct Uniforms {
    mvp: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOut {
    var positions = array<vec3<f32>, 6>(
        vec3<f32>(-1.0, 0.0, 0.0), vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, -1.0, 0.0), vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.0, 0.0, -1.0), vec3<f32>(0.0, 0.0, 1.0),
    );
    var colors = array<vec3<f32>, 3>(
        vec3<f32>(1.0, 0.2, 0.2),
        vec3<f32>(0.2, 1.0, 0.2),
        vec3<f32>(0.3, 0.4, 1.0),
    );
    var out: VertexOut;
    out.position = uniforms.mvp * vec4<f32>(positions[index], 1.0);
    out.color = colors[index / 2u];
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
"#;
