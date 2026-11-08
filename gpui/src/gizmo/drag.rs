//! The drag state machine: one gesture, one handle, three entity fields.
//!
//! Each handle family keeps the maths its source component used:
//!
//! * **arrows** — `GizmoMove`'s closest-approach parameter between the pointer
//!   ray and the drag axis, measured against the gizmo centre captured at press
//!   so frame-to-frame numbers stay in one frame while the model moves;
//! * **the centre nub** — `GizmoCenter`'s world-XY plane hit, Z pinned;
//! * **the ring** — `GizmoRotation`/`rotationDialModel`'s ray-to-ring-plane
//!   intersection, turned into a sweep from the angle the handle was grabbed
//!   at, with the centre singularity and press slop the source keeps;
//! * **the scale hexagons** — `GizmoScale`'s screen-distance ratio along the
//!   press direction, clamped to 0.01…100, applied to the scale the drag found.
//!
//! The protractor dial's magnet is not ported: there is no dial overlay to
//! display it against, and an invisible magnet would read as the ring sticking.

use glam::{Vec2, Vec3};

use super::constants::{DIAL_MOVE_SLOP_PX, ROTATION_CENTER_SINGULARITY_PX};
use super::layout::{axis_param, shortest_angle_delta, wrap_angle, View};
use super::types::{Handle, Transform};

/// A live gesture on one handle.
pub struct Drag {
    pub handle: Handle,
    /// The gizmo centre captured at press. Rotation and scale keep their maths
    /// anchored here for the whole gesture, matching `ScreenSpaceGizmo`'s
    /// `lockedDragPosition`, so the drag cannot chase the model it is moving.
    pub anchor: Vec3,
    /// The live transform, seeded with the pre-drag one.
    transform: Transform,
    /// The transform the drag found, so Escape can put it back.
    base: Transform,
    /// Pointer position at press, in pixels.
    press: Vec2,
    /// Translate: the last axis parameter.
    axis_param: f32,
    /// Rotate: the last ring-local angle, in radians.
    last_angle: f32,
    /// Scale: the screen distance and direction from the centre at press.
    start_distance: f32,
    start_direction: Vec2,
    start_scale: f32,
    /// Scale: the model's underside at press, in world Z, and its local minimum
    /// Z, so `preserveModelBottom`'s rule can be restated here.
    base_bottom: Option<(f32, f32)>,
    /// Centre: the last world point on the drag plane.
    last_point: Option<Vec3>,
}

impl Drag {
    /// Start a gesture, or `None` when this handle cannot start from a press
    /// here (a ring grabbed exactly edge-on, where the ray misses its plane).
    ///
    /// `mesh_min_z` is the model's local minimum Z — the same `mesh_min_z` the
    /// transform panel drops and lifts with — used to keep the model's underside
    /// where it was while scaling.
    pub fn begin(
        view: &View<'_>,
        pointer: Vec2,
        handle: Handle,
        base: Transform,
        mesh_min_z: Option<f32>,
    ) -> Option<Self> {
        let mut drag = Self {
            handle,
            anchor: view.center,
            transform: base,
            base,
            press: pointer,
            axis_param: 0.0,
            last_angle: 0.0,
            start_distance: 1.0,
            start_direction: Vec2::X,
            start_scale: base.scale,
            base_bottom: None,
            last_point: None,
        };

        match handle {
            Handle::Axis(axis) => {
                let (origin, direction) = view.ray(pointer);
                drag.axis_param = axis_param(view.center, axis.unit(), origin, direction);
            }
            Handle::Ring(_) => {
                let (_, angle) = view.ring_hit(pointer)?;
                drag.last_angle = angle;
            }
            Handle::Scale(_) => {
                let center_px = view.project(view.center)?;
                let delta = pointer - center_px;
                let distance = delta.length();
                let safe = distance.max(1e-6);
                drag.start_distance = safe;
                drag.start_direction = if distance > 1e-6 {
                    delta / safe
                } else {
                    Vec2::X
                };
                if let Some(min_z) = mesh_min_z {
                    drag.base_bottom = Some((
                        base.translate.z + base.scale * min_z,
                        min_z,
                    ));
                }
            }
            Handle::Center => {
                // A camera lying in the plate plane has no centre drag to give;
                // refusing to start lets the press fall through to the model.
                drag.last_point = Some(drag.plane_point(view, pointer)?);
            }
        }
        Some(drag)
    }

