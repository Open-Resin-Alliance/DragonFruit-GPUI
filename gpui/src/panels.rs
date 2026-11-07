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
//!   widen its dock; its stored placement survives, as in the source.
//! - **Collapse registration.** The source registers collapsible panels through
//!   context (`useFloatingPanelCollapse`), with `autoCollapse` defaulting to
//!   `true` and a per-panel default expanded state. Every [`PanelDef`] is
//!   collapsible here, with [`PanelDef::collapsed`] and
//!   [`PanelDef::auto_collapse`] carrying the call site's values.
//! - **Collapsed height.** `COLLAPSED_PANEL_HEIGHT` is only the arithmetic
//!   estimate the auto-compact pass subtracts; the source never renders a
//!   48px-tall panel. The header is rendered at its natural height.
//! - **Dock overflow.** `overflowY: auto` / `overflowX: hidden` become
//!   `overflow_y_scroll()` / `overflow_hidden()`; while a drag is in flight
//!   both axes stay visible, as in the source.
//! - **`leftInsetPx` / `rightInsetPx`.** `page.tsx` passes
//!   `TOOL_RAIL_WIDTH_PX` (66) while a model list is on screen and the rail is
//!   a column, and 48 while the layer-slider column is shown (every mode but
//!   printing). [`panel_stack`] derives both from the panel set
//!   ([`PanelStackInsets::for_panels`]) because this port has no `mode`; a
//!   caller that knows better passes [`panel_stack_with`].
//! - **Panel measurement.** `offsetWidth`/`offsetHeight` become the prepaint
//!   bounds of the item wrapper (`on_children_prepainted`).
//! - **`ui::card` radius.** `Card.tsx` uses Tailwind's `rounded-lg` (8px);
//!   `ui::card()` paints `theme::RADIUS_LG` (14px, the CSS custom property).
//!   This module overrides it to 8px. `ui::card_header` and `ui::menu_item`
//!   are no longer used by this module (`card_header` has no leading slot for
//!   the collapse toggle; the menu is `ui::context_menu`).
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
    AnyElement, App, Bounds, BoxShadow, ClickEvent, Div, FontWeight, Hsla, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, SharedString, WeakEntity, Window, div,
    prelude::*, px, rgba, rgb,
};

use crate::DragonFruit;
use crate::icons::icon;
use crate::theme::*;
use crate::ui::{ContextMenuEntry, card, context_menu};

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
/// `FloatingPanelItem`'s inline `borderRadius: 12` — the panel's outer shape.
const CARD_RADIUS: f32 = 12.0;
/// `ui-button !p-2` around a 12px chevron plus its 1px borders.
const COLLAPSE_TOGGLE: f32 = 30.0;
/// `h-3 w-3` on the collapse chevron.
const COLLAPSE_ICON: f32 = 12.0;

/// Which column a panel lives in. Mirrors the source `PanelDock`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelDock {
    Left,
    Right,
    Float,
}

/// Identifies a panel's body so the parent can match it when building content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelKind {
    Models,
    Supports,
    Export,
    Printing,
    Info,
    Transform,
    Orientation,
    Smoothing,
    OrganicCut,
    Arrange,
    Duplicate,
}

/// Where one panel sits. Mirrors the source `PanelPlacement`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelPlacement {
    pub dock: PanelDock,
    /// Position within its dock; floats order their stacking.
    pub order: usize,
    /// Where a floating panel sits, in stack coordinates.
    pub float: Option<(f32, f32)>,
    /// The panel's own width. Docking never changes it; the dock is sized to it.
    pub width: f32,
}

/// The source's `leftInsetPx` / `rightInsetPx` / `railIsColumn` props.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelStackInsets {
    pub left: f32,
    pub right: f32,
    pub rail_is_column: bool,
}

impl Default for PanelStackInsets {
    /// The source's own prop defaults (`0`, `0`, `true`).
    fn default() -> Self {
        Self {
            left: 0.0,
            right: 0.0,
            rail_is_column: true,
        }
    }
}

impl PanelStackInsets {
    /// The values `page.tsx` passes: the rail column's width while a model list
    /// is on screen, and the 48px layer-slider column in every mode but
    /// printing. Derived from the panel set because this port has no `mode`.
    pub fn for_panels(panels: &[PanelDef]) -> Self {
        let models = panels.iter().any(|panel| panel.id == "models");
        let printing = panels.iter().any(|panel| panel.id == "printing");
        Self {
            left: if models { TOOL_RAIL_WIDTH } else { 0.0 },
            right: if models && !printing {
                LAYER_SLIDER_INSET
            } else {
                0.0
            },
            rail_is_column: true,
        }
    }
}

