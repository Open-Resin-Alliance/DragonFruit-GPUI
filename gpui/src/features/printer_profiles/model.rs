//! The Printer & Material Profiles model: `PrinterProfile`, `PrinterPreset`,
//! `MaterialProfile` and the process-wide store the modal's rail and its panes
//! read and write.
//!
//! Ported from `src/components/settings/ProfileSettingsModal.tsx` (the
//! `Printer & Material Profiles` modal) and the store it subscribes to,
//! `src/features/profiles/profileStore.ts`. The field set of every type is the
//! source's own declaration, in the source's order, with `serde`'s
//! `rename_all = "camelCase"` and `default` so a profile or preset round-trips
//! through the shape the source persists (`dragonfruit.profiles.v1`, the
//! profile bundle payload) and the shape the built-in catalogue ships.
//!
//! # Where the profiles come from
//!
//! The presets are the same two JSON files `features::printer_picker` loads:
//!
//! * `profiles/printers/generic/printers.json` — the one `generic-machine`
//!   reference preset;
//! * `plugins/athena/printers/printers.json` — the built-in Athena profile pack
//!   (the 3-bit / 8-bit variant family).
//!
//! Installed profiles are not modelled from scratch: `features::printer_picker`
//! owns the add flow and the profile store in this port, so [`sync`] adopts every
//! profile the picker has installed (matched by its id) and builds the full
//! field set from the preset the profile was created from, exactly as
//! `addPrinterProfileFromPreset` does. Profiles the picker does not know — the
//! custom copies [`duplicate_as_custom`] mints — stay in this store's list.
//!
//! # PORT DEVIATIONS
//!
//! 1. No `localStorage`: the source persists profiles, the active material and
//!    the manual build dimensions. Here the store is process-wide and in memory,
//!    like `features::settings` and `features::printer_picker`.
//! 2. `plugin`-contributed presets, `missingPlugin` marking and the official
//!    profile update diff are not produced. `missing_plugin` is modelled so a
//!    future adapter can set it; nothing in this port does.
//! 3. Plugin material adapters (`getProfileLocalMaterialSettingsAdapter`) have no
//!    port, so `local_settings_by_output` is carried as raw JSON
//!    (`serde_json::Value`) and nothing resolves per-output defaults.
//! 4. `features::printer_picker`'s store still mints `printer-<n>` ids for the
//!    presets it installs; this store adopts them. Custom copies mint
//!    `printer-custom-<n>` because `createId`'s random suffix is not reproduced,
//!    and the picker's store does not know them — so the top bar's active-printer
//!    badge keeps the last official profile when a custom copy is selected here.
//!    The modal itself selects the custom copy correctly.
//! 5. The picker's store keeps its own minimal profile and material records for
//!    the library surface; this store keeps the full field set and the material
//!    rows the modal edits. The two agree on identity and order (this store
//!    mirrors selection, moves and removals through the picker's seam functions)
//!    but are not one record.

use std::sync::{LazyLock, Mutex, MutexGuard};

use serde::{Deserialize, Serialize};

use crate::features::printer_picker;

// ── The source's value shapes ───────────────────────────────────────────────

/// `PrinterBitDepth` (`profileStore.ts:71`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BitDepth {
    pub bits: f32,
    pub description: Option<String>,
}

/// `PrinterPlatformBadge`: the NanoDLP / Odyssey chip (`{ text, color }`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PlatformBadge {
    pub text: String,
    pub color: Option<String>,
}

/// `PrinterPixelSize`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PixelSize {
    pub x: f32,
    pub y: f32,
}

/// `PrinterProfile.buildVolumeMm` / `PrinterPreset.buildVolumeMm`.
///
/// The Athena presets carry `null` for width and depth
/// (`buildDimensionMode: auto` derives them from `pixelSize` × `display`), so
/// the axes are optional exactly as the JSON is; use
/// [`PrinterProfile::effective_width_mm`] for a concrete number.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BuildVolumeMm {
    pub width: Option<f32>,
    pub depth: Option<f32>,
    pub height: Option<f32>,
}

/// `safetyMarginMm`: the four non-negative edges.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SafetyMarginMm {
    pub front: f32,
    pub back: f32,
    pub left: f32,
    pub right: f32,
}

/// `PrinterProfile.display` / `PrinterPreset.display`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DisplaySettings {
    pub resolution_x: f32,
    pub resolution_y: f32,
    /// `PrinterOutputFormat` — a plugin-extensible string (`.svgx`, `.nanodlp`,
    /// …). The source normalises it to lower case.
    pub output_format: String,
    pub format_version: Option<String>,
    pub settings_mode: Option<String>,
    pub webcam_rotation_deg: Option<u32>,
    pub mirror_x: Option<bool>,
    pub mirror_y: Option<bool>,
}

/// `PrinterNetworkSettings`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NetworkSettings {
    pub discovery_enabled: bool,
    pub ip_address: String,
}

impl Default for NetworkSettings {
    /// `DEFAULT_PRINTER_NETWORK_SETTINGS` (`profileStore.ts:726`):
    /// `{ discoveryEnabled: true, ipAddress: '' }`.
    fn default() -> Self {
        Self {
            discovery_enabled: true,
            ip_address: String::new(),
        }
    }
}

/// `PrinterNetworkConnectionState`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NetworkConnection {
    pub mode: String,
    pub connected: bool,
    pub host_name: String,
    pub ip_address: String,
    pub port: u32,
    pub last_checked_at: String,
    pub status_text: Option<String>,
    pub selected_material_id: Option<String>,
    pub selected_material_name: Option<String>,
    pub selected_material_layer_height_mm: Option<f32>,
    pub selected_material_normal_exposure_sec: Option<f32>,
    pub selected_material_bottom_exposure_sec: Option<f32>,
    pub selected_material_bottom_layer_count: Option<f32>,
}

/// `PrinterNetworkDevice` — one fleet unit: a connection state plus its own
/// identity and artwork.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NetworkDevice {
    pub id: String,
    pub display_name: String,
    pub image_data_url: Option<String>,
    pub mode: String,
    pub connected: bool,
    pub host_name: String,
    pub ip_address: String,
    pub port: u32,
    pub last_checked_at: String,
    pub status_text: Option<String>,
}

