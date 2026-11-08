//! `Viewport` construction: every pipeline, buffer, bind group and render
//! target the viewport owns, plus the build-plate geometry from `PlateMetrics`.

use anyhow::Result;
use gpui_interop::WgpuSurface;
use wgpu::util::DeviceExt as _;

use crate::stl::Vertex;

use super::cube::{CUBE_SAMPLE_COUNT, CUBE_SHADER, CUBE_TARGET_SIZE, CubeUniform};
use super::pipeline::{MipGenerator, pipeline, pipeline_blended};
use super::plate::{
    PlateMetrics, axis_arrows, axis_label_texture, build_volume_edges, build_volume_fill,
    decal_vertices, front_texture, grid_vertices, logo_texture, plate_mesh, sprite_vertices,
};
use super::shaders::{
    AXIS_SHADER, BACKGROUND_SHADER, COMPOSITE_SHADER, DECAL_SHADER, FOOTPRINT_SHADER, GIZMO_SHADER,
    MESH_SHADER, PLATE_SHADER, SPRITE_SHADER, SUPPORT_SHADER, VOLUME_SHADER,
};
use super::targets::Targets;
use super::uniforms::{ColorUniform, DecalUniform, SpriteUniform, SupportInstance, Uniforms};
use super::{SAMPLE_COUNT, SUPPORT_COLOR, Viewport};

/// The cube's owned GPU resources, built once in `Viewport::new`.
pub(super) struct CubeResources {
    pub(super) pipeline: wgpu::RenderPipeline,
    pub(super) uniforms: [wgpu::Buffer; 2],
    pub(super) binds: [wgpu::BindGroup; 2],
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) target: wgpu::Texture,
    pub(super) target_view: wgpu::TextureView,
    pub(super) msaa: wgpu::Texture,
    pub(super) msaa_view: wgpu::TextureView,
    pub(super) texture_bind: wgpu::BindGroup,
}

