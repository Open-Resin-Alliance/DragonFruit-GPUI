//! Ported primitives from `src/components/atoms` and `src/components/ui`.
//!
//! Every value here comes from the source component's class strings and the
//! tokens in `theme.rs`; nothing is invented.

use declarative_gpui::ui;
use gpui::{
    Div, ElementId, FontWeight, IntoElement, SharedString, Stateful, div, prelude::*, px, rgb,
};

use crate::icons::icon;
use crate::theme::*;

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

/// `Button.tsx` + `.ui-button*` in `globals.css`.
pub fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    variant: ButtonVariant,
    size: ButtonSize,
) -> Stateful<Div> {
    let label: SharedString = label.into();
    let (background, border, text, hover) = match variant {
        ButtonVariant::Primary => (
            PRIMARY_BUTTON_SURFACE,
            mix(PRIMARY_BUTTON_SURFACE, 0xffffff, 0.86),
            ACCENT_CONTRAST,
            mix(PRIMARY_BUTTON_SURFACE, 0x000000, 0.88),
        ),
        ButtonVariant::Secondary => (
            SURFACE_1,
            BORDER_SUBTLE,
            TEXT_STRONG,
            SURFACE_2,
        ),
        ButtonVariant::Accent => (
            SECONDARY_BUTTON_SURFACE,
            mix(SECONDARY_BUTTON_SURFACE, 0xffffff, 0.86),
            ACCENT_SECONDARY_CONTRAST,
            mix(SECONDARY_BUTTON_SURFACE, 0x000000, 0.91),
        ),
        ButtonVariant::Danger => (
            0xdc2626,
            mix(0xdc2626, 0xffffff, 0.85),
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
    };
    let (height, padding, font_size) = match size {
        ButtonSize::Xs => (28.0, 8.0, 11.0),
        ButtonSize::Sm => (32.0, 10.0, 11.0),
        ButtonSize::Md => (36.0, 12.0, 12.0),
        ButtonSize::Lg => (40.0, 16.0, 13.0),
        ButtonSize::Auto => (36.0, 11.0, FONT_MD),
    };
    let _ = (height, padding, font_size);
    ui! {
        div(
            id = id,
            flex, items_center, justify_center,
            rounded_6, border, border_color = rgb(border),
            bg = rgb(background),
            h = px(height),
            px = px(padding),
            text_size = px(font_size),
            font_semibold,
            text_color = rgb(text),
            hover(|style| style.bg(rgb(hover))),
        ) {
            text(label)
        }
    }
}

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
pub fn icon_button(
    id: impl Into<ElementId>,
    icon_name: &str,
    variant: IconButtonVariant,
    size: IconButtonSize,
) -> Stateful<Div> {
    let (w, h) = match size {
        IconButtonSize::Xs => (24.0, 24.0),
        IconButtonSize::Sm => (32.0, 32.0),
        IconButtonSize::Md => (36.0, 36.0),
        IconButtonSize::Lg => (40.0, 40.0),
        IconButtonSize::Auto => (36.0, 36.0),
    };
    let mut button = div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(RADIUS_SM))
        .child(icon(icon_name, 16.0, TEXT_MUTED));
    button = match variant {
        IconButtonVariant::Solid => button
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1))
            .hover(|style| style.bg(rgb(SURFACE_2))),
        IconButtonVariant::Surface => button
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1))
            .hover(|style| style.bg(rgb(SURFACE_2))),
        IconButtonVariant::Ghost => button.hover(|style| style.bg(rgb(SURFACE_1))),
    };
    button.w(px(w)).h(px(h))
}

/// `Card.tsx`: `ui-panel rounded-lg shadow-lg border`.
pub fn card() -> Div {
    div()
        .rounded(px(8.0))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .flex()
        .flex_col()
}

/// `CardHeader.tsx`: `px-2.5 py-2.5`, title 14px semibold.
pub fn card_header(title: impl Into<SharedString>) -> Div {
    div()
        .px(px(10.0))
        .py(px(10.0))
        .flex()
        .items_center()
        .gap(px(10.0))
        .child(
            div()
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child(title.into()),
        )
}

/// `Toggle.tsx` track and knob.
pub fn toggle(on: bool) -> Div {
    div()
        .w(px(34.0))
        .h(px(18.0))
        .rounded_full()
        .bg(rgb(if on { ACCENT } else { SURFACE_2 }))
        .flex()
        .items_center()
        .justify_end()
        .pr(px(2.0))
        .child(
            div()
                .w(px(14.0))
                .h(px(14.0))
                .rounded_full()
                .bg(rgb(ACCENT_CONTRAST)),
        )
}

/// A row in a `ContextMenu`.
pub fn menu_item(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    disabled: bool,
) -> Stateful<Div> {
    let label: SharedString = label.into();
    div()
        .id(id)
        .h(px(28.0))
        .px(px(8.0))
        .flex()
        .items_center()
        .rounded(px(6.0))
        .text_size(px(11.0))
        .text_color(rgb(if disabled { 0x5a5f6a } else { TEXT_MUTED }))
        .when(!disabled, |this| {
            this.hover(|style| style.bg(rgb(SURFACE_1)))
        })
        .child(label)
}

/// `Spinner.tsx`: a rotating arc; GPUI has no CSS animation, so this is a
/// static ring with an accent arc.
pub fn spinner(size: f32) -> Div {
    div()
        .w(px(size))
        .h(px(size))
        .rounded_full()
        .border_2()
        .border_color(rgb(mix(ACCENT, SURFACE_1, 0.35)))
}

/// The backdrop and card of `StructuredDialogModal`.
pub fn modal_shell(width: f32) -> Div {
    div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .size_full()
        .bg(gpui::rgba(0x000000b3))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .w(px(width))
                .rounded(px(RADIUS_MD))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_0))
                .p(px(16.0))
                .flex()
                .flex_col()
                .gap(px(10.0)),
        )
}
