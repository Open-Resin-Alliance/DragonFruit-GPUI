use gpui::{FontWeight, IntoElement, WeakEntity, div, prelude::*, px, rgb};

use gpui_core::icons::icon;
use gpui_core::support_state::{MAX_INFILL_DIAMETER_MM, MIN_INFILL_DIAMETER_MM, *};
use gpui_core::theme::*;
use gpui_ui::panel_fields::format_number;
use gpui_ui::ui::{
    self, ButtonSize, ButtonSpec, ButtonVariant, SelectOption, select_dropdown, spinner_colored,
};

use super::shared::{
    accent_secondary_action_border, clamp_float, labeled_section_card, mode_pill, notify,
    panel_body, ui_meta,
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
        ui::sub_card_with(Some((ACCENT_SECONDARY, 0.08)), 8.0)
            .border_color(rgb(accent_secondary_action_border()))
            .min_h(px(72.0))
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
        ui::sub_card_with(Some((ACCENT, 0.05)), 8.0)
            .min_h(px(72.0))
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
    let voxel_down_state = state.clone();
    let voxel_down_entity = entity.clone();
    let voxel_up_state = state.clone();
    let voxel_up_entity = entity.clone();
    let voxel_card = labeled_section_card("Voxel Size").child(ui::scrollable_number_field(
        "hollow-voxel-size",
        format_number(voxel_size_mm),
        Some("mm"),
        false,
        is_applying,
        voxel_size_mm < 10.0,
        voxel_size_mm > 0.2,
        move |_, _, cx| {
            voxel_down_state.model_mut().voxel_size_mm =
                clamp_float((voxel_size_mm - 0.1).max(0.2), 0.2, 10.0, 2);
            notify(&voxel_down_entity, cx);
        },
        move |_, _, cx| {
            voxel_up_state.model_mut().voxel_size_mm =
                clamp_float((voxel_size_mm + 0.1).min(10.0), 0.2, 10.0, 2);
            notify(&voxel_up_entity, cx);
        },
    ));

    // Shell Thickness: `clampFloat(value, 0.2, 10, 1)`.
    let shell_down_state = state.clone();
    let shell_down_entity = entity.clone();
    let shell_up_state = state.clone();
    let shell_up_entity = entity.clone();
    let shell_card = labeled_section_card("Shell Thickness").child(ui::scrollable_number_field(
        "hollow-shell-thickness",
        format_number(shell_thickness_mm),
        Some("mm"),
        false,
        is_applying,
        shell_thickness_mm < 10.0,
        shell_thickness_mm > 0.2,
        move |_, _, cx| {
            shell_down_state.model_mut().shell_thickness_mm =
                clamp_float((shell_thickness_mm - 0.1).max(0.2), 0.2, 10.0, 1);
            notify(&shell_down_entity, cx);
        },
        move |_, _, cx| {
            shell_up_state.model_mut().shell_thickness_mm =
                clamp_float((shell_thickness_mm + 0.1).min(10.0), 0.2, 10.0, 1);
            notify(&shell_up_entity, cx);
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
    let density_down_state = state.clone();
    let density_down_entity = entity.clone();
    let density_up_state = state.clone();
    let density_up_entity = entity.clone();
    let density_card = labeled_section_card(density_label).child(ui::scrollable_number_field(
        "hollow-infill-density",
        format_number(infill_density_pct),
        Some("%"),
        false,
        is_applying,
        infill_density_pct < 100.0,
        infill_density_pct > 1.0,
        move |_, _, cx| {
            density_down_state.model_mut().infill_cell_mm =
                HollowingModel::density_pct_to_cell_mm((infill_density_pct - 1.0).max(1.0));
            notify(&density_down_entity, cx);
        },
        move |_, _, cx| {
            density_up_state.model_mut().infill_cell_mm =
                HollowingModel::density_pct_to_cell_mm((infill_density_pct + 1.0).min(100.0));
            notify(&density_up_entity, cx);
        },
    ));

    let diameter_down_state = state.clone();
    let diameter_down_entity = entity.clone();
    let diameter_up_state = state.clone();
    let diameter_up_entity = entity.clone();
    let diameter_card = labeled_section_card("Infill Diameter").child(ui::scrollable_number_field(
        "hollow-infill-diameter",
        format_number(infill_diameter_mm),
        Some("mm"),
        false,
        is_applying,
        infill_diameter_mm < MAX_INFILL_DIAMETER_MM,
        infill_diameter_mm > MIN_INFILL_DIAMETER_MM,
        move |_, _, cx| {
            diameter_down_state.model_mut().infill_beam_radius_mm = clamp_float(
                (infill_diameter_mm - 0.1).max(MIN_INFILL_DIAMETER_MM) / 2.0,
                MIN_INFILL_DIAMETER_MM / 2.0,
                MAX_INFILL_DIAMETER_MM / 2.0,
                2,
            );
            notify(&diameter_down_entity, cx);
        },
        move |_, _, cx| {
            diameter_up_state.model_mut().infill_beam_radius_mm = clamp_float(
                (infill_diameter_mm + 0.1).min(MAX_INFILL_DIAMETER_MM) / 2.0,
                MIN_INFILL_DIAMETER_MM / 2.0,
                MAX_INFILL_DIAMETER_MM / 2.0,
                2,
            );
            notify(&diameter_up_entity, cx);
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
        let clear_enabled = !(is_applying || is_previewing);
        let done_enabled = !(is_applying || is_previewing || is_applying_blockers);
        let mut clear_spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Sm).full();
        if !clear_enabled {
            clear_spec = clear_spec.disabled();
        }
        let mut done_spec = ButtonSpec::new(ButtonVariant::Accent, ButtonSize::Sm).full();
        if !done_enabled {
            done_spec = done_spec.disabled();
        }
        let mut clear = ui::button_with("hollow-edit-clear", "Clear", &clear_spec);
        if clear_enabled {
            clear = clear.on_click(move |_, _, cx| {
                // PORT GAP: no voxel-edit engine — the flags are the port.
                clear_state.model_mut().is_edit_mode = false;
                notify(&clear_entity, cx);
            });
        }
        let mut done = ui::button_with_content(
            "hollow-edit-done",
            &done_spec,
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
        );
        if done_enabled {
            done = done.on_click(move |_, _, cx| {
                let mut model = done_state.model_mut();
                model.is_edit_mode = false;
                model.is_hollowing_applied = true;
                drop(model);
                notify(&done_entity, cx);
            });
        }
        div()
            .flex()
            .gap(px(8.0))
            .child(div().flex_1().child(clear))
            .child(div().flex_1().child(done))
    } else if is_hollowing_applied {
        let reset_state = state.clone();
        let reset_entity = entity.clone();
        let remove_state = state.clone();
        let remove_entity = entity.clone();
        let reset_enabled = !(is_applying || is_previewing);
        let remove_enabled = !(is_applying || is_previewing);
        let mut reset_spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Sm).full();
        if !reset_enabled {
            reset_spec = reset_spec.disabled();
        }
        let mut remove_spec = ButtonSpec::new(ButtonVariant::Accent, ButtonSize::Sm).full();
        if !remove_enabled {
            remove_spec = remove_spec.disabled();
        }
        let mut reset = ui::button_with("hollow-reset-settings", "Reset", &reset_spec);
        if reset_enabled {
            reset = reset.on_click(move |_, _, cx| {
                reset_state.model_mut().reset_settings();
                notify(&reset_entity, cx);
            });
        }
        let mut remove = ui::button_with("hollow-remove", "Remove Hollowing", &remove_spec);
        if remove_enabled {
            remove = remove.on_click(move |_, _, cx| {
                let mut model = remove_state.model_mut();
                model.is_hollowing_applied = false;
                drop(model);
                notify(&remove_entity, cx);
            });
        }
        div()
            .flex()
            .gap(px(8.0))
            .when(can_apply, |this| this.child(div().flex_1().child(reset)))
            .child(div().flex_1().child(remove))
    } else {
        let edit_state = state.clone();
        let edit_entity = entity.clone();
        let apply_state = state.clone();
        let apply_entity = entity.clone();
        let edit_enabled = !(is_applying || is_previewing || !can_edit);
        let apply_enabled = !(is_applying || is_previewing || !can_apply);
        let mut edit_spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Sm).full();
        if !edit_enabled {
            edit_spec = edit_spec.disabled();
        }
        let mut apply_spec = ButtonSpec::new(ButtonVariant::Accent, ButtonSize::Sm).full();
        if !apply_enabled {
            apply_spec = apply_spec.disabled();
        }
        let mut edit = ui::button_with("hollow-solid-areas", "Solid Areas", &edit_spec);
        if edit_enabled {
            edit = edit.on_click(move |_, _, cx| {
                edit_state.model_mut().is_edit_mode = true;
                notify(&edit_entity, cx);
            });
        }
        let mut apply = ui::button_with_content(
            "hollow-apply",
            &apply_spec,
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
        );
        if apply_enabled {
            apply = apply.on_click(move |_, _, cx| {
                // PORT GAP: no hollowing engine — the flag is the port.
                let mut model = apply_state.model_mut();
                model.is_hollowing_applied = true;
                drop(model);
                notify(&apply_entity, cx);
            });
        }
        div()
            .flex()
            .gap(px(8.0))
            .child(div().flex_1().child(edit))
            .child(div().flex_1().child(apply))
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
