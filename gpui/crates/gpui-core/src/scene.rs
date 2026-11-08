//! The plate's models: identity, geometry and placement.
//!
//! The source keeps an array of `LoadedModel` records (a stable id, a display
//! name, the geometry, a `ModelTransform`, visibility and the per-model
//! material/group fields) plus a distinguished active model
//! (`src/features/scene/useSceneCollectionManager.ts:1373-1446`, `:1860`).
//! This module is the port's equivalent for the first slice: `Model` carries
//! identity, geometry and placement, `Scene` owns the list and the active id.
//!
//! The rotation/scale pivot is the model's own bbox centre, not the file
//! origin: the VOXL/STL payloads keep the loader's vertices in file space,
//! where the origin can be metres away, so the pivot has to be written into
//! the matrix (see `Model::matrix`). At identity the matrix is exactly
//! `T(translate)`, which is the placement the import path has always used.
//!
//! Selection sets, plates, groups, materials and supports are deliberately not
//! here yet: the first slice draws N models and picks one active model, and
//! every one of those concepts is migrated by a later wave.

use std::sync::Arc;

use glam::{Mat4, Vec3};

use crate::stl::{self, MeshData};

/// Stable identity of a model within a session.
///
/// The source's ids are uuids minted by the loader; the port allocates a
/// monotonic `u64` instead, because the only consumers are sets and the
/// viewport's upload cache, and neither cares what the document id looks like.
/// Ordering makes the ids usable as `BTreeMap` keys without hashing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModelId(u64);

impl ModelId {
    pub fn get(self) -> u64 {
        self.0
    }
}

/// One model on the plate.
pub struct Model {
    pub id: ModelId,
    /// Display name: the row title, the group-name default and the tooltip.
    /// Not the same thing as `mesh.name` — a shared mesh keeps its owner's.
    pub name: String,
    /// The geometry. `Arc` so a VOXL duplicate of a chunk shares one buffer
    /// with its owner and the viewport uploads it once.
    pub mesh: Arc<MeshData>,
    /// World position in millimetres (`MODL.transform.position`).
    pub translate: [f32; 3],
    /// Euler rotation in radians, `[x, y, z]` about the fixed world axes —
    /// `ModelTransform::rotation` (`Euler('ZYX')` in the source).
    pub rotation: [f32; 3],
    /// Per-axis scale, `1.0` at the model's own size.
    pub scale: [f32; 3],
    /// Eye toggle. Hidden models are not drawn and answer no pointer.
    pub visible: bool,
}

impl Model {
    /// `T(pivot) · T·R·S · T(-pivot)` with `pivot = mesh.center`.
    ///
    /// The centre is invariant under rotation and scale, which is what makes
    /// the gizmo's locked anchor correct (`gizmo::Drag::locked_center`).
    pub fn matrix(&self) -> Mat4 {
        self.matrix_for(
            Vec3::from_array(self.translate),
            self.rotation,
            Vec3::from_array(self.scale),
        )
    }

    /// The same pivot maths for a transform that has not been committed yet
    /// (the running gesture's draft).
    pub fn matrix_for(&self, translate: Vec3, rotation: [f32; 3], scale: Vec3) -> Mat4 {
        let pivot = self.mesh.center;
        let transform =
            Mat4::from_scale_rotation_translation(scale, rotation_quat(rotation), translate);
        Mat4::from_translation(pivot) * transform * Mat4::from_translation(-pivot)
    }

    /// The model's bbox centre in world space, through its own matrix.
    pub fn world_center(&self) -> Vec3 {
        self.matrix().transform_point3(self.mesh.center)
    }

    /// The top of this model's transformed AABB, in world space — the height
    /// the layer stack has to cover.
    pub fn world_top(&self) -> f32 {
        let matrix = self.matrix();
        aabb_corners(&self.mesh)
            .map(|corner| matrix.transform_point3(corner).z)
            .fold(f32::NEG_INFINITY, f32::max)
    }

    /// The lowest point of this model's transformed AABB, in world space.
    pub fn world_bottom(&self) -> f32 {
        let matrix = self.matrix();
        aabb_corners(&self.mesh)
            .map(|corner| matrix.transform_point3(corner).z)
            .fold(f32::INFINITY, f32::min)
    }

    /// Nearest positive ray hit, in world millimetres.
    pub fn raycast_distance(&self, origin: Vec3, direction: Vec3) -> Option<f32> {
        raycast_distance(&self.mesh, self.matrix(), origin, direction)
    }
}

