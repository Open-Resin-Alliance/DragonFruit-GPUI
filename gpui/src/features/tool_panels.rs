//! Mounts the Prepare and Support tool panels whose bodies live in
//! [`crate::transform_panel`], [`crate::smoothing_panel`],
//! [`crate::arrange_panels`], [`crate::organic_cut_panel`] and
//! [`crate::support_panels`].
//!
//! The bodies are written against state/actions pairs (or, for the Support
//! panels, a cloneable state handle plus the entity). Their action callbacks are
//! plain `Rc<dyn Fn(..)>` and therefore receive no `cx`, so they cannot write the
//! entity themselves. This module is the adapter between the two:
//!
//! * every action closure records its change in a module-local
//!   `thread_local!` [`PanelStore`] and raises a dirty flag;
//! * the panel is wrapped in a `div` whose `on_click` runs after the body's own
//!   click handler (GPUI dispatches mouse listeners in bubble order, leaf first)
//!   and flushes the store into the entity, then calls `cx.notify()`;
//! * the wrapper's `on_mouse_down` copies the entity-backed values back into the
//!   store, so a panel opened after a gizmo edit shows the entity's current
//!   transform before the first click.
//!
//! The store is thread-local because the bodies' callbacks are `'static` and the
//! adapter is handed no `cx` (or `App`) with which to read the entity while the
//! element tree is being built. It is not a second source of truth: the entity
//! remains authoritative for the fields it carries (`translate`, `rotate_z`,
//! `scale`, the mesh, the selection and the support list), and the mouse-down
//! sync refreshes those from it. The fields the entity does not carry (the
//! smoothing settings, the arrange/duplicate options, the organic-cut session,
//! auto-lift, lift distance, uniform scaling and local space) live here until the
//! shell grows fields for them.
//!
//! # Rail tool index → panel
//!
//! Prepare (`Mode::Prepare`), indices are [`crate::rail`]'s `PREPARE_TOOLS`
//! order (`buildPrepareToolRailEntries` in `toolRailEntries.ts`):
//!
//! | index | rail tool | panel |
//! |-------|-----------|-------|
//! | 0 | Drag (`select`) | none — the source mounts no panel for `transformMode === 'select'` |
//! | 1 | Transform | [`transform_body`] |
//! | 2 | On-Face (`placeOnFace`) | none — `PlaceOnFaceTool` is a scene tool, not a panel, and is not ported |
//! | 3 | Mirror | none — `MirrorTool` is a scene tool, not a panel, and is not ported |
//! | 4 | Duplicate | [`duplicate_body`] |
//! | 5 | Arrange | [`arrange_body`] |
//! | 6 | Split (`organicCut`) | [`organic_cut_body`] |
//! | 7 | Smooth (`smoothing`) | [`smoothing_body`] |
//!
//! Support (`Mode::Support`), indices are [`crate::rail`]'s `SUPPORT_PANELS`
//! order (`buildSupportToolRailEntries` in `toolRailEntries.ts`):
//!
//! | index | rail tool | panel |
//! |-------|-----------|-------|
//! | 0 | Islands | [`islands_body`] |
//! | 1 | Manual | [`support_settings_body`] (`SupportSidebar`) |
//! | 2 | Automatic | [`crate::panels::content::supports_body`] (`Auto Support (Beta)`) |
//!
//! Export and Printing have no tool rail, matching the React call site, and
//! return `None`.
//!
//! # PORT DEVIATIONS
//!
//! Diffed against `PreparePanelStack.tsx`, the Support stack in `page.tsx`
//! (`SupportSidebar`, `IslandsPanel`, `AutoSupportPanel`, `HollowingPanel`,
//! `HolePunchPanel`), `TransformControls.tsx`, `MeshSmoothingSettingsPanel.tsx`,
//! `ArrangePanel.tsx`, `DuplicatePanel.tsx` and `OrganicCutPanel.tsx`.
//!
//! * **No `cx` in the bodies' callbacks.** The flush happens in the wrapper's
//!   `on_click`, which is an ancestor listener and therefore runs after the
//!   body's own handler. A click that does not bubble to the wrapper (a menu
//!   painted outside the body's bounds, for example) records the change but does
//!   not apply it until the next click inside the wrapper. The source applies on
//!   the control's own event.
//! * **First frame.** `tool_panel` is handed no `cx`, so it cannot read the
//!   entity while building the tree. Entity-backed values start at the store's
//!   defaults and are refreshed by the wrapper's `on_mouse_down`; the first frame
//!   of a panel can therefore show the defaults (Transform at the origin, the
//!   Duplicate panel disabled while no model name is known, Auto Supports at zero
//!   supports) until the first pointer-down inside it.
//! * **Auto Orientation.** `PreparePanelStack` mounts `AutoRotationPanel` under
//!   the Transform panel only while the experiment is enabled
//!   (`orientationPanel !== null`). The port has no experiment flag, so
//!   [`crate::features::transform_panel::orientation_body`] is not stacked here; the caller
//!   can mount it itself when it grows the flag.
//! * **Hollowing / Hole Punch.** The rail models Hollowing as a transform-mode
//!   toggle (`rail.rs`'s `hollowing`), not as a Support panel index, and
//!   `tool_panel` has no flag for it. The caller mounts
//!   [`crate::features::support_panels::hollowing_body`] and
//!   [`crate::features::support_panels::hole_punch_body`] when the rail's Hollowing tile is
//!   lit; they are not reachable through this function's `tool` argument.
//! * **No engines.** Arrange, Duplicate, Organic Cut, Smoothing, Islands,
//!   Hollowing and Hole Punch have no scene services in this port: their Apply /
//!   Confirm / Scan / Generate callbacks only re-render. The bodies' own module
//!   notes record the same gap.
//! * **Transform limits.** The entity has one uniform `scale` and one
//!   `rotate_z`, so X/Y rotation and per-axis scale are recorded in the store but
//!   only `rotation[2]` and `scale[0]` reach the entity. Uniform scaling starts
//!   on (`page.tsx`'s `useState(true)`), matching the entity's single scale.
//!   `Center Selection` targets the plate origin (the port has no
//!   `originMode`/plate-size settings) and solves the translate that puts the
//!   mesh's raw bbox centre there. `Lift`/`Drop` use the mesh's raw minimum Z
//!   and the source's `PLATE_CLEARANCE_MM` (0.001 mm).
//! * **History.** `on_position_commit`, `on_rotation_complete`,
//!   `on_scale_commit` and `on_transform_commit` are no-ops: the port has no
//!   undo history.
//! * **Support settings.** `support_settings_body` keeps its own
//!   `SupportSettingsState`; the source persists the values in the support
//!   settings store. The body's own module note records the missing sections and
//!   preset actions.
//! * **Islands scan dialog.** `islands_body`'s gear opens
//!   `IslandsModel::settings_open`; [`crate::features::support_panels::island_scan_settings_dialog`]
//!   is a window-level dialog and is not mounted here (the shell owns dialogs).
//! * **Panel shells.** The bodies are returned without a card: `panels.rs` owns
//!   the card, its header, collapse chevron and drag handle, so the caller wraps
//!   the result in a `PanelDef` exactly as it wraps `panel_content`'s bodies.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gpui::{AnyElement, App, IntoElement, MouseButton, WeakEntity, div, prelude::*};

