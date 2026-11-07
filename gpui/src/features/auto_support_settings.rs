//! Auto Supports settings dialog — `AutoSupportSettingsBody.tsx` and the
//! components it composes (`AutoSupportPresets.tsx`,
//! `AutoSupportSizingTierField.tsx`, `autoSupportPanelTabs.ts`).
//!
//! The dialog is the 1120px full-screen surface the Auto Supports panel's gear
//! opens: a header, one scrolling body of field cards (the preset strip, the five
//! policy cards, Diagnostics and the warning-toned calibration card) and the
//! pinned Delete / Reset / Save footer.
//!
//! Every size, label and colour traces to a class string or inline style in the
//! React source; Tailwind radii are the v4 scale (`rounded` 4px, `rounded-lg`
//! 8px, `rounded-xl` 12px).
//!
//! # PORT DEVIATIONS
//!
//! 1. The draft, the live block and the preset collection live in one
//!    process-wide [`DialogState`] (`LazyLock<Mutex<…>>`), the way `settings.rs`
//!    keeps its draft: `DragonFruit` owns only the open flag. Selecting a preset
//!    applies it to `live` and writes the self-support angle to
//!    `DragonFruit::support_angle`, the value `stl::overhang_points` classifies
//!    with; the other knobs have no consumer in this slice.
//! 2. No text entry: the name dialog's field renders its value without a caret,
//!    so Create / Rename use the name the dialog opened with (`New Preset`, or
//!    the selected preset's name). The source's typing path is absent.
//! 3. `Export` / `Import` are inert: the source's preset file format needs the
//!    Tauri bridge (`savePrintArtifactWithNativeDialog`) and a JSON reader this
//!    slice does not have. Their buttons render the source's enabled recipe, and
//!    the strip's import-error banner is unreachable, so it is not rendered.
//! 4. `lg:grid-cols-2` is always two columns: the dialog's 1120px cap is wider
//!    than Tailwind's 1024px breakpoint, so the source never shows one column
//!    either.
//! 5. `grid-cols-2` inside a card is a pair of flex rows; a lone cell is padded
//!    with an empty half so it keeps the grid's width.
//! 6. No CSS transitions, `backdrop-blur-sm`, `tracking-wide` (the headers are
//!    uppercased in the string instead), `custom-scrollbar` or portals: the
//!    nested dialogs paint after the card, inside the same backdrop.
//! 7. `title` attributes are the app's own `ui::tooltip` here (GPUI carries no
//!    `title`/`aria-*`).
//! 8. `!h-8` on the preset trigger is not applied — the trigger keeps
//!    `ui::select_dropdown`'s 36px `.ui-input`; the strip and footer buttons do
//!    carry their own heights.
//! 9. The Support Studio preset collection is the three factory tiers
//!    (`detail` / `structure` / `anchor`): the app has no Support Studio store,
//!    so a custom tier cannot appear in the Sizing Tier select.
//! 10. `Debug Mode` and `Simplified` are the dialog's own flags; the panel's
//!     debug report (`AutoSupportRunDiagnostics`) is not ported.

// The shell mounts this dialog once it reads `show_support_settings`; until then
// every item but `auto_support_settings` is unreachable from `main`.
#![allow(dead_code)]

use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use gpui::{
    AnyElement, App, ClickEvent, Div, ElementId, FontWeight, IntoElement, MouseButton, SharedString,
    Stateful, WeakEntity, Window, div, linear_color_stop, linear_gradient, prelude::*, px, relative,
    rgb, rgba,
};

use gpui_core::icons::icon;
use gpui_ui::panel_fields::{disabled_action_style, format_number, with_alpha};
use gpui_core::theme::*;
use gpui_ui::ui::{
    ButtonSize, ButtonSpec, ButtonVariant, IconButtonSize, IconButtonVariant, SelectOption,
    SelectOptionTone, ToggleSize, button_with, icon_button, input_box, labeled_number_field,
    select_dropdown, toggle_sized, tooltip,
};
use crate::DragonFruit;

// ── Constants ───────────────────────────────────────────────────────────────

/// `#ffffff` / `#000000` — the bases of every `color-mix(..., white|black N%)`.
const WHITE: u32 = 0xffffff;
const BLACK: u32 = 0x000000;

/// Tailwind's bare `rounded` (4px) and `rounded-lg` (8px).
const ROUNDED_BARE: f32 = 4.0;
const ROUNDED_LG: f32 = 8.0;
/// `rounded-xl` (12px).
const ROUNDED_XL: f32 = 12.0;

/// `#d97706` — the calibration card's warning tone.
const WARNING: u32 = 0xd97706;

/// `NO_ACTIVE_PRESET_LABEL`.
const NO_ACTIVE_PRESET_LABEL: &str = "Custom — not a preset";
/// `NEW_PRESET_NAME`.
const NEW_PRESET_NAME: &str = "New Preset";
/// `NEW_PRESET_ENTRY`.
const NEW_PRESET_ENTRY: &str = "New preset…";
/// `DEFAULT_AUTO_SUPPORT_PRESET_NAME`.
const DEFAULT_PRESET_NAME: &str = "Auto-Support Preset";

/// `LOCKED_PROFILE_HINT`.
const LOCKED_PROFILE_HINT: &str = "This is a built-in profile, so its fields are fixed. Duplicate it and edit the copy — custom settings belong in a custom profile.";

/// `DEBUG_DIAGNOSTICS_HEADING`.
const DEBUG_DIAGNOSTICS_HEADING: &str = "Diagnostics";

/// `AUTO_SUPPORT_ADVANCED_SECTION`.
const ADVANCED_LABEL: &str = "Advanced (calibration)";
const ADVANCED_HINT: &str = "The sizing constants the engine ships with. They are stored per preset.";

/// `SAVE_STYLE`: `--accent-secondary-action-*`.
fn save_border() -> u32 {
    mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.58)
}
fn save_background() -> u32 {
    mix(ACCENT_SECONDARY, SURFACE_1, 0.08)
}
/// `SAVED_STYLE`: the success treatment.
fn saved_border() -> u32 {
    mix(SUCCESS, BORDER_SUBTLE, 0.60)
}
fn saved_background() -> u32 {
    mix(SUCCESS, SURFACE_1, 0.15)
}
/// A CSS `box-shadow` layer: `offset-x offset-y blur spread rgba(...)`.
/// (`ui.rs` keeps the same helper private; the dialog's `shadow-2xl` needs it.)
fn shadow(x: f32, y: f32, blur: f32, spread: f32, color: u32) -> gpui::BoxShadow {
    gpui::BoxShadow::new(px(x), px(y), rgba(color).into())
        .blur_radius(px(blur))
        .spread_radius(px(spread))
}

/// The `.ui-button:disabled` foreground (`button:disabled` in `globals.css`),
/// for the icons `ButtonSpec` cannot inherit the disabled colour into.
fn disabled_foreground() -> u32 {
    disabled_action_style().2
}

// ── The settings block ──────────────────────────────────────────────────────

/// `AutoSupportSettings` — the whole `autoSupport` block, as the dialog edits it.
#[derive(Clone, Copy, PartialEq)]
pub struct AutoSupportSettings {
    pub enabled: bool,
    pub min_island_area_mm2: f32,
    pub tip_influence_radius_mm: f32,
    pub prioritize_intersection: bool,
    pub max_attachments_per_trunk: f32,
    pub area_per_support_mm2: f32,
    pub grid_area_threshold_mm2: f32,
    pub overhang_self_support_angle_deg: f32,
    pub size_scale: f32,
    pub flat_density_boost: f32,
    pub slope_relax_factor: f32,
    pub suction_area_exponent: f32,
    pub coverage_target_percent: f32,
    pub sizing_preset: SizingTier,
    pub leaf_fan_radius_mm: f32,
    pub leaf_fan_max_angle_deg: f32,
    pub tip_contact_margin_scale: f32,
    pub member_host_shaft_ratio: f32,
    pub model_scale_enabled: bool,
    pub model_size_factor_cap: f32,
    pub model_load_factor_cap: f32,
    pub height_factor_cap: f32,
    pub debug_support_origin_colors: bool,
    pub debug_skip_auto_bracing: bool,
    pub stabilization_enabled: bool,
    pub minima_reinforcement_enabled: bool,
}

impl Default for AutoSupportSettings {
    /// `createDefaultAutoSupportSettings()`.
    fn default() -> Self {
        Self {
            enabled: true,
            min_island_area_mm2: 0.02,
            tip_influence_radius_mm: 0.5,
            prioritize_intersection: false,
            max_attachments_per_trunk: 12.0,
            area_per_support_mm2: 10.0,
            grid_area_threshold_mm2: 25.0,
            overhang_self_support_angle_deg: 45.0,
            size_scale: 1.0,
            flat_density_boost: 0.7,
            slope_relax_factor: 1.3,
            suction_area_exponent: 0.15,
            coverage_target_percent: 95.0,
            sizing_preset: SizingTier::Structure,
            leaf_fan_radius_mm: 8.0,
            leaf_fan_max_angle_deg: 45.0,
            tip_contact_margin_scale: 0.9,
            member_host_shaft_ratio: 0.7,
            model_scale_enabled: true,
            model_size_factor_cap: 1.45,
            model_load_factor_cap: 1.3,
            height_factor_cap: 1.35,
            debug_support_origin_colors: false,
            debug_skip_auto_bracing: false,
            stabilization_enabled: true,
            minima_reinforcement_enabled: true,
        }
    }
}

