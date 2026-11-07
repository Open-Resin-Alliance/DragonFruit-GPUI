//! `src/components/organisms/panels/PreparePanelStack.tsx`.
//!
//! The stack is the PREPARE mode's panel group: the model manager plus the tool
//! panel for the active transform mode. This port keeps the source's gating
//! exactly — which panels are mounted for a given scene and tool — and hands the
//! shell one body per mounted panel.
//!
//! The card shell, its collapse chevron and its drag handle belong to
//! `panels.rs`, so a panel here is a body plus the id and title the shell needs
//! for its [`crate::panels::PanelDef`]. `PanelKind` in `panels.rs` has no variant
//! for these tools yet; the shell will need one per id below before it can mount
//! them (recorded here rather than edited there).
//!
//! # PORT DEVIATIONS
//!
//! * `ModelsPanel`/`ModelsPanel`'s body is owned by `panel_content.rs`, not this
//!   module: the caller passes it in, and the `dimmed` / `hidden` /
//!   `collapsible` / `bottomClearancePx` flags travel on
//!   [`PrepareStackState`] for the shell to apply.
//! * The source's `'use no memo'` and its inline invocation (so
//!   `FloatingPanelStack` can flatten the panels as direct children) have no
//!   GPUI counterpart: [`visible_panels`] returns the same ordered list, and the
//!   shell decides placement.
//! * `scene.geom` becomes `has_geometry`; `scene.models.length > 0` becomes
//!   `model_count > 0`; `transformMgr.transformMode` becomes [`TransformMode`].
//! * The Transform panel's multi-selection wiring (which callback a field calls
//!   when several models are selected) is the caller's: it builds
//!   [`crate::transform_panel::TransformActions`] from either path, exactly as
//!   `PreparePanelStack` chooses between `transformMgr` and the selection
//!   handlers.

use gpui::{AnyElement, IntoElement, div, prelude::*, px};

use crate::arrange_panels::{
    ArrangeActions, ArrangeState, DuplicateActions, DuplicateState, arrange_body, duplicate_body,
};
use crate::organic_cut_panel::{OrganicCutActions, OrganicCutView, organic_cut_body};
use crate::smoothing_panel::{SmoothingActions, SmoothingState, smoothing_body};
use crate::transform_panel::{
    OrientationActions, OrientationState, TransformActions, TransformState, orientation_body,
    transform_body,
};

/// `transformMgr.transformMode`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransformMode {
    Transform,
    Smoothing,
    OrganicCut,
    Arrange,
    Duplicate,
}

/// One panel the PREPARE stack can mount.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PreparePanelId {
    Models,
    Transform,
    Orientation,
    Smoothing,
    OrganicCut,
    Arrange,
    Duplicate,
}

impl PreparePanelId {
    /// The panel's stable id, for [`crate::panels::PanelDef::new`].
    pub fn id(self) -> &'static str {
        match self {
            PreparePanelId::Models => "prepare-models",
            PreparePanelId::Transform => "prepare-transform-controls",
            PreparePanelId::Orientation => "prepare-orientation",
            PreparePanelId::Smoothing => "prepare-smoothing-settings",
            PreparePanelId::OrganicCut => "prepare-organic-cut-panel",
            PreparePanelId::Arrange => "prepare-arrange-panel",
            PreparePanelId::Duplicate => "prepare-duplicate-panel",
        }
    }

    /// The panel's title, as the source's `CardHeader` renders it.
    pub fn title(self) -> &'static str {
        match self {
            PreparePanelId::Models => "Models",
            PreparePanelId::Transform => "Transform",
            PreparePanelId::Orientation => "Auto Orientation (Beta)",
            PreparePanelId::Smoothing => "Mesh Smoothing",
            PreparePanelId::OrganicCut => "Split",
            PreparePanelId::Arrange => "Arrange",
            PreparePanelId::Duplicate => "Duplicate",
        }
    }
}

/// The scene and tool flags the stack gates on.
#[derive(Clone, Debug)]
pub struct PrepareStackState {
    /// `scene.geom` — the active model has readable geometry.
    pub has_geometry: bool,
    /// `scene.models.length`.
    pub model_count: usize,
    pub transform_mode: TransformMode,
    /// The tool rail's `Models` entry: the list stays mounted, it just is not
    /// shown.
    pub models_panel_visible: bool,
    /// The model list is collapsible only while the tool rail is a bar.
    pub models_panel_collapsible: bool,
    /// `showEmptySceneDialog || importOverlayState.active`.
    pub models_dimmed: bool,
    /// `modelStatsBottomClearancePx`.
    pub models_bottom_clearance_px: f32,
    /// `orientationPanel !== null` — the Auto Orientation experiment is on.
    pub orientation_enabled: bool,
}

