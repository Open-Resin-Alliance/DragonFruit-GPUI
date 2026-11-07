//! Support-mode panel bodies.
//!
//! Sources: `src/components/controls/IslandsPanel.tsx`,
//! `src/components/controls/IslandListCard.tsx`,
//! `src/components/controls/autoSupport/AutoSupportSettingsBody.tsx` (through
//! `AutoSupportPresets.tsx` and the `autoSupportPanelTabs.ts` catalogue),
//! `src/features/hollowing/HollowingPanel.tsx` and
//! `src/features/hole-punching/HolePunchPanel.tsx`.
//!
//! The Auto Supports *floating* panel is `panel_content.rs::supports_body`; this
//! module is its settings-dialog body, the three tool panels beside it and the
//! island surfaces. Only bodies are ported: the card, its header, its collapse
//! chevron and the panel placement belong to `panels.rs`.
//!
//! # Entry points
//!
//! * [`islands_body`] plus [`island_scan_settings_dialog`] — the dialog is a
//!   window-level body, mounted while `IslandsModel::settings_open` is set.
//! * [`island_list_body`].
//! * [`support_settings_body`].
//! * [`hollowing_body`] and [`hole_punch_body`].
//!
//! Each takes a cheap, cloneable state handle (`IslandsPanelState`,
//! `IslandListState`, `SupportSettingsState`, `HollowingState`,
//! `HolePunchState` — all `Rc<RefCell<..>>`, like `panels::PanelLayout`) plus the
//! app's `WeakEntity<DragonFruit>`, and returns the body only.
//!
//! # PORT DEVIATIONS
//!
//! ## Shared
//!
//! 1. `panels.rs::card_header` has no right slot, so each panel's header control
//!    (the Islands scan-settings gear, the Hollowing mode badge, the island
//!    list's Hierarchy button) is rendered at the top of the body, right-aligned,
//!    where the header slot would have put it.
//! 2. GPUI has no CSS transitions, `backdrop-blur`, `letter-spacing`,
//!    `tabular-nums` or `filter: grayscale()`: hover fills apply instantly, the
//!    backdrop blur is omitted, tracked labels are not tracked, numerals are not
//!    tabular, and the disabled hole-punch card keeps its opacity but not its
//!    grey cast.
//! 3. `space-y-*` / `space-x-*` are a flex `gap` on the same container.
//! 4. Tailwind radii are v4's defaults — bare `rounded` 4px, `rounded-md` 6px,
//!    `rounded-lg` 8px, `rounded-xl` 12px. `globals.css` declares no `@theme`
//!    radius scale; `--radius-md: 10px` is only read by `.ui-input`,
//!    `.ui-button` and `.ui-panel`.
//! 5. The `.ui-panel` cascade: `.ui-panel .ui-button` / `.ui-input` paint at
//!    `--font-size-md` (14px), so the sources' `text-[10px]` / `text-[11px]` on
//!    those controls never apply inside a card. Raw `<button>`s (the island
//!    layer toggles, the nav arrows) keep their class sizes.
//! 6. No text entry in this slice: `NumberInput`, `ScrollableNumberField` and
//!    the island search box render their value or placeholder without a caret.
//!    The steppers (and, on the dialog fields, the two 12px chevrons) are the
//!    way to change a value.
//! 7. No range inputs: the scan dialog's two `input[type=range]` sliders render
//!    as stepper fields at the same min/max/step.
//! 8. No portals: `island_scan_settings_dialog` is a window-level body the shell
//!    mounts; `ui::dialog_modal`'s backdrop is a plain div, not a fixed layer.
//! 9. No timers: `applySettings`'s 50 ms "Recalculating…" pass is not
//!    reproduced — Apply copies the draft immediately.
//! 10. `ui::select_dropdown`'s trigger is the atom's 36px `.ui-input`; the
//!     sources' `!h-8` (32px) overrides are not applied.
//! 11. `.ui-checkbox`'s 3px accent ring and its rotated-border tick are replaced
//!     by the `check` glyph; the ring is not drawn.
//!
//! ## Islands — `IslandsPanel.tsx`
//!
//! 12. No island scanner: `scanning`, the three totals and the island labels are
//!     the shell's to fill in. The Scan button renders the source's enabled and
//!     disabled recipes but starts nothing, and the `Recalculating…` overlay only
//!     shows while the shell sets `applying_settings`.
//! 13. The body's `maxHeight: calc(100vh - …)` scroll cap is not applied: the
//!     card grows to its content in this shell, so a scroll container would
//!     collapse.
//! 14. `fmt.quantity(draftPxMm, 2, 'mmPerPx')` has no locale formatter here; the
//!     value prints as `0.05 mm/px`.
//!
//! ## Island list — `IslandListCard.tsx`
//!
//! 15. `IslandHierarchyModal` is not ported; the Hierarchy button sets
//!     `show_hierarchy`, which the shell may read.
//! 16. The search box has no text entry, so `search` stays whatever the shell
//!     seeds (the filter itself is ported).
//! 17. The card's `clientWidth <= 286` compact-header measurement is not
//!     reproduced; the full Hierarchy button always renders.
//!
//! ## Support settings — `AutoSupportSettingsBody.tsx`
//!
//! 18. Only the sections this panel is named for are ported: the preset strip,
//!     Density & Sizing, Stability and the footer. Detection, Distribution,
//!     Post-processing, Diagnostics and Advanced (calibration) are not rendered.
//! 19. No preset store: `Rename` / `Export` / `Import` are inert, and the
//!     selector's `New preset…` footer entry does not exist. The three built-ins
//!     (`Light` / `Medium` / `Heavy`) are selectable and copy their
//!     `areaPerSupportMm2` into the draft, and `Duplicate` turns the selection
//!     into a custom profile in the panel's own state — the source's way out of
//!     a locked built-in — but the copy has no name and is not persisted.
//! 19a. The source locks every policy field while a built-in is active
//!     (`presetLocked`). That lock is reproduced; `Duplicate` is the escape, as
//!     at the source.
//! 20. No `AutoSupportSizingTierField`: the sizing band reads Support Studio's
//!     preset collection, which the app does not have.
//! 21. `overhangSelfSupportAngleDeg` is wired to `DragonFruit::support_angle`,
//!     the value `stl::overhang_points` classifies with. The dialog's other
//!     knobs live in the draft: nothing in the app consumes them yet, and
//!     `DragonFruit::support_density` (a 0.05–1.0 count multiplier) is a
//!     different quantity from `areaPerSupportMm2`, so it is not bound here.
//! 22. Save writes the draft over the baseline and clears the modified marker;
//!     there is no settings store to persist to.
//!
//! ## Hollowing — `HollowingPanel.tsx`
//!
//! 23. No hollowing engine: Apply, Remove Hollowing, Solid Areas and the
//!     voxel-edit Done/Clear only flip the panel's own flags.
//! 24. `HollowMode::ShellOpenFace` is unreachable from the panel (the source's
//!     Mode card offers Cavity and Infill only), but its Open Face card is
//!     ported because the shell can put the state there.
//!
//! ## Hole punching — `HolePunchPanel.tsx`
//!
//! 25. No punching engine: Apply and Reset only flip the panel's own flags.

// The shell has not mounted these bodies yet, so every handle and body is
// unreachable from `main` today. The lint is silenced here rather than on each
// item: once `panels.rs` builds the support stack it goes away on its own.
#![allow(dead_code)]

use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

use gpui::{
    App, ClickEvent, Div, ElementId, FontWeight, IntoElement, SharedString, Stateful, WeakEntity,
    Window, div, prelude::*, px, rgb, rgba,
};

use gpui_core::icons::icon;
use gpui_ui::panel_fields::{FieldAlign, Unit, format_number, value_field_at, with_alpha};
use gpui_core::theme::*;
use gpui_core::support_state::{MAX_INFILL_DIAMETER_MM, MIN_INFILL_DIAMETER_MM, NO_ACTIVE_PRESET_LABEL, field_help_text, *};
use gpui_ui::ui::{
    ButtonSize, ButtonVariant, IconTone, SelectOption, button, dialog_modal, select_dropdown,
    spinner_colored,
};
use crate::DragonFruit;

/// `#ffffff` — the base of every `color-mix(..., white N%)` in the sources.
const WHITE: u32 = 0xffffff;
/// `#000000` — the base of every `color-mix(..., black N%)` in the sources.
const BLACK: u32 = 0x000000;

/// Tailwind's bare `rounded`: 4px.
const RADIUS_BARE: f32 = 4.0;
/// Tailwind's `rounded-md`: 6px.
const RADIUS_MD: f32 = 6.0;
/// Tailwind's `rounded-xl`: 12px.
const RADIUS_XL: f32 = 12.0;

/// `ISLAND_LAYER_COLORS` in `islandPuckMarkers.ts`.
const ISLAND_VOXEL: u32 = 0x87cefa;
const ISLAND_MINIMA: u32 = 0x00ff00;
const ISLAND_INTERSECTION: u32 = 0xff0000;
const ISLAND_OVERHANG: u32 = 0xffa500;

/// `#22c55e` / `#4ade80` — the `P+n` child badge's `bg-green-500/20` and
/// `text-green-400`.
const GREEN_500: u32 = 0x22c55e;
const GREEN_400: u32 = 0x4ade80;
/// `#f97316` / `#fb923c` — the `C#n` parent badge's orange pair.
const ORANGE_500: u32 = 0xf97316;
const ORANGE_400: u32 = 0xfb923c;

fn accent_secondary_action_border() -> u32 {
    // `color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 42%)`.
    mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.58)
}

fn accent_secondary_action_bg() -> u32 {
    // `color-mix(in srgb, var(--accent-secondary), var(--surface-1) 92%)`.
    mix(ACCENT_SECONDARY, SURFACE_1, 0.08)
}

/// `activeModeStyle`: the selected pill of a two-way mode choice, as
/// `(background, border, text)`.
///
/// `border-color: color-mix(in srgb, var(--accent), var(--border-subtle) 30%)`
/// and `background: color-mix(in srgb, var(--accent), var(--surface-1) 85%)`.
fn active_pill_style() -> (u32, u32, u32) {
    (
        mix(ACCENT, SURFACE_1, 0.15),
        mix(ACCENT, BORDER_SUBTLE, 0.70),
        TEXT_STRONG,
    )
}

/// `accentCardStyle`: `color-mix(accent, border-subtle 76%)` on
/// `color-mix(accent, surface-1 95%)`.
fn accent_card() -> Div {
    div()
        .rounded(px(RADIUS_MD))
        .border_1()
        .border_color(rgb(mix(ACCENT, BORDER_SUBTLE, 0.24)))
        .bg(rgb(mix(ACCENT, SURFACE_1, 0.05)))
        .p(px(8.0))
}

/// `SECTION_CARD` / `panelCardStyle`: `rounded-md border p-2` on
/// `--border-subtle` / `--surface-1`.
fn section_card() -> Div {
    div()
        .rounded(px(RADIUS_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(8.0))
}

/// `.ui-meta`: 13px `--indicator`.
fn ui_meta(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(FONT_SM))
        .text_color(rgb(INDICATOR))
        .child(text.into())
}

fn notify(entity: &WeakEntity<DragonFruit>, cx: &mut App) {
    entity.update(cx, |_, cx| cx.notify()).ok();
}

// ---------------------------------------------------------------------------
// Shared pieces
// ---------------------------------------------------------------------------

/// The source's `Tooltip`, in the one shape this shell can draw: an absolutely
/// positioned popover hanging under its trigger, revealed by the trigger's
/// hover.
///
/// `ui::tooltip` is not reused because its popover is `left-0 right-0` on the
/// trigger; these triggers are icon tiles and narrow buttons, and the label
/// would wrap into their width.
fn hint(
    id: impl Into<SharedString>,
    trigger: impl IntoElement,
    text: impl Into<SharedString>,
) -> Stateful<Div> {
    let group: SharedString = id.into();
    div()
        .id(group.clone())
        .relative()
        .flex()
        .group(group.clone())
        .child(trigger)
        .child(
            div()
                .absolute()
                .right(px(0.0))
                .top(px(0.0))
                .mt(px(28.0))
                .w(px(200.0))
                .opacity(0.0)
                .group_hover(group, |style| style.opacity(1.0))
                .rounded(px(RADIUS_BARE))
                .px(px(8.0))
                .py(px(6.0))
                .text_size(px(11.0))
                .line_height(px(11.0 * LINE_HEIGHT_TIGHT))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(TEXT_STRONG))
                .bg(rgba(0x181818fa))
                .border_1()
                .border_color(rgb(ACCENT))
                .whitespace_normal()
                .shadow_lg()
                .child(text.into()),
        )
}

