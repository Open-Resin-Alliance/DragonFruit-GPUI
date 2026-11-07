//! The control kit every tab body is built from: the `SettingRow` / `Section`
//! shapes, the segmented control, the number/select/range fields and the small
//! button recipes, each traced to its React class string in the tab sources.

use super::*;

/// `#rrggbb` → `u32`.
pub(crate) fn hex_color(value: &str) -> u32 {
    u32::from_str_radix(value.trim().trim_start_matches('#'), 16).unwrap_or(0xa3a3a3)
}

pub(crate) fn swatch(value: &str, width: f32, height: f32) -> Div {
    div()
        .w(px(width))
        .h(px(height))
        .flex_shrink_0()
        .rounded(px(4.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(hex_color(value)))
}

/// `#rrggbb` plus an alpha byte, for `gpui::rgba`.
pub(crate) fn with_alpha(color: u32, alpha: u8) -> u32 {
    (color << 8) | alpha as u32
}

/// `SelectDropdown` — the trigger plus its menu, with the open state kept in
/// the draft so a re-render keeps it open.
///
/// The source's `leadingDisplay` is trigger-only; [`select_dropdown`] takes the
/// leading glyph from the selected option, so that row also shows it.
pub(crate) fn select_field(
    id: &'static str,
    options: &[(&'static str, &'static str)],
    value: &str,
    leading: Option<&'static str>,
    width: Option<f32>,
    enabled: bool,
    entity: WeakEntity<DragonFruit>,
    on_change: impl Fn(&str, &mut App) + 'static,
) -> Div {
    let open = state()
        .lock()
        .map(|guard| guard.open_select == Some(id))
        .unwrap_or(false);

    let mut items: Vec<SelectOption> = Vec::with_capacity(options.len());
    for (option_value, label) in options {
        let mut option = SelectOption::new(*option_value, *label);
        if *option_value == value {
            option.icon = leading.map(SharedString::from);
        }
        items.push(option);
    }
    let values: Vec<&'static str> = options.iter().map(|(value, _)| *value).collect();

    let toggle_entity = entity.clone();
    let select_entity = entity.clone();
    let on_change: Rc<dyn Fn(&str, &mut App)> = Rc::new(on_change);

    let mut container = div().flex_shrink_0();
    container = match width {
        Some(width) => container.w(px(width)),
        None => container.w_full(),
    };
    container.child(select_dropdown(
        id,
        &items,
        value,
        open,
        !enabled,
        move |_, _, cx| {
            edit_and_notify(&toggle_entity, cx, |draft| {
                draft.open_select = if draft.open_select == Some(id) {
                    None
                } else {
                    Some(id)
                };
            });
        },
        move |index, _, cx| {
            if let Some(chosen) = values.get(index).copied() {
                on_change(chosen, cx);
                edit_and_notify(&select_entity, cx, |draft| draft.open_select = None);
            }
        },
    ))
}

/// Decimal places a step can land on, capped at 6 like `countStepDecimals`.
pub(crate) fn step_decimals(step: f32) -> u32 {
    if !step.is_finite() {
        return 0;
    }
    let text = format!("{step}");
    match text.split_once('.') {
        Some((_, decimals)) => decimals.len().min(6) as u32,
        None => 0,
    }
}

pub(crate) fn round_to(value: f32, decimals: u32) -> f32 {
    let factor = 10f32.powi(decimals as i32);
    (value * factor).round() / factor
}

/// `NumberInput` — the value box with the source's stacked `ChevronUp` /
/// `ChevronDown` stepper (`absolute inset-y-0 right-0.5 w-4`, two 12px tiles).
///
/// The box is not editable here: typing needs a text-input entity this slice
/// does not have (see the module note).
pub(crate) fn number_input(
    id: &'static str,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    width: Option<f32>,
    height: f32,
    font: f32,
    background: u32,
    enabled: bool,
    on_change: impl Fn(f32, &mut App) + 'static,
) -> Div {
    let decimals = step_decimals(step);
    let on_change: Rc<dyn Fn(f32, &mut App)> = Rc::new(on_change);
    let up = on_change.clone();
    let down = on_change;
    let at_max = value >= max;
    let at_min = value <= min;

    let stepper = |id_suffix: &str,
                   slug: &'static str,
                   disabled: bool,
                   delta: f32,
                   callback: Rc<dyn Fn(f32, &mut App)>| {
        let mut button = div()
            .id(format!("{id}-{id_suffix}"))
            .w(px(12.))
            .h(px(12.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(4.))
            .child(icon(slug, 10., TEXT_MUTED));
        if disabled || !enabled {
            button = button.opacity(0.5);
        } else {
            button = button
                .cursor_pointer()
                .hover(|style| style.bg(rgba(with_alpha(WHITE, 0x1a))))
                .on_click(move |_, _, cx| {
                    let next = round_to((value + delta).clamp(min, max), decimals);
                    callback(next, cx);
                });
        }
        button
    };

    let mut container = div().relative().flex_shrink_0();
    container = match width {
        Some(width) => container.w(px(width)),
        None => container.w_full(),
    };

    container
        .child(
            div()
                .h(px(height))
                .w_full()
                .flex()
                .items_center()
                .rounded(px(6.)) // rounded-md
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(background))
                .pl(px(10.))
                .pr(px(20.)) // pr-5
                .text_size(px(font))
                .text_color(rgb(TEXT_STRONG))
                .when(!enabled, |this| this.opacity(0.5))
                .child(format_number(value)),
        )
        .child(
            div()
                .absolute()
                .top(px(0.))
                .bottom(px(0.))
                .right(px(2.))
                .w(px(16.))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(2.))
                .child(stepper("up", "chevron-up", at_max, step, up))
                .child(stepper("down", "chevron-down", at_min, -step, down)),
        )
}

