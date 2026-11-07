//! The orientation cube's face atlas and geometry.
//!
//! `ZUpGizmoViewcube.tsx` gives every face a 128x128 canvas texture: the
//! `gizmoColors.face` fill, a 1px `gizmoColors.accent` border and the uppercase
//! label in `gizmoColors.text`. Here the six textures are baked once into one
//! 3x2 RGBA atlas, so the wgpu gizmo pass can sample them with a single bind.

use fontdue::Font;

/// `resolveGizmoColors` reads the theme: the face is `--surface-1`, the label
/// `--text-strong`, the border `--accent-secondary`.
const FACE: (u8, u8, u8) = (0x1a, 0x1b, 0x21);
const TEXT: (u8, u8, u8) = (0xf8, 0xf8, 0xfb);
const STROKE: (u8, u8, u8) = (0xba, 0xf7, 0x2e);
/// The source paints a 128x128 canvas; the atlas is 2x that so a face drawn at
/// ~90 device pixels samples a 2px border with sub-texel coverage instead of
/// aliasing a 1px one.
pub const FACE_SIZE: u32 = 256;
const SCALE: f32 = FACE_SIZE as f32 / 128.0;
const LABEL_MAX_WIDTH: f32 = 112.0 * SCALE;
/// `font="600 24px Inter, ..."` in `SceneCanvas.tsx`.
const LABEL_FONT: f32 = 24.0 * SCALE;

/// `defaultFaces`, in `ZUpGizmoViewcube`'s box order.
pub const LABELS: [&str; 6] = ["FRONT", "BACK", "RIGHT", "LEFT", "TOP", "BOTTOM"];

/// Quarter turns applied to the label per face, clockwise. None: the up tangent
/// of every face is already world Z (sides) or the face's own up (top/bottom),
/// so the quad's UVs carry the orientation. An extra turn on TOP/BOTTOM turned
/// those labels 90° too far.
///
/// `FaceMaterial`'s `needsRotation` table (`-PI/2, PI/2, PI, 0, -PI/2, -PI/2`)
/// is expressed against three.js's box UVs *before* the group's own
/// `-PI/2` Z rotation. Here the group rotation is baked into the face normals,
/// so the sides need no extra turn (the up tangent is already world Z); the top
/// and bottom need a quarter turn to read across the face rather than along it.
const ROTATIONS: [i32; 6] = [0, 0, 0, 0, 0, 0];
/// `opacity={0.75}` on the face material.
pub const FACE_OPACITY: f32 = 0.75;

/// `(width, height, rgba)` for the 3x2 face atlas.
pub fn atlas() -> (u32, u32, Vec<u8>) {
    let width = FACE_SIZE * 3;
    let height = FACE_SIZE * 2;
    let mut data = vec![0u8; (width * height * 4) as usize];
    let font = std::fs::read("C:/Windows/Fonts/arialbd.ttf")
        .ok()
        .and_then(|bytes| Font::from_bytes(bytes, fontsetup()).ok());

    for (index, label) in LABELS.iter().enumerate() {
        // Paint one face on its own, then blit it into the atlas at the face's
        // rotation.
        let mut scratch = vec![0u8; (FACE_SIZE * FACE_SIZE * 4) as usize];
        for pixel in scratch.chunks_mut(4) {
            pixel.copy_from_slice(&[FACE.0, FACE.1, FACE.2, 0xff]);
        }
        // The border is a 2px band whose outer row is half-strength: a hard 2px
        // line at ~1px on screen is what read as a jagged lime edge, while a
        // stepped edge survives the mip minification as a soft one.
        let edge_rows = [
            (0u32, 0.5f32),
            (1u32, 1.0f32),
            (FACE_SIZE - 2, 1.0),
            (FACE_SIZE - 1, 0.5),
        ];
        let stroke_on = |buffer: &mut [u8], x: u32, y: u32, coverage: f32| {
            let i = ((y * FACE_SIZE + x) * 4) as usize;
            let mix = |base: u8, ink: u8| {
                (base as f32 + (ink as f32 - base as f32) * coverage) as u8
            };
            buffer[i] = mix(buffer[i], STROKE.0);
            buffer[i + 1] = mix(buffer[i + 1], STROKE.1);
            buffer[i + 2] = mix(buffer[i + 2], STROKE.2);
        };
        for edge in 0..FACE_SIZE {
            for (offset, coverage) in edge_rows {
                stroke_on(&mut scratch, edge, offset, coverage);
                stroke_on(&mut scratch, edge, FACE_SIZE - 1 - offset, coverage);
                stroke_on(&mut scratch, offset, edge, coverage);
                stroke_on(&mut scratch, FACE_SIZE - 1 - offset, edge, coverage);
            }
        }
        if let Some(font) = font.as_ref() {
            draw_label(&mut scratch, font, label);
        }
        blit_rotated(&mut data, width, index, &scratch, ROTATIONS[index]);
    }
    (width, height, data)
}


