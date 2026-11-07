//! The wgpu viewport: background gradient, build-plate grid, footprint outline,
//! flat-shaded clay mesh with GGX specular, SSAO, MSAA, and the app's mood
//! overlay.

use anyhow::Result;
use gpui_interop::WgpuSurface;
use wgpu::util::DeviceExt as _;

use crate::camera::Camera;
use crate::stl::{MeshData, Vertex};

const SAMPLE_COUNT: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    inv_proj: [[f32; 4]; 4],
    camera_pos: [f32; 3],
    _pad0: f32,
    base_color: [f32; 3],
    _pad1: f32,
    resolution: [f32; 2],
    _pad2: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct OverlayUniform {
    value: f32,
    _pad: [f32; 3],
}

struct GpuMesh {
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
    footprint_buffer: wgpu::Buffer,
    footprint_vertex_count: u32,
}

struct Targets {
    _scene: wgpu::Texture,
    scene_view: wgpu::TextureView,
    _msaa_color: wgpu::Texture,
    msaa_color_view: wgpu::TextureView,
    _msaa_depth: wgpu::Texture,
    msaa_depth_view: wgpu::TextureView,
    final_texture: wgpu::Texture,
    final_view: wgpu::TextureView,
    composite_bind: wgpu::BindGroup,
}

#[derive(Clone, Copy)]
pub struct ViewLayers {
    pub grid: bool,
    pub plate: bool,
    pub volume: bool,
}

pub struct Viewport {
    surface: WgpuSurface,
    background: wgpu::RenderPipeline,
    plate: wgpu::RenderPipeline,
    plate_vertex_buffer: wgpu::Buffer,
    plate_index_buffer: wgpu::Buffer,
    plate_index_count: u32,
    volume: wgpu::RenderPipeline,
    volume_buffer: wgpu::Buffer,
    volume_vertex_count: u32,
    axis: wgpu::RenderPipeline,
    axis_buffer: wgpu::Buffer,
    axis_vertex_count: u32,
    plate_uniform: wgpu::Buffer,
    plate_bind: wgpu::BindGroup,
    decal: wgpu::RenderPipeline,
    decal_buffer: wgpu::Buffer,
    decal_uniform: wgpu::Buffer,
    decal_bind: wgpu::BindGroup,
    front_bind: wgpu::BindGroup,
    logo_bind: wgpu::BindGroup,
    grid: wgpu::RenderPipeline,
    footprint: wgpu::RenderPipeline,
    mesh_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    scene_bind: wgpu::BindGroup,
    composite_layout: wgpu::BindGroupLayout,
    linear_sampler: wgpu::Sampler,
    targets: Targets,
    size: (u32, u32),
    mesh: Option<GpuMesh>,
    mesh_ptr: usize,
    support_buffer: wgpu::Buffer,
    support_vertex_count: u32,
    support_ptr: usize,
    model_uniform: wgpu::Buffer,
    model_bind: wgpu::BindGroup,
    gizmo_buffer: wgpu::Buffer,
    gizmo_vertex_count: u32,
    gizmo_origin: [f32; 3],
}

impl Viewport {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Self> {
        let surface = WgpuSurface::new(device, 1, 1)?;
        device.on_uncaptured_error(std::sync::Arc::new(|error| eprintln!("wgpu: {error}")));

