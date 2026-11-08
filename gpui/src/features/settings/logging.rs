use super::*;

// ── Logging (`LoggingSettingsTab.tsx`) ───────────────────────────────────

pub(crate) fn logging_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let logging = &state.logging;
    let level_entity = entity.clone();
    let noise_entity = entity.clone();
    let filter_entity = entity.clone();
    let filter_row_entity = entity.clone();
    let live_entity = entity;

    // `resolveLogDirPath()` reads the Tauri app-log directory; without the
    // bridge `logPath` stays null and the path cell keeps the source's
    // "Resolving path…" state below.

    let header = |icon_slug: &'static str, title: &'static str, description: &'static str| {
        div()
            .flex()
            .items_start()
            .gap(px(8.))
            .mb(px(12.))
            .child(icon_chip(icon_slug))
            .child(
                div()
                    .flex_1()
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
                    ),
            )
    };

    let mut root = div().flex().flex_col().gap(px(14.));

    // Log file location
    root = root.child(
        div()
            .rounded(px(12.)) // rounded-xl
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1))
            .p(px(16.))
            .child(header(
                "scroll-text",
                "Log File",
                "DragonFruit writes structured logs to a platform-specific directory. Share this file when reporting startup or network issues.",
            ))
            .child(
                div()
                    // `rounded-lg border flex items-stretch overflow-hidden`
                    // on `--surface-2`.
                    .rounded(px(ROUNDED_LG))
                    .border_1()
                    .border_color(rgb(BORDER_SUBTLE))
                    .bg(rgb(SURFACE_2))
                    .flex()
                    .items_stretch()
                    .overflow_hidden()
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .px(px(12.))
                            .py(px(10.))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            // `logPath === null`: the source shows a spinner
                            // and `Resolving path…` (`LoggingSettingsTab.tsx`),
                            // which is the state a build without the app-log
                            // bridge stays in.
                            .child(spinner(12.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgb(TEXT_MUTED))
                                    .truncate()
                                    .child("Resolving path…"),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .border_l_1()
                            .border_color(rgb(BORDER_SUBTLE))
                            .child(
                                // Copy path — the clipboard is unreachable in
                                // this slice, so the tile is inert.
                                div()
                                    .px(px(12.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(icon(
                                        if logging.copied {
                                            "check"
                                        } else {
                                            "clipboard-copy"
                                        },
                                        14.,
                                        if logging.copied { ACCENT } else { TEXT_MUTED },
                                    )),
                            ),
                    ),
            ),
    );

    // Minimum log level
    let mut level_rows = div().flex().flex_col().gap(px(6.));
    for &(value, label, description, color) in LOG_LEVEL_OPTIONS.iter() {
        let active = logging.level == value;
        let row_entity = level_entity.clone();
        let mut row = div()
            .id(format!("log-level-{value}"))
            .flex()
            .items_center()
            .gap(px(12.))
            // `rounded-lg border px-3 py-2`; active is
            // `color-mix(color, border-subtle 40%)` (60% colour) over
            // `color-mix(color, surface-0 90%)` (10% colour).
            .rounded(px(ROUNDED_LG))
            .border_1()
            .px(px(12.))
            .py(px(8.))
            .border_color(rgb(if active {
                mix(color, BORDER_SUBTLE, 0.6)
            } else {
                BORDER_SUBTLE
            }))
            .bg(rgb(if active {
                mix(color, SURFACE_0, 0.1)
            } else {
                SURFACE_2
            }))
            .cursor_pointer()
            .on_click(move |_, _, cx| {
                edit_and_notify(&row_entity, cx, |draft| draft.logging.level = value);
            })
            .child(
                div()
                    .w(px(8.))
                    .h(px(8.))
                    .flex_shrink_0()
                    .rounded_full()
                    .bg(rgb(color)),
            )
            .child(
                div()
                    .min_w(px(0.))
                    .flex_1()
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(TEXT_STRONG))
                            .child(label),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(TEXT_MUTED))
                            .child(description),
                    ),
            );
        if active {
            row = row.child(icon("check", 14., color));
        }
        level_rows = level_rows.child(row);
    }
    root = root.child(
        div()
            .rounded(px(12.))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1))
            .p(px(16.))
            .child(header(
                "funnel", // `Filter`
                "Minimum Log Level",
                "Controls the least-significant event written to the log file. Changes apply immediately and persist across restarts.",
            ))
            .child(level_rows),
    );

    // Live log viewer
    let filter_meta = LOG_LEVEL_OPTIONS
        .iter()
        .find(|(value, _, _, _)| *value == logging.viewer_filter);
    let filter_color = filter_meta.map(|(_, _, _, color)| *color).unwrap_or(0x6b7280);
    let filter_label = filter_meta.map(|(_, label, _, _)| *label).unwrap_or("?");

    let mut filter_menu = div()
        .absolute()
        .top(px(26.))
        .right(px(0.))
        .min_w(px(100.))
        // `rounded-lg border shadow-lg overflow-hidden min-w-[100px]` on
        // `--surface-2`.
        .rounded(px(ROUNDED_LG))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_2))
        .overflow_hidden();
    for &(value, label, _, color) in LOG_LEVEL_OPTIONS.iter() {
        let active = logging.viewer_filter == value;
        let row_entity = filter_row_entity.clone();
        filter_menu = filter_menu.child(
            div()
                .id(format!("log-filter-{value}"))
                .w_full()
                .flex()
                .items_center()
                .gap(px(8.))
                .px(px(12.))
                .py(px(6.))
                .text_size(px(12.))
                .text_color(rgb(TEXT_STRONG))
                // `color-mix(color, surface-0 85%)` — 15% colour.
                .when(active, |this| this.bg(rgb(mix(color, SURFACE_0, 0.15))))
                .cursor_pointer()
                .on_click(move |_, _, cx| {
                    edit_and_notify(&row_entity, cx, |draft| {
                        draft.logging.viewer_filter = value;
                        draft.logging.filter_open = false;
                    });
                })
                .child(
                    div()
                        .w(px(6.))
                        .h(px(6.))
                        .flex_shrink_0()
                        .rounded_full()
                        .bg(rgb(color)),
                )
                .child(label)
                .when(active, |this| {
                    this.child(div().flex_1()).child(icon("check", 12., color))
                }),
        );
    }

    let mut toolbar = div()
        .flex()
        .items_center()
        .gap(px(8.))
        .px(px(12.))
        .py(px(8.))
        .border_b_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_2))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .flex_1()
                .min_w(px(0.))
                .child(
                    div()
                        .w(px(6.))
                        .h(px(6.))
                        .flex_shrink_0()
                        .rounded_full()
                        .bg(rgb(if logging.is_live { 0x4ade80 } else { 0x6b7280 })),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("Live Log"),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(TEXT_MUTED))
                        .child("0 lines"),
                ),
        )
        .child(
            div()
                .id("log-noise")
                .flex()
                .items_center()
                .gap(px(6.))
                .rounded(px(ROUNDED_MD)) // `rounded-md` toolbar chrome
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .px(px(8.))
                .py(px(4.))
                .text_size(px(11.))
                .text_color(rgb(if logging.hide_noise {
                    TEXT_MUTED
                } else {
                    0xfb923c
                }))
                .when(!logging.hide_noise, |this| {
                    this.bg(rgb(mix(0xfb923c, SURFACE_0, 0.88)))
                })
                .cursor_pointer()
                .on_click(move |_, _, cx| {
                    edit_and_notify(&noise_entity, cx, |draft| {
                        draft.logging.hide_noise = !draft.logging.hide_noise;
                    });
                })
                .child(if logging.hide_noise {
                    "noise hidden"
                } else {
                    "noise shown"
                }),
        )
        .child(
            div()
                .relative()
                .child(
                    div()
                        .id("log-filter")
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .rounded(px(ROUNDED_MD)) // `rounded-md` toolbar chrome
                        .border_1()
                        .border_color(rgb(mix(filter_color, BORDER_SUBTLE, 0.5)))
                        // `color-mix(filterColor, surface-0 88%)` — 12% colour.
                        .bg(rgb(mix(filter_color, SURFACE_0, 0.12)))
                        .px(px(8.))
                        .py(px(4.))
                        .text_size(px(12.))
                        .text_color(rgb(TEXT_STRONG))
                        .cursor_pointer()
                        .on_click(move |_, _, cx| {
                            edit_and_notify(&filter_entity, cx, |draft| {
                                draft.logging.filter_open = !draft.logging.filter_open;
                            });
                        })
                        .child(
                            div()
                                .w(px(6.))
                                .h(px(6.))
                                .flex_shrink_0()
                                .rounded_full()
                                .bg(rgb(filter_color)),
                        )
                        .child(filter_label)
                        .child(
                            div()
                                .opacity(0.6)
                                .child(icon("chevron-down", 12., TEXT_STRONG)),
                        ),
                )
                .when(logging.filter_open, |this| this.child(filter_menu)),
        )
        .child(
            div()
                .id("log-live")
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(ROUNDED_MD)) // `rounded-md` toolbar chrome
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .px(px(8.))
                .py(px(4.))
                .cursor_pointer()
                .on_click(move |_, _, cx| {
                    edit_and_notify(&live_entity, cx, |draft| {
                        draft.logging.is_live = !draft.logging.is_live;
                    });
                })
                .child(icon(
                    if logging.is_live { "pause" } else { "play" },
                    12.,
                    TEXT_MUTED,
                )),
        );
    if !logging.is_live {
        // Manual refresh — `read_log_tail` needs the native bridge.
        toolbar = toolbar.child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(ROUNDED_MD)) // `rounded-md` toolbar chrome
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .px(px(8.))
                .py(px(4.))
                .child(icon("refresh-ccw", 12., TEXT_MUTED)),
        );
    }
    toolbar = toolbar.child(
        div()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(ROUNDED_MD)) // `rounded-md` toolbar chrome
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .px(px(8.))
            .py(px(4.))
            .child(icon("trash-2", 12., TEXT_MUTED)),
    );

    root = root.child(
        div()
            .rounded(px(12.))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1))
            .overflow_hidden()
            .child(toolbar)
            .child(
                div()
                    .h(px(320.))
                    .bg(rgb(SURFACE_0))
                    .px(px(12.))
                    .py(px(8.))
                    .text_size(px(12.))
                    .child(
                        div()
                            .text_color(rgb(0x6b7280))
                            .child("Log viewer only available in the desktop build."),
                    ),
            ),
    );

    // Info callout
    root = root.child(
        div()
            // `rounded-lg border px-3 py-2.5` with
            // `color-mix(accent, border-subtle 60%)` → accent 40%.
            .rounded(px(ROUNDED_LG))
            .border_1()
            .border_color(rgb(mix(ACCENT, BORDER_SUBTLE, 0.4)))
            .bg(rgb(mix(ACCENT, SURFACE_0, 0.06)))
            .px(px(12.))
            .py(px(10.))
            .flex()
            .items_start()
            .gap(px(10.))
            .child(icon("info", 14., ACCENT))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_wrap()
                    .text_size(px(12.))
                    .line_height(relative(1.35))
                    .text_color(rgb(TEXT_MUTED))
                    .child("The minimum log level applies ")
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(TEXT_STRONG))
                            .child("immediately"),
                    )
                    .child(" and is also saved for next launch. The viewer filter and noise suppression are local-only — they do not affect what is written to disk."),
            ),
    );

    root
}