/// `PrinterProfile.missingPlugin`: set when the plugin that contributed the
/// profile's preset was removed. The rail greys the card out.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MissingPlugin {
    pub id: String,
    pub name: String,
}

// ── Material anti-aliasing ──────────────────────────────────────────────────

/// `MaterialAntiAliasingSettings` (`profileStore.ts:271`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AntiAliasingSettings {
    pub enable_custom_settings: bool,
    pub enable_override: bool,
    /// `'Off' | 'Blur' | '3DAA'`.
    pub mode: String,
    pub level: String,
    pub use_custom_level: bool,
    pub blur_brush_radius_px: f32,
    pub use_custom_blur_brush_radius: bool,
    /// `'box' | 'gaussian'`.
    pub blur_brush_kernel: String,
    pub blur_brush_sigma_x: f32,
    pub blur_brush_sigma_y: f32,
    pub z_blur_radius_layers: f32,
    pub use_custom_z_blur_radius: bool,
    /// `'box' | 'gaussian'`.
    pub z_blur_kernel: String,
    pub z_blur_sigma: f32,
    pub z_blend_look_back: f32,
    pub use_custom_z_blend_look_back: bool,
    pub z_blend_auto_mode: bool,
    /// `'uniform' | 'halton' | 'base2'`.
    pub zaa_pattern: String,
    pub zaa_duplicate_z: bool,
    /// `'minimum' | 'lut'`.
    pub blur_gray_source_mode: String,
    /// `'opaque' | 'clear' | 'custom'`.
    pub z_blend_resin_type: String,
    pub selected_lut_curve_id: String,
    pub aa_on_supports: bool,
    pub dither_enabled: bool,
    pub dither_bit_depth: u32,
    pub dither_device_gamma: f32,
    /// `'disabled' | 'auto' | 'manual'`.
    pub tip_offset_mode: String,
    pub tip_offset_mm: f32,
    pub tip_offset_display_in_ui: bool,
    pub support_tip_shrink_percent: f32,
}

impl Default for AntiAliasingSettings {
    /// `DEFAULT_MATERIAL_ANTI_ALIASING_SETTINGS` (`profileStore.ts:302`), value
    /// for value.
    fn default() -> Self {
        Self {
            enable_custom_settings: false,
            enable_override: false,
            mode: "Blur".to_owned(),
            level: "4x".to_owned(),
            use_custom_level: false,
            blur_brush_radius_px: 1.0,
            use_custom_blur_brush_radius: false,
            blur_brush_kernel: "gaussian".to_owned(),
            blur_brush_sigma_x: 0.5,
            blur_brush_sigma_y: 0.5,
            z_blur_radius_layers: 0.0,
            use_custom_z_blur_radius: false,
            z_blur_kernel: "box".to_owned(),
            z_blur_sigma: 0.5,
            z_blend_look_back: 2.0,
            use_custom_z_blend_look_back: false,
            z_blend_auto_mode: true,
            zaa_pattern: "halton".to_owned(),
            zaa_duplicate_z: true,
            blur_gray_source_mode: "lut".to_owned(),
            z_blend_resin_type: "opaque".to_owned(),
            selected_lut_curve_id: "default".to_owned(),
            aa_on_supports: false,
            dither_enabled: false,
            dither_bit_depth: 8,
            dither_device_gamma: 2.2,
            tip_offset_mode: "auto".to_owned(),
            tip_offset_mm: 0.05,
            tip_offset_display_in_ui: false,
            support_tip_shrink_percent: 10.0,
        }
    }
}

/// `scaleCompensationPct`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ScaleCompensationPct {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

// ── MaterialProfile ─────────────────────────────────────────────────────────

/// `MaterialProfile` (`profileStore.ts`, `addMaterialProfile`'s literal).
///
/// `anti_aliasing_settings` is the full source type;
/// `local_settings_by_output` is the plugin adapter's per-output blob, carried
/// as raw JSON because no adapter is ported (deviation 3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MaterialProfile {
    pub id: String,
    pub printer_profile_id: String,
    pub official_template_id: Option<String>,
    pub official_template_version: Option<u32>,
    /// An official profile's material cannot be edited in place.
    pub locked: bool,
    pub name: String,
    pub brand: String,
    /// `#rrggbb`, lower-cased (`sanitizeMaterialColor`).
    pub color: Option<String>,
    pub currency_code: String,
    pub bottle_price: f32,
    pub bottle_capacity_ml: f32,
    pub resin_family: String,
    pub scale_compensation_pct: ScaleCompensationPct,
    pub layer_height_mm: f32,
    pub normal_exposure_sec: f32,
    pub bottom_exposure_sec: f32,
    pub bottom_layer_count: f32,
    pub lift_distance_mm: f32,
    pub lift_speed_mm_min: f32,
    pub retract_speed_mm_min: f32,
    pub minimum_aa_alpha_percent: f32,
    pub anti_aliasing_settings: AntiAliasingSettings,
    pub local_settings_by_output: serde_json::Value,
    /// Printer profiles (other than the owner) the material is shared with.
    pub compatible_printer_profile_ids: Vec<String>,
}

impl Default for MaterialProfile {
    fn default() -> Self {
        // `addMaterialProfile`'s defaults, with the identity left empty for the
        // caller to fill (`Material N`, `Default`, `USD`, …).
        Self {
            id: String::new(),
            printer_profile_id: String::new(),
            official_template_id: None,
            official_template_version: None,
            locked: false,
            name: String::new(),
            brand: "Default".to_owned(),
            color: None,
            currency_code: "USD".to_owned(),
            bottle_price: 0.0,
            bottle_capacity_ml: 1000.0,
            resin_family: "standard".to_owned(),
            scale_compensation_pct: ScaleCompensationPct::default(),
            layer_height_mm: 0.05,
            normal_exposure_sec: 2.5,
            bottom_exposure_sec: 28.0,
            bottom_layer_count: 5.0,
            lift_distance_mm: 6.0,
            lift_speed_mm_min: 60.0,
            retract_speed_mm_min: 150.0,
            minimum_aa_alpha_percent: 35.0,
            anti_aliasing_settings: AntiAliasingSettings::default(),
            local_settings_by_output: serde_json::Value::Null,
            compatible_printer_profile_ids: Vec::new(),
        }
    }
}

