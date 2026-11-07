//! Port of `src/components/settings/SettingsModal.tsx` and every tab body it
//! mounts: General, Camera, Workspace, Multi-Plate, Mesh, Slicing
//! (`PerformanceSettingsTab`), 3D Mouse, UI & Theme, Hotkeys, Plugins,
//! Experiments, Scene Autosave, Backups (`LocalBackupsSettingsTab`), UVTools,
//! Logging, Updates (`UpdatesSettingsTab` + `UpdateCheckerSection`) and About.
//!
//! Every size, colour and label traces to a class string or inline style in the
//! React source; Tailwind radii are the v4 scale (`rounded-sm` 4px, `-md` 6px,
//! `-lg` 8px, `-xl` 12px, `-2xl` 16px).
//!
//! PORT DEVIATIONS (each one forced by the GPUI slice; nothing is silently
//! simplified):
//! - The draft values live in one process-wide [`SettingsState`] because
//!   `DragonFruit` owns only `settings_tab`/`show_settings`. `Apply` and
//!   `Cancel` close the modal; persistence belongs to the parent.
//! - `SelectDropdown` is the real component now: the trigger opens a menu, the
//!   chosen row writes the draft and closes it. GPUI has no portal, so the menu
//!   renders as an absolute child under the trigger (6px below, full trigger
//!   width, 320px cap) and therefore scrolls with the panel; a click outside
//!   does not dismiss it — selecting a row or re-clicking the trigger does.
//! - `NumberInput` / `ScrollableNumberField` show the value and the source's
//!   steppers; typing (and wheel stepping) needs a text-input entity this slice
//!   does not have, so the box is not editable.
//! - Range inputs render as the `.ui-range` track with the accent fill and a
//!   knob at the current value; dragging is not wired in this slice.
//! - The shader preview canvas, `react-colorful` picker and colour-swatch
//!   inputs render as their containers filled with the current colour.
//! - Theme import/export, custom theme profiles and the nested confirm dialogs
//!   are not ported; the built-in preset row is what the source shows by
//!   default. The Slicing tab's temp-file cleanup buttons render but are not
//!   wired to the native slicer bridge.
//! - The active sidebar tab's 1px inset ring (`boxShadow`) has no GPUI
//!   equivalent and is omitted; its border and fill are kept.
//! - `MODEL_EDGE_OVERLAY_ENABLED` is `false` in the source, so the "Higher
//!   Contrast Model Edges" section is absent, exactly as in the app.
//! - No Tauri bridge exists in this slice. The Workspace tab therefore never
//!   sees an active printer profile (`isBuildVolumeManagedByPrinter` false), the
//!   3D Mouse tab never sees navlib active, and the bridge-backed sections of
//!   Scene Autosave, Backups, UVTools, Logging, Updates and Plugins render the
//!   source's own empty/never states with inert actions instead of inventing
//!   data.
//! - Hotkey rows show the source defaults; the recording overlay renders, but
//!   capturing a key needs the hotkey store's global dispatch, so the binding
//!   only changes once that store exists.
//! - Sponsors: remote avatars are not fetched, so every sponsor renders the
//!   source's initials monogram; the 34s marquee has no GPUI equivalent and the
//!   row is static. The tier ring is a conic gradient at the source and renders
//!   as the tier's solid colour.
//! - About reads the GPUI build's version/channel (there is no
//!   `NEXT_PUBLIC_*`), and the build-info chip cannot reach a clipboard, so it
//!   always shows the un-copied `Version …` state. The branding images
//!   (`text_logo.svg`, the ORA logo, `simple_icon.svg`) are not in the GPUI
//!   asset set: the wordmark and the ORA mark render as text and the trailing
//!   app icon is omitted.
//! - The Logging tab's `Copy path` tile cannot reach a clipboard either, so it
//!   is inert; the log path itself is the source's own non-desktop placeholder
//!   and the viewer shows its "desktop build" line.
//! - The Backups delete/restore confirmations can only open from a history row
//!   and the slice lists none; their copy interpolates the snapshot id where
//!   the source formats it as a locale timestamp (no date formatter here).
//! - Icon slugs the generated set lacks are mapped to the nearest available
//!   glyph: `Layers3`→`layers`, `Filter`→`funnel`, `UploadCloud`→`cloud-upload`,
//!   `AlertTriangle`→`triangle-alert`, `CheckCircle2`→`circle-check`,
//!   `RefreshCw`→`refresh-cw`, `RefreshCcw`→`refresh-ccw`.
//! - Backdrop blur, CSS transitions, text/box shadows and `z-index` stacking
//!   have no GPUI equivalent; overlays are painted after the card instead.
//!   GPUI gradients take two stops, so the source's three-or-more-stop
//!   gradients (the About badge, the credits divider) keep their first and last.

#![allow(dead_code)]

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{LazyLock, Mutex};

use gpui::{
    App, Div, FontWeight, IntoElement, MouseButton, SharedString, Stateful, WeakEntity, div,
    linear_color_stop, linear_gradient, prelude::*, px, relative, rgb, rgba,
};

use gpui_core::icons::icon;
use gpui_core::theme::*;
use gpui_ui::ui::{
    IconButtonSize, IconButtonVariant, IconTone, SelectOption, icon_button, icon_button_toned,
    select_dropdown, spinner,
};
use crate::DragonFruit;

// `w-72`, `max-w-[72rem]`, `p-5`, `w-36` (the dropdown trigger width).
const SIDEBAR_WIDTH: f32 = 288.0;
const CARD_MAX_WIDTH: f32 = 1152.0;
const BACKDROP_PADDING: f32 = 20.0;
const CONTROL_WIDTH: f32 = 144.0;

// `DEFAULT_*` constants from the React sources.
const DEFAULT_MESH_COLOR: &str = "#a3a3a3";
const DEFAULT_HEATMAP_MIN_ANGLE: f32 = 0.0;
const DEFAULT_HEATMAP_MAX_ANGLE: f32 = 45.0;
const DEFAULT_AMBIENT_INTENSITY: f32 = 0.28;
const DEFAULT_DIRECTIONAL_INTENSITY: f32 = 1.12;
const DEFAULT_MATERIAL_ROUGHNESS: f32 = 0.55;
const DEFAULT_XRAY_OPACITY: f32 = 0.25;
const DEFAULT_HOVER_TINT_STRENGTH: f32 = 0.5;
const DEFAULT_SELECTED_TINT_STRENGTH: f32 = 0.70;
const DEFAULT_HEATMAP_COLORS: [&str; 5] = ["#E55959", "#E5A559", "#D9D959", "#73D973", "#666666"];
const DEFAULT_FOV: f32 = 50.0;
const FOV_MIN: f32 = 15.0;
const FOV_MAX: f32 = 120.0;
const DEFAULT_UI_SCALE: f32 = 1.0;
const MIN_UI_SCALE: f32 = 0.25;
const MAX_UI_SCALE: f32 = 4.0;
const UI_SCALE_PRESETS: [f32; 6] = [0.75, 0.9, 1.0, 1.1, 1.25, 1.5];
/// `UI_SCALE_PRESETS.map(...)` plus the trailing `Custom` entry.
const UI_SCALE_OPTIONS: [(&str, &str); 7] = [
    ("0.75", "75%"),
    ("0.9", "90%"),
    ("1", "100%"),
    ("1.1", "110%"),
    ("1.25", "125%"),
    ("1.5", "150%"),
    ("custom", "Custom"),
];

const LANGUAGE_LABELS: [&str; 6] = ["English", "Español", "Deutsch", "Français", "Slovenčina", "Čeština"];
/// `SUPPORTED_LOCALES` — the `SelectDropdown` values behind the labels.
const LANGUAGE_VALUES: [&str; 6] = ["en", "es", "de", "fr", "sk", "cs"];
const LANGUAGE_OPTIONS: [(&str, &str); 6] = [
    ("en", "English"),
    ("es", "Español"),
    ("de", "Deutsch"),
    ("fr", "Français"),
    ("sk", "Slovenčina"),
    ("cs", "Čeština"),
];
const TOOL_LAYOUT_LABELS: [&str; 2] = ["Top", "Left"];
const RAFT_LABELS: [&str; 3] = ["Off", "Line", "Solid"];
const FEEL_LABELS: [&str; 4] = ["Raw", "Precise", "Balanced", "Fast"];
const TRACKPAD_LABELS: [&str; 3] = ["Off", "Pan", "Orbit"];
const SHADER_LABELS: [&str; 6] = ["Standard", "Overhangs", "Mesh", "Wireframe", "X-Ray", "Normals"];
const MATCAP_LABELS: [&str; 3] = ["Neutral", "Cool", "Warm"];
const PREVIEW_MODELS: [&str; 3] = ["Cube", "Sphere", "Knot"];
const THEME_PRESETS: [&str; 4] = ["DragonFruit Dark", "DragonFruit Light", "Concepts 3D", "Atlas 3DSS"];
const SCHEME_LABELS: [&str; 2] = ["Dark", "Light"];
const PNG_MODE_LABELS: [&str; 3] = ["Auto", "Off", "On"];

// The `(value, label)` pairs behind each `SelectDropdown` / `Select`.
const TOOL_LAYOUT_OPTIONS: [(&str, &str); 2] = [("horizontal", "Top"), ("vertical", "Left")];
const RAFT_OPTIONS: [(&str, &str); 3] = [("off", "Off"), ("line", "Line"), ("solid", "Solid")];
const FEEL_VALUES: [&str; 4] = ["raw", "precise", "balanced", "fast"];
const FEEL_OPTIONS: [(&str, &str); 4] = [
    ("raw", "Raw"),
    ("precise", "Precise"),
    ("balanced", "Balanced"),
    ("fast", "Fast"),
];
const TRACKPAD_VALUES: [&str; 3] = ["off", "pan", "orbit"];
const TRACKPAD_OPTIONS: [(&str, &str); 3] = [("off", "Off"), ("pan", "Pan"), ("orbit", "Orbit")];
const SHADER_VALUES: [&str; 6] = [
    "soft_clay",
    "overhang_heatmap",
    "opaque_wire_mesh",
    "wireframe",
    "xray",
    "normal_debug",
];
const SHADER_OPTIONS: [(&str, &str); 6] = [
    ("soft_clay", "Standard"),
    ("overhang_heatmap", "Overhangs"),
    ("opaque_wire_mesh", "Mesh"),
    ("wireframe", "Wireframe"),
    ("xray", "X-Ray"),
    ("normal_debug", "Normals"),
];
const PREVIEW_MODEL_VALUES: [&str; 3] = ["cube", "sphere", "knot"];
const PREVIEW_MODEL_OPTIONS: [(&str, &str); 3] =
    [("cube", "Cube"), ("sphere", "Sphere"), ("knot", "Knot")];
const THEME_PRESET_VALUES: [&str; 4] = [
    "dragonfruit-dark",
    "dragonfruit-light",
    "concepts-3d",
    "atlas-3dss",
];
const THEME_PRESET_OPTIONS: [(&str, &str); 4] = [
    ("dragonfruit-dark", "DragonFruit Dark"),
    ("dragonfruit-light", "DragonFruit Light"),
    ("concepts-3d", "Concepts 3D"),
    ("atlas-3dss", "Atlas 3DSS"),
];
const SCHEME_VALUES: [&str; 2] = ["dark", "light"];
const SCHEME_OPTIONS: [(&str, &str); 2] = [("dark", "Dark"), ("light", "Light")];
/// `SpaceMouseSettings['pivotMode']` — the native `<Select>` in the 3D Mouse tab.
const PIVOT_OPTIONS: [(&str, &str); 2] = [("auto", "Auto"), ("camera-ray", "Camera Raycast")];
const WORKSPACES: [(&str, &str); 4] = [
    ("Prepare", "Model prep and transform workflows"),
    ("Analysis", "Island diagnostics and inspection tools"),
    ("Support", "Support placement and editing workspace"),
    ("Export", "Final output and export pipeline"),
];

/// `#ffffff` — the base of the `color-mix(..., white N%)` recipes.
const WHITE: u32 = 0xffffff;
/// `#000000` — the base of the `color-mix(..., black N%)` recipes.
const BLACK: u32 = 0x000000;

// `view3dPreferences.ts` / `DEFAULT_VIEW3D_SETTINGS`.
const DEFAULT_VIEW3D_WIDTH_MM: f32 = 218.0;
const DEFAULT_VIEW3D_DEPTH_MM: f32 = 123.0;
const DEFAULT_VIEW3D_MAX_Z_MM: f32 = 250.0;
const DEFAULT_VIEW3D_SCREEN_WIDTH_PX: f32 = 2560.0;
const DEFAULT_VIEW3D_SCREEN_HEIGHT_PX: f32 = 1440.0;

// `plateCascade.ts`.
const DEFAULT_FIXED_PLATE_COLUMNS: f32 = 3.0;
const MAX_FIXED_PLATE_COLUMNS: f32 = 12.0;

// `spacemousePreferences.ts` / `DEFAULT_SPACEMOUSE_SETTINGS`.
const DEFAULT_SPACEMOUSE_SENSITIVITY: f32 = 2.3;
const DEFAULT_SPACEMOUSE_DEADZONE: f32 = 0.03;

// `sceneAutosavePreferences.ts` / `DEFAULT_SCENE_AUTOSAVE_SETTINGS`.
const DEFAULT_AUTOSAVE_DEBOUNCE_MS: f32 = 45_000.0;
const DEFAULT_AUTOSAVE_COOLDOWN_MS: f32 = 180_000.0;
const DEFAULT_AUTOSAVE_CAP_MS: f32 = 300_000.0;

// `LocalBackupsSettingsTab.tsx` defaults.
const DEFAULT_AUTO_SYNC_MINUTES: f32 = 15.0;

// `LoggingSettingsTab.tsx`.
const LOG_LEVEL_OPTIONS: [(&str, &str, &str, u32); 5] = [
    ("error", "Error", "Only critical failures", 0xf87171),
    ("warn", "Warn", "Warnings and errors", 0xfb923c),
    ("info", "Info", "Normal operational messages (recommended)", 0x4ade80),
    ("debug", "Debug", "Verbose internal state", 0x60a5fa),
    ("trace", "Trace", "Maximum verbosity — may impact performance", 0xa78bfa),
];

/// `SettingsModal.tsx` — `DRAGONFRUIT_VERSION` / `DRAGONFRUIT_BUILD_CHANNEL`.
const DRAGONFRUIT_VERSION: &str = env!("CARGO_PKG_VERSION");
const DRAGONFRUIT_BUILD_CHANNEL: &str = "mainline";
const DRAGONFRUIT_REPO_URL: &str = "https://github.com/Open-Resin-Alliance/DragonFruit";

/// `contributors.json`, in file order; the About tab sorts by tone then name.
const CONTRIBUTORS: [(&str, &str, &str); 22] = [
    ("Ty Mansfield", "Tableflipfoundry", "founder"),
    ("Paul Skapczyk", "PaulGD03", "founder"),
    ("William Patton", "PattonWebz", "secondary"),
    ("Magistr", "umag", "secondary"),
    ("Tim", "tslater2006", "secondary"),
    ("Ada Phillips", "ada-phillips", "secondary"),
    ("Sin", "SinXIV", "accent"),
    ("Aaron Baca", "aaron1138", "secondary"),
    ("David Martínez Moreno", "Jo-Con-El", "accent"),
    ("craigsturgess", "csreades", "secondary"),
    ("pkElectronics", "pkElectronics", "secondary"),
    ("DevMiner", "TheDevMinerTV", "secondary"),
    ("Columbus Labs", "ColumbusLabs", "secondary"),
    ("Aysajan Eziz", "AysajanE", "secondary"),
    ("Norman", "Barasatii", "secondary"),
    ("Jonathan", "mgalore", "secondary"),
    ("AJRubenstein", "AJRubenstein", "secondary"),
    ("CalvinSchulze", "CalvinSchulze", "secondary"),
    ("tforik55", "tforik55", "secondary"),
    ("KPhillips152", "KPhillips152", "secondary"),
    ("LuisTNH", "LuisTNH", "secondary"),
    ("MalibThalion", "MalibThalion", "secondary"),
];

/// `sponsors.json` — the baked-in fallback the carousel paints before (and
/// without) a live fetch.
const SPONSORS: [(&str, Option<&str>, f32); 18] = [
    ("KCast", Some("https://opencollective.com/incognito-511b471d"), 900.0),
    ("Siraya Tech", Some("https://opencollective.com/siraya-tech"), 600.0),
    ("Atlas 3DSS", Some("https://opencollective.com/atlas-3dss"), 300.0),
    ("ben", Some("https://opencollective.com/guest-29653113"), 190.0),
    ("Gobbledygook Games", Some("https://opencollective.com/gggames"), 115.0),
    ("WickedGrey", Some("https://opencollective.com/guest-e7584a76"), 100.0),
    ("jacobi", Some("https://opencollective.com/guest-29b90773"), 50.0),
    ("Concepts 3D", Some("https://opencollective.com/concepts3d"), 45.0),
    ("Kryz Slaughter", Some("https://opencollective.com/kryz-slaughter"), 45.0),
    ("Rob Skinner", Some("https://opencollective.com/rob-skinner"), 45.0),
    ("Runklestiltskin", Some("https://opencollective.com/guest-4717e1d9"), 30.0),
    ("DCent", Some("https://opencollective.com/guest-c5210655"), 20.0),
    ("foors", Some("https://opencollective.com/guest-b4a05279"), 20.0),
    ("Cunabula", Some("https://opencollective.com/cunabula"), 15.0),
    ("Viznoman", Some("https://opencollective.com/viznoman"), 15.0),
    ("Tom", Some("https://opencollective.com/tomb94"), 10.0),
    ("MachineMenace", Some("https://opencollective.com/machinemenace"), 5.0),
    ("Sin", Some("https://opencollective.com/sinxv"), 1.0),
];

/// `EXPERIMENT_DEFINITIONS` from `src/config/experiments.json`.
const EXPERIMENTS: [(&str, &str, &str, bool); 3] = [
    (
        "chitubox-import",
        "Chitubox File Import",
        "Enable importing .chitubox project files. Lacks comprehensive testing and may not work with all files.",
        false,
    ),
    (
        "auto-supports",
        "Auto Supports",
        "Automatic support generation from model analysis. Early access.",
        true,
    ),
    (
        "auto-rotation",
        "Auto Orientation",
        "Orientation suggestions that minimize support contact area. Early access.",
        true,
    ),
];

/// `DEFAULT_KEYBINDINGS` — `(category, action, key, modifier, description)`.
/// The categories the Hotkeys tab groups over.
const HOTKEY_DEFAULTS: [(&str, &str, &str, Option<&str>, &str); 33] = [
    // SUPPORTS
    ("SUPPORTS", "JOINT_CREATION", "j", None, "Hold to enter Joint Creation Mode"),
    ("SUPPORTS", "CURVE_MODE", "c", None, "Hold to create curved segment when moving joint"),
    ("SUPPORTS", "BRANCH_PLACEMENT", "Alt", None, "Hold to enter Branch Placement Mode"),
    ("SUPPORTS", "LEAF_PLACEMENT", "Alt", Some("ctrl"), "Hold to enter Leaf Placement Mode"),
    ("SUPPORTS", "KICKSTAND_PLACEMENT", "Control", None, "Hold to enter Kickstand Placement Mode"),
    ("SUPPORTS", "TEMP_SPOTLIGHT_HOLD", "p", None, "Hold to temporarily enable Spotlight highlight in Support mode"),
    ("SUPPORTS", "FORCE_PLACE_SUPPORT", "q", None, "Hold to force placing support"),
    ("SUPPORTS", "AUTO_BRACING", "g", None, "Generate auto bracing while the Bracing settings page is open"),
    ("SUPPORTS", "SPROUTED_PARENTING_LOCK", "w", None, "Hold to enter Sprouted Leaf Fanning Mode"),
    ("SUPPORTS", "TOGGLE_CONE_KNOT_SELECTION", "e", None, "Press to toggle selection between support cone and parent base knot"),
    // CAMERA
    ("CAMERA", "FOCUS_PICK", "f", None, "Press to focus hovered point on selected model, else snap to best visible model"),
    ("CAMERA", "TOGGLE_PROJECTION", "o", None, "Toggle camera projection (Orthographic / Perspective)"),
    ("CAMERA", "INTERIOR_VIEW", "x", None, "Toggle Interior View Mode"),
    // CANVAS
    ("CANVAS", "TOOL_SELECT", "q", None, "Switch canvas tool to Drag"),
    ("CANVAS", "TOOL_MODIFY", "m", None, "Switch canvas tool to Transform"),
    ("CANVAS", "TOOL_MODIFY_LOCAL", "m", Some("shift"), "Switch canvas tool to Transform with the gizmo in the model's own axes"),
    ("CANVAS", "TOOL_SMOOTH", "s", None, "Switch canvas tool to Brush"),
    ("CANVAS", "TOOL_ARRANGE", "a", None, "Switch canvas tool to Arrange, or arrange all when it is already active"),
    ("CANVAS", "TOOL_DUPLICATE", "d", None, "Switch canvas tool to Duplicate"),
    ("CANVAS", "SELECT_ALL", "a", Some("ctrl"), "Select all visible models"),
    ("CANVAS", "COPY", "c", Some("ctrl"), "Copy selected model(s)"),
    ("CANVAS", "PASTE", "v", Some("ctrl"), "Paste copied model(s)"),
    // PRESETS
    ("PRESETS", "SLOT_1", "1", None, "Pinned Slot 1"),
    ("PRESETS", "SLOT_2", "2", None, "Pinned Slot 2"),
    ("PRESETS", "SLOT_3", "3", None, "Pinned Slot 3"),
    ("PRESETS", "SLOT_4", "4", None, "Pinned Slot 4"),
    ("PRESETS", "SLOT_5", "5", None, "Pinned Slot 5"),
    ("PRESETS", "SLOT_6", "6", None, "Pinned Slot 6"),
    // GLOBAL
    ("GLOBAL", "DELETE", "Backspace", None, "Delete selected item"),
    ("GLOBAL", "UNDO", "z", Some("ctrl"), "Undo last action"),
    ("GLOBAL", "REDO", "z", Some("ctrl+shift"), "Redo last action"),
    ("GLOBAL", "SAVE", "s", Some("ctrl"), "Save active scene"),
    ("GLOBAL", "SAVE_AS", "s", Some("ctrl+shift"), "Save active scene as…"),
];

/// Sidebar order: `sidebarTopTabs` then `sidebarBottomTabs`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Camera,
    Workspace,
    MultiPlate,
    Mesh,
    Slicing,
    Mouse3d,
    Ui,
    Hotkeys,
    Plugins,
    Experiments,
    SceneAutosave,
    Backups,
    UvTools,
    Logging,
    Updates,
    About,
}

struct TabMeta {
    key: &'static str,
    label: &'static str,
    description: &'static str,
    icon: &'static str,
    /// `tone: 'secondary'` in `tabMeta`; primary tabs use the accent.
    primary: bool,
}

