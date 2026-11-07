//! The two unsaved-changes prompts — `SceneFileModals.tsx`.

use std::rc::Rc;

use gpui::{
    App, ClickEvent, Div, ElementId, FontWeight, SharedString, Window, div, prelude::*, px, rgb,
};
use gpui_core::theme::*;
use gpui_ui::ui::{
    ButtonSize, ButtonVariant, IconTone, button, button_disabled, dialog_modal_full,
};

use super::chrome::{
    DIALOG_LG, LEADING_RELAXED, action_slot, muted_small_paragraph, tinted_action,
};

// ── UnsavedChangesDialog ────────────────────────────────────────────────────

/// The shared shell of the two unsaved-changes prompts.
///
/// `UnsavedChangesDialog` in `SceneFileModals.tsx`: title `Unsaved Scene
/// Changes`, `AlertTriangle` on a warning tile, `z-[220]`, a close tile that is
/// disabled while `busy`, and the two actions `Discard Changes`
/// (`tinted-danger`, `Trash2`) and the caller's save label (`secondary`).
fn unsaved_changes_dialog(
    id: &'static str,
    subtitle: SharedString,
    body: Div,
    confirm_label: SharedString,
    busy: bool,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_discard: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_save: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let cancel = Rc::new(on_cancel);
    let close_cancel = cancel.clone();

    let mut discard = tinted_action(
        ElementId::named_usize(id, 1),
        "Discard Changes",
        Some(("trash-2", 14.0)),
        ButtonVariant::TintedDanger,
    )
    .w_full();
    if busy {
        discard = discard.opacity(0.5).cursor_not_allowed();
    } else {
        discard = discard.on_click(move |event, window, cx| on_discard(event, window, cx));
    }

    let save = if busy {
        button_disabled(
            ElementId::named_usize(id, 2),
            confirm_label,
            ButtonSize::Md,
        )
        .w_full()
    } else {
        button(
            ElementId::named_usize(id, 2),
            confirm_label,
            ButtonVariant::Secondary,
            ButtonSize::Md,
        )
        .w_full()
        .on_click(move |event, window, cx| on_save(event, window, cx))
    };

    dialog_modal_full(
        DIALOG_LG,
        "Unsaved Scene Changes",
        Some(subtitle),
        Some("alert-triangle"),
        IconTone::Warning,
        busy,
        body,
        div()
            .flex()
            .gap(px(8.0))
            .child(action_slot(discard))
            .child(action_slot(save)),
        move |event, window, cx| {
            if busy {
                return;
            }
            close_cancel(event, window, cx);
        },
    )
}

/// `UnsavedChangesDialog` mounted as the close-the-program prompt.
///
/// Subtitle is `You have unsaved edits in this scene.` or `This scene is
/// already saved.`; the primary action is `Save & Close`.
pub fn close_unsaved_changes_modal(
    has_unsaved_changes: bool,
    busy: bool,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_discard: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_save: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let subtitle: SharedString = if has_unsaved_changes {
        "You have unsaved edits in this scene.".into()
    } else {
        "This scene is already saved.".into()
    };
    let body = div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(muted_small_paragraph(if has_unsaved_changes {
            "You’re about to close DragonFruit with unsaved scene changes."
        } else {
            "Close DragonFruit now?"
        }))
        .child(
            div()
                .text_size(px(12.0))
                .line_height(px(12.0 * LEADING_RELAXED))
                .text_color(rgb(TEXT_MUTED))
                .font_weight(FontWeight::SEMIBOLD)
                .child("Please ensure you have saved any important work."),
        );
    unsaved_changes_dialog(
        "close-unsaved-changes",
        subtitle,
        body,
        "Save & Close".into(),
        busy,
        on_cancel,
        on_discard,
        on_save,
    )
}

/// `UnsavedChangesDialog` mounted as the new-scene prompt.
///
/// Same shell; the primary action is `Save & New Scene`.
pub fn new_scene_unsaved_changes_modal(
    has_unsaved_changes: bool,
    busy: bool,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_discard: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_save: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let subtitle: SharedString = if has_unsaved_changes {
        "You have unsaved edits in this scene.".into()
    } else {
        "This scene is already saved.".into()
    };
    let body = div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(muted_small_paragraph(if has_unsaved_changes {
            "You’re about to start a new scene with unsaved edits in this one."
        } else {
            "Start a new scene now?"
        }))
        .child(
            div()
                .text_size(px(12.0))
                .line_height(px(12.0 * LEADING_RELAXED))
                .text_color(rgb(TEXT_MUTED))
                .font_weight(FontWeight::SEMIBOLD)
                .child("Please ensure you have saved any important work."),
        );
    unsaved_changes_dialog(
        "new-scene-unsaved-changes",
        subtitle,
        body,
        "Save & New Scene".into(),
        busy,
        on_cancel,
        on_discard,
        on_save,
    )
}

