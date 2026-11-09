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
//! - **Image assets are embedded, and `img()` paints them, not `svg()`.** The
//!   three marks live in the web build's tree at
//!   `public/dragonfruit_assets/branding/` and are `include_bytes!`d into the
//!   binary — a desktop build has no `public/` and no working directory to
//!   resolve against. They are decoded through `img()`, whose SVG branch is
//!   `SvgRenderer::render_single_frame` and keeps the artwork's own colours.
//!   `svg().external_path(…)` — what this tab used before — does load the file,
//!   but it paints the SVG as a *monochrome alpha mask tinted by
//!   `style.text.color`*, and the three elements never set a colour:
//!   `Svg::paint` gates on `self.external_path.as_ref().zip(style.text.color)`,
//!   so with no `text_color` it painted nothing at all. `icons::icon` only works
//!   because it sets one. Setting a tint would still flatten the wordmark's
//!   `fill:white` / `rgb(255,44,126)` / `rgb(191,254,46)`, hence `img()`.
//! - **Sponsors are the baked-in fallback list, fetched never.** The source
//!   paints `sponsors.json` first and then replaces it with a live Open
//!   Collective fetch (two collectives, 6 h `localStorage` cache, per-avatar
//!   blob cache, `SponsorsCarousel.tsx:74-360`). The port has no HTTP client for
//!   the member documents, so it renders exactly the fallback list the source
//!   uses before/without a fetch — the same 18 sponsors, same order, same
//!   amounts — through [`SPONSORS`]. The loading branch (`sponsors === null`)
//!   and the "No sponsors yet" empty branch are therefore unreachable and not
//!   spelled: the list is never empty, so "no spinner forever" is not a state
//!   this tab can enter.
//! - **Avatars are fetched.** `sponsors.json` carries an `image` for 11 of the
//!   18 sponsors; the About tab fetches, decodes and caches those through
//!   [`gpui_core::sponsors`] (see that module for the transport and cache) and
//!   paints the source's `CachedAvatar` 48px disk inside the 3px ring gutter.
//!   Until the bytes land the tile shows the source's `Spinner size="lg"`; if
//!   the fetch or the decode fails it shows the monogram branch — the source
//!   falls back to the bare URL for a direct `<img>` load instead, which leaves
//!   a blank circle once that load errors too. The initials maths is
//!   `sponsorInitials` (`SponsorsCarousel.tsx:57-67`).
//! - **Marquee, driven from the render loop.** The source animates the track
//!   with a 34 s CSS keyframe (`SponsorsCarousel.tsx:627-643`): 3 s of dwell,
//!   28 s translating `0` → `-50%`, 3 s of dwell, `linear`, `infinite`, paused
//!   by `:hover` / `:focus-within`. The port duplicates the list the same way
//!   and shifts the track by `left(-offset)`; the clock is wall time, and the
//!   one-frame-ahead wrap distance is measured from the two copies'
//!   prepainted bounds ([`MARQUEE`], [`marquee_prepaint`]). The trailing dwell
//!   rests on the frame the loop restarts from, so the source's 3 s + 3 s
//!   halves read as one ~6 s rest; the port keeps both, exactly as the keyframe
//!   does. `focus-within` has no port — the tiles are not focusable here.
//!   `:hover` is `marquee_hover_prepaint`: this GPUI line has no callback
//!   hover (`.on_hover` is a `gpui-pre` API), so each prepaint tests
//!   `window.mouse_position()` against the viewport's own painted rect. That
//!   keeps the frame loop running while paused (no frames would mean no
//!   prepaint and the pause could never be released); the clock freezes, so
//!   the picture rests. Frames stop as soon as the About tab is not built, so
//!   a closed modal or another tab costs nothing.
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

use std::cell::RefCell;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use gpui::{Bounds, BoxShadow, Image, ImageFormat, ObjectFit, Pixels, img};
use gpui_core::sponsors::{self, AvatarState};

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

