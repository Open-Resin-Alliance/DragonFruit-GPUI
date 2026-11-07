use super::*;

// ── Camera ───────────────────────────────────────────────────────────────

pub(crate) fn camera_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let mut root = div().flex().flex_col().gap(px(12.));

    let scope_entity = entity.clone();
    let mut defaults = section(
        "camera",
        "Camera Defaults",
        "Global camera projection mode and navigation behavior.",
    )
    .child(
        setting_row(
            "Camera scope",
            "Choose one global projection mode for every workspace, or set projection defaults per workspace.",
            segmented(
                "settings-camera-scope",
                &["GLOBAL", "WORKSPACE"],
                match state.camera_scope {
                    CameraScope::Global => 0,
                    CameraScope::Workspace => 1,
                },
                false,
                move |index, cx| {
                    edit_and_notify(&scope_entity, cx, |draft| {
                        draft.camera_scope = if index == 0 {
                            CameraScope::Global
                        } else {
                            CameraScope::Workspace
                        };
                    });
                },
            ),
        )
        .mt(px(12.)),
    );

    if state.camera_scope == CameraScope::Global {
        let projection_entity = entity.clone();
        let mut block = div()
            .mt(px(8.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(setting_row(
                "Projection mode",
                "Use one projection mode everywhere when global scope is active.",
                segmented(
                    "settings-projection",
                    &["ORTHO", "PERSPECTIVE"],
                    match state.projection {
                        Projection::Ortho => 0,
                        Projection::Perspective => 1,
                    },
                    false,
                    move |index, cx| {
                        edit_and_notify(&projection_entity, cx, |draft| {
                            draft.projection = if index == 0 {
                                Projection::Ortho
                            } else {
                                Projection::Perspective
                            };
                        });
                    },
                ),
            ));
        if state.projection == Projection::Perspective {
            block = block.child(slider_field(
                "Field of view",
                format!("{}°", state.fov.round() as i32),
                state.fov,
                FOV_MIN,
                FOV_MAX));
        }
        defaults = defaults.child(block);
    }

    if state.camera_scope == CameraScope::Workspace {
        let tab_entity = entity.clone();
        let default_entity = entity.clone();
        let active = state.workspace_tab.min(WORKSPACES.len() - 1);
        let block = div()
            .mt(px(8.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                field_card().child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("Workspace camera defaults"),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Pick the default projection mode used when you enter each workspace."),
                )
                .child(
                    div().mt(px(8.)).child(segmented(
                        "settings-workspace-tab",
                        &["PREPARE", "ANALYSIS", "SUPPORT", "EXPORT"],
                        active,
                        true,
                        move |index, cx| {
                            edit_and_notify(&tab_entity, cx, |draft| draft.workspace_tab = index);
                        },
                    )),
                ),
            )
            .child(setting_row(
                format!("{} default camera", WORKSPACES[active].0),
                WORKSPACES[active].1,
                segmented(
                    "settings-workspace-default",
                    &["ORTHO", "PERSPECTIVE"],
                    match state.workspace_defaults[active] {
                        Projection::Ortho => 0,
                        Projection::Perspective => 1,
                    },
                    false,
                    move |index, cx| {
                        edit_and_notify(&default_entity, cx, |draft| {
                            draft.workspace_defaults[active] = if index == 0 {
                                Projection::Ortho
                            } else {
                                Projection::Perspective
                            };
                        });
                    },
                ),
            ));
        defaults = defaults.child(block);
    }

    let feel_entity = entity.clone();
    defaults = defaults.child(
        setting_row(
            "Camera feel",
            "Controls smoothing and movement acceleration while orbiting, panning, and zooming.",
            select_field(
                "settings-camera-feel",
                &FEEL_OPTIONS,
                FEEL_VALUES[state.camera_feel.min(FEEL_VALUES.len() - 1)],
                None,
                Some(CONTROL_WIDTH),
                true,
                feel_entity.clone(),
                move |value, cx| {
                    if let Some(index) = FEEL_VALUES.iter().position(|feel| *feel == value) {
                        edit_and_notify(&feel_entity, cx, |draft| draft.camera_feel = index);
                    }
                },
            ),
        )
        .mt(px(8.)),
    );
    root = root.child(defaults);

    // Trackpad Navigation
    let action_entity = entity.clone();
    let modifier_entity = entity.clone();
    let mut trackpad = section(
        "hand",
        "Trackpad Navigation",
        "Configure two-finger gestures and alternate modifier behavior.",
    )
    .child(
        div()
            .mt(px(12.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(setting_row(
                "Trackpad navigation mode",
                "What a two-finger drag does. Pinch-to-zoom still works either way.",
                select_field(
                    "settings-trackpad-action",
                    &TRACKPAD_OPTIONS,
                    TRACKPAD_VALUES[state.trackpad_action.min(TRACKPAD_VALUES.len() - 1)],
                    None,
                    Some(CONTROL_WIDTH),
                    true,
                    action_entity.clone(),
                    move |value, cx| {
                        if let Some(index) = TRACKPAD_VALUES.iter().position(|action| *action == value)
                        {
                            edit_and_notify(&action_entity, cx, |draft| {
                                draft.trackpad_action = index;
                            });
                        }
                    },
                ),
            )),
    );

    if state.trackpad_action != 0 {
        let alternate = if state.trackpad_action == 1 { "orbit" } else { "pan" };
        trackpad = trackpad
            .child(setting_row(
                "Alternate gesture modifier",
                format!("Hold this key to temporarily switch two-finger drag to {alternate}."),
                segmented(
                    "settings-trackpad-modifier",
                    &["OPTION", "SHIFT"],
                    state.trackpad_modifier.min(1),
                    false,
                    move |index, cx| {
                        edit_and_notify(&modifier_entity, cx, |draft| {
                            draft.trackpad_modifier = index;
                        });
                    },
                ),
            ))
            .child(
                field_card()
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(TEXT_STRONG))
                            .child("Trackpad acceleration"),
                    )
                    .child(
                        div()
                            .mt(px(2.))
                            .text_size(px(12.))
                            .text_color(rgb(TEXT_MUTED))
                            .child("Tune movement speed for two-finger pan/orbit gestures."),
                    )
                    .child(
                        div()
                            .mt(px(8.))
                            .grid()
                            .grid_cols(2)
                            .gap(px(8.))
                            .child(slider_field(
                                "Pan acceleration",
                                format!("{:.2}×", state.pan_acceleration),
                                state.pan_acceleration,
                                0.4,
                                4.0))
                            .child(slider_field(
                                "Orbit acceleration",
                                format!("{:.2}×", state.orbit_acceleration),
                                state.orbit_acceleration,
                                0.4,
                                4.0))
                            .child(slider_field(
                                "Zoom acceleration",
                                format!("{:.2}×", state.zoom_acceleration),
                                state.zoom_acceleration,
                                0.4,
                                4.0)),
                    ),
            );
    }

    root.child(trackpad)
}
