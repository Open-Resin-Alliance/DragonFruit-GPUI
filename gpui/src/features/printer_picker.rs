//! The bar's "Select printer" surface: the printer library plus the "Initial
//! Setup" variant picker.
//!
//! Ported from `src/components/layout/OnboardingPrinterLibrary.tsx` (the library
//! rows: manufacturer/search sidebar, family-grouped preset cards, the
//! `Choose a printer to add` footer) and
//! `src/components/printers/PrinterVariantPickerModal.tsx` (the "Initial Setup"
//! modal: the method chooser, the network scan/status card, the manual-IP card,
//! the bit-depth variant rows and the `Cancel` / `Add printer` actions), with
//! its shared pieces from `src/components/printers/SetupMethodChooser.tsx`.
//!
//! The printer records are the source's preset shape (`PrinterPreset` in
//! `src/features/profiles/profileStore.ts:80`), modelled with serde and seeded
//! from the built-in catalogue the source ships:
//!
//! * `profiles/printers/generic/printers.json` — the one core preset
//!   (`generic-machine`), the Open Resin Alliance reference machine;
//! * `plugins/athena/printers/printers.json` — the built-in Athena profile pack
//!   (`GENERATED_BUILTIN_COMPLEX_PLUGIN_DEFINITIONS` in `src/features/plugins/`),
//!   which is where the 3-bit / 8-bit variant family comes from.
//!
//! A profile (an installed printer) is created from a preset exactly as
//! `addPrinterProfileFromPreset` does, and its material rows come from
//! `ensureActiveMaterialForActivePrinter`'s fallback profile — the source's
//! literal `Standard 405nm` (`profileStore.ts`, the `createdMaterial` branch),
//! not a material-template catalogue, because a fresh store has none.
//!
//! # PORT DEVIATIONS
//!
//! 1. The profile store itself is not ported. [`PrinterStore`] is process-wide
//!    (the same shape `features::settings` and `features::auto_support_settings`
//!    use) and in memory: nothing is persisted to `localStorage` or disk, no
//!    plugin can contribute presets, and a restart starts from the empty state
//!    the source has before a printer is added.
//! 2. `getProfileNetworkUiAdapter` / `pluginNetworkFetch` have no port. The
//!    library therefore never opens on the "choose" method tab: a preset with
//!    `modelVariants` but no adapter falls back to the manual rows, exactly the
//!    source's `networkAdapter ? 'choose' : 'manual'`. Scanning never yields a
//!    device, so the scan tab renders the source's `No printers found.` state,
//!    and the manual-IP card cannot connect.
//! 3. The library search box and the manual-IP box are real editors
//!    (`settings::fields::text_input`): the search commits per keystroke into
//!    `search` and filters name / manufacturer / family exactly as
//!    `OnboardingPrinterLibrary.tsx:86-97` does (a non-empty term drops the
//!    manufacturer filter and shows one flat grid), and the IP box commits into
//!    `networkIpAddress` with Enter/Connect following the source's enabled
//!    state. Connect still cannot reach an adapter (deviation 2). The source's
//!    `Search` glyph inside the search box is omitted: the shared `.ui-input`
//!    element's text inset is fixed at 10px, so the glyph and the first
//!    characters would overlap.
//! 4. Printer artwork is not part of the GPUI asset set, so every card renders
//!    the source's own fallback (`Printer` for the generic preset, `ImagePlus`
//!    for the rest). `profile.thumbnail` on the bar stays `None`.
//! 5. `.ui-modal-backdrop-enter` / `.ui-modal-panel-enter`, `hover:` borders,
//!    `transition-*`, `tracking-[0.08em]`/`letter-spacing`, and the selected
//!    card's `inset 0 0 0 1.5px` ring (GPUI has no box-shadow shorthand here)
//!    are not expressible; the selected card keeps the accent fill and takes an
//!    accent border instead of the ring.
//! 6. The 25s `showConfigureLater` timer and the "Configure Networking Later"
//!    link are not reproduced: without an adapter there is no scan to grow
//!    long, and the fallback would add the printer without networking, which is
//!    what picking a variant row already does.
//! 7. `dialog_modal` supplies the shell (header, close tile, action row), so the
//!    source library's own header band and back button are the dialog header;
//!    the setup footer's mode switch rides in the action row.
//! 8. The library's `height: min(90vh, 1066px)` is a fixed 520px body: GPUI
//!    gives this function no window measurement.
//! 9. Family presets group by `family` and keep their catalogue order (the
//!    source does the same); the generic manufacturer sorts last.

#![allow(dead_code)]

use std::sync::{LazyLock, Mutex, MutexGuard};

use gpui::{
    AnyElement, App, ClickEvent, Div, FontWeight, IntoElement, SharedString, Stateful, WeakEntity,
    Window, div, linear_color_stop, linear_gradient, prelude::*, px, relative, rgb, rgba,
};
use serde::Deserialize;

use gpui_core::icons::icon;
use gpui_core::theme::*;
use gpui_ui::ui::{
    ButtonSize, ButtonSpec, ButtonVariant, IconTone, ProgressSize, button, button_with,
    button_with_content, dialog_modal, fade_text, progress_bar, spinner,
};

use crate::features::settings::fields::text_input;
use crate::DragonFruit;
use crate::chrome::app_bar::PrinterBadge;

const WHITE: u32 = 0xffffff;
const BLACK: u32 = 0x000000;

// ── The source's preset shape ───────────────────────────────────────────────

/// `PrinterBitDepth` (`profileStore.ts:71`).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BitDepth {
    pub bits: f32,
    pub description: Option<String>,
}

/// `PrinterPlatformBadge`: `{ text, color }`, the NanoDLP / Odyssey chip.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PlatformBadge {
    pub text: String,
    pub color: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PixelSize {
    pub x: f32,
    pub y: f32,
}

/// `PrinterPreset.buildVolumeMm`. The Athena presets carry `null` for width and
/// depth (`buildDimensionMode: auto` derives them from `pixelSize` ×
/// `display.resolution`), so the axes are optional exactly as the JSON is.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BuildVolumeMm {
    pub width: Option<f32>,
    pub depth: Option<f32>,
    pub height: Option<f32>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SafetyMarginMm {
    pub front: f32,
    pub back: f32,
    pub left: f32,
    pub right: f32,
}

/// `PrinterPreset.display`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PresetDisplay {
    pub resolution_x: f32,
    pub resolution_y: f32,
    pub output_format: String,
    pub format_version: Option<String>,
    pub settings_mode: Option<String>,
    pub webcam_rotation_deg: Option<u32>,
    pub mirror_x: Option<bool>,
    pub mirror_y: Option<bool>,
}

/// `PrinterPreset` (`profileStore.ts:80-149`). Every field the source declares
/// is modelled so a plugin preset can be dropped in unchanged; the picker reads
/// the identity, family, variant, badge and display subset.
#[derive(Clone, Debug, Default, Deserialize)]
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
    pub build_dimension_mode: Option<String>,
    pub material_mode: Option<String>,
    pub tank_count: Option<u32>,
    pub allow_multi_plate: Option<bool>,
    pub plate_shape: Option<String>,
    /// Concrete variant preset ids of a family preset.
    pub model_variants: Option<Vec<String>>,
    /// The cleaner library card name ("Athena II 16K" for "… · 8-bit").
    pub library_display_name: Option<String>,
    /// HTTP path a plugin adapter probes to auto-detect the variant.
    pub model_variant_detect_path: Option<String>,
    /// A hidden variant: not a standalone library card.
    pub is_model_variant: Option<bool>,
    pub build_volume_mm: BuildVolumeMm,
    pub safety_margin_mm: Option<SafetyMarginMm>,
    pub display: PresetDisplay,
}