/// One sponsor tile — `SponsorsCarousel.tsx:586-620`.
///
/// The outer circle is the source's ring: a `3rem` content box in a 3px gutter
/// (`box-sizing: content-box`, so the tile is 54px), the rank gradient, the
/// glow. Inside it:
/// - with a decoded avatar (`s.image` → `CachedAvatar`, `:314-365`), the 48px
///   image on a `surface-1` disk, `object-cover`;
/// - while the fetch is in flight, the source's `Spinner size="lg"` on
///   `color-mix(surface-1, transparent 20%)` with a `border-subtle` ring;
/// - with no image at all, or after a failed fetch, the monogram branch
///   (`:591-616`): initials, 15px for two letters and 19px for one,
///   `leading-none tracking-wide` at semibold, tier ink on `surface-1`.
fn sponsor_tile(name: &str, total: f32) -> Div {
    let rank = sponsor_rank(total);
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
    let ink = if rank.ring.is_some() {
        rank.border
    } else {
        ACCENT
    };
    let disk = div()
        .size_full()
        .rounded_full()
        .bg(rgb(SURFACE_1))
        .flex()
        .items_center()
        .justify_center();

    let content = match sponsors::avatar_for(name).and_then(sponsors::state) {
        Some(AvatarState::Ready(image)) => disk.overflow_hidden().child(
            img(image)
                .w(px(48.))
                .h(px(48.))
                .flex_shrink_0()
                .object_fit(ObjectFit::Cover),
        ),
        Some(AvatarState::Loading) => disk
            // `background: color-mix(surface-1, transparent 20%)`, border
            // `border-subtle`, `color: text-muted`.
            .bg(rgba(with_alpha(SURFACE_1, 0xCC)))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .child(spinner(20.)),
        Some(AvatarState::Failed) | None => {
            let initials = sponsor_initials(name);
            let short = initials.chars().count() <= 1;
            disk.border_1().border_color(rgba(0x00000000)).child(
                div()
                    .text_size(px(if short { 19. } else { 15. }))
                    .line_height(px(if short { 19. } else { 15. })) // `leading-none`
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(ink))
                    .child(initials),
            )
        }
    };

    tile.child(content)
}

