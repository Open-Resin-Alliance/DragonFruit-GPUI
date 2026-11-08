//! Build-plate metrics, geometry and the baked decal textures.

use crate::stl::Vertex;

use super::mesh::{cross, normalize, push_tri_colors, srgb_linear};

// The scene's own colours (dark theme, `Helpers`).
pub(super) const BUILD_PLATE_COLOR: u32 = 0x323841;
const GRID_MAJOR_COLOR: u32 = 0x4f5560;
const GRID_MINOR_COLOR: u32 = 0x2c3138;

/// `FRONT_MARKER_MAX_TEXT_WIDTH`: the word is fitted to 200 of the texture's
/// 256 pixels.
const FRONT_MARKER_MAX_TEXT_WIDTH: f32 = 200.0;

/// Build-plate decorations, derived exactly as `SceneEnvironment.tsx` does.
/// `width`/`depth` are the build-volume dimensions, not the plate's.
pub(super) struct PlateMetrics {
    pub(super) width: f32,
    pub(super) depth: f32,
    pub(super) center_x: f32,
    pub(super) center_y: f32,
    pub(super) volume_height: f32,
    pub(super) grid_base_size: f32,
    pub(super) grid_divisions: u32,
    pub(super) grid_scale_x: f32,
    pub(super) grid_scale_z: f32,
    pub(super) plate_width: f32,
    pub(super) plate_depth: f32,
    pub(super) plate_thickness: f32,
    pub(super) plate_corner_radius: f32,
    pub(super) plate_center_z: f32,
    pub(super) tab_depth: f32,
    pub(super) tab_back: f32,
    pub(super) tab_front: f32,
    pub(super) marker_width: f32,
    pub(super) marker_depth: f32,
    pub(super) marker_y: f32,
    pub(super) scale: f32,
    pub(super) axis_length: f32,
    pub(super) shaft_radius: f32,
    pub(super) head_radius: f32,
    pub(super) head_length: f32,
    pub(super) label_lift: f32,
    pub(super) label_offset: f32,
    pub(super) label_size: f32,
    pub(super) logo_width: f32,
    pub(super) logo_height: f32,
    pub(super) logo_x: f32,
    pub(super) logo_y: f32,
}

impl PlateMetrics {
    /// `width`/`depth`/`height` are the build volume; `origin_min_x`/`origin_min_y`
    /// are the front-left origin (0,0) or the centred one (-w/2,-d/2).
    pub(super) fn new(width: f32, depth: f32, height: f32, origin_min_x: f32, origin_min_y: f32) -> Self {
        let build_volume_center_x = origin_min_x + width * 0.5;
        let build_volume_center_y = origin_min_y + depth * 0.5;
        let base_size = width.max(depth);
        let base_divisions = (base_size / 5.0).round().clamp(20.0, 240.0);
        let divisions = (base_divisions / 3.0).round().max(8.0) as u32;
        let scale_x = width / base_size;
        let scale_z = depth / base_size;

        let oversize = 3.0;
        let plate_width = width + oversize * 2.0;
        let plate_depth = depth + oversize * 2.0;
        let thickness = (width * 0.05).clamp(1.5, 3.0);
        let corner_radius = 3.0;
        let plate_center_z = -thickness * 0.5 - 0.08;

        let plate_scale = (width / 200.0).clamp(0.16, 1.5);
        let front_scale = (width / 200.0).clamp(0.55, 1.5) * 1.2;
        let tab_depth = 4.2 * front_scale;
        let tab_back = (plate_width - width * 0.072).min(24.0 * front_scale);
        let tab_front = (tab_back - width * 0.018).min(16.0 * front_scale);
        let marker_inset = width * 0.0012;
        let aspect = 256.0 / 72.0;
        let marker_depth =
            (tab_depth - marker_inset * 2.0).min((tab_back - marker_inset * 2.0) / aspect);
        let marker_width = marker_depth * aspect;
        let marker_y = -plate_depth * 0.5 - tab_depth * 0.35;

        let axis_length = 22.0 * plate_scale;
        let shaft_radius = (0.42 * plate_scale).max(0.05);
        let head_radius = (1.3 * plate_scale).max(0.18);
        let head_length = (1.9 * plate_scale).max(0.28);
        let label_lift = (1.0 * plate_scale).max(0.12);
        let label_size = 6.0 * plate_scale;
        // `AxisLabels size={length + 6 * labelScale}`, then
        // `labelOffset = size + 2 * scale`.
        let label_offset = axis_length + 6.0 * plate_scale + 2.0 * plate_scale;

        let logo_aspect = 1772.0 / 304.0;
        let logo_base_width = (width * 0.2).clamp(3.0, 42.0);
        let logo_width = logo_base_width;
        let logo_height = logo_base_width / logo_aspect;
        let logo_inset = (width * 0.08).min(1.6);
        let logo_x = origin_min_x + width - logo_inset - logo_width * 0.5;
        let logo_y = origin_min_y + logo_inset + logo_height * 0.5;

        Self {
            width,
            depth,
            center_x: build_volume_center_x,
            center_y: build_volume_center_y,
            volume_height: height,
            grid_base_size: base_size,
            grid_divisions: divisions,
            grid_scale_x: scale_x,
            grid_scale_z: scale_z,
            plate_width,
            plate_depth,
            plate_thickness: thickness,
            plate_corner_radius: corner_radius,
            plate_center_z,
            tab_depth,
            tab_back,
            tab_front,
            marker_width,
            marker_depth,
            marker_y,
            scale: plate_scale,
            axis_length,
            shaft_radius,
            head_radius,
            head_length,
            label_lift,
            label_offset,
            label_size,
            logo_width,
            logo_height,
            logo_x,
            logo_y,
        }
    }

