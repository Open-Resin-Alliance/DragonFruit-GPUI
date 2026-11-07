//! Ported primitives from `src/components/atoms` and `src/components/ui`.
//!
//! Every value here comes from the source component's class strings and the
//! tokens in `theme.rs`; nothing is invented. Where GPUI has no equivalent for a
//! CSS feature (transitions, gradients, `text-overflow: fade`, portals, backdrop
//! blur) the nearest faithful substitute is used and called out on the item.
//!
//! Tailwind's radius scale is **not** overridden by `globals.css`: its
//! `@theme inline` block only re-declares colours and fonts, so the utilities
//! keep Tailwind v4's defaults — `rounded-sm` = 4px, `rounded-md` = 6px,
//! `rounded-lg` = 8px, `rounded-xl` = 12px, bare `rounded` = 4px,
//! `rounded-[Npx]` = N. The `--radius-sm: 6px` / `--radius-md: 10px` /
//! `--radius-lg: 14px` custom properties in `:root` are a *separate* scale that
//! only the hand-written `.ui-input`, `.ui-button` and `.ui-panel` rules read
//! through `var(--radius-*)`; those keep `theme::RADIUS_*`.
//!
//! ## PORT DEVIATIONS
//!
//! Platform gaps, all recorded at the item they affect:
//!
//! - **Transitions / keyframes** — `transition-colors`, `transition-transform`,
//!   `animate-pulse`, the toast fade-and-rise and the indeterminate progress
//!   sweep are all static here; GPUI has no CSS animation. Every element is
//!   drawn in its settled state.
//! - **`backdrop-filter`** — `backdrop-blur-sm` on the dialog backdrop, the
//!   select menu (`blur(6px) saturate(112%)`) and `backdrop-blur-[1px]` on the
//!   blocking overlay are omitted; the translucent fill is kept.
//! - **Portals** — `Tooltip`, `ContextMenu`, `SelectDropdown`'s menu and
//!   `StructuredDialogModal` are portalled to `<body>` at the source. GPUI has
//!   no portal, so each renders where the caller places it (the select menu and
//!   the tooltip as absolute children of their trigger).
//! - **Measurement** — no `getBoundingClientRect` pass, so the viewport clamps,
//!   the upward flip, the right-edge flip and the cursor-following placement
//!   that the source computes after mount are not reproduced. `MouseTooltip`
//!   takes the cursor point from the caller instead of listening to `mousemove`.
//! - **`letter-spacing`** — `tracking-wide` has no GPUI equivalent and is not
//!   applied (segmented pills, select headings, blocking-overlay elapsed line).
//! - **`title` attributes / accessible names** — GPUI widgets carry no
//!   `aria-*`/`title`, so those are dropped rather than faked.
//! - **`text-overflow: fade`** — `FadeText` clips (`overflow: hidden`,
//!   `white-space: nowrap`, `text-overflow: clip`) without the trailing mask.
//! - **`currentColor`** — an `svg()` element in GPUI does not inherit text
//!   colour, so every icon colour is passed explicitly.
//! - **`shadow-lg` / `shadow-xl` / `shadow-2xl`** — GPUI's `shadow_*` helpers
//!   carry Tailwind's exact values, so those are used as-is. The custom
//!   `box-shadow` strings in the sources are transcribed layer by layer.
//! - **Unitless `line-height`** — CSS inherits `1.45` / `1.3` as a *number* and
//!   recomputes it per element; GPUI's `line_height` is a length, so the value
//!   set on a container is inherited as-is. Elements that change their own font
//!   size therefore set their own line-height, and a container's ratio is
//!   resolved against the container's font size.
//! - **Text editing** — `NumberInput`, `ScrollableNumberField`,
//!   `CompactNumberField` and `MiniStepperField` render the frame, the value and
//!   the steppers. GPUI has no editable text field in this port, so typing, the
//!   locale parse/format, the wheel step and the blur commit stay with the
//!   caller; the step callbacks are the atoms' only interaction.
//! - **`w-[min(520px,92vw)]`** — `BlockingOverlay`'s panel is `92%` of the
//!   overlay (the window) capped at 520px, which is the same box.
//! - **Select menu height** — the source measures the trigger and caps the menu
//!   at `max(120, space below)`; with no measure pass the port caps it at 320px.
//! - **`title` / `aria-*` on options** — `SegmentedOption.title` and the
//!   `aria-label`s the sources pass have no GPUI counterpart.
//! - **Cascade layers (confirmed)** — Tailwind v4 emits its utilities inside
//!   `@layer utilities`, and `globals.css` declares `.ui-input`, `.ui-select`
//!   and `.ui-button` **unlayered**, so those three rules outrank every normal
//!   utility on the same element. Consequences the port applies:
//!   `.ui-input` keeps radius 6px, `padding: 6px 10px`, `font-size: 13px` and
//!   `line-height: 1.3` against `rounded-[4px]`, `px-*`, `py-0`, `pr-10`,
//!   `text-sm` and `text-xs`; `.ui-button` keeps `padding: 5px 11px 7px 11px`
//!   and `font-size: 14px` against `px-2`/`px-2.5`/`px-3`/`px-4` and
//!   `text-[11px]`/`text-xs`/`text-sm`, so a `Button` size class only supplies a
//!   height. `!important` utilities (`!h-7`, `!p-2`, `!bg-[…]`) still win, which
//!   is why the `IconButton` and `ScrollableNumberField` stepper padding applies.
//!   `.ui-panel`'s `h3` rule is likewise unlayered and `!important` on
//!   font-size, so panel titles stay 14px with the 1.2 line-height.

use declarative_gpui::ui;
use gpui::{
    App, BoxShadow, ClickEvent, Div, ElementId, FontWeight, IntoElement, Pixels, Point,
    SharedString, Stateful, Window, div, linear_color_stop, linear_gradient, point, prelude::*, px,
    relative, rgb, rgba,
};
use std::rc::Rc;

use gpui_core::icons::icon;
use gpui_core::theme::*;
pub use gpui_core::state::ToastTone;

/// Tailwind v4's radius scale — what `rounded-*` utilities actually resolve to.
/// See the module docs for why these are not `theme::RADIUS_*`.
const ROUNDED: f32 = 4.0;
const ROUNDED_SM: f32 = 4.0;
const ROUNDED_MD: f32 = 6.0;
const ROUNDED_LG: f32 = 8.0;
const ROUNDED_XL: f32 = 12.0;

/// Tailwind's `leading-tight` (1.25) — distinct from the app's
/// `--line-height-tight` (1.2) that `.ui-panel` reads.
const LEADING_TIGHT: f32 = 1.25;
/// Tailwind's `leading-snug` (1.375).
const LEADING_SNUG: f32 = 1.375;
/// Tailwind's `leading-relaxed` (1.625).
const LEADING_RELAXED: f32 = 1.625;

/// `#ffffff` — the toggle knob and the `bg-white/10` ghost hover.
const WHITE: u32 = 0xffffff;
/// `#000000` — the base of every `color-mix(..., black N%)` in the sources.
const BLACK: u32 = 0x000000;

/// `#rrggbb` plus an alpha byte, for `gpui::rgba`.
fn with_alpha(color: u32, alpha: u8) -> u32 {
    (color << 8) | alpha as u32
}

/// A CSS `box-shadow` layer: `offset-x offset-y blur spread rgba(...)`.
fn shadow(x: f32, y: f32, blur: f32, spread: f32, color: u32) -> BoxShadow {
    BoxShadow::new(px(x), px(y), rgba(color).into())
        .blur_radius(px(blur))
        .spread_radius(px(spread))
}

/// A CSS `box-shadow` layer with `inset`.
fn inset_shadow(x: f32, y: f32, blur: f32, spread: f32, color: u32) -> BoxShadow {
    shadow(x, y, blur, spread, color).inset()
}

// ── Button ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Accent,
    Danger,
    TintedAccent,
    TintedDanger,
    TintedWarning,
    TintedSuccess,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonSize {
    Xs,
    Sm,
    Md,
    Lg,
    Auto,
}

/// `(background, border-color, text-color, hover background)` for a variant.
///
/// `Button.tsx` maps each variant to a `.ui-button*` class; the colours below are
/// that class's declaration in `globals.css`.
fn button_colors(variant: ButtonVariant) -> (u32, u32, u32, u32) {
    match variant {
        ButtonVariant::Primary => (
            PRIMARY_BUTTON_SURFACE,
            mix(PRIMARY_BUTTON_SURFACE, WHITE, 0.86),
            ACCENT_CONTRAST,
            mix(PRIMARY_BUTTON_SURFACE, BLACK, 0.88),
        ),
        ButtonVariant::Secondary => (SURFACE_1, BORDER_SUBTLE, TEXT_STRONG, SURFACE_2),
        ButtonVariant::Accent => (
            SECONDARY_BUTTON_SURFACE,
            mix(SECONDARY_BUTTON_SURFACE, WHITE, 0.86),
            ACCENT_SECONDARY_CONTRAST,
            mix(SECONDARY_BUTTON_SURFACE, BLACK, 0.91),
        ),
        ButtonVariant::Danger => (
            0xdc2626,
            mix(0xdc2626, WHITE, 0.85),
            0xfff4f4,
            0xb91c1c,
        ),
        ButtonVariant::TintedAccent => (
            mix(ACCENT, SURFACE_1, 0.14),
            mix(ACCENT, BORDER_SUBTLE, 0.55),
            ACCENT,
            mix(ACCENT, SURFACE_1, 0.24),
        ),
        ButtonVariant::TintedDanger => (
            mix(DANGER, SURFACE_1, 0.14),
            mix(DANGER, BORDER_SUBTLE, 0.55),
            DANGER,
            mix(DANGER, SURFACE_1, 0.24),
        ),
        ButtonVariant::TintedWarning => (
            mix(0xf59e0b, SURFACE_1, 0.14),
            mix(0xf59e0b, BORDER_SUBTLE, 0.55),
            0xf59e0b,
            mix(0xf59e0b, SURFACE_1, 0.24),
        ),
        ButtonVariant::TintedSuccess => (
            mix(SUCCESS, SURFACE_1, 0.14),
            mix(SUCCESS, BORDER_SUBTLE, 0.55),
            SUCCESS,
            mix(SUCCESS, SURFACE_1, 0.24),
        ),
    }
}

