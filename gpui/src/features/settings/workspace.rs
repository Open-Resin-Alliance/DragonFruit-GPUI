use super::*;

// ── Workspace (`WorkspacesSettingsTab.tsx`) ──────────────────────────────

/// `<label class="text-xs">` with the control under it.
pub(crate) fn number_label(label: &'static str, control: Div) -> Div {
    div()
        .flex()
        .flex_col()
        .text_size(px(12.))
        .text_color(rgb(TEXT_MUTED))
        .child(label)
        .child(div().mt(px(4.)).child(control))
}

pub(crate) fn workspace_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let view = &state.view3d;

    let mut root = div().flex().flex_col().gap(px(12.));
    let enabled_entity = entity.clone();
    let mut inner = div().mt(px(12.)).flex().flex_col();

    // `isBuildVolumeManagedByPrinter` is false here: the slice has no printer
    // profile store, so the source's notice branch never renders.
    inner = inner.child(setting_row(
        "Enable build volume bounds",
        "Shows a faint printer volume outline and enables out-of-bounds checks.",
        segmented(
            "view3d-enabled",
            &["OFF", "ON"],
            usize::from(view.enabled),
            false,
            move |index, cx| {
                edit_and_notify(&enabled_entity, cx, |draft| {
                    draft.view3d.enabled = index == 1;
                });
            },
        ),
    ));

    if view.enabled {
        let width_entity = entity.clone();
        let depth_entity = entity.clone();
        let max_z_entity = entity.clone();
        let origin_entity = entity.clone();
        let screen_w_entity = entity.clone();
        let screen_h_entity = entity.clone();

        let grid = div()
            .mt(px(8.))
            .grid()
            .grid_cols(2)
            .gap(px(8.))
            .child(number_label(
                "Build Width (mm)",
                number_input(
                    "view3d-width",
                    view.width_mm,
                    10.0,
                    f32::INFINITY,
                    1.0,
                    None,
                    36.0, // h-9
                    12.0,
                    SURFACE_1,
                    true,
                    move |next, cx| {
                        edit_and_notify(&width_entity, cx, |draft| {
                            draft.view3d.width_mm = next.max(10.0).round();
                        });
                    },
                ),
            ))
            .child(number_label(
                "Build Depth (mm)",
                number_input(
                    "view3d-depth",
                    view.depth_mm,
                    10.0,
                    f32::INFINITY,
                    1.0,
                    None,
                    36.0,
                    12.0,
                    SURFACE_1,
                    true,
                    move |next, cx| {
                        edit_and_notify(&depth_entity, cx, |draft| {
                            draft.view3d.depth_mm = next.max(10.0).round();
                        });
                    },
                ),
            ))
            .child(number_label(
                "Max Z Height (mm)",
                number_input(
                    "view3d-max-z",
                    view.max_z_mm,
                    10.0,
                    f32::INFINITY,
                    1.0,
                    None,
                    36.0,
                    12.0,
                    SURFACE_1,
                    true,
                    move |next, cx| {
                        edit_and_notify(&max_z_entity, cx, |draft| {
                            draft.view3d.max_z_mm = next.max(10.0).round();
                        });
                    },
                ),
            ))
            .child(
                div()
                    .rounded(px(6.))
                    .border_1()
                    .border_color(rgb(BORDER_SUBTLE))
                    .bg(rgb(SURFACE_1))
                    .px(px(8.))
                    .py(px(6.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(TEXT_MUTED))
                            .child("Build Volume"),
                    )
                    .child(
                        div()
                            .mt(px(2.))
                            .text_size(px(12.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(TEXT_STRONG))
                            .child(format!(
                                "{} × {} × {} mm",
                                view.width_mm.round() as i32,
                                view.depth_mm.round() as i32,
                                view.max_z_mm.round() as i32
                            )),
                    ),
            );

        // The source puts the origin row in the same grid with `col-span-2`;
        // splitting the grid keeps the full-width row without a column span.
        inner = inner.child(grid);
        inner = inner.child(
            setting_row(
                "Build volume origin",
                "Choose where XYZ 0,0,0 is located for the printer volume.",
                segmented(
                    "view3d-origin",
                    &["CENTER", "FRONT-LEFT CORNER"],
                    match view.origin_mode {
                        OriginMode::Center => 0,
                        OriginMode::FrontLeft => 1,
                    },
                    false,
                    move |index, cx| {
                        edit_and_notify(&origin_entity, cx, |draft| {
                            draft.view3d.origin_mode = if index == 0 {
                                OriginMode::Center
                            } else {
                                OriginMode::FrontLeft
                            };
                        });
                    },
                ),
            )
            .mt(px(8.)),
        );
        inner = inner.child(
            div()
                .mt(px(8.))
                .grid()
                .grid_cols(2)
                .gap(px(8.))
                .child(number_label(
                    "Screen Width (px)",
                    number_input(
                        "view3d-screen-w",
                        view.screen_width_px,
                        320.0,
                        f32::INFINITY,
                        1.0,
                        None,
                        36.0,
                        12.0,
                        SURFACE_1,
                        true,
                        move |next, cx| {
                            edit_and_notify(&screen_w_entity, cx, |draft| {
                                draft.view3d.screen_width_px = next.max(320.0).round();
                            });
                        },
                    ),
                ))
                .child(number_label(
                    "Screen Height (px)",
                    number_input(
                        "view3d-screen-h",
                        view.screen_height_px,
                        200.0,
                        f32::INFINITY,
                        1.0,
                        None,
                        36.0,
                        12.0,
                        SURFACE_1,
                        true,
                        move |next, cx| {
                            edit_and_notify(&screen_h_entity, cx, |draft| {
                                draft.view3d.screen_height_px = next.max(200.0).round();
                            });
                        },
                    ),
                )),
        );
    }

    let violation_entity = entity.clone();
    let boxes_entity = entity.clone();
    let sat_entity = entity.clone();
    inner = inner
        .child(
            setting_row(
                "Show out-of-bounds warnings",
                "Warn when any visible model extends beyond the configured build volume.",
                segmented(
                    "view3d-violation",
                    &["OFF", "ON"],
                    usize::from(view.show_violation_warning),
                    false,
                    move |index, cx| {
                        edit_and_notify(&violation_entity, cx, |draft| {
                            draft.view3d.show_violation_warning = index == 1;
                        });
                    },
                ),
            )
            .mt(px(8.)),
        )
        .child(
            setting_row(
                "Show model bounding boxes",
                "Debug overlay: draws world-space bounds for each visible model (red if out-of-bounds).",
                segmented(
                    "view3d-boxes",
                    &["OFF", "ON"],
                    usize::from(view.show_model_bounding_boxes),
                    false,
                    move |index, cx| {
                        edit_and_notify(&boxes_entity, cx, |draft| {
                            draft.view3d.show_model_bounding_boxes = index == 1;
                        });
                    },
                ),
            )
            .mt(px(8.)),
        )
        .child(
            setting_row(
                "Show slice SAT bounding mesh",
                "SAT debug overlay for nesting and diagnostics.",
                segmented(
                    "view3d-sat",
                    &["OFF", "ON"],
                    usize::from(view.show_slice_sat),
                    false,
                    move |index, cx| {
                        edit_and_notify(&sat_entity, cx, |draft| {
                            draft.view3d.show_slice_sat = index == 1;
                        });
                    },
                ),
            )
            .mt(px(8.)),
        );

    if view.show_slice_sat {
        let scope_entity = entity.clone();
        let mode_entity = entity.clone();
        let render_entity = entity.clone();
        inner = inner.child(
            setting_row(
                "SAT debug scope",
                "Show SAT mesh on the active model only, or on all visible models.",
                segmented(
                    "view3d-sat-scope",
                    &["ACTIVE ONLY", "ALL MODELS"],
                    usize::from(view.sat_all_models),
                    false,
                    move |index, cx| {
                        edit_and_notify(&scope_entity, cx, |draft| {
                            draft.view3d.sat_all_models = index == 1;
                        });
                    },
                ),
            )
            .mt(px(8.)),
        );
        inner = inner.child(
            setting_row(
                "SAT mode",
                "Choose accurate convex-hull SAT for nesting, or experimental slice-derived SAT for diagnostics.",
                segmented(
                    "view3d-sat-mode",
                    &["ACCURATE HULL-BASED SAT", "EXPERIMENTAL SLICE-BASED SAT"],
                    match view.sat_mode {
                        SliceSatMode::AccurateHull => 0,
                        SliceSatMode::ExperimentalSlice => 1,
                    },
                    false,
                    move |index, cx| {
                        edit_and_notify(&mode_entity, cx, |draft| {
                            draft.view3d.sat_mode = if index == 0 {
                                SliceSatMode::AccurateHull
                            } else {
                                SliceSatMode::ExperimentalSlice
                            };
                        });
                    },
                ),
            )
            .mt(px(8.)),
        );
        if view.sat_mode == SliceSatMode::ExperimentalSlice {
            inner = inner.child(
                setting_row(
                    "Experimental slice display",
                    "Pick how the experimental slice-derived SAT is visualized.",
                    segmented(
                        "view3d-sat-render",
                        &["SHADED", "WIREFRAME"],
                        match view.sat_render {
                            SliceSatRender::Shaded => 0,
                            SliceSatRender::Wireframe => 1,
                        },
                        false,
                        move |index, cx| {
                            edit_and_notify(&render_entity, cx, |draft| {
                                draft.view3d.sat_render = if index == 0 {
                                    SliceSatRender::Shaded
                                } else {
                                    SliceSatRender::Wireframe
                                };
                            });
                        },
                    ),
                )
                .mt(px(8.)),
            );
        }
    }

    root = root.child(
        section(
            "layers",
            "3D View",
            "Build volume boundaries and display resolution hints used across the workspace.",
        )
        .child(inner),
    );
    root
}