use crate::features::arrange_panels::{
    ArrangeActions, ArrangeState, DuplicateActions, DuplicateState, arrange_body, duplicate_body,
};
use crate::features::organic_cut_panel::{LoopSummary, OrganicCutActions, OrganicCutView, organic_cut_body};
use crate::panels::content;
use crate::features::smoothing_panel::{SmoothingActions, SmoothingState, smoothing_body};
use crate::features::support_panels::{islands_body, support_settings_body};
use gpui_core::support_state::{IslandsPanelState, SupportSettingsState};
use crate::features::transform_panel::{TransformActions, TransformState, transform_body};
use crate::{DragonFruit, Mode};

/// `DEFAULT_LIFT_DISTANCE_MM` in `src/features/transform/liftDefaults.ts`.
const DEFAULT_LIFT_DISTANCE_MM: f32 = 7.0;
/// `PLATE_CLEARANCE_MM` in `src/features/scene/selectionLiftDrop.ts`.
const PLATE_CLEARANCE_MM: f32 = 0.001;

/// The adapter's store for the panel state the entity does not carry.
///
/// Entity-backed fields (`transform.position`, `.rotation`, `.scale`,
/// `.has_bbox`, `.original_size`; `duplicate.active_model_name`;
/// `arrange.model_count` / `.selected_model_count`; `organic.disabled`;
/// `islands.has_geometry`; `support_count`) are refreshed from the entity by
/// [`sync_from_entity`] on the wrapper's mouse-down.
struct PanelStore {
    transform: TransformState,
    smoothing: SmoothingState,
    arrange: ArrangeState,
    duplicate: DuplicateState,
    organic: OrganicCutView,
    islands: IslandsPanelState,
    support_settings: SupportSettingsState,
    /// `scene.supports.length`, for `crate::panels::content::supports_body`.
    support_count: usize,
    /// `scene.geom !== null`.
    has_mesh: bool,
    /// The mesh's raw minimum Z (`center.z - size.z / 2`), before the model
    /// matrix. `Lift`/`Drop` need it to place the lowest world point.
    mesh_min_z: f32,
    /// The mesh's raw bbox centre in X/Y, before the model matrix.
    /// `Center Selection` needs it to put the model on the plate origin.
    mesh_center: [f32; 2],
}

