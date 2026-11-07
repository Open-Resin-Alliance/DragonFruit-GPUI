//! Toggles and segmented controls.

use gpui::{
    AnyElement, App, BoxShadow, Div, ElementId, FontWeight, SharedString, Stateful, Window, div,
    prelude::*, px, rgb, rgba,
};

use declarative_gpui::ui;
use gpui_core::icons::icon;
use gpui_core::theme::*;

use std::rc::Rc;

use super::constants::*;

// ── Toggle ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToggleSize {
    Sm,
    Md,
}

/// `Toggle.tsx` track and knob, `md` geometry.
pub fn toggle(on: bool) -> Div {
    toggle_sized(on, ToggleSize::Md)
}

/// `Toggle.tsx`.
///
/// Track: `flex shrink-0 items-center rounded-full px-0.5` with
/// `h-5 w-9` (sm) or `h-6 w-10` (md); `background: var(--accent)` when checked,
/// `var(--surface-2)` otherwise. Knob: `rounded-full bg-white shadow`, `h-4 w-4`
/// (sm) / `h-5 w-5` (md), translated 16px right when checked.
pub fn toggle_sized(on: bool, size: ToggleSize) -> Div {
    let (track_w, track_h, knob) = match size {
        ToggleSize::Sm => (36.0, 20.0, 16.0),
        ToggleSize::Md => (40.0, 24.0, 20.0),
    };
    ui! {
        div(
            // `px-0.5` is 2px; the knob's `translate-x-4` (16px) then lands it on
            // the far padding edge of both tracks.
            flex, items_center, flex_shrink_0, rounded_full, px = px(2.0),
            w = px(track_w), h = px(track_h),
            bg = rgb(if on { ACCENT } else { SURFACE_2 }),
            when(on, justify_end),
            when(!on, justify_start),
        ) {
            div(rounded_full, bg = rgb(WHITE), shadow_sm, w = px(knob), h = px(knob))
        }
    }
}

/// [`toggle_sized`] as the interactive `role="switch"` the source renders: the
/// track becomes the hit target and reports the next value on click.
pub fn toggle_switch(
    id: impl Into<ElementId>,
    on: bool,
    size: ToggleSize,
    on_change: impl Fn(bool, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    toggle_sized(on, size)
        .id(id)
        .cursor_pointer()
        .on_click(move |_, window, cx| on_change(!on, window, cx))
}

// ── SegmentedControl ────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SegmentedTone {
    Accent,
    AccentSecondary,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SegmentedSize {
    Sm,
    Md,
}

/// One pill of a [`segmented_control`].
pub struct SegmentedOption {
    pub value: SharedString,
    pub label: SharedString,
    pub disabled: bool,
}

impl SegmentedOption {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }
}

/// `SegmentedControl.tsx`.
///
/// The row is `inline-flex items-center gap-1`; each pill is
/// `.ui-segmented-option inline-flex items-center justify-center rounded-sm
/// border font-semibold uppercase tracking-wide` at `h-7 px-2.5 text-xs` (sm) or
/// `h-10 min-w-[92px] px-3 text-[12px]` (md). `--segmented-tone` is the active
/// tint. GPUI has no `letter-spacing`, so `tracking-wide` is not applied; the
/// label is upper-cased in Rust to match `uppercase`.
///
/// `on_change` receives the index of the pressed pill.
pub fn segmented_control(
    id: impl Into<SharedString>,
    options: &[SegmentedOption],
    value: &str,
    size: SegmentedSize,
    tone: SegmentedTone,
    full_width: bool,
    on_change: impl Fn(usize, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let tone_color = match tone {
        SegmentedTone::Accent => ACCENT,
        SegmentedTone::AccentSecondary => ACCENT_SECONDARY,
    };
    let on_change = Rc::new(on_change);
    let (height, pad_x, font_size, min_w) = match size {
        SegmentedSize::Sm => (28.0, 10.0, FONT_XS, 0.0),
        SegmentedSize::Md => (40.0, 12.0, 12.0, 92.0),
    };

    let mut row = div().id(id.clone()).flex().items_center().gap(px(4.0));
    if full_width {
        row = row.w_full();
    }

    for (index, option) in options.iter().enumerate() {
        let active = option.value.as_ref() == value;
        let callback = on_change.clone();
        let mut pill = div()
            .id(ElementId::named_usize(id.clone(), index))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(ROUNDED_SM))
            .border_1()
            .h(px(height))
            .px(px(pad_x))
            .text_size(px(font_size))
            // `text-xs` (sm) carries a 16px line-height; the md pill's
            // `text-[12px]` is arbitrary and inherits.
            .line_height(px(match size {
                SegmentedSize::Sm => 16.0,
                SegmentedSize::Md => 12.0 * LINE_HEIGHT_NORMAL,
            }))
            .font_weight(FontWeight::SEMIBOLD)
            .child(option.label.clone().to_uppercase());
        if min_w > 0.0 {
            pill = pill.min_w(px(min_w));
        }
        if full_width {
            pill = pill.flex_1();
        }

        pill = if option.disabled {
            pill.bg(rgb(mix(SURFACE_1, BLACK, 0.92)))
                .border_color(rgb(mix(BORDER_SUBTLE, BLACK, 0.90)))
                .text_color(rgb(mix(TEXT_MUTED, SURFACE_2, 0.82)))
                .cursor_not_allowed()
        } else if active {
            // `[data-active="true"]` — the tint mixed into surface-0 at 76%,
            // border at 90% of the tint, text at 75% of text-strong. The
            // `:hover:not(:disabled)` rule is more specific than
            // `[data-active]`, so hovering an active pill still shows
            // surface-2.
            pill.bg(rgb(mix(tone_color, SURFACE_0, 0.24)))
                .border_color(rgb(mix(tone_color, WHITE, 0.90)))
                .text_color(rgb(mix(tone_color, TEXT_STRONG, 0.75)))
                .cursor_pointer()
                .hover(|style| style.bg(rgb(SURFACE_2)))
                .on_click(move |_, window, cx| callback(index, window, cx))
        } else {
            pill.bg(rgb(SURFACE_1))
                .border_color(rgb(BORDER_SUBTLE))
                .text_color(rgb(TEXT_MUTED))
                .cursor_pointer()
                .hover(|style| style.bg(rgb(SURFACE_2)))
                .on_click(move |_, window, cx| callback(index, window, cx))
        };
        row = row.child(pill);
    }
    row
}

