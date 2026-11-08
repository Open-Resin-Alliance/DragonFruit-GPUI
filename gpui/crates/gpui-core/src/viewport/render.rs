//! `Viewport::render`: the per-frame passes and the resize path.

use anyhow::Result;
use gpui_interop::WgpuSurface;
use gpui_supports::SupportBatches;

use crate::camera::Camera;
use crate::stl::MeshData;

use wgpu::util::DeviceExt;

use super::mesh::srgb_linear;
use super::plate::BUILD_PLATE_COLOR;
use super::targets::Targets;
use super::uniforms::{
    ColorUniform, DecalUniform, GpuMesh, ModelUniform, SpriteUniform, SupportInstance, Uniforms,
};
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
        supports: Option<&SupportBatches>,
        layers: ViewLayers,
        model: glam::Mat4,
        clip_z: f32,
        gizmo: Option<&[[f32; 7]]>,
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
            //
            // The geometry buffer is *not* part of that: the vertices never
            // change with the transform, and re-uploading them on every frame of
            // a drag is what used to make moving a high-poly model crawl. Only
            // the footprint — a few hundred points — follows the matrix.
            let mesh_changed = self.mesh_ptr != ptr;
            let matrix_changed = self.mesh_matrix != model;
            if mesh_changed || matrix_changed {
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
                if mesh_changed {
                    let vertex_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("viewport vertices"),
                            contents: bytemuck::cast_slice(&mesh.vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        });
                    let footprint_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("viewport footprint"),
                            contents: bytemuck::cast_slice(&outline),
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        });
                    self.mesh = Some(GpuMesh {
                        vertex_buffer,
                        vertex_count: mesh.vertices.len() as u32,
                        footprint_buffer,
                        footprint_vertex_count: outline.len() as u32,
                    });
                    self.mesh_ptr = ptr;
                } else if let Some(gpu) = &self.mesh {
                    // Same mesh, same outline length: the hull is a copy, not a
                    // new buffer.
                    queue.write_buffer(
                        &gpu.footprint_buffer,
                        0,
                        bytemuck::cast_slice(&outline),
                    );
                }
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
                bounds_min: [
                    self.metrics.center_x - self.metrics.plate_width * 0.5,
                    self.metrics.center_y - self.metrics.plate_depth * 0.5,
                    0.0,
                    0.0,
                ],
                bounds_max: [
                    self.metrics.center_x + self.metrics.plate_width * 0.5,
                    self.metrics.center_y + self.metrics.plate_depth * 0.5,
                    self.metrics.volume_height,
                    0.0,
                ],
                // The stripe is on only when the model actually escapes the
                // volume; the AABB test is on the CPU so a fitting model pays
                // nothing in the shader.
                stripe: {
                    let outside = mesh.is_some_and(|mesh| {
                        let centre = model.transform_point3(mesh.center);
                        let half = glam::Vec3::from_array(mesh.size) * 0.5;
                        let min = centre - half;
                        let max = centre + half;
                        min.x < self.metrics.center_x - self.metrics.plate_width * 0.5
                            || max.x > self.metrics.center_x + self.metrics.plate_width * 0.5
                            || min.y < self.metrics.center_y - self.metrics.plate_depth * 0.5
                            || max.y > self.metrics.center_y + self.metrics.plate_depth * 0.5
                            || min.z < 0.0
                            || max.z > self.metrics.volume_height
                    });
                    let color = super::plate::out_of_bounds_stripe_color();
                    [color[0], color[1], color[2], if outside { 1.0 } else { 0.0 }]
                },
            }),
        );
        // The caller builds the gizmo's triangles from the camera and the handle
        // state, so the buffer is rebuilt only when they actually changed. The
        // caller memoises the build itself; the compare here is the last line of
        // defence against an upload every frame.
        let gizmo_vertices = gizmo.unwrap_or_default();
        if gizmo_vertices != self.gizmo_vertices.as_slice() {
            self.gizmo_vertex_count = gizmo_vertices.len() as u32;
            if !gizmo_vertices.is_empty() {
                let bytes: &[u8] = bytemuck::cast_slice(gizmo_vertices);
                if self.gizmo_buffer.size() < bytes.len() as u64 {
                    // Grow in powers of two: the handle set changes size when
                    // hover reveals an outline or a halo, and the buffer must
                    // not be reallocated per frame while a drag moves the gizmo.
                    let capacity = (bytes.len() as u64).next_power_of_two();
                    self.gizmo_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("viewport gizmo"),
                        size: capacity.max(24),
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                }
                queue.write_buffer(&self.gizmo_buffer, 0, bytes);
            }
            self.gizmo_vertices.clear();
            self.gizmo_vertices.extend_from_slice(gizmo_vertices);
        }
        // Supports upload only when the set itself changes; a frame that draws
        // the same `SupportBatches` reuses every buffer, exactly as the mesh
        // does. The caller owns the stable set (a fresh value per frame would
        // rebuild here, so `app.rs` must hand over a held one).
        let supports_ptr = supports.map_or(0, |batches| batches as *const SupportBatches as usize);
        if supports_ptr != self.supports_ptr {
            self.supports_ptr = supports_ptr;
            self.upload_supports(device, supports);
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
            // One draw per non-empty batch, each `SupportBatches::draw_calls()`
            // counts. The bind group is shared: the support colour is constant.
            if self.support_straight_count > 0
                || self.support_tapered_count > 0
                || self.support_node_count > 0
                || self.support_disk_instance_count > 0
            {
                pass.set_pipeline(&self.support);
                pass.set_bind_group(1, &self.support_bind, &[]);
                if let Some(instances) = &self.support_straight_instances {
                    pass.set_vertex_buffer(0, self.support_cylinder.slice(..));
                    pass.set_vertex_buffer(1, instances.slice(..));
                    pass.draw(0..self.support_cylinder_count, 0..self.support_straight_count);
                }
                if let Some(instances) = &self.support_tapered_instances {
                    pass.set_vertex_buffer(0, self.support_cylinder.slice(..));
                    pass.set_vertex_buffer(1, instances.slice(..));
                    pass.draw(0..self.support_cylinder_count, 0..self.support_tapered_count);
                }
                if let Some(instances) = &self.support_node_instances {
                    pass.set_vertex_buffer(0, self.support_sphere.slice(..));
                    pass.set_vertex_buffer(1, instances.slice(..));
                    pass.draw(0..self.support_sphere_count, 0..self.support_node_count);
                }
                if let Some(instances) = &self.support_disk_instances {
                    pass.set_vertex_buffer(0, self.support_disk.slice(..));
                    pass.set_vertex_buffer(1, instances.slice(..));
                    pass.draw(0..self.support_disk_count, 0..self.support_disk_instance_count);
                }
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
            // The gizmo last, so its depth-test-always handles land on top of
            // the model rather than being punched through by it. The source
            // gives its handles `renderOrder: 1_000_000` and `depthTest: false`
            // for the same reason.
            if self.gizmo_vertex_count > 0 {
                pass.set_pipeline(&self.gizmo);
                pass.set_vertex_buffer(0, self.gizmo_buffer.slice(..));
                pass.draw(0..self.gizmo_vertex_count, 0..1);
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

    /// Upload one instance buffer per non-empty batch.
    ///
    /// Called from [`Viewport::render`] only when the set's identity changes,
    /// so the per-frame cost is the pointer comparison, not this walk.
    fn upload_supports(&mut self, device: &wgpu::Device, supports: Option<&SupportBatches>) {
        let Some(batches) = supports else {
            set_support_instances(
                &mut self.support_straight_instances,
                &mut self.support_straight_count,
                None,
            );
            set_support_instances(
                &mut self.support_tapered_instances,
                &mut self.support_tapered_count,
                None,
            );
            set_support_instances(
                &mut self.support_node_instances,
                &mut self.support_node_count,
                None,
            );
            set_support_instances(
                &mut self.support_disk_instances,
                &mut self.support_disk_instance_count,
                None,
            );
            return;
        };

        let straight: Vec<SupportInstance> = batches
            .straight_shafts
            .iter()
            .map(|shaft| SupportInstance {
                matrix: shaft.matrix().to_cols_array_2d(),
                taper: 1.0,
            })
            .collect();
        // The tapered batch carries its ratio per instance; the shader narrows
        // the unit cylinder's `to` end.
        let tapered: Vec<SupportInstance> = batches
            .tapered_shafts
            .iter()
            .map(|shaft| SupportInstance {
                matrix: shaft.matrix().to_cols_array_2d(),
                taper: shaft.taper(),
            })
            .collect();
        let nodes: Vec<SupportInstance> = batches
            .nodes
            .iter()
            .map(|node| SupportInstance {
                matrix: node.matrix().to_cols_array_2d(),
                taper: 1.0,
            })
            .collect();
        let disks: Vec<SupportInstance> = batches
            .disks
            .iter()
            .map(|disk| SupportInstance {
                matrix: disk.matrix().to_cols_array_2d(),
                taper: 1.0,
            })
            .collect();

        set_support_instances(
            &mut self.support_straight_instances,
            &mut self.support_straight_count,
            support_instance_buffer(device, "viewport support straight", &straight),
        );
        set_support_instances(
            &mut self.support_tapered_instances,
            &mut self.support_tapered_count,
            support_instance_buffer(device, "viewport support tapered", &tapered),
        );
        set_support_instances(
            &mut self.support_node_instances,
            &mut self.support_node_count,
            support_instance_buffer(device, "viewport support nodes", &nodes),
        );
        set_support_instances(
            &mut self.support_disk_instances,
            &mut self.support_disk_instance_count,
            support_instance_buffer(device, "viewport support disks", &disks),
        );
    }
}

fn support_instance_buffer(
    device: &wgpu::Device,
    label: &str,
    instances: &[SupportInstance],
) -> Option<(wgpu::Buffer, u32)> {
    if instances.is_empty() {
        return None;
    }
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::cast_slice(instances),
        usage: wgpu::BufferUsages::VERTEX,
    });
    Some((buffer, instances.len() as u32))
}

fn set_support_instances(
    slot: &mut Option<wgpu::Buffer>,
    count: &mut u32,
    buffer: Option<(wgpu::Buffer, u32)>,
) {
    match buffer {
        Some((buffer, instances)) => {
            *slot = Some(buffer);
            *count = instances;
        }
        None => {
            *slot = None;
            *count = 0;
        }
    }
}
