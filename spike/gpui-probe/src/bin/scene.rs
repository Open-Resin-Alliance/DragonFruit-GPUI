//! Scene probe: several models, instanced supports, and a stencil cross-section.
//!
//! Run with `cargo run --bin scene --release`.
//!
//! This is the shape a real DragonFruit viewport has that the single-mesh bench
//! does not:
//!
//! - four models drawn from one vertex/index buffer with per-instance matrices,
//! - 20,000 instanced support cylinders,
//! - a cross-section at `z = 0` implemented the way the app does it: two
//!   stencil passes over the clipped models, then a cap quad where the stencil
//!   is non-zero,
//! - depth testing throughout, and a GPU-waited frame loop for the number.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3 as GVec3};
use gpui::{
    App, Bounds, Context, Corners, IntoElement, Render, Window, WindowBounds, WindowOptions, div,
    gpu_canvas, prelude::*, px, rgb, size,
};
use gpui_interop::{WgpuSurface, attach};
use gpui_probe::{Vertex, sphere};
use gpui_wgpu::{ImportedTextureExt as _, WgpuRenderer};
use wgpu::util::DeviceExt as _;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const MODELS: u32 = 4;
const MODEL_RINGS: u32 = 600;
const MODEL_SEGMENTS: u32 = 600;
const SUPPORTS: u32 = 20_000;
const FRAMES: usize = 240;
const WARMUP: usize = 10;
const CLIP_Z: f32 = 0.0;

fn main() {
    let interactive = std::env::args().any(|arg| arg == "--interactive");
    gpui::application().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(960.), px(640.)), cx);
        let options = gpui_probe::with_wgpu_renderer(WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        });
        cx.open_window(options, move |_, cx| cx.new(|_| SceneProbe::new(interactive)))
            .expect("a window");
        cx.activate(true);
    });
}

struct SceneProbe {
    state: Rc<RefCell<Option<SceneState>>>,
    status: Rc<RefCell<String>>,
    angle: Rc<Cell<f32>>,
    last_x: Rc<Cell<f32>>,
    interactive: bool,
}

impl SceneProbe {
    fn new(interactive: bool) -> Self {
        Self {
            state: Rc::new(RefCell::new(None)),
            status: Rc::new(RefCell::new("waiting for the first paint".to_owned())),
            angle: Rc::new(Cell::new(0.0)),
            last_x: Rc::new(Cell::new(0.0)),
            interactive,
        }
    }
}

