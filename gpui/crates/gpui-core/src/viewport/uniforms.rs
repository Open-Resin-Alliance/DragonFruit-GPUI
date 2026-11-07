//! The `#[repr(C)]` uniform blocks shared with the WGSL shaders, and the GPU
//! mesh handle.

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Uniforms {
    pub(super) view_proj: [[f32; 4]; 4],
    pub(super) inv_proj: [[f32; 4]; 4],
    pub(super) camera_pos: [f32; 3],
    pub(super) _pad0: f32,
    pub(super) base_color: [f32; 3],
    pub(super) _pad1: f32,
    pub(super) resolution: [f32; 2],
    pub(super) _pad2: [f32; 2],
    /// `ViewHeadlight`'s direction: the camera's world forward, negated. The
    /// source light's target sits 100mm down that ray, so its direction is
    /// constant for the frame rather than per-fragment.
    pub(super) headlight_dir: [f32; 3],
    pub(super) _pad3: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct SpriteUniform {
    pub(super) right: [f32; 3],
    pub(super) size: f32,
    pub(super) up: [f32; 3],
    pub(super) _pad: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct DecalUniform {
    /// `1` when the FRONT marker has turned half a revolution.
    pub(super) flip: f32,
    /// `dimFactor` for the marker, `0.4 * dimFactor` for the logo.
    pub(super) opacity: f32,
    pub(super) _pad: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct ColorUniform {
    pub(super) color: [f32; 3],
    pub(super) opacity: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct ModelUniform {
    pub(super) matrix: [[f32; 4]; 4],
    pub(super) clip: [f32; 4],
    /// Build volume in world mm; `stripe.w` is the enabled flag.
    pub(super) bounds_min: [f32; 4],
    pub(super) bounds_max: [f32; 4],
    /// `(r, g, b)` stripe colour, `w` enabled.
    pub(super) stripe: [f32; 4],
}

pub(super) struct GpuMesh {
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) vertex_count: u32,
    pub(super) footprint_buffer: wgpu::Buffer,
    pub(super) footprint_vertex_count: u32,
}
