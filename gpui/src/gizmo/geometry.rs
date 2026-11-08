//! The gizmo's triangle soup, in world space, for the viewport's unlit,
//! alpha-blended pass.
//!
//! The source builds its handles from three.js primitives (`ConeGeometry`,
//! `CylinderGeometry`, `TubeGeometry`, a `BoxGeometry` with `EdgesGeometry`, a
//! `CircleGeometry` and two `RingGeometry`s) under a group that is scaled every
//! frame to hold a constant size on screen. The port has no scene graph on the
//! GPU side: it emits the same primitives as positioned triangles with a colour
//! *and an alpha* per vertex, which is the vertex format the viewport's gizmo
//! pipeline takes (`[pos3, rgba]`). The screen-space scale is applied here, per
//! vertex, from [`View::scale`], so the geometry can only ever be the size the
//! camera says it is.
//!
//! The alpha is not decoration: every handle material in the source is
//! `transparent` with its own `opacity` (0.55 for the centre disc, 0.9 for an
//! arrow, 0.72 for a ring at rest, 0.26 of that for the back arc, 0.15 for a
//! handle dimmed by a running gesture), and the ring's dim half only reads as
//! dim because it composites over the model and the grid. Emitting it opaque is
//! what made a 0.26 arc look like a solid band.
//!
//! Every number below is the source's own: see the table in `crate::gizmo`'s
//! module docs for the constant-by-constant comparison.
//!
//! The vertex buffer is rebuilt only when the key changes (camera pose, model
//! placement, hover, active handle), which is the mesh's caching trick applied
//! to a buffer that is a few thousand vertices rather than a few million.

use std::f32::consts::TAU;
use std::rc::Rc;

use glam::Vec3;

use super::constants::{
    axis_end, axis_ring, axis_start, linear, shaft_gradient, ACTIVE, ARROW_ALPHA, ARROW_HEAD_LENGTH,
    ARROW_HEAD_RADIUS, ARROW_SHAFT_RADIUS, CENTER, CENTER_ALPHA, CENTER_BORDER_SCALE,
    CENTER_DISC_SCALE, CENTER_HALO_INNER_SCALE, CENTER_HALO_OUTER_SCALE, CENTER_RADIUS,
    CUBE_EDGE_DIM, CUBE_EDGE_WIDTH, DIAMOND_OUTLINE_DIM, DIAMOND_OUTLINE_SCALE,
    DIAMOND_SECONDARY_OUTLINE_DIM, DIMMED, DIMMED_ALPHA, HALO_ACTIVE_ALPHA, HALO_ALPHA,
    HEAD_ACTIVE_SCALE, HEAD_HOVER_SCALE, HEAD_OUTLINE_DIM, HEAD_OUTLINE_SCALE, HIGHLIGHT_ALPHA,
    HOVER, RING_ACTIVE_ALPHA, RING_ACTIVE_SCALE, RING_ALPHA, RING_BACK_ALPHA,
    RING_DIAMOND_RADIUS, RING_FRONT_ARC, RING_MAJOR_RADIUS, RING_TUBE_RADIUS, SCALE_ALPHA,
    SCALE_HEXAGON_RADIUS, SCALE_HIT_ACTIVE_SCALE, SCALE_HIT_HOVER_SCALE,
};
use super::layout::View;
use super::types::{Axis, Handle};

/// `[x, y, z, r, g, b, a]`, the gizmo pipeline's vertex.
type Vertex = [f32; 7];
/// Linear RGB plus the source's opacity for the handle it belongs to.
type Color = [f32; 4];

/// `getCachedMoveShaftGeometry`'s `CylinderGeometry(..., 8, 1)`.
const SHAFT_SEGMENTS: usize = 8;
/// `getCachedConeGeometry(headRadius, headLength, 8)`.
const HEAD_SEGMENTS: usize = 8;
/// `getCachedConeGeometry(ringDiamondRadius * 0.36, ringDiamondRadius, 16)`.
const DIAMOND_SEGMENTS: usize = 16;
/// The rotation arc is `TubeGeometry(curve, 72, 0.016, 16, false)`. A 0.016-unit
/// tube is sub-pixel, so 24 stations and an 8-sided section trace the same
/// silhouette — and the section stays round enough that the arc reads as a solid
/// hairline rather than a dashed one.
const ARC_SEGMENTS: usize = 24;
const ARC_SIDES: usize = 8;
/// The dim arc is three times as long, so it takes more stations: 32 over 270°
/// is 8.4° per station, a sagitta of ~0.01 gizmo units — under half a pixel at
/// the reference viewport.
const BACK_ARC_SEGMENTS: usize = 32;
/// `CircleGeometry(radius, 32)` and `RingGeometry(..., 32)`.
const CENTER_SEGMENTS: usize = 32;

