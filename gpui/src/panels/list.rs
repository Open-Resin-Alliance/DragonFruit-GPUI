//! Floating panel stack: a port of
//! `src/components/layout/FloatingPanelStack.tsx`.
//!
//! Panels live in a left dock, a right dock, or float at a remembered
//! position. Dragging a panel's header is the only way its placement changes;
//! dragging a dock's 6px edge resizes that dock. Every constant below traces to
//! the source file's `PANEL_*` constants or to the classes the source renders.
//!
//! # PORT DEVIATIONS
//!
//! Verified against `FloatingPanelStack.tsx` (1051 lines), `Card.tsx`,
//! `PanelCollapseToggle.tsx`, `ContextMenu.tsx`, `floatingLayoutPreferences.ts`
//! and `globals.css`. Everything not listed here is a 1:1 port.
//!
//! - **Context menu contents.** The source's right-click menu is
//!   `Reset Window` / `Reset Layout` with the heading `Window` (lines
//!   1032-1043); it has no dock entries — "dragging a panel between them is the
//!   only way its placement changes". This port previously offered
//!   Float / Dock left / Dock right; that was invented and is gone.
//! - **Persistence.** The source reads/writes `localStorage` under
//!   `lumenslicer:floating-panel-layout:v6` with a v5 legacy migration and a
//!   `lumenslicer:floating-layout-persistence-changed` event. GPUI has no
//!   `localStorage`, so the same `{version:6, panels, docks}` document is
//!   written to `%APPDATA%/DragonFruit/floating-panel-layout.json` (XDG on
//!   other platforms). There is no v5 data to migrate in this port, so the
//!   legacy branch is omitted. The on/off flag is
//!   [`PanelLayout::set_persistence_enabled`]; `settings.rs`'s
//!   `floating_persistence` draft field is not wired to it yet (that file owns
//!   the draft), so it defaults to the source's default (`true`).
//! - **Debug snapshot event.** `FLOATING_LAYOUT_DEBUG_REQUEST_EVENT` has no
//!   equivalent: there is no DOM event bus in this port.
//! - **`pointer-events-none`.** The source's stack root, docks and drop
//!   indicator are `pointer-events-none` and only panels are
//!   `pointer-events-auto`. GPUI has no such property; the stack root instead
//!   registers no hitbox at all (no handlers on it), so the canvas keeps its
//!   pointer. The transient interactions that need window-wide tracking
//!   (panel drag, dock resize, right-button gesture) are served by a
//!   full-screen transparent overlay that exists only while one is active.
//!   Consequence: a drag stops tracking while the pointer is over the app bar,
//!   which is painted above the stack.
//! - **`z-index`.** `20 + index` / `100` dragging / `120` drop indicator /
//!   `130` menu are paint order here: floats in order, then the dragged panel,
//!   then the indicator, then the menu.
//! - **Transitions.** `transition-colors` on the collapse toggle and menu rows
//!   has no GPUI equivalent; states are drawn settled.
//! - **`tracking-wide`** on the menu heading has no GPUI equivalent (no
//!   `letter-spacing`); the heading is upper-cased like the source's
//!   `uppercase`.
//! - **Menu placement.** `ContextMenu` measures itself before paint and clamps
//!   into the viewport with an 8px margin. GPUI cannot measure an element
//!   before it is built, so the height is computed from the same paddings, line
//!   heights and row metrics ([`menu_height`]). A one-frame difference from the
//!   real box is possible.
//! - **Outside dismissal.** `useOutsideDismiss` listens on `document` and lets
//!   the click through. Here `on_mouse_down_out` on the menu closes it and the
//!   press is consumed by the menu's own hit test.
//! - **Escape / resize / scroll dismissal.** The source dismisses the menu on
//!   Escape and on window resize or scroll. Key routing lives in the shell
//!   (`main.rs`) and this module has no focus handle; a resize re-renders the
//!   menu in place instead of closing it.
//! - **Card ownership.** In React each panel owns its `<Card>`/`<CardHeader>`;
//!   here the stack owns the shell so one place can match `Card.tsx`. Panels
//!   that put content in the header's `right` slot (`absolute right-2.5
//!   top-1/2`) cannot: [`PanelDef`] has no right slot.
//! - **Panel ids.** The source keys 25 ids (`prepare-models`, `support-auto`,
//!   …) and seeds their docks from `DEFAULT_PANEL_DOCKS`. This port's
//!   [`PanelDef`] ids are `models`, `supports`, `export`, `printing`, `info`,
//!   and the seed dock is the [`PanelDef::dock`] the parent passes (kept in the
//!   layout so "Reset Window" can restore it). `RESIZABLE_PANELS` /
//!   `LOCKED_PANELS` / `PANEL_WIDTH_OVERRIDES` are mapped onto those ids:
//!   `models` is both `prepare-models` and `support-models`.
//!   `transform-debug-overlay` has no panel here, but its 420px override is
//!   kept for the id.
//! - **Hidden panels.** The source hides unused panels with `display: none` on
//!   their wrapper and still counts their width in `dockWidth`. A panel this
//!   port is not given at all has no placement in the frame, so it does not
//!   widen its dock; its stored placement survives, as in the source. The
//!   Models card is dropped from the dock the same way while the rail's tile
//!   hides it ([`crate::chrome::rail::models_shown`]).
//! - **Collapse registration.** The source registers collapsible panels through
//!   context (`useFloatingPanelCollapse`), with `autoCollapse` defaulting to
//!   `true` and a per-panel default expanded state. Every [`PanelDef`] here is
//!   collapsible except the Models card in the rail's column layout, which the
//!   source also pins expanded (`collapsible={modelsPanelCollapsible}`, and
//!   `page.tsx:10731` makes that `toolLayout === 'horizontal'`).
//!   [`PanelDef::collapsed`] and [`PanelDef::auto_collapse`] carry the call
//!   site's values. The source's `py-4` header for the non-collapsible case is
//!   not reachable through `ui::card_header_with`, whose header padding is
//!   fixed at 10px.
//! - **Collapsed height.** `COLLAPSED_PANEL_HEIGHT` is only the arithmetic
//!   estimate the auto-compact pass subtracts; the source never renders a
//!   48px-tall panel. The header is rendered at its natural height.
//! - **Dock overflow.** `overflowY: auto` / `overflowX: hidden` become
//!   `overflow_y_scroll()` / `overflow_hidden()`; while a drag is in flight
//!   both axes stay visible, as in the source.
//! - **`leftInsetPx` / `rightInsetPx`.** `page.tsx` passes
//!   `TOOL_RAIL_WIDTH_PX` (66) only while a model list is on screen *and* the
//!   rail is a column, and 48 while the layer-slider column is shown (every
//!   mode but printing). [`panel_stack`] takes the panel set from
//!   [`PanelStackInsets::for_panels`] (this port has no `mode`) and the rail
//!   layout from [`crate::chrome::rail::layout_is_column`]; a caller that knows
//!   better passes [`panel_stack_with`].
//! - **Panel measurement.** `offsetWidth`/`offsetHeight` become the prepaint
//!   bounds of the item wrapper (`on_children_prepainted`).
//! - **`ui::card` / `ui::card_header_with`.** `Card.tsx`'s `rounded-lg` reads
//!   the app's `--radius-lg` (14px), which is what `ui::card()` paints; the
//!   header is the shared `card_header_with` with the collapse toggle in its
//!   leading slot.
//!
//! # Content shape
//!
//! `panel_stack` owns the shell (card, header, collapse chevron, drag, menu and
//! drop indicator) but not the body. The parent builds each body and hands it
//! over in [`PanelDef::content`], matching [`PanelKind`] itself when it does so
//! (`PanelDef::new(id, title, kind).content(body)`). A body is an
//! [`AnyElement`] because panel bodies are heterogeneous and are rebuilt by the
//! parent each frame.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, BoxShadow, ClickEvent, Div, Hsla, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, SharedString, WeakEntity, Window, div,
    prelude::*, px, rgba, rgb,
};

