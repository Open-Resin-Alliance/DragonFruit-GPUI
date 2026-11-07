//! The save-choice sheet and the save-error sheet — `SceneFileModals.tsx`.

use std::rc::Rc;

use gpui::{
    App, ClickEvent, Div, FontWeight, SharedString, Window, div, prelude::*, px, rgb, rgba,
};
use gpui_core::theme::*;
use gpui_ui::panel_fields::{BLACK, with_alpha};
use gpui_ui::ui::{ButtonSize, ButtonVariant, IconTone, button, button_disabled, dialog_modal};

use super::chrome::{
    DIALOG_LG, GREEN_500, LEADING_SNUG, LEADING_TIGHT, action_slot, custom_dialog,
    custom_dialog_header, mix_b,
};

// ── Scene save choice / save error ──────────────────────────────────────────

/// `showSceneSaveChoiceModal` — `SceneFileModals.tsx`.
///
/// Title `Save Loaded Scene`, subtitle `Choose where Ctrl+S should save this
/// imported `.voxl` scene.`, a green `CheckCircle2` tile, the loaded-file card
/// and `Save as New Scene` / `Overwrite Loaded Scene` (disabled with no path).
pub fn scene_save_choice_modal(
    file_name: Option<SharedString>,
    path: Option<SharedString>,
    on_save_as: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_overwrite: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let close = Rc::new(on_close);
    let header_close = close.clone();
    // `color-mix(#22c55e, border-subtle 55%)` on `color-mix(#22c55e,
    // surface-1 90%)`, glyph `color-mix(#22c55e, text-strong 18%)`.
    let header = custom_dialog_header(
        "scene-save-choice",
        "circle-check",
        mix_b(GREEN_500, BORDER_SUBTLE, 0.55),
        mix_b(GREEN_500, SURFACE_1, 0.90),
        mix_b(GREEN_500, TEXT_STRONG, 0.18),
        "Save Loaded Scene",
        "Choose where Ctrl+S should save this imported `.voxl` scene.",
        move |event, window, cx| header_close(event, window, cx),
    );

    let card = div()
        .rounded(px(ROUNDED_LG))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        // `color-mix(surface-1, black 8%)`.
        .bg(rgb(mix_b(SURFACE_1, BLACK, 0.08)))
        .px(px(12.0))
        .py(px(10.0))
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(TEXT_MUTED))
                .child("LOADED FILE"),
        )
        .child(
            div()
                .mt(px(4.0))
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .line_height(px(14.0 * LEADING_TIGHT))
                .text_color(rgb(TEXT_STRONG))
                .child(file_name.unwrap_or_else(|| "Loaded scene".into())),
        )
        .child(
            div()
                .mt(px(4.0))
                .text_size(px(11.0))
                .line_height(px(11.0 * LEADING_SNUG))
                .text_color(rgb(TEXT_MUTED))
                .child(path.clone().unwrap_or_else(|| {
                    "Original file path unavailable (overwrite disabled)".into()
                })),
        );

    let save_as = button(
        "scene-save-as",
        "Save as New Scene",
        ButtonVariant::Secondary,
        ButtonSize::Md,
    )
    .whitespace_nowrap()
    .on_click(move |event, window, cx| on_save_as(event, window, cx));

    let overwrite = if path.is_some() {
        button(
            "scene-save-overwrite",
            "Overwrite Loaded Scene",
            ButtonVariant::TintedAccent,
            ButtonSize::Md,
        )
        .whitespace_nowrap()
        .on_click(move |event, window, cx| on_overwrite(event, window, cx))
    } else {
        button_disabled(
            "scene-save-overwrite",
            "Overwrite Loaded Scene",
            ButtonSize::Md,
        )
        .whitespace_nowrap()
    };

    let body = div()
        .flex()
        .flex_col()
        .gap(px(14.0))
        .p(px(20.0))
        .child(card)
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap(px(8.0))
                .pt(px(2.0))
                .child(div().flex_1().child(save_as))
                .child(div().flex_1().child(overwrite)),
        );

    custom_dialog(DIALOG_LG, header, body)
}

/// One save failure: the title, the message and the optional detail block.
pub struct SceneSaveError {
    /// `sceneSaveError.title` — `None` renders the source's `Save error`.
    pub title: Option<SharedString>,
    pub message: SharedString,
    pub detail: Option<SharedString>,
}

/// `sceneSaveError` — `SceneFileModals.tsx`'s `StructuredDialogModal`.
///
/// Title is the error's own title (falling back to `Save error`), a 20px
/// `AlertTriangle` on the warning tile, the message in `--text-strong` and the
/// detail in a `bg-black/30` monospace block. One `OK` action.
pub fn scene_save_error_modal(
    error: SceneSaveError,
    on_dismiss: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let dismiss = Rc::new(on_dismiss);
    let close = dismiss.clone();
    let ok = dismiss.clone();

    let mut body = div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .text_size(px(12.0))
        .text_color(rgb(TEXT_MUTED))
        .child(
            div()
                .text_size(px(14.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(TEXT_STRONG))
                .child(error.message),
        );
    if let Some(detail) = error.detail {
        body = body.child(
            div()
                .mt(px(8.0))
                .rounded(px(4.0))
                .bg(rgba(with_alpha(BLACK, 0x4d)))
                .p(px(8.0))
                .text_size(px(11.0))
                .whitespace_normal()
                .child(detail),
        );
    }

    // The source passes its own 20px amber-400 glyph; the shell's tone tile
    // paints the tone's 16px glyph (see PORT DEVIATIONS).
    dialog_modal(
        DIALOG_LG,
        error.title.unwrap_or_else(|| "Save error".into()),
        None,
        Some("alert-triangle"),
        IconTone::Warning,
        body,
        action_slot(
            button(
                "scene-save-error-ok",
                "OK",
                ButtonVariant::TintedAccent,
                ButtonSize::Md,
            )
            .on_click(move |event, window, cx| ok(event, window, cx)),
        ),
        move |event, window, cx| close(event, window, cx),
    )
}