impl PrinterPreset {
    /// `libraryDisplayName ?? name`.
    fn library_name(&self) -> String {
        self.library_display_name
            .clone()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| self.name.clone())
    }

    /// `preset.modelVariants?.length || preset.modelVariantDetectPath` — the
    /// library opens "Initial Setup" for these instead of selecting them.
    fn opens_setup(&self) -> bool {
        self.model_variants
            .as_ref()
            .is_some_and(|variants| !variants.is_empty())
            || self
                .model_variant_detect_path
                .as_ref()
                .is_some_and(|path| !path.trim().is_empty())
    }

    fn is_generic(&self) -> bool {
        self.manufacturer.eq_ignore_ascii_case("generic")
            || self.name.to_ascii_lowercase().contains("generic")
    }
}

/// The runtime printer profile the picker owns (`PrinterProfile` in the source,
/// narrowed to the fields this surface reads).
#[derive(Clone, Debug)]
pub struct PrinterProfile {
    /// `printer-<n>`: the source mints `createId('printer')`; the port keeps a
    /// stable per-process id.
    pub id: String,
    pub name: String,
    pub manufacturer: String,
    /// `officialPresetId`.
    pub official_preset_id: String,
    pub image_asset_path: Option<String>,
    pub bit_depth: Option<BitDepth>,
    pub build_volume_mm: BuildVolumeMm,
    pub display: PresetDisplay,
}

/// One installed material profile (`MaterialProfile` in the source). The picker
/// only needs what its material rows draw; the exposure/lift fields the source
/// also stores are not read here.
#[derive(Clone, Debug)]
pub struct MaterialProfile {
    pub id: String,
    pub printer_profile_id: String,
    pub name: String,
    pub brand: String,
    /// `color?: '#rrggbb'`, kept as a packed `0xrrggbb`.
    pub color: Option<u32>,
    pub layer_height_mm: f32,
}

impl MaterialProfile {
    /// `ensureActiveMaterialForActivePrinter`'s `createdMaterial`: the one
    /// profile the store mints for a printer that has none, at the source's
    /// literal values (not one of the material templates).
    fn standard(profile_id: &str) -> Self {
        Self {
            id: format!("{profile_id}-material-standard-405"),
            printer_profile_id: profile_id.to_owned(),
            name: "Standard 405nm".to_owned(),
            brand: "Default".to_owned(),
            color: None,
            layer_height_mm: 0.05,
        }
    }
}

/// The printer the setup dialog's network flow resolved, kept so the confirm
/// action can persist the connection (`PrinterVariantPickerNetworkContext`).
#[derive(Clone, Debug)]
pub struct NetworkContext {
    pub host: String,
    pub port: Option<u32>,
    pub printer_name: Option<String>,
    pub mode: Option<String>,
}

// ── The store ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    /// `OnboardingPrinterLibrary`.
    Library,
    /// `PrinterVariantPickerModal`, opened for one family preset.
    Setup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupMode {
    /// The variant rows (`mode === 'manual'`).
    Manual,
    /// The network scan (`mode === 'scan'`).
    Scan,
    /// The manual-IP card (`mode === 'ip'`).
    Ip,
}

/// The picker's own state. The source keeps this in React state; the port keeps
/// it here because `DragonFruit` only carries the `show_printer_picker` flag.
pub struct PrinterStore {
    presets: Vec<PrinterPreset>,
    profiles: Vec<PrinterProfile>,
    materials: Vec<MaterialProfile>,
    active_profile: Option<String>,
    active_material: Option<String>,

    surface: Surface,
    /// The manufacturer rail's selection; `None` until the first view, then the
    /// first manufacturer (`useEffect` default in the library).
    manufacturer: Option<String>,
    /// The library card under the pointer's selection (`selectedPresetId`).
    selected_preset: Option<String>,
    /// The family preset the setup surface is resolving (`variantChooserPreset`).
    family_preset: Option<String>,

    mode: SetupMode,
    /// `search` (`OnboardingPrinterLibrary.tsx:46`): the library's search term.
    search: String,
    /// `networkIpAddress` (`ProfileSettingsModal.tsx:435`): the manual-IP
    /// card's host.
    manual_ip: String,
    /// The network scan's results. Empty in this port (deviation 2) but modelled
    /// so the device rows are real when an adapter exists.
    devices: Vec<DeviceRow>,
    scanning: bool,
    scan_error: Option<String>,
    connecting: Option<String>,
    detected: Option<String>,
    detected_device_name: Option<String>,
    detect_error: Option<String>,
    pending_network: Option<NetworkContext>,
}

/// `DeviceInfo` (`PrinterVariantPickerModal.tsx:96`).
#[derive(Clone, Debug)]
pub struct DeviceRow {
    pub ip_address: String,
    pub port: Option<u32>,
    pub printer_name: Option<String>,
    pub printer_model: Option<String>,
    pub host_name: Option<String>,
}

impl DeviceRow {
    /// `printerName || printerModel || hostName || ipAddress`.
    fn display_name(&self) -> String {
        self.printer_name
            .clone()
            .or_else(|| self.printer_model.clone())
            .or_else(|| self.host_name.clone())
            .unwrap_or_else(|| self.ip_address.clone())
    }
}

impl PrinterStore {
    fn seed() -> Self {
        let mut presets = parse_presets(include_str!(
            "../../../profiles/printers/generic/printers.json"
        ));
        presets.extend(parse_presets(include_str!(
            "../../../plugins/athena/printers/printers.json"
        )));
        // The source's `createDefaultState` ships no printer profiles: the top
        // bar reads "Select printer" and the library is how one is added.
        Self {
            presets,
            profiles: Vec::new(),
            materials: Vec::new(),
            active_profile: None,
            active_material: None,
            surface: Surface::Library,
            manufacturer: None,
            selected_preset: None,
            family_preset: None,
            mode: SetupMode::Manual,
            search: String::new(),
            manual_ip: String::new(),
            devices: Vec::new(),
            scanning: false,
            scan_error: None,
            connecting: None,
            detected: None,
            detected_device_name: None,
            detect_error: None,
            pending_network: None,
        }
    }

    /// `getLibraryPrinterPresets`: the full set minus hidden variants.
    fn library_presets(&self) -> impl Iterator<Item = &PrinterPreset> {
        self.presets
            .iter()
            .filter(|preset| preset.is_model_variant != Some(true))
    }

