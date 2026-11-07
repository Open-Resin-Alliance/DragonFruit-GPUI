//! `src/components/layout/EmptySceneState.tsx` and
//! `src/components/controls/ModelStatsCard.tsx`.
//!
//! Two widgets the shell mounts over the 3D scene: the empty-scene panel (load
//! mesh / import scene, recent files, first-run onboarding) and the bottom-left
//! stats card (printer rows on the front face, model details on the back).
//!
//! Both are render functions: they take the data the source reads from its
//! stores plus the shell entity, and return the element. Every action the port
//! can express is wired to `DragonFruit`; the rest is marked `PORT GAP`.
//!
//! # PORT DEVIATIONS
//!
//! * **Ephemeral state is caller-owned.** `useState` values (`copied`,
//!   `isFlipped`) cannot live in a stateless render function, so each state
//!   struct carries the `Rc<Cell<bool>>` the widget writes back (and then
//!   notifies the entity). The caller creates them with `Default`.
//! * **File pickers.** `onFileChange` / `onImportSceneChange` are hidden
//!   `<input type="file">` elements in the source. GPUI has no file input, so
//!   `Load Mesh` calls `DragonFruit::open_stl` (the same `rfd` dialog the app
//!   menu uses). The scene picker has no ported importer — `PORT GAP`.
//! * **Import Scene** — the source imports `.voxl` / `.lys` scenes; this build
//!   has no scene importer, so the button renders with its label and does
//!   nothing (`PORT GAP`).
//! * **Recent files** — the port has no recent-file store and no cache-backed
//!   reopen, so the list renders read-only: rows are not clickable, the
//!   `Clear` button (gated on `onClearRecentFiles`) is not rendered, and the
//!   `reopenError` line never appears (`PORT GAP`).
//! * **First-run onboarding** — `onAddPrinter` opens the printer library and
//!   `onUseWithoutPrinter` sets the prepare-without-printer flag. Neither store
//!   exists in the port; both buttons render and do nothing (`PORT GAP`).
//! * **Drag & drop zone** — the source tracks its own `isDropActive` /
//!   `isDropUnsupported` from `DataTransfer`. GPUI delivers OS drops to the
//!   canvas (`DragonFruit::drag_active` / `drag_unsupported`), so the panel
//!   takes those two flags and paints the same highlight. The zone itself has
//!   no drop target here; the canvas owns the drop.
//! * **Build identity** — `NEXT_PUBLIC_*` env values become
//!   [`EmptySceneState::version`], [`EmptySceneState::build_channel`] and
//!   [`EmptySceneState::build_label`]. The clipboard write is real; the
//!   `✓ Copied!` flash is the caller's cell plus a 2 s timer.
//! * **Theme.** The source branches on the light theme for the version chip and
//!   the recent-file badges. The port has only the dark tokens (`theme.rs`), so
//!   the dark branch is always taken.
//! * **`.ui-empty-state` width** is `min(90vw, 560px)`; with no window width in
//!   the render function the panel is a flat 560px (`EMPTY_STATE_WIDTH`).
//! * **`.ui-empty-title` font size** is `clamp(20px, 3vw, 28px)`; the port
//!   takes the 28px upper bound.
//! * **`tabular-nums` / `transition-colors` / `tracking-wide`** have no GPUI
//!   equivalents; `uppercase` is applied to the strings.
//! * **Locale formatting** (`useLocaleFormat`) does not exist in the port: the
//!   numeric formatters below reproduce `Intl.NumberFormat`'s fixed-decimal
//!   output without locale grouping, and the "last opened" date uses `M/D/YYYY`
//!   (the `en-US` `toLocaleDateString` shape) instead of the user's locale.
//! * **ModelStatsCard flip.** The source rotates a `preserve-3d` stack and
//!   swaps faces. GPUI has no 3D transform, so only the active face is
//!   rendered — the same two-phase "flat flip" the source falls back to on
//!   Linux, minus the 250 ms edge-on animation.
//! * **ModelStatsCard data.** Printer/material/exposure/print-time/resin values
//!   come from the profile and slicer stores, which the port does not have.
//!   They travel on [`ModelStatsCardState`]; absent values render the source's
//!   `-` fallback. The `printer` / `material` links open the profile settings
//!   modal, which is not ported (`PORT GAP`).
//! * **`select-none`** on the card has no GPUI equivalent.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gpui::{
    BoxShadow, ClipboardItem, Div, FontWeight, IntoElement, SharedString, WeakEntity, div,
    prelude::*, px, relative, rgb, rgba,
};

