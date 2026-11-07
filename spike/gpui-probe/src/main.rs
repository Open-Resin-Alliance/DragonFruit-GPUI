//! GPUI port probe: a window, a 2D canvas, and the GPU bridge.
//!
//! Run with `cargo run` from this directory. What it proves, in order:
//!
//! 1. GPUI builds and opens a window on this machine.
//! 2. `gpui_interop` can attach to the window's renderer inside `gpu_canvas`.
//! 3. A wgpu device can be obtained on the renderer's GPU, and a
//!    `WgpuSurface` can be rendered and composited with `paint_texture`.
//!
//! Step 3 only takes the same-device fast path where the window renderer is
//! wgpu (Linux, or Windows with `WgpuRenderer` installed). On Windows under the
//! default Direct3D 11 renderer the probe reports the renderer it found and
//! stops there; the cross-device hand-off needs a Direct3D 12 producer
//! (`SharedSurface`), which is the second probe.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    App, Bounds, Context, IntoElement, Render, Window, WindowBounds, WindowOptions, div,
    gpu_canvas, prelude::*, px, rgb, size,
};

fn main() {
    gpui::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(960.), px(640.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| Probe::new()),
        )
        .expect("a window");
        cx.activate(true);
    });
}

struct Probe {
    status: Rc<RefCell<String>>,
}

impl Probe {
    fn new() -> Self {
        Self {
            status: Rc::new(RefCell::new("not painted yet".to_owned())),
        }
    }
}

impl Render for Probe {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
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
                    .child("DragonFruit · GPUI probe"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x9aa4b2))
                    .child(text),
            )
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .bg(rgb(0x141a22))
                    .child(
                        gpu_canvas(move |gpu| {
                            let mut slot = status.borrow_mut();
                            if slot.starts_with("not painted") {
                                let report = describe(gpu);
                                eprintln!("gpui-probe: {report}");
                                *slot = report;
                            }
                        })
                        .size_full(),
                    ),
            )
    }
}

/// Report what the canvas lent us. This is the whole feasibility question in
/// one string: which renderer the window uses, and whether a wgpu device is
/// reachable on it.
fn describe(gpu: &mut gpui::GpuCanvasContext) -> String {
    let mut lines = Vec::new();

    match gpui_interop::attach(gpu.try_device::<gpui_wgpu::WgpuRenderer>()) {
        Ok(interop) => {
            lines.push("renderer: wgpu (same-device fast path available)".to_owned());
            match interop.adapter().wgpu() {
                Some((device, _queue)) => {
                    let info = device.limits();
                    lines.push(format!(
                        "wgpu device: max_texture_2d={} max_buffer={} MiB",
                        info.max_texture_dimension_2d,
                        info.max_buffer_size / (1024 * 1024)
                    ));
                }
                None => lines.push("wgpu device: none on this adapter".to_owned()),
            }
        }
        Err(unavailable) => {
            lines.push(format!("renderer: not wgpu ({unavailable:?})"));
            lines.push(
                "cross-device hand-off (Direct3D 12 -> Direct3D 11) is the Windows path; \
                 see the audit"
                    .to_owned(),
            );
        }
    }

    lines.join("\n")
}