impl Default for PrepareStackState {
    fn default() -> Self {
        Self {
            has_geometry: false,
            model_count: 0,
            transform_mode: TransformMode::Transform,
            models_panel_visible: true,
            models_panel_collapsible: true,
            models_dimmed: false,
            models_bottom_clearance_px: 0.0,
            orientation_enabled: false,
        }
    }
}

impl PrepareStackState {
    /// The panels the source mounts, in order.
    ///
    /// `ModelsPanel` is always mounted (its `hidden` prop only hides it), and
    /// each tool panel is gated on the scene and the active mode exactly as the
    /// source's `{scene.geom && transformMgr.transformMode === '…' && …}` guards
    /// are.
    pub fn visible_panels(&self) -> Vec<PreparePanelId> {
        let mut panels = Vec::new();
        // Mounted unconditionally; `hidden` is a prop, not a mount guard.
        panels.push(PreparePanelId::Models);

        match self.transform_mode {
            TransformMode::Transform => {
                if self.has_geometry {
                    panels.push(PreparePanelId::Transform);
                    if self.orientation_enabled {
                        panels.push(PreparePanelId::Orientation);
                    }
                }
            }
            TransformMode::Smoothing => {
                if self.has_geometry {
                    panels.push(PreparePanelId::Smoothing);
                }
            }
            TransformMode::OrganicCut => {
                if self.has_geometry {
                    panels.push(PreparePanelId::OrganicCut);
                }
            }
            TransformMode::Arrange => {
                if self.model_count > 0 {
                    panels.push(PreparePanelId::Arrange);
                }
            }
            TransformMode::Duplicate => {
                if self.model_count > 0 {
                    panels.push(PreparePanelId::Duplicate);
                }
            }
        }
        panels
    }
}

/// Everything the stack's bodies draw from.
///
/// The Models body is the caller's (`panel_content::models_body`); every other
/// body is built from the state/actions pair below.
pub struct PrepareStack {
    pub state: PrepareStackState,
    pub transform: TransformState,
    pub transform_actions: TransformActions,
    pub orientation: OrientationState,
    pub orientation_actions: OrientationActions,
    pub smoothing: SmoothingState,
    pub smoothing_actions: SmoothingActions,
    pub arrange: ArrangeState,
    pub arrange_actions: ArrangeActions,
    pub duplicate: DuplicateState,
    pub duplicate_actions: DuplicateActions,
    pub organic: OrganicCutView,
    pub organic_actions: OrganicCutActions,
    /// The Models body, built by the caller from `panel_content::models_body`.
    pub models_body: Option<AnyElement>,
}

impl PrepareStack {
    /// The body for one mounted panel, or `None` when the caller supplied no
    /// Models body.
    ///
    /// Takes `&mut self` because an [`AnyElement`] is not cloneable: the Models
    /// body is handed over once.
    pub fn body(&mut self, panel: PreparePanelId) -> Option<AnyElement> {
        match panel {
            PreparePanelId::Models => self.models_body.take(),
            PreparePanelId::Transform => Some(
                transform_body(&self.transform, &self.transform_actions).into_any_element(),
            ),
            PreparePanelId::Orientation => Some(
                orientation_body(&self.orientation, &self.orientation_actions)
                    .into_any_element(),
            ),
            PreparePanelId::Smoothing => Some(
                smoothing_body(&self.smoothing, &self.smoothing_actions).into_any_element(),
            ),
            PreparePanelId::OrganicCut => Some(
                organic_cut_body(&self.organic, &self.organic_actions).into_any_element(),
            ),
            PreparePanelId::Arrange => {
                Some(arrange_body(&self.arrange, &self.arrange_actions).into_any_element())
            }
            PreparePanelId::Duplicate => Some(
                duplicate_body(&self.duplicate, &self.duplicate_actions).into_any_element(),
            ),
        }
    }
}

/// The stack's bodies as one column, `PANEL_GAP` apart.
///
/// A standalone rendering for a shell that wants the whole group in one element;
/// a shell that docks panels individually should call [`PrepareStackState::visible_panels`]
/// and [`PrepareStack::body`] instead.
pub fn prepare_stack(stack: &mut PrepareStack) -> impl IntoElement {
    let mut column = div().flex().flex_col().gap(px(crate::panels::PANEL_GAP));
    for panel in stack.state.visible_panels() {
        if let Some(body) = stack.body(panel) {
            column = column.child(body);
        }
    }
    column
}
