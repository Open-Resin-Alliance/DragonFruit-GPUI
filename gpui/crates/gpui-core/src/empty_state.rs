//! Empty-scene and model-stats state owned by `DragonFruit`.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui::SharedString;

const MESH_BADGE_DARK: u32 = 0x2a2e3a;
const SCENE_BADGE_DARK: u32 = 0x3a2e2a;

/// One entry of `scene.recentOpenedFiles`.
#[derive(Clone, Debug)]
pub struct RecentOpenedFile {
    pub id: SharedString,
    pub name: SharedString,
    pub kind: RecentFileKind,
    /// `sizeBytes`; `None` renders the source's `cached` fallback.
    pub size_bytes: Option<u64>,
    /// `openedAt`, milliseconds since the Unix epoch.
    pub opened_at_ms: i64,
}

/// `entry.kind` — the badge on a recent-file row.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RecentFileKind {
    Mesh,
    Scene,
}

impl RecentFileKind {
    pub fn label(self) -> &'static str {
        match self {
            RecentFileKind::Mesh => "MESH",
            RecentFileKind::Scene => "SCENE",
        }
    }

    pub fn accent(self) -> u32 {
        match self {
            RecentFileKind::Mesh => MESH_BADGE_DARK,
            RecentFileKind::Scene => SCENE_BADGE_DARK,
        }
    }
}

/// `TAGLINES` from `EmptySceneState.tsx`.
pub const TAGLINES: [&str; 34] = [
    "Ready for your next adventure?",
    "The build plate awaits.",
    "What are we printing today?",
    "Time to make something real.",
    "Every great print starts with an empty scene.",
    "Nothing to see here. Yet.",
    "Your models called. They want to be sliced.",
    "Suspiciously empty in here.",
    "This scene is aggressively unoccupied.",
    "Zero polygons. Infinite potential.",
    "The resin is ready. Are you?",
    "Currently displaying nothing at maximum fidelity.",
    "Idle hands do no slicing.",
    "The build volume misses you.",
    "Drag a file in. The platform is judging you.",
    "Do or do not. There is no try... oh wait, just click Load mesh.",
    "In space, no one can hear you slice.",
    "You shall not pass... until you load a model.",
    "I am the one who slices.",
    "With great resin comes great responsibility.",
    "Why so empty?",
    "We're gonna need a bigger build plate.",
    "One does not simply import a mesh... or actually, you just click a button.",
    "I'll be back. (Drop a file and I really will be.)",
    "It's a trap! ...just kidding, drop your STL.",
    "The spice must flow. The resin must cure.",
    "Elementary, my dear user — load a model.",
    "Roads? Where we're printing, we don't need roads.",
    "Waiting for first layer adhesion... to reality.",
    "No supports needed for an empty scene.",
    "Layer 0 of 0. Living dangerously.",
    "Anti-aliasing: nothing to anti-alias.",
    "Join the Open Resin Alliance!",
    "Not all slicers are created equal. This one's free!",
];

/// The source picks one tagline per mount (`useState(() => TAGLINES[random])`).
/// The caller calls this once and passes the result on the state.
pub fn pick_tagline() -> &'static str {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.subsec_nanos() as usize)
        .unwrap_or(0);
    TAGLINES[nanos % TAGLINES.len()]
}

/// `EmptySceneStateProps` plus the build identity the source reads from
/// `process.env`.
#[derive(Clone)]
pub struct EmptySceneState {
    /// `isLoading` — the blocking import overlay's inline card.
    pub loading: bool,
    pub loading_label: Option<SharedString>,
    pub loading_detail: Option<SharedString>,
    /// `showFirstTimeOnboarding && !isLoading`.
    pub show_first_time_onboarding: bool,
    pub recent_opened_files: Vec<RecentOpenedFile>,
    /// The tagline chosen for this mount (see [`pick_tagline`]).
    pub tagline: &'static str,
    /// `NEXT_PUBLIC_APP_VERSION`.
    pub version: SharedString,
    /// `NEXT_PUBLIC_BUILD_CHANNEL`, lowercased.
    pub build_channel: SharedString,
    /// `GIT_REF @ GIT_COMMIT`, when the build carries one.
    pub build_label: Option<SharedString>,
    /// `IS_BETA_BUILD`.
    pub beta: bool,
    /// The canvas drag state (`isPrepareDragActive`).
    pub drop_active: bool,
    /// The canvas drag is over an unsupported file (`isPrepareDragUnsupported`).
    pub drop_unsupported: bool,
    /// `copied` — the version chip's flash. Written by this widget.
    pub copied: Rc<Cell<bool>>,
}

impl Default for EmptySceneState {
    fn default() -> Self {
        Self {
            loading: false,
            loading_label: None,
            loading_detail: None,
            show_first_time_onboarding: false,
            recent_opened_files: Vec::new(),
            tagline: TAGLINES[0],
            version: SharedString::default(),
            build_channel: SharedString::default(),
            build_label: None,
            beta: false,
            drop_active: false,
            drop_unsupported: false,
            copied: Rc::new(Cell::new(false)),
        }
    }
}

/// The model the card's back face describes.
#[derive(Clone, Debug)]
pub struct ModelStatsModel {
    pub name: SharedString,
    /// `model.fileSizeBytes`.
    pub file_size_bytes: Option<u64>,
    /// `model.polygonCount`.
    pub polygon_count: usize,
    /// `meshDefects.nativeRepairReport.post.component_count`.
    pub shells: Option<u32>,
    /// `heightMm`.
    pub height_mm: f32,
    /// `meshDefects.repairedFloats`, present only when `hasDefects`.
    pub defect_errors: Option<u32>,
}

/// `ModelStatsCardProps` plus the profile and slicer values the source reads
/// from its stores.
#[derive(Clone)]
pub struct ModelStatsCardState {
    pub model: Option<ModelStatsModel>,
    /// `estimatedSlicerLayerCount`.
    pub num_layers: u32,
    /// `connectedHostName || activePrinterProfile?.name || "No printer connected"`.
    pub header: SharedString,
    /// `networkConnection.connected` — the header's green tint.
    pub header_connected: bool,
    /// The printer is offline — the header's red tint.
    pub header_offline: bool,
    pub printer_name: Option<SharedString>,
    pub material_name: Option<SharedString>,
    /// `effectiveLayerHeightMm`.
    pub layer_height_mm: Option<f32>,
    /// `effectiveNormalExposureSec`.
    pub normal_exposure_s: Option<f32>,
    /// `effectiveBottomExposureSec`.
    pub bottom_exposure_s: Option<f32>,
    /// `estimatedPrintTimeLabelOverride`.
    pub estimated_print_time: Option<SharedString>,
    /// `estimatedResinLabelOverride`.
    pub estimated_resin: Option<SharedString>,
    /// `estimatedResinTotalLabel` — rendered only when there is more than one
    /// plate.
    pub estimated_resin_total: Option<SharedString>,
    /// `isFlipped` — which face is showing. Written by this widget.
    pub flipped: Rc<Cell<bool>>,
}

impl Default for ModelStatsCardState {
    fn default() -> Self {
        Self {
            model: None,
            num_layers: 0,
            header: "No printer connected".into(),
            header_connected: false,
            header_offline: false,
            printer_name: None,
            material_name: None,
            layer_height_mm: None,
            normal_exposure_s: None,
            bottom_exposure_s: None,
            estimated_print_time: None,
            estimated_resin: None,
            estimated_resin_total: None,
            flipped: Rc::new(Cell::new(false)),
        }
    }
}