impl MaterialProfile {
    /// `ensureActiveMaterialForActivePrinter`'s `createdMaterial`
    /// (`profileStore.ts:1621`): the profile the source mints for a printer
    /// that has none. `id` is stable per printer in this port.
    pub fn standard(printer_profile_id: &str) -> Self {
        Self {
            id: format!("{printer_profile_id}-material-standard-405"),
            printer_profile_id: printer_profile_id.to_owned(),
            name: "Standard 405nm".to_owned(),
            brand: "Default".to_owned(),
            bottle_price: 24.99,
            ..Self::default()
        }
    }

    /// `materialProfileIsAvailableForPrinter` (`profileStore.ts:398`).
    pub fn available_for(&self, printer_profile_id: &str) -> bool {
        self.printer_profile_id == printer_profile_id
            || self
                .compatible_printer_profile_ids
                .iter()
                .any(|id| id == printer_profile_id)
    }

    /// `fullMaterialName`: the resin family label prefixes the name when it
    /// does not already, then the brand when that is not a prefix either.
    pub fn full_name(&self) -> String {
        let name = self.name.trim();
        let brand = self.brand.trim();
        let family = resin_family_label(&self.resin_family);
        let name_with_family =
            match family {
                Some(label) if !name.to_ascii_lowercase().starts_with(&label.to_ascii_lowercase()) => {
                    format!("{label} {name}")
                }
                _ => name.to_owned(),
            };
        if brand.is_empty() || name_with_family.to_ascii_lowercase().starts_with(&brand.to_ascii_lowercase()) {
            return name_with_family;
        }
        format!("{brand} {name_with_family}")
    }
}

/// `RESIN_FAMILY_OPTIONS`' label for a stored family value
/// (`profileFormAtoms.ts`). Only the families the modal's own drafts use are
/// listed; an unknown plugin family keeps its raw value.
fn resin_family_label(value: &str) -> Option<&'static str> {
    match value {
        "standard" => Some("Standard"),
        "absLike" | "abs-like" => Some("ABS-Like"),
        "waterWashable" | "water-washable" => Some("Water Washable"),
        "highTemp" | "high-temp" => Some("High Temp"),
        "flexible" => Some("Flexible"),
        "castable" => Some("Castable"),
        "tough" => Some("Tough"),
        "biocompatible" => Some("Biocompatible"),
        _ => None,
    }
}

// ── PrinterProfile ──────────────────────────────────────────────────────────

/// `PrinterProfile` (`profileStore.ts:153`) — every field the source declares,
/// so the editor's panes and the source's bundle payload both round-trip.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PrinterProfile {
    pub id: String,
    pub name: String,
    pub manufacturer: Option<String>,
    /// `imageDataUrl`: uploaded artwork, or the preset's `imageAssetPath`.
    pub image_data_url: Option<String>,
    pub anti_aliasing: Option<bool>,
    /// `PrinterNetworkSupport` — `nanodlp`, `odyssey`, or a plugin mode.
    pub network_support: Option<String>,
    pub has_camera: Option<bool>,
    pub network_filter: Option<String>,
    pub platform_badge: Option<PlatformBadge>,
    pub pixel_size: Option<PixelSize>,
    pub bit_depth: Option<BitDepth>,
    /// `'manual' | 'auto'`.
    pub build_dimension_mode: Option<String>,
    /// `'single' | 'multiTank'`.
    pub material_mode: Option<String>,
    /// How many tanks a Multi-Tank machine carries (2..16).
    pub tank_count: Option<u32>,
    /// `allowMultiPlate`: absent means true.
    pub allow_multi_plate: Option<bool>,
    /// `'rectangle' | 'circle'`.
    pub plate_shape: Option<String>,
    /// Which material sits in each tank, by index; `''` is unassigned. Trimmed
    /// to `tank_count` by the UI and pruned when a material is removed.
    pub tank_materials: Vec<String>,
    /// Free-text notes the user keeps with this machine; never part of a preset.
    pub notes: Option<String>,
    pub official_preset_id: Option<String>,
    pub official_preset_version: Option<u32>,
    pub missing_plugin: Option<MissingPlugin>,
    /// The user chose "None" for network support on a profile whose preset has
    /// one (an absent `network_support` alone cannot express that).
    pub network_support_disabled: bool,
    pub is_official: bool,
    pub is_custom: bool,
    pub safety_margin_mm: Option<SafetyMarginMm>,
    pub build_volume_mm: BuildVolumeMm,
    pub display: DisplaySettings,
    pub network: Option<NetworkSettings>,
    pub network_fleet: Vec<NetworkDevice>,
    pub active_network_device_id: Option<String>,
    pub network_connection: Option<NetworkConnection>,
}

impl PrinterProfile {
    /// `(printer.manufacturer ?? '').toLowerCase() === 'generic' ||
    /// printer.name.toLowerCase().includes('generic')`.
    pub fn is_generic(&self) -> bool {
        self.manufacturer
            .as_deref()
            .is_some_and(|manufacturer| manufacturer.eq_ignore_ascii_case("generic"))
            || self.name.to_ascii_lowercase().contains("generic")
    }

    /// `bitDepthBits`: `Math.round(bitDepth.bits)`.
    pub fn bit_depth_bits(&self) -> Option<u32> {
        self.bit_depth
            .as_ref()
            .map(|depth| depth.bits.round().max(0.0) as u32)
    }

    /// `bitDepthLabel`: `'<n> Bit'`, and nothing for an 8-bit display.
    pub fn bit_depth_label(&self) -> Option<String> {
        self.bit_depth_bits()
            .filter(|bits| *bits != 8)
            .map(|bits| format!("{bits} Bit"))
    }

