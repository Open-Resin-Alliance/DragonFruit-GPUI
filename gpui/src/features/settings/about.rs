//! About — `SettingsModal.tsx:1583` (`activeTab === 'about'`) plus
//! `SponsorsCarousel.tsx`.
//!
//! The source branch is one flex column of five blocks, in this order:
//!
//! 1. the gradient hero (`rounded-xl p-4`, `linear-gradient(145deg, …)`) with
//!    the `text_logo.svg` wordmark, the ORA corner badge, and the version +
//!    build-status pills;
//! 2. the "Team & Credits" card (`rounded-md border p-3`, `surface-1`) with the
//!    contributors grouped by tone and sorted by name inside each group;
//! 3. `<SponsorsCarousel />` (its own `rounded-lg border px-3 py-2` card, also
//!    `surface-1`);
//! 4. the ORA footer card (`rounded-xl border px-4 py-3`, accent-secondary tint)
//!    with the ORA mark, the repository link and the licence.
//!
//! Blocks 1-2 live in the tab's own scroller (`flex-1 min-h-0 overflow-y-auto
//! custom-scrollbar pr-1` → `space-y-3.5 pb-2`, `SettingsModal.tsx:1585-1586`);
//! 3-4 are siblings below it, so they stay pinned while the credits scroll —
//! that is the `flex h-full min-h-0 flex-col gap-3.5` at `SettingsModal.tsx:1584`.
//!
//! The tab has no `<Card>`/`<CardHeader>` anywhere, so `ui::card` (surface-0,
//! `shadow-lg`) is deliberately not used: every block is the source's own
//! inline-styled div, and its radius/background/padding are the class strings.
//!
//! PORT DEVIATIONS (About-specific; the shared ones are listed in `mod.rs`)
//! - **Image assets.** `text_logo.svg`, `simple_icon.svg` and
//!   `open_resin_alliance_logo_darkmode.png` are not in the GPUI asset set
//!   (`gpui/assets` holds fonts and icons only). They *do* exist in the web
//!   build's tree, at `public/dragonfruit_assets/branding/`, and are loaded from
//!   there by compile-time-anchored path — the same trick `icons::icon` uses for
//!   `gpui/assets/icons`. Nothing is missing and nothing is substituted: the
//!   old text stand-in for the wordmark is gone.
//! - **Sponsors are the baked-in fallback list, fetched never.** The source
//!   paints `sponsors.json` first and then replaces it with a live Open
//!   Collective fetch (two collectives, 6 h `localStorage` cache, per-avatar
//!   blob cache, `SponsorsCarousel.tsx:74-360`). The port has no HTTP client, so
//!   it renders exactly the fallback list the source uses before/without a
//!   fetch — the same 18 sponsors, same order, same amounts — through
//!   [`SPONSORS`]. The loading branch (`sponsors === null`) and the
//!   "No sponsors yet" empty branch are therefore unreachable and not spelled:
//!   the list is never empty, so "no spinner forever" is not a state this tab
//!   can enter.
//! - **Avatars.** The fallback entries carry `image` URLs for 12 of the 18
//!   sponsors; loading them needs the network, so every sponsor takes the
//!   source's own monogram branch (`SponsorsCarousel.tsx:590-616`) — initials,
//!   tier ring, tier ink. `SPONSORS` only stores `(name, profile, amount)`, so
//!   the image URLs are not carried; adding them without an image loader would
//!   be dead data. The initials maths is `sponsorInitials`
//!   (`SponsorsCarousel.tsx:57-67`).
//! - **Marquee.** The track is a 34 s CSS keyframe translating `-50%` over a
//!   duplicated list (`SponsorsCarousel.tsx:562-650`). GPUI has no CSS
//!   animation, so the row is a single copy in a horizontal scroller: same
//!   single-row geometry, same 12px gaps, same `py-2`, and every sponsor stays
//!   reachable instead of being clipped. The `ml-6` lead-in only exists to make
//!   the loop restart invisible, so it is not reproduced.
//! - **Tier rings.** The source's ring is a `conic-gradient` plus a two-layer
//!   `box-shadow` glow (`SponsorsCarousel.tsx:22-45`). A GPUI background takes
//!   two stops, so the ring is the conic's first two stops as a 135° linear
//!   gradient; the glow layers and the inset highlight are real `BoxShadow`s.
//! - **Three-stop gradients.** The ORA badge gradient is
//!   `linear-gradient(135deg, #3b0764 0%, #991b1b 50%, #9a3412 100%)`
//!   (`SettingsModal.tsx:1604`); with two stops the 50% `#991b1b` is dropped and
//!   the ends are kept (also noted in `mod.rs`). The credits divider's
//!   transparent-ended rule (`SettingsModal.tsx:1665-1670`) is drawn as two
//!   half-width two-stop gradients, so both ends fade exactly as in the source.
//! - **Text.** `tracking-wide` / `tracking-tighter` and `font-mono`
//!   (`SettingsModal.tsx:1752-1758`) have no GPUI equivalent here: GPUI has no
//!   letter-spacing, and no monospace family is registered with the text system
//!   (`assets/fonts/GeistMono.ttf` is never loaded), so the licence line and the
//!   repo link keep the default family. `tabular-nums` on the version pill goes
//!   the same way. `font-semibold` / `font-medium` / uppercase text are kept.
//! - **Drop shadows.** `filter: drop-shadow(...)` is applied by the source only
//!   in the light theme; this port has one dark palette, so it is not emitted.
//! - **Tooltips.** Every sponsor tile is wrapped in `<Tooltip
//!   content={label} maxWidth={220}>` in the source, portaled to `<body>`. GPUI
//!   has no portal and the row sits in the source's own `overflow-hidden`
//!   viewport, so a popover below a 54px tile (28px offset) falls entirely
//!   outside the clipped box; it is omitted instead of being rendered invisible.
//! - **Inert affordances.** The version pill cannot copy (no clipboard), so it
//!   renders its un-copied `Version …` text and the source's "Click to copy
//!   build info" tooltip is not advertised. The affiliation buttons, "Sponsor
//!   Us", the repo link and the sponsor tiles cannot open a URL (the backend
//!   `open_external_url` lives in the excluded `gpui-backend` crate), so they
//!   render as static text instead of pointer cursors that do nothing.
//! - **`--surface` mixes over transparency.** `color-mix(…, transparent N%)`
//!   has no GPUI form; the alpha that survives is baked into the colour
//!   (`rgba(with_alpha(SURFACE_0, 0xCC))` for the sponsor viewport).

