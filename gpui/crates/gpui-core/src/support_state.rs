//! State for the support panels (islands, island list, support settings,
//! hollowing, hole punching), owned by `DragonFruit`.

use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

use gpui::SharedString;

fn clamp_float(value: f32, min: f32, max: f32, decimals: i32) -> f32 {
    let safe = if value.is_finite() { value } else { min };
    let factor = 10f32.powi(decimals);
    let rounded = (safe * factor).round() / factor;
    rounded.clamp(min, max)
}


/// `useIslands`' draft settings, the fields the Scan Settings dialog edits.
#[derive(Clone, Copy)]
pub struct IslandScanDraft {
    pub px_mm: f32,
    pub support_buf_mm: f32,
    pub consolidate_voxel: bool,
    pub consolidation_distance: f32,
    pub reduce_intersection: bool,
    pub intersection_threshold: f32,
    pub scale_markers_with_area: bool,
    pub enable_contour_regions: bool,
    pub max_contour_regions: f32,
    pub remove_supported_area_clusters: bool,
    pub area_per_support: f32,
}

impl Default for IslandScanDraft {
    fn default() -> Self {
        Self {
            px_mm: 0.05,
            support_buf_mm: 0.25,
            consolidate_voxel: false,
            consolidation_distance: 0.2,
            reduce_intersection: false,
            intersection_threshold: 0.5,
            scale_markers_with_area: true,
            enable_contour_regions: true,
            max_contour_regions: 20.0,
            remove_supported_area_clusters: false,
            area_per_support: 4.0,
        }
    }
}

/// The islands panel's state: `UseIslandsReturn`'s scan state, the counts, the
/// overlay switches and the draft/applied scan settings.
#[derive(Default)]
pub struct IslandsModel {
    pub has_geometry: bool,
    pub scanning: bool,
    pub applying_settings: bool,
    /// `tableStats.voxelTotal` / `.geomTotal` / `.coincidentTotal` / `.allTotal`.
    pub voxel_total: usize,
    pub minima_total: usize,
    pub coincident_total: usize,
    pub all_total: usize,
    /// `orderedIslands`, as the labels the counter prints (`id.replace(/^\D+/, '')`).
    pub islands: Vec<SharedString>,
    pub selected_index: Option<usize>,
    pub advanced_mode: bool,
    pub show_voxel_only: bool,
    pub show_minima_only: bool,
    pub show_intersection: bool,
    pub show_overhangs: bool,
    /// `filterToggles`.
    pub show_already_supported: bool,
    pub show_plate_contact: bool,
    pub settings_open: bool,
    pub draft: IslandScanDraft,
    pub applied: IslandScanDraft,
}

impl IslandsModel {
    pub fn has_data(&self) -> bool {
        self.all_total > 0
    }
}

/// A cheap, cloneable handle over the islands panel's state.
#[derive(Clone, Default)]
pub struct IslandsPanelState {
    model: Rc<RefCell<IslandsModel>>,
}

impl IslandsPanelState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn model(&self) -> Ref<'_, IslandsModel> {
        self.model.borrow()
    }

    pub fn model_mut(&self) -> RefMut<'_, IslandsModel> {
        self.model.borrow_mut()
    }
}

/// The Islands body: the scan button, the type breakdown, the prev/next
/// navigator, the advanced layer toggles and the empty state.
///
/// PORT GAP: no island scanner — the Scan button starts nothing, and the totals
/// and labels are whatever the shell writes into [`IslandsModel`].


/// One row of `IslandListCard`'s list.
#[derive(Clone)]
pub struct IslandRow {
    pub id: u32,
    /// `status === 'active'`.
    pub active: bool,
    pub parent_id: Option<u32>,
    pub child_ids: Vec<u32>,
    pub first_layer: u32,
    pub last_layer: u32,
    pub volume_mm3: Option<f32>,
    pub total_area_mm2: f32,
    pub max_area_mm2: Option<f32>,
    pub max_area_layer: Option<f32>,
}

/// `IslandListCard`'s sort order.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IslandSort {
    Id,
    Volume,
    Layers,
}

