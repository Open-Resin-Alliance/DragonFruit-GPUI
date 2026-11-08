//! Ported from `src/components/gizmo/types.ts`.
//!
//! The source's gizmo carries nine callbacks and a dozen feature flags because
//! it drives a full transform on a group. The port's editor entity carries the
//! same three fields the source's `ModelTransform` does — a translation, a
//! three-component Euler and a per-axis scale — so every handle the source
//! draws has somewhere to write:
//!
//! * `center` / `axis-*` write `translate`;
//! * `ring-x` / `ring-y` / `ring-z` write one component of `rotation`;
//! * `scale-*` multiplies `scale` (all three axes while `uniformScaling` is on,
//!   which is the source's default and the only mode this build exposes).

use glam::{Mat4, Quat, Vec3};

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

    /// The ring's local `+X` and `+Y` axes in world space: the plane a rotation
    /// about this axis lives in, oriented exactly as `ringGroupEuler` orients
    /// the source's ring group.
    ///
    /// * `x`: the group turns by `+90°` about Y, so local `+X` is world `-Z` and
    ///   local `+Y` is world `+Y` (`[0, PI/2, 0]`).
    /// * `y`: the group turns by `-90°` about X, so local `+X` is world `+X` and
    ///   local `+Y` is world `-Z` (`[-PI/2, 0, 0]`).
    /// * `z`: the group is unrotated, so local is world (`[0, 0, 0]`).
    ///
    /// All three pairs are right-handed about the ring's normal (`u × v` is the
    /// axis), which is what makes a positive sweep a positive rotation about
    /// that axis — the sign the dial's `emittedDeltaForSweep` ends up applying
    /// in `SceneCanvas` (`setFromAxisAngle(axis, -angle)` of `-sweep`).
    pub fn ring_basis(self) -> (Vec3, Vec3) {
        match self {
            Axis::X => (-Vec3::Z, Vec3::Y),
            Axis::Y => (Vec3::X, -Vec3::Z),
            Axis::Z => (Vec3::X, Vec3::Y),
        }
    }
}

/// One grab target on the gizmo, the source's part ids (`center`, `axis-x`…,
/// `ring-x`…, `scale-x`…).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Handle {
    /// `move-center`: free movement in the plate plane.
    Center,
    /// `axis-<axis>`: constrained movement along one world axis.
    Axis(Axis),
    /// `ring-<axis>`: rotation about one world axis.
    Ring(Axis),
    /// `scale-<axis>`: scaling anchored at the gizmo centre. The three cubes
    /// are three grab points for one operation — `uniformScaling` is on, so
    /// each of them multiplies all three axes, exactly as `applyScaleFactor`
    /// does for `'uniform'`.
    Scale(Axis),
}

/// The three entity fields a drag writes, matching `ModelTransform` in
/// `src/features/scene` one for one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    /// World position, in millimetres.
    pub translate: Vec3,
    /// The Euler rotation in radians, `[x, y, z]`, about the fixed world axes —
    /// `quaternionFromGlobalEuler` in `src/utils/rotation.ts` builds
    /// `qz · qy · qx` from these, and the scene stores the same three numbers
    /// in an `Euler` of order `'ZYX'`.
    pub rotation: [f32; 3],
    /// Per-axis scale, `1.0` at the model's own size.
    pub scale: Vec3,
}

impl Transform {
    /// `quaternionFromGlobalEuler`: extrinsic XYZ, i.e. `Rz · Ry · Rx`, which
    /// is glam's `EulerRot::XYZEx` ("extrinsic three-axis rotation XYZ").
    pub fn rotation_quat(&self) -> Quat {
        Quat::from_euler(
            glam::EulerRot::XYZEx,
            self.rotation[0],
            self.rotation[1],
            self.rotation[2],
        )
    }

    /// The world-space matrix the viewport uses for the mesh, so the drag maths
    /// and the drawn model cannot disagree about what a transform means.
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation_quat(), self.translate)
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translate: Vec3::ZERO,
            rotation: [0.0; 3],
            scale: Vec3::ONE,
        }
    }
}