use gpui_core::icons::icon;
use gpui_ui::panel_fields::with_alpha;
use gpui_core::theme::*;
use gpui_core::empty_state::*;
use gpui_ui::ui;
use crate::DragonFruit;

const WHITE: u32 = 0xffffff;
const BLACK: u32 = 0x000000;

/// `.ui-empty-state`'s `max-width: 560px` (see the width deviation).
const EMPTY_STATE_WIDTH: f32 = 560.0;
/// `.ui-empty-state`'s `padding: 20px`.
const EMPTY_STATE_PADDING: f32 = 20.0;

/// Tailwind `rounded-md` / `rounded-lg` / `rounded` as the sources use them.
const ROUNDED: f32 = 4.0;
const ROUNDED_MD: f32 = 6.0;
const ROUNDED_LG: f32 = 8.0;

/// The version chip's `px-3.5 py-1 text-[13px]`.
const CHIP_PADDING_X: f32 = 14.0;
const CHIP_PADDING_Y: f32 = 4.0;
const CHIP_FONT_SIZE: f32 = 13.0;
/// The chip's `✓ Copied!` flash, matching the source's `setTimeout(…, 2000)`.
const COPIED_FLASH_MS: u64 = 2000;

/// `#2d8a4e` — the copied chip's green.
const COPY_GREEN: u32 = 0x2d8a4e;
/// `#f97316` — the beta chip's orange.
const BETA_ORANGE: u32 = 0xf97316;
/// `#fdba74` — the beta chip's text on the dark theme.
const BETA_TEXT_DARK: u32 = 0xfdba74;

/// Recent-file badge accents on the dark theme (`#fb923c` scene, `#a78bfa` mesh).
const SCENE_BADGE_DARK: u32 = 0xfb923c;
const MESH_BADGE_DARK: u32 = 0xa78bfa;

/// `#f59e0b` — the defect banner's amber.
const DEFECT_AMBER: u32 = 0xf59e0b;
/// `#f87171` / `#22c55e` — the stats card header's offline / connected colours.
const OFFLINE_RED: u32 = 0xf87171;
const CONNECTED_GREEN: u32 = 0x22c55e;

/// The stats card's `grid-cols-[auto_minmax(0,1fr)]` label column.
///
/// GPUI's `GridTemplate` only carries `repeat` + `min_size`, so there is no
/// `auto` track; the label column is pinned to the width the widest label
/// (`Est. resin (all):`) takes at 11px.
const STATS_LABEL_WIDTH: f32 = 92.0;

// ── Formatters ──────────────────────────────────────────────────────────────

/// `fmt.quantity(value, digits, unit)` — `{value} {unit}` with fixed decimals.
fn quantity(value: f32, digits: usize, unit: &str) -> String {
    format!("{value:.digits$} {unit}")
}

/// `formatBytes` in `EmptySceneState.tsx` (the recent-file size ladder).
fn format_recent_bytes(bytes: Option<u64>) -> Option<String> {
    let bytes = bytes?;
    if bytes < 1024 {
        return Some(format!("{bytes} B"));
    }
    let units = ["KB", "MB", "GB"];
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit < units.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    let rounded = if value >= 10.0 {
        value.round()
    } else {
        (value * 10.0).round() / 10.0
    };
    Some(format!("{rounded} {}", units[unit]))
}

/// `formatFileSize` in `meshStatsFormatting.ts` (the stats card's STL size).
fn format_file_size(bytes: Option<u64>) -> Option<String> {
    // Zero means "not known", not "empty".
    let bytes = bytes.filter(|bytes| *bytes > 0)?;
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        Some(quantity(bytes as f32 / GB as f32, 2, "GB"))
    } else if bytes >= MB {
        Some(quantity(bytes as f32 / MB as f32, 2, "MB"))
    } else if bytes >= KB {
        Some(quantity(bytes as f32 / KB as f32, 1, "KB"))
    } else {
        Some(quantity(bytes as f32, 0, "B"))
    }
}

