use super::*;

// ── Backups (`LocalBackupsSettingsTab.tsx`) ──────────────────────────────

pub(crate) fn backups_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let backups = &state.backups;
    let auto_entity = entity.clone();
    let interval_entity = entity;

    let selected = backups.selected_directory.trim();
    let has_selected = !selected.is_empty();
    // `defaultDirectory` comes from the native backup bridge, which this slice
    // does not have, so the probe never answers: `isUsingDefault` is true and
    // the "Use Default Path" action is absent, exactly as at the source.
    let has_any_sync = false;

    let mut root = div().flex().flex_col().gap(px(12.));

    // Local On-Disk Backups
    root = root.child(
        div()
            .relative()
            .rounded(px(ROUNDED_MD)) // `rounded-md` — 10px in this app
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1))
            .p(px(12.))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(10.))
                    .child(icon_chip("hard-drive"))
                    .child(
                        div()
                            .min_w(px(0.))
                            .flex_1()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(TEXT_STRONG))
                                    .child("Local On-Disk Backups"),
                            )
                            .child(
                                div()
                                    .mt(px(2.))
                                    .text_size(px(12.))
                                    .line_height(relative(1.2))
                                    .text_color(rgb(TEXT_MUTED))
                                    .child("Automatically saves snapshots of your settings and profiles to a folder on your computer."),
                            ),
                    ),
            )
            .child(
                div()
                    .mt(px(8.))
                    .grid()
                    .grid_cols(2)
                    .gap(px(8.))
                    .child(stat_card("DEFAULT DIRECTORY", "Resolving..."))
                    .child(stat_card(
                        "SELECTED DIRECTORY",
                        if has_selected {
                            selected.to_owned()
                        } else {
                            "Not selected".to_owned()
                        },
                    ))
                    .child(stat_card("LAST BACKUP ON DISK", "Never"))
                    .child(stat_card("LAST LOCAL SYNC", "Never")),
            )
            .child(
                div()
                    .mt(px(8.))
                    .grid()
                    .grid_cols(2)
                    .gap(px(8.))
                    .child(
                        button_with(
                            "backups-choose-directory",
                            "Choose Folder",
                            ButtonSpec::new(ButtonVariant::TintedAccent, ButtonSize::Lg)
                                .padding_x(12.0)
                                .padding_y(0.0, 0.0)
                                .font_size(14.0)
                                .gap(6.0)
                                .icon("folder-open", 16.0, ACCENT),
                        )
                        .on_click(|_, _, _| {}),
                    )
                    .child(
                        button_with(
                            "backups-open-directory",
                            "Open Folder",
                            ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Lg)
                                .padding_x(12.0)
                                .padding_y(0.0, 0.0)
                                .font_size(14.0)
                                .gap(6.0)
                                .hover(mix(ACCENT_SECONDARY, SURFACE_1, 0.24))
                                .icon("folder-open", 16.0, ACCENT_SECONDARY),
                        )
                        .border_color(rgb(mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.55)))
                        .bg(rgb(mix(ACCENT_SECONDARY, SURFACE_1, 0.14)))
                        .text_color(rgb(ACCENT_SECONDARY))
                        .on_click(|_, _, _| {})
                        .when(!has_selected, |this| this.opacity(0.6)),
                    ),
            ),
    );

    // Backup Management
    let mut management = div()
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
                .child(icon_chip("refresh-ccw"))
                .child(
                    div()
                        .flex_1()
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child("Backup Management"),
                        )
                        .child(
                            div()
                                .mt(px(2.))
                                .text_size(px(12.))
                                .text_color(rgb(TEXT_MUTED))
                                .child("Save snapshots to your local backup folder, then review and restore any point-in-time snapshot in-app."),
                        ),
                ),
        );

    // Quick actions
    let mut quick = inner_card()
        .child(stat_label("QUICK ACTIONS"))
        .child(
            div()
                .mt(px(2.))
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child("Create a snapshot now and refresh history."),
        );
    quick = quick.child(
        div()
            .mt(px(8.))
            .grid()
            .grid_cols(2)
            .gap(px(8.))
            .child(
                button_with(
                    "backups-now",
                    "Backup Now",
                    ButtonSpec::new(ButtonVariant::TintedAccent, ButtonSize::Lg)
                        .padding_x(12.0)
                        .padding_y(0.0, 0.0)
                        .font_size(14.0)
                        .gap(6.0)
                        .icon("cloud-upload", 16.0, ACCENT),
                )
                .on_click(|_, _, _| {})
                .when(!has_selected, |this| this.opacity(0.6)),
            )
            .child(
                button_with(
                    "backups-refresh-history",
                    "Refresh History",
                    ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Lg)
                        .padding_x(12.0)
                        .padding_y(0.0, 0.0)
                        .font_size(14.0)
                        .gap(6.0)
                        .icon("refresh-ccw", 16.0, TEXT_STRONG),
                )
                .on_click(|_, _, _| {})
                .when(!has_selected, |this| this.opacity(0.6)),
            ),
    );
    management = management.child(quick.mt(px(8.)));

    // Automation
    let mut automation = inner_card().child(stat_label("AUTOMATION"));
    automation = automation.child(
        div()
            .mt(px(8.))
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(setting_row(
                "Enable automatic backups",
                Some("Automatically write snapshots to local disk on an interval.".into()),
                SettingDensity::Compact,
                true,
                SettingSurface::Raised,
                false,
                segmented(
                    "backups-auto",
                    &["OFF", "ON"],
                    usize::from(backups.auto_sync_enabled),
                    false,
                    move |index, cx| {
                        edit_and_notify(&auto_entity, cx, |draft| {
                            draft.backups.auto_sync_enabled = index == 1;
                        });
                    },
                ),
            ))
            .child(setting_row(
                "Sync interval",
                Some("Minutes between automatic local backups.".into()),
                SettingDensity::Compact,
                true,
                SettingSurface::Raised,
                !backups.auto_sync_enabled,
                number_with_unit(
                    number_input(
                        "backups-interval",
                        backups.auto_sync_minutes,
                        1.0,
                        240.0,
                        1.0,
                        Some(120.0),
                        34.0,
                        14.0,
                        SURFACE_0,
                        backups.auto_sync_enabled,
                        move |next, cx| {
                            if !next.is_finite() {
                                return;
                            }
                            edit_and_notify(&interval_entity, cx, |draft| {
                                draft.backups.auto_sync_minutes =
                                    next.round().clamp(1.0, 240.0);
                            });
                        },
                    ),
                    "min",
                ),
            )),
    );
    automation = automation.child(
        div()
            .mt(px(8.))
            .text_size(px(12.))
            .text_color(rgb(TEXT_MUTED))
            .child("Last local sync: never"),
    );
    management = management.child(automation.mt(px(8.)));

    // Manage Backups
    let mut manage = inner_card().child(
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(
                div()
                    .child(stat_label("MANAGE BACKUPS"))
                    .child(
                        div()
                            .mt(px(2.))
                            .text_size(px(12.))
                            .text_color(rgb(TEXT_MUTED))
                            .child("View, restore, or delete older snapshots from disk."),
                    ),
            )
            .child(
                button_with(
                    "backups-manage-refresh",
                    "Refresh",
                    ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Sm)
                        .padding_x(10.0)
                        .padding_y(0.0, 0.0)
                        .font_size(12.0)
                        .gap(6.0)
                        .disabled()
                        .icon("refresh-ccw", 14.0, TEXT_MUTED),
                )
                .opacity(0.6),
            ),
    );
    manage = manage.child(
        div().mt(px(8.)).max_h(px(256.)).overflow_hidden().child(
            div()
                .px(px(12.))
                .py(px(12.))
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child(if has_any_sync {
                    "No history snapshots found in this folder."
                } else {
                    "No snapshots yet. Run \"Backup Now\" to create your first local snapshot."
                }),
        ),
    );
    management = management.child(manage.mt(px(8.)));
    root = root.child(management);

    if let Some((error, text)) = &backups.message {
        root = root.child(message_strip(Some(*error), text));
    }

    root
}