    /// `cardBadgeText` before the update badge: `CUSTOM` for a custom profile,
    /// else the preset's platform chip.
    pub fn card_badge_text(&self) -> Option<String> {
        if self.is_custom {
            return Some("CUSTOM".to_owned());
        }
        self.platform_badge
            .as_ref()
            .map(|badge| badge.text.trim().to_owned())
            .filter(|text| !text.is_empty())
    }

    /// `buildDimensionMode === 'auto'`.
    pub fn auto_build_dimensions(&self) -> bool {
        self.build_dimension_mode.as_deref() == Some("auto")
    }

    /// `buildVolumeMm.width`, derived from `pixelSize` × `resolutionX` when the
    /// profile is in auto mode (`applyAutoBuildDimensions`).
    pub fn effective_width_mm(&self) -> f32 {
        if self.auto_build_dimensions() {
            if let Some(pixel_size) = self.pixel_size {
                return compute_build_dimension_mm(self.display.resolution_x, pixel_size.x);
            }
        }
        self.build_volume_mm.width.unwrap_or(0.0)
    }

    /// `buildVolumeMm.depth`, derived from `pixelSize` × `resolutionY` in auto
    /// mode.
    pub fn effective_depth_mm(&self) -> f32 {
        if self.auto_build_dimensions() {
            if let Some(pixel_size) = self.pixel_size {
                return compute_build_dimension_mm(self.display.resolution_y, pixel_size.y);
            }
        }
        self.build_volume_mm.depth.unwrap_or(0.0)
    }

    /// `buildVolumeMm.height`.
    pub fn height_mm(&self) -> f32 {
        self.build_volume_mm.height.unwrap_or(0.0)
    }

    /// `tankCount ?? 2`, the Multi-Tank UI's default.
    pub fn tank_count_or_default(&self) -> u32 {
        self.tank_count.unwrap_or(2)
    }

    /// `materialMode === 'multiTank'`.
    pub fn multi_tank(&self) -> bool {
        self.material_mode.as_deref() == Some("multiTank")
    }
}

/// `computeBuildDimensionMm` (`ProfileSettingsModal.tsx:339`):
/// `round(resolution) * max(0.001, pixelSizeUm) / 1000`, rounded to three
/// decimals.
pub fn compute_build_dimension_mm(resolution_px: f32, pixel_size_um: f32) -> f32 {
    let safe_resolution = resolution_px.round().max(1.0);
    let safe_pixel_size = if pixel_size_um.is_finite() && pixel_size_um > 0.0 {
        pixel_size_um.max(0.001)
    } else {
        0.001
    };
    // `Number((value).toFixed(3))`.
    let value = (safe_resolution * safe_pixel_size) / 1000.0;
    (value * 1000.0).round() / 1000.0
}

// ── PrinterPreset ───────────────────────────────────────────────────────────

/// `PrinterPreset` (`profileStore.ts:80`): one catalogue entry.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PrinterPreset {
    pub preset_id: String,
    pub profile_version: Option<u32>,
    pub manufacturer: String,
    pub name: String,
    pub family: Option<String>,
    pub image_asset_path: Option<String>,
    pub anti_aliasing: Option<bool>,
    pub network_support: Option<String>,
    pub has_camera: Option<bool>,
    pub network_filter: Option<String>,
    pub platform_badge: Option<PlatformBadge>,
    pub pixel_size: Option<PixelSize>,
    pub bit_depth: Option<BitDepth>,
    /// `'manual' | 'auto'`.
    pub build_dimension_mode: Option<String>,
    /// `'single' | 'multiTank'`.
    pub material_mode: Option<String>,
    pub tank_count: Option<u32>,
    pub allow_multi_plate: Option<bool>,
    /// `'rectangle' | 'circle'`.
    pub plate_shape: Option<String>,
    /// Concrete variant preset ids of a family preset.
    pub model_variants: Vec<String>,
    /// The cleaner library card name ("Athena II 16K" for "… · 8-bit").
    pub library_display_name: Option<String>,
    /// HTTP path a plugin adapter probes to auto-detect the variant.
    pub model_variant_detect_path: Option<String>,
    /// A hidden variant: not a standalone library card.
    pub is_model_variant: Option<bool>,
    pub build_volume_mm: BuildVolumeMm,
    pub safety_margin_mm: Option<SafetyMarginMm>,
    pub display: DisplaySettings,
}

impl PrinterPreset {
    /// `libraryDisplayName ?? name`.
    pub fn library_name(&self) -> String {
        self.library_display_name
            .clone()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| self.name.clone())
    }

    /// `preset.modelVariants?.length || preset.modelVariantDetectPath`.
    pub fn opens_setup(&self) -> bool {
        !self.model_variants.is_empty()
            || self
                .model_variant_detect_path
                .as_ref()
                .is_some_and(|path| !path.trim().is_empty())
    }

    pub fn is_generic(&self) -> bool {
        self.manufacturer.eq_ignore_ascii_case("generic")
            || self.name.to_ascii_lowercase().contains("generic")
    }

    /// `addPrinterProfileFromPreset`'s `addPrinterProfile({ … })`
    /// (`profileStore.ts:1971`): the profile an install of this preset creates.
    pub fn to_profile(&self, id: impl Into<String>) -> PrinterProfile {
        PrinterProfile {
            id: id.into(),
            name: self.name.clone(),
            manufacturer: Some(self.manufacturer.clone()),
            image_data_url: self.image_asset_path.clone(),
            anti_aliasing: self.anti_aliasing,
            network_support: self.network_support.clone(),
            has_camera: self.has_camera,
            network_filter: self.network_filter.clone(),
            platform_badge: self.platform_badge.clone(),
            pixel_size: self.pixel_size,
            bit_depth: self.bit_depth.clone(),
            build_dimension_mode: Some(
                self.build_dimension_mode
                    .clone()
                    .unwrap_or_else(|| "manual".to_owned()),
            ),
            material_mode: self.material_mode.clone(),
            tank_count: self.tank_count,
            allow_multi_plate: self.allow_multi_plate,
            plate_shape: self.plate_shape.clone(),
            tank_materials: Vec::new(),
            notes: None,
            official_preset_id: Some(self.preset_id.clone()),
            official_preset_version: Some(self.profile_version.unwrap_or(1)),
            missing_plugin: None,
            network_support_disabled: false,
            is_official: true,
            is_custom: false,
            safety_margin_mm: self.safety_margin_mm,
            build_volume_mm: self.build_volume_mm,
            display: self.display.clone(),
            // `addPrinterProfile` runs every profile through
            // `sanitizePrinterNetworkSettings`, which fills the defaults.
            network: Some(NetworkSettings::default()),
            network_fleet: Vec::new(),
            active_network_device_id: None,
            network_connection: None,
        }
    }
}

