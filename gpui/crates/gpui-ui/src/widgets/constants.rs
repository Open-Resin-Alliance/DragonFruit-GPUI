//! Geometry and colour helpers shared by the widget modules.
//!
//! These were file-private helpers on the old `ui.rs`; several widgets need
//! them, so they live here and reach the widget modules through `super`.
//!
//! The `rounded-*` scale is **not** here: it is `gpui_core::theme::ROUNDED_*`,
//! the one place the radius tokens live. A widget module glob-imports the theme,
//! so a local copy would shadow it — that ambiguity is how the card radius
//! drifted to 8px the first time.

use gpui::{BoxShadow, px, rgba};

/// Bare `rounded` — Tailwind's `0.25rem`. The theme has no token for it.
pub(crate) const ROUNDED_BARE: f32 = 4.0;

/// Tailwind's `leading-tight` (1.25) — distinct from the app's
/// `--line-height-tight` (1.2) that `.ui-panel` reads.
pub(crate) const LEADING_TIGHT: f32 = 1.25;
/// Tailwind's `leading-snug` (1.375).
pub(crate) const LEADING_SNUG: f32 = 1.375;
/// Tailwind's `leading-relaxed` (1.625).
pub(crate) const LEADING_RELAXED: f32 = 1.625;

/// `#ffffff` — the toggle knob and the `bg-white/10` ghost hover.
pub(crate) const WHITE: u32 = 0xffffff;
/// `#000000` — the base of every `color-mix(..., black N%)` in the sources.
pub(crate) const BLACK: u32 = 0x000000;

/// `#rrggbb` plus an alpha byte, for `gpui::rgba`.
pub(crate) fn with_alpha(color: u32, alpha: u8) -> u32 {
    (color << 8) | alpha as u32
}

/// A CSS `box-shadow` layer: `offset-x offset-y blur spread rgba(...)`.
pub(crate) fn shadow(x: f32, y: f32, blur: f32, spread: f32, color: u32) -> BoxShadow {
    BoxShadow::new(px(x), px(y), rgba(color).into())
        .blur_radius(px(blur))
        .spread_radius(px(spread))
}

/// A CSS `box-shadow` layer with `inset`.
pub(crate) fn inset_shadow(x: f32, y: f32, blur: f32, spread: f32, color: u32) -> BoxShadow {
    shadow(x, y, blur, spread, color).inset()
}