impl AutoSupportSettings {
    fn knob(&self, key: KnobKey) -> f32 {
        match key {
            KnobKey::MinIslandAreaMm2 => self.min_island_area_mm2,
            KnobKey::TipInfluenceRadiusMm => self.tip_influence_radius_mm,
            KnobKey::LeafFanRadiusMm => self.leaf_fan_radius_mm,
            KnobKey::LeafFanMaxAngleDeg => self.leaf_fan_max_angle_deg,
            KnobKey::AreaPerSupportMm2 => self.area_per_support_mm2,
            KnobKey::SizeScale => self.size_scale,
            KnobKey::GridAreaThresholdMm2 => self.grid_area_threshold_mm2,
            KnobKey::FlatDensityBoost => self.flat_density_boost,
            KnobKey::SlopeRelaxFactor => self.slope_relax_factor,
            KnobKey::SuctionAreaExponent => self.suction_area_exponent,
            KnobKey::OverhangSelfSupportAngleDeg => self.overhang_self_support_angle_deg,
            KnobKey::MaxAttachmentsPerTrunk => self.max_attachments_per_trunk,
            KnobKey::CoverageTargetPercent => self.coverage_target_percent,
            KnobKey::TipContactMarginScale => self.tip_contact_margin_scale,
            KnobKey::MemberHostShaftRatio => self.member_host_shaft_ratio,
            KnobKey::ModelSizeFactorCap => self.model_size_factor_cap,
            KnobKey::ModelLoadFactorCap => self.model_load_factor_cap,
            KnobKey::HeightFactorCap => self.height_factor_cap,
        }
    }

    fn set_knob(&mut self, key: KnobKey, value: f32) {
        match key {
            KnobKey::MinIslandAreaMm2 => self.min_island_area_mm2 = value,
            KnobKey::TipInfluenceRadiusMm => self.tip_influence_radius_mm = value,
            KnobKey::LeafFanRadiusMm => self.leaf_fan_radius_mm = value,
            KnobKey::LeafFanMaxAngleDeg => self.leaf_fan_max_angle_deg = value,
            KnobKey::AreaPerSupportMm2 => self.area_per_support_mm2 = value,
            KnobKey::SizeScale => self.size_scale = value,
            KnobKey::GridAreaThresholdMm2 => self.grid_area_threshold_mm2 = value,
            KnobKey::FlatDensityBoost => self.flat_density_boost = value,
            KnobKey::SlopeRelaxFactor => self.slope_relax_factor = value,
            KnobKey::SuctionAreaExponent => self.suction_area_exponent = value,
            KnobKey::OverhangSelfSupportAngleDeg => {
                self.overhang_self_support_angle_deg = value
            }
            KnobKey::MaxAttachmentsPerTrunk => self.max_attachments_per_trunk = value,
            KnobKey::CoverageTargetPercent => self.coverage_target_percent = value,
            KnobKey::TipContactMarginScale => self.tip_contact_margin_scale = value,
            KnobKey::MemberHostShaftRatio => self.member_host_shaft_ratio = value,
            KnobKey::ModelSizeFactorCap => self.model_size_factor_cap = value,
            KnobKey::ModelLoadFactorCap => self.model_load_factor_cap = value,
            KnobKey::HeightFactorCap => self.height_factor_cap = value,
        }
    }

    fn toggle(&self, key: ToggleKey) -> bool {
        match key {
            ToggleKey::Enabled => self.enabled,
            ToggleKey::PrioritizeIntersection => self.prioritize_intersection,
            ToggleKey::StabilizationEnabled => self.stabilization_enabled,
            ToggleKey::MinimaReinforcementEnabled => self.minima_reinforcement_enabled,
            ToggleKey::DebugSupportOriginColors => self.debug_support_origin_colors,
            ToggleKey::DebugSkipAutoBracing => self.debug_skip_auto_bracing,
            ToggleKey::ModelScaleEnabled => self.model_scale_enabled,
        }
    }

    fn set_toggle(&mut self, key: ToggleKey, value: bool) {
        match key {
            ToggleKey::Enabled => self.enabled = value,
            ToggleKey::PrioritizeIntersection => self.prioritize_intersection = value,
            ToggleKey::StabilizationEnabled => self.stabilization_enabled = value,
            ToggleKey::MinimaReinforcementEnabled => self.minima_reinforcement_enabled = value,
            ToggleKey::DebugSupportOriginColors => self.debug_support_origin_colors = value,
            ToggleKey::DebugSkipAutoBracing => self.debug_skip_auto_bracing = value,
            ToggleKey::ModelScaleEnabled => self.model_scale_enabled = value,
        }
    }
}

/// `autoSupportPolicyDiffers`: every key except the two diagnostics, which are
/// view switches rather than the run's instructions.
fn policy_differs(a: &AutoSupportSettings, b: &AutoSupportSettings) -> bool {
    let mut left = *a;
    let mut right = *b;
    left.debug_support_origin_colors = false;
    right.debug_support_origin_colors = false;
    left.debug_skip_auto_bracing = false;
    right.debug_skip_auto_bracing = false;
    left != right
}

// ── Keys ────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum KnobKey {
    MinIslandAreaMm2,
    TipInfluenceRadiusMm,
    LeafFanRadiusMm,
    LeafFanMaxAngleDeg,
    AreaPerSupportMm2,
    SizeScale,
    GridAreaThresholdMm2,
    FlatDensityBoost,
    SlopeRelaxFactor,
    SuctionAreaExponent,
    OverhangSelfSupportAngleDeg,
    MaxAttachmentsPerTrunk,
    CoverageTargetPercent,
    TipContactMarginScale,
    MemberHostShaftRatio,
    ModelSizeFactorCap,
    ModelLoadFactorCap,
    HeightFactorCap,
}

