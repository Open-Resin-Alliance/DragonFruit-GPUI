use super::*;

// ── Updates (`UpdatesSettingsTab.tsx` + `UpdateCheckerSection.tsx`) ──────

pub(crate) fn updates_tab(_state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let gear_entity = entity;

    let card = div()
        .relative()
        .flex()
        .flex_1()
        .min_h(px(0.))
        .flex_col()
        .rounded(px(8.)) // rounded-lg
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(12.))
        .child(
            // `IconButton` absolute right-3 top-3, accent tone.
            div()
                .absolute()
                .top(px(12.))
                .right(px(12.))
                .child(
                    icon_button_toned(
                        "updates-channel-settings",
                        "settings",
                        16.0,
                        IconButtonVariant::Surface,
                        IconButtonSize::Sm,
                        IconTone::Accent,
                        false,
                    )
                    .on_click(move |_, _, cx| {
                        edit_and_notify(&gear_entity, cx, |draft| {
                            draft.updates.show_channel_settings = true;
                        });
                    }),
                ),
        )
        .child(
            div().flex().min_h(px(0.)).flex_1().flex_col().child(
                // `useUpdateChecker` auto-checks on mount; without the update
                // bridge `fetchUpdateInfo` answers `null`, which the source
                // renders as its own up-to-date state.
                div()
                    .flex()
                    .flex_1()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(32.))
                    .py(px(64.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(12.))
                            .text_center()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .w(px(48.))
                                    .h(px(48.))
                                    .flex_shrink_0()
                                    .rounded(px(12.))
                                    .border_1()
                                    .border_color(rgb(mix(SUCCESS, BORDER_SUBTLE, 0.4)))
                                    .bg(rgb(mix(SUCCESS, SURFACE_1, 0.82)))
                                    .child(icon("check", 24., SUCCESS)),
                            )
                            .child(
                                div()
                                    .min_w(px(0.))
                                    .child(
                                        div()
                                            .text_size(px(18.))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(SUCCESS))
                                            .child("Up to date"),
                                    )
                                    .child(
                                        div()
                                            .mt(px(6.))
                                            .text_size(px(14.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(rgb(TEXT_MUTED))
                                            .child("You’re on the latest version"),
                                    ),
                            ),
                    )
                    .child(action_button(
                        "updates-check-again",
                        "Check Again",
                        Some("rotate-ccw"),
                        32.0, // px-8
                        SURFACE_1,
                        BORDER_SUBTLE,
                        TEXT_STRONG,
                        SURFACE_2,
                        |_| {},
                    )),
            ),
        );

    div()
        .flex_1()
        .min_h(px(0.))
        .flex()
        .flex_col()
        .gap(px(12.))
        .child(card)
}