/// A panel the parent wants the stack to show.
///
/// The source's panel entries carry an id, title and body; `kind` is this
/// port's typed handle for the body the parent builds. `dock` is only read the
/// first time a panel is seen; after that its placement lives in
/// [`PanelLayout`], so a tool switch does not erase a move.
pub struct PanelDef {
    pub id: &'static str,
    pub title: SharedString,
    pub kind: PanelKind,
    pub dock: PanelDock,
    /// `CardHeader`'s `right` slot: the panel's own header action (the Models
    /// plus button, the Auto Supports gear).
    pub header_right: Option<AnyElement>,
    /// Body rendered by the parent. `None` renders the header only.
    pub content: Option<AnyElement>,
    /// `useFloatingPanelCollapse(defaultExpanded)` at the call site.
    pub collapsed: bool,
    /// The hook's `autoCollapse` option, `true` unless the call site says so.
    pub auto_collapse: bool,
}

impl PanelDef {
    /// A right-docked panel with no body yet; chain [`Self::dock`] and
    /// [`Self::content`].
    pub fn new(id: &'static str, title: impl Into<SharedString>, kind: PanelKind) -> Self {
        Self {
            id,
            title: title.into(),
            kind,
            dock: PanelDock::Right,
            header_right: None,
            content: None,
            collapsed: false,
            auto_collapse: true,
        }
    }

    /// The dock a panel starts in before the user moves it.
    pub fn dock(mut self, dock: PanelDock) -> Self {
        self.dock = dock;
        self
    }

    /// `CardHeader`'s `right` slot.
    pub fn header_right(mut self, content: impl IntoElement) -> Self {
        self.header_right = Some(content.into_any_element());
        self
    }

    /// The body, built by the parent (usually by matching [`PanelKind`]).
    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    /// Whether the panel starts collapsed (`useFloatingPanelCollapse(false)`).
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Whether the auto-compact pass may collapse this panel.
    pub fn auto_collapse(mut self, auto_collapse: bool) -> Self {
        self.auto_collapse = auto_collapse;
        self
    }
}

#[derive(Clone, Copy)]
struct DragState {
    id: &'static str,
    start: (f32, f32),
    /// Pointer offset inside the panel when the drag began.
    grab: (f32, f32),
    moved: bool,
    /// Pointer delta applied to the panel while dragging.
    offset: (f32, f32),
}

/// The source's right-button gesture tracking: a right drag past 6px suppresses
/// the context menu that would otherwise open on release.
#[derive(Clone, Copy)]
struct RightGesture {
    id: &'static str,
    start: (f32, f32),
    moved: bool,
}

#[derive(Clone, Copy)]
struct DropTarget {
    dock: PanelDock,
    index: usize,
    /// Top of the slot the panel would land in, in stack coordinates.
    slot_top: f32,
    /// The dock's box in stack coordinates, measured when the target was set.
    dock_left: f32,
    dock_width: f32,
    /// The dragged panel's measured height, so the preview is its shape.
    height: f32,
}

struct LayoutState {
    panels: HashMap<&'static str, PanelPlacement>,
    /// The dock each panel was seeded into, for "Reset Window".
    defaults: HashMap<&'static str, PanelDock>,
    /// `useFloatingPanelCollapse`'s `autoCollapse` per panel.
    auto_collapse: HashMap<&'static str, bool>,
    left_width: Option<f32>,
    right_width: Option<f32>,
    /// Panel ids present this frame; stale placements stay but are not laid out.
    visible: Vec<&'static str>,
    /// Measured panel bounds in window coordinates, refreshed at prepaint.
    bounds: HashMap<&'static str, Bounds<Pixels>>,
    /// Placements read from disk before their panel was first seen.
    loaded: HashMap<String, PanelPlacement>,
    /// `{width: 1200, height: 800}` in the source, then the measured window.
    viewport: (f32, f32),
    drag: Option<DragState>,
    right_gesture: Option<RightGesture>,
    drop: Option<DropTarget>,
    /// `(panel_id, client_x, client_y)` in viewport coordinates.
    menu: Option<(&'static str, f32, f32)>,
    collapsed: HashMap<&'static str, bool>,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            panels: HashMap::new(),
            defaults: HashMap::new(),
            auto_collapse: HashMap::new(),
            left_width: None,
            right_width: None,
            visible: Vec::new(),
            bounds: HashMap::new(),
            loaded: HashMap::new(),
            // `React.useState<PanelSize>({ width: 1200, height: 800 })`.
            viewport: (1200.0, 800.0),
            drag: None,
            right_gesture: None,
            drop: None,
            menu: None,
            collapsed: HashMap::new(),
        }
    }
}

