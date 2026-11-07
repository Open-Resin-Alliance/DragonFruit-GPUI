use gpui::{
    App, ClickEvent, Div, FontWeight, IntoElement, SharedString, WeakEntity, Window, div,
    prelude::*, px, rgb,
};

use gpui_core::support_state::*;
use gpui_core::theme::*;
use gpui_ui::panel_fields::format_number;
use gpui_ui::ui::{
    self, ButtonSize, ButtonSpec, ButtonVariant, CheckboxLabel, IconTone, dialog_modal,
};

use super::shared::{clamp_float, notify};
use crate::DragonFruit;

/// `StructuredDialogModal` for the island scan settings: Display, Scan, Filters
/// and Clustering, with Cancel and Apply.
///
/// PORT GAP: no portal — the shell mounts this at window level.
pub fn island_scan_settings_dialog(
    state: &IslandsPanelState,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let model = state.model();
    let draft = model.draft;
    let advanced_mode = model.advanced_mode;
    let show_already_supported = model.show_already_supported;
    let show_plate_contact = model.show_plate_contact;
    drop(model);

    // Display — `Advanced mode`.
    let display_card = ui::sub_card_with(None, 10.0)
        .child(section_header("Display"))
        .child(ui::checkbox(
            "islands-advanced-mode",
            "Advanced mode",
            advanced_mode,
            false,
            CheckboxLabel::Field,
            {
                let state = state.clone();
                let entity = entity.clone();
                move |next, _, cx| {
                    state.model_mut().advanced_mode = next;
                    notify(&entity, cx);
                }
            },
        ))
        .child(
            div()
                .mt(px(4.0))
                .text_size(px(10.0))
                .text_color(rgb(TEXT_MUTED))
                .child("Shows the Voxels, Minima, Coincident, and Overhangs layer toggles."),
        );

    // Scan — Resolution and Support buffer. The source's two `input[type=range]`
    // sliders become stepper fields at the same bounds.
    let resolution_down_state = state.clone();
    let resolution_down_entity = entity.clone();
    let resolution_up_state = state.clone();
    let resolution_up_entity = entity.clone();
    let resolution = draft_field_row(
        "Resolution",
        format!("{} mm/px", format_number(draft.px_mm)),
        ui::scrollable_number_field(
            "islands-draft-px-mm",
            format_number(draft.px_mm),
            None,
            false,
            false,
            draft.px_mm < 0.5,
            draft.px_mm > 0.03,
            move |_, _, cx| {
                resolution_down_state.model_mut().draft.px_mm = (draft.px_mm - 0.01).max(0.03);
                notify(&resolution_down_entity, cx);
            },
            move |_, _, cx| {
                resolution_up_state.model_mut().draft.px_mm = (draft.px_mm + 0.01).min(0.5);
                notify(&resolution_up_entity, cx);
            },
        ),
    );
    let support_buffer_down_state = state.clone();
    let support_buffer_down_entity = entity.clone();
    let support_buffer_up_state = state.clone();
    let support_buffer_up_entity = entity.clone();
    let support_buffer = draft_field_row(
        "Support buffer",
        format!("{} mm", format_number(draft.support_buf_mm)),
        ui::scrollable_number_field(
            "islands-draft-support-buf",
            format_number(draft.support_buf_mm),
            None,
            false,
            false,
            draft.support_buf_mm < 1.0,
            draft.support_buf_mm > 0.0,
            move |_, _, cx| {
                support_buffer_down_state.model_mut().draft.support_buf_mm =
                    (draft.support_buf_mm - 0.05).max(0.0);
                notify(&support_buffer_down_entity, cx);
            },
            move |_, _, cx| {
                support_buffer_up_state.model_mut().draft.support_buf_mm =
                    (draft.support_buf_mm + 0.05).min(1.0);
                notify(&support_buffer_up_entity, cx);
            },
        ),
    );
    let scan_card = ui::sub_card_with(None, 10.0)
        .child(section_header("Scan"))
        .child(div().flex().flex_col().gap(px(12.0)).child(resolution).child(support_buffer));

    // Filters — the two `filterToggles`.
    let filters_card = ui::sub_card_with(None, 10.0)
        .child(section_header("Filters"))
        .child(div().flex().flex_col().gap(px(12.0)).child(ui::checkbox(
            "islands-show-supported",
            "Show already-supported islands",
            show_already_supported,
            false,
            CheckboxLabel::Field,
            {
                let state = state.clone();
                let entity = entity.clone();
                move |next, _, cx| {
                    state.model_mut().show_already_supported = next;
                    notify(&entity, cx);
                }
            },
        ))
        .child(ui::checkbox(
            "islands-show-plate-contact",
            "Show plate-contact islands",
            show_plate_contact,
            false,
            CheckboxLabel::Field,
            {
                let state = state.clone();
                let entity = entity.clone();
                move |next, _, cx| {
                    state.model_mut().show_plate_contact = next;
                    notify(&entity, cx);
                }
            },
        )));

    // Clustering — scale markers, consolidate (indented), contoured regions,
    // remove supported clusters, reduce intersections.
    let consolidate_enabled = draft.scale_markers_with_area;
    let clustering_card = ui::sub_card_with(None, 10.0)
        .child(section_header("Clustering"))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(ui::checkbox(
                    "islands-scale-markers",
                    "Scale markers with area",
                    draft.scale_markers_with_area,
                    false,
                    CheckboxLabel::Field,
                    {
                        let state = state.clone();
                        let entity = entity.clone();
                        move |next, _, cx| {
                            state.model_mut().draft.scale_markers_with_area = next;
                            notify(&entity, cx);
                        }
                    },
                ))
                .child(
                    div()
                        .pl(px(20.0))
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child(ui::checkbox(
                            "islands-consolidate",
                            "Consolidate",
                            draft.consolidate_voxel,
                            !consolidate_enabled,
                            CheckboxLabel::Field,
                            {
                                let state = state.clone();
                                let entity = entity.clone();
                                move |next, _, cx| {
                                    state.model_mut().draft.consolidate_voxel = next;
                                    notify(&entity, cx);
                                }
                            },
                        ))
                        .when(consolidate_enabled && draft.consolidate_voxel, |this| {
                            this.child(unit_field(
                                "islands-consolidation-distance",
                                draft.consolidation_distance,
                                "mm",
                                0.1,
                                0.1,
                                5.0,
                                false,
                                {
                                    let state = state.clone();
                                    let entity = entity.clone();
                                    move |next, _, cx| {
                                        state.model_mut().draft.consolidation_distance =
                                            clamp_float(next, 0.1, 5.0, 1);
                                        notify(&entity, cx);
                                    }
                                },
                            ))
                        }),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child(ui::checkbox(
                            "islands-contour-regions",
                            "Contoured regions",
                            draft.enable_contour_regions,
                            false,
                            CheckboxLabel::Field,
                            {
                                let state = state.clone();
                                let entity = entity.clone();
                                move |next, _, cx| {
                                    state.model_mut().draft.enable_contour_regions = next;
                                    notify(&entity, cx);
                                }
                            },
                        ))
                        .when(draft.enable_contour_regions, |this| {
                            this.child(unit_field(
                                "islands-max-contour-regions",
                                draft.max_contour_regions,
                                "max",
                                1.0,
                                1.0,
                                50.0,
                                false,
                                {
                                    let state = state.clone();
                                    let entity = entity.clone();
                                    move |next, _, cx| {
                                        state.model_mut().draft.max_contour_regions =
                                            clamp_float(next, 1.0, 50.0, 0);
                                        notify(&entity, cx);
                                    }
                                },
                            ))
                        }),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child(ui::checkbox(
                            "islands-remove-supported",
                            "Remove supported clusters",
                            draft.remove_supported_area_clusters,
                            false,
                            CheckboxLabel::Field,
                            {
                                let state = state.clone();
                                let entity = entity.clone();
                                move |next, _, cx| {
                                    state.model_mut().draft.remove_supported_area_clusters = next;
                                    notify(&entity, cx);
                                }
                            },
                        ))
                        .when(draft.remove_supported_area_clusters, |this| {
                            this.child(unit_field(
                                "islands-area-per-support",
                                draft.area_per_support,
                                "mm²",
                                0.5,
                                1.0,
                                10.0,
                                false,
                                {
                                    let state = state.clone();
                                    let entity = entity.clone();
                                    move |next, _, cx| {
                                        state.model_mut().draft.area_per_support =
                                            clamp_float(next, 1.0, 10.0, 1);
                                        notify(&entity, cx);
                                    }
                                },
                            ))
                        }),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child(ui::checkbox(
                            "islands-reduce-intersections",
                            "Reduce intersections",
                            draft.reduce_intersection,
                            false,
                            CheckboxLabel::Field,
                            {
                                let state = state.clone();
                                let entity = entity.clone();
                                move |next, _, cx| {
                                    state.model_mut().draft.reduce_intersection = next;
                                    notify(&entity, cx);
                                }
                            },
                        ))
                        .when(draft.reduce_intersection, |this| {
                            this.child(unit_field(
                                "islands-intersection-threshold",
                                draft.intersection_threshold,
                                "mm²",
                                0.1,
                                0.1,
                                2.0,
                                false,
                                {
                                    let state = state.clone();
                                    let entity = entity.clone();
                                    move |next, _, cx| {
                                        state.model_mut().draft.intersection_threshold =
                                            clamp_float(next, 0.1, 2.0, 1);
                                        notify(&entity, cx);
                                    }
                                },
                            ))
                        }),
                ),
        );

    // `Reset defaults`: secondary, `w-full !h-8 px-3 text-xs`, 12px RotateCcw.
    let reset_state = state.clone();
    let reset_entity = entity.clone();
    let reset_spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Sm)
        .full()
        .gap(6.0)
        .icon("rotate-ccw", 12.0, TEXT_STRONG);
    let reset_button =
        ui::button_with("islands-reset-defaults", "Reset defaults", &reset_spec).on_click(
            move |_, _, cx| {
                reset_state.model_mut().draft = IslandScanDraft::default();
                notify(&reset_entity, cx);
            },
        );

    let body = div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(display_card)
        .child(scan_card)
        .child(filters_card)
        .child(clustering_card)
        .child(reset_button);

    // Actions: Cancel (secondary md) and Apply (tinted-accent md).
    let cancel_state = state.clone();
    let cancel_entity = entity.clone();
    let cancel = div().flex_1().child(
        ui::button_with(
            "islands-settings-cancel",
            "Cancel",
            &ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Md).full(),
        )
        .on_click(move |_, _, cx| {
            cancel_state.model_mut().settings_open = false;
            notify(&cancel_entity, cx);
        }),
    );
    let apply_state = state.clone();
    let apply_entity = entity.clone();
    let apply = div().flex_1().child(
        ui::button_with(
            "islands-settings-apply",
            "Apply",
            &ButtonSpec::new(ButtonVariant::TintedAccent, ButtonSize::Md).full(),
        )
        .on_click(move |_, _, cx| {
            let mut model = apply_state.model_mut();
            model.applied = model.draft;
            model.settings_open = false;
            drop(model);
            notify(&apply_entity, cx);
        }),
    );

    let close_state = state.clone();
    let close_entity = entity.clone();
    dialog_modal(
        512.0,
        "Scan Settings",
        Some("Configure island detection parameters".into()),
        None,
        IconTone::Neutral,
        body,
        div().flex().gap(px(8.0)).child(cancel).child(apply),
        move |_, _, cx| {
            close_state.model_mut().settings_open = false;
            notify(&close_entity, cx);
        },
    )
}