/// The memoised triangle soup and the state it was built for.
pub struct GeometryCache {
    key: Key,
    vertices: Rc<Vec<Vertex>>,
}

#[derive(Clone, Copy, PartialEq)]
struct Key {
    center: [u32; 3],
    scale: u32,
    eye: [u32; 3],
    hovered: Option<Handle>,
    active: Option<Handle>,
}

impl Key {
    fn new(view: &View<'_>, hovered: Option<Handle>, active: Option<Handle>) -> Self {
        let eye = view.camera.eye();
        Self {
            center: [
                view.center.x.to_bits(),
                view.center.y.to_bits(),
                view.center.z.to_bits(),
            ],
            scale: view.scale().to_bits(),
            eye: [eye.x.to_bits(), eye.y.to_bits(), eye.z.to_bits()],
            hovered,
            active,
        }
    }
}

/// The gizmo's triangles for this frame, rebuilt only when something that
/// shapes them changed.
pub fn cached(
    cache: &mut Option<GeometryCache>,
    view: &View<'_>,
    hovered: Option<Handle>,
    active: Option<Handle>,
) -> Rc<Vec<Vertex>> {
    let key = Key::new(view, hovered, active);
    if let Some(cache) = cache {
        if cache.key == key {
            return cache.vertices.clone();
        }
    }
    let vertices = Rc::new(build(view, hovered, active));
    *cache = Some(GeometryCache {
        key,
        vertices: vertices.clone(),
    });
    vertices
}

/// Every handle's triangles for one pose, in the source's draw order: centre,
/// then arrows, then rings, then scale cubes. The pass has no depth test and
/// blends, so that order is what decides which handle covers which.
pub fn build(view: &View<'_>, hovered: Option<Handle>, active: Option<Handle>) -> Vec<Vertex> {
    let mut out = Vec::new();
    centre(view, hovered, active, &mut out);
    for axis in Axis::ALL {
        arrow(view, axis, hovered, active, &mut out);
    }
    for axis in Axis::ALL {
        ring(view, axis, hovered, active, &mut out);
    }
    for axis in Axis::ALL {
        scale_cube(view, axis, hovered, active, &mut out);
    }
    out
}

/// `GizmoCenter`: a flat disc in the plate plane, a white border ring and — only
/// while it is hovered or held — the halo ring outside them.
///
/// The disc, not a sphere: the source's pick proxy is a sphere but it is
/// invisible (`visible={false}`), and the disc is what the user sees. It faces
/// world `+Z` because the centre drag is the world-XY plane at the model's Z.
///
/// The disc and its border sit at `0.55` opacity (`baseOpacity` in
/// `GizmoCenter`), rising to `1` while the nub is hovered or held — the white
/// that reads as a solid blob when the pass has no blend state.
fn centre(
    view: &View<'_>,
    hovered: Option<Handle>,
    active: Option<Handle>,
    out: &mut Vec<Vertex>,
) {
    let scale = view.scale();
    let handle = Handle::Center;
    let dimmed = active.is_some() && active != Some(handle);
    let lit = hovered == Some(handle) || active == Some(handle);
    let alpha = opacity(CENTER_ALPHA, HIGHLIGHT_ALPHA, lit, dimmed);

    // `centerColor`: dimmed, then active, then hover, then `GIZMO_COLORS.center`.
    let disc = if active == Some(handle) {
        linear(ACTIVE)
    } else if dimmed {
        linear(DIMMED)
    } else if hovered == Some(handle) {
        linear(HOVER)
    } else {
        linear(CENTER)
    };
    // The border ring is `#ffffff` — never the centre colour — unless dimmed.
    let border = if dimmed { linear(DIMMED) } else { linear(CENTER) };

    disc_fan(
        out,
        view.center,
        CENTER_RADIUS * CENTER_DISC_SCALE * scale,
        CENTER_SEGMENTS,
        tint(disc, alpha),
    );
    ring_band(
        out,
        view.center,
        CENTER_RADIUS * CENTER_BORDER_SCALE * scale,
        CENTER_RADIUS * CENTER_DISC_SCALE * scale,
        CENTER_SEGMENTS,
        tint(border, alpha),
    );

    // `isHighlighted && !isDimmed && !isHidden`: the halo only exists while the
    // nub is hovered or held.
    if lit && !dimmed {
        ring_band(
            out,
            view.center,
            CENTER_RADIUS * CENTER_HALO_INNER_SCALE * scale,
            CENTER_RADIUS * CENTER_HALO_OUTER_SCALE * scale,
            CENTER_SEGMENTS,
            tint(
                if active == Some(handle) {
                    linear(ACTIVE)
                } else {
                    linear(HOVER)
                },
                if active == Some(handle) {
                    HALO_ACTIVE_ALPHA
                } else {
                    HALO_ALPHA
                },
            ),
        );
    }
}

