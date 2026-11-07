//! The structured dialog modal.

use gpui::{
    App, ClickEvent, Div, ElementId, FontWeight, IntoElement, SharedString, Window, div, prelude::*,
    px, rgb, rgba,
};

use gpui_core::icons::icon;
use gpui_core::theme::*;

use super::button::{IconButtonSize, IconButtonVariant, icon_button};
use super::constants::*;
use super::icon::{IconTone, icon_tone_colors};

// ── StructuredDialogModal ───────────────────────────────────────────────────

/// The backdrop of `StructuredDialogModal`: `fixed inset-0 flex items-center
/// justify-center bg-black/55 px-3`. GPUI has no `backdrop-blur`, so the blur is
/// omitted.
fn modal_backdrop() -> Div {
    div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .px(px(12.0))
        .bg(rgba(with_alpha(BLACK, 0x8c)))
}

/// The panel of `StructuredDialogModal`: `w-full overflow-hidden rounded-xl
/// border shadow-2xl` on `surface-0` with `border-subtle` and
/// `0 24px 46px rgba(0,0,0,0.42)`.
fn modal_panel(width: f32) -> Div {
    div()
        .w_full()
        .max_w(px(width))
        .rounded(px(12.0))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .overflow_hidden()
        .flex()
        .flex_col()
        .shadow(vec![shadow(0.0, 24.0, 46.0, 0.0, with_alpha(BLACK, 0x6b))])
}

/// The backdrop and card of `StructuredDialogModal`.
pub fn modal_shell(width: f32) -> Div {
    modal_backdrop().child(modal_panel(width))
}

/// `StructuredDialogModal.tsx` in full: backdrop, panel, header (toned icon tile,
/// title, subtitle, close button), body and action row.
///
/// Header: `flex items-center justify-between gap-4 border-b px-5 py-4`; icon
/// tile `h-9 w-9 rounded-md border` with the tone's `ICON_TONE_STYLES`; title
/// `text-base font-semibold leading-tight`; subtitle `mt-0.5 text-[11px]
/// leading-snug`. Body: `space-y-4 p-5`. Actions: `grid grid-flow-col
/// auto-cols-fr gap-2 pt-1` — a row of equal columns, so each action should be
/// `flex_1`.
pub fn dialog_modal(
    width: f32,
    title: impl Into<SharedString>,
    subtitle: Option<SharedString>,
    icon_name: Option<&str>,
    tone: IconTone,
    body: impl IntoElement,
    actions: impl IntoElement,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    dialog_modal_full(
        width, title, subtitle, icon_name, tone, false, body, actions, on_close,
    )
}

/// [`dialog_modal`] with the source's `closeDisabled`: the close tile takes the
/// `.ui-button:disabled` recipe and drops its click handler.
#[allow(clippy::too_many_arguments)]
pub fn dialog_modal_full(
    width: f32,
    title: impl Into<SharedString>,
    subtitle: Option<SharedString>,
    icon_name: Option<&str>,
    tone: IconTone,
    close_disabled: bool,
    body: impl IntoElement,
    actions: impl IntoElement,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let title: SharedString = title.into();
    let (tone_border, tone_bg, tone_fg) = icon_tone_colors(tone);

    let mut left = div().flex().items_center().gap(px(12.0)).min_w(px(0.0));
    if let Some(name) = icon_name {
        left = left.child(
            div()
                .flex()
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .w(px(36.0))
                .h(px(36.0))
                .rounded(px(ROUNDED_MD))
                .border_1()
                .border_color(rgb(tone_border))
                .bg(rgb(tone_bg))
                .child(icon(name, 16.0, tone_fg)),
        );
    }

    let mut titles = div().min_w(px(0.0)).pr(px(8.0)).child(
        div()
            .text_size(px(16.0))
            .font_weight(FontWeight::SEMIBOLD)
            .line_height(px(16.0 * LEADING_TIGHT))
            .text_color(rgb(TEXT_STRONG))
            .child(title.clone()),
    );
    if let Some(subtitle) = subtitle {
        titles = titles.child(
            div()
                .mt(px(2.0))
                .text_size(px(11.0))
                .line_height(px(11.0 * LEADING_SNUG))
                .text_color(rgb(TEXT_MUTED))
                .child(subtitle),
        );
    }
    left = left.child(titles);

    let header = div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.0))
        .border_b(px(1.0))
        .border_color(rgb(BORDER_SUBTLE))
        .px(px(20.0))
        .py(px(16.0))
        .child(left)
        .child(if close_disabled {
            div()
                .flex()
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .w(px(36.0))
                .h(px(36.0))
                .rounded(px(ROUNDED_MD))
                .border_1()
                .border_color(rgb(mix(BORDER_SUBTLE, BLACK, 0.90)))
                .bg(rgb(mix(SURFACE_1, BLACK, 0.92)))
                .cursor_not_allowed()
                .child(icon("x", 16.0, mix(TEXT_MUTED, SURFACE_2, 0.82)))
                .into_any_element()
        } else {
            icon_button(
                ElementId::named_usize(title, 0),
                "x",
                IconButtonVariant::Surface,
                IconButtonSize::Md,
            )
            .on_click(on_close)
            .into_any_element()
        });

    let content = div()
        .p(px(20.0))
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(body)
        .child(div().flex().gap(px(8.0)).pt(px(4.0)).child(actions));

    modal_backdrop().child(
        modal_panel(width)
            .child(header)
            .child(content),
    )
}