/// `SectionHeader`: `pb-1.5 text-[10px] font-semibold uppercase tracking-wide
/// text-center` in `--text-strong`.
fn section_header(title: &'static str) -> Div {
    div()
        .pb(px(6.0))
        .text_center()
        .text_size(px(10.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT_STRONG))
        .child(title.to_uppercase())
}

/// A scan-dialog field: the 10px uppercase name and its 11px value over a
/// control (the source's `flex items-center justify-between mb-1` header).
fn draft_field_row(label: &'static str, value: String, control: impl IntoElement) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(10.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_MUTED))
                        .child(label.to_uppercase()),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child(value),
                ),
        )
        .child(control)
}

/// A `NumberInput` at `h-8 w-full px-2.5 text-center no-spinners` with the unit
/// at `right-2`: the clustering card's four value boxes.
fn unit_field(
    id: impl Into<SharedString>,
    value: f32,
    unit: &'static str,
    step: f32,
    min: f32,
    max: f32,
    disabled: bool,
    on_change: impl Fn(f32, &mut Window, &mut App) + Clone + 'static,
) -> impl IntoElement {
    let id: SharedString = id.into();
    let up = on_change.clone();
    let down = on_change;
    let can_up = !disabled && value < max;
    let can_down = !disabled && value > min;
    ui::mini_stepper_field(
        id,
        format_number(value),
        can_up,
        can_down,
        disabled,
        move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
            up((value + step).min(max), window, cx)
        },
        move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
            down((value - step).max(min), window, cx)
        },
    )
    .w_full()
    // The source's unit span: `absolute right-2 top-1/2 -translate-y-1/2
    // text-[10px] font-semibold` in `--text-muted`. The stepper overlay owns
    // `right-0.5`, so the unit sits at its left edge.
    .child(
        div()
            .absolute()
            .right(px(24.0))
            .top(px(0.0))
            .bottom(px(0.0))
            .flex()
            .items_center()
            .text_size(px(10.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(TEXT_MUTED))
            .child(unit),
    )
}
