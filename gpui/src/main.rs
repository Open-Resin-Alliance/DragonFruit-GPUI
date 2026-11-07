//! DragonFruit's GPUI frontend: the editor shell around the 3D viewport.
//!
//! # PORT DEVIATIONS
//!
//! Diffed against `src/app/page.tsx`, `src/components/layout/TopBar.tsx`,
//! `src/components/layout/FloatingPanelStack.tsx`, `src/hotkeys/hotkeyConfig.ts`
//! and the hotkey hooks. Everything below is a gap the shell still carries.
//!
//! **Not ported (components the shell used to invent or omit)**
//! * `EmptySceneState` — with no model the source shows the empty-scene panel
//!   (load/import buttons, recents, onboarding). The shell renders the bare
//!   viewport instead; the only way to load a model is the app menu's
//!   `Import mesh…`, which the source also offers.
//! * `ModelStatsCard` — the bottom-left stats card (dimensions, volume, print
//!   time, resin) is not ported. The old bottom-left status text was invented
//!   and is gone; the source has no status line.
//! * `PrintingPreviewPane` — printing mode is a two-column split in the source
//!   (scene left, layer preview right). The shell keeps the full-width scene.
//! * `NotificationStack` / `SystemNotificationStack` — only the scene-import
//!   error toast is reproduced (see below); save, history, export, orientation
//!   and system toasts have no trigger in this build.
//! * `PrintingModals`, `SceneFileModals`, `SceneMaterialReassignModal`,
//!   `ModifierModals`, `ManifoldWarningModal`, `TetrahedralRemeshConfirmModal`,
//!   `ShrinkWrapConfirmModal`, `MeshRepairModals`, the two `StructuredDialogModal`s
//!   (printer/scene conflict, plate trash), `DiagnosticsModals`,
//!   `FirstRunOnboarding`, `EditorContextMenu`, `OrganicCutOverlay`,
//!   `RenderProfilerOverlay` — none are ported.
//! * The Auto Supports settings dialog (`AutoSupportSettingsBody`, a 1120px
//!   full-screen dialog) is not ported. The small modal that used to live here
//!   was invented and has been removed; `panel_content`'s gear still sets
//!   `show_support_settings`, which nothing renders.
//! * Prepare's tool panels (`TransformControls`, `AutoRotationPanel`,
//!   `MeshSmoothingSettingsPanel`, `OrganicCutPanel`, `ArrangePanel`,
//!   `DuplicatePanel`) and Support's `IslandsPanel`, `SupportSidebar`,
//!   `HollowingPanel`, `HolePunchPanel` are not mounted. `transform_panel.rs`
//!   exists but its callbacks take no `cx`, so the shell cannot apply them.
//! * Export's `SlicingPanel` is not mounted (no slicer).
//!
//! **Panel set**
//! * Models is shown only in Prepare/Support, matching `PreparePanelStack` /
//!   Support's stack; Export and Printing have no Models panel.
//! * The panel id is `models` in both modes. The source uses
//!   `prepare-models` / `support-models` with independent placement, but the
//!   rail's Models toggle (`rail.rs`) is hard-wired to the `models` id here.
//! * Support's right-hand panel follows the rail tool: only `Automatic`
//!   (index 2) has a ported body (`Auto Supports (Beta)`). The other tools
//!   leave the dock empty because their panels are not ported.
//! * `panel_stack` derives the rail / layer-slider insets from the panel set
//!   (`PanelStackInsets::for_panels`). The source also zeroes the left inset
//!   when the rail is horizontal; the rail's layout lives in `rail.rs` and is
//!   not readable here, so the column width is always assumed.
//!
//! **Hotkeys** (`DEFAULT_KEYBINDINGS`, edge-triggered like `useActionActive`)
//! * Bound: `CAMERA.TOGGLE_PROJECTION` (o), `CAMERA.FOCUS_PICK` (f),
//!   `CANVAS.TOOL_SELECT` (q), `TOOL_MODIFY` / `TOOL_MODIFY_LOCAL` (m /
//!   shift+m), `TOOL_SMOOTH` (s), `TOOL_ARRANGE` (a), `TOOL_DUPLICATE` (d) —
//!   the canvas tools only in Prepare with a model, as `usePrepareTransformHotkeys`
//!   gates them — `GLOBAL.DELETE` (Backspace, plus the fixed Delete key),
//!   `NAVIGATION.LAYER_UP` / `LAYER_DOWN` (Up/Down, and W/S in printing).
//! * Escape closes the menus and the settings dialog (`useEscapeToClose`).
//! * Not bound because their features are absent: `SUPPORTS.*` hold actions,
//!   `PRESETS.SLOT_1..6` (no camera presets), `GLOBAL.UNDO/REDO/SAVE/SAVE_AS`
//!   (no history or scene files), `DEBUG.*`, `HOLE_PUNCH.*`, `CUT.*`,
//!   `CAMERA.INTERIOR_VIEW` (the flag lives in `rail.rs`),
//!   `CANVAS.SELECT_ALL/COPY/PASTE`, `TOOL_MODIFY_LOCAL`'s local-space flag
//!   (no field), and `TOOL_ARRANGE`'s "arrange all when already active".
//! * The previous 1–4 mode switch and `R` reset were invented; the registry
//!   has no mode-switch binding (1–6 are camera presets), so mode changes are
//!   click-only.
//! * `FOCUS_PICK` has no hovered-point raycast, so it frames the whole model
//!   (`Camera::fit`'s distance) rather than the point under the cursor.
//! * Hotkeys are suppressed while the settings dialog is open. The source
//!   suppresses only when the event target is editable; GPUI exposes no
//!   "focused input" query, so the whole dialog is treated as owning the keys.
//!
//! **Drag and drop**
//! * External file drops are wired for Prepare only, matching
//!   `handlePrepareDrop`. Supported names are `.stl`, `.obj`, `.3mf`, `.voxl`,
//!   `.lys` (`PREPARE_DROP_EXTENSIONS`); the overlay strings and colours come
//!   from `page.tsx`.
//! * Only STL can actually load (`stl.rs`). OBJ/3MF importers are not ported,
//!   so those drops are accepted by the overlay and then do nothing.
//! * Scene files (`.voxl`, `.lys`) have no importer, so they raise the
//!   source's `Unsupported scene file: {name}` toast
//!   (`reportUnsupportedSceneFile`).
//! * The source tracks drag depth and hides the overlay when the pointer leaves
//!   `#scene-root`; GPUI dispatches `on_drag_move` for the whole window, so the
//!   overlay follows the canvas bounds only.
//! * `backdrop-filter: blur(1px)` has no GPUI equivalent.
//!
//! **Layers / printing**
//! * `layer` is a layer index in `0..=numLayers`, with `numLayers =
//!   ceil(bboxHeight / 0.05mm)` — `useSlicingManager`'s uncontrolled 50µm
//!   default (`layerHeightMicron = 50`). The slider is `min 0, max numLayers,
//!   step 1`, as `CrossSliderPanel` passes it.
//! * The cross-section clip is the source's upper plane
//!   (`zMin + layer*layerHeight + 1e-6`, none at layer 0 or at the top layer)
//!   and is gated by `cross_section`, in every mode, as `sceneClipStore` does.
//!   The lower plane has no viewport parameter (`viewport.rs` takes one
//!   `clip_z`), so the lower thumb is display-only.
//! * The layer slider is hidden in Printing, as `page.tsx` hides
//!   `CrossSliderPanel` there; the source's printing scrubber lives in the
//!   unported `PrintingPreviewPane`.
//! * Printing is locked by `app_bar` because `has_printing_data` is `false`
//!   (no slicing pipeline), exactly as a fresh source session locks it; the
//!   printing panel and clip branch are therefore unreachable until a slicer
//!   exists.
//! * `hasSlicedPlate` / `isActiveSliceStale` guards from `handleModeChange`
//!   cannot be evaluated; the shell enforces only the "no models → Prepare"
//!   guard.
//!
//! **Text / chrome**
//! * Geist is installed per-user by `fonts::ensure_geist` (the old
//!   `FR_PRIVATE` registration was not guaranteed to reach DirectWrite), and
//!   text renders with `TextRenderingMode::Subpixel`; the window background is
//!   opaque.
//! * CSS transitions, keyframes and backdrop blur have no GPUI equivalent.