fn parse_presets(json: &str) -> Vec<PrinterPreset> {
    serde_json::from_str(json).unwrap_or_else(|error| {
        eprintln!("printer profiles: built-in preset catalogue did not parse: {error}");
        Vec::new()
    })
}

/// The same two catalogues `features::printer_picker` seeds from.
fn catalogue() -> Vec<PrinterPreset> {
    let mut presets = parse_presets(include_str!(
        "../../../../profiles/printers/generic/printers.json"
    ));
    presets.extend(parse_presets(include_str!(
        "../../../../plugins/athena/printers/printers.json"
    )));
    presets
}

// ── The store ───────────────────────────────────────────────────────────────

struct Store {
    presets: Vec<PrinterPreset>,
    /// In rail order. The order is this store's; the picker's own list follows
    /// the same moves because [`move_profile`] mirrors them.
    profiles: Vec<PrinterProfile>,
    materials: Vec<MaterialProfile>,
    selected: Option<String>,
    active_material: Option<String>,
    /// `isEditingPrinter`: the nested `Printer Profile Settings` card.
    editing: bool,
    /// `deleteConfirmTarget` when `kind === 'printer'`: `(id, name)`.
    pending_remove: Option<(String, String)>,
    /// `manualBuildDimensionsByPrinterId`: width/depth to restore when a
    /// profile leaves auto build-dimension mode.
    manual_dimensions: Vec<(String, (Option<f32>, Option<f32>))>,
    /// The counter behind `material-<n>` for user-created materials.
    next_material: usize,
    /// The counter behind `printer-custom-<n>` for custom copies.
    next_custom: usize,
}

impl Store {
    fn seed() -> Self {
        Self {
            presets: catalogue(),
            profiles: Vec::new(),
            materials: Vec::new(),
            selected: None,
            active_material: None,
            editing: false,
            pending_remove: None,
            manual_dimensions: Vec::new(),
            next_material: 1,
            next_custom: 1,
        }
    }

    fn next_material_id(&mut self) -> String {
        let id = format!("material-{}", self.next_material);
        self.next_material += 1;
        id
    }

    fn next_custom_id(&mut self) -> String {
        let id = format!("printer-custom-{}", self.next_custom);
        self.next_custom += 1;
        id
    }

    fn preset(&self, preset_id: &str) -> Option<&PrinterPreset> {
        self.presets
            .iter()
            .find(|preset| preset.preset_id == preset_id)
    }

    fn profile_index(&self, id: &str) -> Option<usize> {
        self.profiles.iter().position(|profile| profile.id == id)
    }

    /// Adopt every profile the picker has installed; keep this store's own
    /// profiles (custom copies) untouched.
    fn sync(&mut self) {
        for installed in printer_picker::installed_profiles() {
            if let Some(index) = self.profile_index(&installed.id) {
                self.profiles[index].official_preset_id =
                    Some(installed.official_preset_id.clone());
                continue;
            }
            let Some(preset) = self
                .presets
                .iter()
                .find(|preset| preset.preset_id == installed.official_preset_id)
                .cloned()
            else {
                continue;
            };
            let id = installed.id.clone();
            self.profiles.push(preset.to_profile(id.clone()));
            // `addPrinterProfile` ends in `ensureActiveMaterialForActivePrinter`.
            if !self
                .materials
                .iter()
                .any(|material| material.printer_profile_id == id)
            {
                self.materials.push(MaterialProfile::standard(&id));
            }
        }

        // The modal's selection follows the store's active printer
        // (`handlePickPrinter` writes both), except when the active profile is
        // this store's own custom copy, which the picker does not know.
        if let Some(active) = printer_picker::active_profile_id() {
            if self.profile_index(&active).is_some() {
                self.selected = Some(active);
            }
        }
        if self
            .selected
            .as_ref()
            .is_some_and(|id| self.profile_index(id).is_none())
        {
            self.selected = self.profiles.first().map(|profile| profile.id.clone());
        }
        self.ensure_materials();
    }

    /// Drop material rows whose printer is gone, then make sure the selected
    /// printer has a material and a valid active one
    /// (`ensureActiveMaterialForActivePrinter`).
    fn ensure_materials(&mut self) {
        let printer_ids: Vec<String> = self
            .profiles
            .iter()
            .map(|profile| profile.id.clone())
            .collect();
        self.materials
            .retain(|material| printer_ids.iter().any(|id| id == &material.printer_profile_id));

        let Some(selected) = self.selected.clone() else {
            self.active_material = None;
            return;
        };

        if !self
            .materials
            .iter()
            .any(|material| material.printer_profile_id == selected)
        {
            self.materials.push(MaterialProfile::standard(&selected));
        }

        let valid = self.active_material.as_ref().is_some_and(|id| {
            self.materials
                .iter()
                .any(|material| &material.id == id && material.printer_profile_id == selected)
        });
        if !valid {
            self.active_material = self
                .materials
                .iter()
                .find(|material| material.printer_profile_id == selected)
                .map(|material| material.id.clone());
        }
    }
}

static STORE: LazyLock<Mutex<Store>> = LazyLock::new(|| Mutex::new(Store::seed()));

fn store() -> MutexGuard<'static, Store> {
    STORE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Adopt the picker's installed profiles first, then run `f`. A write is
/// therefore visible to the next read without a second sync.
fn edit<R>(f: impl FnOnce(&mut Store) -> R) -> R {
    let mut guard = store();
    guard.sync();
    f(&mut guard)
}

// ── Reading ─────────────────────────────────────────────────────────────────

/// The printer rail's rows, in order.
pub fn profiles() -> Vec<PrinterProfile> {
    edit(|store| store.profiles.clone())
}