impl Default for PanelStore {
    fn default() -> Self {
        let mut transform = TransformState::default();
        // `page.tsx`'s `useState(true)`.
        transform.uniform_scaling = true;
        // `useTransformManager`'s `DEFAULT_LIFT_DISTANCE_MM`, not the body's own
        // 5.0 fallback.
        transform.lift_distance = DEFAULT_LIFT_DISTANCE_MM;
        Self {
            transform,
            smoothing: SmoothingState::default(),
            arrange: ArrangeState::default(),
            duplicate: DuplicateState::default(),
            organic: OrganicCutView::default(),
            islands: IslandsPanelState::new(),
            support_settings: SupportSettingsState::new(),
            support_count: 0,
            has_mesh: false,
            mesh_min_z: 0.0,
            mesh_center: [0.0; 2],
        }
    }
}

thread_local! {
    static STORE: RefCell<PanelStore> = RefCell::new(PanelStore::default());
}

fn with_store<R>(f: impl FnOnce(&mut PanelStore) -> R) -> R {
    STORE.with(|store| f(&mut store.borrow_mut()))
}

fn mark(dirty: &Rc<Cell<bool>>, notify: &Rc<Cell<bool>>) {
    dirty.set(true);
    notify.set(true);
}

fn mark_notify(notify: &Rc<Cell<bool>>) {
    notify.set(true);
}

/// The panel for the active rail tool, or `None` when that tool has no panel.
///
/// `tool` is the rail's index: [`crate::rail`]'s `PREPARE_TOOLS` order in
/// Prepare and `SUPPORT_PANELS` order in Support. See the module docs for the
/// full table.
pub fn tool_panel(tool: usize, mode: Mode, entity: WeakEntity<DragonFruit>) -> Option<AnyElement> {
    match mode {
        Mode::Prepare => prepare_panel(tool, entity),
        Mode::Support => support_panel(tool, entity),
        Mode::Export | Mode::Printing => None,
    }
}

fn prepare_panel(tool: usize, entity: WeakEntity<DragonFruit>) -> Option<AnyElement> {
    let transform_dirty = Rc::new(Cell::new(false));
    let notify_dirty = Rc::new(Cell::new(false));

    let (id, body) = match tool {
        // `transformMgr.transformMode === 'transform'`.
        1 => {
            let actions = transform_actions(transform_dirty.clone(), notify_dirty.clone());
            let state = with_store(|store| store.transform.clone());
            (
                "tool-panel-transform",
                transform_body(&state, &actions).into_any_element(),
            )
        }
        // `transformMgr.transformMode === 'duplicate'`.
        4 => {
            let actions = duplicate_actions(notify_dirty.clone());
            let state = with_store(|store| store.duplicate.clone());
            (
                "tool-panel-duplicate",
                duplicate_body(&state, &actions).into_any_element(),
            )
        }
        // `transformMgr.transformMode === 'arrange'`.
        5 => {
            let actions = arrange_actions(notify_dirty.clone());
            let state = with_store(|store| store.arrange.clone());
            (
                "tool-panel-arrange",
                arrange_body(&state, &actions).into_any_element(),
            )
        }
        // `transformMgr.transformMode === 'organicCut'`.
        6 => {
            let actions = organic_actions(notify_dirty.clone());
            let state = with_store(|store| store.organic.clone());
            (
                "tool-panel-organic-cut",
                organic_cut_body(&state, &actions).into_any_element(),
            )
        }
        // `transformMgr.transformMode === 'smoothing'`.
        7 => {
            let actions = smoothing_actions(notify_dirty.clone());
            let state = with_store(|store| store.smoothing.clone());
            (
                "tool-panel-smoothing",
                smoothing_body(&state, &actions).into_any_element(),
            )
        }
        // Drag (`select`) mounts no panel; On-Face and Mirror are scene tools.
        _ => return None,
    };

    Some(wrap(id, body, entity, transform_dirty, notify_dirty))
}