impl KnobKey {
    fn slug(self) -> &'static str {
        match self {
            Self::MinIslandAreaMm2 => "min-island-area",
            Self::TipInfluenceRadiusMm => "tip-influence-radius",
            Self::LeafFanRadiusMm => "leaf-fan-radius",
            Self::LeafFanMaxAngleDeg => "leaf-fan-max-angle",
            Self::AreaPerSupportMm2 => "area-per-support",
            Self::SizeScale => "size-scale",
            Self::GridAreaThresholdMm2 => "grid-area-threshold",
            Self::FlatDensityBoost => "flat-density-boost",
            Self::SlopeRelaxFactor => "slope-relax-factor",
            Self::SuctionAreaExponent => "suction-area-exponent",
            Self::OverhangSelfSupportAngleDeg => "self-support-angle",
            Self::MaxAttachmentsPerTrunk => "max-attachments-per-trunk",
            Self::CoverageTargetPercent => "coverage-target",
            Self::TipContactMarginScale => "tip-contact-margin",
            Self::MemberHostShaftRatio => "member-host-shaft-ratio",
            Self::ModelSizeFactorCap => "model-size-factor-cap",
            Self::ModelLoadFactorCap => "model-load-factor-cap",
            Self::HeightFactorCap => "height-factor-cap",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ToggleKey {
    Enabled,
    PrioritizeIntersection,
    StabilizationEnabled,
    MinimaReinforcementEnabled,
    DebugSupportOriginColors,
    DebugSkipAutoBracing,
    ModelScaleEnabled,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SectionKey {
    Detection,
    Distribution,
    Density,
    Stability,
    PostProcessing,
    Debug,
}

impl SectionKey {
    fn slug(self) -> &'static str {
        match self {
            Self::Detection => "detection",
            Self::Distribution => "distribution",
            Self::Density => "density",
            Self::Stability => "stability",
            Self::PostProcessing => "post-processing",
            Self::Debug => "debug",
        }
    }
}

// ── Catalogue — `autoSupportPanelTabs.ts` ───────────────────────────────────

struct SectionDef {
    key: SectionKey,
    label: &'static str,
    subtitle: Option<&'static str>,
    hint: &'static str,
}

/// `AUTO_SUPPORT_POLICY_SECTIONS`.
static POLICY_SECTIONS: [SectionDef; 5] = [
    SectionDef {
        key: SectionKey::Detection,
        label: "Detection",
        subtitle: Some("What needs support, and how close is too close"),
        hint: "What counts as a support-needing surface, and how tightly neighbouring detections merge",
    },
    SectionDef {
        key: SectionKey::Distribution,
        label: "Distribution",
        subtitle: Some("Where contacts land, and how far members fan"),
        hint: "How a region's contacts scatter across it, and how far leaves fan out from a trunk",
    },
    SectionDef {
        key: SectionKey::Density,
        label: "Density & Sizing",
        subtitle: Some("How many supports, and how thick"),
        hint: "How dense and how thick the supports are",
    },
    SectionDef {
        key: SectionKey::Stability,
        label: "Stability",
        subtitle: Some("Held against toppling and peel"),
        hint: "How the part is held against toppling and peel — self-support angle, stabilization anchors and minima reinforcement",
    },
    SectionDef {
        key: SectionKey::PostProcessing,
        label: "Post-processing",
        subtitle: Some("The passes after placement"),
        hint: "Passes that run after placement — how much one trunk may carry, and the coverage the gap-filler must reach",
    },
];

struct KnobDef {
    key: KnobKey,
    section: SectionKey,
    label: &'static str,
    min: f32,
    max: f32,
    step: f32,
    unit: Option<&'static str>,
    hint: &'static str,
}

/// `KNOBS`, in table order.
static KNOBS: [KnobDef; 13] = [
    KnobDef {
        key: KnobKey::MinIslandAreaMm2,
        section: SectionKey::Detection,
        label: "Min Island Size",
        min: 0.01,
        max: 2.0,
        step: 0.01,
        unit: Some("mm²"),
        hint: "Skip detected areas smaller than this — tiny specks rarely need supports",
    },
    KnobDef {
        key: KnobKey::TipInfluenceRadiusMm,
        section: SectionKey::Detection,
        label: "Merge Radius",
        min: 0.1,
        max: 10.0,
        step: 0.1,
        unit: Some("mm"),
        hint: "A candidate within this 3D distance of an existing support merges into it instead of starting a new trunk",
    },
    KnobDef {
        key: KnobKey::LeafFanRadiusMm,
        section: SectionKey::Distribution,
        label: "Fan Reach",
        min: 2.0,
        max: 15.0,
        step: 0.5,
        unit: Some("mm"),
        hint: "Max horizontal distance a fan-out leaf may span from a trunk shaft",
    },
    KnobDef {
        key: KnobKey::LeafFanMaxAngleDeg,
        section: SectionKey::Distribution,
        label: "Fan Angle",
        min: 20.0,
        max: 80.0,
        step: 5.0,
        unit: Some("°"),
        hint: "Max angle from vertical for fan-out leaves",
    },
    KnobDef {
        key: KnobKey::AreaPerSupportMm2,
        section: SectionKey::Density,
        label: "Support Density",
        min: 1.0,
        max: 30.0,
        step: 0.5,
        unit: Some("mm²"),
        hint: "Projected area each support carries — smaller = more, tighter supports (grid spacing ≈ √value)",
    },
    KnobDef {
        key: KnobKey::SizeScale,
        section: SectionKey::Density,
        label: "Support Size",
        min: 0.5,
        max: 2.0,
        step: 0.05,
        unit: Some("×"),
        hint: "Master multiplier over the preset sizing bands — thicker or thinner everywhere",
    },
    KnobDef {
        key: KnobKey::GridAreaThresholdMm2,
        section: SectionKey::Density,
        label: "Grid Threshold",
        min: 5.0,
        max: 200.0,
        step: 5.0,
        unit: Some("mm²"),
        hint: "Flat regions at/above this area get a full grid; smaller regions get a single support",
    },
    KnobDef {
        key: KnobKey::FlatDensityBoost,
        section: SectionKey::Density,
        label: "Flat Boost",
        min: 0.5,
        max: 1.0,
        step: 0.05,
        unit: Some("×"),
        hint: "Grid spacing on flat ceilings — lower = denser supports on anchor surfaces (0.7 = ~2× the supports)",
    },
    KnobDef {
        key: KnobKey::SlopeRelaxFactor,
        section: SectionKey::Density,
        label: "Slope Relax",
        min: 1.0,
        max: 2.0,
        step: 0.1,
        unit: Some("×"),
        hint: "Grid spacing on slopes at the self-support angle — higher = sparser",
    },
    KnobDef {
        key: KnobKey::SuctionAreaExponent,
        section: SectionKey::Density,
        label: "Suction Scale",
        min: 0.0,
        max: 0.4,
        step: 0.05,
        unit: None,
        hint: "How strongly flat density grows with region area — large shallow ceilings carry more peel. 0 = off",
    },
    KnobDef {
        key: KnobKey::OverhangSelfSupportAngleDeg,
        section: SectionKey::Stability,
        label: "Self-Support Angle",
        min: 20.0,
        max: 75.0,
        step: 5.0,
        unit: Some("°"),
        hint: "Surfaces flatter than this angle get supports (resin standard: 45°). Higher = fewer, mostly on the steepest parts.",
    },
    KnobDef {
        key: KnobKey::MaxAttachmentsPerTrunk,
        section: SectionKey::PostProcessing,
        label: "Branches per Column",
        min: 2.0,
        max: 50.0,
        step: 1.0,
        unit: None,
        hint: "Max branches + leaves one trunk may carry before new trunks are started — the cap on chunk consolidation",
    },
    KnobDef {
        key: KnobKey::CoverageTargetPercent,
        section: SectionKey::PostProcessing,
        label: "Coverage Target",
        min: 75.0,
        max: 100.0,
        step: 5.0,
        unit: Some("%"),
        hint: "How much of each region's footprint the grid must cover before gap-filling stops",
    },
];

/// `ADVANCED_CALIBRATION_KNOBS` — bounds from `AUTO_SUPPORT_CONSTRAINTS`.
static CALIBRATION_KNOBS: [KnobDef; 5] = [
    KnobDef {
        key: KnobKey::TipContactMarginScale,
        section: SectionKey::Detection,
        label: "Tip Fit Margin",
        min: 0.4,
        max: 1.0,
        step: 0.05,
        unit: None,
        hint: "How much of the free width a contact tip may occupy. The default, 0.9, keeps the disc just inside the feature it lands on; lower values shrink every tip on a feature narrower than the band tip.",
    },
    KnobDef {
        key: KnobKey::MemberHostShaftRatio,
        section: SectionKey::Detection,
        label: "Member / Host Ratio",
        min: 0.5,
        max: 1.0,
        step: 0.05,
        unit: None,
        hint: "Least diameter a branch or leaf takes as a fraction of the trunk it grows from. The default, 0.7, keeps a hosted member a step below its host instead of a needle beside it.",
    },
    KnobDef {
        key: KnobKey::ModelSizeFactorCap,
        section: SectionKey::Detection,
        label: "Model Size Cap",
        min: 1.0,
        max: 2.0,
        step: 0.05,
        unit: None,
        hint: "Ceiling of the size factor derived from the model's bounding-box diagonal. The default, 1.45, bounds how much a large part's shafts thicken.",
    },
    KnobDef {
        key: KnobKey::ModelLoadFactorCap,
        section: SectionKey::Detection,
        label: "Model Load Cap",
        min: 1.0,
        max: 2.0,
        step: 0.05,
        unit: None,
        hint: "Ceiling of the load factor derived from resin mass per support. The default, 1.3, bounds how much a heavy part's shafts thicken.",
    },
    KnobDef {
        key: KnobKey::HeightFactorCap,
        section: SectionKey::Detection,
        label: "Height Factor Cap",
        min: 1.0,
        max: 2.0,
        step: 0.05,
        unit: None,
        hint: "Ceiling of the height factor for a tall column, whose buckling load falls with height. The default, 1.35, bounds how much a tall support thickens.",
    },
];

struct ToggleDef {
    key: ToggleKey,
    section: SectionKey,
    /// The element-id stem; the two session switches have no draft key of their
    /// own, so it cannot be derived from `key`.
    slug: &'static str,
    label: &'static str,
    hint: &'static str,
}

/// `TOGGLES`, in table order.
static TOGGLES: [ToggleDef; 6] = [
    ToggleDef {
        key: ToggleKey::Enabled,
        section: SectionKey::Detection,
        slug: "enabled",
        label: "Enabled",
        hint: "Generate supports automatically on scan",
    },
    ToggleDef {
        key: ToggleKey::PrioritizeIntersection,
        section: SectionKey::Detection,
        slug: "prioritize-intersection",
        label: "Prioritize Dual",
        hint: "Islands found by BOTH the slice and mesh scans are placed first (they are the most certain)",
    },
    ToggleDef {
        key: ToggleKey::StabilizationEnabled,
        section: SectionKey::Stability,
        slug: "stabilization-enabled",
        label: "Stabilization Anchors",
        hint: "Add contacts along the edge or corner a pose bears on, so a leaning part is held against toppling and peel (default on)",
    },
    ToggleDef {
        key: ToggleKey::MinimaReinforcementEnabled,
        section: SectionKey::Stability,
        slug: "minima-reinforcement-enabled",
        label: "Minima Reinforcement",
        hint: "Ring each mesh minima with contacts on its own flank, so a section starts on a base instead of a needle (default on)",
    },
    ToggleDef {
        key: ToggleKey::DebugSupportOriginColors,
        section: SectionKey::Debug,
        slug: "debug-origin-colors",
        label: "Origin Colors",
        hint: "Debug: color supports by origin — stump (red), overhang (orange), island (blue), standalone (purple), reinforcement (teal)",
    },
    ToggleDef {
        key: ToggleKey::DebugSkipAutoBracing,
        section: SectionKey::Debug,
        slug: "debug-skip-auto-bracing",
        label: "No Brace",
        hint: "Debug: skip automatic bracing for this run",
    },
];

/// `ADVANCED_CALIBRATION_TOGGLE`.
static CALIBRATION_TOGGLE: ToggleDef = ToggleDef {
    key: ToggleKey::ModelScaleEnabled,
    section: SectionKey::Debug,
    slug: "model-scale-enabled",
    label: "Model-Scale Sizing",
    hint: "Master switch for the run-level size, load and height factors. Off pins all three to ×1, so sizing is the active band plus the local terms alone (default: on).",
};

/// `DIAGNOSTIC_TOGGLES` — the debug section's store-backed switches.
static DIAGNOSTIC_TOGGLES: [ToggleDef; 2] = [
    ToggleDef {
        key: ToggleKey::DebugSupportOriginColors,
        section: SectionKey::Debug,
        slug: "debug-origin-colors",
        label: "Origin Colors",
        hint: "Debug: color supports by origin — stump (red), overhang (orange), island (blue), standalone (purple), reinforcement (teal)",
    },
    ToggleDef {
        key: ToggleKey::DebugSkipAutoBracing,
        section: SectionKey::Debug,
        slug: "debug-skip-auto-bracing",
        label: "No Brace",
        hint: "Debug: skip automatic bracing for this run",
    },
];

/// `SIZING_TIER_FIELD`.
const SIZING_TIER_LABEL: &str = "Sizing Tier";
const SIZING_TIER_HINT: &str = "The Support Studio preset whose tip, shaft and root numbers size this run. Editing that preset changes what auto-support prints — Duplicate it first if you only want to change the run.";

// ── Sizing tier ─────────────────────────────────────────────────────────────

/// `sizingPreset` — the Support Studio band a run sizes with.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SizingTier {
    Detail,
    Structure,
    Anchor,
}

impl SizingTier {
    const ALL: [SizingTier; 3] = [Self::Detail, Self::Structure, Self::Anchor];

