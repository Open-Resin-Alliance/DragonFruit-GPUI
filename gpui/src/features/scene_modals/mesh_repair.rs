//! The plate-trash, manifold, remesh, shrink-wrap and mesh-repair dialogs —
//! `ModelManagerPanel.tsx`, `ManifoldWarningModal.tsx`,
//! `TetrahedralRemeshConfirmModal.tsx`, `ShrinkWrapConfirmModal.tsx` and
//! `MeshRepairModals.tsx`.

use std::rc::Rc;

use gpui::{
    App, ClickEvent, Div, ElementId, FontWeight, SharedString, Window, div, prelude::*, px,
    relative, rgb,
};
use gpui_core::icons::icon;
use gpui_core::theme::*;
use gpui_ui::panel_fields::BLACK;
use gpui_ui::ui::{
    ButtonSize, ButtonVariant, IconButtonSize, IconButtonVariant, IconTone, button, dialog_modal,
    dialog_modal_full, icon_button, spinner,
};

use super::chrome::{
    AMBER_600, DIALOG_LG, LEADING_RELAXED, LEADING_SNUG, LEADING_TIGHT, Run, TintedTone,
    action_slot, custom_dialog, identity_card, mix_b, muted_paragraph, muted_small_paragraph,
    rich_paragraph, tinted_action, warning_strip,
};

// ── Plate trash ─────────────────────────────────────────────────────────────

/// `platePendingDelete` — `ModelManagerPanel.tsx`'s `StructuredDialogModal`.
///
/// Title `Delete this plate?`, the plate's name as the subtitle, a `Trash2`
/// danger tile, the delete copy and `Cancel` / `Delete plate`.
pub fn plate_delete_modal(
    plate_name: SharedString,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_delete: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let cancel = Rc::new(on_cancel);
    let close = cancel.clone();
    let action_cancel = cancel.clone();

    dialog_modal(
        DIALOG_LG,
        "Delete this plate?",
        Some(plate_name),
        Some("trash-2"),
        IconTone::Danger,
        muted_paragraph(
            "The models on this plate are deleted with it. Undo brings them back.",
        ),
        div()
            .flex()
            .gap(px(8.0))
            .child(action_slot(
                button(
                    "plate-delete-cancel",
                    "Cancel",
                    ButtonVariant::Secondary,
                    ButtonSize::Md,
                )
                .on_click(move |event, window, cx| action_cancel(event, window, cx)),
            ))
            .child(action_slot(
                button(
                    "plate-delete-confirm",
                    "Delete plate",
                    ButtonVariant::TintedDanger,
                    ButtonSize::Md,
                )
                .on_click(move |event, window, cx| on_delete(event, window, cx)),
            )),
        move |event, window, cx| close(event, window, cx),
    )
}

// ── Manifold / remesh / shrink-wrap ─────────────────────────────────────────

/// `ManifoldWarningModal` — the mesh-issue warning shown when validation fails.
///
/// Title `Mesh Issue detected`, subtitle `This mesh isn't a valid solid`,
/// warning tile, the three explanations, `Dismiss` and `Repair Now` (`Wrench`).
/// The source passes no `onClose` and a no-op `onBackdropClick`, so the close
/// tile is painted but inert; the user leaves through `Dismiss` or `Repair Now`.
pub fn manifold_warning_modal(
    on_dismiss: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_repair_now: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let dismiss = Rc::new(on_dismiss);
    let action_dismiss = dismiss.clone();

    let body = div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(muted_paragraph(
            "Parts of this mesh are not closed — edges are left open, faces cross through each \
             other, or surfaces do not quite meet. Slicing and support placement expect a sealed \
             solid, so defects can reach the print without showing up here first.",
        ))
        .child(muted_paragraph(
            "The red-striped areas in the viewport mark the parts that failed validation.",
        ))
        .child(muted_paragraph(
            "You can dismiss this and keep working, or repair the mesh now.",
        ));

    dialog_modal(
        DIALOG_LG,
        "Mesh Issue detected",
        Some("This mesh isn't a valid solid".into()),
        Some("alert-triangle"),
        IconTone::Warning,
        body,
        div()
            .flex()
            .gap(px(8.0))
            .child(action_slot(
                button(
                    "manifold-dismiss",
                    "Dismiss",
                    ButtonVariant::Secondary,
                    ButtonSize::Md,
                )
                .on_click(move |event, window, cx| action_dismiss(event, window, cx)),
            ))
            .child(action_slot(
                tinted_action(
                    "manifold-repair-now",
                    "Repair Now",
                    Some(("wrench", 14.0)),
                    TintedTone::Accent,
                )
                .on_click(move |event, window, cx| on_repair_now(event, window, cx)),
            )),
        |_, _, _| {},
    )
}