/// `SponsorsCarousel.tsx:491-651` — the `shrink-0 rounded-lg border px-3 py-2`
/// card on `surface-1`, with the pinned header and the animated sponsor row.
fn sponsors_card(entity: WeakEntity<DragonFruit>) -> Div {
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

    // Track (`:562-574`): `flex items-center gap-3 py-2` over `[...sponsors,
    // ...sponsors]`. Each copy is one child so the wrap distance can be
    // measured from the two copies' bounds; the outer `gap-3` is the source's
    // gap across the seam, and `ml-6` on each copy's first tile is its lead-in.
    let offset = marquee_offset();
    let mut track = div()
        .relative()
        // `transform: translateX(-offset)` — the keyframe's only animated
        // property. Taffy resolves a relative inset after layout, so this
        // shifts the painted row without reflowing it.
        .left(px(-offset))
        .flex()
        .items_center()
        .gap(px(12.))
        .py(px(8.)); // py-2
    for _ in 0..2 {
        let mut copy = div().flex().items_center().gap(px(12.)).flex_shrink_0();
        for (index, &(name, _profile, total)) in SPONSORS.iter().enumerate() {
            // Each tile is the source's `inline-flex shrink-0 … py-1` button
            // (`:574-578`); the wrapper keeps its 4px vertical padding, so a
            // 54px ring makes a 62px item. The `Tooltip content={label}` on it
            // (`:571`) is portaled to `<body>` in React and GPUI has no portal,
            // so the popover would be clipped by the viewport — omitted, see
            // the module note. `openExternal(href)` likewise has no port.
            let mut item = div().flex_shrink_0().py(px(4.));
            if index == 0 {
                item = item.ml(px(24.)); // `ml-6`, only on `idx % len === 0`
            }
            copy = copy.child(item.child(sponsor_tile(name, total)));
        }
        track = track.child(copy);
    }

    // The render-loop seat: the prepaint listener runs on every frame the row
    // is painted, which is exactly while the About tab is on screen with its
    // sponsors card pinned. It measures the wrap distance from the two copies
    // and starts the avatar fetches (they need `&mut App`, which only a
    // callback gets). The viewport's own listener asks for the next frame.
    let entity_for_frames = entity.clone();
    track = track.on_children_prepainted(move |bounds, _, cx| {
        marquee_prepaint(&bounds);
        for &(name, _, _) in SPONSORS.iter() {
            if let Some(url) = sponsors::avatar_for(name) {
                sponsors::ensure_loaded(url, cx, entity_for_frames.clone());
            }
        }
    });

    // Viewport (`:550`): `relative mt-2 overflow-hidden rounded-md` on
    // `color-mix(surface-0, transparent 20%)`. The source pauses the keyframe
    // on `:hover` / `:focus-within`. This GPUI line has no callback hover
    // (`Div::on_hover` is a `gpui-pre` API; `gpui-ui` only uses the style-only
    // `.hover(…)`), so the pause is resolved against the pointer in the
    // viewport's own prepainted rect — the same rectangle `:hover` means.
    let mut viewport = div()
        .relative()
        .mt(px(8.))
        .rounded(px(ROUNDED_MD)) // `rounded-md` — 10px in this app
        .overflow_hidden()
        .bg(rgba(with_alpha(SURFACE_0, 0xCC)))
        // First child, so `bounds[0]` in the listener below is the viewport's
        // own painted rect (absolute + `size_full`, invisible).
        .child(div().absolute().top(px(0.)).left(px(0.)).size_full())
        .child(track)
        .on_children_prepainted(move |bounds, window, _| marquee_hover_prepaint(&bounds, window));
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
        .rounded(px(ROUNDED_LG)) // `rounded-lg` — 14px in this app
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .px(px(12.)) // px-3
        .py(px(8.)) // py-2
        .child(header)
        .child(viewport)
}

// ── Marquee (`SponsorsCarousel.tsx:627-643`) ─────────────────────────────

/// `@keyframes sponsors-marquee` runs for 34 s (`animation: … 34s linear
/// infinite`), of which 28 s is the scroll.
const MARQUEE_LOOP: f32 = 34.0;
const MARQUEE_SCROLL: f32 = 28.0;
/// The keyframe rests at `translateX(0)` for `0%`→`8.8%` (2.992 s) and at
/// `translateX(-50%)` for `91.2%`→`100%` (2.992 s); the comments in the source
/// call them 3 s.
const MARQUEE_DWELL: f32 = 3.0;
/// A frame gap longer than this means the page was not on screen: the source
/// remounts the carousel and its keyframe restarts from `0%`, so the port
/// restarts too instead of jumping forward.
const MARQUEE_RESTART_GAP: Duration = Duration::from_millis(250);

struct MarqueeClock {
    started: Option<Instant>,
    last_frame: Option<Instant>,
    /// The distance from the first copy's left edge to the second's — the
    /// `-50%` the keyframe travels. Measured from the two copies' prepainted
    /// bounds rather than assumed, so text metrics or a future avatar size
    /// cannot desynchronise the wrap.
    pitch: f32,
    /// `animation-play-state: paused` while `:hover` holds the row.
    paused_at: Option<Instant>,
    paused_total: Duration,
}

impl Default for MarqueeClock {
    fn default() -> Self {
        Self {
            started: None,
            last_frame: None,
            pitch: 0.0,
            paused_at: None,
            paused_total: Duration::ZERO,
        }
    }
}

thread_local! {
    /// The About tab is one view on the main thread, so a thread-local clock is
    /// enough and costs no lock per frame.
    static MARQUEE: RefCell<MarqueeClock> = RefCell::new(MarqueeClock::default());
}