#[derive(Default)]
pub struct IslandListModel {
    pub islands: Vec<IslandRow>,
    pub selected_id: Option<u32>,
    pub show_merged: bool,
    /// The search box's term; there is no text entry, so the shell seeds it.
    pub search: String,
    pub sort_by: Option<IslandSort>,
    pub show_hierarchy: bool,
    pub layer_height_mm: f32,
    pub z_offset_mm: f32,
}

impl IslandListModel {
    pub fn sort_by(&self) -> IslandSort {
        self.sort_by.unwrap_or(IslandSort::Id)
    }

    pub fn active_count(&self) -> usize {
        self.islands.iter().filter(|island| island.active).count()
    }

    /// The filtered, sorted rows the list renders.
    pub fn rows(&self) -> Vec<IslandRow> {
        let mut rows: Vec<IslandRow> = self
            .islands
            .iter()
            .filter(|island| self.show_merged || island.active)
            .filter(|island| {
                self.search.is_empty() || island.id.to_string().contains(&self.search)
            })
            .cloned()
            .collect();
        match self.sort_by() {
            IslandSort::Id => rows.sort_by_key(|island| island.id),
            IslandSort::Volume => rows.sort_by(|a, b| {
                let volume = |island: &IslandRow| island.volume_mm3.unwrap_or(island.total_area_mm2);
                volume(b)
                    .partial_cmp(&volume(a))
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            IslandSort::Layers => rows.sort_by_key(|island| {
                std::cmp::Reverse(island.last_layer.saturating_sub(island.first_layer))
            }),
        }
        rows
    }
}

/// A cheap, cloneable handle over the island list's state.
#[derive(Clone, Default)]
pub struct IslandListState {
    model: Rc<RefCell<IslandListModel>>,
}

impl IslandListState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn model(&self) -> Ref<'_, IslandListModel> {
        self.model.borrow()
    }

    pub fn model_mut(&self) -> RefMut<'_, IslandListModel> {
        self.model.borrow_mut()
    }
}

/// The Island IDs body: the totals, the show-merged switch, the search and sort
/// row, the island rows and the clear-selection action.
///


/// The dialog's policy knobs, at `AUTO_SUPPORT_CONSTRAINTS`' defaults.
#[derive(Clone, Copy)]
pub struct SupportSettingsValues {
    pub area_per_support_mm2: f32,
    pub size_scale: f32,
    pub grid_area_threshold_mm2: f32,
    pub flat_density_boost: f32,
    pub slope_relax_factor: f32,
    pub suction_area_exponent: f32,
    pub self_support_angle_deg: f32,
    pub stabilization_enabled: bool,
    pub minima_reinforcement_enabled: bool,
}

impl Default for SupportSettingsValues {
    fn default() -> Self {
        Self {
            area_per_support_mm2: 10.0,
            size_scale: 1.0,
            grid_area_threshold_mm2: 25.0,
            flat_density_boost: 0.7,
            slope_relax_factor: 1.3,
            suction_area_exponent: 0.15,
            self_support_angle_deg: 45.0,
            stabilization_enabled: true,
            minima_reinforcement_enabled: true,
        }
    }
}

/// The three built-in auto-support presets, by id.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AutoSupportPreset {
    Light,
    Medium,
    Heavy,
}

impl AutoSupportPreset {
    pub fn id(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Medium => "medium",
            Self::Heavy => "heavy",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Medium => "Medium",
            Self::Heavy => "Heavy",
        }
    }

    /// The Support Studio band the preset sizes with (`sizingPreset`).
    pub fn tier(self) -> &'static str {
        match self {
            Self::Light => "Detail",
            Self::Medium => "Structure",
            Self::Heavy => "Anchor",
        }
    }

    /// The preset's block: the defaults with its own `areaPerSupportMm2`.
    pub fn values(self) -> SupportSettingsValues {
        let mut values = SupportSettingsValues::default();
        match self {
            Self::Light => values.area_per_support_mm2 = 16.0,
            Self::Medium => {}
            Self::Heavy => values.area_per_support_mm2 = 5.0,
        }
        values
    }
}

/// `NO_ACTIVE_PRESET_LABEL`.
pub const NO_ACTIVE_PRESET_LABEL: &str = "Custom — not a preset";

/// `LOCKED_PROFILE_HINT`: why a built-in profile's fields are fixed.
const LOCKED_PROFILE_HINT: &str = "This is a built-in profile, so its fields are fixed. Duplicate it and edit the copy — custom settings belong in a custom profile.";

