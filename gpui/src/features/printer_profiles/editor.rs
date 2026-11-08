//! The selected printer's editor pane.
//!
//! Ported from `src/components/settings/ProfileSettingsModal.tsx:5523-5946` —
//! the nested overlay the modal mounts for
//! `{isEditingPrinter && selectedPrinter && …}`: its own backdrop and card
//! (`Printer Profile Settings` / `{name} • {manufacturer || 'Generic'}` /
//! close tile), then the scrolling body of four section cards and the footer.
//!
//! Section order and copy are the source's:
//!
//! * `Official Profile — Edits Limited` banner, official profiles only
//!   (`:5547-5575`);
//! * `Identity` — `Printer Name`, `Manufacturer`, `User Notes` and the image
//!   well (`:5577-5640`);
//! * `Build Volume` — the `Auto-Calculate Width/Depth` switch, width / depth /
//!   height, then the four safety margins (`:5642-5760`);
//! * `Display` — resolution X/Y, `Bit Depth`, pixel size X/Y, `Anti-Aliasing`
//!   (`:5762-5787`);
//! * `Output` — output format, network support, webcam support/rotation,
//!   format version, settings mode, mirror X/Y, and (only for a format that
//!   carries more than one material) material mode / tank count
//!   (`:5789-5930`);
//! * footer `Changes are applied immediately.` + `Done` (`:5932-5945`).
//!
//! Every label, unit and disabled state traces to that range; the values are
//! the ones the source puts in each control
//! (`buildVolumeMm.*`, `display.*`, `bitDepth.bits`, `pixelSize.*`,
//! `safetyMarginMm.*`, `antiAliasing`, `networkSupport`, `hasCamera`,
//! `materialMode`, `tankCount`, `notes`). Writes go through
//! `model::update_profile` / `model::set_build_dimension_mode` /
//! `model::duplicate_as_custom`, exactly the store calls the source makes.
//!
//! # PORT DEVIATIONS
//!
//! 1. **Text entry.** GPUI has no editable text input in this port (the same
//!    note `features::settings` and `features::printer_picker` carry), so the
//!    `LabeledInput` fields paint their value and the number fields are stepped
//!    only. The source's `onChange` paths exist as the stepping handlers; a
//!    typed character cannot reach them yet.
//! 2. **Profile artwork.** `imageDataUrl` is a data URL and the port has no
//!    decoder or asset pipeline for it, so an uploaded image renders the
//!    well with a `printer` glyph in the image's place — the source's own
//!    fallback glyph, not an invented caption. Clicking the empty well would
//!    open a native file dialog (`triggerImageUpload`); the dialog plus the
//!    canvas-based `normalizeUploadedPrinterImageDataUrl` trim are not ported,
//!    so the well is inert. The clear tile (`Trash2`) stays wired.
//! 3. **Format catalogue.** The source reads versions, settings modes and
//!    multi-tank support from the slicing-format registry
//!    (`src/features/slicing/formats/registry.ts:151-199`,
//!    `plugins/athena/slicing/nanodlpFormatDefinition.ts:8-22`). This port
//!    ships the two built-in formats: `.lumen` has no definition (the registry
//!    labels it with its own value, `registry.ts:116`) and `.nanodlp` is the
//!    Athena format (`displayName: 'NanoDLP (Athena)'`) which declares no
//!    `formatVersions`, no `settingsModes` and no `supportsMultiTank`. So
//!    `Format Version`, `Settings Mode` and `Material Mode`/`Amount of Tanks`
//!    keep the source's own `length > 0` guards and stay hidden — the values
//!    that would reveal them do not exist in this build.
//! 4. **Network modes.** `getAvailableProfileNetworkModes`
//!    (`pluginRegistry.ts:433`) lists plugin adapters; the built-in pack
//!    registers one (`plugins/athena/pluginDefinition.ts:16-30`), `nanodlp` /
//!    `NanoDLP`. The select therefore offers the source's
//!    `None (Local only)` plus `NanoDLP`, and the source's
//!    `Unknown (<mode>)` fallback for a profile holding an unregistered mode.
//!    The fleet dialog the source opens from the rail is a separate component
//!    (`FleetManagement`) and is not this pane.
//! 5. **Official-field decoration.** The "Default" chip and the amber
//!    "modified" wash come from `SelectDropdown`'s `rightContent` / `selectStyle`.
//!    The shared `gpui_ui` trigger pins its own colours (and its 13px label
//!    against the source's `text-sm`) and only exposes a right-aligned label,
//!    so the chip renders as its `Default` text and the wash is omitted; the
//!    *disabled* states — the ones that change what the user can do — are exact.
//! 6. `grid-cols-*` is a wrapping flex layout in GPUI (the same substitution
//!    `features::scene_modals` and `features::settings` document): rows of
//!    `flex_1` cells, padded to the column count so a short last row keeps the
//!    source's track widths.
//! 7. `bg-black/55` is `rgba(BLACK, 0x8c)`; `backdrop-blur`, `transition-*`
//!    and `tracking-wide` have no GPUI equivalent and are omitted.
//! 8. `max-h-[88vh]` is `max_h(relative(0.88))` against the overlay, which is
//!    the window-sized layer the source's `fixed inset-0` paints.
//!
//! `model::PrinterProfile` carries every field this pane reads; nothing is
//! synthesised. The transient dropdown-open state lives in [`UI`] because the
//! pane is a free function re-run each frame, not an entity.

use std::sync::{LazyLock, Mutex, MutexGuard};

use gpui::{
    App, ClickEvent, Div, FontWeight, IntoElement, SharedString, WeakEntity, Window, div,
    prelude::*, px, relative, rgb, rgba,
};

use gpui_core::icons::icon;
use gpui_core::theme::*;
use gpui_ui::panel_fields::{BLACK, WHITE, with_alpha};
use gpui_ui::ui::{
    ButtonSize, ButtonSpec, ButtonVariant, IconButtonSize, IconButtonVariant, SelectOption,
    button_with, icon_button, select_dropdown,
};

use crate::features::settings::widgets::number_input;
use crate::DragonFruit;

use super::model::{self, BitDepth, PixelSize, PrinterProfile, SafetyMarginMm};

/// `#d97706` — amber-600, the official-profile banner's accent (`:5551`).
const AMBER_600: u32 = 0xd97706;
/// `#f59e0b` — amber-500, `MODIFIED_FIELD_STYLE` (`profileFormAtoms.tsx:545`).
const AMBER_500: u32 = 0xf59e0b;
/// `#fca5a5` — red-300, the clear-image tile's glyph (`:5636`).
const RED_300: u32 = 0xfca5a5;
/// `printerImageWellBackground`'s dark-theme literal (`:1650-1652`).
const IMAGE_WELL: u32 = 0x1c2027;