        let mesh_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport mesh"),
            source: wgpu::ShaderSource::Wgsl(MESH_SHADER.into()),
        });
        let plate_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport plate"),
            source: wgpu::ShaderSource::Wgsl(PLATE_SHADER.into()),
        });
        let volume_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport volume"),
            source: wgpu::ShaderSource::Wgsl(VOLUME_SHADER.into()),
        });
        let axis_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport axis"),
            source: wgpu::ShaderSource::Wgsl(AXIS_SHADER.into()),
        });
        let decal_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport decal"),
            source: wgpu::ShaderSource::Wgsl(DECAL_SHADER.into()),
        });
        let grid_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport grid"),
            source: wgpu::ShaderSource::Wgsl(GRID_SHADER.into()),
        });
        let footprint_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport footprint"),
            source: wgpu::ShaderSource::Wgsl(FOOTPRINT_SHADER.into()),
        });
        let background_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport background"),
            source: wgpu::ShaderSource::Wgsl(BACKGROUND_SHADER.into()),
        });
        let composite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport composite"),
            source: wgpu::ShaderSource::Wgsl(COMPOSITE_SHADER.into()),
        });

        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("viewport uniform"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let composite_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("viewport composite"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let uniform_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("viewport scene"),
                bind_group_layouts: &[Some(&uniform_layout)],
                immediate_size: 0,
            });
        let composite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("viewport composite"),
                bind_group_layouts: &[Some(&composite_layout)],
                immediate_size: 0,
            });

        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("viewport uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let scene_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport scene"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });

        let decal_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("viewport decal"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("viewport decal texture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let decal_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("viewport decal"),
                bind_group_layouts: &[
                    Some(&uniform_layout),
                    Some(&decal_layout),
                    Some(&texture_layout),
                ],
                immediate_size: 0,
            });
        let decal_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("viewport decal uniform"),
            size: std::mem::size_of::<OverlayUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let decal_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport decal"),
            layout: &decal_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: decal_uniform.as_entire_binding(),
            }],
        });

        let vertex_layout = wgpu::VertexBufferLayout {
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
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 24,
                    shader_location: 2,
                },
            ],
        };
        let footprint_layout = wgpu::VertexBufferLayout {
            array_stride: 8,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            }],
        };

        let model_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("viewport model"),
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
        let mesh_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("viewport mesh"),
                bind_group_layouts: &[Some(&uniform_layout), Some(&model_layout)],
                immediate_size: 0,
            });
        let model_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("viewport model"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let model_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport model"),
            layout: &model_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: model_uniform.as_entire_binding(),
            }],
        });
        let mesh_pipeline = pipeline(
            device,
            &mesh_pipeline_layout,
            &mesh_shader,
            &[Some(vertex_layout.clone())],
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::Face::Back),
            true,
            wgpu::CompareFunction::Less,
            WgpuSurface::DEFAULT_FORMAT,
            true,
            true,
            SAMPLE_COUNT,
        );
        let grid = pipeline(
            device,
            &uniform_pipeline_layout,
            &grid_shader,
            &[],
            wgpu::PrimitiveTopology::TriangleList,
            None,
            true,
            wgpu::CompareFunction::Less,
            WgpuSurface::DEFAULT_FORMAT,
            true,
            true,
            SAMPLE_COUNT,
        );
        let footprint = pipeline(
            device,
            &uniform_pipeline_layout,
            &footprint_shader,
            &[Some(footprint_layout)],
            wgpu::PrimitiveTopology::LineList,
            None,
            false,
            wgpu::CompareFunction::LessEqual,
            WgpuSurface::DEFAULT_FORMAT,
            true,
            true,
            SAMPLE_COUNT,
        );
        let background = pipeline(
            device,
            &uniform_pipeline_layout,
            &background_shader,
            &[],
            wgpu::PrimitiveTopology::TriangleList,
            None,
            false,
            wgpu::CompareFunction::Always,
            WgpuSurface::DEFAULT_FORMAT,
            true,
            true,
            SAMPLE_COUNT,
        );
        let composite_pipeline = pipeline(
            device,
            &composite_pipeline_layout,
            &composite_shader,
            &[],
            wgpu::PrimitiveTopology::TriangleList,
            None,
            false,
            wgpu::CompareFunction::Always,
            WgpuSurface::DEFAULT_FORMAT,
            false,
            true,
            1,
        );

        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("viewport linear sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let (front_w, front_h, front_data) = front_texture();
        let front_tex = create_texture(device, queue, "front decal", front_w, front_h, &front_data);
        let front_view = front_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let front_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport front decal"),
            layout: &texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&front_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&linear_sampler),
                },
            ],
        });
        let (logo_w, logo_h, logo_data) = logo_texture();
        let logo_tex = create_texture(device, queue, "logo decal", logo_w, logo_h, &logo_data);
        let logo_view = logo_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let logo_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport logo decal"),
            layout: &texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&logo_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&linear_sampler),
                },
            ],
        });

        let plate_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("viewport plate"),
                bind_group_layouts: &[Some(&uniform_layout), Some(&decal_layout)],
                immediate_size: 0,
            });
        let plate_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("viewport plate uniform"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let plate_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport plate"),
            layout: &decal_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: plate_uniform.as_entire_binding(),
            }],
        });
        let plate = pipeline(
            device,
            &plate_pipeline_layout,
            &plate_shader,
            &[Some(wgpu::VertexBufferLayout {
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
            wgpu::PrimitiveTopology::TriangleList,
            None,
            true,
            wgpu::CompareFunction::Less,
            WgpuSurface::DEFAULT_FORMAT,
            true,
            true,
            SAMPLE_COUNT,
        );

        let volume = pipeline(
            device,
            &uniform_pipeline_layout,
            &volume_shader,
            &[Some(wgpu::VertexBufferLayout {
                array_stride: 12,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                }],
            })],
            wgpu::PrimitiveTopology::LineList,
            None,
            false,
            wgpu::CompareFunction::LessEqual,
            WgpuSurface::DEFAULT_FORMAT,
            true,
            true,
            SAMPLE_COUNT,
        );
        let axis = pipeline(
            device,
            &uniform_pipeline_layout,
            &axis_shader,
            &[Some(wgpu::VertexBufferLayout {
                array_stride: 24,
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
            wgpu::PrimitiveTopology::LineList,
            None,
            false,
            wgpu::CompareFunction::LessEqual,
            WgpuSurface::DEFAULT_FORMAT,
            true,
            true,
            SAMPLE_COUNT,
        );
        let axis_edges = axis_edges();
        let axis_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport axes"),
            contents: bytemuck::cast_slice(&axis_edges),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let decal = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("viewport decal"),
            layout: Some(&decal_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &decal_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 20,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 12,
                            shader_location: 1,
                        },
                    ],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &decal_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: WgpuSurface::DEFAULT_FORMAT,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
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
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: SAMPLE_COUNT,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });
        let decal_vertices = decal_vertices();
        let decal_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport decals"),
            contents: bytemuck::cast_slice(&decal_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let mut volume_edges = build_volume_edges(218.88, 122.88, 220.0);
        volume_edges.extend_from_slice(&plate_outline(218.88, 122.88, -0.07));
        let volume_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport volume"),
            contents: bytemuck::cast_slice(&volume_edges),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let (plate_vertices, plate_indices) = plate_mesh(224.88, 128.88, 3.0, -0.08, 3.0, 8);
        let plate_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport plate vertices"),
            contents: bytemuck::cast_slice(&plate_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let plate_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport plate indices"),
            contents: bytemuck::cast_slice(&plate_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let targets = Targets::new(
            device,
            1,
            1,
            &uniform,
            &composite_layout,
            &linear_sampler,
        );

        Ok(Self {
            surface,
            background,
            plate,
            plate_vertex_buffer,
            plate_index_buffer,
            plate_index_count: plate_indices.len() as u32,
            volume,
            volume_buffer,
            volume_vertex_count: volume_edges.len() as u32,
            axis,
            axis_buffer,
            axis_vertex_count: axis_edges.len() as u32,
            plate_uniform,
            plate_bind,
            decal,
            decal_buffer,
            decal_uniform,
            decal_bind,
            front_bind,
            logo_bind,
            grid,
            footprint,
            mesh_pipeline,
            composite_pipeline,
            uniform,
            scene_bind,
            composite_layout,
            linear_sampler,
            targets,
            size: (1, 1),
            mesh: None,
            mesh_ptr: 0,
            support_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("viewport supports"),
                size: 24,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            support_vertex_count: 0,
            support_ptr: 0,
            model_uniform,
            model_bind,
            gizmo_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("viewport gizmo"),
                size: 24,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            gizmo_vertex_count: 0,
            gizmo_origin: [f32::NAN; 3],
        })
    }

    pub fn surface(&self) -> &WgpuSurface {
        &self.surface
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        mesh: Option<&MeshData>,
        supports: &[[f32; 3]],
        layers: ViewLayers,
        model: glam::Mat4,
        gizmo: Option<[f32; 3]>,
        camera: &Camera,
        base_color: [f32; 3],
        width: u32,
        height: u32,
    ) -> Result<()> {
        let width = width.max(1);
        let height = height.max(1);
        if self.size != (width, height) {
            self.surface.resize(device, width, height)?;
            self.targets = Targets::new(
                device,
                width,
                height,
                &self.uniform,
                &self.composite_layout,
                &self.linear_sampler,
            );
            self.size = (width, height);
        }

        if let Some(mesh) = mesh {
            let ptr = mesh as *const MeshData as usize;
            if self.mesh_ptr != ptr {
                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("viewport vertices"),
                    contents: bytemuck::cast_slice(&mesh.vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let mut outline = Vec::with_capacity(mesh.footprint.len() * 2);
                for index in 0..mesh.footprint.len() {
                    outline.push(mesh.footprint[index]);
                    outline.push(mesh.footprint[(index + 1) % mesh.footprint.len()]);
                }
                let footprint_buffer =
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("viewport footprint"),
                        contents: bytemuck::cast_slice(&outline),
                        usage: wgpu::BufferUsages::VERTEX,
                    });
                self.mesh = Some(GpuMesh {
                    vertex_buffer,
                    vertex_count: mesh.vertices.len() as u32,
                    footprint_buffer,
                    footprint_vertex_count: outline.len() as u32,
                });
                self.mesh_ptr = ptr;
            }
        } else {
            self.mesh = None;
            self.mesh_ptr = 0;
        }

        let view_signed_z = -camera.pitch.sin();
        let fade_t = ((view_signed_z + 0.06) / 0.078).clamp(0.0, 1.0);
        let fade = fade_t * fade_t * (3.0 - 2.0 * fade_t);
        let plate_opacity = 1.0 - fade;
        queue.write_buffer(
            &self.plate_uniform,
            0,
            bytemuck::bytes_of(&OverlayUniform {
                value: plate_opacity,
                _pad: [0.0; 3],
            }),
        );

        queue.write_buffer(
            &self.model_uniform,
            0,
            bytemuck::cast_slice(&model.to_cols_array_2d()),
        );
        match gizmo {
            Some(origin) if origin != self.gizmo_origin => {
                self.gizmo_origin = origin;
                let length = 25.0;
                let axes = [
                    ([1.0, 0.0, 0.0], [1.0, 0.05, 0.05]),
                    ([0.0, 1.0, 0.0], [0.05, 1.0, 0.05]),
                    ([0.0, 0.0, 1.0], [0.05, 0.2, 1.0]),
                ];
                let mut vertices = Vec::new();
                for (dir, color) in axes {
                    vertices.push([origin[0], origin[1], origin[2], color[0], color[1], color[2]]);
                    vertices.push([
                        origin[0] + dir[0] * length,
                        origin[1] + dir[1] * length,
                        origin[2] + dir[2] * length,
                        color[0],
                        color[1],
                        color[2],
                    ]);
                }
                self.gizmo_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("viewport gizmo"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                self.gizmo_vertex_count = vertices.len() as u32;
            }
            None => {
                self.gizmo_origin = [f32::NAN; 3];
                self.gizmo_vertex_count = 0;
            }
            _ => {}
        }
        let support_ptr = supports.as_ptr() as usize;
        if support_ptr != self.support_ptr {
            self.support_ptr = support_ptr;
            let mut vertices = Vec::with_capacity(supports.len() * 2);
            for point in supports {
                vertices.push([point[0], point[1], point[2], 0.49, 0.93, 0.03]);
                vertices.push([point[0], point[1], 0.0, 0.49, 0.93, 0.03]);
            }
            if vertices.is_empty() {
                self.support_vertex_count = 0;
            } else {
                self.support_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("viewport supports"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                self.support_vertex_count = vertices.len() as u32;
            }
        }

        let aspect = width as f32 / height as f32;
        let projection = camera.projection(aspect);
        let view_proj = camera.view_proj(aspect);
        let eye = camera.eye();
        queue.write_buffer(
            &self.uniform,
            0,
            bytemuck::bytes_of(&Uniforms {
                view_proj: view_proj.to_cols_array_2d(),
                inv_proj: projection.inverse().to_cols_array_2d(),
                camera_pos: [eye.x, eye.y, eye.z],
                _pad0: 0.0,
                base_color,
                _pad1: 0.0,
                resolution: [width as f32, height as f32],
                _pad2: [0.0; 2],
            }),
        );

        queue.write_buffer(
            &self.decal_uniform,
            0,
            bytemuck::bytes_of(&OverlayUniform {
                value: if eye.y > 0.0 { 1.0 } else { 0.0 },
                _pad: [0.0; 3],
            }),
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("viewport frame"),
        });

        // MSAA scene pass, resolved into the scene texture.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.targets.msaa_color_view,
                    depth_slice: None,
                    resolve_target: Some(&self.targets.scene_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0033,
                            g: 0.0048,
                            b: 0.0069,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.targets.msaa_depth_view,
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
            pass.set_bind_group(0, &self.scene_bind, &[]);
            pass.set_pipeline(&self.background);
            pass.draw(0..3, 0..1);
            let below_plate = view_signed_z >= 0.008;
            if !below_plate && layers.plate {
                pass.set_pipeline(&self.plate);
                pass.set_bind_group(1, &self.plate_bind, &[]);
                pass.set_vertex_buffer(0, self.plate_vertex_buffer.slice(..));
                pass.set_index_buffer(self.plate_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.plate_index_count, 0, 0..1);
            }
            if !below_plate && layers.grid {
                pass.set_pipeline(&self.grid);
                pass.draw(0..6, 0..1);
            }
            if layers.volume {
                pass.set_pipeline(&self.volume);
                pass.set_vertex_buffer(0, self.volume_buffer.slice(..));
                pass.draw(0..self.volume_vertex_count, 0..1);
            }
            pass.set_pipeline(&self.axis);
            pass.set_vertex_buffer(0, self.axis_buffer.slice(..));
            pass.draw(0..self.axis_vertex_count, 0..1);
            pass.set_pipeline(&self.decal);
            pass.set_bind_group(1, &self.decal_bind, &[]);
            pass.set_vertex_buffer(0, self.decal_buffer.slice(..));
            pass.set_bind_group(2, &self.front_bind, &[]);
            pass.draw(0..6, 0..1);
            pass.set_bind_group(2, &self.logo_bind, &[]);
            pass.draw(6..12, 0..1);
            if self.support_vertex_count > 0 {
                pass.set_pipeline(&self.axis);
                pass.set_vertex_buffer(0, self.support_buffer.slice(..));
                pass.draw(0..self.support_vertex_count, 0..1);
            }
            if self.gizmo_vertex_count > 0 {
                pass.set_pipeline(&self.axis);
                pass.set_vertex_buffer(0, self.gizmo_buffer.slice(..));
                pass.draw(0..self.gizmo_vertex_count, 0..1);
            }
            if let Some(mesh) = &self.mesh {
                if !below_plate && mesh.footprint_vertex_count > 0 {
                    pass.set_pipeline(&self.footprint);
                    pass.set_vertex_buffer(0, mesh.footprint_buffer.slice(..));
                    pass.draw(0..mesh.footprint_vertex_count, 0..1);
                }
                pass.set_pipeline(&self.mesh_pipeline);
                pass.set_bind_group(1, &self.model_bind, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.draw(0..mesh.vertex_count, 0..1);
            }
        }

        // Composite: scene * AO + mood overlay.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport composite"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.targets.final_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.targets.composite_bind, &[]);
            pass.set_pipeline(&self.composite_pipeline);
            pass.draw(0..3, 0..1);
        }

        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.targets.final_texture,
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
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));
        Ok(())
    }

    /// Count the pixels the mesh actually painted. Used by `--smoke`.
    pub fn readback(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<(usize, usize)> {
        let (width, height) = self.size;
        let unpadded = width * 4;
        let padded = unpadded.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("viewport readback"),
            size: (padded * height) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("viewport readback"),
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.targets.final_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
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
        let mut painted = 0usize;
        for row in 0..height as usize {
            let start = row * padded as usize;
            for column in 0..width as usize {
                let at = start + column * 4;
                let r = data[at] as i32;
                let g = data[at + 1] as i32;
                let b = data[at + 2] as i32;
                if r > 90 && g > 60 && b > 50 {
                    painted += 1;
                }
            }
        }
        drop(data);
        buffer.unmap();
        Ok((painted, (width * height) as usize))
    }
}

impl Targets {
    fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        uniform: &wgpu::Buffer,
        composite_layout: &wgpu::BindGroupLayout,
        linear_sampler: &wgpu::Sampler,
    ) -> Self {
        let scene = color_target(device, width, height, false);
        let scene_view = scene.create_view(&wgpu::TextureViewDescriptor::default());
        let msaa_color = msaa_color_target(device, width, height);
        let msaa_color_view = msaa_color.create_view(&wgpu::TextureViewDescriptor::default());
        let msaa_depth = msaa_depth_target(device, width, height);
        let msaa_depth_view = msaa_depth.create_view(&wgpu::TextureViewDescriptor::default());
        let final_texture = color_target(device, width, height, true);
        let final_view = final_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let composite_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport composite"),
            layout: composite_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&scene_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(linear_sampler),
                },
            ],
        });

        Self {
            _scene: scene,
            scene_view,
            _msaa_color: msaa_color,
            msaa_color_view,
            _msaa_depth: msaa_depth,
            msaa_depth_view,
            final_texture,
            final_view,
            composite_bind,
        }
    }
}

