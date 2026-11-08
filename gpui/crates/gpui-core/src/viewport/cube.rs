//! The view cube (`ZUpGizmoViewcube`): its shader, uniforms, target textures and
//! render pass. It draws into its own texture so the overlay can import it.

use anyhow::Result;

use crate::camera::Camera;

use super::Viewport;

/// The gizmo widget is 120pt; the target is oversampled so the 1px atlas
/// stroke survives the downscale to the widget.
/// What the pointer is over on the gizmo. Faces tint the atlas; an edge or a
/// corner draws its own box, like `EdgeCube`'s `visible={hover}`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CubeHover {
    Face(usize),
    Edge(usize),
    Corner(usize),
}

pub(super) const CUBE_TARGET_SIZE: u32 = 240;
/// MSAA for the cube: the face labels and the 1px borders alias badly at this
/// size without it.
pub(super) const CUBE_SAMPLE_COUNT: u32 = 4;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct CubeUniform {
    view_proj: [[f32; 4]; 4],
    tint: [f32; 4],
}

/// The orientation cube: one textured quad per face, sampled from the baked
/// atlas (`cube::atlas`). The cube is convex, so back-face culling replaces a
/// depth buffer.
pub(super) const CUBE_SHADER: &str = r#"
struct Uniform { view_proj: mat4x4<f32>, tint: vec4<f32> };
@group(0) @binding(0) var<uniform> u: Uniform;
@group(1) @binding(0) var face_atlas: texture_2d<f32>;
@group(1) @binding(1) var face_sampler: sampler;

struct VsIn { @location(0) pos: vec3<f32>, @location(1) uv: vec2<f32> };
struct VsOut { @builtin(position) clip: vec4<f32>, @location(0) uv: vec2<f32> };

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    out.clip = u.view_proj * vec4<f32>(in.pos, 1.0);
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let texel = textureSample(face_atlas, face_sampler, in.uv);
    // Opaque: the source's `opacity={0.75}` bleeds 25% of the scene through the
    // faces, which reads as washed out against GPUI's lighter gizmo backdrop.
    return vec4<f32>(texel.rgb * u.tint.rgb, texel.a);
}
"#;

impl Viewport {
    /// Non-transparent pixel count in the cube target — the same observation the
    /// smoke test makes of the viewport.
    pub fn cube_readback(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<(usize, usize)> {
        let size = CUBE_TARGET_SIZE;
        let padded = (size * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cube readback"),
            size: (padded * size) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("cube readback"),
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.cube_target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(size),
                },
            },
            wgpu::Extent3d {
                width: size,
                height: size,
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
        for row in 0..size as usize {
            let start = row * padded as usize;
            for column in 0..size as usize {
                if data[start + column * 4 + 3] > 0 {
                    painted += 1;
                }
            }
        }
        drop(data);
        buffer.unmap();
        Ok((painted, (size * size) as usize))
    }

    /// The cube target, for the overlay's imported texture.
    pub fn cube_texture_view(&self) -> &wgpu::TextureView {
        &self.cube_target_view
    }

    /// `ZUpGizmoViewcube`'s render: the scene camera's direction, an orthographic
    /// cube at the origin, and the hovered face tinted with `gizmoColors.accent`.
    pub fn render_cube(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera: &Camera,
        hovered: Option<CubeHover>,
        accent: [f32; 3],
    ) {
        let direction = (camera.eye() - camera.target).normalize_or_zero();
        let direction = if direction.length_squared() < 0.5 {
            glam::Vec3::new(0.0, -1.0, 0.0)
        } else {
            direction
        };
        let eye = direction * 3.2;
        let view = glam::Mat4::look_at_rh(eye, glam::Vec3::ZERO, glam::Vec3::Z);
        // The gizmo camera is orthographic: the cube must not foreshorten.
        // A half-extent of 0.97 puts the rotated cube at ~73% of the widget —
        // the footprint `scale=[60,60,60]` has — while leaving the corner-on
        // silhouette (0.87 against 0.97) inside the target.
        let half = 0.97;
        let proj = glam::Mat4::orthographic_rh(-half, half, -half, half, 0.1, 10.0);
        let view_proj = proj * view;
        for (index, tint) in [[1.0f32, 1.0, 1.0, 1.0], [accent[0], accent[1], accent[2], 1.0]]
            .into_iter()
            .enumerate()
        {
            let uniform = CubeUniform {
                view_proj: view_proj.to_cols_array_2d(),
                tint,
            };
            queue.write_buffer(
                &self.cube_uniforms[index],
                0,
                bytemuck::cast_slice(&[uniform]),
            );
        }

        let mut encoder = device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("viewport cube"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport cube"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.cube_msaa_view,
                    resolve_target: Some(&self.cube_target_view),
                    depth_slice: None,
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
            pass.set_pipeline(&self.cube_pipeline);
            pass.set_vertex_buffer(0, self.cube_vertex_buffer.slice(..));
            pass.set_bind_group(1, &self.cube_texture_bind, &[]);
            pass.set_bind_group(0, &self.cube_binds[0], &[]);
            pass.draw(0..36, 0..1);
            if let Some(index) = hovered.and_then(|hover| match hover {
                CubeHover::Face(index) => Some(index),
                _ => None,
            }) {
                pass.set_bind_group(0, &self.cube_binds[1], &[]);
                pass.draw(index as u32 * 6..index as u32 * 6 + 6, 0..1);
            }
            // `EdgeCube`: one 36-vertex box per edge, then per corner.
            let highlight = match hovered {
                Some(CubeHover::Edge(index)) if index < crate::cube::EDGES.len() => {
                    Some(index as u32 * 36)
                }
                Some(CubeHover::Corner(index)) if index < crate::cube::CORNERS.len() => {
                    Some((crate::cube::EDGES.len() as u32 + index as u32) * 36)
                }
                _ => None,
            };
            if let Some(start) = highlight {
                pass.set_vertex_buffer(0, self.cube_highlight_buffer.slice(..));
                pass.set_bind_group(1, &self.cube_texture_bind, &[]);
                pass.set_bind_group(0, &self.cube_binds[0], &[]);
                pass.draw(start..start + 36, 0..1);
            }
        }
        queue.submit(Some(encoder.finish()));
    }
}
