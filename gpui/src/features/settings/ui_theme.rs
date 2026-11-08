use super::*;

// ── UI & Theme ───────────────────────────────────────────────────────────

pub(crate) fn ui_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let mut root = div().flex().flex_col().gap(px(10.));

    let preset_entity = entity.clone();
    let scheme_entity = entity.clone();
    let reset_entity = entity.clone();
    let theme_section = div()
        .rounded(px(12.)) // rounded-xl
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(10.))
        .child(
            div()
                .mb(px(8.))
                .flex()
                .items_start()
                .gap(px(8.))
                .child(icon_chip("palette"))
                .child(
                    div()
                        .flex_1()
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child("Theme"),
                        )
                        .child(
                            div()
                                .mt(px(2.))
                                .text_size(px(12.))
                                .line_height(relative(1.2))
                                .text_color(rgb(TEXT_MUTED))
                                .child("Preview is live. Apply saves it."),
                        ),
                ),
        )
        .child(
            div()
                .grid()
                .grid_cols(2)
                .gap(px(8.))
                .child(
                    div()
                        .rounded(px(8.)) // rounded-lg
                        .border_1()
                        .border_color(rgb(BORDER_SUBTLE))
                        .bg(rgb(SURFACE_2))
                        .p(px(10.))
                        .child(upper_label("Theme preset"))
                        .child(select_field(
                            "settings-theme-preset",
                            &THEME_PRESET_OPTIONS,
                            THEME_PRESET_VALUES[state.theme_preset.min(THEME_PRESET_VALUES.len() - 1)],
                            None,
                            Some(CONTROL_WIDTH),
                            true,
                            preset_entity.clone(),
                            move |value, cx| {
                                if let Some(index) =
                                    THEME_PRESET_VALUES.iter().position(|preset| *preset == value)
                                {
                                    edit_and_notify(&preset_entity, cx, |draft| {
                                        draft.theme_preset = index;
                                    });
                                }
                            },
                        )),
                )
                .child(
                    div()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(rgb(BORDER_SUBTLE))
                        .bg(rgb(SURFACE_2))
                        .p(px(10.))
                        // `opacity: isBuiltInThemePreset ? 0.5 : 1`
                        .opacity(0.5)
                        .child(upper_label("Color scheme"))
                        .child(select_field(
                            "settings-theme-scheme",
                            &SCHEME_OPTIONS,
                            SCHEME_VALUES[state.theme_preference.min(SCHEME_VALUES.len() - 1)],
                            None,
                            Some(CONTROL_WIDTH),
                            false,
                            scheme_entity.clone(),
                            move |value, cx| {
                                if let Some(index) =
                                    SCHEME_VALUES.iter().position(|scheme| *scheme == value)
                                {
                                    edit_and_notify(&scheme_entity, cx, |draft| {
                                        draft.theme_preference = index;
                                    });
                                }
                            },
                        )),
                ),
        )
        .child(
            div()
                .mt(px(8.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        // Theme import/export is not ported; the button keeps the
                        // source layout and reads as disabled.
                        .child(
                            button_with(
                                "settings-theme-import",
                                "Import",
                                ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Sm)
                                    .padding_x(10.0)
                                    .padding_y(0.0, 0.0)
                                    .font_size(12.0)
                                    .disabled(),
                            )
                            .opacity(0.5),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(
                            button_with(
                                "settings-theme-reset",
                                "Reset",
                                ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Sm)
                                    .padding_x(10.0)
                                    .padding_y(0.0, 0.0)
                                    .font_size(12.0),
                            )
                            .on_click(move |_, _, cx| {
                                edit_and_notify(&reset_entity, cx, |draft| {
                                    // `DEFAULT_THEME_CUSTOM_COLORS`, the store's
                                    // own defaults.
                                    let defaults = gpui_store::ThemeColors::default();
                                    for section in THEME_COLOR_SECTIONS.iter() {
                                        for row in section.rows {
                                            if let Some(value) = defaults.get(row.key) {
                                                draft
                                                    .theme_colors
                                                    .insert(row.key, value.to_owned());
                                            }
                                        }
                                    }
                                });
                            }),
                        ),
                ),
        );

    root = root.child(theme_section);

    // Sub-tab strip
    let general_entity = entity.clone();
    let mesh_entity = entity.clone();
    let cut_entity = entity.clone();
    root = root.child(
        div()
            .flex()
            .items_center()
            .gap(px(6.))
            .border_b_1()
            .border_color(rgb(BORDER_SUBTLE))
            .pb(px(8.))
            .child(theme_sub_tab(
                "settings-theme-tab-general",
                "General UI",
                state.theme_tab == 0,
                move |cx| {
                    edit_and_notify(&general_entity, cx, |draft| draft.theme_tab = 0);
                },
            ))
            .child(theme_sub_tab(
                "settings-theme-tab-mesh",
                "Mesh Highlights",
                state.theme_tab == 1,
                move |cx| {
                    edit_and_notify(&mesh_entity, cx, |draft| draft.theme_tab = 1);
                },
            ))
            .child(theme_sub_tab(
                "settings-theme-tab-cut",
                "Cutting Tool",
                state.theme_tab == 2,
                move |cx| {
                    edit_and_notify(&cut_entity, cx, |draft| draft.theme_tab = 2);
                },
            )),
    );

    match state.theme_tab {
        0 => {
            let mut grid = div().grid().grid_cols(2).gap(px(10.));
            for section in THEME_COLOR_SECTIONS.iter() {
                let mut card = div()
                    .rounded(px(12.))
                    .border_1()
                    .border_color(rgb(BORDER_SUBTLE))
                    .bg(rgb(SURFACE_1))
                    .p(px(10.))
                    .child(
                        div()
                            .mb(px(8.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(TEXT_STRONG))
                                    .child(section.title),
                            )
                            .child(
                                div()
                                    .mt(px(2.))
                                    .text_size(px(11.))
                                    .line_height(relative(1.2))
                                    .text_color(rgb(TEXT_MUTED))
                                    .child(section.description),
                            ),
                    );
                let mut rows = div().flex().flex_col().gap(px(6.));
                for row in section.rows {
                    rows = rows.child(color_row(row, theme_color(state, row.key)));
                }
                card = card.child(rows);
                grid = grid.child(card);
            }
            root.child(grid)
        }
        1 => {
            let mut card = div()
                .rounded(px(12.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_1))
                .p(px(10.))
                .child(
                    div()
                        .mb(px(8.))
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child("Mesh Highlights"),
                        )
                        .child(
                            div()
                                .mt(px(2.))
                                .text_size(px(11.))
                                .line_height(relative(1.2))
                                .text_color(rgb(TEXT_MUTED))
                                .child("Selection and hover tint for models in the 3D view. Also editable from the Mesh tab, which mirrors these two theme colors."),
                        ),
                );
            let mut grid = div().grid().grid_cols(2).gap(px(6.));
            for row in MESH_HIGHLIGHT_COLOR_FIELDS.iter() {
                grid = grid.child(color_row(row, theme_color(state, row.key)));
            }
            card = card.child(grid);
            root.child(card)
        }
        _ => {
            let mut card = div()
                .rounded(px(12.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_1))
                .p(px(10.))
                .child(
                    div()
                        .mb(px(8.))
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child("Cut Tool Colors"),
                        )
                        .child(
                            div()
                                .mt(px(2.))
                                .text_size(px(11.))
                                .line_height(relative(1.2))
                                .text_color(rgb(TEXT_MUTED))
                                .child("Seam, cut surface, tenon, mortise, and waypoint colors for the Cut tool."),
                        ),
                );
            let mut grid = div().grid().grid_cols(2).gap(px(6.));
            for field in CUT_COLOR_FIELDS.iter() {
                let value = state
                    .cut_colors
                    .get(field.key)
                    .cloned()
                    .unwrap_or_else(|| field.placeholder.to_owned());
                grid = grid.child(color_row(field, value));
            }
            card = card.child(grid);
            root.child(card)
        }
    }
}

