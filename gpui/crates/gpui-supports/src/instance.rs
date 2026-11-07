//! Instanced batches — the shape the renderer draws.
//!
//! `InstancedShaftGroup.tsx` batches by what can share one `cylinderGeometry`:
//! a shaft whose `diameterEnd` differs from `diameter` **leaves the batch**,
//! because three.js scales one unit cylinder per instance. The port mirrors
//! that rule — a straight shaft is an instance, a tapered one is its own
//! instance of the same unit cylinder with a per-instance taper — so a support
//! set is a handful of `draw_indexed(.., 0..instances)` calls, not one per
//! support.

use glam::{Mat4, Vec3};

/// A straight shaft instance: a unit cylinder stretched onto `from..to`.
#[derive(Clone, Copy, Debug)]
pub struct ShaftInstance {
    pub from: Vec3,
    pub to: Vec3,
    /// Diameter at `from`, in plate millimetres.
    pub diameter: f32,
    /// Diameter at `to`; `None` keeps the shaft in the constant-width batch.
    pub diameter_end: Option<f32>,
}

impl ShaftInstance {
    /// Whether the shaft can share the constant-width batch.
    ///
    /// The source's rule: set and differing ⇒ the shaft leaves the batch.
    pub fn is_straight(&self) -> bool {
        match self.diameter_end {
            None => true,
            Some(end) => (end - self.diameter).abs() <= f32::EPSILON,
        }
    }

    /// `(translation, rotation, scale)` for the unit cylinder, which spans
    /// `y = -0.5..0.5` with a radius of 0.5 — three.js's `CylinderGeometry`.
    pub fn matrix(&self) -> Mat4 {
        let axis = self.to - self.from;
        let length = axis.length();
        if length <= f32::EPSILON {
            return Mat4::IDENTITY;
        }
        let rotation = Mat4::from_quat(glam::Quat::from_rotation_arc(Vec3::Y, axis / length));
        // The unit cylinder has a 0.5 radius, so the scale is the diameter.
        let scale = Mat4::from_scale(Vec3::new(self.diameter, length, self.diameter));
        Mat4::from_translation(self.from + axis * 0.5) * rotation * scale
    }
}

/// A knot, joint or root: a sphere of `radius` at `position`.
#[derive(Clone, Copy, Debug)]
pub struct NodeInstance {
    pub position: Vec3,
    pub radius: f32,
}

impl NodeInstance {
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_translation(self.position) * Mat4::from_scale(Vec3::splat(self.radius))
    }
}

/// A contact pad: a unit disk facing `normal`.
#[derive(Clone, Copy, Debug)]
pub struct DiskInstance {
    pub position: Vec3,
    pub normal: Vec3,
    pub radius: f32,
}

impl DiskInstance {
    pub fn matrix(&self) -> Mat4 {
        let normal = self.normal.normalize_or_zero();
        let rotation = if normal == Vec3::ZERO {
            Mat4::IDENTITY
        } else {
            Mat4::from_quat(glam::Quat::from_rotation_arc(Vec3::Z, normal))
        };
        Mat4::from_translation(self.position)
            * rotation
            * Mat4::from_scale(Vec3::splat(self.radius))
    }
}

/// One support set, already split into its draw batches.
#[derive(Default, Debug)]
pub struct SupportBatches {
    pub straight_shafts: Vec<ShaftInstance>,
    pub tapered_shafts: Vec<ShaftInstance>,
    pub nodes: Vec<NodeInstance>,
    pub disks: Vec<DiskInstance>,
}

impl SupportBatches {
    /// Sort instances into the batches the renderer can draw in one call each.
    pub fn push(&mut self, shaft: ShaftInstance) {
        if shaft.is_straight() {
            self.straight_shafts.push(shaft);
        } else {
            self.tapered_shafts.push(shaft);
        }
    }

    /// Draw calls this set needs: one per non-empty batch.
    pub fn draw_calls(&self) -> usize {
        [
            !self.straight_shafts.is_empty(),
            !self.tapered_shafts.is_empty(),
            !self.nodes.is_empty(),
            !self.disks.is_empty(),
        ]
        .iter()
        .filter(|used| **used)
        .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tapered_shaft_leaves_the_straight_batch() {
        let mut batches = SupportBatches::default();
        batches.push(ShaftInstance {
            from: Vec3::ZERO,
            to: Vec3::Z,
            diameter: 0.8,
            diameter_end: None,
        });
        batches.push(ShaftInstance {
            from: Vec3::ZERO,
            to: Vec3::Z,
            diameter: 0.8,
            diameter_end: Some(0.4),
        });
        assert_eq!(batches.straight_shafts.len(), 1);
        assert_eq!(batches.tapered_shafts.len(), 1);
        assert_eq!(batches.draw_calls(), 2);
    }

    #[test]
    fn a_unit_shaft_matrix_stretches_onto_the_axis() {
        let shaft = ShaftInstance {
            from: Vec3::new(1.0, 0.0, 0.0),
            to: Vec3::new(1.0, 0.0, 6.0),
            diameter: 2.0,
            diameter_end: None,
        };
        let mid = shaft.matrix().transform_point3(Vec3::ZERO);
        assert!((mid - Vec3::new(1.0, 0.0, 3.0)).length() < 1e-5);
        // The unit cylinder spans ±0.5 in Y; scaled by the diameter, ±1.
        let top = shaft.matrix().transform_point3(Vec3::new(0.5, 0.0, 0.0));
        assert!((top - Vec3::new(2.0, 0.0, 3.0)).length() < 1e-5);
    }
}
