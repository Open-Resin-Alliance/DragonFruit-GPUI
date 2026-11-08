//! `settings.json` — user settings, grouped the way Settings → tabs groups
//! them.
//!
//! Every field is `#[serde(default)]`, so a file written by an older build
//! (which lacks yesterday's fields) still loads. Defaults are the source app's
//! own: each group cites the module under `src/components/settings/` or
//! `src/features/` that owns it. Enum fields also fall back to their default on
//! an unknown value rather than failing the whole document, mirroring the
//! source's `normalize*` helpers.
//!
//! `docs/dev/config-schemas.md` only covers the compile-time `src/config/*.json`
//! files; it carries no runtime-settings schema, so the names and defaults here
//! come from the preference modules cited per field.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Version this crate writes. A file with a higher one is never overwritten
/// (see [`crate::Store::is_read_only`]).
pub const SETTINGS_SCHEMA_VERSION: u32 = 1;

/// Tolerates an unknown enum value the way the source's `normalize*` helpers
/// do; `#[serde(default)]` covers the missing case.
///
/// The value is buffered through `serde_json::Value` first so a rejected
/// variant still consumes exactly one JSON value — returning a default without
/// consuming would desynchronise the enclosing map.
pub(crate) mod de {
    use serde::de::DeserializeOwned;
    use serde::{Deserialize, Deserializer};

    pub fn enum_or_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: DeserializeOwned + Default,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        Ok(serde_json::from_value(value).unwrap_or_default())
    }
}

/// `src/i18n.ts`'s `SUPPORTED_LOCALES` joined by `,`.
pub const SUPPORTED_LOCALES: [&str; 6] = ["en", "es", "de", "fr", "sk", "cs"];
/// `src/i18n.ts`'s `DEFAULT_LOCALE`.
pub const DEFAULT_LOCALE: &str = "en";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// 0 means "written before this crate versioned the file"; normalised to
    /// [`SETTINGS_SCHEMA_VERSION`] on load.
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub general: GeneralSettings,
    #[serde(default)]
    pub camera: CameraSettings,
    #[serde(default)]
    pub workspaces: WorkspaceCameraSettings,
    #[serde(default)]
    pub multi_plate: MultiPlateSettings,
    #[serde(default)]
    pub mesh: MeshSettings,
    #[serde(default)]
    pub performance: PerformanceSettings,
    #[serde(default)]
    pub space_mouse: SpaceMouseSettings,
    #[serde(default)]
    pub plugins: PluginsSettings,
    #[serde(default)]
    pub scene_autosave: SceneAutosaveSettings,
    #[serde(default)]
    pub backups: BackupsSettings,
    #[serde(default)]
    pub uvtools: UvToolsSettings,
    #[serde(default)]
    pub ui: UiSettings,
    #[serde(default)]
    pub hotkeys: HotkeySettings,
    #[serde(default)]
    pub logging: LoggingSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            general: GeneralSettings::default(),
            camera: CameraSettings::default(),
            workspaces: WorkspaceCameraSettings::default(),
            multi_plate: MultiPlateSettings::default(),
            mesh: MeshSettings::default(),
            performance: PerformanceSettings::default(),
            space_mouse: SpaceMouseSettings::default(),
            plugins: PluginsSettings::default(),
            scene_autosave: SceneAutosaveSettings::default(),
            backups: BackupsSettings::default(),
            uvtools: UvToolsSettings::default(),
            ui: UiSettings::default(),
            hotkeys: HotkeySettings::default(),
            logging: LoggingSettings::default(),
        }
    }
}

// --- General -------------------------------------------------------------

/// General tab: `GeneralSettingsTab.tsx`, `uiScalePreference.ts`,
/// `floatingLayoutPreferences.ts`, `importDefaultsPreferences.ts`,
/// `supportPlacementPreferences.ts`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneralSettings {
    /// `DEFAULT_UI_SCALE` (100 %), clamped to `[0.25, 4]` by the source.
    #[serde(default = "default_ui_scale")]
    pub ui_scale: f32,
    /// `DEFAULT_LOCALE` (`src/i18n.ts`).
    #[serde(default = "default_language")]
    pub language: String,
    /// `isFloatingLayoutPersistenceEnabled()` defaults to `true`.
    #[serde(default = "default_true")]
    pub floating_layout_persistence: bool,
    /// `getToolLayout()` falls back to `'horizontal'`.
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub tool_layout: ToolLayout,
    /// `DEFAULT_IMPORT_DEFAULTS_SETTINGS`.
    #[serde(default)]
    pub import_defaults: ImportDefaults,
    /// `DEFAULT_SUPPORT_PLACEMENT_HELP_ENABLED`.
    #[serde(default = "default_true")]
    pub support_placement_help: bool,
}