/// One profile by id.
pub fn profile(id: &str) -> Option<PrinterProfile> {
    edit(|store| {
        let store = &*store;
        store.profiles.iter().find(|profile| profile.id == id).cloned()
    })
}

/// `selectedPrinter`: the modal's current printer.
pub fn selected_profile() -> Option<PrinterProfile> {
    edit(|store| {
        let store = &*store;
        store
            .selected
            .as_deref()
            .and_then(|id| store.profiles.iter().find(|profile| profile.id == id))
            .cloned()
    })
}

pub fn selected_id() -> Option<String> {
    edit(|store| store.selected.clone())
}

/// The printer rail's index of the selected printer (`selectedPrinterRailIndex`).
pub fn selected_index() -> Option<usize> {
    edit(|store| {
        let store = &*store;
        store
            .selected
            .as_deref()
            .and_then(|id| store.profile_index(id))
    })
}

/// `isEditingPrinter`.
pub fn editing() -> bool {
    edit(|store| store.editing)
}

/// `deleteConfirmTarget?.kind === 'printer'`: `(id, name)`.
pub fn pending_remove() -> Option<(String, String)> {
    edit(|store| store.pending_remove.clone())
}

/// The catalogue, for a preset-driven pane.
pub fn presets() -> Vec<PrinterPreset> {
    edit(|store| store.presets.clone())
}

/// The preset a profile was created from (`resolveOfficialPresetIdFromProfile`).
pub fn preset_for(profile: &PrinterProfile) -> Option<PrinterPreset> {
    let preset_id = profile
        .official_preset_id
        .clone()
        .or_else(|| {
            profile
                .id
                .strip_prefix("printer-default-")
                .map(str::to_owned)
        })
        .filter(|id| !id.trim().is_empty())?;
    edit(|store| {
        let store = &*store;
        store.preset(preset_id.trim()).cloned()
    })
}

/// `getMaterialProfilesForPrinter`: the materials the printer owns plus the
/// ones shared with it (`materialProfileIsAvailableForPrinter`).
pub fn materials_for(printer_profile_id: &str) -> Vec<MaterialProfile> {
    edit(|store| {
        store
            .materials
            .iter()
            .filter(|material| material.available_for(printer_profile_id))
            .cloned()
            .collect()
    })
}

/// `activeMaterialProfileId` for the selected printer.
pub fn active_material_id() -> Option<String> {
    edit(|store| {
        let store = &*store;
        store.active_material.clone().filter(|id| {
            store.materials.iter().any(|material| &material.id == id)
        })
    })
}

/// The default material name for a new row: `Material <n>` (`handleAddMaterial`
/// / `addMaterialProfile`).
pub fn next_material_name(printer_profile_id: &str) -> String {
    format!(
        "Material {}",
        materials_for(printer_profile_id).len() + 1
    )
}

// ── Writing ─────────────────────────────────────────────────────────────────

/// `handlePickPrinter`: select locally, then `setActivePrinterProfile`.
pub fn select(id: &str) {
    // The picker is the store's active-printer owner, and [`edit`] syncs the
    // selection from it — write the picker first so the sync agrees.
    printer_picker::select_profile(id);
    edit(|store| {
        if store.profile_index(id).is_none() {
            return;
        }
        store.selected = Some(id.to_owned());
        store.editing = false;
    });
}

/// `handlePickPrinter` + `setIsEditingPrinter(true)` (the card's double-click).
pub fn open_editor(id: &str) {
    printer_picker::select_profile(id);
    edit(|store| {
        if store.profile_index(id).is_none() {
            return;
        }
        store.selected = Some(id.to_owned());
        store.editing = true;
    });
}

pub fn set_editing(editing: bool) {
    edit(|store| store.editing = editing);
}

/// `updatePrinterProfile(id, updates)`.
pub fn update_profile(id: &str, f: impl FnOnce(&mut PrinterProfile)) {
    edit(|store| {
        if let Some(index) = store.profile_index(id) {
            f(&mut store.profiles[index]);
        }
    });
}

/// `moveSelectedPrinterInRail`: `direction` is `-1` (left) or `1` (right).
/// Returns whether the list changed.
pub fn move_selected(direction: i32) -> bool {
    let change = edit(|store| {
        let store = &mut *store;
        let selected = store.selected.clone()?;
        let index = store.profile_index(&selected)?;
        let target = index as i32 + direction;
        if target < 0 || target as usize >= store.profiles.len() {
            return None;
        }
        // `movePrinterProfile(selected.id, direction > 0
        //   ? printerProfiles[targetIndex + 1]?.id
        //   : targetPrinter.id)`
        let before = if direction > 0 {
            store
                .profiles
                .get(target as usize + 1)
                .map(|profile| profile.id.clone())
        } else {
            Some(store.profiles[target as usize].id.clone())
        };
        move_profile_in(store, &selected, before.as_deref());
        Some((selected, before))
    });
    let Some((id, before)) = change else {
        return false;
    };
    printer_picker::move_profile(&id, before.as_deref());
    true
}

/// The rail's drag-and-drop move: insert `id` before `before_id`, or at the end
/// when it is `None` (`movePrinterProfile`).
pub fn move_profile(id: &str, before_id: Option<&str>) {
    let before = before_id.map(str::to_owned);
    edit(|store| move_profile_in(store, id, before.as_deref()));
    printer_picker::move_profile(id, before.as_deref());
}

fn move_profile_in(store: &mut Store, id: &str, before_id: Option<&str>) {
    let Some(index) = store.profile_index(id) else {
        return;
    };
    let moved = store.profiles.remove(index);
    let before = before_id
        .map(str::trim)
        .filter(|before| !before.is_empty())
        .and_then(|before| store.profile_index(before));
    match before {
        Some(index) => store.profiles.insert(index, moved),
        None => store.profiles.push(moved),
    }
}

/// `requestDeleteSelectedPrinter`: open the confirm dialog.
pub fn request_remove_selected() {
    edit(|store| {
        let target = store.selected.as_ref().and_then(|id| {
            store
                .profiles
                .iter()
                .find(|profile| &profile.id == id)
                .map(|profile| (profile.id.clone(), profile.name.clone()))
        });
        store.pending_remove = target;
    });
}