use gpui::{BoxShadow, img, svg};

use super::*;

// ── Sponsors (`SponsorsCarousel.tsx`) ────────────────────────────────────

/// `sponsorInitials` (`SponsorsCarousel.tsx:57-67`) — first letter of the first
/// and last word, with a camelCase split so `WickedGrey` → `WG` and
/// `Kryz Slaughter` → `KS`. A lone word keeps one letter (`ben` → `B`).
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

/// A CSS `box-shadow` layer (`TopBar`'s helper shape).
fn shadow(x: f32, y: f32, blur: f32, spread: f32, color: u32) -> BoxShadow {
    BoxShadow::new(px(x), px(y), rgba(color).into())
        .blur_radius(px(blur))
        .spread_radius(px(spread))
}

/// The ring of a sponsor tile, from `getSponsorRank`
/// (`SponsorsCarousel.tsx:22-45`). Thresholds are lifetime
/// `totalAmountDonated`: gold ≥ 100, silver ≥ 30, bronze ≥ 10, else supporter.
struct SponsorRank {
    /// `gradient` — the conic ring. A GPUI background takes two stops, so this
    /// is the conic's 0° and 45° stops (the 360° stop repeats the 0° one).
    ring: Option<(u32, u32)>,
    /// `borderColor` — also the monogram ink when a ring is present.
    border: u32,
    /// `glow`: `0 0 6px …`, `0 0 12px …` (gold only) and the inset highlight.
    glow: Vec<BoxShadow>,
}

