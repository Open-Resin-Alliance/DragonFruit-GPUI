//! The wgpu viewport: background, build plate, grid, build-volume box, decals,
//! axes, the clay mesh and the app's mood overlay.
//!
//! PORT DEVIATIONS (against `src/components/scene/SceneCanvas/SceneEnvironment.tsx`,
//! `SceneCanvas.tsx`, `StlMesh.tsx` and `src/features/shaders/mesh/softClay.tsx`):
//!
//! Fixed by this pass (were wrong before):
//! - Plate: colour `#323841`, opacity `0.94 * buildPlateOpacity` (alpha-blended
//!   instead of faked by mixing toward the background), thickness
//!   `min(3, max(1.5, width*0.05))`, corner radius `max(0.2, min(3, halfW-0.2,
//!   halfD-0.2))`, `curveSegments 18`, centre `z = -thickness/2 - 0.08`, front
//!   tab from `plateFrontScale`. Lighting is now the real three.js
//!   `MeshStandardMaterial` sum (ambient 1.2, key 0.3 at +Z, fill `#90a7ff` at
//!   0.045 on -Z, hemisphere 0.72 with `n.y` weight, view headlight 1.0,
//!   roughness 1, metalness 0, F0 0.04) with ACES tone mapping, not a hand-tuned
//!   "hemi*0.28 + key*1.12" mix.
//! - Grid: generated as `THREE.GridHelper` does (`baseSize`, `divisions =
//!   max(8, round(max(20, min(240, round(baseSize/5)))/3))`, lines at
//!   `-size/2 + i*step`, `color1` only when `i === divisions/2`), drawn as real
//!   lines with vertex colours `#4f5560`/`#2c3138` at `z = -0.01`, scaled
//!   `(width/baseSize, 1, depth/baseSize)` about the build-volume centre. The
//!   old full-screen-quad shader had a half-cell phase offset, invented cell
//!   sizes and a fake plate underlay.
//! - Build-volume box: edges `#8abfff` at opacity 0.18 (`#ff5b6f`/0.36 when a
//!   model is out of bounds — that state is not wired here), back-face fill
//!   `#78b7ff` at 0.04, both transparent, `depthWrite: false`, tone-mapped,
//!   centred on the build volume and spanning `0..maxZ`.
//! - FRONT decal: plane at `(centerX, centerY + frontMarkerY, 0.001)`, size
//!   `markerWidth x markerDepth`, opacity `dimFactor` (1), UVs flipped 180° with
//!   the source's `depthMm`-wide hysteresis band around the plate centre.
//! - Logo decal: plane at `(logoX, logoY, 0.12)`, size `logoWidth x logoHeight`,
//!   opacity `0.4 * dimFactor` applied as alpha (the texture is no longer
//!   pre-multiplied by 0.4), double-sided, depth bias -2/-2.
//! - Axes: shaft spans `0..axisLength` and the cone `axisLength..axisLength +
//!   headLength` (the old arrow was short by one head), each part shading over
//!   its own length from `#8d232f`/`#1e6b35`/`#21428d` to
//!   `#ff7a7a`/`#74ff95`/`#74a3ff`, 12 segments, origin
//!   `(originMinX, originMinY, 0.5)`.
//! - Axis labels: sprite size `6 * plateScale` (was hard-coded 6.56), offset
//!   `axisLength + 8 * plateScale`, lift `max(0.12, plateScale)`.
//! - Plate opacity / below-plate culling: the source's own formulas
//!   (`updateCameraBelowBuildPlate`) instead of a single ortho band used for
//!   every projection, including the 4.8/7.0 mm height hysteresis and the
//!   side-view fade.
//! - Mesh material: roughness 0.9 (was 0.55), metalness 0.02, F0
//!   `mix(0.04, albedo, 0.02)`, real three.js light sum, fake AO
//!   (`dir (0.35,0.58,0.74)`, strength 0.2, `pow(1-NdotL,1.35)`) and baked AO at
//!   `0.75 * 0.85 = 0.6375` (was 0.85).
//! - Mood overlay: the two CSS layers are composited as the CSS does (linear
//!   alpha ramp, premultiplied gradient stops, `screen` with element alpha)
//!   instead of one smoothstep blob.
//!
//! Still not matched (platform / module boundaries):
//! - `camera.rs` (not this file): near/far are `distance*0.001`/`distance*100`
//!   instead of `CameraClipPlaneStabilizer`'s `clamp(dist/200, 0.02, 0.5)` /
//!   `clamp(dist*50, 200, 5000)`; the ortho frustum uses `near..far` instead of
//!   `±(radius + sceneRadius + 500)`; the default FOV is 45° instead of the
//!   app's 50°; the default view is `fit()` instead of
//!   `target + (-220, -220, 260)`.
//! - Safety-margin hazard stripes (`SafetyStripeMaterial`) are not drawn. The
//!   default printer profile carries no `safetyMarginMm`, so the source draws
//!   nothing either; a profile with margins would show stripes here.
//! - Plate name label, side buttons and ghost plates are 2D/widget work owned by
//!   other modules; nothing is drawn for them here.
//! - The grid's `material-opacity` fade for inactive plates, multi-plate
//!   offsets and the `circle` plate silhouette have no counterpart: this
//!   viewport draws one rectangular plate at the origin.
//! - `polygonOffset` is shared by both decals here: the pipeline uses the
//!   logo's `-2/-2` bias, so the FRONT marker is offset by -2/-2 where the
//!   source uses -1/-1. Both are decals a fraction of a millimetre above the
//!   plate; the difference is below what the depth buffer resolves.
//! - Axis label and FRONT marker glyphs are rasterised with fontdue from Arial
//!   Bold; the source uses the browser's canvas metrics (`Bold 42px Arial`,
//!   `700 70px Arial`), so glyph placement differs by fractions of a pixel.
//! - `SceneMoodOverlay`'s gradient colours are the dark-theme defaults
//!   (`--scene-gradient-radial` etc.); custom themes are not read here.

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
    /// `ViewHeadlight`'s direction: the camera's world forward, negated. The
    /// source light's target sits 100mm down that ray, so its direction is
    /// constant for the frame rather than per-fragment.
    headlight_dir: [f32; 3],
    _pad3: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SpriteUniform {
    right: [f32; 3],
    size: f32,
    up: [f32; 3],
    _pad: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct DecalUniform {
    /// `1` when the FRONT marker has turned half a revolution.
    flip: f32,
    /// `dimFactor` for the marker, `0.4 * dimFactor` for the logo.
    opacity: f32,
    _pad: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ColorUniform {
    color: [f32; 3],
    opacity: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ModelUniform {
    matrix: [[f32; 4]; 4],
    clip: [f32; 4],
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
    /// Non-sRGB view of the scene texture, the volume pass's resolve target.
    scene_unorm_view: wgpu::TextureView,
    _msaa_color: wgpu::Texture,
    msaa_color_view: wgpu::TextureView,
    /// Non-sRGB view of the MSAA samples, for the sRGB-blended volume box.
    msaa_color_unorm_view: wgpu::TextureView,
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
    volume_fill: wgpu::RenderPipeline,
    volume_fill_buffer: wgpu::Buffer,
    volume_fill_vertex_count: u32,
    volume_edge_bind: wgpu::BindGroup,
    volume_fill_bind: wgpu::BindGroup,
    volume_edge_uniform: wgpu::Buffer,
    volume_fill_uniform: wgpu::Buffer,
    axis: wgpu::RenderPipeline,
    axis_mesh: wgpu::RenderPipeline,
    axis_buffer: wgpu::Buffer,
    axis_vertex_count: u32,
    sprite: wgpu::RenderPipeline,
    sprite_uniform: wgpu::Buffer,
    sprite_bind: wgpu::BindGroup,
    sprite_buffer: wgpu::Buffer,
    sprite_vertex_count: u32,
    x_label_bind: wgpu::BindGroup,
    y_label_bind: wgpu::BindGroup,
    z_label_bind: wgpu::BindGroup,
    plate_uniform: wgpu::Buffer,
    plate_bind: wgpu::BindGroup,
    decal: wgpu::RenderPipeline,
    decal_buffer: wgpu::Buffer,
    decal_uniform: wgpu::Buffer,
    decal_bind: wgpu::BindGroup,
    logo_decal_uniform: wgpu::Buffer,
    logo_decal_bind: wgpu::BindGroup,
    front_bind: wgpu::BindGroup,
    logo_bind: wgpu::BindGroup,
    grid: wgpu::RenderPipeline,
    grid_buffer: wgpu::Buffer,
    grid_vertex_count: u32,
    footprint: wgpu::RenderPipeline,
    mesh_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    scene_bind: wgpu::BindGroup,
    composite_layout: wgpu::BindGroupLayout,
    linear_sampler: wgpu::Sampler,
    targets: Targets,
    size: (u32, u32),
    metrics: PlateMetrics,
    /// `isCameraBelowBuildPlate`, with the source's hysteresis.
    below_plate: bool,
    /// `FrontMarkerDecal`'s `sideRef`: `None` before the first frame.
    front_from_back: Option<bool>,
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
            size: 80,
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
        // `meshBasicMaterial` defaults to `FrontSide`.
        let axis_mesh = pipeline(
            device,
            &uniform_pipeline_layout,
            &axis_shader,
            &[Some(axis_layout.clone())],
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::Face::Back),
            false,
            wgpu::CompareFunction::LessEqual,
            WgpuSurface::DEFAULT_FORMAT,
            true,
            true,
            SAMPLE_COUNT,
        );
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
        clip_z: f32,
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

        let eye = camera.eye();
        let forward = (camera.target - eye).normalize();
        let view_signed_z = forward.z;
        let view_abs_z = view_signed_z.abs();
        let camera_z = eye.z;

        // `updateCameraBelowBuildPlate` (SceneCanvas.tsx), verbatim.
        let plate_opacity;
        if camera.orthographic {
            const ORTHO_FADE_VISIBLE_VIEW_Z: f32 = -0.06;
            const ORTHO_FADE_HIDDEN_VIEW_Z: f32 = 0.018;
            const ORTHO_ENTER_BELOW_VIEW_Z: f32 = 0.008;
            const ORTHO_EXIT_BELOW_VIEW_Z: f32 = -0.018;

            let ortho_fade_t = ((view_signed_z - ORTHO_FADE_VISIBLE_VIEW_Z)
                / (ORTHO_FADE_HIDDEN_VIEW_Z - ORTHO_FADE_VISIBLE_VIEW_Z).max(0.0001))
                .clamp(0.0, 1.0);
            let ortho_fade = ortho_fade_t * ortho_fade_t * (3.0 - 2.0 * ortho_fade_t);
            plate_opacity = 1.0 - ortho_fade;

            self.below_plate = if self.below_plate {
                view_signed_z > ORTHO_EXIT_BELOW_VIEW_Z
            } else {
                view_signed_z >= ORTHO_ENTER_BELOW_VIEW_Z
            };
        } else {
            const ENTER_BELOW_Z: f32 = 4.8;
            const EXIT_BELOW_Z: f32 = 7.0;
            const FADE_VISIBLE_Z: f32 = 12.0;
            const FADE_HIDDEN_Z: f32 = 3.8;
            const SIDE_VIEW_FADE_START_ABS_Z: f32 = 0.26;
            const SIDE_VIEW_FADE_END_ABS_Z: f32 = 0.08;
            const SIDE_VIEW_HEIGHT_MAX_Z: f32 = 20.0;
            const SIDE_ENTER_ABS_Z: f32 = 0.10;
            const SIDE_EXIT_ABS_Z: f32 = 0.20;
            const SIDE_ACTIVE_HEIGHT_MAX_Z: f32 = 22.0;

            let height_fade_t = ((camera_z - FADE_HIDDEN_Z)
                / (FADE_VISIBLE_Z - FADE_HIDDEN_Z).max(0.0001))
                .clamp(0.0, 1.0);
            let smooth_height_fade = height_fade_t * height_fade_t * (3.0 - 2.0 * height_fade_t);

            let side_view_raw = ((SIDE_VIEW_FADE_START_ABS_Z - view_abs_z)
                / (SIDE_VIEW_FADE_START_ABS_Z - SIDE_VIEW_FADE_END_ABS_Z).max(0.0001))
                .clamp(0.0, 1.0);
            let side_view_fade = side_view_raw * side_view_raw * (3.0 - 2.0 * side_view_raw);
            let side_height_gate =
                ((SIDE_VIEW_HEIGHT_MAX_Z - camera_z) / SIDE_VIEW_HEIGHT_MAX_Z).clamp(0.0, 1.0);

            plate_opacity = smooth_height_fade.min(1.0 - side_view_fade * side_height_gate);

            let height_triggered = if self.below_plate {
                camera_z < EXIT_BELOW_Z
            } else {
                camera_z < ENTER_BELOW_Z
            };
            let side_triggered = if camera_z < SIDE_ACTIVE_HEIGHT_MAX_Z {
                if self.below_plate {
                    view_abs_z < SIDE_EXIT_ABS_Z
                } else {
                    view_abs_z < SIDE_ENTER_ABS_Z
                }
            } else {
                false
            };
            self.below_plate = height_triggered || side_triggered;
        }
        let plate_opacity = plate_opacity.clamp(0.0, 1.0);
        // `meshStandardMaterial color={buildPlateColor}` with
        // `opacity={0.94 * plateOpacity}`.
        queue.write_buffer(
            &self.plate_uniform,
            0,
            bytemuck::bytes_of(&ColorUniform {
                color: srgb_linear(BUILD_PLATE_COLOR),
                opacity: 0.94 * plate_opacity,
            }),
        );

        let right = if forward.z.abs() > 0.99 {
            glam::Vec3::X
        } else {
            forward.cross(glam::Vec3::Z).normalize()
        };
        let up = right.cross(forward).normalize();
        queue.write_buffer(
            &self.sprite_uniform,
            0,
            bytemuck::bytes_of(&SpriteUniform {
                right: right.to_array(),
                // `AxisLabels`: `scale={[labelSize, labelSize, 1]}`.
                size: self.metrics.label_size,
                up: up.to_array(),
                _pad: 0.0,
            }),
        );

        queue.write_buffer(
            &self.model_uniform,
            0,
            bytemuck::bytes_of(&ModelUniform {
                matrix: model.to_cols_array_2d(),
                clip: [clip_z, 0.0, 0.0, 0.0],
            }),
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
                // `ViewHeadlight`: `light.position = camera.position`,
                // `target = position + viewDir * 100`, so the light shines along
                // `-viewDir`.
                headlight_dir: [-forward.x, -forward.y, -forward.z],
                _pad3: 0.0,
            }),
        );

        // `FrontMarkerDecal`: the marker turns half a revolution once the camera
        // crosses the plate's centre, with a `depthMm`-wide hysteresis band.
        let band = self.metrics.marker_depth.max(1.0);
        let center_y = self.metrics.center_y;
        let from_back = match self.front_from_back {
            None => eye.y > center_y,
            Some(true) => eye.y > center_y - band,
            Some(false) => eye.y > center_y + band,
        };
        self.front_from_back = Some(from_back);
        queue.write_buffer(
            &self.decal_uniform,
            0,
            bytemuck::bytes_of(&DecalUniform {
                flip: if from_back { 1.0 } else { 0.0 },
                opacity: 1.0,
                _pad: [0.0; 2],
            }),
        );
        queue.write_buffer(
            &self.logo_decal_uniform,
            0,
            bytemuck::bytes_of(&DecalUniform {
                flip: 0.0,
                opacity: 0.4,
                _pad: [0.0; 2],
            }),
        );
        queue.write_buffer(
            &self.volume_edge_uniform,
            0,
            bytemuck::bytes_of(&ColorUniform {
                color: srgb_linear(0x8abfff),
                opacity: 0.18,
            }),
        );
        queue.write_buffer(
            &self.volume_fill_uniform,
            0,
            bytemuck::bytes_of(&ColorUniform {
                color: srgb_linear(0x78b7ff),
                opacity: 0.04,
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
                            r: 0.0056,
                            g: 0.0061,
                            b: 0.0080,
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
            let below_plate = self.below_plate;
            let plate_visible = layers.plate && plate_opacity > 0.001;
            if !below_plate && plate_visible {
                pass.set_pipeline(&self.plate);
                pass.set_bind_group(1, &self.plate_bind, &[]);
                pass.set_vertex_buffer(0, self.plate_vertex_buffer.slice(..));
                pass.set_index_buffer(self.plate_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.plate_index_count, 0, 0..1);
            }
            if !below_plate && layers.grid && plate_opacity > 0.001 {
                pass.set_pipeline(&self.grid);
                pass.set_vertex_buffer(0, self.grid_buffer.slice(..));
                pass.draw(0..self.grid_vertex_count, 0..1);
            }
            if layers.grid {
                pass.set_pipeline(&self.axis_mesh);
                pass.set_vertex_buffer(0, self.axis_buffer.slice(..));
                pass.draw(0..self.axis_vertex_count, 0..1);
            }
            if layers.plate && plate_opacity > 0.001 {
                pass.set_pipeline(&self.decal);
                pass.set_vertex_buffer(0, self.decal_buffer.slice(..));
                pass.set_bind_group(1, &self.decal_bind, &[]);
                pass.set_bind_group(2, &self.front_bind, &[]);
                pass.draw(0..6, 0..1);
                if layers.grid {
                    pass.set_bind_group(1, &self.logo_decal_bind, &[]);
                    pass.set_bind_group(2, &self.logo_bind, &[]);
                    pass.draw(6..12, 0..1);
                }
            }
            if layers.grid {
                pass.set_pipeline(&self.sprite);
                pass.set_bind_group(1, &self.sprite_bind, &[]);
                pass.set_vertex_buffer(0, self.sprite_buffer.slice(..));
                pass.set_bind_group(2, &self.x_label_bind, &[]);
                pass.draw(0..6, 0..1);
                pass.set_bind_group(2, &self.y_label_bind, &[]);
                pass.draw(6..12, 0..1);
                pass.set_bind_group(2, &self.z_label_bind, &[]);
                pass.draw(12..18, 0..1);
            }
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

        // Build-volume box, after the opaque scene (three.js `renderOrder` 28/29
        // puts it over the model) and into the non-sRGB view of the MSAA
        // samples, so its transparency blends in sRGB space like the source's
        // RGBA8 canvas instead of in linear space.
        if layers.volume {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport volume"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.targets.msaa_color_unorm_view,
                    depth_slice: None,
                    resolve_target: Some(&self.targets.scene_unorm_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.targets.msaa_depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.scene_bind, &[]);
            pass.set_pipeline(&self.volume_fill);
            pass.set_bind_group(1, &self.volume_fill_bind, &[]);
            pass.set_vertex_buffer(0, self.volume_fill_buffer.slice(..));
            pass.draw(0..self.volume_fill_vertex_count, 0..1);
            pass.set_pipeline(&self.volume);
            pass.set_bind_group(1, &self.volume_edge_bind, &[]);
            pass.set_vertex_buffer(0, self.volume_buffer.slice(..));
            pass.draw(0..self.volume_vertex_count, 0..1);
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
        let scene_unorm_view = scene.create_view(&wgpu::TextureViewDescriptor {
            format: Some(wgpu::TextureFormat::Rgba8Unorm),
            ..Default::default()
        });
        let msaa_color = msaa_color_target(device, width, height);
        let msaa_color_view = msaa_color.create_view(&wgpu::TextureViewDescriptor::default());
        let msaa_color_unorm_view = msaa_color.create_view(&wgpu::TextureViewDescriptor {
            format: Some(wgpu::TextureFormat::Rgba8Unorm),
            ..Default::default()
        });
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
            scene_unorm_view,
            _msaa_color: msaa_color,
            msaa_color_view,
            msaa_color_unorm_view,
            _msaa_depth: msaa_depth,
            msaa_depth_view,
            final_texture,
            final_view,
            composite_bind,
        }
    }
}

// The scene's own colours (dark theme, `Helpers`).
const BUILD_PLATE_COLOR: u32 = 0x323841;
const GRID_MAJOR_COLOR: u32 = 0x4f5560;
const GRID_MINOR_COLOR: u32 = 0x2c3138;

/// `FRONT_MARKER_MAX_TEXT_WIDTH`: the word is fitted to 200 of the texture's
/// 256 pixels.
const FRONT_MARKER_MAX_TEXT_WIDTH: f32 = 200.0;

/// Build-plate decorations, derived exactly as `SceneEnvironment.tsx` does.
/// `width`/`depth` are the build-volume dimensions, not the plate's.
struct PlateMetrics {
    width: f32,
    depth: f32,
    center_x: f32,
    center_y: f32,
    volume_height: f32,
    grid_base_size: f32,
    grid_divisions: u32,
    grid_scale_x: f32,
    grid_scale_z: f32,
    plate_width: f32,
    plate_depth: f32,
    plate_thickness: f32,
    plate_corner_radius: f32,
    plate_center_z: f32,
    tab_depth: f32,
    tab_back: f32,
    tab_front: f32,
    marker_width: f32,
    marker_depth: f32,
    marker_y: f32,
    scale: f32,
    axis_length: f32,
    shaft_radius: f32,
    head_radius: f32,
    head_length: f32,
    label_lift: f32,
    label_offset: f32,
    label_size: f32,
    logo_width: f32,
    logo_height: f32,
    logo_x: f32,
    logo_y: f32,
}

impl PlateMetrics {
    /// `width`/`depth`/`height` are the build volume; `origin_min_x`/`origin_min_y`
    /// are the front-left origin (0,0) or the centred one (-w/2,-d/2).
    fn new(width: f32, depth: f32, height: f32, origin_min_x: f32, origin_min_y: f32) -> Self {
        let build_volume_center_x = origin_min_x + width * 0.5;
        let build_volume_center_y = origin_min_y + depth * 0.5;
        let base_size = width.max(depth);
        let base_divisions = (base_size / 5.0).round().clamp(20.0, 240.0);
        let divisions = (base_divisions / 3.0).round().max(8.0) as u32;
        let scale_x = width / base_size;
        let scale_z = depth / base_size;

        let oversize = 3.0;
        let plate_width = width + oversize * 2.0;
        let plate_depth = depth + oversize * 2.0;
        let thickness = (width * 0.05).clamp(1.5, 3.0);
        let corner_radius = 3.0;
        let plate_center_z = -thickness * 0.5 - 0.08;

        let plate_scale = (width / 200.0).clamp(0.16, 1.5);
        let front_scale = (width / 200.0).clamp(0.55, 1.5) * 1.2;
        let tab_depth = 4.2 * front_scale;
        let tab_back = (plate_width - width * 0.072).min(24.0 * front_scale);
        let tab_front = (tab_back - width * 0.018).min(16.0 * front_scale);
        let marker_inset = width * 0.0012;
        let aspect = 256.0 / 72.0;
        let marker_depth =
            (tab_depth - marker_inset * 2.0).min((tab_back - marker_inset * 2.0) / aspect);
        let marker_width = marker_depth * aspect;
        let marker_y = -plate_depth * 0.5 - tab_depth * 0.35;

        let axis_length = 22.0 * plate_scale;
        let shaft_radius = (0.42 * plate_scale).max(0.05);
        let head_radius = (1.3 * plate_scale).max(0.18);
        let head_length = (1.9 * plate_scale).max(0.28);
        let label_lift = (1.0 * plate_scale).max(0.12);
        let label_size = 6.0 * plate_scale;
        // `AxisLabels size={length + 6 * labelScale}`, then
        // `labelOffset = size + 2 * scale`.
        let label_offset = axis_length + 6.0 * plate_scale + 2.0 * plate_scale;

        let logo_aspect = 1772.0 / 304.0;
        let logo_base_width = (width * 0.2).clamp(3.0, 42.0);
        let logo_width = logo_base_width;
        let logo_height = logo_base_width / logo_aspect;
        let logo_inset = (width * 0.08).min(1.6);
        let logo_x = origin_min_x + width - logo_inset - logo_width * 0.5;
        let logo_y = origin_min_y + logo_inset + logo_height * 0.5;

        Self {
            width,
            depth,
            center_x: build_volume_center_x,
            center_y: build_volume_center_y,
            volume_height: height,
            grid_base_size: base_size,
            grid_divisions: divisions,
            grid_scale_x: scale_x,
            grid_scale_z: scale_z,
            plate_width,
            plate_depth,
            plate_thickness: thickness,
            plate_corner_radius: corner_radius,
            plate_center_z,
            tab_depth,
            tab_back,
            tab_front,
            marker_width,
            marker_depth,
            marker_y,
            scale: plate_scale,
            axis_length,
            shaft_radius,
            head_radius,
            head_length,
            label_lift,
            label_offset,
            label_size,
            logo_width,
            logo_height,
            logo_x,
            logo_y,
        }
    }

    /// The plate's top surface, `z0` in `plate_mesh`.
    fn plate_top_z(&self) -> f32 {
        self.plate_center_z + self.plate_thickness * 0.5
    }

    /// `resolvedOriginMinX`.
    fn origin_min_x(&self) -> f32 {
        self.center_x - self.width * 0.5
    }

    /// `resolvedOriginMinY`.
    fn origin_min_y(&self) -> f32 {
        self.center_y - self.depth * 0.5
    }
}

fn srgb_linear(hex: u32) -> [f32; 3] {
    let channel = |shift: u32| {
        let value = ((hex >> shift) & 0xff) as f32 / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    [channel(16), channel(8), channel(0)]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length <= 1e-9 {
        [0.0, 0.0, 1.0]
    } else {
        [v[0] / length, v[1] / length, v[2] / length]
    }
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The three axis arrows: cylinders with cone heads, coloured with the source's
/// base-to-tip gradients. The shaft spans `0..length` and the cone
/// `length..length + headLength`, each shading over its own length (the source
/// maps the same 16x256 gradient onto both geometries, so the colour restarts at
/// the cone's base).
fn axis_arrows(metrics: &PlateMetrics) -> Vec<[f32; 6]> {
    let origin = [metrics.origin_min_x(), metrics.origin_min_y(), 0.5];
    let length = metrics.axis_length;
    let shaft_radius = metrics.shaft_radius;
    let head_radius = metrics.head_radius;
    let head_length = metrics.head_length;
    let segments = 12;
    let axes: [([f32; 3], u32, u32); 3] = [
        ([1.0, 0.0, 0.0], 0x8d232f, 0xff7a7a),
        ([0.0, 1.0, 0.0], 0x1e6b35, 0x74ff95),
        ([0.0, 0.0, 1.0], 0x21428d, 0x74a3ff),
    ];
    let mut out: Vec<[f32; 6]> = Vec::new();
    let mut push = |point: [f32; 3], color: [f32; 3]| {
        out.push([point[0], point[1], point[2], color[0], color[1], color[2]]);
    };
    for (dir, base_hex, tip_hex) in axes {
        let base = srgb_linear(base_hex);
        let tip = srgb_linear(tip_hex);
        let helper = if dir[2].abs() > 0.9 {
            [1.0, 0.0, 0.0]
        } else {
            [0.0, 0.0, 1.0]
        };
        let u = normalize(cross(dir, helper));
        let v = cross(dir, u);
        let ring = |t: f32, radius: f32, angle: f32| -> [f32; 3] {
            let (sin, cos) = angle.sin_cos();
            [
                origin[0] + dir[0] * t + (u[0] * cos + v[0] * sin) * radius,
                origin[1] + dir[1] * t + (u[1] * cos + v[1] * sin) * radius,
                origin[2] + dir[2] * t + (u[2] * cos + v[2] * sin) * radius,
            ]
        };
        // `v` runs 0 at the geometry's base to 1 at its tip; the canvas gradient
        // is dark at v=0 and light at v=1.
        let shade = |k: f32| -> [f32; 3] {
            let k = k.clamp(0.0, 1.0);
            [
                base[0] + (tip[0] - base[0]) * k,
                base[1] + (tip[1] - base[1]) * k,
                base[2] + (tip[2] - base[2]) * k,
            ]
        };
        let apex = [
            origin[0] + dir[0] * (length + head_length),
            origin[1] + dir[1] * (length + head_length),
            origin[2] + dir[2] * (length + head_length),
        ];
        for i in 0..segments {
            let a0 = i as f32 / segments as f32 * std::f32::consts::TAU;
            let a1 = (i + 1) as f32 / segments as f32 * std::f32::consts::TAU;
            // Shaft: base ring at 0 (dark), top ring at `length` (light).
            let p00 = ring(0.0, shaft_radius, a0);
            let p01 = ring(0.0, shaft_radius, a1);
            let p10 = ring(length, shaft_radius, a0);
            let p11 = ring(length, shaft_radius, a1);
            let c0 = shade(0.0);
            let c1 = shade(1.0);
            push(p00, c0);
            push(p11, c1);
            push(p10, c1);
            push(p00, c0);
            push(p01, c0);
            push(p11, c1);
            // Cone: its own base (dark) to its apex (light).
            let b0 = ring(length, head_radius, a0);
            let b1 = ring(length, head_radius, a1);
            push(b0, c0);
            push(b1, c0);
            push(apex, c1);
        }
    }
    out
}

/// Billboard quads for the X/Y/Z letters, one per axis tip.
fn sprite_vertices(metrics: &PlateMetrics) -> Vec<[f32; 5]> {
    let origin = [metrics.origin_min_x(), metrics.origin_min_y(), 0.5];
    let label_offset = metrics.label_offset;
    let lift = metrics.label_lift;
    let mut out = Vec::new();
    let mut quad = |center: [f32; 3]| {
        let corners = [
            (0.0, 1.0),
            (1.0, 1.0),
            (1.0, 0.0),
            (0.0, 1.0),
            (1.0, 0.0),
            (0.0, 0.0),
        ];
        for (u, v) in corners {
            out.push([center[0], center[1], center[2], u, v]);
        }
    };
    quad([origin[0] + label_offset, origin[1], origin[2] + lift]);
    quad([origin[0], origin[1] + label_offset, origin[2] + lift]);
    quad([origin[0], origin[1], origin[2] + label_offset + lift]);
    out
}

/// `THREE.GridHelper(baseSize, divisions, color1, color2)`, placed, rotated 90°
/// about X and scaled exactly as the source does it: lines every `step` from
/// `-size/2`, `color1` only on the line at index `divisions / 2`.
fn grid_vertices(metrics: &PlateMetrics) -> Vec<[f32; 6]> {
    let size = metrics.grid_base_size;
    let divisions = metrics.grid_divisions;
    let step = size / divisions as f32;
    let half = size * 0.5;
    let center = divisions as f32 * 0.5;
    let major = srgb_linear(GRID_MAJOR_COLOR);
    let minor = srgb_linear(GRID_MINOR_COLOR);
    let z = -0.01;
    let mut out: Vec<[f32; 6]> = Vec::with_capacity((divisions as usize + 1) * 4);
    // Object transform: T(center) * R_x(90°) * S(scaleX, 1, scaleZ).
    let place = |x: f32, k: f32| -> [f32; 3] {
        [
            metrics.center_x + x * metrics.grid_scale_x,
            metrics.center_y - k * metrics.grid_scale_z,
            z,
        ]
    };
    for i in 0..=divisions {
        let k = -half + i as f32 * step;
        let color = if i as f32 == center { major } else { minor };
        let a = place(-half, k);
        let b = place(half, k);
        out.push([a[0], a[1], a[2], color[0], color[1], color[2]]);
        out.push([b[0], b[1], b[2], color[0], color[1], color[2]]);
        let c = place(k, -half);
        let d = place(k, half);
        out.push([c[0], c[1], c[2], color[0], color[1], color[2]]);
        out.push([d[0], d[1], d[2], color[0], color[1], color[2]]);
    }
    out
}

/// `AxisLabels.tsx`: 64x64 canvas, `Bold 42px Arial`, centred, pure colour.
fn axis_label_texture(letter: &str, hex: u32) -> (u32, u32, Vec<u8>) {
    let width = 64u32;
    let height = 64u32;
    let mut data = vec![0u8; (width * height * 4) as usize];
    let Ok(bytes) = std::fs::read("C:/Windows/Fonts/arialbd.ttf") else {
        return (width, height, data);
    };
    let Ok(font) = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()) else {
        return (width, height, data);
    };
    let size = 42.0f32;
    let Some(ch) = letter.chars().next() else {
        return (width, height, data);
    };
    let (metrics, bitmap) = font.rasterize(ch, size);
    let color = [
        ((hex >> 16) & 0xff) as f32,
        ((hex >> 8) & 0xff) as f32,
        (hex & 0xff) as f32,
    ];
    let gx = 32.0 - metrics.advance_width * 0.5 + metrics.xmin as f32;
    let gy = 32.0 + size * 0.35 - metrics.ymin as f32 - metrics.height as f32;
    for row in 0..metrics.height {
        for col in 0..metrics.width {
            let coverage = bitmap[row * metrics.width + col] as f32 / 255.0;
            if coverage <= 0.0 {
                continue;
            }
            let px = gx as i32 + col as i32;
            let py = gy as i32 + row as i32;
            if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
                continue;
            }
            let index = ((py as u32 * width + px as u32) * 4) as usize;
            data[index] = (color[0] * coverage) as u8;
            data[index + 1] = (color[1] * coverage) as u8;
            data[index + 2] = (color[2] * coverage) as u8;
            data[index + 3] = (255.0 * coverage) as u8;
        }
    }
    (width, height, data)
}

/// `EdgesGeometry` of the build-volume box: centred on the build volume, from
/// `z = 0` to `z = height`.
fn build_volume_edges(
    center_x: f32,
    center_y: f32,
    width: f32,
    depth: f32,
    height: f32,
) -> Vec<[f32; 3]> {
    let hx = width * 0.5;
    let hy = depth * 0.5;
    let corners = [
        [center_x - hx, center_y - hy, 0.0],
        [center_x + hx, center_y - hy, 0.0],
        [center_x + hx, center_y + hy, 0.0],
        [center_x - hx, center_y + hy, 0.0],
        [center_x - hx, center_y - hy, height],
        [center_x + hx, center_y - hy, height],
        [center_x + hx, center_y + hy, height],
        [center_x - hx, center_y + hy, height],
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

/// The box's six faces, wound outward; the pipeline culls the front faces so
/// only the far side paints (`side={THREE.BackSide}`).
fn build_volume_fill(
    center_x: f32,
    center_y: f32,
    width: f32,
    depth: f32,
    height: f32,
) -> Vec<[f32; 3]> {
    let hx = width * 0.5;
    let hy = depth * 0.5;
    let c = [
        [center_x - hx, center_y - hy, 0.0],
        [center_x + hx, center_y - hy, 0.0],
        [center_x + hx, center_y + hy, 0.0],
        [center_x - hx, center_y + hy, 0.0],
        [center_x - hx, center_y - hy, height],
        [center_x + hx, center_y - hy, height],
        [center_x + hx, center_y + hy, height],
        [center_x - hx, center_y + hy, height],
    ];
    let faces = [
        // bottom (-Z), top (+Z), front (-Y), right (+X), back (+Y), left (-X)
        (0, 2, 1), (0, 3, 2),
        (4, 5, 6), (4, 6, 7),
        (0, 1, 5), (0, 5, 4),
        (1, 2, 6), (1, 6, 5),
        (2, 3, 7), (2, 7, 6),
        (3, 0, 4), (3, 4, 7),
    ];
    let mut out = Vec::with_capacity(faces.len() * 3);
    for (a, b, d) in faces {
        out.push(c[a]);
        out.push(c[b]);
        out.push(c[d]);
    }
    out
}

/// The plate's `ExtrudeGeometry`: the source's shape (rounded rectangle with a
/// trapezoid tab on the front edge), extruded `thickness` and centred on
/// `plateCenterZ`.
fn plate_mesh(metrics: &PlateMetrics) -> (Vec<Vertex>, Vec<u32>) {
    let width = metrics.plate_width;
    let depth = metrics.plate_depth;
    let thickness = metrics.plate_thickness;
    let top = metrics.plate_top_z();
    let half_w = width * 0.5;
    let half_d = depth * 0.5;
    let radius = metrics.plate_corner_radius;
    let r = radius
        .min(half_w - 0.2)
        .min(half_d - 0.2)
        .max(0.2);
    let segments: u32 = 18;
    let z0 = top;
    let z1 = top - thickness;

    let mut perimeter: Vec<[f32; 2]> = vec![[-half_w + r, -half_d]];
    // Front tab, sized by `PlateMetrics` from the build volume width.
    let back = metrics.tab_back * 0.5;
    let front = metrics.tab_front * 0.5;
    let tab_y = -half_d - metrics.tab_depth;
    perimeter.push([-back, -half_d]);
    perimeter.push([-front, tab_y]);
    perimeter.push([front, tab_y]);
    perimeter.push([back, -half_d]);
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
    // The source positions the mesh at `[buildVolumeCenterX, buildVolumeCenterY,
    // buildPlateCenterZ]`; the shape itself is local.
    let ox = metrics.center_x;
    let oy = metrics.center_y;

    let top_center = vertices.len() as u32;
    vertices.push(Vertex { pos: [ox, oy, z0], normal: [0.0, 0.0, 1.0], ao: 1.0 });
    let top_start = vertices.len() as u32;
    for p in &perimeter {
        vertices.push(Vertex { pos: [p[0] + ox, p[1] + oy, z0], normal: [0.0, 0.0, 1.0], ao: 1.0 });
    }
    for i in 0..count {
        let a = top_start + i as u32;
        let b = top_start + ((i + 1) % count) as u32;
        indices.extend_from_slice(&[top_center, a, b]);
    }

    let bottom_center = vertices.len() as u32;
    vertices.push(Vertex { pos: [ox, oy, z1], normal: [0.0, 0.0, -1.0], ao: 1.0 });
    let bottom_start = vertices.len() as u32;
    for p in &perimeter {
        vertices.push(Vertex { pos: [p[0] + ox, p[1] + oy, z1], normal: [0.0, 0.0, -1.0], ao: 1.0 });
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
        vertices.push(Vertex { pos: [p0[0] + ox, p0[1] + oy, z0], normal, ao: 1.0 });
        vertices.push(Vertex { pos: [p1[0] + ox, p1[1] + oy, z0], normal, ao: 1.0 });
        vertices.push(Vertex { pos: [p1[0] + ox, p1[1] + oy, z1], normal, ao: 1.0 });
        vertices.push(Vertex { pos: [p0[0] + ox, p0[1] + oy, z1], normal, ao: 1.0 });
        indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
    }

    (vertices, indices)
}

/// Generates the mip chain for the decal textures. The source's
/// `CanvasTexture`s use `LinearMipmapLinearFilter`; without mips the 1772px
/// wordmark sampled onto a 42mm quad aliases badly.
struct MipGenerator {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl MipGenerator {
    fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport mip"),
            source: wgpu::ShaderSource::Wgsl(MIP_SHADER.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("viewport mip"),
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
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("viewport mip"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("viewport mip"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
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
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("viewport mip sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        Self {
            pipeline,
            layout,
            sampler,
        }
    }

    fn create(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> wgpu::Texture {
        let levels = 32 - width.max(height).leading_zeros();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
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
        if levels > 1 {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("viewport mips"),
            });
            for level in 1..levels {
                let source = texture.create_view(&wgpu::TextureViewDescriptor {
                    base_mip_level: level - 1,
                    mip_level_count: Some(1),
                    ..Default::default()
                });
                let destination = texture.create_view(&wgpu::TextureViewDescriptor {
                    base_mip_level: level,
                    mip_level_count: Some(1),
                    ..Default::default()
                });
                let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("viewport mip"),
                    layout: &self.layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&source),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                    ],
                });
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("viewport mip pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &destination,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &bind, &[]);
                pass.draw(0..3, 0..1);
            }
            queue.submit(Some(encoder.finish()));
        }
        texture
    }
}

fn front_texture() -> (u32, u32, Vec<u8>) {
    // `SceneEnvironment.tsx`: 256x72 canvas, `700 70px Arial` fitted to 200px,
    // centred at y + 5, colour = gridMajor (#4f5560) lerped 36% toward white.
    let width = 256u32;
    let height = 72u32;
    let mut data = vec![0u8; (width * height * 4) as usize];
    let Ok(bytes) = std::fs::read("C:/Windows/Fonts/arialbd.ttf") else {
        return (width, height, data);
    };
    let Ok(font) = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()) else {
        return (width, height, data);
    };
    let text = "FRONT";
    let measure = |size: f32| -> f32 {
        text.chars()
            .map(|ch| font.metrics(ch, size).advance_width)
            .sum()
    };
    let mut size = 70.0f32;
    let natural = measure(size);
    // `fitFontToWidth`: shrink by the overflow ratio, floored at 8px.
    if natural > FRONT_MARKER_MAX_TEXT_WIDTH && natural > 0.0 {
        size = (size * FRONT_MARKER_MAX_TEXT_WIDTH / natural).floor().max(8.0);
    }
    let total = measure(size);
    let start_x = (width as f32 - total) / 2.0;
    let baseline = height as f32 / 2.0 + 5.0 + size * 0.35;
    let color = front_marker_color();
    let mut pen = start_x;
    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, size);
        let gx = pen + metrics.xmin as f32;
        let gy = baseline - metrics.ymin as f32 - metrics.height as f32;
        for row in 0..metrics.height {
            for col in 0..metrics.width {
                let coverage = bitmap[row * metrics.width + col] as f32 / 255.0;
                if coverage <= 0.0 {
                    continue;
                }
                let px = gx as i32 + col as i32;
                let py = gy as i32 + row as i32;
                if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
                    continue;
                }
                let index = ((py as u32 * width + px as u32) * 4) as usize;
                data[index] = (color[0] as f32 * coverage) as u8;
                data[index + 1] = (color[1] as f32 * coverage) as u8;
                data[index + 2] = (color[2] as f32 * coverage) as u8;
                data[index + 3] = (255.0 * coverage) as u8;
            }
        }
        pen += metrics.advance_width;
    }
    (width, height, data)
}

