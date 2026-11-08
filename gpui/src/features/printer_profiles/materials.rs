//! The modal's Material Settings surface: the manufacturer / material-type /
//! profile columns for the selected printer, the Multi-Tank tank grid, the
//! material editor and create dialogs, and the add / duplicate / lock / delete
//! flows.
//!
//! Ported from `src/components/settings/ProfileSettingsModal.tsx`:
//!
//! * the Multi-Tank section (`line 4497`): `Tank 1..N` selects grouped by
//!   manufacturer, the source's `Math.max(4, selectedPrinter.tankCount ?? 2)`
//!   slots, the red unassigned state, and the green/danger link between two
//!   adjacent assigned tanks (`tankMaterialsDisagree`,
//!   `src/features/profiles/tankMaterialCompatibility.ts`);
//! * the Material Settings section (`line 4652`): the
//!   `Manufacturer | Material Type | Profile` grid, its selected-row recipe and
//!   the `Edit / Duplicate / Lock / Delete` action bar;
//! * `MaterialProfileFormSections` (`profileFormAtoms.tsx:993`): the identity
//!   card, the print-settings card and the scale-compensation card, inside the
//!   source's 920px editor and create dialogs;
//! * `ensureActiveMaterialForActivePrinter`'s fallback
//!   (`profileStore.ts:1621`): every printer's first material is the literal
//!   `Standard 405nm`; `model::materials_for` / `model::add_material` already
//!   mint it, so this pane only renders it.
//!
//! Every string, field label, value and colour traces to the React source.
//!
//! # PORT DEVIATIONS
//!
//! 1. **Text entry.** `Name` and `Manufacturer` are real `.ui-input` editors
//!    (`settings::fields::text_input`) that commit into the form's draft on
//!    every keystroke, as the source's `LabeledInput` does. Nothing reaches the
//!    store until the form's own Save, which is the source's draft contract.
//! 2. **Custom colours** need the source's `ColorSwatchInput` + native picker;
//!    GPUI has only a filled swatch. `Custom Color` commits the source's own
//!    fallback `#8a8f98`, and the row's swatch previews it. The ten named
//!    `MATERIAL_COLOR_PRESETS` are all selectable.
//! 3. **No material library, import or export.** They need the preset
//!    catalogue, the Tauri file bridge and the profile-bundle writer, none of
//!    which this slice has; the source's own `Library` / `Import` / `Export`
//!    buttons are therefore absent rather than inert. `New`, `Edit`,
//!    `Duplicate`, `Lock`/`Unlock` and `Delete` are the action bar.
//! 4. **No official-template updates.** `getOfficialMaterialProfileUpdates` and
//!    `applyOfficialMaterialProfileUpdate` read the template catalogue; without
//!    it the source's `Update Material` affordance never appears.
//! 5. **No anti-aliasing / plugin local-settings sections.** The source's
//!    editor stacks `MaterialAntiAliasingSection` and
//!    `PluginLocalMaterialSettingsSections` under the three cards ported here.
//!    Both need the plugin adapter (`getProfileLocalMaterialSettingsAdapter`)
//!    the port does not have; the model still carries the full
//!    `anti_aliasing_settings` and `local_settings_by_output` values untouched,
//!    so an editor for them can be added without a migration.
//! 6. **Tank comparison dialog.** The source's `octagon-alert` link opens a
//!    `Tank material settings do not match` dialog that can copy one material's
//!    layer stack onto another. The link and its message are ported; the
//!    dialog's copy action is not, because it writes the plugin-owned
//!    `transitionLayerCount` key inside `local_settings_by_output` for an output
//!    format this slice cannot resolve. The link stays a warning.
//! 7. **Select menus are inline.** GPUI has no portal, so a tank/dropdown menu
//!    renders under its trigger and scrolls with the pane (the widget's own
//!    note). `selectedDisplay` / `hideSelectedText` have no equivalent, so an
//!    unassigned tank's trigger carries the source's dash as a menu row
//!    (`Unassigned`) instead of a centred overlay.
//! 8. **Dialog overlays are pane-local.** `StructuredDialogModal` paints a
//!    `size_full` backdrop under its parent; this pane is that parent, so the
//!    editor/create dialogs cover the materials surface rather than the whole
//!    window (`fixed inset-0` in the source). The modal shell keeps working.
//!    The source's `max-h-[88vh]` editor is a fixed 620px, because this
//!    function gets no window measurement.
//! 9. The Multi-Tank grid is always four columns: the source's `xl:grid-cols-4`
//!    is the widest layout and the port has no window-width measurement. The
//!    material columns use the source's `1fr 1fr 1.25fr` flex split.
//! 10. `tracking-wide`, `custom-scrollbar`, `transition-*`,
//!     `.ui-modal-backdrop-enter` and hover brightness have no GPUI equivalent;
//!     the colour recipes are kept.
//! 11. The profile row's `FadeText` clips instead of fading
//!     (`gpui_ui::ui::fade_text`), and the owner/shared tooltips are our
//!     `ui::tooltip` because GPUI carries no `title`/`aria-*`.

#![allow(dead_code)]

use std::sync::{LazyLock, Mutex, MutexGuard};

use gpui::{
    App, ClickEvent, Div, ElementId, FontWeight, IntoElement, MouseButton, MouseDownEvent,
    SharedString, Stateful, WeakEntity, Window, div, prelude::*, px, rgb, rgba,
};
use gpui_core::icons::icon;
use gpui_core::theme::*;
use gpui_ui::ui::{
    ButtonSize, ButtonSpec, ButtonVariant, CheckboxSize, CheckboxTone, IconButtonSize,
    IconButtonVariant, IconTone, SelectOption, SelectOptionTone, button_with, checkbox_with,
    color_swatch, dialog_modal, fade_text, icon_button, select_dropdown, tooltip,
};

use crate::features::settings::fields::{number_on_blur, text_input};
use super::model::{self, MaterialProfile, PrinterProfile};
use crate::DragonFruit;

const SURFACE_0: u32 = 0x111216;
const SURFACE_1: u32 = 0x1a1b21;
const SURFACE_2: u32 = 0x23252e;
const BORDER_SUBTLE: u32 = 0x272a33;
const BORDER_STRONG: u32 = 0x353944;
const TEXT_STRONG: u32 = 0xf8f8fb;
const TEXT_MUTED: u32 = 0xc3c7cf;
const ACCENT: u32 = 0xec2a77;
const ACCENT_SECONDARY: u32 = 0xbaf72e;
const DANGER: u32 = 0xe45454;
const SUCCESS: u32 = 0x2eb67d;
const CROWN: u32 = 0xfbbf24;
const BLACK: u32 = 0x000000;

const FONT_XS: f32 = 12.0;
const FONT_SM: f32 = 13.0;
const FONT_MD: f32 = 14.0;
const LINE_HEIGHT_TIGHT: f32 = 1.2;
const LINE_HEIGHT_SNUG: f32 = 1.375;

/// `RESIN_FAMILY_OPTIONS` (`profileFormAtoms.tsx:48`), in the source's order.
const RESIN_FAMILIES: [(&str, &str); 6] = [
    ("standard", "Standard"),
    ("abs-like", "ABS-like"),
    ("tough", "Tough"),
    ("flexible", "Flexible"),
    ("engineering", "Engineering"),
    ("other", "Other"),
];

/// `CURRENCY_OPTIONS` (`profileFormAtoms.tsx:66`).
const CURRENCIES: [&str; 7] = ["USD", "EUR", "GBP", "CAD", "AUD", "JPY", "CNY"];

/// `MATERIAL_COLOR_PRESETS` (`profileFormAtoms.tsx`, the named swatches).
const COLOR_PRESETS: [(&str, &str); 10] = [
    ("#9ca3af", "Grey"),
    ("#f8fafc", "White"),
    ("#111827", "Black"),
    ("#3b82f6", "Blue"),
    ("#22c55e", "Green"),
    ("#ef4444", "Red"),
    ("#f59e0b", "Amber"),
    ("#ec4899", "Pink"),
    ("#a855f7", "Purple"),
    ("#14b8a6", "Teal"),
];

/// `CUSTOM_MATERIAL_COLOR` — a mode, not a colour.
const CUSTOM_COLOR: &str = "custom";
const CUSTOM_COLOR_FALLBACK: &str = "#8a8f98";

const TANK_MATERIALS_MATCH: &str = "Tank materials have compatible settings.";
const TANK_MATERIALS_MISMATCH: &str =
    "Tank materials have incompatible settings. Please click to resolve mismatches.";

/// Packed `0xRRGGBBAA` for [`rgba`].
fn with_alpha(color: u32, alpha: u8) -> u32 {
    (color << 8) | alpha as u32
}

// ── UI state ────────────────────────────────────────────────────────────────

/// `MaterialDraft` — `handleSaveMaterialEdits`'s fields plus the create draft's
/// (`ProfileSettingsModal.tsx:453` / `:471`). The identity fields are the
/// source's `editMaterialDraft`: name, brand, colour, currency, bottle, resin
/// family, scale and the print settings. The plugin-only anti-aliasing and
/// local-settings drafts are not edited here (deviation 5).
#[derive(Clone, Debug)]
struct MaterialDraft {
    name: String,
    brand: String,
    color: Option<String>,
    currency_code: String,
    bottle_price: f32,
    bottle_capacity_ml: f32,
    resin_family: String,
    scale_x: f32,
    scale_y: f32,
    scale_z: f32,
    layer_height_mm: f32,
    normal_exposure_sec: f32,
    bottom_exposure_sec: f32,
    bottom_layer_count: f32,
    lift_distance_mm: f32,
    lift_speed_mm_min: f32,
    retract_speed_mm_min: f32,
    compatible_printer_profile_ids: Vec<String>,
}

impl MaterialDraft {
    fn from_profile(material: &MaterialProfile) -> Self {
        Self {
            name: material.name.clone(),
            brand: material.brand.clone(),
            color: material.color.clone(),
            currency_code: if material.currency_code.is_empty() {
                "USD".to_owned()
            } else {
                material.currency_code.clone()
            },
            bottle_price: material.bottle_price,
            bottle_capacity_ml: material.bottle_capacity_ml,
            resin_family: material.resin_family.clone(),
            scale_x: material.scale_compensation_pct.x,
            scale_y: material.scale_compensation_pct.y,
            scale_z: material.scale_compensation_pct.z,
            layer_height_mm: material.layer_height_mm,
            normal_exposure_sec: material.normal_exposure_sec,
            bottom_exposure_sec: material.bottom_exposure_sec,
            bottom_layer_count: material.bottom_layer_count,
            lift_distance_mm: material.lift_distance_mm,
            lift_speed_mm_min: material.lift_speed_mm_min,
            retract_speed_mm_min: material.retract_speed_mm_min,
            compatible_printer_profile_ids: material.compatible_printer_profile_ids.clone(),
        }
    }

