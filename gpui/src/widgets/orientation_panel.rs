//! The Auto Orientation panel as the PREPARE stack mounts it — the source's
//! `AutoRotationPanel`, rendered beside the Transform card.
//!
//! `src/components/organisms/panels/PreparePanelStack.tsx:280-283`:
//!
//! ```tsx
//! {scene.geom && transformMgr.transformMode === 'transform' && orientationPanel && (
//!   <AutoRotationPanel key="prepare-orientation" {...orientationPanel} />
//! )}
//! ```
//!
//! `orientationPanel` is the page's props object and is non-null only while the
//! `auto-rotation` experiment is on (`page.tsx:10775`), so the panel has three
//! gates: geometry, the Transform tool, and the experiment. This module keeps
//! all three and returns the [`PanelDef`] the shell pushes.
//!
//! [`crate::features::transform_panel::orientation_body`] draws the panel; this
//! module owns the state behind it (which must outlive a frame — switching tools
//! and back keeps the objective and the blocker mode, as the React component's
//! store-backed fields do), the `PanelDef`, and the wrapper that turns a
//! control's change into a repaint. The wrapper is the same arrangement
//! `features/tool_panels.rs` uses: the body's own handler runs first (GPUI
//! bubbles from the leaf up), then the wrapper's click notifies.
//!
//! # PORT DEVIATIONS
//!
//! * **No orientation advisor.** `Orient Model` reports the intent through
//!   `on_orient` and nothing sweeps: `suggestOrientationForGeometry`, the
//!   world-frame bake and the history apply are unported scene work
//!   (`orientation_body`'s own module note). The button therefore does nothing
//!   visible today — it is `dead` in the app, not wrong.
//! * **No support-blocker mask store.** `blocked_count` is the panel's own: it
//!   starts at 0 and `Clear` zeroes it, so `Clear` is disabled until a store
//!   exists to give it something to clear. The paint mode and the brush size are
//!   the panel's.
//! * **The experiment gate is a cached flag, not a live read.** The settings
//!   module exposes no cheap `is_experiment_enabled(id)` predicate, and cloning
//!   its whole `Settings` snapshot once per frame just to read one bool is not
//!   worth it. [`set_experiment_enabled`] is the hook for whoever owns the
//!   experiments store; the default is the manifest's `defaultEnabled` (`true`
//!   for `auto-rotation`, `src/config/experiments.json:21`), which is also what
//!   `isExperimentEnabled` answers before the user touches the toggle. Until the
//!   hook is called, a user who turns the experiment *off* still sees the panel.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{AnyElement, App, IntoElement, WeakEntity, div, prelude::*};

use crate::features::transform_panel::{
    OrientationActions, OrientationObjective, OrientationState, orientation_body,
};
use crate::DragonFruit;
use gpui_core::panel_layout::{PanelDef, PanelKind};

/// The panel's own state, kept across frames and across tool switches.
///
/// `active_model_id` / `active_model_name` are refreshed from the entity on
/// every build; everything else is what the user set.
thread_local! {
    /// `OrientationState::default()` already carries the panel's own defaults
    /// (`Fewest Supports`, 2.5 mm brush, no error, `has_toggle_blockers`).
    static STATE: RefCell<OrientationState> = RefCell::new(OrientationState::default());
    /// The `auto-rotation` experiment toggle, cached (see the module deviations).
    static EXPERIMENT_ENABLED: RefCell<bool> = RefCell::new(true);
}

/// The `auto-rotation` experiment's state, as the settings store reports it.
///
/// Call this when the experiment changes. The default is the manifest's
/// `defaultEnabled`, so a shell that never calls it matches a fresh profile.
pub fn set_experiment_enabled(enabled: bool) {
    EXPERIMENT_ENABLED.with(|flag| *flag.borrow_mut() = enabled);
}

fn experiment_enabled() -> bool {
    EXPERIMENT_ENABLED.with(|flag| *flag.borrow())
}

/// The Auto Orientation panel's `PanelDef`, or `None` when a gate is closed.
///
/// Gates, in the source's order: `scene.geom` (a readable mesh), the active tool
/// being Transform, and the experiment. The shell pushes the returned def
/// straight after the Transform card's.
pub fn orientation_panel_def(
    model_name: Option<gpui::SharedString>,
    entity: WeakEntity<DragonFruit>,
) -> Option<PanelDef> {
    if !experiment_enabled() {
        return None;
    }

    // `scene.geom` plus the two fields the panel reads: the active model's id
    // and its display name.
    // The name is passed in: reading the entity here would re-enter it, because
    // the panel stack is built inside the entity's own render.
    let model_name = model_name?;

    let state = STATE.with(|state| {
        let mut state = state.borrow_mut();
        // The panel is only mounted while it is the active model's, and the
        // source never keeps a stale error across a model or goal change
        // (`// A new model or goal clears a stale error.`).
        let id: gpui::SharedString = model_name.as_str().into();
        if state.active_model_id.as_ref() != Some(&id) {
            state.error = None;
        }
        state.active_model_id = Some(id);
        state.active_model_name = Some(model_name.as_str().into());
        state.busy = false;
        state.clone()
    });

    let actions = actions();
    let body = wrap(
        orientation_body(&state, &actions).into_any_element(),
        entity,
    );

    Some(
        PanelDef::new(
            "prepare-orientation",
            // `TITLE` in `AutoRotationPanel.tsx`.
            "Auto Orientation (Beta)",
            PanelKind::Orientation,
        )
        .content(body),
    )
}

/// The six callbacks `orientation_body` reports through.
///
/// They mutate the module's state; the wrapper's click notifies, so the next
/// frame draws the new values.
fn actions() -> OrientationActions {
    let on_orient = Rc::new(move || {
        // `handleOrient`'s sweep is unported scene work. Nothing to compute
        // here, and no busy modal to raise: the panel reports the intent and
        // stops.
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.error = None;
        });
    });

    let on_objective_change = Rc::new(move |objective: OrientationObjective| {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            if state.objective != objective {
                // `A new model or goal clears a stale error.`
                state.error = None;
            }
            state.objective = objective;
            state.objective_menu_open = false;
        });
    });

    let on_objective_menu_open_change = Rc::new(move |open: bool| {
        STATE.with(|state| state.borrow_mut().objective_menu_open = open);
    });

    let on_toggle_blockers = Rc::new(move || {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.blockers_active = !state.blockers_active;
            state.objective_menu_open = false;
        });
    });

    let on_clear_blockers = Rc::new(move || {
        STATE.with(|state| state.borrow_mut().blocked_count = 0);
    });

    let on_brush_size_change = Rc::new(move |value: f32| {
        let value = value.clamp(
            crate::features::transform_panel::SUPPORT_BLOCKER_BRUSH_MIN_MM,
            crate::features::transform_panel::SUPPORT_BLOCKER_BRUSH_MAX_MM,
        );
        STATE.with(|state| state.borrow_mut().brush_size_mm = value);
    });

    OrientationActions {
        on_orient,
        on_objective_change,
        on_objective_menu_open_change,
        on_toggle_blockers,
        on_clear_blockers,
        on_brush_size_change,
    }
}

/// Wrap a panel body so a control's change reaches the entity, the way
/// `features/tool_panels.rs::wrap` does for the tool panels.
fn wrap(body: AnyElement, entity: WeakEntity<DragonFruit>) -> AnyElement {
    div()
        .id("prepare-orientation-panel")
        .w_full()
        .on_click(move |_, _, cx| {
            entity.update(cx, |_, cx| cx.notify()).ok();
        })
        .child(body)
        .into_any_element()
}
