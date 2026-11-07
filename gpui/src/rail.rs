//! The tool rail, ported from `src/components/controls/ToolRail.tsx` and
//! `src/components/controls/toolRailEntries.ts`.
//!
//! Two layouts, chosen from the rail's own right-click menu: the docked column
//! down the left edge (54px wide, tiles 54x54) and the bar centred under the app
//! bar. The column carries the Models panel toggle; the bar does not, because the
//! bar's caller in the React app passes no toggle there.
//!
//! # State the parent does not carry
//!
//! `tool_rail` is handed only the app mode, the active tool index, and the
//! entity. Four pieces of the React component therefore live in this module:
//!
//! - the layout preference (`ToolLayout` in the React app, persisted in settings),
//! - the Models panel's visibility (`modelsPanelVisible`), mirrored into the
//!   parent's `collapsed` map — the only panel-visibility lever `PanelLayout`
//!   exposes,
//! - the Support panel mode (`supportRailMode`) and the Hollowing tool, which the
//!   source keeps independent of Prepare's `transformMode`, and
//! - the two Support view toggles (`interiorView`, line/full support drawing),
//!   which the parent has no fields for.
//!
//! `active_tool` is the parent's Prepare tool only. Support's panel mode and the
//! Hollowing tool are rail-local, so selecting a panel never overwrites the
//! Prepare tool the user will return to (the source's per-workspace tool memory).
//!
//! # PORT DEVIATIONS
//!
//! Diffed against `ToolRail.tsx`, `toolRailEntries.ts`, `Tooltip.tsx`,
//! `ContextMenu.tsx`, and the `ToolRail` call site in `app/page.tsx`.
//!
//! Platform gaps (GPUI cannot match):
//! - No CSS transitions: `transition-[color,background-color,border-color,filter,
//!   box-shadow] duration-150` and the layout fold animation (`element.animate`,
//!   240ms, 8ms stagger, ±26° lean, `fill: backwards`) are absent; tiles snap
//!   between recipes and layouts.
//! - No `filter: brightness(1.1)`: hover brightens the background, border and
//!   icon by the same factor instead of filtering the whole layer.
//! - No portals: the tooltip and layout menu are `deferred` in-tree children, so
//!   the rail must not clip (the source's `overflow-y-auto` / `overflow-x-auto`
//!   are omitted) and the tooltip cannot flip to the opposite side or clamp to
//!   the viewport. The layout menu uses `snap_to_window_with_margin(8)`.
//! - No `aria-*` / role / keyboard focus: `aria-pressed`, `role="toolbar"`,
//!   `aria-orientation`, `aria-label` and the focus-reveal of the tooltip are not
//!   representable; the tooltip is hover-only.
//! - No `tracking-wide` (GPUI has no letter-spacing); the menu title is
//!   uppercased to match the source's `uppercase` class.
//! - `backdrop-blur-sm` on the menu surface is omitted (no backdrop filter).
//! - The layout menu opens on right mouse-down rather than the browser's
//!   `contextmenu` event, and the source's Escape / resize / scroll dismissal is
//!   absent (the GPUI backdrop handles outside pointer-down only). Right-clicking
//!   a second tile while the menu is open closes it instead of reopening at the
//!   new position, because the backdrop sits above the tiles.
//!
//! State the parent does not carry (rail-local, as in the source's own state):
//! - `layout`: the source persists it in localStorage and the Settings "Toolbar
//!   position" control also writes it. Rail-local here, and the default stays
//!   `Vertical` because `panels.rs` hardcodes the 72px column inset
//!   (`LEFT_DOCK_LEFT`) with no layout-dependent inset yet. The source defaults to
//!   `horizontal`; flipping this default needs `panels::panel_stack` to take the
//!   rail layout.
//! - `models_visible`: persisted in the source. Mirrored into
//!   `PanelLayout::set_collapsed("models", …)`; the source uses `display: none`
//!   (`hidden`), so the GPUI panel keeps its 48px header instead of disappearing,
//!   and its own chevron can re-expand it without updating this preference.
//!   Switching to the bar re-shows it (`modelsPanelShown = modelsPanelVisible ||
//!   toolLayout === 'horizontal'`); switching back re-applies the preference.
//! - `support_mode` (`supportRailMode`, default `'manual'`) and `hollowing`
//!   (`transformMgr.transformMode === 'hollowing'`). The source keeps both
//!   independent of Prepare's `transformMode`; this module does too, so
//!   `active_tool` is never overwritten from Support. Selecting a panel clears
//!   Hollowing, and selecting Hollowing keeps the panel mode remembered, as in
//!   the source. The selected Support panel is not wired to the panel stack yet:
//!   `main.rs` always mounts the "supports" panel, and `DragonFruit` has no
//!   hollowing transform mode, so Hollow only lights its tile.
//! - `interior_view` and `view_lines` (`interiorView`,
//!   `supportDisplaySettings.navigationDiscsOnly`).
//!
//! Missing parent data:
//! - `interiorViewAvailable` (`hasCavityGeometry`): `DragonFruit` has no cavity
//!   flag, so the Interior View tile is always enabled and always shows the
//!   enabled hint. The disabled recipe and the "Hollow the model first to look
//!   inside it" hint are ported but unreachable.
//! - `onModeHover` (`handleToolRailHover`): the parent has no hover-preview hook.
//!   In the source it only primes the hollowing preview cache for a `'hollowing'`
//!   entry, and no rail entry passes it, so nothing visible is lost.
//! - The source has one shared `transformMode`, so entering Support resets it to
//!   `'select'` (no gizmo) and restores the remembered Prepare tool on the way
//!   back. `DragonFruit::active_tool` is only the Prepare tool here, so the
//!   shell's gizmo check (`active_tool == 1` in `main.rs`) still sees Transform
//!   while Support is showing a panel. Fixing that needs a per-workspace tool
//!   state on the entity.
//! - The source renders the rail only when `scene.models.length > 0`; `main.rs`
//!   renders it unconditionally (the entity carries no model count).