/// [`hint`] for a trigger that spans its container (`w-full` buttons).
fn hint_full(
    id: impl Into<SharedString>,
    trigger: impl IntoElement,
    text: impl Into<SharedString>,
) -> Stateful<Div> {
    hint(id, trigger, text).w_full()
}

/// `FieldHelpTooltip`: the 14px `?` tile beside a field label, with the help
/// text in a 260px popover on hover.
fn field_help(id: impl Into<SharedString>, help: impl Into<SharedString>) -> Stateful<Div> {
    let group: SharedString = id.into();
    div()
        .id(group.clone())
        .relative()
        .flex()
        .flex_shrink_0()
        .group(group.clone())
        .child(
            div()
                .w(px(14.0))
                .h(px(14.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(RADIUS_BARE))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_0))
                .child(icon("circle-question-mark", 10.0, TEXT_MUTED)),
        )
        .child(
            div()
                .absolute()
                .left(px(0.0))
                .top(px(14.0))
                .mt(px(14.0))
                .w(px(260.0))
                .opacity(0.0)
                .group_hover(group, |style| style.opacity(1.0))
                .rounded(px(RADIUS_BARE))
                .px(px(8.0))
                .py(px(6.0))
                .text_size(px(FONT_XS))
                .line_height(px(FONT_XS * 1.25))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(TEXT_STRONG))
                .bg(rgba(0x181818fa))
                .border_1()
                .border_color(rgb(ACCENT))
                .whitespace_normal()
                .shadow_lg()
                .child(help.into()),
        )
}

/// `IconButton` at its default `solid` variant with an explicit padding: the
/// `.ui-button-secondary` tile (surface-1 fill, subtle border, `rounded-sm`).
fn panel_icon_button(
    id: &'static str,
    icon_name: &'static str,
    icon_size: f32,
    pad: f32,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(pad))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(SURFACE_2)))
        .child(icon(icon_name, icon_size, TEXT_MUTED))
        .on_click(on_click)
}

/// A `.ui-button` with its fill, border and text chosen by the caller — the
/// sources' inline `style` triples, which beat the variant's hover fill.
///
/// `height` is the class's `!h-8` / `!h-9` / `size="md"`; the label is
/// `--font-size-md` per the `.ui-panel` cascade.
fn flat_button(
    id: impl Into<ElementId>,
    content: impl IntoElement,
    height: f32,
    background: u32,
    border: u32,
    text: u32,
    hover: Option<u32>,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let (background, border, text) = if enabled {
        (background, border, text)
    } else {
        (
            mix(SURFACE_1, BLACK, 0.92),
            mix(BORDER_SUBTLE, BLACK, 0.90),
            mix(TEXT_MUTED, SURFACE_2, 0.18),
        )
    };
    let mut el = div()
        .id(id)
        .h(px(height))
        .px(px(11.0))
        .pt(px(5.0))
        .pb(px(7.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .text_size(px(FONT_MD))
        .font_weight(FontWeight::SEMIBOLD)
        .line_height(px(FONT_MD))
        .text_color(rgb(text))
        .child(content);
    if enabled {
        el = el.cursor_pointer();
        if let Some(hover) = hover {
            el = el.hover(move |style| style.bg(rgb(hover)));
        }
        el = el.on_click(on_click);
    } else {
        el = el.cursor_not_allowed();
    }
    el
}

/// One `ScrollableNumberField` stepper: `IconButton` at `!h-8 !w-8 !p-0
/// !bg-[var(--surface-0)]`, `hover:!bg-[color-mix(surface-0, white 4%)]`,
/// `disabled` wearing `.ui-button:disabled`.
fn square_stepper(
    id: impl Into<ElementId>,
    name: &'static str,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let el = div()
        .id(id)
        .w(px(32.0))
        .h(px(32.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(RADIUS_SM))
        .border_1()
        .child(icon(name, 14.0, TEXT_MUTED));
    if enabled {
        el.border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_0))
            .cursor_pointer()
            .hover(|style| style.bg(rgb(mix(SURFACE_0, WHITE, 0.96))))
            .on_click(on_click)
    } else {
        el.border_color(rgb(mix(BORDER_SUBTLE, BLACK, 0.90)))
            .bg(rgb(mix(SURFACE_1, BLACK, 0.92)))
            .cursor_not_allowed()
    }
}

/// `ScrollableNumberField`: `flex items-center gap-1` around two 32px steppers
/// and a centred, semibold value box with the unit at `right-2`.
fn scroll_field(
    id: impl Into<SharedString>,
    value: f32,
    unit: Option<&'static str>,
    step: f32,
    min: f32,
    max: f32,
    disabled: bool,
    on_change: impl Fn(f32, &mut Window, &mut App) + Clone + 'static,
) -> Div {
    let id: SharedString = id.into();
    let decrease = on_change.clone();
    let increase = on_change;
    let can_decrease = !disabled && value > min;
    let can_increase = !disabled && value < max;

    div()
        .w_full()
        .min_w(px(0.0))
        .flex()
        .items_center()
        .gap(px(4.0))
        .child(square_stepper(
            format!("{id}-dec"),
            "minus",
            can_decrease,
            move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                decrease((value - step).max(min), window, cx)
            },
        ))
        .child(
            div().flex_1().min_w(px(0.0)).font_weight(FontWeight::SEMIBOLD).child(
                value_field_at(
                    format_number(value),
                    unit.map(|unit| Unit::new(unit, 10.0)),
                    FieldAlign::Center,
                    32.0,
                    disabled,
                    8.0,
                ),
            ),
        )
        .child(square_stepper(
            format!("{id}-inc"),
            "plus",
            can_increase,
            move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                increase((value + step).min(max), window, cx)
            },
        ))
}

/// One `LabeledNumberInput` chevron: `h-3 w-3`, 10px glyph, `rounded`, hover
/// `bg-white/10`, `disabled:pointer-events-none`.
fn chevron_step(
    id: impl Into<ElementId>,
    name: &'static str,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let el = div()
        .id(id)
        .w(px(12.0))
        .h(px(12.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(RADIUS_BARE))
        .child(icon(name, 10.0, TEXT_MUTED));
    if enabled {
        el.cursor_pointer()
            .hover(|style| style.bg(rgba(with_alpha(WHITE, 0x1a))))
            .on_click(on_click)
    } else {
        el.opacity(0.5)
    }
}

/// `LabeledNumberInput`: the `ui-label` name with its help tile over a 36px
/// `.ui-input` box, the unit inside at `right-6` and the two chevrons in the
/// 16px gutter at `right-1`.
fn labeled_number_field(
    id: impl Into<SharedString>,
    label: &'static str,
    help: &'static str,
    value: f32,
    unit: Option<&'static str>,
    step: f32,
    min: f32,
    max: f32,
    disabled: bool,
    on_change: impl Fn(f32, &mut Window, &mut App) + Clone + 'static,
) -> Div {
    let id: SharedString = id.into();
    let help_id = format!("{id}-help");
    let up = on_change.clone();
    let down = on_change;
    let can_up = !disabled && value < max;
    let can_down = !disabled && value > min;

    let mut field = div()
        .relative()
        .w_full()
        .h(px(36.0))
        .pl(px(10.0))
        .pr(px(56.0))
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .text_size(px(FONT_MD))
        .line_height(px(FONT_MD * 1.25))
        .text_color(rgb(TEXT_STRONG))
        .flex()
        .items_center()
        .child(format_number(value));
    if disabled {
        field = field.opacity(0.55).cursor_not_allowed();
    }

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .min_w(px(0.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .text_size(px(FONT_SM))
                .text_color(rgb(INDICATOR))
                .font_weight(FontWeight::MEDIUM)
                .child(label)
                .child(field_help(help_id, help)),
        )
        .child(
            div()
                .relative()
                .w_full()
                .min_w(px(0.0))
                .child(field)
                .when_some(unit, |this, unit| {
                    this.child(
                        div()
                            .absolute()
                            .right(px(24.0))
                            .top(px(0.0))
                            .bottom(px(0.0))
                            .flex()
                            .items_center()
                            .text_size(px(11.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(TEXT_MUTED))
                            .child(unit),
                    )
                })
                .child(
                    div()
                        .absolute()
                        .right(px(4.0))
                        .top(px(0.0))
                        .bottom(px(0.0))
                        .w(px(16.0))
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap(px(2.0))
                        .child(chevron_step(
                            format!("{id}-up"),
                            "chevron-up",
                            can_up,
                            move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                                up((value + step).min(max), window, cx)
                            },
                        ))
                        .child(chevron_step(
                            format!("{id}-down"),
                            "chevron-down",
                            can_down,
                            move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                                down((value - step).max(min), window, cx)
                            },
                        )),
                ),
        )
}

/// `LabeledToggleInput`: the `ui-label` name with its help tile over a 36px
/// `.ui-input` switch row — `Enabled` / `Disabled` on the left, the track on the
/// right.
fn labeled_toggle_field(
    id: impl Into<SharedString>,
    label: &'static str,
    help: &'static str,
    checked: bool,
    disabled: bool,
    on_change: impl Fn(bool, &mut Window, &mut App) + 'static,
) -> Div {
    let id: SharedString = id.into();
    let help_id = format!("{id}-help");

    let (border, background, text) = if disabled {
        (
            BORDER_SUBTLE,
            mix(SURFACE_2, BLACK, 0.92),
            TEXT_MUTED,
        )
    } else if checked {
        (
            mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.64),
            mix(ACCENT_SECONDARY, SURFACE_1, 0.10),
            TEXT_STRONG,
        )
    } else {
        (BORDER_SUBTLE, SURFACE_1, TEXT_MUTED)
    };

    let mut switch = div()
        .id(id)
        .w_full()
        .h(px(36.0))
        .px(px(10.0))
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .text_size(px(FONT_MD))
        .line_height(px(FONT_MD))
        .text_color(rgb(text))
        .flex()
        .items_center()
        .justify_between()
        .child(if checked { "Enabled" } else { "Disabled" })
        .child(
            div()
                .flex()
                .items_center()
                .flex_shrink_0()
                .rounded_full()
                .p(px(2.0))
                .w(px(36.0))
                .h(px(20.0))
                .bg(rgb(if disabled {
                    mix(SURFACE_2, BLACK, 0.92)
                } else if checked {
                    ACCENT_SECONDARY
                } else {
                    SURFACE_2
                }))
                .when(checked, |this| this.justify_end())
                .when(!checked, |this| this.justify_start())
                .child(div().rounded_full().bg(rgb(WHITE)).w(px(16.0)).h(px(16.0))),
        );
    if disabled {
        switch = switch.opacity(0.55).cursor_not_allowed();
    } else {
        switch = switch
            .cursor_pointer()
            .on_click(move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                on_change(!checked, window, cx)
            });
    }

    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .min_w(px(0.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .text_size(px(FONT_SM))
                .text_color(rgb(INDICATOR))
                .font_weight(FontWeight::MEDIUM)
                .child(label)
                .child(field_help(help_id, help)),
        )
        .child(switch)
}