/// `TetrahedralRemeshConfirmModal` — confirms a remesh before it runs.
///
/// Title `Remesh this model?`, subtitle `Rebuilds the surface`, an accent
/// `Pyramid` tile, the warning strip and `Cancel` / `Remesh`. The source has no
/// `onClose` and a no-op backdrop click, so the close tile is inert.
pub fn tetrahedral_remesh_confirm_modal(
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_confirm: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let cancel = Rc::new(on_cancel);
    let action_cancel = cancel.clone();

    dialog_modal(
        DIALOG_LG,
        "Remesh this model?",
        Some("Rebuilds the surface".into()),
        Some("pyramid"),
        IconTone::Accent,
        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .child(muted_paragraph(
                "This rebuilds the model's surface, keeping fine detail.",
            ))
            .child(warning_strip(
                "The rebuilt surface will replace this model in your current scene. Large models \
                 take longer, and some models cannot be rebuilt at all and are left untouched. \
                 You can undo this afterwards.",
            )),
        div()
            .flex()
            .gap(px(8.0))
            .child(action_slot(
                button(
                    "remesh-cancel",
                    "Cancel",
                    ButtonVariant::Secondary,
                    ButtonSize::Md,
                )
                .on_click(move |event, window, cx| action_cancel(event, window, cx)),
            ))
            .child(action_slot(
                tinted_action(
                    "remesh-confirm",
                    "Remesh",
                    Some(("pyramid", 14.0)),
                    TintedTone::Accent,
                )
                .on_click(move |event, window, cx| on_confirm(event, window, cx)),
            )),
        |_, _, _| {},
    )
}

/// `ShrinkWrapConfirmModal` — confirms a shrink-wrap before it runs.
///
/// Title `Shrink-Wrap this model?`, subtitle `Reworks the surface into one
/// solid`, an accent `RefreshCw` tile, the warning strip and `Cancel` /
/// `Shrink-Wrap`. The source has no `onClose` and a no-op backdrop click, so the
/// close tile is inert.
pub fn shrink_wrap_confirm_modal(
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_confirm: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let cancel = Rc::new(on_cancel);
    let action_cancel = cancel.clone();

    dialog_modal(
        DIALOG_LG,
        "Shrink-Wrap this model?",
        Some("Reworks the surface into one solid".into()),
        Some("refresh-cw"),
        IconTone::Accent,
        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .child(muted_paragraph(
                "This merges overlapping shells, closes gaps and drops loose pieces, leaving one \
                 solid.",
            ))
            .child(warning_strip(
                "The rebuilt surface will replace this model in your current scene. A model that \
                 cannot be resolved is left untouched, and you can undo this afterwards.",
            )),
        div()
            .flex()
            .gap(px(8.0))
            .child(action_slot(
                button(
                    "shrink-wrap-cancel",
                    "Cancel",
                    ButtonVariant::Secondary,
                    ButtonSize::Md,
                )
                .on_click(move |event, window, cx| action_cancel(event, window, cx)),
            ))
            .child(action_slot(
                tinted_action(
                    "shrink-wrap-confirm",
                    "Shrink-Wrap",
                    Some(("refresh-cw", 14.0)),
                    TintedTone::Accent,
                )
                .on_click(move |event, window, cx| on_confirm(event, window, cx)),
            )),
        |_, _, _| {},
    )
}