// ── Checkbox ────────────────────────────────────────────────────────────────

/// The boxes `.ui-checkbox` is rendered at: 15px, `!w-3.5 !h-3.5` (14px) and
/// `!w-4 !h-4` (16px).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CheckboxSize {
    Sm,
    Md,
    Lg,
}

/// The accent of the box.
///
/// `Accent` is `.ui-checkbox`'s `--accent` recipe. The other tones stand for a
/// native `<input type="checkbox">` that the source tints with `accent-color`
/// (the plugin warning passes `#f59e0b`): the browser fills the box with the
/// accent and draws the tick in its contrast colour, with none of
/// `.ui-checkbox`'s hover/ring rules.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CheckboxTone {
    Accent,
    Warning,
}

/// `.ui-checkbox` — `display: inline-grid; place-items: center`, `rounded`
/// (4px), `1px solid var(--border-subtle)` on `var(--surface-1)`.
///
/// Checked: border `color-mix(accent, white 12%)`, fill `color-mix(accent,
/// surface-0 18%)`, ring `0 0 0 3px color-mix(accent, transparent 82%)` and the
/// `::after` tick (8×5, 2px, `--accent-contrast`, `rotate(-45deg)`). GPUI
/// cannot rotate a bordered pseudo element, so the tick is the `check` glyph.
/// Hover (unchecked): border `color-mix(accent, border-subtle 55%)` on
/// `color-mix(surface-2, transparent 6%)`.
fn checkbox_box(checked: bool, size: CheckboxSize, tone: CheckboxTone, disabled: bool) -> Div {
    let box_size = match size {
        CheckboxSize::Sm => 14.0,
        CheckboxSize::Md => 15.0,
        CheckboxSize::Lg => 16.0,
    };
    let accent = match tone {
        CheckboxTone::Accent => ACCENT,
        CheckboxTone::Warning => 0xf59e0b,
    };
    let ui_checkbox = tone == CheckboxTone::Accent;

    let mut box_ = div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .w(px(box_size))
        .h(px(box_size))
        .rounded(px(4.0))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1));

    if checked {
        box_ = if ui_checkbox {
            box_.border_color(rgb(mix(accent, WHITE, 0.88)))
                .bg(rgb(mix(accent, SURFACE_0, 0.82)))
                .shadow(vec![
                    BoxShadow::new(px(0.0), px(0.0), rgba(with_alpha(accent, 0x2e)).into())
                        .blur_radius(px(0.0))
                        .spread_radius(px(3.0)),
                ])
                .child(icon("check", 10.0, ACCENT_CONTRAST))
        } else {
            box_.border_color(rgb(accent))
                .bg(rgb(accent))
                .child(icon("check", 10.0, BLACK))
        };
    }

    if disabled {
        box_
    } else if !checked && ui_checkbox {
        box_.cursor_pointer().hover(|style| {
            style
                .border_color(rgb(mix(ACCENT, BORDER_SUBTLE, 0.45)))
                // `color-mix(surface-2, transparent 6%)` leaves 94% surface-2.
                .bg(rgba(with_alpha(SURFACE_2, 0xf0)))
        })
    } else {
        box_.cursor_pointer()
    }
}