/// `.ui-checkbox` (15px, or `!w-4 !h-4` = 16px) with its label:
/// `flex items-center gap-2 cursor-pointer select-none`.
fn checkbox(
    id: impl Into<ElementId>,
    label: &'static str,
    checked: bool,
    disabled: bool,
    size: f32,
    // The island list's `ui-meta` label (13px, `--indicator`) rather than the
    // dialog's `text-[11px] font-medium` one.
    meta_label: bool,
    on_change: impl Fn(bool, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let (border, background) = if checked {
        (mix(ACCENT, WHITE, 0.88), mix(ACCENT, SURFACE_0, 0.82))
    } else {
        (BORDER_SUBTLE, SURFACE_1)
    };
    let mut el = div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(if meta_label { 6.0 } else { 8.0 }))
        .when(meta_label, |this| this.py(px(2.0)).px(px(2.0)))
        .child(
            div()
                .w(px(size))
                .h(px(size))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(RADIUS_BARE))
                .border_1()
                .border_color(rgb(border))
                .bg(rgb(background))
                .when(checked, |this| {
                    this.child(icon("check", 10.0, ACCENT_CONTRAST))
                }),
        )
        .child(
            div()
                .text_size(px(if meta_label { FONT_SM } else { 11.0 }))
                .font_weight(if meta_label {
                    FontWeight::NORMAL
                } else {
                    FontWeight::MEDIUM
                })
                .text_color(rgb(if meta_label { INDICATOR } else { TEXT_STRONG }))
                .child(label),
        );
    if disabled {
        el = el.opacity(0.5).cursor_not_allowed();
    } else {
        el = el
            .cursor_pointer()
            .on_click(move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                on_change(!checked, window, cx)
            });
    }
    el
}

/// A two-way mode pill: `Button variant="secondary" size="auto"` at `!h-8
/// whitespace-nowrap px-1.5`, wearing `activeModeStyle` when selected.
fn mode_pill(
    id: impl Into<ElementId>,
    label: &'static str,
    active: bool,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let (background, border, text) = if active {
        active_pill_style()
    } else {
        (SURFACE_1, BORDER_SUBTLE, TEXT_STRONG)
    };
    flat_button(
        id,
        label,
        32.0,
        background,
        border,
        text,
        if active { None } else { Some(SURFACE_2) },
        enabled,
        on_click,
    )
}

/// `clampFloat` from the panels: finite-or-min, rounded to `decimals`, clamped.
fn clamp_float(value: f32, min: f32, max: f32, decimals: i32) -> f32 {
    let safe = if value.is_finite() { value } else { min };
    let factor = 10f32.powi(decimals);
    let rounded = (safe * factor).round() / factor;
    rounded.clamp(min, max)
}

/// `normalizeFieldValue` from `AutoSupportSettingsBody.tsx`: clamped, then
/// rounded to the decimals the step implies.
fn normalize_value(value: f32, min: f32, max: f32, step: f32) -> f32 {
    let decimals = if step < 0.1 {
        2
    } else if step < 1.0 {
        1
    } else {
        0
    };
    clamp_float(value, min, max, decimals)
}

// ---------------------------------------------------------------------------
// Islands — `IslandsPanel.tsx`
// ---------------------------------------------------------------------------
pub fn islands_body(
    state: &IslandsPanelState,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let model = state.model();
    let has_geometry = model.has_geometry;
    let scanning = model.scanning;
    let applying_settings = model.applying_settings;
    let has_data = model.has_data();
    let voxel_total = model.voxel_total;
    let minima_total = model.minima_total;
    let coincident_total = model.coincident_total;
    let island_count = model.islands.len();
    let selected_index = model.selected_index;
    let current_label = selected_index
        .and_then(|index| model.islands.get(index))
        .cloned();
    let advanced_mode = model.advanced_mode;
    let show_voxel_only = model.show_voxel_only;
    let show_minima_only = model.show_minima_only;
    let show_intersection = model.show_intersection;
    let show_overhangs = model.show_overhangs;
    drop(model);

    // Header right slot: `IconButton className="!p-0.5"` with a 14px gear.
    let gear_state = state.clone();
    let gear_entity = entity.clone();
    let gear = hint(
        "islands-settings-tip",
        panel_icon_button("islands-settings", "settings", 14.0, 2.0, move |_, _, cx| {
            gear_state.model_mut().settings_open = true;
            notify(&gear_entity, cx);
        }),
        "Scan settings",
    );

    // `Button size="auto"` at `w-full !h-8 text-[11px] disabled:opacity-50` with
    // the accent inline style.
    let scan_entity = entity.clone();
    let scan_button = flat_button(
        "islands-scan",
        if scanning { "Scanning…" } else { "Scan Islands" },
        32.0,
        mix(ACCENT, SURFACE_0, 0.14),
        ACCENT,
        ACCENT,
        None,
        has_geometry && !scanning,
        move |_, _, cx| {
            // PORT GAP: no island scanner — the run belongs to the shell.
            notify(&scan_entity, cx);
        },
    )
    .w_full();

    // Breakdown by type: `grid grid-cols-3 gap-2 text-center`.
    let breakdown = section_card().child(
        div()
            .flex()
            .gap(px(8.0))
            .child(island_stat("Voxels", voxel_total, ISLAND_VOXEL))
            .child(island_stat("Minima", minima_total, ISLAND_MINIMA))
            .child(island_stat("Coincident", coincident_total, ISLAND_INTERSECTION)),
    );

    // Navigation: `flex items-center gap-2`, two `flex-1 h-8` arrows around the
    // counter.
    let prev_state = state.clone();
    let prev_entity = entity.clone();
    let next_state = state.clone();
    let next_entity = entity.clone();
    let can_prev = island_count > 0 && selected_index.is_some_and(|index| index > 0);
    // `disabled={orderedIslands.length === 0 || selectedIndex >= orderedIslands.length - 1}`:
    // with nothing selected the source's index is -1, so Next is live.
    let can_next = island_count > 0
        && selected_index.is_none_or(|index| index + 1 < island_count);
    let nav_counter = div()
        .flex_1()
        .min_w(px(0.0))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(4.0))
        .child(
            div()
                .text_size(px(13.0))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(TEXT_STRONG))
                .child(match &current_label {
                    Some(label) => label.to_string(),
                    None => island_count.to_string(),
                }),
        )
        .when(selected_index.is_some(), |this| {
            this.child(
                div()
                    .text_size(px(10.0))
                    .text_color(rgb(TEXT_MUTED))
                    .child(format!("/ {island_count}")),
            )
        });
    let navigation = section_card().child(
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(nav_arrow(
                "islands-prev",
                "chevron-left",
                can_prev,
                move |_, _, cx| {
                    let mut model = prev_state.model_mut();
                    let count = model.islands.len();
                    model.selected_index = match model.selected_index {
                        Some(index) if index > 0 => Some(index - 1),
                        Some(_) => Some(0),
                        None if count > 0 => Some(0),
                        None => None,
                    };
                    drop(model);
                    notify(&prev_entity, cx);
                },
            ))
            .child(nav_counter)
            .child(nav_arrow(
                "islands-next",
                "chevron-right",
                can_next,
                move |_, _, cx| {
                    let mut model = next_state.model_mut();
                    let count = model.islands.len();
                    model.selected_index = match model.selected_index {
                        Some(index) if index + 1 < count => Some(index + 1),
                        None if count > 0 => Some(0),
                        other => other,
                    };
                    drop(model);
                    notify(&next_entity, cx);
                },
            )),
    );

    // Display toggles, advanced mode only: `grid grid-cols-2 gap-1.5`.
    let toggle_row = |a: Stateful<Div>, b: Stateful<Div>| {
        div().flex().gap(px(6.0)).child(div().flex_1().min_w(px(0.0)).child(a)).child(
            div().flex_1().min_w(px(0.0)).child(b),
        )
    };
    let display_toggles = section_card()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(toggle_row(
            layer_toggle(
                "islands-toggle-voxels",
                "Voxels",
                show_voxel_only,
                ISLAND_VOXEL,
                "Slicing islands and suspended areas detected from layer contours",
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        state.model_mut().show_voxel_only = next;
                        notify(&entity, cx);
                    }
                },
            ),
            layer_toggle(
                "islands-toggle-minima",
                "Minima",
                show_minima_only,
                ISLAND_MINIMA,
                "Individual lowest-vertex triangles on the mesh surface",
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        state.model_mut().show_minima_only = next;
                        notify(&entity, cx);
                    }
                },
            ),
        ))
        .child(toggle_row(
            layer_toggle(
                "islands-toggle-coincident",
                "Coincident",
                show_intersection,
                ISLAND_INTERSECTION,
                "Regions where both voxel and geometric islands overlap",
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        state.model_mut().show_intersection = next;
                        notify(&entity, cx);
                    }
                },
            ),
            layer_toggle(
                "islands-toggle-overhangs",
                "Overhangs",
                show_overhangs,
                ISLAND_OVERHANG,
                "Shallow surfaces detected by the mesh-normal classifier (the surfaces auto-supports grid)",
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        state.model_mut().show_overhangs = next;
                        notify(&entity, cx);
                    }
                },
            ),
        ));

    // Empty state: `rounded-md border px-3 py-3 text-center text-[11px]
    // leading-snug` on `color-mix(surface-1, transparent 8%)`.
    let empty_state = div()
        .rounded(px(RADIUS_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgba(with_alpha(SURFACE_1, 0xeb)))
        .px(px(12.0))
        .py(px(12.0))
        .text_center()
        .text_size(px(11.0))
        .line_height(px(11.0 * 1.375))
        .text_color(rgb(TEXT_MUTED))
        .child("Scan the model to detect unsupported islands");

    // `applyingSettings` overlay: `absolute inset-0 … rounded-md` on
    // `color-mix(surface-0 60%, transparent)`.
    let overlay = div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .right(px(0.0))
        .bottom(px(0.0))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .rounded(px(RADIUS_MD))
        .bg(rgba(with_alpha(SURFACE_0, 0x99)))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .px(px(12.0))
                .py(px(8.0))
                .rounded(px(RADIUS_MD))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_1))
                .shadow_md()
                .child(spinner_colored(16.0, ACCENT))
                .child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("Recalculating…"),
                ),
        );

    // `px-2.5 pt-1 pb-3 space-y-2.5`.
    div()
        .relative()
        .px(px(10.0))
        .pt(px(4.0))
        .pb(px(12.0))
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(div().flex().justify_end().child(gear))
        .child(scan_button)
        .when(has_data && !scanning, |this| {
            this.child(breakdown)
                .child(navigation)
                .when(advanced_mode, |this| this.child(display_toggles))
        })
        .when(!has_data && !scanning && has_geometry, |this| {
            this.child(empty_state)
        })
        .when(applying_settings, |this| this.child(overlay))
}

/// One type count: a 6px dot and a 9px uppercase label over a 13px bold value.
fn island_stat(label: &'static str, count: usize, color: u32) -> Div {
    div()
        .flex_1()
        .min_w(px(0.0))
        .flex()
        .flex_col()
        .items_center()
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(4.0))
                .mb(px(2.0))
                .child(div().w(px(6.0)).h(px(6.0)).flex_shrink_0().rounded_full().bg(rgb(color)))
                .child(
                    div()
                        .text_size(px(9.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_MUTED))
                        .truncate()
                        .child(label.to_uppercase()),
                ),
        )
        .child(
            div()
                .text_size(px(13.0))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(TEXT_STRONG))
                .child(count.to_string()),
        )
}

