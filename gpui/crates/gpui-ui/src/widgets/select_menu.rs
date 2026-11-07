//! The open `SelectDropdown` menu.

use gpui::{
    App, ClickEvent, Div, ElementId, FontWeight, SharedString, Stateful, Window, div, prelude::*,
    px, relative, rgb, rgba,
};

use gpui_core::icons::icon;
use gpui_core::theme::*;

use std::rc::Rc;

use super::constants::*;
use super::select::{SelectMenuAlign, SelectOption, SelectOptionTone};

/// [`select_dropdown`](super::select::select_dropdown) with the source's
/// `menuAlign`.
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
            .rounded(px(ROUNDED_BARE))
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