/// The height a size class pins, if any.
///
/// `.ui-button` is **unlayered** CSS, so its `padding: 5px 11px 7px 11px` and
/// `font-size: var(--font-size-md)` beat the layered `px-*` / `text-*` utilities
/// on the same element. Only the height survives from each size class — through
/// `!h-7` / `!h-8` / `!h-9` (important utilities, which *do* outrank unlayered
/// normal declarations) and `h-10` for `lg`, where `.ui-button` sets no height.
/// `auto` emits no height at all.
fn button_height(size: ButtonSize) -> Option<f32> {
    match size {
        ButtonSize::Xs => Some(28.0),
        ButtonSize::Sm => Some(32.0),
        ButtonSize::Md => Some(36.0),
        ButtonSize::Lg => Some(40.0),
        ButtonSize::Auto => None,
    }
}

/// `Button.tsx` + `.ui-button*` in `globals.css`.
///
/// `.ui-button`: `inline-flex items-center justify-center border rounded-sm`,
/// `padding: 5px 11px 7px 11px`, `font-size: var(--font-size-md)` (14px),
/// `font-weight: 600`, `line-height: 1`. The size class only contributes a
/// height — see [`button_height`].
pub fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    variant: ButtonVariant,
    size: ButtonSize,
) -> Stateful<Div> {
    let label: SharedString = label.into();
    let (background, border, text, hover) = button_colors(variant);
    let height = button_height(size);
    let mut el = div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .pt(px(5.0))
        .pb(px(7.0))
        .px(px(11.0))
        .text_size(px(FONT_MD))
        .font_weight(FontWeight::SEMIBOLD)
        .line_height(px(FONT_MD))
        .text_color(rgb(text))
        .cursor_pointer()
        .hover(move |style| style.bg(rgb(hover)))
        .child(label);
    if let Some(height) = height {
        el = el.h(px(height));
    }
    el
}

/// `.ui-button:disabled` — the disabled recipe from `globals.css`.
///
/// GPUI divs carry no `disabled` attribute, so a caller that renders a button
/// which cannot be pressed uses this instead of [`button`].
pub fn button_disabled(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    size: ButtonSize,
) -> Stateful<Div> {
    let label: SharedString = label.into();
    let height = button_height(size);
    let mut el = div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(mix(BORDER_SUBTLE, BLACK, 0.90)))
        .bg(rgb(mix(SURFACE_1, BLACK, 0.92)))
        .pt(px(5.0))
        .pb(px(7.0))
        .px(px(11.0))
        .text_size(px(FONT_MD))
        .font_weight(FontWeight::SEMIBOLD)
        .line_height(px(FONT_MD))
        .text_color(rgb(mix(TEXT_MUTED, SURFACE_2, 0.82)))
        .cursor_not_allowed()
        .child(label);
    if let Some(height) = height {
        el = el.h(px(height));
    }
    el
}

// ── IconButton ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IconButtonVariant {
    Solid,
    Surface,
    Ghost,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IconButtonSize {
    Xs,
    Sm,
    Md,
    Lg,
    Auto,
}

/// `IconButton.tsx`.
///
/// `solid` is the `.ui-button ui-button-secondary` tile (the default), `surface`
/// the bordered `rounded-md` tile a modal uses, `ghost` a bare hit target that
/// shows `bg-white/10` on hover. The icon is 16px (`w-4 h-4`), as at the source
/// call sites.
pub fn icon_button(
    id: impl Into<ElementId>,
    icon_name: &str,
    variant: IconButtonVariant,
    size: IconButtonSize,
) -> Stateful<Div> {
    icon_button_toned(id, icon_name, 16.0, variant, size, IconTone::Neutral, false)
}

/// [`icon_button`] with the source's `tone` (read only by `surface`) and its
/// `active` flag (a `solid` tile turns `ui-button-primary`; a `surface` tile
/// takes the accent tone with `--accent-secondary` as the glyph colour).
pub fn icon_button_toned(
    id: impl Into<ElementId>,
    icon_name: &str,
    icon_size: f32,
    variant: IconButtonVariant,
    size: IconButtonSize,
    tone: IconTone,
    active: bool,
) -> Stateful<Div> {
    match variant {
        IconButtonVariant::Solid => {
            // `active ? 'ui-button-primary' : 'ui-button-secondary'`.
            let (background, border, foreground, hover) = if active {
                button_colors(ButtonVariant::Primary)
            } else {
                button_colors(ButtonVariant::Secondary)
            };
            let mut el = div()
                .id(id)
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(RADIUS_SM))
                .border_1()
                .border_color(rgb(border))
                .bg(rgb(background))
                .cursor_pointer()
                .hover(move |style| style.bg(rgb(hover)))
                .child(icon(icon_name, icon_size, foreground));
            match size {
                // `!p-1` → 4px + icon + 2px border.
                IconButtonSize::Xs => el = el.p(px(4.0)),
                // `h-8 w-8 !p-0`.
                IconButtonSize::Sm => el = el.w(px(32.0)).h(px(32.0)),
                // `!p-2` → 8px + icon + 2px border.
                IconButtonSize::Md => el = el.p(px(8.0)),
                // `h-10 w-10 !p-0`.
                IconButtonSize::Lg => el = el.w(px(40.0)).h(px(40.0)),
                // `.ui-button` base padding.
                IconButtonSize::Auto => el = el.pt(px(5.0)).pb(px(7.0)).px(px(11.0)),
            }
            el
        }
        IconButtonVariant::Surface => {
            // `boxSizeClassMap`: xs h-6 w-6, sm h-8 w-8, md h-9 w-9, lg h-10
            // w-10; `auto` emits no box at all.
            let box_size = match size {
                IconButtonSize::Xs => Some(24.0),
                IconButtonSize::Sm => Some(32.0),
                IconButtonSize::Md => Some(36.0),
                IconButtonSize::Lg => Some(40.0),
                IconButtonSize::Auto => None,
            };
            let (border, background, foreground) = if active {
                let (border, background, _) = icon_tone_colors(IconTone::Accent);
                (border, background, ACCENT_SECONDARY)
            } else {
                icon_tone_colors(tone)
            };
            let mut el = div()
                .id(id)
                .flex()
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .rounded(px(ROUNDED_MD))
                .border_1()
                .border_color(rgb(border))
                .bg(rgb(background))
                .cursor_pointer()
                .child(icon(icon_name, icon_size, foreground));
            if let Some(box_size) = box_size {
                el = el.w(px(box_size)).h(px(box_size));
            }
            el
        }
        IconButtonVariant::Ghost => {
            let mut el = div()
                .id(id)
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(ROUNDED))
                .text_color(rgb(TEXT_MUTED))
                .cursor_pointer()
                .hover(|style| style.bg(rgba(with_alpha(WHITE, 0x1a))))
                .child(icon(icon_name, icon_size, TEXT_MUTED));
            match size {
                IconButtonSize::Xs => el = el.p(px(2.0)),
                IconButtonSize::Sm => el = el.p(px(4.0)),
                IconButtonSize::Md => el = el.p(px(6.0)),
                IconButtonSize::Lg => el = el.p(px(8.0)),
                IconButtonSize::Auto => {}
            }
            el
        }
    }
}

// ── Card ────────────────────────────────────────────────────────────────────

/// `Card.tsx`: `ui-panel rounded-lg shadow-lg border`.
///
/// `.ui-panel` is `surface-0`, `border-subtle`, `--font-size-sm` /
/// `--line-height-normal`.
pub fn card() -> Div {
    ui! {
        div(
            flex, flex_col, w_full,
            rounded = px(ROUNDED_LG),
            border, border_color = rgb(BORDER_SUBTLE),
            bg = rgb(SURFACE_0),
            shadow_lg,
            text_size = px(FONT_SM),
            line_height = px(FONT_SM * LINE_HEIGHT_NORMAL),
        )
    }
}

/// `CardHeader.tsx`: `relative px-2.5 py-2.5 flex items-center`, the title in
/// the left `flex items-center gap-2.5 pr-8` block.
pub fn card_header(title: impl Into<SharedString>) -> Div {
    card_header_with_right(title, None::<Div>)
}

/// [`card_header`] with the `absolute right-2.5 top-1/2` right slot.
pub fn card_header_with_right(
    title: impl Into<SharedString>,
    right: Option<impl IntoElement>,
) -> Div {
    let title: SharedString = title.into();
    let left = div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .pr(px(32.0))
        .child(
            div()
                .text_size(px(FONT_MD))
                // `.ui-panel h3` pins the line-height to `--line-height-tight`.
                .line_height(px(FONT_MD * LINE_HEIGHT_TIGHT))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child(title),
        );
    let mut header = div()
        .relative()
        .px(px(10.0))
        .py(px(10.0))
        .flex()
        .items_center()
        .child(left);
    if let Some(right) = right {
        header = header.child(
            div()
                .absolute()
                .right(px(10.0))
                .top(px(0.0))
                .bottom(px(0.0))
                .flex()
                .items_center()
                .child(right),
        );
    }
    header
}

// ── Toggle ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToggleSize {
    Sm,
    Md,
}

/// `Toggle.tsx` track and knob, `md` geometry.
pub fn toggle(on: bool) -> Div {
    toggle_sized(on, ToggleSize::Md)
}

/// `Toggle.tsx`.
///
/// Track: `flex shrink-0 items-center rounded-full px-0.5` with
/// `h-5 w-9` (sm) or `h-6 w-10` (md); `background: var(--accent)` when checked,
/// `var(--surface-2)` otherwise. Knob: `rounded-full bg-white shadow`, `h-4 w-4`
/// (sm) / `h-5 w-5` (md), translated 16px right when checked.
pub fn toggle_sized(on: bool, size: ToggleSize) -> Div {
    let (track_w, track_h, knob) = match size {
        ToggleSize::Sm => (36.0, 20.0, 16.0),
        ToggleSize::Md => (40.0, 24.0, 20.0),
    };
    ui! {
        div(
            // `px-0.5` is 2px; the knob's `translate-x-4` (16px) then lands it on
            // the far padding edge of both tracks.
            flex, items_center, flex_shrink_0, rounded_full, px = px(2.0),
            w = px(track_w), h = px(track_h),
            bg = rgb(if on { ACCENT } else { SURFACE_2 }),
            when(on, justify_end),
            when(!on, justify_start),
        ) {
            div(rounded_full, bg = rgb(WHITE), shadow_sm, w = px(knob), h = px(knob))
        }
    }
}

/// [`toggle_sized`] as the interactive `role="switch"` the source renders: the
/// track becomes the hit target and reports the next value on click.
pub fn toggle_switch(
    id: impl Into<ElementId>,
    on: bool,
    size: ToggleSize,
    on_change: impl Fn(bool, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    toggle_sized(on, size)
        .id(id)
        .cursor_pointer()
        .on_click(move |_, window, cx| on_change(!on, window, cx))
}

// ── SegmentedControl ────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SegmentedTone {
    Accent,
    AccentSecondary,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SegmentedSize {
    Sm,
    Md,
}

/// One pill of a [`segmented_control`].
pub struct SegmentedOption {
    pub value: SharedString,
    pub label: SharedString,
    pub disabled: bool,
}

impl SegmentedOption {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }
}