/// The panel layout: placements plus dock widths, and the transient
/// interaction state that has to survive a re-render (drag, menu, collapse).
///
/// Cloning is cheap and shares the same state, so the app can keep one in
/// `DragonFruit` while `panel_stack` hands copies to its event handlers.
#[derive(Clone)]
pub struct PanelLayout {
    state: Rc<RefCell<LayoutState>>,
    /// `(is_left, start_x, start_width)` while a dock edge is being dragged.
    dock_resize: Rc<Cell<Option<(bool, f32, f32)>>>,
    /// `isFloatingLayoutPersistenceEnabled()`.
    persistence: Rc<Cell<bool>>,
    storage: Option<PathBuf>,
}

impl Default for PanelLayout {
    fn default() -> Self {
        let storage = storage_path();
        let mut state = LayoutState::default();
        if let Some(path) = storage.as_ref() {
            if let Ok(text) = std::fs::read_to_string(path) {
                apply_loaded(&mut state, &text);
            }
        }
        Self {
            state: Rc::new(RefCell::new(state)),
            dock_resize: Rc::new(Cell::new(None)),
            persistence: Rc::new(Cell::new(true)),
            storage,
        }
    }
}

impl PanelLayout {
    pub fn new() -> Self {
        Self::default()
    }

    /// The panel's current placement, if it has been seen.
    pub fn placement(&self, id: &str) -> Option<PanelPlacement> {
        self.state.borrow().panels.get(id).copied()
    }

    /// The effective width of a dock (a stored width only when the dock holds a
    /// resizable panel, never narrower than its widest panel, clamped 200–520).
    pub fn dock_width(&self, dock: PanelDock) -> f32 {
        effective_dock_width(&self.state.borrow(), dock)
    }

    /// Store a dock width. The drag handler also widens the dock's panels;
    /// this setter is for callers that only want to seed a width.
    pub fn set_dock_width(&self, dock: PanelDock, width: f32) {
        let width = width.clamp(MIN_DOCK_WIDTH, MAX_DOCK_WIDTH);
        {
            let mut state = self.state.borrow_mut();
            match dock {
                PanelDock::Left => state.left_width = Some(width),
                PanelDock::Right => state.right_width = Some(width),
                PanelDock::Float => {}
            }
        }
        self.save();
    }

    /// Move a panel into a dock at `index` (clamped to the dock's length).
    pub fn dock(&self, id: &'static str, dock: PanelDock, index: usize) {
        place_panel(&mut self.state.borrow_mut(), id, dock, index, None);
        self.save();
    }

    /// Move a panel to a floating position in stack coordinates.
    pub fn float(&self, id: &'static str, x: f32, y: f32) {
        place_panel(
            &mut self.state.borrow_mut(),
            id,
            PanelDock::Float,
            0,
            Some((x, y)),
        );
        self.save();
    }

    pub fn is_collapsed(&self, id: &str) -> bool {
        self.state
            .borrow()
            .collapsed
            .get(id)
            .copied()
            .unwrap_or(false)
    }

    pub fn set_collapsed(&self, id: &'static str, collapsed: bool) {
        self.state.borrow_mut().collapsed.insert(id, collapsed);
        self.save();
    }

    /// `isFloatingLayoutPersistenceEnabled()` / `setFloatingLayoutPersistenceEnabled`.
    pub fn set_persistence_enabled(&self, enabled: bool) {
        self.persistence.set(enabled);
        if enabled {
            self.save();
        }
    }

    pub fn persistence_enabled(&self) -> bool {
        self.persistence.get()
    }

    /// Forget every placement, dock width and collapse state, and delete the
    /// stored document (`resetAllWindows`).
    pub fn reset(&self) {
        *self.state.borrow_mut() = LayoutState::default();
        self.dock_resize.set(None);
        if let Some(path) = self.storage.as_ref() {
            let _ = std::fs::remove_file(path);
        }
    }

    /// Give every panel its first placement. Existing placements (including
    /// ones for panels a tool is not showing) are kept, matching the source's
    /// "a tool switch does not erase a move".
    pub fn ensure(&self, panels: &[PanelDef]) {
        let mut state = self.state.borrow_mut();
        state.visible = panels.iter().map(|panel| panel.id).collect();
        for (index, panel) in panels.iter().enumerate() {
            state.defaults.entry(panel.id).or_insert(panel.dock);
            state
                .auto_collapse
                .entry(panel.id)
                .or_insert(panel.auto_collapse);
            state.collapsed.entry(panel.id).or_insert(panel.collapsed);
            if state.panels.contains_key(panel.id) {
                continue;
            }
            let placement = state.loaded.remove(panel.id).unwrap_or(PanelPlacement {
                dock: panel.dock,
                order: index,
                float: None,
                width: base_width(panel.id),
            });
            state.panels.insert(panel.id, placement);
        }
    }

    /// Write the v6 document. `isFloatingLayoutPersistenceEnabled()` gates it.
    fn save(&self) {
        if !self.persistence.get() {
            return;
        }
        let Some(path) = self.storage.as_ref() else {
            return;
        };
        let text = serialize(&self.state.borrow());
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, text);
    }
}