use crate::DragonFruit;
use gpui_core::icons::icon;
use gpui_core::theme::*;
pub use gpui_core::panel_layout::{PanelDef, PanelDock, PanelKind, PanelLayout};
use gpui_core::panel_layout::*;
use gpui_ui::ui::{
    ButtonSize, ButtonSpec, ButtonVariant, ContextMenuEntry, button_with_content, card,
    card_header_with, context_menu,
};

/// `PANEL_MARGIN` in the source.
pub const PANEL_MARGIN: f32 = 6.0;
/// `PANEL_GAP` in the source.
pub const PANEL_GAP: f32 = 6.0;
/// `DEFAULT_PANEL_WIDTH` in the source.
pub const DEFAULT_PANEL_WIDTH: f32 = 320.0;
/// `DEFAULT_PANEL_HEIGHT` in the source.
pub const DEFAULT_PANEL_HEIGHT: f32 = 150.0;
/// `MIN_DOCK_WIDTH` in the source.
pub const MIN_DOCK_WIDTH: f32 = 200.0;
/// `MAX_DOCK_WIDTH` in the source.
pub const MAX_DOCK_WIDTH: f32 = 520.0;
/// `COLLAPSED_PANEL_HEIGHT` in the source: only the auto-compact estimate.
pub const COLLAPSED_PANEL_HEIGHT: f32 = 48.0;