    /// `newMaterialDraft` (`ProfileSettingsModal.tsx:471`): `New Material`,
    /// `Default`, `USD`, 0 / 1000, standard, the zero scale and the
    /// `addMaterialProfile` print defaults.
    fn new_material() -> Self {
        Self {
            name: "New Material".to_owned(),
            brand: "Default".to_owned(),
            color: None,
            currency_code: "USD".to_owned(),
            bottle_price: 0.0,
            bottle_capacity_ml: 1000.0,
            resin_family: "standard".to_owned(),
            scale_x: 0.0,
            scale_y: 0.0,
            scale_z: 0.0,
            layer_height_mm: 0.05,
            normal_exposure_sec: 2.5,
            bottom_exposure_sec: 28.0,
            bottom_layer_count: 5.0,
            lift_distance_mm: 6.0,
            lift_speed_mm_min: 60.0,
            retract_speed_mm_min: 150.0,
            compatible_printer_profile_ids: Vec::new(),
        }
    }

    /// `handleSaveMaterialEdits` / `handleCreateMaterial`: empty identity falls
    /// back to the source's literals, then `updateMaterialProfile` writes the
    /// rest.
    fn apply_to(&self, material: &mut MaterialProfile) {
        let name = self.name.trim();
        if !name.is_empty() {
            material.name = name.to_owned();
        }
        let brand = self.brand.trim();
        material.brand = if brand.is_empty() {
            "Default".to_owned()
        } else {
            brand.to_owned()
        };
        let currency = self.currency_code.trim().to_uppercase();
        material.currency_code = if currency.is_empty() {
            "USD".to_owned()
        } else {
            currency
        };
        // `color: undefined` removes the colour (`updateMaterialProfile`'s
        // hasOwnProperty branch), which `Option` expresses directly.
        material.color = self.color.clone();
        material.bottle_price = self.bottle_price.max(0.0);
        material.bottle_capacity_ml = self.bottle_capacity_ml.max(0.0);
        material.resin_family = self.resin_family.clone();
        material.scale_compensation_pct.x = self.scale_x;
        material.scale_compensation_pct.y = self.scale_y;
        material.scale_compensation_pct.z = self.scale_z;
        material.layer_height_mm = self.layer_height_mm.max(0.0);
        material.normal_exposure_sec = self.normal_exposure_sec.max(0.0);
        material.bottom_exposure_sec = self.bottom_exposure_sec.max(0.0);
        material.bottom_layer_count = self.bottom_layer_count.max(0.0);
        material.lift_distance_mm = self.lift_distance_mm.max(0.0);
        material.lift_speed_mm_min = self.lift_speed_mm_min.max(0.0);
        material.retract_speed_mm_min = self.retract_speed_mm_min.max(0.0);
        material.compatible_printer_profile_ids = self
            .compatible_printer_profile_ids
            .iter()
            .filter(|id| id.as_str() != material.printer_profile_id)
            .cloned()
            .collect();
    }
}

/// The open editor/create dialog, keyed by printer so a rail switch drops the
/// draft (the source resets its material selection on `handlePickPrinter`).
struct OpenEditor {
    material_id: String,
    draft: MaterialDraft,
    /// The source's remembered editor tab is plugin-owned; this pane has no
    /// replacement tabs, so only the one dialog is modelled.
    _tab: (),
}

struct CompatDialog {
    open: bool,
    /// Which draft the sharing list edits (the editor or the create dialog).
    target: FormTarget,
}

#[derive(Default)]
struct MaterialsUi {
    printer_id: Option<String>,
    /// `selectedMaterialId` — the row the action bar targets.
    selected_material: Option<String>,
    /// `selectedManufacturer` / `selectedResinFamily`.
    manufacturer: Option<String>,
    resin_family: Option<String>,
    /// Which `select_dropdown` has its menu open, by element id.
    open_select: Option<String>,
    editor: Option<OpenEditor>,
    /// `isCreateMaterialOpen`, with the source's `newMaterialDraft`.
    create: Option<MaterialDraft>,
    /// `deleteConfirmTarget` for the material kind.
    delete_confirm: Option<(String, String)>,
    /// `showOfficialMaterialLockDialog` + `materialLockDialogIsOfficial`.
    lock_dialog: Option<bool>,
    compat: CompatDialog,
}

impl Default for CompatDialog {
    fn default() -> Self {
        Self {
            open: false,
            target: FormTarget::Editor,
        }
    }
}

static UI: LazyLock<Mutex<MaterialsUi>> = LazyLock::new(|| Mutex::new(MaterialsUi::default()));

fn ui() -> MutexGuard<'static, MaterialsUi> {
    UI.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn ui_edit<R>(f: impl FnOnce(&mut MaterialsUi) -> R) -> R {
    let mut guard = ui();
    f(&mut guard)
}

fn notify(entity: &WeakEntity<DragonFruit>, cx: &mut App) {
    entity.update(cx, |_, cx| cx.notify()).ok();
}

// ── Public entry ────────────────────────────────────────────────────────────

/// The modal's Material Settings pane, rendered below the printer rail for
/// `profile` (the selected printer). Writes go through [`super::model`]; the
/// entity only needs a notify so the modal re-reads the store.
///
/// `escape` is exported for the modal's `useEscapeToClose`: it unwinds this
/// pane's dialogs before the modal's own editor/confirm.
pub fn materials_panel(
    profile: PrinterProfile,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let view = view(&profile);

    let mut pane = div().relative().w_full().flex().flex_col().gap(px(12.0));

    if view.multi_tank {
        // The source puts Multi-Tank above Material Settings (`line 4497`).
        pane = pane.child(multi_tank_section(&view, &entity));
    }
    pane = pane.child(material_settings_section(&view, &entity));

    // The source's nested dialogs are `fixed inset-0`; with no portal they
    // paint inside this pane's bounds (deviation 8).
    pane.child(overlays(&view, &entity))
}

/// `useEscapeToClose` for this pane. Returns whether an overlay consumed the
/// key. The source unwinds the *top-most* layer first, which is the z-index
/// order: the compatible-machines list (`z-[120]`), then the delete
/// confirmation (`z-[76]`), the lock dialog (`z-[75]`), and finally the editor
/// and create dialogs (`z-[60]`).
pub fn escape() -> bool {
    let mut state = ui();
    if state.compat.open {
        state.compat.open = false;
        return true;
    }
    if state.delete_confirm.take().is_some() {
        return true;
    }
    if state.lock_dialog.take().is_some() {
        return true;
    }
    if state.editor.take().is_some() {
        return true;
    }
    if state.create.take().is_some() {
        return true;
    }
    false
}

/// Reset transient UI when the modal opens (`requestOpenProfileSettings`).
pub fn begin() {
    ui_edit(|state| {
        state.editor = None;
        state.create = None;
        state.delete_confirm = None;
        state.lock_dialog = None;
        state.compat.open = false;
        state.open_select = None;
    });
}

// ── Snapshot ────────────────────────────────────────────────────────────────

struct Branch {
    id: String,
    label: String,
}

struct View {
    printer_id: String,
    printer_name: String,
    multi_tank: bool,
    tank_count: u32,
    /// The source's `selectedPrinter.tankMaterials ?? []`, trimmed to
    /// `tankCount` as the store writes it.
    tank_materials: Vec<String>,
    output_format: String,
    materials: Vec<MaterialProfile>,
    /// The three-column filter state after the source's `useEffect` defaults.
    manufacturer: Option<String>,
    resin_family: Option<String>,
    manufacturers: Vec<String>,
    resin_families: Vec<String>,
    visible: Vec<MaterialProfile>,
    selected_material: Option<String>,
    profiles: Vec<PrinterProfile>,
}

/// One tank menu row: the material it assigns, a heading, or the source's
/// `Unassign Tank` command.
#[derive(Clone)]
enum TankChoice {
    Assign(String),
    Unassign,
    Ignore,
}

fn view(profile: &PrinterProfile) -> View {
    let materials = model::materials_for(&profile.id);
    let active = model::active_material_id();
    let profiles = model::profiles();

    let mut state = ui();
    if state.printer_id.as_deref() != Some(profile.id.as_str()) {
        state.printer_id = Some(profile.id.clone());
        state.selected_material = None;
        state.manufacturer = None;
        state.resin_family = None;
        state.open_select = None;
        state.editor = None;
        state.create = None;
        state.delete_confirm = None;
        state.lock_dialog = None;
        state.compat.open = false;

        // `handlePickPrinter` restores the printer's remembered material, then
        // the source's filtered first row. Model-side, the active material is
        // that remembered id.
        if let Some(active) = active
            .as_ref()
            .and_then(|id| materials.iter().find(|material| &material.id == id))
        {
            state.selected_material = Some(active.id.clone());
            state.manufacturer = Some(brand_of(active));
            state.resin_family = Some(active.resin_family.clone());
        }
    }

    let mut manufacturers: Vec<String> = materials.iter().map(brand_of).collect();
    // `availableManufacturers`: `Array.from(new Set(...)).sort(localeCompare)` —
    // exact values, so `sort` + `dedup` keeps one row per stored brand.
    manufacturers.sort();
    manufacturers.dedup();

    let manufacturer = state
        .manufacturer
        .clone()
        .filter(|brand| manufacturers.iter().any(|entry| entry == brand))
        .or_else(|| manufacturers.first().cloned());

    let mut resin_families: Vec<String> = Vec::new();
    for material in materials
        .iter()
        .filter(|material| Some(brand_of(material)) == manufacturer)
    {
        if !resin_families.contains(&material.resin_family) {
            resin_families.push(material.resin_family.clone());
        }
    }
    let resin_family = state
        .resin_family
        .clone()
        .filter(|family| resin_families.iter().any(|entry| entry == family))
        .or_else(|| resin_families.first().cloned());

    let visible: Vec<MaterialProfile> = materials
        .iter()
        .filter(|material| {
            Some(brand_of(material)) == manufacturer
                && Some(material.resin_family.clone()) == resin_family
        })
        .cloned()
        .collect();

    let selected_material = state
        .selected_material
        .clone()
        .filter(|id| visible.iter().any(|material| &material.id == id))
        .or_else(|| visible.first().map(|material| material.id.clone()));

    // Persist the defaults the source's `useEffect` writes back, so a rail
    // switch or a deleted row does not leave a stale id.
    state.manufacturer = manufacturer.clone();
    state.resin_family = resin_family.clone();
    state.selected_material = selected_material.clone();

    let tank_count = profile.tank_count_or_default();
    let mut tank_materials = profile.tank_materials.clone();
    tank_materials.truncate(tank_count as usize);

    View {
        printer_id: profile.id.clone(),
        printer_name: profile.name.clone(),
        // `multiTankMaterialPicker = outputFormatSupportsMultiTank(outputFormat)
        // && materialMode === 'multiTank'` (`:680`, gotcha 18). No built-in
        // format declares `supportsMultiTank`, so with the shipped catalogue the
        // section stays hidden, exactly as in the source.
        multi_tank: profile.multi_tank()
            && super::editor::supports_multi_tank(&profile.display.output_format),
        tank_count,
        tank_materials,
        output_format: profile.display.output_format.clone(),
        materials,
        manufacturer,
        resin_family,
        manufacturers,
        resin_families,
        visible,
        selected_material,
        profiles,
    }
}

