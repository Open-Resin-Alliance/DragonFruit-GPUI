//! The gizmo's triangle soup, in world space, for the viewport's unlit pass.
//!
//! The source builds its handles from three.js primitives (`ConeGeometry`,
//! `CylinderGeometry`, `TubeGeometry`, a box, a sphere) under a group that is
//! scaled every frame to hold a constant size on screen. The port has no scene
//! graph on the GPU side: it emits the same primitives as positioned triangles
//! with a colour per vertex, which is the vertex format the viewport's existing
//! unlit pipeline already takes. The screen-space scale is applied here, per
//! vertex, from [`View::scale`], so the geometry can only ever be the size the
//! camera says it is.
//!
//! The vertex buffer is rebuilt only when the key changes (camera pose, model
//! placement, hover, active handle), which is the mesh's caching trick applied
//! to a buffer that is a few thousand vertices rather than a few million.

use std::f32::consts::{FRAC_PI_4, PI, TAU};
use std::rc::Rc;

use glam::Vec3;

use super::constants::{
    axis_end, linear, shaft_gradient, ACTIVE, ARROW_HEAD_LENGTH, ARROW_HEAD_RADIUS,
    ARROW_SHAFT_RADIUS, CENTER, CENTER_RADIUS, DIMMED, HOVER, RING_DIAMOND_RADIUS,
    RING_MAJOR_RADIUS, RING_MINOR_RADIUS, SCALE_HEXAGON_DEPTH, SCALE_HEXAGON_RADIUS, Z_AXIS_END,
    Z_AXIS_START, Z_RING,
};
use super::layout::View;
use super::types::{Axis, Handle};

type Vertex = [f32; 6];

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

/// Every handle's triangles for one pose.
pub fn build(view: &View<'_>, hovered: Option<Handle>, active: Option<Handle>) -> Vec<Vertex> {
    let mut out = Vec::new();
    let scale = view.scale();
    let dimmed_any = active.is_some();

    // Centre nub. `GizmoCenter` draws a disc, a border ring and a halo ring around
    // one pick sphere; a single sphere is what those three read as at this size.
    sphere(
        &mut out,
        view.center,
        CENTER_RADIUS * scale,
        12,
        24,
        handle_color(Handle::Center, hovered, active, dimmed_any, CENTER),
    );

    // Move arrows, on the camera's side of the model.
    for axis in Axis::ALL {
        let handle = Handle::Axis(axis);
        let tip = view.arrow_tip(axis);
        let dir = (tip - view.center).normalize_or_zero();
        if dir == Vec3::ZERO {
            continue;
        }
        let dimmed = dimmed_any && active != Some(handle);
        let shaft_end = tip - dir * (ARROW_HEAD_LENGTH * 0.5 * scale);
        let (start_hex, end_hex) = shaft_gradient(axis);
        let (shaft_start, shaft_end_color) = if dimmed {
            (linear(DIMMED), linear(DIMMED))
        } else {
            (linear(start_hex), linear(end_hex))
        };
        cylinder(
            &mut out,
            view.center,
            shaft_end,
            ARROW_SHAFT_RADIUS * scale,
            10,
            |t| lerp(shaft_start, shaft_end_color, (t - 0.33).max(0.0) / 0.67),
        );

        let head = handle_color(handle, hovered, active, dimmed_any, axis_end(axis));
        // The darker, slightly larger cone behind the head is the source's
        // outline (`scale={1.15}`, colour × 0.3).
        let outline = multiply(head, 0.3);
        let outline_radius = ARROW_HEAD_RADIUS * 1.15 * scale;
        let outline_length = ARROW_HEAD_LENGTH * 1.15 * scale;
        cone(
            &mut out,
            tip - dir * (outline_length * 0.5),
            dir,
            outline_radius,
            outline_length,
            10,
            outline,
        );
        cone(
            &mut out,
            tip - dir * (ARROW_HEAD_LENGTH * 0.5 * scale),
            dir,
            ARROW_HEAD_RADIUS * scale,
            ARROW_HEAD_LENGTH * scale,
            10,
            head,
        );
    }

    // Rotation ring and its double-arrow handle.
    let ring_axis = view.ring_axis();
    let ring_handle = Handle::Ring(ring_axis);
    let ring_color = handle_color(ring_handle, hovered, active, dimmed_any, Z_RING);
    torus(
        &mut out,
        view.center,
        ring_axis.unit(),
        RING_MAJOR_RADIUS * scale,
        RING_MINOR_RADIUS * scale,
        48,
        6,
        ring_color,
    );

    // `positiveAxisMidpointAngle('z')` — the handle sits inside the ring at 45°,
    // between the positive X and Y arrows.
    let (u, v) = ring_axis.ring_basis();
    let angle = FRAC_PI_4;
    let radial = u * angle.cos() + v * angle.sin();
    let tangent = -u * angle.sin() + v * angle.cos();
    let handle_center = view.center + radial * (RING_MAJOR_RADIUS * scale);
    let primary = handle_color(ring_handle, hovered, active, dimmed_any, Z_AXIS_END);
    let secondary = if dimmed_any && active != Some(ring_handle) {
        linear(DIMMED)
    } else if active == Some(ring_handle) {
        linear(ACTIVE)
    } else if hovered == Some(ring_handle) {
        lerp(linear(HOVER), linear(Z_AXIS_START), 0.35)
    } else {
        linear(Z_AXIS_START)
    };
    let cone_radius = RING_DIAMOND_RADIUS * 0.36 * scale;
    let cone_height = RING_DIAMOND_RADIUS * scale;
    let offset = RING_DIAMOND_RADIUS * 0.52 * scale;
    for (direction, color) in [(tangent, primary), (-tangent, secondary)] {
        // Outline first, at 1.08, then the coloured cone inside it.
        let outline_radius = cone_radius * 1.08;
        let outline_height = cone_height * 1.08;
        let outline_offset = offset - outline_height * 0.5;
        cone(
            &mut out,
            handle_center + direction * outline_offset,
            direction,
            outline_radius,
            outline_height,
            12,
            multiply(color, 0.3),
        );
        cone(
            &mut out,
            handle_center + direction * (offset - cone_height * 0.5),
            direction,
            cone_radius,
            cone_height,
            12,
            color,
        );
    }

    // Scale handles: a hexagonal plate facing along each axis, on the camera's
    // side, `scaleLineLength` out from the centre.
    for axis in Axis::ALL {
        let handle = Handle::Scale(axis);
        let color = handle_color(handle, hovered, active, dimmed_any, axis_end(axis));
        hex_prism(
            &mut out,
            view.scale_handle(axis),
            axis.unit(),
            SCALE_HEXAGON_RADIUS * scale,
            SCALE_HEXAGON_DEPTH * scale,
            color,
        );
    }

    out
}