impl SettingsTab {
    pub const ALL: [SettingsTab; 17] = [
        Self::General,
        Self::Camera,
        Self::Workspace,
        Self::MultiPlate,
        Self::Mesh,
        Self::Slicing,
        Self::Mouse3d,
        Self::Ui,
        Self::Hotkeys,
        Self::Plugins,
        Self::Experiments,
        Self::SceneAutosave,
        Self::Backups,
        Self::UvTools,
        Self::Logging,
        Self::Updates,
        Self::About,
    ];
    const TOP: [SettingsTab; 9] = [
        Self::General,
        Self::Camera,
        Self::Workspace,
        Self::MultiPlate,
        Self::Mesh,
        Self::Slicing,
        Self::Mouse3d,
        Self::Ui,
        Self::Hotkeys,
    ];
    const BOTTOM: [SettingsTab; 8] = [
        Self::Plugins,
        Self::Experiments,
        Self::SceneAutosave,
        Self::Backups,
        Self::UvTools,
        Self::Logging,
        Self::Updates,
        Self::About,
    ];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|tab| *tab == self).unwrap_or(0)
    }

    pub fn from_index(index: usize) -> Self {
        Self::ALL.get(index).copied().unwrap_or(Self::General)
    }

    fn meta(self) -> TabMeta {
        match self {
            Self::General => TabMeta {
                key: "general",
                label: "General",
                description: "Workspace behavior and panel layout",
                icon: "settings-2",
                primary: true,
            },
            Self::Camera => TabMeta {
                key: "camera",
                label: "Camera",
                description: "Projection and navigation behavior",
                icon: "camera",
                primary: true,
            },
            Self::Workspace => TabMeta {
                key: "workspaces",
                label: "Workspace",
                description: "Build volume bounds, origin, and 3D view resolution",
                icon: "monitor-cog",
                primary: true,
            },
            Self::MultiPlate => TabMeta {
                key: "multi-plate",
                label: "Multi-Plate",
                description: "Plate behaviour across several beds",
                icon: "square-stack",
                primary: true,
            },
            Self::Mesh => TabMeta {
                key: "mesh",
                label: "Mesh",
                description: "Shader, rendering options, and selection behavior",
                icon: "grid-3x3",
                primary: true,
            },
            Self::Slicing => TabMeta {
                key: "slicing",
                label: "Slicing",
                description: "PNG compression and engine metadata",
                icon: "monitor-cog",
                primary: true,
            },
            Self::Mouse3d => TabMeta {
                key: "spacemouse",
                label: "3D Mouse",
                description: "3D mouse navigation controls",
                icon: "gamepad-2",
                primary: true,
            },
            Self::Ui => TabMeta {
                key: "ui",
                label: "UI & Theme",
                description: "Theme and custom UI token customization",
                icon: "palette",
                primary: true,
            },
            Self::Hotkeys => TabMeta {
                key: "hotkeys",
                label: "Hotkeys",
                description: "Keyboard bindings and presets",
                icon: "keyboard",
                primary: true,
            },
            Self::Plugins => TabMeta {
                key: "plugins",
                label: "Plugins",
                description: "Load vendor profile plugins",
                icon: "plug",
                primary: false,
            },
            Self::Experiments => TabMeta {
                key: "experiments",
                label: "Experiments",
                description: "Early-access and experimental features",
                icon: "flask-conical",
                primary: false,
            },
            Self::SceneAutosave => TabMeta {
                key: "scene-autosave",
                label: "Scene Autosave",
                description: "Autosave and crash recovery behavior",
                icon: "hard-drive",
                primary: false,
            },
            Self::Backups => TabMeta {
                key: "backups",
                label: "Backups",
                description: "Local on-disk backup snapshots",
                icon: "archive-restore",
                primary: false,
            },
            Self::UvTools => TabMeta {
                key: "uvtools",
                label: "UVTools",
                description: "Send sliced files to UVTools for analysis",
                icon: "external-link",
                primary: false,
            },
            Self::Logging => TabMeta {
                key: "logging",
                label: "Logging",
                description: "Log file location and verbosity",
                icon: "scroll-text",
                primary: false,
            },
            Self::Updates => TabMeta {
                key: "updates",
                label: "Updates",
                description: "Check for new versions and manage channels",
                icon: "cloud-download",
                primary: false,
            },
            Self::About => TabMeta {
                key: "about",
                label: "About",
                description: "Version info and project details",
                icon: "info",
                primary: false,
            },
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToolLayout {
    Top,
    Left,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RaftBottom {
    Off,
    Line,
    Solid,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Projection {
    Ortho,
    Perspective,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CameraScope {
    Global,
    Workspace,
}

// ── Workspace (`view3dPreferences.ts`) ───────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OriginMode {
    Center,
    FrontLeft,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SliceSatMode {
    AccurateHull,
    ExperimentalSlice,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SliceSatRender {
    Shaded,
    Wireframe,
}

#[derive(Clone)]
pub struct View3dState {
    pub enabled: bool,
    pub width_mm: f32,
    pub depth_mm: f32,
    pub max_z_mm: f32,
    pub origin_mode: OriginMode,
    pub screen_width_px: f32,
    pub screen_height_px: f32,
    pub show_violation_warning: bool,
    pub show_model_bounding_boxes: bool,
    pub show_slice_sat: bool,
    pub sat_all_models: bool,
    pub sat_mode: SliceSatMode,
    pub sat_render: SliceSatRender,
}

impl Default for View3dState {
    fn default() -> Self {
        Self {
            enabled: true,
            width_mm: DEFAULT_VIEW3D_WIDTH_MM,
            depth_mm: DEFAULT_VIEW3D_DEPTH_MM,
            max_z_mm: DEFAULT_VIEW3D_MAX_Z_MM,
            origin_mode: OriginMode::Center,
            screen_width_px: DEFAULT_VIEW3D_SCREEN_WIDTH_PX,
            screen_height_px: DEFAULT_VIEW3D_SCREEN_HEIGHT_PX,
            show_violation_warning: true,
            show_model_bounding_boxes: false,
            show_slice_sat: false,
            sat_all_models: false,
            sat_mode: SliceSatMode::AccurateHull,
            sat_render: SliceSatRender::Shaded,
        }
    }
}

// ── Multi-Plate (`multiPlatePreferences.ts`) ─────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PlateOrdering {
    Fixed,
    Dynamic,
}

#[derive(Clone)]
pub struct MultiPlateState {
    pub follow_landed_plate: bool,
    pub plate_ordering: PlateOrdering,
    pub fixed_plate_columns: f32,
}

impl Default for MultiPlateState {
    fn default() -> Self {
        Self {
            follow_landed_plate: true,
            plate_ordering: PlateOrdering::Fixed,
            fixed_plate_columns: DEFAULT_FIXED_PLATE_COLUMNS,
        }
    }
}

// ── 3D Mouse (`spacemousePreferences.ts`) ────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PivotMode {
    Auto,
    CameraRay,
}

#[derive(Clone)]
pub struct SpaceMouseState {
    pub enabled: bool,
    pub pivot_mode: PivotMode,
    pub translation_sensitivity: f32,
    pub rotation_sensitivity: f32,
    pub zoom_sensitivity: f32,
    pub deadzone: f32,
    pub dominant_axis: bool,
    pub invert_tx: bool,
    pub invert_ty: bool,
    pub invert_tz: bool,
    pub invert_rx: bool,
    pub invert_ry: bool,
    pub invert_rz: bool,
    pub blocked_device_ids: Vec<String>,
}

impl Default for SpaceMouseState {
    fn default() -> Self {
        Self {
            enabled: true,
            pivot_mode: PivotMode::Auto,
            translation_sensitivity: DEFAULT_SPACEMOUSE_SENSITIVITY,
            rotation_sensitivity: DEFAULT_SPACEMOUSE_SENSITIVITY,
            zoom_sensitivity: DEFAULT_SPACEMOUSE_SENSITIVITY,
            deadzone: DEFAULT_SPACEMOUSE_DEADZONE,
            dominant_axis: false,
            invert_tx: false,
            invert_ty: true,
            invert_tz: false,
            invert_rx: false,
            invert_ry: true,
            invert_rz: false,
            blocked_device_ids: Vec::new(),
        }
    }
}

// ── Hotkeys (`hotkeyConfig.ts`) ──────────────────────────────────────────

/// One recorded binding; `modifier` is the source's `'ctrl'` / `'ctrl+shift'`.
#[derive(Clone, PartialEq, Eq)]
pub struct HotkeyBinding {
    pub key: String,
    pub modifier: Option<String>,
}

#[derive(Clone)]
pub struct HotkeyState {
    /// `(category, action)` of the row whose recording overlay is open.
    pub recording: Option<(&'static str, &'static str)>,
    /// Overrides keyed by `(category, action)`; unset rows use the defaults.
    pub overrides: HashMap<(&'static str, &'static str), HotkeyBinding>,
}

impl Default for HotkeyState {
    fn default() -> Self {
        Self {
            recording: None,
            overrides: HashMap::new(),
        }
    }
}

impl HotkeyState {
    fn binding(&self, category: &'static str, action: &'static str) -> HotkeyBinding {
        if let Some(binding) = self.overrides.get(&(category, action)) {
            return binding.clone();
        }
        let default = HOTKEY_DEFAULTS
            .iter()
            .find(|entry| entry.0 == category && entry.1 == action);
        match default {
            Some((_, _, key, modifier, _)) => HotkeyBinding {
                key: (*key).to_owned(),
                modifier: modifier.map(|m| m.to_owned()),
            },
            None => HotkeyBinding {
                key: String::new(),
                modifier: None,
            },
        }
    }

    fn reset_category(&mut self, category: &'static str) {
        self.overrides.retain(|(entry, _), _| *entry != category);
    }
}

// ── Plugins (`PluginsSettingsTab.tsx`) ───────────────────────────────────

#[derive(Clone)]
pub struct PluginsState {
    pub repo_url: String,
    pub allowance_dialog_open: bool,
    pub allow_sideload: bool,
    pub details_row: Option<usize>,
    pub capabilities_expanded: bool,
    pub copied_signing_key: bool,
    pub reload_required: bool,
    pub reload_seconds_left: i32,
    pub status: Option<(bool, String)>,
}

impl Default for PluginsState {
    fn default() -> Self {
        Self {
            repo_url: String::new(),
            allowance_dialog_open: false,
            allow_sideload: false,
            details_row: None,
            capabilities_expanded: false,
            copied_signing_key: false,
            reload_required: false,
            reload_seconds_left: 15,
            status: None,
        }
    }
}

/// The one row the slice can list: the built-in profile pack, which ships with
/// the app. `installedPlugins` from the profile store would add the rest.
pub struct BuiltinPlugin {
    pub id: &'static str,
    pub name: &'static str,
    pub version: &'static str,
    pub description: &'static str,
    pub link: &'static str,
}

const BUILTIN_PLUGINS: [BuiltinPlugin; 1] = [BuiltinPlugin {
    id: "athena",
    name: "Athena",
    version: "1.0.0",
    description: "Built-in printer presets and material profiles.",
    link: DRAGONFRUIT_REPO_URL,
}];

// ── Experiments (`experimentsRegistry.ts`) ───────────────────────────────

#[derive(Clone)]
pub struct ExperimentsState {
    /// Module-level `disclaimerSeenThisLaunch`; only "I understand" sets it.
    pub disclaimer_seen: bool,
    pub show_disclaimer: bool,
    pub enabled: HashMap<&'static str, bool>,
}

impl Default for ExperimentsState {
    fn default() -> Self {
        let mut enabled = HashMap::new();
        for (id, _, _, default_enabled) in EXPERIMENTS.iter() {
            enabled.insert(*id, *default_enabled);
        }
        Self {
            disclaimer_seen: false,
            show_disclaimer: true,
            enabled,
        }
    }
}

// ── Scene Autosave (`sceneAutosavePreferences.ts`) ───────────────────────

#[derive(Clone)]
pub struct AutosaveState {
    pub enabled: bool,
    pub recovery_prompt_enabled: bool,
    pub debounce_ms: f32,
    pub cooldown_ms: f32,
    pub cap_ms: f32,
}

impl Default for AutosaveState {
    fn default() -> Self {
        Self {
            enabled: true,
            recovery_prompt_enabled: true,
            debounce_ms: DEFAULT_AUTOSAVE_DEBOUNCE_MS,
            cooldown_ms: DEFAULT_AUTOSAVE_COOLDOWN_MS,
            cap_ms: DEFAULT_AUTOSAVE_CAP_MS,
        }
    }
}

// ── Backups (`LocalBackupsSettingsTab.tsx`) ──────────────────────────────

#[derive(Clone)]
pub struct BackupsState {
    pub selected_directory: String,
    pub auto_sync_enabled: bool,
    pub auto_sync_minutes: f32,
    pub confirm_delete_id: Option<String>,
    pub confirm_restore_id: Option<String>,
    pub message: Option<(bool, String)>,
}

impl Default for BackupsState {
    fn default() -> Self {
        Self {
            selected_directory: String::new(),
            auto_sync_enabled: true,
            auto_sync_minutes: DEFAULT_AUTO_SYNC_MINUTES,
            confirm_delete_id: None,
            confirm_restore_id: None,
            message: None,
        }
    }
}

// ── UVTools (`uvToolsPreferences.ts`) ────────────────────────────────────

#[derive(Clone)]
pub struct UvToolsState {
    pub enabled: bool,
    pub custom_path: String,
    pub discovery_busy: bool,
    pub show_found_glow: bool,
    pub show_not_found: bool,
}

impl Default for UvToolsState {
    fn default() -> Self {
        Self {
            enabled: false,
            custom_path: String::new(),
            discovery_busy: false,
            show_found_glow: false,
            show_not_found: false,
        }
    }
}

// ── Logging (`LoggingSettingsTab.tsx`) ───────────────────────────────────

#[derive(Clone)]
pub struct LoggingState {
    pub level: &'static str,
    pub viewer_filter: &'static str,
    pub hide_noise: bool,
    pub is_live: bool,
    pub filter_open: bool,
    pub copied: bool,
}

impl Default for LoggingState {
    fn default() -> Self {
        Self {
            level: "info",
            viewer_filter: "trace",
            hide_noise: true,
            is_live: true,
            filter_open: false,
            copied: false,
        }
    }
}

// ── Updates (`UpdatesSettingsTab.tsx`) ───────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UpdateChannel {
    Stable,
    Dev,
}

#[derive(Clone)]
pub struct UpdatesState {
    pub channel: UpdateChannel,
    pub pending_channel: UpdateChannel,
    pub show_channel_settings: bool,
}

impl Default for UpdatesState {
    fn default() -> Self {
        Self {
            channel: UpdateChannel::Stable,
            pending_channel: UpdateChannel::Stable,
            show_channel_settings: false,
        }
    }
}

// ── About (`SettingsModal.tsx`) ──────────────────────────────────────────

#[derive(Clone)]
pub struct AboutState {
    pub copied: bool,
}

impl Default for AboutState {
    fn default() -> Self {
        Self { copied: false }
    }
}

/// The modal's draft. Mirrors the `draft*` state in `SettingsModal`; `Apply`
/// hands it to the parent once the parent owns the fields.
#[derive(Clone)]
pub struct SettingsState {
    pub language: usize,
    pub ui_scale: f32,
    pub custom_scale: bool,
    pub tool_layout: ToolLayout,
    pub floating_persistence: bool,
    pub support_help: bool,
    pub raft_bottom: RaftBottom,
    pub raft_wall: bool,
    pub roots_enabled: bool,
    pub auto_repair: bool,
    pub auto_repair_scenes: bool,
    pub camera_scope: CameraScope,
    pub projection: Projection,
    pub fov: f32,
    pub camera_feel: usize,
    pub trackpad_action: usize,
    pub trackpad_modifier: usize,
    pub pan_acceleration: f32,
    pub orbit_acceleration: f32,
    pub zoom_acceleration: f32,
    pub workspace_tab: usize,
    pub workspace_defaults: [Projection; 4],
    pub shader: usize,
    pub matcap: usize,
    pub preview_model: usize,
    pub flat_use_vertex_colors: bool,
    pub mesh_color: String,
    pub ambient_intensity: f32,
    pub directional_intensity: f32,
    pub material_roughness: f32,
    pub baked_ao_intensity: f32,
    pub xray_opacity: f32,
    pub heatmap_min_angle: f32,
    pub heatmap_max_angle: f32,
    pub heatmap_colors: Vec<String>,
    pub active_color_index: usize,
    pub hover_tint_strength: f32,
    pub selected_tint_strength: f32,
    pub preview_hovered: bool,
    pub preview_selected: bool,
    pub thumbnail_gradient: bool,
    pub thumbnail_build_plate: bool,
    pub thumbnail_grid: bool,
    pub png_compression: usize,
    pub theme_preset: usize,
    pub theme_preference: usize,
    pub theme_tab: usize,
    pub theme_colors: HashMap<&'static str, String>,
    pub cut_colors: HashMap<&'static str, String>,
    /// The `SelectDropdown` whose menu is open, by element id.
    pub open_select: Option<&'static str>,
    pub view3d: View3dState,
    pub multi_plate: MultiPlateState,
    pub spacemouse: SpaceMouseState,
    pub hotkeys: HotkeyState,
    pub plugins: PluginsState,
    pub experiments: ExperimentsState,
    pub autosave: AutosaveState,
    pub backups: BackupsState,
    pub uvtools: UvToolsState,
    pub logging: LoggingState,
    pub updates: UpdatesState,
    pub about: AboutState,
    /// `showRestoreDefaultsConfirm` in the modal shell.
    pub show_restore_confirm: bool,
}

impl Default for SettingsState {
    fn default() -> Self {
        let mut theme_colors = HashMap::new();
        for section in THEME_COLOR_SECTIONS.iter() {
            for row in section.rows {
                theme_colors.insert(row.key, row.placeholder.to_owned());
            }
        }
        let mut cut_colors = HashMap::new();
        for field in CUT_COLOR_FIELDS.iter() {
            cut_colors.insert(field.key, field.placeholder.to_owned());
        }
        Self {
            language: 0,
            ui_scale: DEFAULT_UI_SCALE,
            custom_scale: false,
            tool_layout: ToolLayout::Top,
            floating_persistence: true,
            support_help: true,
            raft_bottom: RaftBottom::Solid,
            raft_wall: true,
            roots_enabled: true,
            auto_repair: false,
            auto_repair_scenes: false,
            camera_scope: CameraScope::Global,
            projection: Projection::Ortho,
            fov: DEFAULT_FOV,
            camera_feel: 1,
            trackpad_action: 1,
            trackpad_modifier: 1,
            pan_acceleration: 2.0,
            orbit_acceleration: 2.0,
            zoom_acceleration: 2.0,
            workspace_tab: 0,
            workspace_defaults: [Projection::Ortho; 4],
            shader: 0,
            matcap: 0,
            preview_model: 2,
            flat_use_vertex_colors: true,
            mesh_color: DEFAULT_MESH_COLOR.to_owned(),
            ambient_intensity: DEFAULT_AMBIENT_INTENSITY,
            directional_intensity: DEFAULT_DIRECTIONAL_INTENSITY,
            material_roughness: DEFAULT_MATERIAL_ROUGHNESS,
            baked_ao_intensity: 1.0,
            xray_opacity: DEFAULT_XRAY_OPACITY,
            heatmap_min_angle: DEFAULT_HEATMAP_MIN_ANGLE,
            heatmap_max_angle: DEFAULT_HEATMAP_MAX_ANGLE,
            heatmap_colors: DEFAULT_HEATMAP_COLORS.iter().map(|c| (*c).to_owned()).collect(),
            active_color_index: 0,
            hover_tint_strength: DEFAULT_HOVER_TINT_STRENGTH,
            selected_tint_strength: DEFAULT_SELECTED_TINT_STRENGTH,
            preview_hovered: false,
            preview_selected: false,
            thumbnail_gradient: false,
            thumbnail_build_plate: false,
            thumbnail_grid: false,
            png_compression: 0,
            theme_preset: 0,
            theme_preference: 0,
            theme_tab: 0,
            theme_colors,
            cut_colors,
            open_select: None,
            view3d: View3dState::default(),
            multi_plate: MultiPlateState::default(),
            spacemouse: SpaceMouseState::default(),
            hotkeys: HotkeyState::default(),
            plugins: PluginsState::default(),
            experiments: ExperimentsState::default(),
            autosave: AutosaveState::default(),
            backups: BackupsState::default(),
            uvtools: UvToolsState::default(),
            logging: LoggingState::default(),
            updates: UpdatesState::default(),
            about: AboutState::default(),
            show_restore_confirm: false,
        }
    }
}

impl SettingsState {
    /// `applyRestoreDefaultsToDraft` — only the fields that callback resets.
    /// Language, UI scale, theme profiles, hotkeys, plugins, experiments,
    /// autosave, backups, logging and the update channel are left alone,
    /// exactly as at the source.
    fn restore_defaults(&mut self) {
        self.mesh_color = DEFAULT_MESH_COLOR.to_owned();
        self.shader = 0; // `soft_clay`
        self.matcap = 0; // `neutral`
        self.flat_use_vertex_colors = true;
        self.ambient_intensity = DEFAULT_AMBIENT_INTENSITY;
        self.directional_intensity = DEFAULT_DIRECTIONAL_INTENSITY;
        self.material_roughness = DEFAULT_MATERIAL_ROUGHNESS;
        self.baked_ao_intensity = 1.0;
        self.xray_opacity = DEFAULT_XRAY_OPACITY;
        self.heatmap_min_angle = DEFAULT_HEATMAP_MIN_ANGLE;
        self.heatmap_max_angle = DEFAULT_HEATMAP_MAX_ANGLE;
        self.hover_tint_strength = DEFAULT_HOVER_TINT_STRENGTH;
        self.selected_tint_strength = DEFAULT_SELECTED_TINT_STRENGTH;
        self.projection = Projection::Ortho;
        self.camera_feel = 1; // `precise`
        self.trackpad_action = 1; // `pan`
        self.trackpad_modifier = 1; // `shift`
        self.pan_acceleration = 2.0;
        self.orbit_acceleration = 2.0;
        self.zoom_acceleration = 2.0;
        self.camera_scope = CameraScope::Global;
        self.fov = DEFAULT_FOV;
        self.theme_preference = 0; // `dark`
        self.theme_preset = 0; // `dragonfruit-dark`
        self.theme_colors.clear();
        for section in THEME_COLOR_SECTIONS.iter() {
            for row in section.rows {
                self.theme_colors.insert(row.key, row.placeholder.to_owned());
            }
        }
        self.floating_persistence = true;
        self.raft_bottom = RaftBottom::Solid;
        self.raft_wall = true;
        self.roots_enabled = true;
        self.auto_repair = false;
        self.auto_repair_scenes = false;
        self.spacemouse = SpaceMouseState::default();
        self.workspace_defaults = [Projection::Ortho; 4];
        self.view3d = View3dState::default();
        self.png_compression = 0; // `auto`
        self.thumbnail_gradient = false;
        self.thumbnail_build_plate = false;
        self.thumbnail_grid = false;
        self.uvtools = UvToolsState::default();
    }
}

static SETTINGS: LazyLock<Mutex<SettingsState>> =
    LazyLock::new(|| Mutex::new(SettingsState::default()));
fn state() -> &'static Mutex<SettingsState> {
    &SETTINGS
}

fn edit(f: impl FnOnce(&mut SettingsState)) {
    match state().lock() {
        Ok(mut guard) => f(&mut guard),
        Err(poisoned) => f(&mut poisoned.into_inner()),
    }
}

fn edit_and_notify(
    entity: &WeakEntity<DragonFruit>,
    cx: &mut App,
    f: impl FnOnce(&mut SettingsState),
) {
    edit(f);
    entity.update(cx, |_, cx| cx.notify()).ok();
}

fn close(entity: &WeakEntity<DragonFruit>, cx: &mut App) {
    entity
        .update(cx, |this, cx| {
            this.show_settings = false;
            cx.notify();
        })
        .ok();
}

/// `SettingsModal` for the app bar's settings button.
///
/// `open` mirrors `isOpen` (the parent also gates with `.when`); `tab` is the
/// parent-owned `settings_tab` index into [`SettingsTab::ALL`]. The close
/// button and the backdrop write `show_settings = false` on `entity`; Escape is
/// already handled by the parent's root key listener.
pub fn settings_modal(
    open: bool,
    tab: usize,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    if !open {
        return div();
    }
    {
        use std::sync::atomic::{AtomicBool, Ordering};
        static LOGGED: AtomicBool = AtomicBool::new(false);
        if !LOGGED.swap(true, Ordering::Relaxed) {
            gpui_core::log::log("settings: rendering open");
        }
    }

    // Snapshot the draft: holding the lock across `content` deadlocks because
    // the select fields lock it again to read `open_select`.
    let snapshot = state()
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default();
    let tab = SettingsTab::from_index(tab);

    let (body, overlay) = content(tab, &snapshot, entity.clone());

    let backdrop_entity = entity.clone();
    let card = div()
        .w_full()
        .max_w(px(CARD_MAX_WIDTH))
        .flex()
        .flex_col()
        .rounded(px(16.)) // rounded-2xl
        .border_1()
        .border_color(rgb(BORDER_STRONG))
        .bg(rgb(SURFACE_0))
        .overflow_hidden()
        // Keep backdrop clicks from closing the modal when they land on the card.
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(header(entity.clone()))
        .child(
            div()
                .flex_1()
                .min_h(px(0.))
                .flex()
                .child(sidebar(tab, entity.clone()))
                .child(body),
        )
        .child(footer(entity.clone()));

    let mut layer = div()
        .absolute()
        .top(px(0.))
        .left(px(0.))
        .size_full()
        .p(px(BACKDROP_PADDING))
        .flex()
        .items_stretch()
        .justify_center()
        .bg(rgba(0x00000099)) // bg-black/60
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            close(&backdrop_entity, cx);
        })
        .child(card);

    // The source portals every dialog to `<body>`; here they paint after the
    // card, inside the same backdrop.
    if let Some(overlay) = overlay {
        layer = layer.child(overlay);
    }
    if snapshot.show_restore_confirm {
        layer = layer.child(restore_defaults_dialog(entity));
    }
    layer
}

// ── Shell ────────────────────────────────────────────────────────────────

fn header(entity: WeakEntity<DragonFruit>) -> Div {
    let close_entity = entity;
    div()
        .flex()
        .items_center()
        .justify_between()
        .px(px(16.))
        .py(px(12.))
        .border_b_1()
        .border_color(rgb(BORDER_SUBTLE))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(36.))
                        .h(px(36.))
                        .rounded(px(8.)) // rounded-lg
                        .border_1()
                        .border_color(rgb(BORDER_SUBTLE))
                        .bg(linear_gradient(
                            135.,
                            linear_color_stop(rgb(mix(ACCENT, SURFACE_1, 0.16)), 0.),
                            linear_color_stop(rgb(mix(ACCENT_SECONDARY, SURFACE_1, 0.10)), 1.),
                        ))
                        .child(icon("settings-2", 18., ACCENT)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(16.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child("Settings"),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(TEXT_MUTED))
                                .child("Customize DragonFruit behavior, visuals, and controls."),
                        ),
                ),
        )
        .child(
            icon_button(
                "settings-close",
                "x",
                IconButtonVariant::Solid,
                IconButtonSize::Sm,
            )
            .on_click(move |_, _, cx| close(&close_entity, cx)),
        )
}

fn sidebar(active: SettingsTab, entity: WeakEntity<DragonFruit>) -> Div {
    let mut top = div().flex().flex_col().gap(px(4.));
    for tab in SettingsTab::TOP {
        top = top.child(rail_tab(tab, tab == active, entity.clone()));
    }
    let mut bottom = div().flex().flex_col().gap(px(4.)).mt_auto().pt(px(12.));
    for tab in SettingsTab::BOTTOM {
        bottom = bottom.child(rail_tab(tab, tab == active, entity.clone()));
    }

    div()
        .w(px(SIDEBAR_WIDTH))
        .flex_shrink_0()
        .p(px(10.))
        .border_r_1()
        .border_color(rgb(BORDER_SUBTLE))
        // linear-gradient(180deg, surface-1 94%, accent-secondary 4% over surface-1)
        .bg(linear_gradient(
            180.,
            linear_color_stop(rgb(mix(SURFACE_1, SURFACE_0, 0.94)), 0.),
            linear_color_stop(rgb(mix(ACCENT_SECONDARY, SURFACE_1, 0.04)), 1.),
        ))
        .child(
            div()
                .id("settings-rail")
                .size_full()
                .min_h(px(0.))
                .flex()
                .flex_col()
                .overflow_y_scroll()
                .child(top)
                .child(bottom),
        )
}

fn rail_tab(tab: SettingsTab, active: bool, entity: WeakEntity<DragonFruit>) -> Stateful<Div> {
    let meta = tab.meta();
    let tone = if meta.primary { ACCENT } else { ACCENT_SECONDARY };
    let (border, background) = if active {
        (
            mix(tone, BORDER_SUBTLE, 0.65),
            mix(tone, SURFACE_0, 0.16),
        )
    } else {
        (BORDER_SUBTLE, SURFACE_1)
    };
    let icon_color = if active { tone } else { TEXT_MUTED };
    let index = tab.index();

    div()
        .id(format!("settings-rail-{}", meta.key))
        .w_full()
        .rounded(px(4.)) // rounded-sm
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .px(px(10.))
        .py(px(6.))
        .flex()
        .items_center()
        .gap(px(10.))
        .cursor_pointer()
        .on_click(move |_, _, cx| {
            entity
                .update(cx, |this, cx| {
                    this.settings_tab = index;
                    cx.notify();
                })
                .ok();
        })
        .child(icon(meta.icon, 20., icon_color))
        .child(
            div()
                .min_w(px(0.))
                .flex_1()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(15.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(relative(1.2))
                        .text_color(rgb(TEXT_STRONG))
                        .child(meta.label),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .line_height(relative(1.2))
                        .text_color(rgb(TEXT_MUTED))
                        .truncate()
                        .child(meta.description),
                ),
        )
}

fn content(
    tab: SettingsTab,
    state: &SettingsState,
    entity: WeakEntity<DragonFruit>,
) -> (Stateful<Div>, Option<Div>) {
    let body_entity = entity.clone();
    let body = match tab {
        SettingsTab::General => general_tab(state, body_entity),
        SettingsTab::Camera => camera_tab(state, body_entity),
        SettingsTab::Workspace => workspace_tab(state, body_entity),
        SettingsTab::MultiPlate => multi_plate_tab(state, body_entity),
        SettingsTab::Mesh => mesh_tab(state, body_entity),
        SettingsTab::Slicing => slicing_tab(state, body_entity),
        SettingsTab::Mouse3d => spacemouse_tab(state, body_entity),
        SettingsTab::Ui => ui_tab(state, body_entity),
        SettingsTab::Hotkeys => hotkeys_tab(state, body_entity),
        SettingsTab::Plugins => plugins_tab(state, body_entity),
        SettingsTab::Experiments => experiments_tab(state, body_entity),
        SettingsTab::SceneAutosave => autosave_tab(state, body_entity),
        SettingsTab::Backups => backups_tab(state, body_entity),
        SettingsTab::UvTools => uvtools_tab(state, body_entity),
        SettingsTab::Logging => logging_tab(state, body_entity),
        SettingsTab::Updates => updates_tab(state, body_entity),
        SettingsTab::About => about_tab(state, body_entity),
    };

    // About, Hotkeys and Updates use `usesInternalTabScrollLayout`: the tab owns
    // its own scroller and the content column is a fixed-height flex column.
    let internal_scroll = matches!(
        tab,
        SettingsTab::About | SettingsTab::Hotkeys | SettingsTab::Updates
    );

    let mut column = div()
        .id("settings-content")
        .flex_1()
        .min_w(px(0.))
        .min_h(px(0.))
        .p(px(16.));
    column = if internal_scroll {
        column.flex().flex_col()
    } else {
        column.overflow_y_scroll()
    };

    let body = body;
    let overlay = match tab {
        SettingsTab::Hotkeys => hotkeys_overlay(state, entity.clone()),
        SettingsTab::Plugins => plugins_overlay(state, entity.clone()),
        SettingsTab::Experiments => experiments_overlay(state, entity.clone()),
        SettingsTab::Backups => backups_overlay(state, entity.clone()),
        SettingsTab::Updates => updates_overlay(state, entity.clone()),
        _ => None,
    };

    (column.child(body), overlay)
}

