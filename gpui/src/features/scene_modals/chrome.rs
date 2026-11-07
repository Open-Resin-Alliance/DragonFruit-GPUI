//! Shared dialog chrome for the scene-file, material, mesh-repair and
//! diagnostics dialogs: the scrim, the dialog panel and the title/action rows
//! every dialog below composes.

use gpui::{
    App, BoxShadow, ClickEvent, Div, ElementId, FontWeight, IntoElement, SharedString, Window, div,
    prelude::*, px, rgb, rgba,
};

use gpui_core::icons::icon;
use gpui_core::theme::*;
use gpui_ui::panel_fields::{BLACK, with_alpha};
use gpui_ui::ui::{IconButtonSize, IconButtonVariant, icon_button};

// The `rounded-*` scale and the `#000000` base come from `theme` / `panel_fields`:
// `globals.css` overrides `--radius-md`/`--radius-lg` to 10px/14px, so a local
// copy would drift the dialogs away from the panels.

/// Tailwind `leading-tight`.
pub(super) const LEADING_TIGHT: f32 = 1.25;

/// Tailwind `leading-snug`.
pub(super) const LEADING_SNUG: f32 = 1.375;

/// Tailwind `leading-relaxed`.
pub(super) const LEADING_RELAXED: f32 = 1.625;

/// `#d97706` — amber-600, the warning accent in every dialog here.
pub(super) const AMBER_600: u32 = 0xd97706;

/// `#f59e0b` — amber-500, the checkbox accent and the material note.
pub(super) const AMBER_500: u32 = 0xf59e0b;

/// `#fde68a` — amber-200, the plugin warning's Continue label.
pub(super) const AMBER_200: u32 = 0xfde68a;

/// `#22c55e` — green-500, the save-choice tile.
pub(super) const GREEN_500: u32 = 0x22c55e;

/// `max-w-lg`.
pub(super) const DIALOG_LG: f32 = 512.0;

/// `max-w-xl`.
pub(super) const DIALOG_XL: f32 = 576.0;

/// A CSS `box-shadow` layer.
///
/// `gpui_ui::widgets::constants::shadow` is identical but crate-private, so the
/// dialog chrome cannot import it.
pub(super) fn shadow(x: f32, y: f32, blur: f32, spread: f32, color: u32) -> BoxShadow {
    BoxShadow::new(px(x), px(y), rgba(color).into())
        .blur_radius(px(blur))
        .spread_radius(px(spread))
}

/// `color-mix(in srgb, a, b weight_b)` — `b`'s share is the source's percentage,
/// so `a`'s share is `1 - weight_b`.
pub(super) fn mix_b(a: u32, b: u32, weight_b: f32) -> u32 {
    mix(a, b, 1.0 - weight_b)
}

/// `StructuredDialogModal`'s backdrop: `fixed inset-0 flex items-center
/// justify-center bg-black/<alpha>` with the caller's horizontal padding.
///
/// Every hand-written dialog in this module tree — the structured shells, the
/// custom `SceneFileModals` headers and the three wide debug sheets — paints
/// its scrim through this one function.
pub(super) fn dialog_backdrop(alpha: u32, padding: f32) -> Div {
    let mut backdrop = div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(rgba(with_alpha(BLACK, alpha as u8)));
    if padding > 0.0 {
        backdrop = backdrop.px(px(padding));
    }
    backdrop
}

/// `StructuredDialogModal`'s panel: `w-full rounded-xl border` on `surface-0`,
/// as a `flex-col`. The caller adds its overflow mode and its drop shadow, so
/// the scrolling debug sheets and the clipped custom dialogs share the same
/// frame.
pub(super) fn dialog_panel(width: f32) -> Div {
    div()
        .w_full()
        .max_w(px(width))
        .rounded(px(ROUNDED_XL))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .flex()
        .flex_col()
}

/// The custom dialogs in `SceneFileModals` / `MeshRepairModals` repeat the
/// `StructuredDialogModal` shell by hand (`fixed inset-0 z-[220] … max-w-lg`),
/// so they share the backdrop and panel above.
pub(super) fn custom_dialog(width: f32, header: Div, body: Div) -> Div {
    dialog_backdrop(0x8c, 12.0).child(
        dialog_panel(width)
            .overflow_hidden()
            .shadow(vec![shadow(0.0, 24.0, 46.0, 0.0, with_alpha(BLACK, 0x6b))])
            .child(header)
            .child(body),
    )
}