    fn id(self) -> &'static str {
        match self {
            Self::Detail => "detail",
            Self::Structure => "structure",
            Self::Anchor => "anchor",
        }
    }

    /// The factory preset's name (`DETAIL_PRESET.name` …).
    fn label(self) -> &'static str {
        match self {
            Self::Detail => "Detail",
            Self::Structure => "Structure",
            Self::Anchor => "Anchor",
        }
    }
}

// ── Presets ─────────────────────────────────────────────────────────────────

/// One record of the auto-support preset collection.
#[derive(Clone)]
struct Preset {
    id: SharedString,
    name: SharedString,
    built_in: bool,
    settings: AutoSupportSettings,
}

/// `BUILT_IN_PRESETS`.
fn built_in_presets() -> Vec<Preset> {
    let light = AutoSupportSettings {
        area_per_support_mm2: 16.0,
        sizing_preset: SizingTier::Detail,
        ..AutoSupportSettings::default()
    };
    let medium = AutoSupportSettings::default();
    let heavy = AutoSupportSettings {
        area_per_support_mm2: 5.0,
        sizing_preset: SizingTier::Anchor,
        ..AutoSupportSettings::default()
    };
    vec![
        Preset {
            id: "light".into(),
            name: "Light".into(),
            built_in: true,
            settings: light,
        },
        Preset {
            id: "medium".into(),
            name: "Medium".into(),
            built_in: true,
            settings: medium,
        },
        Preset {
            id: "heavy".into(),
            name: "Heavy".into(),
            built_in: true,
            settings: heavy,
        },
    ]
}

/// `ensureUniquePresetName`: case-insensitive, `" (2)"` onwards.
fn unique_preset_name(presets: &[Preset], desired: &str, exclude: Option<&str>) -> String {
    let base = {
        let trimmed = desired.trim();
        if trimmed.is_empty() {
            DEFAULT_PRESET_NAME
        } else {
            trimmed
        }
    };
    let taken = |candidate: &str| {
        let lower = candidate.to_lowercase();
        presets
            .iter()
            .any(|preset| Some(preset.id.as_ref()) != exclude && preset.name.to_lowercase() == lower)
    };
    let mut name = base.to_string();
    let mut counter = 1;
    while taken(&name) {
        counter += 1;
        name = format!("{base} ({counter})");
    }
    name
}

// ── Dialog state ────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum NameMode {
    Create,
    Rename,
}

struct NameDialog {
    mode: NameMode,
    name: String,
}

struct DialogState {
    /// The dialog's draft — what Save commits and Reset discards.
    draft: AutoSupportSettings,
    /// The applied block (`getSettings().autoSupport`).
    live: AutoSupportSettings,
    presets: Vec<Preset>,
    active: Option<SharedString>,
    preset_menu_open: bool,
    sizing_menu_open: bool,
    /// The store-backed diagnostics: they apply the moment they are flipped.
    origin_colors: bool,
    skip_auto_bracing: bool,
    /// Session switches the panel owns.
    debug_simple_render: bool,
    debug_mode: bool,
    pending_delete: bool,
    name_dialog: Option<NameDialog>,
    /// The save's two-second acknowledgement.
    saved: bool,
    next_custom: u32,
}

impl Default for DialogState {
    fn default() -> Self {
        let live = AutoSupportSettings::default();
        Self {
            draft: live,
            live,
            presets: built_in_presets(),
            active: None,
            preset_menu_open: false,
            sizing_menu_open: false,
            origin_colors: false,
            skip_auto_bracing: false,
            debug_simple_render: false,
            debug_mode: false,
            pending_delete: false,
            name_dialog: None,
            saved: false,
            next_custom: 0,
        }
    }
}

impl DialogState {
    fn active_preset(&self) -> Option<&Preset> {
        let active = self.active.as_ref()?;
        self.presets.iter().find(|preset| preset.id == *active)
    }

    fn is_built_in(&self) -> bool {
        self.active_preset().is_some_and(|preset| preset.built_in)
    }

    /// `isAutoSupportPresetDirty`.
    fn dirty(&self) -> bool {
        self.active_preset()
            .is_some_and(|preset| policy_differs(&preset.settings, &self.live))
    }

    /// `useAutoSupportDialogChanges`: the preset has drifted, or the draft has.
    fn has_changes(&self) -> bool {
        self.dirty() || policy_differs(&self.draft, &self.live)
    }
}

static AUTO_SUPPORT_DIALOG: LazyLock<Mutex<DialogState>> =
    LazyLock::new(|| Mutex::new(DialogState::default()));

fn state() -> &'static Mutex<DialogState> {
    &AUTO_SUPPORT_DIALOG
}

fn edit(f: impl FnOnce(&mut DialogState)) {
    match state().lock() {
        Ok(mut guard) => f(&mut guard),
        Err(poisoned) => f(&mut poisoned.into_inner()),
    }
}

fn edit_and_notify(
    entity: &WeakEntity<DragonFruit>,
    cx: &mut App,
    f: impl FnOnce(&mut DialogState),
) {
    edit(f);
    entity.update(cx, |_, cx| cx.notify()).ok();
}

fn close(entity: &WeakEntity<DragonFruit>, cx: &mut App) {
    entity
        .update(cx, |this, cx| {
            this.show_support_settings = false;
            cx.notify();
        })
        .ok();
}

/// `setActiveAutoSupportPreset`: applies the preset to the live block, and the
/// dialog's draft follows it so the fields show what was applied.
fn select_preset(id: &str) {
    edit(|state| {
        let Some(preset) = state
            .presets
            .iter()
            .find(|preset| preset.id.as_ref() == id)
            .cloned()
        else {
            return;
        };
        state.active = Some(preset.id);
        state.live = preset.settings;
        state.draft = preset.settings;
    });
}

/// The live block reaches the app: `DragonFruit::support_angle` is the one knob
/// `stl::overhang_points` reads.
fn apply_live_to_app(entity: &WeakEntity<DragonFruit>, cx: &mut App) {
    let angle = match state().lock() {
        Ok(guard) => guard.live.overhang_self_support_angle_deg,
        Err(poisoned) => poisoned.into_inner().live.overhang_self_support_angle_deg,
    };
    entity
        .update(cx, |this, cx| {
            this.support_angle = angle;
            cx.notify();
        })
        .ok();
}

/// `normalizeFieldValue`: clamped, then rounded to the decimals the step implies.
fn normalize_value(value: f32, min: f32, max: f32, step: f32) -> f32 {
    let decimals = if step < 0.1 {
        2
    } else if step < 1.0 {
        1
    } else {
        0
    };
    let factor = 10f32.powi(decimals);
    let clamped = value.clamp(min, max);
    (clamped * factor).round() / factor
}

// ── Field atoms ─────────────────────────────────────────────────────────────

/// The `ui-label` line: the name and its ⓘ affordance.
fn label_row(label: &'static str, help: &'static str, id: &str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .text_size(px(FONT_SM))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(INDICATOR))
        .child(label)
        .child(help_chip(id, help))
}

