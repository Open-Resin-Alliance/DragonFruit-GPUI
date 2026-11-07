//! `Viewport::render`: the per-frame passes and the resize path.

use anyhow::Result;
use gpui_interop::WgpuSurface;

use crate::camera::Camera;
use crate::stl::MeshData;

use wgpu::util::DeviceExt;

use super::mesh::srgb_linear;
use super::plate::BUILD_PLATE_COLOR;
use super::targets::Targets;
use super::uniforms::{ColorUniform, DecalUniform, GpuMesh, ModelUniform, SpriteUniform, Uniforms};
use super::{ViewLayers, Viewport};

impl Viewport {
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
            // The footprint outline is drawn with the scene uniform, not the
            // model's, so the hull is baked through `model` here — otherwise it
            // stays at the plate centre after the model is dragged.
            if self.mesh_ptr != ptr || self.mesh_matrix != model {
                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("viewport vertices"),
                    contents: bytemuck::cast_slice(&mesh.vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let mut outline = Vec::with_capacity(mesh.footprint.len() * 2);
                for index in 0..mesh.footprint.len() {
                    for point in [
                        mesh.footprint[index],
                        mesh.footprint[(index + 1) % mesh.footprint.len()],
                    ] {
                        let world = model.transform_point3(glam::Vec3::new(point[0], point[1], 0.0));
                        outline.push([world.x, world.y]);
                    }
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
                self.mesh_matrix = model;
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
}