/// `color-mix(in srgb, a, b N%)` — `b`'s share is the source's percentage.
fn mix_b(a: u32, b: u32, weight_b: f32) -> u32 {
    mix(a, b, 1.0 - weight_b)
}

/// A CSS `box-shadow` layer.
fn shadow(x: f32, y: f32, blur: f32, spread: f32, color: u32) -> gpui::BoxShadow {
    gpui::BoxShadow::new(px(x), px(y), rgba(color).into())
        .blur_radius(px(blur))
        .spread_radius(px(spread))
}

// ── Transient UI state ──────────────────────────────────────────────────────

/// The one dropdown the pane may hold open. `SelectDropdown` keeps this in
/// React state; the pane is a free function here, so it lives beside it (the
/// same shape `features::settings` uses for its own selects).
#[derive(Default)]
struct EditorUi {
    open_select: Option<&'static str>,
}

static UI: LazyLock<Mutex<EditorUi>> = LazyLock::new(|| Mutex::new(EditorUi::default()));

fn ui() -> MutexGuard<'static, EditorUi> {
    UI.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

// ── Source derivations ──────────────────────────────────────────────────────

/// One built-in slicing format, as the registry resolves it for this port
/// (deviation 3).
struct FormatSpec {
    /// `outputFormat` — the `PrinterOutputFormat` string, dots included.
    output: &'static str,
    /// `displayName`, or the value itself when no definition claims it.
    label: &'static str,
    /// `getAvailableFormatVersionOptions` (`registry.ts:151`).
    versions: &'static [(&'static str, &'static str)],
    /// `getAvailableSettingsModeOptions` (`registry.ts:187`).
    modes: &'static [(&'static str, &'static str)],
    /// `outputFormatSupportsMultiTank` (`registry.ts:70`).
    multi_tank: bool,
}

/// `getAvailableOutputFormatOptions()` over the formats this build ships,
/// sorted by `localeCompare` as the source sorts them.
const FORMATS: &[FormatSpec] = &[
    // `profiles/printers/generic/printers.json` writes `.lumen`; no built-in
    // plugin owns it, so `registry.ts:116` falls through to the value.
    FormatSpec {
        output: ".lumen",
        label: ".lumen",
        versions: &[],
        modes: &[],
        multi_tank: false,
    },
    // `plugins/athena/slicing/nanodlpFormatDefinition.ts:8-22`.
    FormatSpec {
        output: ".nanodlp",
        label: "NanoDLP (Athena)",
        versions: &[],
        modes: &[],
        multi_tank: false,
    },
];

fn format_spec(output_format: &str) -> Option<&'static FormatSpec> {
    FORMATS
        .iter()
        .find(|spec| spec.output.eq_ignore_ascii_case(output_format.trim()))
}

/// `getAvailableProfileNetworkModes()` for the built-in pack
/// (`plugins/athena/pluginDefinition.ts:16-30`, deviation 4).
const NETWORK_MODES: &[(&str, &str)] = &[("nanodlp", "NanoDLP")];

const WEBCAM_ROTATION_OPTIONS: &[(&str, &str)] =
    &[("0", "0°"), ("90", "90°"), ("180", "180°"), ("270", "270°")];

const MATERIAL_MODE_OPTIONS: &[(&str, &str)] =
    &[("single", "Single Material"), ("multiTank", "Multi-Tank")];

/// `normalizeFormatVersion` / `normalizeSettingsMode`
/// (`src/features/profiles/outputFormatUtils.ts:46-70`):
/// `^[a-z0-9][a-z0-9._-]{0,63}$`, trimmed, empty is `undefined`.
fn normalize_tag(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    if trimmed.is_empty() || trimmed.len() > 64 {
        return None;
    }
    let mut chars = trimmed.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphanumeric() {
        return None;
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return None;
    }
    Some(trimmed.to_owned())
}

/// `resolveOutputFormatVersion` (`registry.ts:165`) with this port's (empty)
/// version catalogue: the normalized request, else the first option.
fn resolve_format_version(output_format: &str, requested: Option<&str>) -> Option<String> {
    let normalized = normalize_tag(requested);
    let options = format_spec(output_format).map(|spec| spec.versions).unwrap_or(&[]);
    if options.is_empty() {
        return normalized;
    }
    if let Some(requested) = normalized {
        if let Some((value, _)) = options
            .iter()
            .find(|(value, _)| value.eq_ignore_ascii_case(&requested))
        {
            return Some((*value).to_owned());
        }
    }
    options.first().map(|(value, _)| (*value).to_owned())
}

/// `resolveOutputSettingsMode` (`registry.ts:227`).
fn resolve_settings_mode(output_format: &str, requested: Option<&str>) -> Option<String> {
    let normalized = normalize_tag(requested);
    let options = format_spec(output_format).map(|spec| spec.modes).unwrap_or(&[]);
    if options.is_empty() {
        return normalized;
    }
    if let Some(requested) = normalized {
        if let Some((value, _)) = options
            .iter()
            .find(|(value, _)| value.eq_ignore_ascii_case(&requested))
        {
            return Some((*value).to_owned());
        }
    }
    options.first().map(|(value, _)| (*value).to_owned())
}

/// `resolveBuiltinPluginSlicingFormat(...)?.supportsMultiTank === true`.
fn supports_multi_tank(output_format: &str) -> bool {
    format_spec(output_format).is_some_and(|spec| spec.multi_tank)
}

/// `officialFieldDefaults` (`:706-730`): what the preset an official profile
/// came from shipped, so the controls can mark it. Only the members this pane
/// can paint are kept (see [`has_camera_modified`]).
struct OfficialDefaults {
    material_mode: String,
    output_format: String,
    network_support: String,
    has_camera: bool,
    format_version: Option<String>,
    settings_mode: Option<String>,
}

fn official_defaults(profile: &PrinterProfile) -> Option<OfficialDefaults> {
    if !profile.is_official {
        return None;
    }
    let preset = model::preset_for(profile)?;
    Some(OfficialDefaults {
        material_mode: preset
            .material_mode
            .clone()
            .unwrap_or_else(|| "single".to_owned()),
        output_format: preset.display.output_format.trim().to_lowercase(),
        network_support: preset
            .network_support
            .clone()
            .unwrap_or_default()
            .trim()
            .to_lowercase(),
        has_camera: preset.has_camera != Some(false),
        format_version: normalize_tag(preset.display.format_version.as_deref()),
        settings_mode: normalize_tag(preset.display.settings_mode.as_deref()),
    })
}