fn support_panel(tool: usize, entity: WeakEntity<DragonFruit>) -> Option<AnyElement> {
    let transform_dirty = Rc::new(Cell::new(false));
    let notify_dirty = Rc::new(Cell::new(false));

    let (id, body) = match tool {
        // `supportRailMode === 'islands'`.
        0 => {
            let state = with_store(|store| store.islands.clone());
            (
                "tool-panel-islands",
                islands_body(&state, entity.clone()).into_any_element(),
            )
        }
        // `supportRailMode === 'manual'`.
        1 => {
            let state = with_store(|store| store.support_settings.clone());
            (
                "tool-panel-support-settings",
                support_settings_body(&state, entity.clone()).into_any_element(),
            )
        }
        // `supportRailMode === 'auto'`.
        2 => {
            let count = with_store(|store| store.support_count);
            (
                "tool-panel-support-auto",
                crate::panels::content::supports_body(count, entity.clone()).into_any_element(),
            )
        }
        _ => return None,
    };

    Some(wrap(id, body, entity, transform_dirty, notify_dirty))
}

/// Wrap a body so the store's recorded changes reach the entity.
///
/// The wrapper's mouse listeners run after the body's own (GPUI bubbles from the
/// leaf up), so a control's click has already updated the store by the time
/// `on_click` flushes it. `on_mouse_down` refreshes the entity-backed store
/// fields first, so the click that follows edits the entity's current values.
fn wrap(
    id: &'static str,
    body: AnyElement,
    entity: WeakEntity<DragonFruit>,
    transform_dirty: Rc<Cell<bool>>,
    notify_dirty: Rc<Cell<bool>>,
) -> AnyElement {
    let down_entity = entity.clone();
    let click_entity = entity;
    div()
        .id(id)
        .w_full()
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            sync_from_entity(&down_entity, cx);
            down_entity.update(cx, |_, cx| cx.notify()).ok();
        })
        .on_click(move |_, _, cx| {
            flush(&click_entity, &transform_dirty, &notify_dirty, cx);
        })
        .child(body)
        .into_any_element()
}

/// Copy the entity-backed values the panels display into the store.
pub(crate) fn sync_from_entity(entity: &WeakEntity<DragonFruit>, cx: &mut App) {
    entity
        .read_with(cx, |this, _| {
            with_store(|store| {
                store.has_mesh = this.mesh.is_some();
                store.support_count = this.supports.borrow().len();
                store.transform.position = this.translate;
                store.transform.rotation = [0.0, 0.0, this.rotate_z];
                store.transform.scale = [this.scale; 3];
                if let Some(mesh) = this.mesh.as_ref() {
                    store.transform.has_bbox = true;
                    store.transform.original_size = mesh.size;
                    store.mesh_min_z = mesh.center.z - mesh.size[2] * 0.5;
                    store.mesh_center = [mesh.center.x, mesh.center.y];
                    store.duplicate.active_model_name = Some(mesh.name.clone().into());
                } else {
                    store.transform.has_bbox = false;
                    store.transform.original_size = [1.0; 3];
                    store.mesh_min_z = 0.0;
                    store.duplicate.active_model_name = None;
                }
                store.arrange.model_count = if store.has_mesh { 1 } else { 0 };
                store.arrange.selected_model_count = if store.has_mesh && this.selected.get() {
                    1
                } else {
                    0
                };
                store.organic.disabled = !store.has_mesh;
                store.islands.model_mut().has_geometry = store.has_mesh;
            });
        })
        .ok();
}

