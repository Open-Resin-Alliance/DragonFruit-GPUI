//! Ported from `src/components/gizmo/constants.ts`.
//!
//! Only the constants the port actually uses are carried over. `GIZMO_LIGHTING`
//! has no counterpart here — the source floats a point light over each handle to
//! light the model, which the port's single headlight has no way to express —
//! and the move-plane colours (`xyPlane`/`xzPlane`/`yzPlane`) are unused because
//! `showMovePlanes` is off in the app's own gizmo (`DEFAULT_GIZMO_CONFIG`).

use crate::gizmo::types::Axis;

/// `ScreenSpaceGizmo`'s default `scaleFactor`: the gizmo's world size is this
/// fraction of the camera-to-gizmo distance, which is what keeps it the same
/// size on screen at every zoom level.
pub const SCALE_FACTOR: f32 = 0.04;

/// `#rrggbb` to linear RGB, the conversion three.js applies to a colour literal
/// on its way into the working colour space. The viewport's shaders write the
/// scene target in sRGB, so gizmo colours go through the same trip the source's
/// material colours do.
pub fn linear(hex: u32) -> [f32; 3] {
    let channel = |shift: u32| {
        let value = ((hex >> shift) & 0xff) as f32 / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    [channel(16), channel(8), channel(0)]
}

// `GIZMO_COLORS`. Only the entries a drawn handle consumes are defined: the
// port draws no X/Y ring, so their `*Ring` colours and the per-axis `start`
// colours (which only the ring diamonds use) have no consumer. The arrow shafts
// take their gradient from `SHAFT_GRADIENT` below, exactly as the source's
// `getCachedMoveShaftGeometry` does.
pub const X_AXIS_END: u32 = 0xff3120;
pub const Y_AXIS_END: u32 = 0x00ff00;
pub const Z_AXIS_END: u32 = 0x1596ff;
pub const Z_AXIS_START: u32 = 0x00aaff;
pub const Z_RING: u32 = 0x0000ff;
pub const CENTER: u32 = 0xffffff;
pub const HOVER: u32 = 0xffaa00;
pub const ACTIVE: u32 = 0xffffff;
/// `dimmedColor` in `GizmoMove`/`GizmoScale`/`GizmoRotation`.
pub const DIMMED: u32 = 0xcccccc;

/// `getCachedMoveShaftGeometry`'s vertex-colour ramp: a pure centre colour that
/// holds for the first third, then a secondary colour at the tip. Deliberately
/// *not* the same pair as `GIZMO_COLORS.*Axis` — the head uses those.
pub const SHAFT_GRADIENT: [(u32, u32); 3] = [
    (0xff0000, 0xff9900),
    (0x0ce300, 0xffcc00),
    (0x0000ff, 0x1596ff),
];

/// The shaft's start/end colours for one axis, in `Axis::ALL` order.
pub fn shaft_gradient(axis: Axis) -> (u32, u32) {
    SHAFT_GRADIENT[axis.index()]
}

/// The arrow head / scale handle colour for one axis (`GIZMO_COLORS.*Axis.end`).
pub fn axis_end(axis: Axis) -> u32 {
    match axis {
        Axis::X => X_AXIS_END,
        Axis::Y => Y_AXIS_END,
        Axis::Z => Z_AXIS_END,
    }
}

/// `GIZMO_SIZES`, in gizmo units — the units the screen-space scale multiplies.
pub const CENTER_RADIUS: f32 = 0.6;
pub const ARROW_SHAFT_LENGTH: f32 = 6.0;
pub const ARROW_HEAD_RADIUS: f32 = 0.24;
pub const ARROW_HEAD_LENGTH: f32 = 0.6;
pub const RING_MAJOR_RADIUS: f32 = 4.8;
pub const RING_MINOR_RADIUS: f32 = 0.12;
pub const RING_DIAMOND_RADIUS: f32 = 0.48;
pub const SCALE_LINE_LENGTH: f32 = 2.3;
pub const SCALE_HEXAGON_RADIUS: f32 = 0.5;
pub const SCALE_HEXAGON_DEPTH: f32 = 0.3;

/// `GizmoMove`'s own shaft radius: `Math.max(0.008, 0.02 * thicknessScale)`. The
/// source ignores `GIZMO_SIZES.arrowShaftRadius` (0.08) for the shaft it draws.
pub const ARROW_SHAFT_RADIUS: f32 = 0.02;

/// The arrow's invisible pick sphere: `Math.max(0.14, headRadius * 2.35)`.
pub const ARROW_PICK_RADIUS: f32 = 0.564;

/// `GizmoCenter`'s `hoverRadiusPx`: the centre nub answers from this far out,
/// measured in pixels so it never shrinks when the camera pulls back.
pub const CENTER_PICK_PX: f32 = 24.0;

/// `ROTATION_CENTER_SINGULARITY_PX`: inside this radius of the projected centre
/// a pointer angle is pixel quantisation rather than intent, so the sample is
/// dropped and the previous angle stands.
pub const ROTATION_CENTER_SINGULARITY_PX: f32 = 8.0;

/// `DIAL_MOVE_SLOP_PX`: travel below this keeps the sweep at zero, so a press
/// that never meant to rotate does not commit a fraction of a degree.
pub const DIAL_MOVE_SLOP_PX: f32 = 3.0;

/// `SceneCanvas`'s orbit-vs-click distinction: a right button that travelled
/// less than this is a context-menu click, more is an orbit. The source only
/// tracks "the camera moved at all"; a pixel threshold is what
/// `panels::list`'s right-gesture uses here (`RIGHT_DRAG_THRESHOLD_SQ = 36`), so
/// the same 6px is used in both places.
pub const CLICK_SLOP_PX: f32 = 6.0;
