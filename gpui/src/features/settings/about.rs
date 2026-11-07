use super::*;

// ── About (`SettingsModal.tsx`) ──────────────────────────────────────────

/// `sponsorInitials` — first letter of the first and last word, with a
/// camelCase split so `WickedGrey` → `WG` and `Kryz Slaughter` → `KS`.
pub(crate) fn sponsor_initials(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut spaced = String::with_capacity(name.len() + 4);
    for (index, ch) in chars.iter().enumerate() {
        if index > 0 && ch.is_uppercase() {
            let previous = chars[index - 1];
            if previous.is_lowercase() || previous.is_numeric() {
                spaced.push(' ');
            }
        }
        spaced.push(*ch);
    }
    let letters: Vec<char> = spaced
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .filter_map(|word| word.chars().next())
        .filter(|ch| ch.is_alphabetic())
        .collect();
    if letters.is_empty() {
        return "?".to_owned();
    }
    let picked: Vec<char> = if letters.len() == 1 {
        vec![letters[0]]
    } else {
        vec![letters[0], letters[letters.len() - 1]]
    };
    picked.iter().collect::<String>().to_uppercase()
}

/// `getSponsorRank` — the tier ring colour; `supporters` have no ring.
pub(crate) fn sponsor_ring(total: f32) -> (u32, bool) {
    if total >= 100.0 {
        (0xD4AF37, true) // gold
    } else if total >= 30.0 {
        (0xC0C0C0, true) // silver
    } else if total >= 10.0 {
        (0xCD7F32, true) // bronze
    } else {
        (BORDER_SUBTLE, false)
    }
}