fn brand_of(material: &MaterialProfile) -> String {
    let brand = material.brand.trim();
    if brand.is_empty() {
        "Default".to_owned()
    } else {
        brand.to_owned()
    }
}

fn material_by_id<'a>(view: &'a View, id: &str) -> Option<&'a MaterialProfile> {
    view.materials.iter().find(|material| material.id == id)
}

// ── Multi-Tank ──────────────────────────────────────────────────────────────

/// `tankMaterialShape`'s three values (`tankMaterialCompatibility.ts`): layer
/// height, bottom layer count and the transition-layer key inside
/// `localSettingsByOutput[outputFormat]`.
#[derive(Clone, Copy)]
struct TankShape {
    layer_height_mm: f32,
    initial_layers: f32,
    transition_layers: f32,
}

const TRANSITION_LAYER_KEYS: [&str; 3] = ["traLayers", "transitionLayers", "transitionLayerCount"];

fn transition_layers(material: &MaterialProfile, output_format: &str) -> f32 {
    let Some(by_output) = material.local_settings_by_output.as_object() else {
        return 0.0;
    };
    let normalized = output_format.trim();
    let settings = by_output
        .get(normalized)
        .or_else(|| {
            normalized
                .strip_prefix('.')
                .and_then(|stripped| by_output.get(stripped))
        })
        .and_then(|value| value.as_object());
    let Some(settings) = settings else {
        return 0.0;
    };
    for key in TRANSITION_LAYER_KEYS {
        if let Some(value) = settings.get(key) {
            // `Number(settings[key])`, so a numeric string counts too.
            let number = value
                .as_f64()
                .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()));
            if let Some(number) = number.filter(|number| number.is_finite()) {
                return number as f32;
            }
        }
    }
    0.0
}

fn tank_shape(material: &MaterialProfile, output_format: &str) -> TankShape {
    TankShape {
        layer_height_mm: material.layer_height_mm,
        initial_layers: material.bottom_layer_count,
        transition_layers: transition_layers(material, output_format),
    }
}

/// `tankMaterialValueMatches`: `|a - b| < 1e-6`, both finite.
fn tank_values_match(left: f32, right: f32) -> bool {
    left.is_finite() && right.is_finite() && (left - right).abs() < 1e-6
}

/// `tankMaterialsDisagree`.
fn tank_materials_disagree(
    left: &MaterialProfile,
    right: &MaterialProfile,
    output_format: &str,
) -> bool {
    let a = tank_shape(left, output_format);
    let b = tank_shape(right, output_format);
    !tank_values_match(a.layer_height_mm, b.layer_height_mm)
        || !tank_values_match(a.initial_layers, b.initial_layers)
        || !tank_values_match(a.transition_layers, b.transition_layers)
}

/// The source's `tankMaterialOptions`: a heading per manufacturer, the
/// materials sorted by name inside it, then the destructive `Unassign Tank`
/// command behind a rule.
fn tank_options(view: &View) -> (Vec<SelectOption>, Vec<TankChoice>) {
    let mut groups: Vec<(String, Vec<&MaterialProfile>)> = Vec::new();
    for material in &view.materials {
        let brand = brand_of(material);
        if let Some(group) = groups.iter_mut().find(|group| group.0 == brand) {
            group.1.push(material);
        } else {
            groups.push((brand, vec![material]));
        }
    }
    groups.sort_by_key(|(brand, _)| brand.to_lowercase());

    // The trigger has no custom selected display; a real `—` row gives the
    // unassigned tank the source's dash (deviation 7).
    let mut options = vec![SelectOption::new("", "—")];
    let mut choices = vec![TankChoice::Unassign];
    for (brand, mut materials) in groups {
        options.push(SelectOption::heading(brand.clone()));
        choices.push(TankChoice::Ignore);
        materials.sort_by_key(|material| material.name.to_lowercase());
        for material in materials {
            options.push(SelectOption::new(
                material.id.clone(),
                tank_option_label(material),
            ));
            choices.push(TankChoice::Assign(material.id.clone()));
        }
    }
    options.push(SelectOption::divider());
    choices.push(TankChoice::Ignore);
    options.push(SelectOption::action("", "Unassign Tank").with_tone(SelectOptionTone::Danger));
    choices.push(TankChoice::Unassign);
    (options, choices)
}

/// The source's label inside a tank menu: the family prefix in front of the
/// name unless the name already opens with it.
fn tank_option_label(material: &MaterialProfile) -> String {
    let name = material.name.trim();
    let family = RESIN_FAMILIES
        .iter()
        .find(|(value, _)| *value == material.resin_family)
        .map(|(_, label)| *label);
    match family {
        Some(label) if !name.to_lowercase().starts_with(&label.to_lowercase()) => {
            format!("{label} {name}")
        }
        _ => name.to_owned(),
    }
}

fn section_shell() -> Div {
    div()
        .w_full()
        .rounded(px(6.0))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .overflow_hidden()
        .flex()
        .flex_col()
}

fn section_header(
    title: &str,
    subtitle: Option<SharedString>,
    right: Option<Stateful<Div>>,
) -> Div {
    let left = div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .text_size(px(FONT_MD))
                .line_height(px(FONT_MD * LINE_HEIGHT_TIGHT))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child(icon("flask-conical", 16.0, TEXT_STRONG))
                .child(title.to_owned()),
        )
        .children(subtitle.map(|subtitle| {
            div()
                .text_size(px(FONT_SM))
                .line_height(px(FONT_SM * LINE_HEIGHT_SNUG))
                .text_color(rgb(TEXT_MUTED))
                .child(subtitle)
        }));

    let mut header = div()
        .flex()
        .items_start()
        .justify_between()
        .gap(px(8.0))
        .p(px(12.0))
        .border_b(px(1.0))
        .border_color(rgb(BORDER_SUBTLE))
        .child(left);
    if let Some(right) = right {
        header = header.child(div().flex_shrink_0().child(right));
    }
    header
}

fn multi_tank_section(view: &View, entity: &WeakEntity<DragonFruit>) -> Div {
    let mut grid = div().flex().gap(px(10.0));
    let slots = view.tank_count.max(4) as usize;

    for index in 0..slots {
        let is_tank = index < view.tank_count as usize;
        let assigned_id = if is_tank {
            view.tank_materials
                .get(index)
                .map(String::as_str)
                .unwrap_or("")
        } else {
            ""
        };
        let current = if is_tank {
            material_by_id(view, assigned_id)
        } else {
            None
        };
        let previous = if index > 0 {
            view.tank_materials
                .get(index - 1)
                .and_then(|id| material_by_id(view, id))
        } else {
            None
        };

        let mut label_row = div()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(4.0))
            .w_full()
            .text_size(px(FONT_XS))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(TEXT_MUTED));

        // The source's link sits between two tank labels; with no absolute
        // measurement pass it leads the label instead.
        if let (Some(previous), Some(current)) = (previous, current) {
            let disagree = tank_materials_disagree(previous, current, &view.output_format);
            let (name, color, message) = if disagree {
                ("octagon-alert", DANGER, TANK_MATERIALS_MISMATCH)
            } else {
                ("link-2", SUCCESS, TANK_MATERIALS_MATCH)
            };
            label_row = label_row.child(tooltip(
                ElementId::Name(format!("tank-link-{index}").into()),
                icon(name, 14.0, color),
                div()
                    .w(px(240.0))
                    .text_size(px(11.0))
                    .child(message.to_owned()),
            ));
        }
        if is_tank && assigned_id.is_empty() {
            label_row = label_row.child(icon("triangle-alert", 12.0, DANGER));
        }
        label_row = label_row.child(format!("Tank {}", index + 1));

        let (options, choices) = tank_options(view);
        let select_id = format!("tank-{index}");
        let open = ui_edit(|state| state.open_select.as_deref() == Some(select_id.as_str()));

        let select_entity = entity.clone();
        let toggle_id = select_id.clone();
        let on_toggle = move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
            ui_edit(|state| {
                state.open_select = if state.open_select.as_deref() == Some(toggle_id.as_str()) {
                    None
                } else {
                    Some(toggle_id.clone())
                };
            });
            notify(&select_entity, cx);
        };

        let select_entity = entity.clone();
        let printer_id = view.printer_id.clone();
        let choices_for_select = choices.clone();
        let on_select = move |picked: usize, _: &mut Window, cx: &mut App| {
            let choice = choices_for_select.get(picked).cloned();
            match choice {
                Some(TankChoice::Assign(id)) => {
                    model::assign_tank(&printer_id, index, &id);
                }
                Some(TankChoice::Unassign) => {
                    model::assign_tank(&printer_id, index, "");
                }
                Some(TankChoice::Ignore) | None => {}
            }
            ui_edit(|state| state.open_select = None);
            notify(&select_entity, cx);
        };

        let dropdown = select_dropdown(
            SharedString::from(select_id.clone()),
            &options,
            assigned_id,
            open,
            !is_tank,
            on_toggle,
            on_select,
        );

        let mut cell = div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .flex_1()
            .min_w(px(0.0))
            .child(label_row)
            .child(dropdown);
        if !is_tank {
            cell = cell.child(
                div()
                    .text_size(px(11.0))
                    .text_color(rgb(TEXT_MUTED))
                    .text_center()
                    .child("—"),
            );
        }
        grid = grid.child(cell);
    }

    let mut body = div().p(px(12.0)).flex().flex_col().gap(px(8.0)).child(grid);
    if view.materials.is_empty() {
        // `printerMaterials.length === 0` (`line 4617`).
        body = body.child(
            div()
                .text_size(px(FONT_XS))
                .text_color(rgb(TEXT_MUTED))
                .child("Add a material profile to assign it to a tank."),
        );
    }

    section_shell()
        .child(section_header("Multi-Tank", None, None))
        .child(body)
}

// ── Material Settings ───────────────────────────────────────────────────────