fn axis_edges() -> Vec<[f32; 6]> {
    let origin = [-109.44, -61.44, 0.1];
    let length = 20.0;
    let head = 3.0;
    let axes: [([f32; 3], [f32; 3], [f32; 3], [f32; 3]); 3] = [
        ([1.0, 0.0, 0.0], [1.0, 0.05, 0.05], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        ([0.0, 1.0, 0.0], [0.05, 1.0, 0.05], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ([0.0, 0.0, 1.0], [0.05, 0.2, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
    ];
    let mut out = Vec::new();
    let mut push = |a: [f32; 3], b: [f32; 3], color: [f32; 3]| {
        out.push([a[0], a[1], a[2], color[0], color[1], color[2]]);
        out.push([b[0], b[1], b[2], color[0], color[1], color[2]]);
    };
    for (dir, color, u, v) in axes {
        let tip = [
            origin[0] + dir[0] * length,
            origin[1] + dir[1] * length,
            origin[2] + dir[2] * length,
        ];
        push(origin, tip, color);
        let back = [
            tip[0] - dir[0] * head,
            tip[1] - dir[1] * head,
            tip[2] - dir[2] * head,
        ];
        for side in [u, v] {
            push(
                tip,
                [
                    back[0] + side[0] * head * 0.5,
                    back[1] + side[1] * head * 0.5,
                    back[2] + side[2] * head * 0.5,
                ],
                color,
            );
        }
    }
    out
}

fn plate_outline(width: f32, depth: f32, z: f32) -> Vec<[f32; 3]> {
    let hx = width * 0.5;
    let hy = depth * 0.5;
    let corners = [
        [-hx, -hy, z],
        [hx, -hy, z],
        [hx, hy, z],
        [-hx, hy, z],
    ];
    let edges = [(0, 1), (1, 2), (2, 3), (3, 0)];
    edges
        .iter()
        .flat_map(|&(a, b)| [corners[a], corners[b]])
        .collect()
}

fn build_volume_edges(width: f32, depth: f32, height: f32) -> Vec<[f32; 3]> {
    let hx = width * 0.5;
    let hy = depth * 0.5;
    let corners = [
        [-hx, -hy, 0.0],
        [hx, -hy, 0.0],
        [hx, hy, 0.0],
        [-hx, hy, 0.0],
        [-hx, -hy, height],
        [hx, -hy, height],
        [hx, hy, height],
        [-hx, hy, height],
    ];
    let edges = [
        (0, 1), (1, 2), (2, 3), (3, 0),
        (4, 5), (5, 6), (6, 7), (7, 4),
        (0, 4), (1, 5), (2, 6), (3, 7),
    ];
    edges
        .iter()
        .flat_map(|&(a, b)| [corners[a], corners[b]])
        .collect()
}

fn plate_mesh(
    width: f32,
    depth: f32,
    thickness: f32,
    top: f32,
    radius: f32,
    segments: u32,
) -> (Vec<Vertex>, Vec<u32>) {
    let half_w = width * 0.5;
    let half_d = depth * 0.5;
    let r = radius.min(half_w - 0.01).min(half_d - 0.01);
    let z0 = top;
    let z1 = top - thickness;

    let mut perimeter: Vec<[f32; 2]> = vec![[-half_w + r, -half_d]];
    // Front tab: 24mm at the plate edge, 16mm at the tip, 4.2mm deep.
    perimeter.push([-12.0, -half_d]);
    perimeter.push([-8.0, -half_d - 4.2]);
    perimeter.push([8.0, -half_d - 4.2]);
    perimeter.push([12.0, -half_d]);
    let arc = |cx: f32, cy: f32, a0: f32, a1: f32, points: &mut Vec<[f32; 2]>| {
        for i in 1..=segments {
            let a = a0 + (a1 - a0) * i as f32 / segments as f32;
            points.push([cx + r * a.cos(), cy + r * a.sin()]);
        }
    };
    arc(half_w - r, -half_d + r, -std::f32::consts::FRAC_PI_2, 0.0, &mut perimeter);
    arc(half_w - r, half_d - r, 0.0, std::f32::consts::FRAC_PI_2, &mut perimeter);
    arc(-half_w + r, half_d - r, std::f32::consts::FRAC_PI_2, std::f32::consts::PI, &mut perimeter);
    arc(-half_w + r, -half_d + r, std::f32::consts::PI, 1.5 * std::f32::consts::PI, &mut perimeter);
    if let (Some(first), Some(last)) = (perimeter.first(), perimeter.last()) {
        if (first[0] - last[0]).abs() < 1e-5 && (first[1] - last[1]).abs() < 1e-5 {
            perimeter.pop();
        }
    }

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let count = perimeter.len();

    let top_center = vertices.len() as u32;
    vertices.push(Vertex { pos: [0.0, 0.0, z0], normal: [0.0, 0.0, 1.0], ao: 1.0 });
    let top_start = vertices.len() as u32;
    for p in &perimeter {
        vertices.push(Vertex { pos: [p[0], p[1], z0], normal: [0.0, 0.0, 1.0], ao: 1.0 });
    }
    for i in 0..count {
        let a = top_start + i as u32;
        let b = top_start + ((i + 1) % count) as u32;
        indices.extend_from_slice(&[top_center, a, b]);
    }

    let bottom_center = vertices.len() as u32;
    vertices.push(Vertex { pos: [0.0, 0.0, z1], normal: [0.0, 0.0, -1.0], ao: 1.0 });
    let bottom_start = vertices.len() as u32;
    for p in &perimeter {
        vertices.push(Vertex { pos: [p[0], p[1], z1], normal: [0.0, 0.0, -1.0], ao: 1.0 });
    }
    for i in 0..count {
        let a = bottom_start + i as u32;
        let b = bottom_start + ((i + 1) % count) as u32;
        indices.extend_from_slice(&[bottom_center, b, a]);
    }

    for i in 0..count {
        let p0 = perimeter[i];
        let p1 = perimeter[(i + 1) % count];
        let dx = p1[0] - p0[0];
        let dy = p1[1] - p0[1];
        let length = (dx * dx + dy * dy).sqrt().max(1e-6);
        let normal = [dy / length, -dx / length, 0.0];
        let base = vertices.len() as u32;
        vertices.push(Vertex { pos: [p0[0], p0[1], z0], normal, ao: 1.0 });
        vertices.push(Vertex { pos: [p1[0], p1[1], z0], normal, ao: 1.0 });
        vertices.push(Vertex { pos: [p1[0], p1[1], z1], normal, ao: 1.0 });
        vertices.push(Vertex { pos: [p0[0], p0[1], z1], normal, ao: 1.0 });
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    (vertices, indices)
}

fn create_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    width: u32,
    height: u32,
    data: &[u8],
) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    texture
}

fn front_texture() -> (u32, u32, Vec<u8>) {
    let width = 256u32;
    let height = 72u32;
    let mut data = vec![0u8; (width * height * 4) as usize];
    let Ok(bytes) = std::fs::read("C:/Windows/Fonts/segoeuib.ttf") else {
        return (width, height, data);
    };
    let Ok(font) = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()) else {
        return (width, height, data);
    };
    let size = 46.0f32;
    let spacing = 3.0f32;
    let mut glyphs = Vec::new();
    let mut total = 0.0f32;
    for ch in "FRONT".chars() {
        let (metrics, bitmap) = font.rasterize(ch, size);
        glyphs.push((metrics, bitmap, total));
        total += metrics.advance_width + spacing;
    }
    let start_x = (width as f32 - total) / 2.0;
    let baseline = height as f32 / 2.0 + size * 0.35;
    for (metrics, bitmap, advance) in glyphs {
        let gx = start_x + advance + metrics.xmin as f32;
        let gy = baseline - metrics.ymin as f32 - metrics.height as f32;
        for row in 0..metrics.height {
            for col in 0..metrics.width {
                let coverage = bitmap[row * metrics.width + col];
                if coverage == 0 {
                    continue;
                }
                let px = gx as i32 + col as i32;
                let py = gy as i32 + row as i32;
                if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
                    continue;
                }
                let index = ((py as u32 * width + px as u32) * 4) as usize;
                data[index] = coverage;
                data[index + 1] = coverage;
                data[index + 2] = coverage;
                data[index + 3] = coverage;
            }
        }
    }
    (width, height, data)
}

fn logo_texture() -> (u32, u32, Vec<u8>) {
    let width = 1772u32;
    let height = 304u32;
    let Ok(svg) = std::fs::read_to_string("../public/dragonfruit_assets/branding/text_logo.svg") else {
        return (width, height, vec![0u8; (width * height * 4) as usize]);
    };
    let Ok(tree) = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()) else {
        return (width, height, vec![0u8; (width * height * 4) as usize]);
    };
    let Some(mut pixmap) = resvg::tiny_skia::Pixmap::new(width, height) else {
        return (width, height, vec![0u8; (width * height * 4) as usize]);
    };
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    (width, height, pixmap.data().to_vec())
}