    /// The plate's top surface, `z0` in `plate_mesh`.
    fn plate_top_z(&self) -> f32 {
        self.plate_center_z + self.plate_thickness * 0.5
    }

    /// `resolvedOriginMinX`.
    fn origin_min_x(&self) -> f32 {
        self.center_x - self.width * 0.5
    }

    /// `resolvedOriginMinY`.
    fn origin_min_y(&self) -> f32 {
        self.center_y - self.depth * 0.5
    }
}

/// The three axis arrows: cylinders with cone heads, coloured with the source's
/// base-to-tip gradients. The shaft spans `0..length` and the cone
/// `length..length + headLength`, each shading over its own length (the source
/// maps the same 16x256 gradient onto both geometries, so the colour restarts at
/// the cone's base).
pub(super) fn axis_arrows(metrics: &PlateMetrics) -> Vec<[f32; 6]> {
    let origin = [-109.44, -61.44, 0.5];
    let length = metrics.axis_length;
    let shaft_radius = metrics.shaft_radius;
    let head_radius = metrics.head_radius;
    let head_length = metrics.head_length;
    let total = length + head_length;
    let segments = 12;
    let axes: [([f32; 3], u32, u32); 3] = [
        ([1.0, 0.0, 0.0], 0x8d232f, 0xff0000),
        ([0.0, 1.0, 0.0], 0x1e6b35, 0x00ff00),
        ([0.0, 0.0, 1.0], 0x21428d, 0x0000ff),
    ];
    let mut out: Vec<[f32; 6]> = Vec::new();
    for (dir, base_hex, tip_hex) in axes {
        let _base = srgb_linear(base_hex);
        let tip = srgb_linear(tip_hex);
        let helper = if dir[2].abs() > 0.9 {
            [1.0, 0.0, 0.0]
        } else {
            [0.0, 0.0, 1.0]
        };
        let u = normalize(cross(dir, helper));
        let v = cross(dir, u);
        let ring = |t: f32, radius: f32, angle: f32| -> [f32; 3] {
            let (sin, cos) = angle.sin_cos();
            [
                origin[0] + dir[0] * t + (u[0] * cos + v[0] * sin) * radius,
                origin[1] + dir[1] * t + (u[1] * cos + v[1] * sin) * radius,
                origin[2] + dir[2] * t + (u[2] * cos + v[2] * sin) * radius,
            ]
        };
        // A pale start fading to the axis colour by the tip. The earlier
        // version painted whole triangles, which read as facets on the
        // 12-sided cone; shading per vertex by position along the axis
        // interpolates smoothly instead.
        let light = [
            tip[0] * 0.45 + 0.55,
            tip[1] * 0.45 + 0.55,
            tip[2] * 0.45 + 0.55,
        ];
        let shade = |t: f32, span: f32| -> [f32; 3] {
            let k = (t / span.max(1e-6)).clamp(0.0, 1.0);
            [
                light[0] + (tip[0] - light[0]) * k,
                light[1] + (tip[1] - light[1]) * k,
                light[2] + (tip[2] - light[2]) * k,
            ]
        };
        let tip_point = [
            origin[0] + dir[0] * total,
            origin[1] + dir[1] * total,
            origin[2] + dir[2] * total,
        ];
        let center_at = |t: f32| -> [f32; 3] {
            [
                origin[0] + dir[0] * t,
                origin[1] + dir[1] * t,
                origin[2] + dir[2] * t,
            ]
        };
        for i in 0..segments {
            let a0 = i as f32 / segments as f32 * std::f32::consts::TAU;
            let a1 = (i + 1) as f32 / segments as f32 * std::f32::consts::TAU;
            // Shaft: `CylinderGeometry(shaftRadius, shaftRadius, length, 12)`,
            // closed at both ends.
            let p00 = ring(0.0, shaft_radius, a0);
            let p01 = ring(0.0, shaft_radius, a1);
            let p10 = ring(length, shaft_radius, a0);
            let p11 = ring(length, shaft_radius, a1);
            let c0 = shade(0.0, length);
            let c1 = shade(length, length);
            let radial = |a: f32, b: [f32; 3], c: [f32; 3]| -> [f32; 3] {
                let center = center_at(a);
                normalize([
                    (b[0] + c[0]) * 0.5 - center[0],
                    (b[1] + c[1]) * 0.5 - center[1],
                    (b[2] + c[2]) * 0.5 - center[2],
                ])
            };
            let out_radial = radial(0.0, p00, p01);
            push_tri_colors(&mut out, p00, p10, p11, c0, c1, c1, out_radial);
            push_tri_colors(&mut out, p00, p11, p01, c0, c1, c0, out_radial);
            // Bottom cap.
            push_tri_colors(
                &mut out,
                center_at(0.0),
                p01,
                p00,
                c0,
                c0,
                c0,
                [-dir[0], -dir[1], -dir[2]],
            );
            // The shaft's top cap is omitted: the cone's base cap sits on the
            // same plane, and two coplanar discs z-fight into a bright rim.
            // Cone: `ConeGeometry(headRadius, headLength, 12)`, base capped.
            let b0 = ring(length, head_radius, a0);
            let b1 = ring(length, head_radius, a1);
            // Absolute positions, so the cone continues the shaft's ramp
            // instead of restarting it (which read as a bright ring at the
            // shaft/cone seam).
            let cone_base = shade(length, total);
            let cone_tip = shade(total, total);
            let center = center_at(length);
            let cone_radial = radial(length, b0, b1);
            push_tri_colors(
                &mut out,
                b0,
                b1,
                tip_point,
                cone_base,
                cone_base,
                cone_tip,
                cone_radial,
            );
            push_tri_colors(
                &mut out,
                center,
                b1,
                b0,
                cone_base,
                cone_base,
                cone_base,
                [-dir[0], -dir[1], -dir[2]],
            );
        }
    }
    out
}

