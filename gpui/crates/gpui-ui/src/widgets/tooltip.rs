//! Trigger-anchored tooltips and the cursor-following mouse tooltip.

use gpui::{
    Div, ElementId, FontWeight, IntoElement, Pixels, Point, SharedString, Stateful, div, point,
    prelude::*, px, relative, rgb, rgba,
};

use gpui_core::theme::*;

use super::constants::*;

// ── Tooltip ─────────────────────────────────────────────────────────────────

/// `Tooltip.tsx` with its defaults (`offsetY` 28, `maxWidth` 260).
pub fn tooltip(
    id: impl Into<ElementId>,
    trigger: impl IntoElement,
    content: impl IntoElement,
) -> Stateful<Div> {
    tooltip_with(id, trigger, content, 28.0, 260.0)
}

/// Which side a trigger-anchored popover takes (`Tooltip.tsx`'s `placement`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TooltipPlacement {
    Below,
    Right,
}

/// `Tooltip.tsx` — trigger-anchored popover.
///
/// The popover is `rounded px-2 py-1.5 text-[11px] leading-tight font-medium
/// shadow-lg` on `rgba(24, 24, 24, 0.98)`, `1px solid var(--accent)`. React
/// portals it to `<body>`; with no portal, GPUI renders it as an absolute child
/// of the wrapper and reveals it with the wrapper's `group_hover` state (the
/// source's hover/focus reveal). The source's viewport clamp and cursor-following
/// are not representable in a static tree, so the box hangs under the trigger and
/// is centred.
pub fn tooltip_with(
    id: impl Into<ElementId>,
    trigger: impl IntoElement,
    content: impl IntoElement,
    offset_y: f32,
    max_width: f32,
) -> Stateful<Div> {
    tooltip_placed(
        id,
        trigger,
        content,
        offset_y,
        max_width,
        TooltipPlacement::Below,
    )
}

/// [`tooltip_with`] with the source's `placement`: `Below` centres the box under
/// the trigger, `Right` puts it beside the trigger, centred on it (the column
/// layout the rail uses). The source's flip to the opposite side when the box
/// would leave the viewport needs a measure pass and is not reproduced.
pub fn tooltip_placed(
    id: impl Into<ElementId>,
    trigger: impl IntoElement,
    content: impl IntoElement,
    offset_y: f32,
    max_width: f32,
    placement: TooltipPlacement,
) -> Stateful<Div> {
    let id: ElementId = id.into();
    let group: SharedString = id.to_string().into();
    let mut popover = div()
        .absolute()
        .flex()
        .opacity(0.0)
        .group_hover(group.clone(), |style| style.opacity(1.0));
    popover = match placement {
        TooltipPlacement::Below => popover
            .left(px(0.0))
            .right(px(0.0))
            .top(relative(1.0))
            .mt(px(offset_y))
            .justify_center(),
        TooltipPlacement::Right => popover
            .left(relative(1.0))
            .top(px(0.0))
            .bottom(px(0.0))
            .ml(px(offset_y))
            .items_center(),
    };
    div()
        .id(id)
        .relative()
        .flex()
        .group(group)
        .child(trigger)
        .child(popover.child(
            div()
                .rounded(px(ROUNDED_BARE))
                .px(px(8.0))
                .py(px(6.0))
                .text_size(px(11.0))
                .line_height(px(11.0 * LEADING_TIGHT))
                .font_weight(FontWeight::MEDIUM)
                .bg(rgba(with_alpha(0x181818, 0xfa)))
                .text_color(rgb(TEXT_STRONG))
                .border_1()
                .border_color(rgb(ACCENT))
                .max_w(px(max_width))
                .whitespace_normal()
                .text_left()
                .shadow(vec![
                    shadow(0.0, 6.0, 32.0, 0.0, with_alpha(BLACK, 0x70)),
                    shadow(0.0, 1.5, 8.0, 0.0, with_alpha(BLACK, 0x47)),
                ])
                .child(content),
        ))
}


// ── MouseTooltip ────────────────────────────────────────────────────────────

/// `MouseTooltip.tsx` — a box that follows the pointer at a `(15, 15)` offset.
///
/// The source listens to `window.mousemove`, then measures and clamps the box
/// into the viewport, flipping to the other side of the cursor when it would
/// overflow. GPUI has no global pointer listener in a static tree and no measure
/// pass, so the caller supplies the pointer position (from its own
/// `on_mouse_move`) and the box is drawn at `position + offset`. It carries no
/// hit handler, so it never intercepts input (`pointer-events: none`). The clamp
/// and the flip are not reproduced.
pub fn mouse_tooltip(
    position: Point<Pixels>,
    offset: Point<Pixels>,
    content: impl IntoElement,
) -> Div {
    div()
        .absolute()
        .left(position.x + offset.x)
        .top(position.y + offset.y)
        .child(content)
}

/// [`mouse_tooltip`] with the source's default `offset = { x: 15, y: 15 }`.
pub fn mouse_tooltip_default(
    position: Point<Pixels>,
    content: impl IntoElement,
) -> Div {
    mouse_tooltip(position, point(px(15.0), px(15.0)), content)
}
