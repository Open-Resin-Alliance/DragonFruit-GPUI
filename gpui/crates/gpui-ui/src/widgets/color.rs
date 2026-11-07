//! Colour swatches and the CSS hex parser.

use gpui::{App, ClickEvent, Div, ElementId, Stateful, Window, div, prelude::*, px, rgb};

use gpui_core::theme::*;

use super::constants::*;

// ── ColorSwatchInput ────────────────────────────────────────────────────────

/// Parse a CSS `#rrggbb` / `#rgb` / `rrggbb` string into `0xRRGGBB`.
///
/// `ColorSwatchInput`'s `value` is a CSS colour string; GPUI needs the packed
/// integer. Anything unparsable falls back to transparent so a bad value reads
/// as an empty swatch rather than an arbitrary colour.
pub fn parse_hex_color(value: &str) -> u32 {
    let hex = value.trim().trim_start_matches('#');
    match hex.len() {
        3 => {
            let expand = |c: char| u8::from_str_radix(&format!("{c}{c}"), 16).ok();
            let mut chars = hex.chars();
            match (chars.next(), chars.next(), chars.next()) {
                (Some(r), Some(g), Some(b)) => match (expand(r), expand(g), expand(b)) {
                    (Some(r), Some(g), Some(b)) => {
                        (r as u32) << 16 | (g as u32) << 8 | b as u32
                    }
                    _ => 0,
                },
                _ => 0,
            }
        }
        6 => u32::from_str_radix(hex, 16).unwrap_or(0),
        _ => 0,
    }
}

/// `ColorSwatchInput.tsx` — the colour picker's swatch, filling its box.
///
/// `shrink-0 cursor-pointer appearance-none overflow-hidden rounded border p-0`
/// with `border-color: var(--border-subtle)` and
/// `box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.28)`. The hairline keeps every
/// colour the same apparent size, including a pale resin on a pale surface.
///
/// The size comes from the caller, as at the source (the settings rows and the
/// overlay panels use different boxes). GPUI has no native colour picker, so the
/// swatch is a filled box; `on_click` is the caller's hook for opening one.
pub fn color_swatch_input(
    id: impl Into<ElementId>,
    width: f32,
    height: f32,
    value: &str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    color_swatch(width, height, value)
        .id(id)
        .cursor_pointer()
        .on_click(on_click)
}

/// [`color_swatch_input`] without the click hook.
pub fn color_swatch(width: f32, height: f32, value: &str) -> Div {
    div()
        .flex_shrink_0()
        .w(px(width))
        .h(px(height))
        .rounded(px(ROUNDED_BARE))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(parse_hex_color(value)))
        .overflow_hidden()
        .shadow(vec![inset_shadow(
            0.0,
            0.0,
            0.0,
            1.0,
            with_alpha(BLACK, 0x47),
        )])
}
