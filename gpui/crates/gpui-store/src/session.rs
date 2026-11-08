//! `session.json` — what the app carries across launches: the recent-file
//! list, the panel layout and dock widths, the last file the session opened and
//! the active Prepare tool.
//!
//! The panel layout mirrors the document `gpui-core/src/panel_layout.rs`
//! serialises (`version: 6`, `panels`, `docks`), restated here as plain data so
//! the store crate does not have to depend on gpui. The camera field is a
//! legacy key that still loads and is no longer written (see [`Session`]); the
//! tool index is `app.rs`'s `active_tool` (0 = select).

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Version this crate writes.
pub const SESSION_SCHEMA_VERSION: u32 = 1;

/// The panel-layout document version `panel_layout.rs` writes.
pub const LAYOUT_VERSION: u32 = 6;

/// `DEFAULT_PANEL_WIDTH` in `panel_layout.rs`.
pub const DEFAULT_PANEL_WIDTH: f32 = 320.0;

/// `RECENT_OPENED_FILES_LIMIT` in `useSceneCollectionManager.ts`: the list is
/// kept at ten entries even though the empty scene's panel shows eight.
pub const RECENT_FILES_LIMIT: usize = 10;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    #[serde(default)]
    pub schema_version: u32,
    /// The file the user had open. Absent on a first run and after a failed
    /// load; a path that no longer exists is the caller's to check.
    #[serde(default)]
    pub last_file: Option<PathBuf>,
    /// A camera pose from a document written before the port stopped restoring
    /// it. The source never persists or restores the view — a launch has no
    /// model and starts at the home pose — so this build accepts the key and
    /// ignores it.
    #[serde(default)]
    pub camera: Option<CameraPose>,
    /// The model's Euler rotation in radians (XYZ order), the same value
    /// `Transform::rotation` carries in `gizmo/types.rs` and
    /// `TransformState::rotation` shows in degrees. Kept next to the camera so
    /// a reopened file keeps the orientation the user left it at; a session file
    /// from before this field existed has no key and loads as the identity.
    #[serde(default)]
    pub rotation: [f32; 3],
    /// `DragonFruit::active_tool`, the Prepare tool index.
    #[serde(default)]
    pub active_tool: u32,
    /// The empty scene's recent-file list, most recent first. The source keeps
    /// this in `localStorage` under `app-recent-opened-files`; the port keeps it
    /// beside the session so one document carries everything a launch reads.
    #[serde(default)]
    pub recent_files: Vec<RecentFile>,
    #[serde(default)]
    pub panel_layout: SessionLayout,
}

/// `trackRecentOpenedFiles`: one entry per path, newest first, trimmed to
/// [`RECENT_FILES_LIMIT`]. A path that no longer exists is kept — the panel is
/// what reports it missing.
pub fn push_recent(files: &mut Vec<RecentFile>, entry: RecentFile) {
    files.retain(|item| item.path != entry.path);
    files.insert(0, entry);
    files.truncate(RECENT_FILES_LIMIT);
}

impl Default for Session {
    fn default() -> Self {
        Self {
            schema_version: SESSION_SCHEMA_VERSION,
            last_file: None,
            camera: None,
            rotation: [0.0; 3],
            active_tool: 0,
            recent_files: Vec::new(),
            panel_layout: SessionLayout::default(),
        }
    }
}

/// The placements and dock widths from `panel_layout.rs`'s saved document.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionLayout {
    /// `panel_layout.rs` stores `6`; an older layout still loads because every
    /// field is defaulted.
    #[serde(default = "default_layout_version")]
    pub version: u32,
    /// Panel id → placement. Stale ids are kept: the source keeps placements
    /// for panels that are not visible this session.
    #[serde(default)]
    pub panels: BTreeMap<String, PanelPlacement>,
    /// A dock width of `None` means "size the dock to its panels".
    #[serde(default)]
    pub left_dock_width: Option<f32>,
    #[serde(default)]
    pub right_dock_width: Option<f32>,
}

fn default_layout_version() -> u32 {
    LAYOUT_VERSION
}

impl Default for SessionLayout {
    fn default() -> Self {
        Self {
            version: LAYOUT_VERSION,
            panels: BTreeMap::new(),
            left_dock_width: None,
            right_dock_width: None,
        }
    }
}

/// `panel_layout.rs`'s `PanelPlacement`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelPlacement {
    #[serde(default)]
    pub dock: PanelDock,
    #[serde(default)]
    pub order: u32,
    #[serde(default = "default_panel_width")]
    pub width: f32,
    /// Present only for floating panels, in stack coordinates.
    #[serde(default)]
    pub float: Option<FloatPoint>,
}

fn default_panel_width() -> f32 {
    DEFAULT_PANEL_WIDTH
}

impl Default for PanelPlacement {
    fn default() -> Self {
        Self {
            dock: PanelDock::default(),
            order: 0,
            width: DEFAULT_PANEL_WIDTH,
            float: None,
        }
    }
}

/// `panel_layout.rs`'s `PanelDock`; a panel with no stored placement floats.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PanelDock {
    Left,
    Right,
    #[default]
    Float,
}

/// A floating panel's top-left in stack coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FloatPoint {
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
}

/// The orbit camera's pose — `gpui-core/src/camera.rs`'s `Camera`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraPose {
    #[serde(default = "default_yaw")]
    pub yaw: f32,
    #[serde(default = "default_pitch")]
    pub pitch: f32,
    #[serde(default = "default_distance")]
    pub distance: f32,
    #[serde(default)]
    pub target: [f32; 3],
    /// Vertical field of view, radians (`Camera::fov_y`, 45° in `fit`).
    #[serde(default = "default_fov_y")]
    pub fov_y: f32,
    #[serde(default = "default_true")]
    pub orthographic: bool,
}

fn default_yaw() -> f32 {
    // `Camera::parked`'s offset (-220, -220, 260): atan2(-220, -220).
    -std::f32::consts::FRAC_PI_4 * 3.0
}
fn default_pitch() -> f32 {
    // asin(260 / |(-220, -220, 260)|).
    (260.0f32 / 405.524_36).asin()
}
fn default_distance() -> f32 {
    // |(-220, -220, 260)|.
    405.524_36
}
fn default_fov_y() -> f32 {
    45.0f32.to_radians()
}
fn default_true() -> bool {
    true
}

impl Default for CameraPose {
    fn default() -> Self {
        Self {
            yaw: default_yaw(),
            pitch: default_pitch(),
            distance: default_distance(),
            target: [0.0, 0.0, 0.0],
            fov_y: default_fov_y(),
            orthographic: true,
        }
    }
}

/// `RecentOpenedFileKind` in `useSceneCollectionManager.ts`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecentFileKind {
    #[default]
    Mesh,
    Scene,
}

/// One `RecentOpenedFileEntry`. The source's `id` is a uuid that keys the
/// IndexedDB blob cache; the port has no cache, so `path` is the identity and
/// the dedup key.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentFile {
    #[serde(default)]
    pub path: PathBuf,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub kind: RecentFileKind,
    /// `sizeBytes`; `None` renders the panel's `cached` fallback.
    #[serde(default)]
    pub size_bytes: Option<u64>,
    /// `openedAt`, milliseconds since the Unix epoch.
    #[serde(default, rename = "openedAt")]
    pub opened_at_ms: i64,
}