pub fn cancel_remove() {
    edit(|store| store.pending_remove = None);
}

/// `handleConfirmDelete` for `kind === 'printer'`: `removePrinterProfile` plus
/// the local material rows and tank assignments that pointed at it.
pub fn confirm_remove() {
    let removed = edit(|store| {
        let store = &mut *store;
        let (id, _) = store.pending_remove.take()?;
        remove_profile_in(store, &id);
        Some(id)
    });
    if let Some(id) = removed {
        printer_picker::remove_profile(&id);
    }
}

pub fn remove_profile(id: &str) {
    let known = edit(|store| {
        let store = &mut *store;
        let known = store.profile_index(id).is_some();
        remove_profile_in(store, id);
        known
    });
    if known {
        printer_picker::remove_profile(id);
    }
}

/// `removePrinterProfile` (`profileStore.ts:2457`): drop the profile, its own
/// materials, any tank assignment that named it, and fall back to the first
/// remaining profile when it was the selected one.
fn remove_profile_in(store: &mut Store, id: &str) {
    store.profiles.retain(|profile| profile.id != id);
    store
        .materials
        .retain(|material| material.printer_profile_id != id);
    for profile in &mut store.profiles {
        for tank in &mut profile.tank_materials {
            if tank.as_str() == id {
                tank.clear();
            }
        }
    }
    if store.selected.as_deref() == Some(id) {
        store.selected = store.profiles.first().map(|profile| profile.id.clone());
    }
    store.editing = false;
    store.pending_remove = None;
    store.ensure_materials();
}

/// `duplicatePrinterProfileAsCustom`: a custom copy, its materials copied, and
/// the copy made active. The copy keeps `officialPresetId`, as the source's
/// spread does; only `isOfficial` / `isCustom` change.
pub fn duplicate_as_custom(id: &str) -> Option<String> {
    edit(|store| {
        let store = &mut *store;
        let source = store.profiles.iter().find(|profile| profile.id == id)?.clone();
        let duplicate_id = store.next_custom_id();
        let base_name = if source.name.contains("Custom") {
            source.name.clone()
        } else {
            format!("{} Custom", source.name)
        };
        let duplicate_name = if store.profiles.iter().any(|profile| profile.name == base_name) {
            format!("{base_name} {}", store.profiles.len() + 1)
        } else {
            base_name
        };

        let mut duplicated = source.clone();
        duplicated.id = duplicate_id.clone();
        duplicated.name = duplicate_name;
        duplicated.is_official = false;
        duplicated.is_custom = true;
        store.profiles.push(duplicated);

        let source_materials: Vec<MaterialProfile> = store
            .materials
            .iter()
            .filter(|material| material.printer_profile_id == source.id)
            .cloned()
            .collect();
        let mut duplicated_materials: Vec<MaterialProfile> = if source_materials.is_empty() {
            vec![MaterialProfile::standard(&duplicate_id)]
        } else {
            source_materials
                .iter()
                .map(|material| {
                    let mut copy = material.clone();
                    copy.id = store.next_material_id();
                    copy.printer_profile_id = duplicate_id.clone();
                    copy.official_template_id = None;
                    copy.official_template_version = None;
                    copy
                })
                .collect()
        };
        let active = duplicated_materials
            .first()
            .map(|material| material.id.clone());
        store.materials.append(&mut duplicated_materials);
        store.selected = Some(duplicate_id.clone());
        store.active_material = active;
        store.editing = false;
        Some(duplicate_id)
    })
}

/// `setBuildDimensionMode` (`ProfileSettingsModal.tsx:846`): auto derives
/// width/depth from `pixelSize` × resolution; manual restores the remembered
/// pair when there is one.
pub fn set_build_dimension_mode(id: &str, mode: &str) {
    edit(|store| {
        let Some(index) = store.profile_index(id) else {
            return;
        };
        let current = store.profiles[index].build_dimension_mode.clone();
        if current.as_deref() == Some(mode) {
            return;
        }
        let profile = store.profiles[index].clone();

        if mode == "auto" {
            // `manualBuildDimensionsByPrinterId[selectedPrinter.id] = {width, depth}`
            let remembered = (profile.build_volume_mm.width, profile.build_volume_mm.depth);
            match store
                .manual_dimensions
                .iter_mut()
                .find(|entry| entry.0.as_str() == profile.id)
            {
                Some((_, dimensions)) => *dimensions = remembered,
                None => store.manual_dimensions.push((profile.id.clone(), remembered)),
            }
            let mut updated = profile.clone();
            updated.build_dimension_mode = Some("auto".to_owned());
            let pixel_size = profile.pixel_size.unwrap_or_default();
            updated.build_volume_mm.width = Some(compute_build_dimension_mm(
                profile.display.resolution_x,
                pixel_size.x,
            ));
            updated.build_volume_mm.depth = Some(compute_build_dimension_mm(
                profile.display.resolution_y,
                pixel_size.y,
            ));
            store.profiles[index] = updated;
            return;
        }

        let mut updated = profile.clone();
        updated.build_dimension_mode = Some("manual".to_owned());
        if let Some((_, (width, depth))) = store
            .manual_dimensions
            .iter()
            .find(|(id, _)| id == &profile.id)
        {
            updated.build_volume_mm.width = *width;
            updated.build_volume_mm.depth = *depth;
        }
        store.profiles[index] = updated;
    });
}

// ── Materials ───────────────────────────────────────────────────────────────

/// `setSelectedMaterialId` + the active-material sidecar write.
pub fn select_material(id: &str) {
    edit(|store| {
        if store.materials.iter().any(|material| material.id == id) {
            store.active_material = Some(id.to_owned());
        }
    });
}

/// `addMaterialProfile(printerProfileId)`: mints `Material <n>` with the
/// source's defaults and makes it active.
pub fn add_material(printer_id: &str) -> String {
    edit(|store| {
        let id = store.next_material_id();
        let name = format!("Material {}", store.materials.len() + 1);
        let material = MaterialProfile {
            id: id.clone(),
            printer_profile_id: printer_id.to_owned(),
            name,
            ..MaterialProfile::default()
        };
        store.materials.push(material);
        store.active_material = Some(id.clone());
        id
    })
}

