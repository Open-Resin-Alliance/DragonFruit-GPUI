use super::*;

// ── Shared row shapes for the bridge-backed tabs ─────────────────────────

/// A `NumberInput` with the source's unit suffix (`sec`, `min`).
pub(crate) fn number_with_unit(input: Div, unit: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .child(input)
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child(unit),
        )
}