pub(crate) fn theme_sub_tab(
    id: &'static str,
    label: &'static str,
    active: bool,
    on_click: impl Fn(&mut App) + 'static,
) -> Stateful<Div> {
    div()
        .id(id)
        .h(px(28.))
        .px(px(10.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(if active {
            mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.58)
        } else {
            BORDER_SUBTLE
        }))
        .bg(rgb(SURFACE_1))
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(if active { ACCENT_SECONDARY } else { TEXT_MUTED }))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(SURFACE_2)))
        .on_click(move |_, _, cx| on_click(cx))
        .child(label)
}

/// One theme colour entry: label on the left, swatch and stored hex on the
/// right (`grid-cols-[minmax(0,1fr)_10.75rem]`).
pub(crate) fn color_row(field: &'static ColorField, value: String) -> Div {
    let label = field.label;
    let description = field.description;
    div()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        // color-mix(in srgb, surface-0, transparent 8%) over surface-1
        .bg(rgb(mix(SURFACE_0, SURFACE_1, 0.92)))
        .px(px(8.))
        .py(px(6.))
        .child(
            div()
                .grid()
                .grid_cols(2)
                .gap(px(10.))
                .items_center()
                .child(
                    div()
                        .min_w(px(0.))
                        .truncate()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child(label),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(swatch(&value, 32., 28.))
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
                                .child(value),
                        ),
                ),
        )
        .child(
            div()
                .mt(px(2.))
                .text_size(px(11.))
                .text_color(rgb(TEXT_MUTED))
                .child(description),
        )
}

/// The Mesh tab's Selection/Hover colour input (`ColorSwatchInput` + `.ui-input`).
pub(crate) fn color_field(
    label: &'static str,
    value: String,
    id: &'static str,
    entity: WeakEntity<DragonFruit>,
    key: &'static str,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child(label),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .id(format!("{id}-swatch"))
                        .cursor_pointer()
                        .child(swatch(&value, 40., 32.))
                        .on_click(move |_, _, cx| {
                            // A native colour picker is not available; cycling the
                            // stored value keeps the control live.
                            edit_and_notify(&entity, cx, |draft| {
                                let next = next_theme_color(&theme_color(draft, key));
                                draft.theme_colors.insert(key, next);
                            });
                        }),
                )
                .child(
                    div()
                        .w(px(120.)) // w-[7.5rem]
                        .min_w(px(0.))
                        .h(px(32.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .rounded(px(6.))
                        .border_1()
                        .border_color(rgb(BORDER_SUBTLE))
                        .bg(rgb(SURFACE_0))
                        .text_size(px(12.))
                        .text_color(rgb(TEXT_STRONG))
                        .child(value),
                ),
        )
}

pub(crate) fn next_theme_color(current: &str) -> String {
    let palette = ["#ec2a77", "#baf72e", "#487dd5", "#0a667c", "#e45454", "#2eb67d"];
    let index = palette
        .iter()
        .position(|color| color.eq_ignore_ascii_case(current))
        .map(|index| (index + 1) % palette.len())
        .unwrap_or(0);
    palette[index].to_owned()
}
