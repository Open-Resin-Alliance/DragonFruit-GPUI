use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use camera::Camera;
use gpui::{
    AnyElement, Context, Corners, ExternalPaths, FocusHandle, FontWeight, IntoElement,
    MouseButton, Render, ScrollDelta, Window, div, gpu_canvas, prelude::*, px, relative, rgb,
    rgba,
};
use gpui_interop::attach;
use gpui_wgpu::{ImportedTextureExt as _, WgpuRenderer};
use viewport::Viewport;

use crate::chrome::{app_bar, overlays, rail};
use crate::gizmo;
use crate::panels::empty_scene;
use crate::{features, panels};

use anyhow::Result;
use gpui_core::{camera, icons, stl, viewport};
use gpui_ui::{panel_fields, ui};
use gpui_core::theme::*;

// The loader lives at `src/scene/mod.rs` (a sibling of this file), not in the
// `src/app/` submodule directory, so name the path explicitly.
#[path = "scene/mod.rs"]
mod scene;

/// A file the drop/launch path can open: a plain mesh or a VOXL scene.
enum Loaded {
    Mesh(stl::MeshData),
    Scene(scene::Imported),
}

/// The single entry point both the background load and the launch path use.
/// `is_scene_file` already claims `.voxl`/`.lys`, so those go to the VOXL
/// reader and everything else keeps the existing STL path.
fn load_file(path: &Path) -> Result<Loaded> {
    if is_scene_file(path) {
        scene::load(path).map(Loaded::Scene)
    } else {
        stl::load_stl(path).map(Loaded::Mesh)
    }
}

/// The toast for a failed import. VOXL's own errors are worth surfacing; a
/// `.lys` is a Lychee scene, which this build has no importer for.
fn load_failure_text(path: &Path, error: &anyhow::Error) -> String {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    match file_extension(path).as_str() {
        "voxl" => format!("VOXL import failed: {error:#}"),
        "lys" => format!("Unsupported scene file: {name}"),
        _ => format!("Could not import {name}: {error:#}"),
    }
}

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
pub(crate) enum Mode {
    Prepare,
    Support,
    Export,
    Printing,
}

impl Mode {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Mode::Prepare => "Prepare",
            Mode::Support => "Support",
            Mode::Export => "Export",
            Mode::Printing => "Printing",
        }
    }

    pub(crate) fn step(self) -> u32 {
        match self {
            Mode::Prepare => 1,
            Mode::Support => 2,
            Mode::Export => 3,
            Mode::Printing => 4,
        }
    }
}

/// Which panel the Support stack shows.
///
/// The source keeps two independent fields — `supportRailMode`
/// (`islands`/`manual`/`auto`, `page.tsx:1615`) and
/// `transformMgr.transformMode === 'hollowing'` — and renders every panel with
/// `display: none` unless it matches (`page.tsx:10860-10882`). One enum carries
/// both because this build mounts exactly one Support panel at a time: selecting
/// a rail panel writes its variant, and Hollow writes [`Self::Hollowing`],
/// which is the source's "Hollowing wins while it is selected".
///
/// `active_tool` stays the Prepare tool only, so switching Support panels never
/// overwrites the tool the user will return to (the source's
/// `prepareToolMemoryRef`).
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SupportPanel {
    Islands,
    /// The source's `useState<SupportRailMode>('manual')` default.
    #[default]
    Manual,
    Auto,
    Hollowing,
}

impl SupportPanel {
    /// The three rail panel tiles, in `buildSupportToolRailEntries` order
    /// (`toolRailEntries.ts:208-210`): Islands, Manual, Automatic.
    pub(crate) const RAIL: [SupportPanel; 3] = [Self::Islands, Self::Manual, Self::Auto];

    /// The panel a rail tile selects, by its index in [`Self::RAIL`].
    pub(crate) fn from_rail(index: usize) -> Self {
        Self::RAIL.get(index).copied().unwrap_or_default()
    }

    /// The Support panel body's tool index (`features::tool_panels` only knows
    /// the three `supportRailMode` panels; Hollowing has its own body).
    fn tool_index(self) -> Option<usize> {
        match self {
            Self::Islands => Some(0),
            Self::Manual => Some(1),
            Self::Auto => Some(2),
            Self::Hollowing => None,
        }
    }