fn footer(entity: WeakEntity<DragonFruit>) -> Div {
    let restore_entity = entity.clone();
    let cancel_entity = entity.clone();
    let apply_entity = entity;

    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.))
        .px(px(12.))
        .py(px(12.))
        .border_t_1()
        .border_color(rgb(BORDER_SUBTLE))
        // color-mix(in srgb, surface-1, transparent 10%) over surface-0
        .bg(rgb(mix(SURFACE_1, SURFACE_0, 0.9)))
        .child(action_button(
            "settings-restore",
            "Restore Defaults",
            Some("rotate-ccw"),
            14.0,
            mix(ACCENT_SECONDARY, SURFACE_1, 0.08),
            mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.58),
            ACCENT_SECONDARY,
            mix(ACCENT_SECONDARY, SURFACE_1, 0.24),
            move |cx| {
                // `handleRestoreDefaults` opens the confirmation first.
                edit_and_notify(&restore_entity, cx, |draft| {
                    draft.show_restore_confirm = true;
                });
            },
        ))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(action_button(
                    "settings-cancel",
                    "Cancel",
                    None,
                    16.0,
                    SURFACE_1,
                    BORDER_SUBTLE,
                    TEXT_MUTED,
                    SURFACE_2,
                    move |cx| close(&cancel_entity, cx),
                ))
                .child(action_button(
                    "settings-apply",
                    "Apply",
                    Some("check"),
                    16.0,
                    mix(ACCENT, SURFACE_0, 0.16),
                    mix(ACCENT, 0xffffff, 0.9),
                    ACCENT_CONTRAST,
                    mix(ACCENT, SURFACE_0, 0.28),
                    move |cx| close(&apply_entity, cx),
                )),
        )
}

/// `StructuredDialogModal` — the shell every settings dialog shares.
///
/// `max-w-md` default, `rounded-xl border shadow-2xl` on `--surface-0`, a
/// `px-5 py-4` header with an `IconChip` and a `p-5` body, and the action row
/// on the right.
fn dialog_shell(
    width: f32,
    icon_slug: &'static str,
    icon_tone: (u32, u32, u32),
    title: impl Into<SharedString>,
    subtitle: Option<SharedString>,
    body: Div,
    actions: Div,
) -> Div {
    let (icon_fg, icon_border, icon_bg) = icon_tone;
    div()
        .absolute()
        .top(px(0.))
        .left(px(0.))
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .px(px(12.))
        .bg(rgba(0x0000008c)) // bg-black/55
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(
            div()
                .w(px(width))
                .max_w(relative(1.0))
                .flex()
                .flex_col()
                .overflow_hidden()
                .rounded(px(12.)) // rounded-xl
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .px(px(20.))
                        .py(px(16.))
                        .border_b_1()
                        .border_color(rgb(BORDER_SUBTLE))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .w(px(40.))
                                .h(px(40.))
                                .flex_shrink_0()
                                .rounded(px(10.))
                                .border_1()
                                .border_color(rgb(icon_border))
                                .bg(rgb(icon_bg))
                                .child(icon(icon_slug, 18., icon_fg)),
                        )
                        .child(
                            div()
                                .min_w(px(0.))
                                .flex_1()
                                .child(
                                    div()
                                        .text_size(px(16.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .line_height(relative(1.2))
                                        .text_color(rgb(TEXT_STRONG))
                                        .child(title.into()),
                                )
                                .when_some(subtitle, |this, subtitle| {
                                    this.child(
                                        div()
                                            .mt(px(2.))
                                            .text_size(px(12.))
                                            .line_height(relative(1.2))
                                            .text_color(rgb(TEXT_MUTED))
                                            .child(subtitle),
                                    )
                                }),
                        ),
                )
                .child(div().p(px(20.)).child(body))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap(px(8.))
                        .px(px(20.))
                        .pb(px(20.))
                        .child(actions),
                ),
        )
}

fn restore_defaults_dialog(entity: WeakEntity<DragonFruit>) -> Div {
    let cancel_entity = entity.clone();
    let confirm_entity = entity;
    dialog_shell(
        448.0, // max-w-md
        "rotate-ccw",
        (0xf59e0b, mix(0xf59e0b, BORDER_SUBTLE, 0.45), mix(0xf59e0b, SURFACE_1, 0.9)),
        "Restore Defaults?",
        Some("This resets settings in this dialog to their default values.".into()),
        div().child(
            div()
                .flex()
                .flex_wrap()
                .text_size(px(12.))
                .line_height(relative(1.35))
                .text_color(rgb(TEXT_MUTED))
                .child("You can still review the changes before saving. Nothing is written until you click ")
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("Apply"),
                )
                .child("."),
        ),
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(action_button(
                "restore-cancel",
                "Keep Current",
                None,
                16.0,
                SURFACE_1,
                BORDER_SUBTLE,
                TEXT_STRONG,
                SURFACE_2,
                move |cx| {
                    edit_and_notify(&cancel_entity, cx, |draft| {
                        draft.show_restore_confirm = false;
                    });
                },
            ))
            .child(action_button(
                "restore-confirm",
                "Restore Defaults",
                Some("rotate-ccw"),
                16.0,
                mix(ACCENT, SURFACE_1, 0.14),
                mix(ACCENT, BORDER_SUBTLE, 0.55),
                ACCENT,
                mix(ACCENT, SURFACE_1, 0.24),
                move |cx| {
                    edit_and_notify(&confirm_entity, cx, |draft| {
                        draft.restore_defaults();
                        draft.show_restore_confirm = false;
                    });
                },
            )),
    )
}

// ── General ──────────────────────────────────────────────────────────────

fn general_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let mut root = div().flex().flex_col().gap(px(12.));

    // Language
    let language_entity = entity.clone();
    root = root.child(
        section(
            "languages",
            "Language",
            "Choose the interface language. Detected from your system on first run; your choice is remembered.",
        )
        .child(
            setting_row(
                "Interface language",
                "Applied immediately across the app.",
                select_field(
                    "settings-language",
                    &LANGUAGE_OPTIONS,
                    LANGUAGE_VALUES[state.language.min(LANGUAGE_VALUES.len() - 1)],
                    Some("languages"),
                    Some(CONTROL_WIDTH),
                    true,
                    language_entity.clone(),
                    move |value, cx| {
                        if let Some(index) = LANGUAGE_VALUES.iter().position(|code| *code == value) {
                            edit_and_notify(&language_entity, cx, |draft| draft.language = index);
                        }
                    },
                ),
            )
            .mt(px(12.)),
        ),
    );

    // UI Scale
    let is_custom = state.custom_scale
        || !UI_SCALE_PRESETS
            .iter()
            .any(|preset| (*preset - state.ui_scale).abs() < 0.001);
    let scale_value = if is_custom {
        "custom"
    } else {
        UI_SCALE_OPTIONS
            .iter()
            .find(|(value, _)| {
                value
                    .parse::<f32>()
                    .map(|preset| (preset - state.ui_scale).abs() < 0.001)
                    .unwrap_or(false)
            })
            .map(|(value, _)| *value)
            .unwrap_or("custom")
    };
    let scale_entity = entity.clone();
    let mut scale_card = inner_card().child(plain_row(
        "Interface scale",
        "Larger percentages magnify the whole UI.",
        select_field(
            "settings-ui-scale",
            &UI_SCALE_OPTIONS,
            scale_value,
            Some("zoom-in"),
            Some(CONTROL_WIDTH),
            true,
            scale_entity.clone(),
            move |value, cx| {
                if value == "custom" {
                    // Selecting "Custom" arms the field but saves nothing.
                    edit_and_notify(&scale_entity, cx, |draft| draft.custom_scale = true);
                    return;
                }
                if let Ok(next) = value.parse::<f32>() {
                    edit_and_notify(&scale_entity, cx, |draft| {
                        draft.custom_scale = false;
                        draft.ui_scale = next;
                    });
                }
            },
        ),
    ));
    if is_custom {
        let custom_entity = entity.clone();
        scale_card = scale_card.child(
            div()
                .mt(px(10.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Custom scale"),
                )
                .child(number_control(
                    "settings-custom-scale",
                    state.ui_scale * 100.0,
                    "%",
                    1.0,
                    MIN_UI_SCALE * 100.0,
                    MAX_UI_SCALE * 100.0,
                    CONTROL_WIDTH,
                    move |next, cx| {
                        edit_and_notify(&custom_entity, cx, |draft| {
                            draft.ui_scale = (next / 100.0).clamp(MIN_UI_SCALE, MAX_UI_SCALE);
                        });
                    },
                )),
        );
    }
    root = root.child(
        section(
            "zoom-in",
            "UI Scale",
            "Adjusts the size of the entire interface. 100% adapts to your screen automatically.",
        )
        .child(scale_card.mt(px(12.))),
    );

    // Toolbar
    let toolbar_entity = entity.clone();
    root = root.child(
        section(
            "panel-top",
            "Toolbar",
            "Park the tools in a column on the left edge, or in a bar centred under the app bar.",
        )
        .child(
            setting_row(
                "Toolbar position",
                "A left column or a bar centred under the app bar.",
                select_field(
                    "settings-tool-layout",
                    &TOOL_LAYOUT_OPTIONS,
                    match state.tool_layout {
                        ToolLayout::Top => "horizontal",
                        ToolLayout::Left => "vertical",
                    },
                    Some(match state.tool_layout {
                        ToolLayout::Top => "arrow-up",
                        ToolLayout::Left => "arrow-left",
                    }),
                    Some(CONTROL_WIDTH),
                    true,
                    toolbar_entity.clone(),
                    move |value, cx| {
                        let next = if value == "vertical" {
                            ToolLayout::Left
                        } else {
                            ToolLayout::Top
                        };
                        edit_and_notify(&toolbar_entity, cx, |draft| draft.tool_layout = next);
                    },
                ),
            )
            .mt(px(12.)),
        ),
    );

    // Floating Windows
    let persistence_entity = entity.clone();
    let reset_entity = entity.clone();
    root = root.child(
        section(
            "layout-grid",
            "Floating Windows",
            "Keep moved panel positions between sessions, or always start from the default workspace layout.",
        )
        .child(
            setting_row(
                "Remember window positions",
                "Persist dragged panel positions in local storage.",
                segmented(
                    "settings-floating-persistence",
                    &["OFF", "ON"],
                    usize::from(state.floating_persistence),
                    false,
                    move |index, cx| {
                        edit_and_notify(&persistence_entity, cx, |draft| {
                            draft.floating_persistence = index == 1;
                        });
                    },
                ),
            )
            .mt(px(12.)),
        )
        .child(
            setting_row(
                "Reset saved window layout",
                "Forget all stored panel positions and return to seeded layout.",
                action_button(
                    "settings-reset-layout",
                    "Reset",
                    Some("rotate-ccw"),
                    12.0,
                    SURFACE_1,
                    BORDER_SUBTLE,
                    TEXT_STRONG,
                    SURFACE_2,
                    move |cx| {
                        // The saved panel positions live outside this draft; the
                        // only layout preference it owns is the persistence flag,
                        // so Reset returns that to its default.
                        edit(|draft| draft.floating_persistence = true);
                        reset_entity.update(cx, |_, cx| cx.notify()).ok();
                    },
                ),
            )
            .mt(px(8.)),
        ),
    );

    // Support Tooltips
    let support_entity = entity.clone();
    root = root.child(
        section(
            "circle-question-mark",
            "Support Tooltips",
            "Show placement tooltips next to the cursor while adding supports.",
        )
        .child(
            setting_row(
                "Show support tooltips",
                "Shows \"Cannot Place Support\" and \"Stability Warning\" tooltips.",
                segmented(
                    "settings-support-help",
                    &["OFF", "ON"],
                    usize::from(state.support_help),
                    false,
                    move |index, cx| {
                        edit_and_notify(&support_entity, cx, |draft| {
                            draft.support_help = index == 1;
                        });
                    },
                ),
            )
            .mt(px(12.)),
        ),
    );

    // Import Defaults
    let raft_entity = entity.clone();
    let wall_entity = entity.clone();
    let roots_entity = entity.clone();
    let repair_entity = entity.clone();
    let repair_scenes_entity = entity.clone();
    let mut import_section = section(
        "database",
        "Import Defaults",
        "Applied automatically when importing Scene Files.",
    )
    .child(
        setting_row(
            "Default Raft Base",
            "Chooses raft bottom mode for imported supports.",
            select_field(
                "settings-raft-bottom",
                &RAFT_OPTIONS,
                match state.raft_bottom {
                    RaftBottom::Off => "off",
                    RaftBottom::Line => "line",
                    RaftBottom::Solid => "solid",
                },
                None,
                Some(CONTROL_WIDTH),
                true,
                raft_entity.clone(),
                move |value, cx| {
                    let next = match value {
                        "off" => RaftBottom::Off,
                        "line" => RaftBottom::Line,
                        _ => RaftBottom::Solid,
                    };
                    edit_and_notify(&raft_entity, cx, |draft| {
                        draft.raft_bottom = next;
                        if next == RaftBottom::Line {
                            draft.roots_enabled = true;
                        }
                    });
                },
            ),
        )
        .mt(px(12.)),
    );
    if state.raft_bottom == RaftBottom::Solid {
        import_section = import_section.child(
            setting_row(
                "Default Raft Wall",
                "Enable perimeter wall for imported solid rafts.",
                segmented(
                    "settings-raft-wall",
                    &["OFF", "ON"],
                    usize::from(state.raft_wall),
                    false,
                    move |index, cx| {
                        edit_and_notify(&wall_entity, cx, |draft| {
                            draft.raft_wall = index == 1;
                        });
                    },
                ),
            )
            .mt(px(8.)),
        );
    }
    if state.raft_bottom != RaftBottom::Line {
        import_section = import_section.child(
            setting_row(
                "Roots Enabled on Import",
                "OFF makes imported root diameter match trunk diameter.",
                segmented(
                    "settings-roots",
                    &["OFF", "ON"],
                    usize::from(state.roots_enabled),
                    false,
                    move |index, cx| {
                        edit_and_notify(&roots_entity, cx, |draft| {
                            draft.roots_enabled = index == 1;
                        });
                    },
                ),
            )
            .mt(px(8.)),
        );
    }
    import_section = import_section
        .child(
            setting_row(
                "Auto-Repair",
                "Automatically runs native mesh auto-repair for standard mesh imports.",
                segmented(
                    "settings-auto-repair",
                    &["OFF", "ON"],
                    usize::from(state.auto_repair),
                    false,
                    move |index, cx| {
                        edit_and_notify(&repair_entity, cx, |draft| {
                            draft.auto_repair = index == 1;
                        });
                    },
                ),
            )
            .mt(px(8.)),
        )
        .child(
            setting_row(
                "Auto-Repair Scenes",
                "Automatically runs native mesh auto-repair for scene-file imports.",
                segmented(
                    "settings-auto-repair-scenes",
                    &["OFF", "ON"],
                    usize::from(state.auto_repair_scenes),
                    false,
                    move |index, cx| {
                        edit_and_notify(&repair_scenes_entity, cx, |draft| {
                            draft.auto_repair_scenes = index == 1;
                        });
                    },
                ),
            )
            .mt(px(8.)),
        );
    root.child(import_section)
}

// ── Camera ───────────────────────────────────────────────────────────────

fn camera_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let mut root = div().flex().flex_col().gap(px(12.));

    let scope_entity = entity.clone();
    let mut defaults = section(
        "camera",
        "Camera Defaults",
        "Global camera projection mode and navigation behavior.",
    )
    .child(
        setting_row(
            "Camera scope",
            "Choose one global projection mode for every workspace, or set projection defaults per workspace.",
            segmented(
                "settings-camera-scope",
                &["GLOBAL", "WORKSPACE"],
                match state.camera_scope {
                    CameraScope::Global => 0,
                    CameraScope::Workspace => 1,
                },
                false,
                move |index, cx| {
                    edit_and_notify(&scope_entity, cx, |draft| {
                        draft.camera_scope = if index == 0 {
                            CameraScope::Global
                        } else {
                            CameraScope::Workspace
                        };
                    });
                },
            ),
        )
        .mt(px(12.)),
    );

    if state.camera_scope == CameraScope::Global {
        let projection_entity = entity.clone();
        let mut block = div()
            .mt(px(8.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(setting_row(
                "Projection mode",
                "Use one projection mode everywhere when global scope is active.",
                segmented(
                    "settings-projection",
                    &["ORTHO", "PERSPECTIVE"],
                    match state.projection {
                        Projection::Ortho => 0,
                        Projection::Perspective => 1,
                    },
                    false,
                    move |index, cx| {
                        edit_and_notify(&projection_entity, cx, |draft| {
                            draft.projection = if index == 0 {
                                Projection::Ortho
                            } else {
                                Projection::Perspective
                            };
                        });
                    },
                ),
            ));
        if state.projection == Projection::Perspective {
            block = block.child(slider_field(
                "Field of view",
                format!("{}°", state.fov.round() as i32),
                state.fov,
                FOV_MIN,
                FOV_MAX));
        }
        defaults = defaults.child(block);
    }

    if state.camera_scope == CameraScope::Workspace {
        let tab_entity = entity.clone();
        let default_entity = entity.clone();
        let active = state.workspace_tab.min(WORKSPACES.len() - 1);
        let block = div()
            .mt(px(8.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                field_card().child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("Workspace camera defaults"),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Pick the default projection mode used when you enter each workspace."),
                )
                .child(
                    div().mt(px(8.)).child(segmented(
                        "settings-workspace-tab",
                        &["PREPARE", "ANALYSIS", "SUPPORT", "EXPORT"],
                        active,
                        true,
                        move |index, cx| {
                            edit_and_notify(&tab_entity, cx, |draft| draft.workspace_tab = index);
                        },
                    )),
                ),
            )
            .child(setting_row(
                format!("{} default camera", WORKSPACES[active].0),
                WORKSPACES[active].1,
                segmented(
                    "settings-workspace-default",
                    &["ORTHO", "PERSPECTIVE"],
                    match state.workspace_defaults[active] {
                        Projection::Ortho => 0,
                        Projection::Perspective => 1,
                    },
                    false,
                    move |index, cx| {
                        edit_and_notify(&default_entity, cx, |draft| {
                            draft.workspace_defaults[active] = if index == 0 {
                                Projection::Ortho
                            } else {
                                Projection::Perspective
                            };
                        });
                    },
                ),
            ));
        defaults = defaults.child(block);
    }

    let feel_entity = entity.clone();
    defaults = defaults.child(
        setting_row(
            "Camera feel",
            "Controls smoothing and movement acceleration while orbiting, panning, and zooming.",
            select_field(
                "settings-camera-feel",
                &FEEL_OPTIONS,
                FEEL_VALUES[state.camera_feel.min(FEEL_VALUES.len() - 1)],
                None,
                Some(CONTROL_WIDTH),
                true,
                feel_entity.clone(),
                move |value, cx| {
                    if let Some(index) = FEEL_VALUES.iter().position(|feel| *feel == value) {
                        edit_and_notify(&feel_entity, cx, |draft| draft.camera_feel = index);
                    }
                },
            ),
        )
        .mt(px(8.)),
    );
    root = root.child(defaults);

    // Trackpad Navigation
    let action_entity = entity.clone();
    let modifier_entity = entity.clone();
    let mut trackpad = section(
        "hand",
        "Trackpad Navigation",
        "Configure two-finger gestures and alternate modifier behavior.",
    )
    .child(
        div()
            .mt(px(12.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(setting_row(
                "Trackpad navigation mode",
                "What a two-finger drag does. Pinch-to-zoom still works either way.",
                select_field(
                    "settings-trackpad-action",
                    &TRACKPAD_OPTIONS,
                    TRACKPAD_VALUES[state.trackpad_action.min(TRACKPAD_VALUES.len() - 1)],
                    None,
                    Some(CONTROL_WIDTH),
                    true,
                    action_entity.clone(),
                    move |value, cx| {
                        if let Some(index) = TRACKPAD_VALUES.iter().position(|action| *action == value)
                        {
                            edit_and_notify(&action_entity, cx, |draft| {
                                draft.trackpad_action = index;
                            });
                        }
                    },
                ),
            )),
    );

    if state.trackpad_action != 0 {
        let alternate = if state.trackpad_action == 1 { "orbit" } else { "pan" };
        trackpad = trackpad
            .child(setting_row(
                "Alternate gesture modifier",
                format!("Hold this key to temporarily switch two-finger drag to {alternate}."),
                segmented(
                    "settings-trackpad-modifier",
                    &["OPTION", "SHIFT"],
                    state.trackpad_modifier.min(1),
                    false,
                    move |index, cx| {
                        edit_and_notify(&modifier_entity, cx, |draft| {
                            draft.trackpad_modifier = index;
                        });
                    },
                ),
            ))
            .child(
                field_card()
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(TEXT_STRONG))
                            .child("Trackpad acceleration"),
                    )
                    .child(
                        div()
                            .mt(px(2.))
                            .text_size(px(12.))
                            .text_color(rgb(TEXT_MUTED))
                            .child("Tune movement speed for two-finger pan/orbit gestures."),
                    )
                    .child(
                        div()
                            .mt(px(8.))
                            .grid()
                            .grid_cols(2)
                            .gap(px(8.))
                            .child(slider_field(
                                "Pan acceleration",
                                format!("{:.2}×", state.pan_acceleration),
                                state.pan_acceleration,
                                0.4,
                                4.0))
                            .child(slider_field(
                                "Orbit acceleration",
                                format!("{:.2}×", state.orbit_acceleration),
                                state.orbit_acceleration,
                                0.4,
                                4.0))
                            .child(slider_field(
                                "Zoom acceleration",
                                format!("{:.2}×", state.zoom_acceleration),
                                state.zoom_acceleration,
                                0.4,
                                4.0)),
                    ),
            );
    }

    root.child(trackpad)
}

// ── Mesh ─────────────────────────────────────────────────────────────────

fn mesh_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let mut root = div().flex().flex_col().gap(px(12.));

    let total_light = state.ambient_intensity + state.directional_intensity;
    let lightness = total_light.clamp(0.0, 4.0);
    let contrast = if total_light > 0.0 {
        state.directional_intensity / total_light
    } else {
        0.5
    };

    let show_lighting = state.shader == 0 || state.shader == 4;
    let show_roughness = show_lighting;
    let show_baked_ao = state.shader == 0;
    // `matcap` and `flat_unlit` are not offered by `MESH_SHADER_OPTIONS`.
    let has_rendering_options =
        show_roughness || show_baked_ao || show_lighting || state.shader == 1;

    let shader_entity = entity.clone();
    let model_entity = entity.clone();
    let mut shader_section = section(
        "layers",
        "Shader & Preview",
        "Pick the shader type to configure here, and tune the mesh color. The camera dropdown chooses what the viewport renders.",
    );

    let mut selectors = div().mt(px(12.)).grid().grid_cols(2).gap(px(8.)).child(
        field_card()
            .child(field_label("Shader Type"))
            .child(select_field(
                "settings-shader",
                &SHADER_OPTIONS,
                SHADER_VALUES[state.shader.min(SHADER_VALUES.len() - 1)],
                None,
                Some(CONTROL_WIDTH),
                true,
                shader_entity.clone(),
                move |value, cx| {
                    if let Some(index) = SHADER_VALUES.iter().position(|shader| *shader == value) {
                        edit_and_notify(&shader_entity, cx, |draft| {
                            draft.shader = index;
                            if draft.shader == 1
                                && draft.active_color_index >= draft.heatmap_colors.len() + 1
                            {
                                draft.active_color_index = 0;
                            }
                        });
                    }
                },
            )),
    );
    selectors = selectors.child(
        field_card()
            .child(field_label("Preview Model"))
            .child(select_field(
                "settings-preview-model",
                &PREVIEW_MODEL_OPTIONS,
                PREVIEW_MODEL_VALUES[state.preview_model.min(PREVIEW_MODEL_VALUES.len() - 1)],
                None,
                Some(CONTROL_WIDTH),
                true,
                model_entity.clone(),
                move |value, cx| {
                    if let Some(index) = PREVIEW_MODEL_VALUES.iter().position(|model| *model == value)
                    {
                        edit_and_notify(&model_entity, cx, |draft| draft.preview_model = index);
                    }
                },
            )),
    );
    shader_section = shader_section.child(selectors);

    // Preview slot: the source embeds a wgpu canvas; here the slot is filled
    // with the mesh colour and labelled with the shader it would render.
    let preview = div()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(mix(hex_color(&state.mesh_color), SURFACE_0, 0.35)))
        .aspect_ratio(10.0 / 7.0)
        .child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .child(SHADER_LABELS[state.shader.min(SHADER_LABELS.len() - 1)]),
        );

    let active_index = state.active_color_index;
    let active_color = if active_index == 0 {
        state.mesh_color.clone()
    } else {
        state
            .heatmap_colors
            .get(active_index - 1)
            .cloned()
            .unwrap_or_else(|| DEFAULT_MESH_COLOR.to_owned())
    };
    let color_label = if active_index == 0 {
        "Mesh Color".to_owned()
    } else {
        format!("Heatmap {active_index}")
    };

    let mut color_card = div()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .p(px(8.))
        .flex()
        .flex_col()
        .gap(px(6.))
        .aspect_ratio(10.0 / 7.0)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .text_size(px(12.))
                        .flex_shrink_0()
                        .text_color(rgb(TEXT_MUTED))
                        .child(color_label),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .h(px(28.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .rounded(px(6.))
                        .border_1()
                        .border_color(rgb(BORDER_SUBTLE))
                        .bg(rgb(SURFACE_0))
                        .text_size(px(12.))
                        .text_color(rgb(TEXT_STRONG))
                        .child(active_color.clone()),
                ),
        )
        // The picker's saturation/value area: the colour under a white wash.
        .child(
            div()
                .relative()
                .flex_1()
                .min_h(px(0.))
                .rounded(px(4.))
                .overflow_hidden()
                .bg(rgb(hex_color(&active_color)))
                .child(
                    div()
                        .absolute()
                        .top(px(0.))
                        .left(px(0.))
                        .size_full()
                        .bg(linear_gradient(
                            90.,
                            linear_color_stop(rgba(0xffffffff), 0.),
                            linear_color_stop(rgba(0xffffff00), 1.),
                        )),
                ),
        );

    if state.shader == 1 {
        let mut strip = div()
            .flex()
            .items_center()
            .gap(px(2.))
            .pt(px(2.))
            .child(heatmap_swatch(
                0,
                &state.mesh_color,
                active_index == 0,
                entity.clone(),
            ))
            .child(
                div()
                    .w(px(1.))
                    .h(px(16.))
                    .mx(px(2.))
                    .flex_shrink_0()
                    .bg(rgb(BORDER_SUBTLE)),
            );
        for (index, color) in state.heatmap_colors.iter().enumerate() {
            strip = strip.child(heatmap_swatch(
                index + 1,
                color,
                active_index == index + 1,
                entity.clone(),
            ));
        }
        color_card = color_card.child(strip);
    }

    shader_section = shader_section.child(
        div()
            .mt(px(8.))
            .grid()
            .grid_cols(2)
            .gap(px(8.))
            .child(preview)
            .child(color_card),
    );

    if has_rendering_options {
        let mut options = div().mt(px(8.)).grid().grid_cols(2).gap(px(8.));

        if show_roughness {
            options = options.child(slider_field(
                "Roughness",
                format!("{:.2}", state.material_roughness),
                state.material_roughness,
                0.0,
                1.0));
        }
        if show_lighting {
            options = options.child(slider_field(
                "Lightness",
                format!("{:.2}", lightness),
                lightness,
                0.2,
                3.0));
            options = options.child(slider_field(
                "Contrast",
                format!("{:.2}", contrast),
                contrast,
                0.05,
                0.95));
        }
        if show_baked_ao {
            options = options.child(slider_field(
                "Ambient Occlusion",
                format!("{:.2}", state.baked_ao_intensity),
                state.baked_ao_intensity,
                0.0,
                2.0));
        }
        if state.shader == 4 {
            options = options.child(slider_field(
                "X-Ray Opacity",
                format!("{:.2}", state.xray_opacity),
                state.xray_opacity,
                0.02,
                0.85));
        }
        if state.shader == 1 {
            options = options.child(slider_field(
                "Min Overhang",
                format!("{}°", state.heatmap_min_angle.round() as i32),
                state.heatmap_min_angle,
                0.0,
                90.0));
            options = options.child(slider_field(
                "Max Overhang",
                format!("{}°", state.heatmap_max_angle.round() as i32),
                state.heatmap_max_angle,
                0.0,
                90.0));
        }

        shader_section = shader_section.child(divider_label(
            "sliders-horizontal",
            "Rendering Options",
        ));
        shader_section = shader_section.child(options);
    }

    root = root.child(shader_section);

    // Selection & Hover
    let selection_entity = entity.clone();
    let hover_entity = entity.clone();
    let preview_entity = entity.clone();
    let preview_hover_entity = entity.clone();

    let selection_card = field_card()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .mb(px(8.))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("Colors"),
                )
                .child(
                    div()
                        .id("settings-reset-selection-colors")
                        .w(px(24.))
                        .h(px(24.))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(6.))
                        .border_1()
                        .border_color(rgb(BORDER_SUBTLE))
                        .bg(rgb(SURFACE_1))
                        .cursor_pointer()
                        .hover(|style| style.bg(rgb(SURFACE_2)))
                        .child(icon("rotate-ccw", 12., TEXT_STRONG))
                        .on_click(move |_, _, cx| {
                            edit_and_notify(&selection_entity, cx, |draft| {
                                let accent = THEME_COLOR_SECTIONS
                                    .iter()
                                    .flat_map(|section| section.rows)
                                    .find(|row| row.key == "meshSelectionColor")
                                    .map(|row| row.placeholder.to_owned())
                                    .unwrap_or_else(|| "#ec2a77".to_owned());
                                draft
                                    .theme_colors
                                    .insert("meshSelectionColor", accent.clone());
                                draft.theme_colors.insert("meshHoverColor", accent);
                            });
                        }),
                ),
        )
        .child(
            div()
                .grid()
                .grid_cols(2)
                .gap(px(8.))
                .child(color_field(
                    "Selection Color",
                    state
                        .theme_colors
                        .get("meshSelectionColor")
                        .cloned()
                        .unwrap_or_else(|| "#ec2a77".to_owned()),
                    "settings-selection-color",
                    entity.clone(),
                    "meshSelectionColor",
                ))
                .child(color_field(
                    "Hover Color",
                    state
                        .theme_colors
                        .get("meshHoverColor")
                        .cloned()
                        .unwrap_or_else(|| "#ec2a77".to_owned()),
                    "settings-hover-color",
                    hover_entity,
                    "meshHoverColor",
                )),
        );

    let tint_card = field_card()
        .child(
            div()
                .mb(px(8.))
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child("Tint Intensity"),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(slider_field(
                    "Hover",
                    format!("{:.2}", state.hover_tint_strength),
                    state.hover_tint_strength,
                    0.0,
                    1.0))
                .child(slider_field(
                    "Selected",
                    format!("{:.2}", state.selected_tint_strength),
                    state.selected_tint_strength,
                    0.0,
                    1.0)),
        );

    let badge = if state.preview_selected {
        (
            "selected",
            TEXT_STRONG,
            mix(ACCENT, 0xffffff, 0.88),
            mix(ACCENT, SURFACE_0, 0.78),
        )
    } else if state.preview_hovered {
        ("hovered", TEXT_STRONG, BORDER_STRONG, SURFACE_2)
    } else {
        ("idle", TEXT_MUTED, BORDER_SUBTLE, SURFACE_0)
    };
    let preview_tint = if state.preview_selected {
        mix(
            hex_color(
                state
                    .theme_colors
                    .get("meshSelectionColor")
                    .map(String::as_str)
                    .unwrap_or("#ec2a77"),
            ),
            SURFACE_0,
            state.selected_tint_strength,
        )
    } else {
        mix(
            hex_color(
                state
                    .theme_colors
                    .get("meshHoverColor")
                    .map(String::as_str)
                    .unwrap_or("#ec2a77"),
            ),
            SURFACE_0,
            state.hover_tint_strength,
        )
    };

    let preview_card = field_card()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("Selection Preview"),
                )
                .child(
                    div()
                        .px(px(8.))
                        .py(px(2.))
                        .rounded_full()
                        .border_1()
                        .border_color(rgb(badge.2))
                        .bg(rgb(badge.3))
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(badge.1))
                        .child(badge.0),
                ),
        )
        .child(
            div()
                .id("settings-selection-preview")
                .mt(px(8.))
                .flex_1()
                .min_h(px(0.))
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(preview_tint))
                .cursor_pointer()
                .on_hover(move |hovered, _, cx| {
                    edit_and_notify(&preview_hover_entity, cx, |draft| {
                        draft.preview_hovered = *hovered;
                    });
                })
                .on_click(move |_, _, cx| {
                    edit_and_notify(&preview_entity, cx, |draft| {
                        draft.preview_selected = !draft.preview_selected;
                    });
                }),
        );

    root.child(
        section(
            "mouse-pointer-2",
            "Selection & Hover",
            "How selected and hovered models are emphasized throughout the app. These two colors come from the current theme — editing them here changes the theme, exactly like UI & Theme › Mesh Highlights.",
        )
        .child(
            div()
                .mt(px(12.))
                .grid()
                .grid_cols(2)
                .gap(px(8.))
                .child(
                    div()
                        .min_w(px(0.))
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(selection_card)
                        .child(tint_card),
                )
                .child(preview_card),
        ),
    )
}

