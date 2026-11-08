use super::*;

// ── Multi-Plate (`MultiPlateSettingsTab.tsx`) ────────────────────────────

pub(crate) fn multi_plate_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let plate = &state.multi_plate;

    let follow_entity = entity.clone();
    let ordering_entity = entity.clone();
    let columns_entity = entity.clone();

    let mut body = div()
        .flex()
        .flex_col()
        .child(
            setting_row(
                "Follow the plate a model lands on",
                Some("A model dropped on another bed makes that bed the one you work on.".into()),
                SettingDensity::Compact,
                true,
                SettingSurface::Sunken,
                false,
                segmented(
                    "multi-plate-follow",
                    &["ON", "OFF"],
                    usize::from(!plate.follow_landed_plate),
                    false,
                    move |index, cx| {
                        edit_and_notify(&follow_entity, cx, |draft| {
                            draft.multi_plate.follow_landed_plate = index == 0;
                        });
                    },
                ),
            )
            .mt(px(12.)),
        )
        .child(
            setting_row(
                "Plate Ordering",
                Some("Growing grid, or a fixed row length.".into()),
                SettingDensity::Compact,
                true,
                SettingSurface::Sunken,
                false,
                segmented(
                    "multi-plate-ordering",
                    &["FIXED", "DYNAMIC"],
                    match plate.plate_ordering {
                        PlateOrdering::Fixed => 0,
                        PlateOrdering::Dynamic => 1,
                    },
                    false,
                    move |index, cx| {
                        edit_and_notify(&ordering_entity, cx, |draft| {
                            draft.multi_plate.plate_ordering = if index == 0 {
                                PlateOrdering::Fixed
                            } else {
                                PlateOrdering::Dynamic
                            };
                        });
                    },
                ),
            )
            .mt(px(12.)),
        );

    if plate.plate_ordering == PlateOrdering::Fixed {
        body = body.child(
            setting_row(
                "Beds per row",
                Some("1-2-3-4 across, then the next row. Adding a bed never moves the ones already placed.".into()),
                SettingDensity::Compact,
                true,
                SettingSurface::Sunken,
                false,
                number_input(
                    "multi-plate-columns",
                    plate.fixed_plate_columns,
                    1.0,
                    MAX_FIXED_PLATE_COLUMNS,
                    1.0,
                    Some(120.0),
                    34.0, // h-[34px]
                    14.0,
                    SURFACE_0,
                    true,
                    move |next, cx| {
                        if !next.is_finite() {
                            return;
                        }
                        edit_and_notify(&columns_entity, cx, |draft| {
                            draft.multi_plate.fixed_plate_columns = next
                                .clamp(1.0, MAX_FIXED_PLATE_COLUMNS)
                                .round();
                        });
                    },
                ),
            )
            .mt(px(12.)),
        );
    }

    div().flex().flex_col().child(
        div()
            .flex()
            .flex_col()
            .rounded(px(ROUNDED_LG)) // `rounded-lg` — 14px in this app
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1))
            .p(px(12.))
            .child(section_header(
                "square-stack",
                "Plates",
                "Behaviour across the beds of a scene.",
            ))
            .child(body),
    )
}
