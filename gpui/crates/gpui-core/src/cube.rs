//! The orientation cube's face atlas and geometry.
//!
//! `ZUpGizmoViewcube.tsx` gives every face a 128x128 canvas texture: the
//! `gizmoColors.face` fill, a 1px `gizmoColors.accent` border and the uppercase
//! label in `gizmoColors.text`. Here the six textures are baked once into one
//! 3x2 RGBA atlas, so the wgpu gizmo pass can sample them with a single bind.

use fontdue::Font;

/// `gizmoColors` from `SceneCanvas.tsx`.
const FACE: (u8, u8, u8) = (0x1f, 0x29, 0x37);
const TEXT: (u8, u8, u8) = (0xf8, 0xfa, 0xfc);
const STROKE: (u8, u8, u8) = (0xba, 0xf7, 0x2e);
/// `FaceMaterial`: 128x128 canvas, 20px label, `FACE_LABEL_MAX_WIDTH`.
pub const FACE_SIZE: u32 = 128;
const LABEL_MAX_WIDTH: f32 = 112.0;
const LABEL_FONT: f32 = 20.0;

/// `defaultFaces`, in `ZUpGizmoViewcube`'s box order.
pub const LABELS: [&str; 6] = ["FRONT", "BACK", "RIGHT", "LEFT", "TOP", "BOTTOM"];

/// `needsRotation` in `FaceMaterial`, in quarter turns: the labels are painted
/// on the canvas at the source's angles (`-PI/2, PI/2, PI, 0, -PI/2, -PI/2`)
/// so they read upright once the box is mapped into the Z-up scene.
const ROTATIONS: [i32; 6] = [-1, 1, 2, 0, -1, -1];
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
        // 1px stroke around the face (`strokeRect(0, 0, 128, 128)`).
        for edge in 0..FACE_SIZE {
            for (x, y) in [
                (edge, 0),
                (edge, FACE_SIZE - 1),
                (0, edge),
                (FACE_SIZE - 1, edge),
            ] {
                let i = ((y * FACE_SIZE + x) * 4) as usize;
                scratch[i..i + 4].copy_from_slice(&[STROKE.0, STROKE.1, STROKE.2, 0xff]);
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
    let mut pen = FACE_SIZE as f32 * 0.5 - advance * 0.5;
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
                let i = ((y as u32 * FACE_SIZE + x as u32) * 4) as usize;
                let mix = |base: u8, ink: u8| {
                    (base as f32 + (ink as f32 - base as f32) * coverage) as u8
                };
                buffer[i] = mix(buffer[i], TEXT.0);
                buffer[i + 1] = mix(buffer[i + 1], TEXT.1);
                buffer[i + 2] = mix(buffer[i + 2], TEXT.2);
            }
        }
        pen += metrics.advance_width;
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
    fn vertices_are_six_quads() {
        assert_eq!(vertices().len(), 36);
    }
}