fn sponsor_rank(total: f32) -> SponsorRank {
    if total >= 100.0 {
        SponsorRank {
            ring: Some((0xBF953F, 0xFCF6BA)),
            border: 0xD4AF37,
            glow: vec![
                shadow(0., 0., 6., 0., with_alpha(0xD4AF37, 0x59)),
                shadow(0., 0., 12., 0., with_alpha(0xD4AF37, 0x2E)),
                shadow(0., 1., 1., 0., with_alpha(WHITE, 0x80)).inset(),
            ],
        }
    } else if total >= 30.0 {
        SponsorRank {
            ring: Some((0x71706E, 0xE8E8E8)),
            border: 0xC0C0C0,
            glow: vec![
                shadow(0., 0., 6., 0., with_alpha(0xC0C0C0, 0x4D)),
                shadow(0., 1., 1., 0., with_alpha(WHITE, 0x66)).inset(),
            ],
        }
    } else if total >= 10.0 {
        SponsorRank {
            ring: Some((0x6B3A1F, 0xE9BB83)),
            border: 0xCD7F32,
            glow: vec![
                shadow(0., 0., 6., 0., with_alpha(0xCD7F32, 0x40)),
                shadow(0., 1., 1., 0., with_alpha(WHITE, 0x4D)).inset(),
            ],
        }
    } else {
        // `supporter`: no gradient, no glow, `border-subtle` border, accent ink.
        SponsorRank {
            ring: None,
            border: BORDER_SUBTLE,
            glow: Vec::new(),
        }
    }
}

/// One sponsor tile — the `!s.image` branch (`SponsorsCarousel.tsx:590-616`):
/// a `3rem` circle in a 3px gutter ring (`box-sizing: content-box`, so the
/// tile is 54px), the monogram 15px for two letters and 19px for one,
/// `leading-none tracking-wide` at semibold.
fn sponsor_tile(name: &str, total: f32) -> Div {
    let rank = sponsor_rank(total);
    let initials = sponsor_initials(name);
    let short = initials.chars().count() <= 1;

    let mut tile = div()
        .w(px(54.)) // `width: 3rem` + `padding: 3px`, content-box
        .h(px(54.))
        .flex_shrink_0()
        .rounded_full()
        .p(px(3.))
        .flex()
        .items_center()
        .justify_center();
    if let Some((from, to)) = rank.ring {
        tile = tile.bg(linear_gradient(
            135.,
            linear_color_stop(rgb(from), 0.),
            linear_color_stop(rgb(to), 1.),
        ));
    }
    if !rank.glow.is_empty() {
        tile = tile.shadow(rank.glow);
    }

    // `background: var(--surface-1)`, `borderColor: transparent`, and
    // `color: gradient ? borderColor : var(--accent)`.
    let ink = if rank.ring.is_some() { rank.border } else { ACCENT };
    tile.child(
        div()
            .size_full()
            .rounded_full()
            .border_1()
            .border_color(rgba(0x00000000))
            .bg(rgb(SURFACE_1))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_size(px(if short { 19. } else { 15. }))
                    .line_height(px(if short { 19. } else { 15. })) // `leading-none`
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(ink))
                    .child(initials),
            ),
    )
}

/// `SponsorsCarousel.tsx:491-651` — the `shrink-0 rounded-lg border px-3 py-2`
/// card on `surface-1`, with the pinned header and the sponsor row.
fn sponsors_card() -> Div {
    // Header (`:494-514`): `flex items-center justify-between gap-2`, the
    // title at `text-[11px] font-semibold uppercase`, "via Open Collective" at
    // normal weight / 0.85 opacity, "Sponsor Us" at `text-[10px] font-medium`.
    let header = div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.)) // gap-1.5
                .text_size(px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .child(icon("heart", 12., ACCENT_SECONDARY))
                .child("SPONSORS") // `<Trans>Sponsors</Trans>` under `uppercase`
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
        );

    // Track (`:562-574`): `flex items-center gap-3 py-2` over the fallback list.
    // The source duplicates the list for the 34s marquee; with no CSS animation
    // the port keeps one copy and lets the viewport scroll horizontally.
    let mut track = div().flex().items_center().gap(px(12.)).py(px(8.));
    for &(name, _profile, total) in SPONSORS.iter() {
        // Each tile is a `Tooltip content={label} maxWidth={220}`
        // (`:571-614`). React portals that box to `<body>`; GPUI has no portal
        // and the row lives in the source's `overflow-hidden` viewport, so the
        // popover would be clipped away entirely — omitted, see the module note.
        // `openExternal(href)` likewise has no port.
        track = track.child(sponsor_tile(name, total));
    }

    // Viewport (`:550`): `relative mt-2 overflow-hidden rounded-md` on
    // `color-mix(surface-0, transparent 20%)`.
    let mut viewport = div()
        .relative()
        .mt(px(8.))
        .rounded(px(6.))
        .overflow_hidden()
        .bg(rgba(with_alpha(SURFACE_0, 0xCC)))
        .child(
            div()
                .id("about-sponsors-row")
                .overflow_x_scroll()
                .restrict_scroll_to_axis()
                .child(track),
        );
    // Edge fades (`:552-560`): `absolute inset-y-0 w-6`, `surface-1` →
    // transparent, painted over the row on both ends.
    viewport = viewport
        .child(
            div()
                .absolute()
                .left(px(0.))
                .top(px(0.))
                .bottom(px(0.))
                .w(px(24.))
                .bg(linear_gradient(
                    90.,
                    linear_color_stop(rgb(SURFACE_1), 0.),
                    linear_color_stop(rgba(0x00000000), 1.),
                )),
        )
        .child(
            div()
                .absolute()
                .right(px(0.))
                .top(px(0.))
                .bottom(px(0.))
                .w(px(24.))
                .bg(linear_gradient(
                    90.,
                    linear_color_stop(rgba(0x00000000), 0.),
                    linear_color_stop(rgb(SURFACE_1), 1.),
                )),
        );

    div()
        .flex_shrink_0()
        .rounded(px(8.)) // rounded-lg
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .px(px(12.)) // px-3
        .py(px(8.)) // py-2
        .child(header)
        .child(viewport)
}

