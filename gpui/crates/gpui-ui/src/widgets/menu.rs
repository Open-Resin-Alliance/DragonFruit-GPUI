//! Context menus, menu rows and their nested flyouts.

use gpui::{
    App, Bounds, Div, ElementId, FontWeight, Pixels, SharedString, Stateful, Window, div,
    prelude::*, px, relative, rgb,
};

use gpui_core::icons::icon;
use gpui_core::theme::*;

use std::rc::Rc;

use super::constants::*;

/// `EDGE_MARGIN_PX` in `ContextMenu.tsx`: a flyout keeps this much clear of the
/// window edge before it considers flipping.
const EDGE_MARGIN: f32 = 8.0;

/// `SubmenuFlyout`'s `ml-1` / `mr-1`: the gap between a row and its flyout.
const FLYOUT_GAP: f32 = 4.0;

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
    /// Nested flyout. The entry runs no action itself; its children are the
    /// rows of a flyout that opens beside it on hover.
    pub children: Vec<ContextMenuEntry>,
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
            children: Vec::new(),
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

    /// Make this entry a flyout parent: its children become the flyout's rows
    /// and the id itself is never passed to the selection callback.
    pub fn with_children(mut self, children: impl IntoIterator<Item = ContextMenuEntry>) -> Self {
        self.children = children.into_iter().collect();
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
/// it wherever the caller places it. An entry with `children` draws the
/// `ChevronRight` trailing slot and, while its row is hovered, a flyout beside
/// it — the same surface and the same `MenuRow` implementation, nested to any
/// depth.
///
/// `context_menu` and [`context_menu_sized`] open those flyouts to the right;
/// [`context_menu_placed`] takes the side [`flyout_side`] computed from measured
/// bounds, and [`flyout_side`] is the source's flip rule as a pure function.
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
    context_menu_placed(entries, title, width, FlyoutSide::Right, on_select)
}

/// [`context_menu_sized`] with the side every flyout opens on.
///
/// GPUI has no measure pass in an element builder (see "Measurement" in
/// `widgets/mod.rs`), so the side cannot be derived here. A caller that knows
/// the menu's window position computes it with [`flyout_side`]; without one the
/// flyouts open to the right, as the source does before its first layout.
pub fn context_menu_placed(
    entries: &[ContextMenuEntry],
    title: Option<SharedString>,
    width: f32,
    flyout_side: FlyoutSide,
    on_select: impl Fn(&str, &mut Window, &mut App) + 'static,
) -> Div {
    let on_select: Selection = Rc::new(on_select);
    let base = SharedString::from("context-menu");

    let mut menu = menu_surface(width);

    if let Some(title) = title {
        menu = menu.child(menu_title(title));
    }

    menu.child(menu_rows(entries, &base, width, flyout_side, &on_select))
}

/// Which side of its row a submenu flyout opens on.
///
/// `ContextMenu.tsx` measures the flyout and flips it when it would leave the
/// viewport; [`flyout_side`] is that rule. The source rotates the chevron when
/// it flips, and GPUI has no element rotation, so the port swaps
/// `chevron-right` for `chevron-left`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlyoutSide {
    Right,
    Left,
}

/// The source's `SubmenuFlyout` side rule.
///
/// A flyout opens to the right of its row and flips left when it would cross
/// the window's right edge with `EDGE_MARGIN` to spare *and* the row's left side
/// can hold it. `menu` supplies the flyout's width — the source reuses
/// `widthClassName` for the menu and every flyout — and `window` the viewport.
/// Mirrors `left = flyout.right > innerWidth - EDGE_MARGIN_PX && row.left -
/// flyout.width >= EDGE_MARGIN_PX`: the right-edge test includes the `ml-1` gap,
/// the left-edge test does not.
pub fn flyout_side(
    row: Bounds<Pixels>,
    menu: Bounds<Pixels>,
    window: Bounds<Pixels>,
) -> FlyoutSide {
    let width = menu.size.width;
    let overflows_right = row.right() + px(FLYOUT_GAP) + width > window.right() - px(EDGE_MARGIN);
    let fits_left = row.left() - width >= window.left() + px(EDGE_MARGIN);

    if overflows_right && fits_left {
        FlyoutSide::Left
    } else {
        FlyoutSide::Right
    }
}

/// A menu's selection callback, shared with every flyout row.
type Selection = Rc<dyn Fn(&str, &mut Window, &mut App)>;

