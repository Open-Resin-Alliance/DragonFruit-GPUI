use super::*;

// ── Scene Autosave (`SceneAutosaveSettingsTab.tsx`) ──────────────────────

pub(crate) fn autosave_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let autosave = &state.autosave;
    let enabled_entity = entity.clone();
    let debounce_entity = entity.clone();
    let cooldown_entity = entity.clone();
    let cap_entity = entity.clone();
    let prompt_entity = entity;

    let debounce_seconds = (autosave.debounce_ms / 1000.0).round();
    let cooldown_seconds = (autosave.cooldown_ms / 1000.0).round();
    let cap_minutes = (autosave.cap_ms / 60_000.0).round();

    let mut rows = div().mt(px(8.)).flex().flex_col().gap(px(8.));
    rows = rows.child(setting_row_surface(
        "Enable scene autosave",
        "Creates recovery snapshots while editing.",
        false,
        false,
        segmented(
            "autosave-enabled",
            &["OFF", "ON"],
            usize::from(autosave.enabled),
            false,
            move |index, cx| {
                edit_and_notify(&enabled_entity, cx, |draft| {
                    draft.autosave.enabled = index == 1;
                });
            },
        ),
    ));
    rows = rows.child(setting_row_surface(
        "Autosave idle delay",
        "Wait this long after edits before autosaving.",
        false,
        !autosave.enabled,
        number_with_unit(
            number_input(
                "autosave-debounce",
                debounce_seconds,
                15.0,
                900.0,
                5.0,
                Some(120.0),
                34.0, // h-[34px]
                14.0, // text-sm
                SURFACE_0, // .ui-input
                autosave.enabled,
                move |next, cx| {
                    if !next.is_finite() {
                        return;
                    }
                    let next_seconds = next.round().clamp(15.0, 900.0);
                    edit_and_notify(&debounce_entity, cx, |draft| {
                        draft.autosave.debounce_ms = next_seconds * 1000.0;
                        draft.autosave.cap_ms = draft.autosave.cap_ms.max(next_seconds * 1000.0);
                    });
                },
            ),
            "sec",
        ),
    ));
    rows = rows.child(setting_row_surface(
        "Autosave cooldown",
        "After an autosave, defer further automatic saves until this interval ends.",
        false,
        !autosave.enabled,
        number_with_unit(
            number_input(
                "autosave-cooldown",
                cooldown_seconds,
                15.0,
                900.0,
                5.0,
                Some(120.0),
                34.0,
                14.0,
                SURFACE_0,
                autosave.enabled,
                move |next, cx| {
                    if !next.is_finite() {
                        return;
                    }
                    let next_seconds = next.round().clamp(15.0, 900.0);
                    edit_and_notify(&cooldown_entity, cx, |draft| {
                        draft.autosave.cooldown_ms = next_seconds * 1000.0;
                        draft.autosave.cap_ms = draft.autosave.cap_ms.max(next_seconds * 1000.0);
                    });
                },
            ),
            "sec",
        ),
    ));
    rows = rows.child(setting_row_surface(
        "Maximum autosave interval",
        "For continuous edits, save at least this often.",
        false,
        !autosave.enabled,
        number_with_unit(
            number_input(
                "autosave-cap",
                cap_minutes,
                1.0,
                60.0,
                1.0,
                Some(120.0),
                34.0,
                14.0,
                SURFACE_0,
                autosave.enabled,
                move |next, cx| {
                    if !next.is_finite() {
                        return;
                    }
                    let next_minutes = next.round().clamp(1.0, 60.0);
                    edit_and_notify(&cap_entity, cx, |draft| {
                        draft.autosave.cap_ms = (next_minutes * 60_000.0)
                            .max(draft.autosave.debounce_ms)
                            .max(draft.autosave.cooldown_ms);
                    });
                },
            ),
            "min",
        ),
    ));

    let mut root = div().flex().flex_col().gap(px(12.));
    root = root.child(
        section(
            "hard-drive",
            "Scene Autosave",
            "Automatically saves a crash-recovery .voxl scene in the background and offers restore on next launch.",
        )
        .child(rows),
    );

    // Crash Recovery — the source's `IconChip tone="warning"` header.
    root = root.child(
        div()
            .rounded(px(6.))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1))
            .p(px(12.))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(32.))
                            .h(px(32.))
                            .flex_shrink_0()
                            .rounded(px(6.))
                            .border_1()
                            .border_color(rgb(mix(0xd97706, BORDER_SUBTLE, 0.5)))
                            .bg(rgb(mix(0xd97706, SURFACE_1, 0.15)))
                            .child(icon("triangle-alert", 16., 0xd97706)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(TEXT_STRONG))
                                    .child("Crash Recovery"),
                            )
                            .child(
                                div()
                                    .mt(px(2.))
                                    .text_size(px(12.))
                                    .text_color(rgb(TEXT_MUTED))
                                    .child("Show restore/discard prompt if a dirty autosave is detected at startup."),
                            ),
                    ),
            )
            .child(
                setting_row_label_only(
                    "Recovery prompt on startup",
                    segmented(
                        "autosave-recovery-prompt",
                        &["OFF", "ON"],
                        usize::from(autosave.recovery_prompt_enabled),
                        false,
                        move |index, cx| {
                            edit_and_notify(&prompt_entity, cx, |draft| {
                                draft.autosave.recovery_prompt_enabled = index == 1;
                            });
                        },
                    ),
                )
                .mt(px(8.)),
            ),
    );

    // Autosave Status — the desktop bridge is absent, so the source's own
    // "desktop build" state renders and the actions stay disabled.
    root = root.child(
        div()
            .rounded(px(6.))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1))
            .p(px(12.))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(8.))
                    .child(icon_chip("hard-drive"))
                    .child(
                        div()
                            .min_w(px(0.))
                            .flex_1()
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(TEXT_STRONG))
                                    .child("Autosave Status"),
                            )
                            .child(
                                div()
                                    .mt(px(2.))
                                    .text_size(px(12.))
                                    .text_color(rgb(TEXT_MUTED))
                                    .child("Desktop-only diagnostics for autosave files and recovery state."),
                            ),
                    )
                    .child(inert_small_button(
                        "autosave-refresh",
                        "Refresh",
                        "refresh-cw",
                    )),
            )
            .child(
                div()
                    .mt(px(8.))
                    .text_size(px(12.))
                    .text_color(rgb(TEXT_MUTED))
                    .child("Autosave files are available in the DragonFruit desktop build."),
            )
            .child(
                div()
                    .mt(px(8.))
                    .grid()
                    .grid_cols(2)
                    .gap(px(8.))
                    .child(
                        action_button(
                            "autosave-mark-clean",
                            "Mark Recovery Clean",
                            Some("triangle-alert"),
                            12.0,
                            mix(0xef4444, SURFACE_1, 0.86),
                            mix(0xef4444, BORDER_SUBTLE, 0.45),
                            0xef4444,
                            mix(0xef4444, SURFACE_1, 0.76),
                            |_| {},
                        )
                        .opacity(0.6),
                    )
                    .child(
                        action_button(
                            "autosave-reveal",
                            "Open Autosave Location",
                            Some("hard-drive"),
                            12.0,
                            mix(ACCENT, SURFACE_1, 0.14),
                            mix(ACCENT, BORDER_SUBTLE, 0.55),
                            ACCENT,
                            mix(ACCENT, SURFACE_1, 0.24),
                            |_| {},
                        )
                        .opacity(0.6),
                    ),
            ),
    );

    root
}
