use super::*;

// ── Mesh ─────────────────────────────────────────────────────────────────

pub(crate) fn mesh_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let mut root = div().flex().flex_col().gap(px(12.));

    let total_light = state.ambient_intensity + state.directional_intensity;
    let lightness = total_light.clamp(0.0, 4.0);
    let contrast = if total_light > 0.0 {
        state.directional_intensity / total_light
    } else {
        0.5
    };

    let show_lighting = state.shader == 0 || state.shader == 4;
    let show_roughness = show_lighting;
    let show_baked_ao = state.shader == 0;
    // `matcap` and `flat_unlit` are not offered by `MESH_SHADER_OPTIONS`.
    let has_rendering_options =
        show_roughness || show_baked_ao || show_lighting || state.shader == 1;

    let shader_entity = entity.clone();
    let model_entity = entity.clone();
    let mut shader_section = section(
        "layers",
        "Shader & Preview",
        "Pick the shader type to configure here, and tune the mesh color. The camera dropdown chooses what the viewport renders.",
    );

    let mut selectors = div().mt(px(12.)).grid().grid_cols(2).gap(px(8.)).child(
        field_card()
            .child(field_label("Shader Type"))
            .child(select_field(
                "settings-shader",
                &SHADER_OPTIONS,
                SHADER_VALUES[state.shader.min(SHADER_VALUES.len() - 1)],
                None,
                Some(CONTROL_WIDTH),
                true,
                shader_entity.clone(),
                move |value, cx| {
                    if let Some(index) = SHADER_VALUES.iter().position(|shader| *shader == value) {
                        edit_and_notify(&shader_entity, cx, |draft| {
                            draft.shader = index;
                            if draft.shader == 1
                                && draft.active_color_index >= draft.heatmap_colors.len() + 1
                            {
                                draft.active_color_index = 0;
                            }
                        });
                    }
                },
            )),
    );
    selectors = selectors.child(
        field_card()
            .child(field_label("Preview Model"))
            .child(select_field(
                "settings-preview-model",
                &PREVIEW_MODEL_OPTIONS,
                PREVIEW_MODEL_VALUES[state.preview_model.min(PREVIEW_MODEL_VALUES.len() - 1)],
                None,
                Some(CONTROL_WIDTH),
                true,
                model_entity.clone(),
                move |value, cx| {
                    if let Some(index) = PREVIEW_MODEL_VALUES.iter().position(|model| *model == value)
                    {
                        edit_and_notify(&model_entity, cx, |draft| draft.preview_model = index);
                    }
                },
            )),
    );
    shader_section = shader_section.child(selectors);

    // Preview slot: the source embeds a wgpu canvas; here the slot is filled
    // with the mesh colour and labelled with the shader it would render.
    let preview = div()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(mix(hex_color(&state.mesh_color), SURFACE_0, 0.35)))
        .aspect_ratio(10.0 / 7.0)
        .child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .child(SHADER_LABELS[state.shader.min(SHADER_LABELS.len() - 1)]),
        );

    let active_index = state.active_color_index;
    let active_color = if active_index == 0 {
        state.mesh_color.clone()
    } else {
        state
            .heatmap_colors
            .get(active_index - 1)
            .cloned()
            .unwrap_or_else(|| DEFAULT_MESH_COLOR.to_owned())
    };
    let color_label = if active_index == 0 {
        "Mesh Color".to_owned()
    } else {
        format!("Heatmap {active_index}")
    };

    let mut color_card = div()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .p(px(8.))
        .flex()
        .flex_col()
        .gap(px(6.))
        .aspect_ratio(10.0 / 7.0)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .text_size(px(12.))
                        .flex_shrink_0()
                        .text_color(rgb(TEXT_MUTED))
                        .child(color_label),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .h(px(28.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .rounded(px(6.))
                        .border_1()
                        .border_color(rgb(BORDER_SUBTLE))
                        .bg(rgb(SURFACE_0))
                        .text_size(px(12.))
                        .text_color(rgb(TEXT_STRONG))
                        .child(active_color.clone()),
                ),
        )
        // The picker's saturation/value area: the colour under a white wash.
        .child(
            div()
                .relative()
                .flex_1()
                .min_h(px(0.))
                .rounded(px(4.))
                .overflow_hidden()
                .bg(rgb(hex_color(&active_color)))
                .child(
                    div()
                        .absolute()
                        .top(px(0.))
                        .left(px(0.))
                        .size_full()
                        .bg(linear_gradient(
                            90.,
                            linear_color_stop(rgba(0xffffffff), 0.),
                            linear_color_stop(rgba(0xffffff00), 1.),
                        )),
                ),
        );

    if state.shader == 1 {
        let mut strip = div()
            .flex()
            .items_center()
            .gap(px(2.))
            .pt(px(2.))
            .child(heatmap_swatch(
                0,
                &state.mesh_color,
                active_index == 0,
                entity.clone(),
            ))
            .child(
                div()
                    .w(px(1.))
                    .h(px(16.))
                    .mx(px(2.))
                    .flex_shrink_0()
                    .bg(rgb(BORDER_SUBTLE)),
            );
        for (index, color) in state.heatmap_colors.iter().enumerate() {
            strip = strip.child(heatmap_swatch(
                index + 1,
                color,
                active_index == index + 1,
                entity.clone(),
            ));
        }
        color_card = color_card.child(strip);
    }

    shader_section = shader_section.child(
        div()
            .mt(px(8.))
            .grid()
            .grid_cols(2)
            .gap(px(8.))
            .child(preview)
            .child(color_card),
    );

    if has_rendering_options {
        let mut options = div().mt(px(8.)).grid().grid_cols(2).gap(px(8.));

        if show_roughness {
            options = options.child(slider_field(
                "Roughness",
                format!("{:.2}", state.material_roughness),
                state.material_roughness,
                0.0,
                1.0));
        }
        if show_lighting {
            options = options.child(slider_field(
                "Lightness",
                format!("{:.2}", lightness),
                lightness,
                0.2,
                3.0));
            options = options.child(slider_field(
                "Contrast",
                format!("{:.2}", contrast),
                contrast,
                0.05,
                0.95));
        }
        if show_baked_ao {
            options = options.child(slider_field(
                "Ambient Occlusion",
                format!("{:.2}", state.baked_ao_intensity),
                state.baked_ao_intensity,
                0.0,
                2.0));
        }
        if state.shader == 4 {
            options = options.child(slider_field(
                "X-Ray Opacity",
                format!("{:.2}", state.xray_opacity),
                state.xray_opacity,
                0.02,
                0.85));
        }
        if state.shader == 1 {
            options = options.child(slider_field(
                "Min Overhang",
                format!("{}°", state.heatmap_min_angle.round() as i32),
                state.heatmap_min_angle,
                0.0,
                90.0));
            options = options.child(slider_field(
                "Max Overhang",
                format!("{}°", state.heatmap_max_angle.round() as i32),
                state.heatmap_max_angle,
                0.0,
                90.0));
        }

        shader_section = shader_section.child(divider_label(
            "sliders-horizontal",
            "Rendering Options",
        ));
        shader_section = shader_section.child(options);
    }

    root = root.child(shader_section);

    // Selection & Hover
    let selection_entity = entity.clone();
    let hover_entity = entity.clone();
    let preview_entity = entity.clone();
    let preview_hover_entity = entity.clone();

    let selection_card = field_card()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .mb(px(8.))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("Colors"),
                )
                .child(
                    div()
                        .id("settings-reset-selection-colors")
                        .w(px(24.))
                        .h(px(24.))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(6.))
                        .border_1()
                        .border_color(rgb(BORDER_SUBTLE))
                        .bg(rgb(SURFACE_1))
                        .cursor_pointer()
                        .hover(|style| style.bg(rgb(SURFACE_2)))
                        .child(icon("rotate-ccw", 12., TEXT_STRONG))
                        .on_click(move |_, _, cx| {
                            edit_and_notify(&selection_entity, cx, |draft| {
                                let accent = THEME_COLOR_SECTIONS
                                    .iter()
                                    .flat_map(|section| section.rows)
                                    .find(|row| row.key == "meshSelectionColor")
                                    .map(|row| row.placeholder.to_owned())
                                    .unwrap_or_else(|| "#ec2a77".to_owned());
                                draft
                                    .theme_colors
                                    .insert("meshSelectionColor", accent.clone());
                                draft.theme_colors.insert("meshHoverColor", accent);
                            });
                        }),
                ),
        )
        .child(
            div()
                .grid()
                .grid_cols(2)
                .gap(px(8.))
                .child(color_field(
                    "Selection Color",
                    state
                        .theme_colors
                        .get("meshSelectionColor")
                        .cloned()
                        .unwrap_or_else(|| "#ec2a77".to_owned()),
                    "settings-selection-color",
                    entity.clone(),
                    "meshSelectionColor",
                ))
                .child(color_field(
                    "Hover Color",
                    state
                        .theme_colors
                        .get("meshHoverColor")
                        .cloned()
                        .unwrap_or_else(|| "#ec2a77".to_owned()),
                    "settings-hover-color",
                    hover_entity,
                    "meshHoverColor",
                )),
        );

    let tint_card = field_card()
        .child(
            div()
                .mb(px(8.))
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child("Tint Intensity"),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(slider_field(
                    "Hover",
                    format!("{:.2}", state.hover_tint_strength),
                    state.hover_tint_strength,
                    0.0,
                    1.0))
                .child(slider_field(
                    "Selected",
                    format!("{:.2}", state.selected_tint_strength),
                    state.selected_tint_strength,
                    0.0,
                    1.0)),
        );

    let badge = if state.preview_selected {
        (
            "selected",
            TEXT_STRONG,
            mix(ACCENT, 0xffffff, 0.88),
            mix(ACCENT, SURFACE_0, 0.78),
        )
    } else if state.preview_hovered {
        ("hovered", TEXT_STRONG, BORDER_STRONG, SURFACE_2)
    } else {
        ("idle", TEXT_MUTED, BORDER_SUBTLE, SURFACE_0)
    };
    let preview_tint = if state.preview_selected {
        mix(
            hex_color(
                state
                    .theme_colors
                    .get("meshSelectionColor")
                    .map(String::as_str)
                    .unwrap_or("#ec2a77"),
            ),
            SURFACE_0,
            state.selected_tint_strength,
        )
    } else {
        mix(
            hex_color(
                state
                    .theme_colors
                    .get("meshHoverColor")
                    .map(String::as_str)
                    .unwrap_or("#ec2a77"),
            ),
            SURFACE_0,
            state.hover_tint_strength,
        )
    };

    let preview_card = field_card()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("Selection Preview"),
                )
                .child(
                    div()
                        .px(px(8.))
                        .py(px(2.))
                        .rounded_full()
                        .border_1()
                        .border_color(rgb(badge.2))
                        .bg(rgb(badge.3))
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(badge.1))
                        .child(badge.0),
                ),
        )
        .child(
            div()
                .id("settings-selection-preview")
                .mt(px(8.))
                .flex_1()
                .min_h(px(0.))
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(preview_tint))
                .cursor_pointer()
                .on_hover(move |hovered, _, cx| {
                    edit_and_notify(&preview_hover_entity, cx, |draft| {
                        draft.preview_hovered = *hovered;
                    });
                })
                .on_click(move |_, _, cx| {
                    edit_and_notify(&preview_entity, cx, |draft| {
                        draft.preview_selected = !draft.preview_selected;
                    });
                }),
        );

    root.child(
        section(
            "mouse-pointer-2",
            "Selection & Hover",
            "How selected and hovered models are emphasized throughout the app. These two colors come from the current theme — editing them here changes the theme, exactly like UI & Theme › Mesh Highlights.",
        )
        .child(
            div()
                .mt(px(12.))
                .grid()
                .grid_cols(2)
                .gap(px(8.))
                .child(
                    div()
                        .min_w(px(0.))
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(selection_card)
                        .child(tint_card),
                )
                .child(preview_card),
        ),
    )
}

pub(crate) fn heatmap_swatch(
    index: usize,
    color: &str,
    active: bool,
    entity: WeakEntity<DragonFruit>,
) -> Stateful<Div> {
    let first = index == 0;
    div()
        .id(format!("settings-heatmap-swatch-{index}"))
        .when(first, |this| this.w(px(20.)).h(px(20.)))
        .when(!first, |this| this.flex_1().h(px(20.)))
        .rounded(px(4.))
        .border_1()
        .border_color(rgb(if active { TEXT_STRONG } else { BORDER_STRONG }))
        .bg(rgb(hex_color(color)))
        .cursor_pointer()
        .on_click(move |_, _, cx| {
            edit_and_notify(&entity, cx, |draft| {
                draft.active_color_index = index;
            });
        })
}