fn decal_vertices() -> Vec<[f32; 5]> {
    let mut out = Vec::new();
    let mut quad = |cx: f32, cy: f32, width: f32, height: f32, z: f32| {
        let x0 = cx - width * 0.5;
        let x1 = cx + width * 0.5;
        let y0 = cy - height * 0.5;
        let y1 = cy + height * 0.5;
        let corners = [
            (x0, y0, 0.0, 1.0),
            (x1, y0, 1.0, 1.0),
            (x1, y1, 1.0, 0.0),
            (x0, y0, 0.0, 1.0),
            (x1, y1, 1.0, 0.0),
            (x0, y1, 0.0, 0.0),
        ];
        for (x, y, u, v) in corners {
            out.push([x, y, z, u, v]);
        }
    };
    quad(0.0, -65.91, 13.2, 3.7, 0.05);
    quad(92.84, -56.24, 42.0, 7.2, 0.12);
    out
}

fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    buffers: &[Option<wgpu::VertexBufferLayout<'_>>],
    topology: wgpu::PrimitiveTopology,
    cull: Option<wgpu::Face>,
    depth_write: bool,
    depth_compare: wgpu::CompareFunction,
    format: wgpu::TextureFormat,
    depth: bool,
    fragment: bool,
    sample_count: u32,
) -> wgpu::RenderPipeline {
    let color_targets = [Some(wgpu::ColorTargetState {
        format,
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    })];
    let fragment_state = if fragment {
        Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            targets: &color_targets,
            compilation_options: Default::default(),
        })
    } else {
        None
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers,
            compilation_options: Default::default(),
        },
        fragment: fragment_state,
        primitive: wgpu::PrimitiveState {
            topology,
            cull_mode: cull,
            ..Default::default()
        },
        depth_stencil: if depth {
            Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(depth_write),
                depth_compare: Some(depth_compare),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })
        } else {
            None
        },
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        },
        multiview_mask: None,
        cache: None,
    })
}