/// Billboard quads for the X/Y/Z letters, one per axis tip.
pub(super) fn sprite_vertices(metrics: &PlateMetrics) -> Vec<[f32; 5]> {
    let origin = [metrics.origin_min_x(), metrics.origin_min_y(), 0.5];
    let label_offset = metrics.label_offset;
    let lift = metrics.label_lift;
    let mut out = Vec::new();
    let mut quad = |center: [f32; 3]| {
        let corners = [
            (0.0, 1.0),
            (1.0, 1.0),
            (1.0, 0.0),
            (0.0, 1.0),
            (1.0, 0.0),
            (0.0, 0.0),
        ];
        for (u, v) in corners {
            out.push([center[0], center[1], center[2], u, v]);
        }
    };
    quad([origin[0] + label_offset, origin[1], origin[2] + lift]);
    quad([origin[0], origin[1] + label_offset, origin[2] + lift]);
    quad([origin[0], origin[1], origin[2] + label_offset + lift]);
    out
}

/// `THREE.GridHelper(baseSize, divisions, color1, color2)`, placed, rotated 90°
/// about X and scaled exactly as the source does it: lines every `step` from
/// `-size/2`, `color1` only on the line at index `divisions / 2`.
pub(super) fn grid_vertices(metrics: &PlateMetrics) -> Vec<[f32; 6]> {
    let size = metrics.grid_base_size;
    let divisions = metrics.grid_divisions;
    let step = size / divisions as f32;
    let half = size * 0.5;
    let center = divisions as f32 * 0.5;
    let major = srgb_linear(GRID_MAJOR_COLOR);
    let minor = srgb_linear(GRID_MINOR_COLOR);
    let z = -0.01;
    let mut out: Vec<[f32; 6]> = Vec::with_capacity((divisions as usize + 1) * 4);
    // Object transform: T(center) * R_x(90°) * S(scaleX, 1, scaleZ).
    let place = |x: f32, k: f32| -> [f32; 3] {
        [
            metrics.center_x + x * metrics.grid_scale_x,
            metrics.center_y - k * metrics.grid_scale_z,
            z,
        ]
    };
    for i in 0..=divisions {
        let k = -half + i as f32 * step;
        let color = if i as f32 == center { major } else { minor };
        let a = place(-half, k);
        let b = place(half, k);
        out.push([a[0], a[1], a[2], color[0], color[1], color[2]]);
        out.push([b[0], b[1], b[2], color[0], color[1], color[2]]);
        let c = place(k, -half);
        let d = place(k, half);
        out.push([c[0], c[1], c[2], color[0], color[1], color[2]]);
        out.push([d[0], d[1], d[2], color[0], color[1], color[2]]);
    }
    out
}

