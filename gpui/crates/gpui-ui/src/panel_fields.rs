//! Field primitives shared by the Prepare-mode tool panels.
//!
//! Sources: `src/components/ui/NumberInput.tsx`,
//! `src/components/ui/miniStepperField.tsx`,
//! `src/components/ui/compactNumberField.tsx`,
//! `src/components/ui/scrollableNumberField.tsx`, `src/components/atoms/Select.tsx`,
//! and the inline `SectionHeader` in `src/components/controls/TransformControls.tsx`.
//!
//! # The `.ui-panel` cascade
//!
//! Every panel that uses these primitives is a `Card` (`.ui-panel`), and
//! `globals.css` styles the panel's descendants outside Tailwind's
//! `@layer utilities`:
//!
//! ```css
//! .ui-panel .ui-input, .ui-panel .ui-select, .ui-panel .ui-button { font-size: 14px; }
//! .ui-panel .ui-input, .ui-panel .ui-select { padding: 6px 10px; }
//! ```
//!
//! Unlayered rules beat layered utilities whatever their specificity, so inside
//! a panel the `text-xs` / `text-[10px]` / `px-1.5` classes on a `.ui-button` or
//! `.ui-input` never apply: buttons and inputs paint at `--font-size-md` (14px)
//! with the base `.ui-button` padding (`5px 11px 7px`) or the panel input
//! padding (`6px 10px`). `!h-8` still wins (it is `!important`), and `.ui-meta`
//! (13px) beats `text-[10px]` on a label for the same reason. The values below
//! encode what the browser paints, not the class string.
//!
//! # PORT DEVIATIONS
//!
//! * GPUI has no text entry in this slice, so every `NumberInput` renders its
//!   value box (and steppers where the source shows them) without a caret.
//!   Typing is replaced by the steppers, matching `settings.rs`.
//! * `tracking-wide` (`letter-spacing: 0.025em`) has no GPUI text property; it
//!   is omitted.
//! * `uppercase` is applied to the string at build time.
//! * `custom-scrollbar` styling (10px track, accent thumb) is not ported; GPUI
//!   draws its own scrollbar. The scroll containers themselves are kept.
//! * A native `<select>`'s browser-drawn indicator and its OS menu have no GPUI
//!   equivalent: [`select_field`] paints a `chevron-down` in the same 28px right
//!   inset and opens an in-app menu below the trigger (GPUI has no portal, so
//!   the menu is an absolute child and can be clipped by a scrolling parent).

use std::rc::Rc;

use gpui::{
    App, ClickEvent, Div, ElementId, FontWeight, IntoElement, SharedString, Stateful, Window, div,
    prelude::*, px, rgb, rgba,
};

use gpui_core::icons::icon;
use gpui_core::theme::*;

/// `#ffffff`, the base of every `color-mix(..., white N%)` in the sources.
pub const WHITE: u32 = 0xffffff;
/// `#000000`, the base of every `color-mix(..., black N%)` in the sources.
pub const BLACK: u32 = 0x000000;

/// `#rrggbb` plus an alpha byte, for `gpui::rgba`.
pub fn with_alpha(color: u32, alpha: u8) -> u32 {
    (color << 8) | alpha as u32
}

/// `#f87171` — the X axis label and the error text.
pub const AXIS_X: u32 = 0xf87171;
/// `#4ade80` — the Y axis label.
pub const AXIS_Y: u32 = 0x4ade80;
/// `#60a5fa` — the Z axis label.
pub const AXIS_Z: u32 = 0x60a5fa;

/// The Transform panel's section tints: `#4f8cff` move, `#8f6cff` rotate,
/// `#2eb67d` scale, `#f59e0b` lift.
pub const MOVE_TINT: u32 = 0x4f8cff;
pub const ROTATE_TINT: u32 = 0x8f6cff;
pub const SCALE_TINT: u32 = 0x2eb67d;
pub const LIFT_TINT: u32 = 0xf59e0b;

/// `SectionHeader` in `TransformControls.tsx`: `py-0.5 text-center text-xs
/// font-semibold uppercase tracking-wide`, `--text-strong`.
pub fn section_header(title: &str) -> Div {
    div()
        .py(px(2.0))
        .text_center()
        .text_size(px(12.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT_STRONG))
        .child(title.to_uppercase())
}