/// `ContextMenu`'s `space-y-0.5` row stack: separators, rows and flyouts.
///
/// `base` keys the rows' element ids; a flyout appends its index so a child and
/// an ancestor with the same id never collide.
fn menu_rows(
    entries: &[ContextMenuEntry],
    base: &SharedString,
    width: f32,
    side: FlyoutSide,
    on_select: &Selection,
) -> Div {
    let mut rows = div().flex().flex_col().gap(px(2.0));

    for (index, entry) in entries.iter().enumerate() {
        if entry.starts_group && index > 0 {
            rows = rows.child(div().my(px(4.0)).h(px(1.0)).w_full().bg(rgb(BORDER_SUBTLE)));
        }

        let id = ElementId::named_usize(base.clone(), index);

        if entry.children.is_empty() {
            rows = rows.child(entry_row(id, entry, None, on_select));
            continue;
        }

        // The source wraps a flyout parent in `relative` and mounts the flyout
        // while the row is the open submenu.
        let group: SharedString = format!("{id}-submenu").into();
        rows = rows.child(
            div()
                .relative()
                .group(group.clone())
                .child(entry_row(id, entry, Some(side), on_select))
                .child(
                    flyout(entry, base, index, width, side, on_select)
                        .group_hover(group.clone(), |style| style.opacity(1.0))
                        // The source keeps a flyout open while the pointer is
                        // inside it (a 120 ms close delay). `group_hover` ends
                        // at the row's bounds, so the flyout also reveals
                        // itself while it is hovered — that bridges both the
                        // `ml-1` gap and its own body.
                        .hover(|style| style.opacity(1.0)),
                ),
        );
    }

    rows
}

/// One `SubmenuFlyout`: the menu's own surface and rows, anchored beside the
/// row it belongs to (`absolute top-0 left-full ml-1` / `right-full mr-1`) and
/// hidden until that row is hovered.
///
/// The source keeps an `openSubmenuId` with a 120 ms close delay and portals
/// the flyout; the port has no submenu state and no portal, so the flyout is a
/// transparent child of its row that `group_hover` reveals, plus its own
/// `hover` so the pointer can move into it. GPUI only hit-tests laid-out
/// elements, so the closed flyout has to stay in the tree to be enterable.
fn flyout(
    entry: &ContextMenuEntry,
    base: &SharedString,
    index: usize,
    width: f32,
    side: FlyoutSide,
    on_select: &Selection,
) -> Div {
    let child_base: SharedString = format!("{base}-{index}").into();

    let flyout = menu_surface(width)
        .absolute()
        .top(px(0.0))
        .opacity(0.0)
        .child(menu_rows(
            &entry.children,
            &child_base,
            width,
            side,
            on_select,
        ));

    match side {
        FlyoutSide::Right => flyout.left(relative(1.0)).ml(px(FLYOUT_GAP)),
        FlyoutSide::Left => flyout.right(relative(1.0)).mr(px(FLYOUT_GAP)),
    }
}

/// `ContextMenu.tsx`'s `MenuRow`, shared by the menu and every flyout.
///
/// `disclosure` is `Some(side)` for a row whose `children` opened a flyout: the
/// source's parent branch draws the chevron, takes `disabled` from the children
/// alone, and passes no `checked`, `iconBare`, `danger` or `trailing`.
fn entry_row(
    id: ElementId,
    entry: &ContextMenuEntry,
    disclosure: Option<FlyoutSide>,
    on_select: &Selection,
) -> Stateful<Div> {
    let has_children = disclosure.is_some();
    let disabled = if has_children {
        entry.children.iter().all(|child| child.disabled)
    } else {
        entry.disabled
    };
    let checked = entry.checked && !has_children;
    let danger = entry.danger && !has_children;
    let bare = entry.icon_bare && !checked && !has_children;
    let trailing = if has_children {
        None
    } else {
        entry.trailing.as_ref()
    };

    let tone = if disabled {
        TEXT_MUTED
    } else if danger {
        DANGER
    } else {
        TEXT_STRONG
    };
    let hover_fill = if danger {
        mix(DANGER, SURFACE_1, 0.10)
    } else {
        mix(ACCENT, SURFACE_1, 0.18)
    };

    let mut row = div()
        .id(id)
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
    let mut slot = div()
        .flex()
        .items_center()
        .justify_center()
        .w(px(20.0))
        .h(px(20.0))
        .opacity(if disabled { 0.8 } else { 1.0 });
    if !bare {
        slot = slot
            .rounded(px(ROUNDED_BARE))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1));
    }
    if checked {
        let check_color = if disabled { TEXT_MUTED } else { ACCENT };
        slot = slot.child(icon("check", 14.0, check_color));
    } else if let Some(name) = &entry.icon {
        slot = slot.child(icon(name.as_ref(), 14.0, tone));
    }
    row = row.child(slot);

    // `flex-1` only when there is trailing content, the chevron included; the
    // label itself is not truncated by the source.
    let mut label = div().child(entry.label.clone());
    if trailing.is_some() || has_children {
        label = label.flex_1();
    }
    row = row.child(label);

    if let Some(trailing) = trailing {
        row = row.child(div().child(trailing.clone()));
    }

    if let Some(side) = disclosure {
        // `ChevronRight className="h-4 w-4 shrink-0 opacity-60"`.
        row = row.child(div().flex_shrink_0().opacity(0.6).child(icon(
            chevron_icon(side),
            16.0,
            tone,
        )));
    }

    if disabled {
        row.opacity(0.55).cursor_not_allowed()
    } else {
        row = row
            .cursor_pointer()
            .hover(move |style| style.bg(rgb(hover_fill)));
        // A parent row's click opens its flyout in the source; hover already
        // does that here, and a parent's id is never selected.
        if !has_children {
            let callback = on_select.clone();
            let entry_id = entry.id.clone();
            row = row.on_click(move |_, window, cx| {
                callback(entry_id.as_ref(), window, cx);
            });
        }
        row
    }
}