    /// `getPrinterPresetVariants`.
    fn variants_of(&self, preset_id: &str) -> Vec<PrinterPreset> {
        let Some(family) = self
            .presets
            .iter()
            .find(|preset| preset.preset_id == preset_id)
        else {
            return Vec::new();
        };
        family
            .model_variants
            .as_ref()
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.presets.iter().find(|preset| &preset.preset_id == id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    fn preset(&self, preset_id: &str) -> Option<&PrinterPreset> {
        self.presets
            .iter()
            .find(|preset| preset.preset_id == preset_id)
    }

    /// `addedPresetIds`: preset ids already installed as official profiles.
    fn is_added(&self, preset_id: &str) -> bool {
        self.profiles
            .iter()
            .any(|profile| profile.official_preset_id == preset_id)
    }

    /// `manufacturers`: unique, non-generic first in locale order, generic last.
    fn manufacturers(&self) -> Vec<String> {
        let mut unique: Vec<String> = Vec::new();
        for preset in self.library_presets() {
            if !unique.iter().any(|name| name == &preset.manufacturer) {
                unique.push(preset.manufacturer.clone());
            }
        }
        unique.sort_by(|a, b| a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()));
        let (generic, named): (Vec<String>, Vec<String>) = unique
            .into_iter()
            .partition(|name| name.eq_ignore_ascii_case("generic"));
        named.into_iter().chain(generic).collect()
    }

    /// `handleAdd` / `handleSelectVariant`:
    /// `addPrinterProfileFromPreset` + `setActivePrinterProfile`.
    fn add_preset(&mut self, preset_id: &str) {
        let Some(preset) = self.preset(preset_id).cloned() else {
            return;
        };
        // `addPrinterProfileFromPreset` returns the existing official profile.
        if let Some(existing) = self
            .profiles
            .iter()
            .find(|profile| profile.official_preset_id == preset_id)
        {
            self.active_profile = Some(existing.id.clone());
            self.ensure_active_material();
            return;
        }
        let id = format!("printer-{}", self.profiles.len() + 1);
        self.profiles.push(PrinterProfile {
            id: id.clone(),
            name: preset.name.clone(),
            manufacturer: preset.manufacturer.clone(),
            official_preset_id: preset.preset_id.clone(),
            image_asset_path: preset.image_asset_path.clone(),
            bit_depth: preset.bit_depth.clone(),
            build_volume_mm: preset.build_volume_mm,
            display: preset.display.clone(),
        });
        self.active_profile = Some(id.clone());
        // `ensureActiveMaterialForActivePrinter`: a printer with no material
        // profile of its own gets the source's `Standard 405nm` fallback.
        if !self
            .materials
            .iter()
            .any(|material| material.printer_profile_id == id)
        {
            self.materials.push(MaterialProfile::standard(&id));
        }
        self.ensure_active_material();
    }

    fn ensure_active_material(&mut self) {
        let Some(profile_id) = self.active_profile.clone() else {
            return;
        };
        let belongs = self.active_material.as_ref().is_some_and(|id| {
            self.materials
                .iter()
                .any(|material| &material.id == id && material.printer_profile_id == profile_id)
        });
        if belongs {
            return;
        }
        self.active_material = self
            .materials
            .iter()
            .find(|material| material.printer_profile_id == profile_id)
            .map(|material| material.id.clone());
    }

    fn active_printer(&self) -> Option<&PrinterProfile> {
        let id = self.active_profile.as_ref()?;
        self.profiles.iter().find(|profile| &profile.id == id)
    }

    fn materials_for_active(&self) -> Vec<&MaterialProfile> {
        let Some(profile_id) = self.active_profile.as_ref() else {
            return Vec::new();
        };
        self.materials
            .iter()
            .filter(|material| &material.printer_profile_id == profile_id)
            .collect()
    }
}

fn parse_presets(json: &str) -> Vec<PrinterPreset> {
    serde_json::from_str(json).unwrap_or_else(|error| {
        eprintln!("printer picker: built-in preset catalogue did not parse: {error}");
        Vec::new()
    })
}

/// `sanitizeMaterialColor`: `#rrggbb` or nothing.
fn parse_hex_color(value: &str) -> Option<u32> {
    let hex = value.trim().strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}

static STORE: LazyLock<Mutex<PrinterStore>> = LazyLock::new(|| Mutex::new(PrinterStore::seed()));

fn store() -> MutexGuard<'static, PrinterStore> {
    STORE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn edit<R>(f: impl FnOnce(&mut PrinterStore) -> R) -> R {
    let mut guard = store();
    f(&mut guard)
}

/// Drop the setup dialog's transient state and show the library again.
fn pop_to_library(store: &mut PrinterStore) {
    store.surface = Surface::Library;
    store.family_preset = None;
    store.selected_preset = None;
    store.mode = SetupMode::Manual;
    store.detected = None;
    store.detected_device_name = None;
    store.detect_error = None;
    store.connecting = None;
    store.pending_network = None;
}

/// The source's `onClose` differs per surface: the library sheet closes, while
/// `PrinterVariantPickerModal`'s own close (and its `Cancel`) dismisses the
/// nested dialog and leaves the library underneath.
fn dismiss(
    entity: &WeakEntity<DragonFruit>,
    back_to_library: bool,
) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    let entity = entity.clone();
    move |_, _, cx| {
        if back_to_library {
            edit(pop_to_library);
            entity.update(cx, |_, cx| cx.notify()).ok();
        } else {
            entity
                .update(cx, |this, cx| {
                    this.show_printer_picker = false;
                    cx.notify();
                })
                .ok();
        }
    }
}

fn click_edit(
    entity: &WeakEntity<DragonFruit>,
    action: impl Fn(&mut PrinterStore) + 'static,
) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    let entity = entity.clone();
    move |_, _, cx| {
        edit(&action);
        entity.update(cx, |_, cx| cx.notify()).ok();
    }
}

// ── Public entry points ─────────────────────────────────────────────────────

/// Reset the transient surface when the bar reopens the picker: the bar's
/// button always lands on the library, as the source's printer settings do.
pub fn begin() {
    edit(pop_to_library);
}

/// `useEscapeToClose`: the nested setup dialog closes first; only the library
/// closes the picker itself. Returns whether Escape was consumed.
pub fn escape() -> bool {
    let mut store = store();
    if store.surface == Surface::Setup {
        pop_to_library(&mut store);
        true
    } else {
        false
    }
}

/// The top bar's `activePrinterProfile`: name and thumbnail, no fleet (the port
/// has no network devices).
pub fn badge() -> PrinterBadge {
    let store = store();
    let profile = store.active_printer();
    PrinterBadge {
        profile_name: profile.map(|profile| profile.name.clone()),
        thumbnail: None,
        fleet: Vec::new(),
        active_device: None,
    }
}

/// The picker surface. Rendered by the parent after the bar and the settings
/// modal, so it paints over both.
pub fn printer_picker(open: bool, entity: WeakEntity<DragonFruit>) -> impl IntoElement {
    if !open {
        return div();
    }
    let view = view();

    let body = match view.surface {
        Surface::Library => library_body(&view, entity.clone()),
        Surface::Setup => setup_body(&view, entity.clone()),
    };
    let actions = match view.surface {
        Surface::Library => library_actions(&view, entity.clone()),
        Surface::Setup => setup_actions(&view, entity.clone()),
    };
    let (title, subtitle) = match view.surface {
        Surface::Library => (
            SharedString::from("Printer Library"),
            SharedString::from("Choose an official printer preset to add."),
        ),
        Surface::Setup => (
            SharedString::from("Initial Setup"),
            SharedString::from(view.family_title.clone().unwrap_or_default()),
        ),
    };
    let width = match view.surface {
        Surface::Library => 1040.0,
        Surface::Setup => 560.0,
    };
    let close = dismiss(&entity, view.surface == Surface::Setup);
    dialog_modal(
        width,
        title,
        Some(subtitle),
        Some("printer"),
        IconTone::Accent,
        body,
        actions,
        close,
    )
}

// ── Snapshot ────────────────────────────────────────────────────────────────

/// One preset as a card / row.
struct PresetRow {
    preset_id: String,
    /// `libraryDisplayName ?? name` — the library card title.
    title: String,
    /// `libraryDisplayName ?? stripVariantSuffix(name)` — the setup row title.
    variant_title: String,
    manufacturer: String,
    bits: Option<i64>,
    bit_description: Option<String>,
    platform: Option<(String, u32)>,
    added: bool,
    is_generic: bool,
    opens_setup: bool,
}

struct MaterialRow {
    id: String,
    name: String,
    color: Option<u32>,
    layer_height_mm: f32,
}

struct PickerView {
    surface: Surface,
    manufacturers: Vec<String>,
    manufacturer: Option<String>,
    /// `groupedPresets`: `(family, presets)` in catalogue order.
    groups: Vec<(String, Vec<PresetRow>)>,
    selected: Option<String>,
    selected_label: Option<String>,
    /// `preset.modelVariants?.length || preset.modelVariantDetectPath` for the
    /// selected card, resolved against the whole catalogue (the source looks
    /// the selection up in `presets`, not only the visible manufacturer).
    selected_opens_setup: bool,
    family_title: Option<String>,
    variants: Vec<PresetRow>,
    materials: Vec<MaterialRow>,
    active_material: Option<String>,
    mode: SetupMode,
    /// The library's search term, as typed.
    search: String,
    /// `isSearching`: a non-empty term bypasses the manufacturer filter and
    /// shows one flat grid instead of the family sections.
    is_searching: bool,
    /// `networkIpAddress`: the manual-IP card's host.
    manual_ip: String,
    /// The filtered presets in catalogue order, which is the order the flat
    /// searching grid uses.
    flat: Vec<PresetRow>,
    devices: Vec<(String, String, Option<u32>)>,
    scanning: bool,
    scan_error: Option<String>,
    /// The variant the network flow resolved (`detectedVariant`), as the row the
    /// status card and the confirm action draw.
    detected: Option<PresetRow>,
    detected_device_name: Option<String>,
    detect_error: Option<String>,
    connecting: Option<String>,
}