/// `FieldHelpTooltip`: the 14px bordered ⓘ, with the help on hover.
fn help_chip(id: &str, help: &'static str) -> impl IntoElement {
    let chip = div()
        .flex()
        .items_center()
        .justify_center()
        .w(px(14.0))
        .h(px(14.0))
        .rounded(px(ROUNDED_BARE))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .child(icon("circle-question-mark", 10.0, TEXT_MUTED));
    tooltip(
        ElementId::Name(format!("{id}-help").into()),
        chip,
        help_text(help),
    )
}

fn help_text(text: &'static str) -> Div {
    div().w(px(240.0)).child(text)
}

/// `NumberField` — `LabeledNumberInput`'s labelled field with the default
/// stepper, never a slider.
fn number_field(
    def: &'static KnobDef,
    value: f32,
    locked: bool,
    entity: &WeakEntity<DragonFruit>,
) -> AnyElement {
    let hint = if locked { LOCKED_PROFILE_HINT } else { def.hint };
    let id = format!("as-knob-{}", def.key.slug());
    let step = def.step;
    let min = def.min;
    let max = def.max;
    let key = def.key;

    let up_entity = entity.clone();
    let up = move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
        let next = normalize_value(value + step, min, max, step);
        edit_and_notify(&up_entity, cx, |state| state.draft.set_knob(key, next));
    };
    let down_entity = entity.clone();
    let down = move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
        let next = normalize_value(value - step, min, max, step);
        edit_and_notify(&down_entity, cx, |state| state.draft.set_knob(key, next));
    };

    let field = div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .w_full()
        .child(label_row(def.label, hint, &id))
        .child(
            labeled_number_field(
                id.clone(),
                format_number(value),
                def.unit,
                locked,
                !locked,
                !locked,
                up,
                down,
            )
            .w_full(),
        );

    if locked {
        tooltip(
            ElementId::Name(format!("{id}-locked").into()),
            field,
            help_text(hint),
        )
        .into_any_element()
    } else {
        field.into_any_element()
    }
}

/// The switch a `LabeledToggleInput` renders: a 36px `.ui-input` row whose right
/// side is the pill.
///
/// The row keeps `.ui-input`'s 10px padding and its 13px / 1.3 type (the
/// source's `px-2.5 leading-tight text-sm` all lose to the unlayered rule). The
/// pill is `Toggle`'s `sm` track (`h-5 w-9 px-0.5`, 16px knob) repainted with
/// the source's `--accent-secondary` / disabled fill.
fn toggle_switch_row(on: bool, disabled: bool) -> Div {
    let (border, background, text) = if disabled {
        (
            BORDER_SUBTLE,
            mix(SURFACE_2, BLACK, 0.92),
            TEXT_MUTED,
        )
    } else if on {
        (
            mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.64),
            mix(ACCENT_SECONDARY, SURFACE_1, 0.10),
            TEXT_STRONG,
        )
    } else {
        (BORDER_SUBTLE, SURFACE_1, TEXT_MUTED)
    };
    let track = if disabled {
        mix(SURFACE_2, BLACK, 0.92)
    } else if on {
        ACCENT_SECONDARY
    } else {
        SURFACE_2
    };

    div()
        .h(px(36.0))
        .px(px(10.0))
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .text_size(px(FONT_SM))
        .line_height(px(FONT_SM * 1.3))
        .text_color(rgb(text))
        .flex()
        .items_center()
        .justify_between()
        .child(if on { "Enabled" } else { "Disabled" })
        .child(toggle_sized(on, ToggleSize::Sm).bg(rgb(track)))
}

#[derive(Clone, Copy)]
enum ToggleAction {
    Draft(ToggleKey),
    OriginColors,
    SkipAutoBracing,
    Simplified,
    DebugMode,
}

/// `ToggleField` — the labelled switch.
fn toggle_field(
    def: &'static ToggleDef,
    on: bool,
    locked: bool,
    action: ToggleAction,
    entity: &WeakEntity<DragonFruit>,
) -> AnyElement {
    let hint = if locked { LOCKED_PROFILE_HINT } else { def.hint };
    let id = format!("as-toggle-{}", def.slug);

    let mut button = div()
        .id(ElementId::Name(format!("{id}-switch").into()))
        .child(toggle_switch_row(on, locked));
    if locked {
        button = button.opacity(0.55).cursor_not_allowed();
    } else {
        let click_entity = entity.clone();
        button = button.cursor_pointer().on_click(move |_, _, cx| {
            let next = !on;
            match action {
                ToggleAction::Draft(key) => {
                    edit_and_notify(&click_entity, cx, |state| state.draft.set_toggle(key, next));
                }
                ToggleAction::OriginColors => {
                    edit_and_notify(&click_entity, cx, |state| {
                        state.origin_colors = next;
                        state.live.debug_support_origin_colors = next;
                        state.draft.debug_support_origin_colors = next;
                    });
                }
                ToggleAction::SkipAutoBracing => {
                    edit_and_notify(&click_entity, cx, |state| {
                        state.skip_auto_bracing = next;
                        state.live.debug_skip_auto_bracing = next;
                        state.draft.debug_skip_auto_bracing = next;
                    });
                }
                ToggleAction::Simplified => {
                    edit_and_notify(&click_entity, cx, |state| state.debug_simple_render = next);
                }
                ToggleAction::DebugMode => {
                    edit_and_notify(&click_entity, cx, |state| state.debug_mode = next);
                }
            }
        });
    }

    let field = div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .w_full()
        .child(label_row(def.label, hint, &id))
        .child(button);

    if locked {
        tooltip(
            ElementId::Name(format!("{id}-locked").into()),
            field,
            help_text(hint),
        )
        .into_any_element()
    } else {
        field.into_any_element()
    }
}

// ── Cards ───────────────────────────────────────────────────────────────────

enum Cell {
    Half(AnyElement),
    Full(AnyElement),
}

fn half_row(first: AnyElement, second: AnyElement) -> Div {
    div()
        .flex()
        .gap(px(8.0))
        .items_stretch()
        .child(div().flex_1().min_w(px(0.0)).child(first))
        .child(div().flex_1().min_w(px(0.0)).child(second))
}

/// Two cards in one `grid-cols-2` row, stretched to the row's height.
fn two_cards(first: Div, second: Div) -> Div {
    div()
        .flex()
        .gap(px(12.0))
        .items_stretch()
        .child(first.flex_1().min_w(px(0.0)))
        .child(second.flex_1().min_w(px(0.0)))
}

/// The card's `grid grid-cols-2 gap-2`, as rows of two.
fn field_grid(cells: Vec<Cell>) -> Div {
    let mut grid = div().mt(px(8.0)).flex().flex_col().gap(px(8.0));
    let mut pending: Vec<AnyElement> = Vec::new();
    for cell in cells {
        match cell {
            Cell::Half(element) => {
                pending.push(element);
                if pending.len() == 2 {
                    let second = pending.pop().expect("two cells");
                    let first = pending.pop().expect("two cells");
                    grid = grid.child(half_row(first, second));
                }
            }
            Cell::Full(element) => {
                if let Some(first) = pending.pop() {
                    grid = grid.child(half_row(first, div().into_any_element()));
                }
                grid = grid.child(div().flex().child(element));
            }
        }
    }
    if let Some(first) = pending.pop() {
        grid = grid.child(half_row(first, div().into_any_element()));
    }
    grid
}

/// `FieldCard` — the section's uppercase header, its one-liner and its grid.
fn field_card(section: &'static SectionDef, cells: Vec<Cell>) -> Div {
    let header = div()
        .text_size(px(FONT_SM))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(INDICATOR))
        .child(section.label.to_uppercase());
    let mut card = div()
        .flex()
        .flex_col()
        .rounded(px(ROUNDED_XL))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_2))
        .p(px(12.0))
        .child(tooltip(
            ElementId::Name(format!("as-section-{}", section.key.slug()).into()),
            header,
            help_text(section.hint),
        ));
    if let Some(subtitle) = section.subtitle {
        card = card.child(
            div()
                .mt(px(2.0))
                .text_size(px(12.0))
                .line_height(relative(1.375))
                .text_color(rgb(TEXT_MUTED))
                .child(subtitle),
        );
    }
    card.child(field_grid(cells))
}

/// `DiagnosticsCard` — the debug switches and the two session switches.
fn diagnostics_card(
    origin_colors: bool,
    skip_auto_bracing: bool,
    debug_simple_render: bool,
    debug_mode: bool,
    entity: &WeakEntity<DragonFruit>,
) -> Div {
    let mut cells = Vec::new();
    for (index, def) in DIAGNOSTIC_TOGGLES.iter().enumerate() {
        let (on, action) = match index {
            0 => (origin_colors, ToggleAction::OriginColors),
            _ => (skip_auto_bracing, ToggleAction::SkipAutoBracing),
        };
        cells.push(Cell::Half(toggle_field(def, on, false, action, entity)));
    }
    cells.push(Cell::Half(toggle_field(
        &SIMPLIFIED_TOGGLE,
        debug_simple_render,
        false,
        ToggleAction::Simplified,
        entity,
    )));
    cells.push(Cell::Half(toggle_field(
        &DEBUG_MODE_TOGGLE,
        debug_mode,
        false,
        ToggleAction::DebugMode,
        entity,
    )));

    div()
        .flex()
        .flex_col()
        .rounded(px(ROUNDED_XL))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_2))
        .p(px(12.0))
        .child(
            div()
                .text_size(px(FONT_SM))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(INDICATOR))
                .child(DEBUG_DIAGNOSTICS_HEADING.to_uppercase()),
        )
        .child(field_grid(cells))
}