/// `PANEL_WIDTH_OVERRIDES` in the source.
fn base_width(id: &str) -> f32 {
    match id {
        "transform-debug-overlay" => 420.0,
        _ => DEFAULT_PANEL_WIDTH,
    }
}

/// `RESIZABLE_PANELS`: the model list, which the source keys as
/// `prepare-models` in Prepare and `support-models` in Support. This port
/// shows one `models` panel in both modes.
fn is_resizable(id: &str) -> bool {
    id == "models"
}

/// `LOCKED_PANELS`: the model list only while the rail is a column.
fn is_locked(id: &str, rail_is_column: bool) -> bool {
    rail_is_column && id == "models"
}

fn notify(entity: &WeakEntity<DragonFruit>, cx: &mut App) {
    entity.update(cx, |_, cx| cx.notify()).ok();
}

/// Panels in `dock`, in order, excluding `exclude` and any panel the current
/// tool is not showing.
fn dock_ids(
    state: &LayoutState,
    dock: PanelDock,
    exclude: Option<&'static str>,
) -> Vec<&'static str> {
    let mut ids: Vec<&'static str> = state
        .visible
        .iter()
        .copied()
        .filter(|id| Some(*id) != exclude)
        .filter(|id| state.panels.get(*id).map(|p| p.dock) == Some(dock))
        .collect();
    ids.sort_by_key(|id| {
        state
            .panels
            .get(*id)
            .map(|p| p.order)
            .unwrap_or(usize::MAX)
    });
    ids
}

/// The source's `getPanelSize(...).height`, falling back to
/// `DEFAULT_PANEL_HEIGHT` before the first measurement.
fn panel_height(state: &LayoutState, id: &str) -> f32 {
    state
        .bounds
        .get(id)
        .map(|bounds| bounds.size.height.0)
        .unwrap_or(DEFAULT_PANEL_HEIGHT)
}

/// The source's `dockWidth`: the widest panel plus both margins, or a stored
/// drag width when the dock holds a resizable panel, clamped to 200–520.
fn effective_dock_width(state: &LayoutState, dock: PanelDock) -> f32 {
    let stored = match dock {
        PanelDock::Left => state.left_width,
        PanelDock::Right => state.right_width,
        PanelDock::Float => None,
    };
    let ids = dock_ids(state, dock, None);
    let widest = ids
        .iter()
        .filter_map(|id| state.panels.get(*id))
        .map(|placement| placement.width)
        .fold(0.0f32, f32::max);
    let content = (if widest > 0.0 {
        widest
    } else {
        DEFAULT_PANEL_WIDTH
    }) + PANEL_MARGIN * 2.0;
    // Only a dock holding a panel the edge drag can size keeps a stored width;
    // the others stay snug around their panels.
    let resizable = ids.iter().any(|id| is_resizable(id));
    let width = match stored {
        Some(width) if resizable => width.max(content),
        _ => content,
    };
    width.clamp(MIN_DOCK_WIDTH, MAX_DOCK_WIDTH)
}

/// `clampPosition`: keep a floating panel inside the stack, 6px from its edges.
fn clamp_position(position: (f32, f32), size: (f32, f32), bounds: (f32, f32)) -> (f32, f32) {
    let max_x = (bounds.0 - size.0 - PANEL_MARGIN).max(PANEL_MARGIN);
    let max_y = (bounds.1 - size.1 - PANEL_MARGIN).max(PANEL_MARGIN);
    (
        position.0.clamp(PANEL_MARGIN, max_x),
        position.1.clamp(PANEL_MARGIN, max_y),
    )
}

/// `clampToViewport(position, size, { margin: 8 })`, then the stack's own
/// offset, because the menu is painted inside the stack.
fn clamp_menu_position(position: (f32, f32), viewport: (f32, f32)) -> (f32, f32) {
    let window_height = viewport.1 + STACK_TOP;
    let max_x = (viewport.0 - MENU_WIDTH - MENU_EDGE_MARGIN).max(MENU_EDGE_MARGIN);
    let max_y = (window_height - menu_height() - MENU_EDGE_MARGIN).max(MENU_EDGE_MARGIN);
    (
        position.0.clamp(MENU_EDGE_MARGIN, max_x),
        position.1.clamp(MENU_EDGE_MARGIN, max_y) - STACK_TOP,
    )
}

/// The `Window` menu's height, from `ContextMenu.tsx`: `p-1.5` (6px) and a 1px
/// border on both sides, the `mb-1 px-2 py-1` 10px heading, and two
/// `px-2 py-1.5` rows whose 20px icon slot is taller than their 13px label,
/// with `space-y-0.5` between the rows.
fn menu_height() -> f32 {
    const ROW: f32 = 6.0 + 6.0 + 20.0;
    const TITLE: f32 = 4.0 + 4.0 + 10.0 * LINE_HEIGHT_NORMAL + 4.0;
    6.0 * 2.0 + 2.0 + TITLE + ROW * 2.0 + 2.0
}