/// `GizmoMove`: a gradient shaft from the centre out to `arrowShaftLength`, and
/// a cone straddling the tip with a darker, 1.15× outline behind it.
///
/// The arrow sits on the camera's side of the model (`pointsTowardCamera`), so
/// its direction is the axis signed by the camera's side, and the drag axis
/// stays the positive one — the flip is purely visual.
fn arrow(
    view: &View<'_>,
    axis: Axis,
    hovered: Option<Handle>,
    active: Option<Handle>,
    out: &mut Vec<Vertex>,
) {
    let handle = Handle::Axis(axis);
    let scale = view.scale();
    let dir = axis.unit() * view.axis_sign(axis);
    let tip = view.arrow_tip(axis);
    let dimmed = active.is_some() && active != Some(handle);
    // `GizmoMove`'s `baseOpacity`: 0.9 idle, 1 while held. Hover recolours the
    // head but does not change the opacity.
    let alpha = opacity(ARROW_ALPHA, HIGHLIGHT_ALPHA, active == Some(handle), dimmed);

    // `getCachedMoveShaftGeometry`: radius `Math.max(0.008, 0.02)`, length
    // `arrowShaftLength`, colour held for the first third and then ramped to the
    // secondary colour.
    let (start_hex, end_hex) = shaft_gradient(axis);
    let (shaft_start, shaft_end) = if dimmed {
        (linear(DIMMED), linear(DIMMED))
    } else {
        (linear(start_hex), linear(end_hex))
    };
    cylinder(
        out,
        view.center,
        tip,
        ARROW_SHAFT_RADIUS * scale,
        SHAFT_SEGMENTS,
        |t| tint(lerp(shaft_start, shaft_end, (t - 0.33).max(0.0) / 0.67), alpha),
    );

    // The head group carries `handleScale * hoverScale`; `hoverScale` is 1.18
    // while the handle is held and 1.1 while it is hovered.
    let head_scale = if active == Some(handle) {
        HEAD_ACTIVE_SCALE
    } else if hovered == Some(handle) {
        HEAD_HOVER_SCALE
    } else {
        1.0
    };
    let head = if dimmed {
        linear(DIMMED)
    } else if active == Some(handle) {
        linear(ACTIVE)
    } else if hovered == Some(handle) {
        linear(HOVER)
    } else {
        linear(axis_end(axis))
    };
    // The outline is the same cone at 1.15×, darker: `multiplyScalar(0.3)`, or
    // `0.7` of `dimmedColor` when the handle is dimmed.
    let outline = multiply(head, if dimmed { 0.7 } else { HEAD_OUTLINE_DIM });
    let radius = ARROW_HEAD_RADIUS * head_scale * scale;
    let length = ARROW_HEAD_LENGTH * head_scale * scale;
    cone(
        out,
        tip - dir * (length * HEAD_OUTLINE_SCALE * 0.5),
        dir,
        radius * HEAD_OUTLINE_SCALE,
        length * HEAD_OUTLINE_SCALE,
        HEAD_SEGMENTS,
        tint(outline, alpha),
    );
    cone(
        out,
        tip - dir * (length * 0.5),
        dir,
        radius,
        length,
        HEAD_SEGMENTS,
        tint(head, alpha),
    );
}