fn material_settings_section(view: &View, entity: &WeakEntity<DragonFruit>) -> Div {
    let subtitle = SharedString::from(format!(
        "Profiles below are bound to {} and follow the selected printer hardware.",
        view.printer_name
    ));

    let new_entity = entity.clone();
    let new_button = action_button(
        "material-new",
        "New",
        Some("plus"),
        Some(accent_secondary_fill(0.07)),
        false,
        move |_, _, cx| {
            ui_edit(|state| {
                state.create = Some(MaterialDraft::new_material());
                state.open_select = None;
            });
            notify(&new_entity, cx);
        },
    );
    let header = section_header("Material Settings", Some(subtitle), Some(new_button));

    // `grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,1.25fr)]`.
    let manufacturers = manufacturer_column(view, entity);
    let families = resin_family_column(view, entity);
    let profiles = profile_column(view, entity);

    let columns = div()
        .w_full()
        .flex()
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .rounded(px(6.0))
        .bg(rgb(SURFACE_2))
        .overflow_hidden()
        .child(
            div()
                .flex_grow(1.0)
                .flex_basis(px(0.0))
                .min_w(px(0.0))
                .child(manufacturers),
        )
        .child(
            div()
                .flex_grow(1.0)
                .flex_basis(px(0.0))
                .min_w(px(0.0))
                .border_l(px(1.0))
                .border_color(rgb(BORDER_SUBTLE))
                .child(families),
        )
        .child(
            div()
                .flex_grow(1.25)
                .flex_basis(px(0.0))
                .min_w(px(0.0))
                .border_l(px(1.0))
                .border_color(rgb(BORDER_SUBTLE))
                .child(profiles),
        );

    let actions = action_bar(view, entity);

    div()
        .w_full()
        .rounded(px(6.0))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(mix(SURFACE_1, SURFACE_2, 0.92)))
        .overflow_hidden()
        .flex()
        .flex_col()
        .child(header)
        .child(
            div()
                .p(px(12.0))
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(columns)
                .child(actions),
        )
}

fn column_heading(label: &str) -> Div {
    div()
        .px(px(10.0))
        .py(px(8.0))
        .border_b(px(1.0))
        .border_color(rgb(BORDER_SUBTLE))
        .text_size(px(FONT_MD))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT_MUTED))
        .child(label.to_uppercase())
}

fn column_list(id: &'static str) -> Stateful<Div> {
    div()
        .id(ElementId::Name(SharedString::from(id)))
        .h(px(260.0))
        .p(px(6.0))
        .flex()
        .flex_col()
        .gap(px(4.0))
        .overflow_y_scroll()
}

fn rail_row(
    id_prefix: &str,
    label: String,
    badge: Option<u32>,
    selected: bool,
    accent: u32,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    // The manufacturer column is the source's accent-secondary selection; the
    // material-type and profile columns use the accent
    // (`ProfileSettingsModal.tsx:4870-4950`).
    let (border, background, text) = if selected {
        (
            mix(accent, BORDER_SUBTLE, 0.70),
            mix(accent, SURFACE_1, 0.11),
            TEXT_STRONG,
        )
    } else {
        (BORDER_SUBTLE, SURFACE_1, TEXT_MUTED)
    };
    let mut row = div()
        .id(ElementId::Name(SharedString::from(format!(
            "rail-{id_prefix}-{label}"
        ))))
        .w_full()
        .rounded(px(6.0))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .px(px(10.0))
        .py(px(8.0))
        .text_size(px(FONT_MD))
        .text_color(rgb(text))
        .cursor_pointer()
        .on_click(on_click);
    if let Some(color) = badge {
        row = row.child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .w(px(10.0))
                        .h(px(10.0))
                        .rounded(px(2.0))
                        .bg(rgb(color)),
                )
                .child(label),
        );
    } else {
        row = row.child(label);
    }
    row
}

fn manufacturer_column(view: &View, entity: &WeakEntity<DragonFruit>) -> Div {
    let mut list = column_list("material-manufacturers");
    for manufacturer in &view.manufacturers {
        let selected = view.manufacturer.as_deref() == Some(manufacturer.as_str());
        let picked = manufacturer.clone();
        let entity = entity.clone();
        list = list.child(rail_row(
            "manufacturer",
            picked.clone(),
            None,
            selected,
            ACCENT_SECONDARY,
            move |_, _, cx| {
                ui_edit(|state| {
                    state.manufacturer = Some(picked.clone());
                    state.resin_family = None;
                    state.selected_material = None;
                });
                notify(&entity, cx);
            },
        ));
    }
    div()
        .flex()
        .flex_col()
        .min_w(px(0.0))
        .child(column_heading("Manufacturer"))
        .child(list)
}

fn resin_family_column(view: &View, entity: &WeakEntity<DragonFruit>) -> Div {
    let mut list = column_list("material-resin-families");
    for family in &view.resin_families {
        let label = RESIN_FAMILIES
            .iter()
            .find(|(value, _)| value == &family.as_str())
            .map(|(_, label)| (*label).to_owned())
            .unwrap_or_else(|| family.clone());
        let selected = view.resin_family.as_deref() == Some(family.as_str());
        let picked = family.clone();
        let entity = entity.clone();
        list = list.child(rail_row(
            "resin-family",
            label,
            None,
            selected,
            ACCENT,
            move |_, _, cx| {
                ui_edit(|state| {
                    state.resin_family = Some(picked.clone());
                    state.selected_material = None;
                });
                notify(&entity, cx);
            },
        ));
    }
    div()
        .flex()
        .flex_col()
        .min_w(px(0.0))
        .child(column_heading("Material Type"))
        .child(list)
}

fn profile_column(view: &View, entity: &WeakEntity<DragonFruit>) -> Div {
    let mut list = column_list("material-profiles");
    for material in &view.visible {
        list = list.child(material_row(view, material, entity));
    }
    div()
        .flex()
        .flex_col()
        .min_w(px(0.0))
        .child(column_heading("Profile"))
        .child(list)
}

fn material_row(
    view: &View,
    material: &MaterialProfile,
    entity: &WeakEntity<DragonFruit>,
) -> Stateful<Div> {
    let selected = view.selected_material.as_deref() == Some(material.id.as_str());
    let (border, background, text) = if selected {
        (
            mix(ACCENT, BORDER_SUBTLE, 0.70),
            mix(ACCENT, SURFACE_1, 0.11),
            TEXT_STRONG,
        )
    } else {
        (BORDER_SUBTLE, SURFACE_1, TEXT_MUTED)
    };

    let is_owner = material.printer_profile_id == view.printer_id;
    let is_locked = material.locked
        || material
            .official_template_id
            .as_deref()
            .is_some_and(|id| !id.trim().is_empty());

    let mut badge_row = div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .flex_1()
        .min_w(px(0.0))
        .font_weight(FontWeight::SEMIBOLD);

    if is_owner {
        if !material.compatible_printer_profile_ids.is_empty() {
            let names = material
                .compatible_printer_profile_ids
                .iter()
                .filter_map(|id| {
                    view.profiles
                        .iter()
                        .find(|profile| &profile.id == id)
                        .map(|profile| profile.name.clone())
                })
                .collect::<Vec<_>>()
                .join(", ");
            badge_row = badge_row.child(tooltip(
                ElementId::Name(SharedString::from(format!("shared-{}", material.id))),
                icon("crown", 14.0, CROWN),
                div()
                    .w(px(240.0))
                    .text_size(px(11.0))
                    .child(format!("Shared with {names}")),
            ));
        }
    } else {
        let owner = view
            .profiles
            .iter()
            .find(|profile| profile.id == material.printer_profile_id)
            .map(|profile| profile.name.clone())
            .unwrap_or_else(|| "another printer".to_owned());
        badge_row = badge_row.child(tooltip(
            ElementId::Name(SharedString::from(format!("owned-{}", material.id))),
            icon("link-2", 14.0, ACCENT_SECONDARY),
            div()
                .w(px(240.0))
                .text_size(px(11.0))
                .child(format!("Owned by {owner}")),
        ));
    }
    if is_locked {
        badge_row = badge_row.child(icon("lock", 14.0, TEXT_MUTED));
    }
    badge_row = badge_row.child(fade_text(material.name.clone()));

    let mut right = div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .flex_shrink_0()
        .text_color(rgb(TEXT_MUTED));
    if let Some(color) = material.color.as_deref() {
        // `h-3.5 w-3.5 rounded-[4px]` with the inset ring.
        right = right.child(
            div()
                .w(px(14.0))
                .h(px(14.0))
                .rounded(px(4.0))
                .bg(rgb(gpui_ui::ui::parse_hex_color(color)))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE)),
        );
    }
    // `Math.round(material.layerHeightMm * 1000)` μm.
    right = right.child(format!(
        "{} μm",
        (material.layer_height_mm * 1000.0).round() as i64
    ));

    let picked = material.id.clone();
    let row_entity = entity.clone();
    // In Multi-Tank the row is picked for Edit/Duplicate, not as the job's
    // material: the tanks own that (`line 4955`).
    let set_active = !view.multi_tank;
    // `onDoubleClick`: open the editor, or the lock dialog for a read-only
    // profile (`openSelectedMaterialEditor`).
    let is_official = material
        .official_template_id
        .as_deref()
        .is_some_and(|id| !id.trim().is_empty());
    let draft = if is_locked {
        None
    } else {
        Some(MaterialDraft::from_profile(material))
    };
    div()
        .id(ElementId::Name(SharedString::from(format!(
            "material-{}",
            material.id
        ))))
        .w_full()
        .rounded(px(6.0))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .px(px(10.0))
        .py(px(8.0))
        .text_size(px(FONT_MD))
        .text_color(rgb(text))
        .cursor_pointer()
        .on_click(move |event, _, cx| {
            if set_active {
                model::select_material(&picked);
            }
            ui_edit(|state| state.selected_material = Some(picked.clone()));
            let count = match event {
                ClickEvent::Mouse(event) => event.down.click_count,
                ClickEvent::Touch(event) => event.tap_count,
                ClickEvent::Keyboard(_) => 0,
            };
            if count >= 2 {
                if is_locked {
                    ui_edit(|state| state.lock_dialog = Some(is_official));
                } else if let Some(draft) = draft.clone() {
                    let material_id = picked.clone();
                    ui_edit(|state| {
                        state.editor = Some(OpenEditor {
                            material_id,
                            draft,
                            _tab: (),
                        })
                    });
                }
            }
            notify(&row_entity, cx);
        })
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.0))
                .min_w(px(0.0))
                .child(badge_row)
                .child(right),
        )
}