use std::cell::RefCell;

use gpui::{
    Anchor, AnchoredPositionMode, AnyElement, App, BoxShadow, FontWeight, IntoElement, MouseButton,
    MouseDownEvent, Point, Pixels, WeakEntity, Window, anchored, deferred, div, point, prelude::*,
    px, relative, rgb, rgba,
};

use gpui_core::icons::icon;
use gpui_core::theme::*;
use gpui_ui::ui::{ContextMenuEntry, context_menu};
use crate::{DragonFruit, Mode};

/// The tile's side, in pixels (`h-[54px]` / `w-[54px]`).
const TILE_SIZE: f32 = 54.0;
/// The docked column's width (`w-[54px]`).
const RAIL_WIDTH: f32 = 54.0;
/// The rail's inset from the window's left edge (`left-1.5`).
const RAIL_LEFT: f32 = 6.0;
/// The rail starts under the app bar (`top-[var(--topbar-height)]`).
const RAIL_TOP: f32 = TOPBAR_HEIGHT;
/// Gap between tiles (`gap-1.5`) and the column's vertical inset (`py-1.5`).
const RAIL_GAP: f32 = 6.0;
/// The bar sits 4px lower than the column's top (`top-[calc(...+4px)]`).
const BAR_TOP: f32 = TOPBAR_HEIGHT + 4.0;
/// A group gap inside the rail (`mb-3` / `mt-3` / `mr-3` / `ml-3`).
const GROUP_GAP: f32 = 12.0;
/// The tooltip's fixed width (`width={140}`).
const TOOLTIP_WIDTH: f32 = 140.0;
/// The context menu's width (`w-48`).
const MENU_WIDTH: f32 = 192.0;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Layout {
    #[default]
    Vertical,
    Horizontal,
}

