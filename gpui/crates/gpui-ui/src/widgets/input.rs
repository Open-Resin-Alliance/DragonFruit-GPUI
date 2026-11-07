//! The `.ui-input` frame.

use gpui::{Div, ElementId, SharedString, Stateful, div, prelude::*, px, rgb};

use gpui_core::theme::*;

// ── Input ───────────────────────────────────────────────────────────────────

/// `Input.tsx` / `.ui-input`: `border border-subtle bg-surface-0
/// text-text-strong rounded-sm px-2.5 py-1.5 text-sm leading-[1.3]`, with
/// `border-color: var(--accent)` on focus.
///
/// The caller puts its own text element (or a `TextInput`) inside the box.
pub fn input_box(id: impl Into<ElementId>, focused: bool) -> Stateful<Div> {
    div()
        .id(id)
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(if focused { ACCENT } else { BORDER_SUBTLE }))
        .bg(rgb(SURFACE_0))
        .text_color(rgb(TEXT_STRONG))
        .px(px(10.0))
        .py(px(6.0))
        .text_size(px(FONT_SM))
        .line_height(px(FONT_SM * 1.3))
        .flex()
        .items_center()
}

/// [`input_box`] with a single line of text already inside it.
pub fn input_field(
    id: impl Into<ElementId>,
    value: impl Into<SharedString>,
    focused: bool,
) -> Stateful<Div> {
    input_box(id, focused).child(value.into())
}