/// `TOOL_RAIL_WIDTH_PX` in `ToolRail.tsx` (`TILE_SIZE_PX` 54 + 12), the
/// source's `leftInsetPx` while the rail is a column.
const TOOL_RAIL_WIDTH: f32 = 66.0;
/// The source's `rightInsetPx` while the layer-slider column is shown.
const LAYER_SLIDER_INSET: f32 = 48.0;
/// `top-[var(--topbar-height)]` on the stack root.
const STACK_TOP: f32 = TOPBAR_HEIGHT;
/// `bottom-0` on the stack root.
const STACK_BOTTOM: f32 = 0.0;
/// The dock edge handle is 6px wide in the source.
const RESIZE_HANDLE: f32 = 6.0;
/// The source starts a drag after 4px of travel (`dx*dx + dy*dy > 16`).
const DRAG_THRESHOLD_SQ: f32 = 16.0;
/// The source drops a right-click once the button travelled 6px (`> 36`).
const RIGHT_DRAG_THRESHOLD_SQ: f32 = 36.0;
/// `ContextMenu`'s `EDGE_MARGIN_PX`.
const MENU_EDGE_MARGIN: f32 = 8.0;
/// `w-48` in `ContextMenu.tsx`.
const MENU_WIDTH: f32 = 192.0;
/// `ui-button !p-2` around a 12px chevron plus its 1px borders.
const COLLAPSE_TOGGLE: f32 = 30.0;
/// `h-3 w-3` on the collapse chevron.
const COLLAPSE_ICON: f32 = 12.0;

/// `PANEL_WIDTH_OVERRIDES` in the source.
fn collapse_toggle(
    id: &'static str,
    collapsed: bool,
    layout: &PanelLayout,
    entity: &WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let toggle_layout = layout.clone();
    let toggle_entity = entity.clone();
    // `PanelCollapseToggle`: the solid `IconButton` at `!p-2` around a 12px
    // chevron; its `!p-2` is why the tile is 30px, not 18.
    let spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Auto)
        .padding_x(8.0)
        .padding_y(8.0, 8.0)
        .width(COLLAPSE_TOGGLE);
    button_with_content(
        format!("panel-collapse-{id}"),
        &spec,
        icon(
            if collapsed {
                "chevron-right"
            } else {
                "chevron-down"
            },
            COLLAPSE_ICON,
            if collapsed { TEXT_MUTED } else { ACCENT },
        ),
    )
        // The header starts a drag on mouse-down; a chevron click is not a
        // drag (the source blocks drags that start on a button).
        .on_mouse_down(
            MouseButton::Left,
            |_: &MouseDownEvent, _: &mut Window, cx: &mut App| {
                cx.stop_propagation();
            },
        )
        .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
            {
                let mut state = toggle_layout.state.borrow_mut();
                let collapsed = state.collapsed.get(id).copied().unwrap_or(false);
                if collapsed {
                    request_panel_expand(&mut state, id);
                }
                state.collapsed.insert(id, !collapsed);
            }
            toggle_layout.save();
            notify(&toggle_entity, cx);
        })
}

