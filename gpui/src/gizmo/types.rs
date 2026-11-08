//! Ported from `src/components/gizmo/types.ts`.
//!
//! The source's gizmo carries nine callbacks and a dozen feature flags because
//! it drives a full Euler transform on a group. The port's editor entity has a
//! translation, one Z rotation and one uniform scale, so a handle identifies
//! itself and the drag machine in [`crate::gizmo::drag`] turns it into those
//! three fields. Everything the entity cannot express is simply not offered:
//! there are no X/Y rings, no per-axis scale handles and no move planes.

use glam::Vec3;

/// `GizmoAxis`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub const ALL: [Axis; 3] = [Axis::X, Axis::Y, Axis::Z];

    pub fn index(self) -> usize {
        match self {
            Axis::X => 0,
            Axis::Y => 1,
            Axis::Z => 2,
        }
    }

    pub fn unit(self) -> Vec3 {
        match self {
            Axis::X => Vec3::X,
            Axis::Y => Vec3::Y,
            Axis::Z => Vec3::Z,
        }
    }

    /// The two in-plane axes of a rotation ring, so a positive angle turns
    /// counter-clockwise about the ring's normal (the local frame
    /// `ringGroupEuler` builds in `rotationDialModel.ts`).
    pub fn ring_basis(self) -> (Vec3, Vec3) {
        match self {
            Axis::X => (Vec3::Y, Vec3::Z),
            Axis::Y => (Vec3::Z, Vec3::X),
            Axis::Z => (Vec3::X, Vec3::Y),
        }
    }
}

/// One grab target on the gizmo. The source's part ids were `center`,
/// `axis-x`…`axis-z`, `ring-x`… and `scale-x`…; the same three families survive
/// the port, with the rings reduced to the axis the entity can rotate about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Handle {
    /// `move-center`: free movement in the plate plane.
    Center,
    /// `axis-<axis>`: constrained movement along one world axis.
    Axis(Axis),
    /// `ring-<axis>`: rotation about one world axis.
    Ring(Axis),
    /// `scale-<axis>`: uniform scaling, anchored at the gizmo centre.
    Scale(Axis),
}

/// The three entity fields a drag writes. `translate` is in millimetres on the
/// plate plane; `rotate_z` is radians about world Z; `scale` is the entity's
/// single uniform factor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub translate: Vec3,
    pub rotate_z: f32,
    pub scale: f32,
}

impl Transform {
    /// The world-space matrix the viewport uses for the mesh, so the drag maths
    /// and the drawn model cannot disagree about what a transform means.
    pub fn matrix(&self) -> glam::Mat4 {
        glam::Mat4::from_scale_rotation_translation(
            Vec3::splat(self.scale),
            glam::Quat::from_rotation_z(self.rotate_z),
            self.translate,
        )
    }
}