/// Write the store's transform back to the entity and redraw.
fn flush(
    entity: &WeakEntity<DragonFruit>,
    transform_dirty: &Rc<Cell<bool>>,
    notify_dirty: &Rc<Cell<bool>>,
    cx: &mut App,
) {
    let transform_changed = transform_dirty.replace(false);
    let should_notify = notify_dirty.replace(false);
    if !transform_changed && !should_notify {
        return;
    }

    let (position, rotation, scale) = with_store(|store| {
        (
            store.transform.position,
            store.transform.rotation,
            store.transform.scale,
        )
    });

    entity
        .update(cx, |this, cx| {
            if transform_changed {
                this.translate = position;
                // The entity rotates about Z only.
                this.rotate_z = rotation[2];
                // The entity scales uniformly.
                this.scale = scale[0];
            }
            cx.notify();
        })
        .ok();
}

fn transform_actions(
    transform_dirty: Rc<Cell<bool>>,
    notify_dirty: Rc<Cell<bool>>,
) -> TransformActions {
    macro_rules! flags {
        () => {{
            let dirty = transform_dirty.clone();
            let notify = notify_dirty.clone();
            (dirty, notify)
        }};
    }

    let (d, n) = flags!();
    let on_position_change = Rc::new(move |position: [f32; 3]| {
        with_store(|store| store.transform.position = position);
        mark(&d, &n);
    });

    let (d, n) = flags!();
    let on_center = Rc::new(move || {
        // `handleCenterSelectedModels` targets the plate centre. The port has no
        // `originMode`/plate size, and the plate is centred on the origin. The
        // model matrix applies `translate + Rz(rotation) * (scale * raw)`, so
        // solve for the translate that puts the raw bbox centre at (0, 0).
        with_store(|store| {
            let angle = store.transform.rotation[2];
            let (sin, cos) = angle.sin_cos();
            let scale = store.transform.scale[0];
            let x = scale * store.mesh_center[0];
            let y = scale * store.mesh_center[1];
            store.transform.position[0] = -(cos * x - sin * y);
            store.transform.position[1] = -(sin * x + cos * y);
        });
        mark(&d, &n);
    });

    let (d, n) = flags!();
    let on_rotation_change = Rc::new(move |rotation: [f32; 3]| {
        with_store(|store| store.transform.rotation = rotation);
        mark(&d, &n);
    });

    let (d, n) = flags!();
    let on_reset_rotation = Rc::new(move || {
        with_store(|store| store.transform.rotation = [0.0; 3]);
        mark(&d, &n);
    });

    let (d, n) = flags!();
    let on_scale_change = Rc::new(move |scale: [f32; 3]| {
        with_store(|store| store.transform.scale = scale);
        mark(&d, &n);
    });

    let (d, n) = flags!();
    let on_reset_scale = Rc::new(move || {
        with_store(|store| store.transform.scale = [1.0; 3]);
        mark(&d, &n);
    });

    let n = notify_dirty.clone();
    let on_uniform_scaling_change = Rc::new(move |value: bool| {
        with_store(|store| store.transform.uniform_scaling = value);
        mark_notify(&n);
    });

    let n = notify_dirty.clone();
    let on_local_space_change = Rc::new(move |value: bool| {
        with_store(|store| store.transform.local_space = value);
        mark_notify(&n);
    });

    let n = notify_dirty.clone();
    let on_auto_lift_change = Rc::new(move |value: bool| {
        with_store(|store| store.transform.auto_lift = value);
        mark_notify(&n);
    });

    let n = notify_dirty.clone();
    let on_lift_distance_change = Rc::new(move |value: f32| {
        with_store(|store| store.transform.lift_distance = value);
        mark_notify(&n);
    });

    let (d, n) = flags!();
    let on_lift = Rc::new(move || {
        with_store(|store| {
            if store.has_mesh {
                let bottom =
                    store.transform.position[2] + store.transform.scale[0] * store.mesh_min_z;
                store.transform.position[2] += store.transform.lift_distance - bottom;
            }
        });
        mark(&d, &n);
    });

    let (d, n) = flags!();
    let on_drop = Rc::new(move || {
        with_store(|store| {
            if store.has_mesh {
                let bottom =
                    store.transform.position[2] + store.transform.scale[0] * store.mesh_min_z;
                store.transform.position[2] += PLATE_CLEARANCE_MM - bottom;
            }
        });
        mark(&d, &n);
    });

    let n = notify_dirty.clone();
    let on_selection_rotation_mode_change = Rc::new(move |mode| {
        with_store(|store| store.transform.selection_rotation_mode = Some(mode));
        mark_notify(&n);
    });

    TransformActions {
        on_position_change,
        on_position_commit: Rc::new(|| {}),
        on_center,
        on_rotation_change,
        on_reset_rotation,
        on_rotation_complete: Rc::new(|| {}),
        on_scale_change,
        on_reset_scale,
        on_scale_commit: Rc::new(|| {}),
        on_uniform_scaling_change,
        on_local_space_change,
        on_auto_lift_change,
        on_lift_distance_change,
        on_lift,
        on_drop,
        on_transform_commit: Rc::new(|| {}),
        on_selection_rotation_mode_change,
    }
}