/// The source's `placePanel`: insert into a dock at `index` and renumber both
/// the target and the source dock; or park a float at the end of the stack.
fn place_panel(
    state: &mut LayoutState,
    id: &'static str,
    dock: PanelDock,
    index: usize,
    float: Option<(f32, f32)>,
) {
    let source = state.panels.get(id).map(|placement| placement.dock);

    if dock == PanelDock::Float {
        let order = state
            .panels
            .values()
            .filter(|placement| placement.dock == PanelDock::Float)
            .map(|placement| placement.order)
            .max()
            .map(|order| order + 1)
            .unwrap_or(0);
        // The source replaces the placement wholesale, so a floated panel
        // returns to its base width.
        state.panels.insert(
            id,
            PanelPlacement {
                dock: PanelDock::Float,
                order,
                float,
                width: base_width(id),
            },
        );
        return;
    }

    let mut ids = dock_ids(state, dock, Some(id));
    let insert_at = index.min(ids.len());
    ids.insert(insert_at, id);
    for (order, panel_id) in ids.iter().enumerate() {
        if let Some(placement) = state.panels.get_mut(*panel_id) {
            placement.dock = dock;
            placement.order = order;
            placement.float = None;
        }
    }

    if let Some(source) = source {
        if source != dock && source != PanelDock::Float {
            let source_ids = dock_ids(state, source, Some(id));
            for (order, panel_id) in source_ids.iter().enumerate() {
                if let Some(placement) = state.panels.get_mut(*panel_id) {
                    placement.order = order;
                }
            }
        }
    }
}

/// Drop a right-button gesture that never saw its release (the pointer left the
/// window), so the tracking overlay cannot outlive it.
fn clear_right_gesture(layout: &PanelLayout, entity: &WeakEntity<DragonFruit>, cx: &mut App) {
    let had = {
        let mut state = layout.state.borrow_mut();
        let had = state.right_gesture.take().is_some();
        if had {
            state.menu = None;
        }
        had
    };
    if had {
        notify(entity, cx);
    }
}

/// The source's pointer-move hit test: the dragged panel's overlap with a dock
/// decides the dock (not the pointer), then the pointer's y picks the slot.
fn drop_target_at(
    state: &LayoutState,
    dragged: &'static str,
    pointer: (f32, f32),
    grab: (f32, f32),
    insets: PanelStackInsets,
) -> Option<DropTarget> {
    let width = state
        .panels
        .get(dragged)
        .map(|placement| placement.width)
        .unwrap_or(DEFAULT_PANEL_WIDTH);
    let panel_left = pointer.0 - grab.0;
    let panel_right = panel_left + width;
    let left_width = effective_dock_width(state, PanelDock::Left);
    let right_width = effective_dock_width(state, PanelDock::Right);
    let window_width = state.viewport.0;
    let left_dock_right = insets.left + left_width;
    let right_dock_left = window_width - insets.right - right_width;
    let left_overlap = panel_right.min(left_dock_right) - panel_left.max(0.0);
    let right_overlap = panel_right.min(window_width) - panel_left.max(right_dock_left);

    let dock = if left_overlap > width / 2.0 {
        PanelDock::Left
    } else if right_overlap > width / 2.0 {
        PanelDock::Right
    } else {
        return None;
    };

    // Panels a tool is not showing have no box, so they take no part in drop
    // maths; the dragged panel is out of its own list.
    let ids: Vec<&'static str> = dock_ids(state, dock, Some(dragged))
        .into_iter()
        .filter(|id| state.bounds.contains_key(id))
        .collect();

    let mut index = 0;
    for id in &ids {
        if let Some(bounds) = state.bounds.get(*id) {
            if pointer.1 > bounds.origin.y.0 + bounds.size.height.0 / 2.0 {
                index += 1;
            }
        }
    }

    let slot_top = if ids.is_empty() {
        PANEL_MARGIN
    } else if index >= ids.len() {
        state
            .bounds
            .get(ids[ids.len() - 1])
            .map(|bounds| {
                (bounds.origin.y.0 + bounds.size.height.0 - STACK_TOP + PANEL_GAP)
                    .max(PANEL_MARGIN)
            })
            .unwrap_or(PANEL_MARGIN)
    } else {
        state
            .bounds
            .get(ids[index])
            .map(|bounds| (bounds.origin.y.0 - STACK_TOP).max(PANEL_MARGIN))
            .unwrap_or(PANEL_MARGIN)
    };

    Some(DropTarget {
        dock,
        index,
        slot_top,
        dock_left: if dock == PanelDock::Left {
            insets.left
        } else {
            right_dock_left
        },
        dock_width: if dock == PanelDock::Left {
            left_width
        } else {
            right_width
        },
        height: panel_height(state, dragged),
    })
}