fn heatmap_swatch(
    index: usize,
    color: &str,
    active: bool,
    entity: WeakEntity<DragonFruit>,
) -> Stateful<Div> {
    let first = index == 0;
    div()
        .id(format!("settings-heatmap-swatch-{index}"))
        .when(first, |this| this.w(px(20.)).h(px(20.)))
        .when(!first, |this| this.flex_1().h(px(20.)))
        .rounded(px(4.))
        .border_1()
        .border_color(rgb(if active { TEXT_STRONG } else { BORDER_STRONG }))
        .bg(rgb(hex_color(color)))
        .cursor_pointer()
        .on_click(move |_, _, cx| {
            edit_and_notify(&entity, cx, |draft| {
                draft.active_color_index = index;
            });
        })
}

// ── Slicing ──────────────────────────────────────────────────────────────

fn slicing_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let mut root = div().flex().flex_col().gap(px(12.));

    root = root.child(
        section(
            "cpu",
            "Slicing Engine",
            "Native Rust slicing via the DragonFruit engine.",
        )
        .child(
            inner_card()
                .mt(px(12.))
                .child(
                    div()
                        .grid()
                        .grid_cols(2)
                        .gap(px(8.))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(rgb(TEXT_MUTED))
                                        .child("Crate"),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(TEXT_STRONG))
                                        .child("dragonfruit-slicing-engine"),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(rgb(TEXT_MUTED))
                                        .child("Version"),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(TEXT_STRONG))
                                        .child("-"),
                                ),
                        ),
                ),
        ),
    );

    let gradient_entity = entity.clone();
    let plate_entity = entity.clone();
    let grid_entity = entity.clone();
    root = root.child(
        section(
            "image",
            "Thumbnail Rendering",
            "Configure what appears in generated export/slice thumbnails.",
        )
        .child(
            inner_card()
                .mt(px(12.))
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(plain_row(
                    "Background gradient",
                    "Scene mood overlay in thumbnail",
                    segmented(
                        "settings-thumbnail-gradient",
                        &["OFF", "ON"],
                        usize::from(state.thumbnail_gradient),
                        false,
                        move |index, cx| {
                            edit_and_notify(&gradient_entity, cx, |draft| {
                                draft.thumbnail_gradient = index == 1;
                            });
                        },
                    ),
                ))
                .child(plain_row(
                    "Build plate",
                    "Render build plate in thumbnail",
                    segmented(
                        "settings-thumbnail-plate",
                        &["OFF", "ON"],
                        usize::from(state.thumbnail_build_plate),
                        false,
                        move |index, cx| {
                            edit_and_notify(&plate_entity, cx, |draft| {
                                draft.thumbnail_build_plate = index == 1;
                            });
                        },
                    ),
                ))
                .child(plain_row(
                    "Grid",
                    "Render build grid in thumbnail",
                    segmented(
                        "settings-thumbnail-grid",
                        &["OFF", "ON"],
                        usize::from(state.thumbnail_grid),
                        false,
                        move |index, cx| {
                            edit_and_notify(&grid_entity, cx, |draft| {
                                draft.thumbnail_grid = index == 1;
                            });
                        },
                    ),
                )),
        ),
    );

    let compression_entity = entity.clone();
    root = root.child(
        section(
            "zap",
            "PNG Compression",
            "Enable or disable PNG compression for PNG-based formats.",
        )
        .child(
            inner_card().mt(px(12.)).child(plain_row(
                "Compression",
                "Auto adapts by AA level, Off is fastest, On favors smaller PNG files",
                segmented(
                    "settings-png-compression",
                    &["AUTO", "OFF", "ON"],
                    state.png_compression.min(PNG_MODE_LABELS.len() - 1),
                    false,
                    move |index, cx| {
                        edit_and_notify(&compression_entity, cx, |draft| {
                            draft.png_compression = index;
                        });
                    },
                ),
            )),
        ),
    );

    let stale_entity = entity.clone();
    let all_entity = entity.clone();
    root.child(
        section(
            "trash-2",
            "Temp File Cleanup",
            "Free disk space by removing temporary slice files.",
        )
        .child(
            div()
                .mt(px(12.))
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(cleanup_button(
                    "settings-clean-stale",
                    "Clean Stale Files",
                    "Remove temp files older than 1 hour",
                    false,
                    move |cx| {
                        stale_entity.update(cx, |_, cx| cx.notify()).ok();
                    },
                ))
                .child(cleanup_button(
                    "settings-clean-all",
                    "Clean All Files",
                    "Emergency cleanup: delete all temp slices",
                    true,
                    move |cx| {
                        all_entity.update(cx, |_, cx| cx.notify()).ok();
                    },
                )),
        ),
    )
}

fn cleanup_button(
    id: &'static str,
    title: &'static str,
    description: &'static str,
    danger: bool,
    on_click: impl Fn(&mut App) + 'static,
) -> Stateful<Div> {
    let hover = if danger {
        mix(DANGER, SURFACE_0, 0.05)
    } else {
        mix(ACCENT, SURFACE_0, 0.08)
    };
    div()
        .id(id)
        .w_full()
        .p(px(10.))
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .flex()
        .flex_col()
        .cursor_pointer()
        .hover(move |style| style.bg(rgb(hover)))
        .on_click(move |_, _, cx| on_click(cx))
        .child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child(title),
        )
        .child(
            div()
                .mt(px(2.))
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child(description),
        )
}

// ── UI & Theme ───────────────────────────────────────────────────────────

fn ui_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let mut root = div().flex().flex_col().gap(px(10.));

    let preset_entity = entity.clone();
    let scheme_entity = entity.clone();
    let reset_entity = entity.clone();
    let theme_section = div()
        .rounded(px(12.)) // rounded-xl
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(10.))
        .child(
            div()
                .mb(px(8.))
                .flex()
                .items_start()
                .gap(px(8.))
                .child(icon_chip("palette"))
                .child(
                    div()
                        .flex_1()
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child("Theme"),
                        )
                        .child(
                            div()
                                .mt(px(2.))
                                .text_size(px(12.))
                                .line_height(relative(1.2))
                                .text_color(rgb(TEXT_MUTED))
                                .child("Preview is live. Apply saves it."),
                        ),
                ),
        )
        .child(
            div()
                .grid()
                .grid_cols(2)
                .gap(px(8.))
                .child(
                    div()
                        .rounded(px(8.)) // rounded-lg
                        .border_1()
                        .border_color(rgb(BORDER_SUBTLE))
                        .bg(rgb(SURFACE_2))
                        .p(px(10.))
                        .child(upper_label("Theme preset"))
                        .child(select_field(
                            "settings-theme-preset",
                            &THEME_PRESET_OPTIONS,
                            THEME_PRESET_VALUES[state.theme_preset.min(THEME_PRESET_VALUES.len() - 1)],
                            None,
                            Some(CONTROL_WIDTH),
                            true,
                            preset_entity.clone(),
                            move |value, cx| {
                                if let Some(index) =
                                    THEME_PRESET_VALUES.iter().position(|preset| *preset == value)
                                {
                                    edit_and_notify(&preset_entity, cx, |draft| {
                                        draft.theme_preset = index;
                                    });
                                }
                            },
                        )),
                )
                .child(
                    div()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(rgb(BORDER_SUBTLE))
                        .bg(rgb(SURFACE_2))
                        .p(px(10.))
                        // `opacity: isBuiltInThemePreset ? 0.5 : 1`
                        .opacity(0.5)
                        .child(upper_label("Color scheme"))
                        .child(select_field(
                            "settings-theme-scheme",
                            &SCHEME_OPTIONS,
                            SCHEME_VALUES[state.theme_preference.min(SCHEME_VALUES.len() - 1)],
                            None,
                            Some(CONTROL_WIDTH),
                            false,
                            scheme_entity.clone(),
                            move |value, cx| {
                                if let Some(index) =
                                    SCHEME_VALUES.iter().position(|scheme| *scheme == value)
                                {
                                    edit_and_notify(&scheme_entity, cx, |draft| {
                                        draft.theme_preference = index;
                                    });
                                }
                            },
                        )),
                ),
        )
        .child(
            div()
                .mt(px(8.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        // Theme import/export is not ported; the button keeps the
                        // source layout and reads as disabled.
                        .child(small_button(
                            "settings-theme-import",
                            "Import",
                            false,
                            |_| {},
                        )),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(small_button(
                            "settings-theme-reset",
                            "Reset",
                            true,
                            move |cx| {
                                edit_and_notify(&reset_entity, cx, |draft| {
                                    for section in THEME_COLOR_SECTIONS.iter() {
                                        for row in section.rows {
                                            draft
                                                .theme_colors
                                                .insert(row.key, row.placeholder.to_owned());
                                        }
                                    }
                                });
                            },
                        )),
                ),
        );

    root = root.child(theme_section);

    // Sub-tab strip
    let general_entity = entity.clone();
    let mesh_entity = entity.clone();
    let cut_entity = entity.clone();
    root = root.child(
        div()
            .flex()
            .items_center()
            .gap(px(6.))
            .border_b_1()
            .border_color(rgb(BORDER_SUBTLE))
            .pb(px(8.))
            .child(theme_sub_tab(
                "settings-theme-tab-general",
                "General UI",
                state.theme_tab == 0,
                move |cx| {
                    edit_and_notify(&general_entity, cx, |draft| draft.theme_tab = 0);
                },
            ))
            .child(theme_sub_tab(
                "settings-theme-tab-mesh",
                "Mesh Highlights",
                state.theme_tab == 1,
                move |cx| {
                    edit_and_notify(&mesh_entity, cx, |draft| draft.theme_tab = 1);
                },
            ))
            .child(theme_sub_tab(
                "settings-theme-tab-cut",
                "Cutting Tool",
                state.theme_tab == 2,
                move |cx| {
                    edit_and_notify(&cut_entity, cx, |draft| draft.theme_tab = 2);
                },
            )),
    );

    match state.theme_tab {
        0 => {
            let mut grid = div().grid().grid_cols(2).gap(px(10.));
            for section in THEME_COLOR_SECTIONS.iter() {
                let mut card = div()
                    .rounded(px(12.))
                    .border_1()
                    .border_color(rgb(BORDER_SUBTLE))
                    .bg(rgb(SURFACE_1))
                    .p(px(10.))
                    .child(
                        div()
                            .mb(px(8.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(TEXT_STRONG))
                                    .child(section.title),
                            )
                            .child(
                                div()
                                    .mt(px(2.))
                                    .text_size(px(11.))
                                    .line_height(relative(1.2))
                                    .text_color(rgb(TEXT_MUTED))
                                    .child(section.description),
                            ),
                    );
                let mut rows = div().flex().flex_col().gap(px(6.));
                for row in section.rows {
                    let value = state
                        .theme_colors
                        .get(row.key)
                        .cloned()
                        .unwrap_or_else(|| row.placeholder.to_owned());
                    rows = rows.child(color_row(row, value));
                }
                card = card.child(rows);
                grid = grid.child(card);
            }
            root.child(grid)
        }
        1 => {
            let mut card = div()
                .rounded(px(12.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_1))
                .p(px(10.))
                .child(
                    div()
                        .mb(px(8.))
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child("Mesh Highlights"),
                        )
                        .child(
                            div()
                                .mt(px(2.))
                                .text_size(px(11.))
                                .line_height(relative(1.2))
                                .text_color(rgb(TEXT_MUTED))
                                .child("Selection and hover tint for models in the 3D view. Also editable from the Mesh tab, which mirrors these two theme colors."),
                        ),
                );
            let mut grid = div().grid().grid_cols(2).gap(px(6.));
            for row in MESH_HIGHLIGHT_COLOR_FIELDS.iter() {
                let value = state
                    .theme_colors
                    .get(row.key)
                    .cloned()
                    .unwrap_or_else(|| row.placeholder.to_owned());
                grid = grid.child(color_row(row, value));
            }
            card = card.child(grid);
            root.child(card)
        }
        _ => {
            let mut card = div()
                .rounded(px(12.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_1))
                .p(px(10.))
                .child(
                    div()
                        .mb(px(8.))
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child("Cut Tool Colors"),
                        )
                        .child(
                            div()
                                .mt(px(2.))
                                .text_size(px(11.))
                                .line_height(relative(1.2))
                                .text_color(rgb(TEXT_MUTED))
                                .child("Seam, cut surface, tenon, mortise, and waypoint colors for the Cut tool."),
                        ),
                );
            let mut grid = div().grid().grid_cols(2).gap(px(6.));
            for field in CUT_COLOR_FIELDS.iter() {
                let value = state
                    .cut_colors
                    .get(field.key)
                    .cloned()
                    .unwrap_or_else(|| field.placeholder.to_owned());
                grid = grid.child(color_row(field, value));
            }
            card = card.child(grid);
            root.child(card)
        }
    }
}

fn theme_sub_tab(
    id: &'static str,
    label: &'static str,
    active: bool,
    on_click: impl Fn(&mut App) + 'static,
) -> Stateful<Div> {
    div()
        .id(id)
        .h(px(28.))
        .px(px(10.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(if active {
            mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.58)
        } else {
            BORDER_SUBTLE
        }))
        .bg(rgb(SURFACE_1))
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(if active { ACCENT_SECONDARY } else { TEXT_MUTED }))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(SURFACE_2)))
        .on_click(move |_, _, cx| on_click(cx))
        .child(label)
}

/// One theme colour entry: label on the left, swatch and stored hex on the
/// right (`grid-cols-[minmax(0,1fr)_10.75rem]`).
fn color_row(field: &'static ColorField, value: String) -> Div {
    let label = field.label;
    let description = field.description;
    div()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        // color-mix(in srgb, surface-0, transparent 8%) over surface-1
        .bg(rgb(mix(SURFACE_0, SURFACE_1, 0.92)))
        .px(px(8.))
        .py(px(6.))
        .child(
            div()
                .grid()
                .grid_cols(2)
                .gap(px(10.))
                .items_center()
                .child(
                    div()
                        .min_w(px(0.))
                        .truncate()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child(label),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(swatch(&value, 32., 28.))
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.))
                                .h(px(28.))
                                .px(px(8.))
                                .flex()
                                .items_center()
                                .rounded(px(6.))
                                .border_1()
                                .border_color(rgb(BORDER_SUBTLE))
                                .bg(rgb(SURFACE_0))
                                .text_size(px(12.))
                                .text_color(rgb(TEXT_STRONG))
                                .child(value),
                        ),
                ),
        )
        .child(
            div()
                .mt(px(2.))
                .text_size(px(11.))
                .text_color(rgb(TEXT_MUTED))
                .child(description),
        )
}

/// The Mesh tab's Selection/Hover colour input (`ColorSwatchInput` + `.ui-input`).
fn color_field(
    label: &'static str,
    value: String,
    id: &'static str,
    entity: WeakEntity<DragonFruit>,
    key: &'static str,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child(label),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .id(format!("{id}-swatch"))
                        .cursor_pointer()
                        .child(swatch(&value, 40., 32.))
                        .on_click(move |_, _, cx| {
                            // A native colour picker is not available; cycling the
                            // stored value keeps the control live.
                            edit_and_notify(&entity, cx, |draft| {
                                let current = draft
                                    .theme_colors
                                    .get(key)
                                    .cloned()
                                    .unwrap_or_else(|| "#ec2a77".to_owned());
                                let next = next_theme_color(&current);
                                draft.theme_colors.insert(key, next);
                            });
                        }),
                )
                .child(
                    div()
                        .w(px(120.)) // w-[7.5rem]
                        .min_w(px(0.))
                        .h(px(32.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .rounded(px(6.))
                        .border_1()
                        .border_color(rgb(BORDER_SUBTLE))
                        .bg(rgb(SURFACE_0))
                        .text_size(px(12.))
                        .text_color(rgb(TEXT_STRONG))
                        .child(value),
                ),
        )
}

fn next_theme_color(current: &str) -> String {
    let palette = ["#ec2a77", "#baf72e", "#487dd5", "#0a667c", "#e45454", "#2eb67d"];
    let index = palette
        .iter()
        .position(|color| color.eq_ignore_ascii_case(current))
        .map(|index| (index + 1) % palette.len())
        .unwrap_or(0);
    palette[index].to_owned()
}

// ── Shared pieces ────────────────────────────────────────────────────────

struct ColorField {
    key: &'static str,
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
}

struct ColorSection {
    title: &'static str,
    description: &'static str,
    rows: &'static [ColorField],
}

static FOUNDATION_ROWS: [ColorField; 5] = [
    ColorField { key: "background", label: "App background", description: "Outer app frame and page background.", placeholder: "#0b0f14" },
    ColorField { key: "foreground", label: "App foreground", description: "Top-level body text fallback color.", placeholder: "#e6ebf2" },
    ColorField { key: "surface0", label: "Surface 0", description: "Main modal and panel base surface.", placeholder: "#111216" },
    ColorField { key: "surface1", label: "Surface 1", description: "Raised cards, tool panes, and section fills.", placeholder: "#1a1b21" },
    ColorField { key: "surface2", label: "Surface 2", description: "Secondary tiles and inset controls.", placeholder: "#23252e" },
];

static CONTENT_ROWS: [ColorField; 5] = [
    ColorField { key: "textStrong", label: "Text strong", description: "Primary headings and high-contrast labels.", placeholder: "#f8f8fb" },
    ColorField { key: "textMuted", label: "Text muted", description: "Supporting labels, hints, and metadata.", placeholder: "#c3c7cf" },
    ColorField { key: "indicator", label: "Indicator", description: "Neutral dots, markers, and status indicators.", placeholder: "#c3c7cf" },
    ColorField { key: "borderSubtle", label: "Border subtle", description: "Low-contrast panel and input outlines.", placeholder: "#272a33" },
    ColorField { key: "borderStrong", label: "Border strong", description: "Higher-contrast structural dividers.", placeholder: "#353944" },
];

static PRIMARY_BRAND_ROWS: [ColorField; 5] = [
    ColorField { key: "accent", label: "Accent", description: "Primary highlight, active icons, and focus color.", placeholder: "#ec2a77" },
    ColorField { key: "accentHover", label: "Accent hover", description: "Hover/pressed state for primary accent actions.", placeholder: "#d81d67" },
    ColorField { key: "primaryButtonSurface", label: "Primary button surface", description: "Filled primary buttons and pills.", placeholder: "#c11f61" },
    ColorField { key: "accentContrast", label: "Accent contrast", description: "Text/icons shown on primary accent fills.", placeholder: "#fff6ff" },
    ColorField { key: "topbarAccent", label: "Top bar accent", description: "Glow and accent wash used by the app bar.", placeholder: "#ec2a77" },
];

static SECONDARY_BRAND_ROWS: [ColorField; 4] = [
    ColorField { key: "accentSecondary", label: "Accent secondary", description: "Secondary accent and approved-action color.", placeholder: "#baf72e" },
    ColorField { key: "accentSecondaryHover", label: "Secondary hover", description: "Hover/pressed state for green actions.", placeholder: "#a6df29" },
    ColorField { key: "secondaryButtonSurface", label: "Secondary button surface", description: "Filled secondary buttons and badges.", placeholder: "#9bcc26" },
    ColorField { key: "accentSecondaryContrast", label: "Secondary contrast", description: "Text/icons shown on green fills.", placeholder: "#182106" },
];

static SCENE_ROWS: [ColorField; 3] = [
    ColorField { key: "sceneGradientRadial", label: "3D radial glow", description: "Radial color bloom in the scene backdrop.", placeholder: "#ff37aa" },
    ColorField { key: "sceneGradientLinearStart", label: "3D gradient top", description: "Top stop of the linear scene gradient.", placeholder: "#ff37aa" },
    ColorField { key: "sceneGradientLinearMid", label: "3D gradient mid", description: "Middle stop of the linear scene gradient.", placeholder: "#6f33ff" },
];

static STATUS_ROWS: [ColorField; 2] = [
    ColorField { key: "success", label: "Success", description: "Success states, confirmations, and healthy signals.", placeholder: "#2eb67d" },
    ColorField { key: "danger", label: "Danger", description: "Destructive actions, warnings, and errors.", placeholder: "#e45454" },
];

static THEME_COLOR_SECTIONS: [ColorSection; 6] = [
    ColorSection { title: "Foundation surfaces", description: "Core app backgrounds and panel surfaces.", rows: &FOUNDATION_ROWS },
    ColorSection { title: "Content contrast", description: "Typography, borders, and neutral UI signals.", rows: &CONTENT_ROWS },
    ColorSection { title: "Primary brand accent", description: "Primary action styling, highlights, and key brand colors.", rows: &PRIMARY_BRAND_ROWS },
    ColorSection { title: "Secondary brand accent", description: "Secondary action styling, complementary highlights, and secondary brand colors.", rows: &SECONDARY_BRAND_ROWS },
    ColorSection { title: "Scene accents", description: "3D view gradient treatment and scene chrome.", rows: &SCENE_ROWS },
    ColorSection { title: "Semantic status colors", description: "Feedback colors for success and destructive actions.", rows: &STATUS_ROWS },
];

static MESH_HIGHLIGHT_COLOR_FIELDS: [ColorField; 2] = [
    ColorField { key: "meshSelectionColor", label: "Selection color", description: "Tint applied to selected models.", placeholder: "#ec2a77" },
    ColorField { key: "meshHoverColor", label: "Hover color", description: "Tint applied to hovered models.", placeholder: "#ec2a77" },
];