/// Which recipe a tile wears: tools use `--accent`, panels `--accent-secondary`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tone {
    Tool,
    Panel,
}

/// The side of an entry that takes an extra group gap.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Separated {
    Above,
    Below,
}

/// What a click does. Prepare's tools set the parent's `active_tool`; Support's
/// panel mode and Hollowing are rail-local, as in the source's separate
/// `supportRailMode` / `transformMode`; the Models entry toggles the parent's
/// `collapsed` map; the two Support view toggles are rail-local.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    SelectPrepareTool(usize),
    SelectSupportPanel(usize),
    SelectHollowing,
    ToggleModels,
    ToggleInterior,
    ToggleViewLines,
}

#[derive(Clone, Copy)]
struct RailEntry {
    id: &'static str,
    label: &'static str,
    hint: &'static str,
    icon: &'static str,
    active: bool,
    /// `invertLit` in the React entry: wear the off recipe while `active`.
    invert_lit: bool,
    tone: Tone,
    disabled: bool,
    separated: Option<Separated>,
    action: Action,
}

struct RailState {
    layout: Layout,
    hovered: Option<&'static str>,
    /// Window coordinates of the gesture that opened the layout menu.
    menu: Option<(f32, f32)>,
    /// `modelsPanelVisible` — the stored preference, not the shown state.
    models_visible: bool,
    /// `supportRailMode`: 0 Islands, 1 Manual, 2 Automatic (default `'manual'`).
    support_mode: usize,
    /// `transformMgr.transformMode === 'hollowing'`.
    hollowing: bool,
    interior_view: bool,
    view_lines: bool,
}

impl Default for RailState {
    fn default() -> Self {
        Self {
            layout: Layout::Vertical,
            hovered: None,
            menu: None,
            models_visible: true,
            // `useState<SupportRailMode>('manual')`.
            support_mode: 1,
            hollowing: false,
            interior_view: false,
            view_lines: false,
        }
    }
}

thread_local! {
    static RAIL: RefCell<RailState> = RefCell::new(RailState::default());
}

/// `#rrggbb` plus an alpha byte, for `gpui::rgba`.
fn alpha(color: u32, a: u8) -> u32 {
    (color << 8) | a as u32
}