    /// The stack entry's id, title and kind.
    fn meta(self) -> (&'static str, &'static str, panels::list::PanelKind) {
        match self {
            Self::Islands => (
                "support-islands",
                "Islands",
                panels::list::PanelKind::Supports,
            ),
            Self::Manual => (
                "support-settings",
                "Support Settings",
                panels::list::PanelKind::Supports,
            ),
            Self::Auto => (
                "support-auto",
                "Auto Supports (Beta)",
                panels::list::PanelKind::Supports,
            ),
            Self::Hollowing => (
                "support-hollowing",
                "Hollowing",
                panels::list::PanelKind::Supports,
            ),
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

pub(crate) struct DragonFruit {
    pub(crate) mesh: Option<Arc<stl::MeshData>>,
    /// The parsed VOXL container when the last load was a scene: `SUPP` and
    /// every model's `MODL` entry. `mesh`/`translate`/`rotation`/`scale` stay
    /// the single-model view the viewport draws.
    pub(crate) scene: Option<gpui_voxl::Scene>,
    /// Time of the last frame, for the camera's real delta. The source drives
    /// every tween from `requestAnimationFrame`'s timestamp, so the port must
    /// not advance at a fixed step: a 144Hz window would finish a tween 2.4x
    /// early.
    pub(crate) last_frame: Option<std::time::Instant>,
    pub(crate) camera: Rc<RefCell<Camera>>,
    pub(crate) viewport: Rc<RefCell<Option<Viewport>>>,
    /// A live grab: the plate-plane point under the pointer at press time and
    /// the transform the model had then. A ray/plane hit is angle-correct; a
    /// per-pixel scale is not — at a low camera angle vertical pointer travel
    /// covers far less plate distance.
    pub(crate) grab: Option<(glam::Vec3, gizmo::Transform)>,
    pub(crate) drag: Rc<Cell<Option<MouseButton>>>,
    pub(crate) last: Rc<Cell<(f32, f32)>>,
    /// Written by `app_bar` (New scene) and `panel_content` (export). The
    /// source has no status line, so the shell does not render it.
    #[allow(dead_code)]
    pub(crate) status: String,
    pub(crate) loading: bool,
    pub(crate) selected: Rc<Cell<bool>>,
    /// `hoveredMeshModelId` — the pointer is over the model.
    pub(crate) hovered: Rc<Cell<bool>>,
    pub(crate) smoke: bool,
    pub(crate) smoke_done: Rc<Cell<bool>>,
    pub(crate) mode: Mode,
    pub(crate) show_settings: bool,
    pub(crate) layout: gpui_core::panel_layout::PanelLayout,
    pub(crate) empty_scene: gpui_core::empty_state::EmptySceneState,
    pub(crate) stats: gpui_core::empty_state::ModelStatsCardState,
    pub(crate) islands: gpui_core::support_state::IslandsPanelState,
    pub(crate) island_list: gpui_core::support_state::IslandListState,
    pub(crate) hollowing: gpui_core::support_state::HollowingState,
    pub(crate) hole_punch: gpui_core::support_state::HolePunchState,
    pub(crate) support_settings: gpui_core::support_state::SupportSettingsState,
    pub(crate) printing_preview: gpui_core::state::PrintingPreviewState,
    /// Upper layer index, `0..=num_layers`.
    pub(crate) layer: f32,
    /// Lower layer index (the second thumb).
    pub(crate) lower_layer: f32,
    /// The cross-section switch (`crossSectionEnabled`).
    pub(crate) cross_section: bool,
    pub(crate) num_layers: u32,
    pub(crate) supports: Rc<RefCell<Option<gpui_core::viewport::SupportBatches>>>,
    pub(crate) show_grid: bool,
    pub(crate) show_plate: bool,
    pub(crate) show_box: bool,
    pub(crate) overhang_view: bool,
    pub(crate) logo: Option<PathBuf>,
    pub(crate) translate: [f32; 3],
    /// The Euler rotation in radians, `[x, y, z]` about the fixed world axes:
    /// the same three numbers the source's `ModelTransform` stores in an
    /// `Euler` of order `'ZYX'`, which `quaternionFromGlobalEuler` builds as
    /// `qz · qy · qx`.
    pub(crate) rotation: [f32; 3],
    /// Per-axis scale, `1.0` at the model's own size. The gizmo's scale handles
    /// multiply all three (`uniformScaling`, the source's default) and so does
    /// the transform panel's uniform switch; the vector is the type
    /// `applyScaleFactor` multiplies, so a per-axis mode has somewhere to write.
    pub(crate) scale: [f32; 3],
    /// The running gesture's stand-in transform.
    ///
    /// The source separates the two halves of a drag: the live transform moves
    /// the model, and only the release commits it. This field is that stand-in.
    /// The viewport, picking and the gizmo read it in place of the committed
    /// fields above; the transform panel's store and `session.json` see nothing
    /// until [`Self::commit_draft`] folds it in, once, on release.
    pub(crate) draft: Option<gizmo::Transform>,
    pub(crate) support_angle: f32,
    pub(crate) support_density: f32,
    pub(crate) focus: FocusHandle,
    pub(crate) focused: bool,
    pub(crate) show_printer_menu: bool,
    /// The bar's "Select printer" surface (`features::printer_picker`): the
    /// Printer Library, then the `Initial Setup` variant dialog. Set by
    /// `printer-menu-button` when the profile has no fleet units to quick-switch.
    pub(crate) show_printer_picker: bool,
    /// The Auto Supports settings dialog (`AutoSupportSettingsBody`). Set by the
    /// Auto panel's header gear (`tool_panels::supports_settings_gear`) and
    /// cleared by Escape or by the dialog's own close tile.
    pub(crate) show_support_settings: bool,
    pub(crate) show_app_menu: bool,
    /// `TopBar`'s view-mode dropdown (`SelectDropdown` under the trigger).
    pub(crate) show_view_menu: bool,
    /// `viewType` — the persisted mesh shader the dropdown shows.
    pub(crate) view_type: gpui_core::state::MeshShaderType,
    /// Opened by the top bar's monitor button; the printing monitor modal is
    /// not ported.
    #[allow(dead_code)]
    pub(crate) show_monitor: bool,
    /// The fleet unit picked in the printer menu; the monitor modal is not
    /// ported.
    #[allow(dead_code)]
    pub(crate) select_fleet_unit: Option<String>,
    pub(crate) settings_tab: usize,
    /// The Prepare tool index (`active_tool` in the source). Support selection
    /// lives in [`Self::support_panel`] instead, so the two workspaces keep
    /// their own tool, as the source's `prepareToolMemoryRef` does.
    pub(crate) active_tool: usize,
    /// `supportRailMode` plus the Hollowing tool: the Support stack's selection.
    pub(crate) support_panel: SupportPanel,
    /// `settings.json` / `session.json`: the only state that survives a
    /// restart. `None` when the data directory cannot be opened, in which case
    /// the app runs exactly as before and nothing is persisted.
    store: Option<gpui_store::Store>,
    /// The file this session opened, written to `session.json` on exit.
    last_file: Option<PathBuf>,
    /// `session.json`'s `lastFile`, opened on the first frame: `new` has no
    /// built entity to spawn the background load from.
    pub(crate) window_width: f32,
    /// An OS file drag is over the canvas (`isPrepareDragActive`).
    pub(crate) drag_active: bool,
    /// The dragged paths contain no supported extension
    /// (`isPrepareDragUnsupported`).
    pub(crate) drag_unsupported: bool,
    /// The editor context menu's window-space position. Set by a right mouse-up
    /// that did not travel (`EditorContextMenu`); `chrome::editor_menu` reads and
    /// clears it, and a left press or Escape clears it here.
    pub(crate) editor_menu: Option<(f32, f32)>,
    /// The right button's press position, so its release can tell a menu click
    /// from an orbit — a right drag is the camera.
    right_press: Option<(f32, f32)>,
    /// Whether the current right gesture travelled past the click slop at any
    /// point. Set from the move handler, so an orbit that returns to where it
    /// started still counts as an orbit.
    right_dragged: bool,
    /// The gizmo handle under the pointer (`TransformGizmo`'s `hoveredPart`).
    pub(crate) gizmo_hover: Option<gizmo::Handle>,
    /// The running gizmo gesture. A handle owns the pointer while this is set.
    pub(crate) gizmo_drag: Option<gizmo::Drag>,
    /// The gizmo's triangles, rebuilt only when the pose or the handle state
    /// changes (`GeometryCache`).
    gizmo_cache: Option<gizmo::GeometryCache>,
    pub(crate) toast: Option<Toast>,
    pub(crate) toast_seq: u64,
    /// The window scale factor is logged on the first frame.
    pub(crate) scale_logged: bool,
}

impl DragonFruit {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        let smoke = std::env::args().any(|arg| arg == "--smoke");
        let path = std::env::args()
            .skip(1)
            .find(|arg| !arg.starts_with("--"))
            .map(PathBuf::from);

        // The store is the only state that outlives the process:
        // `settings.json` hydrates the settings draft once, and `session.json`
        // restores the last file, camera pose, Prepare tool and panel layout.
        // A store that cannot open leaves every one of those at its default and
        // disables the saves below.
        let store = gpui_store::Store::open().ok();
        let session = match &store {
            Some(store) => {
                features::settings::hydrate(store.load_settings());
                Some(store.load_session())
            }
            None => None,
        };

        let mut app = Self {
            mesh: None,
            scene: None,
            last_frame: None,
            camera: Rc::new(RefCell::new(Camera::parked(glam::Vec3::new(0.0, 0.0, 110.0)))),
            viewport: Rc::new(RefCell::new(None)),
            grab: None,
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
            supports: Rc::new(RefCell::new(None)),
            show_grid: true,
            show_plate: true,
            show_box: true,
            overhang_view: false,
            logo: rasterize_logo(),
            translate: [0.0; 3],
            rotation: [0.0; 3],
            scale: [1.0; 3],
            draft: None,
            support_angle: 45.0,
            support_density: 0.5,
            focus: cx.focus_handle(),
            focused: false,
            show_printer_menu: false,
            show_printer_picker: false,
            show_support_settings: false,
            show_app_menu: false,
            show_view_menu: false,
            view_type: gpui_core::state::MeshShaderType::SoftClay,
            show_monitor: false,
            select_fleet_unit: None,
            settings_tab: 0,
            active_tool: 0,
            support_panel: SupportPanel::default(),
            store,
            last_file: None,
            window_width: 0.0,
            drag_active: false,
            drag_unsupported: false,
            editor_menu: None,
            right_press: None,
            right_dragged: false,
            gizmo_hover: None,
            gizmo_drag: None,
            gizmo_cache: None,
            toast: None,
            toast_seq: 0,
            scale_logged: false,
        };
        if let Some(session) = &session {
            app.apply_session(session);
        }
        if let Some(path) = path {
            match load_file(&path) {
                Ok(loaded) => {
                    app.apply_loaded(loaded);
                    app.last_file = Some(path);
                }
                Err(error) => {
                    eprintln!("{error:#}");
                    let text = load_failure_text(&path, &error);
                    app.show_toast(text, ui::ToastTone::Error, cx);
                }
            }
        }
        // `lastFile` is remembered for the recent list, not reopened: the app
        // starts on an empty plate, which is what the source does.
        app
    }

    /// Restore `session.json`: the camera pose, the Prepare tool and the panel
    /// layout. [`Self::last_file`] keeps the path, which `new` hands to
    /// [`Self::pending_reopen`] when the command line named no file.
    fn apply_session(&mut self, session: &gpui_store::Session) {
        if let Some(pose) = session.camera {
            restore_camera(&self.camera, &pose);
        }
        // A session file from a build without the key loads as the identity;
        // a hand-edited one with a NaN must not poison the model matrix.
        if session.rotation.iter().all(|angle| angle.is_finite()) {
            self.rotation = session.rotation;
        }
        // The rail owns the list; a document from a newer build with more tools
        // must not index past this build's match arms.
        self.active_tool = (session.active_tool as usize).min(rail::prepare_tool_count() - 1);
        apply_session_layout(&self.layout, &session.panel_layout);
        self.last_file = session.last_file.clone().filter(|path| path.is_file());
    }

    /// Persist a settings edit for the frame that just handled input, and wait
    /// on nothing: the debounced writer coalesces edits and owns the disk IO.
    /// The read-only guard covers a `settings.json` written by a newer build,
    /// which must not be downgraded.
    fn persist_settings(&self) {
        if !features::settings::take_dirty() {
            return;
        }
        let Some(store) = &self.store else {
            return;
        };
        if store.is_read_only(gpui_store::StoreFile::Settings) {
            return;
        }
        let _ = store.save_settings_debounced(&features::settings::snapshot());
    }

    /// Write `session.json` now. Session saves are direct, not debounced: they
    /// happen at deliberate points (a load, a scene close, exit), not at input
    /// speed. A newer document is left untouched.
    fn save_session(&self) {
        let Some(store) = &self.store else {
            return;
        };
        if store.is_read_only(gpui_store::StoreFile::Session) {
            return;
        }
        let _ = store.save_session(&self.session_document());
    }

    /// `session.json`'s document for the current state.
    fn session_document(&self) -> gpui_store::Session {
        let camera = self.camera.borrow();
        gpui_store::Session {
            last_file: self.last_file.clone(),
            camera: Some(gpui_store::CameraPose {
                yaw: camera.yaw,
                pitch: camera.pitch,
                distance: camera.distance,
                target: camera.target.to_array(),
                fov_y: camera.fov_y,
                orthographic: camera.orthographic,
            }),
            active_tool: self.active_tool as u32,
            rotation: self.rotation,
            panel_layout: session_layout(&self.layout),
            ..Default::default()
        }
    }

    /// Install a finished load. A plain mesh keeps the identity transform; a
    /// VOXL scene also installs the first model's placement and keeps the
    /// parsed container for the support panel.
    fn apply_loaded(&mut self, loaded: Loaded) {
        match loaded {
            Loaded::Mesh(mesh) => {
                self.scene = None;
                // Plate centre is the origin in the port's plate-local frame;
                // `defaultImportCenterXY` is computed from `plateLocalRect` in the source.
                self.translate = scene::placement(&mesh, [0.0, 0.0]);
                self.rotation = [0.0; 3];
                self.scale = [1.0; 3];
                self.set_mesh(mesh);
            }
            Loaded::Scene(import) => {
                self.translate = import.translate;
                self.rotation = import.rotation;
                self.scale = import.scale;
                if self.smoke {
                    let models = import.scene.models.len();
                    let meshes = import.scene.meshes.len();
                    eprintln!(
                        "smoke: VOXL v{} with {models} model(s), {meshes} MESH chunk(s); \
                         translate [{:.2}, {:.2}, {:.2}], rotation [{:.3}, {:.3}, {:.3}] rad, \
                         scale [{:.3}, {:.3}, {:.3}]",
                        import.scene.version,
                        self.translate[0],
                        self.translate[1],
                        self.translate[2],
                        self.rotation[0],
                        self.rotation[1],
                        self.rotation[2],
                        self.scale[0],
                        self.scale[1],
                        self.scale[2],
                    );
                }
                self.scene = Some(import.scene);
                self.set_mesh(import.mesh);
            }
        }
    }

    pub(crate) fn set_mesh(&mut self, mesh: stl::MeshData) {
        if self.smoke {
            // The gate reads these lines to check a load produced real
            // geometry: triangle count and millimetre bounding box.
            eprintln!(
                "smoke: mesh '{}': {} triangles, size {:.2} x {:.2} x {:.2} mm, \
                 center ({:.2}, {:.2}, {:.2}), radius {:.2} mm",
                mesh.name,
                mesh.triangles,
                mesh.size[0],
                mesh.size[1],
                mesh.size[2],
                mesh.center.x,
                mesh.center.y,
                mesh.center.z,
                mesh.radius,
            );
        }
        let mesh = Arc::new(mesh);
        // The intro animates from wherever the camera is (the parked home pose
        // in a fresh scene) to the model's framing.
        // Frame where the model actually lands: the import placement moves it, so
        // the raw bbox centre is no longer the model's world centre.
        let world_center = mesh.center + glam::Vec3::from_array(self.translate);
        self.camera
            .borrow_mut()
            .focus(world_center, mesh.radius.max(1.0));
        // `useSlicingManager`: `numLayers = ceil(heightMm / layerHeightMm)`.
        self.num_layers = (mesh.size[2] / LAYER_HEIGHT_MM).ceil().max(0.0) as u32;
        // The source opens with the upper thumb at the top and the lower at the
        // bottom, so the whole model is visible.
        self.layer = self.num_layers as f32;
        self.lower_layer = 0.0;
        self.selected.set(true);
        self.mesh = Some(mesh);
    }

    /// Load a mesh or scene file off the UI thread. Mirrors `open_stl`'s
    /// background load; the source's blocking import overlay is not ported.
    pub(crate) fn load_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.loading = true;
        // The empty scene's blocking-import card: label plus the file name.
        self.empty_scene.loading = true;
        self.empty_scene.loading_label = Some("Importing your file…".into());
        self.empty_scene.loading_detail = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_owned().into());
        cx.notify();
        let load_path = path.clone();
        let toast_path = path;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { load_file(&load_path) })
                .await;
            this.update(cx, move |this, cx| {
                match result {
                    Ok(loaded) => {
                        this.apply_loaded(loaded);
                        // The next launch reopens this file (`session.json`);
                        // session saves are direct, so record it now rather
                        // than only at exit.
                        this.last_file = Some(toast_path.clone());
                        this.save_session();
                    }
                    Err(error) => {
                        eprintln!("{error:#}");
                        // A failed import leaves the current scene untouched.
                        let text = load_failure_text(&toast_path, &error);
                        this.show_toast(text, ui::ToastTone::Error, cx);
                    }
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

    pub(crate) fn open_stl(&mut self, cx: &mut Context<Self>) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("STL", &["stl"])
            // `.lys` has no importer here; selecting it raises the unsupported
            // scene file toast after the loader rejects the container.
            .add_filter("VOXL scene", &["voxl", "lys"])
            .pick_file()
        else {
            return;
        };
        self.load_path(path, cx);
    }

    /// `GLOBAL.DELETE` / the fixed `Delete` key: remove the selected model.
    /// The source deletes the selection (or the active model) through the
    /// history store; this build has no undo, so the removal is final.
    pub(crate) fn delete_model(&mut self) {
        self.mesh = None;
        self.scene = None;
        *self.supports.borrow_mut() = None;
        self.selected.set(false);
        self.gizmo_drag = None;
        self.gizmo_hover = None;
        self.draft = None;
        self.grab = None;
        self.editor_menu = None;
        self.num_layers = 0;
        self.layer = 0.0;
        self.lower_layer = 0.0;
        self.mode = Mode::Prepare;
        // There is nothing left to reopen: a deleted model must not come back
        // with the next launch's `session.json`.
        self.last_file = None;
        // Models going to zero resets the camera home (`useStlLoadCameraIntro`).
        self.camera
            .borrow_mut()
            .home(glam::Vec3::new(0.0, 0.0, 110.0));
        self.save_session();
    }

    /// The transform everything on screen draws with: the running gesture's
    /// draft while one is live, the committed fields otherwise.
    pub(crate) fn transform(&self) -> gizmo::Transform {
        self.draft.unwrap_or(gizmo::Transform {
            translate: glam::Vec3::from_array(self.translate),
            rotation: self.rotation,
            scale: glam::Vec3::from_array(self.scale),
        })
    }

    /// The model matrix the viewport, the hover raycast and the gizmo all use.
    ///
    /// The transform's rotation and scale turn the model about **its own bbox
    /// centre**, not about the file origin: the source's loader normalises every
    /// geometry so its bbox centre is the local origin
    /// (`useStlGeometry.ts`: `geometry.translate(-preCenter.x, -preBBox.min.y,
    /// -preCenter.z)`), so `group.rotation`/`group.scale` pivot there. This
    /// port keeps the loader's vertices in file space, where the file origin can
    /// be metres away, so the pivot has to be written into the matrix —
    /// otherwise a rotation swings the model across the plate and leaves the
    /// gizmo (whose centre is the model's centre, and which is locked during a
    /// rotate/scale drag) standing where the model used to be.
    ///
    /// At identity this is exactly `T(translate)`, unchanged from before, so the
    /// import placement, the panel's position rows and the session are
    /// unaffected. Under a rotation the model's centre `translate + center` is
    /// invariant — which is what makes `Drag::locked_center` correct.
    pub(crate) fn model_matrix(&self) -> glam::Mat4 {
        let matrix = self.transform().matrix();
        let Some(mesh) = &self.mesh else {
            return matrix;
        };
        let pivot = mesh.center;
        glam::Mat4::from_translation(pivot) * matrix * glam::Mat4::from_translation(-pivot)
    }

    /// Fold a finished gesture's draft into the committed fields.
    ///
    /// This is the one write a drag performs on the entity: the pointer stream
    /// only ever touched [`Self::draft`]. Returns whether there was a draft to
    /// commit, which is also whether the transform panel needs its one sync.
    pub(crate) fn commit_draft(&mut self) -> bool {
        let Some(draft) = self.draft.take() else {
            return false;
        };
        self.translate = draft.translate.to_array();
        self.rotation = draft.rotation;
        self.scale = draft.scale.to_array();
        true
    }

    /// Throw the draft away, leaving the committed transform as it was. Escape
    /// and a cancelled gesture both end here; nothing was committed, so nothing
    /// needs rebroadcasting.
    pub(crate) fn discard_draft(&mut self) {
        self.draft = None;
    }

    /// Whether the gizmo answers the pointer.
    ///
    /// `SceneCanvas` mounts `UnifiedGizmo` in Prepare, in the Transform tool,
    /// with an active model — and the source's `enabled` gate is what makes a
    /// handle with nothing to transform impossible. The same three conditions
    /// are the gate here, with the port's selection standing in for
    /// `activeModelId`.
    fn gizmo_enabled(&self) -> bool {
        self.mode == Mode::Prepare
            && self.active_tool == 1
            && self.mesh.is_some()
            && self.selected.get()
    }

    /// Where the gizmo sits: the model's own centre through the transform the
    /// viewport draws with, or the position a rotation/scale drag locked at
    /// press (`ScreenSpaceGizmo`'s `lockedDragPosition`).
    fn gizmo_center(&self) -> Option<glam::Vec3> {
        if !self.gizmo_enabled() {
            return None;
        }
        if let Some(center) = self.gizmo_drag.as_ref().and_then(gizmo::Drag::locked_center) {
            return Some(center);
        }
        let mesh = self.mesh.as_ref()?;
        Some(self.model_matrix().transform_point3(mesh.center))
    }

    /// Start a gesture if `pointer` is over a handle. Returns whether the press
    /// was consumed, so the caller knows not to start a model grab.
    fn begin_gizmo_drag(
        &mut self,
        pointer: glam::Vec2,
        width: f32,
        height: f32,
        camera: &camera::Camera,
    ) -> bool {
        let Some(center) = self.gizmo_center() else {
            return false;
        };
        let view = gizmo::View::new(camera, width / height, glam::Vec2::new(width, height), center);
        let Some(handle) = gizmo::pick(&view, pointer) else {
            return false;
        };
        // The transform panel's `mesh_min_z` and the model matrix's pivot: the
        // model's local underside and bbox centre.
        let mesh_bounds = self
            .mesh
            .as_ref()
            .map(|mesh| (mesh.center.z - mesh.size[2] * 0.5, mesh.center.z));
        let Some(drag) = gizmo::Drag::begin(&view, pointer, handle, self.transform(), mesh_bounds)
        else {
            return false;
        };
        // The gesture starts from the committed transform, so the draft is a
        // copy of it until the pointer moves.
        self.draft = Some(drag.base());
        self.gizmo_drag = Some(drag);
        self.gizmo_hover = Some(handle);
        true
    }

    /// Feed one pointer sample to the running gesture.
    ///
    /// Only [`Self::draft`] changes: no panel store write, no history, no
    /// session — the release commits once through [`Self::commit_draft`]. The
    /// caller asks for the frame that repaints from the draft.
    fn update_gizmo_drag(
        &mut self,
        pointer: glam::Vec2,
        width: f32,
        height: f32,
        camera: &camera::Camera,
    ) -> bool {
        let Some(drag) = self.gizmo_drag.as_mut() else {
            return false;
        };
        let view = gizmo::View::new(
            camera,
            width / height,
            glam::Vec2::new(width, height),
            drag.anchor,
        );
        let Some(transform) = drag.update(&view, pointer) else {
            return false;
        };
        self.draft = Some(transform);
        true
    }

    /// The plate-plane grab: drag the model under the pointer in the world XY
    /// plane. `origin` is the plane hit at press and `base` the transform then,
    /// so a low camera angle cannot turn vertical travel into extra distance.
    ///
    /// Like the gizmo handles, this writes the draft only; the release commits.
    fn update_grab(
        &mut self,
        pointer: glam::Vec2,
        width: f32,
        height: f32,
        camera: &camera::Camera,
    ) -> bool {
        let Some((origin, base)) = self.grab else {
            return false;
        };
        let ndc_x = pointer.x / width * 2.0 - 1.0;
        let ndc_y = 1.0 - pointer.y / height * 2.0;
        let Some(now) = plane_hit(camera, width / height, ndc_x, ndc_y, 0.0) else {
            return false;
        };
        let delta = now - origin;
        let mut transform = base;
        transform.translate.x = base.translate.x + delta.x;
        transform.translate.y = base.translate.y + delta.y;
        if self.draft == Some(transform) {
            return false;
        }
        self.draft = Some(transform);
        true
    }

    /// Start the grab gesture under the pointer, if the pointer ray reaches the
    /// plate plane.
    fn begin_grab(
        &mut self,
        pointer: glam::Vec2,
        width: f32,
        height: f32,
        camera: &camera::Camera,
    ) -> bool {
        let ndc_x = pointer.x / width * 2.0 - 1.0;
        let ndc_y = 1.0 - pointer.y / height * 2.0;
        let Some(hit) = plane_hit(camera, width / height, ndc_x, ndc_y, 0.0) else {
            return false;
        };
        let base = self.transform();
        self.grab = Some((hit, base));
        self.draft = Some(base);
        true
    }

    /// Recompute the hovered handle for a pointer with no button down.
    fn update_gizmo_hover(
        &mut self,
        pointer: glam::Vec2,
        width: f32,
        height: f32,
        camera: &camera::Camera,
    ) -> bool {
        if self.gizmo_drag.is_some() {
            return false;
        }
        let hovered = self.gizmo_center().and_then(|center| {
            let view =
                gizmo::View::new(camera, width / height, glam::Vec2::new(width, height), center);
            gizmo::pick(&view, pointer)
        });
        if hovered == self.gizmo_hover {
            return false;
        }
        self.gizmo_hover = hovered;
        true
    }

    /// Raise a toast for `TOAST_DURATION_MS`, replacing the current one.
    pub(crate) fn show_toast(&mut self, text: String, tone: ui::ToastTone, cx: &mut Context<Self>) {
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

    /// `handleDroppedPrepareFiles`: a scene file takes the import path, a mesh
    /// file the load path; both run through [`Self::load_path`].
    pub(crate) fn handle_dropped_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
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

        if let Some(scene_file) = supported.iter().find(|path| is_scene_file(path)) {
            // Scene files (`.voxl`, `.lys`) go through the same loader as the
            // launch path; a `.lys` has no importer and the loader's error
            // raises `reportUnsupportedSceneFile`.
            self.load_path(scene_file.clone(), cx);
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

/// The window closing is this build's exit: write `session.json` and wait for
/// the debounced settings writer so an edit made in the last 500 ms still lands.
/// A read-only document (written by a newer build) is left untouched, matching
/// [`gpui_store::Store::save_session`]'s guard.
impl Drop for DragonFruit {
    fn drop(&mut self) {
        let Some(store) = &self.store else {
            return;
        };
        // An edit in the last frame may not have rendered yet, so the queue
        // could still be empty; take the flag here so closing the window never
        // loses the change.
        if features::settings::take_dirty() && !store.is_read_only(gpui_store::StoreFile::Settings)
        {
            let _ = store.save_settings_debounced(&features::settings::snapshot());
        }
        self.save_session();
        let _ = store.flush_debounced();
    }
}

impl Render for DragonFruit {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // `handleModeChange`: an empty scene can only be Prepare.
        if self.mesh.is_none() && self.mode != Mode::Prepare {
            self.mode = Mode::Prepare;
        }

        // Input handlers run before the frame they invalidate, so a settings
        // edit is on the draft by the time its `cx.notify()` renders. Taking
        // the flag here keeps the write off the click handler's stack.
        self.persist_settings();

        if !self.scale_logged {
            self.scale_logged = true;
            eprintln!("window scale factor: {}", window.scale_factor());
        }

        let now = std::time::Instant::now();
        let dt = self
            .last_frame
            .map(|last| now.duration_since(last).as_secs_f32())
            .unwrap_or(1.0 / 60.0)
            .clamp(1.0 / 1000.0, 0.1);
        self.last_frame = Some(now);
        if self.camera.borrow_mut().step(dt) {
            window.request_animation_frame();
        }
        if self.empty_scene.loading {
            window.request_animation_frame();
        }
        let mesh = self.mesh.clone();
        let click_mesh = self.mesh.clone();
        // `cameraInteractionCycleEnabled = hasModelsOnPlate && ...`: nothing on
        // the plate means the empty scene does not answer the pointer.
        let camera_enabled = Rc::new(Cell::new(self.mesh.is_some()));
        // Grab: a left drag that lands on the model slides it across the plate.
        // The source moves the model under the pointer in every prepare tool, not
        // only the Move tool.
        let grab_mode = self.mesh.is_some();
        // The model matrix is a pure function of the running draft, so the hot
        // path never reads a half-committed transform.
        let model_matrix = self.model_matrix();
        // `publishClip`'s upper plane: none at layer 0 or at the top layer,
        // parked (infinite) when the cross-section switch is closed.
        let supports = self.supports.clone();
        let translate = self.transform().translate;
        let clip_z = if self.cross_section && self.mesh.is_some() {
            let upper = self.layer;
            let max_layer = self.num_layers as f32;
            if upper <= 0.0 || upper >= max_layer {
                f32::INFINITY
            } else {
                self.mesh
                    .as_ref()
                    .map(|mesh| {
                        // The clip plane is in world space, so the model's own
                        // translate has to be included.
                        let z_min = mesh.center.z + translate.z - mesh.size[2] * 0.5;
                        z_min + upper * LAYER_HEIGHT_MM + CLIP_EPSILON_MM
                    })
                    .unwrap_or(f32::INFINITY)
            }
        } else {
            f32::INFINITY
        };
        // The transform gizmo. Mounted exactly under `UnifiedGizmo`'s conditions:
        // Prepare's Transform tool with a selected model — and drawn from the
        // entity's own handle state, so hover and the running gesture are what
        // the viewport sees.
        let gizmo_center = self.gizmo_center();
        let gizmo_draw = match gizmo_center {
            Some(center) => {
                let size = window.viewport_size();
                let width = size.width.0.max(1.0);
                let height = size.height.0.max(1.0);
                let active = self.gizmo_drag.as_ref().map(|drag| drag.handle);
                let camera = self.camera.borrow();
                Some(gizmo::vertices(
                    &mut self.gizmo_cache,
                    &camera,
                    width / height,
                    glam::Vec2::new(width, height),
                    center,
                    self.gizmo_hover,
                    active,
                ))
            }
            None => None,
        };
        let show_layers = viewport::ViewLayers {
            grid: self.show_grid,
            plate: self.show_plate,
            volume: self.show_box,
        };
        let camera = self.camera.clone();
        let viewport = self.viewport.clone();
        let cube_viewport = self.viewport.clone();
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
                supports.borrow().as_ref(),
                show_layers,
                model_matrix,
                clip_z,
                gizmo_draw.as_deref().map(|vertices| vertices.as_slice()),
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
            let camera = camera.clone();
            let camera_enabled = camera_enabled.clone();
            let entity = entity.clone();
            move |event, _, cx| {
                if !camera_enabled.get() {
                    return;
                }
                drag.set(Some(MouseButton::Right));
                camera.borrow_mut().set_dragging(true);
                last.set((event.position.x.0, event.position.y.0));
                // Remembered so the release can tell a context-menu click from
                // an orbit: a right drag turns the camera and must not open one.
                entity
                    .update(cx, |this, _| {
                        this.right_press = Some((event.position.x.0, event.position.y.0));
                        this.right_dragged = false;
                    })
                    .ok();
            }
        })
        .on_mouse_down(MouseButton::Middle, {
            let drag = drag.clone();
            let last = last.clone();
            let camera_enabled = camera_enabled.clone();
            let camera = camera.clone();
            move |event, _, _| {
                if !camera_enabled.get() {
                    return;
                }
                drag.set(Some(MouseButton::Middle));
                // Orbit's flag governs both gestures: without it a pan eases at
                // the release rate instead of tracking the pointer.
                camera.borrow_mut().set_dragging(true);
                last.set((event.position.x.0, event.position.y.0));
            }
        })
        .on_mouse_up(MouseButton::Left, {
            let drag = drag.clone();
            let entity = entity.clone();
            let camera = camera.clone();
            let sync_entity = entity.clone();
            move |event, window, cx| {
                if drag.get() != Some(MouseButton::Left) {
                    return;
                }
                drag.set(None);
                // The gesture's stand-in becomes the committed transform here and
                // nowhere else: one write, one panel sync, one repaint. A press
                // that never moved commits the transform it started from, which
                // is the same value the panel already shows.
                let committed = entity
                    .update(cx, |this, _| {
                        this.gizmo_drag = None;
                        this.grab = None;
                        this.commit_draft()
                    })
                    .unwrap_or(false);
                if committed {
                    // The transform panel reads its own store; this is the one
                    // push a whole gesture makes.
                    features::tool_panels::sync_from_entity(&sync_entity, cx);
                }
                // Re-evaluate the hover from the release point: without this the
                // handle keeps the held highlight until the pointer next moves.
                let size = window.viewport_size();
                let hover_camera = camera.borrow();
                entity
                    .update(cx, |this, _| {
                        this.update_gizmo_hover(
                            glam::Vec2::new(event.position.x.0, event.position.y.0),
                            size.width.0.max(1.0),
                            size.height.0.max(1.0),
                            &hover_camera,
                        )
                    })
                    .ok();
            }
        })
        .on_mouse_up(MouseButton::Right, {
            let drag = drag.clone();
            let camera = camera.clone();
            let entity = entity.clone();
            move |event, _, cx| {
                if drag.get() == Some(MouseButton::Right) {
                    drag.set(None);
                    camera.borrow_mut().set_dragging(false);
                }
                let position = (event.position.x.0, event.position.y.0);
                entity
                    .update(cx, |this, cx| {
                        let pressed = this.right_press.take().is_some();
                        let dragged = std::mem::replace(&mut this.right_dragged, false);
                        if !pressed {
                            return;
                        }
                        // A right button that travelled turned the camera; only
                        // the ones that stayed put open the editor menu.
                        if this.mesh.is_some() && !dragged {
                            this.editor_menu = Some(position);
                            cx.notify();
                        }
                    })
                    .ok();
            }
        })
        .on_mouse_up(MouseButton::Middle, {
            let drag = drag.clone();
            let camera = camera.clone();
            move |_, _, _| {
                if drag.get() == Some(MouseButton::Middle) {
                    drag.set(None);
                    camera.borrow_mut().set_dragging(false);
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
                    if button == MouseButton::Left {
                        let size = window.viewport_size();
                        let width = size.width.0.max(1.0);
                        let height = size.height.0.max(1.0);
                        let pointer = glam::Vec2::new(event.position.x.0, event.position.y.0);
                        let drag_camera = camera.clone();
                        entity
                            .update(cx, |this, cx| {
                                let camera = drag_camera.borrow();
                                // A gizmo handle owns the left button while it is
                                // held; the plate grab below only runs when no
                                // gesture is live. Both write the draft and ask
                                // for the frame that repaints from it — the
                                // panel is told once, on release.
                                let moved = if this.gizmo_drag.is_some() {
                                    this.update_gizmo_drag(pointer, width, height, &camera)
                                } else if this.grab.is_some() {
                                    this.update_grab(pointer, width, height, &camera)
                                } else {
                                    false
                                };
                                if moved {
                                    cx.notify();
                                }
                            })
                            .ok();
                        return;
                    }
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
                    drop(camera);
                    if button == MouseButton::Right {
                        // Once a right gesture has travelled, its release is an
                        // orbit's, whatever the release position says.
                        let pointer = (event.position.x.0, event.position.y.0);
                        entity
                            .update(cx, |this, _| {
                                if let Some(press) = this.right_press {
                                    let dx = pointer.0 - press.0;
                                    let dy = pointer.1 - press.1;
                                    let slop = gizmo::CLICK_SLOP_PX * gizmo::CLICK_SLOP_PX;
                                    if dx * dx + dy * dy > slop {
                                        this.right_dragged = true;
                                    }
                                }
                            })
                            .ok();
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
                // The gizmo's hover is its own question: a handle sits in front
                // of the mesh, so it has to highlight even where the pointer is
                // not over the model at all.
                let size = window.viewport_size();
                let width = size.width.0.max(1.0);
                let height = size.height.0.max(1.0);
                let pointer = glam::Vec2::new(event.position.x.0, event.position.y.0);
                let hover_camera = camera.clone();
                entity
                    .update(cx, |this, cx| {
                        let camera = hover_camera.borrow();
                        if this.update_gizmo_hover(pointer, width, height, &camera) {
                            cx.notify();
                        }
                    })
                    .ok();
            }
        })
        .on_mouse_down(MouseButton::Left, {
            let camera = camera.clone();
            let mesh = click_mesh.clone();
            let entity = entity.clone();
            move |event, window, cx| {
                let size = window.viewport_size();
                let width = size.width.0.max(1.0);
                let height = size.height.0.max(1.0);
                let pointer = glam::Vec2::new(event.position.x.0, event.position.y.0);

                // A left press anywhere closes the editor menu, and a press on a
                // gizmo handle starts a transform gesture instead of a model
                // grab. This has to run before the mesh raycast: the handles sit
                // in front of the model, so the ray would otherwise select the
                // mesh under a handle the user is aiming at.
                let grab_camera = camera.clone();
                let started = entity
                    .update(cx, |this, cx| {
                        let had_menu = this.editor_menu.take().is_some();
                        let started = {
                            let camera = grab_camera.borrow();
                            this.begin_gizmo_drag(pointer, width, height, &camera)
                        };
                        if started || had_menu {
                            cx.notify();
                        }
                        started
                    })
                    .unwrap_or(false);
                if started {
                    // Same drag cell as the camera: the move handler branches on
                    // the button, so a gesture and an orbit cannot both be live.
                    drag.set(Some(MouseButton::Left));
                    last.set((pointer.x, pointer.y));
                    return;
                }

                let Some(mesh) = mesh.as_ref() else {
                    return;
                };
                let ndc_x = pointer.x / width * 2.0 - 1.0;
                let ndc_y = 1.0 - pointer.y / height * 2.0;
                let (origin, direction) = camera.borrow().ray(width / height, ndc_x, ndc_y);
                let inverse = model_matrix.inverse();
                let origin = inverse.transform_point3(origin);
                let direction = inverse.transform_vector3(direction);
                let hit = stl::raycast(mesh, origin, direction);
                if grab_mode && hit {
                    // Same drag cell as the camera: the move handler branches on
                    // the button, so a grab and an orbit cannot both be live.
                    drag.set(Some(MouseButton::Left));
                    last.set((event.position.x.0, event.position.y.0));
                    let grab_camera = camera.clone();
                    entity
                        .update(cx, |this, cx| {
                            this.selected.set(true);
                            let camera = grab_camera.borrow();
                            this.begin_grab(pointer, width, height, &camera);
                            cx.notify();
                        })
                        .ok();
                    return;
                }
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
            let camera_enabled = camera_enabled.clone();
            move |event, _window, cx| {
                if !camera_enabled.get() {
                    return;
                }
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
                            // The printer picker (`useEscapeToClose` in both
                            // the library and `PrinterVariantPickerModal`):
                            // the nested setup dialog closes first.
                            if this.show_printer_picker {
                                if !features::printer_picker::escape() {
                                    this.show_printer_picker = false;
                                }
                                changed = true;
                            }
                            if this.show_view_menu {
                                this.show_view_menu = false;
                                changed = true;
                            }
                            // The Auto Supports settings dialog.
                            if this.show_support_settings {
                                this.show_support_settings = false;
                                changed = true;
                            }
                            // The editor context menu, which is a canvas overlay
                            // rather than a dropdown of the bar.
                            if this.editor_menu.take().is_some() {
                                changed = true;
                            }
                            // A gizmo gesture in flight: `cancelActiveGizmoDrag`
                            // puts back what the gesture changed, so a cancelled
                            // drag leaves the model where the grab found it. The
                            // gesture only ever wrote the draft, so dropping it
                            // *is* the restore — nothing was committed, and the
                            // panel's store never saw the intermediate values.
                            if this.gizmo_drag.take().is_some() {
                                changed = true;
                            }
                            if this.grab.take().is_some() {
                                changed = true;
                            }
                            this.discard_draft();
                            if changed {
                                cx.notify();
                            }
                            return;
                        }

                        // The settings dialog owns the keyboard while open; GPUI
                        // has no "focused input" query to test instead. The Auto
                        // Supports settings dialog and the printer picker take
                        // the same hold.
                        if this.show_settings || this.show_support_settings || this.show_printer_picker {
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
                                let world = this.model_matrix().transform_point3(mesh.center);
                                camera.focus(world, mesh.radius.max(1.0));
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
                this.child(panels::empty_scene::empty_scene_state(
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
                        .child(panels::empty_scene::model_stats_card(
                            self.stats.clone(),
                            entity.clone(),
                        )),
                )
            })
            .when(mode == Mode::Printing && has_models, |this| {
                this.child(features::printing_preview::printing_preview(
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
                this.child(rail::tool_rail(
                    mode,
                    self.active_tool,
                    self.support_panel,
                    entity.clone(),
                ))
            })
            .child({
                let mut defs = Vec::new();
                // `PreparePanelStack` / Support's stack carry the Models panel;
                // Export and Printing do not.
                if has_models && matches!(mode, Mode::Prepare | Mode::Support) {
                    defs.push(
                        panels::list::PanelDef::new("models", "Models", panels::list::PanelKind::Models)
                            .dock(panels::list::PanelDock::Left)
                            .header_right(panels::content::models_add_button(entity.clone()))
                            .content(panels::content::models_body(
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
                            let meta: Option<(&'static str, &'static str, panels::list::PanelKind)> =
                                match self.active_tool {
                                    1 => Some((
                                        "prepare-transform",
                                        "Transform",
                                        panels::list::PanelKind::Transform,
                                    )),
                                    4 => Some((
                                        "prepare-duplicate",
                                        "Duplicate",
                                        panels::list::PanelKind::Duplicate,
                                    )),
                                    5 => Some((
                                        "prepare-arrange",
                                        "Arrange",
                                        panels::list::PanelKind::Arrange,
                                    )),
                                    6 => Some((
                                        "prepare-split",
                                        "Organic Cut",
                                        panels::list::PanelKind::OrganicCut,
                                    )),
                                    7 => Some((
                                        "prepare-smooth",
                                        "Smoothing",
                                        panels::list::PanelKind::Smoothing,
                                    )),
                                    _ => None,
                                };
                            if let Some((id, title, kind)) = meta {
                                if let Some(body) = features::tool_panels::tool_panel(
                                    self.active_tool,
                                    mode,
                                    entity.clone(),
                                ) {
                                    defs.push(
                                        panels::list::PanelDef::new(id, title, kind)
                                            .dock(panels::list::PanelDock::Right)
                                            .content(body),
                                    );
                                }
                            }
                        }
                    }
                    Mode::Support => {
                        // `SupportPanelStack`: `supportRailMode` (and the
                        // Hollowing tool) select one panel. The rail writes
                        // `support_panel`; `active_tool` stays Prepare's.
                        let (id, title, kind) = self.support_panel.meta();
                        let body = match self.support_panel {
                            SupportPanel::Hollowing => Some(
                                features::support_panels::hollowing_body(
                                    &self.hollowing,
                                    entity.clone(),
                                )
                                .into_any_element(),
                            ),
                            panel => panel.tool_index().and_then(|tool| {
                                features::tool_panels::tool_panel(tool, mode, entity.clone())
                            }),
                        };
                        if let Some(body) = body {
                            let def = panels::list::PanelDef::new(id, title, kind)
                                .dock(panels::list::PanelDock::Right)
                                .content(body);
                            // The Auto Supports panel's header gear opens the
                            // settings dialog; only that panel has one.
                            let def = if self.support_panel == SupportPanel::Auto {
                                def.header_right(
                                    features::tool_panels::supports_settings_gear(entity.clone()),
                                )
                            } else {
                                def
                            };
                            defs.push(def);
                        }
                    }
                    Mode::Export => defs.push(
                        panels::list::PanelDef::new(
                            "export-main",
                            "Export",
                            panels::list::PanelKind::Export,
                        )
                        .dock(panels::list::PanelDock::Left)
                        .content(panels::content::export_body(entity.clone())),
                    ),
                    Mode::Printing => defs.push(
                        panels::list::PanelDef::new(
                            "printing",
                            "Printing",
                            panels::list::PanelKind::Printing,
                        )
                        .dock(panels::list::PanelDock::Right)
                        .content(panels::content::printing_body(entity.clone())),
                    ),
                }
                panels::list::panel_stack(&self.layout, defs, entity.clone())
            })
            // The Z-up gizmo sits 72px from the right edge, 22px from the
            // bottom (its 120px widget is centred on the cube).
            .child(
                div()
                    .absolute()
                    .bottom(px(22.))
                    .right(px(72.))
                    .child(overlays::view_cube(
                        camera.clone(),
                        cube_viewport.clone(),
                        entity.clone(),
                    )),
            )
            .child(
                div()
                    .absolute()
                    // `HOME_OFFSET 0.93` along the gizmo's bottom-right diagonal:
                    // the chip's box sits just inside the widget's corner.
                    .bottom(px(10.0))
                    .right(px(60.0))
                    .child(overlays::home_button(camera.clone(), entity.clone())),
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
                    show_printer_menu: self.show_printer_menu,
                    show_app_menu: self.show_app_menu,
                    show_view_menu: self.show_view_menu,
                    logo: self.logo.clone(),
                    window_width: self.window_width,
                    maximized: window.is_maximized(),
                    printer: features::printer_picker::badge(),
                    monitor: Default::default(),
                },
                entity.clone(),
            ))
            .child(features::settings::settings_modal(
                show_settings,
                self.settings_tab,
                entity.clone(),
            ))
            .child(features::auto_support_settings::auto_support_settings(
                self.show_support_settings,
                entity.clone(),
            ))
            // The bar's printer picker: the library sheet and the nested
            // `Initial Setup` dialog, painted over the settings modal.
            .child(features::printer_picker::printer_picker(
                self.show_printer_picker,
                entity.clone(),
            ))
            .when(self.toast.is_some(), |this| {
                let toast = self.toast.clone().expect("checked above");
                this.child(toast_view(toast))
            })
            // `EditorContextMenu` is a portal in the source: the last child, so it
            // paints over the panels, the bar and the modals.
            .child(crate::chrome::editor_menu::editor_menu(
                self.editor_menu,
                entity.clone(),
            ))
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

/// Where the pointer's ray meets the plate plane at `z`.
fn plane_hit(camera: &Camera, aspect: f32, ndc_x: f32, ndc_y: f32, z: f32) -> Option<glam::Vec3> {
    let (origin, direction) = camera.ray(aspect, ndc_x, ndc_y);
    if direction.z.abs() < 1e-6 {
        return None;
    }
    let t = (z - origin.z) / direction.z;
    (t > 0.0).then(|| origin + direction * t)
}

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

/// A restored camera pose, applied to the parked camera before the first frame.
///
/// `Camera`'s smoothing goals are private, and `orient_to` is the only public
/// setter that writes them — writing `yaw`/`pitch` alone is undone by the next
/// `step`. `orient_to` starts the source's 400 ms `tweenCamera`; stepping it
/// once with a full second's delta lands on the end pose and clears the tween,
/// because a restored pose is a restore, not an entry animation. The distance,
/// target and projection set here are the tween's start and end, so only the
/// angles were ever in flight.
fn restore_camera(camera: &Rc<RefCell<Camera>>, pose: &gpui_store::CameraPose) {
    let mut camera = camera.borrow_mut();
    camera.distance = pose.distance;
    camera.target = glam::Vec3::from_array(pose.target);
    camera.fov_y = pose.fov_y;
    camera.orthographic = pose.orthographic;
    camera.orient_to(pose.yaw, pose.pitch);
    let _ = camera.step(1.0);
}

/// `panel_layout.rs`'s placement as the store's plain copy.
fn session_placement(
    placement: gpui_core::panel_layout::PanelPlacement,
) -> gpui_store::PanelPlacement {
    gpui_store::PanelPlacement {
        dock: match placement.dock {
            gpui_core::panel_layout::PanelDock::Left => gpui_store::PanelDock::Left,
            gpui_core::panel_layout::PanelDock::Right => gpui_store::PanelDock::Right,
            gpui_core::panel_layout::PanelDock::Float => gpui_store::PanelDock::Float,
        },
        order: placement.order as u32,
        width: placement.width,
        float: placement
            .float
            .map(|(x, y)| gpui_store::FloatPoint { x, y }),
    }
}

/// The live panel layout as `session.json`'s `panelLayout`.
fn session_layout(layout: &gpui_core::panel_layout::PanelLayout) -> gpui_store::SessionLayout {
    let state = layout.state.borrow();
    let mut panels = std::collections::BTreeMap::new();
    // `loaded` holds placements read from disk for panels not shown yet; the
    // live `panels` map wins for the panels this session did show.
    for (id, placement) in &state.loaded {
        panels.insert(id.clone(), session_placement(*placement));
    }
    for (id, placement) in &state.panels {
        panels.insert(String::from(*id), session_placement(*placement));
    }
    gpui_store::SessionLayout {
        panels,
        left_dock_width: state.left_width,
        right_dock_width: state.right_width,
        ..Default::default()
    }
}

/// Seed the panel layout from `session.json`.
///
/// `PanelLayout` persists itself to `floating-panel-layout.json` and loads that
/// file when it is constructed, so this only has work to do when that file is
/// absent (first run, or a deleted config): the session copy is the fallback,
/// never an override of a layout the user has actually moved since.
fn apply_session_layout(
    layout: &gpui_core::panel_layout::PanelLayout,
    saved: &gpui_store::SessionLayout,
) {
    {
        let state = layout.state.borrow();
        // `loaded` is populated from the layout's own file, so either map being
        // non-empty means the panel document already exists.
        if !state.panels.is_empty() || !state.loaded.is_empty() {
            return;
        }
    }
    let mut state = layout.state.borrow_mut();
    for (id, placement) in &saved.panels {
        let dock = match placement.dock {
            gpui_store::PanelDock::Left => gpui_core::panel_layout::PanelDock::Left,
            gpui_store::PanelDock::Right => gpui_core::panel_layout::PanelDock::Right,
            gpui_store::PanelDock::Float => gpui_core::panel_layout::PanelDock::Float,
        };
        state.loaded.insert(
            id.clone(),
            gpui_core::panel_layout::PanelPlacement {
                dock,
                order: placement.order as usize,
                float: placement.float.map(|point| (point.x, point.y)),
                width: placement.width,
            },
        );
    }
    state.left_width = saved.left_dock_width;
    state.right_width = saved.right_dock_width;
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