/// `officialFieldsModified.hasCamera` (`:745-766`): the one flag this pane can
/// paint. The source's other flags (`materialMode`, `outputFormat`,
/// `networkSupport`, `webcamRotationDeg`, `formatVersion`, `settingsMode`) feed
/// `LabeledSelectInput`'s amber `selectStyle`, which the shared GPUI trigger
/// keeps to itself (deviation 5).
fn has_camera_modified(profile: &PrinterProfile, defaults: Option<&OfficialDefaults>) -> bool {
    let Some(defaults) = defaults else {
        return false;
    };
    (profile.has_camera != Some(false)) != defaults.has_camera
}

// ── Control kit ─────────────────────────────────────────────────────────────

/// `updatePrinterProfile(...)` followed by the re-render the store's
/// subscription gives the source.
fn write(
    entity: &WeakEntity<DragonFruit>,
    id: &str,
    f: impl FnOnce(&mut PrinterProfile),
    cx: &mut App,
) {
    model::update_profile(id, f);
    entity.update(cx, |_, cx| cx.notify()).ok();
}

/// `setIsEditingPrinter(false)`. The dropdown's open state is React state in
/// the source and goes with the unmounted pane; here it is reset explicitly.
fn close_editor(entity: &WeakEntity<DragonFruit>, cx: &mut App) {
    {
        let mut state = ui();
        state.open_select = None;
    }
    model::set_editing(false);
    entity.update(cx, |_, cx| cx.notify()).ok();
}

/// `.ui-label font-medium` — 13px, `--indicator`, medium.
fn field_label(text: &str) -> Div {
    div()
        .text_size(px(FONT_SM))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(INDICATOR))
        .child(SharedString::from(text.to_owned()))
}

/// `ui-meta font-semibold uppercase tracking-wide mb-2`.
fn section_title(text: &str) -> Div {
    div()
        .mb(px(8.0))
        .text_size(px(FONT_SM))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(INDICATOR))
        .child(SharedString::from(text.to_uppercase()))
}

/// `rounded-md border p-3` on `--surface-2`.
fn section_card() -> Div {
    div()
        .flex()
        .flex_col()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_2))
        .p(px(12.0))
}

/// `space-y-1 block`: the label above its control.
fn field_shell(text: &str) -> Div {
    div()
        .flex_1()
        .min_w(px(0.0))
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(field_label(text))
}

/// `LabeledNumberInput`: `min` defaults to 0, no default upper cap, and the
/// stepper's own default step (`|value| < 1 ? 0.01 : 1`) is the caller's.
fn labeled_number(
    id: &'static str,
    label: &str,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    disabled: bool,
    on_change: impl Fn(f32, &mut App) + 'static,
) -> Div {
    let background = if disabled {
        // `color-mix(in srgb, var(--surface-2), black 8%)`.
        mix(SURFACE_2, BLACK, 0.92)
    } else {
        SURFACE_0
    };
    field_shell(label).child(number_input(
        id,
        value,
        min,
        max,
        step,
        None,
        36.0,
        // `.ui-input` is unlayered, so its `font-size: var(--font-size-sm)`
        // (13px) outranks the field's `text-sm` in the source.
        FONT_SM,
        background,
        !disabled,
        on_change,
    ))
}

/// `.ui-input w-full h-[36px] px-2.5 leading-tight text-sm`, painted rather
/// than typed into (deviation 1). The unlayered `.ui-input` rule keeps its
/// 13px type and 1.3 line-height against the field's `text-sm`.
fn readonly_input(value: &str) -> Div {
    div()
        .h(px(36.0))
        .w_full()
        .flex()
        .items_center()
        .rounded(px(ROUNDED_SM))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .px(px(10.0))
        .text_size(px(FONT_SM))
        .line_height(px(FONT_SM * 1.3))
        .text_color(rgb(TEXT_STRONG))
        .child(SharedString::from(value.to_owned()))
}

/// `LabeledInput`'s read-only painting (deviation 1).
fn labeled_text(label: &str, value: &str, disabled: bool) -> Div {
    field_shell(label).child(
        div()
            .when(disabled, |this| this.opacity(0.55))
            .child(readonly_input(value)),
    )
}

/// `LabeledSelectInput` / `SelectDropdown` with `h-[36px] px-2.5 pr-10 text-sm`.
/// The "Default" option carries the source's `DefaultOptionChip` label
/// (deviation 5); the amber wash has no seam in the shared trigger.
fn labeled_select(
    id: &'static str,
    label: &str,
    value: &str,
    options: &[(&str, &str)],
    default_value: Option<&str>,
    enabled: bool,
    entity: &WeakEntity<DragonFruit>,
    on_change: impl Fn(&str, &mut App) + 'static,
) -> Div {
    let entity = WeakEntity::clone(entity);
    let open = ui().open_select == Some(id);
    let mut items: Vec<SelectOption> = Vec::with_capacity(options.len());
    for (option_value, option_label) in options {
        let mut option = SelectOption::new(option_value.to_string(), option_label.to_string());
        if default_value == Some(*option_value) {
            option = option.with_right("Default");
        }
        items.push(option);
    }
    let values: Vec<String> = options
        .iter()
        .map(|(option_value, _)| (*option_value).to_owned())
        .collect();
    let select: std::rc::Rc<dyn Fn(&str, &mut App)> = std::rc::Rc::new(on_change);
    let toggle_entity = entity.clone();
    let select_entity = entity.clone();

    field_shell(label).child(select_dropdown(
        id,
        &items,
        value,
        open,
        !enabled,
        move |_, _, cx| {
            {
                let mut state = ui();
                state.open_select = if state.open_select == Some(id) {
                    None
                } else {
                    Some(id)
                };
            }
            toggle_entity.update(cx, |_, cx| cx.notify()).ok();
        },
        move |index, _, cx| {
            if let Some(chosen) = values.get(index) {
                select(chosen.as_str(), cx);
            }
            {
                let mut state = ui();
                state.open_select = None;
            }
            select_entity.update(cx, |_, cx| cx.notify()).ok();
        },
    ))
}