fn default_ui_scale() -> f32 {
    1.0
}
fn default_language() -> String {
    DEFAULT_LOCALE.to_owned()
}
fn default_true() -> bool {
    true
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            ui_scale: default_ui_scale(),
            language: default_language(),
            floating_layout_persistence: true,
            tool_layout: ToolLayout::default(),
            import_defaults: ImportDefaults::default(),
            support_placement_help: true,
        }
    }
}

/// `'vertical' | 'horizontal'` — `floatingLayoutPreferences.ts`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolLayout {
    Vertical,
    #[default]
    Horizontal,
}

/// `ImportDefaultsSettings` — `src/features/scene/importDefaultsPreferences.ts`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDefaults {
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub raft_bottom_mode: RaftBottomMode,
    #[serde(default = "default_true")]
    pub raft_wall_enabled: bool,
    #[serde(default = "default_true")]
    pub roots_enabled: bool,
    #[serde(default)]
    pub auto_repair: bool,
    #[serde(default)]
    pub auto_repair_scenes: bool,
}

impl Default for ImportDefaults {
    fn default() -> Self {
        Self {
            raft_bottom_mode: RaftBottomMode::Solid,
            raft_wall_enabled: true,
            roots_enabled: true,
            auto_repair: false,
            auto_repair_scenes: false,
        }
    }
}

/// `RaftBottomMode` — `src/supports/Rafts/Crenelated/RaftTypes.ts`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RaftBottomMode {
    Off,
    #[default]
    Solid,
    Line,
}

// --- Camera --------------------------------------------------------------

/// Camera tab: `cameraFeelPreferences.ts`, `cameraFovPreferences.ts`,
/// `cameraProjectionPreferences.ts`, `cameraTrackpadPreferences.ts`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraSettings {
    /// `DEFAULT_CAMERA_FEEL_SETTINGS`.
    #[serde(default)]
    pub feel: CameraFeel,
    /// Vertical field of view in degrees (`DEFAULT_FOV_DEG`).
    #[serde(default = "default_fov")]
    pub fov: f32,
    /// The source stores `{ mode }`; this stores the mode itself.
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub projection: CameraProjection,
    /// `DEFAULT_CAMERA_TRACKPAD_SETTINGS`.
    #[serde(default)]
    pub trackpad: CameraTrackpadSettings,
}

fn default_fov() -> f32 {
    50.0
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            feel: CameraFeel::default(),
            fov: default_fov(),
            projection: CameraProjection::default(),
            trackpad: CameraTrackpadSettings::default(),
        }
    }
}

/// `CameraFeelSettings`'s single field.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraFeel {
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub preset: CameraFeelPreset,
}

impl Default for CameraFeel {
    fn default() -> Self {
        Self {
            preset: CameraFeelPreset::default(),
        }
    }
}

/// `CameraFeelPreset`; `DEFAULT_CAMERA_FEEL_SETTINGS.preset` is `'precise'`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CameraFeelPreset {
    Raw,
    #[default]
    Precise,
    Balanced,
    Fast,
}

/// `CameraProjectionMode`; `DEFAULT_CAMERA_PROJECTION_SETTINGS.mode` is
/// `'orthographic'`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CameraProjection {
    #[default]
    Orthographic,
    Perspective,
}

/// `CameraTrackpadSettings`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraTrackpadSettings {
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub primary_action: CameraTrackpadPrimaryAction,
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub modifier_key: CameraTrackpadModifierKey,
    #[serde(default = "default_trackpad_acceleration")]
    pub pan_acceleration: f32,
    #[serde(default = "default_trackpad_acceleration")]
    pub orbit_acceleration: f32,
    #[serde(default = "default_trackpad_acceleration")]
    pub zoom_acceleration: f32,
}