/// `new THREE.Color(gridMajorColor).lerp(white, 0.36)`.
fn front_marker_color() -> [u8; 3] {
    let channel = |shift: u32| -> u8 {
        let from = ((GRID_MAJOR_COLOR >> shift) & 0xff) as f32;
        let to = 255.0f32;
        (from + (to - from) * 0.36).round() as u8
    };
    [channel(16), channel(8), channel(0)]
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
    // `resvg` hands back premultiplied RGBA; three's `meshBasicMaterial` blends
    // with straight alpha (`SrcAlpha`), so undo the premultiply and leave the
    // 0.4 opacity to the shader.
    let mut data = pixmap.data().to_vec();
    for pixel in data.chunks_exact_mut(4) {
        let alpha = pixel[3] as f32 / 255.0;
        if alpha > 0.0 {
            for channel in &mut pixel[..3] {
                *channel = ((*channel as f32 / alpha).min(255.0)) as u8;
            }
        }
    }
    (width, height, data)
}

fn decal_vertices(metrics: &PlateMetrics) -> Vec<[f32; 5]> {
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
    quad(
        metrics.center_x,
        metrics.center_y + metrics.marker_y,
        metrics.marker_width,
        metrics.marker_depth,
        0.001,
    );
    quad(
        metrics.logo_x,
        metrics.logo_y,
        metrics.logo_width,
        metrics.logo_height,
        0.12,
    );
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

/// The same pipeline, with straight-alpha blending and a depth bias: three's
/// `transparent` materials and its `polygonOffset` decals.
fn pipeline_blended(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    buffers: &[Option<wgpu::VertexBufferLayout<'_>>],
    topology: wgpu::PrimitiveTopology,
    cull: Option<wgpu::Face>,
    depth_write: bool,
    depth_compare: wgpu::CompareFunction,
    format: wgpu::TextureFormat,
    bias: wgpu::DepthBiasState,
    sample_count: u32,
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
            topology,
            cull_mode: cull,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(depth_write),
            depth_compare: Some(depth_compare),
            stencil: wgpu::StencilState::default(),
            bias,
        }),
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
        // The volume pass resolves sRGB-encoded samples through a non-sRGB
        // view; the composite still samples the sRGB view.
        view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
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
        // The build-volume box is blended in sRGB space, as three.js does on
        // its RGBA8 canvas; that needs a non-sRGB view of the same samples.
        view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
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


const SPRITE_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
    headlight_dir: vec3<f32>,
};
struct Sprite {
    right: vec3<f32>,
    size: f32,
    up: vec3<f32>,
    _pad: f32,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var<uniform> sprite: Sprite;
@group(2) @binding(0) var tex: texture_2d<f32>;
@group(2) @binding(1) var samp: sampler;

struct SpriteOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@location(0) center: vec3<f32>, @location(1) uv: vec2<f32>) -> SpriteOut {
    var out: SpriteOut;
    let world = center
        + sprite.right * (uv.x - 0.5) * sprite.size
        + sprite.up * (0.5 - uv.y) * sprite.size;
    out.position = uniforms.view_proj * vec4<f32>(world, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: SpriteOut) -> @location(0) vec4<f32> {
    return textureSample(tex, samp, in.uv);
}
"#;

const MIP_SHADER: &str = r#"
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

struct MipOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> MipOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let p = positions[index];
    var out: MipOut;
    out.position = vec4<f32>(p, 0.0, 1.0);
    out.uv = vec2<f32>((p.x + 1.0) * 0.5, (1.0 - p.y) * 0.5);
    return out;
}