/// `SegmentedControl.tsx`.
///
/// The row is `inline-flex items-center gap-1`; each pill is
/// `.ui-segmented-option inline-flex items-center justify-center rounded-sm
/// border font-semibold uppercase tracking-wide` at `h-7 px-2.5 text-xs` (sm) or
/// `h-10 min-w-[92px] px-3 text-[12px]` (md). `--segmented-tone` is the active
/// tint. GPUI has no `letter-spacing`, so `tracking-wide` is not applied; the
/// label is upper-cased in Rust to match `uppercase`.
///
/// `on_change` receives the index of the pressed pill.
pub fn segmented_control(
    id: impl Into<SharedString>,
    options: &[SegmentedOption],
    value: &str,
    size: SegmentedSize,
    tone: SegmentedTone,
    full_width: bool,
    on_change: impl Fn(usize, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let tone_color = match tone {
        SegmentedTone::Accent => ACCENT,
        SegmentedTone::AccentSecondary => ACCENT_SECONDARY,
    };
    let on_change = Rc::new(on_change);
    let (height, pad_x, font_size, min_w) = match size {
        SegmentedSize::Sm => (28.0, 10.0, FONT_XS, 0.0),
        SegmentedSize::Md => (40.0, 12.0, 12.0, 92.0),
    };

    let mut row = div().id(id.clone()).flex().items_center().gap(px(4.0));
    if full_width {
        row = row.w_full();
    }

    for (index, option) in options.iter().enumerate() {
        let active = option.value.as_ref() == value;
        let callback = on_change.clone();
        let mut pill = div()
            .id(ElementId::named_usize(id.clone(), index))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(ROUNDED_SM))
            .border_1()
            .h(px(height))
            .px(px(pad_x))
            .text_size(px(font_size))
            // `text-xs` (sm) carries a 16px line-height; the md pill's
            // `text-[12px]` is arbitrary and inherits.
            .line_height(px(match size {
                SegmentedSize::Sm => 16.0,
                SegmentedSize::Md => 12.0 * LINE_HEIGHT_NORMAL,
            }))
            .font_weight(FontWeight::SEMIBOLD)
            .child(option.label.clone().to_uppercase());
        if min_w > 0.0 {
            pill = pill.min_w(px(min_w));
        }
        if full_width {
            pill = pill.flex_1();
        }

        pill = if option.disabled {
            pill.bg(rgb(mix(SURFACE_1, BLACK, 0.92)))
                .border_color(rgb(mix(BORDER_SUBTLE, BLACK, 0.90)))
                .text_color(rgb(mix(TEXT_MUTED, SURFACE_2, 0.82)))
                .cursor_not_allowed()
        } else if active {
            // `[data-active="true"]` — the tint mixed into surface-0 at 76%,
            // border at 90% of the tint, text at 75% of text-strong. The
            // `:hover:not(:disabled)` rule is more specific than
            // `[data-active]`, so hovering an active pill still shows
            // surface-2.
            pill.bg(rgb(mix(tone_color, SURFACE_0, 0.24)))
                .border_color(rgb(mix(tone_color, WHITE, 0.90)))
                .text_color(rgb(mix(tone_color, TEXT_STRONG, 0.75)))
                .cursor_pointer()
                .hover(|style| style.bg(rgb(SURFACE_2)))
                .on_click(move |_, window, cx| callback(index, window, cx))
        } else {
            pill.bg(rgb(SURFACE_1))
                .border_color(rgb(BORDER_SUBTLE))
                .text_color(rgb(TEXT_MUTED))
                .cursor_pointer()
                .hover(|style| style.bg(rgb(SURFACE_2)))
                .on_click(move |_, window, cx| callback(index, window, cx))
        };
        row = row.child(pill);
    }
    row
}

// ── Select / SelectDropdown ─────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SelectOptionTone {
    Default,
    Accent,
    Danger,
}

/// One entry of a [`select_dropdown`] menu.
pub struct SelectOption {
    pub value: SharedString,
    pub label: SharedString,
    pub disabled: bool,
    /// Render as a non-interactive group header rather than a choice.
    pub heading: bool,
    /// Render as a thin rule, for separating a destructive choice.
    pub divider: bool,
    /// A command rather than a value: it never shows as the field's choice.
    pub action: bool,
    /// Right-aligned secondary content (`text-[10px]`, `--text-muted`).
    pub right: Option<SharedString>,
    pub tone: SelectOptionTone,
    pub icon: Option<SharedString>,
}

impl SelectOption {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
            heading: false,
            divider: false,
            action: false,
            right: None,
            tone: SelectOptionTone::Default,
            icon: None,
        }
    }

    pub fn heading(label: impl Into<SharedString>) -> Self {
        let label = label.into();
        Self {
            value: label.clone(),
            label,
            disabled: false,
            heading: true,
            divider: false,
            action: false,
            right: None,
            tone: SelectOptionTone::Default,
            icon: None,
        }
    }

    pub fn divider() -> Self {
        Self {
            value: SharedString::default(),
            label: SharedString::default(),
            disabled: false,
            heading: false,
            divider: true,
            action: false,
            right: None,
            tone: SelectOptionTone::Default,
            icon: None,
        }
    }

    /// A command row: it runs `on_select` but never reads as the field's value.
    pub fn action(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            action: true,
            ..Self::new(value, label)
        }
    }

    pub fn with_icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn with_right(mut self, right: impl Into<SharedString>) -> Self {
        self.right = Some(right.into());
        self
    }

    pub fn with_tone(mut self, tone: SelectOptionTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}

/// `menuAlign` — which edge of the trigger the menu is pinned to.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SelectMenuAlign {
    Left,
    Right,
}

