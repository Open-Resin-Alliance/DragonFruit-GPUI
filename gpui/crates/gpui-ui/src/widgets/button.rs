//! Buttons: the `.ui-button` family and the icon-button tiles.

use gpui::{Div, ElementId, FontWeight, SharedString, Stateful, div, prelude::*, px, rgb, rgba};

use gpui_core::icons::icon;
use gpui_core::theme::*;

use super::constants::*;
use super::icon::{IconTone, icon_tone_colors};

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
                .rounded(px(ROUNDED_BARE))
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

// ── ButtonSpec ──────────────────────────────────────────────────────────────

/// Width override for [`ButtonSpec`].
#[derive(Clone, Copy, PartialEq)]
pub enum Width {
    /// `w-full` on the source element.
    Full,
    /// A literal width in px.
    Fixed(f32),
}

/// The `.ui-button` recipe with the utilities a panel layers on top of it.
///
/// Every panel button in the React tree is `.ui-button` plus one or two
/// utilities (`px-2.5`, `rounded-md`, `text-[11px]`, `w-full`). Without this
/// type each panel re-typed the recipe and the paddings drifted to seven
/// different values; with it the recipe is here and the panels only state their
/// deviation.
#[derive(Clone)]
pub struct ButtonSpec {
    pub variant: ButtonVariant,
    pub size: ButtonSize,
    pub width: Option<Width>,
    pub padding_x: Option<f32>,
    /// `(top, bottom)`; the class default is `(5, 7)`.
    pub padding_y: Option<(f32, f32)>,
    pub radius: Option<f32>,
    pub font_size: Option<f32>,
    pub line_height: Option<f32>,
    pub gap: Option<f32>,
    pub disabled: bool,
    /// Explicit hover fill; `None` uses the variant's.
    pub hover: Option<u32>,
    /// `(icon name, px)` drawn before the label.
    pub icon: Option<(SharedString, f32)>,
    /// Icon colour; defaults to the variant's text colour.
    pub icon_color: Option<u32>,
}

impl ButtonSpec {
    pub fn new(variant: ButtonVariant, size: ButtonSize) -> Self {
        Self {
            variant,
            size,
            width: None,
            padding_x: None,
            padding_y: None,
            radius: None,
            font_size: None,
            line_height: None,
            gap: None,
            disabled: false,
            hover: None,
            icon: None,
            icon_color: None,
        }
    }

    pub fn full(mut self) -> Self {
        self.width = Some(Width::Full);
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(Width::Fixed(width));
        self
    }

    pub fn padding_x(mut self, padding: f32) -> Self {
        self.padding_x = Some(padding);
        self
    }

    pub fn padding_y(mut self, top: f32, bottom: f32) -> Self {
        self.padding_y = Some((top, bottom));
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = Some(gap);
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    pub fn icon(mut self, name: &str, size: f32, color: u32) -> Self {
        self.icon = Some((name.into(), size));
        self.icon_color = Some(color);
        self
    }
}

/// [`button`] with per-site overrides.
pub fn button_with(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    spec: &ButtonSpec,
) -> Stateful<Div> {
    let label: SharedString = label.into();
    button_with_content(id, spec, label)
}

/// [`button_with`] for a button whose content is not a plain label.
pub fn button_with_content(
    id: impl Into<ElementId>,
    spec: &ButtonSpec,
    content: impl IntoElement,
) -> Stateful<Div> {
    let (background, border, text, hover) = button_colors(spec.variant);
    let height = button_height(spec.size);
    let (pad_top, pad_bottom) = spec.padding_y.unwrap_or((5.0, 7.0));
    let font = spec.font_size.unwrap_or(FONT_MD);

    let mut el = div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(spec.radius.unwrap_or(RADIUS_SM)))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .pt(px(pad_top))
        .pb(px(pad_bottom))
        .px(px(spec.padding_x.unwrap_or(11.0)))
        .text_size(px(font))
        .font_weight(FontWeight::SEMIBOLD)
        .line_height(px(spec.line_height.unwrap_or(font)))
        .text_color(rgb(text));

    if let Some(gap) = spec.gap {
        el = el.gap(px(gap));
    }
    if let Some(height) = height {
        el = el.h(px(height));
    }
    match spec.width {
        Some(Width::Full) => el = el.w_full(),
        Some(Width::Fixed(width)) => el = el.w(px(width)),
        None => {}
    }

    if spec.disabled {
        // `.ui-button:disabled` — the same overrides `button_disabled` applies.
        el = el
            .border_color(rgb(mix(BORDER_SUBTLE, BLACK, 0.90)))
            .bg(rgb(mix(SURFACE_1, BLACK, 0.92)))
            .text_color(rgb(mix(TEXT_MUTED, SURFACE_2, 0.82)))
            .cursor_not_allowed();
    } else {
        let hover = spec.hover.unwrap_or(hover);
        el = el.cursor_pointer().hover(move |style| style.bg(rgb(hover)));
    }

    let mut content = content.into_any_element();
    if let Some((name, size)) = spec.icon.clone() {
        let color = spec.icon_color.unwrap_or(text);
        content = div()
            .flex()
            .items_center()
            .gap(px(spec.gap.unwrap_or(6.0)))
            .child(icon(&name, size, color))
            .child(content)
            .into_any_element();
    }
    el.child(content)
}
