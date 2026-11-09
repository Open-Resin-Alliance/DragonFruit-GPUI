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
//! here yet: the set is the only selection state this module keeps
//! (`Scene::selected`, with `primary()` derived), and every one of the other
//! concepts is migrated by a later wave.

use std::cmp::Ordering;
use std::sync::Arc;

use glam::{Mat4, Vec2, Vec3};

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

    /// The model's position in the source's sense — `transform.position`.
    ///
    /// The source's loader re-centres every geometry on its bbox
    /// (`useStlGeometry.ts:635-639`: `geometry.translate(-preCenter.x,
    /// -preBBox.min.y, -preCenter.z)`), so a source model's `position` *is*
    /// where its bbox centre sits in the world; the source puts the gizmo there
    /// for one model and for a selection (`SceneCanvas.tsx:7523-7527`,
    /// `getSelectionGizmoCenter`). This port keeps file-space vertices, so its
    /// `translate` is the file origin's placement and the same world point is
    /// `translate + mesh.center`.
    ///
    /// The point is invariant under rotation and scale — the model matrix
    /// pivots on `mesh.center` (`Model::matrix_for`) — so [`Model::world_center`]
    /// and this agree, and a gesture can read it without a matrix.
    pub fn position(&self) -> Vec3 {
        Vec3::from_array(self.translate) + self.mesh.center
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

    /// [`Model::world_bottom`] for a transform that has not been committed —
    /// what `preserveModelBottom` (`selectionLiftDrop.ts`) measures the
    /// before/after gap with while a gesture or a panel edit is live.
    pub fn world_bottom_for(&self, translate: Vec3, rotation: [f32; 3], scale: Vec3) -> f32 {
        let matrix = self.matrix_for(translate, rotation, scale);
        aabb_corners(&self.mesh)
            .map(|corner| matrix.transform_point3(corner).z)
            .fold(f32::INFINITY, f32::min)
    }

    /// Nearest positive ray hit, in world millimetres.
    pub fn raycast_distance(&self, origin: Vec3, direction: Vec3) -> Option<f32> {
        raycast_distance(&self.mesh, self.matrix(), origin, direction)
    }

    /// The model's footprint where it stands, as a world rect —
    /// `modelFootprintRect` (`useSceneCollectionManager.ts:2617-2631`): the
    /// projected size at the model's position. The source's geometry is
    /// centred on its bbox, so its position *is* the footprint centre; this
    /// port keeps file-space vertices, and `world_center` is the same point
    /// (the matrix pivots on `mesh.center`, which the matrix leaves fixed).
    pub fn footprint_rect(&self) -> Rect2D {
        let [width, depth] = projected_footprint_size(&self.mesh, self.rotation, self.scale);
        let center = self.world_center();
        Rect2D::from_center(center.x, center.y, width, depth)
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

/// A rectangle on the plate plane, in world millimetres.
///
/// The source's `Rect2D` (`useSceneCollectionManager.ts:2554`); the field
/// names are snake_case here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect2D {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
}

impl Rect2D {
    /// The rect whose centre is `(center_x, center_y)`.
    pub fn from_center(center_x: f32, center_y: f32, width: f32, depth: f32) -> Self {
        let half_width = width * 0.5;
        let half_depth = depth * 0.5;
        Self {
            min_x: center_x - half_width,
            max_x: center_x + half_width,
            min_y: center_y - half_depth,
            max_y: center_y + half_depth,
        }
    }

    pub fn center(&self) -> [f32; 2] {
        [
            (self.min_x + self.max_x) * 0.5,
            (self.min_y + self.max_y) * 0.5,
        ]
    }

    pub fn width(&self) -> f32 {
        self.max_x - self.min_x
    }

    pub fn depth(&self) -> f32 {
        self.max_y - self.min_y
    }

    /// `isRectInsidePlate` (`useSceneCollectionManager.ts:2577-2585`): every
    /// edge of `rect` is within this rect.
    pub fn contains(&self, rect: &Rect2D) -> bool {
        rect.min_x >= self.min_x
            && rect.max_x <= self.max_x
            && rect.min_y >= self.min_y
            && rect.max_y <= self.max_y
    }

    /// `intersectsRect` (`useSceneCollectionManager.ts:2556-2558`): touching
    /// edges count, because the source negates the separated-axis test.
    pub fn intersects(&self, rect: &Rect2D) -> bool {
        !(rect.max_x < self.min_x
            || rect.min_x > self.max_x
            || rect.max_y < self.min_y
            || rect.min_y > self.max_y)
    }
}

/// `computeProjectedFootprintSize` (`src/utils/modelFootprint.ts:209-248`):
/// the XY extents of the model's vertices after its own rotation and scale are
/// applied about the bbox centre, never below the source's 2 mm minimum.
///
/// PORT DEVIATIONS: the source scans a strided sample of the vertex soup and
/// takes the bounding box of a 2D convex hull; this scans every vertex, so the
/// extents are exact. The source's geometry is centred at import, matching the
/// `mesh.center` subtraction here.
pub fn projected_footprint_size(
    mesh: &MeshData,
    rotation: [f32; 3],
    scale: [f32; 3],
) -> [f32; 2] {
    let quaternion = rotation_quat(rotation);
    let scale = Vec3::from_array(scale);
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    for vertex in &mesh.vertices {
        let local = (Vec3::from_array(vertex.pos) - mesh.center) * scale;
        let projected = quaternion * local;
        min = min.min(Vec2::new(projected.x, projected.y));
        max = max.max(Vec2::new(projected.x, projected.y));
    }
    if !min.is_finite() || !max.is_finite() {
        return [2.0, 2.0];
    }
    [(max.x - min.x).max(2.0), (max.y - min.y).max(2.0)]
}

/// The source's `findFreeSpotCentersForModels` (`:2638-2806`) for one
/// incoming model: the first free centre in ring order around the plate
/// centre.
///
/// * `footprint` — the incoming model's `(width, depth)` from
///   [`projected_footprint_size`] (the source's `placementOffsets`).
/// * `blocked` — the world footprints already on the plate,
///   [`Scene::visible_footprints`]. The source merges
///   `estimateSupportBoundsForModel` into each of these; this build has no
///   support-bounds estimate, so the mesh footprint is the whole rect.
/// * `plate` — the active plate's build volume (`activePlateRect`,
///   `:2560-2575`). The port has one plate and draws it centred on the
///   origin, so `plate.center()` is `defaultImportCenterXY` (`:2536-2550`).
///
/// The numbers are the source's: `stepX = max(4, width + spacing)` and
/// `stepY = max(4, depth + spacing)` (`:2659-2660`), rings outward from the
/// centre sorted by squared distance (`:2692-2743`), a model larger than the
/// plate centred (`:2758-2762`), and the fallback column to the right when the
/// ring is exhausted (`:2796-2804`). The source then tries the scene's other
/// beds (`:2773-2790`); this build has one plate, so that loop has no
/// counterpart.
pub fn find_free_spot_center(
    footprint: [f32; 2],
    blocked: &[Rect2D],
    plate: Rect2D,
    spacing_mm: f32,
) -> [f32; 2] {
    let [center_x, center_y] = plate.center();
    let width = footprint[0].max(2.0);
    let depth = footprint[1].max(2.0);

    // Larger than the bed: centred on the plate being worked on, where it is
    // on screen and one scale away from fitting (`:2758-2762`).
    if width > plate.width() || depth > plate.depth() {
        return [center_x, center_y];
    }

    let spacing = spacing_mm.max(0.0);
    let step_x = (width + spacing).max(4.0);
    let step_y = (depth + spacing).max(4.0);

    // Enough rings to cross the plate on both axes plus the source's 12
    // outside rings (`:2692-2698`).
    let half_span_x = (center_x - plate.min_x)
        .abs()
        .max((plate.max_x - center_x).abs());
    let half_span_y = (center_y - plate.min_y)
        .abs()
        .max((plate.max_y - center_y).abs());
    let in_plate_ring_x = (half_span_x / step_x).ceil() as i32 + 2;
    let in_plate_ring_y = (half_span_y / step_y).ceil() as i32 + 2;
    let max_ring = in_plate_ring_x.max(in_plate_ring_y) + 12;

    // Ring order before the sort; the source's `!==` guards in the two loops
    // are always true for `ring > 0`.
    let mut candidates: Vec<([f32; 2], f32)> = Vec::new();
    for ring in 0..=max_ring {
        if ring == 0 {
            candidates.push(([center_x, center_y], 0.0));
            continue;
        }
        let ring_f = ring as f32;
        for gx in -ring..=ring {
            let x = center_x + gx as f32 * step_x;
            // The row above the centre, then the row below (`:2707-2722`).
            let y_top = center_y + ring_f * step_y;
            let (dx, dy) = (x - center_x, y_top - center_y);
            candidates.push(([x, y_top], dx * dx + dy * dy));
            let y_bottom = center_y - ring_f * step_y;
            let (dx, dy) = (x - center_x, y_bottom - center_y);
            candidates.push(([x, y_bottom], dx * dx + dy * dy));
        }
        for gy in -ring + 1..=ring - 1 {
            let y = center_y + gy as f32 * step_y;
            // The column right of the centre, then the column left
            // (`:2724-2740`).
            let x_right = center_x + ring_f * step_x;
            let (dx, dy) = (x_right - center_x, y - center_y);
            candidates.push(([x_right, y], dx * dx + dy * dy));
            let x_left = center_x - ring_f * step_x;
            let (dx, dy) = (x_left - center_x, y - center_y);
            candidates.push(([x_left, y], dx * dx + dy * dy));
        }
    }

    // `Array.prototype.sort((a, b) => a.distSq - b.distSq)` is stable; Rust's
    // `sort_by` is too, so equal distances keep the ring order above.
    candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));

    for (candidate, _) in candidates {
        let rect = Rect2D::from_center(candidate[0], candidate[1], width, depth);
        if !plate.contains(&rect) {
            continue;
        }
        if blocked.iter().any(|blocked| rect.intersects(blocked)) {
            continue;
        }
        return candidate;
    }

    // The ring is exhausted: the source puts the model down beside the plate
    // rather than refusing the import (`:2796-2804`). `maxRing + 2` clears the
    // searched area and every blocked footprint shifts the column one step
    // farther right.
    let fallback_x = center_x + (max_ring as f32 + 2.0 + blocked.len() as f32) * step_x;
    [fallback_x, center_y]
}

