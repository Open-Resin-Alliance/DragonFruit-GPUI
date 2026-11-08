//! Where a freshly imported mesh is put on the plate.
//!
//! `useSceneCollectionManager.ts` centres the geometry on its own bounding box
//! and then positions it plate-relative:
//!
//! ```ts
//! const initialZ = center.z - bbox.min.z;
//! position: new THREE.Vector3(defaultImportCenterXY.x, defaultImportCenterXY.y, initialZ)
//! ```
//!
//! (`src/features/scene/useSceneCollectionManager.ts`, both the `.3mf` branch
//! around line 3469 and the single-mesh branch around line 3527.) The geometry
//! has already been normalised by then — `useStlGeometry.ts` lines 637-639
//! translate it so its bbox centre is the local origin in the plate plane and
//! its bottom is the local origin's plane — so `center` is `0`, `bbox.min.z`
//! is `-size.z / 2`, and `initialZ` is `size.z / 2`.
//!
//! This port renders the loader's vertices in file space and carries no local
//! translation on the mesh, so the same placement has to be expressed as one
//! translate applied to those vertices: the bbox centre in XY goes to the
//! plate centre and the bbox bottom goes to `z = 0`.

use gpui_core::stl::MeshData;

/// The translate that seats `mesh` on the plate.
///
/// The result is the model's world-space translate: after adding it to every
/// vertex, the bounding box's XY centre equals `plate_center` and its bottom
/// is `z = 0`. For a mesh whose bbox is already centred on the origin this is
/// `[plate_x, plate_y, size.z / 2]` — the source's `initialZ`, since `center.z`
/// is `0` there — and applying it to the placed mesh again yields `[0, 0, 0]`,
/// so the placement is idempotent rather than cumulative.
///
/// The z is `-min_z`, not the source's literal `center.z - bbox.min.z`: the
/// source computes that against an *already centred* bbox (`center.z == 0`),
/// whereas this translate is added to vertices still in file space, where
/// `center.z - bbox.min.z` is `size.z / 2` and would leave an offset mesh's
/// bottom above the plate by its centre's height.
pub fn placement(mesh: &MeshData, plate_center: [f32; 2]) -> [f32; 3] {
    let min_z = mesh.center.z - mesh.size[2] * 0.5;
    [
        plate_center[0] - mesh.center.x,
        plate_center[1] - mesh.center.y,
        -min_z,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_core::stl::{self, MeshData};

    /// A box mesh whose bbox spans `min`..`max`, with the bbox fields the
    /// loader would have computed for it.
    fn box_mesh(min: [f32; 3], max: [f32; 3]) -> MeshData {
        let mut vertices = Vec::with_capacity(8);
        for &x in &[min[0], max[0]] {
            for &y in &[min[1], max[1]] {
                for &z in &[min[2], max[2]] {
                    vertices.push(stl::Vertex {
                        pos: [x, y, z],
                        normal: [0.0, 0.0, 1.0],
                        ao: 1.0,
                    });
                }
            }
        }
        MeshData {
            vertices,
            footprint: Vec::new(),
            center: glam::Vec3::new(
                (min[0] + max[0]) * 0.5,
                (min[1] + max[1]) * 0.5,
                (min[2] + max[2]) * 0.5,
            ),
            radius: 0.0,
            triangles: 0,
            name: String::new(),
            size: [max[0] - min[0], max[1] - min[1], max[2] - min[2]],
            volume: 0.0,
        }
    }

    /// The same mesh with every vertex moved by `translate`.
    fn translated(mesh: &MeshData, translate: [f32; 3]) -> MeshData {
        MeshData {
            vertices: mesh
                .vertices
                .iter()
                .map(|vertex| stl::Vertex {
                    pos: [
                        vertex.pos[0] + translate[0],
                        vertex.pos[1] + translate[1],
                        vertex.pos[2] + translate[2],
                    ],
                    ..*vertex
                })
                .collect(),
            footprint: mesh.footprint.clone(),
            center: mesh.center + glam::Vec3::from_array(translate),
            radius: mesh.radius,
            triangles: mesh.triangles,
            name: mesh.name.clone(),
            size: mesh.size,
            volume: mesh.volume,
        }
    }

    /// Bbox `min` and `max` of the mesh's vertices, in the port's Z-up frame.
    fn bounds(mesh: &MeshData) -> ([f32; 3], [f32; 3]) {
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];
        for vertex in &mesh.vertices {
            for axis in 0..3 {
                min[axis] = min[axis].min(vertex.pos[axis]);
                max[axis] = max[axis].max(vertex.pos[axis]);
            }
        }
        (min, max)
    }

    fn assert_seated(seated: &MeshData, plate_center: [f32; 2]) {
        let (min, max) = bounds(seated);
        assert!(((min[0] + max[0]) * 0.5 - plate_center[0]).abs() <= 1e-4);
        assert!(((min[1] + max[1]) * 0.5 - plate_center[1]).abs() <= 1e-4);
        assert!(min[2].abs() <= 1e-4, "bottom at {}, wanted 0", min[2]);
    }

    #[test]
    fn centred_mesh_only_lifts_to_the_plate() {
        // Already centred at the origin in every axis, as `useStlGeometry`
        // leaves a freshly imported model.
        let mesh = box_mesh([-5.0, -5.0, -5.0], [5.0, 5.0, 5.0]);
        let translate = placement(&mesh, [96.0, 60.0]);
        assert_eq!(translate, [96.0, 60.0, 5.0]);
        assert_seated(&translated(&mesh, translate), [96.0, 60.0]);

        // Idempotent: placing the placed mesh again moves nothing.
        let placed = translated(&mesh, translate);
        let again = placement(&placed, [96.0, 60.0]);
        for value in again {
            assert!(value.abs() <= 1e-4, "second placement was {again:?}");
        }
    }

    #[test]
    fn offset_mesh_lands_centred_on_the_plate() {
        // The acceptance case: geometry sitting at (500, -300, 40) in file
        // space, 10 mm across.
        let mesh = box_mesh([495.0, -305.0, 35.0], [505.0, -295.0, 45.0]);
        let translate = placement(&mesh, [96.0, 60.0]);
        assert_eq!(translate, [-404.0, 360.0, -35.0]);
        assert_seated(&translated(&mesh, translate), [96.0, 60.0]);
    }

    #[test]
    fn bbox_straddling_zero_uses_its_bottom() {
        let mesh = box_mesh([-5.0, -5.0, -3.0], [5.0, 5.0, 7.0]);
        let translate = placement(&mesh, [0.0, 0.0]);
        // `-min_z` is 3, not `size.z / 2` (5): the bottom is what must reach 0.
        assert_eq!(translate, [0.0, 0.0, 3.0]);
        assert_seated(&translated(&mesh, translate), [0.0, 0.0]);
    }

    #[test]
    fn mesh_wholly_below_the_plate_is_lifted() {
        let mesh = box_mesh([-5.0, -5.0, -20.0], [5.0, 5.0, -10.0]);
        let translate = placement(&mesh, [10.0, -20.0]);
        assert_eq!(translate, [10.0, -20.0, 20.0]);
        assert_seated(&translated(&mesh, translate), [10.0, -20.0]);
    }
}
