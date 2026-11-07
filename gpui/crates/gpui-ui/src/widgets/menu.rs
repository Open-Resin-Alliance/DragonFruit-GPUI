//! Context menus and menu rows.

use gpui::{
    App, Div, ElementId, FontWeight, SharedString, Stateful, Window, div, prelude::*, px, rgb,
};

use gpui_core::icons::icon;
use gpui_core::theme::*;

use std::rc::Rc;

use super::constants::*;

// ── ContextMenu ─────────────────────────────────────────────────────────────

/// One right-click / dropdown menu entry (`ContextMenu.tsx`).
pub struct ContextMenuEntry {
    pub id: SharedString,
    pub label: SharedString,
    pub icon: Option<SharedString>,
    /// Draw the icon without the framed slot (`iconBare`). A colour chip already
    /// carries its own edge; framing it again reads as a box inside a box.
    pub icon_bare: bool,
    /// Renders a tick in the icon slot — for toggles.
    pub checked: bool,
    pub disabled: bool,
    /// Destructive action: row text and its hover fill use the danger colour.
    pub danger: bool,
    /// Rule above this entry, breaking the list into groups.
    pub starts_group: bool,
    /// Right-aligned secondary text.
    pub trailing: Option<SharedString>,
}

impl ContextMenuEntry {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            icon_bare: false,
            checked: false,
            disabled: false,
            danger: false,
            starts_group: false,
            trailing: None,
        }
    }

    pub fn with_icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn bare_icon(mut self) -> Self {
        self.icon_bare = true;
        self
    }

    pub fn checked(mut self) -> Self {
        self.checked = true;
        self
    }

    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    pub fn starts_group(mut self) -> Self {
        self.starts_group = true;
        self
    }

    pub fn with_trailing(mut self, trailing: impl Into<SharedString>) -> Self {
        self.trailing = Some(trailing.into());
        self
    }
}

/// `ContextMenu.tsx` — the menu surface, heading, grouping rules and rows.
///
/// Surface: `rounded-lg border p-1.5 shadow-xl` on
/// `color-mix(surface-0, black 10%)` with `border-subtle`. Rows:
/// `flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px]
/// font-medium`, a 20px framed icon slot, `check` for checked rows, and the
/// danger/accent hover fills from the source.
///
/// React portals the menu to the viewport position of the gesture; GPUI renders
/// it wherever the caller places it. Nested flyouts (`entry.children`) are not
/// rendered — the port has no submenu state.
pub fn context_menu(
    entries: &[ContextMenuEntry],
    title: Option<SharedString>,
    on_select: impl Fn(&str, &mut Window, &mut App) + 'static,
) -> Div {
    // `widthClassName` defaults to `w-48`.
    context_menu_sized(entries, title, 192.0, on_select)
}

/// `ContextMenu`'s surface: `rounded-lg border p-1.5 shadow-xl` on
/// `color-mix(surface-0, #000 10%)` with `border-subtle`.
///
/// Exposed for a menu whose rows are not [`ContextMenuEntry`] values — the
/// top bar's fleet-unit quick menu draws printer rows inside the same surface.
pub fn menu_surface(width: f32) -> Div {
    div()
        .w(px(width))
        .rounded(px(ROUNDED_LG))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(mix(SURFACE_0, BLACK, 0.90)))
        .p(px(6.0))
        .shadow_xl()
        .flex()
        .flex_col()
}

/// `ContextMenu`'s heading row: `mb-1 px-2 py-1 text-[10px] font-semibold
/// uppercase tracking-wide`, uppercased at build time.
pub fn menu_title(title: impl Into<SharedString>) -> Div {
    let title: SharedString = title.into();
    div()
        .mb(px(4.0))
        .px(px(8.0))
        .py(px(4.0))
        .text_size(px(10.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT_MUTED))
        .child(title.to_uppercase())
}

/// [`context_menu`] with the source's `widthClassName` resolved to pixels.
pub fn context_menu_sized(
    entries: &[ContextMenuEntry],
    title: Option<SharedString>,
    width: f32,
    on_select: impl Fn(&str, &mut Window, &mut App) + 'static,
) -> Div {
    let on_select = Rc::new(on_select);
    let base = SharedString::from("context-menu");

    let mut menu = menu_surface(width);

    if let Some(title) = title {
        menu = menu.child(menu_title(title));
    }

    let mut rows = div().flex().flex_col().gap(px(2.0));

    for (index, entry) in entries.iter().enumerate() {
        if entry.starts_group && index > 0 {
            rows = rows.child(
                div()
                    .my(px(4.0))
                    .h(px(1.0))
                    .w_full()
                    .bg(rgb(BORDER_SUBTLE)),
            );
        }

        let tone = if entry.disabled {
            TEXT_MUTED
        } else if entry.danger {
            DANGER
        } else {
            TEXT_STRONG
        };
        let hover_fill = if entry.danger {
            mix(DANGER, SURFACE_1, 0.10)
        } else {
            mix(ACCENT, SURFACE_1, 0.18)
        };

        let mut row = div()
            .id(ElementId::named_usize(base.clone(), index))
            .flex()
            .items_center()
            .gap(px(8.0))
            .rounded(px(ROUNDED_MD))
            .px(px(8.0))
            .py(px(6.0))
            .text_size(px(13.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(tone));

        // The 20px icon slot: `rounded border` on `surface-1`, a tick when
        // checked. `iconBare` drops the frame and the fill.
        let bare = entry.icon_bare && !entry.checked;
        let mut slot = div()
            .flex()
            .items_center()
            .justify_center()
            .w(px(20.0))
            .h(px(20.0))
            .opacity(if entry.disabled { 0.8 } else { 1.0 });
        if !bare {
            slot = slot
                .rounded(px(ROUNDED_BARE))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_1));
        }
        if entry.checked {
            let check_color = if entry.disabled { TEXT_MUTED } else { ACCENT };
            slot = slot.child(icon("check", 14.0, check_color));
        } else if let Some(name) = &entry.icon {
            slot = slot.child(icon(name.as_ref(), 14.0, tone));
        }
        row = row.child(slot);

        // `flex-1` only when there is trailing content; the label itself is not
        // truncated by the source.
        let mut label = div().child(entry.label.clone());
        if entry.trailing.is_some() {
            label = label.flex_1();
        }
        row = row.child(label);

        if let Some(trailing) = &entry.trailing {
            row = row.child(div().child(trailing.clone()));
        }

        if entry.disabled {
            row = row.opacity(0.55).cursor_not_allowed();
        } else {
            let callback = on_select.clone();
            let entry_id = entry.id.clone();
            row = row
                .cursor_pointer()
                .hover(move |style| style.bg(rgb(hover_fill)))
                .on_click(move |_, window, cx| {
                    callback(entry_id.as_ref(), window, cx);
                });
        }

        rows = rows.child(row);
    }

    menu.child(rows)
}

/// A row in a `ContextMenu`, without the icon slot.
///
/// Kept from the first slice: `MenuRow`'s geometry and tones, for a caller that
/// only has a label.
pub fn menu_item(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    disabled: bool,
) -> Stateful<Div> {
    let label: SharedString = label.into();
    let mut el = div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(8.0))
        .rounded(px(ROUNDED_MD))
        .px(px(8.0))
        .py(px(6.0))
        .text_size(px(13.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(if disabled { TEXT_MUTED } else { TEXT_STRONG }))
        .child(label);
    if disabled {
        el = el.opacity(0.55).cursor_not_allowed();
    } else {
        el = el
            .cursor_pointer()
            .hover(|style| style.bg(rgb(mix(ACCENT, SURFACE_1, 0.18))));
    }
    el
}