pub(crate) fn about_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let _ = entity;
    let copied = state.about.copied;

    let is_beta_build_channel = DRAGONFRUIT_BUILD_CHANNEL.contains("beta");
    let build_status_label = if is_beta_build_channel {
        "BETA VERSION".to_owned()
    } else if DRAGONFRUIT_BUILD_CHANNEL == "mainline" {
        "Mainline Build".to_owned()
    } else {
        format!("{} Build", DRAGONFRUIT_BUILD_CHANNEL.to_uppercase())
    };
    let (build_status_text, build_status_border, build_status_bg) = if is_beta_build_channel {
        (0xfdba74, mix(0xf97316, BORDER_SUBTLE, 0.16), rgba(with_alpha(0xf97316, 0x0A)))
    } else {
        (
            TEXT_STRONG,
            mix(ACCENT, BORDER_SUBTLE, 0.4),
            rgba(with_alpha(ACCENT, 0x29)),
        )
    };

    // `/dragonfruit_assets/branding/text_logo.svg` is not in the GPUI asset
    // set, so the wordmark renders as text (see the module note).
    let hero = div()
        .rounded(px(12.)) // rounded-xl
        .p(px(16.))
        .bg(linear_gradient(
            145.,
            linear_color_stop(rgb(mix(ACCENT, SURFACE_0, 0.95)), 0.),
            linear_color_stop(rgb(mix(ACCENT_SECONDARY, SURFACE_0, 0.94)), 1.),
        ))
        .child(
            div()
                .relative()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_size(px(28.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("DragonFruit"),
                )
                .child(
                    div()
                        .absolute()
                        .right(px(0.))
                        .top(px(0.))
                        .rounded_full()
                        .px(px(10.))
                        .py(px(2.))
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(WHITE))
                        .bg(linear_gradient(
                            135.,
                            linear_color_stop(rgb(0x3b0764), 0.),
                            linear_color_stop(rgb(0x9a3412), 1.),
                        ))
                        .child("An Open Resin Alliance Project"),
                ),
        )
        .child(
            div()
                .mt(px(12.))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_center()
                        .gap(px(10.))
                        .child(
                            div()
                                .rounded_full()
                                .border_1()
                                .px(px(10.))
                                .py(px(2.))
                                .text_size(px(12.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(if copied { 0x2d8a4e } else { TEXT_STRONG }))
                                .border_color(rgb(if copied {
                                    0x2d8a4e
                                } else {
                                    mix(BORDER_SUBTLE, WHITE, 0.92)
                                }))
                                .bg(if copied {
                                    rgba(with_alpha(0x2d8a4e, 0x1a))
                                } else {
                                    rgba(with_alpha(SURFACE_1, 0xEB))
                                })
                                .child(if copied {
                                    "✓ Copied!".to_owned()
                                } else {
                                    format!("Version {DRAGONFRUIT_VERSION}")
                                }),
                        )
                        .child(
                            div()
                                .rounded_full()
                                .border_1()
                                .px(px(10.))
                                .py(px(2.))
                                .text_size(px(12.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(build_status_text))
                                .border_color(rgb(build_status_border))
                                .bg(build_status_bg)
                                .child(build_status_label),
                        ),
                ),
        );

    // Team & Credits — `contributors` sorted by tone rank, then name.
    let mut people: Vec<(&str, &str, &str)> = CONTRIBUTORS.to_vec();
    people.sort_by(|a, b| {
        let rank = |tone: &str| match tone {
            "founder" => 0,
            "accent" => 1,
            _ => 2,
        };
        rank(a.2).cmp(&rank(b.2)).then_with(|| a.0.cmp(b.0))
    });

    let mut groups = div().mt(px(10.)).flex().flex_col().gap(px(8.));
    for (index, tone) in ["founder", "accent", "secondary"].iter().enumerate() {
        let mut grid = div().grid().grid_cols(2).gap(px(8.));
        let mut count = 0usize;
        for &(name, affiliation, person_tone) in people.iter() {
            if person_tone != *tone {
                continue;
            }
            count += 1;
            let (tone_color, background_mix) = match *tone {
                "founder" => (0xd4a017, 0.94),
                "accent" => (ACCENT, 0.90),
                _ => (ACCENT_SECONDARY, 0.93),
            };
            grid = grid.child(
                div()
                    .rounded(px(6.))
                    .border_1()
                    .border_color(rgb(mix(tone_color, BORDER_SUBTLE, 0.45)))
                    .bg(rgb(mix(tone_color, SURFACE_0, background_mix)))
                    .px(px(12.))
                    .py(px(10.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .text_size(px(14.))
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .truncate()
                                    .text_color(rgb(TEXT_STRONG))
                                    .child(name),
                            )
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .flex()
                                    .items_center()
                                    .gap(px(2.))
                                    .text_color(rgb(TEXT_MUTED))
                                    .child(icon("github", 12., TEXT_MUTED))
                                    .child(affiliation),
                            ),
                    ),
            );
        }
        if count == 0 {
            continue;
        }
        if index > 0 {
            groups = groups.child(
                div()
                    .my(px(2.))
                    .h(px(1.))
                    .rounded_full()
                    .bg(linear_gradient(
                        90.,
                        linear_color_stop(rgba(0x00000000), 0.),
                        linear_color_stop(rgb(BORDER_SUBTLE), 1.),
                    )),
            );
        }
        groups = groups.child(grid);
    }

    let team = div()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(12.))
        .child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .child("TEAM & CREDITS"),
        )
        .child(groups);

    // Sponsors — the baked-in list, monogram-only (no remote avatars).
    let mut sponsor_row = div().mt(px(8.)).flex().flex_wrap().items_center().gap(px(12.));
    for &(name, _link, total) in SPONSORS.iter() {
        let (ring, ringed) = sponsor_ring(total);
        let initials = sponsor_initials(name);
        let mut circle = div()
            .w(px(48.))
            .h(px(48.))
            .flex_shrink_0()
            .rounded_full()
            .flex()
            .items_center()
            .justify_center();
        if ringed {
            circle = circle.bg(rgb(ring));
        }
        circle = circle.child(
            div()
                .w(px(42.))
                .h(px(42.))
                .rounded_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(rgb(SURFACE_1))
                .child(
                    div()
                        .text_size(px(if initials.chars().count() > 1 { 15. } else { 19. }))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(if ringed { ring } else { ACCENT }))
                        .child(initials),
                ),
        );
        sponsor_row = sponsor_row.child(circle);
    }

    let sponsors = div()
        .flex_shrink_0()
        .rounded(px(8.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .px(px(12.))
        .py(px(8.))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_MUTED))
                        .child(icon("heart", 12., ACCENT_SECONDARY))
                        .child("SPONSORS")
                        .child(
                            div()
                                .text_size(px(11.))
                                .font_weight(FontWeight::NORMAL)
                                .text_color(rgb(TEXT_MUTED))
                                .opacity(0.85)
                                .child("via Open Collective"),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .text_size(px(10.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(ACCENT))
                        .child("Sponsor Us")
                        .child(icon("heart", 12., ACCENT)),
                ),
        )
        .child(sponsor_row);

    // ORA footer — the logos are not in the GPUI asset set, so the left mark is
    // the organisation's name and the trailing app icon is omitted.
    let ora = div()
        .flex()
        .items_center()
        .gap(px(16.))
        .rounded(px(12.))
        .border_1()
        .border_color(rgb(mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.52)))
        .bg(rgb(mix(ACCENT_SECONDARY, SURFACE_0, 0.94)))
        .px(px(16.))
        .py(px(12.))
        .child(
            div()
                .w(px(96.))
                .flex_shrink_0()
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(ACCENT_SECONDARY))
                .child("Open Resin Alliance"),
        )
        .child(
            div()
                .min_w(px(0.))
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(px(8.))
                        .text_size(px(12.))
                        .child(icon("github", 14., ACCENT))
                        .child(
                            div()
                                .text_color(rgb(ACCENT))
                                .child("Open-Resin-Alliance/DragonFruit"),
                        )
                        .child(icon("external-link", 12., ACCENT)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(px(8.))
                        .text_size(px(12.))
                        .text_color(rgb(TEXT_STRONG))
                        .child(icon("scroll-text", 14., ACCENT))
                        .child("AGPL-3.0-or-later"),
                ),
        );

    div()
        .flex_1()
        .min_h(px(0.))
        .flex()
        .flex_col()
        .gap(px(14.))
        .child(
            div()
                .id("about-scroll")
                .flex_1()
                .min_h(px(0.))
                .overflow_y_scroll()
                .pr(px(4.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(14.))
                        .pb(px(8.))
                        .child(hero)
                        .child(team),
                ),
        )
        .child(sponsors)
        .child(ora)
}
