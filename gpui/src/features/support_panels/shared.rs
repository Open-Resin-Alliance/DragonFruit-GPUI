use gpui::{
    App, ClickEvent, Div, ElementId, FontWeight, IntoElement, SharedString, Stateful, WeakEntity,
    Window, div, prelude::*, px, rgb, rgba,
};

use gpui_core::icons::icon;
use gpui_core::theme::*;
use gpui_ui::panel_fields::{
    BLACK, FieldAlign, Unit, WHITE, format_number, value_field_at, with_alpha,
};
use crate::DragonFruit;

/// Tailwind's bare `rounded` — `0.25rem`, the one radius the theme has no
/// token for (`--radius-*` only covers `sm`/`md`/`lg`, and `xl`).
pub(super) const ROUNDED_BARE: f32 = 4.0;

/// `ISLAND_LAYER_COLORS` in `islandPuckMarkers.ts`.
pub(super) const ISLAND_VOXEL: u32 = 0x87cefa;
pub(super) const ISLAND_MINIMA: u32 = 0x00ff00;
pub(super) const ISLAND_INTERSECTION: u32 = 0xff0000;
pub(super) const ISLAND_OVERHANG: u32 = 0xffa500;

/// `#22c55e` / `#4ade80` — the `P+n` child badge's `bg-green-500/20` and
/// `text-green-400`.
pub(super) const GREEN_500: u32 = 0x22c55e;
pub(super) const GREEN_400: u32 = 0x4ade80;
/// `#f97316` / `#fb923c` — the `C#n` parent badge's orange pair.
pub(super) const ORANGE_500: u32 = 0xf97316;
pub(super) const ORANGE_400: u32 = 0xfb923c;

pub(super) fn accent_secondary_action_border() -> u32 {
    // `color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 42%)`.
    mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.58)
}

pub(super) fn accent_secondary_action_bg() -> u32 {
    // `color-mix(in srgb, var(--accent-secondary), var(--surface-1) 92%)`.
    mix(ACCENT_SECONDARY, SURFACE_1, 0.08)
}

/// `activeModeStyle`: the selected pill of a two-way mode choice, as
/// `(background, border, text)`.
///
/// `border-color: color-mix(in srgb, var(--accent), var(--border-subtle) 30%)`
/// and `background: color-mix(in srgb, var(--accent), var(--surface-1) 85%)`.
pub(super) fn active_pill_style() -> (u32, u32, u32) {
    (
        mix(ACCENT, SURFACE_1, 0.15),
        mix(ACCENT, BORDER_SUBTLE, 0.70),
        TEXT_STRONG,
    )
}

/// `accentCardStyle`: `color-mix(accent, border-subtle 76%)` on
/// `color-mix(accent, surface-1 95%)`.
pub(super) fn accent_card() -> Div {
    div()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(mix(ACCENT, BORDER_SUBTLE, 0.24)))
        .bg(rgb(mix(ACCENT, SURFACE_1, 0.05)))
        .p(px(8.0))
}

/// `SECTION_CARD` / `panelCardStyle`: `rounded-md border p-2` on
/// `--border-subtle` / `--surface-1`.
pub(super) fn section_card() -> Div {
    div()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(8.0))
}

/// `.ui-meta`: 13px `--indicator`.
pub(super) fn ui_meta(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(FONT_SM))
        .text_color(rgb(INDICATOR))
        .child(text.into())
}

pub(super) fn notify(entity: &WeakEntity<DragonFruit>, cx: &mut App) {
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
pub(super) fn hint(
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
                .rounded(px(ROUNDED_BARE))
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
pub(super) fn hint_full(
    id: impl Into<SharedString>,
    trigger: impl IntoElement,
    text: impl Into<SharedString>,
) -> Stateful<Div> {
    hint(id, trigger, text).w_full()
}

/// `FieldHelpTooltip`: the 14px `?` tile beside a field label, with the help
/// text in a 260px popover on hover.
pub(super) fn field_help(id: impl Into<SharedString>, help: impl Into<SharedString>) -> Stateful<Div> {
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
                .rounded(px(ROUNDED_BARE))
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
                .rounded(px(ROUNDED_BARE))
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
pub(super) fn panel_icon_button(
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
pub(super) fn flat_button(
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
pub(super) fn square_stepper(
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
pub(super) fn scroll_field(
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
pub(super) fn chevron_step(
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
        .rounded(px(ROUNDED_BARE))
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
pub(super) fn labeled_number_field(
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
pub(super) fn labeled_toggle_field(
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
pub(super) fn checkbox(
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
                .rounded(px(ROUNDED_BARE))
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
pub(super) fn mode_pill(
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
pub(super) fn clamp_float(value: f32, min: f32, max: f32, decimals: i32) -> f32 {
    let safe = if value.is_finite() { value } else { min };
    let factor = 10f32.powi(decimals);
    let rounded = (safe * factor).round() / factor;
    rounded.clamp(min, max)
}

/// `normalizeFieldValue` from `AutoSupportSettingsBody.tsx`: clamped, then
/// rounded to the decimals the step implies.
pub(super) fn normalize_value(value: f32, min: f32, max: f32, step: f32) -> f32 {
    let decimals = if step < 0.1 {
        2
    } else if step < 1.0 {
        1
    } else {
        0
    };
    clamp_float(value, min, max, decimals)
}

/// A `SECTION_CARD` whose body is a muted label above the caller's control:
/// `rounded-md border p-2 space-y-1.5` with the source's `text-[13px]` name.
pub(super) fn labeled_section_card(label: impl IntoElement) -> Div {
    section_card()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .text_size(px(FONT_SM))
                .text_color(rgb(TEXT_MUTED))
                .child(label),
        )
}

/// The panel body's own padding: `px-2 pb-2 space-y-2 sm:px-2.5 sm:pb-2.5`.
pub(super) fn panel_body() -> Div {
    div()
        .px(px(10.0))
        .pb(px(10.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
}
