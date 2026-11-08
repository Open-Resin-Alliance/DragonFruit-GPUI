//! The control kit every tab body is built from: the `SettingRow` / `Section`
//! shapes, the segmented control, the number/select/range fields and the small
//! button recipes, each traced to its React class string in the tab sources.

use super::*;

/// `#rrggbb` → `u32`; unparseable text paints black rather than borrowing a
/// control's default colour.
pub(crate) fn hex_color(value: &str) -> u32 {
    u32::from_str_radix(value.trim().trim_start_matches('#'), 16).unwrap_or(BLACK)
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
/// The box is a real editor now: [`number_typed`] commits each keystroke the
/// way `NumberInput.tsx:74-97` does, and the two tiles are the same steppers.
/// `font` and `background` no longer size the box — `.ui-input`'s unlayered
/// `font-size: 13px` and `background: var(--surface-0)` win over the
/// `text-sm`/inline-background the source passes, as they do in the app.
pub(crate) fn number_input(
    id: &'static str,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    width: Option<f32>,
    height: f32,
    _font: f32,
    _background: u32,
    enabled: bool,
    on_change: impl Fn(f32, &mut App) + 'static,
) -> Div {
    let decimals = step_decimals(step);
    let on_change: Rc<dyn Fn(f32, &mut App)> = Rc::new(on_change);
    let up = on_change.clone();
    let down = on_change.clone();
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
        .child(number_typed(
            id,
            value,
            min,
            max,
            // `fmt.field(value, 2)`: at most two fraction digits, no trailing
            // zeros; the stepper rounds to its own step above.
            2,
            !enabled,
            height,
            {
                let on_change = on_change.clone();
                move |next, _, cx| on_change(next, cx)
            },
        ))
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
        // `.ui-checkbox:checked` (`globals.css:526-530`): `color-mix(in srgb,
        // var(--accent), white 12%)` border over `color-mix(in srgb,
        // var(--accent), var(--surface-0) 18%)` fill, i.e. accent 88% / 82%.
        box_ = box_
            .border_color(rgb(mix(ACCENT, WHITE, 0.88)))
            .bg(rgb(mix(ACCENT, SURFACE_0, 0.82)))
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
    // `color-mix(in srgb, #ef4444 | #22c55e, var(--border-subtle) 40%)`: the
    // status colour is the 60% share.
    let (border, color) = match kind {
        Some(true) => (mix(0xef4444, BORDER_SUBTLE, 0.6), 0xfca5a5),
        _ => (mix(0x22c55e, BORDER_SUBTLE, 0.6), 0x86efac),
    };
    div()
        .rounded(px(ROUNDED_MD))
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
        .rounded(px(ROUNDED_MD))
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

/// The source's `.ui-input` text box, now a real editor: [`text_input`] commits
/// every keystroke, the way `PluginsSettingsTab.tsx:805-812` writes `repoUrl`.
///
/// `mono` is accepted for the source's `font-mono` call sites; no monospace face
/// is registered in this slice. `font` is kept for the call sites' own
/// bookkeeping — `.ui-input`'s unlayered 13px type wins over it.
pub(crate) fn text_field(
    key: &'static str,
    value: &str,
    placeholder: impl Into<SharedString>,
    mono: bool,
    height: f32,
    _font: f32,
    on_change: impl Fn(&str, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let _ = mono;
    text_input(key, value, placeholder, false, height, on_change)
}

/// `rounded-md border px-3 py-2` callout with a leading icon.
pub(crate) fn callout(border: u32, background: u32, icon_slug: &'static str, icon_color: u32, text: &str) -> Div {
    div()
        .rounded(px(ROUNDED_MD))
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

pub(crate) fn section(icon_slug: &'static str, title: &'static str, description: &'static str) -> Div {
    div()
        .flex()
        .flex_col()
        .rounded(px(ROUNDED_MD))
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

/// The hand-written 32px section tile (`h-8 w-8 rounded-md border` with
/// `color-mix(surface-2, transparent 8%)` over `surface-1`): the shared
/// `IconChip` md shape with the source's fill and accent glyph instead of a
/// tone.
pub(crate) fn icon_chip(icon_slug: &'static str) -> Div {
    icon_chip_with(
        IconChipSize::Md,
        IconTone::Neutral,
        icon(icon_slug, 16., ACCENT),
    )
    // color-mix(in srgb, surface-2, transparent 8%) over surface-1
    .bg(rgb(mix(SURFACE_2, SURFACE_1, 0.92)))
}

/// `rounded-md border p-2.5` on `--surface-0`.
pub(crate) fn inner_card() -> Div {
    div()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .p(px(10.))
        .flex()
        .flex_col()
}

/// `rounded-md border p-2` on `--surface-0` — the select cards in the Mesh tab
/// and the Profile forms (`MeshSettingsTab.tsx:213`, `:233`; the sibling
/// `field_card` is the `px-2.5 py-1.5` shape).
pub(crate) fn selector_card() -> Div {
    div()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .p(px(8.))
        .flex()
        .flex_col()
}

/// `rounded-md border px-2.5 py-1.5` on `--surface-0`.
pub(crate) fn field_card() -> Div {
    div()
        .rounded(px(ROUNDED_MD))
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

/// `mb-1 block text-xs font-semibold uppercase tracking-wide` in
/// `--text-muted` — the label class the UI & Theme cards use
/// (`UISettingsTab.tsx:296`, `:308`); the source upper-cases through CSS, GPUI
/// has no `text-transform`, so the text is upper-cased here.
pub(crate) fn upper_label(label: &'static str) -> Div {
    div()
        .mb(px(4.))
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT_MUTED))
        .child(label.to_uppercase())
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
        // `color-mix(in srgb, var(--segmented-tone), var(--text-strong) 25%)`
        // (`globals.css:435`): 75% tone + 25% text-strong, so the active label
        // reads as the tone rather than as near-white text.
        let (background, border, text) = if is_active {
            (
                mix(ACCENT, SURFACE_0, 0.24),   // `surface-0 76%`
                mix(ACCENT, 0xffffff, 0.9),     // `white 10%`
                mix(ACCENT, TEXT_STRONG, 0.75), // `text-strong 25%`
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
            // `.ui-segmented-option:hover:not(:disabled) { background: var(--surface-2) }`
            // — it out-specifies the active rule, so the hover applies to both.
            .hover(|style| style.bg(rgb(SURFACE_2)))
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
///
/// The box is editable: [`number_on_blur`] matches this control's
/// `commitOnBlur` mode (`scrollableNumberField.tsx:94-131`), which is the mode
/// the General tab's custom scale uses (`GeneralSettingsTab.tsx:237`).
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

    // `disabled:!bg-[color-mix(in_srgb,var(--surface-1),black_8%)]` — the source
    // disables the decrement at `<= min` and the increment at `>= max` instead
    // of silently clamping a click.
    let stepper = |suffix: &str,
                   slug: &'static str,
                   disabled: bool,
                   delta: f32,
                   callback: Rc<dyn Fn(f32, &mut App)>| {
        let mut button = div()
            .id(format!("{id}-{suffix}"))
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
            .child(icon(slug, 14., TEXT_MUTED));
        if disabled {
            button = button
                .bg(rgb(mix(SURFACE_1, 0x000000, 0.92)))
                .opacity(0.5);
        } else {
            button = button
                .cursor_pointer()
                .hover(|style| style.bg(rgb(mix(SURFACE_0, 0xffffff, 0.96))))
                .on_click(move |_, _, cx| callback((value + delta).clamp(min, max), cx));
        }
        button
    };

    div()
        .w(px(width))
        .flex()
        .items_center()
        .gap(px(4.))
        .child(stepper("minus", "minus", value <= min, -step, decrease))
        .child(
            div().flex_1().min_w(px(0.)).child(number_on_blur(
                id,
                value,
                Some((unit, 1.0)),
                min,
                max,
                2,
                false,
                32.0,
                {
                    let increase = increase.clone();
                    move |next, _, cx| increase(next, cx)
                },
            )),
        )
        .child(stepper("plus", "plus", value >= max, step, increase))
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
    field_card().child(slider_group(label, value_text, value, min, max))
}

/// The same label/value line and track **without** the card: the source's
/// `space-y-0.5` slider groups inside an already-bordered panel
/// (`CameraSettingsTab.tsx:352-398`, `SpaceMouseSettingsTab.tsx:203-284`).
/// Wrapping those in [`slider_field`] drew a border the source never has.
pub(crate) fn slider_group(
    label: &'static str,
    value_text: impl Into<SharedString>,
    value: f32,
    min: f32,
    max: f32,
) -> Div {
    div()
        .flex()
        .flex_col()
        .child(
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


