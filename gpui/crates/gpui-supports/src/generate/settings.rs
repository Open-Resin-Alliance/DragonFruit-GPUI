//! The knobs the contact-placement stage reads.
//!
//! One struct, `SupportSettings`, is the slice of the app's `AutoSupportSettings`
//! (`src/supports/autoSupport/settings.ts`) that contact placement consumes. The
//! defaults are `createDefaultAutoSupportSettings()`'s, taken through
//! `AUTO_SUPPORT_CONSTRAINTS[*].defaultValue`, so a caller that has no stored
//! block gets the same contacts the React app does.
//!
//! # PORT DEVIATIONS
//!
//! - **Only the stage-1 knobs are here.** `sizingPreset`, the leaf-fan knobs,
//!   the caps and the two diagnostics switches drive later stages (placement,
//!   sizing, fanning, bracing) and are not part of contact placement.
//! - **`sizeScale` is carried but not read.** It scales the preset sizing bands
//!   (`sizeParameters`), which is candidate *sizing*; this stage only places the
//!   contacts those bands will size.

/// The auto-support knobs contact placement reads, with
/// `createDefaultAutoSupportSettings()`'s defaults.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SupportSettings {
    /// `enabled` — a disabled block runs nothing.
    pub enabled: bool,
    /// `minIslandAreaMm2` — islands below this projected area are dropped.
    pub min_island_area_mm2: f32,
    /// `tipInfluenceRadiusMm` — the candidate dedupe's suppression radius.
    pub tip_influence_radius_mm: f32,
    /// `areaPerSupportMm2` — projected surface area each grid support carries.
    pub area_per_support_mm2: f32,
    /// `gridAreaThresholdMm2` — at/above this projected area a region gets the
    /// density grid; smaller regions get the single-pillar contact.
    pub grid_area_threshold_mm2: f32,
    /// `overhangSelfSupportAngleDeg` — surface angle from horizontal at and
    /// below which a face counts as an overhang. Also classifier input.
    pub overhang_self_support_angle_deg: f32,
    /// `sizeScale` — master multiplier over the preset sizing bands. Not read
    /// by this stage; carried so one struct describes the block.
    pub size_scale: f32,
    /// `flatDensityBoost` — grid spacing multiplier on flat ceilings (0°).
    pub flat_density_boost: f32,
    /// `slopeRelaxFactor` — grid spacing multiplier at the self-support angle.
    pub slope_relax_factor: f32,
    /// `suctionAreaExponent` — area-scaling exponent for flat-region density.
    pub suction_area_exponent: f32,
}

impl Default for SupportSettings {
    /// `createDefaultAutoSupportSettings()` — every value is that block's
    /// `AUTO_SUPPORT_CONSTRAINTS[key].defaultValue`.
    fn default() -> Self {
        Self {
            enabled: true,
            min_island_area_mm2: 0.02,
            tip_influence_radius_mm: 0.5,
            area_per_support_mm2: 10.0,
            grid_area_threshold_mm2: 25.0,
            overhang_self_support_angle_deg: 45.0,
            size_scale: 1.0,
            flat_density_boost: 0.7,
            slope_relax_factor: 1.3,
            suction_area_exponent: 0.15,
        }
    }
}