fn default_trackpad_acceleration() -> f32 {
    2.0
}

impl Default for CameraTrackpadSettings {
    fn default() -> Self {
        Self {
            primary_action: CameraTrackpadPrimaryAction::Pan,
            modifier_key: CameraTrackpadModifierKey::Shift,
            pan_acceleration: default_trackpad_acceleration(),
            orbit_acceleration: default_trackpad_acceleration(),
            zoom_acceleration: default_trackpad_acceleration(),
        }
    }
}

/// `CameraTrackpadPrimaryAction`; the source falls back to `'pan'`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CameraTrackpadPrimaryAction {
    Off,
    #[default]
    Pan,
    Orbit,
}

/// `CameraTrackpadModifierKey`; the stored default is `'shift'` (the source's
/// invalid-value fallback is `'alt'`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CameraTrackpadModifierKey {
    Alt,
    #[default]
    Shift,
}

// --- Workspaces ----------------------------------------------------------

/// Workspace tab: `workspaceCameraPreferences.ts`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceCameraSettings {
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub scope: CameraScopeMode,
    #[serde(default)]
    pub defaults: CameraModeDefaults,
    #[serde(default)]
    pub selection_highlight_defaults: SelectionHighlightDefaults,
    #[serde(default)]
    pub higher_contrast_model_edges: bool,
}

impl Default for WorkspaceCameraSettings {
    fn default() -> Self {
        Self {
            scope: CameraScopeMode::Global,
            defaults: CameraModeDefaults::default(),
            selection_highlight_defaults: SelectionHighlightDefaults::default(),
            higher_contrast_model_edges: false,
        }
    }
}

/// `CameraScopeMode`; the source falls back to `'workspace'` for an unknown
/// value, `DEFAULT_WORKSPACE_CAMERA_SETTINGS.scope` is `'global'`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CameraScopeMode {
    #[default]
    Global,
    Workspace,
}

/// One projection per `SupportMode` (`src/supports/types.ts`), all
/// `'orthographic'`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraModeDefaults {
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub prepare: CameraProjection,
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub analysis: CameraProjection,
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub support: CameraProjection,
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub export: CameraProjection,
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub printing: CameraProjection,
}

impl Default for CameraModeDefaults {
    fn default() -> Self {
        Self {
            prepare: CameraProjection::Orthographic,
            analysis: CameraProjection::Orthographic,
            support: CameraProjection::Orthographic,
            export: CameraProjection::Orthographic,
            printing: CameraProjection::Orthographic,
        }
    }
}

/// One `SelectionHighlightMode` per `SupportMode`, all `'tint'`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionHighlightDefaults {
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub prepare: SelectionHighlightMode,
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub analysis: SelectionHighlightMode,
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub support: SelectionHighlightMode,
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub export: SelectionHighlightMode,
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub printing: SelectionHighlightMode,
}

impl Default for SelectionHighlightDefaults {
    fn default() -> Self {
        Self {
            prepare: SelectionHighlightMode::Tint,
            analysis: SelectionHighlightMode::Tint,
            support: SelectionHighlightMode::Tint,
            export: SelectionHighlightMode::Tint,
            printing: SelectionHighlightMode::Tint,
        }
    }
}

/// `SelectionHighlightMode` — `src/components/selection`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SelectionHighlightMode {
    #[default]
    Tint,
    Spotlight,
    #[serde(rename = "none")]
    None,
}

// --- Multi-plate ---------------------------------------------------------

/// Multi-Plate tab: `multiPlatePreferences.ts` and
/// `DEFAULT_FIXED_PLATE_COLUMNS` in `plateCascade.ts`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiPlateSettings {
    #[serde(default = "default_true")]
    pub follow_landed_plate: bool,
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub plate_ordering: PlateOrderingMode,
    /// `DEFAULT_FIXED_PLATE_COLUMNS` is 3.
    #[serde(default = "default_fixed_plate_columns")]
    pub fixed_plate_columns: u32,
}

