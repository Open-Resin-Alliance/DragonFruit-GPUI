//! DragonFruit's GPUI frontend, first slice: a 3D viewport with STL loading.

mod camera;
mod stl;
mod viewport;
mod window;

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use camera::Camera;
use gpui::{
    App, Bounds, Context, Corners, FontWeight, IntoElement, MouseButton, Render, ScrollDelta,
    WeakEntity, Window, WindowBounds, WindowOptions, div, gpu_canvas, prelude::*, px, rgb, size,
};
use gpui_interop::attach;
use gpui_wgpu::{ImportedTextureExt as _, WgpuRenderer};
use viewport::Viewport;

// DragonFruit dark-theme tokens.
const BACKGROUND: u32 = 0x0b0f14;
const SURFACE_0: u32 = 0x111216;
const BORDER_SUBTLE: u32 = 0x272a33;
const TEXT_STRONG: u32 = 0xf8f8fb;
const TEXT_MUTED: u32 = 0xc3c7cf;
const PRIMARY: u32 = 0xc11f61;
const PRIMARY_HOVER: u32 = 0xd81d67;
const ACCENT_CONTRAST: u32 = 0xfff6ff;

struct DragonFruit {
    mesh: Option<Arc<stl::MeshData>>,
    camera: Rc<RefCell<Camera>>,
    viewport: Rc<RefCell<Option<Viewport>>>,
    drag: Rc<Cell<Option<MouseButton>>>,
    last: Rc<Cell<(f32, f32)>>,
    status: String,
    loading: bool,
    smoke: bool,
    smoke_done: Rc<Cell<bool>>,
}

impl DragonFruit {
    fn new(_cx: &mut Context<Self>) -> Self {
        let smoke = std::env::args().any(|arg| arg == "--smoke");
        let path = std::env::args()
            .skip(1)
            .find(|arg| !arg.starts_with("--"))
            .map(PathBuf::from);

        let mut app = Self {
            mesh: None,
            camera: Rc::new(RefCell::new(Camera::fit(glam::Vec3::ZERO, 60.0))),
            viewport: Rc::new(RefCell::new(None)),
            drag: Rc::new(Cell::new(None)),
            last: Rc::new(Cell::new((0.0, 0.0))),
            status: "No model loaded".to_owned(),
            loading: false,
            smoke,
            smoke_done: Rc::new(Cell::new(false)),
        };
        if let Some(path) = path {
            match stl::load_stl(&path) {
                Ok(mesh) => app.set_mesh(mesh),
                Err(error) => app.status = format!("{error:#}"),
            }
        }
        app
    }

    fn set_mesh(&mut self, mesh: stl::MeshData) {
        let mesh = Arc::new(mesh);
        *self.camera.borrow_mut() = Camera::fit(mesh.center, mesh.radius);
        self.status = format!("{} · {} triangles", mesh.name, mesh.triangles);
        eprintln!("loaded {}", self.status);
        self.mesh = Some(mesh);
    }

    fn open_stl(&mut self, cx: &mut Context<Self>) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("STL", &["stl"])
            .pick_file()
        else {
            return;
        };
        self.loading = true;
        self.status = format!("Loading {}…", path.display());
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { stl::load_stl(&path) })
                .await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(mesh) => this.set_mesh(mesh),
                    Err(error) => this.status = format!("{error:#}"),
                }
                this.loading = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

impl Render for DragonFruit {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mesh = self.mesh.clone();
        let camera = self.camera.clone();
        let viewport = self.viewport.clone();
        let drag = self.drag.clone();
        let last = self.last.clone();
        let smoke = self.smoke;
        let smoke_done = self.smoke_done.clone();
        let status = self.status.clone();
        let triangles = self.mesh.as_ref().map(|mesh| mesh.triangles).unwrap_or(0);
        let loading = self.loading;
        let entity = cx.entity().downgrade();

