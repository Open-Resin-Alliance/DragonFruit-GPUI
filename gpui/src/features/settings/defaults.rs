use super::*;

// `w-72`, `max-w-[72rem]`, `p-5`, `w-36` (the dropdown trigger width).
pub(crate) const SIDEBAR_WIDTH: f32 = 288.0;
pub(crate) const CARD_MAX_WIDTH: f32 = 1152.0;
pub(crate) const BACKDROP_PADDING: f32 = 20.0;
pub(crate) const CONTROL_WIDTH: f32 = 144.0;

// `DEFAULT_*` constants from the React sources.
pub(crate) const DEFAULT_MESH_COLOR: &str = "#a3a3a3";
pub(crate) const DEFAULT_HEATMAP_MIN_ANGLE: f32 = 0.0;
pub(crate) const DEFAULT_HEATMAP_MAX_ANGLE: f32 = 45.0;
pub(crate) const DEFAULT_AMBIENT_INTENSITY: f32 = 0.28;
pub(crate) const DEFAULT_DIRECTIONAL_INTENSITY: f32 = 1.12;
pub(crate) const DEFAULT_MATERIAL_ROUGHNESS: f32 = 0.55;
pub(crate) const DEFAULT_XRAY_OPACITY: f32 = 0.25;
pub(crate) const DEFAULT_HOVER_TINT_STRENGTH: f32 = 0.5;
pub(crate) const DEFAULT_SELECTED_TINT_STRENGTH: f32 = 0.70;
pub(crate) const DEFAULT_HEATMAP_COLORS: [&str; 5] = ["#E55959", "#E5A559", "#D9D959", "#73D973", "#666666"];
pub(crate) const DEFAULT_FOV: f32 = 50.0;
pub(crate) const FOV_MIN: f32 = 15.0;
pub(crate) const FOV_MAX: f32 = 120.0;
pub(crate) const DEFAULT_UI_SCALE: f32 = 1.0;
pub(crate) const MIN_UI_SCALE: f32 = 0.25;
pub(crate) const MAX_UI_SCALE: f32 = 4.0;
pub(crate) const UI_SCALE_PRESETS: [f32; 6] = [0.75, 0.9, 1.0, 1.1, 1.25, 1.5];
/// `UI_SCALE_PRESETS.map(...)` plus the trailing `Custom` entry.
pub(crate) const UI_SCALE_OPTIONS: [(&str, &str); 7] = [
    ("0.75", "75%"),
    ("0.9", "90%"),
    ("1", "100%"),
    ("1.1", "110%"),
    ("1.25", "125%"),
    ("1.5", "150%"),
    ("custom", "Custom"),
];

pub(crate) const LANGUAGE_LABELS: [&str; 6] = ["English", "Español", "Deutsch", "Français", "Slovenčina", "Čeština"];
/// `SUPPORTED_LOCALES` — the `SelectDropdown` values behind the labels.
pub(crate) const LANGUAGE_VALUES: [&str; 6] = ["en", "es", "de", "fr", "sk", "cs"];
pub(crate) const LANGUAGE_OPTIONS: [(&str, &str); 6] = [
    ("en", "English"),
    ("es", "Español"),
    ("de", "Deutsch"),
    ("fr", "Français"),
    ("sk", "Slovenčina"),
    ("cs", "Čeština"),
];
pub(crate) const TOOL_LAYOUT_LABELS: [&str; 2] = ["Top", "Left"];
pub(crate) const RAFT_LABELS: [&str; 3] = ["Off", "Line", "Solid"];
pub(crate) const FEEL_LABELS: [&str; 4] = ["Raw", "Precise", "Balanced", "Fast"];
pub(crate) const TRACKPAD_LABELS: [&str; 3] = ["Off", "Pan", "Orbit"];
pub(crate) const SHADER_LABELS: [&str; 6] = ["Standard", "Overhangs", "Mesh", "Wireframe", "X-Ray", "Normals"];
pub(crate) const MATCAP_LABELS: [&str; 3] = ["Neutral", "Cool", "Warm"];
pub(crate) const PREVIEW_MODELS: [&str; 3] = ["Cube", "Sphere", "Knot"];
pub(crate) const THEME_PRESETS: [&str; 4] = ["DragonFruit Dark", "DragonFruit Light", "Concepts 3D", "Atlas 3DSS"];
pub(crate) const SCHEME_LABELS: [&str; 2] = ["Dark", "Light"];
pub(crate) const PNG_MODE_LABELS: [&str; 3] = ["Auto", "Off", "On"];

// The `(value, label)` pairs behind each `SelectDropdown` / `Select`.
pub(crate) const TOOL_LAYOUT_OPTIONS: [(&str, &str); 2] = [("horizontal", "Top"), ("vertical", "Left")];
pub(crate) const RAFT_OPTIONS: [(&str, &str); 3] = [("off", "Off"), ("line", "Line"), ("solid", "Solid")];
pub(crate) const FEEL_VALUES: [&str; 4] = ["raw", "precise", "balanced", "fast"];
pub(crate) const FEEL_OPTIONS: [(&str, &str); 4] = [
    ("raw", "Raw"),
    ("precise", "Precise"),
    ("balanced", "Balanced"),
    ("fast", "Fast"),
];
pub(crate) const TRACKPAD_VALUES: [&str; 3] = ["off", "pan", "orbit"];
pub(crate) const TRACKPAD_OPTIONS: [(&str, &str); 3] = [("off", "Off"), ("pan", "Pan"), ("orbit", "Orbit")];
pub(crate) const SHADER_VALUES: [&str; 6] = [
    "soft_clay",
    "overhang_heatmap",
    "opaque_wire_mesh",
    "wireframe",
    "xray",
    "normal_debug",
];
pub(crate) const SHADER_OPTIONS: [(&str, &str); 6] = [
    ("soft_clay", "Standard"),
    ("overhang_heatmap", "Overhangs"),
    ("opaque_wire_mesh", "Mesh"),
    ("wireframe", "Wireframe"),
    ("xray", "X-Ray"),
    ("normal_debug", "Normals"),
];
pub(crate) const PREVIEW_MODEL_VALUES: [&str; 3] = ["cube", "sphere", "knot"];
pub(crate) const PREVIEW_MODEL_OPTIONS: [(&str, &str); 3] =
    [("cube", "Cube"), ("sphere", "Sphere"), ("knot", "Knot")];