static CUT_COLOR_FIELDS: [ColorField; 16] = [
    ColorField { key: "seam", label: "Seam", description: "The cut line you draw.", placeholder: "#00008b" },
    ColorField { key: "seamHover", label: "Seam (hover)", description: "The seam while the cursor is over it.", placeholder: "#c9752e" },
    ColorField { key: "seamInactive", label: "Seam (other loops)", description: "Loops of a multi-loop cut you are not editing.", placeholder: "#5e3418" },
    ColorField { key: "seamGlow", label: "Seam glow", description: "Halo around the hovered seam.", placeholder: "#ffe6cc" },
    ColorField { key: "cutSurface", label: "Cut surface", description: "The contour membrane and the flat cut plane.", placeholder: "#37ff7a" },
    ColorField { key: "tenonFront", label: "Tenon (near faces)", description: "The tenon's faces turned toward you.", placeholder: "#001e57" },
    ColorField { key: "tenonBack", label: "Tenon (far faces)", description: "Far faces, darker so the shape reads solid.", placeholder: "#8a4a08" },
    ColorField { key: "tenonEdge", label: "Tenon edges", description: "The tenon's silhouette lines.", placeholder: "#ff7a00" },
    ColorField { key: "mortiseFront", label: "Mortise (near faces)", description: "The hole carved in the other half.", placeholder: "#c07bff" },
    ColorField { key: "mortiseBack", label: "Mortise (far faces)", description: "Far faces, darker so the shape reads solid.", placeholder: "#4a2a7a" },
    ColorField { key: "mortiseEdge", label: "Mortise edges", description: "The mortise's silhouette lines.", placeholder: "#a24bff" },
    ColorField { key: "tenonHandle", label: "Tenon handle", description: "The dot you drag to slide the tenon.", placeholder: "#0091ff" },
    ColorField { key: "markerFirst", label: "First waypoint", description: "The point the loop starts from.", placeholder: "#37ff7a" },
    ColorField { key: "markerPoint", label: "Waypoint", description: "Every other point on the loop.", placeholder: "#ffd24a" },
    ColorField { key: "markerSelected", label: "Waypoint (selected)", description: "The point you clicked.", placeholder: "#0091ff" },
    ColorField { key: "markerDragging", label: "Waypoint (dragging)", description: "The point being dragged.", placeholder: "#35e3ff" },
];

// ── Workspace (`WorkspacesSettingsTab.tsx`) ──────────────────────────────

/// `<label class="text-xs">` with the control under it.
fn number_label(label: &'static str, control: Div) -> Div {
    div()
        .flex()
        .flex_col()
        .text_size(px(12.))
        .text_color(rgb(TEXT_MUTED))
        .child(label)
        .child(div().mt(px(4.)).child(control))
}

fn workspace_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let view = &state.view3d;

    let mut root = div().flex().flex_col().gap(px(12.));
    let enabled_entity = entity.clone();
    let mut inner = div().mt(px(12.)).flex().flex_col();

    // `isBuildVolumeManagedByPrinter` is false here: the slice has no printer
    // profile store, so the source's notice branch never renders.
    inner = inner.child(setting_row(
        "Enable build volume bounds",
        "Shows a faint printer volume outline and enables out-of-bounds checks.",
        segmented(
            "view3d-enabled",
            &["OFF", "ON"],
            usize::from(view.enabled),
            false,
            move |index, cx| {
                edit_and_notify(&enabled_entity, cx, |draft| {
                    draft.view3d.enabled = index == 1;
                });
            },
        ),
    ));

    if view.enabled {
        let width_entity = entity.clone();
        let depth_entity = entity.clone();
        let max_z_entity = entity.clone();
        let origin_entity = entity.clone();
        let screen_w_entity = entity.clone();
        let screen_h_entity = entity.clone();

        let grid = div()
            .mt(px(8.))
            .grid()
            .grid_cols(2)
            .gap(px(8.))
            .child(number_label(
                "Build Width (mm)",
                number_input(
                    "view3d-width",
                    view.width_mm,
                    10.0,
                    f32::INFINITY,
                    1.0,
                    None,
                    36.0, // h-9
                    12.0,
                    SURFACE_1,
                    true,
                    move |next, cx| {
                        edit_and_notify(&width_entity, cx, |draft| {
                            draft.view3d.width_mm = next.max(10.0).round();
                        });
                    },
                ),
            ))
            .child(number_label(
                "Build Depth (mm)",
                number_input(
                    "view3d-depth",
                    view.depth_mm,
                    10.0,
                    f32::INFINITY,
                    1.0,
                    None,
                    36.0,
                    12.0,
                    SURFACE_1,
                    true,
                    move |next, cx| {
                        edit_and_notify(&depth_entity, cx, |draft| {
                            draft.view3d.depth_mm = next.max(10.0).round();
                        });
                    },
                ),
            ))
            .child(number_label(
                "Max Z Height (mm)",
                number_input(
                    "view3d-max-z",
                    view.max_z_mm,
                    10.0,
                    f32::INFINITY,
                    1.0,
                    None,
                    36.0,
                    12.0,
                    SURFACE_1,
                    true,
                    move |next, cx| {
                        edit_and_notify(&max_z_entity, cx, |draft| {
                            draft.view3d.max_z_mm = next.max(10.0).round();
                        });
                    },
                ),
            ))
            .child(
                div()
                    .rounded(px(6.))
                    .border_1()
                    .border_color(rgb(BORDER_SUBTLE))
                    .bg(rgb(SURFACE_1))
                    .px(px(8.))
                    .py(px(6.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(TEXT_MUTED))
                            .child("Build Volume"),
                    )
                    .child(
                        div()
                            .mt(px(2.))
                            .text_size(px(12.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(TEXT_STRONG))
                            .child(format!(
                                "{} × {} × {} mm",
                                view.width_mm.round() as i32,
                                view.depth_mm.round() as i32,
                                view.max_z_mm.round() as i32
                            )),
                    ),
            );

        // The source puts the origin row in the same grid with `col-span-2`;
        // splitting the grid keeps the full-width row without a column span.
        inner = inner.child(grid);
        inner = inner.child(
            setting_row(
                "Build volume origin",
                "Choose where XYZ 0,0,0 is located for the printer volume.",
                segmented(
                    "view3d-origin",
                    &["CENTER", "FRONT-LEFT CORNER"],
                    match view.origin_mode {
                        OriginMode::Center => 0,
                        OriginMode::FrontLeft => 1,
                    },
                    false,
                    move |index, cx| {
                        edit_and_notify(&origin_entity, cx, |draft| {
                            draft.view3d.origin_mode = if index == 0 {
                                OriginMode::Center
                            } else {
                                OriginMode::FrontLeft
                            };
                        });
                    },
                ),
            )
            .mt(px(8.)),
        );
        inner = inner.child(
            div()
                .mt(px(8.))
                .grid()
                .grid_cols(2)
                .gap(px(8.))
                .child(number_label(
                    "Screen Width (px)",
                    number_input(
                        "view3d-screen-w",
                        view.screen_width_px,
                        320.0,
                        f32::INFINITY,
                        1.0,
                        None,
                        36.0,
                        12.0,
                        SURFACE_1,
                        true,
                        move |next, cx| {
                            edit_and_notify(&screen_w_entity, cx, |draft| {
                                draft.view3d.screen_width_px = next.max(320.0).round();
                            });
                        },
                    ),
                ))
                .child(number_label(
                    "Screen Height (px)",
                    number_input(
                        "view3d-screen-h",
                        view.screen_height_px,
                        200.0,
                        f32::INFINITY,
                        1.0,
                        None,
                        36.0,
                        12.0,
                        SURFACE_1,
                        true,
                        move |next, cx| {
                            edit_and_notify(&screen_h_entity, cx, |draft| {
                                draft.view3d.screen_height_px = next.max(200.0).round();
                            });
                        },
                    ),
                )),
        );
    }

    let violation_entity = entity.clone();
    let boxes_entity = entity.clone();
    let sat_entity = entity.clone();
    inner = inner
        .child(
            setting_row(
                "Show out-of-bounds warnings",
                "Warn when any visible model extends beyond the configured build volume.",
                segmented(
                    "view3d-violation",
                    &["OFF", "ON"],
                    usize::from(view.show_violation_warning),
                    false,
                    move |index, cx| {
                        edit_and_notify(&violation_entity, cx, |draft| {
                            draft.view3d.show_violation_warning = index == 1;
                        });
                    },
                ),
            )
            .mt(px(8.)),
        )
        .child(
            setting_row(
                "Show model bounding boxes",
                "Debug overlay: draws world-space bounds for each visible model (red if out-of-bounds).",
                segmented(
                    "view3d-boxes",
                    &["OFF", "ON"],
                    usize::from(view.show_model_bounding_boxes),
                    false,
                    move |index, cx| {
                        edit_and_notify(&boxes_entity, cx, |draft| {
                            draft.view3d.show_model_bounding_boxes = index == 1;
                        });
                    },
                ),
            )
            .mt(px(8.)),
        )
        .child(
            setting_row(
                "Show slice SAT bounding mesh",
                "SAT debug overlay for nesting and diagnostics.",
                segmented(
                    "view3d-sat",
                    &["OFF", "ON"],
                    usize::from(view.show_slice_sat),
                    false,
                    move |index, cx| {
                        edit_and_notify(&sat_entity, cx, |draft| {
                            draft.view3d.show_slice_sat = index == 1;
                        });
                    },
                ),
            )
            .mt(px(8.)),
        );

    if view.show_slice_sat {
        let scope_entity = entity.clone();
        let mode_entity = entity.clone();
        let render_entity = entity.clone();
        inner = inner.child(
            setting_row(
                "SAT debug scope",
                "Show SAT mesh on the active model only, or on all visible models.",
                segmented(
                    "view3d-sat-scope",
                    &["ACTIVE ONLY", "ALL MODELS"],
                    usize::from(view.sat_all_models),
                    false,
                    move |index, cx| {
                        edit_and_notify(&scope_entity, cx, |draft| {
                            draft.view3d.sat_all_models = index == 1;
                        });
                    },
                ),
            )
            .mt(px(8.)),
        );
        inner = inner.child(
            setting_row(
                "SAT mode",
                "Choose accurate convex-hull SAT for nesting, or experimental slice-derived SAT for diagnostics.",
                segmented(
                    "view3d-sat-mode",
                    &["ACCURATE HULL-BASED SAT", "EXPERIMENTAL SLICE-BASED SAT"],
                    match view.sat_mode {
                        SliceSatMode::AccurateHull => 0,
                        SliceSatMode::ExperimentalSlice => 1,
                    },
                    false,
                    move |index, cx| {
                        edit_and_notify(&mode_entity, cx, |draft| {
                            draft.view3d.sat_mode = if index == 0 {
                                SliceSatMode::AccurateHull
                            } else {
                                SliceSatMode::ExperimentalSlice
                            };
                        });
                    },
                ),
            )
            .mt(px(8.)),
        );
        if view.sat_mode == SliceSatMode::ExperimentalSlice {
            inner = inner.child(
                setting_row(
                    "Experimental slice display",
                    "Pick how the experimental slice-derived SAT is visualized.",
                    segmented(
                        "view3d-sat-render",
                        &["SHADED", "WIREFRAME"],
                        match view.sat_render {
                            SliceSatRender::Shaded => 0,
                            SliceSatRender::Wireframe => 1,
                        },
                        false,
                        move |index, cx| {
                            edit_and_notify(&render_entity, cx, |draft| {
                                draft.view3d.sat_render = if index == 0 {
                                    SliceSatRender::Shaded
                                } else {
                                    SliceSatRender::Wireframe
                                };
                            });
                        },
                    ),
                )
                .mt(px(8.)),
            );
        }
    }

    root = root.child(
        section(
            "layers",
            "3D View",
            "Build volume boundaries and display resolution hints used across the workspace.",
        )
        .child(inner),
    );
    root
}

// ── Multi-Plate (`MultiPlateSettingsTab.tsx`) ────────────────────────────

fn multi_plate_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let plate = &state.multi_plate;

    let follow_entity = entity.clone();
    let ordering_entity = entity.clone();
    let columns_entity = entity.clone();

    let mut body = div()
        .flex()
        .flex_col()
        .child(
            setting_row(
                "Follow the plate a model lands on",
                "A model dropped on another bed makes that bed the one you work on.",
                segmented(
                    "multi-plate-follow",
                    &["ON", "OFF"],
                    usize::from(!plate.follow_landed_plate),
                    false,
                    move |index, cx| {
                        edit_and_notify(&follow_entity, cx, |draft| {
                            draft.multi_plate.follow_landed_plate = index == 0;
                        });
                    },
                ),
            )
            .mt(px(12.)),
        )
        .child(
            setting_row(
                "Plate Ordering",
                "Growing grid, or a fixed row length.",
                segmented(
                    "multi-plate-ordering",
                    &["FIXED", "DYNAMIC"],
                    match plate.plate_ordering {
                        PlateOrdering::Fixed => 0,
                        PlateOrdering::Dynamic => 1,
                    },
                    false,
                    move |index, cx| {
                        edit_and_notify(&ordering_entity, cx, |draft| {
                            draft.multi_plate.plate_ordering = if index == 0 {
                                PlateOrdering::Fixed
                            } else {
                                PlateOrdering::Dynamic
                            };
                        });
                    },
                ),
            )
            .mt(px(12.)),
        );

    if plate.plate_ordering == PlateOrdering::Fixed {
        body = body.child(
            setting_row(
                "Beds per row",
                "1-2-3-4 across, then the next row. Adding a bed never moves the ones already placed.",
                number_input(
                    "multi-plate-columns",
                    plate.fixed_plate_columns,
                    1.0,
                    MAX_FIXED_PLATE_COLUMNS,
                    1.0,
                    Some(120.0),
                    34.0, // h-[34px]
                    14.0,
                    SURFACE_0,
                    true,
                    move |next, cx| {
                        if !next.is_finite() {
                            return;
                        }
                        edit_and_notify(&columns_entity, cx, |draft| {
                            draft.multi_plate.fixed_plate_columns = next
                                .clamp(1.0, MAX_FIXED_PLATE_COLUMNS)
                                .round();
                        });
                    },
                ),
            )
            .mt(px(12.)),
        );
    }

    div().flex().flex_col().child(
        div()
            .flex()
            .flex_col()
            .rounded(px(8.)) // rounded-lg
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1))
            .p(px(12.))
            .child(section_header(
                "square-stack",
                "Plates",
                "Behaviour across the beds of a scene.",
            ))
            .child(body),
    )
}

// ── 3D Mouse (`SpaceMouseSettingsTab.tsx`) ───────────────────────────────

fn spacemouse_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let mouse = &state.spacemouse;
    // The native navlib bridge is absent, so `navlibActive` is always false and
    // the tuning sections render.
    let mut root = div().flex().flex_col().gap(px(12.));

    let enable_entity = entity.clone();
    let pivot_entity = entity.clone();

    // Detected devices: no gamepad API in this slice, so `candidatePads` is
    // empty and the source's empty state renders.
    let devices = div()
        .mt(px(8.))
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .p(px(10.))
        .child(
            div()
                .mb(px(8.))
                .flex()
                .items_center()
                .gap(px(8.))
                .child(icon("gamepad-2", 16., TEXT_MUTED))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("Detected devices"),
                ),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child("No compatible input devices detected. Ensure your 3D mouse driver is installed."),
        );

    let pivot = div()
        .mt(px(8.))
        .rounded(px(6.))
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
                .child("Pivot mode"),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child("Auto picks selected/single/nearest model center. Camera Raycast picks the model center nearest your current view beam."),
        )
        .child(select_field(
            "spacemouse-pivot",
            &PIVOT_OPTIONS,
            match mouse.pivot_mode {
                PivotMode::Auto => "auto",
                PivotMode::CameraRay => "camera-ray",
            },
            None,
            None,
            true,
            pivot_entity.clone(),
            move |value, cx| {
                edit_and_notify(&pivot_entity, cx, |draft| {
                    draft.spacemouse.pivot_mode = if value == "camera-ray" {
                        PivotMode::CameraRay
                    } else {
                        PivotMode::Auto
                    };
                });
            },
        ));

    root = root.child(
        section(
            "mouse-pointer-2",
            "3D Mouse Input",
            "6-DoF navigation for pan, orbit, zoom, and roll using compatible 3D mouse devices.",
        )
        .child(
            setting_row(
                "Enable 3D Mouse",
                "Uses browser gamepad input from supported 3D mouse devices.",
                segmented(
                    "spacemouse-enabled",
                    &["OFF", "ON"],
                    usize::from(mouse.enabled),
                    false,
                    move |index, cx| {
                        edit_and_notify(&enable_entity, cx, |draft| {
                            draft.spacemouse.enabled = index == 1;
                        });
                    },
                ),
            )
            .mt(px(12.)),
        )
        .child(devices)
        .child(pivot),
    );

    // Motion Tuning
    let dominant_entity = entity.clone();
    root = root.child(
        section(
            "sliders-horizontal",
            "Motion Tuning",
            "Adjust sensitivity and movement behavior for translation, rotation, zoom, and deadzone.",
        )
        .child(
            div()
                .mt(px(8.))
                .grid()
                .grid_cols(2)
                .gap(px(8.))
                .child(slider_field(
                    "Translation",
                    format!("{:.2}", mouse.translation_sensitivity),
                    mouse.translation_sensitivity,
                    0.1,
                    4.0,
                ))
                .child(slider_field(
                    "Rotation",
                    format!("{:.2}", mouse.rotation_sensitivity),
                    mouse.rotation_sensitivity,
                    0.1,
                    4.0,
                ))
                .child(slider_field(
                    "Zoom",
                    format!("{:.2}", mouse.zoom_sensitivity),
                    mouse.zoom_sensitivity,
                    0.1,
                    4.0,
                ))
                .child(slider_field(
                    "Deadzone",
                    format!("{:.2}", mouse.deadzone),
                    mouse.deadzone,
                    0.0,
                    0.3,
                )),
        )
        .child(
            setting_row(
                "Dominant axis mode",
                "Prioritize strongest axis per translation/rotation group for steadier input.",
                segmented(
                    "spacemouse-dominant",
                    &["OFF", "ON"],
                    usize::from(mouse.dominant_axis),
                    false,
                    move |index, cx| {
                        edit_and_notify(&dominant_entity, cx, |draft| {
                            draft.spacemouse.dominant_axis = index == 1;
                        });
                    },
                ),
            )
            .mt(px(8.)),
        ),
    );

    // Axis Inversion
    let axes: [(&'static str, &'static str, bool); 6] = [
        ("invertTx", "Invert X pan", mouse.invert_tx),
        ("invertTy", "Invert Y pan", mouse.invert_ty),
        ("invertTz", "Invert zoom", mouse.invert_tz),
        ("invertRx", "Invert pitch", mouse.invert_rx),
        ("invertRy", "Invert yaw", mouse.invert_ry),
        ("invertRz", "Invert roll", mouse.invert_rz),
    ];
    let mut axis_grid = div().mt(px(8.)).grid().grid_cols(3).gap(px(8.));
    for (key, label, checked) in axes {
        let axis_entity = entity.clone();
        axis_grid = axis_grid.child(
            div()
                .id(format!("spacemouse-{key}"))
                .flex()
                .items_center()
                .gap(px(8.))
                .rounded(px(4.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_0))
                .px(px(8.))
                .py(px(6.))
                .text_size(px(12.))
                .cursor_pointer()
                .on_click(move |_, _, cx| {
                    edit_and_notify(&axis_entity, cx, |draft| match key {
                        "invertTx" => draft.spacemouse.invert_tx = !draft.spacemouse.invert_tx,
                        "invertTy" => draft.spacemouse.invert_ty = !draft.spacemouse.invert_ty,
                        "invertTz" => draft.spacemouse.invert_tz = !draft.spacemouse.invert_tz,
                        "invertRx" => draft.spacemouse.invert_rx = !draft.spacemouse.invert_rx,
                        "invertRy" => draft.spacemouse.invert_ry = !draft.spacemouse.invert_ry,
                        _ => draft.spacemouse.invert_rz = !draft.spacemouse.invert_rz,
                    });
                })
                .child(checkbox(checked, true))
                .child(div().text_color(rgb(TEXT_MUTED)).child(label)),
        );
    }
    root = root.child(
        section(
            "arrow-left-right",
            "Axis Inversion",
            "Flip the direction of individual pan, zoom, pitch, yaw, and roll axes.",
        )
        .child(axis_grid),
    );

    root.child(
        div()
            .rounded(px(6.))
            .border_1()
            .border_color(rgb(mix(BORDER_SUBTLE, ACCENT, 0.8)))
            .bg(rgb(mix(SURFACE_1, ACCENT, 0.96)))
            .px(px(12.))
            .py(px(8.))
            .child(
                div()
                    .text_size(px(11.))
                    .line_height(relative(1.2))
                    .text_color(rgb(TEXT_MUTED))
                    .child("Supports 3Dconnexion SpaceMouse devices. “SpaceMouse” is a trademark of 3Dconnexion."),
            ),
    )
}

// ── Hotkeys (`HotkeysSettingsTab.tsx`) ───────────────────────────────────

/// `toKeyLabel` — `' '` reads as Space, a single character is upper-cased.
fn key_label(key: &str) -> String {
    if key == " " {
        return "Space".to_owned();
    }
    if key.chars().count() == 1 {
        return key.to_uppercase();
    }
    key.to_owned()
}

/// `toModifierLabel` — `ctrl` is the platform's primary modifier.
fn modifier_label(modifier: &str, primary: &str) -> String {
    match modifier.trim().to_lowercase().as_str() {
        "ctrl" => primary.to_owned(),
        "shift" => "Shift".to_owned(),
        "alt" => "Alt".to_owned(),
        "meta" => {
            if primary == "Cmd" {
                "Cmd".to_owned()
            } else {
                "Meta".to_owned()
            }
        }
        _ => modifier.to_owned(),
    }
}

fn binding_tokens(binding: &HotkeyBinding, primary: &str) -> Vec<String> {
    let mut tokens: Vec<String> = binding
        .modifier
        .as_deref()
        .map(|modifier| {
            modifier
                .split('+')
                .map(|part| modifier_label(part, primary))
                .collect()
        })
        .unwrap_or_default();
    tokens.push(key_label(&binding.key));
    tokens
}

/// `KbdToken` — `rounded border px-1 py-1 font-mono text-[11px]`.
fn kbd_token(label: &str, muted: bool) -> Div {
    let (border, background, color) = if muted {
        (BORDER_SUBTLE, mix(SURFACE_2, SURFACE_1, 0.6), TEXT_MUTED)
    } else {
        (mix(ACCENT, BORDER_SUBTLE, 0.5), mix(SURFACE_2, SURFACE_1, 0.96), TEXT_STRONG)
    };
    div()
        .min_w(px(20.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .px(px(4.))
        .py(px(4.))
        .text_size(px(11.))
        .text_color(rgb(color))
        .child(label.to_owned())
}

/// One `HotkeyRow`.
fn hotkey_row(
    category: &'static str,
    action: &'static str,
    label: &'static str,
    binding: &HotkeyBinding,
    recording: bool,
    secondary: Option<&'static str>,
    entity: WeakEntity<DragonFruit>,
) -> Div {
    let primary = if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" };
    let row_entity = entity;

    let mut right = div()
        .id(format!("hotkey-{category}-{action}"))
        .min_w(px(92.))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(4.))
        .rounded(px(6.))
        .px(px(4.))
        .py(px(2.))
        .cursor_pointer()
        .on_click(move |_, _, cx| {
            edit_and_notify(&row_entity, cx, |draft| {
                draft.hotkeys.recording = if draft.hotkeys.recording == Some((category, action)) {
                    None
                } else {
                    Some((category, action))
                };
            });
        });

    if recording {
        right = right.child(
            div()
                .text_size(px(11.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(TEXT_MUTED))
                .child("Press keys…"),
        );
    } else {
        for token in binding_tokens(binding, primary) {
            right = right.child(kbd_token(&token, false));
        }
    }

    let mut trailing = div().flex().items_center().justify_end().gap(px(4.)).child(right);
    if !recording {
        if let Some(secondary) = secondary {
            trailing = trailing.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(TEXT_MUTED))
                            .child("/"),
                    )
                    .child(kbd_token(secondary, true)),
            );
        }
    }

    let (border, background) = if recording {
        (mix(ACCENT, BORDER_SUBTLE, 0.65), mix(ACCENT, SURFACE_1, 0.16))
    } else {
        (BORDER_SUBTLE, mix(SURFACE_2, SURFACE_1, 0.9))
    };

    div()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.))
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .px(px(8.))
        .py(px(6.))
        .text_color(rgb(TEXT_STRONG))
        .child(
            div()
                .min_w(px(0.))
                .truncate()
                .text_size(px(11.))
                .text_color(rgb(TEXT_STRONG))
                .child(label),
        )
        .child(trailing)
}

fn hotkey_section(
    title: &'static str,
    description: &'static str,
    categories: &[&'static str],
    state: &SettingsState,
    entity: WeakEntity<DragonFruit>,
) -> Div {
    let reset_entity = entity.clone();
    let reset_categories: Vec<&'static str> = categories.to_vec();

    let mut entries = div().mt(px(8.)).flex().flex_col().gap(px(8.));
    for category in categories {
        let mut group = div().flex().flex_col().gap(px(4.));
        if categories.len() > 1 {
            group = group.child(
                div()
                    .text_size(px(11.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(TEXT_MUTED))
                    .child(category_label(category)),
            );
        }
        for &(entry_category, action, key, modifier, description) in HOTKEY_DEFAULTS.iter() {
            if entry_category != *category {
                continue;
            }
            let _ = (key, modifier);
            let binding = state.hotkeys.binding(entry_category, action);
            let label = if *category == "PRESETS" {
                pinned_slot_label(action)
            } else {
                description
            };
            let recording = state.hotkeys.recording == Some((entry_category, action));
            let secondary = if *category == "GLOBAL" && action == "DELETE" {
                Some("Delete")
            } else {
                None
            };
            group = group.child(hotkey_row(
                entry_category,
                action,
                label,
                &binding,
                recording,
                secondary,
                entity.clone(),
            ));
        }
        entries = entries.child(group);
    }

    div()
        .flex()
        .flex_col()
        .h_full()
        .rounded(px(8.)) // rounded-lg
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(10.))
        .child(
            div()
                .flex()
                .items_start()
                .gap(px(8.))
                .child(icon_chip("keyboard"))
                .child(
                    div()
                        .min_w(px(0.))
                        .flex_1()
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child(title),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(TEXT_MUTED))
                                .child(description),
                        ),
                )
                .child(
                    div()
                        .id(format!("hotkeys-reset-{title}"))
                        .w(px(28.))
                        .h(px(28.))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(6.))
                        .cursor_pointer()
                        .hover(|style| style.bg(rgb(SURFACE_2)))
                        .on_click(move |_, _, cx| {
                            edit_and_notify(&reset_entity, cx, |draft| {
                                for category in reset_categories.iter() {
                                    draft.hotkeys.reset_category(category);
                                }
                            });
                        })
                        .child(icon("rotate-ccw", 14., TEXT_MUTED)),
                ),
        )
        .child(entries)
}

fn category_label(category: &str) -> &'static str {
    match category {
        "GLOBAL" => "General",
        "CAMERA" => "Camera",
        "CANVAS" => "Canvas Tools",
        "SUPPORTS" => "Supports",
        "PRESETS" => "Presets",
        _ => "General",
    }
}

fn pinned_slot_label(action: &str) -> &'static str {
    match action {
        "SLOT_1" => "Slot 1",
        "SLOT_2" => "Slot 2",
        "SLOT_3" => "Slot 3",
        "SLOT_4" => "Slot 4",
        "SLOT_5" => "Slot 5",
        "SLOT_6" => "Slot 6",
        _ => "Slot",
    }
}

fn hotkeys_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    const SECTIONS: [(&str, &str, [&str; 2]); 4] = [
        (
            "Global",
            "General and camera shortcuts available across all workspaces.",
            ["GLOBAL", "CAMERA"],
        ),
        (
            "Prepare",
            "Canvas tool switching and scene arrangement shortcuts.",
            ["CANVAS", ""],
        ),
        (
            "Supports",
            "Support authoring workflow shortcuts.",
            ["SUPPORTS", ""],
        ),
        (
            "Support Presets",
            "Quick-apply support preset shortcuts.",
            ["PRESETS", ""],
        ),
    ];

    let mut rows = div().flex().flex_col().gap(px(10.));
    for pair in SECTIONS.chunks(2) {
        let mut row = div().grid().grid_cols(2).gap(px(10.));
        for (title, description, categories) in pair {
            let categories: Vec<&'static str> = categories
                .iter()
                .copied()
                .filter(|category| !category.is_empty())
                .collect();
            row = row.child(hotkey_section(
                title,
                description,
                &categories,
                state,
                entity.clone(),
            ));
        }
        rows = rows.child(row);
    }

    div()
        .flex_1()
        .min_h(px(0.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            div()
                .id("hotkeys-scroll")
                .flex_1()
                .min_h(px(0.))
                .overflow_y_scroll()
                .pr(px(4.))
                .child(rows),
        )
}