@fragment
fn fs_main(in: MipOut) -> @location(0) vec4<f32> {
    return textureSample(tex, samp, in.uv);
}
"#;

const DECAL_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
    headlight_dir: vec3<f32>,
};
struct Decal {
    flip: f32,
    opacity: f32,
    _pad0: f32,
    _pad1: f32,
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
    let texel = textureSample(tex, samp, in.uv);
    return vec4<f32>(texel.rgb, texel.a * decal.opacity);
}
"#;

const MESH_SHADER: &str = r#"

struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
    headlight_dir: vec3<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct Model {
    matrix: mat4x4<f32>,
    clip: vec4<f32>,
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
fn brdf_ggx(light_dir: vec3<f32>, view_dir: vec3<f32>, n: vec3<f32>, roughness: f32, f0: vec3<f32>) -> vec3<f32> {
    let h = normalize(light_dir + view_dir);
    let n_dot_l = saturate(dot(n, light_dir));
    let n_dot_v = saturate(dot(n, view_dir));
    let n_dot_h = saturate(dot(n, h));
    let v_dot_h = saturate(dot(view_dir, h));

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

// `ACESFilmicToneMapping`, exposure 1.
fn aces_filmic(color: vec3<f32>) -> vec3<f32> {
    let input_mat = mat3x3<f32>(
        0.59719, 0.07600, 0.02840,
        0.35458, 0.90834, 0.13383,
        0.04823, 0.01566, 0.83777,
    );
    let output_mat = mat3x3<f32>(
         1.60475, -0.10208, -0.00327,
        -0.53108,  1.10813, -0.07276,
        -0.07367, -0.00605,  1.07602,
    );
    var c = color * (1.0 / 0.6);
    c = input_mat * c;
    let a = c * (c + 0.0245786) - 0.000090537;
    let b = c * (0.983729 * c + 0.4329510) + 0.238081;
    c = a / b;
    c = output_mat * c;
    return saturate(c);
}

// three.js BRDF_GGX, the dc9dacee calibration.
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
    if (in.world.z > model.clip.x) {
        discard;
    }
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
    headlight_dir: vec3<f32>,
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
    headlight_dir: vec3<f32>,
};
struct Fill {
    color: vec3<f32>,
    opacity: f32,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var<uniform> fill: Fill;

fn aces_filmic(color: vec3<f32>) -> vec3<f32> {
    let input_mat = mat3x3<f32>(
        0.59719, 0.07600, 0.02840,
        0.35458, 0.90834, 0.13383,
        0.04823, 0.01566, 0.83777,
    );
    let output_mat = mat3x3<f32>(
         1.60475, -0.10208, -0.00327,
        -0.53108,  1.10813, -0.07276,
        -0.07367, -0.00605,  1.07602,
    );
    var c = color;
    c = input_mat * c;
    let a = c * (c + 0.0245786) - 0.000090537;
    let b = c * (0.983729 * c + 0.4329510) + 0.238081;
    c = a / b;
    c = output_mat * c;
    return saturate(c);
}

fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
    let cutoff = step(c, vec3<f32>(0.0031308));
    let low = c * 12.92;
    let high = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055;
    return mix(high, low, cutoff);
}

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return uniforms.view_proj * vec4<f32>(position, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    // This pass renders into the non-sRGB view of the MSAA samples so the
    // transparent box blends in sRGB space, as three.js does on its RGBA8
    // canvas; the shader therefore writes sRGB-encoded values.
    return vec4<f32>(linear_to_srgb(aces_filmic(fill.color)), fill.opacity);
}
"#;

const PLATE_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
    headlight_dir: vec3<f32>,
};
struct Plate {
    color: vec3<f32>,
    opacity: f32,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var<uniform> plate: Plate;

// three.js BRDF_GGX: D_GGX * V_GGX_SmithCorrelated * F_Schlick.
fn brdf_ggx(light_dir: vec3<f32>, view_dir: vec3<f32>, n: vec3<f32>, roughness: f32, f0: vec3<f32>) -> vec3<f32> {
    let h = normalize(light_dir + view_dir);
    let n_dot_l = saturate(dot(n, light_dir));
    let n_dot_v = saturate(dot(n, view_dir));
    let n_dot_h = saturate(dot(n, h));
    let v_dot_h = saturate(dot(view_dir, h));

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

// `ACESFilmicToneMapping`, exposure 1.
fn aces_filmic(color: vec3<f32>) -> vec3<f32> {
    let input_mat = mat3x3<f32>(
        0.59719, 0.07600, 0.02840,
        0.35458, 0.90834, 0.13383,
        0.04823, 0.01566, 0.83777,
    );
    let output_mat = mat3x3<f32>(
         1.60475, -0.10208, -0.00327,
        -0.53108,  1.10813, -0.07276,
        -0.07367, -0.00605,  1.07602,
    );
    var c = color * (1.0 / 0.6);
    c = input_mat * c;
    let a = c * (c + 0.0245786) - 0.000090537;
    let b = c * (0.983729 * c + 0.4329510) + 0.238081;
    c = a / b;
    c = output_mat * c;
    return saturate(c);
}

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

    // `meshStandardMaterial` with the defaults: roughness 1, metalness 0, so F0
    // is the dielectric 0.04 and the diffuse term carries the whole albedo.
    let base = plate.color;
    let f0 = vec3<f32>(0.04);
    let roughness = 1.0;

    let key_dir = vec3<f32>(0.0, 0.0, 1.0);
    let fill_dir = vec3<f32>(0.0, 0.0, -1.0);
    let head_dir = uniforms.headlight_dir;
    // `useSceneCollectionManager`'s defaults: ambient 0.28, directional 1.12,
    // headlight 1.0 (`Lights`'s `?? 1.0`). The hemisphere is ambient * 0.6 and
    // the fill is directional * 0.15; the colours are linear.
    let key_color = vec3<f32>(1.0) * 1.12;
    let fill_color = vec3<f32>(0.2789, 0.3864, 1.0) * 0.168;
    let head_color = vec3<f32>(1.0);
    let sky = vec3<f32>(0.9216, 0.8070, 1.0) * 0.168;
    let ground = vec3<f32>(0.0482, 0.0529, 0.1070) * 0.168;
    let hemi = mix(ground, sky, n.y * 0.5 + 0.5);
    let ambient = vec3<f32>(1.0) * 0.28;

    let irradiance = ambient
        + key_color * max(dot(n, key_dir), 0.0)
        + fill_color * max(dot(n, fill_dir), 0.0)
        + hemi
        + head_color * max(dot(n, head_dir), 0.0);
    var color = base * irradiance / 3.14159265;
    color += brdf_ggx(key_dir, view_dir, n, roughness, f0) * key_color * max(dot(n, key_dir), 0.0);
    color += brdf_ggx(fill_dir, view_dir, n, roughness, f0) * fill_color * max(dot(n, fill_dir), 0.0);
    color += brdf_ggx(head_dir, view_dir, n, roughness, f0) * head_color * max(dot(n, head_dir), 0.0);
    return vec4<f32>(aces_filmic(color), plate.opacity);
}
"#;

const FOOTPRINT_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
    headlight_dir: vec3<f32>,
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
    headlight_dir: vec3<f32>,
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
fn fs_main() -> @location(0) vec4<f32> {
    // The canvas is transparent over `backgroundColor: var(--surface-0)`;
    // #111216 in linear light.
    return vec4<f32>(0.0056, 0.0061, 0.0080, 1.0);
}
"#;

const COMPOSITE_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
    headlight_dir: vec3<f32>,
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
    let scene = textureSampleLevel(scene_tex, samp, uv, 0.0).rgb;