/// Project a world point to canvas pixels, using the same matrices the draw uses.
fn project(point: GVec3, angle: f32) -> (f32, f32) {
    let proj = Mat4::perspective_rh(45f32.to_radians(), WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
    let view = Mat4::look_at_rh(GVec3::new(0.0, 0.0, 6.0), GVec3::ZERO, GVec3::Y);
    let view_proj = proj * view * Mat4::from_rotation_z(angle * 0.2);
    let clip = view_proj * point.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    (
        (ndc.x * 0.5 + 0.5) * WIDTH as f32,
        (0.5 - ndc.y * 0.5) * HEIGHT as f32,
    )
}

impl Render for SceneProbe {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.clone();
        let status = self.status.clone();
        let angle = self.angle.clone();
        let last_x = self.last_x.clone();
        let interactive = self.interactive;
        let text = self.status.borrow().clone();

        let canvas_state = state.clone();
        let canvas_status = status.clone();
        let canvas_angle = angle.clone();
        let mut viewport = div()
            .relative()
            .w(px(WIDTH as f32))
            .h(px(HEIGHT as f32))
            .child(
                gpu_canvas(move |gpu| {
                    let mut slot = canvas_state.borrow_mut();
                    let interop = match attach(gpu.try_device::<WgpuRenderer>()) {
                        Ok(interop) => interop,
                        Err(error) => {
                            *canvas_status.borrow_mut() = format!("not wgpu: {error:?}");
                            return;
                        }
                    };
                    let Some((device, queue)) = interop.adapter().wgpu() else {
                        *canvas_status.borrow_mut() = "no wgpu device".to_owned();
                        return;
                    };
                    if slot.is_none() {
                        match SceneState::new(&device, &queue, !interactive) {
                            Ok(built) => {
                                let report = built.report.clone();
                                eprintln!("{report}");
                                *canvas_status.borrow_mut() = report;
                                *slot = Some(built);
                            }
                            Err(error) => {
                                let report = format!("scene: {error:#}");
                                eprintln!("{report}");
                                *canvas_status.borrow_mut() = report;
                                return;
                            }
                        }
                    }
                    let state = slot.as_mut().expect("built above");
                    state.angle = canvas_angle.get();
                    if let Err(error) = state.render(&device, &queue) {
                        *canvas_status.borrow_mut() = format!("render: {error:#}");
                        return;
                    }
                    match state.surface.view().to_imported_handle() {
                        Ok(handle) => {
                            gpu.paint_texture(handle, Corners::default(), 1.0, false);
                            if state.bench && !state.exiting {
                                state.exiting = true;
                                std::thread::spawn(|| {
                                    std::thread::sleep(Duration::from_millis(800));
                                    std::process::exit(0);
                                });
                            }
                        }
                        Err(error) => {
                            *canvas_status.borrow_mut() = format!("import: {error:#}");
                        }
                    }
                })
                .size_full(),
            );

        if interactive {
            let (x, y) = project(GVec3::new(-1.2, -1.2, 0.0), angle.get());
            viewport = viewport.child(
                div()
                    .absolute()
                    .left(px(x))
                    .top(px(y))
                    .bg(rgb(0x1a2230))
                    .border_1()
                    .border_color(rgb(0x3a4a60))
                    .rounded_md()
                    .px_2()
                    .py_1()
                    .text_xs()
                    .text_color(rgb(0xe6edf5))
                    .child("Model 0 · label"),
            );

            // A screen-space gizmo: three bars at the projected origin.
            let (ox, oy) = project(GVec3::ZERO, angle.get());
            for (w, h, color) in [
                (28.0, 2.0, rgb(0xe05252)),
                (2.0, 28.0, rgb(0x52d07a)),
                (20.0, 20.0, rgb(0x5a7cff)),
            ] {
                viewport = viewport.child(
                    div()
                        .absolute()
                        .left(px(ox))
                        .top(px(oy))
                        .w(px(w))
                        .h(px(h))
                        .bg(color),
                );
            }

            let drag_angle = angle.clone();
            let drag_last = last_x.clone();
            viewport = viewport.on_mouse_move(move |event, window, _cx| {
                if event.dragging() {
                    let dx = event.position.x.0 - drag_last.get();
                    drag_angle.set(drag_angle.get() + dx * 0.01);
                    window.request_animation_frame();
                }
                drag_last.set(event.position.x.0);
            });
        }

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
                    .child("DragonFruit · GPUI scene probe"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x9aa4b2))
                    .child(text),
            )
            .child(viewport)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct SceneUniforms {
    view_proj: [[f32; 4]; 4],
    clip_z: f32,
    _pad: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Instance {
    model: [[f32; 4]; 4],
    color: [f32; 4],
}

struct SceneState {
    surface: WgpuSurface,
    scene_texture: wgpu::Texture,
    scene_view: wgpu::TextureView,
    model_back: wgpu::RenderPipeline,
    model_front: wgpu::RenderPipeline,
    cap: wgpu::RenderPipeline,
    supports: wgpu::RenderPipeline,
    model_vertices: wgpu::Buffer,
    model_indices: wgpu::Buffer,
    model_index_count: u32,
    _model_matrices: wgpu::Buffer,
    cylinder_vertices: wgpu::Buffer,
    cylinder_indices: wgpu::Buffer,
    cylinder_index_count: u32,
    instances: wgpu::Buffer,
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    depth_view: wgpu::TextureView,
    angle: f32,
    report: String,
    exiting: bool,
    bench: bool,
}

impl SceneState {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, bench: bool) -> Result<Self> {
        let started = Instant::now();
        let (vertices, indices, _mesh) = sphere(MODEL_RINGS, MODEL_SEGMENTS);
        let (cylinder_vertices, cylinder_indices) = cylinder(12);

        let model_vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scene model vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let model_indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scene model indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let cylinder_vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scene cylinder vertices"),
            contents: bytemuck::cast_slice(&cylinder_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let cylinder_indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scene cylinder indices"),
            contents: bytemuck::cast_slice(&cylinder_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let model_matrices_data: Vec<[[f32; 4]; 4]> = (0..MODELS)
            .map(|i| {
                let x = if i % 2 == 0 { -1.2 } else { 1.2 };
                let y = if i < 2 { -1.2 } else { 1.2 };
                Mat4::from_translation(GVec3::new(x, y, 0.0)).to_cols_array_2d()
            })
            .collect();
        let model_matrices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scene model matrices"),
            contents: bytemuck::cast_slice(&model_matrices_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let instance_data = support_instances(SUPPORTS);
        let instances = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scene support instances"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let surface = WgpuSurface::new(device, WIDTH, HEIGHT)?;
        // Rendered here first so a verification frame can be read back; the
        // surface itself has no COPY_SRC.
        let scene_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scene target"),
            size: wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: WgpuSurface::DEFAULT_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let scene_view = scene_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let model_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene model"),
            source: wgpu::ShaderSource::Wgsl(MODEL_SHADER.into()),
        });
        let cap_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene cap"),
            source: wgpu::ShaderSource::Wgsl(CAP_SHADER.into()),
        });
        let support_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene supports"),
            source: wgpu::ShaderSource::Wgsl(SUPPORT_SHADER.into()),
        });

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("scene"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene uniforms"),
            size: std::mem::size_of::<SceneUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: model_matrices.as_entire_binding(),
                },
            ],
        });

        let model_layout = wgpu::VertexBufferLayout {
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
        };

        let stencil_face = |op| wgpu::StencilFaceState {
            compare: wgpu::CompareFunction::Always,
            fail_op: wgpu::StencilOperation::Keep,
            depth_fail_op: wgpu::StencilOperation::Keep,
            pass_op: op,
        };

        let model_back = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scene model back"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &model_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(model_layout.clone())],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &model_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: WgpuSurface::DEFAULT_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::empty(),
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Front),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState {
                    front: stencil_face(wgpu::StencilOperation::IncrementWrap),
                    back: stencil_face(wgpu::StencilOperation::IncrementWrap),
                    read_mask: 0xff,
                    write_mask: 0xff,
                },
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let model_front = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scene model front"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &model_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(model_layout)],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &model_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: WgpuSurface::DEFAULT_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::empty(),
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState {
                    front: stencil_face(wgpu::StencilOperation::DecrementWrap),
                    back: stencil_face(wgpu::StencilOperation::DecrementWrap),
                    read_mask: 0xff,
                    write_mask: 0xff,
                },
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let cap = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scene cap"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &cap_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &cap_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: WgpuSurface::DEFAULT_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState {
                    front: wgpu::StencilFaceState {
                        compare: wgpu::CompareFunction::NotEqual,
                        fail_op: wgpu::StencilOperation::Keep,
                        depth_fail_op: wgpu::StencilOperation::Keep,
                        pass_op: wgpu::StencilOperation::Keep,
                    },
                    back: wgpu::StencilFaceState {
                        compare: wgpu::CompareFunction::NotEqual,
                        fail_op: wgpu::StencilOperation::Keep,
                        depth_fail_op: wgpu::StencilOperation::Keep,
                        pass_op: wgpu::StencilOperation::Keep,
                    },
                    read_mask: 0xff,
                    write_mask: 0x00,
                },
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let supports = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scene supports"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &support_shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    Some(wgpu::VertexBufferLayout {
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
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Instance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 0,
                                shader_location: 2,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 16,
                                shader_location: 3,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 32,
                                shader_location: 4,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 48,
                                shader_location: 5,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 64,
                                shader_location: 6,
                            },
                        ],
                    }),
                ],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &support_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: WgpuSurface::DEFAULT_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scene depth"),
            size: wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24PlusStencil8,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());

        let mut state = Self {
            surface,
            scene_texture,
            scene_view,
            model_back,
            model_front,
            cap,
            supports,
            model_vertices,
            model_indices,
            model_index_count: indices.len() as u32,
            _model_matrices: model_matrices,
            cylinder_vertices,
            cylinder_indices,
            cylinder_index_count: cylinder_index_count(),
            instances,
            uniform,
            bind_group,
            depth_view,
            angle: 0.0,
            report: String::new(),
            exiting: false,
            bench,
        };

        let setup = started.elapsed();

        if bench {
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

            let verification = state.verify(device, queue)?;

            state.report = format!(
                "{} models × {} tris + {} instanced supports ({} tris each)\n\
                 setup {:.0} ms · {} frames: mean {:.2} ms ({:.1} fps) · p50 {:.2} ms · p95 {:.2} ms\n\
                 {}",
                MODELS,
                MODEL_RINGS * MODEL_SEGMENTS * 2,
                SUPPORTS,
                12 * 2,
                setup.as_secs_f64() * 1000.0,
                FRAMES,
                mean.as_secs_f64() * 1000.0,
                1.0 / mean.as_secs_f64(),
                p50.as_secs_f64() * 1000.0,
                p95.as_secs_f64() * 1000.0,
                verification,
            );
        } else {
            state.report = format!(
                "interactive · {} models × {} tris + {} supports · drag to rotate",
                MODELS,
                MODEL_RINGS * MODEL_SEGMENTS * 2,
                SUPPORTS
            );
        }
        Ok(state)
    }

    fn render(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<()> {
        self.draw(device, queue, false)
    }

    /// Read the last frame back and count the pixels the stencil cap, the
    /// models and the supports should have produced. A frame time says nothing
    /// about whether the cap drew; this does.
    fn verify(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<String> {
        let unpadded = WIDTH * 4;
        let padded = unpadded.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene readback"),
            size: (padded * HEIGHT) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("scene readback"),
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.scene_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(HEIGHT),
                },
            },
            wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));

        let slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv()??;
        let data = slice.get_mapped_range()?;

        let (mut cap, mut model, mut support) = (0usize, 0usize, 0usize);
        for row in 0..HEIGHT as usize {
            let start = row * padded as usize;
            for px in 0..WIDTH as usize {
                let i = start + px * 4;
                let r = data[i] as i32;
                let g = data[i + 1] as i32;
                let b = data[i + 2] as i32;
                if r > 200 && (120..=215).contains(&g) && b < 160 {
                    cap += 1;
                } else if b > r + 20 {
                    model += 1;
                } else if r > 180 && g > 160 && b > 100 {
                    support += 1;
                }
            }
        }
        drop(data);
        buffer.unmap();
        Ok(format!(
            "readback: cap {cap} px · model {model} px · supports {support} px"
        ))
    }

    fn draw(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, wait: bool) -> Result<()> {
        let proj = Mat4::perspective_rh(45f32.to_radians(), WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
        let view = Mat4::look_at_rh(GVec3::new(0.0, 0.0, 6.0), GVec3::ZERO, GVec3::Y);
        let view_proj = proj * view * Mat4::from_rotation_z(self.angle * 0.2);
        queue.write_buffer(
            &self.uniform,
            0,
            bytemuck::bytes_of(&SceneUniforms {
                view_proj: view_proj.to_cols_array_2d(),
                clip_z: CLIP_Z,
                _pad: [0.0; 3],
            }),
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("scene frame"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.scene_view,
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
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: wgpu::StoreOp::Store,
                    }),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.model_vertices.slice(..));
            pass.set_index_buffer(self.model_indices.slice(..), wgpu::IndexFormat::Uint32);

            pass.set_pipeline(&self.model_back);
            pass.draw_indexed(0..self.model_index_count, 0, 0..MODELS);
            pass.set_pipeline(&self.model_front);
            pass.draw_indexed(0..self.model_index_count, 0, 0..MODELS);

            pass.set_stencil_reference(0);
            pass.set_pipeline(&self.cap);
            pass.draw(0..6, 0..1);

            pass.set_pipeline(&self.supports);
            pass.set_vertex_buffer(0, self.cylinder_vertices.slice(..));
            pass.set_vertex_buffer(1, self.instances.slice(..));
            pass.set_index_buffer(self.cylinder_indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..self.cylinder_index_count, 0, 0..SUPPORTS);
        }
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.scene_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: self.surface.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));
        if wait {
            let _ = device.poll(wgpu::PollType::wait_indefinitely());
        }
        Ok(())
    }
}