fn color_target(device: &wgpu::Device, width: u32, height: u32, copy: bool) -> wgpu::Texture {
    let mut usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
    if copy {
        usage |= wgpu::TextureUsages::COPY_SRC;
    }
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("viewport color"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: WgpuSurface::DEFAULT_FORMAT,
        usage,
        view_formats: &[],
    })
}

fn msaa_color_target(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("viewport msaa color"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: SAMPLE_COUNT,
        dimension: wgpu::TextureDimension::D2,
        format: WgpuSurface::DEFAULT_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}

fn msaa_depth_target(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("viewport msaa depth"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: SAMPLE_COUNT,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}


const DECAL_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
};
struct Decal {
    flip: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var<uniform> decal: Decal;
@group(2) @binding(0) var tex: texture_2d<f32>;
@group(2) @binding(1) var samp: sampler;

struct DecalOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>) -> DecalOut {
    var out: DecalOut;
    out.position = uniforms.view_proj * vec4<f32>(position, 1.0);
    var u = uv;
    if (decal.flip > 0.5) {
        u = vec2<f32>(1.0 - u.x, 1.0 - u.y);
    }
    out.uv = u;
    return out;
}

@fragment
fn fs_main(in: DecalOut) -> @location(0) vec4<f32> {
    return textureSample(tex, samp, in.uv);
}
"#;

const MESH_SHADER: &str = r#"

struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct Model {
    matrix: mat4x4<f32>,
};
@group(1) @binding(0) var<uniform> model: Model;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) world: vec3<f32>,
    @location(2) ao: f32,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) ao: f32,
) -> VertexOut {
    var out: VertexOut;
    let world = model.matrix * vec4<f32>(position, 1.0);
    out.position = uniforms.view_proj * world;
    out.normal = (model.matrix * vec4<f32>(normal, 0.0)).xyz;
    out.world = world.xyz;
    out.ao = ao;
    return out;
}