    // `SceneMoodOverlay`: two `mix-blend-mode: screen` layers over the canvas,
    // composited in sRGB the way CSS does it.
    let pink = vec3<f32>(1.0, 0.216, 0.667);   // #ff37aa
    let violet = vec3<f32>(0.435, 0.2, 1.0);   // #6f33ff
    var color = linear_to_srgb(scene);

    // radial-gradient(120% 95% at 50% 46%, transparent 56%, #ff37aa/18% 100%),
    // opacity 0.75.
    let radial = length((uv - vec2<f32>(0.5, 0.46)) / vec2<f32>(1.2, 0.95));
    let radial_alpha = clamp((radial - 0.56) / (1.0 - 0.56), 0.0, 1.0) * 0.18 * 0.75;
    color = mix(color, vec3<f32>(1.0) - (vec3<f32>(1.0) - color) * (vec3<f32>(1.0) - pink), radial_alpha);

    // linear-gradient(180deg, #ff37aa/10% 0%, #6f33ff/6% 40%, transparent 100%),
    // opacity 0.8. CSS interpolates gradients premultiplied, so the colour is
    // recovered by dividing the premultiplied mix back out.
    var layer_color = violet;
    var layer_alpha = 0.0;
    if (uv.y < 0.4) {
        let k = uv.y / 0.4;
        let alpha = 0.10 + (0.06 - 0.10) * k;
        let premultiplied = pink * 0.10 * (1.0 - k) + violet * 0.06 * k;
        layer_color = premultiplied / alpha;
        layer_alpha = alpha;
    } else {
        let k = (uv.y - 0.4) / 0.6;
        layer_color = violet;
        layer_alpha = 0.06 * (1.0 - k);
    }
    layer_alpha *= 0.8;
    color = mix(color, vec3<f32>(1.0) - (vec3<f32>(1.0) - color) * (vec3<f32>(1.0) - layer_color), layer_alpha);

    return vec4<f32>(srgb_to_linear(color), 1.0);
}
"#;