fn cylinder_index_count() -> u32 {
    // 12 sides, two triangles each, no caps.
    12 * 6
}

/// A 12-sided cylinder along Y, radius 1, height 1, centred on the origin.
fn cylinder(sides: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::with_capacity((sides * 2) as usize);
    let mut indices = Vec::with_capacity((sides * 6) as usize);
    for i in 0..sides {
        let a = std::f32::consts::TAU * i as f32 / sides as f32;
        let b = std::f32::consts::TAU * (i + 1) as f32 / sides as f32;
        let (sa, ca) = a.sin_cos();
        let (sb, cb) = b.sin_cos();
        let base = vertices.len() as u32;
        vertices.push(Vertex {
            pos: [ca, -0.5, sa],
            normal: [ca, 0.0, sa],
        });
        vertices.push(Vertex {
            pos: [ca, 0.5, sa],
            normal: [ca, 0.0, sa],
        });
        vertices.push(Vertex {
            pos: [cb, -0.5, sb],
            normal: [cb, 0.0, sb],
        });
        vertices.push(Vertex {
            pos: [cb, 0.5, sb],
            normal: [cb, 0.0, sb],
        });
        indices.extend_from_slice(&[base, base + 1, base + 2]);
        indices.extend_from_slice(&[base + 2, base + 1, base + 3]);
    }
    (vertices, indices)
}

