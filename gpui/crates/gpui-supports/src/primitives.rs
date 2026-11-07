//! Closed triangle meshes for the six support primitives.
//!
//! Every builder returns flat triangles — `[x, y, z]` per corner, no index
//! buffer — because the viewport draws non-indexed geometry and the counts are
//! small (a 12-segment cylinder is 48 triangles).

use glam::Vec3;

use crate::{RADIAL_SEGMENTS, SPHERE_RINGS};

/// A closed cylinder from `from` to `to`, `entry` radius at `from` and `exit` at
/// `to`.
///
/// `InstancedShaftGroup` draws `<cylinderGeometry args={[rTop, rBottom, 1,
/// radialSegments]} />` scaled by the instance matrix; the taper is the source's
/// `shaftTaperGeometry.ts`. Zero at either end gives the contact cone.
pub fn cylinder(from: Vec3, to: Vec3, entry: f32, exit: f32, segments: usize) -> Vec<[f32; 3]> {
    let axis = to - from;
    let height = axis.length();
    if height <= f32::EPSILON || segments < 3 {
        return Vec::new();
    }
    let (basis, bitangent) = ring_basis(axis / height);
    let mut out = Vec::with_capacity(segments * 12);
    for index in 0..segments {
        let a0 = index as f32 / segments as f32 * std::f32::consts::TAU;
        let a1 = (index + 1) as f32 / segments as f32 * std::f32::consts::TAU;
        let (s0, c0) = a0.sin_cos();
        let (s1, c1) = a1.sin_cos();
        let d0 = basis * c0 + bitangent * s0;
        let d1 = basis * c1 + bitangent * s1;
        let bottom0 = from + d0 * entry;
        let bottom1 = from + d1 * entry;
        let top0 = to + d0 * exit;
        let top1 = to + d1 * exit;
        push_quad(&mut out, bottom0, bottom1, top1, top0);
    }
    // Caps, so a shaft end reads solid when the view is at plate level.
    push_fan(&mut out, from, basis, bitangent, entry, segments, true);
    push_fan(&mut out, to, basis, bitangent, exit, segments, false);
    out
}

/// A cone: `base` with `radius`, tapering to `tip` (a cylinder with a zero exit).
pub fn cone(base: Vec3, tip: Vec3, radius: f32, segments: usize) -> Vec<[f32; 3]> {
    cylinder(base, tip, radius, 0.0, segments)
}

/// A UV sphere, `segments` around and [`SPHERE_RINGS`] rings.
pub fn sphere(center: Vec3, radius: f32, segments: usize, rings: usize) -> Vec<[f32; 3]> {
    if rings < 2 || segments < 3 || radius <= 0.0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(segments * rings * 6);
    let point = |ring: usize, segment: usize| {
        let phi = ring as f32 / rings as f32 * std::f32::consts::PI;
        let theta = segment as f32 / segments as f32 * std::f32::consts::TAU;
        center
            + Vec3::new(
                phi.sin() * theta.cos(),
                phi.sin() * theta.sin(),
                phi.cos(),
            ) * radius
    };
    for ring in 0..rings {
        for segment in 0..segments {
            let a = point(ring, segment);
            let b = point(ring, segment + 1);
            let c = point(ring + 1, segment + 1);
            let d = point(ring + 1, segment);
            if ring > 0 {
                out.extend([a.to_array(), b.to_array(), c.to_array()]);
            }
            if ring + 1 < rings {
                out.extend([a.to_array(), c.to_array(), d.to_array()]);
            }
        }
    }
    out
}

/// A flat disk of `radius` centred at `center`, facing `normal`.
///
/// `ContactDisk` draws the pad under a contact point; the source's disk is a
/// `CircleGeometry` with 24 segments.
pub fn disk(center: Vec3, normal: Vec3, radius: f32, segments: usize) -> Vec<[f32; 3]> {
    let normal = normal.normalize_or_zero();
    if radius <= 0.0 || segments < 3 || normal == Vec3::ZERO {
        return Vec::new();
    }
    let (basis, bitangent) = ring_basis(normal);
    push_fan(&mut Vec::new(), center, basis, bitangent, radius, segments, false)
        .into_iter()
        .map(|corner| corner)
        .collect()
}

fn ring_basis(normal: Vec3) -> (Vec3, Vec3) {
    let helper = if normal.z.abs() > 0.9 {
        Vec3::X
    } else {
        Vec3::Z
    };
    let basis = normal.cross(helper).normalize_or_zero();
    (basis, normal.cross(basis))
}

fn push_quad(out: &mut Vec<[f32; 3]>, a: Vec3, b: Vec3, c: Vec3, d: Vec3) {
    out.extend([a.to_array(), b.to_array(), c.to_array()]);
    out.extend([a.to_array(), c.to_array(), d.to_array()]);
}

fn push_fan(
    out: &mut Vec<[f32; 3]>,
    center: Vec3,
    basis: Vec3,
    bitangent: Vec3,
    radius: f32,
    segments: usize,
    flip: bool,
) -> Vec<[f32; 3]> {
    let mut fan = Vec::with_capacity(segments * 3);
    for index in 0..segments {
        let a0 = index as f32 / segments as f32 * std::f32::consts::TAU;
        let a1 = (index + 1) as f32 / segments as f32 * std::f32::consts::TAU;
        let p0 = center + (basis * a0.cos() + bitangent * a0.sin()) * radius;
        let p1 = center + (basis * a1.cos() + bitangent * a1.sin()) * radius;
        if flip {
            fan.extend([center.to_array(), p1.to_array(), p0.to_array()]);
        } else {
            fan.extend([center.to_array(), p0.to_array(), p1.to_array()]);
        }
    }
    if out.is_empty() {
        fan
    } else {
        out.extend(fan.iter().copied());
        Vec::new()
    }
}