/// `.ui-checkbox` with the source's `!w-3.5 !h-3.5` override (14px).
pub(crate) fn checkbox(checked: bool, enabled: bool) -> Div {
    let mut box_ = div()
        .w(px(14.))
        .h(px(14.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .border_1();
    if checked {
        box_ = box_
            .border_color(rgb(mix(ACCENT, WHITE, 0.88)))
            .bg(rgb(mix(ACCENT, SURFACE_0, 0.18)))
            .child(icon("check", 10., ACCENT_CONTRAST));
    } else {
        box_ = box_
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1));
    }
    if !enabled {
        box_ = box_.opacity(0.5);
    }
    box_
}

/// `rounded-md border px-3 py-2 text-xs` message strip.
pub(crate) fn message_strip(kind: Option<bool>, text: &str) -> Div {
    let (border, color) = match kind {
        Some(true) => (mix(0xef4444, BORDER_SUBTLE, 0.4), 0xfca5a5),
        _ => (mix(0x22c55e, BORDER_SUBTLE, 0.4), 0x86efac),
    };
    div()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(SURFACE_1))
        .px(px(12.))
        .py(px(8.))
        .text_size(px(12.))
        .text_color(rgb(color))
        .child(text.to_owned())
}

/// `rounded-md border px-2.5 py-2` fact tile on `--surface-0`.
pub(crate) fn stat_card(label: &str, value: impl Into<SharedString>) -> Div {
    div()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .px(px(10.))
        .py(px(8.))
        .child(
            div()
                .text_size(px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .child(label.to_owned()),
        )
        .child(
            div()
                .mt(px(4.))
                .text_size(px(12.))
                .text_color(rgb(TEXT_STRONG))
                .child(value.into()),
        )
}

/// The `text-[11px] uppercase tracking-wide font-semibold` group label.
pub(crate) fn stat_label(label: &str) -> Div {
    div()
        .text_size(px(11.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT_MUTED))
        .child(label.to_owned())
}

/// A read-only text field in the source's shape. Typing needs a text-input
/// entity this slice does not have, so the value (or placeholder) is painted.
/// `mono` is accepted for the source's `font-mono` call sites; no monospace
/// face is registered in this slice.
pub(crate) fn text_field(
    value: &str,
    placeholder: &str,
    mono: bool,
    height: f32,
    font: f32,
) -> Div {
    let _ = mono;
    let empty = value.is_empty();
    div()
        .h(px(height))
        .w_full()
        .flex()
        .items_center()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .px(px(10.))
        .text_size(px(font))
        .text_color(rgb(if empty { TEXT_MUTED } else { TEXT_STRONG }))
        .child(if empty {
            placeholder.to_owned()
        } else {
            value.to_owned()
        })
}

/// `rounded-md border px-3 py-2` callout with a leading icon.
pub(crate) fn callout(border: u32, background: u32, icon_slug: &'static str, icon_color: u32, text: &str) -> Div {
    div()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .px(px(12.))
        .py(px(8.))
        .flex()
        .items_start()
        .gap(px(8.))
        .child(icon(icon_slug, 14., icon_color))
        .child(
            div()
                .flex_1()
                .text_size(px(12.))
                .line_height(relative(1.35))
                .text_color(rgb(TEXT_MUTED))
                .child(text.to_owned()),
        )
}

/// `SettingRow` with the `surface` / `disabled` props the source uses.
///
/// `raised` is `--surface-1` (a row inside a `--surface-0` panel), the default
/// `sunken` is `--surface-0`; `disabled` dims the row to `opacity: 0.68`.
pub(crate) fn setting_row_surface(
    label: impl Into<SharedString>,
    description: impl Into<SharedString>,
    raised: bool,
    disabled: bool,
    controls: Div,
) -> Div {
    let row = div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.))
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(if raised { SURFACE_1 } else { SURFACE_0 }))
        .p(px(6.))
        .child(
            div()
                .min_w(px(0.))
                .pl(px(4.))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(relative(1.2))
                        .text_color(rgb(TEXT_STRONG))
                        .child(label.into()),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .line_height(relative(1.2))
                        .text_color(rgb(TEXT_MUTED))
                        .child(description.into()),
                ),
        )
        .child(div().flex().items_center().gap(px(8.)).child(controls));
    if disabled {
        row.opacity(0.68)
    } else {
        row
    }
}

