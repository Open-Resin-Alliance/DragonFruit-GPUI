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
    rows = rows.child(setting_row(
        "Enable scene autosave",
        Some("Creates recovery snapshots while editing.".into()),
        SettingDensity::Compact,
        true,
        SettingSurface::Sunken,
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
    rows = rows.child(setting_row(
        "Autosave idle delay",
        Some("Wait this long after edits before autosaving.".into()),
        SettingDensity::Compact,
        true,
        SettingSurface::Sunken,
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
    rows = rows.child(setting_row(
        "Autosave cooldown",
        Some("After an autosave, defer further automatic saves until this interval ends.".into()),
        SettingDensity::Compact,
        true,
        SettingSurface::Sunken,
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
    rows = rows.child(setting_row(
        "Maximum autosave interval",
        Some("For continuous edits, save at least this often.".into()),
        SettingDensity::Compact,
        true,
        SettingSurface::Sunken,
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
            .rounded(px(ROUNDED_MD)) // `rounded-md` — 10px in this app
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1))
            .p(px(12.))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(8.))
                    .child(icon_chip_with(
                        IconChipSize::Md,
                        IconTone::Warning,
                        icon("triangle-alert", 16., 0xd97706),
                    ))
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
                setting_row(
                    "Recovery prompt on startup",
                    None,
                    SettingDensity::Compact,
                    true,
                    SettingSurface::Sunken,
                    false,
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
            .rounded(px(ROUNDED_MD)) // `rounded-md` — 10px in this app
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
                    .child(
                        button_with(
                            "autosave-refresh",
                            "Refresh",
                            ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Sm)
                                .padding_x(10.0)
                                .padding_y(0.0, 0.0)
                                .font_size(12.0)
                                .gap(6.0)
                                .disabled()
                                .icon("refresh-cw", 14.0, TEXT_MUTED),
                        )
                        .opacity(0.6),
                    ),
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
                        button_with(
                            "autosave-mark-clean",
                            "Mark Recovery Clean",
                            ButtonSpec::new(ButtonVariant::TintedDanger, ButtonSize::Lg)
                                .padding_x(12.0)
                                .padding_y(0.0, 0.0)
                                .font_size(14.0)
                                .gap(6.0)
                                .disabled()
                                .icon("triangle-alert", 16.0, DANGER),
                        )
                        .opacity(0.6),
                    )
                    .child(
                        button_with(
                            "autosave-reveal",
                            "Open Autosave Location",
                            ButtonSpec::new(ButtonVariant::TintedAccent, ButtonSize::Lg)
                                .padding_x(12.0)
                                .padding_y(0.0, 0.0)
                                .font_size(14.0)
                                .gap(6.0)
                                .disabled()
                                .icon("hard-drive", 16.0, ACCENT),
                        )
                        .opacity(0.6),
                    ),
            ),
    );

    root
}