/// A field's help text: the lock's when the active preset is a built-in.
pub fn field_help_text(locked: bool, hint: &'static str) -> &'static str {
    if locked {
        LOCKED_PROFILE_HINT
    } else {
        hint
    }
}

pub struct SupportSettingsModel {
    pub draft: SupportSettingsValues,
    pub baseline: SupportSettingsValues,
    pub preset: Option<AutoSupportPreset>,
    pub modified: bool,
    pub preset_menu_open: bool,
}

impl Default for SupportSettingsModel {
    fn default() -> Self {
        Self {
            draft: SupportSettingsValues::default(),
            baseline: SupportSettingsValues::default(),
            preset: None,
            modified: false,
            preset_menu_open: false,
        }
    }
}

/// A cheap, cloneable handle over the support settings' state.
#[derive(Clone, Default)]
pub struct SupportSettingsState {
    model: Rc<RefCell<SupportSettingsModel>>,
}

impl SupportSettingsState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn model(&self) -> Ref<'_, SupportSettingsModel> {
        self.model.borrow()
    }

    pub fn model_mut(&self) -> RefMut<'_, SupportSettingsModel> {
        self.model.borrow_mut()
    }
}

/// The Auto Support settings body: the preset strip, the Density & Sizing and
/// Stability cards, and the dialog's Delete / Reset / Save footer.
///
/// PORT GAP: the remaining sections (Detection, Distribution, Post-processing,
/// Diagnostics, Advanced) are not rendered, and the preset strip's Rename /


/// `HollowMode` from `meshHollowing.ts`, as the panel offers it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HollowMode {
    Cavity,
    Infill,
    ShellOpenFace,
}

/// `InfillMode`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InfillMode {
    Lattice,
    Pillar,
}

/// `OpenFace`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OpenFace {
    XMin,
    XMax,
    YMin,
    YMax,
    ZMin,
    ZMax,
}

impl OpenFace {
    pub fn value(self) -> &'static str {
        match self {
            Self::XMin => "x_min",
            Self::XMax => "x_max",
            Self::YMin => "y_min",
            Self::YMax => "y_max",
            Self::ZMin => "z_min",
            Self::ZMax => "z_max",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::XMin => "X Min",
            Self::XMax => "X Max",
            Self::YMin => "Y Min",
            Self::YMax => "Y Max",
            Self::ZMin => "Z Min",
            Self::ZMax => "Z Max",
        }
    }
}

/// `HollowingPanelState` plus the panel's own action flags.
pub struct HollowingModel {
    pub mode: HollowMode,
    pub voxel_size_mm: f32,
    pub shell_thickness_mm: f32,
    pub infill_mode: InfillMode,
    pub infill_cell_mm: f32,
    pub infill_beam_radius_mm: f32,
    pub open_face: OpenFace,
    pub open_face_menu_open: bool,
    pub is_applying: bool,
    pub is_previewing: bool,
    pub is_applying_blockers: bool,
    pub can_apply: bool,
    pub can_edit: bool,
    pub is_edit_mode: bool,
    pub is_hollowing_applied: bool,
    pub shell_face_selection_pending: bool,
}

impl Default for HollowingModel {
    fn default() -> Self {
        Self {
            mode: HollowMode::Cavity,
            voxel_size_mm: 0.65,
            shell_thickness_mm: 2.0,
            infill_mode: InfillMode::Lattice,
            infill_cell_mm: 4.2426,
            infill_beam_radius_mm: 0.35,
            open_face: OpenFace::ZMax,
            open_face_menu_open: false,
            is_applying: false,
            is_previewing: false,
            is_applying_blockers: false,
            can_apply: true,
            can_edit: true,
            is_edit_mode: false,
            is_hollowing_applied: false,
            shell_face_selection_pending: false,
        }
    }
}

/// `minInfillCellMm` / `maxInfillCellMm`.
const MIN_INFILL_CELL_MM: f32 = 3.0;
const MAX_INFILL_CELL_MM: f32 = 24.0;
/// `minInfillDiameterMm` / `maxInfillDiameterMm`.
pub const MIN_INFILL_DIAMETER_MM: f32 = 0.5;
pub const MAX_INFILL_DIAMETER_MM: f32 = 6.0;

