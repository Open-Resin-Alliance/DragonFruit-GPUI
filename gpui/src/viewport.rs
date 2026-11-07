//! The wgpu viewport: background gradient, build-plate grid, footprint outline,
//! flat-shaded clay mesh, and screen-space ambient occlusion.

use anyhow::Result;
use gpui_interop::WgpuSurface;
use wgpu::util::DeviceExt as _;

use crate::camera::Camera;
use crate::stl::{MeshData, Vertex};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    inv_proj: [[f32; 4]; 4],
    camera_pos: [f32; 3],
    _pad: f32,
    resolution: [f32; 2],
    _pad2: [f32; 2],
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
    _depth: wgpu::Texture,
    depth_view: wgpu::TextureView,
    _ao: wgpu::Texture,
    ao_view: wgpu::TextureView,
    final_texture: wgpu::Texture,
    final_view: wgpu::TextureView,
    ssao_bind: wgpu::BindGroup,
    composite_bind: wgpu::BindGroup,
}

pub struct Viewport {
    surface: WgpuSurface,
    background: wgpu::RenderPipeline,
    grid: wgpu::RenderPipeline,
    footprint: wgpu::RenderPipeline,
    mesh_pipeline: wgpu::RenderPipeline,
    ssao_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    scene_bind: wgpu::BindGroup,
    ssao_layout: wgpu::BindGroupLayout,
    composite_layout: wgpu::BindGroupLayout,
    linear_sampler: wgpu::Sampler,
    targets: Targets,
    size: (u32, u32),
    mesh: Option<GpuMesh>,
    mesh_ptr: usize,
}

impl Viewport {
    pub fn new(device: &wgpu::Device) -> Result<Self> {
        let surface = WgpuSurface::new(device, 1, 1)?;
        device.on_uncaptured_error(std::sync::Arc::new(|error| eprintln!("wgpu: {error}")));

        let mesh_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport mesh"),
            source: wgpu::ShaderSource::Wgsl(MESH_SHADER.into()),
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
        let ssao_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport ssao"),
            source: wgpu::ShaderSource::Wgsl(SSAO_SHADER.into()),
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
        let ssao_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("viewport ssao"),
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
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
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
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
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
        let ssao_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("viewport ssao"),
            bind_group_layouts: &[Some(&ssao_layout)],
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
            ],
        };

        let mesh_pipeline = pipeline(
            device,
            &uniform_pipeline_layout,
            &mesh_shader,
            &[Some(vertex_layout)],
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::Face::Back),
            true,
            wgpu::CompareFunction::Less,
            WgpuSurface::DEFAULT_FORMAT,
            true,
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
        );
        let footprint = pipeline(
            device,
            &uniform_pipeline_layout,
            &footprint_shader,
            &[Some(wgpu::VertexBufferLayout {
                array_stride: 8,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
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
        );
        let ssao_pipeline = pipeline(
            device,
            &ssao_pipeline_layout,
            &ssao_shader,
            &[],
            wgpu::PrimitiveTopology::TriangleList,
            None,
            false,
            wgpu::CompareFunction::Always,
            wgpu::TextureFormat::R8Unorm,
            false,
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
        );

        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("viewport linear sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let targets = Targets::new(
            device,
            1,
            1,
            &uniform,
            &ssao_layout,
            &composite_layout,
            &linear_sampler,
        );

        Ok(Self {
            surface,
            background,
            grid,
            footprint,
            mesh_pipeline,
            ssao_pipeline,
            composite_pipeline,
            uniform,
            scene_bind,
            ssao_layout,
            composite_layout,
            linear_sampler,
            targets,
            size: (1, 1),
            mesh: None,
            mesh_ptr: 0,
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
        camera: &Camera,
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
                &self.ssao_layout,
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
                _pad: 0.0,
                resolution: [width as f32, height as f32],
                _pad2: [0.0; 2],
            }),
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("viewport frame"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.targets.scene_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.043,
                            g: 0.059,
                            b: 0.078,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
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
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.scene_bind, &[]);
            pass.set_pipeline(&self.background);
            pass.draw(0..3, 0..1);
            pass.set_pipeline(&self.grid);
            pass.draw(0..6, 0..1);
            if let Some(mesh) = &self.mesh {
                if mesh.footprint_vertex_count > 0 {
                    pass.set_pipeline(&self.footprint);
                    pass.set_vertex_buffer(0, mesh.footprint_buffer.slice(..));
                    pass.draw(0..mesh.footprint_vertex_count, 0..1);
                }
                pass.set_pipeline(&self.mesh_pipeline);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.draw(0..mesh.vertex_count, 0..1);
            }
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport ssao"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.targets.ao_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.targets.ssao_bind, &[]);
            pass.set_pipeline(&self.ssao_pipeline);
            pass.draw(0..3, 0..1);
        }
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
        ssao_layout: &wgpu::BindGroupLayout,
        composite_layout: &wgpu::BindGroupLayout,
        linear_sampler: &wgpu::Sampler,
    ) -> Self {
        let scene = color_target(device, width, height, false);
        let scene_view = scene.create_view(&wgpu::TextureViewDescriptor::default());
        let depth = depth_target(device, width, height);
        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
        let ao = ao_target(device, width, height);
        let ao_view = ao.create_view(&wgpu::TextureViewDescriptor::default());
        let final_texture = color_target(device, width, height, true);
        let final_view = final_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let ssao_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport ssao"),
            layout: ssao_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&depth_view),
                },
            ],
        });
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
                    resource: wgpu::BindingResource::TextureView(&ao_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(linear_sampler),
                },
            ],
        });

        Self {
            _scene: scene,
            scene_view,
            _depth: depth,
            depth_view,
            _ao: ao,
            ao_view,
            final_texture,
            final_view,
            ssao_bind,
            composite_bind,
        }
    }
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
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers,
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
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
        multisample: wgpu::MultisampleState::default(),
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