/// `Select.tsx` + `SelectDropdown.tsx` — the trigger and its menu.
///
/// The trigger is `.ui-input rounded-[4px] relative w-full h-[36px] px-2.5 py-0
/// pr-10 leading-none text-sm inline-flex items-center text-left`; the open
/// state swaps the border to `color-mix(accent, border-subtle 18%)` and adds the
/// `0 0 0 1px color-mix(accent, transparent 72%) inset` ring. The menu is
/// `rounded-[4px] border shadow-xl` on `color-mix(surface-0, black 6%)`.
///
/// React portals the menu to `<body>`; GPUI has no portal, so it renders as an
/// absolute child under the trigger, 6px below it and matching its width. The
/// source clamps the menu to the space below the trigger; with no measure pass
/// here, the list is capped at 320px and scrolls. The chevron flips from
/// `chevron-down` to `chevron-up` in place of `rotate-180`.
///
/// Not ported, because each needs DOM measurement or an overlay the port has no
/// equivalent for: `endAdornment` (`absolute right-8`), the overlay display
/// (`selectedDisplay` / `hideSelectedText` / `selectedTextAlign` /
/// `selectedDisplayOffsetX`), `hideChevron`/`chevronClassName`, and
/// `menuFooterAction`. `menuAlign` is [`select_dropdown_aligned`].
///
/// `on_select` receives the index of the chosen option.
pub fn select_dropdown(
    id: impl Into<SharedString>,
    options: &[SelectOption],
    value: &str,
    open: bool,
    disabled: bool,
    on_toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select: impl Fn(usize, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    select_dropdown_aligned(
        id,
        options,
        value,
        open,
        disabled,
        SelectMenuAlign::Left,
        on_toggle,
        on_select,
    )
}

/// [`select_dropdown`] with the source's `menuAlign`.
///
/// The menu is forced to the trigger's width, so `Left` and `Right` land on the
/// same box unless a caller narrows the trigger; the parameter exists because
/// the source's `right` mode pins the menu's right edge to the trigger's right
/// edge (`rect.right - measuredMenuWidth`), which is only observable once the
/// two widths differ.
pub fn select_dropdown_aligned(
    id: impl Into<SharedString>,
    options: &[SelectOption],
    value: &str,
    open: bool,
    disabled: bool,
    menu_align: SelectMenuAlign,
    on_toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select: impl Fn(usize, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let on_select = Rc::new(on_select);

    let selected = options.iter().find(|option| {
        !option.heading && !option.divider && !option.action && option.value.as_ref() == value
    });
    let selected_label: SharedString = selected
        .map(|option| option.label.clone())
        .unwrap_or_else(|| value.to_string().into());
    let leading = selected.and_then(|option| option.icon.clone());

    let (trigger_border, trigger_bg, trigger_text) = if disabled {
        (
            mix(BORDER_SUBTLE, BLACK, 0.90),
            mix(SURFACE_1, BLACK, 0.92),
            mix(TEXT_MUTED, SURFACE_2, 0.82),
        )
    } else if open {
        (mix(ACCENT, BORDER_SUBTLE, 0.82), SURFACE_0, TEXT_STRONG)
    } else {
        (BORDER_SUBTLE, SURFACE_0, TEXT_STRONG)
    };

    let mut trigger = div()
        .id(ElementId::named_usize(id.clone(), 0))
        .relative()
        .w_full()
        .h(px(36.0))
        // `.ui-input` is unlayered, so its padding/radius/type win over the
        // trigger's `px-2.5 py-0 pr-10 rounded-[4px] leading-none text-sm`.
        .px(px(10.0))
        .py(px(6.0))
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(trigger_border))
        .bg(rgb(trigger_bg))
        .text_size(px(FONT_SM))
        .line_height(px(FONT_SM * 1.3))
        .text_color(rgb(trigger_text))
        .flex()
        .items_center()
        .text_left();

    if open && !disabled {
        // `box-shadow: 0 0 0 1px color-mix(accent, transparent 72%) inset`.
        trigger = trigger.shadow(vec![inset_shadow(
            0.0,
            0.0,
            0.0,
            1.0,
            with_alpha(ACCENT, 0x47),
        )]);
    }

    if let Some(leading) = leading {
        trigger = trigger.child(
            div()
                .mr(px(6.0))
                .flex()
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .w(px(18.0))
                .h(px(18.0))
                .child(icon(leading.as_ref(), 14.0, trigger_text)),
        );
    }

    trigger = trigger.child(
        div()
            .flex_1()
            .min_w(px(0.0))
            .truncate()
            .child(selected_label),
    );

    trigger = trigger.child(
        div()
            .absolute()
            .right(px(12.0))
            .top(px(0.0))
            .bottom(px(0.0))
            .flex()
            .items_center()
            .child(icon(
                if open { "chevron-up" } else { "chevron-down" },
                14.0,
                TEXT_MUTED,
            )),
    );

    if disabled {
        trigger = trigger.opacity(0.55).cursor_not_allowed();
    } else {
        trigger = trigger
            .cursor_pointer()
            .on_click(move |event, window, cx| on_toggle(event, window, cx));
    }

    let mut container = div().id(id.clone()).relative().w_full().flex().flex_col();
    container = container.child(trigger);

    if open && !disabled {
        let mut list = div()
            .id(ElementId::named_usize(id.clone(), 2))
            .max_h(px(320.0))
            .overflow_y_scroll()
            .flex()
            .flex_col();

        for (index, option) in options.iter().enumerate() {
            if option.heading {
                list = list.child(
                    div()
                        .px(px(12.0))
                        .pt(px(8.0))
                        .pb(px(4.0))
                        .text_size(px(10.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(ACCENT))
                        .border_b(px(1.0))
                        .border_color(rgba(with_alpha(BORDER_SUBTLE, 0xa6)))
                        .child(option.label.clone().to_uppercase()),
                );
                continue;
            }
            if option.divider {
                list = list.child(
                    div()
                        .border_t(px(1.0))
                        .border_color(rgb(BORDER_SUBTLE)),
                );
                continue;
            }

            let is_selected = !option.action && option.value.as_ref() == value;
            let tone = option.tone;
            let disabled_row = option.disabled;
            let border = if index == options.len() - 1
                || options
                    .get(index + 1)
                    .is_some_and(|next| next.heading || next.divider)
            {
                // `border-bottom-color: transparent` on the last row of a group.
                0x00000000
            } else {
                with_alpha(BORDER_SUBTLE, 0xd6)
            };

            let mut row = div()
                .id(ElementId::named_usize(id.clone(), index + 3))
                .w_full()
                .flex()
                .items_center()
                .gap(px(8.0))
                .px(px(12.0))
                .py(px(8.0))
                .text_size(px(FONT_MD))
                .line_height(px(20.0))
                .text_left()
                .border_b(px(1.0))
                .border_color(rgba(border));

            if disabled_row {
                // The disabled branch of the source's style object only sets
                // opacity, the cursor and an opaque `--border-subtle` rule; the
                // text colour is inherited (`--foreground`).
                row = row
                    .opacity(0.55)
                    .cursor_not_allowed()
                    .text_color(rgb(FOREGROUND))
                    .border_color(rgb(BORDER_SUBTLE));
            } else if is_selected {
                // `isSelected` is tested before `isHovered`, so a selected row
                // keeps its accent fill under the pointer.
                let callback = on_select.clone();
                row = row
                    .bg(rgb(mix(ACCENT, SURFACE_0, 0.14)))
                    .text_color(rgb(TEXT_STRONG))
                    .cursor_pointer()
                    .on_click(move |_, window, cx| callback(index, window, cx));
            } else {
                let foreground = match tone {
                    SelectOptionTone::Danger => DANGER,
                    SelectOptionTone::Accent => ACCENT_SECONDARY,
                    SelectOptionTone::Default => TEXT_MUTED,
                };
                let hover_background = match tone {
                    SelectOptionTone::Danger => rgba(with_alpha(DANGER, 0x1f)),
                    SelectOptionTone::Accent => rgb(mix(ACCENT_SECONDARY, SURFACE_0, 0.08)),
                    SelectOptionTone::Default => rgba(with_alpha(SURFACE_2, 0xd1)),
                };
                let hover_foreground = match tone {
                    SelectOptionTone::Danger => DANGER,
                    SelectOptionTone::Accent => ACCENT_SECONDARY,
                    SelectOptionTone::Default => TEXT_STRONG,
                };
                let callback = on_select.clone();
                row = row
                    .text_color(rgb(foreground))
                    .cursor_pointer()
                    .hover(move |style| {
                        style.bg(hover_background).text_color(rgb(hover_foreground))
                    })
                    .on_click(move |_, window, cx| callback(index, window, cx));
            }

            if let Some(name) = &option.icon {
                let icon_color = match tone {
                    SelectOptionTone::Danger => DANGER,
                    SelectOptionTone::Accent => ACCENT_SECONDARY,
                    SelectOptionTone::Default => {
                        if is_selected {
                            ACCENT
                        } else {
                            TEXT_MUTED
                        }
                    }
                };
                row = row.child(
                    div()
                        .flex()
                        .flex_shrink_0()
                        .items_center()
                        .justify_center()
                        .w(px(14.0))
                        .h(px(14.0))
                        .child(icon(name.as_ref(), 14.0, icon_color)),
                );
            }

            row = row.child(
                div()
                    .min_w(px(0.0))
                    .truncate()
                    .text_size(px(13.3))
                    .child(option.label.clone()),
            );

            if let Some(right) = &option.right {
                // `ml-auto inline-flex shrink-0 items-center justify-end
                // text-[10px]`, in `--text-muted`.
                row = row.child(
                    div()
                        .ml_auto()
                        .flex()
                        .flex_shrink_0()
                        .items_center()
                        .justify_end()
                        .text_size(px(10.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child(right.clone()),
                );
            }

            list = list.child(row);
        }

        let mut menu = div()
            .id(ElementId::named_usize(id.clone(), 1))
            .absolute()
            .top(relative(1.0))
            .mt(px(6.0))
            .w_full()
            .rounded(px(ROUNDED))
            .border_1()
            .border_color(rgb(mix(ACCENT, BORDER_SUBTLE, 0.18)))
            .bg(rgb(mix(SURFACE_0, BLACK, 0.94)))
            .flex()
            .flex_col()
            .shadow(vec![
                shadow(0.0, 14.0, 34.0, 0.0, with_alpha(BLACK, 0x75)),
                shadow(0.0, 4.0, 12.0, 0.0, with_alpha(BLACK, 0x52)),
            ]);
        // The source has no clipping on the menu box itself; the rounded corners
        // come from the fill and the border. `menuAlign` pins the matching edge.
        menu = match menu_align {
            SelectMenuAlign::Left => menu.left(px(0.0)),
            SelectMenuAlign::Right => menu.right(px(0.0)),
        };
        container = container.child(menu.child(list));
    }

    container
}

// ── Input ───────────────────────────────────────────────────────────────────

/// `Input.tsx` / `.ui-input`: `border border-subtle bg-surface-0
/// text-text-strong rounded-sm px-2.5 py-1.5 text-sm leading-[1.3]`, with
/// `border-color: var(--accent)` on focus.
///
/// The caller puts its own text element (or a `TextInput`) inside the box.
pub fn input_box(id: impl Into<ElementId>, focused: bool) -> Stateful<Div> {
    div()
        .id(id)
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(if focused { ACCENT } else { BORDER_SUBTLE }))
        .bg(rgb(SURFACE_0))
        .text_color(rgb(TEXT_STRONG))
        .px(px(10.0))
        .py(px(6.0))
        .text_size(px(FONT_SM))
        .line_height(px(FONT_SM * 1.3))
        .flex()
        .items_center()
}

/// [`input_box`] with a single line of text already inside it.
pub fn input_field(
    id: impl Into<ElementId>,
    value: impl Into<SharedString>,
    focused: bool,
) -> Stateful<Div> {
    input_box(id, focused).child(value.into())
}

// ── Spinner ─────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SpinnerSize {
    Xs,
    Sm,
    Md,
    Lg,
    Xl,
}

/// `Spinner.tsx`: the `Loader2` glyph at `h-3.5 w-3.5` (sm).
///
/// The source turns the glyph with SMIL; GPUI has no animation, so the arc is
/// rendered statically. `currentColor` does not reach an `svg()` element in
/// GPUI, so the colour is explicit.
pub fn spinner(size: f32) -> Div {
    spinner_colored(size, TEXT_MUTED)
}

/// [`spinner`] at one of the source's size classes.
pub fn spinner_sized(size: SpinnerSize) -> Div {
    spinner(match size {
        SpinnerSize::Xs => 12.0,
        SpinnerSize::Sm => 14.0,
        SpinnerSize::Md => 16.0,
        SpinnerSize::Lg => 20.0,
        SpinnerSize::Xl => 24.0,
    })
}

/// [`spinner`] with an explicit glyph colour.
pub fn spinner_colored(size: f32, color: u32) -> Div {
    div()
        .flex_shrink_0()
        .w(px(size))
        .h(px(size))
        .child(icon("loader-circle", size, color))
}

// ── Toast ───────────────────────────────────────────────────────────────────


#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastShape {
    Pill,
    Rounded,
}

/// `Toast.tsx`: `border px-4 py-2 text-sm font-semibold shadow-lg`, `rounded-full`
/// (pill) or `rounded-lg` (rounded), tinted per tone.
///
/// GPUI has no transitions or keyframes, so the source's `animated` fade-and-rise
/// and `pulse` are not rendered; the toast is drawn in its final state.
pub fn toast(tone: ToastTone, shape: ToastShape, content: impl IntoElement) -> Div {
    let (border, background) = match tone {
        ToastTone::Neutral => (BORDER_SUBTLE, SURFACE_0),
        ToastTone::Info => (
            mix(0x60a5fa, BORDER_SUBTLE, 0.5),
            mix(0x60a5fa, SURFACE_0, 0.10),
        ),
        ToastTone::Success => (
            mix(0x22c55e, BORDER_SUBTLE, 0.5),
            mix(0x22c55e, SURFACE_0, 0.10),
        ),
        ToastTone::Warning => (
            mix(0xf59e0b, BORDER_SUBTLE, 0.5),
            mix(0xf59e0b, SURFACE_0, 0.10),
        ),
        ToastTone::Error => (
            mix(0xef4444, BORDER_SUBTLE, 0.5),
            mix(0xef4444, SURFACE_0, 0.10),
        ),
    };
    div()
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .px(px(16.0))
        .py(px(8.0))
        .text_size(px(FONT_MD))
        .line_height(px(20.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT_STRONG))
        .shadow_lg()
        .rounded(px(if shape == ToastShape::Pill {
            9999.0
        } else {
            ROUNDED_LG
        }))
        .child(content)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastViewportPosition {
    BottomCenter,
    TopCenter,
}

/// `ToastViewport`: `pointer-events-none fixed inset-x-0 flex justify-center
/// px-3`, pinned to the bottom (`1.25rem`) or top (`1rem`) by the caller's
/// `offset`.
pub fn toast_viewport(
    position: ToastViewportPosition,
    offset: f32,
    content: impl IntoElement,
) -> Div {
    let mut viewport = div()
        .absolute()
        .left(px(0.0))
        .right(px(0.0))
        .flex()
        .justify_center()
        .px(px(12.0));
    viewport = match position {
        ToastViewportPosition::BottomCenter => viewport.bottom(px(offset)),
        ToastViewportPosition::TopCenter => viewport.top(px(offset)),
    };
    viewport.child(content)
}

/// [`toast_viewport`] with the source's defaults: `1.25rem` (20px) from the
/// bottom, `1rem` (16px) from the top.
pub fn toast_viewport_default(
    position: ToastViewportPosition,
    content: impl IntoElement,
) -> Div {
    let offset = match position {
        ToastViewportPosition::BottomCenter => 20.0,
        ToastViewportPosition::TopCenter => 16.0,
    };
    toast_viewport(position, offset, content)
}

// ── Tooltip ─────────────────────────────────────────────────────────────────

/// `Tooltip.tsx` with its defaults (`offsetY` 28, `maxWidth` 260).
pub fn tooltip(
    id: impl Into<ElementId>,
    trigger: impl IntoElement,
    content: impl IntoElement,
) -> Stateful<Div> {
    tooltip_with(id, trigger, content, 28.0, 260.0)
}

/// Which side a trigger-anchored popover takes (`Tooltip.tsx`'s `placement`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TooltipPlacement {
    Below,
    Right,
}

/// `Tooltip.tsx` — trigger-anchored popover.
///
/// The popover is `rounded px-2 py-1.5 text-[11px] leading-tight font-medium
/// shadow-lg` on `rgba(24, 24, 24, 0.98)`, `1px solid var(--accent)`. React
/// portals it to `<body>`; with no portal, GPUI renders it as an absolute child
/// of the wrapper and reveals it with the wrapper's `group_hover` state (the
/// source's hover/focus reveal). The source's viewport clamp and cursor-following
/// are not representable in a static tree, so the box hangs under the trigger and
/// is centred.
pub fn tooltip_with(
    id: impl Into<ElementId>,
    trigger: impl IntoElement,
    content: impl IntoElement,
    offset_y: f32,
    max_width: f32,
) -> Stateful<Div> {
    tooltip_placed(
        id,
        trigger,
        content,
        offset_y,
        max_width,
        TooltipPlacement::Below,
    )
}

/// [`tooltip_with`] with the source's `placement`: `Below` centres the box under
/// the trigger, `Right` puts it beside the trigger, centred on it (the column
/// layout the rail uses). The source's flip to the opposite side when the box
/// would leave the viewport needs a measure pass and is not reproduced.
pub fn tooltip_placed(
    id: impl Into<ElementId>,
    trigger: impl IntoElement,
    content: impl IntoElement,
    offset_y: f32,
    max_width: f32,
    placement: TooltipPlacement,
) -> Stateful<Div> {
    let id: ElementId = id.into();
    let group: SharedString = id.to_string().into();
    let mut popover = div()
        .absolute()
        .flex()
        .opacity(0.0)
        .group_hover(group.clone(), |style| style.opacity(1.0));
    popover = match placement {
        TooltipPlacement::Below => popover
            .left(px(0.0))
            .right(px(0.0))
            .top(relative(1.0))
            .mt(px(offset_y))
            .justify_center(),
        TooltipPlacement::Right => popover
            .left(relative(1.0))
            .top(px(0.0))
            .bottom(px(0.0))
            .ml(px(offset_y))
            .items_center(),
    };
    div()
        .id(id)
        .relative()
        .flex()
        .group(group)
        .child(trigger)
        .child(popover.child(
            div()
                .rounded(px(ROUNDED))
                .px(px(8.0))
                .py(px(6.0))
                .text_size(px(11.0))
                .line_height(px(11.0 * LEADING_TIGHT))
                .font_weight(FontWeight::MEDIUM)
                .bg(rgba(with_alpha(0x181818, 0xfa)))
                .text_color(rgb(TEXT_STRONG))
                .border_1()
                .border_color(rgb(ACCENT))
                .max_w(px(max_width))
                .whitespace_normal()
                .text_left()
                .shadow(vec![
                    shadow(0.0, 6.0, 32.0, 0.0, with_alpha(BLACK, 0x70)),
                    shadow(0.0, 1.5, 8.0, 0.0, with_alpha(BLACK, 0x47)),
                ])
                .child(content),
        ))
}

// ── ContextMenu ─────────────────────────────────────────────────────────────

/// One right-click / dropdown menu entry (`ContextMenu.tsx`).
pub struct ContextMenuEntry {
    pub id: SharedString,
    pub label: SharedString,
    pub icon: Option<SharedString>,
    /// Draw the icon without the framed slot (`iconBare`). A colour chip already
    /// carries its own edge; framing it again reads as a box inside a box.
    pub icon_bare: bool,
    /// Renders a tick in the icon slot — for toggles.
    pub checked: bool,
    pub disabled: bool,
    /// Destructive action: row text and its hover fill use the danger colour.
    pub danger: bool,
    /// Rule above this entry, breaking the list into groups.
    pub starts_group: bool,
    /// Right-aligned secondary text.
    pub trailing: Option<SharedString>,
}

impl ContextMenuEntry {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            icon_bare: false,
            checked: false,
            disabled: false,
            danger: false,
            starts_group: false,
            trailing: None,
        }
    }

    pub fn with_icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn bare_icon(mut self) -> Self {
        self.icon_bare = true;
        self
    }

    pub fn checked(mut self) -> Self {
        self.checked = true;
        self
    }

    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    pub fn starts_group(mut self) -> Self {
        self.starts_group = true;
        self
    }

    pub fn with_trailing(mut self, trailing: impl Into<SharedString>) -> Self {
        self.trailing = Some(trailing.into());
        self
    }
}