fn action_bar(view: &View, entity: &WeakEntity<DragonFruit>) -> Div {
    let selected = view
        .selected_material
        .as_deref()
        .and_then(|id| material_by_id(view, id));
    let is_official = selected
        .and_then(|material| material.official_template_id.as_deref())
        .is_some_and(|id| !id.trim().is_empty());
    // `printerMaterials.length <= 1`: the source's UI guard is the list the
    // printer can see, shared rows included (the store additionally refuses to
    // drop the owner's last material).
    let delete_blocked = selected.is_none()
        || view.materials.len() <= 1
        || selected.is_some_and(|material| material.printer_profile_id != view.printer_id);

    let edit = {
        let material_id = selected.map(|material| material.id.clone());
        // `openSelectedMaterialEditor`: a locked or official row opens the lock
        // dialog instead of the editor, so only an editable row gets a draft.
        let draft = selected
            .filter(|material| !is_official && !material.locked)
            .map(MaterialDraft::from_profile);
        let locked = selected.is_some_and(|material| material.locked);
        let entity = entity.clone();
        action_button(
            "material-edit",
            "Edit",
            Some("pencil"),
            None,
            material_id.is_none(),
            move |_, _, cx| {
                let Some(id) = material_id.clone() else {
                    return;
                };
                if is_official || locked {
                    ui_edit(|state| state.lock_dialog = Some(is_official));
                } else if let Some(draft) = draft.clone() {
                    ui_edit(|state| {
                        state.editor = Some(OpenEditor {
                            material_id: id,
                            draft,
                            _tab: (),
                        })
                    });
                }
                notify(&entity, cx);
            },
        )
    };

    let duplicate = {
        let material_id = selected.map(|material| material.id.clone());
        let printer_id = view.printer_id.clone();
        let entity = entity.clone();
        action_button(
            "material-duplicate",
            "Duplicate",
            Some("copy-plus"),
            None,
            material_id.is_none(),
            move |_, _, cx| {
                if let Some(id) = material_id.as_ref() {
                    let new_id = model::duplicate_material(id);
                    if !new_id.is_empty() {
                        model::select_material(&new_id);
                        // `duplicateSelectedMaterial(true)` opens the editor on
                        // the copy; the draft is read back from the store.
                        let draft = model::materials_for(&printer_id)
                            .into_iter()
                            .find(|material| material.id == new_id)
                            .map(|material| MaterialDraft::from_profile(&material));
                        ui_edit(|state| {
                            state.selected_material = Some(new_id.clone());
                            state.editor = draft.map(|draft| OpenEditor {
                                material_id: new_id.clone(),
                                draft,
                                _tab: (),
                            });
                        });
                    }
                }
                notify(&entity, cx);
            },
        )
    };

    let lock = {
        let material_id = selected.map(|material| material.id.clone());
        let locked = selected.is_some_and(|material| material.locked);
        let (label, icon_name) = if locked {
            ("Unlock", "lock-open")
        } else {
            ("Lock", "lock")
        };
        let entity = entity.clone();
        action_button(
            "material-lock",
            label,
            Some(icon_name),
            None,
            material_id.is_none() || is_official,
            move |_, _, cx| {
                if let Some(id) = material_id.as_ref() {
                    let next = !locked;
                    model::update_material(id, |material| material.locked = next);
                }
                notify(&entity, cx);
            },
        )
    };

    let delete = {
        let target = selected.map(|material| (material.id.clone(), material.name.clone()));
        let enabled = !delete_blocked;
        // `materialOwnedByMessage`: a material owned by another printer cannot
        // be deleted from this one, and the disabled button says who owns it.
        let owner_name = selected
            .filter(|material| material.printer_profile_id != view.printer_id)
            .map(|material| {
                view.profiles
                    .iter()
                    .find(|profile| profile.id == material.printer_profile_id)
                    .map(|profile| profile.name.clone())
                    .unwrap_or_else(|| "another printer".to_owned())
            });
        let entity = entity.clone();
        let label = "Delete";
        // `variant="secondary"` with `color: var(--danger)` while enabled and
        // `var(--text-muted)` while disabled (the button's own disabled branch).
        let button = action_button(
            "material-delete",
            label,
            Some("trash-2"),
            Some((SURFACE_1, BORDER_SUBTLE, DANGER)),
            !enabled,
            move |_, _, cx| {
                if let Some(target) = target.clone() {
                    ui_edit(|state| state.delete_confirm = Some(target));
                }
                notify(&entity, cx);
            },
        );
        match owner_name {
            Some(owner) => tooltip(
                ElementId::Name(SharedString::from("material-delete-tooltip")),
                button,
                div()
                    .w(px(240.0))
                    .text_size(px(11.0))
                    .child(format!("Owned by {owner}")),
            ),
            None => button,
        }
    };

    let mut bar = div()
        .w_full()
        .rounded(px(6.0))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(mix(SURFACE_2, SURFACE_1, 0.92)))
        .p(px(8.0))
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(edit)
        .child(duplicate);
    // The source renders no Lock action for an official (or absent) profile.
    if !is_official && selected.is_some() {
        bar = bar.child(lock);
    }
    bar.child(div().flex_1()).child(delete)
}

// ── Buttons ─────────────────────────────────────────────────────────────────

/// `.ui-button secondary !h-8 !px-3 !py-0 text-sm inline-flex gap-1
/// rounded-md` — the modal's action-bar recipe.
fn accent_secondary_fill(background_weight: f32) -> (u32, u32, u32) {
    (
        mix(ACCENT_SECONDARY, SURFACE_1, background_weight),
        mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.58),
        ACCENT_SECONDARY,
    )
}

fn action_button(
    id: &'static str,
    label: &'static str,
    icon_name: Option<&'static str>,
    fill: Option<(u32, u32, u32)>,
    disabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    button_recipe(
        id,
        label,
        icon_name,
        fill,
        FONT_MD,
        6.0,
        disabled,
        on_click,
    )
}

/// The material editor's footer recipe: the source's `rounded-full` pills
/// (`ProfileSettingsModal.tsx:5478-5520`), `text-sm` except `Cancel`
/// (`text-xs`).
fn pill_button(
    id: &'static str,
    label: &'static str,
    icon_name: Option<&'static str>,
    fill: Option<(u32, u32, u32)>,
    font_size: f32,
    disabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    button_recipe(
        id,
        label,
        icon_name,
        fill,
        font_size,
        999.0,
        disabled,
        on_click,
    )
}

fn button_recipe(
    id: &'static str,
    label: &'static str,
    icon_name: Option<&'static str>,
    fill: Option<(u32, u32, u32)>,
    font_size: f32,
    radius: f32,
    disabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let mut spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Sm);
    spec.padding_x = Some(12.0);
    spec.padding_y = Some((0.0, 0.0));
    spec.radius = Some(radius);
    spec.font_size = Some(font_size);
    spec.gap = Some(6.0);
    spec.disabled = disabled;
    spec.fill = fill;
    spec.icon = icon_name.map(|name| (SharedString::from(name), 14.0));
    if !disabled {
        if let Some((_, _, text)) = fill {
            spec.icon_color = Some(text);
        }
    }
    let element = button_with(id, label, &spec);
    if disabled {
        element
    } else {
        element.on_click(on_click)
    }
}

// ── Overlays ────────────────────────────────────────────────────────────────

fn overlays(view: &View, entity: &WeakEntity<DragonFruit>) -> Div {
    let mut layer = div().absolute().top(px(0.0)).left(px(0.0)).size_full();

    if let Some(create) = ui_edit(|state| state.create.clone()) {
        layer = layer.child(create_dialog(view, &create, entity));
    }
    if let Some(editor) = ui_edit(|state| {
        state
            .editor
            .as_ref()
            .map(|editor| (editor.material_id.clone(), editor.draft.clone()))
    }) {
        let (material_id, draft) = editor;
        layer = layer.child(editor_dialog(view, &material_id, &draft, entity));
    }
    if let Some((id, name)) = ui_edit(|state| state.delete_confirm.clone()) {
        layer = layer.child(delete_dialog(&id, &name, entity));
    }
    if let Some(is_official) = ui_edit(|state| state.lock_dialog) {
        layer = layer.child(lock_dialog(is_official, view, entity));
    }
    let (compat_open, compat_target) = ui_edit(|state| (state.compat.open, state.compat.target));
    if compat_open {
        layer = layer.child(compat_dialog(view, compat_target, entity));
    }
    layer
}

fn dialog_backdrop() -> Div {
    div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .size_full()
        .p(px(20.0))
        .flex()
        .items_center()
        .justify_center()
        .bg(rgba(with_alpha(BLACK, 0x8c)))
}

/// The source's `w-full max-w-[920px] max-h-[88vh] rounded-xl border shadow-2xl
/// flex flex-col` editor/create panel.
fn editor_panel(width: f32) -> Div {
    div()
        .w_full()
        .max_w(px(width))
        .max_h(px(620.0))
        .rounded(px(12.0))
        .border_1()
        .border_color(rgb(BORDER_STRONG))
        .bg(rgb(SURFACE_0))
        .overflow_hidden()
        .flex()
        .flex_col()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
}

fn editor_header(
    title: &str,
    subtitle: String,
    close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .px(px(16.0))
        .py(px(12.0))
        .border_b(px(1.0))
        .border_color(rgb(BORDER_SUBTLE))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(px(FONT_MD))
                        .line_height(px(FONT_MD * LINE_HEIGHT_TIGHT))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child(title.to_owned()),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .line_height(px(11.0 * LINE_HEIGHT_SNUG))
                        .text_color(rgb(TEXT_MUTED))
                        .child(subtitle),
                ),
        )
        .child(
            icon_button(
                ElementId::Name(SharedString::from(format!("close-{title}"))),
                "x",
                IconButtonVariant::Surface,
                IconButtonSize::Sm,
            )
            .on_click(close),
        )
}

fn draft_body(
    draft: &MaterialDraft,
    target: FormTarget,
    view: &View,
    owner_id: &str,
    entity: &WeakEntity<DragonFruit>,
) -> Stateful<Div> {
    div()
        .id(ElementId::Name(SharedString::from(match target {
            FormTarget::Editor => "editor-form-body",
            FormTarget::Create => "create-form-body",
        })))
        .p(px(12.0))
        .flex_1()
        .min_h(px(0.0))
        .flex()
        .flex_col()
        .gap(px(12.0))
        .overflow_y_scroll()
        .child(identity_card(draft, target, view, owner_id, entity))
        .child(print_settings_card(draft, target, entity))
        .child(scale_card(draft, target, entity))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FormTarget {
    Editor,
    Create,
}

#[derive(Clone, Copy)]
enum DraftField {
    BottlePrice,
    BottleCapacity,
    ScaleX,
    ScaleY,
    ScaleZ,
    LayerHeight,
    NormalExposure,
    BottomExposure,
    BottomLayers,
    LiftDistance,
    LiftSpeed,
    RetractSpeed,
}

fn draft_mut<'a>(draft: &'a mut MaterialDraft, field: DraftField) -> &'a mut f32 {
    match field {
        DraftField::BottlePrice => &mut draft.bottle_price,
        DraftField::BottleCapacity => &mut draft.bottle_capacity_ml,
        DraftField::ScaleX => &mut draft.scale_x,
        DraftField::ScaleY => &mut draft.scale_y,
        DraftField::ScaleZ => &mut draft.scale_z,
        DraftField::LayerHeight => &mut draft.layer_height_mm,
        DraftField::NormalExposure => &mut draft.normal_exposure_sec,
        DraftField::BottomExposure => &mut draft.bottom_exposure_sec,
        DraftField::BottomLayers => &mut draft.bottom_layer_count,
        DraftField::LiftDistance => &mut draft.lift_distance_mm,
        DraftField::LiftSpeed => &mut draft.lift_speed_mm_min,
        DraftField::RetractSpeed => &mut draft.retract_speed_mm_min,
    }
}