impl PrinterStore {
    fn row(&self, preset: &PrinterPreset) -> PresetRow {
        PresetRow {
            preset_id: preset.preset_id.clone(),
            title: preset.library_name(),
            variant_title: preset
                .library_display_name
                .clone()
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| strip_variant_suffix(&preset.name)),
            manufacturer: preset.manufacturer.clone(),
            bits: preset
                .bit_depth
                .as_ref()
                .map(|depth| depth.bits.round() as i64),
            bit_description: preset
                .bit_depth
                .as_ref()
                .and_then(|depth| depth.description.clone()),
            platform: preset.platform_badge.as_ref().and_then(|badge| {
                let text = badge.text.trim();
                if text.is_empty() {
                    None
                } else {
                    Some((
                        text.to_owned(),
                        badge
                            .color
                            .as_deref()
                            .and_then(parse_hex_color)
                            .unwrap_or(0x0ea5e9),
                    ))
                }
            }),
            added: self.is_added(&preset.preset_id),
            is_generic: preset.is_generic(),
            opens_setup: preset.opens_setup(),
        }
    }
}

fn view() -> PickerView {
    let store = store();

    // Default to the first manufacturer once the list is known
    // (`useEffect` in the library).
    let manufacturers = store.manufacturers();
    let manufacturer = match &store.manufacturer {
        Some(selected) if manufacturers.iter().any(|name| name == selected) => {
            Some(selected.clone())
        }
        _ => manufacturers.first().cloned(),
    };

    // `filteredPresets` (`OnboardingPrinterLibrary.tsx:86-97`): a search term
    // matches name / manufacturer / family and drops the manufacturer filter,
    // an empty term filters by the selected manufacturer only.
    let term = store.search.trim().to_lowercase();
    let is_searching = !term.is_empty();
    let filtered: Vec<&PrinterPreset> = store
        .library_presets()
        .filter(|preset| {
            let manufacturer_match = term.is_empty()
                || manufacturer.as_deref() == Some(preset.manufacturer.as_str());
            let search_match = term.is_empty()
                || preset.name.to_lowercase().contains(&term)
                || preset.manufacturer.to_lowercase().contains(&term)
                || preset
                    .family
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&term);
            manufacturer_match && search_match
        })
        .collect();
    // The flat grid keeps catalogue order.
    let flat: Vec<PresetRow> = filtered
        .iter()
        .map(|preset| store.row(preset))
        .collect();
    // `groupedPresets`: family order is first-seen order.
    let mut groups: Vec<(String, Vec<PresetRow>)> = Vec::new();
    for preset in filtered {
        let family = preset
            .family
            .clone()
            .filter(|family| !family.trim().is_empty())
            .unwrap_or_else(|| "Other".to_owned());
        let row = store.row(preset);
        // `position` ends its borrow before the arms, so the `None` arm can
        // push into `groups` (a `match` on `iter_mut()` keeps the borrow live
        // in every arm and fails to compile).
        let existing = groups
            .iter()
            .position(|(name, _)| name.as_str() == family.as_str());
        match existing {
            Some(index) => groups[index].1.push(row),
            None => groups.push((family, vec![row])),
        }
    }

    let selected_preset = store
        .selected_preset
        .as_deref()
        .and_then(|id| store.preset(id));
    let selected_label = selected_preset.map(PrinterPreset::library_name);
    let selected_opens_setup = selected_preset.is_some_and(PrinterPreset::opens_setup);

    let family_preset = store
        .family_preset
        .as_deref()
        .and_then(|id| store.preset(id))
        .cloned();
    let family_title = family_preset.as_ref().map(PrinterPreset::library_name);
    let variants = store
        .family_preset
        .as_deref()
        .map(|id| {
            store
                .variants_of(id)
                .iter()
                .map(|preset| store.row(preset))
                .collect()
        })
        .unwrap_or_default();

    let materials = store
        .materials_for_active()
        .into_iter()
        .map(|material| MaterialRow {
            id: material.id.clone(),
            name: material.name.clone(),
            color: material.color,
            layer_height_mm: material.layer_height_mm,
        })
        .collect();

    let devices = store
        .devices
        .iter()
        .map(|device| {
            (
                device.display_name(),
                device.ip_address.clone(),
                device.port,
            )
        })
        .collect();

    PickerView {
        surface: store.surface,
        manufacturers,
        manufacturer,
        groups,
        selected: store.selected_preset.clone(),
        selected_label,
        selected_opens_setup,
        family_title,
        variants,
        materials,
        active_material: store.active_material.clone(),
        mode: store.mode,
        search: store.search.clone(),
        is_searching,
        manual_ip: store.manual_ip.clone(),
        flat,
        devices,
        scanning: store.scanning,
        scan_error: store.scan_error.clone(),
        detected: store
            .detected
            .as_deref()
            .and_then(|id| store.preset(id))
            .map(|preset| store.row(preset)),
        detected_device_name: store.detected_device_name.clone(),
        detect_error: store.detect_error.clone(),
        connecting: store.connecting.clone(),
    }
}

// ── Shared recipes ──────────────────────────────────────────────────────────

/// `stripVariantSuffix`: drop a trailing `· 8-bit` / `– 3 bit` so the row's
/// bit-depth chip does not repeat it.
fn strip_variant_suffix(name: &str) -> String {
    let trimmed = name.trim_end();
    for (index, ch) in trimmed.char_indices().rev() {
        if matches!(ch, '·' | '–' | '—' | '-') {
            let tail = trimmed[index + ch.len_utf8()..].trim();
            if is_bit_depth_tail(tail) {
                let head = trimmed[..index].trim();
                if !head.is_empty() {
                    return head.to_owned();
                }
            }
        }
    }
    name.to_owned()
}

fn is_bit_depth_tail(tail: &str) -> bool {
    let lower = tail.to_ascii_lowercase();
    let Some(digits) = lower.strip_suffix("bit") else {
        return false;
    };
    let digits = digits.trim_end();
    let digits = digits.strip_suffix('-').unwrap_or(digits).trim_end();
    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
}

/// The bit-depth chip's `(border, gradient start, gradient end)`.
///
/// Source: 8-bit uses `#22c55e`, 3-bit `#ef4444`, anything else
/// `--accent-secondary`; each is mixed into `#111827` / `#0b1220` for the
/// gradient.
fn bit_depth_colors(bits: Option<i64>) -> (u32, u32, u32) {
    match bits {
        Some(8) => (
            mix(0x22c55e, WHITE, 0.78),
            mix(0x22c55e, 0x111827, 0.44),
            mix(0x22c55e, 0x0b1220, 0.28),
        ),
        Some(3) => (
            mix(0xef4444, WHITE, 0.82),
            mix(0xef4444, 0x111827, 0.44),
            mix(0xef4444, 0x0b1220, 0.28),
        ),
        _ => (
            mix(ACCENT_SECONDARY, WHITE, 0.80),
            mix(ACCENT_SECONDARY, 0x111827, 0.48),
            mix(ACCENT_SECONDARY, 0x0b1220, 0.32),
        ),
    }
}