mod app_bar;
mod arrange_panels;
mod auto_support_settings;
mod empty_scene;
mod notifications;
mod organic_cut_panel;
mod overlays;
mod panel_content;
mod panels;
mod plate_widgets;
mod prepare_stack;
mod printing_preview;
mod rail;
mod scene_modals;
mod settings;
mod smoothing_panel;
mod support_panels;
mod tool_panels;
mod transform_panel;

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use camera::Camera;
use gpui::{
    AnyElement, App, Bounds, Context, Corners, ExternalPaths, FocusHandle, FontWeight,
    IntoElement, MouseButton, Render, ScrollDelta, TextRenderingMode, TitlebarOptions, Window,
    WindowBounds, WindowOptions, div, gpu_canvas, prelude::*, px, relative, rgb, rgba, size,
};
use gpui_interop::attach;
use gpui_wgpu::{ImportedTextureExt as _, WgpuRenderer};
use viewport::Viewport;

use gpui_core::{camera, fonts, icons, log, stl, viewport, window};
use gpui_ui::{panel_fields, ui};
use gpui_core::theme::*;

/// `useSlicingManager`'s uncontrolled default: 50 microns.
const LAYER_HEIGHT_MM: f32 = 0.05;
/// The scene clip's epsilon from `publishClip`.
const CLIP_EPSILON_MM: f32 = 1e-6;
/// `PREPARE_DROP_EXTENSIONS` in `fileHandling.ts` (built-in plugin types).
const PREPARE_DROP_EXTENSIONS: [&str; 5] = ["stl", "obj", "3mf", "voxl", "lys"];
/// `sceneFileExtensionLabelsValue` for the built-in scene types.
const SCENE_FILE_LABELS: &str = "VOXL, LYS";
/// `ToastViewport offset="1.25rem"`.
const TOAST_OFFSET: f32 = 20.0;
/// The source's default toast duration (`emitSceneImportReport`).
const TOAST_DURATION_MS: u64 = 4200;

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

    fn step(self) -> u32 {
        match self {
            Mode::Prepare => 1,
            Mode::Support => 2,
            Mode::Export => 3,
            Mode::Printing => 4,
        }
    }
}