/// `AxisLabels.tsx`: 64x64 canvas, `Bold 42px Arial`, centred, pure colour.
pub(super) fn axis_label_texture(letter: &str, hex: u32) -> (u32, u32, Vec<u8>) {
    let width = 64u32;
    let height = 64u32;
    let mut data = vec![0u8; (width * height * 4) as usize];
    let Ok(bytes) = std::fs::read("C:/Windows/Fonts/arialbd.ttf") else {
        return (width, height, data);
    };
    let Ok(font) = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()) else {
        return (width, height, data);
    };
    let size = 42.0f32;
    let Some(ch) = letter.chars().next() else {
        return (width, height, data);
    };
    let (metrics, bitmap) = font.rasterize(ch, size);
    let color = [
        ((hex >> 16) & 0xff) as f32,
        ((hex >> 8) & 0xff) as f32,
        (hex & 0xff) as f32,
    ];
    let gx = 32.0 - metrics.advance_width * 0.5 + metrics.xmin as f32;
    let gy = 32.0 + size * 0.35 - metrics.ymin as f32 - metrics.height as f32;
    for row in 0..metrics.height {
        for col in 0..metrics.width {
            let coverage = bitmap[row * metrics.width + col] as f32 / 255.0;
            if coverage <= 0.0 {
                continue;
            }
            let px = gx as i32 + col as i32;
            let py = gy as i32 + row as i32;
            if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
                continue;
            }
            let index = ((py as u32 * width + px as u32) * 4) as usize;
            data[index] = (color[0] * coverage) as u8;
            data[index + 1] = (color[1] * coverage) as u8;
            data[index + 2] = (color[2] * coverage) as u8;
            data[index + 3] = (255.0 * coverage) as u8;
        }
    }
    (width, height, data)
}