/// `quaternionFromGlobalEuler`: extrinsic XYZ, i.e. `Rz · Ry · Rx`
/// (`Euler('ZYX')` in the source, glam's `XYZEx`).
pub fn rotation_quat(rotation: [f32; 3]) -> glam::Quat {
    glam::Quat::from_euler(glam::EulerRot::XYZEx, rotation[0], rotation[1], rotation[2])
}

/// World-space distance to the nearest triangle of `mesh` under `matrix`, or
/// `None` when the ray misses.
///
/// The ray is moved into the model's local frame, the mesh's own
/// `stl::raycast_distance` answers there, and the hit is moved back so models
/// can be ordered by one world distance even when their scales differ.
pub fn raycast_distance(
    mesh: &MeshData,
    matrix: Mat4,
    origin: Vec3,
    direction: Vec3,
) -> Option<f32> {
    let inverse = matrix.inverse();
    let local_origin = inverse.transform_point3(origin);
    let local_direction = inverse.transform_vector3(direction);
    let t = stl::raycast_distance(mesh, local_origin, local_direction)?;
    // `stl::raycast_distance` normalises the direction it is given, so the hit
    // point is reconstructed the same way.
    let hit = local_origin + local_direction.normalize() * t;
    Some((matrix.transform_point3(hit) - origin).length())
}

fn aabb_corners(mesh: &MeshData) -> impl Iterator<Item = Vec3> {
    let half = Vec3::from_array(mesh.size) * 0.5;
    let min = mesh.center - half;
    let max = mesh.center + half;
    (0..8).map(move |index| {
        Vec3::new(
            if index & 1 == 0 { min.x } else { max.x },
            if index & 2 == 0 { min.y } else { max.y },
            if index & 4 == 0 { min.z } else { max.z },
        )
    })
}