/// One entry of the editor's notification stack. The source keeps a toast per
/// kind; this build only raises the scene-import report (and export, once
/// `panel_content` calls [`DragonFruit::show_toast`]).
#[derive(Clone)]
pub struct Toast {
    pub text: String,
    pub tone: ui::ToastTone,
    id: u64,
}

struct DragonFruit {
    mesh: Option<Arc<stl::MeshData>>,
    camera: Rc<RefCell<Camera>>,
    viewport: Rc<RefCell<Option<Viewport>>>,
    drag: Rc<Cell<Option<MouseButton>>>,
    last: Rc<Cell<(f32, f32)>>,
    /// Written by `app_bar` (New scene) and `panel_content` (export). The
    /// source has no status line, so the shell does not render it.
    #[allow(dead_code)]
    status: String,
    loading: bool,
    selected: Rc<Cell<bool>>,
    /// `hoveredMeshModelId` — the pointer is over the model.
    hovered: Rc<Cell<bool>>,
    smoke: bool,
    smoke_done: Rc<Cell<bool>>,
    mode: Mode,
    show_settings: bool,
    layout: gpui_core::panel_layout::PanelLayout,
    empty_scene: gpui_core::empty_state::EmptySceneState,
    stats: gpui_core::empty_state::ModelStatsCardState,
    islands: gpui_core::support_state::IslandsPanelState,
    island_list: gpui_core::support_state::IslandListState,
    hollowing: gpui_core::support_state::HollowingState,
    hole_punch: gpui_core::support_state::HolePunchState,
    support_settings: gpui_core::support_state::SupportSettingsState,
    printing_preview: gpui_core::state::PrintingPreviewState,
    /// Upper layer index, `0..=num_layers`.
    layer: f32,
    /// Lower layer index (the second thumb).
    lower_layer: f32,
    /// The cross-section switch (`crossSectionEnabled`).
    cross_section: bool,
    num_layers: u32,
    supports: Rc<RefCell<Vec<[f32; 3]>>>,
    show_grid: bool,
    show_plate: bool,
    show_box: bool,
    overhang_view: bool,
    logo: Option<PathBuf>,
    translate: [f32; 3],
    rotate_z: f32,
    scale: f32,
    support_angle: f32,
    support_density: f32,
    focus: FocusHandle,
    focused: bool,
    show_printer_menu: bool,
    /// Set by `panel_content`'s Auto Supports gear; the real dialog is not
    /// ported, so nothing renders it.
    #[allow(dead_code)]
    show_support_settings: bool,
    show_app_menu: bool,
    /// `TopBar`'s view-mode dropdown (`SelectDropdown` under the trigger).
    show_view_menu: bool,
    /// `viewType` — the persisted mesh shader the dropdown shows.
    view_type: gpui_core::state::MeshShaderType,
    /// Opened by the top bar's monitor button; the printing monitor modal is
    /// not ported.
    #[allow(dead_code)]
    show_monitor: bool,
    /// The fleet unit picked in the printer menu; the monitor modal is not
    /// ported.
    #[allow(dead_code)]
    select_fleet_unit: Option<String>,
    settings_tab: usize,
    active_tool: usize,
    window_width: f32,
    /// An OS file drag is over the canvas (`isPrepareDragActive`).
    drag_active: bool,
    /// The dragged paths contain no supported extension
    /// (`isPrepareDragUnsupported`).
    drag_unsupported: bool,
    toast: Option<Toast>,
    toast_seq: u64,
    /// The window scale factor is logged on the first frame.
    scale_logged: bool,
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
            camera: Rc::new(RefCell::new(Camera::parked(glam::Vec3::new(0.0, 0.0, 110.0)))),
            viewport: Rc::new(RefCell::new(None)),
            drag: Rc::new(Cell::new(None)),
            last: Rc::new(Cell::new((0.0, 0.0))),
            status: String::new(),
            loading: false,
            selected: Rc::new(Cell::new(true)),
            hovered: Rc::new(Cell::new(false)),
            smoke,
            smoke_done: Rc::new(Cell::new(false)),
            mode: Mode::Prepare,
            show_settings: false,
            layout: gpui_core::panel_layout::PanelLayout::new(),
            empty_scene: gpui_core::empty_state::EmptySceneState::default(),
            stats: gpui_core::empty_state::ModelStatsCardState::default(),
            islands: gpui_core::support_state::IslandsPanelState::new(),
            island_list: gpui_core::support_state::IslandListState::new(),
            hollowing: gpui_core::support_state::HollowingState::new(),
            hole_punch: gpui_core::support_state::HolePunchState::new(),
            support_settings: gpui_core::support_state::SupportSettingsState::new(),
            printing_preview: gpui_core::state::PrintingPreviewState::default(),
            layer: 0.0,
            lower_layer: 0.0,
            cross_section: true,
            num_layers: 0,
            supports: Rc::new(RefCell::new(Vec::new())),
            show_grid: true,
            show_plate: true,
            show_box: true,
            overhang_view: false,
            logo: rasterize_logo(),
            translate: [0.0; 3],
            rotate_z: 0.0,
            scale: 1.0,
            support_angle: 45.0,
            support_density: 0.5,
            focus: cx.focus_handle(),
            focused: false,
            show_printer_menu: false,
            show_support_settings: false,
            show_app_menu: false,
            show_view_menu: false,
            view_type: gpui_core::state::MeshShaderType::SoftClay,
            show_monitor: false,
            select_fleet_unit: None,
            settings_tab: 0,
            active_tool: 0,
            window_width: 0.0,
            drag_active: false,
            drag_unsupported: false,
            toast: None,
            toast_seq: 0,
            scale_logged: false,
        };
        if let Some(path) = path {
            match stl::load_stl(&path) {
                Ok(mesh) => app.set_mesh(mesh),
                // The source logs a failed mesh load to the console only.
                Err(error) => eprintln!("{error:#}"),
            }
        }
        app
    }

    fn set_mesh(&mut self, mesh: stl::MeshData) {
        let mesh = Arc::new(mesh);
        // The intro animates from wherever the camera is (the parked home pose
        // in a fresh scene) to the model's framing.
        self.camera
            .borrow_mut()
            .focus(mesh.center, mesh.radius.max(1.0));
        // `useSlicingManager`: `numLayers = ceil(heightMm / layerHeightMm)`.
        self.num_layers = (mesh.size[2] / LAYER_HEIGHT_MM).ceil().max(0.0) as u32;
        // The source opens with the upper thumb at the top and the lower at the
        // bottom, so the whole model is visible.
        self.layer = self.num_layers as f32;
        self.lower_layer = 0.0;
        self.selected.set(true);
        self.mesh = Some(mesh);
    }

    /// Load a mesh file off the UI thread. Mirrors `open_stl`'s background
    /// load; the source's blocking import overlay is not ported.
    fn load_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.loading = true;
        // The empty scene's blocking-import card: label plus the file name.
        self.empty_scene.loading = true;
        self.empty_scene.loading_label = Some("Importing your file…".into());
        self.empty_scene.loading_detail = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_owned().into());
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { stl::load_stl(&path) })
                .await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(mesh) => this.set_mesh(mesh),
                    Err(error) => eprintln!("{error:#}"),
                }
                this.loading = false;
                this.empty_scene.loading = false;
                this.empty_scene.loading_label = None;
                this.empty_scene.loading_detail = None;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn open_stl(&mut self, cx: &mut Context<Self>) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("STL", &["stl"])
            .pick_file()
        else {
            return;
        };
        self.load_path(path, cx);
    }

    /// `GLOBAL.DELETE` / the fixed `Delete` key: remove the selected model.
    /// The source deletes the selection (or the active model) through the
    /// history store; this build has no undo, so the removal is final.
    fn delete_model(&mut self) {
        self.mesh = None;
        self.supports.borrow_mut().clear();
        self.selected.set(false);
        self.num_layers = 0;
        self.layer = 0.0;
        self.lower_layer = 0.0;
        self.mode = Mode::Prepare;
        // Models going to zero resets the camera home (`useStlLoadCameraIntro`).
        self.camera
            .borrow_mut()
            .home(glam::Vec3::new(0.0, 0.0, 110.0));
    }

    /// Raise a toast for `TOAST_DURATION_MS`, replacing the current one.
    fn show_toast(&mut self, text: String, tone: ui::ToastTone, cx: &mut Context<Self>) {
        self.toast_seq += 1;
        let id = self.toast_seq;
        self.toast = Some(Toast { text, tone, id });
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(TOAST_DURATION_MS))
                .await;
            this.update(cx, |this, cx| {
                if this.toast.as_ref().is_some_and(|toast| toast.id == id) {
                    this.toast = None;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    /// `handleDroppedPrepareFiles`: scene files take the import path, mesh
    /// files the load path.
    fn handle_dropped_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        if self.mode != Mode::Prepare {
            return;
        }
        let supported: Vec<PathBuf> = paths
            .into_iter()
            .filter(|path| is_supported_prepare_drop(path))
            .collect();
        if supported.is_empty() {
            return;
        }

        if let Some(scene) = supported.iter().find(|path| is_scene_file(path)) {
            // No scene importer exists in this build, so the source's
            // `reportUnsupportedSceneFile` is the truthful outcome.
            let name = scene
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_owned();
            self.show_toast(
                format!("Unsupported scene file: {name}"),
                ui::ToastTone::Error,
                cx,
            );
            return;
        }

        if let Some(stl) = supported
            .iter()
            .find(|path| file_extension(path) == "stl")
        {
            self.load_path(stl.clone(), cx);
        }
    }
}

impl Render for DragonFruit {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // `handleModeChange`: an empty scene can only be Prepare.
        if self.mesh.is_none() && self.mode != Mode::Prepare {
            self.mode = Mode::Prepare;
        }

        if !self.scale_logged {
            self.scale_logged = true;
            eprintln!("window scale factor: {}", window.scale_factor());
        }

        if self.camera.borrow_mut().step(1.0 / 60.0) {
            window.request_animation_frame();
        }
        if self.empty_scene.loading {
            window.request_animation_frame();
        }
        let mesh = self.mesh.clone();
        let click_mesh = self.mesh.clone();
        let supports = self.supports.clone();
        let model_matrix = glam::Mat4::from_scale_rotation_translation(
            glam::Vec3::splat(self.scale),
            glam::Quat::from_rotation_z(self.rotate_z),
            glam::Vec3::from_array(self.translate),
        );
        // `publishClip`'s upper plane: none at layer 0 or at the top layer,
        // parked (infinite) when the cross-section switch is closed.
        let clip_z = if self.cross_section && self.mesh.is_some() {
            let upper = self.layer;
            let max_layer = self.num_layers as f32;
            if upper <= 0.0 || upper >= max_layer {
                f32::INFINITY
            } else {
                self.mesh
                    .as_ref()
                    .map(|mesh| {
                        let z_min = mesh.center.z - mesh.size[2] * 0.5;
                        z_min + upper * LAYER_HEIGHT_MM + CLIP_EPSILON_MM
                    })
                    .unwrap_or(f32::INFINITY)
            }
        } else {
            f32::INFINITY
        };
        let gizmo = if self.mode == Mode::Prepare && self.active_tool == 1 {
            self.mesh
                .as_ref()
                .map(|mesh| model_matrix.transform_point3(mesh.center).to_array())
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
        let hovered = self.hovered.clone();
        let last = self.last.clone();
        let smoke = self.smoke;
        let selected_state = self.selected.clone();
        let hovered_state = self.hovered.clone();
        let smoke_done = self.smoke_done.clone();
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
            let base_color = mesh_color(selected_state.get(), hovered_state.get());
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
            let hovered = hovered.clone();
            let hover_mesh = click_mesh.clone();
            move |event, window, cx| {
                if let Some(button) = drag.get() {
                    let (lx, ly) = last.get();
                    let dx = event.position.x.0 - lx;
                    let dy = event.position.y.0 - ly;
                    last.set((event.position.x.0, event.position.y.0));
                    let mut camera = camera.borrow_mut();
                    match button {
                        MouseButton::Right => {
                            camera.orbit(dx, dy, window.viewport_size().height.0.max(1.0))
                        }
                        MouseButton::Middle => {
                            camera.pan(dx, dy, window.viewport_size().height.0.max(1.0))
                        }
                        _ => {}
                    }
                    entity.update(cx, |_, cx| cx.notify()).ok();
                    return;
                }
                // `hoveredMeshModelId`: the pointer over the model tints it.
                let hit = hover_mesh.as_ref().is_some_and(|mesh| {
                    let size = window.viewport_size();
                    let width = size.width.0.max(1.0);
                    let height = size.height.0.max(1.0);
                    let ndc_x = event.position.x.0 / width * 2.0 - 1.0;
                    let ndc_y = 1.0 - event.position.y.0 / height * 2.0;
                    let (origin, direction) =
                        camera.borrow().ray(width / height, ndc_x, ndc_y);
                    let inverse = model_matrix.inverse();
                    let origin = inverse.transform_point3(origin);
                    let direction = inverse.transform_vector3(direction);
                    stl::raycast(mesh, origin, direction)
                });
                if hovered.get() != hit {
                    hovered.set(hit);
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
        })
        // `#scene-root`'s drag handlers: the overlay tracks the canvas bounds.
        .on_drag_move::<ExternalPaths>({
            let entity = entity.clone();
            move |event, _window, cx| {
                let position = event.event.position;
                let bounds = event.bounds;
                let inside = position.x >= bounds.origin.x
                    && position.x <= bounds.origin.x + bounds.size.width
                    && position.y >= bounds.origin.y
                    && position.y <= bounds.origin.y + bounds.size.height;
                let unsupported = !event
                    .drag(cx)
                    .0
                    .iter()
                    .any(|path| is_supported_prepare_drop(path));
                entity
                    .update(cx, |this, cx| {
                        if this.mode != Mode::Prepare {
                            return;
                        }
                        let changed =
                            this.drag_active != inside || this.drag_unsupported != unsupported;
                        this.drag_active = inside;
                        this.drag_unsupported = unsupported;
                        if changed {
                            cx.notify();
                        }
                    })
                    .ok();
            }
        })
        .on_file_drop_exit({
            let entity = entity.clone();
            move |_, _, cx| {
                entity
                    .update(cx, |this, cx| {
                        if this.drag_active || this.drag_unsupported {
                            this.drag_active = false;
                            this.drag_unsupported = false;
                            cx.notify();
                        }
                    })
                    .ok();
            }
        })
        .on_drop::<ExternalPaths>({
            let entity = entity.clone();
            move |paths, _window, cx| {
                let paths: Vec<PathBuf> = paths.0.iter().cloned().collect();
                entity
                    .update(cx, |this, cx| {
                        this.drag_active = false;
                        this.drag_unsupported = false;
                        this.handle_dropped_paths(paths, cx);
                        cx.notify();
                    })
                    .ok();
            }
        });

        let mesh_name = self
            .mesh
            .as_ref()
            .map(|mesh| mesh.name.clone())
            .unwrap_or_else(|| "No model".to_owned());
        let mode = self.mode;
        let show_settings = self.show_settings;
        let has_models = self.mesh.is_some();
        let layer = self.layer;

        self.window_width = window.viewport_size().width.0;
        if !self.focused {
            window.focus(&self.focus, cx);
            self.focused = true;
        }
        let key_entity = entity.clone();
        div()
            .size_full()
            .relative()
            .font_family("Geist")
            .text_size(px(14.))
            .line_height(relative(1.45))
            .bg(rgb(BACKGROUND))
            .track_focus(&self.focus)
            .on_key_down(move |event, _window, cx| {
                let key = event.keystroke.key.to_ascii_lowercase();
                let modifiers = event.keystroke.modifiers;
                let primary = modifiers.control || modifiers.platform;
                let alt = modifiers.alt;
                let shift = modifiers.shift;
                let repeat = event.is_held;
                key_entity
                    .update(cx, |this, cx| {
                        let mut changed = false;

                        // `useEscapeToClose`: Escape closes the dialogs and menus.
                        if key == "escape" {
                            if this.show_settings {
                                this.show_settings = false;
                                changed = true;
                            }
                            if this.show_app_menu {
                                this.show_app_menu = false;
                                changed = true;
                            }
                            if this.show_printer_menu {
                                this.show_printer_menu = false;
                                changed = true;
                            }
                            if this.show_view_menu {
                                this.show_view_menu = false;
                                changed = true;
                            }
                            if changed {
                                cx.notify();
                            }
                            return;
                        }

                        // The settings dialog owns the keyboard while open; GPUI
                        // has no "focused input" query to test instead.
                        if this.show_settings {
                            return;
                        }

                        // `CAMERA.TOGGLE_PROJECTION` (o).
                        if key == "o" && !primary && !alt && !repeat {
                            let mut camera = this.camera.borrow_mut();
                            camera.orthographic = !camera.orthographic;
                            changed = true;
                        }

                        // `CAMERA.FOCUS_PICK` (f): frame the model. The source
                        // prefers the hovered point; there is no hover raycast
                        // here, so the whole model is framed.
                        if key == "f" && !primary && !alt && !repeat {
                            if let Some(mesh) = &this.mesh {
                                let mut camera = this.camera.borrow_mut();
                                camera.focus(mesh.center, mesh.radius.max(1.0));
                                changed = true;
                            }
                        }

                        // The home button / `Home`: `CameraHomeResetController`.
                        if key == "home" && !primary && !alt && !repeat {
                            let mut camera = this.camera.borrow_mut();
                            camera.home(glam::Vec3::new(0.0, 0.0, 110.0));
                            changed = true;
                        }

                        // `usePrepareTransformHotkeys`: canvas tools only in
                        // Prepare with a model. `shift+m` is the local-space
                        // twin; the flag has no field here.
                        if this.mode == Mode::Prepare && this.mesh.is_some() && !primary && !alt {
                            let tool = match key.as_str() {
                                "q" => Some(0),
                                "m" => Some(1),
                                "s" => Some(7),
                                "a" => Some(5),
                                "d" => Some(4),
                                _ => None,
                            };
                            if let Some(tool) = tool {
                                if this.active_tool != tool {
                                    this.active_tool = tool;
                                    changed = true;
                                }
                            }
                            let _ = shift;
                        }

                        // `GLOBAL.DELETE` (Backspace) plus the fixed Delete key:
                        // exact modifier match, no repeat.
                        if (key == "backspace" || key == "delete")
                            && !primary
                            && !alt
                            && !shift
                            && !repeat
                            && this.mesh.is_some()
                        {
                            this.delete_model();
                            changed = true;
                        }

                        // `NAVIGATION.LAYER_UP` / `LAYER_DOWN`: Up/Down, and
                        // W/S in printing. Alt/Ctrl/Meta are ignored by the
                        // source's handler; Shift is allowed.
                        let is_printing = this.mode == Mode::Printing;
                        let is_up = key == "up" || (is_printing && key == "w");
                        let is_down = key == "down" || (is_printing && key == "s");
                        if (is_up || is_down) && !primary && !alt && this.num_layers > 0 {
                            let delta = if is_up { 1.0 } else { -1.0 };
                            let next = (this.layer + delta)
                                .clamp(0.0, this.num_layers as f32);
                            if next != this.layer {
                                this.layer = next;
                                changed = true;
                            }
                        }

                        if changed {
                            cx.notify();
                        }
                    })
                    .ok();
            })
            .child(canvas)
            .when(!has_models, |this| {
                this.child(empty_scene::empty_scene_state(
                    self.empty_scene.clone(),
                    entity.clone(),
                ))
            })
            .when(has_models, |this| {
                this.child(
                    div()
                        .absolute()
                        .bottom(px(4.))
                        .left(px(4.))
                        .child(empty_scene::model_stats_card(
                            self.stats.clone(),
                            entity.clone(),
                        )),
                )
            })
            .when(mode == Mode::Printing && has_models, |this| {
                this.child(printing_preview::printing_preview(
                    self.printing_preview.clone(),
                    entity.clone(),
                ))
            })
            // `isPrepareDragActive` overlay, inside `#scene-root` and below the
            // floating panels.
            .when(self.drag_active && self.mode == Mode::Prepare, |this| {
                this.child(drop_overlay(self.drag_unsupported))
            })
            // `ToolRail` renders only in Prepare/Support with models.
            .when(has_models && matches!(mode, Mode::Prepare | Mode::Support), |this| {
                this.child(rail::tool_rail(mode, self.active_tool, entity.clone()))
            })
            .child({
                let mut defs = Vec::new();
                // `PreparePanelStack` / Support's stack carry the Models panel;
                // Export and Printing do not.
                if has_models && matches!(mode, Mode::Prepare | Mode::Support) {
                    defs.push(
                        panels::PanelDef::new("models", "Models", panels::PanelKind::Models)
                            .dock(panels::PanelDock::Left)
                            .header_right(panel_content::models_add_button(entity.clone()))
                            .content(panel_content::models_body(
                                mesh_name.clone(),
                                triangles,
                                self.selected.get(),
                                entity.clone(),
                            )),
                    );
                }
                match mode {
                    Mode::Prepare => {
                        if has_models {
                            let meta: Option<(&'static str, &'static str, panels::PanelKind)> =
                                match self.active_tool {
                                    1 => Some((
                                        "prepare-transform",
                                        "Transform",
                                        panels::PanelKind::Transform,
                                    )),
                                    4 => Some((
                                        "prepare-duplicate",
                                        "Duplicate",
                                        panels::PanelKind::Duplicate,
                                    )),
                                    5 => Some((
                                        "prepare-arrange",
                                        "Arrange",
                                        panels::PanelKind::Arrange,
                                    )),
                                    6 => Some((
                                        "prepare-split",
                                        "Organic Cut",
                                        panels::PanelKind::OrganicCut,
                                    )),
                                    7 => Some((
                                        "prepare-smooth",
                                        "Smoothing",
                                        panels::PanelKind::Smoothing,
                                    )),
                                    _ => None,
                                };
                            if let Some((id, title, kind)) = meta {
                                if let Some(body) = tool_panels::tool_panel(
                                    self.active_tool,
                                    mode,
                                    entity.clone(),
                                ) {
                                    defs.push(
                                        panels::PanelDef::new(id, title, kind)
                                            .dock(panels::PanelDock::Right)
                                            .content(body),
                                    );
                                }
                            }
                        }
                    }
                    Mode::Support => {
                        // `SupportPanelStack`: the rail tool selects the panel.
                        let meta: Option<(&'static str, &'static str, panels::PanelKind)> =
                            match self.active_tool {
                                0 => Some((
                                    "support-islands",
                                    "Islands",
                                    panels::PanelKind::Supports,
                                )),
                                1 => Some((
                                    "support-settings",
                                    "Support Settings",
                                    panels::PanelKind::Supports,
                                )),
                                2 => Some((
                                    "support-auto",
                                    "Auto Supports (Beta)",
                                    panels::PanelKind::Supports,
                                )),
                                3 => Some((
                                    "support-hollowing",
                                    "Hollowing",
                                    panels::PanelKind::Supports,
                                )),
                                _ => None,
                            };
                        if let Some((id, title, kind)) = meta {
                            let body = if self.active_tool == 3 {
                                Some(
                                    support_panels::hollowing_body(
                                        &self.hollowing,
                                        entity.clone(),
                                    )
                                    .into_any_element(),
                                )
                            } else {
                                tool_panels::tool_panel(
                                    self.active_tool,
                                    mode,
                                    entity.clone(),
                                )
                            };
                            if let Some(body) = body {
                                defs.push(
                                    panels::PanelDef::new(id, title, kind)
                                        .dock(panels::PanelDock::Right)
                                        .content(body),
                                );
                            }
                        }
                    }
                    Mode::Export => defs.push(
                        panels::PanelDef::new(
                            "export-main",
                            "Export",
                            panels::PanelKind::Export,
                        )
                        .dock(panels::PanelDock::Left)
                        .content(panel_content::export_body(entity.clone())),
                    ),
                    Mode::Printing => defs.push(
                        panels::PanelDef::new(
                            "printing",
                            "Printing",
                            panels::PanelKind::Printing,
                        )
                        .dock(panels::PanelDock::Right)
                        .content(panel_content::printing_body(entity.clone())),
                    ),
                }
                panels::panel_stack(&self.layout, defs, entity.clone())
            })
            // The Z-up gizmo sits 72px from the right edge, 22px from the
            // bottom (its 120px widget is centred on the cube).
            .child(
                div()
                    .absolute()
                    .bottom(px(22.))
                    .right(px(72.))
                    .child(overlays::view_cube(camera.clone(), entity.clone())),
            )
            // `CrossSliderPanel` wrapper: hidden in printing (the source's
            // printing scrubber lives in the unported preview pane).
            .when(has_models && mode != Mode::Printing, |this| {
                this.child(
                    div()
                        .absolute()
                        .right(px(0.))
                        .top(px(TOPBAR_HEIGHT + 6.0))
                        .bottom(px(6.0))
                        .w(px(48.))
                        .child(
                            ui::card()
                                .size_full()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .py(px(4.0))
                                        .flex_1()
                                        .min_h(px(0.0))
                                        .flex()
                                        .flex_col()
                                        .child(overlays::layer_slider(
                                            layer,
                                            Some(self.lower_layer),
                                            self.cross_section,
                                            0.0,
                                            self.num_layers as f32,
                                            1.0,
                                            entity.clone(),
                                        )),
                                ),
                        ),
                )
            })
            .child(app_bar::app_bar(
                app_bar::AppBarState {
                    mode,
                    triangles,
                    loading,
                    has_printing_data: false,
                    view_type: self.view_type,
                    overhang: self.overhang_view,
                    show_settings,
                    show_printer_menu: self.show_printer_menu,
                    show_app_menu: self.show_app_menu,
                    show_view_menu: self.show_view_menu,
                    logo: self.logo.clone(),
                    window_width: self.window_width,
                    maximized: window.is_maximized(),
                    printer: Default::default(),
                    monitor: Default::default(),
                },
                entity.clone(),
            ))
            .child(settings::settings_modal(
                show_settings,
                self.settings_tab,
                entity.clone(),
            ))
            .child(auto_support_settings::auto_support_settings(
                self.show_support_settings,
                entity.clone(),
            ))
            .when(self.toast.is_some(), |this| {
                let toast = self.toast.clone().expect("checked above");
                this.child(toast_view(toast))
            })
    }
}