/// One nav arrow: `flex-1 h-8 rounded border flex items-center justify-center`,
/// `disabled:opacity-40`.
fn nav_arrow(
    id: &'static str,
    icon_name: &'static str,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let mut el = div()
        .id(id)
        .flex_1()
        .h(px(32.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(RADIUS_BARE))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .text_color(rgb(TEXT_STRONG))
        .child(icon(icon_name, 14.0, TEXT_STRONG));
    if enabled {
        el = el.cursor_pointer().hover(|style| style.bg(rgb(SURFACE_2))).on_click(on_click);
    } else {
        el = el.opacity(0.4).cursor_not_allowed();
    }
    el
}

/// `ToggleBtn`: `min-h-[36px] w-full rounded-md border px-2 text-[11px]
/// font-semibold uppercase tracking-wide`, tinted by its layer colour when on.
fn layer_toggle(
    id: &'static str,
    label: &'static str,
    checked: bool,
    color: u32,
    hint_text: &'static str,
    on_change: impl Fn(bool, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let (background, border, text) = if checked {
        (
            mix(color, SURFACE_1, 0.16),
            mix(color, WHITE, 0.90),
            mix(color, TEXT_STRONG, 0.75),
        )
    } else {
        (SURFACE_1, BORDER_SUBTLE, TEXT_MUTED)
    };
    let button = div()
        .id(id)
        .min_h(px(36.0))
        .w_full()
        .rounded(px(RADIUS_MD))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .px(px(8.0))
        .text_size(px(11.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(text))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .cursor_pointer()
        .child(label.to_uppercase())
        .on_click(move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
            on_change(!checked, window, cx)
        });
    hint_full(format!("{id}-tip"), button, hint_text)
}

/// `StructuredDialogModal` for the island scan settings: Display, Scan, Filters
/// and Clustering, with Cancel and Apply.
///
/// PORT GAP: no portal — the shell mounts this at window level.
pub fn island_scan_settings_dialog(
    state: &IslandsPanelState,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let model = state.model();
    let draft = model.draft;
    let advanced_mode = model.advanced_mode;
    let show_already_supported = model.show_already_supported;
    let show_plate_contact = model.show_plate_contact;
    drop(model);

    // Display — `Advanced mode`.
    let display_card = dialog_card().child(section_header("Display")).child(checkbox(
        "islands-advanced-mode",
        "Advanced mode",
        advanced_mode,
        false,
        16.0,
        false,
        {
            let state = state.clone();
            let entity = entity.clone();
            move |next, _, cx| {
                state.model_mut().advanced_mode = next;
                notify(&entity, cx);
            }
        },
    ))
    .child(
        div()
            .mt(px(4.0))
            .text_size(px(10.0))
            .text_color(rgb(TEXT_MUTED))
            .child("Shows the Voxels, Minima, Coincident, and Overhangs layer toggles."),
    );

    // Scan — Resolution and Support buffer. The source's two `input[type=range]`
    // sliders become stepper fields at the same bounds.
    let resolution = draft_field_row(
        "Resolution",
        format!("{} mm/px", format_number(draft.px_mm)),
        scroll_field(
            "islands-draft-px-mm",
            draft.px_mm,
            None,
            0.01,
            0.03,
            0.5,
            false,
            {
                let state = state.clone();
                let entity = entity.clone();
                move |next, _, cx| {
                    state.model_mut().draft.px_mm = next;
                    notify(&entity, cx);
                }
            },
        ),
    );
    let support_buffer = draft_field_row(
        "Support buffer",
        format!("{} mm", format_number(draft.support_buf_mm)),
        scroll_field(
            "islands-draft-support-buf",
            draft.support_buf_mm,
            None,
            0.05,
            0.0,
            1.0,
            false,
            {
                let state = state.clone();
                let entity = entity.clone();
                move |next, _, cx| {
                    state.model_mut().draft.support_buf_mm = next;
                    notify(&entity, cx);
                }
            },
        ),
    );
    let scan_card = dialog_card()
        .child(section_header("Scan"))
        .child(div().flex().flex_col().gap(px(12.0)).child(resolution).child(support_buffer));

    // Filters — the two `filterToggles`.
    let filters_card = dialog_card()
        .child(section_header("Filters"))
        .child(div().flex().flex_col().gap(px(12.0)).child(checkbox(
            "islands-show-supported",
            "Show already-supported islands",
            show_already_supported,
            false,
            16.0,
            false,
            {
                let state = state.clone();
                let entity = entity.clone();
                move |next, _, cx| {
                    state.model_mut().show_already_supported = next;
                    notify(&entity, cx);
                }
            },
        ))
        .child(checkbox(
            "islands-show-plate-contact",
            "Show plate-contact islands",
            show_plate_contact,
            false,
            16.0,
            false,
            {
                let state = state.clone();
                let entity = entity.clone();
                move |next, _, cx| {
                    state.model_mut().show_plate_contact = next;
                    notify(&entity, cx);
                }
            },
        )));

    // Clustering — scale markers, consolidate (indented), contoured regions,
    // remove supported clusters, reduce intersections.
    let consolidate_enabled = draft.scale_markers_with_area;
    let clustering_card = dialog_card()
        .child(section_header("Clustering"))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(checkbox(
                    "islands-scale-markers",
                    "Scale markers with area",
                    draft.scale_markers_with_area,
                    false,
                    16.0,
                    false,
                    {
                        let state = state.clone();
                        let entity = entity.clone();
                        move |next, _, cx| {
                            state.model_mut().draft.scale_markers_with_area = next;
                            notify(&entity, cx);
                        }
                    },
                ))
                .child(
                    div()
                        .pl(px(20.0))
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child(checkbox(
                            "islands-consolidate",
                            "Consolidate",
                            draft.consolidate_voxel,
                            !consolidate_enabled,
                            16.0,
                            false,
                            {
                                let state = state.clone();
                                let entity = entity.clone();
                                move |next, _, cx| {
                                    state.model_mut().draft.consolidate_voxel = next;
                                    notify(&entity, cx);
                                }
                            },
                        ))
                        .when(consolidate_enabled && draft.consolidate_voxel, |this| {
                            this.child(unit_field(
                                "islands-consolidation-distance",
                                draft.consolidation_distance,
                                "mm",
                                0.1,
                                0.1,
                                5.0,
                                false,
                                {
                                    let state = state.clone();
                                    let entity = entity.clone();
                                    move |next, _, cx| {
                                        state.model_mut().draft.consolidation_distance =
                                            clamp_float(next, 0.1, 5.0, 1);
                                        notify(&entity, cx);
                                    }
                                },
                            ))
                        }),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child(checkbox(
                            "islands-contour-regions",
                            "Contoured regions",
                            draft.enable_contour_regions,
                            false,
                            16.0,
                            false,
                            {
                                let state = state.clone();
                                let entity = entity.clone();
                                move |next, _, cx| {
                                    state.model_mut().draft.enable_contour_regions = next;
                                    notify(&entity, cx);
                                }
                            },
                        ))
                        .when(draft.enable_contour_regions, |this| {
                            this.child(unit_field(
                                "islands-max-contour-regions",
                                draft.max_contour_regions,
                                "max",
                                1.0,
                                1.0,
                                50.0,
                                false,
                                {
                                    let state = state.clone();
                                    let entity = entity.clone();
                                    move |next, _, cx| {
                                        state.model_mut().draft.max_contour_regions =
                                            clamp_float(next, 1.0, 50.0, 0);
                                        notify(&entity, cx);
                                    }
                                },
                            ))
                        }),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child(checkbox(
                            "islands-remove-supported",
                            "Remove supported clusters",
                            draft.remove_supported_area_clusters,
                            false,
                            16.0,
                            false,
                            {
                                let state = state.clone();
                                let entity = entity.clone();
                                move |next, _, cx| {
                                    state.model_mut().draft.remove_supported_area_clusters = next;
                                    notify(&entity, cx);
                                }
                            },
                        ))
                        .when(draft.remove_supported_area_clusters, |this| {
                            this.child(unit_field(
                                "islands-area-per-support",
                                draft.area_per_support,
                                "mm²",
                                0.5,
                                1.0,
                                10.0,
                                false,
                                {
                                    let state = state.clone();
                                    let entity = entity.clone();
                                    move |next, _, cx| {
                                        state.model_mut().draft.area_per_support =
                                            clamp_float(next, 1.0, 10.0, 1);
                                        notify(&entity, cx);
                                    }
                                },
                            ))
                        }),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child(checkbox(
                            "islands-reduce-intersections",
                            "Reduce intersections",
                            draft.reduce_intersection,
                            false,
                            16.0,
                            false,
                            {
                                let state = state.clone();
                                let entity = entity.clone();
                                move |next, _, cx| {
                                    state.model_mut().draft.reduce_intersection = next;
                                    notify(&entity, cx);
                                }
                            },
                        ))
                        .when(draft.reduce_intersection, |this| {
                            this.child(unit_field(
                                "islands-intersection-threshold",
                                draft.intersection_threshold,
                                "mm²",
                                0.1,
                                0.1,
                                2.0,
                                false,
                                {
                                    let state = state.clone();
                                    let entity = entity.clone();
                                    move |next, _, cx| {
                                        state.model_mut().draft.intersection_threshold =
                                            clamp_float(next, 0.1, 2.0, 1);
                                        notify(&entity, cx);
                                    }
                                },
                            ))
                        }),
                ),
        );

    // `Reset defaults`: secondary, `w-full !h-8 px-3 text-xs`, 12px RotateCcw.
    let reset_state = state.clone();
    let reset_entity = entity.clone();
    let reset_button = flat_button(
        "islands-reset-defaults",
        div()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(6.0))
            .child(icon("rotate-ccw", 12.0, TEXT_STRONG))
            .child("Reset defaults"),
        32.0,
        SURFACE_1,
        BORDER_SUBTLE,
        TEXT_STRONG,
        Some(SURFACE_2),
        true,
        move |_, _, cx| {
            reset_state.model_mut().draft = IslandScanDraft::default();
            notify(&reset_entity, cx);
        },
    )
    .w_full();

    let body = div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(display_card)
        .child(scan_card)
        .child(filters_card)
        .child(clustering_card)
        .child(reset_button);

    // Actions: Cancel (secondary md) and Apply (tinted-accent md).
    let cancel_state = state.clone();
    let cancel_entity = entity.clone();
    let cancel = div().flex_1().child(
        button("islands-settings-cancel", "Cancel", ButtonVariant::Secondary, ButtonSize::Md)
            .w_full()
            .on_click(move |_, _, cx| {
                cancel_state.model_mut().settings_open = false;
                notify(&cancel_entity, cx);
            }),
    );
    let apply_state = state.clone();
    let apply_entity = entity.clone();
    let apply = div().flex_1().child(
        button("islands-settings-apply", "Apply", ButtonVariant::TintedAccent, ButtonSize::Md)
            .w_full()
            .on_click(move |_, _, cx| {
                let mut model = apply_state.model_mut();
                model.applied = model.draft;
                model.settings_open = false;
                drop(model);
                notify(&apply_entity, cx);
            }),
    );

    let close_state = state.clone();
    let close_entity = entity.clone();
    dialog_modal(
        512.0,
        "Scan Settings",
        Some("Configure island detection parameters".into()),
        None,
        IconTone::Neutral,
        body,
        div().flex().gap(px(8.0)).child(cancel).child(apply),
        move |_, _, cx| {
            close_state.model_mut().settings_open = false;
            notify(&close_entity, cx);
        },
    )
}

/// The dialog's card: `rounded-md border p-2.5` on `--surface-1`.
fn dialog_card() -> Div {
    div()
        .rounded(px(RADIUS_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(10.0))
        .flex()
        .flex_col()
        .gap(px(6.0))
}

/// `SectionHeader`: `pb-1.5 text-[10px] font-semibold uppercase tracking-wide
/// text-center` in `--text-strong`.
fn section_header(title: &'static str) -> Div {
    div()
        .pb(px(6.0))
        .text_center()
        .text_size(px(10.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT_STRONG))
        .child(title.to_uppercase())
}

/// A scan-dialog field: the 10px uppercase name and its 11px value over a
/// control (the source's `flex items-center justify-between mb-1` header).
fn draft_field_row(label: &'static str, value: String, control: Div) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(10.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_MUTED))
                        .child(label.to_uppercase()),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child(value),
                ),
        )
        .child(control)
}