fn depth_target(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("viewport depth"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

fn ao_target(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("viewport ao"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

const MESH_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    resolution: vec2<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) world: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
) -> VertexOut {
    var out: VertexOut;
    out.position = uniforms.view_proj * vec4<f32>(position, 1.0);
    out.normal = normal;
    out.world = position;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let view_dir = normalize(uniforms.camera_pos - in.world);
    let key_dir = normalize(vec3<f32>(0.25, 0.35, 0.9));
    let sky = vec3<f32>(0.965, 0.910, 1.0);
    let ground = vec3<f32>(0.243, 0.255, 0.361);
    let hemi = mix(ground, sky, n.z * 0.5 + 0.5);
    let ambient = hemi * 0.65;
    let key = vec3<f32>(1.0) * max(dot(n, key_dir), 0.0) * 0.85;
    let fill_dir = normalize(vec3<f32>(-0.2, -0.3, -0.9));
    let fill = vec3<f32>(0.565, 0.655, 1.0) * max(dot(n, fill_dir), 0.0) * 0.18;
    let head = vec3<f32>(1.0) * max(dot(n, view_dir), 0.0) * 0.28;
    let lighting = ambient + key + fill + head;

    let base = vec3<f32>(0.578, 0.253, 0.323);
    let half = normalize(key_dir + view_dir);
    let spec = pow(max(dot(n, half), 0.0), 24.0) * 0.03;
    let rim = pow(1.0 - max(dot(n, view_dir), 0.0), 3.0) * 0.08;
    let color = base * lighting + vec3<f32>(spec) + sky * rim;
    return vec4<f32>(color, 1.0);
}
"#;

const GRID_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
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
        vec2<f32>(-109.0, -61.5), vec2<f32>(109.0, -61.5), vec2<f32>(109.0, 61.5),
        vec2<f32>(-109.0, -61.5), vec2<f32>(109.0, 61.5), vec2<f32>(-109.0, 61.5),
    );
    var out: GridOut;
    out.position = uniforms.view_proj * vec4<f32>(corners[index], 0.0, 1.0);
    out.world = corners[index];
    return out;
}

@fragment
fn fs_main(in: GridOut) -> @location(0) vec4<f32> {
    let cell = 218.0 / 15.0;
    let coord = in.world / cell;
    let width = fwidth(coord);
    let grid = abs(fract(coord - 0.5) - 0.5) / width;
    let line = 1.0 - min(min(grid.x, grid.y), 1.0);
    let axis = 1.0 - min(
        min(abs(in.world.x) / fwidth(in.world.x), abs(in.world.y) / fwidth(in.world.y)),
        1.0,
    );

    let plate = vec3<f32>(0.032, 0.040, 0.054);
    let minor = vec3<f32>(0.025, 0.031, 0.040);
    let major = vec3<f32>(0.078, 0.090, 0.116);
    var color = mix(plate, minor, line * 0.7);
    color = mix(color, major, axis * 0.6);
    return vec4<f32>(color, 1.0);
}
"#;