/// `ContextMenu.tsx` — the menu surface, heading, grouping rules and rows.
///
/// Surface: `rounded-lg border p-1.5 shadow-xl` on
/// `color-mix(surface-0, black 10%)` with `border-subtle`. Rows:
/// `flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px]
/// font-medium`, a 20px framed icon slot, `check` for checked rows, and the
/// danger/accent hover fills from the source.
///
/// React portals the menu to the viewport position of the gesture; GPUI renders
/// it wherever the caller places it. Nested flyouts (`entry.children`) are not
/// rendered — the port has no submenu state.
pub fn context_menu(
    entries: &[ContextMenuEntry],
    title: Option<SharedString>,
    on_select: impl Fn(&str, &mut Window, &mut App) + 'static,
) -> Div {
    // `widthClassName` defaults to `w-48`.
    context_menu_sized(entries, title, 192.0, on_select)
}

/// [`context_menu`] with the source's `widthClassName` resolved to pixels.
pub fn context_menu_sized(
    entries: &[ContextMenuEntry],
    title: Option<SharedString>,
    width: f32,
    on_select: impl Fn(&str, &mut Window, &mut App) + 'static,
) -> Div {
    let on_select = Rc::new(on_select);
    let base = SharedString::from("context-menu");

    let mut menu = div()
        .w(px(width))
        .rounded(px(ROUNDED_LG))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(mix(SURFACE_0, BLACK, 0.90)))
        .p(px(6.0))
        .shadow_xl()
        .flex()
        .flex_col();

    if let Some(title) = title {
        menu = menu.child(
            div()
                .mb(px(4.0))
                .px(px(8.0))
                .py(px(4.0))
                .text_size(px(10.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .child(title.to_uppercase()),
        );
    }

    let mut rows = div().flex().flex_col().gap(px(2.0));

    for (index, entry) in entries.iter().enumerate() {
        if entry.starts_group && index > 0 {
            rows = rows.child(
                div()
                    .my(px(4.0))
                    .h(px(1.0))
                    .w_full()
                    .bg(rgb(BORDER_SUBTLE)),
            );
        }

        let tone = if entry.disabled {
            TEXT_MUTED
        } else if entry.danger {
            DANGER
        } else {
            TEXT_STRONG
        };
        let hover_fill = if entry.danger {
            mix(DANGER, SURFACE_1, 0.10)
        } else {
            mix(ACCENT, SURFACE_1, 0.18)
        };

        let mut row = div()
            .id(ElementId::named_usize(base.clone(), index))
            .flex()
            .items_center()
            .gap(px(8.0))
            .rounded(px(ROUNDED_MD))
            .px(px(8.0))
            .py(px(6.0))
            .text_size(px(13.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(tone));

        // The 20px icon slot: `rounded border` on `surface-1`, a tick when
        // checked. `iconBare` drops the frame and the fill.
        let bare = entry.icon_bare && !entry.checked;
        let mut slot = div()
            .flex()
            .items_center()
            .justify_center()
            .w(px(20.0))
            .h(px(20.0))
            .opacity(if entry.disabled { 0.8 } else { 1.0 });
        if !bare {
            slot = slot
                .rounded(px(ROUNDED))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_1));
        }
        if entry.checked {
            let check_color = if entry.disabled { TEXT_MUTED } else { ACCENT };
            slot = slot.child(icon("check", 14.0, check_color));
        } else if let Some(name) = &entry.icon {
            slot = slot.child(icon(name.as_ref(), 14.0, tone));
        }
        row = row.child(slot);

        // `flex-1` only when there is trailing content; the label itself is not
        // truncated by the source.
        let mut label = div().child(entry.label.clone());
        if entry.trailing.is_some() {
            label = label.flex_1();
        }
        row = row.child(label);

        if let Some(trailing) = &entry.trailing {
            row = row.child(div().child(trailing.clone()));
        }

        if entry.disabled {
            row = row.opacity(0.55).cursor_not_allowed();
        } else {
            let callback = on_select.clone();
            let entry_id = entry.id.clone();
            row = row
                .cursor_pointer()
                .hover(move |style| style.bg(rgb(hover_fill)))
                .on_click(move |_, window, cx| {
                    callback(entry_id.as_ref(), window, cx);
                });
        }

        rows = rows.child(row);
    }

    menu.child(rows)
}

/// A row in a `ContextMenu`, without the icon slot.
///
/// Kept from the first slice: `MenuRow`'s geometry and tones, for a caller that
/// only has a label.
pub fn menu_item(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    disabled: bool,
) -> Stateful<Div> {
    let label: SharedString = label.into();
    let mut el = div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(8.0))
        .rounded(px(ROUNDED_MD))
        .px(px(8.0))
        .py(px(6.0))
        .text_size(px(13.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(if disabled { TEXT_MUTED } else { TEXT_STRONG }))
        .child(label);
    if disabled {
        el = el.opacity(0.55).cursor_not_allowed();
    } else {
        el = el
            .cursor_pointer()
            .hover(|style| style.bg(rgb(mix(ACCENT, SURFACE_1, 0.18))));
    }
    el
}

