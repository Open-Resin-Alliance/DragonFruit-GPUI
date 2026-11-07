//! Numeric fields: the stepper overlay and its field variants.

use gpui::{
    App, ClickEvent, Div, ElementId, FontWeight, SharedString, Stateful, Window, div, prelude::*,
    px, rgb, rgba,
};

use gpui_core::icons::icon;
use gpui_core::theme::*;

use std::rc::Rc;

use super::constants::*;
use super::input::input_box;

// ── NumberInput ─────────────────────────────────────────────────────────────

/// The stepper overlay of `NumberInput.tsx`:
/// `absolute inset-y-0 right-0.5 flex w-4 flex-col items-center justify-center
/// gap-0.5`, two `h-3 w-3` buttons with `rounded hover:bg-white/10
/// disabled:opacity-50` and a 10px (`h-2.5 w-2.5`) chevron each.
///
/// GPUI has no text editing on a `div`, so the port renders the field's frame
/// and steppers; the caller owns the text and the commit. The overlay is
/// absolutely positioned, so the caller places it in a `relative` parent.
pub fn number_stepper(
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
            .rounded(px(ROUNDED_BARE))
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

/// `LabeledNumberInput`'s field (`profileFormAtoms.tsx`): `ui-input h-[36px]`
/// with the unit inside the right edge and the default stepper.
///
/// The input is `ui-input w-full h-[36px] pl-2.5 pr-14 leading-tight text-sm
/// no-spinners` — of which only `h-[36px]` survives, because `.ui-input` is
/// unlayered and keeps its own padding and type (see the module note). The unit
/// sits at `absolute inset-y-0 right-6 z-10`, 11px semibold in `--text-muted`,
/// left of the 16px stepper gutter. The label above the field belongs to the
/// caller (it carries the help tile).
pub fn labeled_number_field(
    id: impl Into<SharedString>,
    display: impl Into<SharedString>,
    unit: Option<&str>,
    disabled: bool,
    can_increment: bool,
    can_decrement: bool,
    on_up: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_down: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let mut field = input_box(id.clone(), false)
        .h(px(36.0))
        .child(display.into());
    if disabled {
        field = field.opacity(0.55).cursor_not_allowed();
    }

    let mut wrapper = div()
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
        ));
    if let Some(unit) = unit {
        wrapper = wrapper.child(
            div()
                .absolute()
                .right(px(24.0))
                .top(px(0.0))
                .bottom(px(0.0))
                .flex()
                .items_center()
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .child(unit.to_string()),
        );
    }
    wrapper
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