        let canvas_camera = camera.clone();
        let canvas = gpu_canvas(move |gpu| {
            let interop = match attach(gpu.try_device::<WgpuRenderer>()) {
                Ok(interop) => interop,
                Err(error) => {
                    eprintln!("viewport: not wgpu: {error:?}");
                    return;
                }
            };
            let Some((device, queue)) = interop.adapter().wgpu() else {
                eprintln!("viewport: no wgpu device");
                return;
            };
            let mut slot = viewport.borrow_mut();
            if slot.is_none() {
                match Viewport::new(&device) {
                    Ok(built) => *slot = Some(built),
                    Err(error) => {
                        eprintln!("viewport: {error:#}");
                        return;
                    }
                }
            }
            let state = slot.as_mut().expect("built above");
            let bounds = gpu.bounds();
            let scale = gpu.scale_factor();
            let width = (bounds.size.width.0 * scale).round().max(1.0) as u32;
            let height = (bounds.size.height.0 * scale).round().max(1.0) as u32;
            let mesh_ref = mesh.as_deref();
            if let Err(error) =
                state.render(&device, &queue, mesh_ref, &canvas_camera.borrow(), width, height)
            {
                eprintln!("viewport: {error:#}");
                return;
            }
            match state.surface().view().to_imported_handle() {
                Ok(handle) => gpu.paint_texture(handle, Corners::default(), 1.0, false),
                Err(error) => {
                    eprintln!("viewport: import: {error:#}");
                    return;
                }
            }
            if smoke && mesh_ref.is_some() && !smoke_done.get() {
                smoke_done.set(true);
                match state.readback(&device, &queue) {
                    Ok((painted, total)) => {
                        eprintln!("smoke: painted {painted} / {total} pixels");
                        std::thread::spawn(|| {
                            std::thread::sleep(std::time::Duration::from_millis(300));
                            std::process::exit(0);
                        });
                    }
                    Err(error) => eprintln!("smoke: readback: {error:#}"),
                }
            }
        })
        .size_full()
        .on_mouse_down(MouseButton::Left, {
            let drag = drag.clone();
            let last = last.clone();
            move |event, _, _| {
                drag.set(Some(MouseButton::Left));
                last.set((event.position.x.0, event.position.y.0));
            }
        })
        .on_mouse_down(MouseButton::Right, {
            let drag = drag.clone();
            let last = last.clone();
            move |event, _, _| {
                drag.set(Some(MouseButton::Right));
                last.set((event.position.x.0, event.position.y.0));
            }
        })
        .on_mouse_up(MouseButton::Left, {
            let drag = drag.clone();
            move |_, _, _| {
                if drag.get() == Some(MouseButton::Left) {
                    drag.set(None);
                }
            }
        })
        .on_mouse_up(MouseButton::Right, {
            let drag = drag.clone();
            move |_, _, _| {
                if drag.get() == Some(MouseButton::Right) {
                    drag.set(None);
                }
            }
        })
        .on_mouse_move({
            let drag = drag.clone();
            let last = last.clone();
            let camera = camera.clone();
            let entity = entity.clone();
            move |event, _window, cx| {
                if let Some(button) = drag.get() {
                    let (lx, ly) = last.get();
                    let dx = event.position.x.0 - lx;
                    let dy = event.position.y.0 - ly;
                    last.set((event.position.x.0, event.position.y.0));
                    let mut camera = camera.borrow_mut();
                    match button {
                        MouseButton::Left => camera.orbit(dx, dy),
                        MouseButton::Right => camera.pan(dx, dy),
                        _ => {}
                    }
                    entity.update(cx, |_, cx| cx.notify()).ok();
                }
            }
        })
        .on_scroll_wheel({
            let camera = camera.clone();
            let entity = entity.clone();
            move |event, _window, cx| {
                let delta = match event.delta {
                    ScrollDelta::Lines(point) => point.y,
                    ScrollDelta::Pixels(point) => point.y.0 / 40.0,
                };
                camera.borrow_mut().zoom(delta);
                entity.update(cx, |_, cx| cx.notify()).ok();
            }
        });

        div()
            .size_full()
            .relative()
            .bg(rgb(BACKGROUND))
            .child(canvas)
            .child(top_bar(status, triangles, entity, loading))
            .child(
                div()
                    .absolute()
                    .bottom(px(12.))
                    .left(px(12.))
                    .text_size(px(11.))
                    .text_color(rgb(TEXT_MUTED))
                    .child("Left drag orbit · Right drag pan · Wheel zoom"),
            )
    }
}

fn top_bar(
    status: String,
    triangles: usize,
    entity: WeakEntity<DragonFruit>,
    loading: bool,
) -> impl IntoElement {
    let mut button = div()
        .id("open-stl")
        .h(px(32.))
        .px(px(12.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .bg(rgb(if loading { 0x8a1745 } else { PRIMARY }))
        .text_size(px(13.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(ACCENT_CONTRAST))
        .child(if loading { "Loading…" } else { "Open STL" })
        .hover(|style| style.bg(rgb(PRIMARY_HOVER)));
    if !loading {
        button = button.on_click(move |_, _, cx| {
            entity
                .update(cx, |this, cx| this.open_stl(cx))
                .ok();
        });
    }

    div()
        .absolute()
        .top(px(0.))
        .left(px(0.))
        .right(px(0.))
        .h(px(52.))
        .bg(rgb(SURFACE_0))
        .border_b_1()
        .border_color(rgb(BORDER_SUBTLE))
        .px(px(14.))
        .flex()
        .items_center()
        .gap(px(12.))
        .child(
            div()
                .text_size(px(15.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child("DragonFruit"),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child(status),
        )
        .child(div().flex_1())
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child(format!("{triangles} tris")),
        )
        .child(button)
}

fn main() {
    gpui::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1280.), px(800.)), cx);
        let options = window::with_wgpu_renderer(WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        });
        cx.open_window(options, |_, cx| cx.new(DragonFruit::new))
            .expect("a window");
        cx.activate(true);
    });
}