/// The small gradient chip that sits on a card / row.
fn bit_depth_chip(bits: Option<i64>, label: impl Into<SharedString>, height: f32) -> Div {
    let (border, start, end) = bit_depth_colors(bits);
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .h(px(height))
        .min_w(px(if height >= 18.0 { 44.0 } else { 46.0 }))
        .rounded(px(if height >= 18.0 { ROUNDED_MD } else { 4.0 }))
        .border_1()
        .border_color(rgb(border))
        .px(px(6.0))
        .text_size(px(if height >= 18.0 { 9.0 } else { 10.0 }))
        .font_weight(FontWeight::BOLD)
        .line_height(px(if height >= 18.0 { 9.0 } else { 10.0 }))
        .text_color(rgb(0xf8fafc))
        .bg(linear_gradient(
            135.0,
            linear_color_stop(rgb(start), 0.0),
            linear_color_stop(rgb(end), 1.0),
        ))
        .child(label.into())
}

/// `color-mix(in srgb, var(--surface-1), transparent 8%)` over the dialog's
/// `surface-0` panel.
fn translucent_surface_1() -> u32 {
    mix(SURFACE_1, SURFACE_0, 0.92)
}

// ── Library surface ─────────────────────────────────────────────────────────

fn library_body(view: &PickerView, entity: WeakEntity<DragonFruit>) -> Div {
    let sidebar = library_sidebar(view, entity.clone());
    let grid = library_grid(view, entity.clone());

    let mut body = div().flex().flex_col().gap(px(12.0)).child(
        div()
            .flex()
            .h(px(if view.materials.is_empty() {
                480.0
            } else {
                420.0
            }))
            .min_h(px(0.0))
            .rounded(px(ROUNDED_MD))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .overflow_hidden()
            .child(sidebar)
            .child(
                div()
                    .id("printer-library-grid")
                    .flex_1()
                    .min_w(px(0.0))
                    .min_h(px(0.0))
                    .overflow_y_scroll()
                    .p(px(12.0))
                    .child(grid),
            ),
    );

    if !view.materials.is_empty() {
        body = body.child(material_section(view, entity));
    }
    body
}

fn library_sidebar(view: &PickerView, entity: WeakEntity<DragonFruit>) -> Div {
    // `p-2` search band, `border-b`. The source draws a `Search` glyph at
    // `left-3` inside the box and pads the text to 2.5rem; the shared `.ui-input`
    // element's text inset is fixed at 10px, so the glyph would sit on top of the
    // first characters and is left out (see the module deviations).
    let search = div()
        .p(px(8.0))
        .border_b_1()
        .border_color(rgb(BORDER_SUBTLE))
        .child({
            let search_entity = entity.clone();
            div().w_full().child(text_input(
                "printer-library-search",
                view.search.clone(),
                "Search printers",
                false,
                32.0,
                move |text, _, cx| {
                    let text = text.to_owned();
                    edit(move |store| store.search = text);
                    search_entity.update(cx, |_, cx| cx.notify()).ok();
                },
            ))
        });

    let mut list = div()
        .id("printer-library-manufacturers")
        .flex_1()
        .min_h(px(0.0))
        .overflow_y_scroll()
        .p(px(6.0))
        .flex()
        .flex_col()
        .gap(px(4.0));

    for manufacturer in &view.manufacturers {
        let active = view.manufacturer.as_deref() == Some(manufacturer.as_str());
        let click_manufacturer = manufacturer.clone();
        let click = click_edit(&entity, move |store: &mut PrinterStore| {
            // `setSelectedManufacturer` + `setSearch('')`; the source keeps the
            // card selection, and `handleAdd` resolves it in the full set.
            store.manufacturer = Some(click_manufacturer.clone());
            store.search.clear();
        });
        list = list.child(
            div()
                .id(format!("printer-manufacturer-{manufacturer}"))
                .w_full()
                .rounded(px(ROUNDED_MD))
                .border_1()
                .border_color(rgb(if active {
                    mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.65)
                } else {
                    BORDER_SUBTLE
                }))
                .bg(rgb(if active {
                    mix(ACCENT_SECONDARY, SURFACE_1, 0.12)
                } else {
                    SURFACE_1
                }))
                .px(px(10.0))
                .py(px(8.0))
                .text_size(px(FONT_MD))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(if active { TEXT_STRONG } else { TEXT_MUTED }))
                .cursor_pointer()
                .child(manufacturer.clone())
                .on_click(click),
        );
    }

    div()
        .w(px(220.0))
        .flex_shrink_0()
        .flex()
        .flex_col()
        .min_h(px(0.0))
        .border_r_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(translucent_surface_1()))
        .child(search)
        .child(list)
}

fn library_grid(view: &PickerView, entity: WeakEntity<DragonFruit>) -> Div {
    if view.groups.is_empty() {
        // `No printers match your search.`
        return div()
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(12.0))
            .text_color(rgb(TEXT_MUTED))
            .child("No printers match your search.");
    }

    if view.is_searching {
        // `isSearching`: one flat `grid-cols-[repeat(auto-fill,minmax(176px,1fr))]`
        // of every match, without the family sections.
        let mut cards = div().flex().flex_wrap().gap(px(10.0));
        for row in &view.flat {
            cards = cards.child(preset_card(row, view, entity.clone()));
        }
        return cards;
    }

    let mut column = div().flex().flex_col().gap(px(8.0));
    for (family, rows) in &view.groups {
        // The family rule: a hairline that fades in from the edges.
        let line_color = mix(TEXT_MUTED, WHITE, 0.72);
        let rule = div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .mb(px(8.0))
            .child(div().h(px(1.0)).flex_1().bg(linear_gradient(
                90.0,
                linear_color_stop(rgb(translucent_surface_1()), 0.0),
                linear_color_stop(rgb(line_color), 1.0),
            )))
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(px(FONT_MD))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(TEXT_MUTED))
                    .child(family.clone()),
            )
            .child(div().h(px(1.0)).flex_1().bg(linear_gradient(
                90.0,
                linear_color_stop(rgb(line_color), 0.0),
                linear_color_stop(rgb(translucent_surface_1()), 1.0),
            )));

        let mut cards = div().flex().flex_wrap().gap(px(10.0));
        for row in rows {
            cards = cards.child(preset_card(row, view, entity.clone()));
        }
        column = column.child(div().flex().flex_col().child(rule).child(cards));
    }
    column
}