/// The two `msg` descriptors the diagnostics card adds beside the store switches.
static SIMPLIFIED_TOGGLE: ToggleDef = ToggleDef {
    key: ToggleKey::DebugSupportOriginColors,
    section: SectionKey::Debug,
    slug: "debug-simplified",
    label: "Simplified",
    hint: "Debug: simplified support render — contact disks/cones plus line vectors instead of full shafts",
};
static DEBUG_MODE_TOGGLE: ToggleDef = ToggleDef {
    key: ToggleKey::DebugSkipAutoBracing,
    section: SectionKey::Debug,
    slug: "debug-mode",
    label: "Debug Mode",
    hint: "Debug: show the last run's sizing factors and forest report in the panel",
};

/// The calibration card: the warning-toned frame around the six constants.
fn advanced_card(
    draft: &AutoSupportSettings,
    locked: bool,
    entity: &WeakEntity<DragonFruit>,
) -> Div {
    let mut cells = vec![Cell::Half(toggle_field(
        &CALIBRATION_TOGGLE,
        draft.model_scale_enabled,
        locked,
        ToggleAction::Draft(ToggleKey::ModelScaleEnabled),
        entity,
    ))];
    for def in CALIBRATION_KNOBS.iter() {
        cells.push(Cell::Half(number_field(
            def,
            draft.knob(def.key),
            locked,
            entity,
        )));
    }

    let header = div()
        .text_size(px(FONT_SM))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(INDICATOR))
        .child(ADVANCED_LABEL.to_uppercase());
    div()
        .flex()
        .flex_col()
        .rounded(px(ROUNDED_XL))
        .border_1()
        .border_color(rgb(mix(WARNING, BORDER_SUBTLE, 0.64)))
        .bg(rgb(mix(WARNING, SURFACE_1, 0.08)))
        .p(px(12.0))
        .child(tooltip(
            ElementId::Name("as-section-advanced".into()),
            header,
            help_text(ADVANCED_HINT),
        ))
        .child(field_grid(cells))
}

// ── Sizing tier field ───────────────────────────────────────────────────────

fn sizing_tier_field(
    current: SizingTier,
    locked: bool,
    menu_open: bool,
    entity: &WeakEntity<DragonFruit>,
) -> AnyElement {
    let hint = if locked { LOCKED_PROFILE_HINT } else { SIZING_TIER_HINT };
    let options: Vec<SelectOption> = SizingTier::ALL
        .iter()
        .map(|tier| {
            SelectOption::new(tier.id(), tier.label()).with_right("Built-in")
        })
        .collect();

    let toggle_entity = entity.clone();
    let on_toggle = move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
        edit_and_notify(&toggle_entity, cx, |state| {
            state.sizing_menu_open = !state.sizing_menu_open;
        });
    };
    let select_entity = entity.clone();
    let on_select = move |index: usize, _: &mut Window, cx: &mut App| {
        let Some(tier) = SizingTier::ALL.get(index).copied() else {
            return;
        };
        edit_and_notify(&select_entity, cx, |state| {
            state.sizing_menu_open = false;
            state.draft.sizing_preset = tier;
        });
    };

    let field = div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .w_full()
        .child(label_row(SIZING_TIER_LABEL, hint, "as-sizing-tier"))
        .child(select_dropdown(
            "as-sizing-tier-select",
            &options,
            current.id(),
            menu_open,
            locked,
            on_toggle,
            on_select,
        ));

    if locked {
        tooltip(
            ElementId::Name("as-sizing-tier-locked".into()),
            field,
            help_text(hint),
        )
        .into_any_element()
    } else {
        field.into_any_element()
    }
}

// ── Preset strip ────────────────────────────────────────────────────────────

/// `STRIP_BUTTON`: `Button variant="secondary" size="auto"` with
/// `!h-8 !px-2.5 !py-0 text-[11px] inline-flex items-center gap-1
/// disabled:opacity-45`.
///
/// `.ui-button` is unlayered, so the `!`-marked height, horizontal padding and
/// zero vertical padding apply while `text-[11px]` loses to its 14px type.
fn strip_button(
    id: &'static str,
    icon_name: &'static str,
    label: &'static str,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let mut spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Sm)
        .padding_x(10.0)
        .padding_y(0.0, 0.0)
        .gap(4.0)
        .icon(
            icon_name,
            14.0,
            if enabled {
                TEXT_STRONG
            } else {
                disabled_foreground()
            },
        );
    if !enabled {
        spec = spec.disabled();
    }
    let el = button_with(id, label, &spec);
    if enabled {
        el.on_click(on_click)
    } else {
        el.opacity(0.45)
    }
}

fn preset_strip(
    presets: &[Preset],
    active: Option<&SharedString>,
    modified: bool,
    menu_open: bool,
    is_built_in: bool,
    entity: &WeakEntity<DragonFruit>,
) -> Div {
    // ── Options ─────────────────────────────────────────────────────────────
    let mut options: Vec<SelectOption> = Vec::new();
    let mut offset = 0;
    if active.is_none() {
        options.push(SelectOption::new("", NO_ACTIVE_PRESET_LABEL).disabled());
        offset = 1;
    }
    for preset in presets {
        let selected = active.is_some_and(|active| *active == preset.id);
        let mut label = preset.name.to_string();
        if selected && modified {
            label.push_str(" *");
        }
        let mut option = SelectOption::new(preset.id.clone(), label).with_right(format!(
            "{} · {}",
            preset.settings.sizing_preset.label(),
            if preset.built_in { "Built-in" } else { "Custom" }
        ));
        if selected {
            option = option.with_icon("check");
        }
        options.push(option);
    }
    options.push(
        SelectOption::action("new", NEW_PRESET_ENTRY)
            .with_icon("plus")
            .with_tone(SelectOptionTone::Accent),
    );

    let value: String = active.map(|active| active.to_string()).unwrap_or_default();
    let presets_for_select = presets.to_vec();
    let toggle_entity = entity.clone();
    let on_toggle = move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
        edit_and_notify(&toggle_entity, cx, |state| {
            state.preset_menu_open = !state.preset_menu_open;
        });
    };
    let select_entity = entity.clone();
    let on_select = move |index: usize, _: &mut Window, cx: &mut App| {
        let Some(relative) = index.checked_sub(offset) else {
            return;
        };
        if relative == presets_for_select.len() {
            // `New preset…` — the menu's own entry.
            edit_and_notify(&select_entity, cx, |state| {
                state.preset_menu_open = false;
                state.name_dialog = Some(NameDialog {
                    mode: NameMode::Create,
                    name: NEW_PRESET_NAME.to_string(),
                });
            });
            return;
        }
        let Some(preset) = presets_for_select.get(relative) else {
            return;
        };
        let id = preset.id.to_string();
        select_preset(&id);
        edit_and_notify(&select_entity, cx, |state| state.preset_menu_open = false);
        apply_live_to_app(&select_entity, cx);
    };

    let selector = select_dropdown(
        "as-preset-select",
        &options,
        value.as_str(),
        menu_open,
        false,
        on_toggle,
        on_select,
    );

    // ── The row's actions ───────────────────────────────────────────────────
    let has_active = active.is_some();
    let rename_entity = entity.clone();
    let active_name = active
        .and_then(|active| presets.iter().find(|preset| preset.id == *active))
        .map(|preset| preset.name.to_string());
    let rename = strip_button(
        "as-preset-rename",
        "pen-line",
        "Rename",
        has_active && !is_built_in,
        move |_, _, cx| {
            let Some(name) = active_name.clone() else {
                return;
            };
            edit_and_notify(&rename_entity, cx, |state| {
                state.name_dialog = Some(NameDialog {
                    mode: NameMode::Rename,
                    name,
                });
            });
        },
    );

    let duplicate_entity = entity.clone();
    let duplicate = strip_button(
        "as-preset-duplicate",
        "copy",
        "Duplicate",
        has_active,
        move |_, _, cx| {
            edit_and_notify(&duplicate_entity, cx, |state| {
                let Some(source) = state.active_preset().cloned() else {
                    return;
                };
                // `duplicateAutoSupportPreset`: the copy takes the source's
                // stored block, and the selection moves to it. The live block is
                // left alone, so a staged draft survives.
                let desired = format!("{} copy", source.name);
                let name = unique_preset_name(&state.presets, &desired, None);
                state.next_custom += 1;
                let id: SharedString = format!("asp-{}", state.next_custom).into();
                state.presets.push(Preset {
                    id: id.clone(),
                    name: name.into(),
                    built_in: false,
                    settings: source.settings,
                });
                state.active = Some(id);
            });
        },
    );

    // PORT GAP: the preset file format needs the Tauri bridge and a JSON
    // reader; both buttons keep the source's enabled recipe but write nothing.
    let export = strip_button("as-preset-export", "download", "Export", has_active, |_, _, _| {});
    let import = strip_button("as-preset-import", "upload", "Import", true, |_, _, _| {});

    div()
        .flex()
        .flex_col()
        .rounded(px(ROUNDED_XL))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_2))
        .p(px(12.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(div().flex_1().min_w(px(0.0)).child(selector))
                .child(rename)
                .child(duplicate)
                .child(export)
                .child(import),
        )
}

