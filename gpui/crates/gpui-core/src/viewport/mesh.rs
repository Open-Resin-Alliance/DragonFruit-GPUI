//! Vector helpers and the winding-corrected triangle pushers used by the plate
//! geometry.

pub(super) fn srgb_linear(hex: u32) -> [f32; 3] {
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

/// Push a triangle with per-vertex colours, flipping the winding (and the
/// colour order) when its normal faces inward.
pub(super) fn push_tri_colors(
    out: &mut Vec<[f32; 6]>,
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
    ca: [f32; 3],
    cb: [f32; 3],
    cc: [f32; 3],
    outward: [f32; 3],
) {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = cross(ab, ac);
    let dot = n[0] * outward[0] + n[1] * outward[1] + n[2] * outward[2];
    let (b, c, cb, cc) = if dot < 0.0 {
        (c, b, cc, cb)
    } else {
        (b, c, cb, cc)
    };
    for (point, color) in [(a, ca), (b, cb), (c, cc)] {
        out.push([point[0], point[1], point[2], color[0], color[1], color[2]]);
    }
}

/// Push a triangle, flipping the winding when its normal faces inward.
pub(super) fn push_tri(
    out: &mut Vec<[f32; 6]>,
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
    color: [f32; 3],
    outward: [f32; 3],
) {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = cross(ab, ac);
    let dot = n[0] * outward[0] + n[1] * outward[1] + n[2] * outward[2];
    let (b, c) = if dot < 0.0 { (c, b) } else { (b, c) };
    for point in [a, b, c] {
        out.push([point[0], point[1], point[2], color[0], color[1], color[2]]);
    }
}

pub(super) fn normalize(v: [f32; 3]) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length <= 1e-9 {
        [0.0, 0.0, 1.0]
    } else {
        [v[0] / length, v[1] / length, v[2] / length]
    }
}

pub(super) fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