/// Click semantics for [`Scene::select`].
///
/// The source's `selectModel(id, mode)` (`useSceneCollectionManager.ts:3210-3231`)
/// has exactly these three modes; the canvas maps ctrl/meta to `Toggle`, shift
/// to `Add` and a plain click to `Single` (`StlMesh.tsx:1751-1758`), and the
/// Models panel rows map ctrl/meta to `Toggle` and a plain click to `Single`
/// (`ModelManagerPanel.tsx:639-640`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionMode {
    /// Exactly this model.
    Single,
    /// In when it was out, out when it was in.
    Toggle,
    /// In, keeping the rest — the source's `'add'`.
    Add,
}

/// The plate's models and the models the user has selected.
///
/// The selection set is the only selection state. The source kept
/// `selectedModelIds` *and* an `activeModelId` that disagreed with it
/// (`ModelManagerPanel.tsx:615` anchors on `activeModelId ?? last selected`;
/// `page.tsx:2618` falls back to `[activeModelId]` when no ids reach it; Delete
/// acted on `activeModelId` alone while the rows highlighted the set), and that
/// dual state is what made a selection survive or vanish depending on which
/// half a handler read. Here there is one truth and everything else derives
/// from it:
///
/// * [`Scene::primary`] is `selected.last()` — the model the user touched
///   last, which is the source's `activeModelId` wherever a single value is
///   needed. The source itself falls back to the last selected id when it has
///   no active one (`ModelManagerPanel.tsx:615`).
/// * the selection is never empty while a model is being transformed, so a
///   "selected but no primary" state — the source could reach it by deleting
///   the active model and leaving its peers selected
///   (`useSceneCollectionManager.ts:5517-5521`) — cannot exist here.
///
/// Neither field ever holds an id that is not in `models`:
/// [`Scene::remove`] prunes both.
pub struct Scene {
    pub models: Vec<Model>,
    /// The selected models, in the order they joined the set. Re-selecting an
    /// id moves it to the end, so the last entry is the primary model.
    pub selected: Vec<ModelId>,
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
            selected: Vec::new(),
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