/// `EdgesGeometry` of the build-volume box: centred on the build volume, from
/// `z = 0` to `z = height`.
pub(super) fn build_volume_edges(
    center_x: f32,
    center_y: f32,
    width: f32,
    depth: f32,
    height: f32,
) -> Vec<[f32; 3]> {
    let hx = width * 0.5;
    let hy = depth * 0.5;
    let corners = [
        [center_x - hx, center_y - hy, 0.0],
        [center_x + hx, center_y - hy, 0.0],
        [center_x + hx, center_y + hy, 0.0],
        [center_x - hx, center_y + hy, 0.0],
        [center_x - hx, center_y - hy, height],
        [center_x + hx, center_y - hy, height],
        [center_x + hx, center_y + hy, height],
        [center_x - hx, center_y + hy, height],
    ];
    let edges = [
        (0, 1), (1, 2), (2, 3), (3, 0),
        (4, 5), (5, 6), (6, 7), (7, 4),
        (0, 4), (1, 5), (2, 6), (3, 7),
    ];
    edges
        .iter()
        .flat_map(|&(a, b)| [corners[a], corners[b]])
        .collect()
}

/// The box's six faces, wound outward; the pipeline culls the front faces so
/// only the far side paints (`side={THREE.BackSide}`).
pub(super) fn build_volume_fill(
    center_x: f32,
    center_y: f32,
    width: f32,
    depth: f32,
    height: f32,
) -> Vec<[f32; 3]> {
    let hx = width * 0.5;
    let hy = depth * 0.5;
    let c = [
        [center_x - hx, center_y - hy, 0.0],
        [center_x + hx, center_y - hy, 0.0],
        [center_x + hx, center_y + hy, 0.0],
        [center_x - hx, center_y + hy, 0.0],
        [center_x - hx, center_y - hy, height],
        [center_x + hx, center_y - hy, height],
        [center_x + hx, center_y + hy, height],
        [center_x - hx, center_y + hy, height],
    ];
    let faces = [
        // bottom (-Z), top (+Z), front (-Y), right (+X), back (+Y), left (-X)
        (0, 2, 1), (0, 3, 2),
        (4, 5, 6), (4, 6, 7),
        (0, 1, 5), (0, 5, 4),
        (1, 2, 6), (1, 6, 5),
        (2, 3, 7), (2, 7, 6),
        (3, 0, 4), (3, 4, 7),
    ];
    let mut out = Vec::with_capacity(faces.len() * 3);
    for (a, b, d) in faces {
        out.push(c[a]);
        out.push(c[b]);
        out.push(c[d]);
    }
    out
}