/// Support instances: thin cylinders scattered under the four models.
fn support_instances(count: u32) -> Vec<Instance> {
    let mut instances = Vec::with_capacity(count as usize);
    let mut seed = 0x9e37_79b9u32;
    for _ in 0..count {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let x = ((seed >> 8) as f32 / 16_777_216.0 - 0.5) * 4.0;
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let y = ((seed >> 8) as f32 / 16_777_216.0 - 0.5) * 4.0;
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let z = ((seed >> 8) as f32 / 16_777_216.0 - 0.5) * 2.0;
        let model = Mat4::from_scale_rotation_translation(
            GVec3::new(0.012, 0.16, 0.012),
            glam::Quat::IDENTITY,
            GVec3::new(x, y, z),
        );
        instances.push(Instance {
            model: model.to_cols_array_2d(),
            color: [0.85, 0.75, 0.35, 1.0],
        });
    }
    instances
}

const MODEL_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    clip_z: f32,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> models: array<mat4x4<f32>, 4>;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @builtin(instance_index) instance: u32,
) -> VertexOut {
    let model = models[instance];
    let world = model * vec4<f32>(position, 1.0);
    var out: VertexOut;
    out.position = uniforms.view_proj * world;
    out.world = world.xyz;
    out.normal = (model * vec4<f32>(normal, 0.0)).xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    if (in.world.z > uniforms.clip_z) {
        discard;
    }
    let light = normalize(vec3<f32>(0.4, 0.8, 0.5));
    let diffuse = max(dot(normalize(in.normal), light), 0.0);
    return vec4<f32>(vec3<f32>(0.35, 0.55, 0.85) * (0.25 + 0.75 * diffuse), 1.0);
}
"#;