    /// The world footprints of the visible models — the source's
    /// `blockedRects` (`useSceneCollectionManager.ts:2661-2688`), which
    /// filters `model.visible` and merges support bounds. Hidden models block
    /// nothing, matching the source.
    pub fn visible_footprints(&self) -> Vec<Rect2D> {
        self.models
            .iter()
            .filter(|model| model.visible)
            .map(Model::footprint_rect)
            .collect()
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

    /// The selection, in the order the models joined it.
    pub fn selected(&self) -> &[ModelId] {
        &self.selected
    }

    pub fn is_selected(&self, id: ModelId) -> bool {
        self.selected.contains(&id)
    }

    /// The single-value selection: the model the user touched last.
    ///
    /// This is the source's `activeModelId` — the gizmo's anchor, the transform
    /// panel's model, the name a toast shows. It is derived, never stored:
    /// see [`Scene`] for why.
    pub fn primary(&self) -> Option<ModelId> {
        self.selected.last().copied()
    }

    pub fn primary_model(&self) -> Option<&Model> {
        self.primary().and_then(|id| self.model(id))
    }

    pub fn primary_model_mut(&mut self) -> Option<&mut Model> {
        let id = self.primary()?;
        self.models.iter_mut().find(|model| model.id == id)
    }

    /// `selectModel(id, mode)` (`useSceneCollectionManager.ts:3210-3231`).
    ///
    /// A stale id (a row for a model that has since been removed) changes
    /// nothing and returns `false`. The source sets its active id to `id` in
    /// every mode; here that is expressed as `id` becoming the last entry, so
    /// `Toggle` on a selected id removes it and `Add` on a selected id promotes
    /// it to primary without duplicating it.
    pub fn select(&mut self, id: ModelId, mode: SelectionMode) -> bool {
        if self.model(id).is_none() {
            return false;
        }
        match mode {
            SelectionMode::Single => {
                self.selected.clear();
                self.selected.push(id);
            }
            SelectionMode::Toggle => {
                if let Some(index) = self.selected.iter().position(|sid| *sid == id) {
                    self.selected.remove(index);
                } else {
                    self.selected.push(id);
                }
            }
            SelectionMode::Add => {
                self.selected.retain(|sid| *sid != id);
                self.selected.push(id);
            }
        }
        true
    }

    /// `handleModelRangeSelection` (`page.tsx:5804-5815`) over the panel's own
    /// row order: `ids` is the range the caller computed from
    /// [`Scene::ordered_ids`], `clicked` is the row that was shift-clicked, and
    /// `additive` is ctrl/meta held with shift.
    ///
    /// The source sets the active id to `clicked` whatever the mode, so the
    /// clicked model becomes primary here too. Returns whether the range named
    /// anything in the scene; the source returns early on an empty range.
    pub fn select_range(&mut self, ids: &[ModelId], clicked: ModelId, additive: bool) -> bool {
        let valid: Vec<ModelId> = ids
            .iter()
            .copied()
            .filter(|id| self.model(*id).is_some())
            .collect();
        if valid.is_empty() {
            return false;
        }
        if !additive {
            self.selected.clear();
        }
        for id in valid {
            if !self.selected.contains(&id) {
                self.selected.push(id);
            }
        }
        // `setActiveModelId(activeId)` after the set write.
        if self.model(clicked).is_some() {
            self.selected.retain(|sid| *sid != clicked);
            self.selected.push(clicked);
        }
        true
    }

    /// [`Scene::clear_selection`]'s counterpart for a click past every model:
    /// `clearModelSelection` (`useSceneCollectionManager.ts:3236-3239`) empties
    /// the set and the active id together. Modifiers do not change that — the
    /// source's pointer-missed path passes no selection mode
    /// (`SceneCanvas.tsx:4997-5000`).
    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    /// The Models panel's row order — `orderedModelIds`
    /// (`ModelManagerPanel.tsx:362-367`), which is `buildModelGroups`' order:
    /// grouped models first, then one row per ungrouped model sorted by
    /// `a.name.localeCompare(b.name)` (`:159-186`).
    ///
    /// This build has no groups, so it is the ungrouped sort. Without the
    /// locale collator, case folding approximates `localeCompare` and the id
    /// breaks a tie, exactly as the panel's own rows do.
    pub fn ordered_ids(&self) -> Vec<ModelId> {
        let mut models: Vec<&Model> = self.models.iter().collect();
        models.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then(a.id.cmp(&b.id))
        });
        models.into_iter().map(|model| model.id).collect()
    }

    /// The models a panel action or a gizmo gesture targets —
    /// `resolveModelActionTargetIds` (`src/features/scene/modelActionTargets.ts`):
    /// the selection, deduplicated, in selection order.
    ///
    /// The source falls back to `activeModelId` when the selection is empty;
    /// with the set as the only truth the primary *is* the last selected id, so
    /// that fallback has nothing left to add and an empty set targets nothing.
    pub fn action_targets(&self) -> Vec<ModelId> {
        let mut targets: Vec<ModelId> = Vec::with_capacity(self.selected.len());
        for id in &self.selected {
            if self.model(*id).is_some() && !targets.contains(id) {
                targets.push(*id);
            }
        }
        targets
    }

    /// The selection's centre: the mean of the selected models' positions.
    ///
    /// `getSelectionGizmoCenter` (`src/features/scene/selectionPosition.ts:9-28`,
    /// fed by `SceneCanvas.tsx:4064-4075`) averages the models'
    /// `transform.position` — [Model::position] in this port, which is the
    /// world point the source's re-centred geometry stores there. A one-model
    /// selection lands on that model's own position, exactly where the
    /// single-model gizmo sat before the set existed.
    pub fn selection_center(&self) -> Option<Vec3> {
        let mut center = Vec3::ZERO;
        let mut count = 0;
        for id in self.action_targets() {
            if let Some(model) = self.model(id) {
                center += model.position();
                count += 1;
            }
        }
        (count > 0).then(|| center / count as f32)
    }

    /// Remove one model and drop it from the selection.
    ///
    /// The source filters the deleted ids out of `selectedModelIds`
    /// (`useSceneCollectionManager.ts:5520`) instead of promoting a neighbour.
    /// With no stored active id, the remaining members keep their order and the
    /// last one becomes primary — see [`Scene`].
    pub fn remove(&mut self, id: ModelId) -> bool {
        let Some(index) = self.models.iter().position(|model| model.id == id) else {
            return false;
        };
        self.models.remove(index);
        self.selected.retain(|sid| *sid != id);
        true
    }

    /// Remove every selected model — Delete's target. Returns how many went.
    pub fn remove_selected(&mut self) -> usize {
        let targets = self.action_targets();
        let removed = targets
            .iter()
            .filter(|id| self.remove(**id))
            .count();
        self.selected.clear();
        removed
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
        scene.select(first, SelectionMode::Single);
        if two {
            scene.push("b".into(), mesh(), [20.0; 3], [0.0; 3], [1.0; 3], true);
        }
        scene
    }

    /// Three models plus their ids, in import order.
    fn three_models() -> (Scene, [ModelId; 3]) {
        let mut scene = Scene::new();
        let a = scene.push("c".into(), mesh(), [0.0; 3], [0.0; 3], [1.0; 3], true);
        let b = scene.push("a".into(), mesh(), [20.0; 3], [0.0; 3], [1.0; 3], true);
        let c = scene.push("b".into(), mesh(), [40.0; 3], [0.0; 3], [1.0; 3], true);
        (scene, [a, b, c])
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

    /// `selectModel(id, 'single')` — a plain click replaces the set and the
    /// clicked model becomes primary.
    #[test]
    fn a_plain_click_selects_exactly_that_model() {
        let (mut scene, [a, b, c]) = three_models();
        assert!(scene.select(a, SelectionMode::Single));
        assert_eq!(scene.selected(), &[a]);
        assert_eq!(scene.primary(), Some(a));

        assert!(scene.select(c, SelectionMode::Single));
        assert_eq!(scene.selected(), &[c]);
        assert_eq!(scene.primary(), Some(c));

        scene.select(b, SelectionMode::Single);
        assert_eq!(scene.selected(), &[b], "single must drop the rest");
        assert_eq!(scene.primary(), Some(b));
        assert!(!scene.is_selected(a));
    }

    /// `selectModel(id, 'toggle')` — ctrl/meta in both the canvas and the rows.
    #[test]
    fn toggle_adds_then_removes_and_leaves_the_rest() {
        let (mut scene, [a, b, c]) = three_models();
        scene.select(a, SelectionMode::Single);

        scene.select(c, SelectionMode::Toggle);
        assert_eq!(scene.selected(), &[a, c]);
        assert_eq!(scene.primary(), Some(c), "a toggle-in becomes primary");
        scene.select(b, SelectionMode::Toggle);
        assert_eq!(scene.selected(), &[a, c, b]);
        assert_eq!(scene.primary(), Some(b));

        scene.select(c, SelectionMode::Toggle);
        assert_eq!(scene.selected(), &[a, b], "toggle-out removes only that id");
        assert_eq!(scene.primary(), Some(b));

        scene.select(b, SelectionMode::Toggle);
        scene.select(a, SelectionMode::Toggle);
        assert!(scene.selected().is_empty());
        assert_eq!(scene.primary(), None);
    }

    /// `selectModel(id, 'add')` — a canvas shift-click. Adding an id that is
    /// already in the set promotes it to primary without duplicating it, which
    /// is what `setActiveModelId(id)` did alongside the no-op set write.
    #[test]
    fn add_promotes_an_existing_id_instead_of_duplicating_it() {
        let (mut scene, [a, b, c]) = three_models();
        scene.select(a, SelectionMode::Single);
        scene.select(b, SelectionMode::Add);
        assert_eq!(scene.selected(), &[a, b]);

        scene.select(a, SelectionMode::Add);
        assert_eq!(scene.selected(), &[b, a]);
        assert_eq!(scene.primary(), Some(a));

        scene.select(c, SelectionMode::Add);
        assert_eq!(scene.selected(), &[b, a, c]);
        assert_eq!(scene.primary(), Some(c));
    }

    /// `handleModelRangeSelection(ids, clicked.id, 'replace')` with the range
    /// taken from `orderedModelIds`.
    #[test]
    fn a_shift_range_replaces_the_set_and_the_clicked_row_is_primary() {
        let (mut scene, [a, b, c]) = three_models();
        let ordered = scene.ordered_ids();
        // `a`("c"), `b`("a"), `c`("b") sort by name to `b, c, a`.
        assert_eq!(ordered, vec![b, c, a]);

        scene.select(b, SelectionMode::Single);
        // Shift-click the panel's last row: the whole ordered list is the range.
        assert!(scene.select_range(&ordered, a, false));
        assert_eq!(scene.selected(), &[b, c, a]);
        assert_eq!(scene.primary(), Some(a));

        // A narrower range from the middle to the end.
        assert!(scene.select_range(&ordered[1..], a, false));
        assert_eq!(scene.selected(), &[c, a], "replace drops b");
        assert_eq!(scene.primary(), Some(a));
    }

    /// Shift with ctrl/meta held: `handleModelRangeSelection(..., 'add')` keeps
    /// the previous selection and unions the range into it.
    #[test]
    fn a_ctrl_shift_range_unions_with_the_previous_selection() {
        let (mut scene, [a, b, c]) = three_models();
        let ordered = scene.ordered_ids(); // b, c, a
        scene.select(b, SelectionMode::Single);

        assert!(scene.select_range(&ordered[1..], c, true));
        // Membership is the union; the clicked id is last so it stays primary
        // (the source's separate `setActiveModelId(activeId)`).
        assert_eq!(scene.selected(), &[b, a, c]);
        assert_eq!(scene.primary(), Some(c));

        // The additive range repeats ids already held: no duplicates.
        assert!(scene.select_range(&ordered, a, true));
        assert_eq!(scene.selected(), &[b, c, a]);
        assert_eq!(scene.primary(), Some(a));
    }

    /// A range of stale ids changes nothing and reports it, like the source's
    /// early return on an empty range.
    #[test]
    fn a_stale_range_does_not_touch_the_selection() {
        let (mut scene, [a, ..]) = three_models();
        scene.select(a, SelectionMode::Single);
        assert!(!scene.select_range(&[ModelId(999)], ModelId(999), false));
        assert_eq!(scene.selected(), &[a]);
    }

    /// Deleting the primary leaves its peers selected and the next last member
    /// becomes primary — the state the source could not express, because it
    /// nulled `activeModelId` while keeping `selectedModelIds`
    /// (`useSceneCollectionManager.ts:5517-5521`).
    #[test]
    fn removing_the_primary_keeps_the_rest_and_promotes_the_previous_member() {
        let (mut scene, [a, b, c]) = three_models();
        scene.select(a, SelectionMode::Single);
        scene.select(b, SelectionMode::Add);
        scene.select(c, SelectionMode::Add);
        assert_eq!(scene.primary(), Some(c));

        assert!(scene.remove(c));
        assert_eq!(scene.models.len(), 2);
        assert_eq!(scene.selected(), &[a, b]);
        assert_eq!(scene.primary(), Some(b));

        assert!(scene.remove(a));
        assert_eq!(scene.selected(), &[b]);
        assert!(!scene.is_selected(a), "a removal prunes the selection");
    }

    #[test]
    fn removing_a_non_selected_model_leaves_the_selection_alone() {
        let (mut scene, [a, b, _]) = three_models();
        scene.select(b, SelectionMode::Single);
        assert!(scene.remove(a));
        assert_eq!(scene.selected(), &[b]);
        assert_eq!(scene.primary(), Some(b));
    }

    /// Delete acts on the whole selection (`page.tsx:8390`, `dispatchDeleteModelAction`).
    #[test]
    fn remove_selected_removes_every_selected_model() {
        let (mut scene, [a, b, c]) = three_models();
        scene.select(a, SelectionMode::Single);
        scene.select(b, SelectionMode::Toggle);
        assert_eq!(scene.remove_selected(), 2);
        assert_eq!(scene.models.len(), 1);
        assert_eq!(scene.models[0].id, c);
        assert!(scene.selected().is_empty());
        assert_eq!(scene.primary(), None);
    }

    /// `clearModelSelection` (`useSceneCollectionManager.ts:3236-3239`).
    #[test]
    fn clearing_empties_the_set() {
        let (mut scene, [a, b, c]) = three_models();
        scene.select(a, SelectionMode::Single);
        scene.select(b, SelectionMode::Toggle);
        scene.select(c, SelectionMode::Toggle);
        scene.clear_selection();
        assert!(scene.selected().is_empty());
        assert_eq!(scene.primary(), None);
        assert_eq!(scene.models.len(), 3, "clearing selects nothing; it does not delete");
        assert!(scene.action_targets().is_empty());
    }

    #[test]
    fn unbounded_ids_never_join_the_selection() {
        let (mut scene, [a, ..]) = three_models();
        scene.select(a, SelectionMode::Single);
        let stale = ModelId(999);
        assert!(!scene.select(stale, SelectionMode::Single));
        assert!(!scene.select(stale, SelectionMode::Toggle));
        assert!(!scene.select(stale, SelectionMode::Add));
        assert_eq!(scene.selected(), &[a]);
        assert_eq!(scene.action_targets(), vec![a]);
    }

    /// `resolveModelActionTargetIds`: the selection, deduplicated, and nothing
    /// when the set is empty.
    #[test]
    fn action_targets_are_the_selection_in_selection_order() {
        let (mut scene, [a, b, c]) = three_models();
        assert!(scene.action_targets().is_empty());
        scene.select(c, SelectionMode::Single);
        scene.select(a, SelectionMode::Add);
        scene.select(b, SelectionMode::Add);
        assert_eq!(scene.action_targets(), vec![c, a, b]);
    }

    /// The gizmo sits on the mean of the selected models' *positions* —
    /// `getSelectionGizmoCenter`'s average of `transform.position`
    /// (`selectionPosition.ts:9-28`). In this port the position is
    /// `translate + mesh.center` (the source's re-centred geometry stores the
    /// bbox centre there), and `three_models` offsets the translations on all
    /// three axes: a (0,0,0), b (20,20,20), c (40,40,40), with the fixture's
    /// bbox centre (5,5,2), so the positions are (5,5,2), (25,25,22) and
    /// (45,45,42).
    #[test]
    fn selection_centre_averages_the_selected_models() {
        let (mut scene, [a, b, c]) = three_models();
        scene.select(a, SelectionMode::Single);
        let single = scene.selection_center().unwrap();
        assert!((single - Vec3::new(5.0, 5.0, 2.0)).length() < 1e-4);

        scene.select(c, SelectionMode::Add);
        let pair = scene.selection_center().unwrap();
        assert!(
            (pair - Vec3::new(25.0, 25.0, 22.0)).length() < 1e-4,
            "pair centre {pair}"
        );

        scene.select(b, SelectionMode::Add);
        let three = scene.selection_center().unwrap();
        assert!(
            (three - Vec3::new(25.0, 25.0, 22.0)).length() < 1e-4,
            "triple centre {three}"
        );
    }

    /// The panel's row order: `a.name.localeCompare(b.name)`, with the id as a
    /// tie-break.
    #[test]
    fn ordered_ids_follow_the_models_panel_order() {
        let (scene, [a, b, c]) = three_models();
        // Names are "c", "a", "b".
        assert_eq!(scene.ordered_ids(), vec![b, c, a]);

        let mut scene = scene;
        scene.models[2].name = "A".into(); // "b" -> "A": case folding ties with "a"
        assert_eq!(scene.ordered_ids(), vec![b, c, a], "the id breaks the tie");
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

    /// The viewport's build volume (`viewport/construct.rs`'s
    /// `BUILD_VOLUME_MM`), centred on the origin.
    fn plate() -> Rect2D {
        Rect2D::from_center(0.0, 0.0, 218.88, 122.88)
    }

    #[test]
    fn first_import_lands_at_the_plate_centre() {
        let footprint = projected_footprint_size(&mesh(), [0.0; 3], [1.0; 3]);
        assert_eq!(
            find_free_spot_center(footprint, &[], plate(), 5.0),
            [0.0, 0.0]
        );
    }

    #[test]
    fn a_second_model_of_the_same_size_lands_one_step_away() {
        let footprint = projected_footprint_size(&mesh(), [0.0; 3], [1.0; 3]);
        let first = Rect2D::from_center(0.0, 0.0, footprint[0], footprint[1]);
        let center = find_free_spot_center(footprint, &[first], plate(), 5.0);
        // The fixture is 10 x 10 mm; `stepY = depth + spacing = 15` and the
        // (0, 15) candidate sorts before every corner.
        assert_eq!(center, [0.0, 15.0]);
        let second = Rect2D::from_center(center[0], center[1], footprint[0], footprint[1]);
        assert!(!second.intersects(&first), "footprints overlap");
        assert!(plate().contains(&second), "second rect left the plate");
    }

    #[test]
    fn a_model_larger_than_the_plate_is_centred() {
        let oversized = [plate().width() + 1.0, 10.0];
        assert_eq!(
            find_free_spot_center(oversized, &[], plate(), 5.0),
            [0.0, 0.0]
        );
    }

    #[test]
    fn the_ring_step_follows_each_footprint_axis() {
        // stepX = 45, stepY = 15: the near Y candidate wins over the far X one.
        let footprint = [40.0, 10.0];
        let first = Rect2D::from_center(0.0, 0.0, footprint[0], footprint[1]);
        let plate = Rect2D::from_center(0.0, 0.0, 400.0, 400.0);
        assert_eq!(
            find_free_spot_center(footprint, &[first], plate, 5.0),
            [0.0, 15.0]
        );
    }

    #[test]
    fn an_exhausted_ring_falls_back_to_the_column_right_of_the_plate() {
        // No ring-1 candidate fits this 20 x 20 mm plate, so the source's
        // fallback is `center + (maxRing + 2 + blocked) * stepX`.
        let plate = Rect2D::from_center(0.0, 0.0, 20.0, 20.0);
        let footprint = [10.0, 10.0];
        let blocked = Rect2D::from_center(0.0, 0.0, 10.0, 10.0);
        // maxRing = ceil(10 / 15) + 2 + 12 = 15, blocked = 1, stepX = 15.
        assert_eq!(
            find_free_spot_center(footprint, &[blocked], plate, 5.0),
            [270.0, 0.0]
        );
    }

    #[test]
    fn projected_footprint_follows_rotation_and_scale() {
        assert_eq!(
            projected_footprint_size(&mesh(), [0.0; 3], [1.0; 3]),
            [10.0, 10.0]
        );
        assert_eq!(
            projected_footprint_size(&mesh(), [0.0; 3], [2.0, 1.0, 1.0]),
            [20.0, 10.0]
        );
        // The fixture is flat in XY: a quarter turn about X maps its Y extent
        // onto Z, leaving depth at the source's 2 mm floor.
        let [width, depth] = projected_footprint_size(
            &mesh(),
            [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
            [1.0; 3],
        );
        assert!((width - 10.0).abs() < 1e-4, "width {width}");
        assert!((depth - 2.0).abs() < 1e-4, "depth {depth}");
    }

    #[test]
    fn hidden_models_do_not_block_a_free_spot() {
        let mut scene = scene_with(false);
        let footprint = projected_footprint_size(&mesh(), [0.0; 3], [1.0; 3]);
        assert_eq!(scene.visible_footprints().len(), 1);
        scene.models[0].visible = false;
        assert!(scene.visible_footprints().is_empty());
        assert_eq!(
            find_free_spot_center(footprint, &scene.visible_footprints(), plate(), 5.0),
            [0.0, 0.0]
        );
    }
}