// ── Branding assets ──────────────────────────────────────────────────────

/// `public/dragonfruit_assets/branding/` — where the web build keeps the three
/// marks the About tab paints. Anchored at compile time so the binary does not
/// depend on the working directory (`icons::icon` anchors `gpui/assets` the
/// same way).
fn branding_path(file: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../public/dragonfruit_assets/branding")
        .join(file)
}

fn branding_svg(file: &str) -> String {
    branding_path(file).to_string_lossy().into_owned()
}

/// `text_logo.svg`'s `viewBox="0 0 1772 304"`; `h-9` (36px) with `w-auto`.
const TEXT_LOGO_WIDTH: f32 = 36.0 * 1772.0 / 304.0;

// ── The tab ──────────────────────────────────────────────────────────────

/// `SettingsModal.tsx:1583-1766`.
pub(crate) fn about_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let _ = entity;
    let copied = state.about.copied;

    // `isBetaBuildChannel` / `buildStatusLabel` / `buildStatusStyle`
    // (`SettingsModal.tsx:1322-1345`). The dark-theme legs are the only ones
    // this palette has; the light legs (`#9a3412` on `#fed7aa`) do not apply.
    let is_beta_build_channel = DRAGONFRUIT_BUILD_CHANNEL.contains("beta");
    let build_status_label = if is_beta_build_channel {
        "BETA VERSION".to_owned()
    } else if DRAGONFRUIT_BUILD_CHANNEL == "mainline" {
        "Mainline Build".to_owned()
    } else {
        format!("{} Build", DRAGONFRUIT_BUILD_CHANNEL.to_uppercase())
    };
    let (build_status_text, build_status_border, build_status_bg) = if is_beta_build_channel {
        (
            0xfdba74,
            mix(BORDER_SUBTLE, 0xf97316, 0.16),
            rgba(with_alpha(0xf97316, 0x0A)), // `color-mix(#f97316, transparent 96%)`
        )
    } else {
        (
            TEXT_STRONG,
            mix(BORDER_SUBTLE, ACCENT, 0.40),
            rgba(with_alpha(ACCENT, 0x29)), // `color-mix(accent, transparent 84%)`
        )
    };

    // ── Hero (`SettingsModal.tsx:1587-1645`) ─────────────────────────────
    let hero = div()
        .rounded(px(12.)) // rounded-xl
        .p(px(16.)) // p-4
        // `linear-gradient(145deg, color-mix(accent, surface-0 95%),
        // color-mix(accent-secondary, surface-0 94%))`
        .bg(linear_gradient(
            145.,
            linear_color_stop(rgb(mix(SURFACE_0, ACCENT, 0.95)), 0.),
            linear_color_stop(rgb(mix(SURFACE_0, ACCENT_SECONDARY, 0.94)), 1.),
        ))
        .child(
            // `relative flex items-center justify-center` (`:1592`)
            div()
                .relative()
                .flex()
                .items_center()
                .justify_center()
                // `/dragonfruit_assets/branding/text_logo.svg`, `h-9 w-auto
                // object-contain` (`:1593-1598`). The light theme's
                // `drop-shadow` is not emitted (see the module note).
                .child(
                    svg()
                        .external_path(branding_svg("text_logo.svg"))
                        .w(px(TEXT_LOGO_WIDTH))
                        .h(px(36.)),
                )
                // `absolute right-0 top-0 inline-flex shrink-0 rounded-full
                // px-2.5 py-0.5 text-[12px] font-semibold`, white on
                // `linear-gradient(135deg, #3b0764, #991b1b 50%, #9a3412)`
                // (`:1600-1607`). Two stops: the 50% `#991b1b` is dropped.
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
        // `mt-3 flex flex-col items-center gap-2` (`:1610-1611`)
        .child(
            div()
                .mt(px(12.))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(8.))
                // `flex flex-wrap items-center justify-center gap-2.5` (`:1612`)
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_center()
                        .gap(px(10.))
                        // Version pill (`:1618-1636`): rounded-full border,
                        // `px-2.5 py-0.5 text-[12px] font-semibold tabular-nums`;
                        // copied → `#2d8a4e` ink/border on 10% green, otherwise
                        // `text-strong` on `color-mix(border-subtle, white 8%)`
                        // with `color-mix(surface-1, transparent 8%)`.
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
                                    mix(WHITE, BORDER_SUBTLE, 0.92)
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
                        // Build-status pill (`:1637-1642`): `text-xs` (12px)
                        // semibold, styled by `buildStatusStyle`.
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

    // ── Team & Credits (`SettingsModal.tsx:1647-1718`) ───────────────────
    // `contributors` sorted by tone (founder, accent, secondary), then by name
    // (`:1654-1662`). The source's `a.name.localeCompare(b.name)` is
    // case-insensitive and locale-aware; this compares lowercased names and
    // falls back to the raw string, which is what the data's ASCII names need
    // (`Aaron Baca` before `AJRubenstein`, `tforik55` before `Tim`).
    let mut people: Vec<(&str, &str, &str)> = CONTRIBUTORS.to_vec();
    people.sort_by(|a, b| {
        let rank = |tone: &str| match tone {
            "founder" => 0,
            "accent" => 1,
            _ => 2,
        };
        rank(a.2).cmp(&rank(b.2)).then_with(|| {
            let by_fold = a.0.to_lowercase().cmp(&b.0.to_lowercase());
            if by_fold.is_eq() {
                a.0.cmp(b.0)
            } else {
                by_fold
            }
        })
    });

    // `mt-2.5 space-y-2` (`:1654`).
    let mut groups = div().mt(px(10.)).flex().flex_col().gap(px(8.));
    for (index, tone) in ["founder", "accent", "secondary"].iter().enumerate() {
        let mut grid = div().grid().grid_cols(2).gap(px(8.)); // `grid grid-cols-2 gap-2`
        let mut count = 0usize;
        for &(name, affiliation, person_tone) in people.iter() {
            if person_tone != *tone {
                continue;
            }
            count += 1;
            // `toneVar` / `bgMix` (`:1679-1681`): founder `#d4a017` at 94%,
            // accent `var(--accent)` at 90%, secondary at 93%.
            let (tone_color, background_mix) = match *tone {
                "founder" => (0xd4a017, 0.94),
                "accent" => (ACCENT, 0.90),
                _ => (ACCENT_SECONDARY, 0.93),
            };
            // `rounded-md border px-3 py-2.5`, border
            // `color-mix(tone, border-subtle 45%)`, background
            // `color-mix(tone, surface-0 <bgMix>)` (`:1684-1697`).
            grid = grid.child(
                div()
                    .rounded(px(6.))
                    .border_1()
                    .border_color(rgb(mix(BORDER_SUBTLE, tone_color, 0.45)))
                    .bg(rgb(mix(SURFACE_0, tone_color, background_mix)))
                    .px(px(12.))
                    .py(px(10.))
                    // `flex items-center justify-between gap-2 text-sm`
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .text_size(px(14.))
                            .child(
                                // `font-semibold truncate` on `text-strong`
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .truncate()
                                    .text_color(rgb(TEXT_STRONG))
                                    .child(name),
                            )
                            // `inline-flex shrink-0 items-center gap-0.5
                            // font-normal` on `text-muted`, the GitHub mark at
                            // `h-3 w-3` — the source's affiliation button, inert
                            // here (no URL opener).
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .flex()
                                    .items_center()
                                    .gap(px(2.))
                                    .font_weight(FontWeight::NORMAL)
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
            // `my-2.5 h-px rounded-full` hairlines between the tone groups
            // (`:1665-1670`): `linear-gradient(90deg, transparent 0%,
            // border-subtle 22%, border-subtle 78%, transparent 100%)`.
            // Two half-width gradients reproduce both faded ends; the flex gap
            // of 8px stacks with the 10px margin as `space-y-2` does.
            groups = groups.child(
                div()
                    .my(px(10.))
                    .h(px(1.))
                    .flex()
                    .child(div().flex_1().h(px(1.)).bg(linear_gradient(
                        90.,
                        linear_color_stop(rgba(0x00000000), 0.),
                        linear_color_stop(rgb(BORDER_SUBTLE), 1.),
                    )))
                    .child(div().flex_1().h(px(1.)).bg(linear_gradient(
                        90.,
                        linear_color_stop(rgb(BORDER_SUBTLE), 0.),
                        linear_color_stop(rgba(0x00000000), 1.),
                    ))),
            );
        }
        groups = groups.child(grid);
    }

    // `rounded-md border p-3` on `surface-1` (`:1647`), header at `text-xs
    // font-semibold uppercase` on `text-muted` (`:1648-1651`).
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
                .child("TEAM & CREDITS"), // `<h5>` + `uppercase`
        )
        .child(groups);

    // ── ORA footer (`SettingsModal.tsx:1726-1765`) ───────────────────────
    // `flex items-center gap-4 rounded-xl border px-4 py-3`, border
    // `color-mix(accent-secondary, border-subtle 52%)`, background
    // `color-mix(accent-secondary, surface-0 94%)`.
    let ora = div()
        .flex()
        .items_center()
        .gap(px(16.))
        .rounded(px(12.))
        .border_1()
        .border_color(rgb(mix(BORDER_SUBTLE, ACCENT_SECONDARY, 0.52)))
        .bg(rgb(mix(SURFACE_0, ACCENT_SECONDARY, 0.94)))
        .px(px(16.))
        .py(px(12.))
        // `ORA_LOGO_DARK_URL` at `h-14 w-auto object-contain shrink-0`
        // (`:1727-1734`); the PNG is 1500×1500, so `h-14` is 56×56. The light
        // theme's `drop-shadow` is not emitted.
        .child(
            img(branding_path("open_resin_alliance_logo_darkmode.png"))
                .w(px(56.))
                .h(px(56.))
                .flex_shrink_0(),
        )
        // `min-w-0 flex-1 space-y-2 text-center` (`:1736`)
        .child(
            div()
                .min_w(px(0.))
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(8.))
                // `flex items-center justify-center gap-2 text-[12px]` — the
                // repo link (`:1737-1750`): GitHub mark `h-3.5 w-3.5` on accent,
                // the label underlined at `underline-offset-2 font-mono
                // tracking-tighter`, then `ExternalLink` at `h-3 w-3`.
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
                                .underline()
                                .child("Open-Resin-Alliance/DragonFruit"),
                        )
                        .child(icon("external-link", 12., ACCENT)),
                )
                // `flex items-center justify-center gap-2 text-[12px]` on
                // `text-strong` — the licence (`:1752-1759`): `ScrollText` at
                // `h-3.5 w-3.5` on accent, `font-mono tracking-tighter` label.
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
        )
        // `simple_icon.svg` at `h-10 w-auto object-contain shrink-0`
        // (`:1761-1765`); its `viewBox` is square, so `h-10` is 40×40.
        .child(
            svg()
                .external_path(branding_svg("simple_icon.svg"))
                .w(px(40.))
                .h(px(40.))
                .flex_shrink_0(),
        );

    // `flex h-full min-h-0 flex-col gap-3.5` (`:1584`) — the tab's own scroller
    // plus the two pinned cards. `flex-1 min-h-0 overflow-y-auto
    // custom-scrollbar pr-1` (`:1585`) → `space-y-3.5 pb-2` (`:1586`).
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
        .child(sponsors_card())
        .child(ora)
}