/// `LabeledToggleInput`: the `ui-input`-shaped switch with its
/// `Enabled` / `Disabled` caption.
fn labeled_toggle(
    id: &'static str,
    label: &str,
    checked: bool,
    disabled: bool,
    highlight: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let (border, background, text) = if disabled {
        (
            BORDER_SUBTLE,
            mix_b(SURFACE_2, BLACK, 0.08),
            TEXT_MUTED,
        )
    } else {
        (
            if highlight {
                mix_b(AMBER_500, BORDER_SUBTLE, 0.25)
            } else if checked {
                mix_b(ACCENT_SECONDARY, BORDER_SUBTLE, 0.36)
            } else {
                BORDER_SUBTLE
            },
            if checked {
                mix_b(ACCENT_SECONDARY, SURFACE_1, 0.90)
            } else {
                SURFACE_1
            },
            if checked { TEXT_STRONG } else { TEXT_MUTED },
        )
    };
    let track = if disabled {
        mix_b(SURFACE_2, BLACK, 0.08)
    } else if checked {
        ACCENT_SECONDARY
    } else {
        SURFACE_2
    };

    let switch = div()
        .h(px(20.0))
        .w(px(36.0))
        .flex_shrink_0()
        .rounded_full()
        .p(px(2.0))
        .flex()
        .items_center()
        .bg(rgb(track))
        .child(
            div()
                .h(px(16.0))
                .w(px(16.0))
                .rounded_full()
                .bg(rgb(WHITE)),
        );
    let switch = if checked {
        switch.justify_end()
    } else {
        switch.justify_start()
    };

    let mut control = div()
        .id(id)
        .h(px(36.0))
        .w_full()
        .px(px(10.0))
        .rounded(px(ROUNDED_SM))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        // `.ui-input`'s unlayered `font-size: 13px` / `line-height: 1.3`.
        .text_size(px(FONT_SM))
        .line_height(px(FONT_SM * 1.3))
        .text_color(rgb(text))
        .flex()
        .items_center()
        .justify_between()
        .child(SharedString::from(if checked { "Enabled" } else { "Disabled" }))
        .child(switch);
    if disabled {
        control = control.opacity(0.55).cursor_not_allowed();
    } else {
        control = control.cursor_pointer().on_click(on_click);
    }

    field_shell(label).child(control)
}

/// `grid-cols-N`: rows of `flex_1` cells, the last row padded so its tracks
/// keep the source's width (deviation 6).
fn grid_rows(mut cells: Vec<Div>, columns: usize, gap: f32) -> Div {
    let mut rows = div().flex().flex_col().gap(px(gap));
    while !cells.is_empty() {
        let take = columns.min(cells.len());
        let mut row = div().flex().gap(px(gap));
        for _ in 0..take {
            row = row.child(cells.remove(0));
        }
        for _ in take..columns {
            row = row.child(div().flex_1());
        }
        rows = rows.child(row);
    }
    rows
}

/// `accentSecondaryActionStyle92` (`:1637-1649`), dark theme.
fn accent_secondary_action() -> (u32, u32) {
    (
        mix_b(ACCENT_SECONDARY, SURFACE_1, 0.92),
        mix_b(ACCENT_SECONDARY, BORDER_SUBTLE, 0.42),
    )
}

// ── The pane ────────────────────────────────────────────────────────────────

/// The source's nested overlay (`isEditingPrinter && selectedPrinter`,
/// `ProfileSettingsModal.tsx:5523`). Mounted by `mod.rs` inside the outer modal;
/// its backdrop is `absolute` and sized to that parent.
pub fn printer_editor(
    profile: model::PrinterProfile,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let official = profile.is_official;
    let defaults = official_defaults(&profile);
    let camera_modified = has_camera_modified(&profile, defaults.as_ref());

    let subtitle = format!(
        "{} • {}",
        profile.name,
        profile.manufacturer.clone().unwrap_or_else(|| "Generic".to_owned())
    );

    // Header: `flex items-center justify-between px-4 py-3 border-b`.
    let close_entity = entity.clone();
    let header = div()
        .flex()
        .items_center()
        .justify_between()
        .px(px(16.0))
        .py(px(12.0))
        .border_b(px(1.0))
        .border_color(rgb(BORDER_SUBTLE))
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(FONT_MD))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("Printer Profile Settings"),
                )
                .child(
                    div()
                        .text_size(px(FONT_SM))
                        .text_color(rgb(INDICATOR))
                        .child(SharedString::from(subtitle)),
                ),
        )
        .child(
            icon_button(
                "printer-editor-close",
                "x",
                IconButtonVariant::Surface,
                IconButtonSize::Sm,
            )
            .on_click(move |_, _, cx| {
                close_editor(&close_entity, cx);
            }),
        );

    // Body: `p-3 space-y-3 overflow-y-auto flex-1`.
    let mut body = div()
        .id("printer-editor-body")
        .p(px(12.0))
        .flex()
        .flex_col()
        .gap(px(12.0))
        .flex_1()
        .min_h(px(0.0))
        .overflow_y_scroll();

    if official {
        body = body.child(official_banner(&profile.id, &entity));
    }
    body = body.child(identity_section(&profile, &entity, official));
    body = body.child(build_volume_section(&profile, &entity, official));
    body = body.child(display_section(&profile, &entity, official));
    body = body.child(output_section(
        &profile,
        &entity,
        defaults.as_ref(),
        camera_modified,
    ));

    // Footer: `px-3 py-3 border-t flex items-center justify-between gap-2`.
    let done_entity = entity.clone();
    let (done_bg, done_border) = accent_secondary_action();
    let done_spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Auto)
        .padding_x(12.0)
        .padding_y(0.0, 0.0)
        .font_size(FONT_MD)
        .radius(999.0)
        .gap(4.0)
        .fill(done_bg, done_border, ACCENT_SECONDARY)
        .icon("check", 14.0, ACCENT_SECONDARY);
    let footer = div()
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .px(px(12.0))
        .py(px(12.0))
        .border_t(px(1.0))
        .border_color(rgb(BORDER_SUBTLE))
        .child(
            div()
                .text_size(px(FONT_XS))
                .text_color(rgb(TEXT_MUTED))
                .child("Changes are applied immediately."),
        )
        .child(
            button_with("printer-editor-done", "Done", &done_spec)
                .h(px(32.0))
                .on_click(move |_, _, cx| {
                    close_editor(&done_entity, cx);
                }),
        );

    // `fixed inset-0 z-[60] flex items-center justify-center bg-black/55 p-4`.
    let card = div()
        .w_full()
        .max_w(px(960.0))
        .max_h(relative(0.88))
        .rounded(px(ROUNDED_XL))
        .border_1()
        .border_color(rgb(BORDER_STRONG))
        .bg(rgb(SURFACE_0))
        .overflow_hidden()
        .flex()
        .flex_col()
        .shadow(vec![shadow(0.0, 25.0, 50.0, -12.0, with_alpha(BLACK, 0x40))])
        .child(header)
        .child(body)
        .child(footer);

    div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .p(px(16.0))
        .bg(rgba(with_alpha(BLACK, 0x8c)))
        .child(card)
}

