//! The transform gizmo, ported from `src/components/gizmo/`
//! (`TransformGizmo.tsx`, `ScreenSpaceGizmo.tsx`, `move/`, `rotate/`,
//! `scale/`, `constants.ts`, `gizmoGeometryCache.ts`, `rotationDialModel.ts`,
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
//! * [`drag`] — one gesture, resolved into `translate` / `rotation` / `scale`,
//!   the three fields of the entity's transform.
//! * [`geometry`] — the triangles the viewport's unlit pass draws, rebuilt only
//!   when the camera pose, the model placement or the handle state changes.
//!
//! # Geometry, against the source
//!
//! Every drawn constant in [`geometry`] is the source's own value, in gizmo
//! units (the units `ScreenSpaceGizmo` scales by `distance × 0.04`):
//!
//! | handle | source | port |
//! |---|---|---|
//! | centre | `CircleGeometry(centerRadius·1.05, 32)` in the XY plane | same disc, 32 segments |
//! | centre border | `RingGeometry(1.0, 1.05, 32)` × `centerRadius`, white | same |
//! | centre halo | `RingGeometry(1.1, 1.32, 32)` × `centerRadius`, hover/active only | same |
//! | arrow shaft | `CylinderGeometry(max(.008, .02), …, arrowShaftLength, 8, 1)` | radius `0.02`, length `6.0`, 8 segments, centre → tip |
//! | arrow head | `ConeGeometry(arrowHeadRadius, arrowHeadLength, 8)` centred on the tip | `0.24 × 0.6`, 8 segments, centred on the tip |
//! | head outline | same cone at `1.15`, colour × `0.3` (× `0.7` when dimmed) | same |
//! | head hover | group scale `1.1` hovered, `1.18` active | same |
//! | ring | `TubeGeometry(90° arc, 72, 0.016, 16)` at `ringMajorRadius` | same arc and radius, 24 × 8, radius floored at one pixel |
//! | ring back arc | `Line`, `π/2…3π/2`, `opacity × 0.26` | same span as a tube, alpha × `0.26` |
//! | diamond | `ConeGeometry(ringDiamondRadius·0.36, ringDiamondRadius, 16)` at `±0.52·r` along the tangent | same |
//! | diamond outline | `scale 1.08`, × `0.3` / × `0.32` | same |
//! | scale handle | `BoxGeometry(1,1,1)` scaled by `scaleHexagonRadius` (**a cube**, not a hexagon) + `EdgesGeometry` × `0.3` | cube of half-size `0.25`, 12 edge prisms |
//! | scale placement | `±scaleLineLength` on the camera's side | same |
//!
//! Every handle also carries the source's own alpha, per vertex: the centre's
//! disc and border at `0.55` (`1.0` while hovered or held), arrows at `0.9`
//! (`1.0` while held), rings at `0.72` (`0.95` while held), scale cubes at
//! `0.9` (`1.0` while hovered or held), any handle another gesture is not using
//! at `0.15`, and the ring's back arc at `0.26` of its ring's opacity. The
//! gizmo pipeline blends, which is what makes the 90° arc the bright part of
//! the ring and the other 180° a wash over the model.
//!
//! `GIZMO_SIZES.ringMinorRadius` (0.12) and `scaleHexagonDepth` (0.3) are
//! declared in the source but reached by no draw call — the ring's tube is a
//! `0.016` literal and the "hexagon" is a cube scaled uniformly — so neither
//! exists here.
//!
//! # PORT DEVIATIONS
//!
//! * **Euler components, not a quaternion.** `SceneCanvas` premultiplies the
//!   group's quaternion by a world-axis rotation and decomposes the result back
//!   into an `Euler('ZYX')`. The entity stores the three angles, so a ring adds
//!   its sweep to its own component — the value the transform panel's row shows.
//!   With the other two components zero the two are identical; with a model
//!   already tilted, the source redistributes the turn across components.
//! * **Uniform scale only.** `GizmoScale` offers three cubes; with
//!   `uniformScaling` on (the source's default, and the only mode the panel
//!   drives) each of them multiplies all three axes. [`types::Transform::scale`]
//!   is a vector so a per-axis mode has somewhere to write; no such mode is
//!   exposed.
//! * **No protractor dial.** `RotationDial`, `rotationDialModel`'s magnet
//!   (`resolveDialAngle`, `DIAL_ANATOMY`, `MAGNET_ENTER_PX`/`EXIT_PX`,
//!   `DIAL_STEP_DEG`, `GIZMO_DIAL_COLORS`) and the snap-angle readout are 2D
//!   overlay work with no GPUI counterpart yet. The sweep itself is the source's
//!   (`rayToRingLocal` + `shortestAngleDelta`), minus the magnet.
//! * **No handle lighting.** `GIZMO_LIGHTING`'s emissive intensities and the
//!   per-handle point lights that wash colour onto the model are not ported; the
//!   viewport has one headlight and the gizmo pass is unlit, exactly like the
//!   source's own `meshBasicMaterial` handles.
//! * **The whole ring is pickable.** The source picks only the diamond handle.
//!   A 22px diamond that moves with the camera is a much smaller target than the
//!   circumference it rides, and the circumference is already drawn.
//! * **No line overlays.** The two `Line` overlays the source draws over the
//!   front arc (and the back arc itself) are screen-space line primitives; the
//!   viewport has none, so the arc is the tube alone. Its radius is floored at
//!   one pixel so a 0.016-unit tube cannot break up under MSAA and read as a
//!   dashed ring — that floor is the triangle stand-in for the source's
//!   `lineWidth 0.92` / `1.34` overlays.
//! * **The viewport never culls and never depth-tests the gizmo.** Both are the
//!   source's own material settings (`depthTest: false`, `depthWrite: false`),
//!   so a handle is never hidden behind the model and triangles that overlap
//!   paint in emission order — which is why the build keeps the source's handle
//!   order. The pass blends (`ALPHA_BLENDING`), so the source's per-handle
//!   opacities composite over the model and the grid as they do in three.js;
//!   the blend happens in the sRGB target rather than on an sRGB canvas, so a
//!   partially transparent handle is one gamma step off.
//! * **No retarget damping.** `ScreenSpaceGizmo` eases the gizmo from its old
//!   position when the active model changes (`SWITCH_DAMP_LAMBDA`,
//!   `SWITCH_SNAP_EPSILON`, the 72 ms `AXIS_SUPPRESS_MS` arc-animation hold) and
//!   damps its scale every frame. The port recomputes the pose straight from the
//!   camera and the transform: there is no in-flight transition to smooth, and
//!   one selected model at a time means a retarget is a jump in the transform
//!   panel's own values, not a new object.
//! * **No view culling.** `computeViewCullState` fades a handle whose axis
//!   points at the camera. Every handle stays at full strength here; the pass is
//!   opaque, so "fade" would have to be a colour, and the arrows already flip to
//!   the camera's side.
//! * **Local space is not wired.** `gizmoFrameRotation` rotates the gizmo into
//!   the model's own frame when the Transform panel's local-space switch is on;
//!   the entity's store flag is not read here, so the port's gizmo is always
//!   world aligned, which is the source's default.
//! * **The gizmo is drawn in the scene pass, blended.** The source renders its
//!   handles as `transparent` materials sorted by `renderOrder`; here the whole
//!   gizmo is one vertex stream, last in the scene pass, drawn with straight
//!   alpha and no depth test. Overlapping handles composite in emission order
//!   rather than back-to-front by depth, which is invisible at these opacities
//!   because every handle already sits in front of the model.

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
) -> Rc<Vec<[f32; 7]>> {
    let view = View::new(camera, aspect, viewport, center);
    geometry::cached(cache, &view, hovered, active)
}