/// A `NumberInput` at `h-8 w-full px-2.5 text-center no-spinners` with the unit
/// at `right-2`: the clustering card's four value boxes.
fn unit_field(
    id: impl Into<SharedString>,
    value: f32,
    unit: &'static str,
    step: f32,
    min: f32,
    max: f32,
    disabled: bool,
    on_change: impl Fn(f32, &mut Window, &mut App) + Clone + 'static,
) -> Div {
    let id: SharedString = id.into();
    let up = on_change.clone();
    let down = on_change;
    let can_up = !disabled && value < max;
    let can_down = !disabled && value > min;
    div()
        .relative()
        .w_full()
        .min_w(px(0.0))
        .child(
            div()
                .w_full()
                .h(px(32.0))
                .pl(px(10.0))
                .pr(px(36.0))
                .rounded(px(RADIUS_SM))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_0))
                .text_size(px(FONT_SM))
                .text_color(rgb(TEXT_STRONG))
                .flex()
                .items_center()
                .justify_center()
                .child(format_number(value)),
        )
        .child(
            div()
                .absolute()
                .right(px(8.0))
                .top(px(0.0))
                .bottom(px(0.0))
                .flex()
                .items_center()
                .text_size(px(10.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .child(unit),
        )
        .child(
            div()
                .absolute()
                .right(px(2.0))
                .top(px(0.0))
                .bottom(px(0.0))
                .w(px(16.0))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(2.0))
                .child(chevron_step(
                    format!("{id}-up"),
                    "chevron-up",
                    can_up,
                    move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                        up((value + step).min(max), window, cx)
                    },
                ))
                .child(chevron_step(
                    format!("{id}-down"),
                    "chevron-down",
                    can_down,
                    move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                        down((value - step).max(min), window, cx)
                    },
                )),
        )
}

// ---------------------------------------------------------------------------
// Island list — `IslandListCard.tsx`
// ---------------------------------------------------------------------------
/// PORT GAP: the Hierarchy button sets `show_hierarchy`; `IslandHierarchyModal`
/// is not ported.
pub fn island_list_body(
    state: &IslandListState,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let model = state.model();
    let total = model.islands.len();
    let active = model.active_count();
    let show_merged = model.show_merged;
    let sort_by = model.sort_by();
    let selected_id = model.selected_id;
    let rows = model.rows();
    let search = model.search.clone();
    let layer_height_mm = model.layer_height_mm;
    let z_offset_mm = model.z_offset_mm;
    drop(model);

    // Header right slot: `Button variant="accent" size="sm"` at `!h-9 !px-2.5
    // !py-0 text-[12px]` with an `sm` IconChip and a chevron.
    let hierarchy_state = state.clone();
    let hierarchy_entity = entity.clone();
    let hierarchy = flat_button(
        "island-list-hierarchy",
        div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(
                div()
                    .w(px(20.0))
                    .h(px(20.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(RADIUS_BARE))
                    .border_1()
                    .border_color(rgba(with_alpha(ACCENT_SECONDARY_CONTRAST, 0x8c)))
                    .bg(rgba(with_alpha(ACCENT_SECONDARY_CONTRAST, 0x24)))
                    .child(icon("network", 14.0, ACCENT_SECONDARY_CONTRAST)),
            )
            .child(
                div()
                    .line_height(px(FONT_MD))
                    .text_color(rgb(ACCENT_SECONDARY_CONTRAST))
                    .child("Hierarchy"),
            )
            .child(icon(
                "chevron-right",
                14.0,
                mix(ACCENT_SECONDARY_CONTRAST, WHITE, 0.82),
            )),
        36.0,
        SECONDARY_BUTTON_SURFACE,
        mix(SECONDARY_BUTTON_SURFACE, WHITE, 0.86),
        ACCENT_SECONDARY_CONTRAST,
        Some(mix(SECONDARY_BUTTON_SURFACE, BLACK, 0.91)),
        true,
        move |_, _, cx| {
            hierarchy_state.model_mut().show_hierarchy = true;
            notify(&hierarchy_entity, cx);
        },
    )
    .px(px(10.0));

    // Summary stats: `grid grid-cols-2 gap-1.5`, each `rounded p-1` on surface-1.
    let stat = |label: &'static str, value: usize| {
        div()
            .flex_1()
            .rounded(px(RADIUS_BARE))
            .bg(rgb(SURFACE_1))
            .p(px(4.0))
            .child(ui_meta(label))
            .child(
                div()
                    .text_size(px(FONT_MD))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(TEXT_STRONG))
                    .child(value.to_string()),
            )
    };
    let stats = div()
        .flex()
        .gap(px(6.0))
        .child(stat("Total Islands", total))
        .child(stat("Active", active));

    // Show child islands.
    let merged_toggle = checkbox(
        "island-list-show-merged",
        "Show child islands",
        show_merged,
        false,
        15.0,
        true,
        {
            let state = state.clone();
            let entity = entity.clone();
            move |next, _, cx| {
                state.model_mut().show_merged = next;
                notify(&entity, cx);
            }
        },
    );

    // Search box: `Input` at `w-full !h-8 px-2 text-sm`.
    let search_box = div()
        .w_full()
        .h(px(32.0))
        .px(px(8.0))
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .text_size(px(FONT_MD))
        .text_color(rgb(TEXT_MUTED))
        .flex()
        .items_center()
        .child(if search.is_empty() {
            "Search island ID…".to_owned()
        } else {
            search.clone()
        });

    // Sort row: ID / Volume / Layers, the active one `primary`.
    let sort_button = |id: &'static str, label: &'static str, sort: IslandSort| {
        let state = state.clone();
        let entity = entity.clone();
        let active = sort_by == sort;
        div().flex_1().child(
            button(
                id,
                label,
                if active {
                    ButtonVariant::Primary
                } else {
                    ButtonVariant::Secondary
                },
                ButtonSize::Auto,
            )
            .h(px(32.0))
            .px(px(8.0))
            .w_full()
            .on_click(move |_, _, cx| {
                state.model_mut().sort_by = Some(sort);
                notify(&entity, cx);
            }),
        )
    };
    let sort_row = div()
        .flex()
        .gap(px(4.0))
        .child(sort_button("island-sort-id", "ID", IslandSort::Id))
        .child(sort_button("island-sort-volume", "Volume", IslandSort::Volume))
        .child(sort_button("island-sort-layers", "Layers", IslandSort::Layers));

    // The list: `space-y-1 max-h-[30rem] overflow-y-auto`.
    let mut list = div()
        .id("island-list-scroll")
        .flex()
        .flex_col()
        .gap(px(4.0))
        .max_h(px(480.0))
        .overflow_y_scroll();
    list = list.child(
        div()
            .px(px(4.0))
            .text_size(px(FONT_SM))
            .text_color(rgb(INDICATOR))
            .child(if rows.len() == 1 {
                "1 island".to_owned()
            } else {
                format!("{} islands", rows.len())
            }),
    );
    for row in &rows {
        list = list.child(island_list_row(
            row,
            selected_id == Some(row.id),
            layer_height_mm,
            z_offset_mm,
            {
                let state = state.clone();
                let entity = entity.clone();
                let id = row.id;
                let is_selected = selected_id == Some(row.id);
                move |_, _, cx| {
                    state.model_mut().selected_id = if is_selected { None } else { Some(id) };
                    notify(&entity, cx);
                }
            },
        ));
    }

    // Clear selection.
    let clear_state = state.clone();
    let clear_entity = entity.clone();
    let clear = div().child(
        button(
            "island-list-clear",
            "Clear Selection",
            ButtonVariant::Secondary,
            ButtonSize::Auto,
        )
        .h(px(32.0))
        .px(px(10.0))
        .w_full()
        .on_click(move |_, _, cx| {
            clear_state.model_mut().selected_id = None;
            notify(&clear_entity, cx);
        }),
    );

    // `px-2.5 pt-1 pb-2.5 space-y-1.5`.
    div()
        .px(px(10.0))
        .pt(px(4.0))
        .pb(px(10.0))
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(div().flex().justify_end().child(hierarchy))
        .child(stats)
        .child(merged_toggle)
        .child(div().flex().flex_col().gap(px(4.0)).child(search_box).child(sort_row))
        .child(list)
        .when(selected_id.is_some(), |this| this.child(clear))
}

/// One island row: `p-1.5 rounded cursor-pointer border`, accent-tinted when
/// selected.
fn island_list_row(
    island: &IslandRow,
    selected: bool,
    layer_height_mm: f32,
    z_offset_mm: f32,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let layer_span = island.last_layer.saturating_sub(island.first_layer) + 1;
    let merged = !island.active;

    let mut badges = div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .flex_wrap()
        .mb(px(2.0))
        .child(
            div()
                .text_size(px(FONT_XS))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child(format!("#{}", island.id)),
        );
    if !island.child_ids.is_empty() {
        badges = badges.child(
            div()
                .px(px(4.0))
                .py(px(2.0))
                .rounded(px(RADIUS_BARE))
                .bg(rgba(with_alpha(GREEN_500, 0x33)))
                .text_size(px(10.0))
                .line_height(px(10.0))
                .text_color(rgb(GREEN_400))
                .child(format!("P+{}", island.child_ids.len())),
        );
    }
    if merged {
        if let Some(parent_id) = island.parent_id {
            badges = badges.child(
                div()
                    .px(px(4.0))
                    .py(px(2.0))
                    .rounded(px(RADIUS_BARE))
                    .bg(rgba(with_alpha(ORANGE_500, 0x33)))
                    .text_size(px(10.0))
                    .line_height(px(10.0))
                    .text_color(rgb(ORANGE_400))
                    .child(format!("C#{parent_id}")),
            );
        }
    }

    let mut details = div()
        .flex()
        .flex_col()
        .child(
            div()
                .text_size(px(FONT_SM))
                .line_height(px(FONT_SM * 1.25))
                .text_color(rgb(INDICATOR))
                .child(format!(
                    "L{}–{} ({layer_span})",
                    island.first_layer, island.last_layer
                )),
        )
        .child(
            div()
                .text_size(px(FONT_SM))
                .line_height(px(FONT_SM * 1.25))
                .text_color(rgb(INDICATOR))
                .child(match island.volume_mm3 {
                    Some(volume) => format!("{} mm³", format_number(volume)),
                    None => format!("{} mm²", format_number(island.total_area_mm2)),
                }),
        );
    if let (Some(max_area), Some(max_layer)) = (island.max_area_mm2, island.max_area_layer) {
        let layer = if layer_height_mm > 0.0 {
            (max_layer + z_offset_mm / layer_height_mm).round()
        } else {
            max_layer.round()
        };
        details = details.child(
            div()
                .text_size(px(FONT_SM))
                .line_height(px(FONT_SM * 1.25))
                .text_color(rgb(mix(TEXT_MUTED, BLACK, 0.80)))
                .child(format!("Max: {} mm² @ L{layer}", format_number(max_area))),
        );
    }

    let mut body = div()
        .flex()
        .items_start()
        .justify_between()
        .gap(px(6.0))
        .child(div().flex_1().min_w(px(0.0)).child(badges).child(details));

    if merged {
        if let Some(parent_id) = island.parent_id {
            body = body.child(
                div()
                    .mt(px(4.0))
                    .pt(px(2.0))
                    .border_t(px(1.0))
                    .border_color(rgb(BORDER_SUBTLE))
                    .text_size(px(FONT_SM))
                    .text_color(rgb(INDICATOR))
                    .child(format!("→ Child of #{parent_id}")),
            );
        }
    }
    if !island.child_ids.is_empty() {
        let children = island
            .child_ids
            .iter()
            .map(|id| format!("#{id}"))
            .collect::<Vec<_>>()
            .join(", ");
        body = body.child(
            div()
                .mt(px(4.0))
                .pt(px(2.0))
                .border_t(px(1.0))
                .border_color(rgb(BORDER_SUBTLE))
                .text_size(px(FONT_SM))
                .text_color(rgb(INDICATOR))
                .child(format!("Children: {children}")),
        );
    }

    let (background, border) = if selected {
        (
            mix(ACCENT, SURFACE_0, 0.14),
            mix(ACCENT, BORDER_SUBTLE, 0.55),
        )
    } else {
        (SURFACE_1, 0x00000000)
    };

    div()
        .id(format!("island-row-{}", island.id))
        .p(px(6.0))
        .rounded(px(RADIUS_BARE))
        .border_1()
        .border_color(rgba(border))
        .bg(rgb(background))
        .cursor_pointer()
        .when(!selected, |this| {
            this.hover(|style| style.bg(rgba(with_alpha(BLACK, 0x1a))))
        })
        .child(body)
        .on_click(on_click)
}