// ── Sections ────────────────────────────────────────────────────────────────

/// `Official Profile — Edits Limited` + `Create Custom Copy` (`:5547-5575`).
fn official_banner(profile_id: &str, entity: &WeakEntity<DragonFruit>) -> Div {
    let entity = WeakEntity::clone(entity);
    let (background, border) = (
        mix_b(AMBER_600, SURFACE_1, 0.92),
        mix_b(AMBER_600, BORDER_SUBTLE, 0.36),
    );
    let (copy_bg, copy_border) = accent_secondary_action();
    let spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Auto)
        .padding_x(12.0)
        .padding_y(0.0, 0.0)
        .font_size(FONT_MD)
        .radius(ROUNDED_MD)
        .gap(4.0)
        .fill(copy_bg, copy_border, ACCENT_SECONDARY)
        .icon("plus", 14.0, ACCENT_SECONDARY);

    // `handleDuplicateSelectedPrinterAsCustom` (`:3526-3531`): the copy is made,
    // picked, and the editor stays open on it.
    let click_entity = entity.clone();
    let profile_id = profile_id.to_owned();
    let copy = button_with("printer-editor-custom-copy", "Create Custom Copy", &spec)
        .h(px(32.0))
        .on_click(move |_, _, cx| {
            if let Some(copy_id) = model::duplicate_as_custom(&profile_id) {
                model::open_editor(&copy_id);
            }
            click_entity.update(cx, |_, cx| cx.notify()).ok();
        });

    div()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .px(px(12.0))
        .py(px(10.0))
        .flex()
        .flex_wrap()
        .items_center()
        // `gap-x-3 gap-y-1.5`; GPUI has one gap axis here.
        .gap(px(8.0))
        .child(
            div()
                .flex_1()
                .min_w(px(240.0))
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .text_size(px(FONT_MD))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child(icon("triangle-alert", 16.0, AMBER_600))
                        .child("Official Profile — Edits Limited"),
                )
                .child(
                    div()
                        .mt(px(2.0))
                        .text_size(px(FONT_XS))
                        .line_height(relative(1.375))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Only some fields are editable here; everything else unlocks with a custom copy. Custom copies may increase the risk of print failure or injury."),
                ),
        )
        .child(div().flex_shrink_0().child(copy))
}

/// `Identity` (`:5577-5640`).
fn identity_section(profile: &PrinterProfile, entity: &WeakEntity<DragonFruit>, official: bool) -> Div {
    let fields = div()
        .flex_1()
        .min_w(px(0.0))
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(labeled_text("Printer Name", &profile.name, official))
        .child(labeled_text(
            "Manufacturer",
            profile.manufacturer.as_deref().unwrap_or(""),
            official,
        ))
        .child(labeled_text(
            "User Notes",
            profile.notes.as_deref().unwrap_or(""),
            false,
        ));

    section_card()
        .child(section_title("Identity"))
        .child(
            div()
                .flex()
                .gap(px(12.0))
                .items_stretch()
                .child(fields)
                .child(image_well(profile, entity, official)),
        )
}

/// The `min-h-[190px]` image well and its clear tile (`:5590-5640`).
fn image_well(profile: &PrinterProfile, entity: &WeakEntity<DragonFruit>, official: bool) -> Div {
    let entity = WeakEntity::clone(entity);
    let has_image = profile
        .image_data_url
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());

    let mut well = div()
        .relative()
        .w(px(220.0))
        .flex_shrink_0()
        .min_h(px(190.0))
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(IMAGE_WELL))
        .overflow_hidden()
        .flex()
        .items_center()
        .justify_center();

    if has_image {
        // The bitmap is a data URL with no decoder here: the source's own
        // fallback glyph stands in for `AutoTrimmedImage` (deviation 2).
        well = well.child(icon("printer", 24.0, TEXT_MUTED));
    } else {
        let caption = if official {
            "No preview image"
        } else {
            "Click to add a profile image"
        };
        well = well.child(
            div()
                .px(px(12.0))
                .text_center()
                .text_size(px(FONT_XS))
                .text_color(rgb(TEXT_MUTED))
                .child(div().flex().justify_center().mb(px(4.0)).child(icon(
                    "image-plus",
                    24.0,
                    TEXT_MUTED,
                )))
                .child(caption),
        );
    }

    if has_image && !official {
        // `absolute top-1.5 right-1.5` surface tile with a red `Trash2`.
        let id = profile.id.clone();
        let clear_entity = entity.clone();
        well = well.child(
            div()
                .id("printer-editor-clear-image")
                .absolute()
                .top(px(6.0))
                .right(px(6.0))
                .w(px(32.0))
                .h(px(32.0))
                .flex()
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .rounded(px(ROUNDED_MD))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_1))
                .cursor_pointer()
                .child(icon("trash-2", 14.0, RED_300))
                .on_click(move |_, _, cx| {
                    write(&clear_entity, &id, |p| p.image_data_url = None, cx);
                }),
        );
    }

    well
}

