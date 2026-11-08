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


mod construct;
mod cube;

pub use cube::CubeHover;
mod mesh;
mod pipeline;
mod plate;
mod render;
mod shaders;
mod targets;
mod uniforms;

use anyhow::Result;
use gpui_interop::WgpuSurface;

use self::plate::PlateMetrics;
use self::targets::Targets;
use self::uniforms::GpuMesh;

const SAMPLE_COUNT: u32 = 4;

/// `DEFAULT_SUPPORT_COLOR` in `src/supports/SupportProxyMeshLayer.tsx` — the
/// proxy layer's base grey, the colour `meshBasicMaterial` paints a support
/// with when nothing is selected or hovered.
pub const SUPPORT_COLOR: u32 = 0x9a9a9a;

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
    cube_pipeline: wgpu::RenderPipeline,
    cube_uniforms: [wgpu::Buffer; 2],
    cube_binds: [wgpu::BindGroup; 2],
    cube_vertex_buffer: wgpu::Buffer,
    /// `cube::highlight_vertices()`: the edge and corner boxes.
    cube_highlight_buffer: wgpu::Buffer,
    cube_target: wgpu::Texture,
    cube_target_view: wgpu::TextureView,
    cube_msaa: wgpu::Texture,
    cube_msaa_view: wgpu::TextureView,
    cube_texture_bind: wgpu::BindGroup,
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
    /// The transform the footprint hull was baked through.
    mesh_matrix: glam::Mat4,
    support: wgpu::RenderPipeline,
    support_uniform: wgpu::Buffer,
    support_bind: wgpu::BindGroup,
    /// Unit meshes: the cylinder the shaft batches scale, the sphere the nodes
    /// scale, the disk the contact pads scale.
    support_cylinder: wgpu::Buffer,
    support_cylinder_count: u32,
    support_sphere: wgpu::Buffer,
    support_sphere_count: u32,
    support_disk: wgpu::Buffer,
    support_disk_count: u32,
    support_straight_instances: Option<wgpu::Buffer>,
    support_straight_count: u32,
    support_tapered_instances: Option<wgpu::Buffer>,
    support_tapered_count: u32,
    support_node_instances: Option<wgpu::Buffer>,
    support_node_count: u32,
    support_disk_instances: Option<wgpu::Buffer>,
    support_disk_instance_count: u32,
    /// Identity of the support set whose instances are uploaded, so a frame
    /// that draws the same set pays no buffer rebuild — the mesh's trick.
    supports_ptr: usize,
    model_uniform: wgpu::Buffer,
    model_bind: wgpu::BindGroup,
    gizmo_buffer: wgpu::Buffer,
    gizmo_vertex_count: u32,
    gizmo_origin: [f32; 3],
}

impl Viewport {
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