const CAP_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    clip_z: f32,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-3.0, -3.0), vec2<f32>(3.0, -3.0), vec2<f32>(3.0, 3.0),
        vec2<f32>(-3.0, -3.0), vec2<f32>(3.0, 3.0), vec2<f32>(-3.0, 3.0),
    );
    return uniforms.view_proj * vec4<f32>(corners[index], uniforms.clip_z, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.95, 0.55, 0.20, 1.0);
}
"#;

const SUPPORT_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    clip_z: f32,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) m0: vec4<f32>,
    @location(3) m1: vec4<f32>,
    @location(4) m2: vec4<f32>,
    @location(5) m3: vec4<f32>,
    @location(6) color: vec4<f32>,
) -> VertexOut {
    let model = mat4x4<f32>(m0, m1, m2, m3);
    let world = model * vec4<f32>(position, 1.0);
    var out: VertexOut;
    out.position = uniforms.view_proj * world;
    out.world = world.xyz;
    out.normal = (model * vec4<f32>(normal, 0.0)).xyz;
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    if (in.world.z > uniforms.clip_z) {
        discard;
    }
    let light = normalize(vec3<f32>(0.4, 0.8, 0.5));
    let diffuse = max(dot(normalize(in.normal), light), 0.0);
    return vec4<f32>(in.color.rgb * (0.3 + 0.7 * diffuse), 1.0);
}
"#;