/// `Build Volume` (`:5642-5760`).
fn build_volume_section(
    profile: &PrinterProfile,
    entity: &WeakEntity<DragonFruit>,
    official: bool,
) -> Div {
    let entity = WeakEntity::clone(entity);
    let auto = profile.auto_build_dimensions();
    let margins = profile.safety_margin_mm.unwrap_or_default();
    let id = profile.id.clone();

    // `setBuildDimensionMode` (`:846-884`): the store remembers the manual pair
    // and swaps the mode.
    let toggle_entity = entity.clone();
    let toggle_id = id.clone();
    let (auto_bg, auto_border) = accent_secondary_action();
    let mut toggle_spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Auto)
        .padding_x(12.0)
        .padding_y(0.0, 0.0)
        .font_size(FONT_XS)
        .radius(ROUNDED_MD);
    if auto {
        toggle_spec = toggle_spec.fill(auto_bg, auto_border, ACCENT_SECONDARY);
    }
    if official {
        toggle_spec = toggle_spec.disabled();
    }
    let toggle_label = if auto { "Auto" } else { "Manual" };
    let mut toggle = button_with("printer-editor-build-mode", toggle_label, &toggle_spec).h(px(32.0));
    if !official {
        toggle = toggle.on_click(move |_, _, cx| {
            let next = if auto { "manual" } else { "auto" };
            model::set_build_dimension_mode(&toggle_id, next);
            toggle_entity.update(cx, |_, cx| cx.notify()).ok();
        });
    }

    // `mb-3 rounded-md border p-2.5` on `color-mix(surface-1, transparent 7%)`
    // over the section's `surface-2`.
    let auto_card = div()
        .mb(px(12.0))
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(mix_b(SURFACE_1, SURFACE_2, 0.07)))
        .p(px(10.0))
        .flex()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(FONT_MD))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("Auto-Calculate Width/Depth"),
                )
                .child(
                    div()
                        .text_size(px(FONT_XS))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Uses Resolution × Pixel Size. Non-destructive: switching back restores previous manual width/depth."),
                ),
        )
        .child(toggle);

    let volume_entity = entity.clone();
    let volume_id = id.clone();
    let width_value = profile.effective_width_mm();
    let width_disabled = auto || official;
    let volume_entity_2 = entity.clone();
    let volume_id_2 = id.clone();
    let depth_value = profile.effective_depth_mm();
    let height_entity = entity.clone();
    let height_id = id.clone();

    let mut cells: Vec<Div> = Vec::with_capacity(3);
    cells.push(labeled_number(
        "printer-editor-build-width",
        "Build Width (mm)",
        width_value,
        0.0,
        f32::INFINITY,
        step_for(width_value),
        width_disabled,
        move |value, cx| {
            write(
                &volume_entity,
                &volume_id,
                |p| p.build_volume_mm.width = Some(value.max(0.0)),
                cx,
            );
        },
    ));
    cells.push(labeled_number(
        "printer-editor-build-depth",
        "Build Depth (mm)",
        depth_value,
        0.0,
        f32::INFINITY,
        step_for(depth_value),
        width_disabled,
        move |value, cx| {
            write(
                &volume_entity_2,
                &volume_id_2,
                |p| p.build_volume_mm.depth = Some(value.max(0.0)),
                cx,
            );
        },
    ));
    cells.push(labeled_number(
        "printer-editor-build-height",
        "Build Height (mm)",
        profile.height_mm(),
        0.0,
        f32::INFINITY,
        step_for(profile.height_mm()),
        official,
        move |value, cx| {
            write(
                &height_entity,
                &height_id,
                |p| p.build_volume_mm.height = Some(value.max(0.0)),
                cx,
            );
        },
    ));

    // Safety margins: `clampNonNegativeNumber(safetyMarginMm?.front ?? 0)`.
    let margin_cells = [
        (
            "printer-editor-margin-front",
            "Front Margin (mm)",
            MarginEdge::Front,
            margins.front,
        ),
        (
            "printer-editor-margin-back",
            "Back Margin (mm)",
            MarginEdge::Back,
            margins.back,
        ),
        (
            "printer-editor-margin-left",
            "Left Margin (mm)",
            MarginEdge::Left,
            margins.left,
        ),
        (
            "printer-editor-margin-right",
            "Right Margin (mm)",
            MarginEdge::Right,
            margins.right,
        ),
    ];
    let mut margins_vec: Vec<Div> = Vec::with_capacity(4);
    for (field_id, label, edge, value) in margin_cells {
        let value = value.max(0.0);
        let entity = entity.clone();
        let profile_id = id.clone();
        margins_vec.push(labeled_number(
            field_id,
            label,
            value,
            0.0,
            f32::INFINITY,
            step_for(value),
            official,
            move |value, cx| {
                write(
                    &entity,
                    &profile_id,
                    |p| {
                        let margin = p.safety_margin_mm.get_or_insert_default();
                        *edge.slot(margin) = value.max(0.0);
                    },
                    cx,
                );
            },
        ));
    }

    section_card()
        .child(section_title("Build Volume"))
        .child(auto_card)
        .child(grid_rows(cells, 3, 10.0))
        .child(
            div()
                .mt(px(10.0))
                .child(grid_rows(margins_vec, 4, 10.0)),
        )
}

/// `Display` (`:5762-5787`).
fn display_section(profile: &PrinterProfile, entity: &WeakEntity<DragonFruit>, official: bool) -> Div {
    let entity = WeakEntity::clone(entity);
    let id = profile.id.clone();
    let resolution = &profile.display;
    let pixel_size = profile.pixel_size.unwrap_or(PixelSize { x: 1.0, y: 1.0 });

    let res_x_entity = entity.clone();
    let res_x_id = id.clone();
    let res_x = resolution.resolution_x;
    let res_y_entity = entity.clone();
    let res_y_id = id.clone();
    let res_y = resolution.resolution_y;
    let bits_entity = entity.clone();
    let bits_id = id.clone();
    let bits = resolution_bits(profile);
    let pixel_x_entity = entity.clone();
    let pixel_x_id = id.clone();
    let pixel_y_entity = entity.clone();
    let pixel_y_id = id.clone();
    let aa_entity = entity.clone();
    let aa_id = id.clone();

    let cells: Vec<Div> = vec![
        // `Math.max(1, Math.round(value))`, and in auto mode the derived
        // width/depth follow the new resolution (`handlePrinterDisplayChange`).
        labeled_number(
            "printer-editor-resolution-x",
            "Resolution X (px)",
            res_x,
            0.0,
            f32::INFINITY,
            step_for(res_x),
            official,
            move |value, cx| {
                let next = value.round().max(1.0);
                write(
                    &res_x_entity,
                    &res_x_id,
                    |p| {
                        p.display.resolution_x = next;
                        sync_auto_build_volume(p);
                    },
                    cx,
                );
            },
        ),
        labeled_number(
            "printer-editor-resolution-y",
            "Resolution Y (px)",
            res_y,
            0.0,
            f32::INFINITY,
            step_for(res_y),
            official,
            move |value, cx| {
                let next = value.round().max(1.0);
                write(
                    &res_y_entity,
                    &res_y_id,
                    |p| {
                        p.display.resolution_y = next;
                        sync_auto_build_volume(p);
                    },
                    cx,
                );
            },
        ),
        // `bitDepth.bits ?? 8`; `handlePrinterBitDepthChange` keeps the
        // description the preset shipped.
        labeled_number(
            "printer-editor-bit-depth",
            "Bit Depth",
            bits as f32,
            0.0,
            f32::INFINITY,
            step_for(bits as f32),
            official,
            move |value, cx| {
                let bits = value.round().max(1.0);
                write(
                    &bits_entity,
                    &bits_id,
                    |p| {
                        let description = p
                            .bit_depth
                            .as_ref()
                            .and_then(|depth| depth.description.clone());
                        p.bit_depth = Some(BitDepth {
                            bits,
                            description,
                        });
                    },
                    cx,
                );
            },
        ),
        // `pixelSize?.x ?? 1`, floored at `0.001` (`handlePrinterPixelSizeChange`).
        labeled_number(
            "printer-editor-pixel-size-x",
            "Pixel Size X (μm)",
            pixel_size.x,
            0.0,
            f32::INFINITY,
            step_for(pixel_size.x),
            official,
            move |value, cx| {
                let safe = safe_pixel_size(value);
                write(
                    &pixel_x_entity,
                    &pixel_x_id,
                    |p| {
                        let y = p.pixel_size.map(|size| size.y).unwrap_or(1.0);
                        p.pixel_size = Some(PixelSize { x: safe, y });
                        sync_auto_build_volume(p);
                    },
                    cx,
                );
            },
        ),
        labeled_number(
            "printer-editor-pixel-size-y",
            "Pixel Size Y (μm)",
            pixel_size.y,
            0.0,
            f32::INFINITY,
            step_for(pixel_size.y),
            official,
            move |value, cx| {
                let safe = safe_pixel_size(value);
                write(
                    &pixel_y_entity,
                    &pixel_y_id,
                    |p| {
                        let x = p.pixel_size.map(|size| size.x).unwrap_or(1.0);
                        p.pixel_size = Some(PixelSize { x, y: safe });
                        sync_auto_build_volume(p);
                    },
                    cx,
                );
            },
        ),
        // `antiAliasing === true`.
        labeled_toggle(
            "printer-editor-anti-aliasing",
            "Anti-Aliasing",
            profile.anti_aliasing == Some(true),
            official,
            false,
            move |_, _, cx| {
                write(
                    &aa_entity,
                    &aa_id,
                    |p| p.anti_aliasing = Some(p.anti_aliasing != Some(true)),
                    cx,
                );
            },
        ),
    ];

    section_card()
        .child(section_title("Display"))
        .child(grid_rows(cells, 3, 10.0))
}