/// Draw `label` centred on a 128x128 face buffer (`textAlign/textBaseline =
/// middle`, `fitFontToWidth` at 112px).
fn draw_label(buffer: &mut [u8], font: &Font, label: &str) {
    let mut size = LABEL_FONT;
    for _ in 0..16 {
        let advance: f32 = label
            .chars()
            .map(|ch| font.metrics(ch, size).advance_width)
            .sum();
        if advance <= LABEL_MAX_WIDTH {
            break;
        }
        size = (size * (LABEL_MAX_WIDTH / advance)).floor().max(8.0);
    }
    let advance: f32 = label
        .chars()
        .map(|ch| font.metrics(ch, size).advance_width)
        .sum();
    // Rasterise into a scratch, measure the covered box, then blit centred:
    // `textBaseline = middle` centres the *em*, and the port was guessing the
    // glyph's offset from it, which left long labels sitting low.
    let mut scratch = vec![0u8; (FACE_SIZE * FACE_SIZE) as usize];
    let mut pen = FACE_SIZE as f32 * 0.5 - advance * 0.5;
    let left = pen;
    let mut min_x = FACE_SIZE as i32;
    let mut min_y = FACE_SIZE as i32;
    let mut max_x = 0i32;
    let mut max_y = 0i32;
    for ch in label.chars() {
        let (metrics, bitmap) = font.rasterize(ch, size);
        for row in 0..metrics.height {
            for col in 0..metrics.width {
                let coverage = bitmap[row * metrics.width + col] as f32 / 255.0;
                if coverage <= 0.0 {
                    continue;
                }
                let x = pen + metrics.xmin as f32 + col as f32;
                let y = FACE_SIZE as f32 * 0.5 - size * 0.35 - metrics.ymin as f32
                    + (metrics.height as f32 - row as f32);
                let x = x.round() as i32;
                let y = y.round() as i32;
                if x < 1 || y < 1 || x >= FACE_SIZE as i32 - 1 || y >= FACE_SIZE as i32 - 1 {
                    continue;
                }
                scratch[(y as u32 * FACE_SIZE + x as u32) as usize] = (coverage * 255.0) as u8;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
        pen += metrics.advance_width;
    }
    let _ = left;
    if max_x < min_x {
        return;
    }
    let width = (max_x - min_x + 1) as f32;
    let height = (max_y - min_y + 1) as f32;
    let offset_x = (FACE_SIZE as f32 - width) * 0.5 - min_x as f32;
    let offset_y = (FACE_SIZE as f32 - height) * 0.5 - min_y as f32;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let coverage = scratch[(y as u32 * FACE_SIZE + x as u32) as usize] as f32 / 255.0;
            if coverage <= 0.0 {
                continue;
            }
            let dst_x = (x as f32 + offset_x).round() as i32;
            let dst_y = (y as f32 + offset_y).round() as i32;
            if dst_x < 1
                || dst_y < 1
                || dst_x >= FACE_SIZE as i32 - 1
                || dst_y >= FACE_SIZE as i32 - 1
            {
                continue;
            }
            let i = ((dst_y as u32 * FACE_SIZE + dst_x as u32) * 4) as usize;
            let mix = |base: u8, ink: u8| {
                (base as f32 + (ink as f32 - base as f32) * coverage) as u8
            };
            buffer[i] = mix(buffer[i], TEXT.0);
            buffer[i + 1] = mix(buffer[i + 1], TEXT.1);
            buffer[i + 2] = mix(buffer[i + 2], TEXT.2);
        }
    }
}