/// `ui-meta mb-1 block text-center` with the axis colour inline.
///
/// `.ui-meta` is 13px `--indicator`; the inline `style` only overrides the
/// colour, so the size stays 13px even where the class list also says
/// `text-[10px]` (see the module note on the cascade).
pub fn axis_label(axis: &str, color: u32) -> Div {
    div()
        .mb(px(4.0))
        .text_center()
        .text_size(px(FONT_SM))
        .text_color(rgb(color))
        .child(axis.to_string())
}

/// Where a value sits inside its box: `text-left` or `text-center`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FieldAlign {
    Left,
    Center,
}

/// The unit suffix painted inside a field: `mm`, `%`, `°`, `×`.
#[derive(Clone, Copy)]
pub struct Unit {
    pub text: &'static str,
    pub size: f32,
}

impl Unit {
    pub const fn new(text: &'static str, size: f32) -> Self {
        Self { text, size }
    }
}

/// `.ui-input` inside a panel: `h-8` (32px), 14px text, `6px 10px` padding,
/// `--surface-0` fill, 6px radius, `--border-subtle` border.
///
/// The optional unit is the source's `absolute right-2 top-1/2
/// -translate-y-1/2` span, vertically centred by the flex row instead of a
/// transform.
pub fn value_field(
    value: impl Into<SharedString>,
    unit: Option<Unit>,
    align: FieldAlign,
    height: f32,
    disabled: bool,
) -> Div {
    value_field_at(value, unit, align, height, disabled, 8.0)
}

/// [`value_field`] with the unit's right inset chosen by the caller: the plain
/// field keeps the source's `right-2` (8px), a field that also carries the
/// port's chevrons moves the unit clear of their 16px gutter.
pub fn value_field_at(
    value: impl Into<SharedString>,
    unit: Option<Unit>,
    align: FieldAlign,
    height: f32,
    disabled: bool,
    unit_right: f32,
) -> Div {
    let value: SharedString = value.into();
    let mut field = div()
        .w_full()
        .h(px(height))
        .px(px(10.0))
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .text_size(px(FONT_MD))
        .text_color(rgb(TEXT_STRONG))
        .flex()
        .items_center()
        .child(value);
    field = match align {
        FieldAlign::Left => field.justify_start(),
        FieldAlign::Center => field.justify_center(),
    };

    let mut wrapper = div().relative().w_full().min_w(px(0.0)).child(field);
    if let Some(unit) = unit {
        wrapper = wrapper.child(
            div()
                .absolute()
                .right(px(unit_right))
                .top(px(0.0))
                .bottom(px(0.0))
                .flex()
                .items_center()
                .text_size(px(unit.size))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .child(unit.text),
        );
    }
    if disabled {
        wrapper = wrapper.opacity(0.5).cursor_not_allowed();
    }
    wrapper
}

