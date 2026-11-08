//! STL loading: binary and ASCII, flat per-face normals (STL is faceted), and a
//! plate footprint hull.

use std::cmp::Ordering;
use std::path::Path;

use anyhow::{Context, Result, bail};
use dragonfruit_mesh_core::mesh::{IndexedMesh, Vec3};
use dragonfruit_mesh_core::refine::{refine_long_edges_with_budget, refinement_budget};
use dragonfruit_mesh_core::vertex_occlusion::bake_smoothed_vertex_occlusion;
use glam::Vec3 as GVec3;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub ao: f32,
}

pub struct MeshData {
    pub vertices: Vec<Vertex>,
    /// Convex hull of the model's XY projection, for the plate outline.
    pub footprint: Vec<[f32; 2]>,
    pub center: GVec3,
    pub radius: f32,
    pub triangles: usize,
    pub name: String,
    /// Bounding-box size in millimetres.
    pub size: [f32; 3],
    /// Signed volume in cubic millimetres.
    pub volume: f32,
}

pub fn load_stl(path: &Path) -> Result<MeshData> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let soup = if is_binary(&bytes) {
        parse_binary(&bytes)?
    } else {
        parse_ascii(&bytes)?
    };
    if soup.len() < 9 {
        bail!("{} contains no triangles", path.display());
    }

    let mesh = IndexedMesh::from_triangle_soup(&soup, 1e-6);
    if mesh.triangles.is_empty() {
        bail!("{} welded to an empty mesh", path.display());
    }

    let mut min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
    let mut max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);
    for p in &mesh.positions {
        min = min.min(*p);
        max = max.max(*p);
    }

    // The app refines faces longer than 2% of the diagonal before deriving
    // per-vertex data, so a coarse face does not carry one AO value across a
    // wedge. Same threshold, same budget.
    let diagonal = max.sub(min).length();
    let max_edge = (diagonal * 0.02).max(0.01);
    let budget = refinement_budget(mesh.triangles.len());
    let mesh = refine_long_edges_with_budget(&mesh, max_edge, 8, budget);

    // The app's bake: 32 rays, turned fan, one graph smoothing pass.
    let ao = bake_smoothed_vertex_occlusion(&mesh, 32, None);

    // Flat normals: three vertices per triangle, each carrying the face normal.
    let mut vertices = Vec::with_capacity(mesh.triangles.len() * 3);
    for tri in &mesh.triangles {
        let a = mesh.positions[tri[0] as usize];
        let b = mesh.positions[tri[1] as usize];
        let c = mesh.positions[tri[2] as usize];
        let n = b.sub(a).cross(c.sub(a));
        let length = n.length();
        let normal = if length > 1e-12 {
            n.scale(1.0 / length)
        } else {
            Vec3::new(0.0, 0.0, 1.0)
        };
        for &vertex in tri {
            let p = mesh.positions[vertex as usize];
            vertices.push(Vertex {
                pos: [p.x, p.y, p.z],
                normal: [normal.x, normal.y, normal.z],
                ao: ao.get(vertex as usize).copied().unwrap_or(1.0),
            });
        }
    }

    let center = GVec3::new(
        (min.x + max.x) * 0.5,
        (min.y + max.y) * 0.5,
        (min.z + max.z) * 0.5,
    );
    let mut radius: f32 = 0.0;
    for p in &mesh.positions {
        let d = GVec3::new(p.x - center.x, p.y - center.y, p.z - center.z).length();
        radius = radius.max(d);
    }
    let radius = radius.max(0.001);
    let footprint = convex_hull(&mesh.positions);

    let mut volume = 0.0f64;
    for tri in &mesh.triangles {
        let a = mesh.positions[tri[0] as usize];
        let b = mesh.positions[tri[1] as usize];
        let c = mesh.positions[tri[2] as usize];
        volume += a.dot(b.cross(c)) as f64 / 6.0;
    }

    Ok(MeshData {
        vertices,
        footprint,
        center,
        radius,
        triangles: mesh.triangles.len(),
        size: [max.x - min.x, max.y - min.y, max.z - min.z],
        volume: volume.abs() as f32,
        name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string()),
    })
}