/// The `Release Channel` dialog.
pub(crate) fn updates_overlay(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Option<Div> {
    if !state.updates.show_channel_settings {
        return None;
    }
    let updates = &state.updates;
    let pending = updates.pending_channel;
    let is_pending_dev_switch = pending == UpdateChannel::Dev && updates.channel != UpdateChannel::Dev;

    let cancel_entity = entity.clone();
    let apply_entity = entity.clone();
    let stable_entity = entity.clone();
    let dev_entity = entity;

    let channel_card = |id: &'static str,
                        active: bool,
                        slug: &'static str,
                        title: &'static str,
                        subtitle: &'static str,
                        target: UpdateChannel,
                        card_entity: WeakEntity<DragonFruit>| {
        div()
            .id(id)
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(6.))
            .px(px(8.))
            .py(px(24.))
            .min_h(px(64.))
            .rounded(px(6.))
            .border_1()
            .border_color(rgb(if active {
                mix(ACCENT, BORDER_SUBTLE, 0.3)
            } else {
                BORDER_SUBTLE
            }))
            .bg(rgb(if active {
                mix(ACCENT, SURFACE_1, 0.85)
            } else {
                SURFACE_1
            }))
            .cursor_pointer()
            .on_click(move |_, _, cx| {
                edit_and_notify(&card_entity, cx, |draft| {
                    draft.updates.pending_channel = target;
                });
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(TEXT_STRONG))
                    .child(icon(slug, 14., TEXT_STRONG))
                    .child(title),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(TEXT_MUTED))
                    .child(subtitle),
            )
    };

    let mut body = div()
        .flex()
        .flex_col()
        .gap(px(10.))
        .child(channel_card(
            "updates-channel-stable",
            pending == UpdateChannel::Stable,
            "shield-check",
            "Stable Builds",
            "Recommended · Production",
            UpdateChannel::Stable,
            stable_entity,
        ))
        .child(channel_card(
            "updates-channel-dev",
            pending == UpdateChannel::Dev,
            "flask-conical",
            "Development Previews",
            "Early access · May be unstable",
            UpdateChannel::Dev,
            dev_entity,
        ));

    if is_pending_dev_switch {
        body = body.child(
            div()
                .mt(px(2.))
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(mix(0xf59e0b, BORDER_SUBTLE, 0.4)))
                .bg(rgb(mix(0xf59e0b, SURFACE_1, 0.9)))
                .px(px(12.))
                .py(px(10.))
                .flex()
                .items_start()
                .gap(px(10.))
                .child(icon("triangle-alert", 16., 0xf59e0b))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_wrap()
                        .text_size(px(12.))
                        .line_height(relative(1.5))
                        .text_color(rgb(TEXT_MUTED))
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child("Development Previews"),
                        )
                        .child(" are built from the ")
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child("dev"),
                        )
                        .child(" branch for early testing. Expect bugs and breaking changes — switch back to Stable Builds at any time."),
                ),
        );
    }

    Some(dialog_shell(
        512.0, // max-w-lg
        "settings",
        (TEXT_MUTED, BORDER_SUBTLE, SURFACE_1),
        "Release Channel",
        Some("Choose which update feed to check for new versions.".into()),
        body,
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(action_button(
                "updates-channel-cancel",
                "Cancel",
                None,
                16.0,
                SURFACE_1,
                BORDER_SUBTLE,
                TEXT_STRONG,
                SURFACE_2,
                move |cx| {
                    edit_and_notify(&cancel_entity, cx, |draft| {
                        draft.updates.show_channel_settings = false;
                    });
                },
            ))
            .child(
                action_button(
                    "updates-channel-apply",
                    if is_pending_dev_switch {
                        "Switch to Previews"
                    } else {
                        "Apply"
                    },
                    if is_pending_dev_switch {
                        Some("flask-conical")
                    } else {
                        None
                    },
                    16.0,
                    if pending == updates.channel {
                        SURFACE_1
                    } else if is_pending_dev_switch {
                        mix(0xf59e0b, SURFACE_1, 0.86)
                    } else {
                        mix(ACCENT, SURFACE_1, 0.14)
                    },
                    if pending == updates.channel {
                        BORDER_SUBTLE
                    } else if is_pending_dev_switch {
                        mix(0xf59e0b, BORDER_SUBTLE, 0.45)
                    } else {
                        mix(ACCENT, BORDER_SUBTLE, 0.55)
                    },
                    if pending == updates.channel {
                        TEXT_MUTED
                    } else if is_pending_dev_switch {
                        0xf59e0b
                    } else {
                        ACCENT
                    },
                    if pending == updates.channel {
                        SURFACE_2
                    } else if is_pending_dev_switch {
                        mix(0xf59e0b, SURFACE_1, 0.76)
                    } else {
                        mix(ACCENT, SURFACE_1, 0.24)
                    },
                    move |cx| {
                        edit_and_notify(&apply_entity, cx, |draft| {
                            if draft.updates.pending_channel != draft.updates.channel {
                                draft.updates.channel = draft.updates.pending_channel;
                            }
                            draft.updates.show_channel_settings = false;
                        });
                    },
                )
                .when(pending == updates.channel, |this| this.opacity(0.6)),
            ),
    ))
}