/// `GizmoRotation`: the bright 90° arc, the faint 180° back arc, and the
/// double-cone diamond handle, all in the ring's own plane.
///
/// The handle and the bright arc ride the camera's azimuth within the ring
/// (`getCameraAlignedAngle`), so the readable quarter of the ring is always the
/// one facing the viewer — the source animates that per frame, the port derives
/// it from the same camera pose the geometry is built for.
fn ring(
    view: &View<'_>,
    axis: Axis,
    hovered: Option<Handle>,
    active: Option<Handle>,
    out: &mut Vec<Vertex>,
) {
    let handle = Handle::Ring(axis);
    let scale = view.scale();
    let dimmed = active.is_some() && active != Some(handle);
    let held = active == Some(handle);
    // `GizmoRotation`'s `baseOpacity`: 0.72 at rest, 0.95 while held. Hovering
    // recolours the diamond only.
    let alpha = opacity(RING_ALPHA, RING_ACTIVE_ALPHA, held, dimmed);

    // `ringColor`: dimmed, then active, then the ring's own colour.
    let arc_color = if dimmed {
        linear(DIMMED)
    } else if held {
        linear(ACTIVE)
    } else {
        linear(axis_ring(axis))
    };
    let arc_major = RING_MAJOR_RADIUS * scale * if held { RING_ACTIVE_SCALE } else { 1.0 };
    // The tube is never allowed under a pixel and a half across, so a bevelled
    // 0.016-unit arc cannot break up under MSAA and read as a dashed line. The
    // pixel floor is capped so a degenerate viewport (`pixels_per_unit` near
    // zero) cannot turn the ring into a fat torus.
    let pixel = (1.0 / view.pixels_per_unit()).min(0.05);
    let tube = RING_TUBE_RADIUS.max(pixel) * scale;
    let angle = view.camera_aligned_angle(axis);

    // Front arc: `arcAngle = π/2` centred on the handle.
    arc_tube(
        out,
        view.center,
        axis,
        angle - RING_FRONT_ARC * 0.5,
        RING_FRONT_ARC,
        arc_major,
        tube,
        ARC_SEGMENTS,
        ARC_SIDES,
        tint(arc_color, alpha),
    );
    // Back arc: the complement of the bright span — everything the front arc
    // does not cover — so the ring reads as one circle with one bright quarter
    // and the dim part turns with the camera instead of standing still. (The
    // source draws a fixed 180° here, in the ring's own frame, which leaves two
    // 45° gaps that move against the camera; see the module docs.)
    arc_tube(
        out,
        view.center,
        axis,
        angle + RING_FRONT_ARC * 0.5,
        TAU - RING_FRONT_ARC,
        arc_major,
        tube,
        BACK_ARC_SEGMENTS,
        ARC_SIDES,
        tint(arc_color, alpha * RING_BACK_ALPHA),
    );

    // `handleScale * 1.08` while the handle is hovered or held. The handle sits
    // on the ring's own radius even while the active arc grows by 1.02.
    let highlight = if hovered == Some(handle) || held { 1.08 } else { 1.0 };
    let diamond = RING_DIAMOND_RADIUS * scale * highlight;
    let (u, v) = axis.ring_basis();
    let radial = u * angle.cos() + v * angle.sin();
    let tangent = -u * angle.sin() + v * angle.cos();
    let center = view.center + radial * (RING_MAJOR_RADIUS * scale);

    let primary = if dimmed {
        linear(DIMMED)
    } else if held {
        linear(ACTIVE)
    } else if hovered == Some(handle) {
        linear(HOVER)
    } else {
        linear(axis_end(axis))
    };
    let secondary = if dimmed {
        linear(DIMMED)
    } else if held {
        linear(ACTIVE)
    } else if hovered == Some(handle) {
        lerp(linear(HOVER), linear(axis_start(axis)), 0.35)
    } else {
        linear(axis_start(axis))
    };

    let cone_radius = diamond * 0.36;
    let offset = diamond * 0.52;
    let outline_radius = cone_radius * DIAMOND_OUTLINE_SCALE;
    let outline_height = diamond * DIAMOND_OUTLINE_SCALE;
    for (direction, color, dim) in [
        (tangent, primary, DIAMOND_OUTLINE_DIM),
        (-tangent, secondary, DIAMOND_SECONDARY_OUTLINE_DIM),
    ] {
        // The outline mesh is the same cone scaled 1.08 about its own centre,
        // so its base sits further back than the coloured cone's.
        cone(
            out,
            center + direction * (offset - outline_height * 0.5),
            direction,
            outline_radius,
            outline_height,
            DIAMOND_SEGMENTS,
            tint(multiply(color, dim), alpha),
        );
        cone(
            out,
            center + direction * (offset - diamond * 0.5),
            direction,
            cone_radius,
            diamond,
            DIAMOND_SEGMENTS,
            tint(color, alpha),
        );
    }
}

