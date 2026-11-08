//! Screen-space layout of the gizmo: where every handle is on the glass.
//!
//! The source keeps the gizmo a constant size on screen by scaling its root
//! group every frame from the camera (`ScreenSpaceGizmo`). The port does the
//! same arithmetic here, then projects the handle anchors into pixels. Picking
//! and drag resolution both work in that pixel space, which is what makes a
//! handle grabbable at any zoom without a different threshold for each one.

use std::f32::consts::TAU;

use glam::{Vec2, Vec3};
use gpui_core::camera::Camera;

use super::constants::{
    ARROW_SHAFT_LENGTH, RING_MAJOR_RADIUS, SCALE_FACTOR, SCALE_LINE_LENGTH,
};
use super::types::Axis;

/// A camera, a viewport and the gizmo's world centre. Every screen-space
/// question — the gizmo's size, where a handle is, which handle the pointer is
/// over — is asked of this.
pub struct View<'a> {
    pub camera: &'a Camera,
    pub aspect: f32,
    /// The viewport in physical pixels. Pointer positions arriving from GPUI are
    /// in the same space (`window.viewport_size()`).
    pub viewport: Vec2,
    pub center: Vec3,
}

impl<'a> View<'a> {
    pub fn new(
        camera: &'a Camera,
        aspect: f32,
        viewport: Vec2,
        center: Vec3,
    ) -> Self {
        Self {
            camera,
            aspect: aspect.max(0.0001),
            viewport: Vec2::new(viewport.x.max(1.0), viewport.y.max(1.0)),
            center,
        }
    }

    /// `computeScreenSpaceScale` in `ScreenSpaceGizmo.tsx`, verbatim in spirit:
    /// a perspective camera measures to the gizmo's own position, an
    /// orthographic one uses the frustum height it is currently showing.
    ///
    /// The source's ortho form is `(top - bottom) / zoom`; this camera folds the
    /// ortho zoom into `distance` (the frustum is
    /// `±distance * tan(fov/2)`), so the frustum height is the equivalent term.
    pub fn scale(&self) -> f32 {
        if self.camera.orthographic {
            let world_height = 2.0 * self.camera.distance * (self.camera.fov_y * 0.5).tan();
            world_height * SCALE_FACTOR
        } else {
            self.camera.eye().distance(self.center) * SCALE_FACTOR
        }
    }

    /// Pixels per gizmo unit. Because [`Self::scale`] rides the camera distance,
    /// this is independent of zoom: a handle drawn at one gizmo unit is the same
    /// number of pixels at any distance.
    pub fn pixels_per_unit(&self) -> f32 {
        let height = self.viewport.y;
        if self.camera.orthographic {
            height * SCALE_FACTOR
        } else {
            // A world unit at the gizmo's distance spans `height / (2·d·tan(fov/2))`
            // pixels; the gizmo is `d * SCALE_FACTOR` world units per unit.
            height * SCALE_FACTOR / (2.0 * (self.camera.fov_y * 0.5).tan()).max(1e-6)
        }
    }

    /// World point to viewport pixels, top-left origin. `None` when the point is
    /// behind the camera, which is a handle that cannot be seen or grabbed.
    pub fn project(&self, world: Vec3) -> Option<Vec2> {
        let clip = self.camera.view_proj(self.aspect) * world.extend(1.0);
        if clip.w <= 1e-6 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        Some(Vec2::new(
            (ndc.x + 1.0) * 0.5 * self.viewport.x,
            (1.0 - ndc.y) * 0.5 * self.viewport.y,
        ))
    }

    /// Pixels to a world-space ray, through the same projection the viewport
    /// draws with.
    pub fn ray(&self, pointer: Vec2) -> (Vec3, Vec3) {
        let ndc_x = pointer.x / self.viewport.x * 2.0 - 1.0;
        let ndc_y = 1.0 - pointer.y / self.viewport.y * 2.0;
        self.camera.ray(self.aspect, ndc_x, ndc_y)
    }

    /// Which side of the gizmo a handle of this axis is drawn on: `+1` when the
    /// camera is on the positive side of the axis, `-1` otherwise.
    ///
    /// `GizmoMove`/`GizmoScale` put the arrow and the scale box on the camera's
    /// side of the model so they never hide behind it (`pointsTowardCamera`).
    pub fn axis_sign(&self, axis: Axis) -> f32 {
        if axis.unit().dot(self.camera.eye() - self.center) > 0.0 {
            1.0
        } else {
            -1.0
        }
    }

    /// Tip of the move arrow, in world space, on the camera's side.
    pub fn arrow_tip(&self, axis: Axis) -> Vec3 {
        self.center
            + axis.unit() * (self.axis_sign(axis) * ARROW_SHAFT_LENGTH * self.scale())
    }