fn preset_card(
    row: &PresetRow,
    view: &PickerView,
    entity: WeakEntity<DragonFruit>,
) -> Stateful<Div> {
    let selected = view.selected.as_deref() == Some(row.preset_id.as_str());
    let preset_id = row.preset_id.clone();
    let opens_setup = row.opens_setup;
    let click = click_edit(&entity, move |store: &mut PrinterStore| {
        if opens_setup {
            // The library opens Initial Setup for variant families / presets
            // with a network detect path instead of selecting them.
            store.family_preset = Some(preset_id.clone());
            store.surface = Surface::Setup;
            store.mode = SetupMode::Manual;
            store.selected_preset = None;
            store.detected = None;
            store.detected_device_name = None;
            store.detect_error = None;
            store.pending_network = None;
        } else {
            store.selected_preset = if store.selected_preset.as_deref() == Some(preset_id.as_str())
            {
                None
            } else {
                Some(preset_id.clone())
            };
        }
    });

    // `rounded-lg border p-2.5`, accent fill while selected.
    let mut card = div()
        .id(format!("printer-card-{}", row.preset_id))
        .w(px(176.0))
        .flex()
        .flex_col()
        .rounded(px(ROUNDED_LG))
        .border_1()
        .border_color(rgb(if selected {
            mix(ACCENT, BORDER_SUBTLE, 0.60)
        } else {
            BORDER_SUBTLE
        }))
        .bg(rgb(if selected {
            mix(ACCENT, SURFACE_1, 0.14)
        } else {
            SURFACE_1
        }))
        .p(px(10.0))
        .cursor_pointer()
        .on_click(click);

    // `relative flex h-[132px] items-center justify-center overflow-hidden
    // rounded-md border` on `#2b3039`.
    let mut art = div()
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .h(px(132.0))
        .w_full()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(0x2b3039))
        .overflow_hidden();

    // No printer artwork ships in the GPUI asset set, so this is always the
    // source's own fallback (deviation 4).
    art = art.child(icon(
        if row.is_generic {
            "printer"
        } else {
            "image-plus"
        },
        20.0,
        TEXT_MUTED,
    ));

    if selected {
        // `absolute left-1 top-1 h-5 w-5 rounded-full` in the accent.
        art = art.child(
            div()
                .absolute()
                .left(px(4.0))
                .top(px(4.0))
                .flex()
                .items_center()
                .justify_center()
                .w(px(20.0))
                .h(px(20.0))
                .rounded_full()
                .bg(rgb(ACCENT))
                .child(icon("check", 12.0, 0x0a0f0a)),
        );
    }
    if let Some((text, color)) = &row.platform {
        art = art.child(
            div()
                .absolute()
                .right(px(4.0))
                .top(px(4.0))
                .flex()
                .items_center()
                .justify_center()
                .h(px(18.0))
                .min_w(px(44.0))
                .rounded(px(ROUNDED_MD))
                .px(px(6.0))
                .text_size(px(9.0))
                .font_weight(FontWeight::BOLD)
                .line_height(px(9.0))
                .text_color(rgb(0xffffff))
                .bg(linear_gradient(
                    135.0,
                    linear_color_stop(rgb(mix(*color, WHITE, 0.86)), 0.0),
                    linear_color_stop(rgb(mix(*color, BLACK, 0.82)), 1.0),
                ))
                .child(text.clone()),
        );
    }
    // The card chip shows only a non-8-bit depth (`bitDepthLabel`).
    if let Some(bits) = row.bits.filter(|bits| *bits != 8) {
        art = art.child(
            div()
                .absolute()
                .right(px(4.0))
                .bottom(px(4.0))
                .child(bit_depth_chip(Some(bits), format!("{bits} Bit"), 18.0)),
        );
    }

    card = card.child(art).child(
        div()
            .mt(px(10.0))
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .child(
                div()
                    .text_size(px(12.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .line_height(px(14.0))
                    .text_color(rgb(TEXT_STRONG))
                    .child(fade_text(row.title.clone())),
            )
            .child(
                div()
                    .mt(px(2.0))
                    .text_size(px(FONT_XS))
                    .text_color(rgb(TEXT_MUTED))
                    .child(fade_text(row.manufacturer.clone())),
            ),
    );
    card
}

/// The material rows for the active printer, mirroring the material tab's
/// `Profile` list (`ProfileSettingsModal.tsx` around line 4940): a selectable
/// row per material with its swatch and layer height.
fn material_section(view: &PickerView, entity: WeakEntity<DragonFruit>) -> Div {
    let mut rows = div().flex().flex_wrap().gap(px(8.0));
    for material in &view.materials {
        let active = view.active_material.as_deref() == Some(material.id.as_str());
        let id = material.id.clone();
        let click = click_edit(&entity, move |store: &mut PrinterStore| {
            store.active_material = Some(id.clone());
        });
        let mut row = div()
            .id(format!("printer-material-{}", material.id))
            .flex()
            .items_center()
            .gap(px(8.0))
            .rounded(px(ROUNDED_MD))
            .border_1()
            .border_color(rgb(if active {
                mix(ACCENT, BORDER_SUBTLE, 0.70)
            } else {
                BORDER_SUBTLE
            }))
            .bg(rgb(if active {
                mix(ACCENT, SURFACE_1, 0.11)
            } else {
                SURFACE_1
            }))
            .px(px(10.0))
            .py(px(8.0))
            .text_size(px(12.0))
            .text_color(rgb(if active { TEXT_STRONG } else { TEXT_MUTED }))
            .cursor_pointer()
            .on_click(click);
        if let Some(color) = material.color {
            // `h-3.5 w-3.5 rounded-[4px]` swatch with a 1px inner edge.
            row = row.child(
                div()
                    .w(px(14.0))
                    .h(px(14.0))
                    .rounded(px(4.0))
                    .bg(rgb(color))
                    .border_1()
                    .border_color(rgba(0x00000059)),
            );
        }
        row = row
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(material.name.clone()),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(rgb(TEXT_MUTED))
                    // `Math.round(layerHeightMm * 1000) μm`.
                    .child(format!(
                        "{} μm",
                        (material.layer_height_mm * 1000.0).round() as i64
                    )),
            );
        rows = rows.child(row);
    }

    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .child("MATERIAL"),
        )
        .child(rows)
}

fn library_actions(view: &PickerView, entity: WeakEntity<DragonFruit>) -> Div {
    let label: SharedString = view
        .selected_label
        .clone()
        .unwrap_or_else(|| "Choose a printer to add".to_owned())
        .into();

    let can_add = view.selected.is_some();
    let spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Auto)
        .radius(ROUNDED_MD)
        .padding_x(12.0)
        .padding_y(0.0, 0.0)
        .gap(6.0)
        .icon(
            "plus",
            14.0,
            if can_add {
                ACCENT_SECONDARY_CONTRAST
            } else {
                TEXT_MUTED
            },
        )
        .fill(
            if can_add {
                SECONDARY_BUTTON_SURFACE
            } else {
                SURFACE_1
            },
            if can_add {
                mix(SECONDARY_BUTTON_SURFACE, WHITE, 0.86)
            } else {
                BORDER_SUBTLE
            },
            if can_add {
                ACCENT_SECONDARY_CONTRAST
            } else {
                TEXT_MUTED
            },
        );

    let add: AnyElement = if can_add {
        let selected = view.selected.clone().unwrap_or_default();
        let opens_setup = view.selected_opens_setup;
        let click = click_edit(&entity, move |store: &mut PrinterStore| {
            if opens_setup {
                store.family_preset = Some(selected.clone());
                store.surface = Surface::Setup;
                store.mode = SetupMode::Manual;
                store.detected = None;
                store.detected_device_name = None;
                store.detect_error = None;
            } else {
                store.add_preset(&selected);
                store.selected_preset = None;
            }
        });
        button_with("printer-add", "Add printer", &spec)
            .h(px(32.0))
            .on_click(click)
            .into_any_element()
    } else {
        // `aria-disabled` keeps the secondary recipe with the muted fill.
        button_with("printer-add", "Add printer", &spec)
            .h(px(32.0))
            .cursor_not_allowed()
            .into_any_element()
    };

    div()
        .flex()
        .w_full()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .child(
            div()
                .text_size(px(12.0))
                .text_color(rgb(TEXT_MUTED))
                .child(label),
        )
        .child(add)
}

// ── Initial Setup surface ───────────────────────────────────────────────────

fn setup_body(view: &PickerView, entity: WeakEntity<DragonFruit>) -> Div {
    // `!networkAdapter && variants.length === 0`.
    if view.variants.is_empty() {
        return div()
            .text_size(px(12.0))
            .text_color(rgb(TEXT_MUTED))
            .child("This printer has no setup options.");
    }

    let content = match view.mode {
        SetupMode::Manual => variant_rows(view, entity.clone()),
        SetupMode::Scan => scan_body(view, entity.clone()),
        SetupMode::Ip => ip_body(view, entity),
    };
    // `overflow_y_scroll` needs an element id, and an id makes the element
    // stateful; the extra plain wrapper keeps this function returning `Div`.
    div().child(
        div()
            .id("printer-picker-body")
            .max_h(px(420.0))
            .min_h(px(0.0))
            .overflow_y_scroll()
            .child(content),
    )
}