/// `Card` + `CardHeader` + the collapse toggle, with the header as drag handle
/// and right-click menu target.
fn panel_card(
    def: PanelDef,
    width: f32,
    collapsed: bool,
    insets: PanelStackInsets,
    layout: &PanelLayout,
    entity: &WeakEntity<DragonFruit>,
) -> AnyElement {
    let PanelDef {
        id,
        title,
        content,
        header_right,
        kind,
        ..
    } = def;

    // `collapsible={modelsPanelCollapsible}`, and `page.tsx:10731` sets it to
    // `toolLayout === 'horizontal'`: only the bar's list has the chevron. In the
    // column the rail's Models tile is the show/hide lever (and dropping the
    // card from the dock is the hide), so the panel is pinned expanded and its
    // header has no leading toggle.
    let collapsible = !(kind == PanelKind::Models && insets.rail_is_column);
    let collapsed = collapsed && collapsible;

    let drag_layout = layout.clone();
    let drag_entity = entity.clone();

    let leading: Option<gpui::AnyElement> = if collapsible {
        Some(collapse_toggle(id, collapsed, layout, entity).into_any_element())
    } else {
        None
    };

    let header = card_header_with(title, leading, header_right)
    .on_mouse_down(
        MouseButton::Left,
        move |event: &MouseDownEvent, _: &mut Window, cx: &mut App| {
            if is_locked(id, insets.rail_is_column) {
                return;
            }
            {
                let mut state = drag_layout.state.borrow_mut();
                state.menu = None;
                state.drop = None;
                let grab = state
                    .bounds
                    .get(id)
                    .map(|bounds| {
                        (
                            event.position.x.0 - bounds.origin.x.0,
                            event.position.y.0 - bounds.origin.y.0,
                        )
                    })
                    .unwrap_or((12.0, 12.0));
                state.drag = Some(DragState {
                    id,
                    start: (event.position.x.0, event.position.y.0),
                    grab,
                    moved: false,
                    offset: (0.0, 0.0),
                });
            }
            notify(&drag_entity, cx);
        },
    );

    // `Card`: `ui-panel rounded-lg shadow-lg border` on `--border-subtle`.
    let mut card_element = card().w(px(width)).child(header);

    if !collapsed {
        if let Some(content) = content {
            card_element = card_element.child(div().w_full().flex().flex_col().child(content));
        }
    }

    card_element.into_any_element()
}

/// The measured wrapper around one card. `on_children_prepainted` reports the
/// card's window bounds, which is how the drop index and drag overlay know
/// where a panel actually is.
fn panel_item(
    def: PanelDef,
    width: f32,
    collapsed: bool,
    insets: PanelStackInsets,
    layout: &PanelLayout,
    entity: &WeakEntity<DragonFruit>,
) -> Div {
    let id = def.id;
    let card_element = panel_card(def, width, collapsed, insets, layout, entity);
    let measure_layout = layout.clone();
    // The source's `FloatingPanelItem` takes the right-button gesture on the
    // whole panel, not just its header, so a right-click on the body opens the
    // `Window` menu too. Inner handlers still run first, as they do there.
    let right_layout = layout.clone();
    let right_entity = entity.clone();
    let up_layout = layout.clone();
    let up_entity = entity.clone();
    div()
        .w(px(width))
        .flex_shrink_0()
        .on_mouse_down(
            MouseButton::Right,
            move |event: &MouseDownEvent, _: &mut Window, cx: &mut App| {
                {
                    let mut state = right_layout.state.borrow_mut();
                    state.drag = None;
                    state.drop = None;
                    state.menu = None;
                    state.right_gesture = Some(RightGesture {
                        id,
                        start: (event.position.x.0, event.position.y.0),
                        moved: false,
                    });
                }
                notify(&right_entity, cx);
            },
        )
        // Fallback for a release that lands before the tracking overlay is
        // painted: the overlay is above the panel, so only one of the two ever
        // sees the release.
        .on_mouse_up(
            MouseButton::Right,
            move |event: &MouseUpEvent, _: &mut Window, cx: &mut App| {
                let mut state = up_layout.state.borrow_mut();
                let Some(gesture) = state.right_gesture.take() else {
                    return;
                };
                if !gesture.moved {
                    state.menu = Some((gesture.id, event.position.x.0, event.position.y.0));
                }
                drop(state);
                notify(&up_entity, cx);
            },
        )
        .on_children_prepainted(
            move |children: Vec<Bounds<Pixels>>, _: &mut Window, _: &mut App| {
                if let Some(bounds) = children.first() {
                    measure_layout
                        .state
                        .borrow_mut()
                        .bounds
                        .insert(id, *bounds);
                }
            },
        )
        .child(card_element)
}