// ---------------------------------------------------------------------------
// Support settings — `AutoSupportSettingsBody.tsx`
// ---------------------------------------------------------------------------
/// Duplicate / Export / Import have no store behind them.
pub fn support_settings_body(
    state: &SupportSettingsState,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let model = state.model();
    let draft = model.draft;
    let preset = model.preset;
    let modified = model.modified;
    let preset_menu_open = model.preset_menu_open;
    drop(model);
    // Every preset this shell ships is a built-in, so selecting one locks the
    // policy fields: an edit made here could never be kept.
    let locked = preset.is_some();

    // ── Preset strip ────────────────────────────────────────────────────────
    let trigger_label: SharedString = match preset {
        Some(preset) => {
            if modified {
                format!("{} *", preset.label()).into()
            } else {
                preset.label().into()
            }
        }
        None => NO_ACTIVE_PRESET_LABEL.into(),
    };
    let options: Vec<SelectOption> = [AutoSupportPreset::Light, AutoSupportPreset::Medium, AutoSupportPreset::Heavy]
        .into_iter()
        .map(|preset| {
            let mut option = SelectOption::new(preset.id(), preset.label());
            option.right = Some(format!("{} · Built-in", preset.tier()).into());
            option
        })
        .collect();
    let current_id: SharedString = match preset {
        Some(preset) => preset.id().into(),
        None => "".into(),
    };

    let menu_state = state.clone();
    let menu_entity = entity.clone();
    let toggle_entity = entity.clone();
    let toggle_state = state.clone();
    let selector = select_dropdown(
        "support-preset-select",
        &options,
        current_id.as_ref(),
        preset_menu_open,
        false,
        move |_, _, cx| {
            let open = toggle_state.model().preset_menu_open;
            toggle_state.model_mut().preset_menu_open = !open;
            notify(&toggle_entity, cx);
        },
        move |index, _, cx| {
            let preset = [AutoSupportPreset::Light, AutoSupportPreset::Medium, AutoSupportPreset::Heavy]
                .get(index)
                .copied();
            let mut model = menu_state.model_mut();
            model.preset_menu_open = false;
            if let Some(preset) = preset {
                let values = preset.values();
                model.draft = values;
                model.baseline = values;
                model.preset = Some(preset);
                model.modified = false;
            }
            drop(model);
            notify(&menu_entity, cx);
        },
    );
    // The trigger's label is the preset name; `select_dropdown` derives it from
    // the option list, so an unselected state needs its own label.
    let selector = if preset.is_none() {
        div()
            .relative()
            .w_full()
            .child(selector)
            .child(
                div()
                    .absolute()
                    .left(px(10.0))
                    .right(px(40.0))
                    .top(px(0.0))
                    .bottom(px(0.0))
                    .flex()
                    .items_center()
                    .text_size(px(12.0))
                    .text_color(rgb(TEXT_STRONG))
                    .child(trigger_label.clone()),
            )
    } else {
        div().w_full().child(selector)
    };

    let strip_button = |id: &'static str, icon_name: &'static str, label: &'static str, enabled: bool| {
        flat_button(
            id,
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .child(icon(icon_name, 14.0, if enabled { TEXT_STRONG } else { TEXT_MUTED }))
                .child(label),
            32.0,
            SURFACE_1,
            BORDER_SUBTLE,
            TEXT_STRONG,
            Some(SURFACE_2),
            enabled,
            |_, _, _| {},
        )
    };
    let has_preset = preset.is_some();
    // `Duplicate` is the way out of a locked built-in: the copy is a custom
    // profile, so the policy fields unlock and Save can write it.
    let duplicate_state = state.clone();
    let duplicate_entity = entity.clone();
    let duplicate = flat_button(
        "support-preset-duplicate",
        div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .child(icon(
                "copy",
                14.0,
                if has_preset { TEXT_STRONG } else { TEXT_MUTED },
            ))
            .child("Duplicate"),
        32.0,
        SURFACE_1,
        BORDER_SUBTLE,
        TEXT_STRONG,
        Some(SURFACE_2),
        has_preset,
        move |_, _, cx| {
            let mut model = duplicate_state.model_mut();
            model.preset = None;
            model.baseline = model.draft;
            model.modified = false;
            drop(model);
            notify(&duplicate_entity, cx);
        },
    );
    let strip = div()
        .rounded(px(RADIUS_XL))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_2))
        .p(px(12.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(div().flex_1().min_w(px(0.0)).child(selector))
                .child(strip_button("support-preset-rename", "pen-line", "Rename", false))
                .child(duplicate)
                .child(strip_button("support-preset-export", "download", "Export", has_preset))
                .child(strip_button("support-preset-import", "upload", "Import", true)),
        );

    // ── Density & Sizing ────────────────────────────────────────────────────
    let density_card = field_card(
        "Density & Sizing",
        Some("How many supports, and how thick"),
        vec![
            labeled_number_field(
                "support-area-per-support",
                "Support Density",
                field_help_text(locked, "Projected area each support carries — smaller = more, tighter supports (grid spacing ≈ √value)"),
                draft.area_per_support_mm2,
                Some("mm²"),
                0.5,
                1.0,
                30.0,
                locked,
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        let mut model = state.model_mut();
                        model.draft.area_per_support_mm2 = normalize_value(next, 1.0, 30.0, 0.5);
                        model.modified = true;
                        drop(model);
                        notify(&entity, cx);
                    }
                },
            ),
            labeled_number_field(
                "support-size-scale",
                "Support Size",
                field_help_text(locked, "Master multiplier over the preset sizing bands — thicker or thinner everywhere"),
                draft.size_scale,
                Some("×"),
                0.05,
                0.5,
                2.0,
                locked,
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        let mut model = state.model_mut();
                        model.draft.size_scale = normalize_value(next, 0.5, 2.0, 0.05);
                        model.modified = true;
                        drop(model);
                        notify(&entity, cx);
                    }
                },
            ),
            labeled_number_field(
                "support-grid-threshold",
                "Grid Threshold",
                field_help_text(locked, "Flat regions at/above this area get a full grid; smaller regions get a single support"),
                draft.grid_area_threshold_mm2,
                Some("mm²"),
                5.0,
                5.0,
                200.0,
                locked,
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        let mut model = state.model_mut();
                        model.draft.grid_area_threshold_mm2 = normalize_value(next, 5.0, 200.0, 5.0);
                        model.modified = true;
                        drop(model);
                        notify(&entity, cx);
                    }
                },
            ),
            labeled_number_field(
                "support-flat-boost",
                "Flat Boost",
                field_help_text(locked, "Grid spacing on flat ceilings — lower = denser supports on anchor surfaces (0.7 = ~2× the supports)"),
                draft.flat_density_boost,
                Some("×"),
                0.05,
                0.5,
                1.0,
                locked,
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        let mut model = state.model_mut();
                        model.draft.flat_density_boost = normalize_value(next, 0.5, 1.0, 0.05);
                        model.modified = true;
                        drop(model);
                        notify(&entity, cx);
                    }
                },
            ),
            labeled_number_field(
                "support-slope-relax",
                "Slope Relax",
                field_help_text(locked, "Grid spacing on slopes at the self-support angle — higher = sparser"),
                draft.slope_relax_factor,
                Some("×"),
                0.1,
                1.0,
                2.0,
                locked,
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        let mut model = state.model_mut();
                        model.draft.slope_relax_factor = normalize_value(next, 1.0, 2.0, 0.1);
                        model.modified = true;
                        drop(model);
                        notify(&entity, cx);
                    }
                },
            ),
            labeled_number_field(
                "support-suction-scale",
                "Suction Scale",
                field_help_text(locked, "How strongly flat density grows with region area — large shallow ceilings carry more peel. 0 = off"),
                draft.suction_area_exponent,
                None,
                0.05,
                0.0,
                0.4,
                locked,
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        let mut model = state.model_mut();
                        model.draft.suction_area_exponent = normalize_value(next, 0.0, 0.4, 0.05);
                        model.modified = true;
                        drop(model);
                        notify(&entity, cx);
                    }
                },
            ),
        ],
    );

    // ── Stability ───────────────────────────────────────────────────────────
    let stability_card = field_card(
        "Stability",
        Some("Held against toppling and peel"),
        vec![
            labeled_number_field(
                "support-self-support-angle",
                "Self-Support Angle",
                field_help_text(locked, "Surfaces flatter than this angle get supports (resin standard: 45°). Higher = fewer, mostly on the steepest parts."),
                draft.self_support_angle_deg,
                Some("°"),
                5.0,
                20.0,
                75.0,
                locked,
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        let angle = normalize_value(next, 20.0, 75.0, 5.0);
                        state.model_mut().draft.self_support_angle_deg = angle;
                        state.model_mut().modified = true;
                        // Wired: the app classifies overhangs with this value.
                        entity
                            .update(cx, |this, cx| {
                                this.support_angle = angle;
                                cx.notify();
                            })
                            .ok();
                    }
                },
            ),
            labeled_toggle_field(
                "support-stabilization",
                "Stabilization Anchors",
                field_help_text(locked, "Add contacts along the edge or corner a pose bears on, so a leaning part is held against toppling and peel (default on)"),
                draft.stabilization_enabled,
                locked,
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        let mut model = state.model_mut();
                        model.draft.stabilization_enabled = next;
                        model.modified = true;
                        drop(model);
                        notify(&entity, cx);
                    }
                },
            ),
            labeled_toggle_field(
                "support-minima-reinforcement",
                "Minima Reinforcement",
                field_help_text(locked, "Ring each mesh minima with contacts on its own flank, so a section starts on a base instead of a needle (default on)"),
                draft.minima_reinforcement_enabled,
                locked,
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        let mut model = state.model_mut();
                        model.draft.minima_reinforcement_enabled = next;
                        model.modified = true;
                        drop(model);
                        notify(&entity, cx);
                    }
                },
            ),
        ],
    );

    // ── Footer ──────────────────────────────────────────────────────────────
    // `Button variant="tinted-danger" size="md"`: the `.ui-button-tint-danger`
    // fill, with the trash glyph before the label.
    let delete = flat_button(
        "support-preset-delete",
        div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(icon("trash-2", 14.0, DANGER))
            .child("Delete"),
        36.0,
        mix(DANGER, SURFACE_1, 0.14),
        mix(DANGER, BORDER_SUBTLE, 0.55),
        DANGER,
        Some(mix(DANGER, SURFACE_1, 0.24)),
        // `disabled={!activePreset || isBuiltIn}`: every preset here is a
        // built-in, so Delete is always greyed.
        false,
        |_, _, _| {},
    );

    let reset_state = state.clone();
    let reset_entity = entity.clone();
    let reset = flat_button(
        "support-preset-reset",
        div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(icon("rotate-ccw", 14.0, TEXT_STRONG))
            .child("Reset"),
        36.0,
        SURFACE_1,
        BORDER_SUBTLE,
        TEXT_STRONG,
        Some(SURFACE_2),
        modified,
        move |_, _, cx| {
            let mut model = reset_state.model_mut();
            model.draft = model.baseline;
            model.modified = false;
            drop(model);
            notify(&reset_entity, cx);
        },
    );

    let save_state = state.clone();
    let save_entity = entity.clone();
    let save = flat_button(
        "support-preset-save",
        "Save",
        36.0,
        accent_secondary_action_bg(),
        accent_secondary_action_border(),
        ACCENT_SECONDARY,
        None,
        modified && !has_preset,
        move |_, _, cx| {
            let mut model = save_state.model_mut();
            model.baseline = model.draft;
            model.modified = false;
            drop(model);
            notify(&save_entity, cx);
        },
    );

    let footer = div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .px(px(16.0))
        .py(px(12.0))
        .flex_shrink_0()
        .border_t(px(1.0))
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgba(with_alpha(SURFACE_1, 0xe6)))
        .child(delete)
        .child(div().flex().items_center().gap(px(8.0)).child(reset).child(save));

    // `flex-1 min-h-0 flex flex-col`: the scrolling surface then the footer.
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .p(px(16.0))
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(strip)
                .child(density_card)
                .child(stability_card),
        )
        .child(footer)
}