/// The flipped-side chevron. The source rotates `ChevronRight` 180°; GPUI has
/// no element rotation, so the mirrored icon stands in.
fn chevron_icon(side: FlyoutSide) -> &'static str {
    match side {
        FlyoutSide::Right => "chevron-right",
        FlyoutSide::Left => "chevron-left",
    }
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

#[cfg(test)]
mod tests {
    use super::{FlyoutSide, flyout_side};
    use gpui::{Bounds, Size, point, px};

    /// A `Bounds` literal for the pure placement rule.
    fn bounds(x: f32, y: f32, width: f32, height: f32) -> Bounds<gpui::Pixels> {
        Bounds::new(point(px(x), px(y)), Size::new(px(width), px(height)))
    }

    #[test]
    fn flyout_opens_right_when_it_fits() {
        let window = bounds(0.0, 0.0, 1280.0, 800.0);
        // The source's default `w-48` menu; the row is the content width left
        // by its `p-1.5` (192 - 12).
        let menu = bounds(100.0, 100.0, 192.0, 392.0);
        let row = bounds(106.0, 120.0, 180.0, 30.0);

        // 286 + 4 + 192 = 482, well clear of 1280 - 8.
        assert_eq!(flyout_side(row, menu, window), FlyoutSide::Right);
    }

    #[test]
    fn flyout_flips_left_when_the_right_edge_would_overflow() {
        let window = bounds(0.0, 0.0, 1280.0, 800.0);
        let menu = bounds(1080.0, 100.0, 192.0, 392.0);
        let row = bounds(1086.0, 120.0, 180.0, 30.0);

        // 1266 + 4 + 192 = 1462 > 1272; the left side has 1086 - 192 = 894.
        assert_eq!(flyout_side(row, menu, window), FlyoutSide::Left);
    }

    #[test]
    fn flyout_keeps_the_right_side_when_neither_side_fits() {
        let window = bounds(0.0, 0.0, 400.0, 800.0);
        let menu = bounds(100.0, 100.0, 192.0, 392.0);
        let row = bounds(106.0, 120.0, 180.0, 30.0);

        // 286 + 4 + 192 = 482 > 392, and 106 - 192 is left of the margin, so
        // the source leaves the flyout on the right rather than off-screen.
        assert_eq!(flyout_side(row, menu, window), FlyoutSide::Right);
    }

    #[test]
    fn flyout_width_follows_the_menu_width() {
        let window = bounds(0.0, 0.0, 1200.0, 800.0);
        // Far enough right that the left side clears the margin, so the flip can
        // actually happen (the "neither side fits" rule otherwise keeps Right).
        let row = bounds(800.0, 120.0, 180.0, 30.0);
        let narrow = bounds(50.0, 100.0, 192.0, 392.0);
        let wide = bounds(50.0, 100.0, 256.0, 392.0);

        // 980 + 4 + 192 = 1176 <= 1192; 980 + 4 + 256 = 1240 > 1192.
        assert_eq!(flyout_side(row, narrow, window), FlyoutSide::Right);
        assert_eq!(flyout_side(row, wide, window), FlyoutSide::Left);
    }
}