pub(crate) fn section(icon_slug: &'static str, title: &'static str, description: &'static str) -> Div {
    div()
        .flex()
        .flex_col()
        .rounded(px(6.)) // rounded-md
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(12.))
        .child(section_header(icon_slug, title, description))
}

pub(crate) fn section_header(icon_slug: &'static str, title: &'static str, description: &'static str) -> Div {
    div()
        .flex()
        .items_start()
        .gap(px(8.))
        .child(icon_chip(icon_slug))
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child(title),
                )
                .child(
                    div()
                        .mt(px(2.))
                        .text_size(px(12.))
                        .text_color(rgb(TEXT_MUTED))
                        .child(description),
                ),
        )
}

pub(crate) fn icon_chip(icon_slug: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .w(px(32.))
        .h(px(32.))
        .flex_shrink_0()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        // color-mix(in srgb, surface-2, transparent 8%) over surface-1
        .bg(rgb(mix(SURFACE_2, SURFACE_1, 0.92)))
        .child(icon(icon_slug, 16., ACCENT))
}

/// `SettingRow` with `bordered`: `rounded-md border p-1.5` on `--surface-0`.
pub(crate) fn setting_row(
    label: impl Into<SharedString>,
    description: impl Into<SharedString>,
    control: impl IntoElement,
) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.))
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .p(px(6.))
        .child(
            div()
                .min_w(px(0.))
                .pl(px(4.))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(relative(1.2))
                        .text_color(rgb(TEXT_STRONG))
                        .child(label.into()),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .line_height(relative(1.2))
                        .text_color(rgb(TEXT_MUTED))
                        .child(description.into()),
                ),
        )
        .child(div().flex().items_center().gap(px(8.)).child(control))
}

/// `SettingRow` without `bordered`.
pub(crate) fn plain_row(
    label: impl Into<SharedString>,
    description: impl Into<SharedString>,
    control: impl IntoElement,
) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.))
        .child(
            div()
                .min_w(px(0.))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(relative(1.2))
                        .text_color(rgb(TEXT_STRONG))
                        .child(label.into()),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .line_height(relative(1.2))
                        .text_color(rgb(TEXT_MUTED))
                        .child(description.into()),
                ),
        )
        .child(div().flex().items_center().gap(px(8.)).child(control))
}