// three.js BRDF_GGX: D_GGX * V_GGX_SmithCorrelated * F_Schlick.
fn specular(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, roughness: f32, f0: vec3<f32>) -> vec3<f32> {
    let h = normalize(v + l);
    let n_dot_l = max(dot(n, l), 0.0);
    let n_dot_v = max(dot(n, v), 0.0);
    let n_dot_h = max(dot(n, h), 0.0);
    let v_dot_h = max(dot(v, h), 0.0);

    let alpha = roughness * roughness;
    let a2 = alpha * alpha;
    let d = (n_dot_h * a2 - n_dot_h) * n_dot_h + 1.0;
    let distribution = a2 / (3.14159265 * d * d);

    let gv = n_dot_l * sqrt(n_dot_v * n_dot_v * (1.0 - a2) + a2);
    let gl = n_dot_v * sqrt(n_dot_l * n_dot_l * (1.0 - a2) + a2);
    let visibility = 0.5 / max(gv + gl, 0.0001);

    let f = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - v_dot_h, 5.0);
    return distribution * visibility * f;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let view_dir = normalize(uniforms.camera_pos - in.world);

    let key_dir = normalize(vec3<f32>(0.0, 0.0, 1.0));
    let fill_dir = normalize(vec3<f32>(0.0, 0.0, -1.0));
    let sky = vec3<f32>(0.965, 0.910, 1.0);
    let ground = vec3<f32>(0.243, 0.255, 0.361);
    let hemi = mix(ground, sky, n.z * 0.5 + 0.5);

    let ambient = hemi * 0.25;
    let key = vec3<f32>(1.0) * max(dot(n, key_dir), 0.0) * 1.2;
    let fill = vec3<f32>(0.565, 0.655, 1.0) * max(dot(n, fill_dir), 0.0) * 0.168;
    let head = vec3<f32>(1.0) * max(dot(n, view_dir), 0.0) * 1.0;
    let lighting = (ambient + key + fill + head) / 3.14159265;

    let base = uniforms.base_color;
    let f0 = mix(vec3<f32>(0.04), base, 0.02);
    let spec = (specular(n, view_dir, key_dir, 0.55, f0) * 1.2 * max(dot(n, key_dir), 0.0)
        + specular(n, view_dir, view_dir, 0.55, f0) * 1.0 * max(dot(n, view_dir), 0.0)) * 1.5;
    let n_dot_l = max(dot(n, key_dir), 0.0);
    let cavity = pow(1.0 - n_dot_l, 1.35);
    let fake_ao = 1.0 - cavity * 0.2;
    let baked = mix(1.0, clamp(in.ao, 0.0, 1.0), 0.85);
    let color = base * lighting * fake_ao * baked + spec;
    return vec4<f32>(color, 1.0);
}
"#;