/// The keyframe's `translateX` at `elapsed` into the loop, in pixels: flat for
/// the leading 3 s, linear to the copy distance over the next 28 s, flat again
/// for the trailing 3 s, then back to `0` (which is the same picture, because
/// the second copy lands exactly where the first one started).
fn marquee_offset_at(elapsed: Duration, pitch: f32) -> f32 {
    let phase = elapsed.as_secs_f32() % MARQUEE_LOOP;
    let progress = if phase <= MARQUEE_DWELL {
        0.0
    } else if phase >= MARQUEE_LOOP - MARQUEE_DWELL {
        1.0
    } else {
        (phase - MARQUEE_DWELL) / MARQUEE_SCROLL
    };
    pitch * progress
}

/// This frame's offset. Called once per build, before the track is assembled,
/// which is also where a cold mount restarts the clock (and lets the avatars
/// that failed last time try again).
fn marquee_offset() -> f32 {
    MARQUEE.with(|cell| {
        let mut clock = cell.borrow_mut();
        let now = Instant::now();
        let restart = match clock.last_frame {
            Some(last) => now.saturating_duration_since(last) > MARQUEE_RESTART_GAP,
            None => true,
        };
        if restart {
            clock.started = Some(now);
            // A paused row stops requesting frames, so the gap that follows a
            // long hover is not an absence: re-anchor the pause instead of
            // clearing it, and the row simply rests at `0` until the pointer
            // leaves.
            clock.paused_at = clock.paused_at.map(|_| now);
            clock.paused_total = Duration::ZERO;
            sponsors::reset_failed();
        }
        clock.last_frame = Some(now);
        let started = clock.started.unwrap_or(now);
        let at = clock.paused_at.unwrap_or(now);
        let elapsed = at
            .saturating_duration_since(started)
            .saturating_sub(clock.paused_total);
        marquee_offset_at(elapsed, clock.pitch)
    })
}

/// The track's prepaint half of the clock: read the wrap distance from the two
/// copies' painted rects. It requests no frame; the viewport listener below is
/// the single place that does.
fn marquee_prepaint(bounds: &[Bounds<Pixels>]) {
    if let (Some(first), Some(second)) = (bounds.first(), bounds.get(1)) {
        set_marquee_pitch(second.origin.x.0 - first.origin.x.0);
    }
}

fn set_marquee_pitch(pitch: f32) {
    if pitch > 0.0 {
        MARQUEE.with(|cell| cell.borrow_mut().pitch = pitch);
    }
}

/// `:hover` for the viewport, resolved against the pointer and the viewport's
/// own painted rect (`bounds[0]`, the invisible first child), then the next
/// frame request — the only one.
///
/// The loop keeps running while paused on purpose: no frames means no prepaint,
/// and the pause would never be released. Pausing freezes the *clock*, so the
/// picture rests even though frames continue.
fn marquee_hover_prepaint(bounds: &[Bounds<Pixels>], window: &mut Window) {
    if let Some(area) = bounds.first() {
        let hovered = area.contains(&window.mouse_position());
        set_marquee_hovered(hovered);
    }
    window.request_animation_frame();
}

/// `animation-play-state` on hover. `true` when the state changed.
fn set_marquee_hovered(hovered: bool) -> bool {
    MARQUEE.with(|cell| {
        let mut clock = cell.borrow_mut();
        let now = Instant::now();
        match (hovered, clock.paused_at) {
            (true, None) => {
                clock.paused_at = Some(now);
                true
            }
            (false, Some(at)) => {
                clock.paused_total += now.saturating_duration_since(at);
                clock.paused_at = None;
                true
            }
            _ => false,
        }
    })
}

/// The pause test needs the keyframe's scroll phase without waiting out the
/// 3 s lead-in dwell, so it rewinds this thread's clock instead.
#[cfg(test)]
fn marquee_seek(elapsed: Duration) {
    MARQUEE.with(|cell| {
        let mut clock = cell.borrow_mut();
        let now = Instant::now();
        clock.started = Some(now - elapsed);
        clock.last_frame = Some(now);
        clock.paused_at = None;
        clock.paused_total = Duration::ZERO;
    });
}

