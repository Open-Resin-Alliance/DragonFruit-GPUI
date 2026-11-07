//! Single-line text and setting-row layout primitives.

use gpui::{
    App, ClickEvent, Div, ElementId, FontWeight, IntoElement, SharedString, Stateful, Window, div,
    prelude::*, px, rgb,
};

use gpui_core::icons::icon;
use gpui_core::theme::*;

use super::constants::*;

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

// ── Section headers ─────────────────────────────────────────────────────────

/// `SectionHeader` (`TransformControls.tsx`, `MeshSmoothingSettingsPanel.tsx`):
/// `py-0.5 text-center text-xs font-semibold uppercase tracking-wide` in
/// `--text-strong`. `tracking-wide` has no GPUI equivalent; the label is
/// upper-cased in the string, as the `uppercase` utility would.
pub fn section_header(title: impl Into<SharedString>) -> Div {
    let title: SharedString = title.into();
    div()
        .py(px(2.0))
        .text_center()
        .text_size(px(FONT_XS))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT_STRONG))
        .child(title.to_uppercase())
}

/// [`section_header`] between two `flex-1` slots: the Transform panel's centred
/// section title with a control at either end (`Move`'s Local toggle on the
/// right, `Rotate`'s Group pill on the left).
pub fn section_row(title: impl Into<SharedString>, leading: Option<Div>, trailing: Option<Div>) -> Div {
    div()
        .flex()
        .items_center()
        .child(div().flex_1().when_some(leading, |this, leading| {
            this.flex().justify_start().child(leading)
        }))
        .child(section_header(title))
        .child(div().flex_1().when_some(trailing, |this, trailing| {
            this.flex().justify_end().child(trailing)
        }))
}

/// The muted `ui-meta` caption above a control group (`Cut Mode`, `Layout
/// Mode`, `Arrange Distance`, `Placement Anchor`): 13px in `--text-muted`.
pub fn section_label(text: impl Into<SharedString>) -> Div {
    let text: SharedString = text.into();
    div()
        .text_size(px(FONT_SM))
        .text_color(rgb(TEXT_MUTED))
        .child(text)
}

/// The `text-[10px] uppercase tracking-wide` column caption of a grid
/// (`ArrangePanel`'s Count/Gap row), centred in its column, in `--text-muted`.
pub fn column_heading(text: impl Into<SharedString>) -> Div {
    let text: SharedString = text.into();
    div()
        .flex_1()
        .text_center()
        .text_size(px(10.0))
        .text_color(rgb(TEXT_MUTED))
        .child(text.to_uppercase())
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

// ── Badge ───────────────────────────────────────────────────────────────────

/// The pill badge a panel header carries (`P+3`, `C#2`, a mode name).
///
/// `rounded-full px-2 py-0.5 text-[10px]`, icon at 12px in the accent colour.
pub fn badge(label: impl Into<gpui::SharedString>, icon: Option<(&str, u32)>) -> Div {
    let label: gpui::SharedString = label.into();
    let mut el = div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .rounded_full()
        .border_1()
        .border_color(rgb(mix(BORDER_SUBTLE, BLACK, 0.94)))
        .bg(rgb(mix(SURFACE_2, BLACK, 0.92)))
        .px(px(8.0))
        .py(px(2.0))
        .text_size(px(10.0))
        .line_height(px(12.0))
        .text_color(rgb(TEXT_MUTED));
    if let Some((name, color)) = icon {
        el = el.child(gpui_core::icons::icon(name, 12.0, color));
    }
    el.child(label)
}

/// The badge with no icon and the muted recipe `header_badge` uses.
pub fn badge_plain(label: impl Into<gpui::SharedString>) -> Div {
    badge(label, None)
}
