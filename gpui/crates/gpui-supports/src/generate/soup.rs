//! The model mesh, as the contact-placement stage consumes it.
//!
//! `place` takes a plain triangle soup because that is what an STL is and
//! because the classifier upstream of this stage (`scan_overhangs` in the Tauri
//! backend) works on exactly that: `overhang_and_stability_from_soup` welds a
//! 9-floats-per-triangle array with `IndexedMesh::from_triangle_soup(positions,
//! 1e-5)` before classifying. [`WeldedMesh`] here is that weld, and
//! [`MERGE_EPSILON`] is the same epsilon.

use glam::Vec3;

/// Merge epsilon handed to `IndexedMesh::from_triangle_soup`
/// (`src-tauri/src/overhang.rs`, `overhang_and_stability_from_soup`).
pub(crate) const MERGE_EPSILON: f32 = 1e-5;

/// A triangle soup in world millimetres — the shape of an imported STL.
#[derive(Clone, Debug, Default)]
pub struct MeshTriangles {
    /// One entry per face: three corners, counter-clockwise seen from outside.
    pub triangles: Vec<[[f32; 3]; 3]>,
}

impl MeshTriangles {
    pub fn new() -> Self {
        Self::default()
    }

    /// `soup` holds three corners per triangle, flat.
    pub fn from_soup(soup: &[[f32; 3]]) -> Self {
        Self {
            triangles: soup.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect(),
        }
    }

    /// Nine floats per triangle, as the backend's soup IPC carries it.
    pub fn from_flat(values: &[f32]) -> Self {
        Self {
            triangles: values
                .chunks_exact(9)
                .map(|c| {
                    [
                        [c[0], c[1], c[2]],
                        [c[3], c[4], c[5]],
                        [c[6], c[7], c[8]],
                    ]
                })
                .collect(),
        }
    }

    pub fn from_indexed(positions: &[[f32; 3]], indices: &[[u32; 3]]) -> Self {
        let mut triangles = Vec::with_capacity(indices.len());
        for tri in indices {
            let corner = |i: u32| {
                positions
                    .get(i as usize)
                    .copied()
                    .unwrap_or([0.0, 0.0, 0.0])
            };
            triangles.push([corner(tri[0]), corner(tri[1]), corner(tri[2])]);
        }
        Self { triangles }
    }

    pub fn push(&mut self, a: [f32; 3], b: [f32; 3], c: [f32; 3]) {
        self.triangles.push([a, b, c]);
    }

    pub fn len(&self) -> usize {
        self.triangles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }

    /// Weld shared corners to vertex ids, exactly as
    /// `IndexedMesh::from_triangle_soup_with_corner_map` does: one quantization
    /// step derived from the bounding-box diagonal, and the first corner seen
    /// keeps the id.
    pub(crate) fn weld(&self) -> WeldedMesh {
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for tri in &self.triangles {
            for corner in tri {
                let p = Vec3::from_array(*corner);
                min = min.min(p);
                max = max.max(p);
            }
        }
        let diagonal = (max - min).length().max(1e-6);
        let step = (MERGE_EPSILON * diagonal).max(1e-7);
        let inv_step = 1.0 / step;

        let mut positions: Vec<Vec3> = Vec::with_capacity(self.triangles.len() * 3 / 2);
        let mut triangles: Vec<[u32; 3]> = Vec::with_capacity(self.triangles.len());
        let mut interned: std::collections::HashMap<(i32, i32, i32), u32> =
            std::collections::HashMap::with_capacity(self.triangles.len() * 2);

        for tri in &self.triangles {
            let mut ids = [0u32; 3];
            for (slot, corner) in ids.iter_mut().zip(tri.iter()) {
                let p = Vec3::from_array(*corner);
                let key = (
                    (p.x * inv_step).round() as i32,
                    (p.y * inv_step).round() as i32,
                    (p.z * inv_step).round() as i32,
                );
                let next = positions.len() as u32;
                let id = *interned.entry(key).or_insert_with(|| {
                    positions.push(p);
                    next
                });
                *slot = id;
            }
            triangles.push(ids);
        }

        WeldedMesh {
            positions,
            triangles,
        }
    }
}

/// An indexed mesh with welded vertices — the classifier's working form.
#[derive(Clone, Debug, Default)]
pub(crate) struct WeldedMesh {
    pub positions: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
}

impl WeldedMesh {
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    pub fn triangle(&self, face: u32) -> [u32; 3] {
        self.triangles[face as usize]
    }

    /// Corner positions of a face (`IndexedMesh::tri_positions`).
    pub fn tri_positions(&self, face: u32) -> [Vec3; 3] {
        let t = self.triangles[face as usize];
        [
            self.positions[t[0] as usize],
            self.positions[t[1] as usize],
            self.positions[t[2] as usize],
        ]
    }

    /// `IndexedMesh::tri_area` — half the cross-product length.
    pub fn tri_area(&self, face: u32) -> f32 {
        let [a, b, c] = self.tri_positions(face);
        (b - a).cross(c - a).length() * 0.5
    }

    /// `IndexedMesh::tri_normal` — the raw winding normal, normalized; zero for
    /// a degenerate face, never flipped.
    pub fn tri_normal(&self, face: u32) -> Vec3 {
        let [a, b, c] = self.tri_positions(face);
        let n = (b - a).cross(c - a);
        let len = n.length();
        if len > 0.0 {
            n / len
        } else {
            Vec3::ZERO
        }
    }
}