/// `hover:brightness-110`: CSS filters the whole tile, GPUI cannot, so the
/// background, border and icon are brightened by the same factor.
fn brighten(color: u32, factor: f32) -> u32 {
    let channel = |shift: u32| {
        (((color >> shift) & 0xff) as f32 * factor)
            .min(255.0)
            .round() as u32
    };
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

/// The three colours a tile paints, plus the lit recipe's inset ring.
struct TileColors {
    background: u32,
    border: u32,
    icon: u32,
    /// `boxShadow: 0 0 0 1px color-mix(hue, transparent 74%) inset`.
    ring: Option<u32>,
}

/// The off/on recipes from `railEntryStyles`, with the hue chosen by tone.
fn entry_colors(entry: &RailEntry) -> TileColors {
    if entry.disabled {
        return TileColors {
            background: SURFACE_1,
            border: BORDER_SUBTLE,
            icon: TEXT_MUTED,
            ring: None,
        };
    }
    let hue = match entry.tone {
        Tone::Tool => ACCENT,
        Tone::Panel => ACCENT_SECONDARY,
    };
    // `invertLit` flips which recipe is worn, never what `active` means.
    let lit = if entry.invert_lit {
        !entry.active
    } else {
        entry.active
    };
    if lit {
        TileColors {
            // `color-mix(in srgb, hue, var(--surface-0) 78%)`.
            background: mix(hue, SURFACE_0, 0.22),
            // `color-mix(in srgb, hue, white 14%)`.
            border: mix(hue, 0xffffff, 0.86),
            icon: hue,
            // `color-mix(in srgb, hue, transparent 74%)` = hue at 26%.
            ring: Some(alpha(hue, 0x42)),
        }
    } else {
        TileColors {
            // `color-mix(in srgb, hue 10%, var(--surface-1))`.
            background: mix(hue, SURFACE_1, 0.10),
            // `color-mix(in srgb, hue 12%, var(--border-subtle))`.
            border: mix(hue, BORDER_SUBTLE, 0.12),
            icon: TEXT_MUTED,
            ring: None,
        }
    }
}

fn models_entry(models_visible: bool) -> RailEntry {
    RailEntry {
        id: "models",
        label: "Models",
        hint: "Show or hide the model list",
        icon: "boxes",
        active: models_visible,
        invert_lit: true,
        tone: Tone::Panel,
        disabled: false,
        separated: Some(Separated::Below),
        action: Action::ToggleModels,
    }
}

/// The eight Prepare tools, in rail order (`buildPrepareToolRailEntries`).
const PREPARE_TOOLS: [(&str, &str, &str, &str, Option<Separated>); 8] = [
    (
        "select",
        "Drag",
        "Move and place models on the plate",
        "hand",
        None,
    ),
    (
        "transform",
        "Transform",
        "Move, rotate, and scale",
        "move-3d",
        None,
    ),
    (
        "placeOnFace",
        "On-Face",
        "Orient flat against plate",
        "place-on-face",
        None,
    ),
    (
        "mirror",
        "Mirror",
        "Mirror across X, Y, or Z",
        "flip-horizontal-2",
        None,
    ),
    (
        "duplicate",
        "Duplicate",
        "Array copies across the plate",
        "copy",
        Some(Separated::Above),
    ),
    (
        "arrange",
        "Arrange",
        "Arrange models on the plate",
        "layout-grid",
        None,
    ),
    (
        "organicCut",
        "Split",
        "Split the model along a drawn seam",
        "cut-seam",
        Some(Separated::Above),
    ),
    (
        "smoothing",
        "Smooth",
        "Sculpt and smooth surface",
        "smoothing-sphere",
        None,
    ),
];

/// The three Support panel tools, in rail order (`buildSupportToolRailEntries`).
const SUPPORT_PANELS: [(&str, &str, &str, &str, Option<Separated>); 3] = [
    (
        "islands",
        "Islands",
        "Scan the model for unsupported islands",
        "island-palm",
        None,
    ),
    (
        "manual",
        "Manual",
        "Support settings and presets",
        "pencil-ruler",
        Some(Separated::Above),
    ),
    (
        "auto",
        "Automatic",
        "Auto support generation",
        "auto-support",
        None,
    ),
];

fn entries_for(mode: Mode, active_tool: usize, layout: Layout) -> Vec<RailEntry> {
    let (models_visible, support_mode, hollowing, interior_view, view_lines) = RAIL.with(|state| {
        let state = state.borrow();
        (
            state.models_visible,
            state.support_mode,
            state.hollowing,
            state.interior_view,
            state.view_lines,
        )
    });

    let mut entries = Vec::new();
    // The bar's caller passes no Models toggle, so the tile is column-only.
    if layout == Layout::Vertical {
        entries.push(models_entry(models_visible));
    }

    match mode {
        Mode::Prepare => {
            for (index, (id, label, hint, icon, separated)) in PREPARE_TOOLS.iter().enumerate() {
                entries.push(RailEntry {
                    id,
                    label,
                    hint,
                    icon,
                    active: active_tool == index,
                    invert_lit: false,
                    tone: Tone::Tool,
                    disabled: false,
                    separated: *separated,
                    action: Action::SelectPrepareTool(index),
                });
            }
        }
        Mode::Support => {
            for (index, (id, label, hint, icon, separated)) in
                SUPPORT_PANELS.iter().enumerate()
            {
                entries.push(RailEntry {
                    id,
                    label,
                    hint,
                    icon,
                    // A panel tool is lit only while Hollowing is not showing.
                    active: !hollowing && support_mode == index,
                    invert_lit: false,
                    tone: Tone::Tool,
                    disabled: false,
                    separated: *separated,
                    action: Action::SelectSupportPanel(index),
                });
            }
            entries.push(RailEntry {
                id: "hollowing",
                label: "Hollow",
                hint: "Create cavity or open-face shell",
                icon: "hollow-shell",
                active: hollowing,
                invert_lit: false,
                tone: Tone::Tool,
                disabled: false,
                separated: Some(Separated::Above),
                action: Action::SelectHollowing,
            });
            entries.push(RailEntry {
                id: "interiorView",
                label: "Interior View",
                // `interiorViewAvailable ? enabled hint : "Hollow the model first…"`
                hint: "Look inside the hollow, at the cavity and its drain holes",
                icon: "interior-view",
                active: interior_view,
                invert_lit: false,
                tone: Tone::Tool,
                disabled: false,
                separated: None,
                action: Action::ToggleInterior,
            });
            entries.push(RailEntry {
                id: "viewMode",
                label: "View Mode",
                hint: "How the support forest is drawn",
                // The tile wears the icon of the mode in use as well as the lit
                // state, so the rail answers "how am I looking at this?".
                icon: if view_lines {
                    "line-support"
                } else {
                    "solid-support"
                },
                active: view_lines,
                invert_lit: false,
                tone: Tone::Tool,
                disabled: false,
                separated: Some(Separated::Above),
                action: Action::ToggleViewLines,
            });
        }
        Mode::Export | Mode::Printing => {}
    }

    entries
}

fn apply_action(action: Action, entity: &WeakEntity<DragonFruit>, cx: &mut App) {
    match action {
        Action::SelectPrepareTool(index) => {
            entity
                .update(cx, |this, cx| {
                    this.active_tool = index;
                    cx.notify();
                })
                .ok();
        }
        Action::SelectSupportPanel(index) => {
            RAIL.with(|state| {
                let mut state = state.borrow_mut();
                state.support_mode = index;
                // The source leaves Hollowing for the selected panel.
                state.hollowing = false;
            });
            entity.update(cx, |_, cx| cx.notify()).ok();
        }
        Action::SelectHollowing => {
            // Hollowing keeps the panel mode, so leaving it lands on the panel
            // the user last chose (`supportToolMemoryRef`).
            RAIL.with(|state| state.borrow_mut().hollowing = true);
            entity.update(cx, |_, cx| cx.notify()).ok();
        }
        Action::ToggleModels => {
            let visible = RAIL.with(|state| {
                let mut state = state.borrow_mut();
                state.models_visible = !state.models_visible;
                state.models_visible
            });
            entity
                .update(cx, |this, cx| {
                    // The panel layout owns collapse; `true` hides the body.
                    this.layout.set_collapsed("models", !visible);
                    cx.notify();
                })
                .ok();
        }
        Action::ToggleInterior => {
            RAIL.with(|state| state.borrow_mut().interior_view ^= true);
            entity.update(cx, |_, cx| cx.notify()).ok();
        }
        Action::ToggleViewLines => {
            RAIL.with(|state| state.borrow_mut().view_lines ^= true);
            entity.update(cx, |_, cx| cx.notify()).ok();
        }
    }
}

fn set_hovered(
    id: &'static str,
    entering: bool,
    entity: &WeakEntity<DragonFruit>,
    cx: &mut App,
) {
    let changed = RAIL.with(|state| {
        let mut state = state.borrow_mut();
        let next = if entering {
            Some(id)
        } else if state.hovered == Some(id) {
            None
        } else {
            state.hovered
        };
        let changed = next != state.hovered;
        state.hovered = next;
        changed
    });
    if changed {
        entity.update(cx, |_, cx| cx.notify()).ok();
    }
}

fn open_menu(position: Point<Pixels>, entity: &WeakEntity<DragonFruit>, cx: &mut App) {
    RAIL.with(|state| {
        let mut state = state.borrow_mut();
        state.menu = Some((position.x.0, position.y.0));
        state.hovered = None;
    });
    entity.update(cx, |_, cx| cx.notify()).ok();
}

fn close_menu_handler(
    entity: WeakEntity<DragonFruit>,
) -> impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static {
    move |_, _, cx| {
        let open = RAIL.with(|state| state.borrow().menu.is_some());
        if open {
            RAIL.with(|state| state.borrow_mut().menu = None);
            entity.update(cx, |_, cx| cx.notify()).ok();
        }
    }
}

/// `Tooltip.tsx`'s two `box-shadow` layers:
/// `0 6px 32px 0 rgba(0,0,0,0.44), 0 1.5px 8px 0 rgba(0,0,0,0.28)`.
fn tooltip_shadows() -> Vec<BoxShadow> {
    vec![
        BoxShadow::new(px(0.), px(6.), rgba(0x00000070).into())
            .blur_radius(px(32.))
            .spread_radius(px(0.)),
        BoxShadow::new(px(0.), px(1.5), rgba(0x00000047).into())
            .blur_radius(px(8.))
            .spread_radius(px(0.)),
    ]
}

/// The tooltip popover: a fixed 140px box, label over hint, pinned beside the
/// tile in the column and under it in the bar (`Tooltip.tsx` with
/// `anchor="trigger"`, `offsetY` 8).
fn tooltip(entry: &RailEntry, layout: Layout) -> impl IntoElement {
    let (position, anchor) = match layout {
        // The column's box goes beside the tile, centred on it.
        Layout::Vertical => (
            point(px(TILE_SIZE + 8.0), px(TILE_SIZE / 2.0)),
            Anchor::LeftCenter,
        ),
        // The bar's box goes under the tile, centred on it.
        Layout::Horizontal => (
            point(px(TILE_SIZE / 2.0), px(TILE_SIZE + 8.0)),
            Anchor::TopCenter,
        ),
    };

    // The popover is portalled above the panels in React; `deferred` is GPUI's
    // equivalent — it keeps the layout here but paints after the rest of the tree.
    deferred(
        anchored()
            .position_mode(AnchoredPositionMode::Local)
            .position(position)
            .anchor(anchor)
            .child(
                div()
                    .w(px(TOOLTIP_WIDTH))
                    .rounded(px(4.))
                    .border_1()
                    .border_color(rgb(ACCENT))
                    .bg(rgba(0x181818fa))
                    .px(px(8.))
                    .py(px(6.))
                    .text_size(px(11.))
                    .line_height(relative(1.25))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(TEXT_STRONG))
                    .shadow(tooltip_shadows())
                    .flex()
                    .flex_col()
                    .items_center()
                    .child(div().text_center().child(entry.label))
                    .child(
                        div()
                            .text_center()
                            .text_color(rgb(TEXT_MUTED))
                            .child(entry.hint),
                    ),
            ),
    )
    .with_priority(0)
}