/// A `FieldCard`: `rounded-xl border p-3` on `--surface-2`, the uppercase header,
/// its one-liner and a two-column field grid.
fn field_card(
    title: &'static str,
    subtitle: Option<&'static str>,
    fields: Vec<Div>,
) -> Div {
    let mut grid = div().flex().flex_col().gap(px(8.0));
    let mut row = div().flex().gap(px(8.0));
    let mut filled = 0;
    for field in fields {
        row = row.child(div().flex_1().min_w(px(0.0)).child(field));
        filled += 1;
        if filled == 2 {
            grid = grid.child(row);
            row = div().flex().gap(px(8.0));
            filled = 0;
        }
    }
    if filled == 1 {
        grid = grid.child(row.child(div().flex_1().min_w(px(0.0))));
    }

    let mut card = div()
        .rounded(px(RADIUS_XL))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_2))
        .p(px(12.0))
        .flex()
        .flex_col()
        .child(
            div()
                .text_size(px(FONT_SM))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(INDICATOR))
                .child(title.to_uppercase()),
        );
    if let Some(subtitle) = subtitle {
        card = card.child(
            div()
                .mt(px(2.0))
                .text_size(px(FONT_XS))
                .line_height(px(FONT_XS * 1.375))
                .text_color(rgb(TEXT_MUTED))
                .child(subtitle),
        );
    }
    card.child(div().mt(px(8.0)).child(grid))
}

