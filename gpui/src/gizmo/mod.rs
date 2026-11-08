//! The transform gizmo, ported from `src/components/gizmo/`
//! (`TransformGizmo.tsx`, `ScreenSpaceGizmo.tsx`, `move/`, `rotate/`,
//! `scale/`, `constants.ts`, `gizmoDragRegistry.ts`, `rotationDialModel.ts`,
//! `types.ts`).
//!
//! The source is a three.js/R3F tree: `TransformGizmo` owns the handle list,
//! `ScreenSpaceGizmo` rescales its root every frame so the handles keep a
//! constant screen size, and `move/`, `rotate/` and `scale/` own one handle
//! family each. This module keeps that split as data plus maths:
//!
//! * [`layout`] — the camera maths. [`View::scale`] is `ScreenSpaceGizmo`'s
//!   `computeScreenSpaceScale` (distance × 0.04 by default), and every handle
//!   anchor is projected into pixels from it. Constant screen size falls out of
//!   this, and so does constant-size picking.
//! * [`pick`] — which handle the pointer is over, by pixel distance to the
//!   projected handle. The source GPU-picks invisible proxies; a pixel test is
//!   the same question with the handles' own screen geometry as the answer.
//! * [`drag`] — one gesture, resolved into `translate` / `rotate_z` / `scale`,
//!   the only three transform fields the editor entity carries.
//! * [`geometry`] — the triangles the viewport's unlit pass draws, rebuilt only
//!   when the camera pose, the model placement or the handle state changes.
//!
//! # PORT DEVIATIONS
//!
//! * **One rotation axis.** The source shows X, Y and Z rings and rotates a full
//!   Euler. The entity has `rotate_z` and nothing else, so only the Z ring is
//!   drawn: a ring that cannot turn is worse than no ring. The X/Y rings are the
//!   single largest thing this port cannot express.
//! * **Uniform scale only.** `GizmoScale` offers three hexagons; with
//!   `uniformScaling` on (the source's default) each of them scales all three
//!   axes. The entity's one `scale` field means that is the only branch that can
//!   exist, so all three hexagons drive it.
//! * **No protractor dial.** `RotationDial`, `rotationDialModel`'s magnet
//!   (`resolveDialAngle`, `DIAL_ANATOMY`, `MAGNET_ENTER_PX`/`EXIT_PX`,
//!   `DIAL_STEP_DEG`, `GIZMO_DIAL_COLORS`) and the snap-angle readout are 2D
//!   overlay work with no GPUI counterpart yet. The sweep itself is the source's
//!   (`rayToRingLocal` + `shortestAngleDelta`), minus the magnet.
//! * **No handle lighting.** `GIZMO_LIGHTING`'s emissive intensities and the
//!   per-handle point lights that wash colour onto the model are not ported; the
//!   viewport has one headlight and the gizmo pass is unlit, exactly like the
//!   source's own `meshBasicMaterial` handles.
//! * **The ring is drawn as a full torus.** The source draws a 90° tube arc that
//!   billboards toward the camera plus a fading back arc, and only its double-arrow
//!   handle is pickable. A full ring in the same plane is stable at every camera
//!   angle, and the whole circumference is the pick target — the alternative is a
//!   single 22px diamond that moves with the camera.
//! * **No retarget damping.** `ScreenSpaceGizmo` eases the gizmo from its old
//!   position when the active model changes (`SWITCH_DAMP_LAMBDA`,
//!   `SWITCH_SNAP_EPSILON`, the 72 ms `AXIS_SUPPRESS_MS` arc-animation hold) and
//!   damps its scale every frame. The port recomputes the pose straight from the
//!   camera and the transform: there is no in-flight transition to smooth, and one
//!   selected model at a time means a retarget is a jump in the transform panel's
//!   own values, not a new object.
//! * **Local space is not wired.** `gizmoFrameRotation` rotates the gizmo into the
//!   model's own frame when the Transform panel's local-space switch is on; the
//!   entity's store flag is not read here, so the port's gizmo is always world
//!   aligned, which is the source's default.
//! * **The gizmo is opaque.** The source dims a non-active handle to 0.15 alpha;
//!   this pass has no alpha, so it uses the source's `dimmedColor` (`#cccccc`)
//!   instead — the same colour, without the blend.

mod constants;
mod drag;
mod geometry;
mod layout;
mod pick;
mod types;

pub use constants::CLICK_SLOP_PX;
pub use drag::Drag;
pub use geometry::GeometryCache;
pub use layout::View;
pub use pick::pick;
pub use types::{Handle, Transform};

use glam::Vec3;
use gpui_core::camera::Camera;
use std::rc::Rc;

/// The gizmo's triangles for this frame, memoised in `cache`.
pub fn vertices(
    cache: &mut Option<GeometryCache>,
    camera: &Camera,
    aspect: f32,
    viewport: glam::Vec2,
    center: Vec3,
    hovered: Option<Handle>,
    active: Option<Handle>,
) -> Rc<Vec<[f32; 6]>> {
    let view = View::new(camera, aspect, viewport, center);
    geometry::cached(cache, &view, hovered, active)
}
