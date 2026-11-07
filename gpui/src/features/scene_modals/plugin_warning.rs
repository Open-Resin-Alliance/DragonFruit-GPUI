//! The experimental plugin-import warning — `SceneFileModals.tsx`.

use std::rc::Rc;

use gpui::{App, ClickEvent, Div, FontWeight, SharedString, Window, div, prelude::*, px, rgb};
use gpui_core::theme::*;
use gpui_ui::ui::{
    ButtonSize, ButtonVariant, CheckboxSize, CheckboxTone, IconTone, button, checkbox_with,
    dialog_modal,
};

use super::chrome::{AMBER_200, AMBER_500, DIALOG_LG, mix_b, muted_small_paragraph};

// ── Plugin import warning ───────────────────────────────────────────────────

/// `showPluginImportWarningModal` — `SceneFileModals.tsx`.
///
/// Title is `{feature} is Experimental` when the caller names the feature,
/// otherwise `Plugin Import is Experimental`; subtitle `This feature is still
/// under development.`; warning tile; the body paragraph; and the footer with
/// the `Do not remind again` checkbox, `Cancel` and the amber `Continue`.
pub fn plugin_import_warning_modal(
    plugin_import_warning_title: Option<SharedString>,
    plugin_import_warning_body: Option<SharedString>,
    skip_future: bool,
    on_toggle_skip: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_continue: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let title: SharedString = match plugin_import_warning_title {
        Some(feature) => format!("{feature} is Experimental").into(),
        None => "Plugin Import is Experimental".into(),
    };
    let body_text: SharedString = plugin_import_warning_body.unwrap_or_else(|| {
        "Geometry, support placement, and transforms can import differently across scene \
         variants, so unforeseen results are still possible."
            .into()
    });

    let cancel_close = Rc::new(on_cancel);
    let close_cancel = cancel_close.clone();

    let checkbox = div()
        .id("plugin-import-skip")
        .flex()
        .items_center()
        .gap(px(8.0))
        .cursor_pointer()
        .on_click(move |event, window, cx| on_toggle_skip(event, window, cx))
        .child(checkbox_with(
            skip_future,
            CheckboxSize::Sm,
            CheckboxTone::Warning,
            false,
        ))
        .child(
            div()
                .text_size(px(12.0))
                .text_color(rgb(TEXT_MUTED))
                .child("Do not remind again"),
        );

    let cancel = button(
        "plugin-import-cancel",
        "Cancel",
        ButtonVariant::Secondary,
        ButtonSize::Md,
    )
    .on_click(move |event, window, cx| close_cancel(event, window, cx));

    // `Continue` overrides the button's own colours with the amber pair:
    // `color-mix(#f59e0b, border-subtle 45%)` on `color-mix(#f59e0b,
    // surface-1 86%)`, label `#fde68a`.
    let continue_ = div()
        .id("plugin-import-continue")
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(mix_b(AMBER_500, BORDER_SUBTLE, 0.45)))
        .bg(rgb(mix_b(AMBER_500, SURFACE_1, 0.86)))
        .px(px(11.0))
        .pt(px(5.0))
        .pb(px(7.0))
        .h(px(36.0))
        .text_size(px(FONT_MD))
        .font_weight(FontWeight::SEMIBOLD)
        .line_height(px(FONT_MD))
        .text_color(rgb(AMBER_200))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(mix_b(AMBER_500, SURFACE_1, 0.76))))
        .on_click(move |event, window, cx| on_continue(event, window, cx))
        .child("Continue");

    // The source keeps the checkbox and the two buttons in the body's last row;
    // the dialog's action slot is that row here, so the shell adds no empty one.
    let footer = div()
        .flex()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .child(checkbox)
        .child(
            div()
                .flex()
                .flex_shrink_0()
                .items_center()
                .gap(px(8.0))
                .child(cancel)
                .child(continue_),
        );

    dialog_modal(
        DIALOG_LG,
        title,
        Some("This feature is still under development.".into()),
        Some("alert-triangle"),
        IconTone::Warning,
        muted_small_paragraph(body_text),
        footer,
        move |event, window, cx| cancel_close(event, window, cx),
    )
}