/// `Output` (`:5789-5930`).
fn output_section(
    profile: &PrinterProfile,
    entity: &WeakEntity<DragonFruit>,
    defaults: Option<&OfficialDefaults>,
    camera_modified: bool,
) -> Div {
    let entity = WeakEntity::clone(entity);
    let id = profile.id.clone();
    let output_format = profile.display.output_format.clone();
    let spec = format_spec(&output_format);
    let format_options: Vec<(&'static str, &'static str)> = FORMATS
        .iter()
        .map(|spec| (spec.output, spec.label))
        .collect();
    let default_output = defaults.map(|defaults| defaults.output_format.as_str());

    // `getAvailableFormatVersionOptions` / `getAvailableSettingsModeOptions`.
    let version_options: Vec<(&'static str, &'static str)> =
        spec.map(|spec| spec.versions).unwrap_or(&[]).to_vec();
    let mode_options: Vec<(&'static str, &'static str)> =
        spec.map(|spec| spec.modes).unwrap_or(&[]).to_vec();
    let resolved_version =
        resolve_format_version(&output_format, profile.display.format_version.as_deref());
    let resolved_mode =
        resolve_settings_mode(&output_format, profile.display.settings_mode.as_deref());
    let multi_tank_format = supports_multi_tank(&output_format);
    let has_camera = profile.has_camera != Some(false);

    let format_entity = entity.clone();
    let format_id = id.clone();
    let network_entity = entity.clone();
    let network_id = id.clone();
    let camera_entity = entity.clone();
    let camera_id = id.clone();
    let rotation_entity = entity.clone();
    let rotation_id = id.clone();
    let version_entity = entity.clone();
    let version_id = id.clone();
    let mode_entity = entity.clone();
    let mode_id = id.clone();
    let mirror_x_entity = entity.clone();
    let mirror_x_id = id.clone();
    let mirror_y_entity = entity.clone();
    let mirror_y_id = id.clone();
    let material_entity = entity.clone();
    let material_id = id.clone();
    let tank_entity = entity.clone();
    let tank_id = id.clone();

    let network_value = profile
        .network_support
        .clone()
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    // `selectedPrinterNetworkModeOptions` (`:1254-1264`).
    let unknown_network = if !network_value.is_empty()
        && !NETWORK_MODES
            .iter()
            .any(|(mode, _)| mode.eq_ignore_ascii_case(&network_value))
    {
        Some(format!("Unknown ({network_value})"))
    } else {
        None
    };
    let mut network_options: Vec<(&str, &str)> = vec![("", "None (Local only)")];
    for (mode, label) in NETWORK_MODES.iter() {
        network_options.push((*mode, *label));
    }
    if let Some(unknown) = unknown_network.as_deref() {
        network_options.push((network_value.as_str(), unknown));
    }

    let mut cells: Vec<Div> = Vec::with_capacity(10);
    cells.push(labeled_select(
        "printer-editor-output-format",
        "Output Format",
        &output_format,
        &format_options,
        default_output,
        true,
        &entity,
        move |value, cx| {
            // `handlePrinterDisplayChange`: the format's version and settings
            // mode are re-resolved against the new format (`:906-931`).
            let value = value.to_owned();
            write(
                &format_entity,
                &format_id,
                |p| {
                    p.display.output_format = value.clone();
                    p.display.format_version = resolve_format_version(
                        &value,
                        p.display.format_version.as_deref(),
                    );
                    p.display.settings_mode =
                        resolve_settings_mode(&value, p.display.settings_mode.as_deref());
                    sync_auto_build_volume(p);
                },
                cx,
            );
        },
    ));
    cells.push(labeled_select(
        "printer-editor-network-support",
        "Network Support",
        &network_value,
        &network_options,
        defaults.map(|defaults| defaults.network_support.as_str()),
        true,
        &entity,
        move |value, cx| {
            // `nextMode.trim().toLowerCase()`, empty → `undefined`; the store
            // then records the explicit "None" (`networkSupportDisabled`).
            let next = value.trim().to_lowercase();
            let next = if next.is_empty() { None } else { Some(next) };
            write(
                &network_entity,
                &network_id,
                |p| {
                    p.network_support = next.clone();
                    p.network_support_disabled = next.is_none();
                },
                cx,
            );
        },
    ));
    cells.push(labeled_toggle(
        "printer-editor-webcam-support",
        "Webcam Support",
        has_camera,
        false,
        camera_modified,
        move |_, _, cx| {
            write(
                &camera_entity,
                &camera_id,
                |p| p.has_camera = Some(p.has_camera == Some(false)),
                cx,
            );
        },
    ));
    if has_camera {
        let rotation = profile.display.webcam_rotation_deg.unwrap_or(0);
        let rotation_text = rotation.to_string();
        cells.push(labeled_select(
            "printer-editor-webcam-rotation",
            "Webcam Rotation",
            &rotation_text,
            WEBCAM_ROTATION_OPTIONS,
            // The source hands `defaultValue` to a plain `SelectDropdown`, which
            // has no such prop — this dropdown shows no default chip.
            None,
            true,
            &entity,
            move |value, cx| {
                let parsed = value
                    .parse::<u32>()
                    .ok()
                    .filter(|value| matches!(*value, 0 | 90 | 180 | 270));
                write(
                    &rotation_entity,
                    &rotation_id,
                    |p| p.display.webcam_rotation_deg = Some(parsed.unwrap_or(0)),
                    cx,
                );
            },
        ));
    }
    if !version_options.is_empty() {
        let first = version_options[0].0;
        cells.push(labeled_select(
            "printer-editor-format-version",
            "Format Version",
            resolved_version.as_deref().unwrap_or(first),
            &version_options,
            defaults.map(|defaults| defaults.format_version.as_deref().unwrap_or(first)),
            true,
            &entity,
            move |value, cx| {
                let value = value.to_owned();
                write(
                    &version_entity,
                    &version_id,
                    |p| {
                        let format = p.display.output_format.clone();
                        p.display.format_version =
                            resolve_format_version(&format, Some(value.as_str()));
                    },
                    cx,
                );
            },
        ));
    }
    if !mode_options.is_empty() {
        let first = mode_options[0].0;
        cells.push(labeled_select(
            "printer-editor-settings-mode",
            "Settings Mode",
            resolved_mode.as_deref().unwrap_or(first),
            &mode_options,
            defaults.map(|defaults| defaults.settings_mode.as_deref().unwrap_or(first)),
            true,
            &entity,
            move |value, cx| {
                let value = value.to_owned();
                write(
                    &mode_entity,
                    &mode_id,
                    |p| {
                        let format = p.display.output_format.clone();
                        p.display.settings_mode =
                            resolve_settings_mode(&format, Some(value.as_str()));
                    },
                    cx,
                );
            },
        ));
    }
    cells.push(labeled_toggle(
        "printer-editor-mirror-x",
        "Mirror X",
        profile.display.mirror_x == Some(true),
        profile.is_official,
        false,
        move |_, _, cx| {
            write(
                &mirror_x_entity,
                &mirror_x_id,
                |p| p.display.mirror_x = Some(p.display.mirror_x != Some(true)),
                cx,
            );
        },
    ));
    cells.push(labeled_toggle(
        "printer-editor-mirror-y",
        "Mirror Y",
        profile.display.mirror_y == Some(true),
        profile.is_official,
        false,
        move |_, _, cx| {
            write(
                &mirror_y_entity,
                &mirror_y_id,
                |p| p.display.mirror_y = Some(p.display.mirror_y != Some(true)),
                cx,
            );
        },
    ));
    // Both are parked behind the source's `supportsMultiTankFormat` guard, which
    // no built-in format satisfies (deviation 3).
    if multi_tank_format {
        let material_mode = profile
            .material_mode
            .clone()
            .unwrap_or_else(|| "single".to_owned());
        let default_mode = defaults.map(|defaults| defaults.material_mode.as_str());
        cells.push(labeled_select(
            "printer-editor-material-mode",
            "Material Mode",
            &material_mode,
            MATERIAL_MODE_OPTIONS,
            default_mode,
            true,
            &entity,
            move |value, cx| {
                let next = if value == "multiTank" { "multiTank" } else { "single" };
                let next = next.to_owned();
                write(
                    &material_entity,
                    &material_id,
                    |p| {
                        p.material_mode = Some(next.clone());
                        if next == "multiTank" && p.tank_count.is_none() {
                            p.tank_count = Some(2);
                        }
                    },
                    cx,
                );
            },
        ));
        if profile.multi_tank() {
            let tank_count = profile.tank_count_or_default() as f32;
            cells.push(labeled_number(
                "printer-editor-tank-count",
                "Amount of Tanks",
                tank_count,
                2.0,
                16.0,
                1.0,
                false,
                move |value, cx| {
                    write(
                        &tank_entity,
                        &tank_id,
                        |p| p.tank_count = Some(value.round() as u32),
                        cx,
                    );
                },
            ));
        }
    }

    section_card()
        .child(section_title("Output"))
        .child(grid_rows(cells, 3, 10.0))
}