/// `formatPolygonCountCompact` — `1.38M`, `50K`, `999`.
fn format_polygon_count(count: usize) -> String {
    if count >= 1_000_000 {
        let millions = count as f32 / 1_000_000.0;
        quantity(millions, if millions >= 10.0 { 0 } else { 2 }, "M")
    } else if count >= 1_000 {
        let thousands = count as f32 / 1_000.0;
        quantity(thousands, if thousands >= 10.0 { 0 } else { 1 }, "K")
    } else {
        format!("{count}")
    }
}

/// Days since 1970-01-01 to a proleptic Gregorian date (Howard Hinnant's
/// `civil_from_days`).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// `formatRecentOpenedAt` — `None` for "just now", a compact period otherwise.
fn format_recent_opened_at(opened_at_ms: i64) -> Option<String> {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or(opened_at_ms);
    let delta_ms = now_ms - opened_at_ms;
    if delta_ms < 0 {
        return None;
    }
    let delta_sec = delta_ms / 1000;
    if delta_sec < 60 {
        return None;
    }
    let delta_min = delta_sec / 60;
    if delta_min < 60 {
        return Some(format!("{delta_min} min"));
    }
    let delta_hours = delta_min / 60;
    if delta_hours < 24 {
        return Some(format!("{delta_hours} h"));
    }
    let delta_days = delta_hours / 24;
    if delta_days < 7 {
        return Some(format!("{delta_days} d"));
    }
    let (year, month, day) = civil_from_days(opened_at_ms.div_euclid(86_400_000));
    Some(format!("{month}/{day}/{year}"))
}

// ── EmptySceneState ─────────────────────────────────────────────────────────


/// `EmptySceneState.tsx`.
///
/// The outer box is `absolute inset-0 top-[var(--topbar-height)]` and centres
/// the panel; it carries no handlers, so it does not intercept the canvas.
pub fn empty_scene_state(
    state: EmptySceneState,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let EmptySceneState {
        loading,
        loading_label,
        loading_detail,
        show_first_time_onboarding,
        recent_opened_files,
        tagline,
        version,
        build_channel,
        build_label,
        beta,
        drop_active,
        drop_unsupported,
        copied,
    } = state;

    let build_info = match &build_label {
        Some(label) => format!(
            "DragonFruit {version} ({build_channel}) — {label}"
        ),
        None => format!("DragonFruit {version} ({build_channel})"),
    };

    // ── Version chip ────────────────────────────────────────────────────────
    let was_copied = copied.get();
    let (chip_text, chip_text_color, chip_border, chip_background, chip_shadow) = if was_copied {
        (
            "✓ Copied!".to_owned(),
            COPY_GREEN,
            COPY_GREEN,
            rgba(with_alpha(COPY_GREEN, 0x1f)),
            Some(with_alpha(COPY_GREEN, 0x4d)),
        )
    } else if beta {
        (
            format!("Version {version} — Beta"),
            BETA_TEXT_DARK,
            mix(BETA_ORANGE, BORDER_SUBTLE, 0.60),
            rgba(with_alpha(BETA_ORANGE, 0x0f)),
            Some(with_alpha(BETA_ORANGE, 0x29)),
        )
    } else {
        (
            format!("Version {version}"),
            ACCENT,
            mix(ACCENT, BORDER_SUBTLE, 0.60),
            rgba(with_alpha(ACCENT, 0x0f)),
            Some(with_alpha(ACCENT, 0x29)),
        )
    };

    let copy_entity = entity.clone();
    let copy_cell = copied.clone();
    let copy_text = build_info.clone();
    let mut chip = div()
        .id("empty-scene-build-chip")
        .flex()
        .items_center()
        .rounded_full()
        .border_1()
        .border_color(rgb(chip_border))
        .bg(chip_background)
        .px(px(CHIP_PADDING_X))
        .py(px(CHIP_PADDING_Y))
        .text_size(px(CHIP_FONT_SIZE))
        .font_weight(FontWeight::BOLD)
        .text_color(rgb(chip_text_color))
        .cursor_pointer()
        .child(chip_text);
    if let Some(shadow) = chip_shadow {
        chip = chip.shadow(vec![BoxShadow::new(
            px(0.0),
            px(0.0),
            rgba(shadow).into(),
        )
        .blur_radius(px(6.0))]);
    }
    chip = chip.on_click(move |_, _, cx| {
        cx.write_to_clipboard(ClipboardItem::new_string(copy_text.clone()));
        copy_cell.set(true);
        let entity = copy_entity.clone();
        let copied = copy_cell.clone();
        cx.spawn(async move |cx| {
            cx.background_executor()
                .timer(Duration::from_millis(COPIED_FLASH_MS))
                .await;
            copied.set(false);
            entity.update(cx, |_, cx| cx.notify()).ok();
        })
        .detach();
    });

    // `Tooltip content={…}`: "Click to copy build info" plus the build label.
    let tip = div()
        .flex()
        .flex_col()
        .child("Click to copy build info")
        .when_some(build_label.clone(), |this, label| this.child(label));
    let chip = ui::tooltip("empty-scene-build-chip-tip", chip, tip).mb(px(8.0));

    // ── Title and copy ──────────────────────────────────────────────────────
    let title = div()
        .text_size(px(28.0))
        .line_height(relative(LINE_HEIGHT_TIGHT))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT_STRONG))
        .max_w(px(460.0))
        .mb(px(SPACE_3))
        .child(tagline);

    let body_copy = div()
        .text_size(px(FONT_MD))
        .text_color(rgb(TEXT_MUTED))
        .max_w(px(560.0))
        .mb(px(SPACE_5))
        .child(
            "Bring in a mesh or scene to start preparing, analyzing, supporting, and exporting \
             your print.",
        );

    // ── Loading / onboarding / actions ──────────────────────────────────────
    let content = if loading {
        loading_card(loading_label, loading_detail)
    } else if show_first_time_onboarding {
        onboarding_card()
    } else {
        actions_and_recents(
            recent_opened_files,
            drop_active,
            drop_unsupported,
            entity.clone(),
        )
    };

    // `.ui-empty-state`
    let panel = div()
        .w(px(EMPTY_STATE_WIDTH))
        .max_w(px(EMPTY_STATE_WIDTH))
        .rounded(px(RADIUS_LG))
        .border_1()
        .border_color(rgba(with_alpha(BORDER_SUBTLE, 0xe6)))
        .bg(rgba(with_alpha(SURFACE_0, 0xf0)))
        .p(px(EMPTY_STATE_PADDING))
        .text_center()
        .flex()
        .flex_col()
        .items_center()
        .shadow(vec![BoxShadow::new(
            px(0.0),
            px(16.0),
            rgba(0x00000038).into(),
        )
        .blur_radius(px(34.0))])
        .child(chip)
        .child(title)
        .child(body_copy)
        .child(content);

    div()
        .absolute()
        .top(px(TOPBAR_HEIGHT))
        .left(px(0.0))
        .right(px(0.0))
        .bottom(px(0.0))
        .flex()
        .items_center()
        .justify_center()
        .child(panel)
}

