use super::*;

// ── Slicing ──────────────────────────────────────────────────────────────

pub(crate) fn slicing_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let mut root = div().flex().flex_col().gap(px(12.));

    root = root.child(
        section(
            "cpu",
            "Slicing Engine",
            "Native Rust slicing via the DragonFruit engine.",
        )
        .child(
            inner_card()
                .mt(px(12.))
                .child(
                    div()
                        .grid()
                        .grid_cols(2)
                        .gap(px(8.))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(rgb(TEXT_MUTED))
                                        .child("Crate"),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(TEXT_STRONG))
                                        .child("dragonfruit-slicing-engine"),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(rgb(TEXT_MUTED))
                                        .child("Version"),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(TEXT_STRONG))
                                        .child("-"),
                                ),
                        ),
                ),
        ),
    );

    let gradient_entity = entity.clone();
    let plate_entity = entity.clone();
    let grid_entity = entity.clone();
    root = root.child(
        section(
            "image",
            "Thumbnail Rendering",
            "Configure what appears in generated export/slice thumbnails.",
        )
        .child(
            inner_card()
                .mt(px(12.))
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(setting_row(
                    "Background gradient",
                    Some("Scene mood overlay in thumbnail".into()),
                    SettingDensity::Compact,
                    false,
                    SettingSurface::Sunken,
                    false,
                    segmented(
                        "settings-thumbnail-gradient",
                        &["OFF", "ON"],
                        usize::from(state.thumbnail_gradient),
                        false,
                        move |index, cx| {
                            edit_and_notify(&gradient_entity, cx, |draft| {
                                draft.thumbnail_gradient = index == 1;
                            });
                        },
                    ),
                ))
                .child(setting_row(
                    "Build plate",
                    Some("Render build plate in thumbnail".into()),
                    SettingDensity::Compact,
                    false,
                    SettingSurface::Sunken,
                    false,
                    segmented(
                        "settings-thumbnail-plate",
                        &["OFF", "ON"],
                        usize::from(state.thumbnail_build_plate),
                        false,
                        move |index, cx| {
                            edit_and_notify(&plate_entity, cx, |draft| {
                                draft.thumbnail_build_plate = index == 1;
                            });
                        },
                    ),
                ))
                .child(setting_row(
                    "Grid",
                    Some("Render build grid in thumbnail".into()),
                    SettingDensity::Compact,
                    false,
                    SettingSurface::Sunken,
                    false,
                    segmented(
                        "settings-thumbnail-grid",
                        &["OFF", "ON"],
                        usize::from(state.thumbnail_grid),
                        false,
                        move |index, cx| {
                            edit_and_notify(&grid_entity, cx, |draft| {
                                draft.thumbnail_grid = index == 1;
                            });
                        },
                    ),
                )),
        ),
    );

    let compression_entity = entity.clone();
    root = root.child(
        section(
            "zap",
            "PNG Compression",
            "Enable or disable PNG compression for PNG-based formats.",
        )
        .child(
            inner_card().mt(px(12.)).child(setting_row(
                "Compression",
                Some("Auto adapts by AA level, Off is fastest, On favors smaller PNG files".into()),
                SettingDensity::Compact,
                false,
                SettingSurface::Sunken,
                false,
                segmented(
                    "settings-png-compression",
                    &["AUTO", "OFF", "ON"],
                    state.png_compression.min(PNG_MODE_LABELS.len() - 1),
                    false,
                    move |index, cx| {
                        edit_and_notify(&compression_entity, cx, |draft| {
                            draft.png_compression = index;
                        });
                    },
                ),
            )),
        ),
    );

    let stale_entity = entity.clone();
    let all_entity = entity.clone();
    root.child(
        section(
            "trash-2",
            "Temp File Cleanup",
            "Free disk space by removing temporary slice files.",
        )
        .child(
            div()
                .mt(px(12.))
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(cleanup_button(
                    "settings-clean-stale",
                    "Clean Stale Files",
                    "Remove temp files older than 1 hour",
                    false,
                    move |cx| {
                        stale_entity.update(cx, |_, cx| cx.notify()).ok();
                    },
                ))
                .child(cleanup_button(
                    "settings-clean-all",
                    "Clean All Files",
                    "Emergency cleanup: delete all temp slices",
                    true,
                    move |cx| {
                        all_entity.update(cx, |_, cx| cx.notify()).ok();
                    },
                )),
        ),
    )
}

pub(crate) fn cleanup_button(
    id: &'static str,
    title: &'static str,
    description: &'static str,
    danger: bool,
    on_click: impl Fn(&mut App) + 'static,
) -> Stateful<Div> {
    let hover = if danger {
        mix(DANGER, SURFACE_0, 0.05)
    } else {
        mix(ACCENT, SURFACE_0, 0.08)
    };
    div()
        .id(id)
        .w_full()
        .p(px(10.))
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .flex()
        .flex_col()
        .cursor_pointer()
        .hover(move |style| style.bg(rgb(hover)))
        .on_click(move |_, _, cx| on_click(cx))
        .child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child(title),
        )
        .child(
            div()
                .mt(px(2.))
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child(description),
        )
}