fn variant_rows(view: &PickerView, entity: WeakEntity<DragonFruit>) -> Div {
    let mut list = div().flex().flex_col().gap(px(8.0));
    for row in &view.variants {
        let id = row.preset_id.clone();
        let added = row.added;
        let click = click_edit(&entity, move |store: &mut PrinterStore| {
            // `handleSelectVariant`: add the concrete variant and return to
            // the library, where the profile is now active.
            store.add_preset(&id);
            store.surface = Surface::Library;
            store.family_preset = None;
            store.detected = None;
            store.detected_device_name = None;
            store.pending_network = None;
        });

        // `flex w-full items-center gap-3 rounded-xl border px-3 py-2.5`.
        let mut row_el = div()
            .id(format!("printer-variant-{}", row.preset_id))
            .flex()
            .items_center()
            .gap(px(12.0))
            .rounded(px(12.0))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(mix(SURFACE_1, SURFACE_0, 0.60)))
            .px(px(12.0))
            .py(px(10.0));

        // `h-6 min-w-[46px] rounded border px-1.5 text-[10px] font-bold`.
        row_el = row_el.child(bit_depth_chip(
            row.bits,
            row.bits
                .map(|bits| format!("{bits}-bit"))
                .unwrap_or_default(),
            24.0,
        ));

        let mut text = div().flex_1().min_w(px(0.0)).flex().flex_col().child(
            div()
                .text_size(px(12.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child(fade_text(row.variant_title.clone())),
        );
        if let Some(description) = &row.bit_description {
            text = text.child(
                div()
                    .mt(px(2.0))
                    .text_size(px(11.0))
                    .text_color(rgb(TEXT_MUTED))
                    .child(fade_text(description.clone())),
            );
        }
        row_el = row_el.child(text);

        if added {
            // `Added` in the source's uppercase tracking-wide caption.
            row_el = row_el.child(
                div()
                    .flex_shrink_0()
                    .text_size(px(10.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(TEXT_MUTED))
                    .child("ADDED"),
            );
        } else {
            row_el = row_el.child(icon("chevron-right", 16.0, TEXT_MUTED));
        }

        if added {
            row_el = row_el.opacity(0.55).cursor_not_allowed();
        } else {
            row_el = row_el.cursor_pointer().on_click(click);
        }
        list = list.child(row_el);
    }
    list
}

fn scan_body(view: &PickerView, entity: WeakEntity<DragonFruit>) -> Div {
    let preset_name = view
        .family_title
        .clone()
        .unwrap_or_else(|| "printer".to_owned());

    // The status card: `rounded-xl border px-3 py-2.5` on
    // `color-mix(surface-1, surface-0 30%)`.
    let card_bg = mix(SURFACE_1, SURFACE_0, 0.70);
    let mut status = div()
        .rounded(px(12.0))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(card_bg))
        .px(px(12.0))
        .py(px(10.0));

    if let Some(detected) = &view.detected {
        let display_name = view
            .detected_device_name
            .clone()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| detected.variant_title.clone());
        status = status.child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child(format!("Detected: {display_name}")),
                )
                .child(
                    div()
                        .mt(px(2.0))
                        .text_size(px(11.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child(if view.detected_device_name.is_some() {
                            detected.variant_title.clone()
                        } else {
                            "The printer reported this variant.".to_owned()
                        }),
                ),
        );
        if detected.added {
            status = status.child(
                div()
                    .mt(px(6.0))
                    .text_size(px(11.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(TEXT_MUTED))
                    .child("This model is already added."),
            );
        }
    } else if view.connecting.is_some() {
        status = status.child(
            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                .child(spinner(16.0))
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Detecting model…"),
                ),
        );
    } else if view.scanning {
        status = status.child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child(fade_text(format!(
                            "Scanning your network for {preset_name} printers…"
                        ))),
                )
                .child(progress_bar(0.0, ProgressSize::Xs, true)),
        );
    } else if !view.devices.is_empty() {
        let count = view.devices.len();
        let found = if count == 1 {
            "1 printer found".to_owned()
        } else {
            format!("{count} printers found")
        };
        status = status.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.0))
                .child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child(found),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Select a printer to detect its model."),
                ),
        );
    } else {
        let rescan = click_edit(&entity, |store: &mut PrinterStore| {
            // `runScan` needs the plugin network adapter, which this port does
            // not have (deviation 2): the scan stays empty.
            store.mode = SetupMode::Scan;
            store.scan_error = None;
        });
        let message = view
            .scan_error
            .clone()
            .unwrap_or_else(|| "No printers found.".to_owned());
        status = status.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.0))
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child(message),
                )
                .child(
                    button(
                        "printer-scan-again",
                        "Scan again",
                        ButtonVariant::Secondary,
                        ButtonSize::Auto,
                    )
                    .h(px(24.0))
                    .px(px(10.0))
                    .pt(px(0.0))
                    .pb(px(0.0))
                    .rounded(px(ROUNDED_SM))
                    .text_size(px(11.0))
                    .on_click(rescan),
                ),
        );
    }

    let mut body = div().flex().flex_col().gap(px(12.0)).child(status);

    // The device list (`!detectedVariant && devices.length > 0`).
    if view.detected.is_none() && !view.devices.is_empty() {
        let mut list = div().flex().flex_col().gap(px(6.0));
        for (name, address, port) in &view.devices {
            let host = address.clone();
            let port = *port;
            let click = click_edit(&entity, move |store: &mut PrinterStore| {
                // `connectAndDetect` needs the plugin adapter; without it the
                // source's own connect error is what remains.
                store.detect_error = Some(format!("Couldn't connect to {host}"));
                store.pending_network = Some(NetworkContext {
                    host: host.clone(),
                    port,
                    printer_name: None,
                    mode: None,
                });
            });
            let mut row = div()
                .id(format!("printer-device-{address}-{}", port.unwrap_or(0)))
                .flex()
                .items_center()
                .gap(px(12.0))
                .rounded(px(12.0))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(mix(SURFACE_1, SURFACE_0, 0.60)))
                .px(px(12.0))
                .py(px(10.0))
                .cursor_pointer()
                .on_click(click)
                .child(icon("printer", 16.0, ACCENT_SECONDARY));
            let text = div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child(fade_text(name.clone())),
                )
                .child(
                    div()
                        .mt(px(2.0))
                        .text_size(px(11.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child(match port {
                            Some(port) => format!("{address}:{port}"),
                            None => address.clone(),
                        }),
                );
            row = row
                .child(text)
                .child(icon("chevron-right", 16.0, TEXT_MUTED));
            list = list.child(row);
        }
        body = body.child(list);
    }

    if view.detected.is_none() {
        if let Some(error) = &view.detect_error {
            body = body.child(
                div()
                    .text_size(px(11.0))
                    .line_height(relative(1.3))
                    .text_color(rgb(0xf87171))
                    .child(error.clone()),
            );
        }
    }
    body
}

fn ip_body(view: &PickerView, entity: WeakEntity<DragonFruit>) -> Div {
    let mut body = div().flex().flex_col().gap(px(12.0));

    if let Some(detected) = &view.detected {
        let display_name = view
            .detected_device_name
            .clone()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| detected.variant_title.clone());
        body = body.child(
            div()
                .rounded(px(12.0))
                .border_1()
                .border_color(rgb(mix(ACCENT, BORDER_SUBTLE, 0.45)))
                .bg(rgb(mix(ACCENT, SURFACE_1, 0.08)))
                .px(px(12.0))
                .py(px(10.0))
                .child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child(format!("Detected: {display_name}")),
                )
                .child(
                    div()
                        .mt(px(2.0))
                        .text_size(px(11.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child(if view.detected_device_name.is_some() {
                            detected.variant_title.clone()
                        } else {
                            "The printer reported this variant.".to_owned()
                        }),
                ),
        );
    } else {
        body = body.child(manual_ip_card(view, entity));
    }
    body
}