// ── Icon tones ──────────────────────────────────────────────────────────────

/// `iconTone.ts`'s `IconTone`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IconTone {
    Accent,
    Warning,
    Danger,
    Neutral,
}

/// `(border, background, foreground)` from `ICON_TONE_STYLES`.
fn icon_tone_colors(tone: IconTone) -> (u32, u32, u32) {
    match tone {
        IconTone::Warning => (
            mix(0xd97706, BORDER_SUBTLE, 0.5),
            mix(0xd97706, SURFACE_1, 0.15),
            0xd97706,
        ),
        IconTone::Danger => (
            mix(0xef4444, BORDER_SUBTLE, 0.45),
            mix(0xef4444, SURFACE_1, 0.12),
            DANGER,
        ),
        IconTone::Accent => (
            mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.55),
            mix(ACCENT_SECONDARY, SURFACE_1, 0.10),
            ACCENT_SECONDARY,
        ),
        IconTone::Neutral => (BORDER_SUBTLE, SURFACE_1, TEXT_MUTED),
    }
}

// ── IconChip ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IconChipSize {
    Xs,
    Sm,
    Md,
    Lg,
}

/// `IconChip.tsx`: the bordered square an icon lives in.
///
/// `xs` is the 16px badge (`rounded-[3px] text-[10px] font-bold`), `sm` the 20px
/// menu-row slot (`rounded`), `md` the 32px toolbar tile (`rounded-md`), `lg` the
/// 36px modal-header tile (`rounded-md`). `tone: Neutral` is the source's
/// `inherit` chip fill (`border-subtle` / `surface-1`); GPUI has no
/// `currentColor`, so the icon colour is explicit.
pub fn icon_chip(size: IconChipSize, tone: IconTone, icon_name: &str) -> Div {
    let icon_size = match size {
        // `iconSizeClassMap.xs` is empty, so the source leaves the glyph at
        // lucide's 24px default inside a 16px box. The port uses the box's own
        // 16px (`h-4 w-4`), which is what every `xs` call site passes as a
        // child instead of using the `icon` prop.
        IconChipSize::Xs => 16.0,
        IconChipSize::Sm => 14.0,
        IconChipSize::Md => 16.0,
        IconChipSize::Lg => 16.0,
    };
    let (_, _, foreground) = icon_tone_colors(tone);
    icon_chip_with(size, tone, icon(icon_name, icon_size, foreground))
}

/// [`icon_chip`] with arbitrary content (a tick, a number, a glyph) in the slot.
pub fn icon_chip_with(size: IconChipSize, tone: IconTone, content: impl IntoElement) -> Div {
    // Only `xs` carries a type size (`text-[10px] font-bold leading-none`);
    // `sm`/`md`/`lg` have none, so their content inherits the surrounding type.
    let (width, height, radius, font_size) = match size {
        IconChipSize::Xs => (16.0, 16.0, 3.0, Some(10.0)),
        IconChipSize::Sm => (20.0, 20.0, ROUNDED, None),
        IconChipSize::Md => (32.0, 32.0, ROUNDED_MD, None),
        IconChipSize::Lg => (36.0, 36.0, ROUNDED_MD, None),
    };
    let (border, background, foreground) = icon_tone_colors(tone);
    let mut chip = div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .rounded(px(radius))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .w(px(width))
        .h(px(height))
        .text_color(rgb(foreground))
        .child(content);
    if let Some(font_size) = font_size {
        chip = chip.text_size(px(font_size));
    }
    if size == IconChipSize::Xs {
        chip = chip.font_weight(FontWeight::BOLD).line_height(px(10.0));
    }
    chip
}

// ── FadeText ────────────────────────────────────────────────────────────────

/// `FadeText.tsx`: one line that clips its overflow.
///
/// The source's inline style is `display: block; min-width: 0; overflow: hidden;
/// white-space: nowrap; text-overflow: clip`, and it adds a `mask-image` fade of
/// the last 18px only once a measurement says the text overflows. GPUI has no
/// mask and no measure pass here, so the line clips with no ellipsis and no soft
/// edge — the same single-line, hard-clipped result for the overflowing case.
pub fn fade_text(text: impl Into<SharedString>) -> Div {
    div()
        .block()
        .min_w(px(0.0))
        .w_full()
        .overflow_hidden()
        .whitespace_nowrap()
        .child(text.into())
}

// ── ProgressBar ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProgressSize {
    Xs,
    Sm,
    Md,
}

/// `ProgressBar.tsx`.
///
/// Track: `w-full overflow-hidden rounded-full` at `h-1.5` / `h-2` / `h-2.5`,
/// filled with `color-mix(surface-2, black 20%)`. Fill:
/// `linear-gradient(90deg, var(--accent), #ff79c6)`. Indeterminate renders the
/// 32% sweeping indicator statically (GPUI has no keyframes).
pub fn progress_bar(value: f32, size: ProgressSize, indeterminate: bool) -> Div {
    let height = match size {
        ProgressSize::Xs => 6.0,
        ProgressSize::Sm => 8.0,
        ProgressSize::Md => 10.0,
    };
    let percent = value.clamp(0.0, 100.0);
    let fill = || {
        linear_gradient(
            90.0,
            linear_color_stop(rgb(ACCENT), 0.0),
            linear_color_stop(rgb(0xff79c6), 1.0),
        )
    };
    let mut track = div()
        .relative()
        .w_full()
        .h(px(height))
        .overflow_hidden()
        .rounded_full()
        .bg(rgb(mix(SURFACE_2, BLACK, 0.80)));
    if indeterminate {
        track = track.child(
            div()
                .absolute()
                .left(px(0.0))
                .top(px(0.0))
                .bottom(px(0.0))
                .w(relative(0.32))
                .rounded_full()
                .bg(fill()),
        );
    } else {
        track = track.child(
            div()
                .h_full()
                .rounded_full()
                .w(relative(percent / 100.0))
                .bg(fill()),
        );
    }
    track
}

// ── SettingRow ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingDensity {
    Compact,
    Comfortable,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingSurface {
    Sunken,
    Raised,
}

/// `SettingRow.tsx`: label and description on the left, control on the right,
/// optionally in the inset card the settings tabs use.
///
/// `flex items-center justify-between gap-3`; bordered adds `rounded-md border`
/// with `p-1.5` (compact) or `p-2` (comfortable), `surface-0` (sunken) or
/// `surface-1` (raised) and a `pl-1` on the text block. The label is
/// `text-xs font-semibold`; the description `text-[11px] leading-tight`
/// (compact) or `text-xs` (comfortable), in `text-muted`.
pub fn setting_row(
    label: impl Into<SharedString>,
    description: Option<SharedString>,
    density: SettingDensity,
    bordered: bool,
    surface: SettingSurface,
    disabled: bool,
    control: impl IntoElement,
) -> Div {
    let compact = density == SettingDensity::Compact;
    let mut row = div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.0));
    if bordered {
        row = row
            .rounded(px(ROUNDED_MD))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(match surface {
                SettingSurface::Sunken => SURFACE_0,
                SettingSurface::Raised => SURFACE_1,
            }))
            .p(px(if compact { 6.0 } else { 8.0 }));
    }
    if disabled {
        row = row.opacity(0.68);
    }

    let mut text = div().min_w(px(0.0)).child(
        div()
            .text_size(px(FONT_XS))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(TEXT_STRONG))
            // `text-xs` carries a 16px line-height; `leading-tight` (compact)
            // replaces it with 1.25.
            .line_height(px(if compact {
                FONT_XS * LEADING_TIGHT
            } else {
                16.0
            }))
            .child(label.into()),
    );
    if let Some(description) = description {
        text = text.child(
            div()
                .text_size(px(if compact { 11.0 } else { FONT_XS }))
                .text_color(rgb(TEXT_MUTED))
                .line_height(px(if compact {
                    11.0 * LEADING_TIGHT
                } else {
                    16.0
                }))
                .child(description),
        );
    }

    row = row.child(if bordered { text.pl(px(4.0)) } else { text });
    row.child(div().flex().items_center().gap(px(8.0)).child(control))
}

// ── PanelCollapseToggle ─────────────────────────────────────────────────────

/// `PanelCollapseToggle.tsx`: the chevron that expands and collapses a panel.
///
/// The default `IconButton` tile (`.ui-button ui-button-secondary !p-2`) around a
/// 12px chevron — 30x30 including the border — with the chevron in `--accent`
/// when expanded and `--text-muted` when collapsed.
pub fn panel_collapse_toggle(
    id: impl Into<ElementId>,
    expanded: bool,
    on_toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let (name, color) = if expanded {
        ("chevron-down", ACCENT)
    } else {
        ("chevron-right", TEXT_MUTED)
    };
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .p(px(8.0))
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(SURFACE_2)))
        .on_click(on_toggle)
        .child(icon(name, 12.0, color))
}

// ── StructuredDialogModal ───────────────────────────────────────────────────

/// The backdrop of `StructuredDialogModal`: `fixed inset-0 flex items-center
/// justify-center bg-black/55 px-3`. GPUI has no `backdrop-blur`, so the blur is
/// omitted.
fn modal_backdrop() -> Div {
    div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .px(px(12.0))
        .bg(rgba(with_alpha(BLACK, 0x8c)))
}

/// The panel of `StructuredDialogModal`: `w-full overflow-hidden rounded-xl
/// border shadow-2xl` on `surface-0` with `border-subtle` and
/// `0 24px 46px rgba(0,0,0,0.42)`.
fn modal_panel(width: f32) -> Div {
    div()
        .w_full()
        .max_w(px(width))
        .rounded(px(12.0))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .overflow_hidden()
        .flex()
        .flex_col()
        .shadow(vec![shadow(0.0, 24.0, 46.0, 0.0, with_alpha(BLACK, 0x6b))])
}

/// The backdrop and card of `StructuredDialogModal`.
pub fn modal_shell(width: f32) -> Div {
    modal_backdrop().child(modal_panel(width))
}