    /// The transform the gesture found, for a cancelled drag.
    pub fn base(&self) -> Transform {
        self.base
    }

    /// The centre the gizmo must be *drawn* at while this drag runs.
    ///
    /// Rotation and scale lock it, as `ScreenSpaceGizmo` does; the move handles
    /// let it follow the model, which is the point of them.
    pub fn locked_center(&self) -> Option<Vec3> {
        match self.handle {
            Handle::Ring(_) | Handle::Scale(_) => Some(self.anchor),
            _ => None,
        }
    }

    /// One pointer sample. `None` means "no usable sample" — keep the previous
    /// transform and draw nothing new.
    pub fn update(&mut self, view: &View<'_>, pointer: Vec2) -> Option<Transform> {
        match self.handle {
            Handle::Axis(axis) => {
                let (origin, direction) = view.ray(pointer);
                let next = axis_param(self.anchor, axis.unit(), origin, direction);
                // The pointer is measured against the *positive* axis even when
                // the arrow was drawn on the camera-facing side: `GizmoMove`'s
                // flip is purely visual, and its drag uses the same
                // world-positive `worldAxisDir`.
                // `MIN_AXIS_DELTA`: below this the pointer has not moved along
                // the axis at all, and the source skips the callback.
                let magnitude = next - self.axis_param;
                if !magnitude.is_finite() || magnitude.abs() < 1e-10 {
                    return None;
                }
                self.transform.translate += axis.unit() * magnitude;
                self.axis_param = next;
            }
            Handle::Ring(_) => {
                // `DIAL_MOVE_SLOP_PX`: a press that never meant to rotate stays
                // at zero instead of committing a fraction of a degree.
                if pointer.distance(self.press) <= DIAL_MOVE_SLOP_PX {
                    return None;
                }
                // `ROTATION_CENTER_SINGULARITY_PX`: right next to the projected
                // centre a single pixel swings the angle by a quadrant.
                if let Some(center_px) = view.project(self.anchor) {
                    if pointer.distance(center_px) < ROTATION_CENTER_SINGULARITY_PX {
                        return None;
                    }
                }
                let (_, angle) = view.ring_hit(pointer)?;
                let delta = shortest_angle_delta(self.last_angle, angle);
                if delta == 0.0 {
                    return None;
                }
                // `SceneCanvas` applies the emitted delta negated, so a positive
                // sweep is a positive Z rotation; the entity stores radians and
                // the panel reads them back wrapped, as a quaternion would.
                self.transform.rotate_z = wrap_angle(self.transform.rotate_z + delta);
                self.last_angle = angle;
            }
            Handle::Scale(_) => {
                let center_px = view.project(self.anchor)?;
                let delta = pointer - center_px;
                let projected = delta.dot(self.start_direction);
                let ratio = (projected / self.start_distance.max(1e-6)).clamp(0.01, 100.0);
                // `applyScaleFactor`'s floor, then the source's uniform branch.
                self.transform.scale = self.start_scale * ratio.max(0.0001);
                if let Some((bottom, min_z)) = self.base_bottom {
                    // `preserveModelBottom`: the model's underside stays put, so
                    // scaling does not sink it into the plate or lift it off.
                    self.transform.translate.z = bottom - self.transform.scale * min_z;
                }
            }
            Handle::Center => {
                let point = self.plane_point(view, pointer)?;
                let previous = self.last_point?;
                let delta = point - previous;
                self.transform.translate.x += delta.x;
                self.transform.translate.y += delta.y;
                self.last_point = Some(point);
            }
        }
        Some(self.transform)
    }

    fn plane_point(&self, view: &View<'_>, pointer: Vec2) -> Option<Vec3> {
        let (origin, direction) = view.ray(pointer);
        if direction.z.abs() < 1e-6 {
            return None;
        }
        let t = (self.anchor.z - origin.z) / direction.z;
        (t > 0.0).then(|| origin + direction * t)
    }
}
