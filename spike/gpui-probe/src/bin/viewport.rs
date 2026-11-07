//! Zero-copy 3D viewport probe.
//!
//! Runs a real `wgpu` render pass into a `WgpuSurface` on the window renderer's
//! own device, then composites it with `paint_texture` — no CPU copy.
//!
//! Run with `cargo run --bin viewport`.
//!
//! This installs `WgpuRenderer` as the window renderer through
//! `WindowOptions::with_renderer_factory`. That is the same-device fast path:
//! Linux uses it by default, Windows needs the factory, and macOS cannot use it
//! at all yet (`gpui_wgpu`'s `platform_renderer` module is `cfg`-excluded there
//! because the wgpu renderer cannot present through a `CAMetalLayer`).

use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;
use gpui::{
    App, Bounds, Context, Corners, DevicePixels, IntoElement, Render, Window, WindowBounds,
    WindowOptions, div, gpu_canvas, prelude::*, px, rgb, size,
};
use gpui_interop::{WgpuSurface, attach};
use gpui_wgpu::{ImportedTextureExt as _, WgpuContextSlot, WgpuRenderer, WgpuSurfaceConfig};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WindowHandle,
};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;

fn main() {
    gpui::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(960.), px(640.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        }
        .with_renderer_factory(gpui::FnRendererFactory(
            |target: gpui::RendererTarget<'_>| -> Result<Box<dyn gpui::PlatformRenderer>> {
                let handles = Handles {
                    window: target.window_handle,
                    display: target.display_handle,
                };
                let config = WgpuSurfaceConfig {
                    size: size(
                        DevicePixels((target.size.width.0 * target.scale_factor).round() as i32),
                        DevicePixels((target.size.height.0 * target.scale_factor).round() as i32),
                    ),
                    transparent: target.transparent,
                    preferred_present_mode: None,
                };
                let slot: WgpuContextSlot = Rc::new(RefCell::new(None));
                let renderer =
                    WgpuRenderer::new(slot, &handles, config, None, gpui::GpuPreference::Auto)?;
                Ok(Box::new(renderer))
            },
        ));

        cx.open_window(options, |_, cx| cx.new(|_| ViewportProbe::new()))
            .expect("a window");
        cx.activate(true);
    });
}

/// The raw handles a renderer factory is handed, in the shape
/// `WgpuRenderer::new` wants. The window outlives the renderer, so borrowing
/// them is sound for the renderer's lifetime.
#[derive(Clone, Copy, Debug)]
struct Handles {
    window: Option<RawWindowHandle>,
    display: Option<RawDisplayHandle>,
}

// Safety: the handles name process-global native objects that outlive the
// renderer, the same guarantee `gpui_wgpu`'s own `RawWindowHandles` makes.
unsafe impl Send for Handles {}
unsafe impl Sync for Handles {}

impl HasWindowHandle for Handles {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        match self.window {
            // Safety: the backend keeps the native window alive while the
            // renderer exists (the contract `RendererFactory` documents).
            Some(raw) => Ok(unsafe { WindowHandle::borrow_raw(raw) }),
            None => Err(HandleError::Unavailable),
        }
    }
}

impl HasDisplayHandle for Handles {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        match self.display {
            Some(raw) => Ok(unsafe { DisplayHandle::borrow_raw(raw) }),
            None => Err(HandleError::Unavailable),
        }
    }
}

struct ViewportProbe {
    viewport: Rc<RefCell<Option<Viewport>>>,
    status: Rc<RefCell<String>>,
}

impl ViewportProbe {
    fn new() -> Self {
        Self {
            viewport: Rc::new(RefCell::new(None)),
            status: Rc::new(RefCell::new("waiting for the first paint".to_owned())),
        }
    }
}

impl Render for ViewportProbe {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let viewport = self.viewport.clone();
        let status = self.status.clone();
        let text = self.status.borrow().clone();

        div()
            .size_full()
            .bg(rgb(0x0b0f14))
            .flex()
            .flex_col()
            .gap_4()
            .p_8()
            .child(
                div()
                    .text_2xl()
                    .text_color(rgb(0xffffff))
                    .child("DragonFruit · zero-copy viewport probe"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x9aa4b2))
                    .child(text),
            )
            .child(
                div()
                    .w(px(WIDTH as f32))
                    .h(px(HEIGHT as f32))
                    .child(
                        gpu_canvas(move |gpu| {
                            let mut slot = viewport.borrow_mut();
                            let interop = match attach(gpu.try_device::<WgpuRenderer>()) {
                                Ok(interop) => interop,
                                Err(error) => {
                                    let report = format!("not wgpu: {error:?}");
                                    eprintln!("viewport: {report}");
                                    *status.borrow_mut() = report;
                                    return;
                                }
                            };
                            let Some((device, queue)) = interop.adapter().wgpu() else {
                                let report = "no wgpu device on this adapter".to_owned();
                                eprintln!("viewport: {report}");
                                *status.borrow_mut() = report;
                                return;
                            };
                            if slot.is_none() {
                                match Viewport::new(&device) {
                                    Ok(viewport) => *slot = Some(viewport),
                                    Err(error) => {
                                        let report = format!("viewport: {error:#}");
                                        eprintln!("viewport: {report}");
                                        *status.borrow_mut() = report;
                                        return;
                                    }
                                }
                            }
                            let viewport = slot.as_mut().expect("built above");
                            if let Err(error) = viewport.render(&device, &queue) {
                                let report = format!("render: {error:#}");
                                eprintln!("viewport: {report}");
                                *status.borrow_mut() = report;
                                return;
                            }
                            match viewport.surface.view().to_imported_handle() {
                                Ok(handle) => {
                                    gpu.paint_texture(handle, Corners::default(), 1.0, false);
                                    let report = format!(
                                        "wgpu same-device path · {} frames · {}×{}",
                                        viewport.frames, WIDTH, HEIGHT
                                    );
                                    eprintln!("viewport: {report}");
                                    *status.borrow_mut() = report;
                                }
                                Err(error) => {
                                    let report = format!("import: {error:#}");
                                    eprintln!("viewport: {report}");
                                    *status.borrow_mut() = report;
                                }
                            }
                        })
                        .size_full(),
                    ),
            )
    }
}

struct Viewport {
    surface: WgpuSurface,
    pipeline: wgpu::RenderPipeline,
    frames: u64,
}

impl Viewport {
    fn new(device: &wgpu::Device) -> Result<Self> {
        let surface = WgpuSurface::new(device, WIDTH, HEIGHT)?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("probe triangle"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("probe triangle"),
            layout: None,
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
                    format: WgpuSurface::DEFAULT_FORMAT,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        Ok(Self {
            surface,
            pipeline,
            frames: 0,
        })
    }

    fn render(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<()> {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("probe frame"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("probe frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.surface.view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.02,
                            g: 0.03,
                            b: 0.05,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.draw(0..3, 0..1);
        }
        queue.submit(Some(encoder.finish()));
        self.frames += 1;
        Ok(())
    }
}

const SHADER: &str = r#"
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-0.6, -0.5),
        vec2<f32>(0.6, -0.5),
        vec2<f32>(0.0, 0.6),
    );
    return vec4<f32>(positions[index], 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.20, 0.80, 0.50, 1.0);
}
"#;
