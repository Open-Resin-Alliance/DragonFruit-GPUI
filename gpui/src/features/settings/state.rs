use super::*;

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
    pub(crate) fn restore_defaults(&mut self) {
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

pub(crate) static SETTINGS: LazyLock<Mutex<SettingsState>> =
    LazyLock::new(|| Mutex::new(SettingsState::default()));
pub(crate) fn state() -> &'static Mutex<SettingsState> {
    &SETTINGS
}

pub(crate) fn edit(f: impl FnOnce(&mut SettingsState)) {
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
