//! STL loading: binary and ASCII, welded into an indexed mesh with smooth
//! vertex normals, baked per-vertex occlusion, and a plate footprint hull.

use std::cmp::Ordering;
use std::path::Path;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use dragonfruit_mesh_core::mesh::{IndexedMesh, Vec3};
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
    pub indices: Vec<u32>,
    /// Convex hull of the model's XY projection, for the plate outline.
    pub footprint: Vec<[f32; 2]>,
    pub center: GVec3,
    pub radius: f32,
    pub triangles: usize,
    pub name: String,
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

    let mut normals = vec![Vec3::ZERO; mesh.positions.len()];
    for tri in &mesh.triangles {
        let a = mesh.positions[tri[0] as usize];
        let b = mesh.positions[tri[1] as usize];
        let c = mesh.positions[tri[2] as usize];
        let n = b.sub(a).cross(c.sub(a));
        for &index in tri {
            let slot = &mut normals[index as usize];
            *slot = slot.add(n);
        }
    }
    for normal in &mut normals {
        let length = normal.length();
        if length > 1e-12 {
            *normal = normal.scale(1.0 / length);
        }
    }

    let ao_started = Instant::now();
    let ao = bake_smoothed_vertex_occlusion(&mesh, 32, None);
    eprintln!(
        "ao: baked {} vertices in {:.0} ms",
        ao.len(),
        ao_started.elapsed().as_secs_f64() * 1000.0
    );

    let mut min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
    let mut max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);
    for p in &mesh.positions {
        min = min.min(*p);
        max = max.max(*p);
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

    let vertices = mesh
        .positions
        .iter()
        .zip(&normals)
        .enumerate()
        .map(|(index, (p, n))| Vertex {
            pos: [p.x, p.y, p.z],
            normal: [n.x, n.y, n.z],
            ao: ao.get(index).copied().unwrap_or(1.0),
        })
        .collect();
    let indices = mesh.triangles.iter().flatten().copied().collect();
    let footprint = convex_hull(&mesh.positions);

    Ok(MeshData {
        vertices,
        indices,
        footprint,
        center,
        radius,
        triangles: mesh.triangles.len(),
        name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string()),
    })
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
