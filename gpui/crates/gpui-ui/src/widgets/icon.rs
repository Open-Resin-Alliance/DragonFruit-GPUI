//! Icon tones (`iconTone.ts`) and the `IconChip` square.

use gpui::{
    App, ClickEvent, Div, ElementId, FontWeight, IntoElement, Stateful, Window, div, prelude::*, px,
    rgb,
};

use gpui_core::icons::icon;
use gpui_core::theme::*;

use super::constants::*;

// ── Icon tones ──────────────────────────────────────────────────────────────

/// `iconTone.ts`'s `IconTone`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IconTone {
    Accent,
    Warning,
    Danger,
    Neutral,
}

/// `(border, background, foreground)` from `ICON_TONE_STYLES`.
pub(crate) fn icon_tone_colors(tone: IconTone) -> (u32, u32, u32) {
    match tone {
        IconTone::Warning => (
            mix(0xd97706, BORDER_SUBTLE, 0.5),
            mix(0xd97706, SURFACE_1, 0.15),
            0xd97706,
        ),
        IconTone::Danger => (
            mix(0xef4444, BORDER_SUBTLE, 0.45),
            mix(0xef4444, SURFACE_1, 0.12),
            DANGER,
        ),
        IconTone::Accent => (
            mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.55),
            mix(ACCENT_SECONDARY, SURFACE_1, 0.10),
            ACCENT_SECONDARY,
        ),
        IconTone::Neutral => (BORDER_SUBTLE, SURFACE_1, TEXT_MUTED),
    }
}

// ── IconChip ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IconChipSize {
    Xs,
    Sm,
    Md,
    Lg,
}

/// `IconChip.tsx`: the bordered square an icon lives in.
///
/// `xs` is the 16px badge (`rounded-[3px] text-[10px] font-bold`), `sm` the 20px
/// menu-row slot (`rounded`), `md` the 32px toolbar tile (`rounded-md`), `lg` the
/// 36px modal-header tile (`rounded-md`). `tone: Neutral` is the source's
/// `inherit` chip fill (`border-subtle` / `surface-1`); GPUI has no
/// `currentColor`, so the icon colour is explicit.
pub fn icon_chip(size: IconChipSize, tone: IconTone, icon_name: &str) -> Div {
    let icon_size = match size {
        // `iconSizeClassMap.xs` is empty, so the source leaves the glyph at
        // lucide's 24px default inside a 16px box. The port uses the box's own
        // 16px (`h-4 w-4`), which is what every `xs` call site passes as a
        // child instead of using the `icon` prop.
        IconChipSize::Xs => 16.0,
        IconChipSize::Sm => 14.0,
        IconChipSize::Md => 16.0,
        IconChipSize::Lg => 16.0,
    };
    let (_, _, foreground) = icon_tone_colors(tone);
    icon_chip_with(size, tone, icon(icon_name, icon_size, foreground))
}

/// [`icon_chip`] with arbitrary content (a tick, a number, a glyph) in the slot.
pub fn icon_chip_with(size: IconChipSize, tone: IconTone, content: impl IntoElement) -> Div {
    // Only `xs` carries a type size (`text-[10px] font-bold leading-none`);
    // `sm`/`md`/`lg` have none, so their content inherits the surrounding type.
    let (width, height, radius, font_size) = match size {
        IconChipSize::Xs => (16.0, 16.0, 3.0, Some(10.0)),
        IconChipSize::Sm => (20.0, 20.0, ROUNDED_BARE, None),
        IconChipSize::Md => (32.0, 32.0, ROUNDED_MD, None),
        IconChipSize::Lg => (36.0, 36.0, ROUNDED_MD, None),
    };
    let (border, background, foreground) = icon_tone_colors(tone);
    let mut chip = div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .rounded(px(radius))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .w(px(width))
        .h(px(height))
        .text_color(rgb(foreground))
        .child(content);
    if let Some(font_size) = font_size {
        chip = chip.text_size(px(font_size));
    }
    if size == IconChipSize::Xs {
        chip = chip.font_weight(FontWeight::BOLD).line_height(px(10.0));
    }
    chip
}

// ── IconButton tile ─────────────────────────────────────────────────────────

/// `IconButton`'s `solid` tile at a call site that pins its own padding
/// (`className="!p-0.5"`): the `.ui-button-secondary` box — `rounded-sm`, the
/// variant's `surface-1` fill, `border-subtle` edge and `text-muted` glyph —
/// made interactive.
///
/// [`super::button::icon_button_toned`] owns the size-class paddings; this is
/// the one variant the class list cannot express, because the source overrides
/// the padding per call site with an `!p-*` utility.
pub fn icon_button_tile(
    id: impl Into<ElementId>,
    icon_name: &str,
    icon_size: f32,
    tone: IconTone,
    pad: f32,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let (border, background, foreground) = icon_tone_colors(tone);
    div()
        .id(id)
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .p(px(pad))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(SURFACE_2)))
        .child(icon(icon_name, icon_size, foreground))
        .on_click(on_click)
}