/// `rounded-md border p-2.5` on `--surface-0`.
pub(crate) fn inner_card() -> Div {
    div()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .p(px(10.))
        .flex()
        .flex_col()
}

/// `rounded-md border px-2.5 py-1.5` on `--surface-0`.
pub(crate) fn field_card() -> Div {
    div()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .px(px(10.))
        .py(px(6.))
        .flex()
        .flex_col()
}

pub(crate) fn field_label(label: &'static str) -> Div {
    div()
        .mb(px(4.))
        .text_size(px(12.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(TEXT_MUTED))
        .child(label)
}

pub(crate) fn upper_label(label: &'static str) -> Div {
    div()
        .mb(px(4.))
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT_MUTED))
        .child(label)
}

pub(crate) fn divider_label(icon_slug: &'static str, label: &'static str) -> Div {
    div()
        .mt(px(12.))
        .flex()
        .items_center()
        .gap(px(8.))
        .child(div().h(px(1.)).flex_1().bg(rgb(BORDER_SUBTLE)))
        .child(
            div()
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .child(icon(icon_slug, 12., TEXT_MUTED))
                .child(label),
        )
        .child(div().h(px(1.)).flex_1().bg(rgb(BORDER_SUBTLE)))
}

/// `SegmentedControl`: `h-10 min-w-[92px] px-3 text-[12px]` pills, uppercase
/// labels applied at the call site (the source uses CSS `uppercase`).
pub(crate) fn segmented(
    id: &'static str,
    options: &'static [&'static str],
    active: usize,
    full_width: bool,
    on_select: impl Fn(usize, &mut App) + 'static,
) -> Div {
    let on_select: Rc<dyn Fn(usize, &mut App)> = Rc::new(on_select);
    let mut row = div().flex().items_center().gap(px(4.));
    if full_width {
        row = row.w_full();
    }
    for (index, label) in options.iter().enumerate() {
        let is_active = index == active;
        let handler = on_select.clone();
        let (background, border, text) = if is_active {
            (
                mix(ACCENT, SURFACE_0, 0.24),
                mix(ACCENT, 0xffffff, 0.9),
                mix(ACCENT, TEXT_STRONG, 0.25),
            )
        } else {
            (SURFACE_1, BORDER_SUBTLE, TEXT_MUTED)
        };
        let mut option = div()
            .id(format!("{id}-{index}"))
            .h(px(40.))
            .px(px(12.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(6.))
            .border_1()
            .border_color(rgb(border))
            .bg(rgb(background))
            .text_size(px(12.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(text))
            .cursor_pointer()
            .on_click(move |_, _, cx| handler(index, cx))
            .child(*label);
        option = if full_width {
            option.flex_1()
        } else {
            option.min_w(px(92.))
        };
        row = row.child(option);
    }
    row
}

/// `ScrollableNumberField`: 32px steppers around a centred value box.
pub(crate) fn number_control(
    id: &'static str,
    value: f32,
    unit: &'static str,
    step: f32,
    min: f32,
    max: f32,
    width: f32,
    on_change: impl Fn(f32, &mut App) + 'static,
) -> Div {
    let on_change: Rc<dyn Fn(f32, &mut App)> = Rc::new(on_change);
    let decrease = on_change.clone();
    let increase = on_change;
    div()
        .w(px(width))
        .flex()
        .items_center()
        .gap(px(4.))
        .child(
            div()
                .id(format!("{id}-minus"))
                .w(px(32.))
                .h(px(32.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_0))
                .cursor_pointer()
                .hover(|style| style.bg(rgb(mix(SURFACE_0, 0xffffff, 0.96))))
                .child(icon("minus", 14., TEXT_MUTED))
                .on_click(move |_, _, cx| decrease((value - step).max(min), cx)),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .h(px(32.))
                .flex()
                .items_center()
                .justify_center()
                .gap(px(2.))
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_0))
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child(format_number(value))
                .child(
                    div()
                        .text_size(px(10.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(TEXT_MUTED))
                        .child(unit),
                ),
        )
        .child(
            div()
                .id(format!("{id}-plus"))
                .w(px(32.))
                .h(px(32.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_0))
                .cursor_pointer()
                .hover(|style| style.bg(rgb(mix(SURFACE_0, 0xffffff, 0.96))))
                .child(icon("plus", 14., TEXT_MUTED))
                .on_click(move |_, _, cx| increase((value + step).min(max), cx)),
        )
}

pub(crate) fn format_number(value: f32) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    if (rounded - rounded.round()).abs() < 0.0001 {
        format!("{:.0}", rounded)
    } else {
        let mut text = format!("{:.2}", rounded);
        while text.ends_with('0') {
            text.pop();
        }
        text
    }
}

