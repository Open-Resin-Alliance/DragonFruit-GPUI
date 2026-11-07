//! The supports system's 3D primitives.
//!
//! The React app builds a support out of instanced three.js primitives —
//! `SupportPrimitives/{Shaft,Knot,Joint,Roots,ContactCone,ContactDisk}` — one
//! draw per primitive type per batch. The port has no scene graph: it generates
//! the same shapes as plain triangle lists here and the viewport uploads them,
//! so a support set is one vertex buffer instead of hundreds of instanced draws.
//!
//! # PORT DEVIATIONS
//!
//! - **Instancing becomes baking.** three.js draws one `CylinderGeometry` per
//!   radius bucket and multiplies it by a per-instance matrix; here each shaft
//!   is emitted into one triangle list. Same silhouette, one draw.
//! - **Curved shafts are truncated.** `InstancedShaftGroup` renders a bezier
//!   shaft as a `TubeGeometry` warped along the curve; [`mesh::SupportMesh`]
//!   builds straight shafts only until the curve types are ported.
//! - **No raycast or gizmo.** `instancedRaycast.ts`, the knot/joint gizmos and
//!   the contact-disk drag controller are interaction, not geometry; they stay
//!   in the binary with the rest of the picking code.
//! - **Colour is not baked.** the source writes per-instance colours through
//!   `instanceColorWriter.ts`; the viewport applies its own material.

pub mod mesh;
pub mod primitives;

pub use mesh::{SupportMesh, SupportNode, SupportShaft};
pub use primitives::{cone, cylinder, disk, sphere};

/// `radialSegments` in `InstancedShaftGroup.tsx` — twelve around a shaft.
pub const RADIAL_SEGMENTS: usize = 12;
/// Rings in the knot/joint spheres (`SphereGeometry(widthSegments, heightSegments)`).
pub const SPHERE_RINGS: usize = 10;