/// `GizmoScale`: a cube of side `scaleHexagonRadius` with its twelve edges drawn
/// over it in a darker tone. `scaleHexagonDepth` and the "hexagon" rotation are
/// both dead in the source — the handle is a `BoxGeometry(1, 1, 1)` scaled
/// uniformly, so only the radius reaches the screen.
fn scale_cube(
    view: &View<'_>,
    axis: Axis,
    hovered: Option<Handle>,
    active: Option<Handle>,
    out: &mut Vec<Vertex>,
) {
    let handle = Handle::Scale(axis);
    let scale = view.scale();
    let dimmed = active.is_some() && active != Some(handle);
    let held = active == Some(handle);
    let highlight = if held {
        SCALE_HIT_ACTIVE_SCALE
    } else if hovered == Some(handle) {
        SCALE_HIT_HOVER_SCALE
    } else {
        1.0
    };
    // `baseOpacity`: 0.9, rising to 1 while hovered or held.
    let alpha = opacity(
        SCALE_ALPHA,
        HIGHLIGHT_ALPHA,
        hovered == Some(handle) || held,
        dimmed,
    );
    let color = if dimmed {
        linear(DIMMED)
    } else if held {
        linear(ACTIVE)
    } else if hovered == Some(handle) {
        linear(HOVER)
    } else {
        linear(axis_end(axis))
    };

    let center = view.scale_handle(axis);
    let half = SCALE_HEXAGON_RADIUS * highlight * 0.5 * scale;
    box_solid(out, center, half, tint(color, alpha));
    // `EdgesGeometry` × `handleColor × 0.3`, drawn after the cube.
    edge_box(
        out,
        center,
        half,
        CUBE_EDGE_WIDTH * scale,
        tint(multiply(color, CUBE_EDGE_DIM), alpha),
        view.camera.eye(),
    );
}

/// The source's `baseOpacity` ladder: a handle another gesture dimmed drops to
/// `DIMMED_ALPHA`, a lit one goes to `lit`, everything else takes `idle`.
fn opacity(idle: f32, lit: f32, lit_state: bool, dimmed: bool) -> f32 {
    if dimmed {
        DIMMED_ALPHA
    } else if lit_state {
        lit
    } else {
        idle
    }
}

fn tint(rgb: [f32; 3], alpha: f32) -> Color {
    [rgb[0], rgb[1], rgb[2], alpha]
}