// ── MeshRepairModals ────────────────────────────────────────────────────────

/// `StructuredDialog_damagedModel` — `MeshRepairModals.tsx`.
///
/// Title `Mesh quality too low`, subtitle `Boolean operation requires a
/// manifold mesh`, a danger `AlertTriangle`, `closeDisabled`, and one `Got it`
/// action. The close tile is painted disabled and carries no handler.
pub fn damaged_model_modal(
    on_dismiss: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let dismiss = Rc::new(on_dismiss);
    let action_dismiss = dismiss.clone();

    dialog_modal_full(
        DIALOG_LG,
        "Mesh quality too low",
        Some("Boolean operation requires a manifold mesh".into()),
        Some("alert-triangle"),
        IconTone::Danger,
        true,
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(muted_small_paragraph(
                "This mesh contains too many self-intersecting triangles and non-manifold edges \
                 for boolean operations to succeed. The import repair pass reduced but could not \
                 fully resolve all issues in the geometry.",
            ))
            .child(rich_paragraph(
                12.0,
                LEADING_RELAXED,
                vec![
                    Run::plain(
                        "We recommend repairing the mesh in a dedicated 3D modeling program such \
                         as ",
                    ),
                    Run::strong("Blender"),
                    Run::plain(" or "),
                    Run::strong("Netfabb"),
                    Run::plain(" before importing it into DragonFruit."),
                ],
            )),
        action_slot(
            button(
                "damaged-model-ok",
                "Got it",
                ButtonVariant::TintedAccent,
                ButtonSize::Md,
            )
            .on_click(move |event, window, cx| action_dismiss(event, window, cx)),
        ),
        move |_event, _window, _cx| {},
    )
}

/// The live state of a manual repair run.
pub struct RepairProgressState {
    pub model_name: Option<SharedString>,
    pub model_index: usize,
    pub model_count: usize,
    pub stage: Option<SharedString>,
    /// `state.stages.length` — the number of progress events seen so far.
    pub stage_events: usize,
}

/// `translateRepairStage` from
/// `src/components/scene/repairProgressMessages.ts`.
///
/// The engine reports a language-neutral stage code; only the label is
/// translated, and an unknown code falls back to the raw code rather than
/// rendering blank.
fn translate_repair_stage(stage: Option<&str>) -> String {
    let Some(stage) = stage else {
        return "Starting".into();
    };
    match stage {
        "repair" => "Repairing",
        "rebuild" => "Rebuilding the surface into a solid",
        "orient_components" => "Pointing faces the same way",
        "stitch" => "Joining loose edges",
        "holes_trivial" => "Closing small holes",
        "double_faces" => "Removing doubled faces",
        "orient_holes_stitch" => "Closing gaps",
        "orient_holes_all_stitch" => "Closing the remaining gaps",
        "double_faces_after_split" => "Removing doubled faces again",
        "split_self_intersections" => "Separating surfaces that overlap",
        "split_non_manifold_edges" => "Separating surfaces that share an edge",
        "union_overlapping_parts" => "Merging overlapping parts",
        "filter_components" => "Keeping the largest pieces",
        "remove_inner_faces" => "Removing faces inside the model",
        "noise_shells" => "Discarding scraps",
        "settle" => "Tidying up",
        "prune_and_classify" => "Separating model and supports",
        other => other,
    }
    .to_string()
}

/// `repairProgressFraction` — how full the bar is, 0..1.
///
/// The stage count is unknown in advance, so the fraction is a saturating
/// function of the events seen: it always moves and never claims the work is
/// nearly done.
fn repair_progress_fraction(state: &RepairProgressState) -> f32 {
    let model_fraction = if state.stage.is_some() {
        0.1 + 0.8 * (1.0 - (-(state.stage_events as f32) / 6.0).exp())
    } else {
        0.06
    };
    (state.model_index as f32 + model_fraction) / (state.model_count.max(1) as f32)
}