// ── Branding assets ──────────────────────────────────────────────────────

/// `public/dragonfruit_assets/branding/` — where the web build keeps the three
/// marks the About tab paints. They are embedded at compile time: a desktop
/// binary has no `public/`, and a path read at paint time would tie the marks
/// to the source tree the binary was built from.
///
/// # Which mark goes down which path
///
/// All three go through `img()`, and none may go through `svg()`:
///
/// | file | `img()` branch | why |
/// |------|----------------|-----|
/// | `text_logo.svg` | `ImageFormat::Svg` → `SvgRenderer::render_single_frame` | polychrome: `fill:white`, `rgb(255,44,126)`, `rgb(191,254,46)` |
/// | `simple_icon.svg` | `ImageFormat::Svg` → `SvgRenderer::render_single_frame` | same rasterizer; its paths are tinted too |
/// | `open_resin_alliance_logo_darkmode.png` | `ImageFormat::Png` → `decode_static_image` | a raster, no vector pass |
///
/// `svg()` is the dead end this tab used to sit on: it paints a *monochrome
/// alpha mask tinted with the element's text colour* (`Svg::paint` gates on
/// `external_path.zip(style.text.color)`, and no colour was ever set, so it
/// painted nothing at all). A tint would still flatten the wordmark. `img()`'s
/// SVG branch keeps the artwork's own colours. `Image::from_bytes` hashes the
/// bytes for its cache key, so GPUI decodes each mark once and every frame
/// reuses the `RenderImage`.
static TEXT_LOGO: LazyLock<Arc<Image>> = LazyLock::new(|| {
    Arc::new(Image::from_bytes(
        ImageFormat::Svg,
        include_bytes!("../../../../public/dragonfruit_assets/branding/text_logo.svg").to_vec(),
    ))
});

static SIMPLE_ICON: LazyLock<Arc<Image>> = LazyLock::new(|| {
    Arc::new(Image::from_bytes(
        ImageFormat::Svg,
        include_bytes!("../../../../public/dragonfruit_assets/branding/simple_icon.svg").to_vec(),
    ))
});

static ORA_LOGO: LazyLock<Arc<Image>> = LazyLock::new(|| {
    Arc::new(Image::from_bytes(
        ImageFormat::Png,
        include_bytes!(
            "../../../../public/dragonfruit_assets/branding/open_resin_alliance_logo_darkmode.png"
        )
        .to_vec(),
    ))
});

/// `text_logo.svg`'s `viewBox="0 0 1772 304"` with `h-9` (36px) and `w-auto`:
/// the aspect is `1772 / 304 = 5.8289…`, so the painted width is
/// `36 × (1772 / 304) = 209.84 px`. `object-contain` adds nothing because the
/// element's aspect already matches the artwork's.
const TEXT_LOGO_WIDTH: f32 = 36.0 * 1772.0 / 304.0;

// ── The tab ──────────────────────────────────────────────────────────────