impl Default for MultiPlateSettings {
    fn default() -> Self {
        Self {
            follow_landed_plate: true,
            plate_ordering: PlateOrderingMode::Fixed,
            fixed_plate_columns: default_fixed_plate_columns(),
        }
    }
}

fn default_fixed_plate_columns() -> u32 {
    3
}

/// `PlateOrdering['mode']`; `'fixed'` is the default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlateOrderingMode {
    #[default]
    Fixed,
    Dynamic,
}

// --- Mesh ----------------------------------------------------------------

/// Mesh tab: the `PersistedMeshAppearance` document written under
/// `mesh-appearance-settings` in
/// `src/features/scene/useSceneCollectionManager.ts`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeshSettings {
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub shader_type: MeshShaderType,
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub matcap_variant: MatcapVariant,
    #[serde(default = "default_true")]
    pub flat_use_vertex_colors: bool,
    #[serde(default = "default_ambient_intensity")]
    pub ambient_intensity: f32,
    #[serde(default = "default_directional_intensity")]
    pub directional_intensity: f32,
    #[serde(default = "default_material_roughness")]
    pub material_roughness: f32,
    /// `DEFAULT_BAKED_OCCLUSION_INTENSITY` — `src/features/scene/bakedOcclusion.ts`.
    #[serde(default = "default_baked_ao_intensity")]
    pub baked_ao_intensity: f32,
    #[serde(default = "default_wireframe_thickness")]
    pub wireframe_thickness_px: f32,
    #[serde(default = "default_xray_opacity")]
    pub xray_opacity: f32,
    #[serde(default)]
    pub heatmap_min_angle: f32,
    #[serde(default = "default_heatmap_max_angle")]
    pub heatmap_max_angle: f32,
    /// `DEFAULT_HEATMAP_COLORS`, five stops.
    #[serde(default = "default_heatmap_colors")]
    pub heatmap_colors: Vec<String>,
    #[serde(default = "default_mesh_color")]
    pub mesh_color: String,
    #[serde(default = "default_hover_tint_strength")]
    pub hover_tint_strength: f32,
    #[serde(default = "default_selected_tint_strength")]
    pub selected_tint_strength: f32,
}

fn default_ambient_intensity() -> f32 {
    0.28
}
fn default_directional_intensity() -> f32 {
    1.12
}
fn default_material_roughness() -> f32 {
    0.55
}
fn default_baked_ao_intensity() -> f32 {
    0.85
}
fn default_wireframe_thickness() -> f32 {
    1.5
}
fn default_xray_opacity() -> f32 {
    0.25
}
fn default_heatmap_max_angle() -> f32 {
    45.0
}
fn default_heatmap_colors() -> Vec<String> {
    ["#E55959", "#E5A559", "#D9D959", "#73D973", "#666666"]
        .iter()
        .map(|color| (*color).to_owned())
        .collect()
}
fn default_mesh_color() -> String {
    "#a3a3a3".to_owned()
}
fn default_hover_tint_strength() -> f32 {
    0.5
}
fn default_selected_tint_strength() -> f32 {
    0.70
}

impl Default for MeshSettings {
    fn default() -> Self {
        Self {
            shader_type: MeshShaderType::default(),
            matcap_variant: MatcapVariant::default(),
            flat_use_vertex_colors: true,
            ambient_intensity: default_ambient_intensity(),
            directional_intensity: default_directional_intensity(),
            material_roughness: default_material_roughness(),
            baked_ao_intensity: default_baked_ao_intensity(),
            wireframe_thickness_px: default_wireframe_thickness(),
            xray_opacity: default_xray_opacity(),
            heatmap_min_angle: 0.0,
            heatmap_max_angle: default_heatmap_max_angle(),
            heatmap_colors: default_heatmap_colors(),
            mesh_color: default_mesh_color(),
            hover_tint_strength: default_hover_tint_strength(),
            selected_tint_strength: default_selected_tint_strength(),
        }
    }
}

/// `MeshShaderType`'s stored values (`gpui-core/src/state.rs`,
/// `MESH_SHADER_TYPES`); the default is `'soft_clay'`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeshShaderType {
    #[default]
    SoftClay,
    FlatUnlit,
    Matcap,
    NormalDebug,
    Wireframe,
    OpaqueWireMesh,
    Xray,
    OverhangHeatmap,
}