// ── Value helpers ───────────────────────────────────────────────────────────

/// One `safetyMarginMm` edge, the key the source spreads into its update
/// (`:5717-5757`).
#[derive(Clone, Copy)]
enum MarginEdge {
    Front,
    Back,
    Left,
    Right,
}

impl MarginEdge {
    fn slot(self, margin: &mut SafetyMarginMm) -> &mut f32 {
        match self {
            MarginEdge::Front => &mut margin.front,
            MarginEdge::Back => &mut margin.back,
            MarginEdge::Left => &mut margin.left,
            MarginEdge::Right => &mut margin.right,
        }
    }
}

/// `bitDepth?.bits ?? 8`, rounded (`bitDepthBits`).
fn resolution_bits(profile: &PrinterProfile) -> u32 {
    profile.bit_depth_bits().unwrap_or(8)
}

/// `LabeledNumberInput`'s default step: `|current| < 1 ? 0.01 : 1`.
fn step_for(value: f32) -> f32 {
    if value.abs() < 1.0 {
        0.01
    } else {
        1.0
    }
}

/// `Math.max(0.001, Number(value) || 0.001)`.
fn safe_pixel_size(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value.max(0.001)
    } else {
        0.001
    }
}

/// `applyAutoBuildDimensions` (`:830-844`): in auto mode the stored width and
/// depth follow resolution × pixel size.
fn sync_auto_build_volume(profile: &mut PrinterProfile) {
    if !profile.auto_build_dimensions() {
        return;
    }
    let pixel_size = profile.pixel_size.unwrap_or(PixelSize { x: 1.0, y: 1.0 });
    profile.build_volume_mm.width = Some(model::compute_build_dimension_mm(
        profile.display.resolution_x,
        pixel_size.x,
    ));
    profile.build_volume_mm.depth = Some(model::compute_build_dimension_mm(
        profile.display.resolution_y,
        pixel_size.y,
    ));
}
