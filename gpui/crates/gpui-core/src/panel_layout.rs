//! The floating panel layout: placements, dock widths and persistence.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use gpui::{AnyElement, App, Bounds, IntoElement, Pixels, SharedString, WeakEntity};
use crate::theme::LINE_HEIGHT_NORMAL;

const TOOL_RAIL_WIDTH: f32 = 66.0;
pub const MIN_DOCK_WIDTH: f32 = 200.0;
pub const MAX_DOCK_WIDTH: f32 = 520.0;
pub const DEFAULT_PANEL_WIDTH: f32 = 320.0;
pub const DEFAULT_PANEL_HEIGHT: f32 = 150.0;
pub const PANEL_MARGIN: f32 = 6.0;
pub const PANEL_GAP: f32 = 6.0;
pub const COLLAPSED_PANEL_HEIGHT: f32 = 48.0;
const STACK_TOP: f32 = crate::theme::TOPBAR_HEIGHT;
const STACK_BOTTOM: f32 = 0.0;
const MENU_WIDTH: f32 = 192.0;
const MENU_EDGE_MARGIN: f32 = 8.0;
const RESIZE_HANDLE: f32 = 6.0;
const DRAG_THRESHOLD_SQ: f32 = 16.0;
const RIGHT_DRAG_THRESHOLD_SQ: f32 = 36.0;
const COLLAPSE_TOGGLE: f32 = 30.0;
const COLLAPSE_ICON: f32 = 12.0;
const CARD_RADIUS: f32 = crate::theme::ROUNDED_LG;
const LAYER_SLIDER_INSET: f32 = 48.0;

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
pub struct DragState {
    pub id: &'static str,
    pub start: (f32, f32),
    /// Pointer offset inside the panel when the drag began.
    pub grab: (f32, f32),
    pub moved: bool,
    /// Pointer delta applied to the panel while dragging.
    pub offset: (f32, f32),
}

/// The source's right-button gesture tracking: a right drag past 6px suppresses
/// the context menu that would otherwise open on release.
#[derive(Clone, Copy)]
pub struct RightGesture {
    pub id: &'static str,
    pub start: (f32, f32),
    pub moved: bool,
}

#[derive(Clone, Copy)]
pub struct DropTarget {
    pub dock: PanelDock,
    pub index: usize,
    /// Top of the slot the panel would land in, in stack coordinates.
    pub slot_top: f32,
    /// The dock's box in stack coordinates, measured when the target was set.
    pub dock_left: f32,
    pub dock_width: f32,
    /// The dragged panel's measured height, so the preview is its shape.
    pub height: f32,
}

pub struct LayoutState {
    pub panels: HashMap<&'static str, PanelPlacement>,
    /// The dock each panel was seeded into, for "Reset Window".
    pub defaults: HashMap<&'static str, PanelDock>,
    /// `useFloatingPanelCollapse`'s `autoCollapse` per panel.
    pub auto_collapse: HashMap<&'static str, bool>,
    pub left_width: Option<f32>,
    pub right_width: Option<f32>,
    /// Panel ids present this frame; stale placements stay but are not laid out.
    pub visible: Vec<&'static str>,
    /// Measured panel bounds in window coordinates, refreshed at prepaint.
    pub bounds: HashMap<&'static str, Bounds<Pixels>>,
    /// Placements read from disk before their panel was first seen.
    pub loaded: HashMap<String, PanelPlacement>,
    /// `{width: 1200, height: 800}` in the source, then the measured window.
    pub viewport: (f32, f32),
    pub drag: Option<DragState>,
    pub right_gesture: Option<RightGesture>,
    pub drop: Option<DropTarget>,
    /// `(panel_id, client_x, client_y)` in viewport coordinates.
    pub menu: Option<(&'static str, f32, f32)>,
    pub collapsed: HashMap<&'static str, bool>,
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
    pub state: Rc<RefCell<LayoutState>>,
    /// `(is_left, start_x, start_width)` while a dock edge is being dragged.
    pub dock_resize: Rc<Cell<Option<(bool, f32, f32)>>>,
    /// `isFloatingLayoutPersistenceEnabled()`.
    pub persistence: Rc<Cell<bool>>,
    pub storage: Option<PathBuf>,
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
    pub fn save(&self) {
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

pub fn base_width(id: &str) -> f32 {
    match id {
        "transform-debug-overlay" => 420.0,
        _ => DEFAULT_PANEL_WIDTH,
    }
}

/// `RESIZABLE_PANELS`: the model list, which the source keys as
/// `prepare-models` in Prepare and `support-models` in Support. This port
/// shows one `models` panel in both modes.
pub fn is_resizable(id: &str) -> bool {
    id == "models"
}

/// `LOCKED_PANELS`: the model list only while the rail is a column.
pub fn is_locked(id: &str, rail_is_column: bool) -> bool {
    rail_is_column && id == "models"
}

pub fn notify<D: 'static>(entity: &WeakEntity<D>, cx: &mut App) {
    entity.update(cx, |_, cx| cx.notify()).ok();
}

/// Panels in `dock`, in order, excluding `exclude` and any panel the current
/// tool is not showing.
pub fn dock_ids(
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
pub fn panel_height(state: &LayoutState, id: &str) -> f32 {
    state
        .bounds
        .get(id)
        .map(|bounds| bounds.size.height.0)
        .unwrap_or(DEFAULT_PANEL_HEIGHT)
}

/// The source's `dockWidth`: the widest panel plus both margins, or a stored
/// drag width when the dock holds a resizable panel, clamped to 200–520.
pub fn effective_dock_width(state: &LayoutState, dock: PanelDock) -> f32 {
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
pub fn clamp_position(position: (f32, f32), size: (f32, f32), bounds: (f32, f32)) -> (f32, f32) {
    let max_x = (bounds.0 - size.0 - PANEL_MARGIN).max(PANEL_MARGIN);
    let max_y = (bounds.1 - size.1 - PANEL_MARGIN).max(PANEL_MARGIN);
    (
        position.0.clamp(PANEL_MARGIN, max_x),
        position.1.clamp(PANEL_MARGIN, max_y),
    )
}

/// `clampToViewport(position, size, { margin: 8 })`, then the stack's own
/// offset, because the menu is painted inside the stack.
pub fn clamp_menu_position(position: (f32, f32), viewport: (f32, f32)) -> (f32, f32) {
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
pub fn menu_height() -> f32 {
    const ROW: f32 = 6.0 + 6.0 + 20.0;
    const TITLE: f32 = 4.0 + 4.0 + 10.0 * LINE_HEIGHT_NORMAL + 4.0;
    6.0 * 2.0 + 2.0 + TITLE + ROW * 2.0 + 2.0
}

/// The source's `placePanel`: insert into a dock at `index` and renumber both
/// the target and the source dock; or park a float at the end of the stack.
pub fn place_panel(
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
pub fn clear_right_gesture<D: 'static>(layout: &PanelLayout, entity: &WeakEntity<D>, cx: &mut App) {
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
pub fn drop_target_at(
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
pub fn request_panel_expand(state: &mut LayoutState, id: &'static str) {
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
pub fn auto_compact(state: &mut LayoutState) {
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

pub fn storage_path() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(
        base.join("DragonFruit")
            .join("floating-panel-layout.json"),
    )
}

pub fn serialize(state: &LayoutState) -> String {
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

pub fn apply_loaded(state: &mut LayoutState, text: &str) {
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

/// The v6 document's home. `localStorage` has no GPUI counterpart; the app's
/// per-user config directory is the closest thing.

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