/// `requestPanelExpand`: collapse other panels in the dock, bottom up, until
/// the dock fits again.
fn request_panel_expand(state: &mut LayoutState, id: &'static str) {
    let Some(placement) = state.panels.get(id).copied() else {
        return;
    };
    if placement.dock == PanelDock::Float {
        return;
    }

    let available = state.viewport.1 - PANEL_MARGIN * 2.0;
    let ids = dock_ids(state, placement.dock, None);
    let mut total: f32 = ids
        .iter()
        .map(|panel_id| panel_height(state, panel_id) + PANEL_GAP)
        .sum();
    if total <= available {
        return;
    }

    for other in ids.iter().rev().filter(|panel_id| **panel_id != id) {
        if total <= available {
            break;
        }
        if !state.auto_collapse.get(*other).copied().unwrap_or(true) {
            continue;
        }
        if state.collapsed.get(*other).copied().unwrap_or(false) {
            continue;
        }
        let height = panel_height(state, other);
        state.collapsed.insert(*other, true);
        total -= (height - COLLAPSED_PANEL_HEIGHT).max(0.0);
    }
}

/// Auto-compact a dock that no longer fits, from the bottom up. The source runs
/// this in an effect after every measurement; here it runs before the frame
/// that lays the panels out, using the previous frame's measurements.
fn auto_compact(state: &mut LayoutState) {
    if state.drag.is_some() {
        return;
    }

    let available = state.viewport.1 - PANEL_MARGIN * 2.0;
    for dock in [PanelDock::Left, PanelDock::Right] {
        let ids = dock_ids(state, dock, None);
        let mut total: f32 = ids
            .iter()
            .map(|id| panel_height(state, id) + PANEL_GAP)
            .sum();
        if total <= available {
            continue;
        }

        for id in ids.iter().rev() {
            if total <= available {
                break;
            }
            if !state.auto_collapse.get(*id).copied().unwrap_or(true) {
                continue;
            }
            if state.collapsed.get(*id).copied().unwrap_or(false) {
                continue;
            }
            let height = panel_height(state, id);
            state.collapsed.insert(*id, true);
            total -= (height - COLLAPSED_PANEL_HEIGHT).max(0.0);
        }
    }
}

/// The collapse chevron: `PanelCollapseToggle` inside `IconButton`
/// (`ui-button ui-button-secondary !p-2`), 30x30 with a 12px chevron.
fn collapse_toggle(
    id: &'static str,
    collapsed: bool,
    layout: &PanelLayout,
    entity: &WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let toggle_layout = layout.clone();
    let toggle_entity = entity.clone();
    div()
        .id(format!("panel-collapse-{id}"))
        .w(px(COLLAPSE_TOGGLE))
        .h(px(COLLAPSE_TOGGLE))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .hover(|style| style.bg(rgb(SURFACE_2)))
        .cursor_pointer()
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
        .child(icon(
            if collapsed {
                "chevron-right"
            } else {
                "chevron-down"
            },
            COLLAPSE_ICON,
            if collapsed { TEXT_MUTED } else { ACCENT },
        ))
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
        ..
    } = def;

    let drag_layout = layout.clone();
    let drag_entity = entity.clone();

    // `CardHeader`: `relative px-2.5 py-2.5 flex items-center`, the title in a
    // `flex items-center gap-2.5 pr-8` block with the toggle first.
    let header = div()
        .relative()
        .px(px(10.0))
        .py(px(10.0))
        .flex()
        .items_center()
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
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                .pr(px(32.0))
                .child(collapse_toggle(id, collapsed, layout, entity))
                .child(
                    div()
                        .text_size(px(FONT_MD))
                        .line_height(px(FONT_MD * LINE_HEIGHT_TIGHT))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child(title),
                ),
        )
        .when_some(header_right, |this, right| {
            this.child(
                div()
                    .absolute()
                    .right(px(10.0))
                    .top(px(0.0))
                    .bottom(px(0.0))
                    .flex()
                    .items_center()
                    .child(right),
            )
        });

    // `Card`: `ui-panel rounded-lg shadow-lg border` on `--border-subtle`.
    let mut card_element = card()
        .rounded(px(CARD_RADIUS))
        .w(px(width))
        .child(header);

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
    panels: Vec<PanelDef>,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let insets = PanelStackInsets::for_panels(&panels);
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
                .rounded(px(CARD_RADIUS))
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

/// The v6 document's home. `localStorage` has no GPUI counterpart; the app's
/// per-user config directory is the closest thing.
fn storage_path() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(
        base.join("DragonFruit")
            .join("floating-panel-layout.json"),
    )
}

