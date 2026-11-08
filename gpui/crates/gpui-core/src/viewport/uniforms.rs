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
    /// The model's own albedo. Per model, not per scene: the selection and
    /// hover tints are per model (`StlMesh`'s tint ignores every mesh but the
    /// active one), so the colour cannot live in the shared scene uniform.
    pub(super) color: [f32; 4],
}

/// One support instance: the unit mesh's transform plus its taper ratio.
///
/// The source keeps the taper in the tapered batch's mesh and buckets shafts by
/// it (`taperBuckets` in `InstancedShaftGroup.tsx`); the port carries it on the
/// instance so every tapered shaft — whatever its ratio — is one draw. The
/// straight, node and disk batches write `1.0`.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct SupportInstance {
    pub(super) matrix: [[f32; 4]; 4],
    pub(super) taper: f32,
}

pub(super) struct GpuMesh {
    /// `wgpu`'s `max_buffer_size` is 256 MiB by default and the device is
    /// GPUI's, so a large model is split into chunks instead of failing
    /// validation on one buffer.
    pub(super) vertex_chunks: Vec<(wgpu::Buffer, u32)>,
    /// The footprint outline, already baked into world space, and its vertex
    /// count. `None` while no outline has been built yet (and for a mesh whose
    /// convex hull is degenerate).
    pub(super) footprint: Option<(wgpu::Buffer, u32)>,
    /// The `Arc<MeshData>` pointer the vertices came from, so a replaced
    /// geometry re-uploads while a shared one does not.
    pub(super) mesh_ptr: usize,
    /// The transform the footprint hull was baked through.
    pub(super) matrix: glam::Mat4,
    /// The model's uniform and bind group. One pair per model, so a drag on
    /// one model touches only that model's uniform.
    pub(super) uniform: wgpu::Buffer,
    pub(super) bind: wgpu::BindGroup,
}