/// The delete/restore confirmations. Both ids are set from a history row, and
/// the slice lists no rows, so these never open; the dialogs still carry the
/// source's copy.
pub(crate) fn backups_overlay(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Option<Div> {
    let backups = &state.backups;

    if let Some(id) = backups.confirm_delete_id.clone() {
        let cancel_entity = entity.clone();
        let confirm_entity = entity.clone();
        let close_entity = entity;
        return Some(dialog_modal_full(
            448.0, // max-w-md
            "Delete Backup Snapshot",
            Some("This action cannot be undone.".into()),
            Some("trash-2"),
            IconTone::Danger,
            false,
            div().child(
                div()
                    .text_size(px(12.))
                    .line_height(relative(1.35))
                    .text_color(rgb(TEXT_MUTED))
                    .child(format!(
                        "Backup snapshot from {id} will be permanently removed from disk."
                    )),
            ),
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(
                    button_with(
                        "backups-delete-cancel",
                        "Cancel",
                        ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Lg)
                            .padding_x(16.0)
                            .padding_y(0.0, 0.0)
                            .font_size(14.0),
                    )
                    .on_click(move |_, _, cx| {
                        edit_and_notify(&cancel_entity, cx, |draft| {
                            draft.backups.confirm_delete_id = None;
                        });
                    }),
                )
                .child(
                    button_with(
                        "backups-delete-confirm",
                        "Delete",
                        ButtonSpec::new(ButtonVariant::TintedDanger, ButtonSize::Lg)
                            .padding_x(16.0)
                            .padding_y(0.0, 0.0)
                            .font_size(14.0)
                            .gap(6.0)
                            .icon("trash-2", 16.0, DANGER),
                    )
                    .on_click(move |_, _, cx| {
                        edit_and_notify(&confirm_entity, cx, |draft| {
                            draft.backups.confirm_delete_id = None;
                        });
                    }),
                ),
            move |_, _, cx| {
                edit_and_notify(&close_entity, cx, |draft| {
                    draft.backups.confirm_delete_id = None;
                });
            },
        ));
    }

    if let Some(id) = backups.confirm_restore_id.clone() {
        let cancel_entity = entity.clone();
        let confirm_entity = entity.clone();
        let close_entity = entity;
        return Some(dialog_modal_full(
            448.0,
            "Restore Snapshot",
            Some(format!("Snapshot from {id}").into()),
            Some("triangle-alert"),
            IconTone::Warning,
            false,
            div().child(
                div()
                    .text_size(px(12.))
                    .line_height(relative(1.35))
                    .text_color(rgb(TEXT_MUTED))
                    .child("This will overwrite your current app settings and profiles with the data from this snapshot, then reload the app. This action cannot be undone."),
            ),
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(
                    button_with(
                        "backups-restore-cancel",
                        "Cancel",
                        ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Lg)
                            .padding_x(16.0)
                            .padding_y(0.0, 0.0)
                            .font_size(14.0),
                    )
                    .on_click(move |_, _, cx| {
                        edit_and_notify(&cancel_entity, cx, |draft| {
                            draft.backups.confirm_restore_id = None;
                        });
                    }),
                )
                .child(
                    button_with(
                        "backups-restore-confirm",
                        "Yes, restore",
                        ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Lg)
                            .padding_x(16.0)
                            .padding_y(0.0, 0.0)
                            .font_size(14.0)
                            .gap(6.0)
                            .hover(mix(ACCENT_SECONDARY, SURFACE_1, 0.24))
                            .icon("archive-restore", 16.0, ACCENT_SECONDARY),
                    )
                    // `accentSecondaryActionStyle92`: accent-secondary over surface-1.
                    .border_color(rgb(mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.58)))
                    .bg(rgb(mix(ACCENT_SECONDARY, SURFACE_1, 0.08)))
                    .text_color(rgb(ACCENT_SECONDARY))
                    .on_click(move |_, _, cx| {
                        edit_and_notify(&confirm_entity, cx, |draft| {
                            draft.backups.confirm_restore_id = None;
                        });
                    }),
                ),
            move |_, _, cx| {
                edit_and_notify(&close_entity, cx, |draft| {
                    draft.backups.confirm_restore_id = None;
                });
            },
        ));
    }

    None
}