fn smoothing_actions(notify_dirty: Rc<Cell<bool>>) -> SmoothingActions {
    let n = notify_dirty.clone();
    let on_brush_size_change = Rc::new(move |value: f32| {
        with_store(|store| store.smoothing.brush_size_mm = value);
        mark_notify(&n);
    });

    let n = notify_dirty.clone();
    let on_strength_change = Rc::new(move |value: f32| {
        with_store(|store| store.smoothing.strength = value);
        mark_notify(&n);
    });

    let n = notify_dirty.clone();
    let on_iterations_change = Rc::new(move |value: f32| {
        with_store(|store| store.smoothing.iterations = value);
        mark_notify(&n);
    });

    let n = notify_dirty.clone();
    let on_falloff_change = Rc::new(move |falloff| {
        with_store(|store| store.smoothing.falloff = falloff);
        mark_notify(&n);
    });

    let n = notify_dirty.clone();
    let on_highlight_color_change = Rc::new(move |color| {
        with_store(|store| store.smoothing.highlight_color = color);
        mark_notify(&n);
    });

    let n = notify_dirty;
    let on_restore_defaults = Rc::new(move || {
        with_store(|store| store.smoothing = SmoothingState::default());
        mark_notify(&n);
    });

    SmoothingActions {
        on_brush_size_change,
        on_strength_change,
        on_iterations_change,
        on_falloff_change,
        on_highlight_color_change,
        on_restore_defaults,
    }
}

fn arrange_actions(notify_dirty: Rc<Cell<bool>>) -> ArrangeActions {
    let n = notify_dirty.clone();
    let on_precision_mode_change = Rc::new(move |mode| {
        with_store(|store| store.arrange.precision_mode = mode);
        mark_notify(&n);
    });

    let n = notify_dirty.clone();
    let on_fill_mode_change = Rc::new(move |mode| {
        with_store(|store| store.arrange.fill_mode = mode);
        mark_notify(&n);
    });

    let n = notify_dirty.clone();
    let on_layout_mode_change = Rc::new(move |mode| {
        with_store(|store| store.arrange.layout_mode = mode);
        mark_notify(&n);
    });

    let n = notify_dirty.clone();
    let on_spacing_change = Rc::new(move |value: f32| {
        with_store(|store| store.arrange.spacing_mm = value);
        mark_notify(&n);
    });

    let n = notify_dirty.clone();
    let on_allow_rotate_on_z_change = Rc::new(move |value: bool| {
        with_store(|store| store.arrange.allow_rotate_on_z = value);
        mark_notify(&n);
    });

    let n = notify_dirty.clone();
    let on_array_count_change = Rc::new(move |axis: usize, count: i32| {
        with_store(|store| {
            if axis < 3 {
                store.arrange.array_count[axis] = count;
            }
        });
        mark_notify(&n);
    });

    let n = notify_dirty.clone();
    let on_array_gap_change = Rc::new(move |axis: usize, gap: i32| {
        with_store(|store| {
            if axis < 3 {
                store.arrange.array_gap[axis] = gap;
            }
        });
        mark_notify(&n);
    });

    let n = notify_dirty.clone();
    let on_anchor_mode_change = Rc::new(move |mode| {
        with_store(|store| store.arrange.anchor_mode = mode);
        mark_notify(&n);
    });

    let n = notify_dirty.clone();
    let on_anchor_menu_open_change = Rc::new(move |open: bool| {
        with_store(|store| store.arrange.anchor_menu_open = open);
        mark_notify(&n);
    });

    // PORT GAP: no arrange engine — the action only re-renders.
    let n = notify_dirty.clone();
    let on_apply_all = Rc::new(move || mark_notify(&n));

    let n = notify_dirty;
    let on_apply_selected = Rc::new(move || mark_notify(&n));

    ArrangeActions {
        on_precision_mode_change,
        on_fill_mode_change,
        on_layout_mode_change,
        on_spacing_change,
        on_allow_rotate_on_z_change,
        on_array_count_change,
        on_array_gap_change,
        on_anchor_mode_change,
        on_anchor_menu_open_change,
        on_apply_all,
        on_apply_selected,
    }
}