pub(crate) const THEME_PRESET_VALUES: [&str; 4] = [
    "dragonfruit-dark",
    "dragonfruit-light",
    "concepts-3d",
    "atlas-3dss",
];
pub(crate) const THEME_PRESET_OPTIONS: [(&str, &str); 4] = [
    ("dragonfruit-dark", "DragonFruit Dark"),
    ("dragonfruit-light", "DragonFruit Light"),
    ("concepts-3d", "Concepts 3D"),
    ("atlas-3dss", "Atlas 3DSS"),
];
pub(crate) const SCHEME_VALUES: [&str; 2] = ["dark", "light"];
pub(crate) const SCHEME_OPTIONS: [(&str, &str); 2] = [("dark", "Dark"), ("light", "Light")];
/// `SpaceMouseSettings['pivotMode']` — the native `<Select>` in the 3D Mouse tab.
pub(crate) const PIVOT_OPTIONS: [(&str, &str); 2] = [("auto", "Auto"), ("camera-ray", "Camera Raycast")];
pub(crate) const WORKSPACES: [(&str, &str); 4] = [
    ("Prepare", "Model prep and transform workflows"),
    ("Analysis", "Island diagnostics and inspection tools"),
    ("Support", "Support placement and editing workspace"),
    ("Export", "Final output and export pipeline"),
];

/// `#ffffff` — the base of the `color-mix(..., white N%)` recipes.
pub(crate) const WHITE: u32 = 0xffffff;
/// `#000000` — the base of the `color-mix(..., black N%)` recipes.
pub(crate) const BLACK: u32 = 0x000000;

// `view3dPreferences.ts` / `DEFAULT_VIEW3D_SETTINGS`.
pub(crate) const DEFAULT_VIEW3D_WIDTH_MM: f32 = 218.0;
pub(crate) const DEFAULT_VIEW3D_DEPTH_MM: f32 = 123.0;
pub(crate) const DEFAULT_VIEW3D_MAX_Z_MM: f32 = 250.0;
pub(crate) const DEFAULT_VIEW3D_SCREEN_WIDTH_PX: f32 = 2560.0;
pub(crate) const DEFAULT_VIEW3D_SCREEN_HEIGHT_PX: f32 = 1440.0;

// `plateCascade.ts`.
pub(crate) const DEFAULT_FIXED_PLATE_COLUMNS: f32 = 3.0;
pub(crate) const MAX_FIXED_PLATE_COLUMNS: f32 = 12.0;

// `spacemousePreferences.ts` / `DEFAULT_SPACEMOUSE_SETTINGS`.
pub(crate) const DEFAULT_SPACEMOUSE_SENSITIVITY: f32 = 2.3;
pub(crate) const DEFAULT_SPACEMOUSE_DEADZONE: f32 = 0.03;

// `sceneAutosavePreferences.ts` / `DEFAULT_SCENE_AUTOSAVE_SETTINGS`.
pub(crate) const DEFAULT_AUTOSAVE_DEBOUNCE_MS: f32 = 45_000.0;
pub(crate) const DEFAULT_AUTOSAVE_COOLDOWN_MS: f32 = 180_000.0;
pub(crate) const DEFAULT_AUTOSAVE_CAP_MS: f32 = 300_000.0;

// `LocalBackupsSettingsTab.tsx` defaults.
pub(crate) const DEFAULT_AUTO_SYNC_MINUTES: f32 = 15.0;

// `LoggingSettingsTab.tsx`.
pub(crate) const LOG_LEVEL_OPTIONS: [(&str, &str, &str, u32); 5] = [
    ("error", "Error", "Only critical failures", 0xf87171),
    ("warn", "Warn", "Warnings and errors", 0xfb923c),
    ("info", "Info", "Normal operational messages (recommended)", 0x4ade80),
    ("debug", "Debug", "Verbose internal state", 0x60a5fa),
    ("trace", "Trace", "Maximum verbosity — may impact performance", 0xa78bfa),
];

/// `SettingsModal.tsx` — `DRAGONFRUIT_VERSION` / `DRAGONFRUIT_BUILD_CHANNEL`.
pub(crate) const DRAGONFRUIT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const DRAGONFRUIT_BUILD_CHANNEL: &str = "mainline";
pub(crate) const DRAGONFRUIT_REPO_URL: &str = "https://github.com/Open-Resin-Alliance/DragonFruit";

/// `contributors.json`, in file order; the About tab sorts by tone then name.
pub(crate) const CONTRIBUTORS: [(&str, &str, &str); 22] = [
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
pub(crate) const SPONSORS: [(&str, Option<&str>, f32); 18] = [
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
pub(crate) const EXPERIMENTS: [(&str, &str, &str, bool); 3] = [
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
pub(crate) const HOTKEY_DEFAULTS: [(&str, &str, &str, Option<&str>, &str); 33] = [
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