fn draft_for(state: &mut MaterialsUi, target: FormTarget) -> Option<&mut MaterialDraft> {
    match target {
        FormTarget::Editor => state.editor.as_mut().map(|editor| &mut editor.draft),
        FormTarget::Create => state.create.as_mut(),
    }
}

fn toggle_draft(state: &mut MaterialsUi, target: FormTarget, f: impl FnOnce(&mut MaterialDraft)) {
    if let Some(draft) = draft_for(state, target) {
        f(draft);
    }
}

fn card_shell(title: &str) -> Div {
    div()
        .w_full()
        .rounded(px(12.0))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_2))
        .p(px(12.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .child(title.to_uppercase()),
        )
}

fn field_label(label: &str) -> Div {
    div()
        .text_size(px(11.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(TEXT_MUTED))
        .child(label.to_owned())
}

/// `LabeledInput` (`profileFormAtoms.tsx:186-216`): the `.ui-input` frame with a
/// real editor that commits every keystroke into the form's draft.
fn editable_field(
    id: String,
    value: &str,
    on_change: impl Fn(&str, &mut Window, &mut App) + 'static,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .flex_1()
        .min_w(px(0.0))
        .child(text_input(id, value.to_owned(), "", false, 36.0, on_change))
}

fn paired_cell(left: Div, right: Option<Div>) -> Div {
    let mut row = div().w_full().flex().items_end().gap(px(8.0)).child(left);
    if let Some(right) = right {
        row = row.child(right);
    }
    row
}

fn labelled_text(
    label: &str,
    id: String,
    value: &str,
    on_change: impl Fn(&str, &mut Window, &mut App) + 'static,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .flex_1()
        .min_w(px(0.0))
        .child(field_label(label))
        .child(editable_field(id, value, on_change))
}

fn nudge(value: &mut f32, direction: f32) {
    // `LabeledNumberInput.nudge`: the step is 0.01 below 1 and 1 above it;
    // three decimals are kept for the fine step, integers otherwise.
    let step = if value.abs() < 1.0 { 0.01 } else { 1.0 };
    let next = *value + direction * step;
    let next = next.max(0.0);
    *value = if step < 1.0 {
        (next * 1000.0).round() / 1000.0
    } else {
        next.round()
    };
}

fn format_value(value: f32) -> String {
    if !value.is_finite() {
        return "0".to_owned();
    }
    if (value - value.round()).abs() < 1e-6 {
        format!("{:.0}", value)
    } else {
        let mut text = format!("{value:.3}");
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
        text
    }
}

fn number_cell(
    id: String,
    label: &str,
    value: f32,
    field: DraftField,
    target: FormTarget,
    entity: &WeakEntity<DragonFruit>,
) -> Div {
    let up_entity = entity.clone();
    let down_entity = entity.clone();
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .flex_1()
        .min_w(px(0.0))
        .child(field_label(label))
        .child({
            let field_entity = entity.clone();
            div()
                .relative()
                .w_full()
                .child(number_on_blur(
                    id,
                    value,
                    None,
                    0.0,
                    f32::INFINITY,
                    3,
                    false,
                    36.0,
                    move |next, _, cx| {
                        let next = if next.is_finite() { next.max(0.0) } else { 0.0 };
                        ui_edit(move |state| {
                            toggle_draft(state, target, |draft| {
                                *draft_mut(draft, field) = next;
                            })
                        });
                        notify(&field_entity, cx);
                    },
                ))
                .child(
                    div()
                        .absolute()
                        .top(px(0.0))
                        .bottom(px(0.0))
                        .right(px(4.0))
                        .w(px(16.0))
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap(px(2.0))
                        .child({
                            let button = div()
                                .id("material-field-up")
                                .w(px(12.0))
                                .h(px(12.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(4.0))
                                .child(icon("chevron-up", 10.0, TEXT_MUTED));
                            button.cursor_pointer().on_click(move |_, _, cx| {
                                ui_edit(|state| {
                                    toggle_draft(state, target, |draft| {
                                        nudge(draft_mut(draft, field), 1.0)
                                    })
                                });
                                notify(&up_entity, cx);
                            })
                        })
                        .child({
                            let button = div()
                                .w(px(12.0))
                                .id("material-field-down")
                                .h(px(12.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(4.0))
                                .child(icon("chevron-down", 10.0, TEXT_MUTED));
                            if value > 0.0 {
                                button.cursor_pointer().on_click(move |_, _, cx| {
                                    ui_edit(|state| {
                                        toggle_draft(state, target, |draft| {
                                            nudge(draft_mut(draft, field), -1.0)
                                        })
                                    });
                                    notify(&down_entity, cx);
                                })
                            } else {
                                button.opacity(0.5)
                            }
                        }),
                )
        })
}

fn target_key(target: FormTarget) -> &'static str {
    match target {
        FormTarget::Editor => "editor",
        FormTarget::Create => "create",
    }
}

/// A labelled `SelectDropdown` inside a form card.
fn form_select(
    id: String,
    label: &str,
    value: &str,
    options: Vec<SelectOption>,
    target: FormTarget,
    entity: &WeakEntity<DragonFruit>,
    on_pick: impl Fn(&mut MaterialDraft, usize) + 'static,
) -> Div {
    let open = ui_edit(|state| state.open_select.as_deref() == Some(id.as_str()));

    let toggle_entity = entity.clone();
    let toggle_id = id.clone();
    let on_toggle = move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
        ui_edit(|state| {
            state.open_select = if state.open_select.as_deref() == Some(toggle_id.as_str()) {
                None
            } else {
                Some(toggle_id.clone())
            };
        });
        notify(&toggle_entity, cx);
    };

    let select_entity = entity.clone();
    let on_select = move |picked: usize, _: &mut Window, cx: &mut App| {
        ui_edit(|state| {
            if let Some(draft) = draft_for(state, target) {
                on_pick(draft, picked);
            }
            state.open_select = None;
        });
        notify(&select_entity, cx);
    };

    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .flex_1()
        .min_w(px(0.0))
        .child(field_label(label))
        .child(select_dropdown(
            SharedString::from(id),
            &options,
            value,
            open,
            false,
            on_toggle,
            on_select,
        ))
}

fn resin_family_options() -> Vec<SelectOption> {
    RESIN_FAMILIES
        .iter()
        .map(|(value, label)| SelectOption::new(*value, *label))
        .collect()
}

fn currency_options() -> Vec<SelectOption> {
    CURRENCIES
        .iter()
        .map(|code| SelectOption::new(*code, *code))
        .collect()
}

fn color_options() -> Vec<SelectOption> {
    let mut options = vec![SelectOption::new("", "None")];
    options.extend(
        COLOR_PRESETS
            .iter()
            .map(|(value, label)| SelectOption::new(*value, *label)),
    );
    options
        .push(SelectOption::new(CUSTOM_COLOR, "Custom Color").with_tone(SelectOptionTone::Accent));
    options
}

/// The colour token a draft's value maps to in [`color_options`]: `''` for
/// none, a preset's hex, or `custom`.
fn color_token(color: Option<&str>) -> String {
    match color {
        None => String::new(),
        Some(color)
            if COLOR_PRESETS
                .iter()
                .any(|(value, _)| value.eq_ignore_ascii_case(color)) =>
        {
            COLOR_PRESETS
                .iter()
                .find(|(value, _)| value.eq_ignore_ascii_case(color))
                .map(|(value, _)| (*value).to_owned())
                .unwrap_or_default()
        }
        Some(_) => CUSTOM_COLOR.to_owned(),
    }
}

/// `materialSharingPrinters`' filter: the owner, plus printers whose output
/// format and settings mode match it.
fn is_shareable(printer: &PrinterProfile, owner: &PrinterProfile) -> bool {
    printer.id == owner.id
        || (printer.display.output_format == owner.display.output_format
            && printer.display.settings_mode.clone().unwrap_or_default()
                == owner.display.settings_mode.clone().unwrap_or_default())
}

fn owner_profile<'a>(view: &'a View, owner_id: &str) -> Option<&'a PrinterProfile> {
    view.profiles.iter().find(|profile| profile.id == owner_id)
}