impl CubeResources {
    pub(super) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        mip: &MipGenerator,
        texture_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let cube_uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("viewport cube uniform"),
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
        let cube_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("viewport cube"),
                bind_group_layouts: &[Some(&cube_uniform_layout), Some(&texture_layout)],
                immediate_size: 0,
            });
        let cube_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport cube"),
            source: wgpu::ShaderSource::Wgsl(CUBE_SHADER.into()),
        });
        let cube_uniforms = std::array::from_fn(|_| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("viewport cube uniform"),
                size: std::mem::size_of::<CubeUniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        });
        let cube_binds = std::array::from_fn(|index| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("viewport cube uniform"),
                layout: &cube_uniform_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: cube_uniforms[index].as_entire_binding(),
                }],
            })
        });
        let (atlas_w, atlas_h, atlas_data) = crate::cube::atlas();
        // Mipmapped: at ~40 device pixels per face a 1px atlas stroke is
        // sub-pixel, and point sampling it reads as a dotted outline.
        let atlas = mip.create(device, queue, "viewport cube atlas", atlas_w, atlas_h, &atlas_data);
        let atlas_view = atlas.create_view(&wgpu::TextureViewDescriptor::default());
        let cube_texture_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport cube atlas"),
            layout: &texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&mip.sampler),
                },
            ],
        });
        let cube_vertices = crate::cube::vertices();
        let cube_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport cube vertices"),
            contents: bytemuck::cast_slice(&cube_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let cube_target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("viewport cube target"),
            size: wgpu::Extent3d {
                width: CUBE_TARGET_SIZE,
                height: CUBE_TARGET_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let cube_target_view = cube_target.create_view(&wgpu::TextureViewDescriptor::default());
        let cube_msaa = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("viewport cube msaa"),
            size: wgpu::Extent3d {
                width: CUBE_TARGET_SIZE,
                height: CUBE_TARGET_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: CUBE_SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let cube_msaa_view = cube_msaa.create_view(&wgpu::TextureViewDescriptor::default());
        let cube_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("viewport cube"),
            layout: Some(&cube_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &cube_shader,
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
                module: &cube_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: CUBE_SAMPLE_COUNT,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline: cube_pipeline,
            uniforms: cube_uniforms,
            binds: cube_binds,
            vertex_buffer: cube_vertex_buffer,
            target: cube_target,
            target_view: cube_target_view,
            msaa: cube_msaa,
            msaa_view: cube_msaa_view,
            texture_bind: cube_texture_bind,
        }
    }
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
        let gizmo_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport gizmo"),
            source: wgpu::ShaderSource::Wgsl(GIZMO_SHADER.into()),
        });
        let decal_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport decal"),
            source: wgpu::ShaderSource::Wgsl(DECAL_SHADER.into()),
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
            size: std::mem::size_of::<DecalUniform>() as u64,
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
        // The logo is not a marker: it never turns, and it is drawn at 0.4.
        let logo_decal_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("viewport logo decal uniform"),
            size: std::mem::size_of::<DecalUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let logo_decal_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport logo decal"),
            layout: &decal_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: logo_decal_uniform.as_entire_binding(),
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
        // Position + linear colour: the axes, the grid and the supports all draw
        // with it.
        let axis_layout = wgpu::VertexBufferLayout {
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
        };

        let model_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("viewport model"),
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
        let mesh_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("viewport mesh"),
                bind_group_layouts: &[Some(&uniform_layout), Some(&model_layout)],
                immediate_size: 0,
            });
        let model_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("viewport model"),
            // `ModelUniform`: matrix + clip + bounds_min/max + stripe.
            size: std::mem::size_of::<super::uniforms::ModelUniform>() as u64,
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
        // `GridHelper`: `LineSegments` with vertex colours, opaque, depth-written.
        let grid = pipeline(
            device,
            &uniform_pipeline_layout,
            &axis_shader,
            &[Some(axis_layout.clone())],
            wgpu::PrimitiveTopology::LineList,
            None,
            true,
            wgpu::CompareFunction::LessEqual,
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
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let mip = MipGenerator::new(device);

        let (front_w, front_h, front_data) = front_texture();
        let front_tex = mip.create(device, queue, "front decal", front_w, front_h, &front_data);
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
        let logo_tex = mip.create(device, queue, "logo decal", logo_w, logo_h, &logo_data);
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
        let volume_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("viewport volume"),
                bind_group_layouts: &[Some(&uniform_layout), Some(&decal_layout)],
                immediate_size: 0,
            });
        let volume_edge_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("viewport volume edges"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let volume_edge_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport volume edges"),
            layout: &decal_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: volume_edge_uniform.as_entire_binding(),
            }],
        });
        let volume_fill_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("viewport volume fill"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let volume_fill_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport volume fill"),
            layout: &decal_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: volume_fill_uniform.as_entire_binding(),
            }],
        });
        // Orientation cube: a textured quad per face into its own target, so the
        // overlay can import and paint it (`ZUpGizmoViewcube`).
        let CubeResources {
            pipeline: cube_pipeline,
            uniforms: cube_uniforms,
            binds: cube_binds,
            vertex_buffer: cube_vertex_buffer,
            target: cube_target,
            target_view: cube_target_view,
            msaa: cube_msaa,
            msaa_view: cube_msaa_view,
            texture_bind: cube_texture_bind,
        } = CubeResources::new(device, queue, &mip, &texture_layout);
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
        let plate = pipeline_blended(
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
            Some(wgpu::Face::Back),
            true,
            wgpu::CompareFunction::Less,
            WgpuSurface::DEFAULT_FORMAT,
            wgpu::DepthBiasState::default(),
            SAMPLE_COUNT,
        );

        let volume = pipeline_blended(
            device,
            &volume_pipeline_layout,
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
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::DepthBiasState::default(),
            SAMPLE_COUNT,
        );
        // `side={THREE.BackSide}` on the box's faint fill: only the far faces.
        let volume_fill = pipeline_blended(
            device,
            &volume_pipeline_layout,
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
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::Face::Front),
            false,
            wgpu::CompareFunction::LessEqual,
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::DepthBiasState::default(),
            SAMPLE_COUNT,
        );
        let axis = pipeline(
            device,
            &uniform_pipeline_layout,
            &axis_shader,
            &[Some(axis_layout.clone())],
            wgpu::PrimitiveTopology::LineList,
            None,
            false,
            wgpu::CompareFunction::LessEqual,
            WgpuSurface::DEFAULT_FORMAT,
            true,
            true,
            SAMPLE_COUNT,
        );
        // `meshBasicMaterial` defaults to `FrontSide`, but the generated caps
        // are closed from both sides; culling none keeps them visible.
        let axis_mesh = pipeline(
            device,
            &uniform_pipeline_layout,
            &axis_shader,
            &[Some(axis_layout.clone())],
            wgpu::PrimitiveTopology::TriangleList,
            None,
            false,
            wgpu::CompareFunction::LessEqual,
            WgpuSurface::DEFAULT_FORMAT,
            true,
            true,
            SAMPLE_COUNT,
        );
        // The transform gizmo: an unlit vertex-colour pass on triangles, with the
        // depth test disabled and no depth write — the source's
        // `depthTest: false` / `depthWrite: false` on every handle material, so a
        // handle is never hidden behind the model. It is the last draw of the
        // scene pass, which is what makes straight-alpha blending safe: every
        // handle composites over the finished model and grid, exactly as
        // three.js blends its `transparent` handle materials over its scene.
        //
        // The vertex carries `[pos3, rgba]`; the alpha is the source's own
        // per-handle opacity (0.55 centre, 0.9 arrow/scale, 0.72 ring, 0.26 of
        // that for the back arc, 0.15 for a handle dimmed by a running drag).
        let gizmo_layout = wgpu::VertexBufferLayout {
            array_stride: 28,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 12,
                    shader_location: 1,
                },
            ],
        };
        let gizmo = pipeline_blended(
            device,
            &uniform_pipeline_layout,
            &gizmo_shader,
            &[Some(gizmo_layout)],
            wgpu::PrimitiveTopology::TriangleList,
            None,
            false,
            wgpu::CompareFunction::Always,
            WgpuSurface::DEFAULT_FORMAT,
            wgpu::DepthBiasState::default(),
            SAMPLE_COUNT,
        );
        // Supports: one instanced pass per batch, `meshBasicMaterial`'s flat
        // colour. The three unit meshes are the source's instanced ones — a
        // 1×1 cylinder (radius 0.5, `y = -0.5..0.5`), a 1-radius sphere and a
        // 1-radius disk — so every millimetre dimension rides the instance
        // matrix.
        let support_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport supports"),
            source: wgpu::ShaderSource::Wgsl(SUPPORT_SHADER.into()),
        });
        let support_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("viewport supports"),
                bind_group_layouts: &[Some(&uniform_layout), Some(&decal_layout)],
                immediate_size: 0,
            });
        let support_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("viewport support colour"),
            size: std::mem::size_of::<ColorUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let support_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport support colour"),
            layout: &decal_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: support_uniform.as_entire_binding(),
            }],
        });
        // `DEFAULT_SUPPORT_COLOR` is an sRGB hex; the scene target is sRGB.
        queue.write_buffer(
            &support_uniform,
            0,
            bytemuck::bytes_of(&ColorUniform {
                color: super::mesh::srgb_linear(SUPPORT_COLOR),
                opacity: 1.0,
            }),
        );
        let support_vertex_layout = wgpu::VertexBufferLayout {
            array_stride: 12,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            }],
        };
        let support_instance_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SupportInstance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 1,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 16,
                    shader_location: 2,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 32,
                    shader_location: 3,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 48,
                    shader_location: 4,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 64,
                    shader_location: 5,
                },
            ],
        };
        // `meshBasicMaterial`'s default is `FrontSide`, but the generated caps
        // close both ends; culling none keeps a plate-level view solid.
        let support = pipeline(
            device,
            &support_pipeline_layout,
            &support_shader,
            &[Some(support_vertex_layout), Some(support_instance_layout)],
            wgpu::PrimitiveTopology::TriangleList,
            None,
            true,
            wgpu::CompareFunction::LessEqual,
            WgpuSurface::DEFAULT_FORMAT,
            true,
            true,
            SAMPLE_COUNT,
        );
        let support_mesh = |label: &str, vertices: &[[f32; 3]]| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            })
        };
        let support_cylinder_vertices = gpui_supports::primitives::cylinder(
            glam::Vec3::new(0.0, -0.5, 0.0),
            glam::Vec3::new(0.0, 0.5, 0.0),
            0.5,
            0.5,
            gpui_supports::RADIAL_SEGMENTS,
        );
        let support_sphere_vertices = gpui_supports::primitives::sphere(
            glam::Vec3::ZERO,
            1.0,
            gpui_supports::RADIAL_SEGMENTS,
            gpui_supports::SPHERE_RINGS,
        );
        // `ContactDisk`: a `CircleGeometry` with 24 segments.
        let support_disk_vertices =
            gpui_supports::primitives::disk(glam::Vec3::ZERO, glam::Vec3::Z, 1.0, 24);
        let support_cylinder = support_mesh("viewport support cylinder", &support_cylinder_vertices);
        let support_sphere = support_mesh("viewport support sphere", &support_sphere_vertices);
        let support_disk = support_mesh("viewport support disk", &support_disk_vertices);
        let support_cylinder_count = support_cylinder_vertices.len() as u32;
        let support_sphere_count = support_sphere_vertices.len() as u32;
        let support_disk_count = support_disk_vertices.len() as u32;
        // The VOXL profile's build volume (`profiles/printers/generic`), centred
        // origin (`originMode: 'center'`). The app takes these from the active
        // printer profile; this viewport has no profile store yet.
        const BUILD_VOLUME_MM: (f32, f32, f32) = (218.88, 122.88, 220.0);
        let metrics = PlateMetrics::new(
            BUILD_VOLUME_MM.0,
            BUILD_VOLUME_MM.1,
            BUILD_VOLUME_MM.2,
            -BUILD_VOLUME_MM.0 * 0.5,
            -BUILD_VOLUME_MM.1 * 0.5,
        );
        let axis_edges = axis_arrows(&metrics);
        let axis_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport axes"),
            contents: bytemuck::cast_slice(&axis_edges),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let grid_edges = grid_vertices(&metrics);
        let grid_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport grid"),
            contents: bytemuck::cast_slice(&grid_edges),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let sprite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport sprite"),
            source: wgpu::ShaderSource::Wgsl(SPRITE_SHADER.into()),
        });
        let sprite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("viewport sprite"),
                bind_group_layouts: &[
                    Some(&uniform_layout),
                    Some(&decal_layout),
                    Some(&texture_layout),
                ],
                immediate_size: 0,
            });
        let sprite_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("viewport sprite uniform"),
            size: std::mem::size_of::<SpriteUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sprite_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport sprite"),
            layout: &decal_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: sprite_uniform.as_entire_binding(),
            }],
        });
        let sprite = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("viewport sprite"),
            layout: Some(&sprite_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &sprite_shader,
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
                module: &sprite_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: WgpuSurface::DEFAULT_FORMAT,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
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
        let sprite_vertices = sprite_vertices(&metrics);
        let sprite_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport sprites"),
            contents: bytemuck::cast_slice(&sprite_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let label_bind = |letter: &str, color: u32| {
            let (w, h, data) = axis_label_texture(letter, color);
            let texture = mip.create(device, queue, letter, w, h, &data);
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(letter),
                layout: &texture_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&linear_sampler),
                    },
                ],
            })
        };
        let x_label_bind = label_bind("X", 0xff0000);
        let y_label_bind = label_bind("Y", 0x00ff00);
        let z_label_bind = label_bind("Z", 0x0000ff);
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
                            src_factor: wgpu::BlendFactor::SrcAlpha,
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
                // `polygonOffsetFactor/Units: -2` on both decals.
                bias: wgpu::DepthBiasState {
                    constant: -2,
                    slope_scale: -2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState {
                count: SAMPLE_COUNT,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });
        let decal_vertices = decal_vertices(&metrics);
        let decal_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport decals"),
            contents: bytemuck::cast_slice(&decal_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let volume_edges = build_volume_edges(
            metrics.center_x,
            metrics.center_y,
            metrics.width,
            metrics.depth,
            metrics.volume_height,
        );
        let volume_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport volume"),
            contents: bytemuck::cast_slice(&volume_edges),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let volume_fill_vertices = build_volume_fill(
            metrics.center_x,
            metrics.center_y,
            metrics.width,
            metrics.depth,
            metrics.volume_height,
        );
        let volume_fill_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport volume fill"),
            contents: bytemuck::cast_slice(&volume_fill_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let (plate_vertices, plate_indices) = plate_mesh(&metrics);
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

        // Edge and corner boxes share the cube pipeline: their UVs point at the
        // atlas border texel, so a white tint paints them solid accent.
        let highlight = crate::cube::highlight_vertices();
        let highlight_flat: Vec<[f32; 5]> = highlight
            .iter()
            .map(|p| [p[0], p[1], p[2], 0.0, 0.0])
            .collect();
        let cube_highlight_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport cube highlights"),
            contents: bytemuck::cast_slice(&highlight_flat),
            usage: wgpu::BufferUsages::VERTEX,
        });
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
            volume_fill,
            volume_fill_buffer,
            volume_fill_vertex_count: volume_fill_vertices.len() as u32,
            volume_edge_bind,
            volume_fill_bind,
            volume_edge_uniform,
            volume_fill_uniform,
            axis,
            axis_mesh,
            axis_buffer,
            axis_vertex_count: axis_edges.len() as u32,
            sprite,
            sprite_uniform,
            sprite_bind,
            sprite_buffer,
            sprite_vertex_count: sprite_vertices.len() as u32,
            x_label_bind,
            y_label_bind,
            z_label_bind,
            plate_uniform,
            plate_bind,
            cube_pipeline,
            cube_uniforms,
            cube_binds,
            cube_vertex_buffer,
            cube_highlight_buffer,
            cube_target,
            cube_target_view,
            cube_msaa,
            cube_msaa_view,
            cube_texture_bind,
            decal,
            decal_buffer,
            decal_uniform,
            decal_bind,
            logo_decal_uniform,
            logo_decal_bind,
            front_bind,
            logo_bind,
            grid,
            grid_buffer,
            grid_vertex_count: grid_edges.len() as u32,
            footprint,
            mesh_pipeline,
            composite_pipeline,
            uniform,
            scene_bind,
            composite_layout,
            linear_sampler,
            targets,
            size: (1, 1),
            metrics,
            below_plate: false,
            front_from_back: None,
            mesh: None,
            mesh_ptr: 0,
            mesh_matrix: glam::Mat4::IDENTITY,
            support,
            support_uniform,
            support_bind,
            support_cylinder,
            support_cylinder_count,
            support_sphere,
            support_sphere_count,
            support_disk,
            support_disk_count,
            support_straight_instances: None,
            support_straight_count: 0,
            support_tapered_instances: None,
            support_tapered_count: 0,
            support_node_instances: None,
            support_node_count: 0,
            support_disk_instances: None,
            support_disk_instance_count: 0,
            supports_ptr: 0,
            model_uniform,
            model_bind,
            gizmo,
            gizmo_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("viewport gizmo"),
                // One vertex worth of room until the first frame fills it; the
                // buffer is only ever read for `gizmo_vertex_count` vertices.
                size: 24,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            gizmo_vertex_count: 0,
            gizmo_vertices: Vec::new(),
        })
    }
}