/// The hand-written header of the custom dialogs: `flex items-center
/// justify-between gap-4 border-b px-5 py-4`, a 36px `IconChip` with the
/// caller's tint, `text-base font-semibold leading-tight` over `text-[11px]
/// leading-snug`, and a `surface` `IconButton`.
pub(super) fn custom_dialog_header(
    id: &'static str,
    icon_name: &str,
    icon_border: u32,
    icon_background: u32,
    icon_foreground: u32,
    title: impl Into<SharedString>,
    subtitle: impl Into<SharedString>,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.0))
        .border_b(px(1.0))
        .border_color(rgb(BORDER_SUBTLE))
        .px(px(20.0))
        .py(px(16.0))
        .child(
            div()
                .flex()
                .min_w(px(0.0))
                .items_center()
                .gap(px(12.0))
                .child(
                    div()
                        .flex()
                        .flex_shrink_0()
                        .items_center()
                        .justify_center()
                        .w(px(36.0))
                        .h(px(36.0))
                        .rounded(px(ROUNDED_MD))
                        .border_1()
                        .border_color(rgb(icon_border))
                        .bg(rgb(icon_background))
                        .child(icon(icon_name, 16.0, icon_foreground)),
                )
                .child(
                    div()
                        .min_w(px(0.0))
                        .pr(px(8.0))
                        .child(
                            div()
                                .text_size(px(16.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .line_height(px(16.0 * LEADING_TIGHT))
                                .text_color(rgb(TEXT_STRONG))
                                .child(title.into()),
                        )
                        .child(
                            div()
                                .mt(px(2.0))
                                .text_size(px(11.0))
                                .line_height(px(11.0 * LEADING_SNUG))
                                .text_color(rgb(TEXT_MUTED))
                                .child(subtitle.into()),
                        ),
                ),
        )
        .child(
            icon_button(
                ElementId::named_usize(id, 0),
                "x",
                IconButtonVariant::Surface,
                IconButtonSize::Md,
            )
            .on_click(on_close),
        )
}

/// A `text-sm leading-relaxed` paragraph in `--text-muted`.
pub(super) fn muted_paragraph(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(14.0))
        .line_height(px(14.0 * LEADING_RELAXED))
        .text_color(rgb(TEXT_MUTED))
        .child(text.into())
}

/// A `text-xs leading-relaxed` paragraph in `--text-muted`.
pub(super) fn muted_small_paragraph(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(12.0))
        .line_height(px(12.0 * LEADING_RELAXED))
        .text_color(rgb(TEXT_MUTED))
        .child(text.into())
}

/// One run of a paragraph: plain, or the source's `<strong>` emphasis
/// (`--text-strong`, `font-medium`).
pub(super) enum Run {
    Plain(SharedString),
    Strong(SharedString),
}

impl Run {
    pub(super) fn plain(text: impl Into<SharedString>) -> Self {
        Self::Plain(text.into())
    }

    pub(super) fn strong(text: impl Into<SharedString>) -> Self {
        Self::Strong(text.into())
    }
}

/// A paragraph with inline emphasis. CSS flows the runs as one line box; GPUI
/// text elements are block-level, so the runs sit in a wrapping flex row and
/// wrap at run boundaries (see the module's PORT DEVIATIONS).
pub(super) fn rich_paragraph(size: f32, leading: f32, runs: Vec<Run>) -> Div {
    let mut row = div()
        .flex()
        .flex_wrap()
        .items_baseline()
        .text_size(px(size))
        .line_height(px(size * leading))
        .text_color(rgb(TEXT_MUTED));
    for run in runs {
        row = row.child(match run {
            Run::Plain(text) => div().child(text),
            Run::Strong(text) => div()
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(TEXT_STRONG))
                .child(text),
        });
    }
    row
}

/// The `rounded-md border px-3 py-2` warning strip the confirm dialogs share:
/// `color-mix(#d97706, border-subtle 40%)` on `color-mix(#d97706,
/// surface-1 92%)`, with a 14px `alert-triangle` and `text-xs` copy.
pub(super) fn warning_strip(text: impl Into<SharedString>) -> Div {
    div()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(mix_b(AMBER_600, BORDER_SUBTLE, 0.40)))
        .bg(rgb(mix_b(AMBER_600, SURFACE_1, 0.92)))
        .px(px(12.0))
        .py(px(8.0))
        .child(
            div()
                .flex()
                .items_start()
                .gap(px(8.0))
                .child(
                    div()
                        .mt(px(2.0))
                        .flex_shrink_0()
                        .child(icon("alert-triangle", 14.0, AMBER_600)),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .line_height(px(12.0 * LEADING_RELAXED))
                        .text_color(rgb(TEXT_MUTED))
                        .child(text.into()),
                ),
        )
}