fn lerp(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn multiply(color: [f32; 3], factor: f32) -> [f32; 3] {
    [color[0] * factor, color[1] * factor, color[2] * factor]
}

/// An orthonormal pair perpendicular to `dir`.
fn frame(dir: Vec3) -> (Vec3, Vec3) {
    let helper = if dir.z.abs() < 0.9 { Vec3::Z } else { Vec3::X };
    let u = dir.cross(helper).normalize_or_zero();
    let v = dir.cross(u).normalize_or_zero();
    (u, v)
}

fn push(out: &mut Vec<Vertex>, point: Vec3, color: Color) {
    out.push([point.x, point.y, point.z, color[0], color[1], color[2], color[3]]);
}

fn quad(
    out: &mut Vec<Vertex>,
    a: Vec3,
    b: Vec3,
    c: Vec3,
    d: Vec3,
    ca: Color,
    cb: Color,
    cc: Color,
    cd: Color,
) {
    push(out, a, ca);
    push(out, b, cb);
    push(out, c, cc);
    push(out, a, ca);
    push(out, c, cc);
    push(out, d, cd);
}

fn tri(out: &mut Vec<Vertex>, a: Vec3, b: Vec3, c: Vec3, color: Color) {
    push(out, a, color);
    push(out, b, color);
    push(out, c, color);
}

/// A closed cylinder whose colour runs along its length. Ring stations are at
/// 0, 1/3 and 1 so the source's flat-then-ramp shaft gradient survives.
fn cylinder(
    out: &mut Vec<Vertex>,
    start: Vec3,
    end: Vec3,
    radius: f32,
    segments: usize,
    color_at: impl Fn(f32) -> Color,
) {
    let axis = end - start;
    let Some(dir) = axis.try_normalize() else {
        return;
    };
    let (u, v) = frame(dir);
    for (t0, t1) in [(0.0f32, 1.0f32 / 3.0), (1.0 / 3.0, 1.0)] {
        let p0 = start + axis * t0;
        let p1 = start + axis * t1;
        let c0 = color_at(t0);
        let c1 = color_at(t1);
        for index in 0..segments {
            let a0 = index as f32 / segments as f32 * TAU;
            let a1 = (index + 1) as f32 / segments as f32 * TAU;
            let o0 = (u * a0.cos() + v * a0.sin()) * radius;
            let o1 = (u * a1.cos() + v * a1.sin()) * radius;
            quad(out, p0 + o0, p0 + o1, p1 + o1, p1 + o0, c0, c0, c1, c1);
        }
    }
    // `CylinderGeometry` closes both ends; the shaft's start cap is hidden by
    // the centre disc, but the tip cap is what the head sits on.
    let c0 = color_at(0.0);
    let c1 = color_at(1.0);
    for index in 0..segments {
        let a0 = index as f32 / segments as f32 * TAU;
        let a1 = (index + 1) as f32 / segments as f32 * TAU;
        let o0 = (u * a0.cos() + v * a0.sin()) * radius;
        let o1 = (u * a1.cos() + v * a1.sin()) * radius;
        tri(out, start, start + o1, start + o0, c0);
        tri(out, end, end + o0, end + o1, c1);
    }
}

/// A cone with its base at `base` and its apex `height` along `dir`.
fn cone(
    out: &mut Vec<Vertex>,
    base: Vec3,
    dir: Vec3,
    radius: f32,
    height: f32,
    segments: usize,
    color: Color,
) {
    let apex = base + dir * height;
    let (u, v) = frame(dir);
    for index in 0..segments {
        let a0 = index as f32 / segments as f32 * TAU;
        let a1 = (index + 1) as f32 / segments as f32 * TAU;
        let o0 = (u * a0.cos() + v * a0.sin()) * radius;
        let o1 = (u * a1.cos() + v * a1.sin()) * radius;
        tri(out, apex, base + o0, base + o1, color);
        tri(out, base, base + o1, base + o0, color);
    }
}

/// A tube swept along `sweep` radians of a circle of radius `major` in the
/// ring's plane, starting at `start`. `TubeGeometry` over the source's arc
/// points, without the Frenet frames a planar circle does not need.
#[allow(clippy::too_many_arguments)]
fn arc_tube(
    out: &mut Vec<Vertex>,
    center: Vec3,
    axis: Axis,
    start: f32,
    sweep: f32,
    major: f32,
    tube: f32,
    segments: usize,
    sides: usize,
    color: Color,
) {
    let (u, v) = axis.ring_basis();
    let normal = axis.unit();
    let point = |station: usize, side: usize| {
        let theta = start + sweep * station as f32 / segments as f32;
        let radial = u * theta.cos() + v * theta.sin();
        let phi = side as f32 / sides as f32 * TAU;
        center + radial * (major + tube * phi.cos()) + normal * (tube * phi.sin())
    };
    for station in 0..segments {
        for side in 0..sides {
            quad(
                out,
                point(station, side),
                point(station + 1, side),
                point(station + 1, side + 1),
                point(station, side + 1),
                color,
                color,
                color,
                color,
            );
        }
    }
}

/// `CircleGeometry`: a fan in the XY plane, normal `+Z`.
fn disc_fan(out: &mut Vec<Vertex>, center: Vec3, radius: f32, segments: usize, color: Color) {
    for index in 0..segments {
        let a0 = index as f32 / segments as f32 * TAU;
        let a1 = (index + 1) as f32 / segments as f32 * TAU;
        let p0 = center + Vec3::new(a0.cos(), a0.sin(), 0.0) * radius;
        let p1 = center + Vec3::new(a1.cos(), a1.sin(), 0.0) * radius;
        tri(out, center, p0, p1, color);
    }
}

/// `RingGeometry(inner, outer, segments)`: a flat annulus in the XY plane.
fn ring_band(
    out: &mut Vec<Vertex>,
    center: Vec3,
    inner: f32,
    outer: f32,
    segments: usize,
    color: Color,
) {
    for index in 0..segments {
        let a0 = index as f32 / segments as f32 * TAU;
        let a1 = (index + 1) as f32 / segments as f32 * TAU;
        let d0 = Vec3::new(a0.cos(), a0.sin(), 0.0);
        let d1 = Vec3::new(a1.cos(), a1.sin(), 0.0);
        quad(
            out,
            center + d0 * inner,
            center + d0 * outer,
            center + d1 * outer,
            center + d1 * inner,
            color,
            color,
            color,
            color,
        );
    }
}

/// The six faces of an axis-aligned box of half-size `half`.
fn box_solid(out: &mut Vec<Vertex>, center: Vec3, half: f32, color: Color) {
    let x = Vec3::X * half;
    let y = Vec3::Y * half;
    let z = Vec3::Z * half;
    let corners = [
        center - x - y - z,
        center + x - y - z,
        center + x + y - z,
        center - x + y - z,
        center - x - y + z,
        center + x - y + z,
        center + x + y + z,
        center - x + y + z,
    ];
    // Counter-clockwise seen from outside; the pass culls nothing, so the
    // winding only has to be consistent, not visible.
    const FACES: [[usize; 4]; 6] = [
        [0, 1, 2, 3], // -Z
        [4, 5, 6, 7], // +Z
        [0, 1, 5, 4], // -Y
        [3, 2, 6, 7], // +Y
        [0, 3, 7, 4], // -X
        [1, 2, 6, 5], // +X
    ];
    for face in FACES {
        quad(
            out,
            corners[face[0]],
            corners[face[1]],
            corners[face[2]],
            corners[face[3]],
            color,
            color,
            color,
            color,
        );
    }
}

/// `EdgesGeometry` on a box: the twelve edges as square-section prisms `width`
/// across, so a `LineSegments` becomes triangles.
/// The cube's outline, with the edges that face away from the camera dropped.
///
/// The source draws `EdgesGeometry` in a depth-tested line pass, so hidden
/// edges are occluded by the cube itself; this pass is depth-always, so the
/// same read is produced by testing each edge's two adjacent face normals
/// against the view direction and skipping an edge only when both face away.
fn edge_box(out: &mut Vec<Vertex>, center: Vec3, half: f32, width: f32, color: Color, eye: Vec3) {
    let t = width * 0.5;
    let to_eye = (eye - center).normalize_or_zero();
    for axis in 0..3 {
        let along = match axis {
            0 => Vec3::X,
            1 => Vec3::Y,
            _ => Vec3::Z,
        };
        let (u, v) = frame(along);
        for su in [-1.0f32, 1.0] {
            for sv in [-1.0f32, 1.0] {
                // This edge sits where the `u * su` and `v * sv` faces meet; it
                // is hidden only when both of those faces point away.
                let visible = (u * su).dot(to_eye) > 0.0 || (v * sv).dot(to_eye) > 0.0;
                if !visible {
                    continue;
                }
                let offset = (u * su + v * sv) * (half - t);
                let a = center + offset - along * half;
                let b = center + offset + along * half;
                let ou = u * t;
                let ov = v * t;
                quad(
                    out,
                    a - ou - ov,
                    a + ou - ov,
                    b + ou - ov,
                    b - ou - ov,
                    color,
                    color,
                    color,
                    color,
                );
                quad(
                    out,
                    a - ou + ov,
                    a + ou + ov,
                    b + ou + ov,
                    b - ou + ov,
                    color,
                    color,
                    color,
                    color,
                );
                quad(
                    out,
                    a - ou - ov,
                    a - ou + ov,
                    b - ou + ov,
                    b - ou - ov,
                    color,
                    color,
                    color,
                    color,
                );
                quad(
                    out,
                    a + ou - ov,
                    a + ou + ov,
                    b + ou + ov,
                    b + ou - ov,
                    color,
                    color,
                    color,
                    color,
                );
            }
        }
    }
}
