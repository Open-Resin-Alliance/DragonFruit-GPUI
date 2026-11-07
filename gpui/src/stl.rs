//! STL loading: binary and ASCII, welded into an indexed mesh with smooth
//! vertex normals.

use std::path::Path;

use anyhow::{bail, Context, Result};
use dragonfruit_mesh_core::mesh::{IndexedMesh, Vec3};
use glam::Vec3 as GVec3;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
}

pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub center: GVec3,
    pub radius: f32,
    pub triangles: usize,
    pub name: String,
}

pub fn load_stl(path: &Path) -> Result<MeshData> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("reading {}", path.display()))?;
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
    let radius = ((max.sub(min)).length() * 0.5).max(0.001);

    let vertices = mesh
        .positions
        .iter()
        .zip(&normals)
        .map(|(p, n)| Vertex {
            pos: [p.x, p.y, p.z],
            normal: [n.x, n.y, n.z],
        })
        .collect();
    let indices = mesh.triangles.iter().flatten().copied().collect();

    Ok(MeshData {
        vertices,
        indices,
        center,
        radius,
        triangles: mesh.triangles.len(),
        name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string()),
    })
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
        let mut values = rest.split_whitespace().filter_map(|value| value.parse::<f32>().ok());
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