/// Write a binary STL with the model matrix applied.
pub fn export_stl(mesh: &MeshData, transform: glam::Mat4, path: &Path) -> Result<()> {
    let mut out = Vec::with_capacity(84 + mesh.triangles * 50);
    out.extend_from_slice(&[0u8; 80]);
    out.extend_from_slice(&(mesh.triangles as u32).to_le_bytes());
    for triangle in mesh.vertices.chunks_exact(3) {
        let normal = transform
            .transform_vector3(glam::Vec3::from_array(triangle[0].normal))
            .normalize();
        for value in [normal.x, normal.y, normal.z] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        for vertex in triangle {
            let position = transform.transform_point3(glam::Vec3::from_array(vertex.pos));
            for value in [position.x, position.y, position.z] {
                out.extend_from_slice(&value.to_le_bytes());
            }
        }
        out.extend_from_slice(&0u16.to_le_bytes());
    }
    std::fs::write(path, out).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Centroids of downward-facing triangles above the plate, the points a
/// support preview drops from.
pub fn overhang_points(mesh: &MeshData, angle_degrees: f32) -> Vec<[f32; 3]> {
    let threshold = -angle_degrees.to_radians().cos();
    let mut points = Vec::new();
    for triangle in mesh.vertices.chunks_exact(3) {
        if triangle[0].normal[2] > threshold {
            continue;
        }
        let a = triangle[0].pos;
        let b = triangle[1].pos;
        let c = triangle[2].pos;
        let z = (a[2] + b[2] + c[2]) / 3.0;
        if z < 0.5 {
            continue;
        }
        points.push([(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, z]);
    }
    points
}

/// Whether a world-space ray hits the model, by Moller-Trumbore.
pub fn raycast(mesh: &MeshData, origin: GVec3, direction: GVec3) -> bool {
    raycast_distance(mesh, origin, direction).is_some()
}

/// Distance along the ray to the nearest triangle, by Moller-Trumbore.
///
/// `direction` is normalised first, so the returned `t` is in millimetres
/// along the ray. Picking a model out of a crowd needs the distance, not just
/// the boolean: the nearest hit wins, and testing the first model in the list
/// would pick whichever happens to come first in import order.
pub fn raycast_distance(mesh: &MeshData, origin: GVec3, direction: GVec3) -> Option<f32> {
    let direction = direction.normalize();
    // Slab test against the bounding box first: hover raycasts run on every
    // pointer move, and most of them miss the model entirely.
    let half = GVec3::from_array(mesh.size) * 0.5;
    let min = mesh.center - half;
    let max = mesh.center + half;
    let mut t_min = f32::NEG_INFINITY;
    let mut t_max = f32::INFINITY;
    for axis in 0..3 {
        let d = direction[axis];
        let o = origin[axis];
        if d.abs() < 1e-8 {
            if o < min[axis] || o > max[axis] {
                return None;
            }
        } else {
            let t1 = (min[axis] - o) / d;
            let t2 = (max[axis] - o) / d;
            t_min = t_min.max(t1.min(t2));
            t_max = t_max.min(t1.max(t2));
        }
    }
    if t_max < t_min.max(0.0) {
        return None;
    }
    let mut nearest = f32::INFINITY;
    for triangle in mesh.vertices.chunks_exact(3) {
        let a = GVec3::from_array(triangle[0].pos);
        let b = GVec3::from_array(triangle[1].pos);
        let c = GVec3::from_array(triangle[2].pos);
        let edge1 = b - a;
        let edge2 = c - a;
        let pvec = direction.cross(edge2);
        let det = edge1.dot(pvec);
        if det.abs() < 1e-8 {
            continue;
        }
        let inv = 1.0 / det;
        let tvec = origin - a;
        let u = tvec.dot(pvec) * inv;
        if !(0.0..=1.0).contains(&u) {
            continue;
        }
        let qvec = tvec.cross(edge1);
        let v = direction.dot(qvec) * inv;
        if v < 0.0 || u + v > 1.0 {
            continue;
        }
        // Moller-Trumbore's ray parameter: `edge2 · qvec / det`, *not*
        // `direction · qvec / det` (that is `v`, a barycentric coordinate the
        // old boolean hit test reused as a positivity check). Using `v` here
        // makes every model report its barycentric coordinate, so models at
        // different depths compare equal.
        let t = edge2.dot(qvec) * inv;
        if t > 0.0 {
            nearest = nearest.min(t);
        }
    }
    nearest.is_finite().then_some(nearest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_binary_stl() {
        let mesh = MeshData {
            vertices: vec![
                Vertex {
                    pos: [0.0, 0.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    ao: 1.0,
                },
                Vertex {
                    pos: [1.0, 0.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    ao: 1.0,
                },
                Vertex {
                    pos: [0.0, 1.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    ao: 1.0,
                },
            ],
            footprint: Vec::new(),
            center: GVec3::ZERO,
            radius: 1.0,
            triangles: 1,
            name: "test".to_owned(),
            size: [1.0, 1.0, 0.0],
            volume: 0.0,
        };
        let path = std::env::temp_dir().join("dragonfruit-gpui-export-test.stl");
        export_stl(&mesh, glam::Mat4::IDENTITY, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes.len(), 84 + 50);
        assert_eq!(u32::from_le_bytes(bytes[80..84].try_into().unwrap()), 1);
        let _ = std::fs::remove_file(&path);
    }
}

fn convex_hull(points: &[Vec3]) -> Vec<[f32; 2]> {
    let mut sorted: Vec<[f32; 2]> = points.iter().map(|p| [p.x, p.y]).collect();
    sorted.sort_by(|a, b| {
        a[0].partial_cmp(&b[0])
            .unwrap_or(Ordering::Equal)
            .then(a[1].partial_cmp(&b[1]).unwrap_or(Ordering::Equal))
    });
    sorted.dedup_by(|a, b| (a[0] - b[0]).abs() < 1e-4 && (a[1] - b[1]).abs() < 1e-4);
    if sorted.len() < 3 {
        return sorted;
    }

    let cross = |o: [f32; 2], a: [f32; 2], b: [f32; 2]| {
        (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    };
    let mut lower: Vec<[f32; 2]> = Vec::new();
    for &point in &sorted {
        while lower.len() >= 2
            && cross(lower[lower.len() - 2], lower[lower.len() - 1], point) <= 0.0
        {
            lower.pop();
        }
        lower.push(point);
    }
    let mut upper: Vec<[f32; 2]> = Vec::new();
    for &point in sorted.iter().rev() {
        while upper.len() >= 2
            && cross(upper[upper.len() - 2], upper[upper.len() - 1], point) <= 0.0
        {
            upper.pop();
        }
        upper.push(point);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn is_binary(bytes: &[u8]) -> bool {
    if bytes.len() < 84 {
        return false;
    }
    let count = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
    bytes.len() == 84 + count * 50
}

fn parse_binary(bytes: &[u8]) -> Result<Vec<f32>> {
    let count = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
    let mut soup = Vec::with_capacity(count * 9);
    for index in 0..count {
        let base = 84 + index * 50 + 12; // skip the facet normal
        for corner in 0..3 {
            let offset = base + corner * 12;
            for axis in 0..3 {
                let at = offset + axis * 4;
                soup.push(f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()));
            }
        }
    }
    Ok(soup)
}

fn parse_ascii(bytes: &[u8]) -> Result<Vec<f32>> {
    let text = String::from_utf8_lossy(bytes);
    let mut soup = Vec::new();
    let mut triangle = Vec::with_capacity(9);
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix("vertex ") else {
            continue;
        };
        let mut values = rest
            .split_whitespace()
            .filter_map(|value| value.parse::<f32>().ok());
        let (Some(x), Some(y), Some(z)) = (values.next(), values.next(), values.next()) else {
            continue;
        };
        triangle.extend_from_slice(&[x, y, z]);
        if triangle.len() == 9 {
            soup.extend_from_slice(&triangle);
            triangle.clear();
        }
    }
    Ok(soup)
}
