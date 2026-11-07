use super::*;

// ── Experiments (`ExperimentsSettingsTab.tsx`) ───────────────────────────

pub(crate) fn experiments_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let mut root = div().flex().flex_col().gap(px(12.));
    root = root.child(
        div()
            .rounded(px(6.))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(mix(SURFACE_1, SURFACE_0, 0.94)))
            .p(px(12.))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(8.))
                    .child(
                        icon_chip_with(
                            IconChipSize::Md,
                            IconTone::Neutral,
                            icon("flask-conical", 16., ACCENT_SECONDARY),
                        )
                        // color-mix(in srgb, surface-2, transparent 8%) over surface-1
                        .bg(rgb(mix(SURFACE_2, SURFACE_1, 0.92))),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(TEXT_STRONG))
                                    .child("Experiments"),
                            )
                            .child(
                                div()
                                    .mt(px(2.))
                                    .text_size(px(12.))
                                    .line_height(relative(1.2))
                                    .text_color(rgb(TEXT_MUTED))
                                    .child("Early-access features that are still in testing. They may change or disappear without notice."),
                            ),
                    ),
            ),
    );

    for (id, name, description, default_enabled) in EXPERIMENTS.iter() {
        let toggle_entity = entity.clone();
        let enabled = state
            .experiments
            .enabled
            .get(id)
            .copied()
            .unwrap_or(*default_enabled);
        root = root.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.))
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_1))
                .px(px(10.))
                .py(px(8.))
                .child(
                    div()
                        .min_w(px(0.))
                        .flex_1()
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child(*name),
                        )
                        .child(
                            div()
                                .mt(px(2.))
                                .text_size(px(12.))
                                .line_height(relative(1.2))
                                .text_color(rgb(TEXT_MUTED))
                                .child(*description),
                        ),
                )
                .child(segmented(
                    "experiment",
                    &["OFF", "ON"],
                    usize::from(enabled),
                    false,
                    move |index, cx| {
                        edit_and_notify(&toggle_entity, cx, |draft| {
                            draft.experiments.enabled.insert(id, index == 1);
                        });
                    },
                )),
        );
    }

    root.child(
        div()
            .rounded(px(6.))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1))
            .px(px(12.))
            .py(px(8.))
            .text_size(px(12.))
            .text_color(rgb(TEXT_MUTED))
            .child("Experiment changes apply after you restart DragonFruit or reload the window."),
    )
}

/// The ORA disclaimer gate, shown before the tab content.
pub(crate) fn experiments_overlay(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Option<Div> {
    if !state.experiments.show_disclaimer {
        return None;
    }
    let exit_entity = entity.clone();
    let ack_entity = entity;

    Some(
        div()
            .absolute()
            .top(px(0.))
            .left(px(0.))
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .px(px(12.))
            .bg(rgba(0x0000008c))
            .child(
                div()
                    .w(px(512.)) // max-w-lg
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .rounded(px(12.))
                    .border_1()
                    .border_color(rgb(BORDER_SUBTLE))
                    .bg(rgb(SURFACE_0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(16.))
                            .border_b_1()
                            .border_color(rgb(BORDER_SUBTLE))
                            .px(px(20.))
                            .py(px(16.))
                            .child(icon_chip_with(
                                IconChipSize::Lg,
                                IconTone::Warning,
                                icon("triangle-alert", 16., 0xd97706),
                            ))
                            .child(
                                div()
                                    .min_w(px(0.))
                                    .child(
                                        div()
                                            .text_size(px(16.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .line_height(relative(1.15))
                                            .text_color(rgb(TEXT_STRONG))
                                            .child("Experimental Features"),
                                    )
                                    .child(
                                        div()
                                            .mt(px(2.))
                                            .text_size(px(11.))
                                            .line_height(relative(1.2))
                                            .text_color(rgb(TEXT_MUTED))
                                            .child("Use at your own risk."),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(16.))
                            .p(px(20.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(relative(1.6))
                                    .text_color(rgb(TEXT_MUTED))
                                    .child("Experiments are early-access features still under active development. They are provided \"as is\" without any warranty, and without guarantee of correctness, reliability, or compatibility."),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(relative(1.6))
                                    .text_color(rgb(TEXT_MUTED))
                                    .child("The Open Resin Alliance and its contributors accept no responsibility or liability for any loss, damage, or data loss arising from the use of experimental features."),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(relative(1.6))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0xfca5a5))
                                    .child("Enable and use them at your own risk."),
                            )
                            .child(
                                div()
                                    .grid()
                                    .grid_cols(2)
                                    .gap(px(8.))
                                    .pt(px(4.))
                                    .child(
                                        button_with(
                                            "experiments-back",
                                            "Take me back!",
                                            ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Lg)
                                                .full()
                                                .padding_x(16.0)
                                                .padding_y(0.0, 0.0)
                                                .font_size(14.0),
                                        )
                                        .on_click(move |_, _, cx| {
                                            // "Take me back" keeps the gate for the
                                            // next entry, so it can reappear.
                                            edit_and_notify(&exit_entity, cx, |draft| {
                                                draft.experiments.show_disclaimer = false;
                                            });
                                        }),
                                    )
                                    .child(
                                        button_with(
                                            "experiments-ack",
                                            "I understand",
                                            ButtonSpec::new(
                                                ButtonVariant::TintedWarning,
                                                ButtonSize::Lg,
                                            )
                                            .full()
                                            .padding_x(16.0)
                                            .padding_y(0.0, 0.0)
                                            .font_size(14.0),
                                        )
                                        .on_click(move |_, _, cx| {
                                            edit_and_notify(&ack_entity, cx, |draft| {
                                                draft.experiments.disclaimer_seen = true;
                                                draft.experiments.show_disclaimer = false;
                                            });
                                        }),
                                    ),
                            ),
                    ),
            ),
    )
}
