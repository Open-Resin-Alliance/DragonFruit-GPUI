use super::*;

use std::sync::atomic::{AtomicBool, Ordering};

use gpui_store::{
    CameraFeelPreset, CameraProjection, CameraScopeMode, CameraTrackpadModifierKey,
    CameraTrackpadPrimaryAction, HotkeyBinding as StoreHotkeyBinding, HotkeySettings as StoreHotkeys,
    LogLevel, MatcapVariant, MeshShaderType, PlateOrderingMode, PngCompressionStrategy,
    RaftBottomMode, Settings, SliceSatMode as StoreSliceSatMode,
    SliceSatRender as StoreSliceSatRender, SpaceMousePivotMode, ThemePreference,
    ToolLayout as StoreToolLayout, UpdateChannel as StoreUpdateChannel, View3dOriginMode,
    View3dSettings,
};

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

impl View3dState {
    fn from_store(store: &View3dSettings) -> Self {
        Self {
            enabled: store.enabled,
            width_mm: store.width_mm,
            depth_mm: store.depth_mm,
            max_z_mm: store.max_z_mm,
            origin_mode: match store.origin_mode {
                View3dOriginMode::Center => OriginMode::Center,
                View3dOriginMode::FrontLeft => OriginMode::FrontLeft,
            },
            screen_width_px: store.screen_width_px as f32,
            screen_height_px: store.screen_height_px as f32,
            show_violation_warning: store.show_violation_warning,
            show_model_bounding_boxes: store.show_model_bounding_boxes,
            show_slice_sat: store.show_slice_sat,
            sat_all_models: store.sat_all_models,
            sat_mode: match store.sat_mode {
                StoreSliceSatMode::AccurateHull => SliceSatMode::AccurateHull,
                StoreSliceSatMode::ExperimentalSlice => SliceSatMode::ExperimentalSlice,
            },
            sat_render: match store.sat_render {
                StoreSliceSatRender::Shaded => SliceSatRender::Shaded,
                StoreSliceSatRender::Wireframe => SliceSatRender::Wireframe,
            },
        }
    }

    fn write_store(&self, store: &mut View3dSettings) {
        store.enabled = self.enabled;
        store.width_mm = self.width_mm;
        store.depth_mm = self.depth_mm;
        store.max_z_mm = self.max_z_mm;
        store.origin_mode = match self.origin_mode {
            OriginMode::Center => View3dOriginMode::Center,
            OriginMode::FrontLeft => View3dOriginMode::FrontLeft,
        };
        store.screen_width_px = self.screen_width_px.round().max(0.0) as u32;
        store.screen_height_px = self.screen_height_px.round().max(0.0) as u32;
        store.show_violation_warning = self.show_violation_warning;
        store.show_model_bounding_boxes = self.show_model_bounding_boxes;
        store.show_slice_sat = self.show_slice_sat;
        store.sat_all_models = self.sat_all_models;
        store.sat_mode = match self.sat_mode {
            SliceSatMode::AccurateHull => StoreSliceSatMode::AccurateHull,
            SliceSatMode::ExperimentalSlice => StoreSliceSatMode::ExperimentalSlice,
        };
        store.sat_render = match self.sat_render {
            SliceSatRender::Shaded => StoreSliceSatRender::Shaded,
            SliceSatRender::Wireframe => StoreSliceSatRender::Wireframe,
        };
    }
}