/// One `NumberInput` chevron: `h-3 w-3` (12px), 10px glyph, `rounded`, hover
/// `bg-white/10`, `disabled:opacity-50`.
fn chevron_button(
    id: impl Into<ElementId>,
    name: &'static str,
    disabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let el = div()
        .id(id)
        .w(px(12.0))
        .h(px(12.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .child(icon(name, 10.0, TEXT_MUTED));
    if disabled {
        el.opacity(0.5).cursor_not_allowed()
    } else {
        el.cursor_pointer()
            .hover(|style| style.bg(rgba(with_alpha(WHITE, 0x1a))))
            .on_click(on_click)
    }
}

/// `MiniStepperField`: a 32px centred integer box with the two 12px chevrons at
/// `right-0.5` (`NumberInput`'s stepper, `showStepper` left at its default).
///
/// The source also steps on wheel; GPUI has no per-element wheel hook here, so
/// the chevrons are the only steppers (recorded in the module deviations).
pub fn chevron_field(
    id: impl Into<SharedString>,
    value: i32,
    min: i32,
    max: i32,
    disabled: bool,
    on_change: Rc<dyn Fn(i32)>,
) -> Div {
    let id: SharedString = id.into();
    let up = on_change.clone();
    let down = on_change;
    let can_up = !disabled && value < max;
    let can_down = !disabled && value > min;

    div()
        .relative()
        .w_full()
        .min_w(px(0.0))
        .child(value_field(
            value.to_string(),
            None,
            FieldAlign::Center,
            32.0,
            disabled,
        ))
        .child(
            div()
                .absolute()
                .right(px(2.0))
                .top(px(0.0))
                .bottom(px(0.0))
                .w(px(16.0))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(2.0))
                .child(chevron_button(
                    ElementId::named_usize(id.clone(), 0),
                    "chevron-up",
                    !can_up,
                    move |_, _, _| up((value + 1).min(max)),
                ))
                .child(chevron_button(
                    ElementId::named_usize(id, 1),
                    "chevron-down",
                    !can_down,
                    move |_, _, _| down((value - 1).max(min)),
                )),
        )
}

/// One `ScrollableNumberField` stepper: `IconButton` at `!h-8 !w-8 !p-0
/// !bg-[var(--surface-0)]`, `hover:!bg-[color-mix(surface-0, white 4%)]`,
/// `disabled` wearing `.ui-button:disabled`.
fn square_stepper(
    id: impl Into<ElementId>,
    name: &'static str,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let el = div()
        .id(id)
        .w(px(32.0))
        .h(px(32.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(RADIUS_SM))
        .border_1()
        .child(icon(name, 14.0, TEXT_MUTED));
    if enabled {
        el.border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_0))
            .cursor_pointer()
            .hover(|style| style.bg(rgb(mix(SURFACE_0, WHITE, 0.96))))
            .on_click(on_click)
    } else {
        el.border_color(rgb(mix(BORDER_SUBTLE, BLACK, 0.90)))
            .bg(rgb(mix(SURFACE_1, BLACK, 0.92)))
            .cursor_not_allowed()
    }
}

/// `ScrollableNumberField`: `flex items-center gap-1` around two 32px steppers
/// and a centred, semibold value box with the unit at `right-2`.
pub fn scrollable_field(
    id: impl Into<SharedString>,
    value: f32,
    unit: Option<Unit>,
    step: f32,
    min: f32,
    max: f32,
    disabled: bool,
    on_change: Rc<dyn Fn(f32)>,
) -> Div {
    let id: SharedString = id.into();
    let decrease = on_change.clone();
    let increase = on_change;
    let can_decrease = !disabled && value > min;
    let can_increase = !disabled && value < max;

    div()
        .w_full()
        .min_w(px(0.0))
        .flex()
        .items_center()
        .gap(px(4.0))
        .child(square_stepper(
            ElementId::named_usize(id.clone(), 0),
            "minus",
            can_decrease,
            move |_, _, _| decrease((value - step).max(min)),
        ))
        .child(div().flex_1().min_w(px(0.0)).child(value_field(
            format_number(value),
            unit,
            FieldAlign::Center,
            32.0,
            disabled,
        )))
        .child(square_stepper(
            ElementId::named_usize(id, 1),
            "plus",
            can_increase,
            move |_, _, _| increase((value + step).min(max)),
        ))
}

/// A `NumberInput` with `showStepper={false}` in the source, given the port's
/// two 12px chevrons in a 16px right gutter.
///
/// The source has no stepper at all on these fields — they are typed into. With
/// no text entry in this slice, the chevrons are the only way to change the
/// value; the unit moves to `right-2` + the gutter (20px) so the two do not
/// overlap. This is the port's one added affordance, recorded in the module
/// deviations.
pub fn stepped_field(
    id: impl Into<SharedString>,
    value: f32,
    unit: Option<Unit>,
    align: FieldAlign,
    step: f32,
    min: f32,
    max: f32,
    disabled: bool,
    on_change: Rc<dyn Fn(f32)>,
) -> Div {
    let id: SharedString = id.into();
    let up = on_change.clone();
    let down = on_change;
    let can_up = !disabled && value < max;
    let can_down = !disabled && value > min;

    div()
        .relative()
        .w_full()
        .min_w(px(0.0))
        .child(value_field_at(
            format_number(value),
            unit,
            align,
            32.0,
            disabled,
            20.0,
        ))
        .child(
            div()
                .absolute()
                .right(px(2.0))
                .top(px(0.0))
                .bottom(px(0.0))
                .w(px(16.0))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(2.0))
                .child(chevron_button(
                    ElementId::named_usize(id.clone(), 0),
                    "chevron-up",
                    !can_up,
                    move |_, _, _| up((value + step).min(max)),
                ))
                .child(chevron_button(
                    ElementId::named_usize(id, 1),
                    "chevron-down",
                    !can_down,
                    move |_, _, _| down((value - step).max(min)),
                )),
        )
}