/// The plate's `ExtrudeGeometry`: the source's shape (rounded rectangle with a
/// trapezoid tab on the front edge), extruded `thickness` and centred on
/// `plateCenterZ`.
pub(super) fn plate_mesh(metrics: &PlateMetrics) -> (Vec<Vertex>, Vec<u32>) {
    let width = metrics.plate_width;
    let depth = metrics.plate_depth;
    let thickness = metrics.plate_thickness;
    let top = metrics.plate_top_z();
    let half_w = width * 0.5;
    let half_d = depth * 0.5;
    let radius = metrics.plate_corner_radius;
    let r = radius
        .min(half_w - 0.2)
        .min(half_d - 0.2)
        .max(0.2);
    let segments: u32 = 18;
    let z0 = top;
    let z1 = top - thickness;

    let mut perimeter: Vec<[f32; 2]> = vec![[-half_w + r, -half_d]];
    // Front tab, sized by `PlateMetrics` from the build volume width.
    let back = metrics.tab_back * 0.5;
    let front = metrics.tab_front * 0.5;
    let tab_y = -half_d - metrics.tab_depth;
    perimeter.push([-back, -half_d]);
    perimeter.push([-front, tab_y]);
    perimeter.push([front, tab_y]);
    perimeter.push([back, -half_d]);
    let arc = |cx: f32, cy: f32, a0: f32, a1: f32, points: &mut Vec<[f32; 2]>| {
        for i in 1..=segments {
            let a = a0 + (a1 - a0) * i as f32 / segments as f32;
            points.push([cx + r * a.cos(), cy + r * a.sin()]);
        }
    };
    arc(half_w - r, -half_d + r, -std::f32::consts::FRAC_PI_2, 0.0, &mut perimeter);
    arc(half_w - r, half_d - r, 0.0, std::f32::consts::FRAC_PI_2, &mut perimeter);
    arc(-half_w + r, half_d - r, std::f32::consts::FRAC_PI_2, std::f32::consts::PI, &mut perimeter);
    arc(-half_w + r, -half_d + r, std::f32::consts::PI, 1.5 * std::f32::consts::PI, &mut perimeter);
    if let (Some(first), Some(last)) = (perimeter.first(), perimeter.last()) {
        if (first[0] - last[0]).abs() < 1e-5 && (first[1] - last[1]).abs() < 1e-5 {
            perimeter.pop();
        }
    }

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let count = perimeter.len();
    // The source positions the mesh at `[buildVolumeCenterX, buildVolumeCenterY,
    // buildPlateCenterZ]`; the shape itself is local.
    let ox = metrics.center_x;
    let oy = metrics.center_y;

    let top_center = vertices.len() as u32;
    vertices.push(Vertex { pos: [ox, oy, z0], normal: [0.0, 0.0, 1.0], ao: 1.0 });
    let top_start = vertices.len() as u32;
    for p in &perimeter {
        vertices.push(Vertex { pos: [p[0] + ox, p[1] + oy, z0], normal: [0.0, 0.0, 1.0], ao: 1.0 });
    }
    for i in 0..count {
        let a = top_start + i as u32;
        let b = top_start + ((i + 1) % count) as u32;
        indices.extend_from_slice(&[top_center, a, b]);
    }

    let bottom_center = vertices.len() as u32;
    vertices.push(Vertex { pos: [ox, oy, z1], normal: [0.0, 0.0, -1.0], ao: 1.0 });
    let bottom_start = vertices.len() as u32;
    for p in &perimeter {
        vertices.push(Vertex { pos: [p[0] + ox, p[1] + oy, z1], normal: [0.0, 0.0, -1.0], ao: 1.0 });
    }
    for i in 0..count {
        let a = bottom_start + i as u32;
        let b = bottom_start + ((i + 1) % count) as u32;
        indices.extend_from_slice(&[bottom_center, b, a]);
    }

    for i in 0..count {
        let p0 = perimeter[i];
        let p1 = perimeter[(i + 1) % count];
        let dx = p1[0] - p0[0];
        let dy = p1[1] - p0[1];
        let length = (dx * dx + dy * dy).sqrt().max(1e-6);
        let normal = [dy / length, -dx / length, 0.0];
        let base = vertices.len() as u32;
        vertices.push(Vertex { pos: [p0[0] + ox, p0[1] + oy, z0], normal, ao: 1.0 });
        vertices.push(Vertex { pos: [p1[0] + ox, p1[1] + oy, z0], normal, ao: 1.0 });
        vertices.push(Vertex { pos: [p1[0] + ox, p1[1] + oy, z1], normal, ao: 1.0 });
        vertices.push(Vertex { pos: [p0[0] + ox, p0[1] + oy, z1], normal, ao: 1.0 });
        indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
    }

    (vertices, indices)
}

pub(super) fn front_texture() -> (u32, u32, Vec<u8>) {
    // `SceneEnvironment.tsx`: 256x72 canvas, `700 70px Arial` fitted to 200px,
    // centred at y + 5, colour = gridMajor (#4f5560) lerped 36% toward white.
    let width = 256u32;
    let height = 72u32;
    let mut data = vec![0u8; (width * height * 4) as usize];
    let Ok(bytes) = std::fs::read("C:/Windows/Fonts/arialbd.ttf") else {
        return (width, height, data);
    };
    let Ok(font) = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()) else {
        return (width, height, data);
    };
    let text = "FRONT";
    let measure = |size: f32| -> f32 {
        text.chars()
            .map(|ch| font.metrics(ch, size).advance_width)
            .sum()
    };
    let mut size = 70.0f32;
    let natural = measure(size);
    // `fitFontToWidth`: shrink by the overflow ratio, floored at 8px.
    if natural > FRONT_MARKER_MAX_TEXT_WIDTH && natural > 0.0 {
        size = (size * FRONT_MARKER_MAX_TEXT_WIDTH / natural).floor().max(8.0);
    }
    let total = measure(size);
    let start_x = (width as f32 - total) / 2.0;
    let baseline = height as f32 / 2.0 + 5.0 + size * 0.35;
    let color = front_marker_color();
    let mut pen = start_x;
    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, size);
        let gx = pen + metrics.xmin as f32;
        let gy = baseline - metrics.ymin as f32 - metrics.height as f32;
        for row in 0..metrics.height {
            for col in 0..metrics.width {
                let coverage = bitmap[row * metrics.width + col] as f32 / 255.0;
                if coverage <= 0.0 {
                    continue;
                }
                let px = gx as i32 + col as i32;
                let py = gy as i32 + row as i32;
                if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
                    continue;
                }
                let index = ((py as u32 * width + px as u32) * 4) as usize;
                data[index] = (color[0] as f32 * coverage) as u8;
                data[index + 1] = (color[1] as f32 * coverage) as u8;
                data[index + 2] = (color[2] as f32 * coverage) as u8;
                data[index + 3] = (255.0 * coverage) as u8;
            }
        }
        pen += metrics.advance_width;
    }
    (width, height, data)
}