fn duplicate_actions(notify_dirty: Rc<Cell<bool>>) -> DuplicateActions {
    let n = notify_dirty.clone();
    let on_layout_mode_change = Rc::new(move |mode| {
        with_store(|store| store.duplicate.layout_mode = mode);
        mark_notify(&n);
    });

    let n = notify_dirty.clone();
    let on_precision_mode_change = Rc::new(move |mode| {
        with_store(|store| store.duplicate.precision_mode = mode);
        mark_notify(&n);
    });

    let n = notify_dirty.clone();
    let on_total_copies_change = Rc::new(move |value: f32| {
        with_store(|store| store.duplicate.total_copies = value);
        mark_notify(&n);
    });

    let n = notify_dirty.clone();
    let on_spacing_change = Rc::new(move |value: f32| {
        with_store(|store| store.duplicate.spacing_mm = value);
        mark_notify(&n);
    });

    let n = notify_dirty.clone();
    let on_array_count_change = Rc::new(move |axis: usize, count: i32| {
        with_store(|store| {
            if axis < 3 {
                store.duplicate.array_count[axis] = count;
            }
        });
        mark_notify(&n);
    });

    let n = notify_dirty.clone();
    let on_array_gap_change = Rc::new(move |axis: usize, gap: i32| {
        with_store(|store| {
            if axis < 3 {
                store.duplicate.array_gap[axis] = gap;
            }
        });
        mark_notify(&n);
    });

    // PORT GAP: no duplicate engine — the action only re-renders.
    let n = notify_dirty.clone();
    let on_confirm = Rc::new(move || mark_notify(&n));

    let n = notify_dirty;
    let on_fill_plate = Rc::new(move || mark_notify(&n));

    DuplicateActions {
        on_layout_mode_change,
        on_precision_mode_change,
        on_total_copies_change,
        on_spacing_change,
        on_array_count_change,
        on_array_gap_change,
        on_confirm,
        on_fill_plate,
    }
}

fn organic_actions(notify_dirty: Rc<Cell<bool>>) -> OrganicCutActions {
    let n = notify_dirty.clone();
    let on_state_change = Rc::new(move |state| {
        with_store(|store| store.organic.state = state);
        mark_notify(&n);
    });

    let n = notify_dirty.clone();
    let on_clear_loop = Rc::new(move || {
        with_store(|store| {
            if let Some(summary) = store
                .organic
                .loop_summaries
                .get_mut(store.organic.active_loop_index)
            {
                summary.point_count = 0;
            }
            store.organic.can_apply = false;
        });
        mark_notify(&n);
    });

    // PORT GAP: no geometry snapping — the action only re-renders.
    let n = notify_dirty.clone();
    let on_snap_to_edges = Rc::new(move || mark_notify(&n));

    let n = notify_dirty.clone();
    let on_select_loop = Rc::new(move |index: usize| {
        with_store(|store| store.organic.active_loop_index = index);
        mark_notify(&n);
    });

    let n = notify_dirty.clone();
    let on_add_loop = Rc::new(move || {
        with_store(|store| {
            let index = store.organic.loop_count;
            store.organic.loop_count += 1;
            store.organic.active_loop_index = index;
            store.organic.loop_summaries.push(LoopSummary {
                index,
                point_count: 0,
            });
        });
        mark_notify(&n);
    });

    let n = notify_dirty.clone();
    let on_remove_loop = Rc::new(move |index: usize| {
        with_store(|store| {
            if index < store.organic.loop_summaries.len() {
                store.organic.loop_summaries.remove(index);
            }
            store.organic.loop_count = store.organic.loop_count.saturating_sub(1);
            if store.organic.active_loop_index >= store.organic.loop_count {
                store.organic.active_loop_index = store.organic.loop_count.saturating_sub(1);
            }
        });
        mark_notify(&n);
    });

    // PORT GAP: no cut engine — the action only re-renders.
    let n = notify_dirty;
    let on_apply = Rc::new(move || mark_notify(&n));

    OrganicCutActions {
        on_state_change,
        on_clear_loop,
        on_snap_to_edges,
        on_select_loop,
        on_add_loop,
        on_remove_loop,
        on_apply,
    }
}