/// `CompactNumberField`: a centred `ui-meta` label over the value box with the
/// unit inside it (10px, medium).
pub fn compact_field(
    id: impl Into<SharedString>,
    label: &str,
    value: f32,
    unit: Option<Unit>,
    step: f32,
    min: f32,
    max: f32,
    disabled: bool,
    on_change: Rc<dyn Fn(f32)>,
) -> Div {
    div()
        .min_w(px(0.0))
        .flex()
        .flex_col()
        .child(
            div()
                .text_center()
                .text_size(px(FONT_SM))
                .text_color(rgb(TEXT_MUTED))
                .child(label.to_string()),
        )
        .child(
            div().mt(px(4.0)).w_full().min_w(px(0.0)).child(stepped_field(
                id, value, unit, FieldAlign::Center, step, min, max, disabled, on_change,
            )),
        )
}

/// The Transform panel's small toggle: `h-7 min-w-[64px] rounded-md border px-2
/// text-[10px] font-semibold uppercase tracking-wide`.
///
/// This is a bare `<button>`, not `.ui-button`, so the class sizes do apply: 28px
/// tall, 64px minimum width, 10px type, 10px radius, 8px horizontal padding.
pub fn toggle_button(
    id: impl Into<ElementId>,
    label: &str,
    active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let (border, background, text) = if active {
        (
            mix(ACCENT, WHITE, 0.90),
            mix(ACCENT, SURFACE_0, 0.24),
            ACCENT_CONTRAST,
        )
    } else {
        (BORDER_SUBTLE, SURFACE_1, TEXT_MUTED)
    };
    div()
        .id(id)
        .h(px(28.0))
        .min_w(px(64.0))
        .px(px(8.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .text_size(px(10.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(text))
        .cursor_pointer()
        .on_click(on_click)
        .child(label.to_uppercase())
}

/// A `rounded-md border p-2` sub-card with an explicit tint pair, the shape every
/// section of the Transform, Arrange, Duplicate, Smoothing and Split panels uses.
pub fn sub_card(border: u32, background: u32) -> Div {
    div()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .p(px(8.0))
}

/// `rounded-md border p-2` on `--border-subtle` / `--surface-1` — the neutral
/// sub-card (`panelCardStyle` in the React panels).
pub fn plain_sub_card() -> Div {
    sub_card(BORDER_SUBTLE, SURFACE_1)
}

/// `accentCardStyle`: `color-mix(accent, border-subtle 76%)` on
/// `color-mix(accent, surface-1 95%)`.
pub fn accent_sub_card() -> Div {
    sub_card(
        mix(ACCENT, BORDER_SUBTLE, 0.24),
        mix(ACCENT, SURFACE_1, 0.05),
    )
}

/// `activeModeStyle`: the selected pill of a two/three-way mode choice, as
/// `(background, border, text)` — the order [`panel_button`] takes.
pub fn active_mode_style() -> (u32, u32, u32) {
    (
        mix(ACCENT, SURFACE_1, 0.15),
        mix(ACCENT, BORDER_SUBTLE, 0.30),
        TEXT_STRONG,
    )
}

/// `disabledModeStyle` / `disabledActionStyle`, as `(background, border, text)`.
pub fn disabled_action_style() -> (u32, u32, u32) {
    (
        mix(SURFACE_1, BLACK, 0.92),
        mix(BORDER_SUBTLE, BLACK, 0.90),
        mix(TEXT_MUTED, SURFACE_2, 0.18),
    )
}

/// The panel body: `px-2 pb-2 space-y-2 sm:px-2.5 sm:pb-2.5`.
///
/// The `sm:` step is the same 10px/10px the base class resolves to at the
/// panel's own width, so only one pair is needed.
pub fn panel_body() -> Div {
    div()
        .px(px(10.0))
        .pb(px(10.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
}

/// The Auto Orientation / Organic Cut body spacing: `px-2.5 pb-3 space-y-2.5`.
pub fn panel_body_wide() -> Div {
    div()
        .px(px(10.0))
        .pb(px(12.0))
        .flex()
        .flex_col()
        .gap(px(10.0))
}

/// `formatNumber` in the ported fields: two decimals, trailing zeros dropped.
pub fn format_number(value: f32) -> String {
    if !value.is_finite() {
        return "0".to_string();
    }
    let rounded = (value * 100.0).round() / 100.0;
    if (rounded - rounded.round()).abs() < 0.0001 {
        format!("{:.0}", rounded)
    } else if (rounded * 10.0 - (rounded * 10.0).round()).abs() < 0.0001 {
        format!("{:.1}", rounded)
    } else {
        format!("{:.2}", rounded)
    }
}

/// `Select.tsx` / `.ui-select` with an in-app menu.
///
/// The trigger is the `.ui-select` box: 32px tall, `--surface-0` (or the
/// caller's fill), `--border-subtle`, 6px radius, `padding-left: 10px`,
/// `padding-right: 28px`, 14px inside a panel, `line-height: 1`. The browser's
/// own indicator is replaced by a `chevron-down` in the same 28px inset, and the
/// native menu by the app's dropdown surface (see the module deviations).
pub fn select_field(
    id: impl Into<SharedString>,
    value_label: &str,
    options: &[(&'static str, &'static str)],
    open: bool,
    disabled: bool,
    background: u32,
    centered: bool,
    on_toggle: Rc<dyn Fn()>,
    on_select: Rc<dyn Fn(&'static str)>,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let mut trigger = div()
        .id(ElementId::named_usize(id.clone(), 0))
        .relative()
        .w_full()
        .h(px(32.0))
        .pl(px(10.0))
        .pr(px(28.0))
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(if open {
            mix(ACCENT, BORDER_SUBTLE, 0.82)
        } else {
            BORDER_SUBTLE
        }))
        .bg(rgb(background))
        .text_size(px(FONT_MD))
        .text_color(rgb(TEXT_STRONG))
        .flex()
        .items_center()
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .truncate()
                .when(centered, |this| this.text_center())
                .child(value_label.to_string()),
        )
        .child(
            div()
                .absolute()
                .right(px(10.0))
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
            .on_click(move |_, _, _| on_toggle());
    }

    let mut container = div()
        .id(ElementId::named_usize(id.clone(), 1))
        .relative()
        .w_full()
        .flex()
        .flex_col()
        .child(trigger);

    if open && !disabled {
        let mut menu = div()
            .id(ElementId::named_usize(id.clone(), 2))
            .absolute()
            .left(px(0.0))
            .right(px(0.0))
            .top(px(32.0))
            .mt(px(6.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(rgb(mix(ACCENT, BORDER_SUBTLE, 0.18)))
            .bg(rgb(mix(SURFACE_0, BLACK, 0.94)))
            .overflow_hidden()
            .flex()
            .flex_col()
            .shadow(vec![
                gpui::BoxShadow::new(px(0.0), px(14.0), rgba(with_alpha(BLACK, 0x75)).into())
                    .blur_radius(px(34.0)),
                gpui::BoxShadow::new(px(0.0), px(4.0), rgba(with_alpha(BLACK, 0x52)).into())
                    .blur_radius(px(12.0)),
            ]);

        for (index, (option_value, option_label)) in options.iter().enumerate() {
            let selected = *option_label == value_label;
            let select = on_select.clone();
            let option_value = *option_value;
            menu = menu.child(
                div()
                    .id(ElementId::named_usize(id.clone(), index + 3))
                    .w_full()
                    .px(px(12.0))
                    .py(px(8.0))
                    .text_size(px(FONT_MD))
                    .text_color(rgb(if selected { TEXT_STRONG } else { TEXT_MUTED }))
                    .when(selected, |this| this.bg(rgb(mix(ACCENT, SURFACE_0, 0.14))))
                    .cursor_pointer()
                    .hover(|style| {
                        style
                            .bg(rgba(with_alpha(SURFACE_2, 0xd1)))
                            .text_color(rgb(TEXT_STRONG))
                    })
                    .on_click(move |_, _, _| select(option_value))
                    .child(*option_label),
            );
        }
        container = container.child(menu);
    }

    container
}

/// A `Button` (`Button.tsx`) sized for a panel: `.ui-button`'s base recipe with
/// `!h-8`, painted at the panel's 14px (see the module note).
///
/// `variant` is [`PanelButton::Secondary`] or [`PanelButton::Accent`]; the
/// primary variant is the same `.ui-button-primary` fill.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PanelButton {
    Secondary,
    Accent,
    Primary,
}

/// The fill/border/text/hover of a [`PanelButton`], straight from
/// `.ui-button-secondary` / `-accent` / `-primary`.
pub fn panel_button_colors(variant: PanelButton) -> (u32, u32, u32, u32) {
    match variant {
        PanelButton::Secondary => (SURFACE_1, BORDER_SUBTLE, TEXT_STRONG, SURFACE_2),
        PanelButton::Accent => (
            SECONDARY_BUTTON_SURFACE,
            mix(SECONDARY_BUTTON_SURFACE, WHITE, 0.86),
            ACCENT_SECONDARY_CONTRAST,
            mix(SECONDARY_BUTTON_SURFACE, BLACK, 0.91),
        ),
        PanelButton::Primary => (
            PRIMARY_BUTTON_SURFACE,
            mix(PRIMARY_BUTTON_SURFACE, WHITE, 0.86),
            ACCENT_CONTRAST,
            mix(PRIMARY_BUTTON_SURFACE, BLACK, 0.88),
        ),
    }
}

/// The busy label: `inline-flex items-center gap-1.5` around the spinner and the
/// text, as `Arranging…` / `Filling Plate…` / `Duplicating…` / `Cutting...` wear
/// it.
pub fn busy_label(text: &str, spinner_size: f32) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(crate::ui::spinner(spinner_size))
        .child(text.to_string())
}

/// A panel action button: `.ui-button` geometry, `!h-8`, `w-full`, label centred.
///
/// `style` overrides the variant's fill/border/text when the source sets an
/// inline `style` triple; `enabled == false` paints `.ui-button:disabled`.
pub fn panel_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    variant: PanelButton,
    style: Option<(u32, u32, u32)>,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let label: SharedString = label.into();
    panel_button_element(id, label, variant, style, enabled, on_click)
}

/// [`panel_button`] with an element for its content — the busy states that put a
/// spinner beside the label (`Arranging…`, `Filling Plate…`, `Duplicating…`,
/// `Cutting...`).
pub fn panel_button_element(
    id: impl Into<ElementId>,
    content: impl IntoElement,
    variant: PanelButton,
    style: Option<(u32, u32, u32)>,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let content = content.into_any_element();
    let (background, border, text, hover) = panel_button_colors(variant);
    let (background, border, text) = style.unwrap_or((background, border, text));
    let el = div()
        .id(id)
        .w_full()
        .h(px(32.0))
        .px(px(11.0))
        .pt(px(5.0))
        .pb(px(7.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(RADIUS_SM))
        .border_1()
        .text_size(px(FONT_MD))
        .font_weight(FontWeight::SEMIBOLD)
        .line_height(px(FONT_MD))
        .child(content);
    if enabled {
        el.border_color(rgb(border))
            .bg(rgb(background))
            .text_color(rgb(text))
            .cursor_pointer()
            .hover(move |s| s.bg(rgb(hover)))
            .on_click(on_click)
    } else {
        let (border, background, text) = disabled_action_style();
        el.border_color(rgb(border))
            .bg(rgb(background))
            .text_color(rgb(text))
            .cursor_not_allowed()
    }
}
