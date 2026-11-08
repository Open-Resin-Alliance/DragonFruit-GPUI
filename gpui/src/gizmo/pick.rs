//! Picking against the screen-space-projected handles.
//!
//! The source raycasts (and GPU-picks) invisible proxy meshes that it scales
//! with the gizmo. The port has every handle's anchor projected into pixels
//! already, so the same question is answered by distance in pixels: a handle is
//! hit when the pointer is within its own pixel threshold. Pixel thresholds are
//! what make a handle as easy to grab at 20mm as at 200mm, because the handles
//! themselves are drawn the same size on screen at both.
//!
//! The `GizmoRotation` arc is *not* one of those targets, exactly as in the
//! source: only the double-cone diamond carries a pointer handler and a
//! registered pick mesh, so a rotation starts and hovers on the diamond. The
//! ring itself is decoration — a full circle of it is a much easier thing to hit
//! by accident than to grab on purpose, especially where it crosses the arrows.

use glam::Vec2;

use super::constants::{
    ARROW_PICK_RADIUS, CENTER_PICK_PX, DIAMOND_PICK_RADIUS, SCALE_HEXAGON_RADIUS,
};
use super::layout::View;
use super::types::{Axis, Handle};

/// `GizmoScale`'s proxy box is `scaleHexagonRadius * 2.3` across; half of that
/// is the distance from its centre to its face, which is the sensible pixel
/// radius for a round hit test.
fn scale_pick_px(pixels_per_unit: f32) -> f32 {
    SCALE_HEXAGON_RADIUS * 2.3 * 0.5 * pixels_per_unit
}

/// The handle under `pointer`, or `None` when the pointer is off the gizmo.
///
/// Ties are resolved by distance, so where a diamond overlaps an arrow the
/// nearer one wins. The centre nub is tested too: it is an opaque disc in front
/// of everything else, exactly as the source's disc and pick sphere are.
pub fn pick(view: &View<'_>, pointer: Vec2) -> Option<Handle> {
    let center_px = view.project(view.center)?;
    let pixels_per_unit = view.pixels_per_unit();

    let mut best: Option<(Handle, f32)> = None;
    let mut consider = |handle: Handle, distance: f32, threshold: f32| {
        if distance > threshold {
            return;
        }
        if best.is_none_or(|(_, current)| distance < current) {
            best = Some((handle, distance));
        }
    };

    consider(
        Handle::Center,
        pointer.distance(center_px),
        CENTER_PICK_PX,
    );

    for axis in Axis::ALL {
        // Arrows: the source registers one invisible sphere at the arrow tip, so
        // the tip is the target. A round test on the projected tip is the same
        // proxy without a raycast.
        if let Some(tip_px) = view.project(view.arrow_tip(axis)) {
            consider(
                Handle::Axis(axis),
                pointer.distance(tip_px),
                ARROW_PICK_RADIUS * pixels_per_unit,
            );
        }

        // Scale handles: a round test at the cube's centre.
        if let Some(handle_px) = view.project(view.scale_handle(axis)) {
            consider(
                Handle::Scale(axis),
                pointer.distance(handle_px),
                scale_pick_px(pixels_per_unit),
            );
        }

        // Rotation rings: the source's pick sphere sits on the diamond, radius
        // `max(0.18, ringDiamondRadius · 0.9)`. At the reference viewport that is
        // ~19px around a diamond drawn ~9px across, so it is already generous;
        // one pixel is added because the drawn cones run ~2px past the sphere at
        // each tip.
        if let Some(diamond_px) = view.project(view.ring_handle(axis)) {
            consider(
                Handle::Ring(axis),
                pointer.distance(diamond_px),
                DIAMOND_PICK_RADIUS * pixels_per_unit + 1.0,
            );
        }
    }

    best.map(|(handle, _)| handle)
}