impl Default for View3dState {
    fn default() -> Self {
        Self::from_store(&View3dSettings::default())
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

impl MultiPlateState {
    fn from_store(store: &gpui_store::MultiPlateSettings) -> Self {
        Self {
            follow_landed_plate: store.follow_landed_plate,
            plate_ordering: match store.plate_ordering {
                PlateOrderingMode::Fixed => PlateOrdering::Fixed,
                PlateOrderingMode::Dynamic => PlateOrdering::Dynamic,
            },
            fixed_plate_columns: store.fixed_plate_columns as f32,
        }
    }

    fn write_store(&self, store: &mut gpui_store::MultiPlateSettings) {
        store.follow_landed_plate = self.follow_landed_plate;
        store.plate_ordering = match self.plate_ordering {
            PlateOrdering::Fixed => PlateOrderingMode::Fixed,
            PlateOrdering::Dynamic => PlateOrderingMode::Dynamic,
        };
        store.fixed_plate_columns = self.fixed_plate_columns.round().max(0.0) as u32;
    }
}

impl Default for MultiPlateState {
    fn default() -> Self {
        Self::from_store(&gpui_store::MultiPlateSettings::default())
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

impl SpaceMouseState {
    fn from_store(store: &gpui_store::SpaceMouseSettings) -> Self {
        Self {
            enabled: store.enabled,
            pivot_mode: match store.pivot_mode {
                SpaceMousePivotMode::Auto => PivotMode::Auto,
                SpaceMousePivotMode::CameraRay => PivotMode::CameraRay,
            },
            translation_sensitivity: store.translation_sensitivity,
            rotation_sensitivity: store.rotation_sensitivity,
            zoom_sensitivity: store.zoom_sensitivity,
            deadzone: store.deadzone,
            dominant_axis: store.dominant_axis,
            invert_tx: store.invert_tx,
            invert_ty: store.invert_ty,
            invert_tz: store.invert_tz,
            invert_rx: store.invert_rx,
            invert_ry: store.invert_ry,
            invert_rz: store.invert_rz,
            blocked_device_ids: store.blocked_device_ids.clone(),
        }
    }

    fn write_store(&self, store: &mut gpui_store::SpaceMouseSettings) {
        store.enabled = self.enabled;
        store.pivot_mode = match self.pivot_mode {
            PivotMode::Auto => SpaceMousePivotMode::Auto,
            PivotMode::CameraRay => SpaceMousePivotMode::CameraRay,
        };
        store.translation_sensitivity = self.translation_sensitivity;
        store.rotation_sensitivity = self.rotation_sensitivity;
        store.zoom_sensitivity = self.zoom_sensitivity;
        store.deadzone = self.deadzone;
        store.dominant_axis = self.dominant_axis;
        store.invert_tx = self.invert_tx;
        store.invert_ty = self.invert_ty;
        store.invert_tz = self.invert_tz;
        store.invert_rx = self.invert_rx;
        store.invert_ry = self.invert_ry;
        store.invert_rz = self.invert_rz;
        store.blocked_device_ids = self.blocked_device_ids.clone();
    }
}

impl Default for SpaceMouseState {
    fn default() -> Self {
        Self::from_store(&gpui_store::SpaceMouseSettings::default())
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

impl HotkeyState {
    fn from_store(store: &StoreHotkeys) -> Self {
        let mut overrides = HashMap::new();
        for (category, action, key, modifier, _) in HOTKEY_DEFAULTS.iter() {
            let Some(binding) = store
                .bindings
                .get(*category)
                .and_then(|group| group.get(*action))
            else {
                continue;
            };
            let recorded = HotkeyBinding {
                key: if binding.key.is_empty() {
                    (*key).to_owned()
                } else {
                    binding.key.clone()
                },
                modifier: binding.modifier.clone(),
            };
            if recorded.key != *key || recorded.modifier.as_deref() != *modifier {
                overrides.insert((*category, *action), recorded);
            }
        }
        Self {
            recording: None,
            overrides,
        }
    }

    fn write_store(&self, store: &mut StoreHotkeys) {
        // Only bindings the user changed are stored, exactly as at the source,
        // so the defaults stay in `hotkeyConfig.ts` and every run inherits a
        // fixed default. Unknown categories and actions from a newer document
        // are left alone.
        for (category, action, _, _, _) in HOTKEY_DEFAULTS.iter() {
            if let Some(group) = store.bindings.get_mut(*category) {
                group.remove(*action);
            }
        }
        store.bindings.retain(|_, group| !group.is_empty());
        for (category, action, key, modifier, description) in HOTKEY_DEFAULTS.iter() {
            let Some(binding) = self.overrides.get(&(*category, *action)) else {
                continue;
            };
            if binding.key == *key && binding.modifier.as_deref() == *modifier {
                continue;
            }
            store.bindings.entry((*category).to_owned()).or_default().insert(
                (*action).to_owned(),
                StoreHotkeyBinding {
                    key: binding.key.clone(),
                    modifier: binding.modifier.clone(),
                    description: (*description).to_owned(),
                },
            );
        }
    }
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
    pub(crate) fn binding(&self, category: &'static str, action: &'static str) -> HotkeyBinding {
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

    pub(crate) fn reset_category(&mut self, category: &'static str) {
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

impl PluginsState {
    fn from_store(store: &gpui_store::PluginsSettings) -> Self {
        Self {
            repo_url: String::new(),
            allowance_dialog_open: false,
            allow_sideload: store.allow_sideloaded_complex_plugins,
            details_row: None,
            capabilities_expanded: false,
            copied_signing_key: false,
            reload_required: false,
            reload_seconds_left: 15,
            status: None,
        }
    }

    fn write_store(&self, store: &mut gpui_store::PluginsSettings) {
        store.allow_sideloaded_complex_plugins = self.allow_sideload;
    }
}

impl Default for PluginsState {
    fn default() -> Self {
        Self::from_store(&gpui_store::PluginsSettings::default())
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

pub(crate) const BUILTIN_PLUGINS: [BuiltinPlugin; 1] = [BuiltinPlugin {
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

impl ExperimentsState {
    fn from_store(store: &gpui_store::ExperimentsSettings) -> Self {
        let mut enabled = HashMap::new();
        for (id, _, _, default_enabled) in EXPERIMENTS.iter() {
            enabled.insert(
                *id,
                store.enabled.get(*id).copied().unwrap_or(*default_enabled),
            );
        }
        Self {
            disclaimer_seen: false,
            show_disclaimer: true,
            enabled,
        }
    }

    fn write_store(&self, store: &mut gpui_store::ExperimentsSettings) {
        // A missing id means "use the registry default", so only the
        // experiments the user moved away from their default are written.
        store.enabled.clear();
        for (id, _, _, default_enabled) in EXPERIMENTS.iter() {
            let value = self.enabled.get(*id).copied().unwrap_or(*default_enabled);
            if value != *default_enabled {
                store.enabled.insert((*id).to_owned(), value);
            }
        }
    }
}

impl Default for ExperimentsState {
    fn default() -> Self {
        Self::from_store(&gpui_store::ExperimentsSettings::default())
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

impl AutosaveState {
    fn from_store(store: &gpui_store::SceneAutosaveSettings) -> Self {
        Self {
            enabled: store.enabled,
            recovery_prompt_enabled: store.recovery_prompt_enabled,
            debounce_ms: store.debounce_ms as f32,
            cooldown_ms: store.cooldown_ms as f32,
            cap_ms: store.cap_ms as f32,
        }
    }

    fn write_store(&self, store: &mut gpui_store::SceneAutosaveSettings) {
        store.enabled = self.enabled;
        store.recovery_prompt_enabled = self.recovery_prompt_enabled;
        store.debounce_ms = self.debounce_ms.round().max(0.0) as u64;
        store.cooldown_ms = self.cooldown_ms.round().max(0.0) as u64;
        store.cap_ms = self.cap_ms.round().max(0.0) as u64;
    }
}

impl Default for AutosaveState {
    fn default() -> Self {
        Self::from_store(&gpui_store::SceneAutosaveSettings::default())
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

impl BackupsState {
    fn from_store(store: &gpui_store::BackupsSettings) -> Self {
        Self {
            selected_directory: store.directory.clone(),
            auto_sync_enabled: store.auto_sync_enabled,
            auto_sync_minutes: store.auto_sync_minutes as f32,
            confirm_delete_id: None,
            confirm_restore_id: None,
            message: None,
        }
    }

    fn write_store(&self, store: &mut gpui_store::BackupsSettings) {
        store.directory = self.selected_directory.clone();
        store.auto_sync_enabled = self.auto_sync_enabled;
        store.auto_sync_minutes = self.auto_sync_minutes.round().max(0.0) as u32;
    }
}

impl Default for BackupsState {
    fn default() -> Self {
        Self::from_store(&gpui_store::BackupsSettings::default())
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

impl UvToolsState {
    fn from_store(store: &gpui_store::UvToolsSettings) -> Self {
        Self {
            enabled: store.enabled,
            custom_path: store.custom_path.clone(),
            discovery_busy: false,
            show_found_glow: false,
            show_not_found: false,
        }
    }

    fn write_store(&self, store: &mut gpui_store::UvToolsSettings) {
        store.enabled = self.enabled;
        store.custom_path = self.custom_path.clone();
    }
}

impl Default for UvToolsState {
    fn default() -> Self {
        Self::from_store(&gpui_store::UvToolsSettings::default())
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

impl LoggingState {
    fn from_store(store: &gpui_store::LoggingSettings) -> Self {
        // `LOG_LEVEL_OPTIONS` lists error, warn, info, debug, trace.
        let level = match store.min_level {
            LogLevel::Error => 0,
            LogLevel::Warn => 1,
            LogLevel::Info => 2,
            LogLevel::Debug => 3,
            LogLevel::Trace => 4,
        };
        Self {
            level: LOG_LEVEL_OPTIONS[level].0,
            viewer_filter: "trace",
            hide_noise: true,
            is_live: true,
            filter_open: false,
            copied: false,
        }
    }

    fn write_store(&self, store: &mut gpui_store::LoggingSettings) {
        store.min_level = match self.level {
            "error" => LogLevel::Error,
            "warn" => LogLevel::Warn,
            "debug" => LogLevel::Debug,
            "trace" => LogLevel::Trace,
            _ => LogLevel::Info,
        };
    }
}

impl Default for LoggingState {
    fn default() -> Self {
        Self::from_store(&gpui_store::LoggingSettings::default())
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

impl UpdatesState {
    fn from_store(store: &gpui_store::UpdateSettings) -> Self {
        let channel = match store.channel {
            StoreUpdateChannel::Stable => UpdateChannel::Stable,
            StoreUpdateChannel::Dev => UpdateChannel::Dev,
        };
        Self {
            channel,
            // The dialog's draft starts on the saved channel.
            pending_channel: channel,
            show_channel_settings: false,
        }
    }

    fn write_store(&self, store: &mut gpui_store::UpdateSettings) {
        store.channel = match self.channel {
            UpdateChannel::Stable => StoreUpdateChannel::Stable,
            UpdateChannel::Dev => StoreUpdateChannel::Dev,
        };
    }
}

impl Default for UpdatesState {
    fn default() -> Self {
        Self::from_store(&gpui_store::UpdateSettings::default())
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
///
/// Every persisted value is seeded from [`gpui_store::Settings`] — the store is
/// the only place a default lives — and [`snapshot`] writes the draft back into
/// a copy of the document it was hydrated from, so values this draft cannot
/// express survive a save. The remaining fields are view state (open menus,
/// the selected sub-tab, confirmations, transient status).
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
    /// The shader type the store can hold but this tab's dropdown cannot show
    /// (`flat_unlit`, `matcap`). Kept so a save does not rewrite the user's
    /// viewport shader behind their back; a dropdown pick clears it.
    pub unlisted_shader: Option<MeshShaderType>,
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
    /// `previousTabRef` in the shell: the tab that was active before the
    /// current one, so the Experiments disclaimer's "Take me back!" can return
    /// there (`SettingsModal.tsx:418`, `:419-424`).
    pub prev_tab: usize,
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

// ── Store ↔ draft conversions ────────────────────────────────────────────
//
// The store owns the values and their defaults; these convert between its
// enums/units and the indexes the tab controls use. Every mapping is total, so
// a document written by a newer build always lands on a renderable control.

fn projection_from_store(projection: CameraProjection) -> Projection {
    match projection {
        CameraProjection::Orthographic => Projection::Ortho,
        CameraProjection::Perspective => Projection::Perspective,
    }
}

fn projection_to_store(projection: Projection) -> CameraProjection {
    match projection {
        Projection::Ortho => CameraProjection::Orthographic,
        Projection::Perspective => CameraProjection::Perspective,
    }
}

/// `FEEL_VALUES` order: raw, precise, balanced, fast.
fn camera_feel_index(preset: CameraFeelPreset) -> usize {
    match preset {
        CameraFeelPreset::Raw => 0,
        CameraFeelPreset::Precise => 1,
        CameraFeelPreset::Balanced => 2,
        CameraFeelPreset::Fast => 3,
    }
}

fn camera_feel_preset(index: usize) -> CameraFeelPreset {
    match index {
        0 => CameraFeelPreset::Raw,
        2 => CameraFeelPreset::Balanced,
        3 => CameraFeelPreset::Fast,
        _ => CameraFeelPreset::Precise,
    }
}

/// `TRACKPAD_VALUES` order: off, pan, orbit.
fn trackpad_action_index(action: CameraTrackpadPrimaryAction) -> usize {
    match action {
        CameraTrackpadPrimaryAction::Off => 0,
        CameraTrackpadPrimaryAction::Pan => 1,
        CameraTrackpadPrimaryAction::Orbit => 2,
    }
}

fn trackpad_action_value(index: usize) -> CameraTrackpadPrimaryAction {
    match index {
        1 => CameraTrackpadPrimaryAction::Pan,
        2 => CameraTrackpadPrimaryAction::Orbit,
        _ => CameraTrackpadPrimaryAction::Off,
    }
}

/// The tab's `OPTION` / `SHIFT` segments.
fn trackpad_modifier_index(key: CameraTrackpadModifierKey) -> usize {
    match key {
        CameraTrackpadModifierKey::Alt => 0,
        CameraTrackpadModifierKey::Shift => 1,
    }
}

fn trackpad_modifier_value(index: usize) -> CameraTrackpadModifierKey {
    match index {
        0 => CameraTrackpadModifierKey::Alt,
        _ => CameraTrackpadModifierKey::Shift,
    }
}

/// `SHADER_VALUES` order: soft_clay, overhang_heatmap, opaque_wire_mesh,
/// wireframe, xray, normal_debug. `flat_unlit` and `matcap` are render types the
/// tab does not offer, so they have no index.
fn shader_index(shader: MeshShaderType) -> Option<usize> {
    Some(match shader {
        MeshShaderType::SoftClay => 0,
        MeshShaderType::OverhangHeatmap => 1,
        MeshShaderType::OpaqueWireMesh => 2,
        MeshShaderType::Wireframe => 3,
        MeshShaderType::Xray => 4,
        MeshShaderType::NormalDebug => 5,
        MeshShaderType::FlatUnlit | MeshShaderType::Matcap => return None,
    })
}

fn shader_type(index: usize) -> MeshShaderType {
    match index {
        1 => MeshShaderType::OverhangHeatmap,
        2 => MeshShaderType::OpaqueWireMesh,
        3 => MeshShaderType::Wireframe,
        4 => MeshShaderType::Xray,
        5 => MeshShaderType::NormalDebug,
        _ => MeshShaderType::SoftClay,
    }
}

/// `MATCAP_LABELS` order: neutral, cool, warm.
fn matcap_index(variant: MatcapVariant) -> usize {
    match variant {
        MatcapVariant::Neutral => 0,
        MatcapVariant::Cool => 1,
        MatcapVariant::Warm => 2,
    }
}

fn matcap_variant(index: usize) -> MatcapVariant {
    match index {
        1 => MatcapVariant::Cool,
        2 => MatcapVariant::Warm,
        _ => MatcapVariant::Neutral,
    }
}

/// The `AUTO` / `OFF` / `ON` segments: the source shows `'fastest'` as Off and
/// every other compressing strategy as On.
fn png_mode_index(strategy: PngCompressionStrategy) -> usize {
    match strategy {
        PngCompressionStrategy::Auto => 0,
        PngCompressionStrategy::Fastest => 1,
        PngCompressionStrategy::Balanced
        | PngCompressionStrategy::Smallest
        | PngCompressionStrategy::Optimal => 2,
    }
}

fn png_mode_strategy(index: usize) -> PngCompressionStrategy {
    match index {
        1 => PngCompressionStrategy::Fastest,
        2 => PngCompressionStrategy::Balanced,
        _ => PngCompressionStrategy::Auto,
    }
}

/// One hydrated theme colour, keyed the way the UI & Theme tables key their
/// rows. The map is filled from the store by [`hydrate`], so this never needs a
/// second copy of a default.
pub(crate) fn theme_color(state: &SettingsState, key: &str) -> String {
    state.theme_colors.get(key).cloned().unwrap_or_default()
}

impl SettingsState {
    /// Builds the draft from the document the app loaded. Every persisted field
    /// comes from `settings` — the store is the only place its default lives —
    /// so the tabs read the hydrated value and never a literal.
    fn from_settings(settings: &Settings) -> Self {
        // The store carries every colour these tables name; a key the store
        // lacks renders empty rather than falling back to a second copy of the
        // default.
        let mut theme_colors = HashMap::new();
        for section in THEME_COLOR_SECTIONS.iter() {
            for row in section.rows {
                let value = settings.ui.theme_colors.get(row.key).unwrap_or_default();
                theme_colors.insert(row.key, value.to_owned());
            }
        }
        // The Cut tool keeps its own colours; no store group carries them, and
        // this tab only displays them.
        let mut cut_colors = HashMap::new();
        for field in CUT_COLOR_FIELDS.iter() {
            cut_colors.insert(field.key, field.placeholder.to_owned());
        }

        let general = &settings.general;
        let import = &general.import_defaults;
        let camera = &settings.camera;
        let workspace_defaults = &settings.workspaces.defaults;
        let mesh = &settings.mesh;
        let thumbnail = &settings.performance.thumbnail;
        let shader = shader_index(mesh.shader_type);

        Self {
            language: LANGUAGE_VALUES
                .iter()
                .position(|code| *code == general.language.as_str())
                .unwrap_or(0),
            ui_scale: general.ui_scale,
            custom_scale: !UI_SCALE_PRESETS
                .iter()
                .any(|preset| (*preset - general.ui_scale).abs() < 0.001),
            tool_layout: match general.tool_layout {
                StoreToolLayout::Horizontal => ToolLayout::Top,
                StoreToolLayout::Vertical => ToolLayout::Left,
            },
            floating_persistence: general.floating_layout_persistence,
            support_help: general.support_placement_help,
            raft_bottom: match import.raft_bottom_mode {
                RaftBottomMode::Off => RaftBottom::Off,
                RaftBottomMode::Line => RaftBottom::Line,
                RaftBottomMode::Solid => RaftBottom::Solid,
            },
            raft_wall: import.raft_wall_enabled,
            roots_enabled: import.roots_enabled,
            auto_repair: import.auto_repair,
            auto_repair_scenes: import.auto_repair_scenes,
            camera_scope: match settings.workspaces.scope {
                CameraScopeMode::Global => CameraScope::Global,
                CameraScopeMode::Workspace => CameraScope::Workspace,
            },
            projection: projection_from_store(camera.projection),
            fov: camera.fov,
            camera_feel: camera_feel_index(camera.feel.preset),
            trackpad_action: trackpad_action_index(camera.trackpad.primary_action),
            trackpad_modifier: trackpad_modifier_index(camera.trackpad.modifier_key),
            pan_acceleration: camera.trackpad.pan_acceleration,
            orbit_acceleration: camera.trackpad.orbit_acceleration,
            zoom_acceleration: camera.trackpad.zoom_acceleration,
            workspace_tab: 0,
            workspace_defaults: [
                projection_from_store(workspace_defaults.prepare),
                projection_from_store(workspace_defaults.analysis),
                projection_from_store(workspace_defaults.support),
                projection_from_store(workspace_defaults.export),
            ],
            shader: shader.unwrap_or(0),
            unlisted_shader: shader.is_none().then_some(mesh.shader_type),
            matcap: matcap_index(mesh.matcap_variant),
            // The source's own local `previewModel` state, default `'knot'`.
            preview_model: 2,
            flat_use_vertex_colors: mesh.flat_use_vertex_colors,
            mesh_color: mesh.mesh_color.clone(),
            ambient_intensity: mesh.ambient_intensity,
            directional_intensity: mesh.directional_intensity,
            material_roughness: mesh.material_roughness,
            baked_ao_intensity: mesh.baked_ao_intensity,
            xray_opacity: mesh.xray_opacity,
            heatmap_min_angle: mesh.heatmap_min_angle,
            heatmap_max_angle: mesh.heatmap_max_angle,
            heatmap_colors: mesh.heatmap_colors.clone(),
            active_color_index: 0,
            hover_tint_strength: mesh.hover_tint_strength,
            selected_tint_strength: mesh.selected_tint_strength,
            preview_hovered: false,
            preview_selected: false,
            thumbnail_gradient: thumbnail.include_gradient,
            thumbnail_build_plate: thumbnail.include_build_plate,
            thumbnail_grid: thumbnail.include_grid,
            png_compression: png_mode_index(settings.performance.png_compression_strategy),
            theme_preset: THEME_PRESET_VALUES
                .iter()
                .position(|preset| *preset == settings.ui.theme_preset.as_str())
                .unwrap_or(0),
            theme_preference: match settings.ui.theme {
                ThemePreference::Dark => 0,
                ThemePreference::Light => 1,
            },
            theme_tab: 0,
            theme_colors,
            cut_colors,
            open_select: None,
            prev_tab: 0,
            view3d: View3dState::from_store(&settings.view3d),
            multi_plate: MultiPlateState::from_store(&settings.multi_plate),
            spacemouse: SpaceMouseState::from_store(&settings.space_mouse),
            hotkeys: HotkeyState::from_store(&settings.hotkeys),
            plugins: PluginsState::from_store(&settings.plugins),
            experiments: ExperimentsState::from_store(&settings.experiments),
            autosave: AutosaveState::from_store(&settings.scene_autosave),
            backups: BackupsState::from_store(&settings.backups),
            uvtools: UvToolsState::from_store(&settings.uvtools),
            logging: LoggingState::from_store(&settings.logging),
            updates: UpdatesState::from_store(&settings.updates),
            about: AboutState::default(),
            show_restore_confirm: false,
        }
    }

    /// The draft as a document: `base` (the document this draft was hydrated
    /// from) with every field the draft exposes written over it. Fields the
    /// draft does not expose (`printing` builds a camera default, a newer
    /// document's extra keys, hotkeys for actions this build does not list)
    /// keep their loaded value.
    fn to_settings(&self, base: &Settings) -> Settings {
        let mut out = base.clone();

        out.general.ui_scale = self.ui_scale;
        out.general.language = LANGUAGE_VALUES
            [self.language.min(LANGUAGE_VALUES.len() - 1)]
        .to_owned();
        out.general.floating_layout_persistence = self.floating_persistence;
        out.general.tool_layout = match self.tool_layout {
            ToolLayout::Top => StoreToolLayout::Horizontal,
            ToolLayout::Left => StoreToolLayout::Vertical,
        };
        out.general.support_placement_help = self.support_help;
        out.general.import_defaults.raft_bottom_mode = match self.raft_bottom {
            RaftBottom::Off => RaftBottomMode::Off,
            RaftBottom::Line => RaftBottomMode::Line,
            RaftBottom::Solid => RaftBottomMode::Solid,
        };
        out.general.import_defaults.raft_wall_enabled = self.raft_wall;
        out.general.import_defaults.roots_enabled = self.roots_enabled;
        out.general.import_defaults.auto_repair = self.auto_repair;
        out.general.import_defaults.auto_repair_scenes = self.auto_repair_scenes;

        out.camera.projection = projection_to_store(self.projection);
        out.camera.fov = self.fov;
        out.camera.feel.preset = camera_feel_preset(self.camera_feel);
        out.camera.trackpad.primary_action = trackpad_action_value(self.trackpad_action);
        out.camera.trackpad.modifier_key = trackpad_modifier_value(self.trackpad_modifier);
        out.camera.trackpad.pan_acceleration = self.pan_acceleration;
        out.camera.trackpad.orbit_acceleration = self.orbit_acceleration;
        out.camera.trackpad.zoom_acceleration = self.zoom_acceleration;

        out.workspaces.scope = match self.camera_scope {
            CameraScope::Global => CameraScopeMode::Global,
            CameraScope::Workspace => CameraScopeMode::Workspace,
        };
        out.workspaces.defaults.prepare = projection_to_store(self.workspace_defaults[0]);
        out.workspaces.defaults.analysis = projection_to_store(self.workspace_defaults[1]);
        out.workspaces.defaults.support = projection_to_store(self.workspace_defaults[2]);
        out.workspaces.defaults.export = projection_to_store(self.workspace_defaults[3]);

        self.view3d.write_store(&mut out.view3d);

        out.mesh.shader_type = match self.unlisted_shader {
            Some(shader) => shader,
            None => shader_type(self.shader),
        };
        out.mesh.matcap_variant = matcap_variant(self.matcap);
        out.mesh.flat_use_vertex_colors = self.flat_use_vertex_colors;
        out.mesh.mesh_color = self.mesh_color.clone();
        out.mesh.ambient_intensity = self.ambient_intensity;
        out.mesh.directional_intensity = self.directional_intensity;
        out.mesh.material_roughness = self.material_roughness;
        out.mesh.baked_ao_intensity = self.baked_ao_intensity;
        out.mesh.xray_opacity = self.xray_opacity;
        out.mesh.heatmap_min_angle = self.heatmap_min_angle;
        out.mesh.heatmap_max_angle = self.heatmap_max_angle;
        out.mesh.heatmap_colors = self.heatmap_colors.clone();
        out.mesh.hover_tint_strength = self.hover_tint_strength;
        out.mesh.selected_tint_strength = self.selected_tint_strength;

        // `Smallest` / `Optimal` have no segment of their own: only rewrite
        // the strategy when the user actually moved the control, so a loaded
        // value the tab cannot show survives a save.
        if self.png_compression != png_mode_index(out.performance.png_compression_strategy) {
            out.performance.png_compression_strategy = png_mode_strategy(self.png_compression);
        }
        out.performance.thumbnail.include_gradient = self.thumbnail_gradient;
        out.performance.thumbnail.include_build_plate = self.thumbnail_build_plate;
        out.performance.thumbnail.include_grid = self.thumbnail_grid;

        self.multi_plate.write_store(&mut out.multi_plate);
        self.spacemouse.write_store(&mut out.space_mouse);
        self.plugins.write_store(&mut out.plugins);
        self.experiments.write_store(&mut out.experiments);
        self.autosave.write_store(&mut out.scene_autosave);
        self.backups.write_store(&mut out.backups);
        self.uvtools.write_store(&mut out.uvtools);

        out.ui.theme = match self.theme_preference {
            0 => ThemePreference::Dark,
            _ => ThemePreference::Light,
        };
        out.ui.theme_preset =
            THEME_PRESET_VALUES[self.theme_preset.min(THEME_PRESET_VALUES.len() - 1)].to_owned();
        for section in THEME_COLOR_SECTIONS.iter() {
            for row in section.rows {
                if let Some(value) = self.theme_colors.get(row.key) {
                    out.ui.theme_colors.set(row.key, value.clone());
                }
            }
        }

        self.hotkeys.write_store(&mut out.hotkeys);
        self.logging.write_store(&mut out.logging);
        self.updates.write_store(&mut out.updates);

        out
    }

    /// `applyRestoreDefaultsToDraft` — only the fields that callback resets.
    /// Language, UI scale, theme profiles, hotkeys, plugins, experiments,
    /// autosave, backups, logging and the update channel are left alone,
    /// exactly as at the source.
    pub(crate) fn restore_defaults(&mut self) {
        let defaults = SettingsState::from_settings(&Settings::default());
        self.mesh_color = defaults.mesh_color;
        self.shader = defaults.shader;
        self.unlisted_shader = defaults.unlisted_shader;
        self.matcap = defaults.matcap;
        self.flat_use_vertex_colors = defaults.flat_use_vertex_colors;
        self.ambient_intensity = defaults.ambient_intensity;
        self.directional_intensity = defaults.directional_intensity;
        self.material_roughness = defaults.material_roughness;
        self.baked_ao_intensity = defaults.baked_ao_intensity;
        self.xray_opacity = defaults.xray_opacity;
        self.heatmap_min_angle = defaults.heatmap_min_angle;
        self.heatmap_max_angle = defaults.heatmap_max_angle;
        self.hover_tint_strength = defaults.hover_tint_strength;
        self.selected_tint_strength = defaults.selected_tint_strength;
        self.projection = defaults.projection;
        self.camera_feel = defaults.camera_feel;
        self.trackpad_action = defaults.trackpad_action;
        self.trackpad_modifier = defaults.trackpad_modifier;
        self.pan_acceleration = defaults.pan_acceleration;
        self.orbit_acceleration = defaults.orbit_acceleration;
        self.zoom_acceleration = defaults.zoom_acceleration;
        self.camera_scope = defaults.camera_scope;
        self.fov = defaults.fov;
        self.theme_preference = defaults.theme_preference;
        self.theme_preset = defaults.theme_preset;
        self.theme_colors = defaults.theme_colors;
        self.floating_persistence = defaults.floating_persistence;
        self.raft_bottom = defaults.raft_bottom;
        self.raft_wall = defaults.raft_wall;
        self.roots_enabled = defaults.roots_enabled;
        self.auto_repair = defaults.auto_repair;
        self.auto_repair_scenes = defaults.auto_repair_scenes;
        self.spacemouse = defaults.spacemouse;
        self.workspace_defaults = defaults.workspace_defaults;
        self.view3d = defaults.view3d;
        self.png_compression = defaults.png_compression;
        self.thumbnail_gradient = defaults.thumbnail_gradient;
        self.thumbnail_build_plate = defaults.thumbnail_build_plate;
        self.thumbnail_grid = defaults.thumbnail_grid;
        self.uvtools = defaults.uvtools;
    }
}

impl Default for SettingsState {
    fn default() -> Self {
        Self::from_settings(&Settings::default())
    }
}

pub(crate) static SETTINGS: LazyLock<Mutex<SettingsState>> =
    LazyLock::new(|| Mutex::new(SettingsState::default()));

/// The document the draft was hydrated from. [`snapshot`] writes over a copy of
/// it, so anything the draft cannot express survives a save. Kept out of
/// [`SettingsState`] so the modal's per-frame clone stays small.
static BASE: LazyLock<Mutex<Settings>> = LazyLock::new(|| Mutex::new(Settings::default()));

/// True between [`edit`] and the next [`hydrate`] / [`take_dirty`].
static DIRTY: AtomicBool = AtomicBool::new(false);

pub(crate) fn state() -> &'static Mutex<SettingsState> {
    &SETTINGS
}

/// Loads a document into the modal's draft: the app calls this once at startup
/// with `Store::load_settings()`'s value, and again after any external change.
/// Every tab then renders the hydrated value; no control falls back to its own
/// default.
pub fn hydrate(settings: Settings) {
    let next = SettingsState::from_settings(&settings);
    match BASE.lock() {
        Ok(mut base) => *base = settings,
        Err(poisoned) => *poisoned.into_inner() = settings,
    }
    match state().lock() {
        Ok(mut guard) => *guard = next,
        Err(poisoned) => *poisoned.into_inner() = next,
    }
    DIRTY.store(false, Ordering::Relaxed);
}

/// The draft as a document, ready for `Store::save_settings_debounced`.
///
/// Built from a copy of the hydrated document, so a value this draft cannot
/// express — a shader type the Mesh tab does not list, a PNG strategy with no
/// segment — survives unless the user moved that control.
pub fn snapshot() -> Settings {
    let base = match BASE.lock() {
        Ok(guard) => guard.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    };
    match state().lock() {
        Ok(guard) => guard.to_settings(&base),
        Err(poisoned) => poisoned.into_inner().to_settings(&base),
    }
}

/// Records that the draft changed. [`edit`] calls this; a caller that mutates
/// the draft through [`state`] directly must call it too.
pub fn mark_dirty() {
    DIRTY.store(true, Ordering::Relaxed);
}

/// Whether the draft changed since it was hydrated or last taken.
pub fn is_dirty() -> bool {
    DIRTY.load(Ordering::Relaxed)
}

/// Reads and clears the dirty flag: `true` means the app should save
/// [`snapshot`].
pub fn take_dirty() -> bool {
    DIRTY.swap(false, Ordering::Relaxed)
}

pub(crate) fn edit(f: impl FnOnce(&mut SettingsState)) {
    match state().lock() {
        Ok(mut guard) => f(&mut guard),
        Err(poisoned) => f(&mut poisoned.into_inner()),
    }
    mark_dirty();
}

/// Mutates view state (the previous tab, an open menu, a confirmation) without
/// marking the draft dirty: nothing here reaches [`snapshot`], so a save would
/// rewrite the document unchanged.
pub(crate) fn edit_view(f: impl FnOnce(&mut SettingsState)) {
    match state().lock() {
        Ok(mut guard) => f(&mut guard),
        Err(poisoned) => f(&mut poisoned.into_inner()),
    }
}

pub(crate) fn edit_and_notify(
    entity: &WeakEntity<DragonFruit>,
    cx: &mut App,
    f: impl FnOnce(&mut SettingsState),
) {
    edit(f);
    entity.update(cx, |_, cx| cx.notify()).ok();
}