impl HollowingModel {
    /// `handleResetHollowingSettings`: the panel's fields back to their
    /// defaults, leaving an applied hollow in place.
    pub fn reset_settings(&mut self) {
        let defaults = Self::default();
        self.mode = defaults.mode;
        self.voxel_size_mm = defaults.voxel_size_mm;
        self.shell_thickness_mm = defaults.shell_thickness_mm;
        self.infill_mode = defaults.infill_mode;
        self.infill_cell_mm = defaults.infill_cell_mm;
        self.infill_beam_radius_mm = defaults.infill_beam_radius_mm;
        self.open_face = defaults.open_face;
    }

    /// `infillDensityPct`: `3²·100 / cell²`, clamped to 1–100 and rounded.
    pub fn infill_density_pct(&self) -> f32 {
        let cell = self.infill_cell_mm.clamp(MIN_INFILL_CELL_MM, MAX_INFILL_CELL_MM);
        clamp_float(
            MIN_INFILL_CELL_MM * MIN_INFILL_CELL_MM * 100.0 / (cell * cell),
            1.0,
            100.0,
            0,
        )
    }

    /// `infillDiameterMm`: `radius·2`, clamped to 0.5–6.
    pub fn infill_diameter_mm(&self) -> f32 {
        clamp_float(
            self.infill_beam_radius_mm * 2.0,
            MIN_INFILL_DIAMETER_MM,
            MAX_INFILL_DIAMETER_MM,
            2,
        )
    }

    /// `densityPctToCellMm`: `3 / √(pct/100)`, clamped to 3–24.
    pub fn density_pct_to_cell_mm(pct: f32) -> f32 {
        let density = clamp_float(pct, 1.0, 100.0, 0);
        clamp_float(
            MIN_INFILL_CELL_MM / (density / 100.0).sqrt(),
            MIN_INFILL_CELL_MM,
            MAX_INFILL_CELL_MM,
            1,
        )
    }
}

/// A cheap, cloneable handle over the hollowing panel's state.
#[derive(Clone, Default)]
pub struct HollowingState {
    model: Rc<RefCell<HollowingModel>>,
}

impl HollowingState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn model(&self) -> Ref<'_, HollowingModel> {
        self.model.borrow()
    }

    pub fn model_mut(&self) -> RefMut<'_, HollowingModel> {
        self.model.borrow_mut()
    }
}

/// The Hollowing body: the mode badge, the Cavity/Infill choice, the voxel and
/// shell fields, the infill fields and the action row.
///
/// PORT GAP: no hollowing engine — Apply, Remove Hollowing, Solid Areas and the


/// `HolePunchPanelState.depthMode`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HolePunchDepthMode {
    Manual,
    Auto,
}

/// `HolePunchPanelState` plus the panel's own action flags.
pub struct HolePunchModel {
    pub radius_mm: f32,
    pub radius_y_mm: Option<f32>,
    pub depth_mm: f32,
    pub depth_mode: HolePunchDepthMode,
    pub linked: bool,
    pub can_use_auto_depth: bool,
    pub is_applying: bool,
    pub can_apply: bool,
    pub can_reset: bool,
    pub disabled: bool,
    pub interior_view: bool,
    pub interior_view_available: bool,
}

impl Default for HolePunchModel {
    fn default() -> Self {
        Self {
            radius_mm: 2.0,
            radius_y_mm: None,
            // `getDefaultHolePunchDepthMm(2.0)` = 2 + 1.
            depth_mm: 3.0,
            depth_mode: HolePunchDepthMode::Manual,
            linked: true,
            can_use_auto_depth: true,
            is_applying: false,
            can_apply: false,
            can_reset: true,
            disabled: false,
            interior_view: false,
            interior_view_available: false,
        }
    }
}

/// A cheap, cloneable handle over the hole-punch panel's state.
#[derive(Clone, Default)]
pub struct HolePunchState {
    model: Rc<RefCell<HolePunchModel>>,
}

impl HolePunchState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn model(&self) -> Ref<'_, HolePunchModel> {
        self.model.borrow()
    }

    pub fn model_mut(&self) -> RefMut<'_, HolePunchModel> {
        self.model.borrow_mut()
    }
}