/// `SettingsModal.tsx:1583-1766`.
pub(crate) fn about_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
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
                // `text_logo.svg` at `h-9 w-auto object-contain`
                // (`:1593-1598`). The artwork is `img()`-painted, not `svg()`,
                // so the wordmark keeps the SVG's own three fills. The light
                // theme's `drop-shadow` is not emitted (see the module note).
                .child(
                    img(TEXT_LOGO.clone())
                        .w(px(TEXT_LOGO_WIDTH))
                        .h(px(36.))
                        .flex_shrink_0(),
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
                    .rounded(px(ROUNDED_MD)) // `rounded-md` — 10px in this app
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
        .rounded(px(ROUNDED_MD)) // `rounded-md` — 10px in this app
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
        .child(img(ORA_LOGO.clone()).w(px(56.)).h(px(56.)).flex_shrink_0())
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
        // (`:1761-1765`). Its `viewBox="0 0 422 422"` is square
        // (`422 / 422 = 1`), so `w-auto` is `40 × 1 = 40` and the box is
        // 40×40.
        .child(
            img(SIMPLE_ICON.clone())
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
        .child(sponsors_card(entity))
        .child(ora)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The keyframe's own numbers: 3 s rest, 28 s across one copy, 3 s rest,
    /// then the loop point (which is the same picture, because the second copy
    /// lands where the first one started).
    #[test]
    fn marquee_speed_and_dwells_match_the_source_keyframe() {
        const PITCH: f32 = 1212.0;
        let secs = |seconds: f32| Duration::from_secs_f32(seconds);

        assert_eq!(marquee_offset_at(Duration::ZERO, PITCH), 0.0);
        assert_eq!(marquee_offset_at(secs(3.0), PITCH), 0.0);
        // 28 s to travel the copy: a quarter of it after 7 s of scrolling.
        assert!((marquee_offset_at(secs(10.0), PITCH) - PITCH * 0.25).abs() < 0.01);
        assert!((marquee_offset_at(secs(17.0), PITCH) - PITCH * 0.5).abs() < 0.01);
        assert!((marquee_offset_at(secs(31.0), PITCH) - PITCH).abs() < 0.01);
        // The trailing dwell, then the wrap back to the loop's first frame.
        assert!((marquee_offset_at(secs(33.0), PITCH) - PITCH).abs() < 0.01);
        assert!(marquee_offset_at(secs(34.0), PITCH).abs() < 0.01);
    }

    /// Smoke line for the render loop: between two consecutive 60 Hz frames in
    /// the scroll phase the offset advances by `pitch / 28 s * dt`.
    #[test]
    fn marquee_offset_advances_between_frames() {
        const PITCH: f32 = 1212.0;
        let frame = Duration::from_secs_f32(1.0 / 60.0);
        let before = marquee_offset_at(Duration::from_secs_f32(10.0), PITCH);
        let after = marquee_offset_at(Duration::from_secs_f32(10.0) + frame, PITCH);
        let expected = PITCH / MARQUEE_SCROLL * frame.as_secs_f32();
        assert!(after > before, "{after} did not advance from {before}");
        assert!((after - before - expected).abs() < 0.01);
    }

    /// The pause is observable, not prose: the hovered row's offset is exactly
    /// the same across two reads a sleep apart, and leaving resumes it.
    #[test]
    fn hovering_freezes_the_marquee_and_leaving_resumes_it() {
        // Each test runs on its own thread, and the clock is thread-local, so
        // this starts from `Default`.
        set_marquee_pitch(1200.0);
        marquee_seek(Duration::from_secs(10)); // mid-scroll, past the dwell

        let before = marquee_offset();
        std::thread::sleep(Duration::from_millis(12));
        let after = marquee_offset();
        assert!(after > before, "the row must be moving: {before} → {after}");

        // Pointer inside the viewport's painted rect.
        assert!(set_marquee_hovered(true), "hover-in must change the state");
        let frozen = marquee_offset();
        std::thread::sleep(Duration::from_millis(24));
        assert_eq!(
            marquee_offset(),
            frozen,
            "a hovered row advanced while it should be paused"
        );

        // Pointer left: the clock resumes where it stopped.
        assert!(
            set_marquee_hovered(false),
            "hover-out must change the state"
        );
        std::thread::sleep(Duration::from_millis(12));
        assert!(
            marquee_offset() > frozen,
            "leaving the row did not resume the keyframe"
        );
    }

    /// The source's own `sponsorInitials` cases (`SponsorsCarousel.tsx:57-67`).
    #[test]
    fn initials_match_the_source() {
        assert_eq!(sponsor_initials("WickedGrey"), "WG");
        assert_eq!(sponsor_initials("Kryz Slaughter"), "KS");
        assert_eq!(sponsor_initials("ben"), "B");
        // `3D` splits into `3` / `D`; the digit is dropped as a non-letter, so
        // the two survivors are first + last: `C` + `D`.
        assert_eq!(sponsor_initials("Concepts 3D"), "CD");
        assert_eq!(sponsor_initials("SIN"), "S");
    }
}