const FOOTPRINT_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
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
    let top = vec3<f32>(0.006, 0.008, 0.012);
    let bottom = vec3<f32>(0.043, 0.059, 0.078);
    return vec4<f32>(mix(top, bottom, t), 1.0);
}
"#;

const SSAO_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    resolution: vec2<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var depth_tex: texture_depth_2d;

fn depth_at(uv: vec2<f32>) -> f32 {
    let max = uniforms.resolution - vec2<f32>(1.0, 1.0);
    let coord = vec2<i32>(clamp(uv * uniforms.resolution, vec2<f32>(0.0, 0.0), max));
    return textureLoad(depth_tex, coord, 0);
}

const KERNEL = array<vec3<f32>, 16>(
    vec3<f32>(0.10, 0.10, 0.10),
    vec3<f32>(-0.10, 0.10, 0.20),
    vec3<f32>(0.10, -0.10, 0.20),
    vec3<f32>(-0.10, -0.10, 0.10),
    vec3<f32>(0.20, 0.10, 0.30),
    vec3<f32>(-0.20, 0.10, 0.10),
    vec3<f32>(0.10, 0.20, 0.30),
    vec3<f32>(-0.10, -0.20, 0.10),
    vec3<f32>(0.30, 0.20, 0.20),
    vec3<f32>(-0.30, 0.20, 0.10),
    vec3<f32>(0.20, -0.30, 0.30),
    vec3<f32>(-0.20, -0.30, 0.20),
    vec3<f32>(0.40, 0.30, 0.20),
    vec3<f32>(-0.40, 0.10, 0.30),
    vec3<f32>(0.30, 0.40, 0.10),
    vec3<f32>(-0.30, -0.40, 0.20),
);

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0),
    );
    return vec4<f32>(corners[index], 0.0, 1.0);
}

fn view_position(uv: vec2<f32>) -> vec3<f32> {
    let depth = depth_at(uv);
    let ndc = vec3<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, depth);
    let p = uniforms.inv_proj * vec4<f32>(ndc, 1.0);
    return p.xyz / p.w;
}

@fragment
fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) f32 {
    let uv = frag.xy / uniforms.resolution;
    let depth = depth_at(uv);
    if (depth >= 1.0) {
        return 1.0;
    }
    let p = view_position(uv);
    let n = normalize(cross(dpdx(p), dpdy(p)));
    let noise = fract(sin(dot(frag.xy, vec2<f32>(12.9898, 78.233))) * 43758.5453) * 6.2831853;
    let rand = vec3<f32>(cos(noise), sin(noise), 0.0);
    let tangent = normalize(rand - n * dot(rand, n));
    let bitangent = cross(n, tangent);
    let radius = 4.0;
    var occlusion = 0.0;
    for (var i = 0; i < 16; i = i + 1) {
        let sample = KERNEL[i] * (0.25 + 0.75 * f32(i) / 16.0);
        let dir = tangent * sample.x + bitangent * sample.y + n * sample.z;
        let sample_pos = p + dir * radius;
        let clip = uniforms.view_proj * vec4<f32>(sample_pos, 1.0);
        let ndc = clip.xyz / clip.w;
        let suv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
        if (suv.x < 0.0 || suv.x > 1.0 || suv.y < 0.0 || suv.y > 1.0) {
            continue;
        }
        let scene_pos = view_position(suv);
        let range = smoothstep(0.0, 1.0, radius / abs(p.z - scene_pos.z));
        if (scene_pos.z >= sample_pos.z + 0.05) {
            occlusion += range;
        }
    }
    return clamp(1.0 - occlusion / 16.0, 0.0, 1.0);
}
"#;

const COMPOSITE_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    resolution: vec2<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var scene_tex: texture_2d<f32>;
@group(0) @binding(2) var ao_tex: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0),
    );
    return vec4<f32>(corners[index], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = frag.xy / uniforms.resolution;
    let color = textureSampleLevel(scene_tex, samp, uv, 0.0).rgb;
    let ao = textureSampleLevel(ao_tex, samp, uv, 0.0).r;
    return vec4<f32>(color * mix(1.0, ao, 0.8), 1.0);
}
"#;