/// A dock column plus its 6px edge handle.
fn render_dock(
    dock: PanelDock,
    ids: Vec<&'static str>,
    width: f32,
    insets: PanelStackInsets,
    state: &LayoutState,
    layout: &PanelLayout,
    entity: &WeakEntity<DragonFruit>,
    defs: &mut HashMap<&'static str, PanelDef>,
) -> AnyElement {
    let dragging = state.drag.is_some();
    // The id is what lets the dock scroll (GPUI keeps scroll state per element).
    let mut column = div()
        .id(match dock {
            PanelDock::Left => "panel-dock-left",
            PanelDock::Right => "panel-dock-right",
            PanelDock::Float => "panel-dock-float",
        })
        .absolute()
        .top(px(0.))
        .bottom(px(0.))
        .w(px(width))
        .p(px(PANEL_MARGIN))
        .flex()
        .flex_col()
        .gap(px(PANEL_GAP));
    column = match dock {
        PanelDock::Left => column.left(px(insets.left)).items_start(),
        PanelDock::Right => column.right(px(insets.right)).items_end(),
        PanelDock::Float => column,
    };
    // `overflowY: drag ? 'visible' : 'auto'`, `overflowX: drag ? 'visible' : 'hidden'`.
    if !dragging {
        column = column.overflow_hidden().overflow_y_scroll();
    }

    let handle_layout = layout.clone();
    let handle_entity = entity.clone();
    let is_left = dock == PanelDock::Left;
    let start_width = width;
    let mut handle = div()
        .absolute()
        .top(px(0.))
        .bottom(px(0.))
        .w(px(RESIZE_HANDLE))
        .cursor_col_resize()
        .on_mouse_down(
            MouseButton::Left,
            move |event: &MouseDownEvent, _: &mut Window, cx: &mut App| {
                handle_layout
                    .dock_resize
                    .set(Some((is_left, event.position.x.0, start_width)));
                // The tracking overlay only exists once this frame is rebuilt.
                notify(&handle_entity, cx);
                cx.stop_propagation();
            },
        );
    handle = match dock {
        PanelDock::Left => handle.right(px(0.)),
        PanelDock::Right => handle.left(px(0.)),
        PanelDock::Float => handle,
    };
    column = column.child(handle);

    for id in ids {
        let panel_width = state
            .panels
            .get(id)
            .map(|placement| placement.width)
            .unwrap_or(DEFAULT_PANEL_WIDTH);
        let collapsed = state.collapsed.get(id).copied().unwrap_or(false);

        if state.drag.filter(|drag| drag.moved).map(|drag| drag.id) == Some(id) {
            // Keep the slot in the flow while the panel is drawn as an overlay.
            let height = panel_height(state, id);
            column = column.child(div().w(px(panel_width)).h(px(height)).flex_shrink_0());
        } else if let Some(def) = defs.remove(id) {
            column = column.child(panel_item(
                def,
                panel_width,
                collapsed,
                insets,
                layout,
                entity,
            ));
        }
    }

    column.into_any_element()
}

/// The source's `Window` menu: `Reset Window` and `Reset Layout`.
fn panel_menu(
    id: &'static str,
    position: (f32, f32),
    viewport: (f32, f32),
    layout: &PanelLayout,
    entity: &WeakEntity<DragonFruit>,
) -> Div {
    let menu_layout = layout.clone();
    let menu_entity = entity.clone();

    let menu = context_menu(
        &[
            ContextMenuEntry::new("reset-window", "Reset Window").with_icon("rotate-ccw"),
            ContextMenuEntry::new("reset-all-windows", "Reset Layout")
                .with_icon("layout-panel-top"),
        ],
        Some("Window".into()),
        move |selected: &str, _: &mut Window, cx: &mut App| {
            match selected {
                "reset-window" => {
                    {
                        let mut state = menu_layout.state.borrow_mut();
                        state.menu = None;
                        let dock = state.defaults.get(id).copied().unwrap_or(PanelDock::Right);
                        place_panel(&mut state, id, dock, usize::MAX, None);
                    }
                    menu_layout.save();
                }
                "reset-all-windows" => {
                    menu_layout.state.borrow_mut().menu = None;
                    menu_layout.reset();
                }
                _ => {}
            }
            notify(&menu_entity, cx);
        },
    );

    let close_layout = layout.clone();
    let close_entity = entity.clone();
    let (x, y) = clamp_menu_position(position, viewport);
    menu
        .absolute()
        .left(px(x))
        .top(px(y))
        .on_mouse_down_out(move |_: &MouseDownEvent, _: &mut Window, cx: &mut App| {
            let had_menu = close_layout.state.borrow_mut().menu.take().is_some();
            if had_menu {
                notify(&close_entity, cx);
            }
        })
}