fn tile(
    entry: RailEntry,
    index: usize,
    layout: Layout,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let colors = entry_colors(&entry);
    let hovered = RAIL.with(|state| state.borrow().hovered == Some(entry.id));
    let disabled = entry.disabled;
    let action = entry.action;
    let id = entry.id;

    // `hover:brightness-110` filters the whole tile, icon included.
    let (background, border, icon_color) = if hovered {
        (
            brighten(colors.background, 1.1),
            brighten(colors.border, 1.1),
            brighten(colors.icon, 1.1),
        )
    } else {
        (colors.background, colors.border, colors.icon)
    };

    let mut tile = div()
        .id((id, index))
        .h(px(TILE_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(5.5))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .child(icon(entry.icon, 28., icon_color));

    // The lit recipe's inset ring: `0 0 0 1px color-mix(hue, transparent 74%)`.
    if let Some(ring) = colors.ring {
        tile = tile.shadow(vec![
            BoxShadow::new(px(0.), px(0.), rgba(ring).into())
                .blur_radius(px(0.))
                .spread_radius(px(1.))
                .inset(),
        ]);
    }

    tile = match layout {
        Layout::Vertical => tile.w_full().flex_shrink_0(),
        Layout::Horizontal => tile.w(px(TILE_SIZE)).flex_shrink_0(),
    };

    tile = match entry.separated {
        Some(Separated::Above) => match layout {
            Layout::Vertical => tile.mt(px(GROUP_GAP)),
            Layout::Horizontal => tile.ml(px(GROUP_GAP)),
        },
        Some(Separated::Below) => match layout {
            Layout::Vertical => tile.mb(px(GROUP_GAP)),
            Layout::Horizontal => tile.mr(px(GROUP_GAP)),
        },
        None => tile,
    };

    // The tooltip still explains a disabled tile in the source; only the tile's
    // own `onHover` callback is withheld. Here `set_hovered` is the tooltip's
    // state, so it stays attached either way.
    let hover_entity = entity.clone();
    tile = tile.on_hover(move |entering, _, cx| {
        set_hovered(id, *entering, &hover_entity, cx)
    });

    if disabled {
        // `DISABLED_STYLES` dims the tile rather than its icon.
        tile = tile.opacity(0.45).cursor_not_allowed();
    } else {
        let click_entity = entity.clone();
        let menu_entity = entity.clone();
        tile = tile
            .on_click(move |_, _, cx| apply_action(action, &click_entity, cx))
            .on_mouse_down(MouseButton::Right, move |event, _, cx| {
                open_menu(event.position, &menu_entity, cx)
            });
    }

    if hovered {
        tile = tile.child(tooltip(&entry, layout));
    }

    tile
}

/// `ContextMenu.tsx` with the rail's two entries; the atom owns the surface,
/// title casing, row geometry and check slot.
fn menu_card(entity: WeakEntity<DragonFruit>) -> impl IntoElement {
    let layout = RAIL.with(|state| state.borrow().layout);

    let mut vertical = ContextMenuEntry::new("vertical", "Vertical");
    vertical.checked = layout == Layout::Vertical;
    let mut horizontal = ContextMenuEntry::new("horizontal", "Horizontal");
    horizontal.checked = layout == Layout::Horizontal;
    let entries = [vertical, horizontal];

    let select_entity = entity;
    let menu = context_menu(&entries, Some("Tool layout".into()), move |id, _, cx| {
        let target = if id == "horizontal" {
            Layout::Horizontal
        } else {
            Layout::Vertical
        };
        set_layout(target, &select_entity, cx);
    });

    // The menu surface blocks the backdrop underneath it; the rows stay clickable.
    div().occlude().w(px(MENU_WIDTH)).child(menu)
}

/// Switch the rail's layout, and re-apply the Models panel's shown state:
/// `modelsPanelShown = modelsPanelVisible || toolLayout === 'horizontal'`.
fn set_layout(target: Layout, entity: &WeakEntity<DragonFruit>, cx: &mut App) {
    let shown = RAIL.with(|state| {
        let mut state = state.borrow_mut();
        state.layout = target;
        state.menu = None;
        state.hovered = None;
        state.models_visible || target == Layout::Horizontal
    });
    entity
        .update(cx, |this, cx| {
            this.layout.set_collapsed("models", !shown);
            cx.notify();
        })
        .ok();
}

fn menu_backdrop(entity: WeakEntity<DragonFruit>) -> AnyElement {
    let close_left = close_menu_handler(entity.clone());
    let close_right = close_menu_handler(entity);
    deferred(
        anchored()
            .position(point(px(0.), px(0.)))
            .child(
                div()
                    .w(px(10_000.))
                    .h(px(10_000.))
                    .on_mouse_down(MouseButton::Left, close_left)
                    .on_mouse_down(MouseButton::Right, close_right),
            ),
    )
    .with_priority(1)
    .into_any_element()
}

/// The backdrop and the menu, in paint order, when the menu is open.
fn overlays(entity: WeakEntity<DragonFruit>) -> Vec<AnyElement> {
    let Some((x, y)) = RAIL.with(|state| state.borrow().menu) else {
        return Vec::new();
    };
    vec![
        menu_backdrop(entity.clone()),
        deferred(
            anchored()
                .position(point(px(x), px(y)))
                .snap_to_window_with_margin(px(8.))
                .child(menu_card(entity)),
        )
        .with_priority(2)
        .into_any_element(),
    ]
}

fn vertical_rail(
    entries: Vec<RailEntry>,
    entity: WeakEntity<DragonFruit>,
    overlays: Vec<AnyElement>,
) -> AnyElement {
    div()
        .id("tool-rail")
        .absolute()
        .left(px(RAIL_LEFT))
        .top(px(RAIL_TOP))
        .bottom(px(0.))
        .w(px(RAIL_WIDTH))
        .flex()
        .flex_col()
        .items_stretch()
        .gap(px(RAIL_GAP))
        .py(px(RAIL_GAP))
        .children(
            entries
                .into_iter()
                .enumerate()
                .map(|(index, entry)| tile(entry, index, Layout::Vertical, entity.clone())),
        )
        .children(overlays)
        .into_any_element()
}

fn horizontal_rail(
    entries: Vec<RailEntry>,
    entity: WeakEntity<DragonFruit>,
    overlays: Vec<AnyElement>,
) -> AnyElement {
    div()
        .absolute()
        .left_0()
        .right_0()
        .top(px(BAR_TOP))
        .flex()
        .justify_center()
        .child(
            div()
                .id("tool-rail")
                .flex()
                .flex_row()
                .items_stretch()
                .gap(px(RAIL_GAP))
                .px(px(RAIL_GAP))
                .children(
                    entries
                        .into_iter()
                        .enumerate()
                        .map(|(index, entry)| tile(entry, index, Layout::Horizontal, entity.clone())),
                )
                .children(overlays),
        )
        .into_any_element()
}

/// The tool rail for `mode`, with `active_tool` selecting the current Prepare
/// tool.
///
/// Prepare's indices are the eight tools in `PREPARE_TOOLS` order (0 Drag,
/// 1 Transform, … 7 Smooth). Support's panel mode and Hollowing are rail-local
/// (`support_mode`, `hollowing`), matching the source's `supportRailMode` and
/// `transformMode === 'hollowing'`, so `active_tool` is never overwritten from
/// Support. Export and Printing have no rail, matching the React call site,
/// which renders it only in Prepare and Support.
pub fn tool_rail(
    mode: Mode,
    active_tool: usize,
    entity: WeakEntity<DragonFruit>,
) -> AnyElement {
    // The source renders the rail only in Prepare and Support.
    if matches!(mode, Mode::Export | Mode::Printing) {
        return div().into_any_element();
    }
    let layout = RAIL.with(|state| state.borrow().layout);
    let entries = entries_for(mode, active_tool, layout);
    let overlays = overlays(entity.clone());
    let rail = match layout {
        Layout::Vertical => vertical_rail(entries, entity, overlays),
        Layout::Horizontal => horizontal_rail(entries, entity, overlays),
    };
    // The source's rail is `z-40`: above the panel stack (`zIndex: 20 + index`)
    // and below the app bar (`z-50`). GPUI paints in child order and `main.rs`
    // renders the panels after the rail, so deferring lifts the rail above them.
    // The rail never overlaps the app bar — it starts at the bar's bottom edge —
    // so painting above it is harmless.
    deferred(rail).with_priority(0).into_any_element()
}