const AXIS_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct AxisOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) color: vec3<f32>) -> AxisOut {
    var out: AxisOut;
    out.position = uniforms.view_proj * vec4<f32>(position, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: AxisOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
"#;

const VOLUME_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return uniforms.view_proj * vec4<f32>(position, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.045, 0.055, 0.068, 1.0);
}
"#;

const PLATE_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
};
struct Plate {
    opacity: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var<uniform> plate: Plate;

struct PlateOut {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) world: vec3<f32>,
};

@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) normal: vec3<f32>) -> PlateOut {
    var out: PlateOut;
    out.position = uniforms.view_proj * vec4<f32>(position, 1.0);
    out.normal = normal;
    out.world = position;
    return out;
}

@fragment
fn fs_main(in: PlateOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let view_dir = normalize(uniforms.camera_pos - in.world);
    let key = max(dot(n, vec3<f32>(0.0, 0.0, 1.0)), 0.0);
    let fill = max(dot(n, vec3<f32>(0.0, 0.0, -1.0)), 0.0);
    let head = max(dot(n, view_dir), 0.0);
    let hemi = mix(vec3<f32>(0.243, 0.255, 0.361), vec3<f32>(0.965, 0.910, 1.0), n.z * 0.5 + 0.5);
    let lighting = (hemi * 0.28 + key * 1.12 + fill * 0.168 + head * 1.0) / 3.14159265;
    let base = vec3<f32>(0.032, 0.040, 0.054);
    let fade = 1.0 - plate.opacity;
    let color = mix(base * lighting, vec3<f32>(0.0056, 0.0061, 0.0080), fade);
    return vec4<f32>(color, 1.0);
}
"#;