/// `page.tsx`'s drop overlay: a dimmed backdrop and a dashed card.
fn drop_overlay(unsupported: bool) -> impl IntoElement {
    let accent = if unsupported { DANGER } else { ACCENT };
    // `color-mix(in srgb, black, transparent 86%)` / `(danger, transparent 90%)`.
    let backdrop = if unsupported {
        rgba(0xe454541a)
    } else {
        rgba(0x00000024)
    };
    // `color-mix(in srgb, danger|accent, surface-0 88%/90%)`.
    let card = if unsupported {
        mix(DANGER, SURFACE_0, 0.12)
    } else {
        mix(ACCENT, SURFACE_0, 0.10)
    };
    let title = if unsupported {
        "Unsupported file format"
    } else {
        "Drop supported files to import"
    };
    let detail = if unsupported {
        format!("Please use: STL, OBJ, 3MF, {SCENE_FILE_LABELS}")
    } else {
        format!("Supported: STL, OBJ, 3MF, {SCENE_FILE_LABELS}")
    };

    div()
        .absolute()
        .top(px(0.))
        .left(px(0.))
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .absolute()
                .top(px(0.))
                .left(px(0.))
                .size_full()
                .bg(backdrop),
        )
        .child(
            div()
                .min_w(px(380.))
                .max_w(px(640.))
                .rounded(px(12.))
                .border_1()
                .border_dashed()
                .border_color(rgb(accent))
                .bg(rgb(card))
                .px(px(32.))
                .py(px(24.))
                .flex()
                .flex_col()
                .items_center()
                .child(
                    div()
                        .text_size(px(16.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child(title),
                )
                .child(
                    div()
                        .mt(px(8.))
                        .text_size(px(14.))
                        .text_color(rgb(TEXT_MUTED))
                        .child(detail),
                ),
        )
}

/// `NotificationStack`'s toast: pill, `px-4 py-2 text-sm font-semibold`, the
/// tone icon at 16px and an 8px gap.
fn toast_view(toast: Toast) -> impl IntoElement {
    let icon_name = match toast.tone {
        ui::ToastTone::Error | ui::ToastTone::Warning => "triangle-alert",
        ui::ToastTone::Success => "circle-check",
        _ => "info",
    };
    ui::toast_viewport(
        ui::ToastViewportPosition::BottomCenter,
        TOAST_OFFSET,
        ui::toast(
            toast.tone,
            ui::ToastShape::Pill,
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(icons::icon(icon_name, 16., TEXT_STRONG))
                .child(toast.text),
        ),
    )
}

fn file_extension(path: &Path) -> String {
    path.extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// `isSupportedPrepareDropName`.
fn is_supported_prepare_drop(path: &Path) -> bool {
    PREPARE_DROP_EXTENSIONS.contains(&file_extension(path).as_str())
}

/// `isSceneFileName`: `.voxl` plus the built-in plugin scene types (`.lys`).
fn is_scene_file(path: &Path) -> bool {
    matches!(file_extension(path).as_str(), "voxl" | "lys")
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
fn mesh_color(selected: bool, hovered: bool) -> [f32; 3] {
    // Calibrated against the real app's selected poussin render: the model's
    // material is a saturated pink, not the #a3a3a3 default blended with the
    // selection tint (whose red cannot reach this value).
    if selected {
        [0.78, 0.10, 0.22]
    } else if hovered {
        // `blendTintColor('#a3a3a3', meshHoverColor, 0.5)` in linear space;
        // the dark theme's hover colour is the accent.
        let base = srgb_to_linear(0xa3);
        let tint = [
            srgb_to_linear(0xec),
            srgb_to_linear(0x2a),
            srgb_to_linear(0x77),
        ];
        [
            base + (tint[0] - base) * 0.5,
            base + (tint[1] - base) * 0.5,
            base + (tint[2] - base) * 0.5,
        ]
    } else {
        [srgb_to_linear(0xa3), srgb_to_linear(0xa3), srgb_to_linear(0xa3)]
    }
}

fn rasterize_logo() -> Option<PathBuf> {
    let svg =
        std::fs::read_to_string("../public/dragonfruit_assets/branding/simple_icon.svg").ok()?;
    let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).ok()?;
    let size = tree.size();
    let scale = 112.0 / size.width().max(size.height());
    let mut pixmap = resvg::tiny_skia::Pixmap::new(112, 112)?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    let path = std::env::temp_dir().join("dragonfruit-logo.png");
    pixmap.save_png(&path).ok()?;
    Some(path)
}

fn main() {
    fonts::ensure_geist();
    gpui::application().run(|cx: &mut App| {
        // Subpixel needs an opaque window background, which is the default
        // (`window.rs` never sets `background_appearance`).
        cx.set_text_rendering_mode(TextRenderingMode::Subpixel);
        let bounds = Bounds::centered(None, size(px(1280.), px(800.)), cx);
        let options = window::with_wgpu_renderer(WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("DragonFruit".into()),
                appears_transparent: true,
                traffic_light_position: None,
            }),
            ..Default::default()
        });
        log::log("gpui: opening window");
        cx.open_window(options, |_, cx| cx.new(DragonFruit::new))
            .expect("a window");
        cx.activate(true);
    });
}
