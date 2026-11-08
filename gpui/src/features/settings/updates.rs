use super::*;

// ── Updates (`UpdatesSettingsTab.tsx` + `UpdateCheckerSection.tsx`) ──────

pub(crate) fn updates_tab(_state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let mut card = div()
        .relative()
        .flex()
        .flex_1()
        .min_h(px(0.))
        .flex_col()
        .rounded(px(ROUNDED_LG)) // `rounded-lg border p-3` — 14px here
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(12.));

    // The gear is rendered only when `!isLinux`: Linux updates through Flatpak,
    // so there is no feed to choose (`UpdatesSettingsTab.tsx`, `isLinux` prop).
    if !cfg!(target_os = "linux") {
        let gear_entity = entity;
        card = card.child(
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
        );
    }

    let card = card
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
                                    // `color-mix(success, border-subtle 40%)` /
                                    // `color-mix(success, surface-1 82%)` —
                                    // 60% / 18% success.
                                    .border_color(rgb(mix(SUCCESS, BORDER_SUBTLE, 0.6)))
                                    .bg(rgb(mix(SUCCESS, SURFACE_1, 0.18)))
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
                    .child(
                        button_with(
                            "updates-check-again",
                            "Check Again",
                            ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Lg)
                                .padding_x(32.0) // px-8
                                .padding_y(0.0, 0.0)
                                .font_size(14.0)
                                .gap(6.0)
                                .icon("rotate-ccw", 16.0, TEXT_STRONG),
                        ),
                    ),
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
    let dev_entity = entity.clone();
    let close_entity = entity;

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
            .rounded(px(6.)) // `.ui-button` radius (--radius-sm)
            .border_1()
            // `activeChannelStyle`: `color-mix(accent, border-subtle 30%)`
            // (70% accent) over `color-mix(accent, surface-1 85%)` (15%).
            .border_color(rgb(if active {
                mix(ACCENT, BORDER_SUBTLE, 0.7)
            } else {
                BORDER_SUBTLE
            }))
            .bg(rgb(if active {
                mix(ACCENT, SURFACE_1, 0.15)
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
                .mt(px(12.))
                // `rounded-md border px-3 py-2.5` with
                // `color-mix(#f59e0b, border-subtle 40%)` (60% amber) over
                // `color-mix(#f59e0b, surface-1 90%)` (10%).
                .rounded(px(ROUNDED_MD))
                .border_1()
                .border_color(rgb(mix(0xf59e0b, BORDER_SUBTLE, 0.6)))
                .bg(rgb(mix(0xf59e0b, SURFACE_1, 0.1)))
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

    Some(dialog_modal_full(
        512.0, // max-w-lg
        "Release Channel",
        Some("Choose which update feed to check for new versions.".into()),
        Some("settings"),
        IconTone::Neutral,
        false,
        body,
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(
                button_with(
                    "updates-channel-cancel",
                    "Cancel",
                    ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Lg)
                        .padding_x(16.0)
                        .padding_y(0.0, 0.0)
                        .font_size(14.0),
                )
                .on_click(move |_, _, cx| {
                    edit_and_notify(&cancel_entity, cx, |draft| {
                        draft.updates.show_channel_settings = false;
                    });
                }),
            )
            .child({
                let mut spec = ButtonSpec::new(
                    if pending == updates.channel {
                        ButtonVariant::Secondary
                    } else if is_pending_dev_switch {
                        ButtonVariant::TintedWarning
                    } else {
                        ButtonVariant::TintedAccent
                    },
                    ButtonSize::Lg,
                )
                .padding_x(16.0)
                .padding_y(0.0, 0.0)
                .font_size(14.0);
                if is_pending_dev_switch {
                    spec = spec.gap(6.0).icon("flask-conical", 14.0, 0xf59e0b);
                }
                if pending == updates.channel {
                    // The source dims the current channel's Apply to 60%.
                    spec = spec.disabled();
                }
                button_with(
                    "updates-channel-apply",
                    if is_pending_dev_switch {
                        "Switch to Previews"
                    } else {
                        "Apply"
                    },
                    &spec,
                )
                .when(pending == updates.channel, |this| this.opacity(0.6))
                .on_click(move |_, _, cx| {
                    edit_and_notify(&apply_entity, cx, |draft| {
                        if draft.updates.pending_channel != draft.updates.channel {
                            draft.updates.channel = draft.updates.pending_channel;
                        }
                        draft.updates.show_channel_settings = false;
                    });
                })
            }),
        move |_, _, cx| {
            edit_and_notify(&close_entity, cx, |draft| {
                draft.updates.show_channel_settings = false;
            });
        },
    ))
}