/// The compatible-machines button (`MaterialProfileIdentitySection`).
fn compat_button(
    count: usize,
    enabled: bool,
    target: FormTarget,
    entity: &WeakEntity<DragonFruit>,
) -> Stateful<Div> {
    let el = div()
        .id(ElementId::Name(SharedString::from(format!(
            "{}-compat",
            target_key(target)
        ))))
        .w_full()
        .h(px(36.0))
        .px(px(10.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .text_size(px(FONT_MD))
        .text_color(rgb(TEXT_STRONG))
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(icon("link-2", 14.0, TEXT_STRONG))
                .child("Compatible Machines"),
        )
        .child(div().text_color(rgb(TEXT_MUTED)).child(format!("{count}")));
    if enabled {
        let entity = entity.clone();
        el.cursor_pointer().on_click(move |_, _, cx| {
            ui_edit(|state| {
                state.compat.open = true;
                state.compat.target = target;
            });
            notify(&entity, cx);
        })
    } else {
        // `hasCompatiblePeer ? undefined : <Trans>No other printer uses the
        // same material settings and file format</Trans>`.
        tooltip(
            ElementId::Name(SharedString::from(format!(
                "{}-compat-disabled",
                target_key(target)
            ))),
            el.opacity(0.55).cursor_not_allowed(),
            div()
                .w(px(240.0))
                .text_size(px(11.0))
                .child("No other printer uses the same material settings and file format"),
        )
    }
}

/// `MaterialProfileIdentitySection` (`profileFormAtoms.tsx:832`): the Material
/// Profile card — brand, name, colour, compatible machines, resin family,
/// currency, bottle price and capacity.
fn identity_card(
    draft: &MaterialDraft,
    target: FormTarget,
    view: &View,
    owner_id: &str,
    entity: &WeakEntity<DragonFruit>,
) -> Div {
    let target_key = target_key(target);
    let brand_entity = entity.clone();
    let brand = labelled_text(
        "Manufacturer",
        format!("{target_key}-brand"),
        &draft.brand,
        move |text, _, cx| {
            let text = text.to_owned();
            ui_edit(move |state| toggle_draft(state, target, |draft| draft.brand = text));
            notify(&brand_entity, cx);
        },
    );
    let name_entity = entity.clone();
    let name = labelled_text(
        "Name",
        format!("{target_key}-name"),
        &draft.name,
        move |text, _, cx| {
            let text = text.to_owned();
            ui_edit(move |state| toggle_draft(state, target, |draft| draft.name = text));
            notify(&name_entity, cx);
        },
    );

    let color_value = color_token(draft.color.as_deref());
    let color_select = {
        let base = div()
            .flex()
            .items_end()
            .gap(px(8.0))
            .flex_1()
            .min_w(px(0.0));
        let mut block = form_select(
            format!("{target_key}-color"),
            "Color",
            &color_value,
            color_options(),
            target,
            entity,
            |draft, picked| {
                if picked == 0 {
                    draft.color = None;
                } else if picked <= COLOR_PRESETS.len() {
                    draft.color = Some(COLOR_PRESETS[picked - 1].0.to_owned());
                } else {
                    draft.color = Some(
                        draft
                            .color
                            .clone()
                            .filter(|color| color.starts_with('#'))
                            .unwrap_or_else(|| CUSTOM_COLOR_FALLBACK.to_owned()),
                    );
                }
            },
        );
        if color_value == CUSTOM_COLOR {
            block = block.child(div().mt(px(4.0)).child(color_swatch(
                36.0,
                30.0,
                draft.color.as_deref().unwrap_or(CUSTOM_COLOR_FALLBACK),
            )));
        }
        base.child(block)
    };

    let owner = owner_profile(view, owner_id);
    let can_share = owner.is_some() && !view.profiles.is_empty();
    let share_count = draft
        .compatible_printer_profile_ids
        .iter()
        .filter(|id| {
            owner
                .and_then(|owner| {
                    view.profiles
                        .iter()
                        .find(|profile| &profile.id == *id)
                        .filter(|profile| is_shareable(profile, owner))
                })
                .is_some()
        })
        .count();
    let has_peer = owner.is_some_and(|owner| {
        view.profiles
            .iter()
            .any(|profile| profile.id != owner.id && is_shareable(profile, owner))
    });

    let mut second_row = div().flex().items_end().gap(px(8.0));
    second_row = second_row.child(color_select);
    if can_share {
        second_row = second_row.child(
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .flex_1()
                .min_w(px(0.0))
                .child(field_label("Compatible Machines"))
                .child(compat_button(share_count, has_peer, target, entity)),
        );
    }

    let resin = form_select(
        format!("{target_key}-resin"),
        "Resin Family",
        &draft.resin_family,
        resin_family_options(),
        target,
        entity,
        |draft, picked| {
            if let Some((value, _)) = RESIN_FAMILIES.get(picked) {
                draft.resin_family = (*value).to_owned();
            }
        },
    );
    let currency = form_select(
        format!("{target_key}-currency"),
        "Currency",
        &draft.currency_code,
        currency_options(),
        target,
        entity,
        |draft, picked| {
            if let Some(code) = CURRENCIES.get(picked) {
                draft.currency_code = (*code).to_owned();
            }
        },
    );
    let price = number_cell(
        format!("{target_key}-price"),
        "Bottle Price",
        draft.bottle_price,
        DraftField::BottlePrice,
        target,
        entity,
    );
    let capacity = number_cell(
        format!("{target_key}-capacity"),
        "Bottle Capacity (ml)",
        draft.bottle_capacity_ml,
        DraftField::BottleCapacity,
        target,
        entity,
    );

    card_shell("Material Profile")
        .child(paired_cell(brand, Some(name)))
        .child(second_row)
        .child(
            div()
                .w_full()
                .flex()
                .gap(px(8.0))
                .child(resin)
                .child(currency),
        )
        .child(
            div()
                .w_full()
                .flex()
                .gap(px(8.0))
                .child(price)
                .child(capacity),
        )
}

/// `MaterialProfileFormSections`' Print Settings card (`profileFormAtoms.tsx`).
fn print_settings_card(
    draft: &MaterialDraft,
    target: FormTarget,
    entity: &WeakEntity<DragonFruit>,
) -> Div {
    let key = target_key(target);
    let fields = vec![
        (
            format!("{key}-layer-height"),
            "Layer height (mm)",
            draft.layer_height_mm,
            DraftField::LayerHeight,
        ),
        (
            format!("{key}-normal-exposure"),
            "Normal exposure (s)",
            draft.normal_exposure_sec,
            DraftField::NormalExposure,
        ),
        (
            format!("{key}-bottom-exposure"),
            "Bottom exposure (s)",
            draft.bottom_exposure_sec,
            DraftField::BottomExposure,
        ),
        (
            format!("{key}-bottom-layers"),
            "Bottom layers",
            draft.bottom_layer_count,
            DraftField::BottomLayers,
        ),
        (
            format!("{key}-lift-distance"),
            "Lift distance (mm)",
            draft.lift_distance_mm,
            DraftField::LiftDistance,
        ),
        (
            format!("{key}-lift-speed"),
            "Lift speed (mm/min)",
            draft.lift_speed_mm_min,
            DraftField::LiftSpeed,
        ),
        (
            format!("{key}-retract-speed"),
            "Retract speed (mm/min)",
            draft.retract_speed_mm_min,
            DraftField::RetractSpeed,
        ),
    ];

    let mut body = card_shell("Print Settings");
    let mut row = div().w_full().flex().gap(px(8.0));
    for (index, (id, label, value, field)) in fields.into_iter().enumerate() {
        if index > 0 && index % 2 == 0 {
            body = body.child(row);
            row = div().w_full().flex().gap(px(8.0));
        }
        row = row.child(number_cell(id, label, value, field, target, entity));
    }
    body = body.child(row);
    body
}

/// `MaterialProfileFormSections`' Scale Compensation card.
fn scale_card(draft: &MaterialDraft, target: FormTarget, entity: &WeakEntity<DragonFruit>) -> Div {
    let key = target_key(target);
    card_shell("Scale Compensation (% shrinkage)").child(
        div()
            .w_full()
            .flex()
            .gap(px(8.0))
            .child(number_cell(
                format!("{key}-scale-x"),
                "Scale X (%)",
                draft.scale_x,
                DraftField::ScaleX,
                target,
                entity,
            ))
            .child(number_cell(
                format!("{key}-scale-y"),
                "Scale Y (%)",
                draft.scale_y,
                DraftField::ScaleY,
                target,
                entity,
            ))
            .child(number_cell(
                format!("{key}-scale-z"),
                "Scale Z (%)",
                draft.scale_z,
                DraftField::ScaleZ,
                target,
                entity,
            )),
    )
}

// ── Editor / create dialogs ─────────────────────────────────────────────────

fn dialog_owner(view: &View, material_id: &str) -> String {
    material_by_id(view, material_id)
        .map(|material| material.printer_profile_id.clone())
        .unwrap_or_else(|| view.printer_id.clone())
}

fn editor_dialog(
    view: &View,
    material_id: &str,
    draft: &MaterialDraft,
    entity: &WeakEntity<DragonFruit>,
) -> Div {
    let material = material_by_id(view, material_id);
    let subtitle = match material {
        Some(material) => format!("{} • {}", material.name, material.brand),
        None => draft.name.clone(),
    };
    let owner_id = dialog_owner(view, material_id);

    let delete_blocked = material.is_none()
        || view.materials.len() <= 1
        || material.is_some_and(|material| material.printer_profile_id != view.printer_id);

    // Header close.
    let close_entity = entity.clone();
    let header_close = move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
        ui_edit(|state| state.editor = None);
        notify(&close_entity, cx);
    };

    // Backdrop close.
    let backdrop_entity = entity.clone();
    let backdrop_close = move |_: &MouseDownEvent, _: &mut Window, cx: &mut App| {
        ui_edit(|state| state.editor = None);
        notify(&backdrop_entity, cx);
    };

    let delete = {
        let target = material.map(|material| (material.id.clone(), material.name.clone()));
        // `materialOwnedByMessage` on the disabled Delete Material button.
        let owner_name = material
            .filter(|material| material.printer_profile_id != view.printer_id)
            .map(|material| {
                view.profiles
                    .iter()
                    .find(|profile| profile.id == material.printer_profile_id)
                    .map(|profile| profile.name.clone())
                    .unwrap_or_else(|| "another printer".to_owned())
            });
        let entity = entity.clone();
        let button = pill_button(
            "editor-delete",
            "Delete Material",
            Some("trash-2"),
            Some((SURFACE_1, BORDER_SUBTLE, DANGER)),
            FONT_MD,
            delete_blocked,
            move |_, _, cx| {
                if let Some(target) = target.clone() {
                    ui_edit(|state| {
                        state.delete_confirm = Some(target);
                        state.editor = None;
                    });
                }
                notify(&entity, cx);
            },
        );
        match owner_name {
            Some(owner) => tooltip(
                ElementId::Name(SharedString::from("editor-delete-tooltip")),
                button,
                div()
                    .w(px(240.0))
                    .text_size(px(11.0))
                    .child(format!("Owned by {owner}")),
            ),
            None => button,
        }
    };

    let cancel_entity = entity.clone();
    let cancel = pill_button(
        "editor-cancel",
        "Cancel",
        None,
        None,
        FONT_XS,
        false,
        move |_, _, cx| {
            ui_edit(|state| state.editor = None);
            notify(&cancel_entity, cx);
        },
    );

    let save_entity = entity.clone();
    let save_material_id = material_id.to_owned();
    let save_draft = draft.clone();
    let save = pill_button(
        "editor-save",
        "Save Material",
        Some("check"),
        Some(accent_secondary_fill(0.08)),
        FONT_MD,
        false,
        move |_, _, cx| {
            model::update_material(&save_material_id, |material| save_draft.apply_to(material));
            ui_edit(|state| state.editor = None);
            notify(&save_entity, cx);
        },
    );

    let panel = editor_panel(920.0)
        .child(editor_header(
            "Material Profile Settings",
            subtitle,
            header_close,
        ))
        .child(draft_body(
            draft,
            FormTarget::Editor,
            view,
            &owner_id,
            entity,
        ))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.0))
                .px(px(12.0))
                .py(px(12.0))
                .border_t(px(1.0))
                .border_color(rgb(BORDER_SUBTLE))
                .child(delete)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(cancel)
                        .child(save),
                ),
        );

    dialog_backdrop()
        .on_mouse_down(MouseButton::Left, backdrop_close)
        .child(panel)
}