/// [`checkbox_box`] for a caller that renders its own label and hit target:
/// disabled boxes take `.ui-checkbox:disabled` (`opacity: 0.5`,
/// `cursor: not-allowed`).
pub fn checkbox_with(checked: bool, size: CheckboxSize, tone: CheckboxTone, disabled: bool) -> Div {
    let block = checkbox_box(checked, size, tone, disabled);
    if disabled {
        block.opacity(0.5).cursor_not_allowed()
    } else {
        block
    }
}

/// The label treatment of [`checkbox`] — the two the sources use.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CheckboxLabel {
    /// The island list's `ui-meta` row: `gap-1.5 px-0.5 py-0.5` around the
    /// default 15px `.ui-checkbox`, the label at 13px `--indicator`.
    Meta,
    /// The island dialog's `!w-4 !h-4` box: `gap-2`, the label at 11px
    /// `font-medium` in `--text-strong`.
    Field,
}

/// `.ui-checkbox` with its label: `flex items-center gap-2 cursor-pointer
/// select-none`.
///
/// `.ui-checkbox` is 15px with a 4px radius; `!w-4 !h-4` (the dialog's) is
/// 16px. The box itself is [`checkbox_box`], so the checked ring, the tick and
/// the hover state are the shared recipe.
pub fn checkbox(
    id: impl Into<ElementId>,
    label: &'static str,
    checked: bool,
    disabled: bool,
    label_style: CheckboxLabel,
    on_change: impl Fn(bool, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let (box_size, gap, meta) = match label_style {
        CheckboxLabel::Meta => (CheckboxSize::Md, 6.0, true),
        CheckboxLabel::Field => (CheckboxSize::Lg, 8.0, false),
    };
    let mut el = div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(gap))
        .when(meta, |this| this.py(px(2.0)).px(px(2.0)))
        .cursor_pointer()
        .child(checkbox_box(
            checked,
            box_size,
            CheckboxTone::Accent,
            disabled,
        ))
        .child(
            div()
                .text_size(px(if meta { FONT_SM } else { 11.0 }))
                .font_weight(if meta {
                    FontWeight::NORMAL
                } else {
                    FontWeight::MEDIUM
                })
                .text_color(rgb(if meta { INDICATOR } else { TEXT_STRONG }))
                .child(label),
        );
    if disabled {
        // `.ui-checkbox:disabled`: `opacity: 0.5; cursor: not-allowed`.
        el = el.opacity(0.5).cursor_not_allowed();
    } else {
        el = el.on_click(move |_, window, cx| on_change(!checked, window, cx));
    }
    el
}

// ── Toggle row ──────────────────────────────────────────────────────────────

/// `LabeledToggleInput`: the `ui-label` name with its help affordance over the
/// 36px `.ui-input` switch row — `Enabled` / `Disabled` on the left, the `sm`
/// track on the right.
///
/// `help` is the caller's `FieldHelpTooltip` tile; the row only places it.
pub fn toggle_row(
    id: impl Into<ElementId>,
    label: &'static str,
    help: Option<AnyElement>,
    checked: bool,
    disabled: bool,
    on_change: impl Fn(bool, &mut Window, &mut App) + 'static,
) -> Div {
    let (border, background, text) = if disabled {
        (BORDER_SUBTLE, mix(SURFACE_2, BLACK, 0.92), TEXT_MUTED)
    } else if checked {
        (
            mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.64),
            mix(ACCENT_SECONDARY, SURFACE_1, 0.10),
            TEXT_STRONG,
        )
    } else {
        (BORDER_SUBTLE, SURFACE_1, TEXT_MUTED)
    };

    let mut row = div()
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
        .child(toggle_sized(checked, ToggleSize::Sm));
    if disabled {
        row = row.opacity(0.55).cursor_not_allowed();
    } else {
        row = row
            .cursor_pointer()
            .on_click(move |_, window, cx| on_change(!checked, window, cx));
    }

    let mut label_row = div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .text_size(px(FONT_SM))
        .text_color(rgb(INDICATOR))
        .font_weight(FontWeight::MEDIUM)
        .child(label);
    if let Some(help) = help {
        label_row = label_row.child(help);
    }

    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .min_w(px(0.0))
        .child(label_row)
        .child(row)
}