/// A container for floating UI panels that overlays the canvas. Panels live in
/// a left dock, a right dock, or float at a remembered position; dragging a
/// panel between them is the only way its placement changes.
pub fn panel_stack(
    layout: &PanelLayout,
    mut panels: Vec<PanelDef>,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let mut insets = PanelStackInsets::for_panels(&panels);
    // `leftInsetPx` is `TOOL_RAIL_WIDTH_PX` only while the rail is a column
    // (`page.tsx:10718`: `… && toolLayout === 'vertical' ? TOOL_RAIL_WIDTH_PX :
    // 0`); the bar takes no width from the stack. The layout is the rail's own
    // state, so ask it rather than assuming the column default. The same flag is
    // `LOCKED_PANELS`' condition (`is_locked`): the model list is drag-locked
    // only while it has the column to hide beside.
    let rail_is_column = crate::chrome::rail::layout_is_column();
    insets.rail_is_column = rail_is_column;
    if !rail_is_column {
        insets.left = 0.0;
    }
    // The Models card leaves the dock while the rail's tile hides it — the
    // source's `display: none` (`ModelManagerPanel.tsx:376-378`). It stays in
    // [`PanelLayout`] (placement, width and collapse state), which is the part
    // the window-layout profiles resolve against; only this frame's dock skips
    // it, so no header and no `gap` is left behind.
    panels.retain(|panel| panel.id != "models" || crate::chrome::rail::models_shown());
    panel_stack_with(insets, layout, panels, entity)
}

