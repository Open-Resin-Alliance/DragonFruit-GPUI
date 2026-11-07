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
    App, Bounds, Context, Corners, FocusHandle, FontWeight, IntoElement, MouseButton, Render,
    ScrollDelta, WeakEntity, Window, WindowBounds, WindowOptions, div, gpu_canvas, prelude::*, px,
    rgb, rgba, size,
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
const SURFACE_1: u32 = 0x1a1b21;
const SURFACE_2: u32 = 0x23252e;
const ACCENT: u32 = 0xec2a77;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Prepare,
    Support,
    Export,
    Printing,
}

impl Mode {
    fn label(self) -> &'static str {
        match self {
            Mode::Prepare => "Prepare",
            Mode::Support => "Support",
            Mode::Export => "Export",
            Mode::Printing => "Printing",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            Mode::Prepare => "Arrange model and transforms",
            Mode::Support => "Build and tune supports",
            Mode::Export => "Finalize and export output",
            Mode::Printing => "Inspect sliced layers before printing",
        }
    }

    fn step(self) -> u32 {
        match self {
            Mode::Prepare => 1,
            Mode::Support => 2,
            Mode::Export => 3,
            Mode::Printing => 4,
        }
    }
}

struct DragonFruit {
    mesh: Option<Arc<stl::MeshData>>,
    camera: Rc<RefCell<Camera>>,
    viewport: Rc<RefCell<Option<Viewport>>>,
    drag: Rc<Cell<Option<MouseButton>>>,
    last: Rc<Cell<(f32, f32)>>,
    status: String,
    loading: bool,
    selected: Rc<Cell<bool>>,
    smoke: bool,
    smoke_done: Rc<Cell<bool>>,
    mode: Mode,
    show_settings: bool,
    show_models: bool,
    show_supports: bool,
    layer: f32,
    supports: Rc<RefCell<Vec<[f32; 3]>>>,
    show_grid: bool,
    show_plate: bool,
    show_box: bool,
    translate: [f32; 3],
    rotate_z: f32,
    scale: f32,
    support_angle: f32,
    focus: FocusHandle,
    focused: bool,
    show_printer_menu: bool,
    show_help: bool,
    show_support_settings: bool,
    show_app_menu: bool,
    settings_tab: usize,
    active_tool: usize,
    window_width: f32,
}