// ── Footer ──────────────────────────────────────────────────────────────────

fn footer_delete_button(enabled: bool, entity: &WeakEntity<DragonFruit>) -> Stateful<Div> {
    // `Button variant="tinted-danger" size="md"`, whose `.ui-button:disabled`
    // recipe `ButtonSpec::disabled` already paints.
    let mut spec = ButtonSpec::new(ButtonVariant::TintedDanger, ButtonSize::Md)
        .gap(6.0)
        .icon(
            "trash-2",
            14.0,
            if enabled { DANGER } else { disabled_foreground() },
        );
    if !enabled {
        spec = spec.disabled();
    }
    let el = button_with("as-footer-delete", "Delete", &spec);
    if enabled {
        let click_entity = entity.clone();
        el.on_click(move |_, _, cx| {
            edit_and_notify(&click_entity, cx, |state| state.pending_delete = true);
        })
    } else {
        el
    }
}

fn footer_reset_button(enabled: bool, entity: &WeakEntity<DragonFruit>) -> Stateful<Div> {
    // `FOOTER_BUTTON`: `Button variant="secondary" size="auto"` with `!h-9
    // px-3 text-[12px] disabled:opacity-45`; the unlayered `.ui-button` keeps
    // its 11px padding and 14px type, so only the 36px height applies.
    let mut spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Md)
        .gap(6.0)
        .icon(
            "rotate-ccw",
            14.0,
            if enabled {
                TEXT_STRONG
            } else {
                disabled_foreground()
            },
        );
    if !enabled {
        spec = spec.disabled();
    }
    let el = button_with("as-footer-reset", "Reset", &spec);
    if enabled {
        let click_entity = entity.clone();
        el.on_click(move |_, _, cx| {
            edit(|state| {
                let restored = state.active_preset().map(|preset| preset.settings);
                if let Some(restored) = restored {
                    state.live = restored;
                }
                state.draft = state.live;
            });
            apply_live_to_app(&click_entity, cx);
        })
    } else {
        el.opacity(0.45)
    }
}

fn footer_save_button(
    enabled: bool,
    saved: bool,
    entity: &WeakEntity<DragonFruit>,
) -> Stateful<Div> {
    // `Button size="md"` (secondary) with the source's inline `SAVE_STYLE` /
    // `SAVED_STYLE`; `.ui-button:disabled` stays the base while disabled.
    let mut spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Md).gap(6.0);
    if saved {
        spec = spec.icon("check", 14.0, SUCCESS);
    }
    if !enabled {
        spec = spec.disabled();
    }
    let el = button_with(
        "as-footer-save",
        if saved { "Saved!" } else { "Save" },
        &spec,
    );
    let el = if saved {
        el.border_color(rgb(saved_border()))
            .bg(rgb(saved_background()))
            .text_color(rgb(SUCCESS))
    } else if enabled {
        el.border_color(rgb(save_border()))
            .bg(rgb(save_background()))
            .text_color(rgb(ACCENT_SECONDARY))
    } else {
        el
    };
    if enabled && !saved {
        let click_entity = entity.clone();
        el.on_click(move |_, _, cx| {
                edit(|state| {
                    // `commitAutoSupportSettings`: the draft becomes the live
                    // block, and the active preset is written with it when the
                    // two differ (a built-in is refused, hence Save is disabled).
                    let draft = state.draft;
                    state.live = draft;
                    if let Some(active) = state.active.clone() {
                        if let Some(preset) = state
                            .presets
                            .iter_mut()
                            .find(|preset| preset.id == active && !preset.built_in)
                        {
                            preset.settings = draft;
                        }
                    }
                    state.saved = true;
                });
                apply_live_to_app(&click_entity, cx);
                // Two seconds of acknowledgement, then back to the button.
                click_entity
                    .update(cx, |_, cx| {
                        cx.spawn(async move |this, cx| {
                            cx.background_executor()
                                .timer(Duration::from_millis(2000))
                                .await;
                            this.update(cx, |_, cx| {
                                edit(|state| state.saved = false);
                                cx.notify();
                            })
                            .ok();
                        })
                        .detach();
                    })
                    .ok();
        })
    } else {
        el.cursor_not_allowed()
    }
}

fn footer(
    has_active: bool,
    is_built_in: bool,
    has_changes: bool,
    saved: bool,
    entity: &WeakEntity<DragonFruit>,
) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .px(px(16.0))
        .py(px(12.0))
        .flex_shrink_0()
        .border_t_1()
        .border_color(rgb(BORDER_SUBTLE))
        // `color-mix(in srgb, var(--surface-1), transparent 10%)`.
        .bg(rgba(with_alpha(SURFACE_1, 0xe6)))
        .child(footer_delete_button(has_active && !is_built_in, entity))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(footer_reset_button(has_changes, entity))
                .child(footer_save_button(has_changes && !is_built_in, saved, entity)),
        )
}

// ── Nested dialogs ──────────────────────────────────────────────────────────

fn dialog_cancel_button(entity: &WeakEntity<DragonFruit>, action: &'static str) -> Stateful<Div> {
    let click_entity = entity.clone();
    // `Button variant="secondary" size="md"` — the base `.ui-button` recipe.
    let spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Md);
    button_with(
        ElementId::Name(format!("as-{action}-cancel").into()),
        "Cancel",
        &spec,
    )
    .on_click(move |_, _, cx| {
        edit_and_notify(&click_entity, cx, |state| {
            state.name_dialog = None;
            state.pending_delete = false;
        });
    })
}

fn delete_dialog(preset_name: &str, entity: &WeakEntity<DragonFruit>) -> Div {
    let close_entity = entity.clone();
    let on_close = move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
        edit_and_notify(&close_entity, cx, |state| state.pending_delete = false);
    };
    let actions = {
        let confirm_entity = entity.clone();
        let mut delete = div()
            .id("as-delete-confirm")
            .flex()
            .items_center()
            .justify_center()
            .h(px(36.0))
            .px(px(11.0))
            .rounded(px(RADIUS_SM))
            .border_1()
            .border_color(rgb(mix(DANGER, BORDER_SUBTLE, 0.55)))
            .bg(rgb(mix(DANGER, SURFACE_1, 0.12)))
            .text_size(px(FONT_MD))
            .font_weight(FontWeight::SEMIBOLD)
            .line_height(px(FONT_MD))
            .text_color(rgb(DANGER))
            .cursor_pointer()
            .hover(|style| style.bg(rgb(mix(DANGER, SURFACE_1, 0.22))))
            .child("Delete");
        delete = delete.on_click(move |_, _, cx| {
            edit(|state| {
                // `deleteActiveAutoSupportPreset`: the store clears the
                // selection; the dialog falls back to the balanced built-in.
                if let Some(active) = state.active.clone() {
                    state.presets.retain(|preset| preset.id != active);
                }
                state.pending_delete = false;
            });
            select_preset("medium");
            apply_live_to_app(&confirm_entity, cx);
        });
        div()
            .flex()
            .gap(px(8.0))
            .pt(px(4.0))
            .child(dialog_cancel_button(entity, "delete"))
            .child(delete)
    };

    gpui_ui::ui::dialog_modal(
        512.0,
        format!("Delete \"{preset_name}\"?"),
        Some("This cannot be undone.".into()),
        None,
        gpui_ui::ui::IconTone::Danger,
        div()
            .text_size(px(11.0))
            .line_height(relative(1.625))
            .text_color(rgb(TEXT_MUTED))
            .child("The preset is removed from the list. The current settings are left as they are."),
        actions,
        on_close,
    )
}

