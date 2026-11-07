//! The scene's colour/MSAA/depth targets and the composite bind group.

use super::pipeline::{color_target, msaa_color_target, msaa_depth_target};

pub(super) struct Targets {
    _scene: wgpu::Texture,
    pub(super) scene_view: wgpu::TextureView,
    /// Non-sRGB view of the scene texture, the volume pass's resolve target.
    pub(super) scene_unorm_view: wgpu::TextureView,
    _msaa_color: wgpu::Texture,
    pub(super) msaa_color_view: wgpu::TextureView,
    /// Non-sRGB view of the MSAA samples, for the sRGB-blended volume box.
    pub(super) msaa_color_unorm_view: wgpu::TextureView,
    _msaa_depth: wgpu::Texture,
    pub(super) msaa_depth_view: wgpu::TextureView,
    pub(super) final_texture: wgpu::Texture,
    pub(super) final_view: wgpu::TextureView,
    pub(super) composite_bind: wgpu::BindGroup,
}

impl Targets {
    pub(super) fn new(
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