/// The `Recording Shortcut` overlay.
fn hotkeys_overlay(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Option<Div> {
    state.hotkeys.recording?;
    let cancel_entity = entity;
    Some(
        div()
            .absolute()
            .top(px(0.))
            .left(px(0.))
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .px(px(16.))
            .bg(rgba(0x0000008c)) // bg-black/55
            .child(
                div()
                    .w(px(448.)) // max-w-md
                    .rounded(px(12.))
                    .border_1()
                    .border_color(rgb(BORDER_STRONG))
                    .bg(rgb(SURFACE_0))
                    .p(px(20.))
                    .child(
                        div()
                            .text_size(px(16.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(TEXT_STRONG))
                            .child("Recording Shortcut"),
                    )
                    .child(
                        div()
                            .mt(px(8.))
                            .text_size(px(14.))
                            .text_color(rgb(TEXT_MUTED))
                            .child("Press the new key combination now."),
                    )
                    .child(
                        div()
                            .mt(px(4.))
                            .flex()
                            .flex_wrap()
                            .text_size(px(12.))
                            .text_color(rgb(TEXT_MUTED))
                            .child("Press ")
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(TEXT_STRONG))
                                    .child("Esc"),
                            )
                            .child(" or click outside to cancel."),
                    )
                    .child(
                        div().mt(px(16.)).flex().justify_end().child(
                            div()
                                .id("hotkeys-cancel-recording")
                                .h(px(32.))
                                .px(px(12.))
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
                                .cursor_pointer()
                                .hover(|style| style.bg(rgb(SURFACE_2)))
                                .on_click(move |_, _, cx| {
                                    edit_and_notify(&cancel_entity, cx, |draft| {
                                        draft.hotkeys.recording = None;
                                    });
                                })
                                .child("Cancel"),
                        ),
                    ),
            ),
    )
}

// ── Plugins (`PluginsSettingsTab.tsx`) ───────────────────────────────────

fn plugins_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let plugins = &state.plugins;
    let allowance_entity = entity.clone();
    let install_entity = entity.clone();

    // `installedRows` is empty: the profile store and the generated built-in
    // plugin manifests are not part of this slice.
    let installed_count = 0usize;

    let mut root = div().flex().flex_col().gap(px(12.));

    let mut header = section(
        "plug",
        "Plugins",
        "Profile packs add presets and material profiles. A sideloaded package adds code and needs a publisher you trust.",
    )
    .relative();
    // `IconButton` absolute right-3 top-3, `PackageOpen`, warning tone.
    header = header.child(
        div()
            .absolute()
            .top(px(12.))
            .right(px(12.))
            .child(
                div()
                    .id("plugins-allowance")
                    .w(px(28.))
                    .h(px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(6.))
                    .border_1()
                    .border_color(rgb(mix(0xf59e0b, BORDER_SUBTLE, 0.45)))
                    .bg(rgb(mix(0xf59e0b, SURFACE_1, 0.9)))
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(mix(0xf59e0b, SURFACE_1, 0.82))))
                    .on_click(move |_, _, cx| {
                        edit_and_notify(&allowance_entity, cx, |draft| {
                            draft.plugins.allowance_dialog_open = true;
                        });
                    })
                    .child(icon("package-open", 16., 0xf59e0b)),
            ),
    );

    let install_row = setting_row_surface(
        "Install profile pack",
        "Adds presets and materials from a repository. Contains no code.",
        false,
        false,
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .w(px(420.))
            .child(
                div().flex_1().min_w(px(0.)).child(text_field(
                    &plugins.repo_url,
                    "https://github.com/owner/repo",
                    false,
                    34.0,
                    14.0,
                )),
            )
            .child(
                div()
                    .id("plugins-install")
                    .h(px(36.))
                    .px(px(12.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(rgb(mix(ACCENT, BORDER_SUBTLE, 0.45)))
                    .bg(rgb(mix(ACCENT, SURFACE_1, 0.86)))
                    .text_size(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(ACCENT))
                    .child(icon("download", 14., ACCENT))
                    .child("Install"),
            ),
    );

    let mut rows = div().mt(px(8.)).flex().flex_col().gap(px(8.)).child(install_row);
    if plugins.allow_sideload {
        rows = rows.child(setting_row_surface(
            "Import package",
            "Checks the publisher signature, then unseals and installs the .dfplugin file.",
            false,
            false,
            div()
                .h(px(36.))
                .px(px(12.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(6.))
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_1))
                .text_size(px(14.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child(icon("package-plus", 14., TEXT_STRONG))
                .child("Choose file…"),
        ));
    }
    header = header.child(rows);
    if let Some((error, message)) = &plugins.status {
        header = header.child(
            div()
                .mt(px(8.))
                .text_size(px(12.))
                .text_color(rgb(if *error { 0xfca5a5 } else { 0x86efac }))
                .child(message.clone()),
        );
    }
    root = root.child(header);

    // Installed plugins
    let mut installed = section(
        "package-check",
        "Installed plugins",
        if installed_count == 1 {
            "1 plugin in this profile."
        } else {
            "0 plugins in this profile."
        },
    );
    installed = installed.child(
        div()
            .mt(px(8.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .px(px(12.))
                    .py(px(8.))
                    .text_size(px(12.))
                    .text_color(rgb(TEXT_MUTED))
                    .child("No plugins installed in this profile."),
            ),
    );
    root = root.child(installed);

    // `isDevRuntime` is false for a release build, so the Plugin Creation Studio
    // link is absent, exactly as at the source.
    let _ = install_entity;
    root
}

/// The `Sideloaded packages` allowance dialog.
fn plugins_overlay(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Option<Div> {
    if !state.plugins.allowance_dialog_open {
        return None;
    }
    let plugins = &state.plugins;
    let cancel_entity = entity.clone();
    let toggle_entity = entity;

    let body = if plugins.allow_sideload {
        div().child(
            div()
                .text_size(px(14.))
                .line_height(relative(1.45))
                .text_color(rgb(TEXT_MUTED))
                .child("Packages you import run with your permissions and can read any file you can read, open network connections, and crash the app if they misbehave. Disabling this stops new imports; packages already installed stay on disk."),
        )
    } else {
        div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .text_size(px(14.))
                    .line_height(relative(1.45))
                    .text_color(rgb(TEXT_STRONG))
                    .child("A sideloaded package is a native program, not a profile pack."),
            )
            .child(
                div()
                    .text_size(px(14.))
                    .line_height(relative(1.45))
                    .text_color(rgb(TEXT_MUTED))
                    .child("Once imported it runs with your permissions: it can read any file you can read, open network connections, and crash the app if it misbehaves."),
            )
    };

    Some(dialog_shell(
        512.0, // max-w-lg
        "shield-alert",
        (0xf59e0b, mix(0xf59e0b, BORDER_SUBTLE, 0.45), mix(0xf59e0b, SURFACE_1, 0.9)),
        if plugins.allow_sideload {
            "Sideloading is on"
        } else {
            "Allow untrusted plugins?"
        },
        Some("Sideloaded packages".into()),
        body.child(
            div()
                .mt(px(12.))
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(mix(0xd97706, BORDER_SUBTLE, 0.4)))
                .bg(rgb(mix(0xd97706, SURFACE_1, 0.92)))
                .px(px(12.))
                .py(px(8.))
                .flex()
                .items_start()
                .gap(px(8.))
                .child(icon("triangle-alert", 14., 0xd97706))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(12.))
                        .line_height(relative(1.45))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Signatures prove who built the package, not that it is safe. Only enable this for a publisher you trust."),
                ),
        ),
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(action_button(
                "plugins-allowance-cancel",
                "Cancel",
                None,
                16.0,
                SURFACE_1,
                BORDER_SUBTLE,
                TEXT_STRONG,
                SURFACE_2,
                move |cx| {
                    edit_and_notify(&cancel_entity, cx, |draft| {
                        draft.plugins.allowance_dialog_open = false;
                    });
                },
            ))
            .child(action_button(
                "plugins-allowance-toggle",
                if plugins.allow_sideload {
                    "Disable Sideloading"
                } else {
                    "Enable Sideloading"
                },
                Some("shield-alert"),
                16.0,
                if plugins.allow_sideload {
                    mix(0xef4444, SURFACE_1, 0.86)
                } else {
                    mix(0xf59e0b, SURFACE_1, 0.86)
                },
                mix(0xf59e0b, BORDER_SUBTLE, 0.45),
                if plugins.allow_sideload { 0xef4444 } else { 0xf59e0b },
                mix(0xf59e0b, SURFACE_1, 0.76),
                move |cx| {
                    edit_and_notify(&toggle_entity, cx, |draft| {
                        draft.plugins.allow_sideload = !draft.plugins.allow_sideload;
                        draft.plugins.allowance_dialog_open = false;
                    });
                },
            )),
    ))
}

// ── Experiments (`ExperimentsSettingsTab.tsx`) ───────────────────────────

fn experiments_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let mut root = div().flex().flex_col().gap(px(12.));
    root = root.child(
        div()
            .rounded(px(6.))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(mix(SURFACE_1, SURFACE_0, 0.94)))
            .p(px(12.))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(32.))
                            .h(px(32.))
                            .flex_shrink_0()
                            .rounded(px(6.))
                            .border_1()
                            .border_color(rgb(BORDER_SUBTLE))
                            .bg(rgb(mix(SURFACE_2, SURFACE_1, 0.92)))
                            .child(icon("flask-conical", 16., ACCENT_SECONDARY)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(TEXT_STRONG))
                                    .child("Experiments"),
                            )
                            .child(
                                div()
                                    .mt(px(2.))
                                    .text_size(px(12.))
                                    .line_height(relative(1.2))
                                    .text_color(rgb(TEXT_MUTED))
                                    .child("Early-access features that are still in testing. They may change or disappear without notice."),
                            ),
                    ),
            ),
    );

    for (id, name, description, default_enabled) in EXPERIMENTS.iter() {
        let toggle_entity = entity.clone();
        let enabled = state
            .experiments
            .enabled
            .get(id)
            .copied()
            .unwrap_or(*default_enabled);
        root = root.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.))
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_1))
                .px(px(10.))
                .py(px(8.))
                .child(
                    div()
                        .min_w(px(0.))
                        .flex_1()
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child(*name),
                        )
                        .child(
                            div()
                                .mt(px(2.))
                                .text_size(px(12.))
                                .line_height(relative(1.2))
                                .text_color(rgb(TEXT_MUTED))
                                .child(*description),
                        ),
                )
                .child(segmented(
                    "experiment",
                    &["OFF", "ON"],
                    usize::from(enabled),
                    false,
                    move |index, cx| {
                        edit_and_notify(&toggle_entity, cx, |draft| {
                            draft.experiments.enabled.insert(id, index == 1);
                        });
                    },
                )),
        );
    }

    root.child(
        div()
            .rounded(px(6.))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1))
            .px(px(12.))
            .py(px(8.))
            .text_size(px(12.))
            .text_color(rgb(TEXT_MUTED))
            .child("Experiment changes apply after you restart DragonFruit or reload the window."),
    )
}

/// The ORA disclaimer gate, shown before the tab content.
fn experiments_overlay(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Option<Div> {
    if !state.experiments.show_disclaimer {
        return None;
    }
    let exit_entity = entity.clone();
    let ack_entity = entity;

    Some(
        div()
            .absolute()
            .top(px(0.))
            .left(px(0.))
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .px(px(12.))
            .bg(rgba(0x0000008c))
            .child(
                div()
                    .w(px(512.)) // max-w-lg
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .rounded(px(12.))
                    .border_1()
                    .border_color(rgb(BORDER_SUBTLE))
                    .bg(rgb(SURFACE_0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(16.))
                            .border_b_1()
                            .border_color(rgb(BORDER_SUBTLE))
                            .px(px(20.))
                            .py(px(16.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .w(px(40.))
                                    .h(px(40.))
                                    .flex_shrink_0()
                                    .rounded(px(10.))
                                    .border_1()
                                    .border_color(rgb(mix(0xf59e0b, BORDER_SUBTLE, 0.45)))
                                    .bg(rgb(mix(0xf59e0b, SURFACE_1, 0.9)))
                                    .child(icon("triangle-alert", 18., 0xf59e0b)),
                            )
                            .child(
                                div()
                                    .min_w(px(0.))
                                    .child(
                                        div()
                                            .text_size(px(16.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .line_height(relative(1.15))
                                            .text_color(rgb(TEXT_STRONG))
                                            .child("Experimental Features"),
                                    )
                                    .child(
                                        div()
                                            .mt(px(2.))
                                            .text_size(px(11.))
                                            .line_height(relative(1.2))
                                            .text_color(rgb(TEXT_MUTED))
                                            .child("Use at your own risk."),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(16.))
                            .p(px(20.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(relative(1.6))
                                    .text_color(rgb(TEXT_MUTED))
                                    .child("Experiments are early-access features still under active development. They are provided \"as is\" without any warranty, and without guarantee of correctness, reliability, or compatibility."),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(relative(1.6))
                                    .text_color(rgb(TEXT_MUTED))
                                    .child("The Open Resin Alliance and its contributors accept no responsibility or liability for any loss, damage, or data loss arising from the use of experimental features."),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(relative(1.6))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0xfca5a5))
                                    .child("Enable and use them at your own risk."),
                            )
                            .child(
                                div()
                                    .grid()
                                    .grid_cols(2)
                                    .gap(px(8.))
                                    .pt(px(4.))
                                    .child(action_button(
                                        "experiments-back",
                                        "Take me back!",
                                        None,
                                        16.0,
                                        SURFACE_1,
                                        BORDER_SUBTLE,
                                        TEXT_STRONG,
                                        SURFACE_2,
                                        move |cx| {
                                            // "Take me back" keeps the gate for the
                                            // next entry, so it can reappear.
                                            edit_and_notify(&exit_entity, cx, |draft| {
                                                draft.experiments.show_disclaimer = false;
                                            });
                                        },
                                    ))
                                    .child(action_button(
                                        "experiments-ack",
                                        "I understand",
                                        None,
                                        16.0,
                                        mix(0xf59e0b, SURFACE_1, 0.86),
                                        mix(0xf59e0b, BORDER_SUBTLE, 0.45),
                                        0xf59e0b,
                                        mix(0xf59e0b, SURFACE_1, 0.76),
                                        move |cx| {
                                            edit_and_notify(&ack_entity, cx, |draft| {
                                                draft.experiments.disclaimer_seen = true;
                                                draft.experiments.show_disclaimer = false;
                                            });
                                        },
                                    )),
                            ),
                    ),
            ),
    )
}

// ── Shared row shapes for the bridge-backed tabs ─────────────────────────

/// `SettingRow` with a `label` only — the source's rows that pass no
/// `description`.
fn setting_row_label_only(label: impl Into<SharedString>, control: impl IntoElement) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.))
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .p(px(6.))
        .child(
            div()
                .min_w(px(0.))
                .pl(px(4.))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(relative(1.2))
                        .text_color(rgb(TEXT_STRONG))
                        .child(label.into()),
                ),
        )
        .child(div().flex().items_center().gap(px(8.)).child(control))
}

/// A `NumberInput` with the source's unit suffix (`sec`, `min`).
fn number_with_unit(input: Div, unit: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .child(input)
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child(unit),
        )
}

/// `!h-8 !px-2.5 text-xs` secondary button with a leading glyph in the
/// source's disabled state (the bridge that would run it is absent here).
fn inert_small_button(id: &'static str, label: &'static str, slug: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .h(px(32.))
        .px(px(10.))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.))
        .flex_shrink_0()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT_MUTED))
        .opacity(0.6)
        .child(icon(slug, 14., TEXT_MUTED))
        .child(label)
}

// ── Scene Autosave (`SceneAutosaveSettingsTab.tsx`) ──────────────────────

fn autosave_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let autosave = &state.autosave;
    let enabled_entity = entity.clone();
    let debounce_entity = entity.clone();
    let cooldown_entity = entity.clone();
    let cap_entity = entity.clone();
    let prompt_entity = entity;

    let debounce_seconds = (autosave.debounce_ms / 1000.0).round();
    let cooldown_seconds = (autosave.cooldown_ms / 1000.0).round();
    let cap_minutes = (autosave.cap_ms / 60_000.0).round();

    let mut rows = div().mt(px(8.)).flex().flex_col().gap(px(8.));
    rows = rows.child(setting_row_surface(
        "Enable scene autosave",
        "Creates recovery snapshots while editing.",
        false,
        false,
        segmented(
            "autosave-enabled",
            &["OFF", "ON"],
            usize::from(autosave.enabled),
            false,
            move |index, cx| {
                edit_and_notify(&enabled_entity, cx, |draft| {
                    draft.autosave.enabled = index == 1;
                });
            },
        ),
    ));
    rows = rows.child(setting_row_surface(
        "Autosave idle delay",
        "Wait this long after edits before autosaving.",
        false,
        !autosave.enabled,
        number_with_unit(
            number_input(
                "autosave-debounce",
                debounce_seconds,
                15.0,
                900.0,
                5.0,
                Some(120.0),
                34.0, // h-[34px]
                14.0, // text-sm
                SURFACE_0, // .ui-input
                autosave.enabled,
                move |next, cx| {
                    if !next.is_finite() {
                        return;
                    }
                    let next_seconds = next.round().clamp(15.0, 900.0);
                    edit_and_notify(&debounce_entity, cx, |draft| {
                        draft.autosave.debounce_ms = next_seconds * 1000.0;
                        draft.autosave.cap_ms = draft.autosave.cap_ms.max(next_seconds * 1000.0);
                    });
                },
            ),
            "sec",
        ),
    ));
    rows = rows.child(setting_row_surface(
        "Autosave cooldown",
        "After an autosave, defer further automatic saves until this interval ends.",
        false,
        !autosave.enabled,
        number_with_unit(
            number_input(
                "autosave-cooldown",
                cooldown_seconds,
                15.0,
                900.0,
                5.0,
                Some(120.0),
                34.0,
                14.0,
                SURFACE_0,
                autosave.enabled,
                move |next, cx| {
                    if !next.is_finite() {
                        return;
                    }
                    let next_seconds = next.round().clamp(15.0, 900.0);
                    edit_and_notify(&cooldown_entity, cx, |draft| {
                        draft.autosave.cooldown_ms = next_seconds * 1000.0;
                        draft.autosave.cap_ms = draft.autosave.cap_ms.max(next_seconds * 1000.0);
                    });
                },
            ),
            "sec",
        ),
    ));
    rows = rows.child(setting_row_surface(
        "Maximum autosave interval",
        "For continuous edits, save at least this often.",
        false,
        !autosave.enabled,
        number_with_unit(
            number_input(
                "autosave-cap",
                cap_minutes,
                1.0,
                60.0,
                1.0,
                Some(120.0),
                34.0,
                14.0,
                SURFACE_0,
                autosave.enabled,
                move |next, cx| {
                    if !next.is_finite() {
                        return;
                    }
                    let next_minutes = next.round().clamp(1.0, 60.0);
                    edit_and_notify(&cap_entity, cx, |draft| {
                        draft.autosave.cap_ms = (next_minutes * 60_000.0)
                            .max(draft.autosave.debounce_ms)
                            .max(draft.autosave.cooldown_ms);
                    });
                },
            ),
            "min",
        ),
    ));

    let mut root = div().flex().flex_col().gap(px(12.));
    root = root.child(
        section(
            "hard-drive",
            "Scene Autosave",
            "Automatically saves a crash-recovery .voxl scene in the background and offers restore on next launch.",
        )
        .child(rows),
    );

    // Crash Recovery — the source's `IconChip tone="warning"` header.
    root = root.child(
        div()
            .rounded(px(6.))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1))
            .p(px(12.))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(32.))
                            .h(px(32.))
                            .flex_shrink_0()
                            .rounded(px(6.))
                            .border_1()
                            .border_color(rgb(mix(0xd97706, BORDER_SUBTLE, 0.5)))
                            .bg(rgb(mix(0xd97706, SURFACE_1, 0.15)))
                            .child(icon("triangle-alert", 16., 0xd97706)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(TEXT_STRONG))
                                    .child("Crash Recovery"),
                            )
                            .child(
                                div()
                                    .mt(px(2.))
                                    .text_size(px(12.))
                                    .text_color(rgb(TEXT_MUTED))
                                    .child("Show restore/discard prompt if a dirty autosave is detected at startup."),
                            ),
                    ),
            )
            .child(
                setting_row_label_only(
                    "Recovery prompt on startup",
                    segmented(
                        "autosave-recovery-prompt",
                        &["OFF", "ON"],
                        usize::from(autosave.recovery_prompt_enabled),
                        false,
                        move |index, cx| {
                            edit_and_notify(&prompt_entity, cx, |draft| {
                                draft.autosave.recovery_prompt_enabled = index == 1;
                            });
                        },
                    ),
                )
                .mt(px(8.)),
            ),
    );

    // Autosave Status — the desktop bridge is absent, so the source's own
    // "desktop build" state renders and the actions stay disabled.
    root = root.child(
        div()
            .rounded(px(6.))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1))
            .p(px(12.))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(8.))
                    .child(icon_chip("hard-drive"))
                    .child(
                        div()
                            .min_w(px(0.))
                            .flex_1()
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(TEXT_STRONG))
                                    .child("Autosave Status"),
                            )
                            .child(
                                div()
                                    .mt(px(2.))
                                    .text_size(px(12.))
                                    .text_color(rgb(TEXT_MUTED))
                                    .child("Desktop-only diagnostics for autosave files and recovery state."),
                            ),
                    )
                    .child(inert_small_button(
                        "autosave-refresh",
                        "Refresh",
                        "refresh-cw",
                    )),
            )
            .child(
                div()
                    .mt(px(8.))
                    .text_size(px(12.))
                    .text_color(rgb(TEXT_MUTED))
                    .child("Autosave files are available in the DragonFruit desktop build."),
            )
            .child(
                div()
                    .mt(px(8.))
                    .grid()
                    .grid_cols(2)
                    .gap(px(8.))
                    .child(
                        action_button(
                            "autosave-mark-clean",
                            "Mark Recovery Clean",
                            Some("triangle-alert"),
                            12.0,
                            mix(0xef4444, SURFACE_1, 0.86),
                            mix(0xef4444, BORDER_SUBTLE, 0.45),
                            0xef4444,
                            mix(0xef4444, SURFACE_1, 0.76),
                            |_| {},
                        )
                        .opacity(0.6),
                    )
                    .child(
                        action_button(
                            "autosave-reveal",
                            "Open Autosave Location",
                            Some("hard-drive"),
                            12.0,
                            mix(ACCENT, SURFACE_1, 0.14),
                            mix(ACCENT, BORDER_SUBTLE, 0.55),
                            ACCENT,
                            mix(ACCENT, SURFACE_1, 0.24),
                            |_| {},
                        )
                        .opacity(0.6),
                    ),
            ),
    );

    root
}

// ── Backups (`LocalBackupsSettingsTab.tsx`) ──────────────────────────────

fn backups_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let backups = &state.backups;
    let auto_entity = entity.clone();
    let interval_entity = entity;

    let selected = backups.selected_directory.trim();
    let has_selected = !selected.is_empty();
    // `defaultDirectory` comes from the native backup bridge, which this slice
    // does not have, so the probe never answers: `isUsingDefault` is true and
    // the "Use Default Path" action is absent, exactly as at the source.
    let has_any_sync = false;

    let mut root = div().flex().flex_col().gap(px(12.));

    // Local On-Disk Backups
    root = root.child(
        div()
            .relative()
            .rounded(px(6.))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1))
            .p(px(12.))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(10.))
                    .child(icon_chip("hard-drive"))
                    .child(
                        div()
                            .min_w(px(0.))
                            .flex_1()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(TEXT_STRONG))
                                    .child("Local On-Disk Backups"),
                            )
                            .child(
                                div()
                                    .mt(px(2.))
                                    .text_size(px(12.))
                                    .line_height(relative(1.2))
                                    .text_color(rgb(TEXT_MUTED))
                                    .child("Automatically saves snapshots of your settings and profiles to a folder on your computer."),
                            ),
                    ),
            )
            .child(
                div()
                    .mt(px(8.))
                    .grid()
                    .grid_cols(2)
                    .gap(px(8.))
                    .child(stat_card("DEFAULT DIRECTORY", "Resolving..."))
                    .child(stat_card(
                        "SELECTED DIRECTORY",
                        if has_selected {
                            selected.to_owned()
                        } else {
                            "Not selected".to_owned()
                        },
                    ))
                    .child(stat_card("LAST BACKUP ON DISK", "Never"))
                    .child(stat_card("LAST LOCAL SYNC", "Never")),
            )
            .child(
                div()
                    .mt(px(8.))
                    .grid()
                    .grid_cols(2)
                    .gap(px(8.))
                    .child(action_button(
                        "backups-choose-directory",
                        "Choose Folder",
                        Some("folder-open"),
                        12.0,
                        mix(ACCENT, SURFACE_1, 0.14),
                        mix(ACCENT, BORDER_SUBTLE, 0.55),
                        ACCENT,
                        mix(ACCENT, SURFACE_1, 0.24),
                        |_| {},
                    ))
                    .child(
                        action_button(
                            "backups-open-directory",
                            "Open Folder",
                            Some("folder-open"),
                            12.0,
                            mix(ACCENT_SECONDARY, SURFACE_1, 0.86),
                            mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.45),
                            ACCENT_SECONDARY,
                            mix(ACCENT_SECONDARY, SURFACE_1, 0.76),
                            |_| {},
                        )
                        .when(!has_selected, |this| this.opacity(0.6)),
                    ),
            ),
    );

    // Backup Management
    let mut management = div()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(12.))
        .child(
            div()
                .flex()
                .items_start()
                .gap(px(8.))
                .child(icon_chip("refresh-ccw"))
                .child(
                    div()
                        .flex_1()
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child("Backup Management"),
                        )
                        .child(
                            div()
                                .mt(px(2.))
                                .text_size(px(12.))
                                .text_color(rgb(TEXT_MUTED))
                                .child("Save snapshots to your local backup folder, then review and restore any point-in-time snapshot in-app."),
                        ),
                ),
        );

    // Quick actions
    let mut quick = inner_card()
        .child(stat_label("QUICK ACTIONS"))
        .child(
            div()
                .mt(px(2.))
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child("Create a snapshot now and refresh history."),
        );
    quick = quick.child(
        div()
            .mt(px(8.))
            .grid()
            .grid_cols(2)
            .gap(px(8.))
            .child(
                action_button(
                    "backups-now",
                    "Backup Now",
                    Some("cloud-upload"),
                    12.0,
                    mix(ACCENT, SURFACE_1, 0.14),
                    mix(ACCENT, BORDER_SUBTLE, 0.55),
                    ACCENT,
                    mix(ACCENT, SURFACE_1, 0.24),
                    |_| {},
                )
                .when(!has_selected, |this| this.opacity(0.6)),
            )
            .child(
                action_button(
                    "backups-refresh-history",
                    "Refresh History",
                    Some("refresh-ccw"),
                    12.0,
                    SURFACE_1,
                    BORDER_SUBTLE,
                    TEXT_STRONG,
                    SURFACE_2,
                    |_| {},
                )
                .when(!has_selected, |this| this.opacity(0.6)),
            ),
    );
    management = management.child(quick.mt(px(8.)));

    // Automation
    let mut automation = inner_card().child(stat_label("AUTOMATION"));
    automation = automation.child(
        div()
            .mt(px(8.))
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(setting_row_surface(
                "Enable automatic backups",
                "Automatically write snapshots to local disk on an interval.",
                true,
                false,
                segmented(
                    "backups-auto",
                    &["OFF", "ON"],
                    usize::from(backups.auto_sync_enabled),
                    false,
                    move |index, cx| {
                        edit_and_notify(&auto_entity, cx, |draft| {
                            draft.backups.auto_sync_enabled = index == 1;
                        });
                    },
                ),
            ))
            .child(setting_row_surface(
                "Sync interval",
                "Minutes between automatic local backups.",
                true,
                !backups.auto_sync_enabled,
                number_with_unit(
                    number_input(
                        "backups-interval",
                        backups.auto_sync_minutes,
                        1.0,
                        240.0,
                        1.0,
                        Some(120.0),
                        34.0,
                        14.0,
                        SURFACE_0,
                        backups.auto_sync_enabled,
                        move |next, cx| {
                            if !next.is_finite() {
                                return;
                            }
                            edit_and_notify(&interval_entity, cx, |draft| {
                                draft.backups.auto_sync_minutes =
                                    next.round().clamp(1.0, 240.0);
                            });
                        },
                    ),
                    "min",
                ),
            )),
    );
    automation = automation.child(
        div()
            .mt(px(8.))
            .text_size(px(12.))
            .text_color(rgb(TEXT_MUTED))
            .child("Last local sync: never"),
    );
    management = management.child(automation.mt(px(8.)));

    // Manage Backups
    let mut manage = inner_card().child(
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(
                div()
                    .child(stat_label("MANAGE BACKUPS"))
                    .child(
                        div()
                            .mt(px(2.))
                            .text_size(px(12.))
                            .text_color(rgb(TEXT_MUTED))
                            .child("View, restore, or delete older snapshots from disk."),
                    ),
            )
            .child(inert_small_button(
                "backups-manage-refresh",
                "Refresh",
                "refresh-ccw",
            )),
    );
    manage = manage.child(
        div().mt(px(8.)).max_h(px(256.)).overflow_hidden().child(
            div()
                .px(px(12.))
                .py(px(12.))
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child(if has_any_sync {
                    "No history snapshots found in this folder."
                } else {
                    "No snapshots yet. Run \"Backup Now\" to create your first local snapshot."
                }),
        ),
    );
    management = management.child(manage.mt(px(8.)));
    root = root.child(management);

    if let Some((error, text)) = &backups.message {
        root = root.child(message_strip(Some(*error), text));
    }

    root
}

