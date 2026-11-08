//! `Card` and its header.

use gpui::{Div, FontWeight, IntoElement, SharedString, div, prelude::*, px, rgb};

use declarative_gpui::ui;
use gpui_core::theme::*;

use super::constants::*;

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

/// [`card_header`] with both slots: the leading block (`CardHeader.tsx`'s
/// `left`, which carries an icon and a title) and the absolute right slot.
///
/// `panels/list.rs` rebuilt the whole header locally because the shared one had
/// no leading slot; this is that slot, so a panel header is spelled once.
pub fn card_header_with(
    title: impl Into<SharedString>,
    leading: Option<impl IntoElement>,
    right: Option<impl IntoElement>,
) -> Div {
    let title: SharedString = title.into();
    let mut title_block = div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .pr(px(32.0));
    if let Some(leading) = leading {
        title_block = title_block.child(leading);
    }
    let left = title_block.child(
        div()
            .text_size(px(FONT_MD))
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

// ── Sub-card ────────────────────────────────────────────────────────────────

/// `panelCardStyle`: the `rounded-md border p-2` block a panel groups controls
/// in.
///
/// The port had this shape in four places with radii of 6, 8 and 10 and tint
/// pairs of `0.24/0.05` and `0.22/0.06`; `setting_row`/`section_card` variants
/// all resolve to this one builder.
pub fn sub_card() -> Div {
    sub_card_with(None, 8.0)
}

/// [`sub_card`] with an accent tint — `(tone, background share)` — and padding.
///
/// The border keeps the `accentCardStyle` share (`accent 24%` of
/// `--border-subtle`), which is also the Lift card's. A panel whose own tint
/// pair differs — Move is 22%, Rotate and Scale are 20% — calls
/// [`sub_card_tinted`] with its own border share.
pub fn sub_card_with(tint: Option<(u32, f32)>, padding: f32) -> Div {
    let (border, background) = match tint {
        Some((tone, share)) => (
            mix(tone, BORDER_SUBTLE, TINT_BORDER_SHARE),
            mix(tone, SURFACE_1, share),
        ),
        None => (mix(BORDER_SUBTLE, BLACK, 0.94), SURFACE_1),
    };
    card_shell(border, background, padding)
}

/// The share of `--border-subtle` a tinted sub-card keeps when the caller does
/// not say otherwise: `accentCardStyle`'s `color-mix(accent, border-subtle 76%)`.
pub const TINT_BORDER_SHARE: f32 = 0.24;

/// A sub-card whose tint pair is stated in full — the source's inline
/// `{ borderColor: color-mix(tone, border-subtle N%), background:
/// color-mix(tone, surface-1 M%) }`.
///
/// `background_share` and `border_share` are the **tone's** share, so
/// `sub_card_tinted(MOVE_TINT, 0.06, 0.22)` is `#4f8cff` at 6% over
/// `--surface-1` and at 22% over `--border-subtle`
/// (`TransformControls.tsx`'s `moveCardStyle`).
pub fn sub_card_tinted(tone: u32, background_share: f32, border_share: f32) -> Div {
    card_shell(
        mix(tone, BORDER_SUBTLE, border_share),
        mix(tone, SURFACE_1, background_share),
        8.0,
    )
}

/// The shape every sub-card shares: `rounded-md border p-2 space-y-1.5` with a
/// flex column and a 6px gap.
fn card_shell(border: u32, background: u32, padding: f32) -> Div {
    div()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .p(px(padding))
        .flex()
        .flex_col()
        .gap(px(6.0))
}