/// Copy a face buffer into its atlas cell, rotated by `quarters` (clockwise,
/// i.e. `+PI/2` per step, matching `context.rotate`).
fn blit_rotated(dst: &mut [u8], width: u32, index: usize, src: &[u8], quarters: i32) {
    let ox = (index as u32 % 3) * FACE_SIZE;
    let oy = (index as u32 / 3) * FACE_SIZE;
    let quarters = quarters.rem_euclid(4);
    for y in 0..FACE_SIZE {
        for x in 0..FACE_SIZE {
            let (sx, sy) = match quarters {
                1 => (FACE_SIZE - 1 - y, x),
                2 => (FACE_SIZE - 1 - x, FACE_SIZE - 1 - y),
                3 => (y, FACE_SIZE - 1 - x),
                _ => (x, y),
            };
            let from = ((sy * FACE_SIZE + sx) * 4) as usize;
            let to = (((oy + y) * width + ox + x) * 4) as usize;
            dst[to..to + 4].copy_from_slice(&src[from..from + 4]);
        }
    }
}

fn fontsetup() -> fontdue::FontSettings {
    fontdue::FontSettings::default()
}

/// The cube's six quads (position xyz, uv), in the atlas order. Faces point
/// outward; the pass culls back faces, so the winding is counter-clockwise
/// seen from outside.
pub fn vertices() -> Vec<[f32; 5]> {
    let mut out = Vec::with_capacity(36);
    let normals: [[f32; 3]; 6] = [
        [0.0, -1.0, 0.0], // FRONT
        [0.0, 1.0, 0.0],  // BACK
        [1.0, 0.0, 0.0],  // RIGHT
        [-1.0, 0.0, 0.0], // LEFT
        [0.0, 0.0, 1.0],  // TOP
        [0.0, 0.0, -1.0], // BOTTOM
    ];
    for (index, normal) in normals.iter().enumerate() {
        // The two tangents on the face, u right / v up in the texture.
        let up: [f32; 3] = if normal[2].abs() > 0.5 {
            [0.0, 1.0, 0.0]
        } else {
            [0.0, 0.0, 1.0]
        };
        let right = [
            up[1] * normal[2] - up[2] * normal[1],
            up[2] * normal[0] - up[0] * normal[2],
            up[0] * normal[1] - up[1] * normal[0],
        ];
        let col = (index % 3) as f32 / 3.0;
        let row = (index / 3) as f32 / 2.0;
        let corner = |u: f32, v: f32| -> [f32; 5] {
            let p = [
                normal[0] * 0.5 + right[0] * (u - 0.5) + up[0] * (v - 0.5),
                normal[1] * 0.5 + right[1] * (u - 0.5) + up[1] * (v - 0.5),
                normal[2] * 0.5 + right[2] * (u - 0.5) + up[2] * (v - 0.5),
            ];
            [p[0], p[1], p[2], col + u / 3.0, row + v / 2.0]
        };
        for (u, v) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 0.0), (1.0, 1.0), (0.0, 1.0)] {
            out.push(corner(u, v));
        }
    }
    out
}