/// The plate's models, in import order, plus the active model.
///
/// `active` is the model the gizmo, the transform panel and every single-model
/// edit target — the source's `activeModelId`. It is `None` only when the
/// scene is empty or the user clicked past every model; it is never an id that
/// is not in `models`.
pub struct Scene {
    pub models: Vec<Model>,
    pub active: Option<ModelId>,
    next_id: u64,
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene {
    pub fn new() -> Self {
        Self {
            models: Vec::new(),
            active: None,
            next_id: 1,
        }
    }

    /// Append a model and return its id. Allocation order is the import order,
    /// so a scene built from a VOXL file lists its models in `MODL` order.
    pub fn push(
        &mut self,
        name: String,
        mesh: Arc<MeshData>,
        translate: [f32; 3],
        rotation: [f32; 3],
        scale: [f32; 3],
        visible: bool,
    ) -> ModelId {
        let id = ModelId(self.next_id);
        self.next_id += 1;
        self.models.push(Model {
            id,
            name,
            mesh,
            translate,
            rotation,
            scale,
            visible,
        });
        id
    }

    pub fn models(&self) -> &[Model] {
        &self.models
    }

    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    pub fn model(&self, id: ModelId) -> Option<&Model> {
        self.models.iter().find(|model| model.id == id)
    }

    pub fn model_mut(&mut self, id: ModelId) -> Option<&mut Model> {
        self.models.iter_mut().find(|model| model.id == id)
    }

    pub fn active_model(&self) -> Option<&Model> {
        self.active.and_then(|id| self.model(id))
    }

    pub fn active_model_mut(&mut self) -> Option<&mut Model> {
        let id = self.active?;
        self.models.iter_mut().find(|model| model.id == id)
    }

    /// Make `id` active. Returns whether it names a model in the scene, so a
    /// stale id (a deleted model) clears nothing.
    pub fn set_active(&mut self, id: ModelId) -> bool {
        if self.model(id).is_some() {
            self.active = Some(id);
            true
        } else {
            false
        }
    }

    pub fn clear_active(&mut self) {
        self.active = None;
    }

    /// Remove a model. If it was active, the first remaining model becomes
    /// active, which is the source's behaviour when the deleted id was the
    /// only selection.
    pub fn remove(&mut self, id: ModelId) -> bool {
        let Some(index) = self.models.iter().position(|model| model.id == id) else {
            return false;
        };
        self.models.remove(index);
        if self.active == Some(id) {
            self.active = self.models.first().map(|model| model.id);
        }
        true
    }

    /// The scene's height above the plate: the tallest visible model's top.
    /// `0` for an empty or fully hidden scene.
    pub fn max_height_world(&self) -> f32 {
        self.models
            .iter()
            .filter(|model| model.visible)
            .map(Model::world_top)
            .fold(0.0, f32::max)
    }

    /// The lowest visible point in world space, or `0` when nothing is
    /// visible — the plate's own plane is the floor.
    pub fn min_z_world(&self) -> f32 {
        self.models
            .iter()
            .filter(|model| model.visible)
            .map(Model::world_bottom)
            .fold(0.0, f32::min)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stl::Vertex;

    fn mesh() -> Arc<MeshData> {
        Arc::new(MeshData {
            vertices: vec![
                Vertex {
                    pos: [0.0, 0.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    ao: 1.0,
                },
                Vertex {
                    pos: [10.0, 0.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    ao: 1.0,
                },
                Vertex {
                    pos: [0.0, 10.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    ao: 1.0,
                },
            ],
            footprint: vec![[0.0, 0.0], [10.0, 0.0], [0.0, 10.0]],
            center: Vec3::new(5.0, 5.0, 2.0),
            radius: 7.0,
            triangles: 1,
            name: "test".into(),
            size: [10.0, 10.0, 4.0],
            volume: 1.0,
        })
    }

    fn scene_with(two: bool) -> Scene {
        let mut scene = Scene::new();
        let first = scene.push("a".into(), mesh(), [0.0; 3], [0.0; 3], [1.0; 3], true);
        scene.set_active(first);
        if two {
            scene.push("b".into(), mesh(), [20.0; 3], [0.0; 3], [1.0; 3], true);
        }
        scene
    }

    #[test]
    fn matrix_pivots_on_the_mesh_centre() {
        let scene = scene_with(false);
        let model = &scene.models[0];
        // At identity the pivot cancels: exactly `T(translate)`.
        assert_eq!(model.matrix(), Mat4::from_translation(Vec3::ZERO));
        // Under a rotation the centre is invariant.
        let rotated = model.matrix_for(
            Vec3::from_array(model.translate),
            [0.0, 0.0, std::f32::consts::FRAC_PI_2],
            Vec3::ONE,
        );
        let centre = rotated.transform_point3(model.mesh.center);
        assert!((centre - model.mesh.center).length() < 1e-4);
    }

    #[test]
    fn removing_the_active_model_promotes_the_next() {
        let mut scene = scene_with(true);
        let active = scene.active.unwrap();
        assert!(scene.remove(active));
        assert_eq!(scene.models.len(), 1);
        assert_eq!(scene.active, Some(scene.models[0].id));
        scene.remove(scene.models[0].id);
        assert!(scene.is_empty());
        assert_eq!(scene.active, None);
    }

    #[test]
    fn unbounded_ids_do_not_become_active() {
        let mut scene = scene_with(false);
        let stale = ModelId(999);
        assert!(!scene.set_active(stale));
        assert_ne!(scene.active, Some(stale));
    }

    #[test]
    fn scene_height_uses_the_visible_models_only() {
        let mut scene = scene_with(false);
        // Seated at the origin the AABB top is the mesh height.
        assert!((scene.max_height_world() - 4.0).abs() < 1e-4);
        scene.models[0].visible = false;
        assert_eq!(scene.max_height_world(), 0.0);
    }

    /// The slice's picking invariant: each model reports its **true** ray
    /// parameter in world millimetres, so models at different depths order by
    /// distance from the camera. The fixture's triangle sits at local `z = 0`,
    /// the camera ray starts at world `z = 200` pointing down `-Z`, and the
    /// model matrices only translate (the bbox pivot cancels), so the two hits
    /// are exactly 200 mm and 100 mm away.
    #[test]
    fn raycast_orders_models_by_world_distance() {
        let mut scene = Scene::new();
        let low = scene.push("low".into(), mesh(), [0.0; 3], [0.0; 3], [1.0; 3], true);
        let raised = scene.push(
            "raised".into(),
            mesh(),
            [0.0, 0.0, 100.0],
            [0.0; 3],
            [1.0; 3],
            true,
        );
        let origin = Vec3::new(2.0, 2.0, 200.0);
        let direction = Vec3::new(0.0, 0.0, -1.0);
        let hit = |id: ModelId| scene.model(id).unwrap().raycast_distance(origin, direction);
        let low_hit = hit(low).expect("low model hit");
        let raised_hit = hit(raised).expect("raised model hit");
        // Distances, not just their order: a bogus `t` (the barycentric `v`)
        // would make both 0.2 mm.
        assert!((low_hit - 200.0).abs() < 1e-3, "low hit at {low_hit}");
        assert!(
            (raised_hit - 100.0).abs() < 1e-3,
            "raised hit at {raised_hit}"
        );
        assert!(raised_hit < low_hit);
    }
}
