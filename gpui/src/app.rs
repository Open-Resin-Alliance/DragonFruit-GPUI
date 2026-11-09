use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use camera::Camera;
use gpui::{
    AnyElement, Context, Corners, ExternalPaths, FocusHandle, FontWeight, IntoElement, MouseButton,
    Render, ScrollDelta, SharedString, Window, div, gpu_canvas, prelude::*, px, relative, rgb,
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
use gpui_core::theme::*;
use gpui_core::{camera, icons, stl, viewport};
use gpui_ui::{panel_fields, ui};

// The loader lives at `src/scene/mod.rs` (a sibling of this file), not in the
// `src/app/` submodule directory, so name the path explicitly.
#[path = "scene/mod.rs"]
mod scene;

// `--scenario`'s driver lives at `src/automation.rs`, a sibling of this file
// rather than in the `src/app/` submodule directory, so name the path.
#[path = "automation.rs"]
pub(crate) mod automation;

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

/// A model's committed transform, in the gizmo's terms.
fn committed_transform(model: &gpui_core::scene::Model) -> gizmo::Transform {
    gizmo::Transform {
        translate: glam::Vec3::from_array(model.translate),
        rotation: model.rotation,
        scale: glam::Vec3::from_array(model.scale),
    }
}

/// `safeRatio` (`src/features/scene/selectionTransform.ts`): a baseline near
/// zero yields 1, so a multiply never explodes or collapses the set.
fn safe_ratio(next: f32, current: f32) -> f32 {
    if current.abs() <= 1e-8 {
        return 1.0;
    }
    let ratio = next / current;
    if ratio.is_finite() { ratio } else { 1.0 }
}

/// `transform.position` for a live (uncommitted) transform — the model's world
/// bbox centre through that transform.
///
/// The source's loader re-centres geometry on its bbox, so its `position` is
/// that point; this port's `translate` is the file origin's placement, so the
/// same point is `translate + mesh.center` ([`gpui_core::scene::Model::position`]
/// for a committed model). The point is invariant under rotation and scale,
/// so a drag can read it without building the matrix.
fn live_position(transform: &gizmo::Transform, model: &gpui_core::scene::Model) -> glam::Vec3 {
    transform.translate + model.mesh.center
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
/// The build volume the viewport draws: `BUILD_VOLUME_MM` in
/// `gpui-core/src/viewport/construct.rs:912` (the source's generic VOXL
/// printer profile, `originMode: 'center'`). Import placement searches this
/// rect; the constant is repeated because the viewport's copy is private and
/// this slice only owns `app.rs`.
const PLATE_BUILD_VOLUME_MM: (f32, f32) = (218.88, 122.88);
/// `findFreeSpotCentersForModels`'s default argument
/// (`useSceneCollectionManager.ts:2640`).
const IMPORT_SPACING_MM: f32 = 5.0;
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

/// What a live gesture does to the selection.
///
/// The source's multi-selection maths is keyed on the operation
/// (`buildMultiSelectionTransformsFromActive`, `SceneCanvas.tsx:3042-3108`):
/// a move shifts every model by the primary's delta, a rotate orbits every
/// model about the selection pivot, a scale grows every model and its offset
/// from the pivot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GestureOperation {
    Move,
    Rotate,
    Scale,
}

/// A live gesture's selection-wide frame, captured at press.
///
/// `before` is the selection as it stood when the gesture began — the source's
/// `gizmoGroupStartSnapshot.beforeByModelId` (`SceneCanvas.tsx:1196-1206`) —
/// so a run of pointer samples is always derived from one state and cannot
/// drift. `pivot` is `getSelectionPivot`: the mean of the selected models'
/// positions, the point a group rotation or scale turns about.
pub(crate) struct Gesture {
    /// The model the gizmo's drag maths is driven by: `selected.last()` at
    /// press, the source's `activeModelId` for the gesture.
    pub(crate) primary: gpui_core::scene::ModelId,
    pub(crate) operation: GestureOperation,
    pub(crate) pivot: glam::Vec3,
    /// The selection's committed transforms at press, in selection order.
    pub(crate) before: Vec<(gpui_core::scene::ModelId, gizmo::Transform)>,
}

pub(crate) struct DragonFruit {
    /// The plate's models and the selection set. Every single-model surface
    /// (the gizmo's anchor, the transform panel, Delete, the name a toast
    /// shows) reads [`gpui_core::scene::Scene::primary`] — `selected.last()` —
    /// instead of a second stored "active" id.
    pub(crate) scene: gpui_core::scene::Scene,
    /// The parsed VOXL container when the last load was a scene: `SUPP` and
    /// every model's `MODL` entry. The scene above is the drawable view of it.
    /// Retained for the support panel's `SUPP` document.
    #[allow(dead_code)]
    pub(crate) voxl: Option<gpui_voxl::Scene>,
    /// `session.json`'s per-model transforms, held while the plate is empty.
    /// The port never reopens `last_file` at boot, so a launch has no scene to
    /// apply them to; [`Self::install_models`] and [`Self::append_mesh`] (for
    /// the first model) restore them when the same scene is loaded again.
    session_models: Vec<gpui_store::session::SessionModel>,
    /// `session.json`'s saved `active` index into [`Self::session_models`].
    ///
    /// This is the session file's own key (the source writes
    /// `scene.activeModelId`), kept only as a restore input: a matched load
    /// turns it into a one-model selection. The scene itself stores no active
    /// id — see [`gpui_core::scene::Scene`].
    session_active: Option<usize>,
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
    /// Loads requested while one is in flight, drained one at a time. Dropping
    /// several files asks for several appends at once, and two concurrent
    /// loads would each compute a free spot against the same plate and land
    /// their models on top of each other.
    pending_loads: std::collections::VecDeque<PathBuf>,
    /// `hoveredMeshModelId` — the model under the pointer, if any.
    pub(crate) hovered: Rc<Cell<Option<gpui_core::scene::ModelId>>>,
    pub(crate) smoke: bool,
    pub(crate) smoke_done: Rc<Cell<bool>>,
    /// `--scenario`: the steps the first frame hands to the driver. Taken once,
    /// so the driver is spawned exactly once.
    pub(crate) scenario: Option<automation::Scenario>,
    /// Whether the driver has been spawned (the `--scenario` run is one-shot).
    pub(crate) automation_started: bool,
    /// Frames rendered since launch. The automation driver waits for this to
    /// advance before it clicks again, so a scenario never races a paint.
    pub(crate) frames: u64,
    /// The panel ids the last frame passed to `panel_stack`. The automation
    /// snapshot reads this to prove a rail tile mounted its panel rather than
    /// merely moved a flag.
    pub(crate) mounted_panels: Vec<&'static str>,
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
    /// The running gesture's stand-in transform for the primary model.
    ///
    /// The source separates the two halves of a drag: the live transform moves
    /// the model, and only the release commits it. This field is that stand-in
    /// for the primary model. The viewport, picking and the gizmo read it in
    /// place of the committed model transform; the transform panel's store and
    /// `session.json` see nothing until [`Self::commit_draft`] folds it in,
    /// once, on release.
    pub(crate) draft: Option<gizmo::Transform>,
    /// The live gesture's selection-wide frame, and `draft_models` the
    /// per-model transforms it derives.
    pub(crate) gesture: Option<Gesture>,
    /// Every model the running gesture moves, with its live transform.
    ///
    /// The source keeps `multiGizmoPreviewTransformsById` for the same job
    /// (`SceneCanvas.tsx:3110-3120`): the primary drives the maths and the rest
    /// of the selection follows. Empty when no gesture is live; a single-model
    /// selection still stores its one entry, so the draw loop has a uniform
    /// source of truth.
    pub(crate) draft_models: Vec<(gpui_core::scene::ModelId, gizmo::Transform)>,
    pub(crate) support_angle: f32,
    pub(crate) support_density: f32,
    pub(crate) focus: FocusHandle,
    pub(crate) focused: bool,
    pub(crate) show_printer_menu: bool,
    /// The bar's "Select printer" surface (`features::printer_picker`): the
    /// Printer Library, then the `Initial Setup` variant dialog. Set by
    /// `printer-menu-button` when the profile has no fleet units to quick-switch.
    pub(crate) show_printer_picker: bool,
    /// The `Printer & Material Profiles` modal (`features::printer_profiles`).
    pub(crate) show_printer_profiles: bool,
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
    /// `session.json`'s `recentFiles`, most recent first. The rows the empty
    /// scene shows are rebuilt from it (see [`Self::sync_recents`]).
    recent_files: Vec<gpui_store::session::RecentFile>,
    /// The source's `reopenError`: a recent row's path no longer exists.
    reopen_error: Option<SharedString>,
    /// The viewport width from the last frame, handed to the top bar's
    /// responsive layout (`app_bar::State::window_width`).
    pub(crate) window_width: f32,
    /// The viewport height from the last frame. The profile modals' `88vh`
    /// caps are viewport percentages, and a percentage in GPUI would resolve
    /// against the padded overlay instead, so `automation`'s geometry facts
    /// need the real height.
    pub(crate) window_height: f32,
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
        let cli = cli_args();
        let smoke = cli.smoke;
        // A malformed scenario is a runner bug, not a UI state: report it and
        // leave before a window exists.
        let (scenario, scenario_model) = match cli.scenario {
            Some(path) => match automation::Scenario::load(&path) {
                Ok(scenario) => {
                    let model = scenario.model_path(&path);
                    (Some(scenario), model)
                }
                Err(error) => {
                    eprintln!("scenario: {error}");
                    std::process::exit(2);
                }
            },
            None => (None, None),
        };
        // A file named on the command line wins over the one a scenario
        // declares, which keeps `--scenario x.json other.stl` working.
        let mut paths = cli.files;
        if let Some(model) = scenario_model {
            if paths.is_empty() {
                paths.push(model);
            }
        }

        if scenario.is_some() {
            // A scenario must never hang a gate: if the driver stalls (a frame
            // that never paints, a dead window), fail loudly rather than sit
            // until the harness kills the process and reports nothing useful.
            std::thread::spawn(|| {
                std::thread::sleep(Duration::from_secs(90));
                eprintln!("scenario: timed out after 90s");
                std::process::exit(1);
            });
        }

        // The store is the only state that outlives the process:
        // `settings.json` hydrates the settings draft once, and `session.json`
        // restores the recent-file list, the Prepare tool and the panel layout.
        // The camera is not restored: a launch has no open file, so it starts
        // at the home pose the source uses. A store that cannot open leaves
        // every one of those at its default and disables the saves below.
        //
        // A scenario gets a throwaway store: it must not read the user's
        // restored tool/rotation, or the same script would pass on one machine
        // and fail on another, and it must not write over their session.
        let store = if scenario.is_some() {
            let root = std::env::temp_dir().join("dragonfruit-gpui-scenario");
            let _ = std::fs::remove_dir_all(&root);
            Some(gpui_store::Store::at(root))
        } else {
            gpui_store::Store::open().ok()
        };
        let session = match &store {
            Some(store) => {
                features::settings::hydrate(store.load_settings());
                Some(store.load_session())
            }
            None => None,
        };

        let mut app = Self {
            scene: gpui_core::scene::Scene::new(),
            voxl: None,
            session_models: Vec::new(),
            session_active: None,
            last_frame: None,
            camera: Rc::new(RefCell::new(Camera::parked(glam::Vec3::new(
                0.0, 0.0, 110.0,
            )))),
            viewport: Rc::new(RefCell::new(None)),
            grab: None,
            drag: Rc::new(Cell::new(None)),
            last: Rc::new(Cell::new((0.0, 0.0))),
            status: String::new(),
            loading: false,
            pending_loads: std::collections::VecDeque::new(),
            hovered: Rc::new(Cell::new(None)),
            smoke,
            smoke_done: Rc::new(Cell::new(false)),
            scenario,
            automation_started: false,
            frames: 0,
            mounted_panels: Vec::new(),
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
            draft: None,
            gesture: None,
            draft_models: Vec::new(),
            support_angle: 45.0,
            support_density: 0.5,
            focus: cx.focus_handle(),
            focused: false,
            show_printer_menu: false,
            show_printer_picker: false,
            show_printer_profiles: false,
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
            recent_files: Vec::new(),
            reopen_error: None,
            window_width: 0.0,
            window_height: 0.0,
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
        // Each path loads on top of the last: a mesh appends (a second gate
        // model must land beside the first), a scene file still replaces.
        for path in paths {
            match load_file(&path) {
                Ok(loaded) => {
                    app.apply_loaded(loaded);
                    app.last_file = Some(path.clone());
                    app.record_recent(&path);
                    app.save_session();
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

    /// Restore `session.json`: the Prepare tool, the panel layout, the
    /// recent-file list and the per-model transforms. [`Self::last_file`] keeps
    /// the path for the record only; the source never reopens a file on launch,
    /// so the camera stays where `new` parked it and the app always starts on
    /// the home pose.
    ///
    /// The plate is empty at this point, so the stored models are inert: there
    /// is no model to carry a transform yet. They stay in
    /// [`Self::session_models`] and [`Self::install_models`] applies them when
    /// a load produces the same scene (matched by name, in import order);
    /// [`Self::append_mesh`] applies a one-model session to the first STL.
    fn apply_session(&mut self, session: &gpui_store::Session) {
        // A newer build or a hand-edited file may hold a stale index; `Session`
        // sanitised it at the parse boundary, and the check keeps the
        // invariant local in case this is called with a hand-built document.
        let mut active = session.active;
        if active.is_some_and(|index| index >= session.models.len()) {
            active = None;
        }
        self.session_models = session.models.clone();
        self.session_active = active;
        // The rail owns the list; a document from a newer build with more tools
        // must not index past this build's match arms.
        self.active_tool = (session.active_tool as usize).min(rail::prepare_tool_count() - 1);
        apply_session_layout(&self.layout, &session.panel_layout);
        self.last_file = session.last_file.clone().filter(|path| path.is_file());
        self.recent_files = session.recent_files.clone();
        self.recent_files
            .truncate(gpui_store::session::RECENT_FILES_LIMIT);
        self.sync_recents();
    }

    /// Rebuild the empty scene's rows from [`Self::recent_files`].
    ///
    /// `RecentOpenedFile::id` is the entry's path: the source's `id` is a uuid
    /// keying its IndexedDB blob cache, which the port does not have, so the
    /// path is the only handle a row can reopen.
    fn sync_recents(&mut self) {
        self.empty_scene.recent_opened_files = self
            .recent_files
            .iter()
            .map(|entry| gpui_core::empty_state::RecentOpenedFile {
                id: entry.path.to_string_lossy().into_owned().into(),
                name: entry.name.clone().into(),
                kind: match entry.kind {
                    gpui_store::session::RecentFileKind::Mesh => {
                        gpui_core::empty_state::RecentFileKind::Mesh
                    }
                    gpui_store::session::RecentFileKind::Scene => {
                        gpui_core::empty_state::RecentFileKind::Scene
                    }
                },
                size_bytes: entry.size_bytes,
                opened_at_ms: entry.opened_at_ms,
            })
            .collect();
    }

    /// `trackRecentOpenedFiles`: newest first, one row per path, trimmed to the
    /// source's limit, with the empty scene's rows rebuilt. Runs before the
    /// load, as the source does, so a file that fails to parse is still listed;
    /// the caller persists the document.
    fn record_recent(&mut self, path: &Path) {
        if path.as_os_str().is_empty() {
            return;
        }
        let entry = gpui_store::session::RecentFile {
            path: path.to_path_buf(),
            name: path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            kind: if is_scene_file(path) {
                gpui_store::session::RecentFileKind::Scene
            } else {
                gpui_store::session::RecentFileKind::Mesh
            },
            size_bytes: std::fs::metadata(path).ok().map(|meta| meta.len()),
            opened_at_ms: now_ms(),
        };
        gpui_store::session::push_recent(&mut self.recent_files, entry);
        self.sync_recents();
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
    ///
    /// `camera` is not written: the source starts every launch at the home
    /// pose (a cold start has no model, and it never reopens the last file), so
    /// there is no pose for this build to restore. The field still loads.
    fn session_document(&self) -> gpui_store::Session {
        // With no scene, echo what `apply_session` read: the port never
        // reopens `last_file`, so the transforms stay inert until a load, and
        // an exit before that must not erase them. A scene writes its own
        // models, in import order, and its active model's index.
        let (models, active) = if self.scene.is_empty() {
            (self.session_models.clone(), self.session_active)
        } else {
            let models = self
                .scene
                .models()
                .iter()
                .enumerate()
                .map(|(index, model)| gpui_store::session::SessionModel {
                    // A scene's models all come from `last_file`; only a
                    // single-model load names one file of its own.
                    path: if self.scene.models().len() == 1 && index == 0 {
                        self.last_file.clone()
                    } else {
                        None
                    },
                    name: model.name.clone(),
                    translate: model.translate,
                    rotation: model.rotation,
                    scale: model.scale,
                    visible: model.visible,
                })
                .collect();
            let active = self.scene.primary().and_then(|id| {
                self.scene
                    .models()
                    .iter()
                    .position(|model| model.id == id)
            });
            (models, active)
        };
        gpui_store::Session {
            last_file: self.last_file.clone(),
            active_tool: self.active_tool as u32,
            models,
            active,
            recent_files: self.recent_files.clone(),
            panel_layout: session_layout(&self.layout),
            ..Default::default()
        }
    }

    /// Install a finished load. The two formats have different semantics,
    /// matching the source:
    ///
    /// * a single mesh (STL) **appends** one model to the plate — the source
    ///   pushes the new `LoadedModel` onto its collection
    ///   (`useSceneCollectionManager.ts:3525-3563`);
    /// * a VOXL scene replaces the plate with the file's own contents, because
    ///   it describes a whole collection.
    fn apply_loaded(&mut self, loaded: Loaded) {
        match loaded {
            Loaded::Mesh(mesh) => self.append_mesh(mesh),
            Loaded::Scene(import) => {
                if self.smoke {
                    let models = import.scene.models.len();
                    let meshes = import.scene.meshes.len();
                    let resolved = import.models.len();
                    eprintln!(
                        "smoke: VOXL v{} with {models} model(s), {meshes} MESH chunk(s), \
                         {resolved} resolved",
                        import.scene.version,
                    );
                }
                self.voxl = Some(import.scene);
                self.install_models(import.models);
            }
        }
    }

    /// Append one imported STL to the plate.
    ///
    /// The source pushes the new `LoadedModel` onto `models`, then moves it to
    /// the centre `findFreeSpotCentersForModels` reports
    /// (`useSceneCollectionManager.ts:3525-3563`, the placement at `:3555`).
    /// Every model already on the plate keeps its transform; only the new model
    /// is placed, made active and framed.
    fn append_mesh(&mut self, mesh: stl::MeshData) {
        self.voxl = None;
        let name = mesh.name.clone();
        // `defaultImportCenterXY` is the active plate's centre in world
        // coordinates (`useSceneCollectionManager.ts:2536-2550`). The port has
        // one plate and the viewport draws it centred on the origin
        // (`originMode: 'center'`), so the plate rect is symmetric about (0, 0)
        // and the two coincide.
        let plate = gpui_core::scene::Rect2D::from_center(
            0.0,
            0.0,
            PLATE_BUILD_VOLUME_MM.0,
            PLATE_BUILD_VOLUME_MM.1,
        );
        // A single STL arrives unrotated and unscaled (`:3539-3541`), so its
        // footprint is the mesh's own projected size. Placement sees the
        // visible models only, as `modelsRef.current.filter(model => model.visible)`
        // does (`:2661-2688`).
        let footprint = gpui_core::scene::projected_footprint_size(&mesh, [0.0; 3], [1.0; 3]);
        let blocked = self.scene.visible_footprints();
        let center = gpui_core::scene::find_free_spot_center(
            footprint,
            &blocked,
            plate,
            IMPORT_SPACING_MM,
        );
        let mut translate = scene::placement(&mesh, center);
        let mut rotation = [0.0; 3];
        let mut scale = [1.0; 3];
        let mut visible = true;
        let mesh = Arc::new(mesh);

        // On an empty plate `session.json`'s transform for this file is still
        // waiting, exactly as it was for a replacing load: apply it to the
        // first model, then consume the stored list. Once a model is on the
        // plate the scene owns the transforms and a later import must not
        // inherit them.
        if self.scene.is_empty() {
            let mut scratch = gpui_core::scene::Scene::new();
            scratch.push(
                name.clone(),
                mesh.clone(),
                translate,
                rotation,
                scale,
                visible,
            );
            if self.restore_session_models(&mut scratch) {
                let restored = &scratch.models[0];
                translate = restored.translate;
                rotation = restored.rotation;
                scale = restored.scale;
                visible = restored.visible;
            }
            self.session_models.clear();
            self.session_active = None;
        }

        self.log_smoke_model(&name, &mesh, translate);
        let id = self
            .scene
            .push(name, mesh, translate, rotation, scale, visible);
        // The import's new model is selected and framed: the caller passes the
        // id, the scene stores no "active model" for the intro to find.
        self.scene
            .select(id, gpui_core::scene::SelectionMode::Single);
        self.frame_primary_model();
    }

    /// Apply `session.json`'s stored transforms to a freshly loaded `scene`.
    ///
    /// The match is strict — same count, same names in the same import order —
    /// so a different file (or a different scene) does not inherit the old
    /// placement. A matched scene also restores the document's saved active
    /// model as a one-model selection, or clears the selection when the saved
    /// session had none. Returns whether the scene matched.
    fn restore_session_models(&self, scene: &mut gpui_core::scene::Scene) -> bool {
        let saved = &self.session_models;
        if saved.is_empty()
            || saved.len() != scene.models.len()
            || !scene
                .models
                .iter()
                .zip(saved)
                .all(|(model, entry)| model.name == entry.name)
        {
            return false;
        }
        for (model, entry) in scene.models.iter_mut().zip(saved) {
            let finite = entry
                .translate
                .iter()
                .chain(entry.rotation.iter())
                .chain(entry.scale.iter())
                .all(|value| value.is_finite());
            let non_zero_scale = entry.scale.iter().all(|value| *value != 0.0);
            if finite && non_zero_scale {
                model.translate = entry.translate;
                model.rotation = entry.rotation;
                model.scale = entry.scale;
            }
            model.visible = entry.visible;
        }
        let active = self
            .session_active
            .and_then(|index| scene.models.get(index).map(|model| model.id));
        match active {
            Some(id) => {
                scene.select(id, gpui_core::scene::SelectionMode::Single);
            }
            None => scene.clear_selection(),
        }
        true
    }

    /// Replace the plate's contents with `models` — the VOXL scene path — select
    /// the first one and frame it. The camera intro frames the primary model
    /// only; the scene's whole-bounds framing is a later step.
    fn install_models(&mut self, models: Vec<scene::ImportedModel>) {
        let mut installed = gpui_core::scene::Scene::new();
        for model in models {
            self.log_smoke_model(&model.name, &model.mesh, model.translate);
            installed.push(
                model.name,
                model.mesh,
                model.translate,
                model.rotation,
                model.scale,
                model.visible,
            );
        }
        // `session.json`'s transforms are inert while the plate is empty (the
        // port never reopens `last_file`), so this is the first moment they can
        // apply. The saved scene replaces the load's defaults only when it
        // matches the loaded models exactly; a different file keeps its own
        // placement. Either way the stored list is consumed: the scene now owns
        // the transforms, and the next save writes them.
        let restored = self.restore_session_models(&mut installed);
        if !restored {
            if let Some(first) = installed.models.first().map(|model| model.id) {
                installed.select(first, gpui_core::scene::SelectionMode::Single);
            }
        }
        self.session_models.clear();
        self.session_active = None;
        self.scene = installed;
        self.frame_primary_model();
    }

    /// The smoke gate's model line: triangle count, millimetre bounding box and
    /// the model's final world translate. It reads these lines to check a load
    /// produced real geometry.
    fn log_smoke_model(&self, name: &str, mesh: &stl::MeshData, translate: [f32; 3]) {
        if !self.smoke {
            return;
        }
        eprintln!(
            "smoke: model '{}': {} triangles, size {:.2} x {:.2} x {:.2} mm, \
             center ({:.2}, {:.2}, {:.2}), radius {:.2} mm, \
             translate [{:.2}, {:.2}, {:.2}]",
            name,
            mesh.triangles,
            mesh.size[0],
            mesh.size[1],
            mesh.size[2],
            mesh.center.x,
            mesh.center.y,
            mesh.center.z,
            mesh.radius,
            translate[0],
            translate[1],
            translate[2],
        );
    }

    /// The shared tail of installing or appending models: end any gesture and
    /// frame the primary model.
    ///
    /// The intro animates from wherever the camera is (the parked home pose in
    /// a fresh scene) to the primary model's framing. Frame where the model
    /// actually lands: the import placement moves it, so the raw bbox centre is
    /// no longer the model's world centre.
    fn frame_primary_model(&mut self) {
        self.end_gesture();
        self.hovered.set(None);
        self.gizmo_cache = None;

        if let Some(model) = self.scene.primary_model() {
            let world_center = model.world_center();
            self.camera
                .borrow_mut()
                .focus(world_center, model.mesh.radius.max(1.0));
            // `useSlicingManager`: `numLayers = ceil(heightMm / layerHeightMm)`,
            // where the height is the scene's tallest visible model.
            self.num_layers = (self.scene.max_height_world() / LAYER_HEIGHT_MM)
                .ceil()
                .max(0.0) as u32;
            // The source opens with the upper thumb at the top and the lower at
            // the bottom, so the whole model is visible.
            self.layer = self.num_layers as f32;
            self.lower_layer = 0.0;
        } else {
            self.num_layers = 0;
            self.layer = 0.0;
            self.lower_layer = 0.0;
        }
    }

    /// Load a mesh or scene file off the UI thread. Mirrors `open_stl`'s
    /// background load; the source's blocking import overlay is not ported.
    pub(crate) fn load_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if self.loading {
            self.pending_loads.push_back(path);
            return;
        }
        self.loading = true;
        // `trackRecentOpenedFiles` runs before the parse, so a file that fails
        // still lands in the list (the source tracks meshes the same way).
        self.reopen_error = None;
        self.record_recent(&path);
        self.save_session();
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
                // The next queued load starts only now, so it sees the model
                // this one appended and picks a free spot beside it.
                let next = this.pending_loads.pop_front();
                this.loading = next.is_some();
                if let Some(next) = next {
                    this.load_path(next, cx);
                    return;
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

    /// A recent row's click — the source's `onReopenRecentFile`.
    ///
    /// The source reopens from an IndexedDB blob, or from the entry's
    /// `sourcePath` when it has one; the port's entry *is* its path, so a path
    /// that no longer exists is the source's failed reopen and raises its
    /// `reopenError` line instead of a toast.
    pub(crate) fn reopen_recent(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if !path.is_file() {
            self.reopen_error = Some("Could not reopen this file from cache.".into());
            cx.notify();
            return;
        }
        self.load_path(path, cx);
    }

    /// `GLOBAL.DELETE` / the fixed `Delete` key: remove the selection.
    ///
    /// The source deletes the whole selection through
    /// `dispatchDeleteModelAction` (`src/features/scene/modelActionTargets.ts`),
    /// which resolves the target ids from `selectedModelIds`; this build has no
    /// undo, so the removal is final.
    ///
    /// With models left on the plate the scene keeps its placement; only the
    /// last model takes the empty-scene reset and forgets the reopenable file.
    pub(crate) fn delete_selected(&mut self) {
        if self.scene.remove_selected() == 0 {
            return;
        }
        self.after_scene_edit();
        self.save_session();
    }

    /// Remove one model — the Models panel row's trash, the source's
    /// `onDelete(model.id)`. Unlike [`Self::delete_selected`] this does not
    /// depend on the selection; it prunes the model from the set if it was
    /// there.
    pub(crate) fn delete_model(&mut self, id: gpui_core::scene::ModelId) {
        if !self.scene.remove(id) {
            return;
        }
        self.after_scene_edit();
        self.save_session();
    }

    /// The app menu's `New scene`: empty the plate whatever is selected. The
    /// source's `newScene` clears the collection; the port's old delete did
    /// both jobs because "the model" was the whole plate.
    pub(crate) fn clear_scene(&mut self) {
        self.scene = gpui_core::scene::Scene::new();
        self.after_scene_edit();
        self.save_session();
    }

    /// End any live gesture without committing it.
    ///
    /// Escape, a cancelled drag and every scene edit that invalidates the
    /// selection all end here: the draft is dropped, which *is* the restore,
    /// because nothing was ever written to a model.
    pub(crate) fn end_gesture(&mut self) {
        self.draft = None;
        self.draft_models.clear();
        self.gesture = None;
        self.gizmo_drag = None;
        self.gizmo_hover = None;
        self.grab = None;
    }

    /// The shared tail of a removal: drop the support set generated from the
    /// gone models, end any gesture, and re-derive the layer stack (or reset
    /// the empty scene).
    fn after_scene_edit(&mut self) {
        // The support set was generated from the deleted model.
        *self.supports.borrow_mut() = None;
        self.hovered.set(None);
        self.end_gesture();
        self.editor_menu = None;
        if self.scene.is_empty() {
            self.voxl = None;
            self.num_layers = 0;
            self.layer = 0.0;
            self.lower_layer = 0.0;
            self.mode = Mode::Prepare;
            // There is nothing left to reopen: a deleted model must not come
            // back with the next launch's `session.json`.
            self.last_file = None;
            self.session_models.clear();
            self.session_active = None;
            // Models going to zero resets the camera home
            // (`useStlLoadCameraIntro`).
            self.camera
                .borrow_mut()
                .home(glam::Vec3::new(0.0, 0.0, 110.0));
        } else {
            self.num_layers = (self.scene.max_height_world() / LAYER_HEIGHT_MM)
                .ceil()
                .max(0.0) as u32;
            self.layer = self.layer.clamp(0.0, self.num_layers as f32);
            self.lower_layer = self.lower_layer.clamp(0.0, self.layer);
        }
    }

    /// The primary model's transform: the running gesture's draft while one is
    /// live, the committed model transform otherwise.
    ///
    /// "Primary" is [`gpui_core::scene::Scene::primary`] — the last model the
    /// user selected, which is the source's `activeModelId` wherever a single
    /// transform is read (the gizmo's base, `session.json`, the automation
    /// snapshot). The transform panel asks for [`Self::panel_transform`]
    /// instead, which averages a multi-selection's position.
    pub(crate) fn transform(&self) -> gizmo::Transform {
        if let Some(draft) = self.draft {
            return draft;
        }
        self.scene
            .primary_model()
            .map(committed_transform)
            .unwrap_or_default()
    }

    /// The transform the Transform panel displays.
    ///
    /// `TransformControls` takes `selectionPositionOrigin` for the position
    /// while several models are selected — the mean of their
    /// `transform.position`s (`src/features/scene/selectionPosition.ts`,
    /// `getSelectionPositionOrigin`) — and the active model's rotation and
    /// scale (`PreparePanelStack.tsx:220-236`).
    ///
    /// PORT DEVIATION: the port's Transform panel has always shown the model's
    /// `translate` in its position rows (its `Center` action writes
    /// `-mesh.center`, which is translate-space), because that is the only
    /// position the editor has; so a multi-selection averages `translate` too,
    /// keeping one meaning for the field across one and many selected models.
    /// The source's `transform.position` is its geometry's bbox centre
    /// (`useStlGeometry.ts:635-639` re-centres on load); the gizmo uses that
    /// point ([`Self::gizmo_center`]), the panel keeps translate.
    pub(crate) fn panel_transform(&self) -> gizmo::Transform {
        let mut transform = self.transform();
        let targets = self.scene.action_targets();
        if targets.len() > 1 {
            let mut center = glam::Vec3::ZERO;
            let mut count = 0;
            for id in targets {
                if self.scene.model(id).is_some() {
                    center += self.live_transform(id).translate;
                    count += 1;
                }
            }
            if count > 0 {
                transform.translate = center / count as f32;
            }
        }
        transform
    }

    /// The transform `model` is drawn and picked with: the running gesture's
    /// per-model draft, or its committed transform.
    pub(crate) fn live_transform(&self, id: gpui_core::scene::ModelId) -> gizmo::Transform {
        if let Some((_, transform)) = self
            .draft_models
            .iter()
            .find(|(model_id, _)| *model_id == id)
        {
            return *transform;
        }
        if self.scene.primary() == Some(id) {
            if let Some(draft) = self.draft {
                return draft;
            }
        }
        self.scene
            .model(id)
            .map(committed_transform)
            .unwrap_or_default()
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
        let transform = self.transform();
        let Some(model) = self.scene.primary_model() else {
            return transform.matrix();
        };
        model.matrix_for(transform.translate, transform.rotation, transform.scale)
    }

    /// [`Self::model_matrix`] for any model, live draft included.
    pub(crate) fn model_matrix_of(&self, model: &gpui_core::scene::Model) -> glam::Mat4 {
        let transform = self.live_transform(model.id);
        model.matrix_for(transform.translate, transform.rotation, transform.scale)
    }

    /// The live gesture's world-space frame.
    ///
    /// `before` is the selection's committed state at press and `pivot` the
    /// mean of its positions — `getSelectionPivot`
    /// (`src/features/scene/selectionTransform.ts`), which the source captures
    /// into `gizmoGroupStartSnapshot` so a run of pointer samples cannot drift
    /// (`SceneCanvas.tsx:3042-3108`). [`live_position`] is the source's
    /// `transform.position`, the point the source's re-centred geometry keeps
    /// there.
    fn begin_gesture(&self, operation: GestureOperation) -> Option<Gesture> {
        let targets = self.scene.action_targets();
        let primary = self.scene.primary()?;
        let mut before = Vec::with_capacity(targets.len());
        let mut pivot = glam::Vec3::ZERO;
        for id in &targets {
            let model = self.scene.model(*id)?;
            let transform = committed_transform(model);
            pivot += live_position(&transform, model);
            before.push((*id, transform));
        }
        pivot /= before.len().max(1) as f32;
        Some(Gesture {
            primary,
            operation,
            pivot,
            before,
        })
    }

    /// Derive every selected model's live transform from the primary's.
    ///
    /// The source's `buildMultiSelectionTransformsFromActive`
    /// (`SceneCanvas.tsx:3042-3108`) is the same shape: the primary's gesture
    /// is the driver, every other selected model follows it.
    ///
    /// * **move** — the primary's translation delta, added to every model. The
    ///   source applies it as `activeAfter.position - activeBefore.position`.
    /// * **rotate** — the group mode (`selectionRotationMode`'s default,
    ///   `page.tsx:1141`): every model turns by the primary's rotation delta and
    ///   its position orbits the pivot, so the selection keeps its shape.
    /// * **scale** — the primary's per-axis scale ratio grows every model and
    ///   its offset from the pivot, and each model's own bottom is held
    ///   (`preserveModelBottom` in `selectionLiftDrop.ts`) so a seated model
    ///   does not sink or lift as it grows.
    fn apply_gesture(&mut self, primary: gizmo::Transform) {
        let Some(gesture) = self.gesture.as_ref() else {
            return;
        };
        let base = gesture
            .before
            .iter()
            .find(|(id, _)| *id == gesture.primary)
            .map(|(_, transform)| *transform);
        let mut drafts: Vec<(gpui_core::scene::ModelId, gizmo::Transform)> =
            Vec::with_capacity(gesture.before.len());
        match (gesture.operation, base) {
            (GestureOperation::Move, Some(base)) => {
                let delta = primary.translate - base.translate;
                for (id, before) in &gesture.before {
                    let mut next = *before;
                    next.translate += delta;
                    drafts.push((*id, next));
                }
            }
            (GestureOperation::Rotate, Some(base)) => {
                let delta = primary.rotation_quat() * base.rotation_quat().inverse();
                for (id, before) in &gesture.before {
                    let mut next = *before;
                    if let Some(model) = self.scene.model(*id) {
                        // `rotateSelectionFromActive`: positions orbit the
                        // pivot; rotations compose with the same delta.
                        let world_center = live_position(before, model);
                        let orbited =
                            gesture.pivot + delta * (world_center - gesture.pivot);
                        next.translate = orbited - model.mesh.center;
                    }
                    let (rx, ry, rz) = (delta * before.rotation_quat())
                        .to_euler(glam::EulerRot::XYZEx);
                    next.rotation = [rx, ry, rz];
                    drafts.push((*id, next));
                }
            }
            (GestureOperation::Scale, Some(base)) => {
                let ratio = glam::Vec3::new(
                    safe_ratio(primary.scale.x, base.scale.x),
                    safe_ratio(primary.scale.y, base.scale.y),
                    safe_ratio(primary.scale.z, base.scale.z),
                );
                for (id, before) in &gesture.before {
                    let Some(model) = self.scene.model(*id) else {
                        continue;
                    };
                    // `scaleSelectionFromActive`: the offset from the pivot
                    // grows with the model, and its own scale multiplies.
                    let world_center = live_position(before, model);
                    let offset = (world_center - gesture.pivot) * ratio;
                    let mut next = *before;
                    next.translate = gesture.pivot + offset - model.mesh.center;
                    next.scale *= ratio;
                    // `preserveModelBottom`.
                    let bottom_before = model.world_bottom_for(
                        before.translate,
                        before.rotation,
                        before.scale,
                    );
                    let bottom_after = model.world_bottom_for(
                        next.translate,
                        next.rotation,
                        next.scale,
                    );
                    next.translate.z += bottom_before - bottom_after;
                    drafts.push((*id, next));
                }
            }
            (_, None) => return,
        }
        self.draft_models = drafts;
    }

    /// Fold a finished gesture's drafts into every model they moved.
    ///
    /// This is the one write a drag performs on the entity: the pointer stream
    /// only ever touched [`Self::draft_models`]. Returns whether there was a
    /// draft to commit, which is also whether the transform panel needs its one
    /// sync.
    pub(crate) fn commit_draft(&mut self) -> bool {
        if self.draft.take().is_none() {
            return false;
        }
        let drafts = std::mem::take(&mut self.draft_models);
        self.gesture = None;
        let mut committed = false;
        for (id, transform) in drafts {
            let Some(model) = self.scene.model_mut(id) else {
                continue;
            };
            model.translate = transform.translate.to_array();
            model.rotation = transform.rotation;
            model.scale = transform.scale.to_array();
            committed = true;
        }
        committed
    }

    /// Throw the draft away, leaving the committed transform as it was. Escape
    /// and a cancelled gesture both end here; nothing was committed, so nothing
    /// needs rebroadcasting.
    pub(crate) fn discard_draft(&mut self) {
        self.draft = None;
        self.draft_models.clear();
        self.gesture = None;
    }

    /// Whether the plate holds a model. The panels' `has_mesh` snapshot and
    /// every "is there anything to act on" gate read this, so a scene with N
    /// models answers exactly as the single-model scene did.
    pub(crate) fn has_mesh(&self) -> bool {
        !self.scene.is_empty()
    }

    /// The primary model's geometry.
    pub(crate) fn primary_mesh(&self) -> Option<&Arc<stl::MeshData>> {
        self.scene.primary_model().map(|model| &model.mesh)
    }

    /// The primary model's display name — the panel's row title.
    pub(crate) fn primary_model_name(&self) -> Option<&str> {
        self.scene
            .primary_model()
            .map(|model| model.name.as_str())
    }

    /// How many models are on the plate.
    pub(crate) fn model_count(&self) -> usize {
        self.scene.models().len()
    }

    /// How many models are selected — the rows' badge and the panel's
    /// multi-selection gates read it.
    pub(crate) fn selection_count(&self) -> usize {
        self.scene.action_targets().len()
    }

    /// Whether anything is selected. This is the source's `activeModelId !==
    /// null` for every gate that only asks "is there something to act on"; the
    /// set itself is read through [`Self::scene`].
    pub(crate) fn has_selection(&self) -> bool {
        !self.scene.selected.is_empty()
    }

    /// A selection click — the source's `selectModel(id, mode)`
    /// (`useSceneCollectionManager.ts:3210-3231`). A stale id (a row for a
    /// model that has since been removed) is ignored. The painter picks up the
    /// new selection from `cx.notify()`; the session records it on the next
    /// save.
    pub(crate) fn select_model(
        &mut self,
        id: gpui_core::scene::ModelId,
        mode: gpui_core::scene::SelectionMode,
    ) {
        self.scene.select(id, mode);
    }

    /// A shift-clicked row's range — `handleModelRangeSelection`
    /// (`page.tsx:5804-5815`), with `ids` taken from the panel's own row order
    /// and `additive` set when ctrl/meta was held with shift.
    pub(crate) fn select_model_range(
        &mut self,
        ids: &[gpui_core::scene::ModelId],
        clicked: gpui_core::scene::ModelId,
        additive: bool,
    ) {
        self.scene.select_range(ids, clicked, additive);
    }

    /// A click past every model: `clearModelSelection`
    /// (`useSceneCollectionManager.ts:3236-3239`). Modifiers do not change it —
    /// the source's pointer-missed path passes no selection mode
    /// (`SceneCanvas.tsx:4997-5000`).
    pub(crate) fn clear_selection(&mut self) {
        self.scene.clear_selection();
    }

    /// The Models panel's eye toggle. Visibility feeds `max_height_world`, so
    /// the layer stack is re-derived exactly as a removal does, and the new
    /// state is written to `session.json` at once — unlike a selection, a
    /// hidden model changes what the slice preview and export cover.
    pub(crate) fn set_model_visible(&mut self, id: gpui_core::scene::ModelId, visible: bool) {
        let Some(model) = self.scene.model_mut(id) else {
            return;
        };
        if model.visible == visible {
            return;
        }
        model.visible = visible;
        self.after_scene_edit();
        self.save_session();
    }

    /// The transform panel's write-back.
    ///
    /// One model selected: its three fields are written directly.
    ///
    /// Several selected: the panel shows the selection's position origin and
    /// the primary's rotation/scale, and an edit is applied to the whole
    /// selection the way the source's three handlers do —
    /// `handlePositionSelectedModels` translates every model by the shift of
    /// the origin, `handleRotateSelectedModels` turns the group about the
    /// selection pivot, `handleScaleSelectedModels` scales the group about it
    /// (`page.tsx:9215-9312`). Only the component that changed is applied: the
    /// panel flushes all three at once.
    pub(crate) fn set_selection_transform(
        &mut self,
        translate: [f32; 3],
        rotation: [f32; 3],
        scale: [f32; 3],
    ) {
        let targets = self.scene.action_targets();
        if targets.len() <= 1 {
            let Some(model) = self.scene.primary_model_mut() else {
                return;
            };
            model.translate = translate;
            model.rotation = rotation;
            model.scale = scale;
            return;
        }

        let position = glam::Vec3::from_array(translate);
        let scale_next = glam::Vec3::from_array(scale);
        let Some(primary_id) = self.scene.primary() else {
            return;
        };
        let Some(primary) = self.scene.model(primary_id) else {
            return;
        };
        let primary_before = committed_transform(primary);
        let mut pivot = glam::Vec3::ZERO;
        // `(id, transform before the edit, the model's position)` — positions
        // are the source's `transform.position`, which is what its handlers
        // shift and orbit (`selectionPosition.ts`, `selectionTransform.ts`).
        let mut entries: Vec<(gpui_core::scene::ModelId, gizmo::Transform, glam::Vec3)> =
            Vec::with_capacity(targets.len());
        for id in &targets {
            let Some(model) = self.scene.model(*id) else {
                continue;
            };
            let before = committed_transform(model);
            let model_position = live_position(&before, model);
            pivot += model_position;
            entries.push((*id, before, model_position));
        }
        if entries.is_empty() {
            return;
        }
        pivot /= entries.len() as f32;

        // `handlePositionSelectedModels`: a typed position is the selection's
        // origin, so every model moves by the shift of that origin.
        let origin_shift = position - self.panel_transform().translate;
        if origin_shift.length() > 1e-6 {
            for (id, _, _) in &entries {
                if let Some(model) = self.scene.model_mut(*id) {
                    let next = glam::Vec3::from_array(model.translate) + origin_shift;
                    model.translate = next.to_array();
                }
            }
        }

        // `rotateSelectionFromActive(..., selectionRotationMode = 'group')`.
        if rotation
            .iter()
            .zip(primary_before.rotation.iter())
            .any(|(next, before)| (next - before).abs() > 1e-6)
        {
            let delta = gpui_core::scene::rotation_quat(rotation)
                * primary_before.rotation_quat().inverse();
            for (id, before, model_position) in &entries {
                let Some(model) = self.scene.model_mut(*id) else {
                    continue;
                };
                let orbited = pivot + delta * (*model_position - pivot);
                model.translate = (orbited - model.mesh.center).to_array();
                let (rx, ry, rz) = (delta * before.rotation_quat())
                    .to_euler(glam::EulerRot::XYZEx);
                model.rotation = [rx, ry, rz];
            }
        }

        // `scaleSelectionFromActive` + `holdSelectionBottoms`.
        if scale
            .iter()
            .zip(primary_before.scale.to_array().iter())
            .any(|(next, before)| (next - before).abs() > 1e-6)
        {
            let ratio = glam::Vec3::new(
                safe_ratio(scale_next.x, primary_before.scale.x),
                safe_ratio(scale_next.y, primary_before.scale.y),
                safe_ratio(scale_next.z, primary_before.scale.z),
            );
            for (id, before, model_position) in &entries {
                let Some(model) = self.scene.model_mut(*id) else {
                    continue;
                };
                let offset = (*model_position - pivot) * ratio;
                let mut next = *before;
                next.translate = pivot + offset - model.mesh.center;
                next.scale *= ratio;
                let bottom_before =
                    model.world_bottom_for(before.translate, before.rotation, before.scale);
                let bottom_after =
                    model.world_bottom_for(next.translate, next.rotation, next.scale);
                next.translate.z += bottom_before - bottom_after;
                model.translate = next.translate.to_array();
                model.scale = next.scale.to_array();
            }
        }
    }

    /// Whether the gizmo answers the pointer.
    ///
    /// `SceneCanvas` mounts `UnifiedGizmo` in Prepare, in the Transform tool,
    /// with a selection — and the source's `enabled` gate is what makes a
    /// handle with nothing to transform impossible. The same three conditions
    /// are the gate here.
    fn gizmo_enabled(&self) -> bool {
        self.mode == Mode::Prepare && self.active_tool == 1 && self.has_selection()
    }

    /// Where the gizmo sits: the mean of the selected models' world centres
    /// through their live transforms, or the position a rotation/scale drag
    /// locked at press (`ScreenSpaceGizmo`'s `lockedDragPosition`).
    ///
    /// `getSelectionGizmoCenter` averages every selected model's position; a
    /// one-model selection lands on that model's own centre, exactly where the
    /// single-model gizmo sat before the set existed.
    fn gizmo_center(&self) -> Option<glam::Vec3> {
        if !self.gizmo_enabled() {
            return None;
        }
        if let Some(center) = self
            .gizmo_drag
            .as_ref()
            .and_then(gizmo::Drag::locked_center)
        {
            return Some(center);
        }
        let targets = self.scene.action_targets();
        let mut center = glam::Vec3::ZERO;
        let mut count = 0;
        for id in targets {
            let Some(model) = self.scene.model(id) else {
                continue;
            };
            center += live_position(&self.live_transform(id), model);
            count += 1;
        }
        (count > 0).then(|| center / count as f32)
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
        let view = gizmo::View::new(
            camera,
            width / height,
            glam::Vec2::new(width, height),
            center,
        );
        let Some(handle) = gizmo::pick(&view, pointer) else {
            return false;
        };
        // The transform panel's `mesh_min_z` and the model matrix's pivot: the
        // primary model's local underside and bbox centre.
        let mesh_bounds = self.scene.primary_model().map(|model| {
            (
                model.mesh.center.z - model.mesh.size[2] * 0.5,
                model.mesh.center.z,
            )
        });
        let Some(drag) = gizmo::Drag::begin(&view, pointer, handle, self.transform(), mesh_bounds)
        else {
            return false;
        };
        let operation = match handle {
            gizmo::Handle::Ring(_) => GestureOperation::Rotate,
            gizmo::Handle::Scale(_) => GestureOperation::Scale,
            gizmo::Handle::Center | gizmo::Handle::Axis(_) => GestureOperation::Move,
        };
        self.gesture = self.begin_gesture(operation);
        // The gesture starts from the committed transform, so the draft is a
        // copy of it until the pointer moves.
        let base = drag.base();
        self.apply_gesture(base);
        self.draft = Some(base);
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
        self.apply_gesture(transform);
        true
    }

    /// The plate-plane grab: drag the selection under the pointer in the world
    /// XY plane. `origin` is the plane hit at press and `base` the primary's
    /// transform then, so a low camera angle cannot turn vertical travel into
    /// extra distance.
    ///
    /// Like the gizmo handles, this writes the draft only; the release commits.
    /// A multi-selection moves as one: the primary's XY delta is applied to
    /// every selected model through [`Self::apply_gesture`], which is the
    /// source's move rule (`buildMultiSelectionTransformsFromActive`).
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
        self.apply_gesture(transform);
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
        self.gesture = self.begin_gesture(GestureOperation::Move);
        self.apply_gesture(base);
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
            let view = gizmo::View::new(
                camera,
                width / height,
                glam::Vec2::new(width, height),
                center,
            );
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

        let stls: Vec<PathBuf> = supported
            .into_iter()
            .filter(|path| file_extension(path) == "stl")
            .collect();
        for stl in stls {
            self.load_path(stl, cx);
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
        // `--scenario`'s driver paces itself on this: a frame it can see here
        // has already been painted, so the hitboxes of the state change before
        // it are in `rendered_frame` by the time it clicks again.
        self.frames = self.frames.wrapping_add(1);

        // `handleModeChange`: an empty scene can only be Prepare.
        if self.scene.is_empty() && self.mode != Mode::Prepare {
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
        // Resolve the scene into the viewport's per-model draw list once per
        // frame. Every model is drawn through its live transform, so a gesture
        // that moves the whole selection moves every one of them; the
        // selection tint follows set membership, not one "active" model.
        let draw_models: Vec<viewport::ModelDraw> = self
            .scene
            .models()
            .iter()
            .filter(|model| model.visible)
            .map(|model| viewport::ModelDraw {
                id: model.id,
                mesh: model.mesh.clone(),
                matrix: self.model_matrix_of(model),
                color: mesh_color(
                    self.scene.is_selected(model.id),
                    self.hovered.get() == Some(model.id),
                ),
            })
            .collect();
        // The pointer handlers raycast the same geometry/matrix pairs the
        // viewport draws, so hover and click cannot disagree with the frame.
        let pick_models = draw_models.clone();
        // `cameraInteractionCycleEnabled = hasModelsOnPlate && ...`: nothing on
        // the plate means the empty scene does not answer the pointer.
        let camera_enabled = Rc::new(Cell::new(self.has_mesh()));
        // Grab: a left drag that lands on a model slides it across the plate.
        // The source moves the model under the pointer in every prepare tool, not
        // only the Move tool.
        let grab_mode = self.has_mesh();
        // `publishClip`'s upper plane: none at layer 0 or at the top layer,
        // parked (infinite) when the cross-section switch is closed.
        let supports = self.supports.clone();
        let clip_z = if self.cross_section && self.has_mesh() {
            let upper = self.layer;
            let max_layer = self.num_layers as f32;
            if upper <= 0.0 || upper >= max_layer {
                f32::INFINITY
            } else {
                // The clip plane is in world space; the scene's lowest visible
                // point is where the layer stack starts.
                self.scene.min_z_world() + upper * LAYER_HEIGHT_MM + CLIP_EPSILON_MM
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
        let smoke_done = self.smoke_done.clone();
        let triangles = self
            .scene
            .models()
            .iter()
            .map(|model| model.mesh.triangles)
            .sum();
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
            let mesh_ref = &draw_models;
            if let Err(error) = state.render(
                &device,
                &queue,
                mesh_ref,
                supports.borrow().as_ref(),
                show_layers,
                clip_z,
                gizmo_draw.as_deref().map(|vertices| vertices.as_slice()),
                &canvas_camera.borrow(),
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
            if smoke && !mesh_ref.is_empty() && !smoke_done.get() {
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
                        if this.has_mesh() && !dragged {
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
            let hover_models = pick_models.clone();
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
                // `hoveredMeshModelId`: the pointer over a model tints that
                // model, and the nearest hit wins.
                let hit = {
                    let size = window.viewport_size();
                    let width = size.width.0.max(1.0);
                    let height = size.height.0.max(1.0);
                    let ndc_x = event.position.x.0 / width * 2.0 - 1.0;
                    let ndc_y = 1.0 - event.position.y.0 / height * 2.0;
                    let (origin, direction) = camera.borrow().ray(width / height, ndc_x, ndc_y);
                    pick_model(&hover_models, origin, direction)
                };
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
            // This listener is on the full-window canvas, which is painted
            // first and therefore sits *below* every panel, the rail and the
            // app bar. GPUI dispatches a press to every hitbox under the
            // pointer unless one stops propagation or occludes
            // (`bite-gp-authoring`'s `hit_test`/`dispatch_mouse_event`), so
            // without those occluders a press on any control also reached this
            // handler and its `None` branch cleared the selection. The layers
            // above now `occlude`; this handler runs only for presses that
            // actually landed on the canvas.
            let camera = camera.clone();
            let hit_models = pick_models.clone();
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

                let ndc_x = pointer.x / width * 2.0 - 1.0;
                let ndc_y = 1.0 - pointer.y / height * 2.0;
                let (origin, direction) = camera.borrow().ray(width / height, ndc_x, ndc_y);
                let hit = pick_model(&hit_models, origin, direction);
                // `StlMesh.tsx:1751-1758`: ctrl/meta is `toggle`, shift is
                // `add`, anything else is `single`. The modifiers are read once
                // so the selection and the grab below agree on the gesture.
                let modifiers = event.modifiers;
                let primary = modifiers.control || modifiers.platform;
                let mode = if primary {
                    gpui_core::scene::SelectionMode::Toggle
                } else if modifiers.shift {
                    gpui_core::scene::SelectionMode::Add
                } else {
                    gpui_core::scene::SelectionMode::Single
                };
                if grab_mode && hit.is_some() {
                    // `selectModel` runs on the press, before any drag: the
                    // source's model pointer-down selects and only then reports
                    // a select-mode drag (`StlMesh.tsx:1751-1768`). A modifier
                    // press is a multi-select gesture, not a move.
                    if primary || modifiers.shift {
                        entity
                            .update(cx, |this, cx| {
                                if let Some(id) = hit {
                                    this.select_model(id, mode);
                                }
                                cx.notify();
                            })
                            .ok();
                        return;
                    }
                    // Same drag cell as the camera: the move handler branches on
                    // the button, so a grab and an orbit cannot both be live.
                    drag.set(Some(MouseButton::Left));
                    last.set((event.position.x.0, event.position.y.0));
                    let grab_camera = camera.clone();
                    entity
                        .update(cx, |this, cx| {
                            // The grab reads `transform()`, so the picked model
                            // has to be primary before the gesture starts.
                            if let Some(id) = hit {
                                this.select_model(id, gpui_core::scene::SelectionMode::Single);
                            }
                            let camera = grab_camera.borrow();
                            this.begin_grab(pointer, width, height, &camera);
                            cx.notify();
                        })
                        .ok();
                    return;
                }
                entity
                    .update(cx, |this, cx| {
                        // The source's `handleSceneModelSelection`: a model
                        // press selects with the modifiers' mode, a press past
                        // every model clears the selection. The pointer-missed
                        // path passes no selection mode, so a modifier-held
                        // empty click clears too (`SceneCanvas.tsx:4997-5000`).
                        match hit {
                            Some(id) => this.select_model(id, mode),
                            None => this.clear_selection(),
                        }
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

        let mode = self.mode;
        let show_settings = self.show_settings;
        let has_models = self.has_mesh();
        let layer = self.layer;

        self.window_width = window.viewport_size().width.0;
        self.window_height = window.viewport_size().height.0;
        if !self.focused {
            window.focus(&self.focus, cx);
            self.focused = true;
        }
        // `--scenario`: hand the steps to the driver once, from the frame whose
        // paint gives it hitboxes to click. The driver waits for that paint
        // itself (see `automation::run`).
        if !self.automation_started {
            if let Some(scenario) = self.scenario.take() {
                self.automation_started = true;
                automation::start(scenario, window, cx);
            }
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
                            if this.show_printer_profiles {
                                if !features::printer_profiles::escape() {
                                    this.show_printer_profiles = false;
                                }
                                changed = true;
                            }
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
                        if this.show_settings
                            || this.show_support_settings
                            || this.show_printer_picker
                            || this.show_printer_profiles
                        {
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
                            if let Some(model) = this.scene.primary_model() {
                                let world = this.model_matrix().transform_point3(model.mesh.center);
                                let radius = model.mesh.radius.max(1.0);
                                let mut camera = this.camera.borrow_mut();
                                camera.focus(world, radius);
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
                        if this.mode == Mode::Prepare && this.has_mesh() && !primary && !alt {
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
                            && this.has_mesh()
                        {
                            this.delete_selected();
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
                            let next = (this.layer + delta).clamp(0.0, this.num_layers as f32);
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
                    self.reopen_error.clone(),
                    entity.clone(),
                ))
            })
            .when(has_models, |this| {
                this.child(
                    div()
                        .absolute()
                        .bottom(px(4.))
                        .left(px(4.))
                        // The chip is a real surface: its presses belong to it,
                        // not to the canvas behind (`occlude`).
                        .occlude()
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
            .when(
                has_models && matches!(mode, Mode::Prepare | Mode::Support),
                |this| {
                    this.child(rail::tool_rail(
                        mode,
                        self.active_tool,
                        self.support_panel,
                        entity.clone(),
                    ))
                },
            )
            .child({
                let mut defs = Vec::new();
                // `PreparePanelStack` / Support's stack carry the Models panel;
                // Export and Printing do not.
                if has_models && matches!(mode, Mode::Prepare | Mode::Support) {
                    defs.push(
                        panels::list::PanelDef::new(
                            "models",
                            "Models",
                            panels::list::PanelKind::Models,
                        )
                        .dock(panels::list::PanelDock::Left)
                        .header_right(panels::content::models_add_button(entity.clone()))
                        .content(panels::content::models_body(
                            self.scene.models(),
                            self.scene.selected(),
                            entity.clone(),
                        )),
                    );
                }
                match mode {
                    Mode::Prepare => {
                        if has_models {
                            let meta: Option<(
                                &'static str,
                                &'static str,
                                panels::list::PanelKind,
                            )> = match self.active_tool {
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
                                    "Split",
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
                            // `PreparePanelStack.tsx:280`: the orientation panel
                            // rides beside the Transform card on the same
                            // `scene.geom && transformMode === 'transform'` gate,
                            // plus the `auto-rotation` experiment.
                            if self.active_tool == 1 {
                                if let Some(def) =
                                    crate::widgets::orientation_panel::orientation_panel_def(
                                        self.scene
                                            .primary_model()
                                            .map(|model| model.name.clone().into()),
                                        entity.clone(),
                                    )
                                {
                                    defs.push(def);
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
                                def.header_right(features::tool_panels::supports_settings_gear(
                                    entity.clone(),
                                ))
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
                // What the frame is about to mount, for `--scenario`'s
                // snapshot: this is the panel stack's own input, so a rail tile
                // that sets a flag but mounts nothing shows up as a missing id.
                self.mounted_panels = defs.iter().map(|def| def.id).collect();
                panels::list::panel_stack(&self.layout, defs, entity.clone())
            })
            // The Z-up gizmo sits 72px from the right edge, 22px from the
            // bottom (its 120px widget is centred on the cube).
            .child(
                div()
                    .absolute()
                    .bottom(px(22.))
                    .right(px(72.))
                    .occlude()
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
                    .occlude()
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
                        .occlude()
                        .child(
                            ui::card().size_full().flex().flex_col().child(
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
            // The printer profiles modal paints under the picker, which the
            // modal opens for its Add flow.
            .child(features::printer_profiles::printer_profiles(
                self.show_printer_profiles,
                self.window_height,
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

/// The launch arguments the shell understands.
///
/// `--scenario <path>` takes a value, so its argument must not be mistaken for
/// the model file the way a bare first non-flag argument is.
struct Cli {
    smoke: bool,
    scenario: Option<PathBuf>,
    /// Every positional path. More than one is the append gate: each loads on
    /// top of the previous, which is exactly what the Models panel's plus and
    /// a multi-file drop do.
    files: Vec<PathBuf>,
}

fn cli_args() -> Cli {
    let mut cli = Cli {
        smoke: false,
        scenario: None,
        files: Vec::new(),
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut index = 0;
    while index < args.len() {
        let argument = args[index].as_str();
        let mut advance = 1;
        match argument.split_once('=') {
            Some(("--scenario", value)) => cli.scenario = Some(PathBuf::from(value)),
            _ => match argument {
                "--smoke" => cli.smoke = true,
                "--scenario" => {
                    if let Some(value) = args.get(index + 1) {
                        cli.scenario = Some(PathBuf::from(value));
                        advance = 2;
                    }
                }
                other if other.starts_with("--") => {}
                other => cli.files.push(PathBuf::from(other)),
            },
        }
        index += advance;
    }
    cli
}

/// Milliseconds since the Unix epoch, the unit `openedAt` uses.
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or(0)
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

/// The nearest model along a world-space ray.
///
/// Each model answers in its own local frame and reports a world distance
/// (`gpui_core::scene::raycast_distance`), so the winner is the nearest hit
/// rather than whichever model happens to come first in import order.
fn pick_model(
    models: &[viewport::ModelDraw],
    origin: glam::Vec3,
    direction: glam::Vec3,
) -> Option<gpui_core::scene::ModelId> {
    models
        .iter()
        .filter_map(|model| {
            gpui_core::scene::raycast_distance(&model.mesh, model.matrix, origin, direction)
                .map(|distance| (model.id, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}

/// Where the pointer's ray meets the plate plane at `z`.
fn plane_hit(camera: &Camera, aspect: f32, ndc_x: f32, ndc_y: f32, z: f32) -> Option<glam::Vec3> {
    let (origin, direction) = camera.ray(aspect, ndc_x, ndc_y);
    if direction.z.abs() < 1e-6 {
        return None;
    }
    let t = (z - origin.z) / direction.z;
    (t > 0.0).then(|| origin + direction * t)
}

/// `blendTintColor` from `src/features/shaders/mesh/tint.ts`: linear lerp from
/// the unselected `#a3a3a3` toward the theme selection `#ec2a77` at 0.70 —
/// and the selected pink the real app renders the active model with.
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
        [
            srgb_to_linear(0xa3),
            srgb_to_linear(0xa3),
            srgb_to_linear(0xa3),
        ]
    }
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