/// The `rounded-md border px-3 py-2` identity card the import prompts and the
/// repair sheet share: `--border-subtle` on `--surface-1`, an 11px uppercase
/// label over a `text-sm font-semibold` value.
pub(super) fn identity_card(label: impl Into<SharedString>, value: impl Into<SharedString>) -> Div {
    div()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .px(px(12.0))
        .py(px(8.0))
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(TEXT_MUTED))
                .child(label.into().to_uppercase()),
        )
        .child(
            div()
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .truncate()
                .child(value.into()),
        )
}

/// One action of a dialog's action row. `StructuredDialogModal`'s row is
/// `grid grid-flow-col auto-cols-fr`, so every action takes an equal column.
pub(super) fn action_slot(child: impl IntoElement) -> Div {
    div().flex_1().child(child)
}

/// The two tinted button tones the dialogs use with a leading glyph.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TintedTone {
    Accent,
    Danger,
}

/// A `Button` in a tinted variant with an optional 14px glyph before its label.
///
/// `Button.tsx`'s tinted classes are `.ui-button-tint` plus the tone's
/// declaration in `globals.css`; the values are the ones `ui.rs`'
/// `button_colors` carries for `TintedAccent` / `TintedDanger`. The source's
/// `className="inline-flex items-center justify-center gap-1.5"` supplies the
/// 6px gap, which `button` cannot add because it has already appended its label.
pub(super) fn tinted_action(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    glyph: Option<(&str, f32)>,
    tone: TintedTone,
) -> gpui::Stateful<Div> {
    let label: SharedString = label.into();
    let (background, border, text, hover) = match tone {
        TintedTone::Accent => (
            mix(ACCENT, SURFACE_1, 0.14),
            mix(ACCENT, BORDER_SUBTLE, 0.55),
            ACCENT,
            mix(ACCENT, SURFACE_1, 0.24),
        ),
        TintedTone::Danger => (
            mix(DANGER, SURFACE_1, 0.14),
            mix(DANGER, BORDER_SUBTLE, 0.55),
            DANGER,
            mix(DANGER, SURFACE_1, 0.24),
        ),
    };
    let mut el = div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .h(px(36.0))
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
        .hover(move |style| style.bg(rgb(hover)));
    if let Some((name, size)) = glyph {
        el = el.child(icon(name, size, text));
    }
    el.child(label)
}

// ── Scene import placement / hollowing ──────────────────────────────────────

/// A two-column button row (`grid grid-cols-2 gap-2`).
pub(super) fn two_button_row(left: impl IntoElement, right: impl IntoElement) -> Div {
    div()
        .flex()
        .gap(px(8.0))
        .pt(px(4.0))
        .child(div().flex_1().child(left))
        .child(div().flex_1().child(right))
}

/// One table header cell: `p-2 font-medium`.
pub(super) fn table_header_cell(label: impl Into<SharedString>) -> Div {
    div()
        .flex_1()
        .min_w(px(0.0))
        .p(px(8.0))
        .font_weight(FontWeight::MEDIUM)
        .child(label.into())
}

/// One table body cell: `p-2`.
pub(super) fn table_cell(child: impl IntoElement) -> Div {
    div()
        .flex_1()
        .min_w(px(0.0))
        .p(px(8.0))
        .child(child)
}

/// The table frame: `overflow-hidden rounded-md border` on `--border-subtle`.
pub(super) fn table_frame() -> Div {
    div()
        .overflow_hidden()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
}

/// The table header row: `background: var(--surface-1)`, `--text-muted`.
pub(super) fn table_head() -> Div {
    div()
        .flex()
        .bg(rgb(SURFACE_1))
        .text_color(rgb(TEXT_MUTED))
}

/// A table body row: `border-t` on `--border-subtle`.
pub(super) fn table_row() -> Div {
    div()
        .flex()
        .items_start()
        .border_t(px(1.0))
        .border_color(rgb(BORDER_SUBTLE))
}