/// The blocking-import card: `rounded-lg border px-4 py-4 text-left` plus an
/// indeterminate progress bar.
fn loading_card(
    label: Option<SharedString>,
    detail: Option<SharedString>,
) -> Div {
    div()
        .w_full()
        .rounded(px(ROUNDED_LG))
        .border_1()
        .border_color(rgb(mix(ACCENT, BORDER_SUBTLE, 0.35)))
        .bg(rgba(with_alpha(SURFACE_1, 0xeb)))
        .px(px(16.0))
        .py(px(16.0))
        .text_left()
        .child(
            div()
                .text_size(px(FONT_SM))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child(label.unwrap_or_else(|| "Importing your file…".into())),
        )
        .child(
            div()
                .mt(px(4.0))
                .text_size(px(11.0))
                .text_color(rgb(TEXT_MUTED))
                .child(
                    detail.unwrap_or_else(|| {
                        "Please hang tight while we prepare your scene.".into()
                    }),
                ),
        )
        .child(
            div()
                .mt(px(12.0))
                .child(ui::progress_bar(0.0, ui::ProgressSize::Sm, true)),
        )
}

/// The first-run card: two tall action buttons and the top-bar hint.
fn onboarding_card() -> Div {
    let primary_detail = mix(ACCENT_CONTRAST, BLACK, 0.84);
    let secondary_detail = mix(ACCENT_SECONDARY_CONTRAST, BLACK, 0.82);

    let add_printer = div()
        .id("empty-scene-add-printer")
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(mix(PRIMARY_BUTTON_SURFACE, WHITE, 0.84)))
        .bg(rgb(PRIMARY_BUTTON_SURFACE))
        .px(px(12.0))
        .py(px(12.0))
        .text_left()
        // `PORT GAP:` `onAddPrinter` opens the printer library, which the port
        // does not have.
        .child(
            div()
                .mb(px(4.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .text_size(px(FONT_SM))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(ACCENT_CONTRAST))
                .child(icon("printer", 16.0, ACCENT_CONTRAST))
                .child("Add printer"),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(primary_detail))
                .child("Open printer library and add one now."),
        );

    let use_without = div()
        .id("empty-scene-use-without-printer")
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(mix(SECONDARY_BUTTON_SURFACE, WHITE, 0.84)))
        .bg(rgb(SECONDARY_BUTTON_SURFACE))
        .px(px(12.0))
        .py(px(12.0))
        .text_left()
        // `PORT GAP:` `onUseWithoutPrinter` sets `allowPrepareWithoutPrinter`,
        // which the port does not track.
        .child(
            div()
                .mb(px(4.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .text_size(px(FONT_SM))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(ACCENT_SECONDARY_CONTRAST))
                .child(icon("wrench", 16.0, ACCENT_SECONDARY_CONTRAST))
                .child("Use without printer"),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(secondary_detail))
                .child("Keep going without a printer. You can add one later."),
        );

    div()
        .w_full()
        .rounded(px(ROUNDED_LG))
        .border_1()
        .border_color(rgba(with_alpha(BORDER_SUBTLE, 0xeb)))
        .bg(rgba(with_alpha(SURFACE_1, 0xf0)))
        .p(px(12.0))
        .text_left()
        .child(
            div()
                .mb(px(8.0))
                .text_size(px(10.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .child("GET STARTED"),
        )
        .child(div().flex().flex_col().gap(px(8.0)).child(add_printer).child(use_without))
        .child(
            div()
                .mt(px(8.0))
                .text_size(px(11.0))
                .text_color(rgb(TEXT_MUTED))
                .child("Add or switch printer anytime from the top bar."),
        )
}

/// The two action buttons plus the recent-files card and the drop zone.
fn actions_and_recents(
    recent_opened_files: Vec<RecentOpenedFile>,
    drop_active: bool,
    drop_unsupported: bool,
    entity: WeakEntity<DragonFruit>,
) -> Div {
    // ── Load Mesh (wired) ───────────────────────────────────────────────────
    let load_entity = entity.clone();
    let load_mesh = div()
        .id("empty-scene-load-mesh")
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(mix(PRIMARY_BUTTON_SURFACE, WHITE, 0.84)))
        .bg(rgb(PRIMARY_BUTTON_SURFACE))
        .px(px(12.0))
        .py(px(12.0))
        .text_center()
        .cursor_pointer()
        .on_click(move |_, _, cx| {
            load_entity
                .update(cx, |this, cx| this.open_stl(cx))
                .ok();
        })
        .child(
            div()
                .mb(px(4.0))
                .w_full()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(6.0))
                .text_size(px(FONT_SM))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(ACCENT_CONTRAST))
                .child(icon("upload", 16.0, ACCENT_CONTRAST))
                .child("Load Mesh"),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(mix(ACCENT_CONTRAST, BLACK, 0.84)))
                .child(".stl • .obj • .3mf"),
        );

    // ── Import Scene (`PORT GAP:`) ──────────────────────────────────────────
    // The source imports `.voxl` / `.lys` scenes through
    // `onImportSceneChange`; the port has no scene importer.
    let import_scene = div()
        .id("empty-scene-import-scene")
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(mix(SECONDARY_BUTTON_SURFACE, WHITE, 0.84)))
        .bg(rgb(SECONDARY_BUTTON_SURFACE))
        .px(px(12.0))
        .py(px(12.0))
        .text_center()
        .child(
            div()
                .mb(px(4.0))
                .w_full()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(6.0))
                .text_size(px(FONT_SM))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(ACCENT_SECONDARY_CONTRAST))
                .child(icon("folder-input", 16.0, ACCENT_SECONDARY_CONTRAST))
                .child("Import Scene"),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(mix(ACCENT_SECONDARY_CONTRAST, BLACK, 0.82)))
                // `sceneExtensionLabels.map(label => "." + label.toLowerCase())`.
                .child(".voxl • .lys"),
        );

    let buttons = div()
        .w_full()
        .flex()
        .gap(px(8.0))
        .child(div().flex_1().min_w(px(0.0)).child(load_mesh))
        .child(div().flex_1().min_w(px(0.0)).child(import_scene));

    // ── Recent files ────────────────────────────────────────────────────────
    let count = recent_opened_files.len().min(8);
    let header = div()
        .flex()
        .items_center()
        .justify_between()
        .text_size(px(10.0))
        .text_color(rgb(TEXT_MUTED))
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .child("RECENT FILES"),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                // `PORT GAP:` the `Clear` button renders only when
                // `onClearRecentFiles` is set; the port has no recent store.
                .child(
                    div()
                        .min_w(px(20.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .px(px(6.0))
                        .py(px(2.0))
                        .text_size(px(9.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_MUTED))
                        .bg(rgba(with_alpha(SURFACE_2, 0xbd)))
                        .border_1()
                        .border_color(rgba(with_alpha(BORDER_SUBTLE, 0xcc)))
                        .child(format!("{count}")),
                ),
        );

    let rows: Vec<_> = recent_opened_files
        .iter()
        .rev()
        .take(8)
        .map(recent_row)
        .collect();

    let list = if rows.is_empty() {
        div()
            .mt(px(6.0))
            .h(px(38.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(ROUNDED_MD))
            .text_size(px(10.0))
            .text_color(rgb(TEXT_MUTED))
            .child("No recent files")
    } else {
        div()
            .mt(px(6.0))
            .grid()
            .grid_cols(2)
            .gap(px(6.0))
            .children(rows)
    };

    // ── Drop zone ───────────────────────────────────────────────────────────
    let drop_border = if drop_active {
        if drop_unsupported { DANGER } else { ACCENT }
    } else {
        BORDER_SUBTLE
    };
    let drop_background = if drop_active {
        if drop_unsupported {
            rgba(with_alpha(mix(DANGER, SURFACE_0, 0.12), 0xff))
        } else {
            rgba(with_alpha(mix(ACCENT, SURFACE_0, 0.10), 0xff))
        }
    } else {
        rgba(with_alpha(SURFACE_1, 0xd6))
    };
    let drop_icon_color = if drop_unsupported { DANGER } else { ACCENT };
    let drop_title = if drop_unsupported {
        "Unsupported file type"
    } else {
        "Drag & Drop Area"
    };

    let drop_zone = div()
        .mt(px(8.0))
        .min_h(px(64.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_dashed()
        .border_color(rgb(drop_border))
        .bg(drop_background)
        .px(px(16.0))
        .py(px(12.0))
        .text_center()
        .child(
            div()
                .min_w(px(0.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(px(8.0))
                        .child(icon("upload", 16.0, drop_icon_color))
                        .child(
                            div()
                                .truncate()
                                .text_size(px(FONT_SM))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child(drop_title),
                        ),
                )
                .when(drop_unsupported, |this| {
                    this.child(
                        div()
                            .mt(px(4.0))
                            .text_size(px(11.0))
                            .text_color(rgb(DANGER))
                            .child("Unsupported format detected."),
                    )
                }),
        );

    let recents = div()
        .mt(px(8.0))
        .w_full()
        .rounded(px(ROUNDED_LG))
        .border_1()
        .border_color(rgba(with_alpha(BORDER_SUBTLE, 0xeb)))
        .bg(rgba(with_alpha(SURFACE_1, 0xf0)))
        .p(px(12.0))
        .text_left()
        .child(header)
        .child(list)
        // `PORT GAP:` `reopenError` — nothing can reopen a recent file here.
        .child(drop_zone);

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(buttons)
        .child(recents)
}

/// One recent-file row. The source's `<button>` is `disabled` whenever
/// `onReopenRecentFile` is absent, which is always the case here.
fn recent_row(entry: &RecentOpenedFile) -> Div {
    let accent = entry.kind.accent();
    let period = format_recent_opened_at(entry.opened_at_ms);
    let meta = match period {
        Some(period) => format!("last opened {period} ago"),
        None => "last opened just now".to_owned(),
    };
    let size = format_recent_bytes(entry.size_bytes).unwrap_or_else(|| "cached".to_owned());

    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgba(with_alpha(BORDER_SUBTLE, 0xdb)))
        .bg(rgba(with_alpha(SURFACE_1, 0xeb)))
        .px(px(8.0))
        .py(px(6.0))
        .text_size(px(10.0))
        .text_left()
        .text_color(rgb(TEXT_STRONG))
        .cursor_not_allowed()
        .child(
            div()
                .min_w(px(0.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .rounded(px(ROUNDED))
                        .px(px(4.0))
                        .py(px(2.0))
                        .text_size(px(9.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(accent))
                        .bg(rgb(mix(accent, SURFACE_0, 0.88)))
                        .border_1()
                        .border_color(rgb(mix(accent, BORDER_SUBTLE, 0.46)))
                        .child(entry.kind.label()),
                )
                .child(
                    div()
                        .min_w(px(0.0))
                        .child(
                            div()
                                .max_w(px(132.0))
                                .truncate()
                                .text_size(px(10.0))
                                .line_height(relative(LINE_HEIGHT_TIGHT))
                                .child(entry.name.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(9.0))
                                .text_color(rgb(TEXT_MUTED))
                                .child(meta),
                        ),
                ),
        )
        .child(
            div()
                .flex_shrink_0()
                .text_size(px(9.0))
                .text_color(rgb(TEXT_MUTED))
                .child(size),
        )
}

// ── ModelStatsCard ──────────────────────────────────────────────────────────

/// details on click. See the module deviations for the missing 3D rotation.
pub fn model_stats_card(
    state: ModelStatsCardState,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let ModelStatsCardState {
        model,
        num_layers,
        header,
        header_connected,
        header_offline,
        printer_name,
        material_name,
        layer_height_mm,
        normal_exposure_s,
        bottom_exposure_s,
        estimated_print_time,
        estimated_resin,
        estimated_resin_total,
        flipped,
    } = state;

    let show_back = flipped.get();
    let header_color = if header_offline {
        mix(OFFLINE_RED, TEXT_STRONG, 0.58)
    } else if header_connected {
        mix(CONNECTED_GREEN, TEXT_STRONG, 0.18)
    } else {
        TEXT_STRONG
    };

    let face = if show_back {
        card_back(model)
    } else {
        card_front(
            header,
            header_color,
            printer_name,
            material_name,
            layer_height_mm,
            normal_exposure_s,
            bottom_exposure_s,
            num_layers,
            estimated_print_time,
            estimated_resin,
            estimated_resin_total,
        )
    };

    let flip_cell = flipped.clone();
    let flip_entity = entity.clone();
    div()
        .id("model-stats-card")
        .w(px(320.0))
        .max_w(px(320.0))
        .cursor_pointer()
        .on_click(move |_, _, cx| {
            flip_cell.set(!flip_cell.get());
            flip_entity.update(cx, |_, cx| cx.notify()).ok();
        })
        .child(face)
}

/// The shared `.ui-panel rounded-md px-3 py-2.5 shadow-md space-y-1.5` box.
fn stats_panel() -> Div {
    div()
        .w_full()
        .min_w(px(0.0))
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgba(with_alpha(SURFACE_0, 0xeb)))
        .px(px(12.0))
        .py(px(10.0))
        .flex()
        .flex_col()
        .gap(px(6.0))
        .shadow_md()
}

/// The rows container: `grid-cols-[auto_minmax(0,1fr)] gap-x-2 gap-y-0.5
/// text-[11px]`.
fn stats_rows() -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .text_size(px(11.0))
        .text_color(rgb(TEXT_MUTED))
}

/// One `label / value` row. See [`STATS_LABEL_WIDTH`] for the pinned column.
fn stats_row(label: &'static str, value: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .gap(px(8.0))
        .child(div().w(px(STATS_LABEL_WIDTH)).flex_shrink_0().child(label))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .truncate()
                .text_color(rgb(TEXT_STRONG))
                .child(value.into()),
        )
}

/// A row whose value is a link into the profile settings modal.
fn stats_link_row(label: &'static str, value: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .gap(px(8.0))
        .child(div().w(px(STATS_LABEL_WIDTH)).flex_shrink_0().child(label))
        // `PORT GAP:` the value opens `openProfileSettingsModal('printer' |
        // 'material')`; the profile modal is not ported.
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .truncate()
                .text_left()
                .text_color(rgb(TEXT_STRONG))
                .child(value.into()),
        )
}

/// The front face: printer, material, layer profile, exposure, layers and the
/// slicer estimates.
#[allow(clippy::too_many_arguments)]
fn card_front(
    header: SharedString,
    header_color: u32,
    printer_name: Option<SharedString>,
    material_name: Option<SharedString>,
    layer_height_mm: Option<f32>,
    normal_exposure_s: Option<f32>,
    bottom_exposure_s: Option<f32>,
    num_layers: u32,
    estimated_print_time: Option<SharedString>,
    estimated_resin: Option<SharedString>,
    estimated_resin_total: Option<SharedString>,
) -> Div {
    let layer_profile = match layer_height_mm {
        Some(height) => quantity(height * 1000.0, 0, "μm"),
        None => "-".to_owned(),
    };
    let exposure = match normal_exposure_s {
        Some(normal) => format!(
            "{} • {}",
            quantity(normal, 1, "s"),
            quantity(bottom_exposure_s.unwrap_or(normal), 1, "s")
        ),
        None => "-".to_owned(),
    };
    let layers = if num_layers > 0 {
        format!("{num_layers}")
    } else {
        "-".to_owned()
    };

    let mut rows = stats_rows()
        .child(stats_link_row(
            "Printer:",
            printer_name.unwrap_or_else(|| "-".into()),
        ))
        .child(stats_link_row(
            "Material:",
            material_name.unwrap_or_else(|| "-".into()),
        ))
        .child(stats_row("Layer profile:", layer_profile))
        .child(stats_row("Exposure:", exposure))
        .child(stats_row("Layers:", layers))
        .child(stats_row(
            "Est. print time:",
            estimated_print_time.unwrap_or_else(|| "-".into()),
        ))
        .child(stats_row(
            "Est. resin:",
            estimated_resin.unwrap_or_else(|| "-".into()),
        ));
    if let Some(total) = estimated_resin_total {
        rows = rows
            .child(stats_row("Est. resin (all):", total));
    }

    stats_panel()
        .child(
            div()
                .truncate()
                .text_size(px(12.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(header_color))
                .child(header),
        )
        .child(rows)
        .child(
            div()
                .pt(px(2.0))
                .text_size(px(10.0))
                .text_color(rgb(TEXT_MUTED))
                .child("Click card to view model details"),
        )
}

/// The back face: STL size, triangles, shells and height.
fn card_back(model: Option<ModelStatsModel>) -> Div {
    let name = model
        .as_ref()
        .map(|model| model.name.clone())
        .unwrap_or_else(|| "No model selected".into());
    let stl_size = model
        .as_ref()
        .and_then(|model| format_file_size(model.file_size_bytes))
        .unwrap_or_else(|| "-".to_owned());
    let triangles = model
        .as_ref()
        .map(|model| format_polygon_count(model.polygon_count))
        .unwrap_or_else(|| "-".to_owned());
    let shells = model
        .as_ref()
        .and_then(|model| model.shells)
        .map(|shells| shells.to_string())
        .unwrap_or_else(|| "-".to_owned());
    let height = model
        .as_ref()
        .map(|model| quantity(model.height_mm, 2, "mm"))
        .unwrap_or_else(|| "-".to_owned());
    let defects = model.as_ref().and_then(|model| model.defect_errors);

    let mut panel = stats_panel().child(
        div()
            .truncate()
            .text_size(px(12.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(TEXT_STRONG))
            .child(name),
    );

    panel = panel.child(
        stats_rows()
            .child(stats_row("STL size:", stl_size))
            .child(stats_row("Triangles:", triangles))
            .child(stats_row("Shells:", shells))
            .child(stats_row("Height:", height)),
    );

    if let Some(errors) = defects {
        panel = panel.child(
            div()
                .flex()
                .items_start()
                .gap(px(6.0))
                .rounded(px(ROUNDED))
                .px(px(8.0))
                .py(px(4.0))
                .text_size(px(10.0))
                .bg(rgb(mix(DEFECT_AMBER, SURFACE_1, 0.82)))
                .text_color(rgb(mix(DEFECT_AMBER, TEXT_STRONG, 0.20)))
                .border_1()
                .border_color(rgba(with_alpha(DEFECT_AMBER, 0x73)))
                .child("⚠")
                .child(format!("Defective — {errors} errors")),
        );
    }

    panel.child(
        div()
            .pt(px(2.0))
            .text_size(px(10.0))
            .text_color(rgb(TEXT_MUTED))
            .child("Click card to return to print settings"),
    )
}
