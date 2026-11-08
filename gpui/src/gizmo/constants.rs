//! Ported from `src/components/gizmo/constants.ts`.
//!
//! `GIZMO_LIGHTING` has no counterpart here — the source floats a point light
//! over each handle to light the model, which the port's single headlight has
//! no way to express — and the move-plane colours
//! (`xyPlane`/`xzPlane`/`yzPlane`) are unused because `showMovePlanes` is off in
//! the app's own gizmo (`DEFAULT_GIZMO_CONFIG`).

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

// `GIZMO_COLORS`, in `Axis::ALL` order.

/// `GIZMO_COLORS.*Axis.start` — the gradient's centre colour, and the ring
/// diamond's counter-clockwise cone. `xRing.diamond`/`yRing.diamond`/
/// `zRing.diamond` are the same three literals as `*Axis.start`, so the diamond
/// reads as the axis it belongs to.
const AXIS_START: [u32; 3] = [0xff6600, 0xaaff00, 0x00aaff];
/// `GIZMO_COLORS.*Axis.end` — the arrow head and the scale cube.
const AXIS_END: [u32; 3] = [0xff3120, 0x00ff00, 0x1596ff];
/// `GIZMO_COLORS.*Ring.ring` — the ring arc.
const AXIS_RING: [u32; 3] = [0xff0000, 0x00ff00, 0x0000ff];

pub fn axis_start(axis: Axis) -> u32 {
    AXIS_START[axis.index()]
}

/// The arrow head / scale handle colour for one axis (`GIZMO_COLORS.*Axis.end`).
pub fn axis_end(axis: Axis) -> u32 {
    AXIS_END[axis.index()]
}

/// The rotation arc's colour for one axis (`GIZMO_COLORS.*Ring.ring`).
pub fn axis_ring(axis: Axis) -> u32 {
    AXIS_RING[axis.index()]
}

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

/// `GIZMO_SIZES`, in gizmo units — the units the screen-space scale multiplies.
pub const CENTER_RADIUS: f32 = 0.6;
pub const ARROW_SHAFT_LENGTH: f32 = 6.0;
pub const ARROW_HEAD_RADIUS: f32 = 0.24;
pub const ARROW_HEAD_LENGTH: f32 = 0.6;
pub const RING_MAJOR_RADIUS: f32 = 4.8;
pub const RING_DIAMOND_RADIUS: f32 = 0.48;
pub const SCALE_LINE_LENGTH: f32 = 2.3;
pub const SCALE_HEXAGON_RADIUS: f32 = 0.5;

/// `GizmoMove`'s own shaft radius: `Math.max(0.008, 0.02 * thicknessScale)`. The
/// source ignores `GIZMO_SIZES.arrowShaftRadius` (0.08) for the shaft it draws.
pub const ARROW_SHAFT_RADIUS: f32 = 0.02;

/// The arrow's invisible pick sphere: `Math.max(0.14, headRadius * 2.35)`.
pub const ARROW_PICK_RADIUS: f32 = 0.564;

/// `getCachedRotationArcGeometry`'s tube radius, passed as a literal to
/// `TubeGeometry(curve, segments, 0.016, 16, false)`. `GIZMO_SIZES.ringMinorRadius`
/// (0.12) is declared but never used by the source's ring — a tube seven times
/// thicker is what a full torus at that radius draws as.
pub const RING_TUBE_RADIUS: f32 = 0.016;

/// The front arc's angular span (`getCachedRotationArcPoints('front')`):
/// `±π/4` about the handle, so a quarter of the ring is the bright arc.
pub const RING_FRONT_ARC: f32 = std::f32::consts::FRAC_PI_2;
/// The back arc spans `π` from `π/2` to `3π/2` in the ring's own frame, at
/// `opacity * 0.26` and `lineWidth 0.8`.
pub const RING_BACK_ARC_START: f32 = std::f32::consts::FRAC_PI_2;
pub const RING_BACK_ARC: f32 = std::f32::consts::PI;
/// The back arc's opacity in the source. The gizmo pass is opaque, so the
/// colour is multiplied by it instead — see the module docs of
/// [`crate::gizmo`].
pub const RING_BACK_ALPHA: f32 = 0.26;

/// `GizmoCenter`'s disc, border ring and halo, as multiples of
/// `GIZMO_SIZES.centerRadius`.
pub const CENTER_DISC_SCALE: f32 = 1.05;
pub const CENTER_BORDER_SCALE: f32 = 1.0;
pub const CENTER_HALO_INNER_SCALE: f32 = 1.1;
pub const CENTER_HALO_OUTER_SCALE: f32 = 1.32;

/// `hoverScale` on the move arrow's head group.
pub const HEAD_HOVER_SCALE: f32 = 1.1;
pub const HEAD_ACTIVE_SCALE: f32 = 1.18;
/// `highlightScale` on the scale cube.
pub const SCALE_HIT_HOVER_SCALE: f32 = 1.08;
pub const SCALE_HIT_ACTIVE_SCALE: f32 = 1.14;
/// `scale={ringIsActive ? 1.02 : 1.0}` on the rotation arc.
pub const RING_ACTIVE_SCALE: f32 = 1.02;

/// The darker, slightly larger copy of a head that reads as its outline:
/// `scale={1.15}` and `multiplyScalar(0.3)`.
pub const HEAD_OUTLINE_SCALE: f32 = 1.15;
pub const HEAD_OUTLINE_DIM: f32 = 0.3;
/// The diamond's own outline: `scale={1.08}`, colour × `0.3` (primary) and
/// × `0.32` (secondary).
pub const DIAMOND_OUTLINE_SCALE: f32 = 1.08;
pub const DIAMOND_OUTLINE_DIM: f32 = 0.3;
pub const DIAMOND_SECONDARY_OUTLINE_DIM: f32 = 0.32;
/// `EdgesGeometry` colour on the scale cube: `handleColor × 0.3`.
pub const CUBE_EDGE_DIM: f32 = 0.3;
/// The cube's edges are a 1px `LineSegments`. The viewport has no line
/// primitive, so each edge is a square prism: `0.023` gizmo units is one pixel
/// at a 900px-tall, 45°-fov viewport (`height · 0.04 / (2·tan(fov/2)) ≈ 43 px
/// per unit`), and the whole gizmo scales with the camera, so it stays 1px at
/// every zoom.
pub const CUBE_EDGE_WIDTH: f32 = 0.023;

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