fn serialize(state: &LayoutState) -> String {
    // Placements for panels this session has not shown yet are kept: the
    // source's `layout.panels` survives a tool switch, and so must the file.
    let mut merged: Vec<(String, PanelPlacement)> = state
        .loaded
        .iter()
        .map(|(id, placement)| (id.clone(), *placement))
        .collect();
    for (id, placement) in &state.panels {
        match merged.iter_mut().find(|(name, _)| name.as_str() == *id) {
            Some((_, existing)) => *existing = *placement,
            None => merged.push(((*id).to_owned(), *placement)),
        }
    }
    merged.sort_by(|left, right| left.0.cmp(&right.0));

    let mut panels = Vec::with_capacity(merged.len());
    for (id, placement) in merged {
        let mut fields = vec![
            (
                "dock".to_owned(),
                json::Value::Text(
                    match placement.dock {
                        PanelDock::Left => "left",
                        PanelDock::Right => "right",
                        PanelDock::Float => "float",
                    }
                    .to_owned(),
                ),
            ),
            (
                "order".to_owned(),
                json::Value::Number(placement.order as f64),
            ),
            (
                "width".to_owned(),
                json::Value::Number(placement.width as f64),
            ),
        ];
        if let Some((x, y)) = placement.float {
            fields.push((
                "float".to_owned(),
                json::Value::Object(vec![
                    ("x".to_owned(), json::Value::Number(x as f64)),
                    ("y".to_owned(), json::Value::Number(y as f64)),
                ]),
            ));
        }
        panels.push((id, json::Value::Object(fields)));
    }

    let mut docks = Vec::new();
    if let Some(width) = state.left_width {
        docks.push(("leftWidth".to_owned(), json::Value::Number(width as f64)));
    }
    if let Some(width) = state.right_width {
        docks.push(("rightWidth".to_owned(), json::Value::Number(width as f64)));
    }

    let root = json::Value::Object(vec![
        ("version".to_owned(), json::Value::Number(6.0)),
        ("panels".to_owned(), json::Value::Object(panels)),
        ("docks".to_owned(), json::Value::Object(docks)),
    ]);
    let mut out = String::new();
    json::write(&root, &mut out);
    out
}

fn apply_loaded(state: &mut LayoutState, text: &str) {
    let Some(document) = json::parse(text) else {
        return;
    };
    if document.get("version").and_then(|value| value.number()) != Some(6.0) {
        return;
    }

    if let Some(docks) = document.get("docks") {
        state.left_width = docks.get("leftWidth").and_then(|value| value.number());
        state.right_width = docks.get("rightWidth").and_then(|value| value.number());
    }

    let Some(panels) = document.get("panels").and_then(|value| value.object()) else {
        return;
    };
    for (id, placement) in panels {
        let dock = match placement.get("dock").and_then(|value| value.text()) {
            Some("left") => PanelDock::Left,
            Some("right") => PanelDock::Right,
            Some("float") => PanelDock::Float,
            _ => continue,
        };
        let order = placement
            .get("order")
            .and_then(|value| value.number())
            .unwrap_or(0.0)
            .max(0.0) as usize;
        let width = placement
            .get("width")
            .and_then(|value| value.number())
            .unwrap_or_else(|| base_width(id));
        let float = placement
            .get("float")
            .and_then(|value| Some((value.get("x")?.number()?, value.get("y")?.number()?)));
        state.loaded.insert(
            id.clone(),
            PanelPlacement {
                dock,
                order,
                float,
                width,
            },
        );
    }
}

/// Just enough JSON for the one document this module writes: the v6 layout.
mod json {
    #[derive(Clone, Debug, PartialEq)]
    pub enum Value {
        Null,
        Bool(bool),
        Number(f64),
        Text(String),
        Array(Vec<Value>),
        Object(Vec<(String, Value)>),
    }

    impl Value {
        pub fn get(&self, key: &str) -> Option<&Value> {
            match self {
                Value::Object(entries) => {
                    entries.iter().find(|(name, _)| name == key).map(|(_, v)| v)
                }
                _ => None,
            }
        }

        pub fn number(&self) -> Option<f32> {
            match self {
                Value::Number(value) => Some(*value as f32),
                _ => None,
            }
        }

        pub fn text(&self) -> Option<&str> {
            match self {
                Value::Text(text) => Some(text),
                _ => None,
            }
        }

        pub fn object(&self) -> Option<&[(String, Value)]> {
            match self {
                Value::Object(entries) => Some(entries),
                _ => None,
            }
        }
    }

    pub fn parse(input: &str) -> Option<Value> {
        let mut parser = Parser {
            bytes: input.as_bytes(),
            pos: 0,
        };
        let value = parser.value()?;
        parser.skip_ws();
        if parser.pos == parser.bytes.len() {
            Some(value)
        } else {
            None
        }
    }