/// The delete/restore confirmations. Both ids are set from a history row, and
/// the slice lists no rows, so these never open; the dialogs still carry the
/// source's copy.
fn backups_overlay(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Option<Div> {
    let backups = &state.backups;

    if let Some(id) = backups.confirm_delete_id.clone() {
        let cancel_entity = entity.clone();
        let confirm_entity = entity;
        return Some(dialog_shell(
            448.0, // max-w-md
            "trash-2",
            (
                0xef4444,
                mix(0xef4444, BORDER_SUBTLE, 0.45),
                mix(0xef4444, SURFACE_1, 0.12),
            ),
            "Delete Backup Snapshot",
            Some("This action cannot be undone.".into()),
            div().child(
                div()
                    .text_size(px(12.))
                    .line_height(relative(1.35))
                    .text_color(rgb(TEXT_MUTED))
                    .child(format!(
                        "Backup snapshot from {id} will be permanently removed from disk."
                    )),
            ),
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(action_button(
                    "backups-delete-cancel",
                    "Cancel",
                    None,
                    16.0,
                    SURFACE_1,
                    BORDER_SUBTLE,
                    TEXT_STRONG,
                    SURFACE_2,
                    move |cx| {
                        edit_and_notify(&cancel_entity, cx, |draft| {
                            draft.backups.confirm_delete_id = None;
                        });
                    },
                ))
                .child(action_button(
                    "backups-delete-confirm",
                    "Delete",
                    Some("trash-2"),
                    16.0,
                    mix(0xef4444, SURFACE_1, 0.86),
                    mix(0xef4444, BORDER_SUBTLE, 0.45),
                    0xef4444,
                    mix(0xef4444, SURFACE_1, 0.76),
                    move |cx| {
                        edit_and_notify(&confirm_entity, cx, |draft| {
                            draft.backups.confirm_delete_id = None;
                        });
                    },
                )),
        ));
    }

    if let Some(id) = backups.confirm_restore_id.clone() {
        let cancel_entity = entity.clone();
        let confirm_entity = entity;
        return Some(dialog_shell(
            448.0,
            "triangle-alert",
            (
                0xf59e0b,
                mix(0xf59e0b, BORDER_SUBTLE, 0.45),
                mix(0xf59e0b, SURFACE_1, 0.9),
            ),
            "Restore Snapshot",
            Some(format!("Snapshot from {id}").into()),
            div().child(
                div()
                    .text_size(px(12.))
                    .line_height(relative(1.35))
                    .text_color(rgb(TEXT_MUTED))
                    .child("This will overwrite your current app settings and profiles with the data from this snapshot, then reload the app. This action cannot be undone."),
            ),
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(action_button(
                    "backups-restore-cancel",
                    "Cancel",
                    None,
                    16.0,
                    SURFACE_1,
                    BORDER_SUBTLE,
                    TEXT_STRONG,
                    SURFACE_2,
                    move |cx| {
                        edit_and_notify(&cancel_entity, cx, |draft| {
                            draft.backups.confirm_restore_id = None;
                        });
                    },
                ))
                .child(action_button(
                    "backups-restore-confirm",
                    "Yes, restore",
                    Some("archive-restore"),
                    16.0,
                    mix(ACCENT_SECONDARY, SURFACE_1, 0.86),
                    mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.45),
                    ACCENT_SECONDARY,
                    mix(ACCENT_SECONDARY, SURFACE_1, 0.76),
                    move |cx| {
                        edit_and_notify(&confirm_entity, cx, |draft| {
                            draft.backups.confirm_restore_id = None;
                        });
                    },
                )),
        ));
    }

    None
}

// ── UVTools (`UvToolsSettingsTab.tsx`) ───────────────────────────────────

fn uvtools_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let uvtools = &state.uvtools;
    let enabled_entity = entity;
    // `detectPlatform() === 'mac'` decides the executable label.
    let executable_label = if cfg!(target_os = "macos") {
        "UVtools.app"
    } else {
        "UVTools.exe"
    };

    let mut card = section(
        "external-link",
        "UVTools Integration",
        "Automatically open sliced print files in UVTools for further inspection and repair. After slicing, the file is sent directly to UVTools for analysis.",
    );
    card = card.child(
        setting_row(
            "Enable UVTools Integration",
            "Adds a “Send to UVTools” option in the slicing panel.",
            segmented(
                "uvtools-enabled",
                &["OFF", "ON"],
                usize::from(uvtools.enabled),
                false,
                move |index, cx| {
                    edit_and_notify(&enabled_entity, cx, |draft| {
                        draft.uvtools.enabled = index == 1;
                    });
                },
            ),
        )
        .mt(px(12.)),
    );

    if uvtools.enabled {
        let (background, icon_slug, icon_color, label) = if uvtools.show_found_glow {
            (
                mix(ACCENT_SECONDARY, SURFACE_1, 0.88),
                "circle-check",
                ACCENT_SECONDARY,
                "Found!",
            )
        } else if uvtools.show_not_found {
            (
                mix(DANGER, SURFACE_1, 0.90),
                "search",
                TEXT_MUTED,
                "Auto-Discover",
            )
        } else {
            (SURFACE_1, "search", TEXT_MUTED, "Auto-Discover")
        };

        // `autoDiscoverUvToolsPath` needs the native bridge, so the button is
        // inert; `discoveryBusy` still drives the source's spinner.
        let mut discover = div()
            .h(px(36.))
            .px(px(12.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(6.))
            .rounded(px(6.))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(background))
            .text_size(px(14.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(TEXT_STRONG));
        if uvtools.discovery_busy {
            discover = discover.child(spinner(16.));
        } else {
            discover = discover.child(icon(icon_slug, 16., icon_color));
        }
        discover = discover.child(if uvtools.discovery_busy {
            "Scanning…"
        } else {
            label
        });

        card = card.child(
            inner_card()
                .mt(px(8.))
                .child(plain_row(
                    "UVTools Executable Path",
                    format!("Use auto-discover or enter the path to {executable_label} manually."),
                    discover,
                ))
                .child(
                    div().mt(px(8.)).child(text_field(
                        &uvtools.custom_path,
                        &format!("Select or type the path to {executable_label}"),
                        true,
                        30.0,
                        12.0,
                    )),
                ),
        );
    }

    div().flex().flex_col().gap(px(12.)).child(card)
}

// ── Logging (`LoggingSettingsTab.tsx`) ───────────────────────────────────

fn logging_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let logging = &state.logging;
    let level_entity = entity.clone();
    let noise_entity = entity.clone();
    let filter_entity = entity.clone();
    let filter_row_entity = entity.clone();
    let live_entity = entity;

    // `resolveLogDirPath()` reads the Tauri app-log directory; without the
    // bridge the source shows its own non-desktop placeholder.
    let log_path = "<only available in desktop build>";

    let header = |icon_slug: &'static str, title: &'static str, description: &'static str| {
        div()
            .flex()
            .items_start()
            .gap(px(8.))
            .mb(px(12.))
            .child(icon_chip(icon_slug))
            .child(
                div()
                    .flex_1()
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(TEXT_STRONG))
                            .child(title),
                    )
                    .child(
                        div()
                            .mt(px(2.))
                            .text_size(px(12.))
                            .text_color(rgb(TEXT_MUTED))
                            .child(description),
                    ),
            )
    };

    let mut root = div().flex().flex_col().gap(px(14.));

    // Log file location
    root = root.child(
        div()
            .rounded(px(12.)) // rounded-xl
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1))
            .p(px(16.))
            .child(header(
                "scroll-text",
                "Log File",
                "DragonFruit writes structured logs to a platform-specific directory. Share this file when reporting startup or network issues.",
            ))
            .child(
                div()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(rgb(BORDER_SUBTLE))
                    .bg(rgb(SURFACE_2))
                    .flex()
                    .items_stretch()
                    .overflow_hidden()
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .px(px(12.))
                            .py(px(10.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgb(TEXT_STRONG))
                                    .truncate()
                                    .child(log_path),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .border_l_1()
                            .border_color(rgb(BORDER_SUBTLE))
                            .child(
                                // Copy path — the clipboard is unreachable in
                                // this slice, so the tile is inert.
                                div()
                                    .px(px(12.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(icon(
                                        if logging.copied {
                                            "check"
                                        } else {
                                            "clipboard-copy"
                                        },
                                        14.,
                                        if logging.copied { ACCENT } else { TEXT_MUTED },
                                    )),
                            ),
                    ),
            ),
    );

    // Minimum log level
    let mut level_rows = div().flex().flex_col().gap(px(6.));
    for &(value, label, description, color) in LOG_LEVEL_OPTIONS.iter() {
        let active = logging.level == value;
        let row_entity = level_entity.clone();
        let mut row = div()
            .id(format!("log-level-{value}"))
            .flex()
            .items_center()
            .gap(px(12.))
            .rounded(px(8.))
            .border_1()
            .px(px(12.))
            .py(px(8.))
            .border_color(rgb(if active {
                mix(color, BORDER_SUBTLE, 0.4)
            } else {
                BORDER_SUBTLE
            }))
            .bg(rgb(if active {
                mix(color, SURFACE_0, 0.9)
            } else {
                SURFACE_2
            }))
            .cursor_pointer()
            .on_click(move |_, _, cx| {
                edit_and_notify(&row_entity, cx, |draft| draft.logging.level = value);
            })
            .child(
                div()
                    .w(px(8.))
                    .h(px(8.))
                    .flex_shrink_0()
                    .rounded_full()
                    .bg(rgb(color)),
            )
            .child(
                div()
                    .min_w(px(0.))
                    .flex_1()
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(TEXT_STRONG))
                            .child(label),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(TEXT_MUTED))
                            .child(description),
                    ),
            );
        if active {
            row = row.child(icon("check", 14., color));
        }
        level_rows = level_rows.child(row);
    }
    root = root.child(
        div()
            .rounded(px(12.))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1))
            .p(px(16.))
            .child(header(
                "funnel", // `Filter`
                "Minimum Log Level",
                "Controls the least-significant event written to the log file. Changes apply immediately and persist across restarts.",
            ))
            .child(level_rows),
    );

    // Live log viewer
    let filter_meta = LOG_LEVEL_OPTIONS
        .iter()
        .find(|(value, _, _, _)| *value == logging.viewer_filter);
    let filter_color = filter_meta.map(|(_, _, _, color)| *color).unwrap_or(0x6b7280);
    let filter_label = filter_meta.map(|(_, label, _, _)| *label).unwrap_or("?");

    let mut filter_menu = div()
        .absolute()
        .top(px(26.))
        .right(px(0.))
        .min_w(px(100.))
        .rounded(px(8.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_2))
        .overflow_hidden();
    for &(value, label, _, color) in LOG_LEVEL_OPTIONS.iter() {
        let active = logging.viewer_filter == value;
        let row_entity = filter_row_entity.clone();
        filter_menu = filter_menu.child(
            div()
                .id(format!("log-filter-{value}"))
                .w_full()
                .flex()
                .items_center()
                .gap(px(8.))
                .px(px(12.))
                .py(px(6.))
                .text_size(px(12.))
                .text_color(rgb(TEXT_STRONG))
                .when(active, |this| this.bg(rgb(mix(color, SURFACE_0, 0.85))))
                .cursor_pointer()
                .on_click(move |_, _, cx| {
                    edit_and_notify(&row_entity, cx, |draft| {
                        draft.logging.viewer_filter = value;
                        draft.logging.filter_open = false;
                    });
                })
                .child(
                    div()
                        .w(px(6.))
                        .h(px(6.))
                        .flex_shrink_0()
                        .rounded_full()
                        .bg(rgb(color)),
                )
                .child(label)
                .when(active, |this| {
                    this.child(div().flex_1()).child(icon("check", 12., color))
                }),
        );
    }

    let mut toolbar = div()
        .flex()
        .items_center()
        .gap(px(8.))
        .px(px(12.))
        .py(px(8.))
        .border_b_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_2))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .flex_1()
                .min_w(px(0.))
                .child(
                    div()
                        .w(px(6.))
                        .h(px(6.))
                        .flex_shrink_0()
                        .rounded_full()
                        .bg(rgb(if logging.is_live { 0x4ade80 } else { 0x6b7280 })),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("Live Log"),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(TEXT_MUTED))
                        .child("0 lines"),
                ),
        )
        .child(
            div()
                .id("log-noise")
                .flex()
                .items_center()
                .gap(px(6.))
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .px(px(8.))
                .py(px(4.))
                .text_size(px(11.))
                .text_color(rgb(if logging.hide_noise {
                    TEXT_MUTED
                } else {
                    0xfb923c
                }))
                .when(!logging.hide_noise, |this| {
                    this.bg(rgb(mix(0xfb923c, SURFACE_0, 0.88)))
                })
                .cursor_pointer()
                .on_click(move |_, _, cx| {
                    edit_and_notify(&noise_entity, cx, |draft| {
                        draft.logging.hide_noise = !draft.logging.hide_noise;
                    });
                })
                .child(if logging.hide_noise {
                    "noise hidden"
                } else {
                    "noise shown"
                }),
        )
        .child(
            div()
                .relative()
                .child(
                    div()
                        .id("log-filter")
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .rounded(px(6.))
                        .border_1()
                        .border_color(rgb(mix(filter_color, BORDER_SUBTLE, 0.5)))
                        .bg(rgb(mix(filter_color, SURFACE_0, 0.88)))
                        .px(px(8.))
                        .py(px(4.))
                        .text_size(px(12.))
                        .text_color(rgb(TEXT_STRONG))
                        .cursor_pointer()
                        .on_click(move |_, _, cx| {
                            edit_and_notify(&filter_entity, cx, |draft| {
                                draft.logging.filter_open = !draft.logging.filter_open;
                            });
                        })
                        .child(
                            div()
                                .w(px(6.))
                                .h(px(6.))
                                .flex_shrink_0()
                                .rounded_full()
                                .bg(rgb(filter_color)),
                        )
                        .child(filter_label)
                        .child(
                            div()
                                .opacity(0.6)
                                .child(icon("chevron-down", 12., TEXT_STRONG)),
                        ),
                )
                .when(logging.filter_open, |this| this.child(filter_menu)),
        )
        .child(
            div()
                .id("log-live")
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .px(px(8.))
                .py(px(4.))
                .cursor_pointer()
                .on_click(move |_, _, cx| {
                    edit_and_notify(&live_entity, cx, |draft| {
                        draft.logging.is_live = !draft.logging.is_live;
                    });
                })
                .child(icon(
                    if logging.is_live { "pause" } else { "play" },
                    12.,
                    TEXT_MUTED,
                )),
        );
    if !logging.is_live {
        // Manual refresh — `read_log_tail` needs the native bridge.
        toolbar = toolbar.child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .px(px(8.))
                .py(px(4.))
                .child(icon("refresh-ccw", 12., TEXT_MUTED)),
        );
    }
    toolbar = toolbar.child(
        div()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(6.))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .px(px(8.))
            .py(px(4.))
            .child(icon("trash-2", 12., TEXT_MUTED)),
    );

    root = root.child(
        div()
            .rounded(px(12.))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1))
            .overflow_hidden()
            .child(toolbar)
            .child(
                div()
                    .h(px(320.))
                    .bg(rgb(SURFACE_0))
                    .px(px(12.))
                    .py(px(8.))
                    .text_size(px(12.))
                    .child(
                        div()
                            .text_color(rgb(0x6b7280))
                            .child("Log viewer only available in the desktop build."),
                    ),
            ),
    );

    // Info callout
    root = root.child(
        div()
            .rounded(px(8.))
            .border_1()
            .border_color(rgb(mix(ACCENT, BORDER_SUBTLE, 0.6)))
            .bg(rgb(mix(ACCENT, SURFACE_0, 0.06)))
            .px(px(12.))
            .py(px(10.))
            .flex()
            .items_start()
            .gap(px(10.))
            .child(icon("info", 14., ACCENT))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_wrap()
                    .text_size(px(12.))
                    .line_height(relative(1.35))
                    .text_color(rgb(TEXT_MUTED))
                    .child("The minimum log level applies ")
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(TEXT_STRONG))
                            .child("immediately"),
                    )
                    .child(" and is also saved for next launch. The viewer filter and noise suppression are local-only — they do not affect what is written to disk."),
            ),
    );

    root
}

// ── Updates (`UpdatesSettingsTab.tsx` + `UpdateCheckerSection.tsx`) ──────

fn updates_tab(_state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let gear_entity = entity;

    let card = div()
        .relative()
        .flex()
        .flex_1()
        .min_h(px(0.))
        .flex_col()
        .rounded(px(8.)) // rounded-lg
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(12.))
        .child(
            // `IconButton` absolute right-3 top-3, accent tone.
            div()
                .absolute()
                .top(px(12.))
                .right(px(12.))
                .child(
                    icon_button_toned(
                        "updates-channel-settings",
                        "settings",
                        16.0,
                        IconButtonVariant::Surface,
                        IconButtonSize::Sm,
                        IconTone::Accent,
                        false,
                    )
                    .on_click(move |_, _, cx| {
                        edit_and_notify(&gear_entity, cx, |draft| {
                            draft.updates.show_channel_settings = true;
                        });
                    }),
                ),
        )
        .child(
            div().flex().min_h(px(0.)).flex_1().flex_col().child(
                // `useUpdateChecker` auto-checks on mount; without the update
                // bridge `fetchUpdateInfo` answers `null`, which the source
                // renders as its own up-to-date state.
                div()
                    .flex()
                    .flex_1()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(32.))
                    .py(px(64.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(12.))
                            .text_center()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .w(px(48.))
                                    .h(px(48.))
                                    .flex_shrink_0()
                                    .rounded(px(12.))
                                    .border_1()
                                    .border_color(rgb(mix(SUCCESS, BORDER_SUBTLE, 0.4)))
                                    .bg(rgb(mix(SUCCESS, SURFACE_1, 0.82)))
                                    .child(icon("check", 24., SUCCESS)),
                            )
                            .child(
                                div()
                                    .min_w(px(0.))
                                    .child(
                                        div()
                                            .text_size(px(18.))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(SUCCESS))
                                            .child("Up to date"),
                                    )
                                    .child(
                                        div()
                                            .mt(px(6.))
                                            .text_size(px(14.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(rgb(TEXT_MUTED))
                                            .child("You’re on the latest version"),
                                    ),
                            ),
                    )
                    .child(action_button(
                        "updates-check-again",
                        "Check Again",
                        Some("rotate-ccw"),
                        32.0, // px-8
                        SURFACE_1,
                        BORDER_SUBTLE,
                        TEXT_STRONG,
                        SURFACE_2,
                        |_| {},
                    )),
            ),
        );

    div()
        .flex_1()
        .min_h(px(0.))
        .flex()
        .flex_col()
        .gap(px(12.))
        .child(card)
}

/// The `Release Channel` dialog.
fn updates_overlay(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Option<Div> {
    if !state.updates.show_channel_settings {
        return None;
    }
    let updates = &state.updates;
    let pending = updates.pending_channel;
    let is_pending_dev_switch = pending == UpdateChannel::Dev && updates.channel != UpdateChannel::Dev;

    let cancel_entity = entity.clone();
    let apply_entity = entity.clone();
    let stable_entity = entity.clone();
    let dev_entity = entity;

    let channel_card = |id: &'static str,
                        active: bool,
                        slug: &'static str,
                        title: &'static str,
                        subtitle: &'static str,
                        target: UpdateChannel,
                        card_entity: WeakEntity<DragonFruit>| {
        div()
            .id(id)
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(6.))
            .px(px(8.))
            .py(px(24.))
            .min_h(px(64.))
            .rounded(px(6.))
            .border_1()
            .border_color(rgb(if active {
                mix(ACCENT, BORDER_SUBTLE, 0.3)
            } else {
                BORDER_SUBTLE
            }))
            .bg(rgb(if active {
                mix(ACCENT, SURFACE_1, 0.85)
            } else {
                SURFACE_1
            }))
            .cursor_pointer()
            .on_click(move |_, _, cx| {
                edit_and_notify(&card_entity, cx, |draft| {
                    draft.updates.pending_channel = target;
                });
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(TEXT_STRONG))
                    .child(icon(slug, 14., TEXT_STRONG))
                    .child(title),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(TEXT_MUTED))
                    .child(subtitle),
            )
    };

    let mut body = div()
        .flex()
        .flex_col()
        .gap(px(10.))
        .child(channel_card(
            "updates-channel-stable",
            pending == UpdateChannel::Stable,
            "shield-check",
            "Stable Builds",
            "Recommended · Production",
            UpdateChannel::Stable,
            stable_entity,
        ))
        .child(channel_card(
            "updates-channel-dev",
            pending == UpdateChannel::Dev,
            "flask-conical",
            "Development Previews",
            "Early access · May be unstable",
            UpdateChannel::Dev,
            dev_entity,
        ));

    if is_pending_dev_switch {
        body = body.child(
            div()
                .mt(px(2.))
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(mix(0xf59e0b, BORDER_SUBTLE, 0.4)))
                .bg(rgb(mix(0xf59e0b, SURFACE_1, 0.9)))
                .px(px(12.))
                .py(px(10.))
                .flex()
                .items_start()
                .gap(px(10.))
                .child(icon("triangle-alert", 16., 0xf59e0b))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_wrap()
                        .text_size(px(12.))
                        .line_height(relative(1.5))
                        .text_color(rgb(TEXT_MUTED))
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child("Development Previews"),
                        )
                        .child(" are built from the ")
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child("dev"),
                        )
                        .child(" branch for early testing. Expect bugs and breaking changes — switch back to Stable Builds at any time."),
                ),
        );
    }

    Some(dialog_shell(
        512.0, // max-w-lg
        "settings",
        (TEXT_MUTED, BORDER_SUBTLE, SURFACE_1),
        "Release Channel",
        Some("Choose which update feed to check for new versions.".into()),
        body,
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(action_button(
                "updates-channel-cancel",
                "Cancel",
                None,
                16.0,
                SURFACE_1,
                BORDER_SUBTLE,
                TEXT_STRONG,
                SURFACE_2,
                move |cx| {
                    edit_and_notify(&cancel_entity, cx, |draft| {
                        draft.updates.show_channel_settings = false;
                    });
                },
            ))
            .child(
                action_button(
                    "updates-channel-apply",
                    if is_pending_dev_switch {
                        "Switch to Previews"
                    } else {
                        "Apply"
                    },
                    if is_pending_dev_switch {
                        Some("flask-conical")
                    } else {
                        None
                    },
                    16.0,
                    if pending == updates.channel {
                        SURFACE_1
                    } else if is_pending_dev_switch {
                        mix(0xf59e0b, SURFACE_1, 0.86)
                    } else {
                        mix(ACCENT, SURFACE_1, 0.14)
                    },
                    if pending == updates.channel {
                        BORDER_SUBTLE
                    } else if is_pending_dev_switch {
                        mix(0xf59e0b, BORDER_SUBTLE, 0.45)
                    } else {
                        mix(ACCENT, BORDER_SUBTLE, 0.55)
                    },
                    if pending == updates.channel {
                        TEXT_MUTED
                    } else if is_pending_dev_switch {
                        0xf59e0b
                    } else {
                        ACCENT
                    },
                    if pending == updates.channel {
                        SURFACE_2
                    } else if is_pending_dev_switch {
                        mix(0xf59e0b, SURFACE_1, 0.76)
                    } else {
                        mix(ACCENT, SURFACE_1, 0.24)
                    },
                    move |cx| {
                        edit_and_notify(&apply_entity, cx, |draft| {
                            if draft.updates.pending_channel != draft.updates.channel {
                                draft.updates.channel = draft.updates.pending_channel;
                            }
                            draft.updates.show_channel_settings = false;
                        });
                    },
                )
                .when(pending == updates.channel, |this| this.opacity(0.6)),
            ),
    ))
}

// ── About (`SettingsModal.tsx`) ──────────────────────────────────────────

/// `sponsorInitials` — first letter of the first and last word, with a
/// camelCase split so `WickedGrey` → `WG` and `Kryz Slaughter` → `KS`.
fn sponsor_initials(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut spaced = String::with_capacity(name.len() + 4);
    for (index, ch) in chars.iter().enumerate() {
        if index > 0 && ch.is_uppercase() {
            let previous = chars[index - 1];
            if previous.is_lowercase() || previous.is_numeric() {
                spaced.push(' ');
            }
        }
        spaced.push(*ch);
    }
    let letters: Vec<char> = spaced
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .filter_map(|word| word.chars().next())
        .filter(|ch| ch.is_alphabetic())
        .collect();
    if letters.is_empty() {
        return "?".to_owned();
    }
    let picked: Vec<char> = if letters.len() == 1 {
        vec![letters[0]]
    } else {
        vec![letters[0], letters[letters.len() - 1]]
    };
    picked.iter().collect::<String>().to_uppercase()
}

/// `getSponsorRank` — the tier ring colour; `supporters` have no ring.
fn sponsor_ring(total: f32) -> (u32, bool) {
    if total >= 100.0 {
        (0xD4AF37, true) // gold
    } else if total >= 30.0 {
        (0xC0C0C0, true) // silver
    } else if total >= 10.0 {
        (0xCD7F32, true) // bronze
    } else {
        (BORDER_SUBTLE, false)
    }
}