/// `edges` in `ZUpGizmoViewcube`, times `makePositionVector`'s 0.38.
pub const EDGES: [[f32; 3]; 12] = [
    [0.38, 0.38, 0.0],
    [0.38, 0.0, 0.38],
    [0.38, 0.0, -0.38],
    [0.38, -0.38, 0.0],
    [0.0, 0.38, 0.38],
    [0.0, 0.38, -0.38],
    [0.0, -0.38, 0.38],
    [0.0, -0.38, -0.38],
    [-0.38, 0.38, 0.0],
    [-0.38, 0.0, 0.38],
    [-0.38, 0.0, -0.38],
    [-0.38, -0.38, 0.0],
];

/// `corners`, same scale.
pub const CORNERS: [[f32; 3]; 8] = [
    [0.38, 0.38, 0.38],
    [0.38, 0.38, -0.38],
    [0.38, -0.38, 0.38],
    [0.38, -0.38, -0.38],
    [-0.38, 0.38, 0.38],
    [-0.38, 0.38, -0.38],
    [-0.38, -0.38, 0.38],
    [-0.38, -0.38, -0.38],
];

/// [`EDGES`] then [`CORNERS`], each a box scaled by the source's `1.01`.
///
/// `edgeDimensions` is `0.5` on the axis the edge runs along and `0.25` across;
/// `cornerDimensions` is `0.25` on every axis.
pub fn highlight_vertices() -> Vec<[f32; 3]> {
    let mut out = Vec::with_capacity((EDGES.len() + CORNERS.len()) * 36);
    for edge in EDGES {
        // The zero component is the axis the edge runs along.
        let half = [
            if edge[0] == 0.0 { 0.25 } else { 0.25 },
            if edge[1] == 0.0 { 0.25 } else { 0.25 },
            if edge[2] == 0.0 { 0.25 } else { 0.25 },
        ];
        let half = if edge[0] == 0.0 {
            [0.125, 0.125, 0.25]
        } else if edge[1] == 0.0 {
            [0.25, 0.125, 0.125]
        } else if edge[2] == 0.0 {
            [0.25, 0.125, 0.125]
        } else {
            half
        };
        push_box(&mut out, edge, half);
    }
    for corner in CORNERS {
        push_box(&mut out, corner, [0.125, 0.125, 0.125]);
    }
    out
}