/// The colour a handle takes from its own state: active wins, then any other
/// active handle dims it, then hover, and only then the per-handle base colour.
fn handle_color(
    handle: Handle,
    hovered: Option<Handle>,
    active: Option<Handle>,
    dimmed_any: bool,
    base: u32,
) -> [f32; 3] {
    if active == Some(handle) {
        linear(ACTIVE)
    } else if dimmed_any {
        linear(DIMMED)
    } else if hovered == Some(handle) {
        linear(HOVER)
    } else {
        linear(base)
    }
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

fn push(out: &mut Vec<Vertex>, point: Vec3, color: [f32; 3]) {
    out.push([point.x, point.y, point.z, color[0], color[1], color[2]]);
}

fn quad(
    out: &mut Vec<Vertex>,
    a: Vec3,
    b: Vec3,
    c: Vec3,
    d: Vec3,
    ca: [f32; 3],
    cb: [f32; 3],
    cc: [f32; 3],
    cd: [f32; 3],
) {
    push(out, a, ca);
    push(out, b, cb);
    push(out, c, cc);
    push(out, a, ca);
    push(out, c, cc);
    push(out, d, cd);
}

fn tri(out: &mut Vec<Vertex>, a: Vec3, b: Vec3, c: Vec3, color: [f32; 3]) {
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
    color_at: impl Fn(f32) -> [f32; 3],
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
}

/// A cone with its base at `base` and its apex `height` along `dir`.
fn cone(
    out: &mut Vec<Vertex>,
    base: Vec3,
    dir: Vec3,
    radius: f32,
    height: f32,
    segments: usize,
    color: [f32; 3],
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

fn torus(
    out: &mut Vec<Vertex>,
    center: Vec3,
    normal: Vec3,
    major: f32,
    minor: f32,
    tubular: usize,
    radial: usize,
    color: [f32; 3],
) {
    let (u, v) = frame(normal);
    let point = |i: usize, j: usize| {
        let theta = (i % tubular) as f32 / tubular as f32 * TAU;
        let phi = (j % radial) as f32 / radial as f32 * TAU;
        let ring = u * theta.cos() + v * theta.sin();
        center + ring * (major + minor * phi.cos()) + normal * (minor * phi.sin())
    };
    for i in 0..tubular {
        for j in 0..radial {
            quad(
                out,
                point(i, j),
                point(i + 1, j),
                point(i + 1, j + 1),
                point(i, j + 1),
                color,
                color,
                color,
                color,
            );
        }
    }
}

fn sphere(
    out: &mut Vec<Vertex>,
    center: Vec3,
    radius: f32,
    rings: usize,
    segments: usize,
    color: [f32; 3],
) {
    let point = |i: usize, j: usize| {
        let theta = (i % rings) as f32 / rings as f32 * PI;
        let phi = (j % segments) as f32 / segments as f32 * TAU;
        center
            + Vec3::new(
                theta.sin() * phi.cos(),
                theta.sin() * phi.sin(),
                theta.cos(),
            ) * radius
    };
    for i in 0..rings {
        for j in 0..segments {
            quad(
                out,
                point(i, j),
                point(i + 1, j),
                point(i + 1, j + 1),
                point(i, j + 1),
                color,
                color,
                color,
                color,
            );
        }
    }
}

/// A six-sided plate whose flat faces look along `axis`, as `GizmoScale`'s
/// hexagon does (`CylinderGeometry` with six radial segments, axis rotated onto
/// the handle's axis).
fn hex_prism(
    out: &mut Vec<Vertex>,
    center: Vec3,
    axis: Vec3,
    radius: f32,
    depth: f32,
    color: [f32; 3],
) {
    let (u, v) = frame(axis);
    let half = depth * 0.5;
    let a = center - axis * half;
    let b = center + axis * half;
    for index in 0..6 {
        let a0 = index as f32 / 6.0 * TAU;
        let a1 = (index + 1) as f32 / 6.0 * TAU;
        let o0 = (u * a0.cos() + v * a0.sin()) * radius;
        let o1 = (u * a1.cos() + v * a1.sin()) * radius;
        quad(out, a + o0, a + o1, b + o1, b + o0, color, color, color, color);
        tri(out, b, b + o0, b + o1, color);
        tri(out, a, a + o1, a + o0, color);
    }
}