/// `formatElapsed` — `0:04.2`.
fn format_elapsed(ms: f64) -> String {
    let total_seconds = ms.max(0.0) / 1000.0;
    let minutes = (total_seconds / 60.0).floor();
    let seconds = total_seconds - minutes * 60.0;
    format!("{}:{:04.1}", minutes as u64, seconds)
}

/// `RepairProgressPanel` — the model position, the stage label, the elapsed
/// clock and the amber bar.
fn repair_progress_panel(
    state: &RepairProgressState,
    elapsed_ms: f64,
    cancelling: bool,
) -> Div {
    let stage_label: SharedString = if cancelling {
        "Stopping at the end of this step…".into()
    } else {
        translate_repair_stage(state.stage.as_deref()).into()
    };

    let mut panel = div().flex().flex_col().gap(px(12.0));
    if state.model_count > 1 {
        if let Some(name) = &state.model_name {
            panel = panel.child(
                div()
                    .mx(px(6.0))
                    .truncate()
                    .text_size(px(11.0))
                    .text_color(rgb(TEXT_MUTED))
                    .child(format!(
                        "Model {} of {}: {name}",
                        state.model_index + 1,
                        state.model_count
                    )),
            );
        }
    }
    panel = panel
        .child(
            div()
                .mx(px(6.0))
                .flex()
                .items_baseline()
                .justify_between()
                .gap(px(12.0))
                .child(
                    div()
                        .min_w(px(0.0))
                        .truncate()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(TEXT_STRONG))
                        .child(stage_label),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_size(px(11.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child(format_elapsed(elapsed_ms)),
                ),
        )
        .child(
            div()
                .mx(px(6.0))
                .h(px(6.0))
                .overflow_hidden()
                .rounded_full()
                .bg(rgb(SURFACE_2))
                .child(
                    div()
                        .h_full()
                        .rounded_full()
                        .w(relative(repair_progress_fraction(state)))
                        .bg(rgb(AMBER_600)),
                ),
        );
    panel
}

/// The manual repair sheet — `MeshRepairModals.tsx`'s `manualRepairModelIds`
/// dialog.
///
/// Title is `Repairing Mesh` when the run auto-started, otherwise
/// `Repair Mesh` / `Repair N Meshes`; subtitle `Fixing geometry issues.` or
/// `Fixes geometry issues before you keep working.`; the model card, the
/// progress panel (or the one-line explanation), the amber warning strip and
/// the `Keep Original` / `Stop` / `Repair` footer.
#[allow(clippy::too_many_arguments)]
pub fn repair_mesh_modal(
    model_names: SharedString,
    model_count: usize,
    auto_start: bool,
    repairing: bool,
    cancelling: bool,
    progress: Option<RepairProgressState>,
    elapsed_ms: f64,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_keep_original: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_stop: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_repair: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let title: SharedString = if auto_start {
        "Repairing Mesh".into()
    } else if model_count == 1 {
        "Repair Mesh".into()
    } else {
        format!("Repair {model_count} Meshes").into()
    };
    let subtitle: SharedString = if auto_start {
        "Fixing geometry issues.".into()
    } else {
        "Fixes geometry issues before you keep working.".into()
    };

    let close = Rc::new(on_close);
    let header_close = close.clone();

    // The close tile is disabled mid-repair, so it drops its handler.
    let close_button = if repairing {
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
            ElementId::named_usize("repair-mesh", 0),
            "x",
            IconButtonVariant::Surface,
            IconButtonSize::Md,
        )
        .on_click(move |event, window, cx| header_close(event, window, cx))
        .into_any_element()
    };

    let header = div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.0))
        .border_b(px(1.0))
        .border_color(rgb(BORDER_SUBTLE))
        .px(px(20.0))
        .py(px(16.0))
        .child(
            div()
                .flex()
                .min_w(px(0.0))
                .items_center()
                .gap(px(12.0))
                .child(
                    div()
                        .flex()
                        .flex_shrink_0()
                        .items_center()
                        .justify_center()
                        .w(px(36.0))
                        .h(px(36.0))
                        .rounded(px(ROUNDED_MD))
                        .border_1()
                        .border_color(rgb(mix_b(AMBER_600, BORDER_SUBTLE, 0.45)))
                        .bg(rgb(mix_b(AMBER_600, SURFACE_1, 0.88)))
                        .child(icon("alert-triangle", 16.0, AMBER_600)),
                )
                .child(
                    div()
                        .min_w(px(0.0))
                        .pr(px(8.0))
                        .child(
                            div()
                                .text_size(px(16.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .line_height(px(16.0 * LEADING_TIGHT))
                                .text_color(rgb(TEXT_STRONG))
                                .child(title),
                        )
                        .child(
                            div()
                                .mt(px(2.0))
                                .text_size(px(11.0))
                                .line_height(px(11.0 * LEADING_SNUG))
                                .text_color(rgb(TEXT_MUTED))
                                .child(subtitle),
                        ),
                ),
        )
        .child(close_button);

    let model_card = identity_card(
        if model_count > 1 { "Models" } else { "Model" },
        model_names,
    );

    let middle: Div = match &progress {
        Some(state) => repair_progress_panel(state, elapsed_ms, cancelling),
        None => muted_small_paragraph(
            "Repairs holes, broken surfaces, and other mesh problems.",
        ),
    };

    let warning = warning_strip(if model_count > 1 {
        "The repaired results will replace these models in your current scene. Large or badly \
         damaged meshes can take longer, and some files may still need manual cleanup afterward."
    } else {
        "The repaired result will replace this model in your current scene. Large or badly \
         damaged meshes can take longer, and some files may still need manual cleanup afterward."
    });

    // `Keep Original` / `Stop` / `Stopping…`.
    let keep_label: SharedString = if cancelling {
        "Stopping…".into()
    } else if repairing {
        "Stop".into()
    } else {
        "Keep Original".into()
    };
    let keep = {
        let mut el = div()
            .id("repair-mesh-keep")
            .flex()
            .items_center()
            .justify_center()
            .gap(px(6.0))
            .w_full()
            .h(px(36.0))
            .rounded(px(RADIUS_SM))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1))
            .pt(px(5.0))
            .pb(px(7.0))
            .px(px(11.0))
            .text_size(px(FONT_MD))
            .font_weight(FontWeight::SEMIBOLD)
            .line_height(px(FONT_MD))
            .text_color(rgb(TEXT_STRONG));
        if cancelling {
            el = el
                .opacity(0.6)
                .cursor_not_allowed()
                .child(spinner(14.0))
                .child(keep_label.clone());
        } else {
            el = el
                .cursor_pointer()
                .hover(|style| style.bg(rgb(SURFACE_2)));
            el = if repairing {
                el.on_click(move |event, window, cx| on_stop(event, window, cx))
            } else {
                el.on_click(move |event, window, cx| on_keep_original(event, window, cx))
            };
            el = el.child(keep_label.clone());
        }
        el
    };

    let footer = if auto_start || repairing {
        div().pt(px(4.0)).child(keep)
    } else {
        div()
            .flex()
            .gap(px(8.0))
            .pt(px(4.0))
            .child(div().flex_1().child(keep))
            .child(div().flex_1().child(
                tinted_action(
                    "repair-mesh-repair",
                    "Repair",
                    Some(("wrench", 14.0)),
                    TintedTone::Accent,
                )
                .w_full()
                .on_click(move |event, window, cx| on_repair(event, window, cx)),
            ))
    };

    let body = div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .p(px(20.0))
        .child(model_card)
        .child(middle)
        .child(warning)
        .child(footer);

    custom_dialog(DIALOG_LG, header, body)
}