/// `new THREE.Color(gridMajorColor).lerp(white, 0.36)`.
fn front_marker_color() -> [u8; 3] {
    let channel = |shift: u32| -> u8 {
        let from = ((GRID_MAJOR_COLOR >> shift) & 0xff) as f32;
        let to = 255.0f32;
        (from + (to - from) * 0.36).round() as u8
    };
    [channel(16), channel(8), channel(0)]
}

pub(super) fn logo_texture() -> (u32, u32, Vec<u8>) {
    let width = 1772u32;
    let height = 304u32;
    let Ok(svg) = std::fs::read_to_string("../public/dragonfruit_assets/branding/text_logo.svg") else {
        return (width, height, vec![0u8; (width * height * 4) as usize]);
    };
    let Ok(tree) = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()) else {
        return (width, height, vec![0u8; (width * height * 4) as usize]);
    };
    let Some(mut pixmap) = resvg::tiny_skia::Pixmap::new(width, height) else {
        return (width, height, vec![0u8; (width * height * 4) as usize]);
    };
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    // `resvg` hands back premultiplied RGBA; three's `meshBasicMaterial` blends
    // with straight alpha (`SrcAlpha`), so undo the premultiply and leave the
    // 0.4 opacity to the shader.
    let mut data = pixmap.data().to_vec();
    for pixel in data.chunks_exact_mut(4) {
        let alpha = pixel[3] as f32 / 255.0;
        if alpha > 0.0 {
            for channel in &mut pixel[..3] {
                *channel = ((*channel as f32 / alpha).min(255.0)) as u8;
            }
        }
    }
    (width, height, data)
}

pub(super) fn decal_vertices(metrics: &PlateMetrics) -> Vec<[f32; 5]> {
    let mut out = Vec::new();
    let mut quad = |cx: f32, cy: f32, width: f32, height: f32, z: f32| {
        let x0 = cx - width * 0.5;
        let x1 = cx + width * 0.5;
        let y0 = cy - height * 0.5;
        let y1 = cy + height * 0.5;
        let corners = [
            (x0, y0, 0.0, 1.0),
            (x1, y0, 1.0, 1.0),
            (x1, y1, 1.0, 0.0),
            (x0, y0, 0.0, 1.0),
            (x1, y1, 1.0, 0.0),
            (x0, y1, 0.0, 0.0),
        ];
        for (x, y, u, v) in corners {
            out.push([x, y, z, u, v]);
        }
    };
    quad(
        metrics.center_x,
        metrics.center_y + metrics.marker_y,
        metrics.marker_width,
        metrics.marker_depth,
        0.001,
    );
    quad(
        metrics.logo_x,
        metrics.logo_y,
        metrics.logo_width,
        metrics.logo_height,
        0.12,
    );
    out
}

/// `--out-of-bounds-stripe` (`resolveOutOfBoundsStripeColor`), sRGB-linear.
/// The source falls back to `#b6ff2e` when the variable is absent; the app's
/// token is the same hue.
pub(super) fn out_of_bounds_stripe_color() -> [f32; 3] {
    srgb_linear(0xb6ff2e)
}
