use gpui::{FontWeight, IntoElement, WeakEntity, div, prelude::*, px, rgb};

use gpui_core::icons::icon;
use gpui_core::support_state::{MAX_INFILL_DIAMETER_MM, MIN_INFILL_DIAMETER_MM, *};
use gpui_core::theme::*;
use gpui_ui::panel_fields::{BLACK, WHITE};
use gpui_ui::ui::{SelectOption, select_dropdown, spinner_colored};

use super::shared::{
    accent_card, accent_secondary_action_bg, accent_secondary_action_border, clamp_float,
    flat_button, labeled_section_card, mode_pill, notify, panel_body, scroll_field, ui_meta,
};
use crate::DragonFruit;

// ---------------------------------------------------------------------------
// Hollowing — `HollowingPanel.tsx`
// ---------------------------------------------------------------------------
/// voxel-edit Done/Clear only flip the panel's own flags.
pub fn hollowing_body(
    state: &HollowingState,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let model = state.model();
    let mode = model.mode;
    let voxel_size_mm = model.voxel_size_mm;
    let shell_thickness_mm = model.shell_thickness_mm;
    let infill_mode = model.infill_mode;
    let infill_density_pct = model.infill_density_pct();
    let infill_diameter_mm = model.infill_diameter_mm();
    let open_face = model.open_face;
    let open_face_menu_open = model.open_face_menu_open;
    let is_applying = model.is_applying;
    let is_previewing = model.is_previewing;
    let is_applying_blockers = model.is_applying_blockers;
    let can_apply = model.can_apply;
    let can_edit = model.can_edit;
    let is_edit_mode = model.is_edit_mode;
    let is_hollowing_applied = model.is_hollowing_applied;
    let shell_face_selection_pending = model.shell_face_selection_pending;
    drop(model);

    // Header right slot: the `rounded-full` mode badge with a 12px Droplets.
    let badge = div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .rounded_full()
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .px(px(8.0))
        .py(px(2.0))
        .child(icon("droplets", 12.0, ACCENT))
        .child(
            div()
                .text_size(px(10.0))
                .text_color(rgb(TEXT_MUTED))
                .child(match mode {
                    HollowMode::Infill => "Infill",
                    HollowMode::Cavity | HollowMode::ShellOpenFace => "Cavity",
                }),
        );

    // Mode card (or the voxel-edit hint): `rounded-md border p-2 space-y-1.5
    // min-h-[4.5rem] box-border`.
    let mode_card = if is_edit_mode {
        div()
            .rounded(px(ROUNDED_MD))
            .border_1()
            .border_color(rgb(accent_secondary_action_border()))
            .bg(rgb(accent_secondary_action_bg()))
            .p(px(8.0))
            .min_h(px(72.0))
            .flex()
            .flex_col()
            .gap(px(6.0))
            .text_center()
            .child(
                div()
                    .text_size(px(FONT_SM))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(ACCENT_SECONDARY))
                    .child("Solid Areas"),
            )
            .child(
                div()
                    .text_size(px(10.0))
                    .line_height(px(10.0 * 1.375))
                    .text_color(rgb(TEXT_MUTED))
                    .child("Click or lasso to keep voxels solid, Alt+Lasso to release. Clear to reset all, Done to apply."),
            )
    } else {
        let cavity_state = state.clone();
        let cavity_entity = entity.clone();
        let infill_state = state.clone();
        let infill_entity = entity.clone();
        accent_card()
            .min_h(px(72.0))
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(ui_meta("Mode"))
            .child(
                div()
                    .flex()
                    .gap(px(4.0))
                    .child(div().flex_1().child(mode_pill(
                        "hollow-mode-cavity",
                        "Cavity",
                        mode == HollowMode::Cavity,
                        !is_applying,
                        move |_, _, cx| {
                            cavity_state.model_mut().mode = HollowMode::Cavity;
                            notify(&cavity_entity, cx);
                        },
                    )))
                    .child(div().flex_1().child(mode_pill(
                        "hollow-mode-infill",
                        "Infill",
                        mode == HollowMode::Infill,
                        !is_applying,
                        move |_, _, cx| {
                            infill_state.model_mut().mode = HollowMode::Infill;
                            notify(&infill_entity, cx);
                        },
                    ))),
            )
    };

    // Voxel Size: `clampFloat(value, 0.2, 10, 2)`.
    let voxel_card = labeled_section_card("Voxel Size")
        .child(scroll_field(
            "hollow-voxel-size",
            voxel_size_mm,
            Some("mm"),
            0.1,
            0.2,
            10.0,
            is_applying,
            {
                let state = state.clone();
                let entity = entity.clone();
                move |next, _, cx| {
                    state.model_mut().voxel_size_mm = clamp_float(next, 0.2, 10.0, 2);
                    notify(&entity, cx);
                }
            },
        ));

    // Shell Thickness: `clampFloat(value, 0.2, 10, 1)`.
    let shell_card = labeled_section_card("Shell Thickness")
        .child(scroll_field(
            "hollow-shell-thickness",
            shell_thickness_mm,
            Some("mm"),
            0.1,
            0.2,
            10.0,
            is_applying,
            {
                let state = state.clone();
                let entity = entity.clone();
                move |next, _, cx| {
                    state.model_mut().shell_thickness_mm = clamp_float(next, 0.2, 10.0, 1);
                    notify(&entity, cx);
                }
            },
        ));

    // Infill Type: Lattice / Pillar.
    let lattice_state = state.clone();
    let lattice_entity = entity.clone();
    let pillar_state = state.clone();
    let pillar_entity = entity.clone();
    let infill_type_card = labeled_section_card("Infill Type")
        .child(
            div()
                .flex()
                .gap(px(4.0))
                .child(div().flex_1().child(mode_pill(
                    "hollow-infill-lattice",
                    "Lattice",
                    infill_mode == InfillMode::Lattice,
                    !is_applying,
                    move |_, _, cx| {
                        lattice_state.model_mut().infill_mode = InfillMode::Lattice;
                        notify(&lattice_entity, cx);
                    },
                )))
                .child(div().flex_1().child(mode_pill(
                    "hollow-infill-pillar",
                    "Pillar",
                    infill_mode == InfillMode::Pillar,
                    !is_applying,
                    move |_, _, cx| {
                        pillar_state.model_mut().infill_mode = InfillMode::Pillar;
                        notify(&pillar_entity, cx);
                    },
                ))),
        );

    let density_label = if infill_mode == InfillMode::Pillar {
        "Pillar Spacing"
    } else {
        "Infill Density"
    };
    let density_card = labeled_section_card(density_label)
        .child(scroll_field(
            "hollow-infill-density",
            infill_density_pct,
            Some("%"),
            1.0,
            1.0,
            100.0,
            is_applying,
            {
                let state = state.clone();
                let entity = entity.clone();
                move |next, _, cx| {
                    state.model_mut().infill_cell_mm = HollowingModel::density_pct_to_cell_mm(next);
                    notify(&entity, cx);
                }
            },
        ));

    let diameter_card = labeled_section_card("Infill Diameter")
        .child(scroll_field(
            "hollow-infill-diameter",
            infill_diameter_mm,
            Some("mm"),
            0.1,
            MIN_INFILL_DIAMETER_MM,
            MAX_INFILL_DIAMETER_MM,
            is_applying,
            {
                let state = state.clone();
                let entity = entity.clone();
                move |next, _, cx| {
                    state.model_mut().infill_beam_radius_mm = clamp_float(
                        next / 2.0,
                        MIN_INFILL_DIAMETER_MM / 2.0,
                        MAX_INFILL_DIAMETER_MM / 2.0,
                        2,
                    );
                    notify(&entity, cx);
                }
            },
        ));

    // Open Face — `Select className="mt-1 w-full !h-8 px-2 text-xs"`.
    let face_options: Vec<SelectOption> = [
        OpenFace::XMin,
        OpenFace::XMax,
        OpenFace::YMin,
        OpenFace::YMax,
        OpenFace::ZMin,
        OpenFace::ZMax,
    ]
    .into_iter()
    .map(|face| SelectOption::new(face.value(), face.label()))
    .collect();
    let face_toggle_state = state.clone();
    let face_toggle_entity = entity.clone();
    let face_select_state = state.clone();
    let face_select_entity = entity.clone();
    let open_face_card = labeled_section_card("Open Face")
        .child(select_dropdown(
            "hollow-open-face",
            &face_options,
            open_face.value(),
            open_face_menu_open,
            is_applying,
            move |_, _, cx| {
                let open = face_toggle_state.model().open_face_menu_open;
                face_toggle_state.model_mut().open_face_menu_open = !open;
                notify(&face_toggle_entity, cx);
            },
            move |index, _, cx| {
                let face = [
                    OpenFace::XMin,
                    OpenFace::XMax,
                    OpenFace::YMin,
                    OpenFace::YMax,
                    OpenFace::ZMin,
                    OpenFace::ZMax,
                ]
                .get(index)
                .copied();
                let mut model = face_select_state.model_mut();
                model.open_face_menu_open = false;
                if let Some(face) = face {
                    model.open_face = face;
                }
                drop(model);
                notify(&face_select_entity, cx);
            },
        ))
        .when(shell_face_selection_pending, |this| {
            this.child(
                div()
                    .mt(px(6.0))
                    .text_size(px(11.0))
                    .line_height(px(11.0 * 1.25))
                    .text_color(rgb(TEXT_MUTED))
                    .child("Click the face you want to open in the scene."),
            )
        });

    // Action row.
    let actions = if is_edit_mode {
        let clear_state = state.clone();
        let clear_entity = entity.clone();
        let done_state = state.clone();
        let done_entity = entity.clone();
        div()
            .flex()
            .gap(px(8.0))
            .child(div().flex_1().child(flat_button(
                "hollow-edit-clear",
                "Clear",
                32.0,
                SURFACE_1,
                BORDER_SUBTLE,
                TEXT_STRONG,
                Some(SURFACE_2),
                !(is_applying || is_previewing),
                move |_, _, cx| {
                    // PORT GAP: no voxel-edit engine — the flags are the port.
                    clear_state.model_mut().is_edit_mode = false;
                    notify(&clear_entity, cx);
                },
            )))
            .child(div().flex_1().child(flat_button(
                "hollow-edit-done",
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.0))
                    .when(is_applying_blockers, |this| {
                        this.child(spinner_colored(12.0, ACCENT_SECONDARY_CONTRAST))
                    })
                    .child(if is_applying_blockers {
                        "Applying Solid Areas"
                    } else {
                        "Done"
                    }),
                32.0,
                SECONDARY_BUTTON_SURFACE,
                mix(SECONDARY_BUTTON_SURFACE, WHITE, 0.86),
                ACCENT_SECONDARY_CONTRAST,
                Some(mix(SECONDARY_BUTTON_SURFACE, BLACK, 0.91)),
                !(is_applying || is_previewing || is_applying_blockers),
                move |_, _, cx| {
                    let mut model = done_state.model_mut();
                    model.is_edit_mode = false;
                    model.is_hollowing_applied = true;
                    drop(model);
                    notify(&done_entity, cx);
                },
            )))
    } else if is_hollowing_applied {
        let reset_state = state.clone();
        let reset_entity = entity.clone();
        let remove_state = state.clone();
        let remove_entity = entity.clone();
        div()
            .flex()
            .gap(px(8.0))
            .when(can_apply, |this| {
                this.child(div().flex_1().child(flat_button(
                    "hollow-reset-settings",
                    "Reset",
                    32.0,
                    SURFACE_1,
                    BORDER_SUBTLE,
                    TEXT_STRONG,
                    Some(SURFACE_2),
                    !(is_applying || is_previewing),
                    move |_, _, cx| {
                        reset_state.model_mut().reset_settings();
                        notify(&reset_entity, cx);
                    },
                )))
            })
            .child(div().flex_1().child(flat_button(
                "hollow-remove",
                "Remove Hollowing",
                32.0,
                SECONDARY_BUTTON_SURFACE,
                mix(SECONDARY_BUTTON_SURFACE, WHITE, 0.86),
                ACCENT_SECONDARY_CONTRAST,
                Some(mix(SECONDARY_BUTTON_SURFACE, BLACK, 0.91)),
                !(is_applying || is_previewing),
                move |_, _, cx| {
                    let mut model = remove_state.model_mut();
                    model.is_hollowing_applied = false;
                    drop(model);
                    notify(&remove_entity, cx);
                },
            )))
    } else {
        let edit_state = state.clone();
        let edit_entity = entity.clone();
        let apply_state = state.clone();
        let apply_entity = entity.clone();
        div()
            .flex()
            .gap(px(8.0))
            .child(div().flex_1().child(flat_button(
                "hollow-solid-areas",
                "Solid Areas",
                32.0,
                SURFACE_1,
                BORDER_SUBTLE,
                TEXT_STRONG,
                Some(SURFACE_2),
                !(is_applying || is_previewing || !can_edit),
                move |_, _, cx| {
                    edit_state.model_mut().is_edit_mode = true;
                    notify(&edit_entity, cx);
                },
            )))
            .child(div().flex_1().child(flat_button(
                "hollow-apply",
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.0))
                    .when(is_previewing, |this| {
                        this.child(spinner_colored(12.0, ACCENT_SECONDARY_CONTRAST))
                    })
                    .child(if is_applying {
                        "Applying…".to_owned()
                    } else if is_previewing {
                        "Updating".to_owned()
                    } else {
                        "Apply".to_owned()
                    }),
                32.0,
                SECONDARY_BUTTON_SURFACE,
                mix(SECONDARY_BUTTON_SURFACE, WHITE, 0.86),
                ACCENT_SECONDARY_CONTRAST,
                Some(mix(SECONDARY_BUTTON_SURFACE, BLACK, 0.91)),
                !(is_applying || is_previewing || !can_apply),
                move |_, _, cx| {
                    // PORT GAP: no hollowing engine — the flag is the port.
                    let mut model = apply_state.model_mut();
                    model.is_hollowing_applied = true;
                    drop(model);
                    notify(&apply_entity, cx);
                },
            )))
    };

    // `px-2 pb-2 space-y-2 sm:px-2.5 sm:pb-2.5`.
    panel_body()
        .child(div().flex().justify_end().child(badge))
        .child(mode_card)
        .child(voxel_card)
        .child(shell_card)
        .when(mode == HollowMode::Infill, |this| {
            this.child(infill_type_card)
                .child(density_card)
                .child(diameter_card)
        })
        .when(mode == HollowMode::ShellOpenFace, |this| this.child(open_face_card))
        .child(actions)
}