/// [`panel_stack`] with the source's `leftInsetPx` / `rightInsetPx` /
/// `railIsColumn` supplied by the caller.
pub fn panel_stack_with(
    insets: PanelStackInsets,
    layout: &PanelLayout,
    panels: Vec<PanelDef>,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    layout.ensure(&panels);
    let mut defs: HashMap<&'static str, PanelDef> =
        panels.into_iter().map(|panel| (panel.id, panel)).collect();

    {
        let mut state = layout.state.borrow_mut();
        auto_compact(&mut state);
    }

    let state = layout.state.borrow();
    let left_width = effective_dock_width(&state, PanelDock::Left);
    let right_width = effective_dock_width(&state, PanelDock::Right);
    let left_ids = dock_ids(&state, PanelDock::Left, None);
    let right_ids = dock_ids(&state, PanelDock::Right, None);
    let float_ids = dock_ids(&state, PanelDock::Float, None);
    let drag = state.drag;
    let drop_target = state.drop;
    let menu = state.menu;
    let viewport = state.viewport;

    // The stack root is `pointer-events-none` in the source, so it carries no
    // handlers here: the canvas below keeps its pointer.
    let mut stack = div()
        .absolute()
        .top(px(STACK_TOP))
        .bottom(px(STACK_BOTTOM))
        .left(px(0.))
        .right(px(0.))
        .on_children_prepainted({
            let measure_layout = layout.clone();
            move |_: Vec<Bounds<Pixels>>, window: &mut Window, _: &mut App| {
                let size = window.viewport_size();
                let next = (size.width.0, (size.height.0 - STACK_TOP).max(0.0));
                let mut state = measure_layout.state.borrow_mut();
                if state.viewport != next {
                    state.viewport = next;
                    drop(state);
                    // The floats and the auto-compact pass were clamped against
                    // the previous size; one more frame settles them.
                    window.refresh();
                }
            }
        });

    stack = stack.child(render_dock(
        PanelDock::Left,
        left_ids,
        left_width,
        insets,
        &state,
        layout,
        &entity,
        &mut defs,
    ));
    stack = stack.child(render_dock(
        PanelDock::Right,
        right_ids,
        right_width,
        insets,
        &state,
        layout,
        &entity,
        &mut defs,
    ));

    for (index, id) in float_ids.iter().enumerate() {
        if drag.filter(|drag| drag.moved).map(|drag| drag.id) == Some(*id) {
            continue;
        }
        let Some(def) = defs.remove(*id) else {
            continue;
        };
        let placement = state.panels.get(*id).copied();
        let width = placement
            .map(|placement| placement.width)
            .unwrap_or(DEFAULT_PANEL_WIDTH);
        let collapsed = state.collapsed.get(*id).copied().unwrap_or(false);
        // `clampPosition(placement.float ?? { x: 6, y: 6 + index * 18 }, ...)`.
        let (x, y) = clamp_position(
            placement
                .and_then(|placement| placement.float)
                .unwrap_or((PANEL_MARGIN, PANEL_MARGIN + index as f32 * 18.0)),
            (width, DEFAULT_PANEL_HEIGHT),
            viewport,
        );
        stack = stack.child(
            div()
                .absolute()
                .left(px(x))
                .top(px(y))
                .w(px(width))
                .child(panel_item(def, width, collapsed, insets, layout, &entity)),
        );
    }

    // The dragged panel is painted last so it stays above both docks.
    if let Some(drag) = drag {
        if drag.moved {
            if let Some(def) = defs.remove(drag.id) {
                let width = state
                    .panels
                    .get(drag.id)
                    .map(|placement| placement.width)
                    .unwrap_or(DEFAULT_PANEL_WIDTH);
                let collapsed = state.collapsed.get(drag.id).copied().unwrap_or(false);
                let (x, y) = state
                    .bounds
                    .get(drag.id)
                    .map(|bounds| {
                        (
                            bounds.origin.x.0 + drag.offset.0,
                            bounds.origin.y.0 - STACK_TOP + drag.offset.1,
                        )
                    })
                    .unwrap_or((
                        insets.left + PANEL_MARGIN + drag.offset.0,
                        PANEL_MARGIN + drag.offset.1,
                    ));
                stack = stack.child(
                    div()
                        .absolute()
                        .left(px(x))
                        .top(px(y))
                        .w(px(width))
                        .cursor_grabbing()
                        .child(panel_card(def, width, collapsed, insets, layout, &entity)),
                );
            }
        }
    }

    // The panel's shape where it will land: `rounded-lg`, a 2px dashed accent
    // border mixed 12% toward white, a 12% accent fill and a 4px 18% ring.
    if let Some(drop) = drop_target {
        let width = (drop.dock_width - PANEL_MARGIN * 2.0).max(0.0);
        let border = mix(ACCENT, 0xffffff, 0.88);
        stack = stack.child(
            div()
                .absolute()
                .left(px(drop.dock_left + PANEL_MARGIN))
                .top(px(drop.slot_top))
                .w(px(width))
                .h(px(drop.height))
                .rounded(px(ROUNDED_LG))
                .border(px(2.0))
                .border_dashed()
                .border_color(rgb(border))
                .bg(rgba((ACCENT << 8) | 0x1f))
                .shadow(vec![
                    BoxShadow::new(px(0.), px(0.), Hsla::from(rgba((ACCENT << 8) | 0x2e)))
                        .spread_radius(px(4.0)),
                ]),
        );
    }

    // Window-wide tracking for the gestures the source listens for on `window`:
    // panel drag, dock resize and the right-button gesture. It exists only
    // while one is in flight, so it never swallows a canvas click.
    if drag.is_some() || layout.dock_resize.get().is_some() || state.right_gesture.is_some() {
        stack = stack.child(
            div()
                .absolute()
                .top(px(0.))
                .bottom(px(0.))
                .left(px(0.))
                .right(px(0.))
                .on_mouse_down(MouseButton::Left, {
                    let guard_layout = layout.clone();
                    let guard_entity = entity.clone();
                    move |_: &MouseDownEvent, _: &mut Window, cx: &mut App| {
                        clear_right_gesture(&guard_layout, &guard_entity, cx);
                    }
                })
                .on_mouse_down(MouseButton::Right, {
                    let guard_layout = layout.clone();
                    let guard_entity = entity.clone();
                    move |_: &MouseDownEvent, _: &mut Window, cx: &mut App| {
                        clear_right_gesture(&guard_layout, &guard_entity, cx);
                    }
                })
                .on_mouse_move({
                    let move_layout = layout.clone();
                    let move_entity = entity.clone();
                    move |event: &MouseMoveEvent, window: &mut Window, cx: &mut App| {
                        let size = window.viewport_size();
                        let next_viewport = (size.width.0, (size.height.0 - STACK_TOP).max(0.0));
                        let mut state = move_layout.state.borrow_mut();
                        let mut changed = false;
                        if state.viewport != next_viewport {
                            state.viewport = next_viewport;
                        }

                        if let Some((is_left, start_x, start_width)) = move_layout.dock_resize.get()
                        {
                            let delta = event.position.x.0 - start_x;
                            let width = (start_width + if is_left { delta } else { -delta })
                                .clamp(MIN_DOCK_WIDTH, MAX_DOCK_WIDTH);
                            let dock = if is_left {
                                PanelDock::Left
                            } else {
                                PanelDock::Right
                            };
                            // The drag sets the panels' own width; the dock
                            // follows them, and can only widen past a panel's
                            // natural width.
                            let mut widest = 0.0f32;
                            for id in dock_ids(&state, dock, None) {
                                if !is_resizable(id) {
                                    continue;
                                }
                                let next_width =
                                    (width - PANEL_MARGIN * 2.0).max(base_width(id));
                                widest = widest.max(next_width);
                                if let Some(placement) = state.panels.get_mut(id) {
                                    placement.width = next_width;
                                }
                            }
                            let dock_width = if widest > 0.0 {
                                widest + PANEL_MARGIN * 2.0
                            } else {
                                width
                            };
                            match dock {
                                PanelDock::Left => state.left_width = Some(dock_width),
                                PanelDock::Right => state.right_width = Some(dock_width),
                                PanelDock::Float => {}
                            }
                            changed = true;
                        }

                        if let Some(mut drag) = state.drag.take() {
                            let dx = event.position.x.0 - drag.start.0;
                            let dy = event.position.y.0 - drag.start.1;
                            if !drag.moved && dx * dx + dy * dy >= DRAG_THRESHOLD_SQ {
                                drag.moved = true;
                            }
                            if drag.moved {
                                drag.offset = (dx, dy);
                                state.drop = drop_target_at(
                                    &state,
                                    drag.id,
                                    (event.position.x.0, event.position.y.0),
                                    drag.grab,
                                    insets,
                                );
                            }
                            state.drag = Some(drag);
                            changed |= drag.moved;
                        }

                        if let Some(mut gesture) = state.right_gesture.take() {
                            let dx = event.position.x.0 - gesture.start.0;
                            let dy = event.position.y.0 - gesture.start.1;
                            if dx * dx + dy * dy > RIGHT_DRAG_THRESHOLD_SQ {
                                gesture.moved = true;
                            }
                            state.right_gesture = Some(gesture);
                        }

                        drop(state);
                        if changed {
                            notify(&move_entity, cx);
                        }
                    }
                })
                .on_mouse_up(MouseButton::Left, {
                    let up_layout = layout.clone();
                    let up_entity = entity.clone();
                    move |event: &MouseUpEvent, window: &mut Window, cx: &mut App| {
                        let size = window.viewport_size();
                        let viewport = (size.width.0, (size.height.0 - STACK_TOP).max(0.0));
                        let mut changed = false;
                        {
                            let mut state = up_layout.state.borrow_mut();
                            if up_layout.dock_resize.get().is_some() {
                                up_layout.dock_resize.set(None);
                                changed = true;
                            }

                            if let Some(drag) = state.drag.take() {
                                let target = state.drop.take();
                                if drag.moved {
                                    if let Some(target) = target {
                                        place_panel(
                                            &mut state,
                                            drag.id,
                                            target.dock,
                                            target.index,
                                            None,
                                        );
                                    } else {
                                        let width = state
                                            .panels
                                            .get(drag.id)
                                            .map(|placement| placement.width)
                                            .unwrap_or(DEFAULT_PANEL_WIDTH);
                                        let height = panel_height(&state, drag.id);
                                        let float = clamp_position(
                                            (
                                                event.position.x.0 - drag.grab.0,
                                                event.position.y.0 - STACK_TOP - drag.grab.1,
                                            ),
                                            (width, height),
                                            viewport,
                                        );
                                        place_panel(
                                            &mut state,
                                            drag.id,
                                            PanelDock::Float,
                                            0,
                                            Some(float),
                                        );
                                    }
                                    changed = true;
                                }
                            }
                        }
                        if changed {
                            up_layout.save();
                            notify(&up_entity, cx);
                        }
                    }
                })
                .on_mouse_up(MouseButton::Right, {
                    let up_layout = layout.clone();
                    let up_entity = entity.clone();
                    move |event: &MouseUpEvent, _: &mut Window, cx: &mut App| {
                        let mut state = up_layout.state.borrow_mut();
                        let Some(gesture) = state.right_gesture.take() else {
                            return;
                        };
                        if !gesture.moved {
                            state.menu =
                                Some((gesture.id, event.position.x.0, event.position.y.0));
                        }
                        drop(state);
                        notify(&up_entity, cx);
                    }
                }),
        );
    }

    if let Some((id, x, y)) = menu {
        stack = stack.child(panel_menu(id, (x, y), viewport, layout, &entity));
    }

    drop(state);
    stack
}

// ── persistence ─────────────────────────────────────────────────────────────