/// `MatcapVariant`; the default is `'neutral'`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatcapVariant {
    #[default]
    Neutral,
    Cool,
    Warm,
}

// --- Performance ---------------------------------------------------------

/// Slicing/performance tab: `performancePreferences.ts`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceSettings {
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub png_compression_strategy: PngCompressionStrategy,
    #[serde(default)]
    pub aa_on_supports_experimental: bool,
}

impl Default for PerformanceSettings {
    fn default() -> Self {
        Self {
            png_compression_strategy: PngCompressionStrategy::Auto,
            aa_on_supports_experimental: false,
        }
    }
}

/// `PngCompressionStrategy`; the default is `'auto'`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PngCompressionStrategy {
    #[default]
    Auto,
    Fastest,
    Balanced,
    Smallest,
    Optimal,
}

// --- 3D mouse ------------------------------------------------------------

/// 3D Mouse tab: `spaceMousePreferences.ts`'s `DEFAULT_SPACEMOUSE_SETTINGS`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceMouseSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub pivot_mode: SpaceMousePivotMode,
    #[serde(default = "default_space_mouse_sensitivity")]
    pub translation_sensitivity: f32,
    #[serde(default = "default_space_mouse_sensitivity")]
    pub rotation_sensitivity: f32,
    #[serde(default = "default_space_mouse_sensitivity")]
    pub zoom_sensitivity: f32,
    #[serde(default = "default_space_mouse_deadzone")]
    pub deadzone: f32,
    #[serde(default)]
    pub dominant_axis: bool,
    #[serde(default)]
    pub invert_tx: bool,
    /// Defaults to `true` (Y pan is inverted out of the box).
    #[serde(default = "default_true")]
    pub invert_ty: bool,
    #[serde(default)]
    pub invert_tz: bool,
    #[serde(default)]
    pub invert_rx: bool,
    /// Defaults to `true` (yaw is inverted out of the box).
    #[serde(default = "default_true")]
    pub invert_ry: bool,
    #[serde(default)]
    pub invert_rz: bool,
    #[serde(default)]
    pub blocked_device_ids: Vec<String>,
}

fn default_space_mouse_sensitivity() -> f32 {
    2.3
}
fn default_space_mouse_deadzone() -> f32 {
    0.03
}

impl Default for SpaceMouseSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            pivot_mode: SpaceMousePivotMode::Auto,
            translation_sensitivity: default_space_mouse_sensitivity(),
            rotation_sensitivity: default_space_mouse_sensitivity(),
            zoom_sensitivity: default_space_mouse_sensitivity(),
            deadzone: default_space_mouse_deadzone(),
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

/// `SpaceMousePivotMode`; the stored value is `'camera-ray'` for the alternate
/// mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpaceMousePivotMode {
    #[default]
    Auto,
    #[serde(rename = "camera-ray")]
    CameraRay,
}

// --- Plugins -------------------------------------------------------------

/// Plugins tab. The only setting the backend persists today is the sideload
/// switch in `src-tauri/src/plugin_sideload.rs` (`Settings`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginsSettings {
    #[serde(default)]
    pub allow_sideloaded_complex_plugins: bool,
}

// --- Scene autosave ------------------------------------------------------

/// Scene Autosave tab: `DEFAULT_SCENE_AUTOSAVE_SETTINGS`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneAutosaveSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub recovery_prompt_enabled: bool,
    /// 45 s; the source clamps to `[15 s, 15 min]`.
    #[serde(default = "default_autosave_debounce_ms")]
    pub debounce_ms: u64,
    /// 3 min; the source clamps to `[15 s, 15 min]`.
    #[serde(default = "default_autosave_cooldown_ms")]
    pub cooldown_ms: u64,
    /// 5 min; the source clamps to `[1 min, 1 h]` and to at least the other
    /// two intervals.
    #[serde(default = "default_autosave_cap_ms")]
    pub cap_ms: u64,
}

fn default_autosave_debounce_ms() -> u64 {
    45_000
}
fn default_autosave_cooldown_ms() -> u64 {
    180_000
}
fn default_autosave_cap_ms() -> u64 {
    300_000
}