/// `StructuredDialogModal.tsx` in full: backdrop, panel, header (toned icon tile,
/// title, subtitle, close button), body and action row.
///
/// Header: `flex items-center justify-between gap-4 border-b px-5 py-4`; icon
/// tile `h-9 w-9 rounded-md border` with the tone's `ICON_TONE_STYLES`; title
/// `text-base font-semibold leading-tight`; subtitle `mt-0.5 text-[11px]
/// leading-snug`. Body: `space-y-4 p-5`. Actions: `grid grid-flow-col
/// auto-cols-fr gap-2 pt-1` — a row of equal columns, so each action should be
/// `flex_1`.
pub fn dialog_modal(
    width: f32,
    title: impl Into<SharedString>,
    subtitle: Option<SharedString>,
    icon_name: Option<&str>,
    tone: IconTone,
    body: impl IntoElement,
    actions: impl IntoElement,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    dialog_modal_full(
        width, title, subtitle, icon_name, tone, false, body, actions, on_close,
    )
}

/// [`dialog_modal`] with the source's `closeDisabled`: the close tile takes the
/// `.ui-button:disabled` recipe and drops its click handler.
#[allow(clippy::too_many_arguments)]
pub fn dialog_modal_full(
    width: f32,
    title: impl Into<SharedString>,
    subtitle: Option<SharedString>,
    icon_name: Option<&str>,
    tone: IconTone,
    close_disabled: bool,
    body: impl IntoElement,
    actions: impl IntoElement,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let title: SharedString = title.into();
    let (tone_border, tone_bg, tone_fg) = icon_tone_colors(tone);

    let mut left = div().flex().items_center().gap(px(12.0)).min_w(px(0.0));
    if let Some(name) = icon_name {
        left = left.child(
            div()
                .flex()
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .w(px(36.0))
                .h(px(36.0))
                .rounded(px(ROUNDED_MD))
                .border_1()
                .border_color(rgb(tone_border))
                .bg(rgb(tone_bg))
                .child(icon(name, 16.0, tone_fg)),
        );
    }

    let mut titles = div().min_w(px(0.0)).pr(px(8.0)).child(
        div()
            .text_size(px(16.0))
            .font_weight(FontWeight::SEMIBOLD)
            .line_height(px(16.0 * LEADING_TIGHT))
            .text_color(rgb(TEXT_STRONG))
            .child(title.clone()),
    );
    if let Some(subtitle) = subtitle {
        titles = titles.child(
            div()
                .mt(px(2.0))
                .text_size(px(11.0))
                .line_height(px(11.0 * LEADING_SNUG))
                .text_color(rgb(TEXT_MUTED))
                .child(subtitle),
        );
    }
    left = left.child(titles);

    let header = div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.0))
        .border_b(px(1.0))
        .border_color(rgb(BORDER_SUBTLE))
        .px(px(20.0))
        .py(px(16.0))
        .child(left)
        .child(if close_disabled {
            div()
                .flex()
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .w(px(36.0))
                .h(px(36.0))
                .rounded(px(ROUNDED_MD))
                .border_1()
                .border_color(rgb(mix(BORDER_SUBTLE, BLACK, 0.90)))
                .bg(rgb(mix(SURFACE_1, BLACK, 0.92)))
                .cursor_not_allowed()
                .child(icon("x", 16.0, mix(TEXT_MUTED, SURFACE_2, 0.82)))
                .into_any_element()
        } else {
            icon_button(
                ElementId::named_usize(title, 0),
                "x",
                IconButtonVariant::Surface,
                IconButtonSize::Md,
            )
            .on_click(on_close)
            .into_any_element()
        });

    let content = div()
        .p(px(20.0))
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(body)
        .child(div().flex().gap(px(8.0)).pt(px(4.0)).child(actions));

    modal_backdrop().child(
        modal_panel(width)
            .child(header)
            .child(content),
    )
}

// ── ColorSwatchInput ────────────────────────────────────────────────────────

/// Parse a CSS `#rrggbb` / `#rgb` / `rrggbb` string into `0xRRGGBB`.
///
/// `ColorSwatchInput`'s `value` is a CSS colour string; GPUI needs the packed
/// integer. Anything unparsable falls back to transparent so a bad value reads
/// as an empty swatch rather than an arbitrary colour.
pub fn parse_hex_color(value: &str) -> u32 {
    let hex = value.trim().trim_start_matches('#');
    match hex.len() {
        3 => {
            let expand = |c: char| u8::from_str_radix(&format!("{c}{c}"), 16).ok();
            let mut chars = hex.chars();
            match (chars.next(), chars.next(), chars.next()) {
                (Some(r), Some(g), Some(b)) => match (expand(r), expand(g), expand(b)) {
                    (Some(r), Some(g), Some(b)) => {
                        (r as u32) << 16 | (g as u32) << 8 | b as u32
                    }
                    _ => 0,
                },
                _ => 0,
            }
        }
        6 => u32::from_str_radix(hex, 16).unwrap_or(0),
        _ => 0,
    }
}

/// `ColorSwatchInput.tsx` — the colour picker's swatch, filling its box.
///
/// `shrink-0 cursor-pointer appearance-none overflow-hidden rounded border p-0`
/// with `border-color: var(--border-subtle)` and
/// `box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.28)`. The hairline keeps every
/// colour the same apparent size, including a pale resin on a pale surface.
///
/// The size comes from the caller, as at the source (the settings rows and the
/// overlay panels use different boxes). GPUI has no native colour picker, so the
/// swatch is a filled box; `on_click` is the caller's hook for opening one.
pub fn color_swatch_input(
    id: impl Into<ElementId>,
    width: f32,
    height: f32,
    value: &str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    color_swatch(width, height, value)
        .id(id)
        .cursor_pointer()
        .on_click(on_click)
}

/// [`color_swatch_input`] without the click hook.
pub fn color_swatch(width: f32, height: f32, value: &str) -> Div {
    div()
        .flex_shrink_0()
        .w(px(width))
        .h(px(height))
        .rounded(px(ROUNDED))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(parse_hex_color(value)))
        .overflow_hidden()
        .shadow(vec![inset_shadow(
            0.0,
            0.0,
            0.0,
            1.0,
            with_alpha(BLACK, 0x47),
        )])
}

// ── NumberInput ─────────────────────────────────────────────────────────────

/// The stepper overlay of `NumberInput.tsx`:
/// `absolute inset-y-0 right-0.5 flex w-4 flex-col items-center justify-center
/// gap-0.5`, two `h-3 w-3` buttons with `rounded hover:bg-white/10
/// disabled:opacity-50` and a 10px (`h-2.5 w-2.5`) chevron each.
///
/// GPUI has no text editing on a `div`, so the port renders the field's frame
/// and steppers; the caller owns the text and the commit.
fn number_stepper(
    id: &SharedString,
    can_increment: bool,
    can_decrement: bool,
    on_up: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_down: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let button = |index: usize,
                  name: &'static str,
                  enabled: bool,
                  callback: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>| {
        let mut el = div()
            .id(ElementId::named_usize(id.clone(), index))
            .flex()
            .items_center()
            .justify_center()
            .w(px(12.0))
            .h(px(12.0))
            .rounded(px(ROUNDED))
            .child(icon(name, 10.0, TEXT_STRONG));
        if enabled {
            el = el
                .cursor_pointer()
                .hover(|style| style.bg(rgba(with_alpha(WHITE, 0x1a))))
                .on_click(move |event, window, cx| callback(event, window, cx));
        } else {
            // `disabled:opacity-50`.
            el = el.opacity(0.5).cursor_not_allowed();
        }
        el
    };

    div()
        .absolute()
        .right(px(2.0))
        .top(px(0.0))
        .bottom(px(0.0))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(2.0))
        .w(px(16.0))
        .child(button(0, "chevron-up", can_increment, Rc::new(on_up)))
        .child(button(1, "chevron-down", can_decrement, Rc::new(on_down)))
}

/// `NumberInput.tsx` without the stepper (`showStepper={false}`).
///
/// The source renders a bare `<input>` and lets the call site supply the class
/// string, so the port renders the `.ui-input` frame and leaves height, padding
/// and type size to the caller (`number_input(...).h(px(32.0))`).
pub fn number_input(
    id: impl Into<ElementId>,
    display: impl Into<SharedString>,
    focused: bool,
    disabled: bool,
) -> Stateful<Div> {
    let mut el = input_box(id, focused).child(display.into());
    if disabled {
        el = el.opacity(0.55).cursor_not_allowed();
    }
    el
}

/// `NumberInput.tsx` with its default stepper.
///
/// The field is `relative min-w-0` around the input, with the stepper overlay
/// pinned to the right edge. `can_increment` / `can_decrement` are the source's
/// `maxBound` / `minBound` checks (plus `disabled`).
pub fn number_input_with_stepper(
    id: impl Into<SharedString>,
    display: impl Into<SharedString>,
    focused: bool,
    disabled: bool,
    can_increment: bool,
    can_decrement: bool,
    on_up: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_down: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let mut field = input_box(id.clone(), focused).child(display.into());
    if disabled {
        field = field.opacity(0.55).cursor_not_allowed();
    }
    div()
        .id(ElementId::named_usize(id.clone(), 8))
        .relative()
        .min_w(px(0.0))
        .child(field)
        .child(number_stepper(
            &id,
            can_increment && !disabled,
            can_decrement && !disabled,
            on_up,
            on_down,
        ))
}