    pub fn write(value: &Value, out: &mut String) {
        match value {
            Value::Null => out.push_str("null"),
            Value::Bool(true) => out.push_str("true"),
            Value::Bool(false) => out.push_str("false"),
            Value::Number(number) => {
                if number.fract() == 0.0 && number.is_finite() {
                    out.push_str(&format!("{}", *number as i64));
                } else {
                    out.push_str(&format!("{number}"));
                }
            }
            Value::Text(text) => write_string(text, out),
            Value::Array(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    write(item, out);
                }
                out.push(']');
            }
            Value::Object(entries) => {
                out.push('{');
                for (index, (key, item)) in entries.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    write_string(key, out);
                    out.push(':');
                    write(item, out);
                }
                out.push('}');
            }
        }
    }

    fn write_string(text: &str, out: &mut String) {
        out.push('"');
        for character in text.chars() {
            match character {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                control if (control as u32) < 0x20 => {
                    out.push_str(&format!("\\u{:04x}", control as u32));
                }
                other => out.push(other),
            }
        }
        out.push('"');
    }

    struct Parser<'a> {
        bytes: &'a [u8],
        pos: usize,
    }

    impl Parser<'_> {
        fn peek(&self) -> Option<u8> {
            self.bytes.get(self.pos).copied()
        }

        fn bump(&mut self) -> Option<u8> {
            let byte = self.peek()?;
            self.pos += 1;
            Some(byte)
        }

        fn skip_ws(&mut self) {
            while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
                self.pos += 1;
            }
        }

        fn value(&mut self) -> Option<Value> {
            self.skip_ws();
            match self.peek()? {
                b'{' => self.object(),
                b'[' => self.array(),
                b'"' => self.string().map(Value::Text),
                b't' => self.literal(b"true", Value::Bool(true)),
                b'f' => self.literal(b"false", Value::Bool(false)),
                b'n' => self.literal(b"null", Value::Null),
                _ => self.number(),
            }
        }

        fn literal(&mut self, word: &[u8], value: Value) -> Option<Value> {
            if self.bytes.get(self.pos..self.pos + word.len())? == word {
                self.pos += word.len();
                Some(value)
            } else {
                None
            }
        }

        fn object(&mut self) -> Option<Value> {
            self.bump()?;
            let mut entries = Vec::new();
            self.skip_ws();
            if self.peek()? == b'}' {
                self.pos += 1;
                return Some(Value::Object(entries));
            }
            loop {
                self.skip_ws();
                let key = self.string()?;
                self.skip_ws();
                if self.bump()? != b':' {
                    return None;
                }
                let value = self.value()?;
                entries.push((key, value));
                self.skip_ws();
                match self.bump()? {
                    b',' => continue,
                    b'}' => return Some(Value::Object(entries)),
                    _ => return None,
                }
            }
        }

        fn array(&mut self) -> Option<Value> {
            self.bump()?;
            let mut items = Vec::new();
            self.skip_ws();
            if self.peek()? == b']' {
                self.pos += 1;
                return Some(Value::Array(items));
            }
            loop {
                items.push(self.value()?);
                self.skip_ws();
                match self.bump()? {
                    b',' => continue,
                    b']' => return Some(Value::Array(items)),
                    _ => return None,
                }
            }
        }

        fn string(&mut self) -> Option<String> {
            if self.bump()? != b'"' {
                return None;
            }
            let mut bytes: Vec<u8> = Vec::new();
            loop {
                match self.bump()? {
                    b'"' => return String::from_utf8(bytes).ok(),
                    b'\\' => match self.bump()? {
                        b'"' => bytes.push(b'"'),
                        b'\\' => bytes.push(b'\\'),
                        b'/' => bytes.push(b'/'),
                        b'b' => bytes.push(0x08),
                        b'f' => bytes.push(0x0c),
                        b'n' => bytes.push(b'\n'),
                        b'r' => bytes.push(b'\r'),
                        b't' => bytes.push(b'\t'),
                        b'u' => {
                            let digits = self.bytes.get(self.pos..self.pos + 4)?;
                            let digits = std::str::from_utf8(digits).ok()?;
                            let code = u32::from_str_radix(digits, 16).ok()?;
                            self.pos += 4;
                            let character = char::from_u32(code).unwrap_or('\u{fffd}');
                            let mut buffer = [0u8; 4];
                            bytes.extend_from_slice(character.encode_utf8(&mut buffer).as_bytes());
                        }
                        _ => return None,
                    },
                    byte => bytes.push(byte),
                }
            }
        }

        fn number(&mut self) -> Option<Value> {
            let start = self.pos;
            while matches!(
                self.peek(),
                Some(b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
            ) {
                self.pos += 1;
            }
            if self.pos == start {
                return None;
            }
            let text = std::str::from_utf8(&self.bytes[start..self.pos]).ok()?;
            text.parse::<f64>().ok().map(Value::Number)
        }
    }
}
