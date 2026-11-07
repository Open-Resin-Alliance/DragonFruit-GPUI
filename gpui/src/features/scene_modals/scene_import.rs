//! The scene-import placement and hollowing decisions — `SceneFileModals.tsx`.

use std::rc::Rc;

use gpui::{App, ClickEvent, Div, FontWeight, SharedString, Window, div, prelude::*, px, rgb};
use gpui_core::theme::*;
use gpui_ui::ui::{ButtonSize, ButtonVariant, button, modal_shell};

use super::chrome::{
    DIALOG_LG, LEADING_RELAXED, Run, custom_dialog_header, mix_b, rich_paragraph, two_button_row,
};
use super::format::{grouped};

/// `sceneImportPlacementPrompt` — `SceneFileModals.tsx`'s inline dialog.
///
/// Title `Scene may be off-plate`, subtitle `Choose how to place imported
/// models.`, a `LayoutGrid` accent tile, the imported-file card with the
/// off-plate count, the two explanations and `Load As-Is` / `Auto-Arrange`.
/// Backdrop clicks and the close tile resolve `load_as_is`.
pub fn scene_import_placement_modal(
    file_name: SharedString,
    off_plate_model_count: usize,
    model_count: usize,
    on_load_as_is: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_auto_arrange: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let close = Rc::new(on_load_as_is);
    let header_close = close.clone();
    let header = custom_dialog_header(
        "scene-import-placement",
        "layout-grid",
        mix_b(ACCENT, BORDER_SUBTLE, 0.45),
        mix_b(ACCENT, SURFACE_1, 0.88),
        ACCENT,
        "Scene may be off-plate",
        "Choose how to place imported models.",
        move |event, window, cx| header_close(event, window, cx),
    );

    let plural = if model_count == 1 { "" } else { "s" };
    let card = div()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .px(px(12.0))
        .py(px(8.0))
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(TEXT_MUTED))
                .child("IMPORTED SCENE"),
        )
        .child(
            div()
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .truncate()
                .child(file_name),
        )
        .child(
            div()
                .mt(px(4.0))
                .text_size(px(12.0))
                .text_color(rgb(TEXT_MUTED))
                .child(format!(
                    "{} of {} model{plural} appear outside the build plate.",
                    grouped(off_plate_model_count as f64),
                    grouped(model_count as f64)
                )),
        );

    let explanation = div()
        .text_size(px(12.0))
        .line_height(px(12.0 * LEADING_RELAXED))
        .text_color(rgb(TEXT_MUTED))
        .child(rich_paragraph(
            12.0,
            LEADING_RELAXED,
            vec![
                Run::strong("Auto-Arrange"),
                Run::plain(" will reposition imported models onto free space on the plate."),
            ],
        ))
        .child(
            div().mt(px(4.0)).child(rich_paragraph(
                12.0,
                LEADING_RELAXED,
                vec![
                    Run::strong("Load As-Is"),
                    Run::plain(" keeps scene coordinates exactly as stored in the file."),
                ],
            )),
        );

    let load_as_is = button(
        "scene-import-load-as-is",
        "Load As-Is",
        ButtonVariant::Secondary,
        ButtonSize::Md,
    )
    .w_full()
    .on_click({
        let close = close.clone();
        move |event, window, cx| close(event, window, cx)
    });
    let auto_arrange = button(
        "scene-import-auto-arrange",
        "Auto-Arrange",
        ButtonVariant::TintedAccent,
        ButtonSize::Md,
    )
    .w_full()
    .on_click(move |event, window, cx| on_auto_arrange(event, window, cx));

    let body = div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .p(px(20.0))
        .child(card)
        .child(explanation)
        .child(two_button_row(load_as_is, auto_arrange));

    modal_shell(DIALOG_LG).child(header).child(body)
}

/// `sceneImportHollowingPrompt` — `SceneFileModals.tsx`'s inline dialog.
///
/// Title `Scene was hollowed before export`, subtitle `Choose whether to keep
/// the hollow.`, a `CircleDashed` accent tile, the imported-file card with the
/// cavity count, the two explanations and `Import Solid` / `Keep Hollow`.
pub fn scene_import_hollowing_modal(
    file_name: SharedString,
    hollowed_model_count: usize,
    model_count: usize,
    on_import_solid: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_keep_hollow: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let close = Rc::new(on_keep_hollow);
    let header_close = close.clone();
    let header = custom_dialog_header(
        "scene-import-hollowing",
        "circle-dashed",
        mix_b(ACCENT, BORDER_SUBTLE, 0.45),
        mix_b(ACCENT, SURFACE_1, 0.88),
        ACCENT,
        "Scene was hollowed before export",
        "Choose whether to keep the hollow.",
        move |event, window, cx| header_close(event, window, cx),
    );

    let plural = if model_count == 1 { "model" } else { "models" };
    let verb = if model_count == 1 { "carries" } else { "carry" };
    let card = div()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .px(px(12.0))
        .py(px(8.0))
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(TEXT_MUTED))
                .child("IMPORTED SCENE"),
        )
        .child(
            div()
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .truncate()
                .child(file_name),
        )
        .child(
            div()
                .mt(px(4.0))
                .text_size(px(12.0))
                .text_color(rgb(TEXT_MUTED))
                .child(format!(
                    "{} of {model_count} {plural} {verb} an interior cavity.",
                    grouped(hollowed_model_count as f64)
                )),
        );

    let explanation = div()
        .text_size(px(12.0))
        .line_height(px(12.0 * LEADING_RELAXED))
        .text_color(rgb(TEXT_MUTED))
        .child(rich_paragraph(
            12.0,
            LEADING_RELAXED,
            vec![
                Run::strong("Keep Hollow"),
                Run::plain(
                    " builds the stored cavity into each model, so it prints hollow exactly as authored.",
                ),
            ],
        ))
        .child(
            div().mt(px(4.0)).child(rich_paragraph(
                12.0,
                LEADING_RELAXED,
                vec![
                    Run::strong("Import Solid"),
                    Run::plain(
                        " loads the outer shape only. The cavity is kept, so you can apply it later from the Hollowing panel.",
                    ),
                ],
            )),
        );

    let import_solid = button(
        "scene-import-solid",
        "Import Solid",
        ButtonVariant::Secondary,
        ButtonSize::Md,
    )
    .w_full()
    .on_click(move |event, window, cx| on_import_solid(event, window, cx));
    let keep_hollow = button(
        "scene-import-keep-hollow",
        "Keep Hollow",
        ButtonVariant::TintedAccent,
        ButtonSize::Md,
    )
    .w_full()
    .on_click({
        let close = close.clone();
        move |event, window, cx| close(event, window, cx)
    });

    let body = div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .p(px(20.0))
        .child(card)
        .child(explanation)
        .child(two_button_row(import_solid, keep_hollow));

    modal_shell(DIALOG_LG).child(header).child(body)
}

