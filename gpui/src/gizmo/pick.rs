//! Picking against the screen-space-projected handles.
//!
//! The source raycasts (and GPU-picks) invisible proxy meshes that it scales
//! with the gizmo. The port has every handle's anchor projected into pixels
//! already, so the same question is answered by distance in pixels: a handle is
//! hit when the pointer is within its own pixel threshold. Pixel thresholds are
//! what make a handle as easy to grab at 20mm as at 200mm, because the handles
//! themselves are drawn the same size on screen at both.

use glam::Vec2;

use super::constants::{
    ARROW_PICK_RADIUS, CENTER_PICK_PX, RING_MINOR_RADIUS, SCALE_HEXAGON_RADIUS,
};
use super::layout::{distance_to_segment, View};
use super::types::{Axis, Handle};

/// A ring is a thin tube; a pick radius three times the drawn thickness is the
/// smallest band that is comfortable with a mouse, and never below 6px so an
/// edge-on ring stays grabbable.
fn ring_pick_px(pixels_per_unit: f32) -> f32 {
    (RING_MINOR_RADIUS * 3.0 * pixels_per_unit).max(6.0)
}

/// `GizmoScale`'s proxy box is `scaleHexagonRadius * 2.3` across; half of that
/// is the distance from its centre to its face, which is the sensible pixel
/// radius for a round hit test.
fn scale_pick_px(pixels_per_unit: f32) -> f32 {
    SCALE_HEXAGON_RADIUS * 2.3 * 0.5 * pixels_per_unit
}

/// The handle under `pointer`, or `None` when the pointer is off the gizmo.
///
/// Ties are resolved by distance, so where an arrow crosses its own ring the
/// nearer one wins. The centre nub is tested too: it is an opaque ball in front
/// of everything else, exactly as the source's pick sphere is.
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

        // Scale handles: a round test at the hexagon's centre.
        if let Some(handle_px) = view.project(view.scale_handle(axis)) {
            consider(
                Handle::Scale(axis),
                pointer.distance(handle_px),
                scale_pick_px(pixels_per_unit),
            );
        }
    }

    // Rotation ring: distance to the projected circle, sampled as a polyline.
    let ring_axis = view.ring_axis();
    let ring_threshold = ring_pick_px(pixels_per_unit);
    const RING_SEGMENTS: usize = 48;
    let ring: Vec<Vec2> = (0..RING_SEGMENTS)
        .filter_map(|index| {
            let angle = index as f32 / RING_SEGMENTS as f32 * std::f32::consts::TAU;
            view.project(view.ring_point(ring_axis, angle))
        })
        .collect();
    if ring.len() > 1 {
        let mut nearest = f32::INFINITY;
        for index in 0..ring.len() {
            let a = ring[index];
            let b = ring[(index + 1) % ring.len()];
            nearest = nearest.min(distance_to_segment(pointer, a, b));
        }
        consider(Handle::Ring(ring_axis), nearest, ring_threshold);
    }

    best.map(|(handle, _)| handle)
}