/// `.ui-range`: an 8px track with the accent fill and a knob at its end.
pub(crate) fn slider(value: f32, min: f32, max: f32) -> Div {
    let fraction = if (max - min).abs() < f32::EPSILON {
        0.0
    } else {
        ((value - min) / (max - min)).clamp(0.0, 1.0)
    };
    div()
        .w_full()
        .h(px(8.))
        .rounded_full()
        .bg(rgb(SURFACE_2))
        .child(
            div()
                .h(px(8.))
                .w(relative(fraction))
                .rounded_full()
                .bg(rgb(ACCENT))
                .flex()
                .items_center()
                .justify_end()
                .child(
                    div()
                        .w(px(12.))
                        .h(px(12.))
                        .flex_shrink_0()
                        .rounded_full()
                        .border_1()
                        .border_color(rgb(ACCENT))
                        .bg(rgb(ACCENT_CONTRAST)),
                ),
        )
}

/// Label + value line above a range track, inside a `px-2.5 py-1.5` card.
///
/// The track renders the current value; GPUI has no DOM range input in this
/// slice, so there is no drag handler (see the module note).
pub(crate) fn slider_field(
    label: &'static str,
    value_text: impl Into<SharedString>,
    value: f32,
    min: f32,
    max: f32,
) -> Div {
    field_card().child(
        div()
            .flex()
            .items_center()
            .justify_between()
            .mb(px(6.))
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(TEXT_MUTED))
                    .child(label),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(TEXT_STRONG))
                    .child(value_text.into()),
            ),
    )
    .child(slider(value, min, max))
}

/// Footer action: `!h-10 !px-4 text-sm gap-1.5`.
pub(crate) fn action_button(
    id: &'static str,
    label: &'static str,
    icon_slug: Option<&'static str>,
    padding: f32,
    background: u32,
    border: u32,
    text: u32,
    hover: u32,
    on_click: impl Fn(&mut App) + 'static,
) -> Stateful<Div> {
    let mut button = div()
        .id(id)
        .h(px(40.))
        .px(px(padding))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.))
        .flex_shrink_0()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .text_size(px(14.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(text))
        .cursor_pointer()
        .hover(move |style| style.bg(rgb(hover)))
        .on_click(move |_, _, cx| on_click(cx));
    if let Some(slug) = icon_slug {
        button = button.child(icon(slug, 16., text));
    }
    button.child(label)
}

/// `!h-8 !px-2.5 text-xs` theme action (`ui-button-secondary`).
pub(crate) fn small_button(
    id: &'static str,
    label: &'static str,
    enabled: bool,
    on_click: impl Fn(&mut App) + 'static,
) -> Stateful<Div> {
    let (background, border, text) = if enabled {
        (SURFACE_1, BORDER_SUBTLE, TEXT_STRONG)
    } else {
        (SURFACE_1, BORDER_SUBTLE, TEXT_MUTED)
    };
    div()
        .id(id)
        .h(px(32.))
        .px(px(10.))
        .flex()
        .items_center()
        .justify_center()
        .flex_shrink_0()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(text))
        .when(enabled, |this| {
            this.cursor_pointer()
                .hover(|style| style.bg(rgb(SURFACE_2)))
                .on_click(move |_, _, cx| on_click(cx))
        })
        .when(!enabled, |this| this.opacity(0.5))
        .child(label)
}
