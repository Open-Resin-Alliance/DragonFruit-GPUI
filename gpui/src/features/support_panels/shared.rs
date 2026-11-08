use gpui::{
    App, ClickEvent, Div, ElementId, FontWeight, IntoElement, SharedString, Stateful, WeakEntity,
    Window, div, prelude::*, px, rgb, rgba,
};

use gpui_core::icons::icon;
use gpui_core::theme::*;
use gpui_ui::panel_fields::{active_mode_style, format_number};
use gpui_ui::ui::{self, ButtonSize, ButtonSpec, ButtonVariant};
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
pub(crate) fn hint(
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
                .child(ui::number_stepper(
                    &id,
                    can_up,
                    can_down,
                    move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                        up((value + step).min(max), window, cx)
                    },
                    move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                        down((value - step).max(min), window, cx)
                    },
                )),
        )
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
    let mut spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Sm);
    if active {
        let (background, border, text) = active_mode_style();
        // The source's inline `style` outranks `:hover`, and `fill` pins the
        // hover fill to the same background.
        spec = spec.fill(background, border, text);
    }
    if !enabled {
        spec = spec.disabled();
    }
    let el = ui::button_with(id, label, &spec);
    if enabled {
        el.on_click(on_click)
    } else {
        el
    }
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
    ui::sub_card_with(None, 8.0).child(
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