/// `ManualIpEntryCard` (`SetupMethodChooser.tsx:106-160`): the IP box commits
/// every keystroke into `networkIpAddress`, Enter connects, and Connect stays in
/// the source's disabled state while the host is blank.
///
/// The connect action still cannot reach a network adapter in this slice
/// (deviation 2), so pressing it clears nothing and reports nothing — the box
/// and its enabled/disabled state are the source's.
fn manual_ip_card(view: &PickerView, entity: WeakEntity<DragonFruit>) -> Div {
    let connecting = view.connecting.is_some();
    let can_connect = !view.manual_ip.trim().is_empty();
    let connect: AnyElement = if connecting {
        let spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Auto)
            .radius(999.0)
            .padding_x(12.0)
            .padding_y(0.0, 0.0)
            .gap(4.0)
            .fill(SURFACE_1, mix(BORDER_SUBTLE, BLACK, 0.90), TEXT_MUTED);
        button_with_content(
            "printer-manual-connect",
            &spec,
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .child(spinner(14.0))
                .child("Connecting…"),
        )
        .h(px(34.0))
        .opacity(0.45)
        .into_any_element()
    } else {
        let spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Auto)
            .radius(999.0)
            .padding_x(12.0)
            .padding_y(0.0, 0.0)
            .gap(4.0)
            .icon("wifi", 14.0, ACCENT_SECONDARY)
            .fill(SURFACE_1, BORDER_SUBTLE, ACCENT_SECONDARY);
        let connect = button_with_content(
            "printer-manual-connect",
            &spec,
            div().flex().items_center().child("Connect"),
        )
        .h(px(34.0));
        if can_connect {
            connect
                .on_click(click_edit(&entity, |_store: &mut PrinterStore| {
                    // `handleManualIpConnect` needs the network adapter this
                    // slice does not ship (deviation 2).
                }))
                .into_any_element()
        } else {
            // `disabled={isConnecting || !value.trim()}`.
            connect
                .opacity(0.45)
                .cursor_not_allowed()
                .into_any_element()
        }
    };

    div()
        .rounded(px(12.0))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(14.0))
        .child(
            div()
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .child("MANUAL IP ENTRY"),
        )
        .child(
            div()
                .mt(px(8.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .child({
                    let ip_entity = entity.clone();
                    div().min_w(px(0.0)).flex_1().child(text_input(
                        "printer-manual-ip",
                        view.manual_ip.clone(),
                        "192.168.1.100",
                        false,
                        34.0,
                        move |text, _, cx| {
                            let text = text.to_owned();
                            edit(move |store| store.manual_ip = text);
                            ip_entity.update(cx, |_, cx| cx.notify()).ok();
                        },
                    ))
                })
                .child(connect),
        )
}

fn setup_actions(view: &PickerView, entity: WeakEntity<DragonFruit>) -> Div {
    // `SetupModeButton`: it names the mode it switches to.
    let label = if view.mode == SetupMode::Scan {
        "Manual"
    } else {
        "Auto"
    };
    let switch_spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Auto)
        .radius(999.0)
        .padding_x(12.0)
        .padding_y(0.0, 0.0);
    let switch = button_with("printer-setup-mode", label, &switch_spec)
        .h(px(32.0))
        .on_click(click_edit(&entity, |store: &mut PrinterStore| {
            // `mode === 'scan' ? setMode('ip') : (setMode('scan'); runScan())`.
            if store.mode == SetupMode::Scan {
                store.mode = SetupMode::Ip;
            } else {
                store.mode = SetupMode::Scan;
                store.scan_error = None;
                store.scanning = false;
                store.devices.clear();
            }
        }));

    let cancel_spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Auto)
        .radius(999.0)
        .padding_x(12.0)
        .padding_y(0.0, 0.0);
    let cancel = button_with("printer-setup-cancel", "Cancel", &cancel_spec)
        .h(px(32.0))
        .on_click(dismiss(&entity, true));

    let mut right = div().flex().items_center().gap(px(8.0)).child(cancel);

    // `showNetworkMode && detectedVariant && !detectedAlreadyAdded`.
    if view.mode == SetupMode::Scan {
        if let Some(detected) = &view.detected {
            if !detected.added {
                let id = detected.preset_id.clone();
                // `ACCENT_ACTION_STYLE`.
                let spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Auto)
                    .radius(999.0)
                    .padding_x(12.0)
                    .padding_y(0.0, 0.0)
                    .gap(6.0)
                    .icon("check", 14.0, ACCENT_SECONDARY)
                    .fill(
                        mix(ACCENT_SECONDARY, SURFACE_1, 0.08),
                        mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.42),
                        ACCENT_SECONDARY,
                    );
                let add = button_with("printer-confirm-detected", "Add printer", &spec)
                    .h(px(32.0))
                    .on_click(click_edit(&entity, move |store: &mut PrinterStore| {
                        store.add_preset(&id);
                        store.surface = Surface::Library;
                        store.family_preset = None;
                        store.detected = None;
                        store.detected_device_name = None;
                        store.pending_network = None;
                    }));
                right = right.child(add);
            }
        }
    }

    div()
        .flex()
        .w_full()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .child(switch)
        .child(right)
}

// ── The printer-profiles modal's view of this store ─────────────────────────
//
// `features::printer_profiles` is the `Printer & Material Profiles` modal. The
// add flow stays here — the library and `Initial Setup` — but the modal's rail
// lists the installed profiles and owns their order, so it reads this store's
// list and keeps its selection, moves and removals in step. These functions are
// that seam; every surface above is unchanged.

/// One installed profile, as the modal needs to identify it: the store's id and
/// the preset it was created from (`officialPresetId`).
pub struct InstalledProfile {
    pub id: String,
    pub official_preset_id: String,
}

impl PrinterStore {
    /// `movePrinterProfile` (`profileStore.ts:2558`): insert `id` before
    /// `before_id`, or at the end when there is no such profile.
    fn move_before(&mut self, id: &str, before_id: Option<&str>) {
        let Some(index) = self.profiles.iter().position(|profile| profile.id == id) else {
            return;
        };
        let moved = self.profiles.remove(index);
        let before = before_id
            .map(str::trim)
            .filter(|before| !before.is_empty())
            .and_then(|before| self.profiles.iter().position(|profile| profile.id == before));
        match before {
            Some(index) => self.profiles.insert(index, moved),
            None => self.profiles.push(moved),
        }
    }

    /// `removePrinterProfile` (`profileStore.ts:2457`), minus the material
    /// sharing bookkeeping the modal keeps itself: drop the profile and its own
    /// materials, and move the active printer to the first survivor.
    fn remove(&mut self, id: &str) {
        if !self.profiles.iter().any(|profile| profile.id == id) {
            return;
        }
        self.profiles.retain(|profile| profile.id != id);
        self.materials
            .retain(|material| material.printer_profile_id != id);
        if self.active_profile.as_deref() == Some(id) {
            self.active_profile = self.profiles.first().map(|profile| profile.id.clone());
        }
        if self.active_profile.is_none() {
            self.active_material = None;
            return;
        }
        self.ensure_active_material();
    }
}

/// `profileState.printerProfiles`, in rail order.
pub fn installed_profiles() -> Vec<InstalledProfile> {
    store()
        .profiles
        .iter()
        .map(|profile| InstalledProfile {
            id: profile.id.clone(),
            official_preset_id: profile.official_preset_id.clone(),
        })
        .collect()
}

/// `profileState.activePrinterProfileId`.
pub fn active_profile_id() -> Option<String> {
    store().active_profile.clone()
}

/// `setActivePrinterProfile`.
pub fn select_profile(id: &str) {
    let mut guard = store();
    if guard.profiles.iter().any(|profile| profile.id == id) {
        guard.active_profile = Some(id.to_owned());
        guard.ensure_active_material();
    }
}

/// `movePrinterProfile`.
pub fn move_profile(id: &str, before_id: Option<&str>) {
    store().move_before(id, before_id);
}

/// `removePrinterProfile`.
pub fn remove_profile(id: &str) {
    store().remove(id);
}