impl Default for SceneAutosaveSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            recovery_prompt_enabled: true,
            debounce_ms: default_autosave_debounce_ms(),
            cooldown_ms: default_autosave_cooldown_ms(),
            cap_ms: default_autosave_cap_ms(),
        }
    }
}

// --- Backups -------------------------------------------------------------

/// Backups tab: the three device-local keys in
/// `LocalBackupsSettingsTab.tsx` (`autoSyncEnabled`, `autoSyncMinutes`,
/// `directory`). The last-sync timestamp is runtime state, not a setting.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupsSettings {
    /// `AUTO_SYNC_ENABLED_KEY` defaults to on (`!== 'false'`).
    #[serde(default = "default_true")]
    pub auto_sync_enabled: bool,
    /// `AUTO_SYNC_MINUTES_KEY` defaults to 15, clamped to `[1, 240]`.
    #[serde(default = "default_auto_sync_minutes")]
    pub auto_sync_minutes: u32,
    /// `SELECTED_DIRECTORY_KEY`; empty when the user has not chosen one.
    #[serde(default)]
    pub directory: String,
}

fn default_auto_sync_minutes() -> u32 {
    15
}

impl Default for BackupsSettings {
    fn default() -> Self {
        Self {
            auto_sync_enabled: true,
            auto_sync_minutes: default_auto_sync_minutes(),
            directory: String::new(),
        }
    }
}

// --- UVTools -------------------------------------------------------------

/// UVTools tab: `DEFAULT_UVTOOLS_SETTINGS`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UvToolsSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub custom_path: String,
}

// --- UI ------------------------------------------------------------------

/// UI & Theme tab: `themeCustomizations.ts`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiSettings {
    /// `getSavedThemePreference()` falls back to `'dark'`.
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub theme: ThemePreference,
    /// `getSavedThemePreset()` falls back to `'dragonfruit-dark'`.
    #[serde(default = "default_theme_preset")]
    pub theme_preset: String,
    /// `DEFAULT_THEME_CUSTOM_COLORS`.
    #[serde(default)]
    pub theme_colors: ThemeColors,
}

fn default_theme_preset() -> String {
    "dragonfruit-dark".to_owned()
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            theme: ThemePreference::Dark,
            theme_preset: default_theme_preset(),
            theme_colors: ThemeColors::default(),
        }
    }
}

/// `ThemePreference` — `themeCustomizations.ts`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    #[default]
    Dark,
    Light,
}

/// `ThemeCustomColors` with `DEFAULT_THEME_CUSTOM_COLORS` as the defaults
/// (`DEFAULT_ACCENT` is `#ec2a77`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeColors {
    #[serde(default = "color_background")]
    pub background: String,
    #[serde(default = "color_foreground")]
    pub foreground: String,
    #[serde(default = "color_surface0")]
    pub surface0: String,
    #[serde(default = "color_surface1")]
    pub surface1: String,
    #[serde(default = "color_surface2")]
    pub surface2: String,
    #[serde(default = "color_text_strong")]
    pub text_strong: String,
    #[serde(default = "color_text_muted")]
    pub text_muted: String,
    #[serde(default = "color_indicator")]
    pub indicator: String,
    #[serde(default = "color_border_subtle")]
    pub border_subtle: String,
    #[serde(default = "color_border_strong")]
    pub border_strong: String,
    #[serde(default = "color_accent")]
    pub accent: String,
    #[serde(default = "color_accent_hover")]
    pub accent_hover: String,
    #[serde(default = "color_primary_button_surface")]
    pub primary_button_surface: String,
    #[serde(default = "color_accent_contrast")]
    pub accent_contrast: String,
    #[serde(default = "color_accent_secondary")]
    pub accent_secondary: String,
    #[serde(default = "color_accent_secondary_hover")]
    pub accent_secondary_hover: String,
    #[serde(default = "color_secondary_button_surface")]
    pub secondary_button_surface: String,
    #[serde(default = "color_accent_secondary_contrast")]
    pub accent_secondary_contrast: String,
    #[serde(default = "color_accent")]
    pub topbar_accent: String,
    #[serde(default = "color_scene_gradient_radial")]
    pub scene_gradient_radial: String,
    #[serde(default = "color_scene_gradient_radial")]
    pub scene_gradient_linear_start: String,
    #[serde(default = "color_scene_gradient_linear_mid")]
    pub scene_gradient_linear_mid: String,
    #[serde(default = "color_accent")]
    pub mesh_selection_color: String,
    #[serde(default = "color_accent")]
    pub mesh_hover_color: String,
    #[serde(default = "color_danger")]
    pub danger: String,
    #[serde(default = "color_success")]
    pub success: String,
}

