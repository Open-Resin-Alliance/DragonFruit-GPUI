//! Toggles and segmented controls.

use gpui::{
    App, Div, ElementId, FontWeight, SharedString, Stateful, Window, div, prelude::*, px, rgb,
};

use declarative_gpui::ui;
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