/// A copy of a material for the same printer (`duplicate` in the material
/// editor).
pub fn duplicate_material(id: &str) -> String {
    edit(|store| {
        let Some(source) = store
            .materials
            .iter()
            .find(|material| material.id == id)
            .cloned()
        else {
            return String::new();
        };
        let duplicate_id = store.next_material_id();
        let mut duplicate = source;
        duplicate.id = duplicate_id.clone();
        duplicate.name = format!("{} Copy", duplicate.name);
        duplicate.official_template_id = None;
        duplicate.official_template_version = None;
        duplicate.locked = false;
        store.materials.push(duplicate);
        store.active_material = Some(duplicate_id.clone());
        duplicate_id
    })
}

pub fn update_material(id: &str, f: impl FnOnce(&mut MaterialProfile)) {
    edit(|store| {
        if let Some(material) = store.materials.iter_mut().find(|material| material.id == id) {
            f(material);
        }
    });
}

/// `removeMaterialProfile`: clears the material from every tank that used it.
pub fn remove_material(id: &str) -> bool {
    edit(|store| {
        let before = store.materials.len();
        store.materials.retain(|material| material.id != id);
        if store.materials.len() == before {
            return false;
        }
        for profile in &mut store.profiles {
            for tank in &mut profile.tank_materials {
                if tank.as_str() == id {
                    tank.clear();
                }
            }
        }
        if store.active_material.as_deref() == Some(id) {
            store.active_material = None;
        }
        true
    })
}

/// `tankMaterials[index] = materialId`; `''` clears the tank. The vector is
/// trimmed to `tankCount` first, as the source does.
pub fn assign_tank(printer_id: &str, index: usize, material_id: &str) {
    edit(|store| {
        let Some(position) = store.profile_index(printer_id) else {
            return;
        };
        let count = store.profiles[position].tank_count_or_default() as usize;
        let profile = &mut store.profiles[position];
        if profile.tank_materials.len() > count {
            profile.tank_materials.truncate(count);
        }
        if profile.tank_materials.len() < count {
            profile.tank_materials.resize(count, String::new());
        }
        if index < profile.tank_materials.len() {
            profile.tank_materials[index] = material_id.to_owned();
        }
    });
}

// ── Escape ──────────────────────────────────────────────────────────────────

/// `useEscapeToClose` inside the modal: unwinds the confirm dialog, then the
/// printer editor. Returns `true` when this modal consumed the key; `false`
/// means the parent should close the modal.
pub fn escape() -> bool {
    let mut store = store();
    if store.pending_remove.take().is_some() {
        return true;
    }
    if store.editing {
        store.editing = false;
        return true;
    }
    false
}

/// Reset the transient surfaces when the modal opens (`requestOpenProfileSettings`).
pub fn begin() {
    edit(|store| {
        store.editing = false;
        store.pending_remove = None;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The built-in catalogue is the picker's two files and must keep parsing.
    #[test]
    fn catalogue_parses() {
        let presets = catalogue();
        assert!(
            presets
                .iter()
                .any(|preset| preset.preset_id == "generic-machine"),
            "the generic reference preset is in profiles/printers/generic/printers.json"
        );
    }

    /// `PrinterPreset` → `PrinterProfile` → JSON → back, unchanged.
    #[test]
    fn profile_round_trips() {
        let preset = catalogue()
            .into_iter()
            .find(|preset| preset.preset_id == "generic-machine")
            .expect("the generic reference preset");
        let profile = preset.to_profile("printer-1");
        let json = serde_json::to_string(&profile).expect("serialize");
        let restored: PrinterProfile = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(profile, restored);
    }

    /// The preset shape the source ships round-trips too.
    #[test]
    fn preset_round_trips() {
        let preset = catalogue()
            .into_iter()
            .find(|preset| preset.preset_id == "generic-machine")
            .expect("the generic reference preset");
        let json = serde_json::to_string(&preset).expect("serialize");
        let restored: PrinterPreset = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(preset, restored);
    }

    /// `#[serde(default)]`: a profile written by an older schema loads.
    #[test]
    fn missing_fields_take_defaults() {
        let restored: PrinterProfile =
            serde_json::from_str(r#"{"id":"printer-1","name":"Generic Machine"}"#)
                .expect("deserialize");
        assert_eq!(restored.id, "printer-1");
        assert!(restored.build_volume_mm.width.is_none());
        assert!(restored.tank_materials.is_empty());
        assert!(!restored.is_official);
        assert_eq!(restored.display.output_format, "");
    }

    /// The icon-well label and the bit-depth chip read the source's rules.
    #[test]
    fn bit_depth_rules() {
        let mut profile = PrinterProfile::default();
        profile.bit_depth = Some(BitDepth {
            bits: 8.4,
            description: None,
        });
        assert_eq!(profile.bit_depth_bits(), Some(8));
        assert_eq!(profile.bit_depth_label(), None);
        profile.bit_depth = Some(BitDepth {
            bits: 3.0,
            description: None,
        });
        assert_eq!(profile.bit_depth_label().as_deref(), Some("3 Bit"));
    }

    /// `computeBuildDimensionMm(2560, 19)` = `2560 × 19 µm / 1000` = `48.64`.
    #[test]
    fn build_dimension_math() {
        assert_eq!(compute_build_dimension_mm(2560.0, 19.0), 48.64);
        // The source's `toFixed(3)` step: a 14 µm pixel at 15120 px.
        assert_eq!(compute_build_dimension_mm(15120.0, 14.0), 211.68);
    }

    /// `MaterialProfile` defaults and round-trip, including the AA settings.
    #[test]
    fn material_round_trips() {
        let material = MaterialProfile::standard("printer-1");
        let json = serde_json::to_string(&material).expect("serialize");
        let restored: MaterialProfile = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(material, restored);
        assert_eq!(material.name, "Standard 405nm");
        assert_eq!(material.bottle_price, 24.99);
        assert_eq!(material.anti_aliasing_settings.mode, "Blur");
    }
}