const GRID_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct GridOut {
    @builtin(position) position: vec4<f32>,
    @location(0) world: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> GridOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-109.44, -61.44), vec2<f32>(109.44, -61.44), vec2<f32>(109.44, 61.44),
        vec2<f32>(-109.44, -61.44), vec2<f32>(109.44, 61.44), vec2<f32>(-109.44, 61.44),
    );
    var out: GridOut;
    out.position = uniforms.view_proj * vec4<f32>(corners[index], 0.0, 1.0);
    out.world = corners[index];
    return out;
}

@fragment
fn fs_main(in: GridOut) -> @location(0) vec4<f32> {
    let cell = vec2<f32>(218.88 / 15.0, 122.88 / 15.0);
    let coord = in.world / cell;
    let width = fwidth(coord);
    let grid = abs(fract(coord) - 0.5) / width;
    let line = 1.0 - min(min(grid.x, grid.y), 1.0);
    let view_dir = normalize(uniforms.camera_pos - vec3<f32>(in.world, 0.0));
    let head = max(view_dir.z, 0.0);
    let lighting = (vec3<f32>(0.965, 0.910, 1.0) * 0.28 + 1.12 + head * 1.0) / 3.14159265;
    let plate = vec3<f32>(0.032, 0.040, 0.054) * lighting;
    let grid_color = vec3<f32>(0.035, 0.042, 0.052);
    let color = mix(plate, grid_color, line);
    return vec4<f32>(color, 1.0);
}
"#;

const FOOTPRINT_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(@location(0) position: vec2<f32>) -> @builtin(position) vec4<f32> {
    return uniforms.view_proj * vec4<f32>(position, 0.05, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.58, 0.18, 0.024, 1.0);
}
"#;

const BACKGROUND_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0),
    );
    return vec4<f32>(corners[index], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let t = clamp(position.y / uniforms.resolution.y, 0.0, 1.0);
    let background = vec3<f32>(0.0056, 0.0061, 0.0080);
    return vec4<f32>(background, 1.0);
}
"#;

const COMPOSITE_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var scene_tex: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0),
    );
    return vec4<f32>(corners[index], 0.0, 1.0);
}

fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
    let cutoff = step(c, vec3<f32>(0.0031308));
    let low = c * 12.92;
    let high = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055;
    return mix(high, low, cutoff);
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let cutoff = step(c, vec3<f32>(0.04045));
    let low = c / 12.92;
    let high = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return mix(high, low, cutoff);
}

@fragment
fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = frag.xy / uniforms.resolution;
    var color = textureSampleLevel(scene_tex, samp, uv, 0.0).rgb;

    // SceneMoodOverlay: CSS screen blend happens in sRGB, not linear.
    let radial = length((uv - vec2<f32>(0.5, 0.46)) / vec2<f32>(1.2, 0.95));
    let radial_alpha = smoothstep(0.56, 1.0, radial) * 0.18 * 0.75;
    let pink = vec3<f32>(1.0, 0.216, 0.667);
    let violet = vec3<f32>(0.435, 0.2, 1.0);
    var linear_color = pink;
    var linear_alpha = 0.10 * 0.8;
    if (uv.y > 0.4) {
        let k = (uv.y - 0.4) / 0.6;
        linear_color = mix(violet, vec3<f32>(0.0), k);
        linear_alpha = mix(0.06 * 0.8, 0.0, k);
    } else {
        let k = uv.y / 0.4;
        linear_color = mix(pink, violet, k);
        linear_alpha = mix(0.10 * 0.8, 0.06 * 0.8, k);
    }
    let overlay = pink * radial_alpha + linear_color * linear_alpha;
    let scene_srgb = linear_to_srgb(color);
    let blended = vec3<f32>(1.0) - (vec3<f32>(1.0) - scene_srgb) * (vec3<f32>(1.0) - overlay);
    return vec4<f32>(srgb_to_linear(blended), 1.0);
}
"#;