impl DragonFruit {
    fn new(cx: &mut Context<Self>) -> Self {
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
            selected: Rc::new(Cell::new(true)),
            smoke,
            smoke_done: Rc::new(Cell::new(false)),
            mode: Mode::Prepare,
            show_settings: false,
            show_models: true,
            show_supports: true,
            layer: 0.0,
            supports: Rc::new(RefCell::new(Vec::new())),
            show_grid: true,
            show_plate: true,
            show_box: true,
            translate: [0.0; 3],
            rotate_z: 0.0,
            scale: 1.0,
            support_angle: 45.0,
            focus: cx.focus_handle(),
            focused: false,
            show_printer_menu: false,
            show_help: false,
            show_support_settings: false,
            show_app_menu: false,
            settings_tab: 0,
            active_tool: 0,
            window_width: 0.0,
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
        *self.camera.borrow_mut() = Camera::fit(mesh.center, mesh.radius.max(110.0));
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mesh = self.mesh.clone();
        let click_mesh = self.mesh.clone();
        let supports = self.supports.clone();
        let model_matrix = glam::Mat4::from_scale_rotation_translation(
            glam::Vec3::splat(self.scale),
            glam::Quat::from_rotation_z(self.rotate_z),
            glam::Vec3::from_array(self.translate),
        );
        let clip_z = if self.mode == Mode::Printing {
            self.mesh
                .as_ref()
                .map(|mesh| {
                    mesh.center.z - mesh.size[2] * 0.5 + self.layer * mesh.size[2]
                })
                .unwrap_or(f32::INFINITY)
        } else {
            f32::INFINITY
        };
        let gizmo = if self.active_tool < 3 {
            self.mesh.as_ref().map(|mesh| {
                model_matrix
                    .transform_point3(mesh.center)
                    .to_array()
            })
        } else {
            None
        };
        let show_layers = viewport::ViewLayers {
            grid: self.show_grid,
            plate: self.show_plate,
            volume: self.show_box,
        };
        let camera = self.camera.clone();
        let viewport = self.viewport.clone();
        let drag = self.drag.clone();
        let last = self.last.clone();
        let smoke = self.smoke;
        let selected = self.selected.get();
        let selected_state = self.selected.clone();
        let smoke_done = self.smoke_done.clone();
        let status = self.status.clone();
        let triangles = self.mesh.as_ref().map(|mesh| mesh.triangles).unwrap_or(0);
        let loading = self.loading;
        let orthographic = self.camera.borrow().orthographic;
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
                match Viewport::new(&device, &queue) {
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
            let base_color = mesh_color(selected_state.get());
            if let Err(error) = state.render(
                &device,
                &queue,
                mesh_ref,
                &supports.borrow(),
                show_layers,
                model_matrix,
                clip_z,
                gizmo,
                &canvas_camera.borrow(),
                base_color,
                width,
                height,
            ) {
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
        .on_mouse_down(MouseButton::Right, {
            let drag = drag.clone();
            let last = last.clone();
            move |event, _, _| {
                drag.set(Some(MouseButton::Right));
                last.set((event.position.x.0, event.position.y.0));
            }
        })
        .on_mouse_down(MouseButton::Middle, {
            let drag = drag.clone();
            let last = last.clone();
            move |event, _, _| {
                drag.set(Some(MouseButton::Middle));
                last.set((event.position.x.0, event.position.y.0));
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
        .on_mouse_up(MouseButton::Middle, {
            let drag = drag.clone();
            move |_, _, _| {
                if drag.get() == Some(MouseButton::Middle) {
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
                        MouseButton::Right => camera.orbit(dx, dy),
                        MouseButton::Middle => camera.pan(dx, dy),
                        _ => {}
                    }
                    entity.update(cx, |_, cx| cx.notify()).ok();
                }
            }
        })
        .on_mouse_down(MouseButton::Left, {
            let camera = camera.clone();
            let mesh = click_mesh.clone();
            let entity = entity.clone();
            move |event, window, cx| {
                let Some(mesh) = mesh.as_ref() else {
                    return;
                };
                let size = window.viewport_size();
                let width = size.width.0.max(1.0);
                let height = size.height.0.max(1.0);
                let ndc_x = event.position.x.0 / width * 2.0 - 1.0;
                let ndc_y = 1.0 - event.position.y.0 / height * 2.0;
                let (origin, direction) = camera.borrow().ray(width / height, ndc_x, ndc_y);
                let inverse = model_matrix.inverse();
                let origin = inverse.transform_point3(origin);
                let direction = inverse.transform_vector3(direction);
                let hit = stl::raycast(mesh, origin, direction);
                entity
                    .update(cx, |this, cx| {
                        this.selected.set(hit);
                        cx.notify();
                    })
                    .ok();
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

        let mesh_name = self
            .mesh
            .as_ref()
            .map(|mesh| mesh.name.clone())
            .unwrap_or_else(|| "No model".to_owned());
        let mode = self.mode;
        let show_settings = self.show_settings;
        let layer = self.layer;
        let (size, volume) = self
            .mesh
            .as_ref()
            .map(|mesh| (mesh.size, mesh.volume))
            .unwrap_or(([0.0; 3], 0.0));

        self.window_width = window.viewport_size().width.0;
        if !self.focused {
            window.focus(&self.focus, cx);
            self.focused = true;
        }
        let key_entity = entity.clone();
        div()
            .size_full()
            .relative()
            .bg(rgb(BACKGROUND))
            .track_focus(&self.focus)
            .on_key_down(move |event, _window, cx| {
                let key = event.keystroke.key.as_str();
                let next_mode = match key {
                    "1" => Some(Mode::Prepare),
                    "2" => Some(Mode::Support),
                    "3" => Some(Mode::Export),
                    "4" => Some(Mode::Printing),
                    _ => None,
                };
                let escape = key == "escape";
                let reset = key == "r";
                let transform = matches!(
                    key,
                    "left" | "right" | "up" | "down" | "pageup" | "pagedown"
                );
                if next_mode.is_none() && !escape && !reset && !transform {
                    return;
                }
                key_entity
                    .update(cx, |this, cx| {
                        if escape {
                            this.show_settings = false;
                        }
                        if reset {
                            this.translate = [0.0; 3];
                            this.rotate_z = 0.0;
                            this.scale = 1.0;
                        }
                        if let Some(mode) = next_mode {
                            this.mode = mode;
                        }
                        if transform {
                            let step = 1.0;
                            match this.active_tool {
                                0 => match key {
                                    "left" => this.translate[0] -= step,
                                    "right" => this.translate[0] += step,
                                    "up" => this.translate[1] += step,
                                    "down" => this.translate[1] -= step,
                                    "pageup" => this.translate[2] += step,
                                    "pagedown" => this.translate[2] -= step,
                                    _ => {}
                                },
                                1 => match key {
                                    "left" => this.rotate_z -= 5f32.to_radians(),
                                    "right" => this.rotate_z += 5f32.to_radians(),
                                    _ => {}
                                },
                                _ => match key {
                                    "up" => this.scale = (this.scale * 1.05).min(10.0),
                                    "down" => this.scale = (this.scale / 1.05).max(0.1),
                                    _ => {}
                                },
                            }
                        }
                        cx.notify();
                    })
                    .ok();
            })
            .child(canvas)
            .child(left_rail(entity.clone(), self.active_tool))
            .child(left_panel(
                mesh_name.clone(),
                triangles,
                entity.clone(),
                loading,
                mode,
                self.supports.borrow().len(),
                self.show_models,
                self.show_supports,
                self.selected.get(),
            ))
            .child(orientation_cube(camera.clone(), entity.clone()))
            .child(info_card(mesh_name, triangles, size, volume))
            .child(layer_bar(layer, entity.clone(), self.window_width))
            .child(
                div()
                    .absolute()
                    .bottom(px(110.))
                    .left(px(56.))
                    .max_w(px(420.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(rgb(BORDER_SUBTLE))
                    .bg(rgb(SURFACE_0))
                    .px(px(10.))
                    .py(px(6.))
                    .text_size(px(11.))
                    .text_color(rgb(TEXT_MUTED))
                    .child(status.clone()),
            )
            .child(top_bar(
                mode,
                status.clone(),
                triangles,
                entity.clone(),
                loading,
                camera.clone(),
                orthographic,
                selected,
                show_settings,
                self.show_printer_menu,
                self.show_app_menu,
            ))
            .when(show_settings, |this| {
                this.child(settings_modal(
                    entity.clone(),
                    self.settings_tab,
                    self.show_grid,
                    self.show_plate,
                    self.show_box,
                ))
            })
            .when(self.show_help, |this| {
                this.child(help_modal(entity.clone()))
            })
            .when(self.show_support_settings, |this| {
                this.child(support_settings_modal(
                    entity.clone(),
                    self.support_angle,
                ))
            })
    }
}

fn step_button(
    mode: Mode,
    active: Mode,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let is_active = mode == active;
    div()
        .id(format!("step-{}", mode.step()))
        .flex()
        .items_center()
        .gap(px(6.))
        .h(px(32.))
        .px(px(10.))
        .rounded(px(6.))
        .bg(rgb(if is_active { SURFACE_2 } else { SURFACE_0 }))
        .border_1()
        .border_color(rgb(if is_active { ACCENT } else { BORDER_SUBTLE }))
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(if is_active { TEXT_STRONG } else { TEXT_MUTED }))
        .child(
            div()
                .w(px(16.))
                .h(px(16.))
                .rounded_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(rgb(if is_active { ACCENT } else { SURFACE_2 }))
                .text_size(px(9.))
                .text_color(rgb(ACCENT_CONTRAST))
                .child(format!("{}", mode.step())),
        )
        .child(mode.label())
        .on_click(move |_, _, cx| {
            entity
                .update(cx, |this, cx| {
                    this.mode = mode;
                    cx.notify();
                })
                .ok();
        })
}

fn top_bar(
    mode: Mode,
    _status: String,
    triangles: usize,
    entity: WeakEntity<DragonFruit>,
    loading: bool,
    camera: Rc<RefCell<Camera>>,
    orthographic: bool,
    selected: bool,
    show_settings: bool,
    show_printer_menu: bool,
    show_app_menu: bool,
) -> impl IntoElement {
    let selection_entity = entity.clone();
    let selection = div()
        .id("selection")
        .h(px(32.))
        .px(px(10.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT_STRONG))
        .child(if selected { "Selected" } else { "Unselected" })
        .on_click(move |_, _, cx| {
            selection_entity
                .update(cx, |this, cx| {
                    this.selected.set(!this.selected.get());
                    cx.notify();
                })
                .ok();
        });
    let toggle_camera = camera.clone();
    let toggle_entity = entity.clone();
    let projection = div()
        .id("projection")
        .h(px(32.))
        .px(px(10.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT_STRONG))
        .child(if orthographic { "Ortho" } else { "Persp" })
        .on_click(move |_, _, cx| {
            let next = !toggle_camera.borrow().orthographic;
            toggle_camera.borrow_mut().orthographic = next;
            toggle_entity.update(cx, |_, cx| cx.notify()).ok();
        });
    let printer_entity = entity.clone();
    let printer = div()
        .id("printer")
        .h(px(32.))
        .px(px(10.))
        .flex()
        .items_center()
        .gap(px(4.))
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(if show_printer_menu { ACCENT } else { BORDER_SUBTLE }))
        .bg(rgb(SURFACE_1))
        .text_size(px(11.))
        .text_color(rgb(TEXT_MUTED))
        .child("Printer")
        .child("\u{25BE}")
        .on_click(move |_, _, cx| {
            printer_entity
                .update(cx, |this, cx| {
                    this.show_printer_menu = !this.show_printer_menu;
                    cx.notify();
                })
                .ok();
        });
    let help_entity = entity.clone();
    let help = div()
        .id("help")
        .w(px(32.))
        .h(px(32.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .text_size(px(13.))
        .text_color(rgb(TEXT_MUTED))
        .child("?")
        .on_click(move |_, _, cx| {
            help_entity
                .update(cx, |this, cx| {
                    this.show_help = !this.show_help;
                    cx.notify();
                })
                .ok();
        });
    let settings_entity = entity.clone();
    let settings = div()
        .id("settings")
        .w(px(32.))
        .h(px(32.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(if show_settings { ACCENT } else { BORDER_SUBTLE }))
        .bg(rgb(SURFACE_1))
        .text_size(px(14.))
        .text_color(rgb(TEXT_MUTED))
        .child("\u{2699}")
        .on_click(move |_, _, cx| {
            settings_entity
                .update(cx, |this, cx| {
                    this.show_settings = !this.show_settings;
                    cx.notify();
                })
                .ok();
        });
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
        let open_entity = entity.clone();
        button = button.on_click(move |_, _, cx| {
            open_entity
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
        .child({
            let logo_entity = entity.clone();
            div()
                .id("logo-menu")
                .w(px(24.))
                .h(px(24.))
                .rounded(px(6.))
                .bg(rgb(ACCENT))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.))
                .text_color(rgb(ACCENT_CONTRAST))
                .child("D")
                .on_click(move |_, _, cx| {
                    logo_entity
                        .update(cx, |this, cx| {
                            this.show_app_menu = !this.show_app_menu;
                            cx.notify();
                        })
                        .ok();
                })
        })
        .child(
            div()
                .text_size(px(15.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child("DragonFruit"),
        )
        .child(div().w(px(8.)))
        .child(step_button(Mode::Prepare, mode, entity.clone()))
        .child(step_button(Mode::Support, mode, entity.clone()))
        .child(step_button(Mode::Export, mode, entity.clone()))
        .child(step_button(Mode::Printing, mode, entity.clone()))
        .child(div().flex_1())
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child(format!("{triangles} tris")),
        )
        .child(projection)
        .child(selection)
        .child(printer)
        .child(settings)
        .child(help)
        .child(button)
        .child(if show_app_menu {
            let import_entity = entity.clone();
            let close_entity = entity.clone();
            div()
                .absolute()
                .top(px(44.))
                .left(px(8.))
                .w(px(220.))
                .rounded(px(10.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_0))
                .p(px(6.))
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(menu_row("New scene", false, None))
                .child(menu_row("Save scene", false, None))
                .child(menu_row("Open scene…", false, None))
                .child(
                    div()
                        .id("menu-import")
                        .h(px(28.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .rounded(px(6.))
                        .text_size(px(11.))
                        .text_color(rgb(TEXT_MUTED))
                        .hover(|style| style.bg(rgb(SURFACE_1)))
                        .child("Import mesh…")
                        .on_click(move |_, _, cx| {
                            import_entity
                                .update(cx, |this, cx| {
                                    this.show_app_menu = false;
                                    this.open_stl(cx);
                                })
                                .ok();
                        }),
                )
                .child(
                    div()
                        .id("menu-close")
                        .h(px(28.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .rounded(px(6.))
                        .text_size(px(11.))
                        .text_color(rgb(0xe45454))
                        .hover(|style| style.bg(rgb(SURFACE_1)))
                        .child("Close program")
                        .on_click(move |_, _, cx| {
                            let _ = close_entity;
                            cx.quit();
                        }),
                )
        } else {
            div()
        })
        .child(if show_printer_menu {
            div()
                .absolute()
                .top(px(44.))
                .right(px(210.))
                .w(px(240.))
                .rounded(px(10.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_0))
                .p(px(10.))
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("No printer configured"),
                )
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Add a printer in Settings \u{2192} Printer."),
                )
        } else {
            div()
        })
}

fn menu_row(label: &'static str, enabled: bool, _action: Option<()>) -> impl IntoElement {
    div()
        .h(px(28.))
        .px(px(8.))
        .flex()
        .items_center()
        .rounded(px(6.))
        .text_size(px(11.))
        .text_color(rgb(if enabled { TEXT_MUTED } else { 0x5a5f6a }))
        .child(label)
}

fn left_rail(entity: WeakEntity<DragonFruit>, active_tool: usize) -> impl IntoElement {
    let tools = ["\u{2725}", "\u{27F3}", "\u{2921}", "\u{2693}", "\u{25D0}", "\u{2702}"];
    let mut rail = div()
        .absolute()
        .top(px(52.))
        .left(px(0.))
        .bottom(px(0.))
        .w(px(44.))
        .bg(rgb(SURFACE_0))
        .border_r_1()
        .border_color(rgb(BORDER_SUBTLE))
        .flex()
        .flex_col()
        .items_center()
        .pt(px(10.))
        .gap(px(6.));
    for (index, glyph) in tools.iter().enumerate() {
        let tool_entity = entity.clone();
        let is_active = index == active_tool;
        rail = rail.child(
            div()
                .id(format!("tool-{index}"))
                .w(px(32.))
                .h(px(32.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(6.))
                .bg(rgb(if is_active { SURFACE_2 } else { SURFACE_0 }))
                .text_size(px(14.))
                .text_color(rgb(if is_active { ACCENT } else { TEXT_MUTED }))
                .hover(|style| style.bg(rgb(SURFACE_1)))
                .child(*glyph)
                .on_click(move |_, _, cx| {
                    tool_entity
                        .update(cx, |this, cx| {
                            this.active_tool = index;
                            cx.notify();
                        })
                        .ok();
                }),
        );
    }
    rail
}

fn left_panel(
    mesh_name: String,
    triangles: usize,
    entity: WeakEntity<DragonFruit>,
    loading: bool,
    mode: Mode,
    support_count: usize,
    show_models: bool,
    show_supports: bool,
    selected: bool,
) -> impl IntoElement {
    let add_entity = entity.clone();
    let mut add_button = div()
        .id("add-model")
        .h(px(30.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT_STRONG))
        .child(if loading { "Loading…" } else { "Add Model" })
        .hover(|style| style.bg(rgb(SURFACE_2)));
    if !loading {
        add_button = add_button.on_click(move |_, _, cx| {
            add_entity.update(cx, |this, cx| this.open_stl(cx)).ok();
        });
    }

    let row_entity = entity.clone();
    let model_row = div()
        .id("model-row")
        .flex()
        .items_center()
        .gap(px(8.))
        .px(px(10.))
        .py(px(8.))
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(if selected { ACCENT } else { SURFACE_1 }))
        .bg(rgb(SURFACE_1))
        .on_click(move |_, _, cx| {
            row_entity
                .update(cx, |this, cx| {
                    this.selected.set(!this.selected.get());
                    cx.notify();
                })
                .ok();
        })
        .child(
            div()
                .w(px(18.))
                .h(px(18.))
                .rounded(px(4.))
                .bg(rgb(ACCENT))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(9.))
                .text_color(rgb(ACCENT_CONTRAST))
                .child("3D"),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child(mesh_name),
                )
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(TEXT_MUTED))
                        .child(format!("{triangles} triangles")),
                ),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child("\u{25C9}"),
        );

    div()
        .absolute()
        .top(px(52.))
        .left(px(44.))
        .bottom(px(0.))
        .w(px(300.))
        .bg(rgb(BACKGROUND))
        .border_r_1()
        .border_color(rgb(BORDER_SUBTLE))
        .flex()
        .flex_col()
        .gap(px(8.))
        .p(px(10.))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .rounded(px(10.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_0))
                .p(px(10.))
                .child({
                    let header_entity = entity.clone();
                    div()
                        .id("models-header")
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child("Models"),
                        )
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(rgb(TEXT_MUTED))
                                .child(if triangles > 0 { "1" } else { "0" }),
                        )
                        .on_click(move |_, _, cx| {
                            header_entity
                                .update(cx, |this, cx| {
                                    this.show_models = !this.show_models;
                                    cx.notify();
                                })
                                .ok();
                        })
                })
                .when(show_models, |this| this.child(model_row).child(add_button)),
        )
        .child(match mode {
            Mode::Prepare | Mode::Support => {
                support_card(entity.clone(), support_count, show_supports)
            }
            Mode::Export => export_card(entity.clone()),
            Mode::Printing => printing_card(),
        })
}

fn card_shell(title: &'static str) -> gpui::Div {
    div()
        .relative()
        .flex()
        .flex_col()
        .gap(px(8.))
        .rounded(px(10.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .p(px(10.))
        .child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(TEXT_STRONG))
                .child(title),
        )
}

fn support_card(
    entity: WeakEntity<DragonFruit>,
    support_count: usize,
    visible: bool,
) -> gpui::Div {
    let gear_entity = entity.clone();
    let mut card = card_shell("Auto Supports (Beta)");
    let header_entity = entity.clone();
    card = card.child(
        div()
            .id("supports-header")
            .absolute()
            .top(px(0.))
            .left(px(0.))
            .right(px(0.))
            .h(px(20.))
            .on_click(move |_, _, cx| {
                header_entity
                    .update(cx, |this, cx| {
                        this.show_supports = !this.show_supports;
                        cx.notify();
                    })
                    .ok();
            }),
    );
    card = card.child(
        div()
            .id("support-settings")
            .absolute()
            .top(px(6.))
            .right(px(6.))
            .w(px(20.))
            .h(px(20.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(4.))
            .text_size(px(11.))
            .text_color(rgb(TEXT_MUTED))
            .child("\u{2699}")
            .on_click(move |_, _, cx| {
                gear_entity
                    .update(cx, |this, cx| {
                        this.show_support_settings = true;
                        cx.notify();
                    })
                    .ok();
            }),
    );
    card.when(visible, |this| {
        this.child(
            div()
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(island_row("Voxel", 0))
                .child(island_row("Minima", 0))
                .child(island_row("Total", support_count)),
        )
        .child(
            div()
                .text_size(px(10.))
                .text_color(rgb(TEXT_MUTED))
                .child("Load a model and scan for islands."),
        )
        .child({
            let generate_entity = entity.clone();
            div()
                .id("generate-supports")
                .h(px(30.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(6.))
                .bg(rgb(PRIMARY))
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(ACCENT_CONTRAST))
                .child("Generate Supports")
                .hover(|style| style.bg(rgb(PRIMARY_HOVER)))
                .on_click(move |_, _, cx| {
                    generate_entity
                        .update(cx, |this, cx| {
                            if let Some(mesh) = this.mesh.as_ref() {
                                let model = glam::Mat4::from_scale_rotation_translation(
                                    glam::Vec3::splat(this.scale),
                                    glam::Quat::from_rotation_z(this.rotate_z),
                                    glam::Vec3::from_array(this.translate),
                                );
                                let points = stl::overhang_points(mesh, this.support_angle);
                                let stride = (points.len() / 300).max(1);
                                *this.supports.borrow_mut() = points
                                    .into_iter()
                                    .step_by(stride)
                                    .map(|p| {
                                        model
                                            .transform_point3(glam::Vec3::from_array(p))
                                            .to_array()
                                    })
                                    .collect();
                            }
                            cx.notify();
                        })
                        .ok();
                })
        })
    })
}

fn export_card(entity: WeakEntity<DragonFruit>) -> gpui::Div {
    let export_entity = entity.clone();
    card_shell("Export")
        .child(modal_row("Format", "STL"))
        .child(modal_row("Units", "Millimetres"))
        .child(
            div()
                .id("export")
                .h(px(30.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(6.))
                .bg(rgb(PRIMARY))
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(ACCENT_CONTRAST))
                .child("Export")
                .hover(|style| style.bg(rgb(PRIMARY_HOVER)))
                .on_click(move |_, _, cx| {
                    export_entity
                        .update(cx, |this, cx| {
                            let Some(mesh) = this.mesh.as_ref() else {
                                return;
                            };
                            let Some(path) = rfd::FileDialog::new()
                                .add_filter("STL", &["stl"])
                                .set_file_name("export.stl")
                                .save_file()
                            else {
                                return;
                            };
                            let transform = glam::Mat4::from_scale_rotation_translation(
                                glam::Vec3::splat(this.scale),
                                glam::Quat::from_rotation_z(this.rotate_z),
                                glam::Vec3::from_array(this.translate),
                            );
                            this.status = match stl::export_stl(mesh, transform, &path) {
                                Ok(()) => format!("Exported {}", path.display()),
                                Err(error) => format!("{error:#}"),
                            };
                            cx.notify();
                        })
                        .ok();
                }),
        )
}

fn printing_card() -> gpui::Div {
    card_shell("Printing")
        .child(modal_row("Layers", "0"))
        .child(modal_row("Layer height", "0.05 mm"))
        .child(
            div()
                .text_size(px(10.))
                .text_color(rgb(TEXT_MUTED))
                .child("Slice a model to inspect layers."),
        )
}

fn island_row(label: &'static str, count: usize) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .text_size(px(11.))
        .text_color(rgb(TEXT_MUTED))
        .child(label)
        .child(format!("{count}"))
}

fn orientation_cube(
    camera: Rc<RefCell<Camera>>,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let face = |label: &'static str,
                yaw: f32,
                pitch: f32,
                entity: WeakEntity<DragonFruit>,
                camera: Rc<RefCell<Camera>>| {
        div()
            .id(format!("face-{label}"))
            .w(px(44.))
            .h(px(30.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(4.))
            .bg(rgb(SURFACE_1))
            .text_size(px(9.))
            .text_color(rgb(TEXT_MUTED))
            .child(label)
            .hover(|style| style.bg(rgb(SURFACE_2)))
            .on_click(move |_, _, cx| {
                let mut camera = camera.borrow_mut();
                camera.yaw = yaw;
                camera.pitch = pitch;
                drop(camera);
                entity.update(cx, |_, cx| cx.notify()).ok();
            })
    };
    div()
        .absolute()
        .top(px(64.))
        .right(px(12.))
        .w(px(104.))
        .rounded(px(10.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .p(px(6.))
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(
            div()
                .flex()
                .justify_between()
                .child(face("Top", -1.5708, 1.55, entity.clone(), camera.clone()))
                .child(face("Back", 1.5708, 0.0, entity.clone(), camera.clone())),
        )
        .child(
            div()
                .flex()
                .justify_between()
                .child(face("Left", 3.1416, 0.0, entity.clone(), camera.clone()))
                .child(face("Right", 0.0, 0.0, entity.clone(), camera.clone())),
        )
        .child(
            div()
                .flex()
                .justify_between()
                .child(face("Bottom", -1.5708, -1.55, entity.clone(), camera.clone()))
                .child(face("Front", -1.5708, 0.0, entity.clone(), camera.clone())),
        )
}

fn info_card(mesh_name: String, triangles: usize, size: [f32; 3], volume: f32) -> impl IntoElement {
    let dimensions = format!("{:.1} × {:.1} × {:.1} mm", size[0], size[1], size[2]);
    let volume = format!("{:.1} cm³", volume / 1000.0);
    div()
        .absolute()
        .bottom(px(64.))
        .right(px(12.))
        .w(px(220.))
        .rounded(px(10.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .p(px(10.))
        .flex()
        .flex_col()
        .gap(px(6.))
        .child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(TEXT_STRONG))
                .child(mesh_name),
        )
        .child(info_row("Dimensions", &dimensions))
        .child(info_row("Volume", &volume))
        .child(info_row("Triangles", &format!("{triangles}")))
}

fn info_row(label: &'static str, value: &str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .text_size(px(11.))
        .text_color(rgb(TEXT_MUTED))
        .child(label)
        .child(value.to_owned())
}

fn layer_bar(layer: f32, entity: WeakEntity<DragonFruit>, window_width: f32) -> impl IntoElement {
    let minus_entity = entity.clone();
    let plus_entity = entity.clone();
    let track_entity = entity.clone();
    let track_left = (window_width - 420.0) * 0.5 + 48.0;
    let track = div()
        .w(px(220.))
        .h(px(6.))
        .rounded_full()
        .bg(rgb(SURFACE_2))
        .child(
            div()
                .h(px(6.))
                .w(px(220. * layer))
                .rounded_full()
                .bg(rgb(ACCENT)),
        )
        .on_mouse_down(MouseButton::Left, move |event, _, cx| {
            let value = ((event.position.x.0 - track_left) / 220.0).clamp(0.0, 1.0);
            track_entity
                .update(cx, |this, cx| {
                    this.layer = value;
                    cx.notify();
                })
                .ok();
        });
    div()
        .absolute()
        .bottom(px(64.))
        .left(px(0.))
        .right(px(0.))
        .flex()
        .justify_center()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.))
                .h(px(40.))
                .px(px(14.))
                .rounded(px(10.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_0))
                .child(
                    div()
                        .id("layer-minus")
                        .w(px(24.))
                        .h(px(24.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.))
                        .bg(rgb(SURFACE_1))
                        .text_size(px(12.))
                        .text_color(rgb(TEXT_STRONG))
                        .child("\u{2212}")
                        .on_click(move |_, _, cx| {
                            minus_entity
                                .update(cx, |this, cx| {
                                    this.layer = (this.layer - 0.05).max(0.0);
                                    cx.notify();
                                })
                                .ok();
                        }),
                )
                .child(track)
                .child(
                    div()
                        .id("layer-plus")
                        .w(px(24.))
                        .h(px(24.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.))
                        .bg(rgb(SURFACE_1))
                        .text_size(px(12.))
                        .text_color(rgb(TEXT_STRONG))
                        .child("+")
                        .on_click(move |_, _, cx| {
                            plus_entity
                                .update(cx, |this, cx| {
                                    this.layer = (this.layer + 0.05).min(1.0);
                                    cx.notify();
                                })
                                .ok();
                        }),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(TEXT_MUTED))
                        .child(format!("Layer {} / 100", (layer * 100.0).round() as u32)),
                ),
        )
}

fn settings_modal(
    entity: WeakEntity<DragonFruit>,
    tab: usize,
    grid: bool,
    plate: bool,
    volume: bool,
) -> impl IntoElement {
    let close_entity = entity.clone();
    div()
        .absolute()
        .top(px(0.))
        .left(px(0.))
        .size_full()
        .bg(rgba(0x000000b3))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .w(px(520.))
                .rounded(px(12.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_0))
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(16.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child("Settings"),
                        )
                        .child(
                            div()
                                .id("settings-close")
                                .w(px(26.))
                                .h(px(26.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(6.))
                                .bg(rgb(SURFACE_1))
                                .text_size(px(13.))
                                .text_color(rgb(TEXT_MUTED))
                                .child("\u{00D7}")
                                .on_click(move |_, _, cx| {
                                    close_entity
                                        .update(cx, |this, cx| {
                                            this.show_settings = false;
                                            cx.notify();
                                        })
                                        .ok();
                                }),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(6.))
                        .child(modal_tab("Appearance", tab == 0, entity.clone(), 0))
                        .child(modal_tab("Mesh", tab == 1, entity.clone(), 1))
                        .child(modal_tab("Performance", tab == 2, entity.clone(), 2)),
                )
                .child(match tab {
                    1 => div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(modal_row("Selected colour", "#ec2a77"))
                        .child(modal_row("Unselected colour", "#a3a3a3"))
                        .child(modal_row("Roughness", "0.45")),
                    2 => div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(modal_row("MSAA", "4×"))
                        .child(modal_row("AO rays", "32"))
                        .child(modal_row("Refine threshold", "2% of diagonal")),
                    _ => div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(modal_row("Theme", "Dark"))
                        .child(modal_row("Accent", "#ec2a77"))
                        .child(toggle_row("Grid", grid, entity.clone(), 0))
                        .child(toggle_row("Build plate", plate, entity.clone(), 1))
                        .child(toggle_row("Build volume", volume, entity.clone(), 2)),
                }),
        )
}

fn support_settings_modal(
    entity: WeakEntity<DragonFruit>,
    angle: f32,
) -> impl IntoElement {
    let close_entity = entity.clone();
    let minus_entity = entity.clone();
    let plus_entity = entity.clone();
    div()
        .absolute()
        .top(px(0.))
        .left(px(0.))
        .size_full()
        .bg(rgba(0x000000b3))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .w(px(460.))
                .rounded(px(12.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_0))
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(15.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child("Auto Supports (Beta) Settings"),
                        )
                        .child(
                            div()
                                .id("support-settings-close")
                                .w(px(26.))
                                .h(px(26.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(6.))
                                .bg(rgb(SURFACE_1))
                                .text_size(px(13.))
                                .text_color(rgb(TEXT_MUTED))
                                .child("\u{00D7}")
                                .on_click(move |_, _, cx| {
                                    close_entity
                                        .update(cx, |this, cx| {
                                            this.show_support_settings = false;
                                            cx.notify();
                                        })
                                        .ok();
                                }),
                        ),
                )
                .child(modal_row("Density", "0.50"))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .h(px(32.))
                        .px(px(10.))
                        .rounded(px(6.))
                        .bg(rgb(SURFACE_1))
                        .text_size(px(11.))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Overhang angle")
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .id("angle-minus")
                                        .w(px(22.))
                                        .h(px(22.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(4.))
                                        .bg(rgb(SURFACE_2))
                                        .child("\u{2212}")
                                        .on_click(move |_, _, cx| {
                                            minus_entity
                                                .update(cx, |this, cx| {
                                                    this.support_angle =
                                                        (this.support_angle - 5.0).max(5.0);
                                                    cx.notify();
                                                })
                                                .ok();
                                        }),
                                )
                                .child(format!("{angle:.0}°"))
                                .child(
                                    div()
                                        .id("angle-plus")
                                        .w(px(22.))
                                        .h(px(22.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(4.))
                                        .bg(rgb(SURFACE_2))
                                        .child("+")
                                        .on_click(move |_, _, cx| {
                                            plus_entity
                                                .update(cx, |this, cx| {
                                                    this.support_angle =
                                                        (this.support_angle + 5.0).min(85.0);
                                                    cx.notify();
                                                })
                                                .ok();
                                        }),
                                ),
                        ),
                )
                .child(modal_row("Preset", "Default"))
                .child(modal_row("Stability", "Balanced")),
        )
}

fn help_modal(entity: WeakEntity<DragonFruit>) -> impl IntoElement {
    let close_entity = entity.clone();
    div()
        .absolute()
        .top(px(0.))
        .left(px(0.))
        .size_full()
        .bg(rgba(0x000000b3))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .w(px(420.))
                .rounded(px(12.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_0))
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(16.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child("Keyboard & mouse"),
                        )
                        .child(
                            div()
                                .id("help-close")
                                .w(px(26.))
                                .h(px(26.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(6.))
                                .bg(rgb(SURFACE_1))
                                .text_size(px(13.))
                                .text_color(rgb(TEXT_MUTED))
                                .child("\u{00D7}")
                                .on_click(move |_, _, cx| {
                                    close_entity
                                        .update(cx, |this, cx| {
                                            this.show_help = false;
                                            cx.notify();
                                        })
                                        .ok();
                                }),
                        ),
                )
                .child(modal_row("1 – 4", "Switch workflow step"))
                .child(modal_row("Esc", "Close dialogs"))
                .child(modal_row("Right drag", "Orbit"))
                .child(modal_row("Middle drag", "Pan"))
                .child(modal_row("Wheel", "Zoom")),
        )
}

fn modal_tab(
    label: &'static str,
    active: bool,
    entity: WeakEntity<DragonFruit>,
    index: usize,
) -> impl IntoElement {
    div()
        .id(format!("settings-tab-{index}"))
        .h(px(28.))
        .px(px(10.))
        .flex()
        .items_center()
        .rounded(px(6.))
        .bg(rgb(if active { SURFACE_2 } else { SURFACE_1 }))
        .text_size(px(11.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(if active { TEXT_STRONG } else { TEXT_MUTED }))
        .child(label)
        .on_click(move |_, _, cx| {
            entity
                .update(cx, |this, cx| {
                    this.settings_tab = index;
                    cx.notify();
                })
                .ok();
        })
}

fn toggle_row(
    label: &'static str,
    value: bool,
    entity: WeakEntity<DragonFruit>,
    kind: usize,
) -> impl IntoElement {
    div()
        .id(format!("toggle-{kind}"))
        .flex()
        .items_center()
        .justify_between()
        .h(px(32.))
        .px(px(10.))
        .rounded(px(6.))
        .bg(rgb(SURFACE_1))
        .text_size(px(11.))
        .text_color(rgb(TEXT_MUTED))
        .child(label)
        .child(
            div()
                .w(px(34.))
                .h(px(18.))
                .rounded_full()
                .bg(rgb(if value { ACCENT } else { SURFACE_2 }))
                .flex()
                .items_center()
                .justify_end()
                .pr(px(2.))
                .child(
                    div()
                        .w(px(14.))
                        .h(px(14.))
                        .rounded_full()
                        .bg(rgb(ACCENT_CONTRAST)),
                ),
        )
        .on_click(move |_, _, cx| {
            entity
                .update(cx, |this, cx| {
                    match kind {
                        0 => this.show_grid = !this.show_grid,
                        1 => this.show_plate = !this.show_plate,
                        _ => this.show_box = !this.show_box,
                    }
                    cx.notify();
                })
                .ok();
        })
}

fn modal_row(label: &'static str, value: &'static str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .h(px(32.))
        .px(px(10.))
        .rounded(px(6.))
        .bg(rgb(SURFACE_1))
        .text_size(px(11.))
        .text_color(rgb(TEXT_MUTED))
        .child(label)
        .child(value)
}

fn srgb_to_linear(channel: u8) -> f32 {
    let value = channel as f32 / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

/// `blendTintColor` from `src/features/shaders/mesh/tint.ts`: linear lerp from
/// the unselected `#a3a3a3` toward the theme selection `#ec2a77` at 0.70.
fn mesh_color(selected: bool) -> [f32; 3] {
    // Calibrated against the real app's selected poussin render: the model's
    // material is a saturated pink, not the #a3a3a3 default blended with the
    // selection tint (whose red cannot reach this value).
    if selected {
        [0.78, 0.10, 0.22]
    } else {
        [srgb_to_linear(0xa3), srgb_to_linear(0xa3), srgb_to_linear(0xa3)]
    }
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