    /// Centre of the scale handle, in world space, on the camera's side.
    pub fn scale_handle(&self, axis: Axis) -> Vec3 {
        self.center
            + axis.unit() * (self.axis_sign(axis) * SCALE_LINE_LENGTH * self.scale())
    }

    /// A point on the rotation ring at `angle`, in the ring's local frame.
    pub fn ring_point(&self, axis: Axis, angle: f32) -> Vec3 {
        let (u, v) = axis.ring_basis();
        self.center + (u * angle.cos() + v * angle.sin()) * (RING_MAJOR_RADIUS * self.scale())
    }

    /// Where a pointer ray crosses the ring's plane, as `(radius, angle)` in
    /// gizmo units and ring-local radians.
    ///
    /// `rayToRingLocal` in `rotationDialModel.ts`, with the scale divided out by
    /// hand — the source does it with the ring's full inverse world matrix, for
    /// the same reason: the ring's radius is only comparable with
    /// `RING_MAJOR_RADIUS` in gizmo units.
    ///
    /// `None` means no usable sample (a ray parallel to the plane, or a plane
    /// behind the pointer); the caller keeps its previous angle rather than
    /// inventing one, so a grazing ring stalls instead of flinging the model.
    pub fn ring_hit(&self, pointer: Vec2) -> Option<(f32, f32)> {
        let (origin, direction) = self.ray(pointer);
        let normal = self.ring_normal();
        let denom = direction.dot(normal);
        if denom.abs() < 1e-6 {
            return None;
        }
        let t = (self.center - origin).dot(normal) / denom;
        if t <= 0.0 {
            return None;
        }
        let local = origin + direction * t - self.center;
        let (u, v) = self.ring_axis().ring_basis();
        let radius = local.length() / self.scale().max(1e-6);
        Some((radius, wrap_angle(local.dot(v).atan2(local.dot(u)))))
    }

    /// The axis the port rotates about. The entity carries one Z rotation, so
    /// the Z ring is the only one drawn — see the module docs of
    /// [`crate::gizmo`].
    pub fn ring_axis(&self) -> Axis {
        Axis::Z
    }

    fn ring_normal(&self) -> Vec3 {
        self.ring_axis().unit()
    }
}

/// `wrapAngle` in `rotationDialModel.ts`: into `(-PI, PI]`, modular rather than
/// through `atan2` so an angle already in range comes back bit-for-bit.
pub fn wrap_angle(rad: f32) -> f32 {
    let mut wrapped = rad % TAU;
    if wrapped > std::f32::consts::PI {
        wrapped -= TAU;
    } else if wrapped <= -std::f32::consts::PI {
        wrapped += TAU;
    }
    if wrapped == 0.0 {
        0.0
    } else {
        wrapped
    }
}

/// `shortestAngleDelta`: the signed step from one angle to another, the short
/// way round, so crossing the wrap boundary reads as a small step.
pub fn shortest_angle_delta(from: f32, to: f32) -> f32 {
    wrap_angle(to - from)
}

/// `GizmoMove`'s closest-approach parameter between the pointer ray and the
/// drag axis line: the `t` such that `origin + axis * t` is nearest the ray.
///
/// The axis reference point is the gizmo centre captured at press, so the
/// numbers are comparable frame to frame even as the model (and so the gizmo)
/// moves in response to the drag.
pub fn axis_param(origin: Vec3, direction: Vec3, ray_origin: Vec3, ray_direction: Vec3) -> f32 {
    let axis_to_ray = origin - ray_origin;
    let axis_dot_ray = direction.dot(ray_direction);
    let axis_dot_axis_to_ray = direction.dot(axis_to_ray);
    let ray_dot_axis_to_ray = ray_direction.dot(axis_to_ray);
    let denom = 1.0 - axis_dot_ray * axis_dot_ray;

    // Degenerate case: the pointer ray is nearly parallel to the axis, where the
    // closest point is not resolvable and the projection is the honest answer.
    if denom.abs() < 1e-6 {
        return axis_dot_axis_to_ray;
    }
    ((axis_dot_ray * ray_dot_axis_to_ray) - axis_dot_axis_to_ray) / denom
}

/// Distance from a point to a line segment, in whatever space the points live
/// in — used in pixels for handle picking.
pub fn distance_to_segment(point: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let length_sq = ab.length_squared();
    if length_sq < 1e-6 {
        return point.distance(a);
    }
    let t = ((point - a).dot(ab) / length_sq).clamp(0.0, 1.0);
    point.distance(a + ab * t)
}