// ---------------------------------------------------------------------------
// Hollowing — `HollowingPanel.tsx`
// ---------------------------------------------------------------------------
/// voxel-edit Done/Clear only flip the panel's own flags.
pub fn hollowing_body(
    state: &HollowingState,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let model = state.model();
    let mode = model.mode;
    let voxel_size_mm = model.voxel_size_mm;
    let shell_thickness_mm = model.shell_thickness_mm;
    let infill_mode = model.infill_mode;
    let infill_density_pct = model.infill_density_pct();
    let infill_diameter_mm = model.infill_diameter_mm();
    let open_face = model.open_face;
    let open_face_menu_open = model.open_face_menu_open;
    let is_applying = model.is_applying;
    let is_previewing = model.is_previewing;
    let is_applying_blockers = model.is_applying_blockers;
    let can_apply = model.can_apply;
    let can_edit = model.can_edit;
    let is_edit_mode = model.is_edit_mode;
    let is_hollowing_applied = model.is_hollowing_applied;
    let shell_face_selection_pending = model.shell_face_selection_pending;
    drop(model);

    // Header right slot: the `rounded-full` mode badge with a 12px Droplets.
    let badge = div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .rounded_full()
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .px(px(8.0))
        .py(px(2.0))
        .child(icon("droplets", 12.0, ACCENT))
        .child(
            div()
                .text_size(px(10.0))
                .text_color(rgb(TEXT_MUTED))
                .child(match mode {
                    HollowMode::Infill => "Infill",
                    HollowMode::Cavity | HollowMode::ShellOpenFace => "Cavity",
                }),
        );

    // Mode card (or the voxel-edit hint): `rounded-md border p-2 space-y-1.5
    // min-h-[4.5rem] box-border`.
    let mode_card = if is_edit_mode {
        div()
            .rounded(px(RADIUS_MD))
            .border_1()
            .border_color(rgb(accent_secondary_action_border()))
            .bg(rgb(accent_secondary_action_bg()))
            .p(px(8.0))
            .min_h(px(72.0))
            .flex()
            .flex_col()
            .gap(px(6.0))
            .text_center()
            .child(
                div()
                    .text_size(px(FONT_SM))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(ACCENT_SECONDARY))
                    .child("Solid Areas"),
            )
            .child(
                div()
                    .text_size(px(10.0))
                    .line_height(px(10.0 * 1.375))
                    .text_color(rgb(TEXT_MUTED))
                    .child("Click or lasso to keep voxels solid, Alt+Lasso to release. Clear to reset all, Done to apply."),
            )
    } else {
        let cavity_state = state.clone();
        let cavity_entity = entity.clone();
        let infill_state = state.clone();
        let infill_entity = entity.clone();
        accent_card()
            .min_h(px(72.0))
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(ui_meta("Mode"))
            .child(
                div()
                    .flex()
                    .gap(px(4.0))
                    .child(div().flex_1().child(mode_pill(
                        "hollow-mode-cavity",
                        "Cavity",
                        mode == HollowMode::Cavity,
                        !is_applying,
                        move |_, _, cx| {
                            cavity_state.model_mut().mode = HollowMode::Cavity;
                            notify(&cavity_entity, cx);
                        },
                    )))
                    .child(div().flex_1().child(mode_pill(
                        "hollow-mode-infill",
                        "Infill",
                        mode == HollowMode::Infill,
                        !is_applying,
                        move |_, _, cx| {
                            infill_state.model_mut().mode = HollowMode::Infill;
                            notify(&infill_entity, cx);
                        },
                    ))),
            )
    };

    // Voxel Size: `clampFloat(value, 0.2, 10, 2)`.
    let voxel_card = section_card()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .text_size(px(FONT_SM))
                .text_color(rgb(TEXT_MUTED))
                .child("Voxel Size"),
        )
        .child(scroll_field(
            "hollow-voxel-size",
            voxel_size_mm,
            Some("mm"),
            0.1,
            0.2,
            10.0,
            is_applying,
            {
                let state = state.clone();
                let entity = entity.clone();
                move |next, _, cx| {
                    state.model_mut().voxel_size_mm = clamp_float(next, 0.2, 10.0, 2);
                    notify(&entity, cx);
                }
            },
        ));

    // Shell Thickness: `clampFloat(value, 0.2, 10, 1)`.
    let shell_card = section_card()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .text_size(px(FONT_SM))
                .text_color(rgb(TEXT_MUTED))
                .child("Shell Thickness"),
        )
        .child(scroll_field(
            "hollow-shell-thickness",
            shell_thickness_mm,
            Some("mm"),
            0.1,
            0.2,
            10.0,
            is_applying,
            {
                let state = state.clone();
                let entity = entity.clone();
                move |next, _, cx| {
                    state.model_mut().shell_thickness_mm = clamp_float(next, 0.2, 10.0, 1);
                    notify(&entity, cx);
                }
            },
        ));

    // Infill Type: Lattice / Pillar.
    let lattice_state = state.clone();
    let lattice_entity = entity.clone();
    let pillar_state = state.clone();
    let pillar_entity = entity.clone();
    let infill_type_card = section_card()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .text_size(px(FONT_SM))
                .text_color(rgb(TEXT_MUTED))
                .child("Infill Type"),
        )
        .child(
            div()
                .flex()
                .gap(px(4.0))
                .child(div().flex_1().child(mode_pill(
                    "hollow-infill-lattice",
                    "Lattice",
                    infill_mode == InfillMode::Lattice,
                    !is_applying,
                    move |_, _, cx| {
                        lattice_state.model_mut().infill_mode = InfillMode::Lattice;
                        notify(&lattice_entity, cx);
                    },
                )))
                .child(div().flex_1().child(mode_pill(
                    "hollow-infill-pillar",
                    "Pillar",
                    infill_mode == InfillMode::Pillar,
                    !is_applying,
                    move |_, _, cx| {
                        pillar_state.model_mut().infill_mode = InfillMode::Pillar;
                        notify(&pillar_entity, cx);
                    },
                ))),
        );

    let density_card = section_card()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .text_size(px(FONT_SM))
                .text_color(rgb(TEXT_MUTED))
                .child(if infill_mode == InfillMode::Pillar {
                    "Pillar Spacing"
                } else {
                    "Infill Density"
                }),
        )
        .child(scroll_field(
            "hollow-infill-density",
            infill_density_pct,
            Some("%"),
            1.0,
            1.0,
            100.0,
            is_applying,
            {
                let state = state.clone();
                let entity = entity.clone();
                move |next, _, cx| {
                    state.model_mut().infill_cell_mm = HollowingModel::density_pct_to_cell_mm(next);
                    notify(&entity, cx);
                }
            },
        ));

    let diameter_card = section_card()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .text_size(px(FONT_SM))
                .text_color(rgb(TEXT_MUTED))
                .child("Infill Diameter"),
        )
        .child(scroll_field(
            "hollow-infill-diameter",
            infill_diameter_mm,
            Some("mm"),
            0.1,
            MIN_INFILL_DIAMETER_MM,
            MAX_INFILL_DIAMETER_MM,
            is_applying,
            {
                let state = state.clone();
                let entity = entity.clone();
                move |next, _, cx| {
                    state.model_mut().infill_beam_radius_mm = clamp_float(
                        next / 2.0,
                        MIN_INFILL_DIAMETER_MM / 2.0,
                        MAX_INFILL_DIAMETER_MM / 2.0,
                        2,
                    );
                    notify(&entity, cx);
                }
            },
        ));

    // Open Face — `Select className="mt-1 w-full !h-8 px-2 text-xs"`.
    let face_options: Vec<SelectOption> = [
        OpenFace::XMin,
        OpenFace::XMax,
        OpenFace::YMin,
        OpenFace::YMax,
        OpenFace::ZMin,
        OpenFace::ZMax,
    ]
    .into_iter()
    .map(|face| SelectOption::new(face.value(), face.label()))
    .collect();
    let face_toggle_state = state.clone();
    let face_toggle_entity = entity.clone();
    let face_select_state = state.clone();
    let face_select_entity = entity.clone();
    let open_face_card = section_card()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .text_size(px(FONT_SM))
                .text_color(rgb(TEXT_MUTED))
                .child("Open Face"),
        )
        .child(select_dropdown(
            "hollow-open-face",
            &face_options,
            open_face.value(),
            open_face_menu_open,
            is_applying,
            move |_, _, cx| {
                let open = face_toggle_state.model().open_face_menu_open;
                face_toggle_state.model_mut().open_face_menu_open = !open;
                notify(&face_toggle_entity, cx);
            },
            move |index, _, cx| {
                let face = [
                    OpenFace::XMin,
                    OpenFace::XMax,
                    OpenFace::YMin,
                    OpenFace::YMax,
                    OpenFace::ZMin,
                    OpenFace::ZMax,
                ]
                .get(index)
                .copied();
                let mut model = face_select_state.model_mut();
                model.open_face_menu_open = false;
                if let Some(face) = face {
                    model.open_face = face;
                }
                drop(model);
                notify(&face_select_entity, cx);
            },
        ))
        .when(shell_face_selection_pending, |this| {
            this.child(
                div()
                    .mt(px(6.0))
                    .text_size(px(11.0))
                    .line_height(px(11.0 * 1.25))
                    .text_color(rgb(TEXT_MUTED))
                    .child("Click the face you want to open in the scene."),
            )
        });

    // Action row.
    let actions = if is_edit_mode {
        let clear_state = state.clone();
        let clear_entity = entity.clone();
        let done_state = state.clone();
        let done_entity = entity.clone();
        div()
            .flex()
            .gap(px(8.0))
            .child(div().flex_1().child(flat_button(
                "hollow-edit-clear",
                "Clear",
                32.0,
                SURFACE_1,
                BORDER_SUBTLE,
                TEXT_STRONG,
                Some(SURFACE_2),
                !(is_applying || is_previewing),
                move |_, _, cx| {
                    // PORT GAP: no voxel-edit engine — the flags are the port.
                    clear_state.model_mut().is_edit_mode = false;
                    notify(&clear_entity, cx);
                },
            )))
            .child(div().flex_1().child(flat_button(
                "hollow-edit-done",
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.0))
                    .when(is_applying_blockers, |this| {
                        this.child(spinner_colored(12.0, ACCENT_SECONDARY_CONTRAST))
                    })
                    .child(if is_applying_blockers {
                        "Applying Solid Areas"
                    } else {
                        "Done"
                    }),
                32.0,
                SECONDARY_BUTTON_SURFACE,
                mix(SECONDARY_BUTTON_SURFACE, WHITE, 0.86),
                ACCENT_SECONDARY_CONTRAST,
                Some(mix(SECONDARY_BUTTON_SURFACE, BLACK, 0.91)),
                !(is_applying || is_previewing || is_applying_blockers),
                move |_, _, cx| {
                    let mut model = done_state.model_mut();
                    model.is_edit_mode = false;
                    model.is_hollowing_applied = true;
                    drop(model);
                    notify(&done_entity, cx);
                },
            )))
    } else if is_hollowing_applied {
        let reset_state = state.clone();
        let reset_entity = entity.clone();
        let remove_state = state.clone();
        let remove_entity = entity.clone();
        div()
            .flex()
            .gap(px(8.0))
            .when(can_apply, |this| {
                this.child(div().flex_1().child(flat_button(
                    "hollow-reset-settings",
                    "Reset",
                    32.0,
                    SURFACE_1,
                    BORDER_SUBTLE,
                    TEXT_STRONG,
                    Some(SURFACE_2),
                    !(is_applying || is_previewing),
                    move |_, _, cx| {
                        reset_state.model_mut().reset_settings();
                        notify(&reset_entity, cx);
                    },
                )))
            })
            .child(div().flex_1().child(flat_button(
                "hollow-remove",
                "Remove Hollowing",
                32.0,
                SECONDARY_BUTTON_SURFACE,
                mix(SECONDARY_BUTTON_SURFACE, WHITE, 0.86),
                ACCENT_SECONDARY_CONTRAST,
                Some(mix(SECONDARY_BUTTON_SURFACE, BLACK, 0.91)),
                !(is_applying || is_previewing),
                move |_, _, cx| {
                    let mut model = remove_state.model_mut();
                    model.is_hollowing_applied = false;
                    drop(model);
                    notify(&remove_entity, cx);
                },
            )))
    } else {
        let edit_state = state.clone();
        let edit_entity = entity.clone();
        let apply_state = state.clone();
        let apply_entity = entity.clone();
        div()
            .flex()
            .gap(px(8.0))
            .child(div().flex_1().child(flat_button(
                "hollow-solid-areas",
                "Solid Areas",
                32.0,
                SURFACE_1,
                BORDER_SUBTLE,
                TEXT_STRONG,
                Some(SURFACE_2),
                !(is_applying || is_previewing || !can_edit),
                move |_, _, cx| {
                    edit_state.model_mut().is_edit_mode = true;
                    notify(&edit_entity, cx);
                },
            )))
            .child(div().flex_1().child(flat_button(
                "hollow-apply",
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.0))
                    .when(is_previewing, |this| {
                        this.child(spinner_colored(12.0, ACCENT_SECONDARY_CONTRAST))
                    })
                    .child(if is_applying {
                        "Applying…".to_owned()
                    } else if is_previewing {
                        "Updating".to_owned()
                    } else {
                        "Apply".to_owned()
                    }),
                32.0,
                SECONDARY_BUTTON_SURFACE,
                mix(SECONDARY_BUTTON_SURFACE, WHITE, 0.86),
                ACCENT_SECONDARY_CONTRAST,
                Some(mix(SECONDARY_BUTTON_SURFACE, BLACK, 0.91)),
                !(is_applying || is_previewing || !can_apply),
                move |_, _, cx| {
                    // PORT GAP: no hollowing engine — the flag is the port.
                    let mut model = apply_state.model_mut();
                    model.is_hollowing_applied = true;
                    drop(model);
                    notify(&apply_entity, cx);
                },
            )))
    };

    // `px-2 pb-2 space-y-2 sm:px-2.5 sm:pb-2.5`.
    div()
        .px(px(10.0))
        .pb(px(10.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(div().flex().justify_end().child(badge))
        .child(mode_card)
        .child(voxel_card)
        .child(shell_card)
        .when(mode == HollowMode::Infill, |this| {
            this.child(infill_type_card)
                .child(density_card)
                .child(diameter_card)
        })
        .when(mode == HollowMode::ShellOpenFace, |this| this.child(open_face_card))
        .child(actions)
}

// ---------------------------------------------------------------------------
// Hole punching — `HolePunchPanel.tsx`
// ---------------------------------------------------------------------------
/// The Hole Punching body: the interior-view hint, the depth mode, the punch
/// depth, the two hole diameters and the Reset / Apply row.
///
/// PORT GAP: no punching engine — Apply and Reset only flip the panel's flags.
pub fn hole_punch_body(
    state: &HolePunchState,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let model = state.model();
    let radius_mm = model.radius_mm;
    let effective_radius_y_mm = model.radius_y_mm.unwrap_or(model.radius_mm);
    let depth_mm = model.depth_mm;
    let depth_mode = model.depth_mode;
    let linked = model.linked;
    let can_use_auto_depth = model.can_use_auto_depth;
    let is_applying = model.is_applying;
    let can_apply = model.can_apply;
    let can_reset = model.can_reset;
    let disabled = model.disabled;
    let interior_view = model.interior_view;
    let interior_view_available = model.interior_view_available;
    drop(model);

    // `disabledStyle`: `opacity: 0.45` (the `grayscale(0.7)` has no GPUI
    // equivalent).
    let body_opacity = if disabled { 0.45 } else { 1.0 };

    // Interior View Mode card: the accent-secondary fill.
    let interior_card = div()
        .rounded(px(RADIUS_MD))
        .border_1()
        .border_color(rgb(accent_secondary_action_border()))
        .bg(rgb(accent_secondary_action_bg()))
        .p(px(8.0))
        .min_h(px(72.0))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .text_center()
        .child(
            div()
                .text_size(px(FONT_SM))
                .text_color(rgb(ACCENT_SECONDARY))
                .child("Interior View Mode"),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Press"),
                )
                .child(
                    div()
                        .px(px(4.0))
                        .rounded(px(RADIUS_BARE))
                        .bg(rgb(SURFACE_2))
                        .text_size(px(FONT_XS))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(ACCENT_SECONDARY))
                        .child("X"),
                )
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child("for"),
                )
                .child(
                    div()
                        .text_size(px(10.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(ACCENT_SECONDARY))
                        .child(if interior_view {
                            "Exterior View"
                        } else {
                            "Interior View"
                        }),
                ),
        );

    // Depth Mode: Auto / Manual.
    let auto_state = state.clone();
    let auto_entity = entity.clone();
    let manual_state = state.clone();
    let manual_entity = entity.clone();
    let depth_card = accent_card()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(ui_meta("Depth Mode"))
        .child(
            div()
                .flex()
                .gap(px(4.0))
                .child(div().flex_1().child(mode_pill(
                    "hole-punch-auto",
                    "Auto",
                    depth_mode == HolePunchDepthMode::Auto,
                    !(disabled || is_applying || !can_use_auto_depth),
                    move |_, _, cx| {
                        auto_state.model_mut().depth_mode = HolePunchDepthMode::Auto;
                        notify(&auto_entity, cx);
                    },
                )))
                .child(div().flex_1().child(mode_pill(
                    "hole-punch-manual",
                    "Manual",
                    depth_mode == HolePunchDepthMode::Manual,
                    !(disabled || is_applying),
                    move |_, _, cx| {
                        manual_state.model_mut().depth_mode = HolePunchDepthMode::Manual;
                        notify(&manual_entity, cx);
                    },
                ))),
        );

    // Punch Depth: `clampFloat(value, 1, 120, 1)`.
    let depth_field_card = section_card()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .text_size(px(FONT_SM))
                .text_color(rgb(TEXT_MUTED))
                .child("Punch Depth"),
        )
        .child(scroll_field(
            "hole-punch-depth",
            depth_mm,
            Some("mm"),
            0.5,
            1.0,
            120.0,
            disabled || is_applying,
            {
                let state = state.clone();
                let entity = entity.clone();
                move |next, _, cx| {
                    state.model_mut().depth_mm = clamp_float(next, 1.0, 120.0, 1);
                    notify(&entity, cx);
                }
            },
        ));

    // Hole Diameter: the link toggle, then the X and Y diameters.
    let link_state = state.clone();
    let link_entity = entity.clone();
    let link_toggle = div()
        .id("hole-punch-link")
        .w(px(20.0))
        .h(px(20.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(RADIUS_BARE))
        .when(!(disabled || is_applying), |this| {
            this.cursor_pointer()
                .hover(|style| style.bg(rgba(with_alpha(WHITE, 0x1a))))
        })
        .child(icon(
            if linked { "link" } else { "unlink" },
            14.0,
            if linked { ACCENT } else { TEXT_MUTED },
        ))
        .when(!(disabled || is_applying), |this| {
            this.on_click(move |_, _, cx| {
                let mut model = link_state.model_mut();
                model.linked = !model.linked;
                if model.linked {
                    model.radius_y_mm = None;
                } else {
                    // Initialize Y to the current X so it stays independent.
                    model.radius_y_mm = Some(model.radius_mm);
                }
                drop(model);
                notify(&link_entity, cx);
            })
        });

    let x_state = state.clone();
    let x_entity = entity.clone();
    let y_state = state.clone();
    let y_entity = entity.clone();
    let diameter_card = section_card()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(FONT_SM))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Hole Diameter"),
                )
                .child(link_toggle),
        )
        .child(scroll_field(
            "hole-punch-diameter-x",
            radius_mm * 2.0,
            Some("mm"),
            0.1,
            0.4,
            40.0,
            disabled || is_applying,
            move |next, _, cx| {
                let next_radius = clamp_float(next * 0.5, 0.2, 20.0, 2);
                let mut model = x_state.model_mut();
                model.radius_mm = next_radius;
                if model.linked {
                    model.radius_y_mm = None;
                }
                drop(model);
                notify(&x_entity, cx);
            },
        ))
        .child(scroll_field(
            "hole-punch-diameter-y",
            effective_radius_y_mm * 2.0,
            Some("mm"),
            0.1,
            0.4,
            40.0,
            disabled || is_applying || linked,
            move |next, _, cx| {
                y_state.model_mut().radius_y_mm = Some(clamp_float(next * 0.5, 0.2, 20.0, 2));
                notify(&y_entity, cx);
            },
        ));

    // Reset / Apply.
    let reset_state = state.clone();
    let reset_entity = entity.clone();
    let apply_state = state.clone();
    let apply_entity = entity.clone();
    let actions = div()
        .flex()
        .gap(px(8.0))
        .child(div().flex_1().child(flat_button(
            "hole-punch-reset",
            "Reset",
            32.0,
            SURFACE_1,
            BORDER_SUBTLE,
            TEXT_STRONG,
            Some(SURFACE_2),
            !(disabled || is_applying || !can_reset),
            move |_, _, cx| {
                *reset_state.model_mut() = HolePunchModel::default();
                notify(&reset_entity, cx);
            },
        )))
        .child(div().flex_1().child(flat_button(
            "hole-punch-apply",
            div()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(6.0))
                .when(is_applying, |this| {
                    this.child(spinner_colored(12.0, ACCENT_SECONDARY_CONTRAST))
                })
                .child(if is_applying { "Applying…" } else { "Apply" }),
            32.0,
            SECONDARY_BUTTON_SURFACE,
            mix(SECONDARY_BUTTON_SURFACE, WHITE, 0.86),
            ACCENT_SECONDARY_CONTRAST,
            Some(mix(SECONDARY_BUTTON_SURFACE, BLACK, 0.91)),
            !(disabled || is_applying || !can_apply),
            move |_, _, cx| {
                // PORT GAP: no punching engine — the flag is the port.
                let mut model = apply_state.model_mut();
                model.is_applying = false;
                drop(model);
                notify(&apply_entity, cx);
            },
        )));

    // `px-2 pb-2 space-y-2 sm:px-2.5 sm:pb-2.5`.
    div()
        .px(px(10.0))
        .pb(px(10.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .opacity(body_opacity)
        .when(can_use_auto_depth && interior_view_available, |this| {
            this.child(interior_card)
        })
        .child(depth_card)
        .when(depth_mode == HolePunchDepthMode::Manual, |this| {
            this.child(depth_field_card)
        })
        .child(diameter_card)
        .child(actions)
}