fn color(hex: &str) -> String {
    hex.to_owned()
}
fn color_background() -> String {
    color("#0b0f14")
}
fn color_foreground() -> String {
    color("#e6ebf2")
}
fn color_surface0() -> String {
    color("#111216")
}
fn color_surface1() -> String {
    color("#1a1b21")
}
fn color_surface2() -> String {
    color("#23252e")
}
fn color_text_strong() -> String {
    color("#f8f8fb")
}
fn color_text_muted() -> String {
    color("#c3c7cf")
}
fn color_indicator() -> String {
    color("#c3c7cf")
}
fn color_border_subtle() -> String {
    color("#272a33")
}
fn color_border_strong() -> String {
    color("#353944")
}
fn color_accent() -> String {
    color("#ec2a77")
}
fn color_accent_hover() -> String {
    color("#d81d67")
}
fn color_primary_button_surface() -> String {
    color("#c11f61")
}
fn color_accent_contrast() -> String {
    color("#fff6ff")
}
fn color_accent_secondary() -> String {
    color("#baf72e")
}
fn color_accent_secondary_hover() -> String {
    color("#a6df29")
}
fn color_secondary_button_surface() -> String {
    color("#9bcc26")
}
fn color_accent_secondary_contrast() -> String {
    color("#182106")
}
fn color_scene_gradient_radial() -> String {
    color("#ff37aa")
}
fn color_scene_gradient_linear_mid() -> String {
    color("#6f33ff")
}
fn color_danger() -> String {
    color("#e45454")
}
fn color_success() -> String {
    color("#2eb67d")
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            background: color_background(),
            foreground: color_foreground(),
            surface0: color_surface0(),
            surface1: color_surface1(),
            surface2: color_surface2(),
            text_strong: color_text_strong(),
            text_muted: color_text_muted(),
            indicator: color_indicator(),
            border_subtle: color_border_subtle(),
            border_strong: color_border_strong(),
            accent: color_accent(),
            accent_hover: color_accent_hover(),
            primary_button_surface: color_primary_button_surface(),
            accent_contrast: color_accent_contrast(),
            accent_secondary: color_accent_secondary(),
            accent_secondary_hover: color_accent_secondary_hover(),
            secondary_button_surface: color_secondary_button_surface(),
            accent_secondary_contrast: color_accent_secondary_contrast(),
            topbar_accent: color_accent(),
            scene_gradient_radial: color_scene_gradient_radial(),
            scene_gradient_linear_start: color_scene_gradient_radial(),
            scene_gradient_linear_mid: color_scene_gradient_linear_mid(),
            mesh_selection_color: color_accent(),
            mesh_hover_color: color_accent(),
            danger: color_danger(),
            success: color_success(),
        }
    }
}

// --- Hotkeys -------------------------------------------------------------

/// Hotkeys tab: `HotkeyContext.tsx`'s `app-hotkeys-config`.
///
/// The source stores the category → action tree directly; this nests it under
/// `bindings` so the document keeps a version field beside it. Only bindings
/// the user changed are stored — the defaults live in code (`hotkeyConfig.ts`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeySettings {
    #[serde(default)]
    pub bindings: BTreeMap<String, BTreeMap<String, HotkeyBinding>>,
}

/// `HotkeyBinding`; `description` is the app's own wording for the action and
/// is never edited by the user.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyBinding {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub modifier: Option<String>,
    #[serde(default)]
    pub description: String,
}

// --- Logging -------------------------------------------------------------

/// Logging tab: `LoggingSettingsTab.tsx`'s `dragonfruit-logging:min-level`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingSettings {
    #[serde(default, deserialize_with = "de::enum_or_default")]
    pub min_level: LogLevel,
}

/// `LogLevelFilter`; `DEFAULT_LOG_LEVEL` is `'info'`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}