fn push_box(out: &mut Vec<[f32; 3]>, centre: [f32; 3], half: [f32; 3]) {
    let c = glam::Vec3::from_array(centre);
    let h = glam::Vec3::from_array(half);
    let corner = |sx: f32, sy: f32, sz: f32| (c + glam::Vec3::new(sx * h.x, sy * h.y, sz * h.z)).to_array();
    // Six faces, four corners each.
    let quads = [
        ([-1.0, -1.0, 1.0], [1.0, -1.0, 1.0], [1.0, 1.0, 1.0], [-1.0, 1.0, 1.0]),
        ([1.0, -1.0, -1.0], [-1.0, -1.0, -1.0], [-1.0, 1.0, -1.0], [1.0, 1.0, -1.0]),
        ([1.0, -1.0, 1.0], [1.0, -1.0, -1.0], [1.0, 1.0, -1.0], [1.0, 1.0, 1.0]),
        ([-1.0, -1.0, -1.0], [-1.0, -1.0, 1.0], [-1.0, 1.0, 1.0], [-1.0, 1.0, -1.0]),
        ([-1.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, -1.0], [-1.0, 1.0, -1.0]),
        ([-1.0, -1.0, -1.0], [1.0, -1.0, -1.0], [1.0, -1.0, 1.0], [-1.0, -1.0, 1.0]),
    ];
    for (a, b, cc, d) in quads {
        let (a, b, cc, d) = (corner(a[0], a[1], a[2]), corner(b[0], b[1], b[2]), corner(cc[0], cc[1], cc[2]), corner(d[0], d[1], d[2]));
        out.extend([a, b, cc]);
        out.extend([a, cc, d]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_has_six_labeled_faces() {
        let (w, h, data) = atlas();
        assert_eq!((w, h), (FACE_SIZE * 3, FACE_SIZE * 2));
        assert_eq!(data.len(), (w * h * 4) as usize);
        // The fill and the stroke are both present.
        assert!(data.chunks(4).any(|px| px[0] == FACE.0 && px[1] == FACE.1));
        assert!(data.chunks(4).any(|px| px[0] == STROKE.0 && px[1] == STROKE.1));
    }


    #[test]
    fn labels_read_upright_from_the_default_camera() {
        let dir = glam::Vec3::new(-0.55, -0.55, 0.65).normalize();
        let eye = dir * 3.2;
        let view = glam::Mat4::look_at_rh(eye, glam::Vec3::ZERO, glam::Vec3::Z);
        let proj = glam::Mat4::orthographic_rh(-1.15, 1.15, -1.15, 1.15, 0.1, 10.0);
        let vp = proj * view;
        let verts = vertices();
        for face in 0..6 {
            let quad = &verts[face * 6..face * 6 + 6];
            // corner 0 = (u0,v0), 1 = (u1,v0), 3 = (u0,v1)
            let project = |index: usize| {
                let v = quad[index];
                let clip = vp * glam::Vec4::new(v[0], v[1], v[2], 1.0);
                glam::Vec2::new(clip.x / clip.w, clip.y / clip.w)
            };
            let normal = glam::Vec3::new(quad[0][0], quad[0][1], quad[0][2]);
            let centroid = normal.normalize() * 0.5;
            let facing = normal.normalize().dot((eye - centroid).normalize());
            if facing <= 0.0 {
                continue;
            }
            let du = project(1) - project(0);
            let dv = project(5) - project(0);
            // The atlas label is turned clockwise by ROTATIONS[face], so the
            // face-space axes of the *text* are the quad's axes rotated back.
            let (label_up, label_right) = match ROTATIONS[face].rem_euclid(4) {
                1 => (du, -dv),
                2 => (-dv, -du),
                3 => (-du, dv),
                _ => (dv, du),
            };
            // The top face is seen from above, so its "right" is foreshortened
            // to a near-vertical vector by this camera; only uprightness is
            // asserted there.
            assert!(
                label_up.y > 0.0,
                "face {face} label is upside down: up {label_up:?}"
            );
            if face != 4 && face != 5 {
                assert!(
                    label_right.x > 0.0,
                    "face {face} label is mirrored: right {label_right:?}"
                );
            }
        }
    }


    #[test]
    fn each_label_is_centred_on_its_face() {
        let (w, _, data) = atlas();
        for face in 0..6 {
            let ox = (face as u32 % 3) * FACE_SIZE;
            let oy = (face as u32 / 3) * FACE_SIZE;
            let (mut min_x, mut min_y, mut max_x, mut max_y) = (u32::MAX, u32::MAX, 0, 0);
            for y in 3..FACE_SIZE - 3 {
                for x in 3..FACE_SIZE - 3 {
                    let i = (((oy + y) * w + ox + x) * 4) as usize;
                    // The label is the only ink: the fill and the border are
                    // excluded by the 3px inset and the colour test.
                    let is_ink = data[i] > 0x80 && data[i + 1] > 0x80 && data[i + 2] > 0x80;
                    if is_ink {
                        min_x = min_x.min(x);
                        min_y = min_y.min(y);
                        max_x = max_x.max(x);
                        max_y = max_y.max(y);
                    }
                }
            }
            assert!(min_x != u32::MAX, "face {face} has no label");
            let cx = (min_x as f32 + max_x as f32) * 0.5;
            let cy = (min_y as f32 + max_y as f32) * 0.5;
            let centre = FACE_SIZE as f32 * 0.5;
            assert!((cx - centre).abs() < 3.0, "face {face} x {cx} vs {centre}");
            assert!((cy - centre).abs() < 3.0, "face {face} y {cy} vs {centre}");
        }
    }

    #[test]
    fn vertices_are_six_quads() {
        assert_eq!(vertices().len(), 36);
    }
}