/// `compactNumberField.tsx` — a labelled number field, two per row.
///
/// Label: `ui-meta block text-center` (13px) in `--text-muted`. Field:
/// `ui-input h-8 w-full min-w-0 pl-1.5 pr-6 text-xs text-center no-spinners
/// !bg-[var(--surface-0)]` — of which only `h-8`, `w-full`, `min-w-0`,
/// `text-center` and the important background survive, because `.ui-input` is
/// unlayered and keeps its 6px/10px padding and 13px type. Unit: `absolute
/// right-2 top-1/2 -translate-y-1/2 text-[10px] font-medium` in `--text-muted`.
/// No stepper (`showStepper={false}`).
pub fn compact_number_field(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    value: impl Into<SharedString>,
    unit: Option<&str>,
    disabled: bool,
) -> Div {
    let id: SharedString = id.into();
    let mut field = input_box(id, false)
        // `h-8` is the only utility that survives here: `.ui-input`'s unlayered
        // padding (6px 10px) and `--font-size-sm` beat `pl-1.5 pr-6 text-xs`.
        .h(px(32.0))
        .w_full()
        .min_w(px(0.0))
        .text_center()
        .justify_center()
        .bg(rgb(SURFACE_0))
        .child(value.into());
    if disabled {
        field = field.opacity(0.55).cursor_not_allowed();
    }

    let mut box_ = div().relative().mt(px(4.0)).child(field);
    if let Some(unit) = unit {
        box_ = box_.child(
            div()
                .absolute()
                .right(px(8.0))
                .top(px(0.0))
                .bottom(px(0.0))
                .flex()
                .items_center()
                .text_size(px(10.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(TEXT_MUTED))
                .child(unit.to_string()),
        );
    }

    div()
        .min_w(px(0.0))
        .child(
            div()
                .block()
                .text_center()
                .text_size(px(FONT_SM))
                .text_color(rgb(TEXT_MUTED))
                .child(label.into()),
        )
        .child(box_)
}

/// `miniStepperField.tsx` — a compact integer field with the small chevrons.
///
/// Field: `ui-input h-8 w-full min-w-0 pl-1.5 pr-5 text-xs text-center
/// no-spinners !bg-[var(--surface-0)]` — the height, width and centring apply;
/// the padding and type size lose to the unlayered `.ui-input`. The stepper is
/// `NumberInput`'s default overlay.
pub fn mini_stepper_field(
    id: impl Into<SharedString>,
    value: impl Into<SharedString>,
    can_increment: bool,
    can_decrement: bool,
    disabled: bool,
    on_up: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_down: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let mut field = input_box(id.clone(), false)
        // `h-8` survives; `.ui-input`'s padding and type beat `pl-1.5 pr-5
        // text-xs`.
        .h(px(32.0))
        .w_full()
        .min_w(px(0.0))
        .text_center()
        .justify_center()
        .bg(rgb(SURFACE_0))
        .child(value.into());
    if disabled {
        field = field.opacity(0.55).cursor_not_allowed();
    }
    div()
        .id(ElementId::named_usize(id.clone(), 8))
        .relative()
        .min_w(px(0.0))
        .child(field)
        .child(number_stepper(
            &id,
            can_increment && !disabled,
            can_decrement && !disabled,
            on_up,
            on_down,
        ))
}

/// `scrollableNumberField.tsx` — `−  value  +` with full-size steppers.
///
/// Root: `flex min-w-0 items-center gap-1` (`gap-0.5` compact). Steppers are
/// `IconButton`s at `!h-8 !w-8 !p-0 !bg-[var(--surface-0)]` with a
/// `hover:!bg-[color-mix(surface-0, white 4%)]` and a
/// `disabled:!bg-[color-mix(surface-1, black 8%)]`; compact shrinks them to
/// `!h-6 !w-6`. Icons are `h-3.5 w-3.5` (compact `h-3 w-3`).
///
/// The field is `ui-input w-full text-center tabular-nums font-semibold
/// no-spinners !bg-[var(--surface-0)]` with `h-7` (compact) or `h-8`, plus
/// `px-1 text-[11px]` / `px-1.5 text-xs sm:text-sm` — the padding and type
/// utilities lose to the unlayered `.ui-input`, so both densities keep its
/// 6px/10px padding and 13px type and differ only in height. GPUI has no
/// `font-variant-numeric`, so `tabular-nums` is not applied.
pub fn scrollable_number_field(
    id: impl Into<SharedString>,
    value: impl Into<SharedString>,
    unit: Option<&str>,
    compact: bool,
    disabled: bool,
    can_increment: bool,
    can_decrement: bool,
    on_decrement: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_increment: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let stepper_id = id.clone();

    let stepper = |index: usize,
                   name: &'static str,
                   size: f32,
                   enabled: bool,
                   callback: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>| {
        // `.ui-button-secondary`'s text colour is `--text-strong`; the global
        // `button:disabled` rule swaps it for `color-mix(text-muted, surface-2
        // 18%)`.
        let active = enabled && !disabled;
        let foreground = if active {
            TEXT_STRONG
        } else {
            mix(TEXT_MUTED, SURFACE_2, 0.82)
        };
        let mut el = div()
            .id(ElementId::named_usize(stepper_id.clone(), index))
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .rounded(px(RADIUS_SM))
            .border_1()
            .bg(rgb(SURFACE_0))
            .w(px(size))
            .h(px(size))
            .child(icon(name, if compact { 12.0 } else { 14.0 }, foreground));
        if active {
            el = el
                .border_color(rgb(BORDER_SUBTLE))
                .cursor_pointer()
                .hover(|style| style.bg(rgb(mix(SURFACE_0, WHITE, 0.96))))
                .on_click(move |event, window, cx| callback(event, window, cx));
        } else {
            // `disabled:!bg-[color-mix(surface-1, black 8%)]`, plus the global
            // `button:disabled` border and text colours.
            el = el
                .border_color(rgb(mix(BORDER_SUBTLE, BLACK, 0.90)))
                .bg(rgb(mix(SURFACE_1, BLACK, 0.92)))
                .text_color(rgb(mix(TEXT_MUTED, SURFACE_2, 0.82)))
                .cursor_not_allowed();
        }
        el
    };

    let stepper_size = if compact { 24.0 } else { 32.0 };

    let mut field = input_box(ElementId::named_usize(id.clone(), 2), false)
        .w_full()
        .h(px(if compact { 28.0 } else { 32.0 }))
        // `px-1`/`px-1.5`, `text-[11px]`, `text-xs` and `sm:text-sm` all lose to
        // the unlayered `.ui-input` padding (6px 10px) and 13px type, so only
        // the height and the weight differ between the two densities.
        .text_center()
        .justify_center()
        .font_weight(FontWeight::SEMIBOLD)
        .bg(rgb(SURFACE_0))
        .child(value.into());
    if disabled {
        field = field.opacity(0.55).cursor_not_allowed();
    }

    let mut middle = div().relative().min_w(px(0.0)).flex_1().child(field);
    if let Some(unit) = unit {
        middle = middle.child(
            div()
                .absolute()
                .right(px(if compact { 4.0 } else { 8.0 }))
                .top(px(0.0))
                .bottom(px(0.0))
                .flex()
                .items_center()
                .text_size(px(if compact { 8.0 } else { 10.0 }))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(TEXT_MUTED))
                .child(unit.to_string()),
        );
    }

    div()
        .id(id)
        .flex()
        .min_w(px(0.0))
        .items_center()
        .gap(px(if compact { 2.0 } else { 4.0 }))
        .child(stepper(
            0,
            "minus",
            stepper_size,
            can_decrement,
            Rc::new(on_decrement),
        ))
        .child(middle)
        .child(stepper(
            1,
            "plus",
            stepper_size,
            can_increment,
            Rc::new(on_increment),
        ))
}

// ── BlockingOverlay ─────────────────────────────────────────────────────────

/// The built-in progress bar of [`blocking_overlay`].
pub enum BlockingProgress {
    /// `progress` omitted — no bar at all.
    Hidden,
    /// `progress === null` — the sweeping indicator.
    Indeterminate,
    /// `progress` 0–100.
    Value(f32),
}

/// `BlockingOverlay.tsx` — the non-dismissible busy sheet.
///
/// Backdrop: `absolute inset-0 flex items-center justify-center bg-black/45
/// backdrop-blur-[1px]` (the blur is omitted). Panel:
/// `w-[min(520px,92vw)] rounded-xl border px-5 py-4 shadow-xl` on
/// `color-mix(surface-0, black 10%)` with `border-subtle`.
///
/// Title `text-sm font-semibold` in `--text-strong`; details
/// `mt-1 space-y-0.5 text-xs leading-relaxed` in `--text-muted`; elapsed
/// `mt-2 text-[11px] font-medium tracking-wide` in `--accent`; footnote
/// `mt-1 text-[11px]` in `--text-muted`; the bar sits in an `mt-3` wrapper.
pub fn blocking_overlay(
    title: impl Into<SharedString>,
    details: &[SharedString],
    elapsed: Option<SharedString>,
    footnote: Option<SharedString>,
    progress: BlockingProgress,
    children: Option<impl IntoElement>,
) -> Div {
    let mut panel = div()
        // `w-[min(520px,92vw)]` — the overlay is the full window, so 92% of it
        // is 92vw.
        .w(relative(0.92))
        .max_w(px(520.0))
        .rounded(px(ROUNDED_XL))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(mix(SURFACE_0, BLACK, 0.90)))
        .px(px(20.0))
        .py(px(16.0))
        .shadow_xl()
        .flex()
        .flex_col()
        .child(
            div()
                .text_size(px(FONT_MD))
                .line_height(px(20.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child(title.into()),
        );

    if !details.is_empty() {
        let mut block = div()
            .mt(px(4.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .text_size(px(FONT_XS))
            .line_height(px(FONT_XS * LEADING_RELAXED))
            .text_color(rgb(TEXT_MUTED));
        for line in details {
            block = block.child(div().child(line.clone()));
        }
        panel = panel.child(block);
    }

    if let Some(elapsed) = elapsed {
        panel = panel.child(
            div()
                .mt(px(8.0))
                .text_size(px(11.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(ACCENT))
                .child(elapsed),
        );
    }

    if let Some(footnote) = footnote {
        panel = panel.child(
            div()
                .mt(px(4.0))
                .text_size(px(11.0))
                .text_color(rgb(TEXT_MUTED))
                .child(footnote),
        );
    }

    match progress {
        BlockingProgress::Hidden => {}
        BlockingProgress::Indeterminate => {
            panel = panel.child(div().mt(px(12.0)).child(progress_bar(
                0.0,
                ProgressSize::Md,
                true,
            )));
        }
        BlockingProgress::Value(value) => {
            panel = panel.child(div().mt(px(12.0)).child(progress_bar(
                value,
                ProgressSize::Md,
                false,
            )));
        }
    }

    if let Some(children) = children {
        panel = panel.child(children);
    }

    div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(rgba(with_alpha(BLACK, 0x73)))
        .child(panel)
}

// ── MouseTooltip ────────────────────────────────────────────────────────────

/// `MouseTooltip.tsx` — a box that follows the pointer at a `(15, 15)` offset.
///
/// The source listens to `window.mousemove`, then measures and clamps the box
/// into the viewport, flipping to the other side of the cursor when it would
/// overflow. GPUI has no global pointer listener in a static tree and no measure
/// pass, so the caller supplies the pointer position (from its own
/// `on_mouse_move`) and the box is drawn at `position + offset`. It carries no
/// hit handler, so it never intercepts input (`pointer-events: none`). The clamp
/// and the flip are not reproduced.
pub fn mouse_tooltip(
    position: Point<Pixels>,
    offset: Point<Pixels>,
    content: impl IntoElement,
) -> Div {
    div()
        .absolute()
        .left(position.x + offset.x)
        .top(position.y + offset.y)
        .child(content)
}

/// [`mouse_tooltip`] with the source's default `offset = { x: 15, y: 15 }`.
pub fn mouse_tooltip_default(
    position: Point<Pixels>,
    content: impl IntoElement,
) -> Div {
    mouse_tooltip(position, point(px(15.0), px(15.0)), content)
}
