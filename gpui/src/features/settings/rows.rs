use super::*;

// ── Shared row shapes for the bridge-backed tabs ─────────────────────────

/// `SettingRow` with a `label` only — the source's rows that pass no
/// `description`.
pub(crate) fn setting_row_label_only(label: impl Into<SharedString>, control: impl IntoElement) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.))
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .p(px(6.))
        .child(
            div()
                .min_w(px(0.))
                .pl(px(4.))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(relative(1.2))
                        .text_color(rgb(TEXT_STRONG))
                        .child(label.into()),
                ),
        )
        .child(div().flex().items_center().gap(px(8.)).child(control))
}

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

/// `!h-8 !px-2.5 text-xs` secondary button with a leading glyph in the
/// source's disabled state (the bridge that would run it is absent here).
pub(crate) fn inert_small_button(id: &'static str, label: &'static str, slug: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .h(px(32.))
        .px(px(10.))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.))
        .flex_shrink_0()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT_MUTED))
        .opacity(0.6)
        .child(icon(slug, 14., TEXT_MUTED))
        .child(label)
}
