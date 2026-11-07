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
}

impl Viewport {
    pub fn new(device: &wgpu::Device) -> Result<Self> {
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

        let mesh_pipeline = pipeline(
            device,
            &uniform_pipeline_layout,
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

        let plate = pipeline(
            device,
            &uniform_pipeline_layout,
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
            let below_plate = eye.z < 0.0;
            if !below_plate {
                pass.set_pipeline(&self.plate);
                pass.set_vertex_buffer(0, self.plate_vertex_buffer.slice(..));
                pass.set_index_buffer(self.plate_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.plate_index_count, 0, 0..1);
                pass.set_pipeline(&self.grid);
                pass.draw(0..6, 0..1);
            }
            pass.set_pipeline(&self.volume);
            pass.set_vertex_buffer(0, self.volume_buffer.slice(..));
            pass.draw(0..self.volume_vertex_count, 0..1);
            if let Some(mesh) = &self.mesh {
                if !below_plate && mesh.footprint_vertex_count > 0 {
                    pass.set_pipeline(&self.footprint);
                    pass.set_vertex_buffer(0, mesh.footprint_buffer.slice(..));
                    pass.draw(0..mesh.footprint_vertex_count, 0..1);
                }
                pass.set_pipeline(&self.mesh_pipeline);
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

const MESH_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

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
    out.position = uniforms.view_proj * vec4<f32>(position, 1.0);
    out.normal = normal;
    out.world = position;
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
    return vec4<f32>(0.035, 0.042, 0.052, 1.0);
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
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

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
    let plate = vec3<f32>(0.032, 0.040, 0.054);
    return vec4<f32>(plate * lighting, 1.0);
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
    let grid_color = vec3<f32>(0.020, 0.024, 0.030);
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
    let background = vec3<f32>(0.0033, 0.0048, 0.0069);
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
    var linear_alpha = 0.10 * 0.75;
    if (uv.y > 0.4) {
        let k = (uv.y - 0.4) / 0.6;
        linear_color = mix(violet, vec3<f32>(0.0), k);
        linear_alpha = mix(0.06 * 0.75, 0.0, k);
    } else {
        let k = uv.y / 0.4;
        linear_color = mix(pink, violet, k);
        linear_alpha = mix(0.10 * 0.75, 0.06 * 0.75, k);
    }
    let overlay = pink * radial_alpha + linear_color * linear_alpha;
    let scene_srgb = linear_to_srgb(color);
    let blended = vec3<f32>(1.0) - (vec3<f32>(1.0) - scene_srgb) * (vec3<f32>(1.0) - overlay);
    return vec4<f32>(srgb_to_linear(blended), 1.0);
}
"#;