fn name_dialog(dialog: &NameDialog, entity: &WeakEntity<DragonFruit>) -> Div {
    let is_create = dialog.mode == NameMode::Create;
    let title: SharedString = if is_create { "New Preset" } else { "Rename Preset" }.into();
    let subtitle: SharedString = if is_create {
        "The current settings are saved under this name, and it becomes the selected preset."
    } else {
        "The selected preset is renamed; its settings are not touched."
    }
    .into();
    let close_entity = entity.clone();
    let on_close = move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
        edit_and_notify(&close_entity, cx, |state| state.name_dialog = None);
    };

    let confirm_entity = entity.clone();
    let confirm_name = dialog.name.clone();
    let mut confirm = div()
        .id("as-name-confirm")
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .h(px(36.0))
        .px(px(11.0))
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(save_border()))
        .bg(rgb(save_background()))
        .text_size(px(FONT_MD))
        .font_weight(FontWeight::SEMIBOLD)
        .line_height(px(FONT_MD))
        .text_color(rgb(ACCENT_SECONDARY))
        .child(icon("check", 14.0, ACCENT_SECONDARY))
        .child(if is_create { "Create" } else { "Save Name" });
    let enabled = !confirm_name.trim().is_empty();
    if enabled {
        confirm = confirm.cursor_pointer().on_click(move |_, _, cx| {
            edit(|state| {
                let name = confirm_name.trim().to_string();
                match state.name_dialog.as_ref().map(|dialog| dialog.mode) {
                    Some(NameMode::Create) => {
                        let unique = unique_preset_name(&state.presets, &name, None);
                        state.next_custom += 1;
                        let id: SharedString = format!("asp-{}", state.next_custom).into();
                        // `createAutoSupportPreset`: the live block is the new
                        // preset's, and the new preset becomes the selection.
                        state.presets.push(Preset {
                            id: id.clone(),
                            name: unique.into(),
                            built_in: false,
                            settings: state.live,
                        });
                        state.active = Some(id);
                    }
                    Some(NameMode::Rename) => {
                        let active = state.active.clone();
                        if let Some(active) = active {
                            let unique = unique_preset_name(
                                &state.presets,
                                &name,
                                Some(active.as_ref()),
                            );
                            if let Some(preset) = state
                                .presets
                                .iter_mut()
                                .find(|preset| preset.id == active && !preset.built_in)
                            {
                                preset.name = unique.into();
                            }
                        }
                    }
                    None => {}
                }
                state.name_dialog = None;
                state.draft = state.live;
            });
            confirm_entity.update(cx, |_, cx| cx.notify()).ok();
        });
    } else {
        confirm = confirm.opacity(0.4).cursor_not_allowed();
    }

    let actions = div()
        .flex()
        .gap(px(8.0))
        .pt(px(4.0))
        .child(dialog_cancel_button(entity, "name"))
        .child(confirm);

    let body = div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .text_size(px(12.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .child("PRESET NAME"),
        )
        // No text entry in this slice: the field shows the name the dialog
        // opened with (see the module's PORT DEVIATIONS).
        .child(
            input_box(ElementId::Name("as-name-input".into()), false)
                .h(px(36.0))
                .w_full()
                .text_size(px(12.0))
                .child(SharedString::from(dialog.name.clone())),
        );

    gpui_ui::ui::dialog_modal(
        512.0,
        title,
        Some(subtitle),
        Some("pen-line"),
        gpui_ui::ui::IconTone::Accent,
        body,
        actions,
        on_close,
    )
}

// ── Header ──────────────────────────────────────────────────────────────────

fn dialog_header(entity: &WeakEntity<DragonFruit>) -> Div {
    let close_entity = entity.clone();
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.0))
        .px(px(16.0))
        .py(px(12.0))
        .flex_shrink_0()
        .border_b_1()
        .border_color(rgb(BORDER_SUBTLE))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                .min_w(px(0.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(36.0))
                        .h(px(36.0))
                        .flex_shrink_0()
                        .rounded(px(ROUNDED_LG))
                        .border_1()
                        .border_color(rgb(BORDER_SUBTLE))
                        .bg(linear_gradient(
                            135.0,
                            linear_color_stop(rgb(mix(ACCENT, SURFACE_1, 0.16)), 0.0),
                            linear_color_stop(rgb(mix(ACCENT_SECONDARY, SURFACE_1, 0.10)), 1.0),
                        ))
                        .child(icon("settings-2", 16.0, ACCENT)),
                )
                .child(
                    div()
                        .min_w(px(0.0))
                        .child(
                            div()
                                .text_size(px(16.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child("Auto Supports (Beta) Settings"),
                        )
                        .child(
                            div()
                                .mt(px(2.0))
                                .text_size(px(12.0))
                                .line_height(relative(1.375))
                                .text_color(rgb(TEXT_MUTED))
                                .child("Detected surfaces, density and sizing, stability, and the saved presets a run follows"),
                        ),
                ),
        )
        .child(
            icon_button(
                ElementId::Name("as-close".into()),
                "x",
                IconButtonVariant::Solid,
                IconButtonSize::Sm,
            )
            .on_click(move |_, _, cx| close(&close_entity, cx)),
        )
}

// ── The dialog ──────────────────────────────────────────────────────────────

/// The Auto Supports settings dialog.
///
/// `open` mirrors the shell's `show_support_settings` (the panel's gear sets it);
/// the close tile and the backdrop clear it. Escape is the shell's key listener,
/// which does not know this flag yet.
pub fn auto_support_settings(open: bool, entity: WeakEntity<DragonFruit>) -> impl IntoElement {
    if !open {
        return div();
    }

    let (
        draft,
        presets,
        active,
        preset_menu_open,
        sizing_menu_open,
        origin_colors,
        skip_auto_bracing,
        debug_simple_render,
        debug_mode,
        pending_delete,
        name_dialog_state,
        saved,
        is_built_in,
        has_changes,
        modified,
        active_name,
    ) = {
        let guard = match state().lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let has_changes = guard.has_changes();
        (
            guard.draft,
            guard.presets.clone(),
            guard.active.clone(),
            guard.preset_menu_open,
            guard.sizing_menu_open,
            guard.origin_colors,
            guard.skip_auto_bracing,
            guard.debug_simple_render,
            guard.debug_mode,
            guard.pending_delete,
            guard.name_dialog.as_ref().map(|dialog| NameDialog {
                mode: dialog.mode,
                name: dialog.name.clone(),
            }),
            guard.saved,
            guard.is_built_in(),
            has_changes,
            has_changes,
            guard
                .active_preset()
                .map(|preset| preset.name.to_string()),
        )
    };

    let locked = is_built_in;

    // ── Body ────────────────────────────────────────────────────────────────
    let mut scroll = div()
        .id("as-settings-scroll")
        .flex()
        .flex_col()
        .gap(px(12.0))
        .p(px(16.0))
        .flex_1()
        .min_h(px(0.0))
        .overflow_y_scroll()
        .child(preset_strip(
            &presets,
            active.as_ref(),
            modified,
            preset_menu_open,
            is_built_in,
            &entity,
        ));

    let mut grid = div().flex().flex_col().gap(px(12.0));
    let mut policy_cards: Vec<Div> = Vec::new();
    for section in POLICY_SECTIONS.iter() {
        let mut cells: Vec<Cell> = Vec::new();
        for def in TOGGLES.iter().filter(|def| def.section == section.key) {
            cells.push(Cell::Half(toggle_field(
                def,
                draft.toggle(def.key),
                locked,
                ToggleAction::Draft(def.key),
                &entity,
            )));
        }
        if section.key == SectionKey::Density {
            cells.push(Cell::Full(sizing_tier_field(
                draft.sizing_preset,
                locked,
                sizing_menu_open,
                &entity,
            )));
        }
        for def in KNOBS.iter().filter(|def| def.section == section.key) {
            cells.push(Cell::Half(number_field(
                def,
                draft.knob(def.key),
                locked,
                &entity,
            )));
        }
        policy_cards.push(field_card(section, cells));
    }

    // Six cards fill three rows exactly — the five policy cards and
    // Diagnostics — and the calibration card spans both columns below them.
    let mut cards = policy_cards.into_iter().peekable();
    while let Some(first) = cards.next() {
        let second = if cards.peek().is_some() {
            cards.next().expect("paired card")
        } else {
            diagnostics_card(
                origin_colors,
                skip_auto_bracing,
                debug_simple_render,
                debug_mode,
                &entity,
            )
        };
        grid = grid.child(two_cards(first, second));
    }
    grid = grid.child(
        div()
            .flex()
            .items_stretch()
            .child(advanced_card(&draft, locked, &entity).flex_1().min_w(px(0.0))),
    );

    scroll = scroll.child(grid);

    let body = div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(0.0))
        .child(scroll)
        .child(footer(
            active.is_some(),
            is_built_in,
            has_changes,
            saved,
            &entity,
        ));

    // ── Panel ───────────────────────────────────────────────────────────────
    let panel = div()
        .w_full()
        .max_w(px(1120.0))
        .flex()
        .flex_col()
        .rounded(px(ROUNDED_XL))
        .border_1()
        .border_color(rgb(BORDER_STRONG))
        .bg(rgb(SURFACE_0))
        .overflow_hidden()
        .shadow(vec![shadow(
            0.0,
            24.0,
            46.0,
            0.0,
            with_alpha(BLACK, 0x6b),
        )])
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(dialog_header(&entity))
        .child(body);

    // ── Backdrop ────────────────────────────────────────────────────────────
    let backdrop_entity = entity.clone();
    let mut layer = div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .size_full()
        .p(px(20.0))
        .flex()
        .items_stretch()
        .justify_center()
        // `bg-black/58`.
        .bg(rgba(with_alpha(BLACK, 0x94)))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            close(&backdrop_entity, cx);
        })
        .child(panel);

    if pending_delete {
        if let Some(name) = active_name.as_deref() {
            layer = layer.child(delete_dialog(name, &entity));
        }
    }
    if let Some(dialog) = name_dialog_state.as_ref() {
        layer = layer.child(name_dialog(dialog, &entity));
    }

    layer
}