fn create_dialog(view: &View, draft: &MaterialDraft, entity: &WeakEntity<DragonFruit>) -> Div {
    let subtitle = view.printer_name.clone();

    let close_entity = entity.clone();
    let header_close = move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
        ui_edit(|state| state.create = None);
        notify(&close_entity, cx);
    };
    let backdrop_entity = entity.clone();
    let backdrop_close = move |_: &MouseDownEvent, _: &mut Window, cx: &mut App| {
        ui_edit(|state| state.create = None);
        notify(&backdrop_entity, cx);
    };

    let cancel_entity = entity.clone();
    let cancel = pill_button(
        "create-cancel",
        "Cancel",
        None,
        None,
        FONT_XS,
        false,
        move |_, _, cx| {
            ui_edit(|state| state.create = None);
            notify(&cancel_entity, cx);
        },
    );

    let save_entity = entity.clone();
    let save_draft = draft.clone();
    let printer_id = view.printer_id.clone();
    // `handleCreateMaterial`: `addMaterialProfile(selectedPrinter.id, {
    // ...newMaterialDraft, name: ..., brand: ..., localSettingsByOutput })`.
    let save = pill_button(
        "create-save",
        "Save Material",
        Some("check"),
        Some(accent_secondary_fill(0.08)),
        FONT_MD,
        false,
        move |_, _, cx| {
            let new_id = model::add_material(&printer_id);
            if !new_id.is_empty() {
                model::update_material(&new_id, |material| save_draft.apply_to(material));
                model::select_material(&new_id);
                ui_edit(|state| {
                    state.create = None;
                    state.selected_material = Some(new_id.clone());
                    state.manufacturer = Some(brand_of_draft(&save_draft));
                    state.resin_family = Some(save_draft.resin_family.clone());
                });
            }
            notify(&save_entity, cx);
        },
    );

    let panel = editor_panel(920.0)
        .child(editor_header(
            "Create Material Profile",
            subtitle,
            header_close,
        ))
        .child(draft_body(
            draft,
            FormTarget::Create,
            view,
            &view.printer_id,
            entity,
        ))
        .child(
            div()
                .flex()
                .items_center()
                .justify_end()
                .gap(px(8.0))
                .px(px(12.0))
                .py(px(12.0))
                .border_t(px(1.0))
                .border_color(rgb(BORDER_SUBTLE))
                .child(cancel)
                .child(save),
        );

    dialog_backdrop()
        .on_mouse_down(MouseButton::Left, backdrop_close)
        .child(panel)
}

fn brand_of_draft(draft: &MaterialDraft) -> String {
    let brand = draft.brand.trim();
    if brand.is_empty() {
        "Default".to_owned()
    } else {
        brand.to_owned()
    }
}

// ── Compatible Machines ─────────────────────────────────────────────────────

/// `MaterialProfileIdentitySection`'s `StructuredDialogModal`: every printer,
/// owner first, blocked when its output format or settings mode differs. The
/// owner row is the crown; the others are checkboxes that write
/// `compatiblePrinterProfileIds`.
fn compat_dialog(view: &View, target: FormTarget, entity: &WeakEntity<DragonFruit>) -> Div {
    let draft = ui_edit(|state| draft_for(state, target).map(|draft| draft.clone()));
    let Some(draft) = draft else {
        return div();
    };
    let owner_id = match target {
        FormTarget::Editor => ui_edit(|state| {
            state
                .editor
                .as_ref()
                .map(|editor| dialog_owner(view, &editor.material_id))
        })
        .unwrap_or_else(|| view.printer_id.clone()),
        FormTarget::Create => view.printer_id.clone(),
    };
    let owner = owner_profile(view, &owner_id);

    let mut rows: Vec<&PrinterProfile> = view.profiles.iter().collect();
    rows.sort_by_key(|profile| if profile.id == owner_id { 0 } else { 1 });

    let mut list = div().flex().flex_col().gap(px(6.0));
    for printer in rows {
        let is_owner = printer.id == owner_id;
        let shareable = owner.is_some_and(|owner| is_shareable(printer, owner));
        let blocked = !shareable;
        let checked =
            is_owner || (!blocked && draft.compatible_printer_profile_ids.contains(&printer.id));

        let mut row = div()
            .id(ElementId::Name(SharedString::from(format!(
                "compat-{}",
                printer.id
            ))))
            .w_full()
            .rounded(px(6.0))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1))
            .px(px(10.0))
            .py(px(8.0))
            .text_size(px(FONT_MD))
            .text_color(rgb(TEXT_STRONG))
            .flex()
            .items_center()
            .gap(px(8.0));

        if is_owner {
            row = row
                .child(icon("crown", 16.0, CROWN))
                .child(div().flex_1().child(printer.name.clone()))
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Owner"),
                );
        } else if blocked {
            row = row
                .opacity(0.55)
                .child(icon("triangle-alert", 16.0, DANGER))
                .child(div().flex_1().child(printer.name.clone()))
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Incompatible"),
                );
        } else {
            let printer_id = printer.id.clone();
            let row_entity = entity.clone();
            row = row
                .cursor_pointer()
                .child(checkbox_with(
                    checked,
                    CheckboxSize::Md,
                    CheckboxTone::Accent,
                    false,
                ))
                .child(div().flex_1().child(printer.name.clone()))
                .on_click(move |_, _, cx| {
                    ui_edit(|state| {
                        toggle_draft(state, target, |draft| {
                            if let Some(index) = draft
                                .compatible_printer_profile_ids
                                .iter()
                                .position(|id| id == &printer_id)
                            {
                                draft.compatible_printer_profile_ids.remove(index);
                            } else {
                                draft
                                    .compatible_printer_profile_ids
                                    .push(printer_id.clone());
                            }
                        });
                    });
                    notify(&row_entity, cx);
                });
        }
        list = list.child(row);
    }

    let close_entity = entity.clone();
    let close = move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
        ui_edit(|state| state.compat.open = false);
        notify(&close_entity, cx);
    };
    let done_entity = entity.clone();
    let done = {
        let spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Md);
        button_with("compat-done", "Done", &spec).on_click(move |_, _, cx| {
            ui_edit(|state| state.compat.open = false);
            notify(&done_entity, cx);
        })
    };

    dialog_modal(
        448.0,
        "Compatible Machines",
        Some(SharedString::from(
            "Share this material profile with other printers",
        )),
        Some("link-2"),
        IconTone::Accent,
        list,
        done,
        close,
    )
}

// ── Delete / lock dialogs ───────────────────────────────────────────────────

fn delete_dialog(id: &str, name: &str, entity: &WeakEntity<DragonFruit>) -> Div {
    let close_entity = entity.clone();
    let close = move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
        ui_edit(|state| state.delete_confirm = None);
        notify(&close_entity, cx);
    };
    let cancel_entity = entity.clone();
    let cancel = {
        let spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Md);
        button_with("delete-cancel", "Cancel", &spec).on_click(move |_, _, cx| {
            ui_edit(|state| state.delete_confirm = None);
            notify(&cancel_entity, cx);
        })
    };
    let delete_entity = entity.clone();
    let delete_id = id.to_owned();
    let delete = {
        let spec = ButtonSpec::new(ButtonVariant::TintedDanger, ButtonSize::Md);
        button_with("delete-confirm", "Delete", &spec).on_click(move |_, _, cx| {
            model::remove_material(&delete_id);
            ui_edit(|state| {
                state.delete_confirm = None;
                state.editor = None;
                state.selected_material = None;
            });
            notify(&delete_entity, cx);
        })
    };

    dialog_modal(
        448.0,
        "Delete Material Profile",
        Some(SharedString::from("This action cannot be undone.")),
        Some("triangle-alert"),
        IconTone::Warning,
        div()
            .text_size(px(FONT_XS))
            .line_height(px(FONT_XS * LINE_HEIGHT_SNUG))
            .text_color(rgb(TEXT_MUTED))
            .child(format!("Delete material profile {name}?")),
        div()
            .w_full()
            .flex()
            .gap(px(8.0))
            .child(div().flex_1().child(cancel))
            .child(div().flex_1().child(delete)),
        close,
    )
}

fn lock_dialog(is_official: bool, view: &View, entity: &WeakEntity<DragonFruit>) -> Div {
    let close_entity = entity.clone();
    let close = move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
        ui_edit(|state| state.lock_dialog = None);
        notify(&close_entity, cx);
    };
    let cancel_entity = entity.clone();
    let cancel = {
        let spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Md);
        button_with("lock-cancel", "Cancel", &spec).on_click(move |_, _, cx| {
            ui_edit(|state| state.lock_dialog = None);
            notify(&cancel_entity, cx);
        })
    };

    let selected_id = view.selected_material.clone();
    let unlock = {
        let spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Md);
        let selected_id = selected_id.clone();
        let entity = entity.clone();
        button_with("lock-unlock", "Unlock", &spec).on_click(move |_, _, cx| {
            if let Some(id) = selected_id.as_ref() {
                model::update_material(id, |material| material.locked = false);
            }
            ui_edit(|state| state.lock_dialog = None);
            notify(&entity, cx);
        })
    };

    // `handleDuplicateMaterialAsCustom` → `duplicateSelectedMaterial(true)`.
    let make_copy = {
        let spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Md);
        let selected_id = selected_id.clone();
        let printer_id = view.printer_id.clone();
        let entity = entity.clone();
        button_with("lock-copy", "Make Custom Copy", &spec).on_click(move |_, _, cx| {
            if let Some(id) = selected_id.as_ref() {
                let new_id = model::duplicate_material(id);
                if !new_id.is_empty() {
                    model::select_material(&new_id);
                    let draft = model::materials_for(&printer_id)
                        .into_iter()
                        .find(|material| material.id == new_id)
                        .map(|material| MaterialDraft::from_profile(&material));
                    ui_edit(|state| {
                        state.lock_dialog = None;
                        state.selected_material = Some(new_id.clone());
                        state.editor = draft.map(|draft| OpenEditor {
                            material_id: new_id.clone(),
                            draft,
                            _tab: (),
                        });
                    });
                }
            }
            notify(&entity, cx);
        })
    };

    let body = if is_official {
        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .text_size(px(FONT_XS))
            .line_height(px(FONT_XS * LINE_HEIGHT_SNUG))
            .text_color(rgb(TEXT_MUTED))
            .child("Official material profiles cannot be edited directly.")
            .child("Choose Make Custom Copy to duplicate and adjust exposure settings safely.")
            .child(
                div()
                    .text_color(rgb(DANGER))
                    .child("Warning: Custom exposure settings may affect print quality and could damage the machine or cause personal injury."),
            )
    } else {
        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .text_size(px(FONT_XS))
            .line_height(px(FONT_XS * LINE_HEIGHT_SNUG))
            .text_color(rgb(TEXT_MUTED))
            .child("This material profile is locked, so its settings are read-only.")
            .child("Choose Unlock to edit it directly, or Make Custom Copy to duplicate it as a new editable profile.")
    };

    let actions = if is_official {
        div()
            .w_full()
            .flex()
            .gap(px(8.0))
            .child(div().flex_1().child(cancel))
            .child(div().flex_1().child(make_copy))
    } else {
        div()
            .w_full()
            .flex()
            .gap(px(8.0))
            .child(div().flex_1().child(cancel))
            .child(div().flex_1().child(unlock))
            .child(div().flex_1().child(make_copy))
    };

    dialog_modal(
        448.0,
        if is_official {
            "Official Profile Locked"
        } else {
            "Profile Locked"
        },
        Some(SharedString::from(if is_official {
            "Official material profiles can't be edited directly."
        } else {
            "This material profile is locked to prevent accidental edits."
        })),
        Some("lock"),
        IconTone::Warning,
        body,
        actions,
        close,
    )
}