fn about_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let _ = entity;
    let copied = state.about.copied;

    let is_beta_build_channel = DRAGONFRUIT_BUILD_CHANNEL.contains("beta");
    let build_status_label = if is_beta_build_channel {
        "BETA VERSION".to_owned()
    } else if DRAGONFRUIT_BUILD_CHANNEL == "mainline" {
        "Mainline Build".to_owned()
    } else {
        format!("{} Build", DRAGONFRUIT_BUILD_CHANNEL.to_uppercase())
    };
    let (build_status_text, build_status_border, build_status_bg) = if is_beta_build_channel {
        (0xfdba74, mix(0xf97316, BORDER_SUBTLE, 0.16), rgba(with_alpha(0xf97316, 0x0A)))
    } else {
        (
            TEXT_STRONG,
            mix(ACCENT, BORDER_SUBTLE, 0.4),
            rgba(with_alpha(ACCENT, 0x29)),
        )
    };

    // `/dragonfruit_assets/branding/text_logo.svg` is not in the GPUI asset
    // set, so the wordmark renders as text (see the module note).
    let hero = div()
        .rounded(px(12.)) // rounded-xl
        .p(px(16.))
        .bg(linear_gradient(
            145.,
            linear_color_stop(rgb(mix(ACCENT, SURFACE_0, 0.95)), 0.),
            linear_color_stop(rgb(mix(ACCENT_SECONDARY, SURFACE_0, 0.94)), 1.),
        ))
        .child(
            div()
                .relative()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_size(px(28.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("DragonFruit"),
                )
                .child(
                    div()
                        .absolute()
                        .right(px(0.))
                        .top(px(0.))
                        .rounded_full()
                        .px(px(10.))
                        .py(px(2.))
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(WHITE))
                        .bg(linear_gradient(
                            135.,
                            linear_color_stop(rgb(0x3b0764), 0.),
                            linear_color_stop(rgb(0x9a3412), 1.),
                        ))
                        .child("An Open Resin Alliance Project"),
                ),
        )
        .child(
            div()
                .mt(px(12.))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_center()
                        .gap(px(10.))
                        .child(
                            div()
                                .rounded_full()
                                .border_1()
                                .px(px(10.))
                                .py(px(2.))
                                .text_size(px(12.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(if copied { 0x2d8a4e } else { TEXT_STRONG }))
                                .border_color(rgb(if copied {
                                    0x2d8a4e
                                } else {
                                    mix(BORDER_SUBTLE, WHITE, 0.92)
                                }))
                                .bg(if copied {
                                    rgba(with_alpha(0x2d8a4e, 0x1a))
                                } else {
                                    rgba(with_alpha(SURFACE_1, 0xEB))
                                })
                                .child(if copied {
                                    "✓ Copied!".to_owned()
                                } else {
                                    format!("Version {DRAGONFRUIT_VERSION}")
                                }),
                        )
                        .child(
                            div()
                                .rounded_full()
                                .border_1()
                                .px(px(10.))
                                .py(px(2.))
                                .text_size(px(12.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(build_status_text))
                                .border_color(rgb(build_status_border))
                                .bg(build_status_bg)
                                .child(build_status_label),
                        ),
                ),
        );

    // Team & Credits — `contributors` sorted by tone rank, then name.
    let mut people: Vec<(&str, &str, &str)> = CONTRIBUTORS.to_vec();
    people.sort_by(|a, b| {
        let rank = |tone: &str| match tone {
            "founder" => 0,
            "accent" => 1,
            _ => 2,
        };
        rank(a.2).cmp(&rank(b.2)).then_with(|| a.0.cmp(b.0))
    });

    let mut groups = div().mt(px(10.)).flex().flex_col().gap(px(8.));
    for (index, tone) in ["founder", "accent", "secondary"].iter().enumerate() {
        let mut grid = div().grid().grid_cols(2).gap(px(8.));
        let mut count = 0usize;
        for &(name, affiliation, person_tone) in people.iter() {
            if person_tone != *tone {
                continue;
            }
            count += 1;
            let (tone_color, background_mix) = match *tone {
                "founder" => (0xd4a017, 0.94),
                "accent" => (ACCENT, 0.90),
                _ => (ACCENT_SECONDARY, 0.93),
            };
            grid = grid.child(
                div()
                    .rounded(px(6.))
                    .border_1()
                    .border_color(rgb(mix(tone_color, BORDER_SUBTLE, 0.45)))
                    .bg(rgb(mix(tone_color, SURFACE_0, background_mix)))
                    .px(px(12.))
                    .py(px(10.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .text_size(px(14.))
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .truncate()
                                    .text_color(rgb(TEXT_STRONG))
                                    .child(name),
                            )
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .flex()
                                    .items_center()
                                    .gap(px(2.))
                                    .text_color(rgb(TEXT_MUTED))
                                    .child(icon("github", 12., TEXT_MUTED))
                                    .child(affiliation),
                            ),
                    ),
            );
        }
        if count == 0 {
            continue;
        }
        if index > 0 {
            groups = groups.child(
                div()
                    .my(px(2.))
                    .h(px(1.))
                    .rounded_full()
                    .bg(linear_gradient(
                        90.,
                        linear_color_stop(rgba(0x00000000), 0.),
                        linear_color_stop(rgb(BORDER_SUBTLE), 1.),
                    )),
            );
        }
        groups = groups.child(grid);
    }

    let team = div()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(12.))
        .child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .child("TEAM & CREDITS"),
        )
        .child(groups);

    // Sponsors — the baked-in list, monogram-only (no remote avatars).
    let mut sponsor_row = div().mt(px(8.)).flex().flex_wrap().items_center().gap(px(12.));
    for &(name, _link, total) in SPONSORS.iter() {
        let (ring, ringed) = sponsor_ring(total);
        let initials = sponsor_initials(name);
        let mut circle = div()
            .w(px(48.))
            .h(px(48.))
            .flex_shrink_0()
            .rounded_full()
            .flex()
            .items_center()
            .justify_center();
        if ringed {
            circle = circle.bg(rgb(ring));
        }
        circle = circle.child(
            div()
                .w(px(42.))
                .h(px(42.))
                .rounded_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(rgb(SURFACE_1))
                .child(
                    div()
                        .text_size(px(if initials.chars().count() > 1 { 15. } else { 19. }))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(if ringed { ring } else { ACCENT }))
                        .child(initials),
                ),
        );
        sponsor_row = sponsor_row.child(circle);
    }

    let sponsors = div()
        .flex_shrink_0()
        .rounded(px(8.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .px(px(12.))
        .py(px(8.))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_MUTED))
                        .child(icon("heart", 12., ACCENT_SECONDARY))
                        .child("SPONSORS")
                        .child(
                            div()
                                .text_size(px(11.))
                                .font_weight(FontWeight::NORMAL)
                                .text_color(rgb(TEXT_MUTED))
                                .opacity(0.85)
                                .child("via Open Collective"),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .text_size(px(10.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(ACCENT))
                        .child("Sponsor Us")
                        .child(icon("heart", 12., ACCENT)),
                ),
        )
        .child(sponsor_row);

    // ORA footer — the logos are not in the GPUI asset set, so the left mark is
    // the organisation's name and the trailing app icon is omitted.
    let ora = div()
        .flex()
        .items_center()
        .gap(px(16.))
        .rounded(px(12.))
        .border_1()
        .border_color(rgb(mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.52)))
        .bg(rgb(mix(ACCENT_SECONDARY, SURFACE_0, 0.94)))
        .px(px(16.))
        .py(px(12.))
        .child(
            div()
                .w(px(96.))
                .flex_shrink_0()
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(ACCENT_SECONDARY))
                .child("Open Resin Alliance"),
        )
        .child(
            div()
                .min_w(px(0.))
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(px(8.))
                        .text_size(px(12.))
                        .child(icon("github", 14., ACCENT))
                        .child(
                            div()
                                .text_color(rgb(ACCENT))
                                .child("Open-Resin-Alliance/DragonFruit"),
                        )
                        .child(icon("external-link", 12., ACCENT)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(px(8.))
                        .text_size(px(12.))
                        .text_color(rgb(TEXT_STRONG))
                        .child(icon("scroll-text", 14., ACCENT))
                        .child("AGPL-3.0-or-later"),
                ),
        );

    div()
        .flex_1()
        .min_h(px(0.))
        .flex()
        .flex_col()
        .gap(px(14.))
        .child(
            div()
                .id("about-scroll")
                .flex_1()
                .min_h(px(0.))
                .overflow_y_scroll()
                .pr(px(4.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(14.))
                        .pb(px(8.))
                        .child(hero)
                        .child(team),
                ),
        )
        .child(sponsors)
        .child(ora)
}

/// `#rrggbb` → `u32`.
fn hex_color(value: &str) -> u32 {
    u32::from_str_radix(value.trim().trim_start_matches('#'), 16).unwrap_or(0xa3a3a3)
}

fn swatch(value: &str, width: f32, height: f32) -> Div {
    div()
        .w(px(width))
        .h(px(height))
        .flex_shrink_0()
        .rounded(px(4.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(hex_color(value)))
}

/// `#rrggbb` plus an alpha byte, for `gpui::rgba`.
fn with_alpha(color: u32, alpha: u8) -> u32 {
    (color << 8) | alpha as u32
}

/// `SelectDropdown` — the trigger plus its menu, with the open state kept in
/// the draft so a re-render keeps it open.
///
/// The source's `leadingDisplay` is trigger-only; [`select_dropdown`] takes the
/// leading glyph from the selected option, so that row also shows it.
fn select_field(
    id: &'static str,
    options: &[(&'static str, &'static str)],
    value: &str,
    leading: Option<&'static str>,
    width: Option<f32>,
    enabled: bool,
    entity: WeakEntity<DragonFruit>,
    on_change: impl Fn(&str, &mut App) + 'static,
) -> Div {
    let open = state()
        .lock()
        .map(|guard| guard.open_select == Some(id))
        .unwrap_or(false);

    let mut items: Vec<SelectOption> = Vec::with_capacity(options.len());
    for (option_value, label) in options {
        let mut option = SelectOption::new(*option_value, *label);
        if *option_value == value {
            option.icon = leading.map(SharedString::from);
        }
        items.push(option);
    }
    let values: Vec<&'static str> = options.iter().map(|(value, _)| *value).collect();

    let toggle_entity = entity.clone();
    let select_entity = entity.clone();
    let on_change: Rc<dyn Fn(&str, &mut App)> = Rc::new(on_change);

    let mut container = div().flex_shrink_0();
    container = match width {
        Some(width) => container.w(px(width)),
        None => container.w_full(),
    };
    container.child(select_dropdown(
        id,
        &items,
        value,
        open,
        !enabled,
        move |_, _, cx| {
            edit_and_notify(&toggle_entity, cx, |draft| {
                draft.open_select = if draft.open_select == Some(id) {
                    None
                } else {
                    Some(id)
                };
            });
        },
        move |index, _, cx| {
            if let Some(chosen) = values.get(index).copied() {
                on_change(chosen, cx);
                edit_and_notify(&select_entity, cx, |draft| draft.open_select = None);
            }
        },
    ))
}

/// Decimal places a step can land on, capped at 6 like `countStepDecimals`.
fn step_decimals(step: f32) -> u32 {
    if !step.is_finite() {
        return 0;
    }
    let text = format!("{step}");
    match text.split_once('.') {
        Some((_, decimals)) => decimals.len().min(6) as u32,
        None => 0,
    }
}

fn round_to(value: f32, decimals: u32) -> f32 {
    let factor = 10f32.powi(decimals as i32);
    (value * factor).round() / factor
}

/// `NumberInput` — the value box with the source's stacked `ChevronUp` /
/// `ChevronDown` stepper (`absolute inset-y-0 right-0.5 w-4`, two 12px tiles).
///
/// The box is not editable here: typing needs a text-input entity this slice
/// does not have (see the module note).
fn number_input(
    id: &'static str,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    width: Option<f32>,
    height: f32,
    font: f32,
    background: u32,
    enabled: bool,
    on_change: impl Fn(f32, &mut App) + 'static,
) -> Div {
    let decimals = step_decimals(step);
    let on_change: Rc<dyn Fn(f32, &mut App)> = Rc::new(on_change);
    let up = on_change.clone();
    let down = on_change;
    let at_max = value >= max;
    let at_min = value <= min;

    let stepper = |id_suffix: &str,
                   slug: &'static str,
                   disabled: bool,
                   delta: f32,
                   callback: Rc<dyn Fn(f32, &mut App)>| {
        let mut button = div()
            .id(format!("{id}-{id_suffix}"))
            .w(px(12.))
            .h(px(12.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(4.))
            .child(icon(slug, 10., TEXT_MUTED));
        if disabled || !enabled {
            button = button.opacity(0.5);
        } else {
            button = button
                .cursor_pointer()
                .hover(|style| style.bg(rgba(with_alpha(WHITE, 0x1a))))
                .on_click(move |_, _, cx| {
                    let next = round_to((value + delta).clamp(min, max), decimals);
                    callback(next, cx);
                });
        }
        button
    };

    let mut container = div().relative().flex_shrink_0();
    container = match width {
        Some(width) => container.w(px(width)),
        None => container.w_full(),
    };

    container
        .child(
            div()
                .h(px(height))
                .w_full()
                .flex()
                .items_center()
                .rounded(px(6.)) // rounded-md
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(background))
                .pl(px(10.))
                .pr(px(20.)) // pr-5
                .text_size(px(font))
                .text_color(rgb(TEXT_STRONG))
                .when(!enabled, |this| this.opacity(0.5))
                .child(format_number(value)),
        )
        .child(
            div()
                .absolute()
                .top(px(0.))
                .bottom(px(0.))
                .right(px(2.))
                .w(px(16.))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(2.))
                .child(stepper("up", "chevron-up", at_max, step, up))
                .child(stepper("down", "chevron-down", at_min, -step, down)),
        )
}

/// `.ui-checkbox` with the source's `!w-3.5 !h-3.5` override (14px).
fn checkbox(checked: bool, enabled: bool) -> Div {
    let mut box_ = div()
        .w(px(14.))
        .h(px(14.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .border_1();
    if checked {
        box_ = box_
            .border_color(rgb(mix(ACCENT, WHITE, 0.88)))
            .bg(rgb(mix(ACCENT, SURFACE_0, 0.18)))
            .child(icon("check", 10., ACCENT_CONTRAST));
    } else {
        box_ = box_
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1));
    }
    if !enabled {
        box_ = box_.opacity(0.5);
    }
    box_
}

/// `rounded-md border px-3 py-2 text-xs` message strip.
fn message_strip(kind: Option<bool>, text: &str) -> Div {
    let (border, color) = match kind {
        Some(true) => (mix(0xef4444, BORDER_SUBTLE, 0.4), 0xfca5a5),
        _ => (mix(0x22c55e, BORDER_SUBTLE, 0.4), 0x86efac),
    };
    div()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(SURFACE_1))
        .px(px(12.))
        .py(px(8.))
        .text_size(px(12.))
        .text_color(rgb(color))
        .child(text.to_owned())
}

/// `rounded-md border px-2.5 py-2` fact tile on `--surface-0`.
fn stat_card(label: &str, value: impl Into<SharedString>) -> Div {
    div()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .px(px(10.))
        .py(px(8.))
        .child(
            div()
                .text_size(px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .child(label.to_owned()),
        )
        .child(
            div()
                .mt(px(4.))
                .text_size(px(12.))
                .text_color(rgb(TEXT_STRONG))
                .child(value.into()),
        )
}

/// The `text-[11px] uppercase tracking-wide font-semibold` group label.
fn stat_label(label: &str) -> Div {
    div()
        .text_size(px(11.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT_MUTED))
        .child(label.to_owned())
}

/// A read-only text field in the source's shape. Typing needs a text-input
/// entity this slice does not have, so the value (or placeholder) is painted.
/// `mono` is accepted for the source's `font-mono` call sites; no monospace
/// face is registered in this slice.
fn text_field(
    value: &str,
    placeholder: &str,
    mono: bool,
    height: f32,
    font: f32,
) -> Div {
    let _ = mono;
    let empty = value.is_empty();
    div()
        .h(px(height))
        .w_full()
        .flex()
        .items_center()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .px(px(10.))
        .text_size(px(font))
        .text_color(rgb(if empty { TEXT_MUTED } else { TEXT_STRONG }))
        .child(if empty {
            placeholder.to_owned()
        } else {
            value.to_owned()
        })
}

/// `rounded-md border px-3 py-2` callout with a leading icon.
fn callout(border: u32, background: u32, icon_slug: &'static str, icon_color: u32, text: &str) -> Div {
    div()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .px(px(12.))
        .py(px(8.))
        .flex()
        .items_start()
        .gap(px(8.))
        .child(icon(icon_slug, 14., icon_color))
        .child(
            div()
                .flex_1()
                .text_size(px(12.))
                .line_height(relative(1.35))
                .text_color(rgb(TEXT_MUTED))
                .child(text.to_owned()),
        )
}

/// `SettingRow` with the `surface` / `disabled` props the source uses.
///
/// `raised` is `--surface-1` (a row inside a `--surface-0` panel), the default
/// `sunken` is `--surface-0`; `disabled` dims the row to `opacity: 0.68`.
fn setting_row_surface(
    label: impl Into<SharedString>,
    description: impl Into<SharedString>,
    raised: bool,
    disabled: bool,
    controls: Div,
) -> Div {
    let row = div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.))
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(if raised { SURFACE_1 } else { SURFACE_0 }))
        .p(px(6.))
        .child(
            div()
                .min_w(px(0.))
                .pl(px(4.))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(relative(1.2))
                        .text_color(rgb(TEXT_STRONG))
                        .child(label.into()),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .line_height(relative(1.2))
                        .text_color(rgb(TEXT_MUTED))
                        .child(description.into()),
                ),
        )
        .child(div().flex().items_center().gap(px(8.)).child(controls));
    if disabled {
        row.opacity(0.68)
    } else {
        row
    }
}

fn section(icon_slug: &'static str, title: &'static str, description: &'static str) -> Div {
    div()
        .flex()
        .flex_col()
        .rounded(px(6.)) // rounded-md
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(12.))
        .child(section_header(icon_slug, title, description))
}

fn section_header(icon_slug: &'static str, title: &'static str, description: &'static str) -> Div {
    div()
        .flex()
        .items_start()
        .gap(px(8.))
        .child(icon_chip(icon_slug))
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child(title),
                )
                .child(
                    div()
                        .mt(px(2.))
                        .text_size(px(12.))
                        .text_color(rgb(TEXT_MUTED))
                        .child(description),
                ),
        )
}

fn icon_chip(icon_slug: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .w(px(32.))
        .h(px(32.))
        .flex_shrink_0()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        // color-mix(in srgb, surface-2, transparent 8%) over surface-1
        .bg(rgb(mix(SURFACE_2, SURFACE_1, 0.92)))
        .child(icon(icon_slug, 16., ACCENT))
}

/// `SettingRow` with `bordered`: `rounded-md border p-1.5` on `--surface-0`.
fn setting_row(
    label: impl Into<SharedString>,
    description: impl Into<SharedString>,
    control: impl IntoElement,
) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.))
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .p(px(6.))
        .child(
            div()
                .min_w(px(0.))
                .pl(px(4.))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(relative(1.2))
                        .text_color(rgb(TEXT_STRONG))
                        .child(label.into()),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .line_height(relative(1.2))
                        .text_color(rgb(TEXT_MUTED))
                        .child(description.into()),
                ),
        )
        .child(div().flex().items_center().gap(px(8.)).child(control))
}

/// `SettingRow` without `bordered`.
fn plain_row(
    label: impl Into<SharedString>,
    description: impl Into<SharedString>,
    control: impl IntoElement,
) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.))
        .child(
            div()
                .min_w(px(0.))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(relative(1.2))
                        .text_color(rgb(TEXT_STRONG))
                        .child(label.into()),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .line_height(relative(1.2))
                        .text_color(rgb(TEXT_MUTED))
                        .child(description.into()),
                ),
        )
        .child(div().flex().items_center().gap(px(8.)).child(control))
}

/// `rounded-md border p-2.5` on `--surface-0`.
fn inner_card() -> Div {
    div()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .p(px(10.))
        .flex()
        .flex_col()
}

/// `rounded-md border px-2.5 py-1.5` on `--surface-0`.
fn field_card() -> Div {
    div()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .px(px(10.))
        .py(px(6.))
        .flex()
        .flex_col()
}

fn field_label(label: &'static str) -> Div {
    div()
        .mb(px(4.))
        .text_size(px(12.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(TEXT_MUTED))
        .child(label)
}

fn upper_label(label: &'static str) -> Div {
    div()
        .mb(px(4.))
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT_MUTED))
        .child(label)
}

fn divider_label(icon_slug: &'static str, label: &'static str) -> Div {
    div()
        .mt(px(12.))
        .flex()
        .items_center()
        .gap(px(8.))
        .child(div().h(px(1.)).flex_1().bg(rgb(BORDER_SUBTLE)))
        .child(
            div()
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .child(icon(icon_slug, 12., TEXT_MUTED))
                .child(label),
        )
        .child(div().h(px(1.)).flex_1().bg(rgb(BORDER_SUBTLE)))
}

/// `SegmentedControl`: `h-10 min-w-[92px] px-3 text-[12px]` pills, uppercase
/// labels applied at the call site (the source uses CSS `uppercase`).
fn segmented(
    id: &'static str,
    options: &'static [&'static str],
    active: usize,
    full_width: bool,
    on_select: impl Fn(usize, &mut App) + 'static,
) -> Div {
    let on_select: Rc<dyn Fn(usize, &mut App)> = Rc::new(on_select);
    let mut row = div().flex().items_center().gap(px(4.));
    if full_width {
        row = row.w_full();
    }
    for (index, label) in options.iter().enumerate() {
        let is_active = index == active;
        let handler = on_select.clone();
        let (background, border, text) = if is_active {
            (
                mix(ACCENT, SURFACE_0, 0.24),
                mix(ACCENT, 0xffffff, 0.9),
                mix(ACCENT, TEXT_STRONG, 0.25),
            )
        } else {
            (SURFACE_1, BORDER_SUBTLE, TEXT_MUTED)
        };
        let mut option = div()
            .id(format!("{id}-{index}"))
            .h(px(40.))
            .px(px(12.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(6.))
            .border_1()
            .border_color(rgb(border))
            .bg(rgb(background))
            .text_size(px(12.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(text))
            .cursor_pointer()
            .on_click(move |_, _, cx| handler(index, cx))
            .child(*label);
        option = if full_width {
            option.flex_1()
        } else {
            option.min_w(px(92.))
        };
        row = row.child(option);
    }
    row
}

/// `ScrollableNumberField`: 32px steppers around a centred value box.
fn number_control(
    id: &'static str,
    value: f32,
    unit: &'static str,
    step: f32,
    min: f32,
    max: f32,
    width: f32,
    on_change: impl Fn(f32, &mut App) + 'static,
) -> Div {
    let on_change: Rc<dyn Fn(f32, &mut App)> = Rc::new(on_change);
    let decrease = on_change.clone();
    let increase = on_change;
    div()
        .w(px(width))
        .flex()
        .items_center()
        .gap(px(4.))
        .child(
            div()
                .id(format!("{id}-minus"))
                .w(px(32.))
                .h(px(32.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_0))
                .cursor_pointer()
                .hover(|style| style.bg(rgb(mix(SURFACE_0, 0xffffff, 0.96))))
                .child(icon("minus", 14., TEXT_MUTED))
                .on_click(move |_, _, cx| decrease((value - step).max(min), cx)),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .h(px(32.))
                .flex()
                .items_center()
                .justify_center()
                .gap(px(2.))
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_0))
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child(format_number(value))
                .child(
                    div()
                        .text_size(px(10.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(TEXT_MUTED))
                        .child(unit),
                ),
        )
        .child(
            div()
                .id(format!("{id}-plus"))
                .w(px(32.))
                .h(px(32.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_0))
                .cursor_pointer()
                .hover(|style| style.bg(rgb(mix(SURFACE_0, 0xffffff, 0.96))))
                .child(icon("plus", 14., TEXT_MUTED))
                .on_click(move |_, _, cx| increase((value + step).min(max), cx)),
        )
}

fn format_number(value: f32) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    if (rounded - rounded.round()).abs() < 0.0001 {
        format!("{:.0}", rounded)
    } else {
        let mut text = format!("{:.2}", rounded);
        while text.ends_with('0') {
            text.pop();
        }
        text
    }
}

/// `.ui-range`: an 8px track with the accent fill and a knob at its end.
fn slider(value: f32, min: f32, max: f32) -> Div {
    let fraction = if (max - min).abs() < f32::EPSILON {
        0.0
    } else {
        ((value - min) / (max - min)).clamp(0.0, 1.0)
    };
    div()
        .w_full()
        .h(px(8.))
        .rounded_full()
        .bg(rgb(SURFACE_2))
        .child(
            div()
                .h(px(8.))
                .w(relative(fraction))
                .rounded_full()
                .bg(rgb(ACCENT))
                .flex()
                .items_center()
                .justify_end()
                .child(
                    div()
                        .w(px(12.))
                        .h(px(12.))
                        .flex_shrink_0()
                        .rounded_full()
                        .border_1()
                        .border_color(rgb(ACCENT))
                        .bg(rgb(ACCENT_CONTRAST)),
                ),
        )
}

/// Label + value line above a range track, inside a `px-2.5 py-1.5` card.
///
/// The track renders the current value; GPUI has no DOM range input in this
/// slice, so there is no drag handler (see the module note).
fn slider_field(
    label: &'static str,
    value_text: impl Into<SharedString>,
    value: f32,
    min: f32,
    max: f32,
) -> Div {
    field_card().child(
        div()
            .flex()
            .items_center()
            .justify_between()
            .mb(px(6.))
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(TEXT_MUTED))
                    .child(label),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(TEXT_STRONG))
                    .child(value_text.into()),
            ),
    )
    .child(slider(value, min, max))
}

/// Footer action: `!h-10 !px-4 text-sm gap-1.5`.
fn action_button(
    id: &'static str,
    label: &'static str,
    icon_slug: Option<&'static str>,
    padding: f32,
    background: u32,
    border: u32,
    text: u32,
    hover: u32,
    on_click: impl Fn(&mut App) + 'static,
) -> Stateful<Div> {
    let mut button = div()
        .id(id)
        .h(px(40.))
        .px(px(padding))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.))
        .flex_shrink_0()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .text_size(px(14.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(text))
        .cursor_pointer()
        .hover(move |style| style.bg(rgb(hover)))
        .on_click(move |_, _, cx| on_click(cx));
    if let Some(slug) = icon_slug {
        button = button.child(icon(slug, 16., text));
    }
    button.child(label)
}

/// `!h-8 !px-2.5 text-xs` theme action (`ui-button-secondary`).
fn small_button(
    id: &'static str,
    label: &'static str,
    enabled: bool,
    on_click: impl Fn(&mut App) + 'static,
) -> Stateful<Div> {
    let (background, border, text) = if enabled {
        (SURFACE_1, BORDER_SUBTLE, TEXT_STRONG)
    } else {
        (SURFACE_1, BORDER_SUBTLE, TEXT_MUTED)
    };
    div()
        .id(id)
        .h(px(32.))
        .px(px(10.))
        .flex()
        .items_center()
        .justify_center()
        .flex_shrink_0()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(text))
        .when(enabled, |this| {
            this.cursor_pointer()
                .hover(|style| style.bg(rgb(SURFACE_2)))
                .on_click(move |_, _, cx| on_click(cx))
        })
        .when(!enabled, |this| this.opacity(0.5))
        .child(label)
}
