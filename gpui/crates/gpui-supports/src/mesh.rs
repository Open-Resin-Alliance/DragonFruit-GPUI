//! A support set assembled from the primitives.

use glam::Vec3;

use crate::primitives;

/// One support node — a knot where shafts meet, or a contact point on the model.
#[derive(Clone, Copy, Debug)]
pub struct SupportNode {
    pub position: Vec3,
    /// Sphere radius; `ContactCone`/`ContactDisk` use their own shapes.
    pub radius: f32,
}

/// One straight shaft between two nodes.
#[derive(Clone, Copy, Debug)]
pub struct SupportShaft {
    pub from: Vec3,
    pub to: Vec3,
    pub entry_radius: f32,
    /// Tapered tip; equal to `entry_radius` for a plain pillar.
    pub exit_radius: f32,
}

/// The triangle soup the viewport uploads for a support set.
#[derive(Default, Debug)]
pub struct SupportMesh {
    pub vertices: Vec<[f32; 3]>,
}

impl SupportMesh {
    /// Bake shafts, knots and base roots into one vertex list.
    ///
    /// `segments` is `radialSegments` from `InstancedShaftGroup.tsx` — pass
    /// [`crate::RADIAL_SEGMENTS`] to match the source's tessellation.
    pub fn build(
        shafts: &[SupportShaft],
        nodes: &[SupportNode],
        roots: &[SupportNode],
        segments: usize,
    ) -> Self {
        let mut vertices = Vec::new();
        for shaft in shafts {
            vertices.extend(primitives::cylinder(
                shaft.from,
                shaft.to,
                shaft.entry_radius,
                shaft.exit_radius,
                segments,
            ));
        }
        for node in nodes.iter().chain(roots) {
            vertices.extend(primitives::sphere(
                node.position,
                node.radius,
                segments,
                crate::SPHERE_RINGS,
            ));
        }
        Self { vertices }
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    pub fn triangle_count(&self) -> usize {
        self.vertices.len() / 3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shaft_and_a_knot_produce_closed_geometry() {
        let shafts = [SupportShaft {
            from: Vec3::new(0.0, 0.0, 0.0),
            to: Vec3::new(0.0, 0.0, 10.0),
            entry_radius: 0.3,
            exit_radius: 0.2,
        }];
        let nodes = [SupportNode {
            position: Vec3::new(0.0, 0.0, 10.0),
            radius: 0.4,
        }];
        let mesh = SupportMesh::build(&shafts, &nodes, &[], 12);
        assert!(!mesh.is_empty());
        // 12 sides + two 12-triangle caps, then a 12x10 UV sphere: two
        // triangle bands per ring gap, so `2 * segments * (rings - 1)`.
        assert_eq!(mesh.triangle_count(), 12 * 4 + 2 * 12 * (crate::SPHERE_RINGS - 1));
        assert!(mesh
            .vertices
            .iter()
            .all(|v| v.iter().all(|value| value.is_finite())));
    }

    #[test]
    fn a_degenerate_shaft_emits_nothing() {
        let shaft = SupportShaft {
            from: Vec3::ZERO,
            to: Vec3::ZERO,
            entry_radius: 0.3,
            exit_radius: 0.3,
        };
        assert!(SupportMesh::build(&[shaft], &[], &[], 12).is_empty());
    }

    #[test]
    fn a_cone_tapers_to_a_point() {
        let cone = primitives::cone(Vec3::ZERO, Vec3::new(0.0, 0.0, 2.0), 0.5, 12);
        // Every vertex at the tip height sits on the axis.
        let tip = cone
            .iter()
            .filter(|v| (v[2] - 2.0).abs() < 1e-5)
            .all(|v| v[0].abs() < 1e-5 && v[1].abs() < 1e-5);
        assert!(tip);
    }

    #[test]
    fn a_disk_is_flat_and_faces_its_normal() {
        let disk = primitives::disk(Vec3::new(0.0, 0.0, 5.0), Vec3::Z, 1.0, 24);
        assert_eq!(disk.len(), 24 * 3);
        assert!(disk.iter().all(|v| (v[2] - 5.0).abs() < 1e-5));
    }
}
