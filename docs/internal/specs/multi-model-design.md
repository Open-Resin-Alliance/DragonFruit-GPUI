# Multi-model data model — design note

Status: design, pre-implementation. Read-only investigation; nothing in this
note has been merged. It is the contract the implementation wave follows.

Prerequisite reading: `docs/internal/gpui-port-handoff.md` (§ decisions 5–7 in
particular), `docs/internal/specs/panels.md`, `docs/internal/specs/transforms.md`,
`docs/internal/specs/transforms-overlays-findings.md` and
`docs/internal/multi-material.md`.

The port models **one model**. Every panel, every pointer path and the entire
renderer assume it: `DragonFruit` owns `mesh: Option<Arc<MeshData>>` plus a
single `translate` / `rotation` / `scale` / `selected: Rc<Cell<bool>>`
(`gpui/src/app.rs:190-273`). The React app models a **plate of models**: an
array of `LoadedModel`, an active model id, a selection set, plates, groups and
per-model material/classification. The audits already recorded the consequence:
`transforms-overlays-findings.md` §2 says Arrange counts are `1/0` and the Group
rotation toggle can never render; `panels.md` §"Data the port lacks" lists the
model list, per-model selection and plates as missing. That is this note's
subject.

---

## 1. What the source models

### 1.1 The model record

`LoadedModel` is defined at `src/features/scene/useSceneCollectionManager.ts:1373-1446`.
The VOXL document shape it serialises to is `VoxlModelEntry`
(`src/features/scene/voxl/types.ts:62-130`). Fields, grouped by ownership:

| Field | Meaning | Kept where |
|---|---|---|
| `id` | Stable identity within the session. Not the file's id after a load unless the loader maps it (`useSceneCollectionManager.ts:7470-7475`). Every store, history entry, support and history label keys on it. | model record |
| `name` | Display name; also the default group name and the row tooltip. | model record |
| `geometry` | `GeometryWithBounds` — a `THREE.BufferGeometry` plus its bounds. The heavy object; snapshots clone the model, not this buffer (`cloneLoadedModel`, and the spill path in `sceneSnapshotSpill.ts`). | model record (shared) |
| `transform` | `ModelTransform` = `position: Vector3`, `rotation: Euler('ZYX')` in radians, `scale: Vector3`. The committed placement. | model record |
| `visible` | Eye toggle. Hidden models are skipped by the renderer, the pointer and every count (`SceneCanvas.tsx:2519`). | model record |
| `color` | Mesh colour. Per model, not per scene. | model record |
| `polygonCount`, `fileSizeBytes` | Panel info only. | model record |
| `sourcePath`, `originalRef`, `nativePreview` | Original-file linkage for reduced native previews. | model record |
| `plateId` | **Hint** for which plate the model belongs to; membership is resolved from position at read time (see 1.4). | model record |
| `groupId`, `groupName` | Import-group membership. A group has no entity of its own; the pair is stamped on each member and the list derives folders from it (`ModelManagerPanel.tsx:159-180`). | model record |
| `linkGroupId` | Transform-link membership, separate from `groupId`: linked models propagate transforms (`src/features/scene/voxl/__tests__/linkGroupTransformPropagation.test.ts`). | model record |
| `isSupportGeometry` | The model has been designated support geometry, not build geometry. | model record |
| `classification` | Baked `MeshHealthReport` for this model's triangle order (VOXL V3.3), so the model/support split survives a reload. | model record |
| `materialProfileId`, `materialBaseColor` | Per-model material assignment and the colour it replaced (multi-tank printers). | model record |
| `bodyMaterialProfileIds`, `bodyColors`, `splitBodies` | Multi-body 3MF: per-body material and the pre-computed body geometries. | model record |
| `meshModifiers`, `autoLift`, `liftRestoreMm` | Geometry-edit snapshot; lift behaviour after transforms. | model record |
| `fileUrl` | Blob URL for the current session's geometry. Not a document field. | session-only |

The important structural point: **a model owns its transform and its identity;
everything else about the scene is either a separate array (plates), a separate
store (supports) or derived at render time (groups, plate membership, selection
tints).**

### 1.2 Transform: committed versus live

Two transforms exist, and only one is in the array:

- `model.transform` is committed state. Every history capture, save and panel
  read uses it.
- `SceneCanvas` holds a live override for the **active model(s)** during a
  gesture (`SceneCanvas.tsx:2274-2327`, `:4023-4026`, `:4432-4456`). The override
  is a single `ModelTransform` in the port's terms; the source applies a group
  delta for a multi-selection (`selectionTransform.ts`, `rotateSelectionFromActive`).
  On release the override commits to every affected model's `transform` and one
  history entry lands.

The port already has this shape with `draft: Option<gizmo::Transform>`
(`app.rs:273`, `transform()/commit_draft()` at `:817-865`). The multi-model
change is to key the draft by model (or by gesture) instead of owning a single
`Option`.

### 1.3 Selection

State lives in the collection hook, not in a store:

- `activeModelId: string | null` (`useSceneCollectionManager.ts:1860`) — the
  model the gizmo, the transform panel and single-model actions target. It is
  the **last clicked** model, and a click always sets it.
- `selectedModelIds: string[]` (`:1861`) — the set the bulk actions operate on.
- `selectModel(id, mode)` (`:3210-3231`): `single` replaces the set with `[id]`;
  `toggle` removes if present else appends; `add` appends if absent. `activeModelId`
  is set to `id` in all three modes.
- `clearModelSelection()` (`:3238-3241`) clears both.
- A locked plate refuses selection for its models (`:3210-3219`; locking a plate
  also drops any standing selection, `:3233-3236`).
- The Models panel adds shift-range (`onSelectRange`) and group selection
  (`onSelectGroup`) on top of the three modes (`ModelManagerPanel.tsx:100-146`).
- Selection is what gates the gizmo, the transform panel's group-rotation mode
  (`selection_rotation_mode` set only for more than one model) and the arrange
  "Selected" counts.
- The renderer uses selection for the tint (`StlMesh.tsx`, `selectedTintStrength`,
  `activeModelId`, `selectedModelIds`) and the gizmo pivot for a multi-selection
  (`selectionPosition.ts:getSelectionGizmoCenter`).

So selection is a **set plus a distinguished active member**, not a boolean.
The set is authoritative for bulk operations; the active member is authoritative
for the gizmo and the panel's edit target. `useSceneCollectionManager.ts:3203-3208`
prunes ids that no longer exist.

### 1.4 Plates, and what "on a plate" means

- `ScenePlate = { id: string; name: string }` (`useSceneCollectionManager.ts:1366-1371`).
- `plates: ScenePlate[]` and `activePlateId: string` are state (`:1864-1877`).
  Plates are described as a **document fact**, deliberately outside the
  model-grouping snapshot machinery.
- `lockedPlateIds: string[]` is session state, not persisted with the scene
  (`:1886-1904`).
- Plate geometry is **derived**: `computePlateFrames` (`plates/plateLayout.ts:57-73`)
  places each plate from the printer footprint, the origin mode and the cascade
  ordering (`plateCascadeOffsetMm`). Frames are never stored per plate.
- Model membership is **derived from position**: `resolveModelPlateId`
  (`plateLayout.ts:78-95`) finds the frame containing the model's XY position,
  falls back to the stored `plateId` hint only when the model stands outside
  every frame, then to the first plate. `withPlateMembership` stamps the hint
  after a move, but the hint never overrides standing position.
- Multi-plate scoping (`plates/plateInteractivity.ts`): a model on the active
  plate is live; a model on another plate is *scenery* — dimmed by `SCENERY_DIM`
  (0.4), does not answer the pointer, unless it is selected, in which case it
  still answers so it can be dragged back. A model on no plate is `loose` and
  stays live. With one plate the rules are identity.
- `follow_landed_plate` is a setting (`src/features/settings` and the port's
  `MultiPlateState`): a drop that lands wholly on one plate may switch the
  active plate; selecting a plate from the Models panel does, a drag does not
  move the camera.

### 1.5 Groups and links

- `groupId` / `groupName` are **stamped on members**. `applyModelGrouping`
  (`modelGroupingHistory.ts:20-66`) picks a common existing group id or a new
  one, picks the first non-empty group name or the first member's name, and
  rewrites members. Ungrouping clears both.
- The Models panel derives folders: `buildModelGroups` groups by
  `groupId ?? "single-<id>"`, sorts grouped folders before strays, sorts members
  by name, and nests a plate's children (`ModelManagerPanel.tsx:159-180`,
  `:290-325`). With one ungrouped model the result is a single row with no
  folder header.
- `linkGroupId` is a different concept: linked models move/rotate/scale together
  outside the gizmo's multi-selection (`linkModels`, `unlinkModels` in the
  collection hook; propagation test under `voxl/__tests__`).

### 1.6 Material and classification

- Material is **per model** when the printer is multi-tank: `materialProfileId`
  names a profile, `materialBaseColor` remembers the pre-assignment colour, and
  clearing the assignment restores it (see `docs/internal/multi-material.md`
  "What is on the branch"). Multi-body models add a parallel array per body.
- `classification` is a baked per-model report; `isSupportGeometry` is a
  designation that removes the model from the build set. Both are model fields,
  not scene fields.

### 1.7 Supports are a separate store keyed by model

Supports do **not** live on `LoadedModel`. `src/supports/state` is an external
store of support entities; each entity carries a `modelId`
(`SupportProxyMeshLayer.tsx:645-661`, `getModelIdForSupportEntityId`, and the
`reassignAllSupportModelIds` / `removeSupportEntitiesForModels` /
`transformSupportsForModel` helpers imported by the collection manager at
`:28-36`). The rendering layer builds per-model entries, filters by
`visible`, and applies each model's transform to its supports. Deleting a model
deletes its supports; splitting/merging models reassigns ids.

### 1.8 What is stored, what is derived, what is persisted

| Category | Contents |
|---|---|
| **Stored** (hook state/refs) | `models[]` (immutable replacement per edit), `activeModelId`, `selectedModelIds`, `plates[]`, `activePlateId`, `lockedPlateIds`, mode/tool, view settings, clipboard, import report |
| **Derived per render** | `activeModel` (find by id), `modelById`, active plate + `plateName`, plate frames, per-model plate membership, scenery/dim sets, out-of-bounds list, model world bounds, grouping/folders, selection tints, projected meshes for marquee |
| **External store** | supports and rafts, keyed by model id; mesh modifiers by model id; auto-support mesh store |
| **History** | `SceneSnapshot { models, activeModelId, selectedModelIds, printerProfileId, plates, activePlateId, supportState, modifierRecord }` (`:320-341`); transform-only entries replay inverse transforms instead of restoring geometry; geometry may spill to disk |
| **Document (.voxl)** | `MODL` entries (transform, visibility, colour, plateId, groups, material, classification, mesh ref), `SCNE` (active/selected ids), `SUPP`, `MATP`/`BODY`; plate list is written through the scene/plate extension. Locked plates, camera and current tool are **not** in the file |

The port persists none of this per model today; it writes `session.json` only
(`gpui-store/src/session.rs`).

---

## 2. What the port has, and every place that assumes exactly one

The port totals 2,459 lines in `app.rs`, 866 in `tool_panels.rs`, 1,376 in
`panels/content.rs`, and 700 + 1,298 + 330 in the viewport's render, construct
and module files. The assumption is spread through all of them.

### 2.1 Entity state (`gpui/src/app.rs`)

| Location | Assumption |
|---|---|
| `:190-195` | `mesh: Option<Arc<MeshData>>`; `scene: Option<gpui_voxl::Scene>` is kept only as a bag of untouched JSON — its `models` vector is never drawn |
| `:213-215` | `selected: Rc<Cell<bool>>`, `hovered: Rc<Cell<bool>>` — one model's selection and hover |
| `:243-250` | one `layer`/`lower_layer`/`num_layers`, one `supports: Rc<RefCell<Option<SupportBatches>>>` |
| `:256-273` | `translate`, `rotation`, `scale`, `draft: Option<gizmo::Transform>` — a single placement and a single in-flight gesture |
| `:633-696` | `apply_loaded` installs the VOXL's first model's transform; the rest of the container is stored but not rendered |
| `:698-707` | `set_mesh` derives `num_layers` from the one mesh's height and selects it |
| `:793-814` | `delete_model` clears the mesh, scene, supports, selection, layers and resets the camera — deleting "the model" is deleting the plate |
| `:817-865` | `transform()` / `model_matrix()` / `commit_draft()` operate on one draft and one transform |
| `:882-901` | `gizmo_enabled` requires `mesh.is_some() && selected.get()`; `gizmo_center` is the one mesh's centre through `model_matrix()` |
| `:907-960` | `begin_gizmo_drag` passes `self.transform()` and the one mesh's `(min_z, center_z)` |
| `:966-1020` | `begin_grab` / `update_grab` write one draft's translation |
| `:1145-1157` | per-frame snapshot of one mesh, one model matrix, one grab-mode flag |
| `:1162-1184` | `clip_z` from the one mesh's world bottom and the one layer count |
| `:1220` | triangle total from the one mesh |
| `:1254-1271` | one `render` call with one mesh, one matrix, one clip |
| `:1483-1585` | hover and click raycast the one mesh through `model_matrix().inverse()`; a hit selects/deselects the boolean |
| `:1666-1668`, `:1913-1921` | the Models panel receives one `mesh_name`, one triangle count and the boolean selection |
| `:1796-1798` | camera focus uses the one mesh's centre/radius |
| `:1974-1977` | orientation panel receives one mesh name |

### 2.2 Transform contract

`model_matrix()` (`app.rs:844-851`) is `T(pivot) · T·R·S · T(-pivot)` with
`pivot = mesh.center` of the single mesh. The comment at `:829-843` states the
invariant the gizmo depends on: the model's centre is invariant under rotation,
which is what makes `Drag::locked_center` correct. With N models this must
become a per-model matrix derived from that model's own `MeshData::center`, and
the gizmo's anchor must become an explicit world point rather than an implicit
`mesh.center` lookup.

### 2.3 Viewport: one mesh buffer, one matrix, one uniform, one draw

- `Viewport` stores `mesh: Option<GpuMesh>`, `mesh_ptr: usize`,
  `mesh_matrix: Mat4` and a single `model_uniform` / `model_bind`
  (`viewport/mod.rs`, `~:158-176`; `uniforms.rs` `GpuMesh`, `ModelUniform`).
- `Viewport::render` takes `mesh: Option<&MeshData>`, `model: Mat4`,
  `clip_z: f32`, one base colour, one gizmo vertex list
  (`viewport/render.rs:25-39`).
- Upload/reuse logic keys on the mesh pointer and the matrix
  (`render.rs:58-121`); the footprint hull is rebaked through the one matrix.
- `ModelUniform` is written once per frame with one `matrix`, one `clip`
  (`[clip_z,0,0,0]`), one build-volume AABB and one `stripe` flag
  (`render.rs:224-266`).
- The mesh pass sets `model_bind` once and issues one `draw` per vertex chunk
  for the single mesh (`render.rs:478-485`). The footprint pass is the same
  (`render.rs:470-475`).
- The out-of-bounds test is a single model's AABB against the plate/volume
  rectangle (`render.rs:243-263`); the volume box's red edge colour is described
  as unwired in `viewport/mod.rs`'s deviations list.

Consequence: there is exactly one transform slot and one geometry slot in the
render pass. N models need either N uniform/bind pairs and N draws, or a
restructure of the mesh pass.

### 2.4 Gizmo

- `gizmo::Transform` (`gizmo/types.rs:96-128`) is the three-field placement;
  `Drag` holds exactly one `transform` and one `base`
  (`gizmo/drag.rs:52-84`), and `locked_center` returns one anchor
  (`drag.rs:139-147`).
- `gizmo::vertices` takes one `center` (`gizmo/mod.rs:152-...`) and a
  `GeometryCache` keyed on the pose/hover/active handle.
- The app drives it from `self.draft` / `self.gizmo_drag`; Escape discards one
  draft.

Consequence: a multi-selection has nowhere to put per-model transforms, and the
gizmo's pivot cannot be the centroid of a selection. The source's answer is
`getSelectionGizmoCenter` for the pivot and `rotateSelectionFromActive` for the
per-model result (`SceneCanvas.tsx:3020-3032`, `:7518+`).

### 2.5 Layers and clip

`num_layers` is `ceil(mesh.size[2] / LAYER_HEIGHT_MM)` (`app.rs:701`) and
`clip_z` is `mesh.min.z + translate.z + layer*height` (`:1162-1184`). There is
one layer slider for the whole viewport. In the source the layer count is a
scene/plate value (the tallest model on the plate; `page.tsx:11382`,
`ModelStatsCard.tsx:252`) and the clip plane is a global world Z
(`sceneClipStore`), so this is one of the few places the port's shape survives —
but the height must be the scene's max, not the one mesh's.

### 2.6 Supports

`self.supports` is one `SupportBatches` set for the whole scene (`app.rs:250`),
built from the one mesh (`panels/content.rs:665-670` for generation, and the
SUPP parse path). The viewport draws it with one bind group and four instanced
draws (`render.rs:430-478`). There is no model id on a batch, so removing or
hiding one model cannot hide only its supports.

### 2.7 Import

`gpui/src/scene/mod.rs:1-14` says it outright: the module turns **the first
model** into the port's single-model view. `load` (`:45-...)` takes
`models.first()`, `meshes.first()`, refuses a first model whose mesh is not
chunk 0, and returns one `Imported { mesh, translate, rotation, scale }`. The
`gpui-voxl` reader already returns `models: Vec<Value>` and every MESH payload
(`gpui-voxl/src/lib.rs:69-93`), so the loss happens in `scene::load`, not in the
container reader.

### 2.8 Session document

`gpui-store/src/session.rs:44-60` carries `rotation: [f32; 3]` with a comment
that calls it the port's single-model orientation. `app.rs:519-520` applies it
at startup to the empty scene, and `apply_loaded` overwrites it from the file
(`app.rs:639-647`). It is effectively a vestigial single-model field: there is
no per-model scene in the session document.

### 2.9 Panels

`PanelStore` (`features/tool_panels.rs:151-195`) caches `has_mesh: bool`,
`mesh_min_z: f32`, `mesh_center_z: f32`, `mesh_center: [f32; 2]`.
`sync_from_entity` (`:389-425`) writes `arrange.model_count = 1`, `selected_model_count
= 1/0`, `duplicate.active_model_name = Some(one name)`, `organic.disabled`,
`islands.has_geometry`. `features/transform_panel.rs` has no model list at all;
it is handed one `TransformState`. `panels/content.rs:246-283` builds exactly
one model row from `(mesh_name, triangles, selected)`; its group, folder, body
and context-menu functions are ported but `#[allow(dead_code)]` (`:512-626`).
The Export body takes no model list (`:1124-1129`, single
`stl::export_stl(mesh, model_matrix, path)`).

### 2.10 Automation and the empty scene

`automation::Snapshot` (`automation.rs:246-310`) exposes `has_mesh: bool`,
`selected: bool`, one `translate`/`rotation`/`scale`. Scenarios assert those
keys (`gpui/scenarios/new-scene.json`, `support-rail.json`, `app-bar.json`).
The empty-scene/stat cards read the same single-model numbers, and
`delete_model` is the only path to an empty plate.

### 2.11 Backend handoff

`gpui/backend` works on one staging mesh per call: `MeshStaging` holds one
triangle soup, `export_mesh_file` writes it, the slicer takes one
`MeshStaging` + job JSON (`backend/src/export.rs:183`, `backend/src/slicing.rs:554`,
`backend/src/staging.rs`). The source assembles a slice job with a `models[]`
array. Multi-model does not require changing these signatures in the first
slice, but a scene export/slice must eventually loop per model (or merge with
per-model transforms) and say which.

---

## 3. Proposed shape

### 3.1 Ownership

| Layer | Owns | Does not own |
|---|---|---|
| `gpui-core` (new `scene` module, no gpui dependency) | `ModelId`, `Model`, `Scene`, `Selection`, `Plate`, `Transform`, pure plate-frame/membership math, scene-level derived helpers (max height, bounds union, active model) | GPUI entities, rendering, supports store, persistence |
| `DragonFruit` (`gpui/src/app.rs`) | one `scene: Scene`, ephemeral gesture state (`draft` per gesture, `grab`, `gizmo_drag`, hover id), camera, modes, panels, supports handle, layer state, store | per-model transforms, selection, plate membership (they live in `Scene`) |
| `Viewport` | per-model GPU buffers and uniforms, keyed by `ModelId`/mesh pointer; one draw per visible model | selection semantics, plate scoping, transforms (it receives matrices) |
| `gpui-store` | session/recents/panel layout. `rotation` demoted to a legacy read-only key; no `Scene` persistence in the session | scene contents |
| Supports | one batch set per model, keyed by `ModelId` (phase 2), or one combined set (first slice) | model geometry/selection |

Why `gpui-core`: the scene is pure data and math, needs `glam` and
`Arc<MeshData>` only, and both the viewport and the app crate depend on
`gpui-core` already. Putting it in the app crate would force the viewport to
depend on the binary crate (it already cannot) and would make the plate math
untestable without a window. `gpui-store` is the wrong home: it is a
persistence crate with no `gpui` dependency and no geometry types.

### 3.2 Types (described, no code)

- **`ModelId`** — a newtype over a monotonic session id (source ids are uuids;
  the port can use a `u64` counter or the VOXL `id` string). It must be `Ord` +
  `Hash` for sets and stable maps. Recommend: keep the document id when the
  loader has one, otherwise allocate; store a `BTreeMap` separately if string
  ids complicate the viewport keys. The viewport can key on `ModelId` without
  caring.
- **`Transform`** — move `gizmo::Transform` into `gpui-core` verbatim
  (`translate: Vec3`, `rotation: [f32;3]`, `scale: Vec3`, `matrix()` pivoting
  done by the caller as today). `gizmo` re-exports it so call sites change
  mechanically; do **not** duplicate the type.
- **`Model`** — `id`, `name`, `mesh: Arc<MeshData>`, `transform`, `visible`,
  `color: u32`, `plate_hint: Option<PlateId>`, `group: Option<GroupRef>`
  (`id`, `name`), `link_group: Option<String>`, `support_geometry: bool`,
  `classification: Option<MeshHealthReport>`, `material: Option<MaterialAssignment>`
  (profile id + base colour + optional body arrays). `selected` and `active` are
  **not** fields on `Model` — they live in `Selection` so there is one truth.
- **`Selection`** — `active: Option<ModelId>` and `ids: BTreeSet<ModelId>`.
  Methods mirror `selectModel` (`single`/`toggle`/`add`), `clear`, `prune`
  (drop ids not in the scene, and fix `active` when it was dropped),
  `contains`, `is_multi` (ids.len() > 1). This makes the Group rotation toggle
  expressible and the panel counts real.
- **`Plate` / `PlateId`** — `id`, `name`. `Scene` holds `plates: Vec<Plate>` (at
  least one, created on demand) and `active_plate: PlateId`,
  `locked_plates: BTreeSet<PlateId>` (session-local; never serialised into a
  document). Frames are computed on read from the printer footprint + ordering,
  not stored.
- **`Scene`** — `models: Vec<Model>` in import order, `selection`, `plates`,
  `active_plate`, `locked_plates`. Scene-level derived methods: `active_model`,
  `model(id)`, `visible(id)`, `max_height_world()` (tallest visible model's top
  after transform), `world_bounds()` (union), `models_on_plate(plate)`,
  `model_plate(model)` (the derived rule), `is_outside(model)`.
  Optional `revision: u64` bumped on structural edits, so the viewport can diff
  cheaply instead of walking the vector every frame.

### 3.3 Selection and active model

- Every click resolves a `ModelId` (nearest ray hit, see 3.6) and calls the
  selection mode the modifiers imply: plain click = `single`, Shift = `add`,
  Ctrl/Cmd = `toggle`. The Models panel passes the same modes plus range add.
- `active` is set on every selection, matching the source. A bulk action that
  selects N models (import, arrange, split) sets `ids` to N and `active` to the
  first, exactly as the import path does (`useSceneCollectionManager.ts:7470-7475`).
- The gizmo, transform panel and single-model edits read `active`; Arrange
  "Selected", duplicate-selected, delete-selected and material assignment read
  `ids`. This split is what the source calls "active model" versus "selection".
- Plate lock filtering happens in `selectModel` (refuse), exactly as at
  `:3210-3219`; locking a plate clears ids on it.

### 3.4 Plates

- Add `compute_plate_frames(plates, footprint, ordering)` to `gpui-core` as a
  direct port of `plates/plateLayout.ts:57-73`; add its cascade offset helper
  for `plateCascadeOffsetMm` with the fixed/dynamic orderings (the settings tab
  already stores the ordering and columns in `MultiPlateState`,
  `features/settings/state.rs:139-172`).
- `Scene::model_plate(model)` implements `resolveModelPlateId`: containment in
  a frame wins, `plate_hint` is the fallback, first plate is the last resort.
  Stamp `plate_hint` after a move with `withPlateMembership`, so a model outside
  every bed remembers where it came from.
- `active_plate` gates the scene exactly as `plateInteractivity.ts`: `active`
  models are live; `other` models are drawn dimmed (`SCENERY_DIM = 0.4`) and
  answer no pointer unless selected; `loose` models stay live. With one plate
  every model is `active`.
- `follow_landed_plate` consumes the existing `MultiPlateState` setting when a
  drag lands models on one plate; an explicit plate pick in the Models panel
  switches regardless.
- Rendering N plates is **out of the first slice**: the viewport draws one plate
  at the origin today. Keep that until plate frames exist; then pass a list of
  plate rects/offsets to the viewport and draw each with the active one at full
  opacity (the source's ghost/dim plates).

### 3.5 Rendering: one buffer per model, not merged, not instanced

Recommendation: **keep one `GpuMesh` per model**, keyed by `ModelId` (with the
mesh `Arc` pointer as the sub-key so a replaced geometry re-uploads and a shared
`Arc` does not). Each cached entry owns its vertex chunks, footprint buffer,
`ModelUniform`, and `model_bind`. The frame iterates visible models and sets the
per-model bind group before its draws.

| Option | Verdict | Why |
|---|---|---|
| One merged vertex buffer for the scene | Reject | Picking needs per-model identity; selection tint, visibility, per-model material/colour, per-model out-of-bounds stripes and per-model clipping all become impossible or require vertex attributes and index ranges. Any transform change rebuilds the whole scene buffer. |
| Instancing one mesh N times | Reject | Models have distinct geometry; instancing only helps duplicates, and the source already dedupes identical chunks at the file level (`chunkIndex`). |
| One buffer per model | **Adopt** | Direct extension of today's `GpuMesh` cache; N draws is what the source does (one `StlMesh` per model, `SceneCanvas.tsx:6780-7047`); per-model uniforms give per-model tint, stripe, clip and matrix for free; a transform drag touches only that model's uniform. |

Consequences:

- **Picking**: compute `ray` in world space, then for each pointer-relevant,
  visible model transform the ray by that model's `model_matrix().inverse()`,
  call the mesh raycast, and keep the nearest positive world-space `t`. The
  current `stl::raycast` returns `bool` (`gpui-core/src/stl.rs`, ~:265-320); add
  a `raycast_distance(mesh, origin, dir) -> Option<f32>` (the same slab + Möller–
  Trumbore loop, returning `t`) and leave `raycast` as a wrapper. Respect plate
  scoping: only active/loose models, plus selected models on other plates.
- **Hover**: `hovered: Rc<Cell<Option<ModelId>>>` (or a plain field on the
  entity). Resolve once per move; the tint becomes per-model through the
  uniform.
- **Gizmo**: `gizmo_center` becomes `selection_center`: the active model's
  bbox centre through its matrix for one model; the centroid/bounds centre of
  the selected models' world AABBs for many (`getSelectionGizmoCenter`). `Drag`
  keeps its one-anchor maths; the app applies the resulting delta to each
  selected model (first slice: only the active one). `locked_center` still
  returns the anchor for ring/scale drags.
- **Out-of-bounds**: the CPU AABB test runs per model inside the frame loop and
  sets that model's uniform `stripe.w`. The build-volume box's red edge is a
  frame-level aggregate: `any(model.stripe)`.
- **Upload caching**: keep the existing "pointer + matrix unchanged means no
  upload" rule per model. Removing a model drops its cache entry. If `Scene`
  gains `revision`, diff ids and drop stale entries; otherwise a `retain` over
  the cached map each frame is cheap for realistic model counts.
- **Duplicate geometry**: a VOXL v3 file shares a MESH chunk across duplicates.
  The loader should parse the chunk once and hand every duplicate the same
  `Arc<MeshData>`; `Arc::as_ptr` then naturally dedupes the GPU upload while the
  per-model uniform still differs.

### 3.6 Layer count and clip plane

- `num_layers` becomes `ceil(scene.max_height_world() / LAYER_HEIGHT_MM)`, where
  max height is over the visible models on the active plate (or all models when
  there is one plate), matching the source's plate-scoped estimate.
- `clip_z` stays a single world-space plane; its lower bound is the scene's
  minimum visible Z (or the plate's 0) and its upper bound is
  `min_z + layer * height + epsilon`. No per-model clip state is needed.
- Per-model cross-section caps and the stencil cap renderer are out of scope;
  the existing single clip plane applies to every model, which is correct for a
  world-space section.

### 3.7 Supports, materials, classification

- **First slice**: keep one combined `SupportBatches` but regenerate it from the
  scene (transform each model's generated points by that model's matrix). This
  preserves today's rendering while fixing the transform bug class where
  supports stay at the old placement.
- **Next slice**: change `Option<SupportBatches>` to `BTreeMap<ModelId,
  SupportBatches>` (or a `Vec<(ModelId, SupportBatches)>`), add `model_id` to
  the batch, and let the viewport draw each set with its own transform. Hiding
  or deleting a model then hides/deletes only its supports, which is the
  source's rule.
- Materials: `Model.material` is data only in this wave; the renderer already
  takes one base colour per draw, so per-model colour/tint is free. Material
  assignment UI and the silhouette-on-selected colour rule stay on the ledger
  until the multi-material branch is adopted.
- Classification: `support_geometry` models are excluded from the build set and
  drawn as supports (or hidden) according to the source's support-designation
  rule; `classification` is carried through for the repair/split panels.

### 3.8 Persistence and the session document

- `Scene` owns per-model transforms. `session.rotation` must stop being an
  authoritative value: keep the field readable for old `session.json` files
  (schema v1) and write it as the first model's rotation (or zeros) for
  compatibility. Do **not** add per-model scene data to `session.json`; the
  source never persists geometry in the session, and the `.voxl` file is the
  document.
- When VOXL export lands, the writer iterates `Scene.models` and writes each
  model's transform/visibility/colour/plate hint/group/material, which is
  exactly `VoxlModelEntry`. That is a later wave; this note only requires the
  in-memory shape to be sufficient for it.

### 3.9 Invariants

1. `Scene.selection.ids` contains only ids present in `Scene.models`;
   `active` is either `None` or in `ids`.
2. Every `Model` has a non-empty `id`; ids are unique and survive renames.
3. A model's plate membership is never authoritative state; `plate_hint` is a
   fallback only.
4. The viewport never sees a model without a matrix; the app derives one per
   model from `Transform` and `mesh.center` (same pivot rule as today).
5. No draw call uses one model's uniform with another model's vertex buffer;
   the bind group is set immediately before each model's draws.
6. `clip_z` and `num_layers` are scene-level, never per-model.
7. Selection writes go through `Selection`/`Scene` methods; no panel mutates a
   model's selection flag.

---

## 4. Migration path

Each step ends with the handoff's single gate: `cargo check`, `cargo test`,
build, `--smoke`, `check:docs`, commit (`gpui-port-handoff.md` decision 7). No
step may leave half-migrated callers.

| # | Step | Behaviour change | Keeps gates green because |
|---|---|---|---|
| 0 | **Baseline fixtures.** Add a two-model VOXL fixture (two `MODL` entries, two `MESH` chunks, distinct translates) and a scenario that loads it; record today's smoke output as the "one model drawn" baseline. | No | Test data only. |
| 1 | **Move `Transform` into `gpui-core`** (new `scene` module) and re-export from `gizmo`. Move `model_matrix`'s pivot maths into a free function that takes a transform + local centre. Update imports. | No — mechanical | Same values; existing tests (gizmo, placement) pass unchanged. |
| 2 | **Add `Scene`/`Model`/`Selection`/plates maths alongside the singleton**, plus unit tests for selection transitions, plate frames, membership, max height, and per-model matrix. `gpui/src/scene::load` gains a function that returns `Vec<ImportedModel>` while the existing `Imported` stays. | No | Nothing consumes the new API yet; the old path still builds. |
| 3 | **Switch `DragonFruit` storage** to `scene: Scene` with exactly one model, keeping the old accessor surface as methods (`mesh()`, `transform()`, `model_matrix()` reading the active model). Keep `selected` as a derived accessor until step 5. | No — mechanical refactor | All panels/callers keep their shapes; only field access changes. |
| 4 | **Multi-model import and rendering.** `load` returns every model; `apply_loaded` fills `Scene`; the viewport takes a slice of per-model render inputs, caches one `GpuMesh` each, draws visible models. Delete the singleton mesh fields. This is the first real behaviour change: a VOXL with N models draws N. | **Yes** | Gate must add the two-model smoke assertion (painted pixels and both model names logged). All single-model scenarios still pass because `has_mesh` becomes `!models.is_empty()`. |
| 5 | **Selection set.** Replace `selected: Rc<Cell<bool>>` and `hovered: Rc<Cell<bool>>` with `Selection` + `Option<ModelId>`; picking returns the nearest model id; click modes are single/add/toggle; the Models panel gains the real list (rows from `Scene.models`, grouped by `group`, selected/active styling). Panels read `active`/`ids` counts. | **Yes** | All callers must move in this step; `tool_panels::sync_from_entity` and `automation::Snapshot` change in the same commit, and the scenarios are updated together. |
| 6 | **Gizmo on the active model; draft per gesture.** `gizmo_center` uses the active model or the selection centroid; `draft` becomes per-gesture (a map or a small struct with the base transforms of every affected model); a drag writes only the active model in this step. Group transform of N models lands after the step. | Partial — same behaviour for one model | Gizmo tests unchanged; the Group toggle can now render but may remain disabled until step 6b. |
| 6b | **Multi-selection transform.** Apply a world delta to every selected model (`selectionTransform.ts` equivalent), with the pivot at `getSelectionGizmoCenter`; commit all base transforms in one gesture. | **Yes** | New tests for the delta maths; one-model path is the identity case. |
| 7 | **Plates.** Add `Plate` records and frames, `active_plate`, derived membership, `plate_hint` stamping, plate scoping for picking and dimming. The Multi-Plate settings tab gains its consumer. The viewport still draws one plate until the plate render pass lands; models on other plates can be dimmed immediately. | **Yes** | Plate math is pure and unit-tested before it is wired; the one-plate path is the identity. |
| 8 | **Per-model supports, materials, classification.** Key `SupportBatches` by model id; carry `material` and `classification` through the loader; per-model colour from `materialBaseColor` when assigned. | **Yes** | Each sub-step has a single-model fallback (one map entry) and existing support tests. |

Mechanical steps: 1, 2, 3. Behaviour-changing steps: 4, 5, 6b, 7, 8.
Step 4 is the point of no return for the viewport signature; steps 5–7 are the
point of no return for panel state.

**Do not** attempt steps 4 and 5 in parallel: the picking path, `PanelStore`,
`automation::Snapshot` and the Models panel must land together, or the gate sees
a selection set that no caller reads.

---

## 5. Blast radius

Counts are approximate and derived from the anchors above: "changed" includes
the lines a step rewrites, moves or re-types, not blank lines. New modules are
counted separately.

### 5.1 Files and lines

| File | Lines now | Est. changed | Nature of change |
|---|---:|---:|---|
| `gpui/crates/gpui-core/src/scene.rs` (**new**) | — | ~450–550 | Model/Scene/Selection/Plate/Transform, plate frames/membership, max height, bounds; unit tests |
| `gpui/crates/gpui-core/src/lib.rs` | small | ~5 | export the module |
| `gpui/crates/gpui-core/src/stl.rs` | 342 | ~25 | `raycast_distance` + tests; keep `raycast` as wrapper |
| `gpui/crates/gpui-core/src/viewport/render.rs` | 700 | ~170 | per-model input slice, per-model upload cache, uniform per model, draw loop, per-model stripe |
| `gpui/crates/gpui-core/src/viewport/mod.rs` | ~330 | ~35 | `GpuMesh` map + per-model bind groups; cache eviction |
| `gpui/crates/gpui-core/src/viewport/uniforms.rs` | ~100 | ~5 | doc/ownership notes; `ModelUniform` unchanged |
| `gpui/crates/gpui-core/src/viewport/construct.rs` | 1,298 | ~20–40 | create per-model uniform buffers/binds lazily rather than one pair |
| `gpui/src/app.rs` | 2,459 | ~500–600 | struct fields; load/apply/delete; transform/model_matrix; gizmo wiring; render closure; picking; layer/clip; session document; panel args |
| `gpui/src/automation.rs` | 1,004 | ~50–60 | `Snapshot` gains `model_count`, `active_model_id`, `selected_model_ids`; `has_mesh`/transform kept as single-model aliases; expect keys |
| `gpui/src/scene/mod.rs` | ~216 | ~120–150 | enumerate all models, per-model transform/name/visibility, chunk-index resolution, shared `Arc` dedupe |
| `gpui/src/gizmo/types.rs` | ~150 | ~40 | `Transform` re-export/import move |
| `gpui/src/gizmo/drag.rs` | ~280 | ~10–20 | type import; no maths change until 6b |
| `gpui/src/gizmo/mod.rs` | 163 | ~5 | re-export |
| `gpui/src/features/tool_panels.rs` | 866 | ~120–150 | `PanelStore` fields; `sync_from_entity` reads `Scene`; counts from selection |
| `gpui/src/features/transform_panel.rs` | 910 | ~10–20 | state still one model; `selection_rotation_mode` becomes reachable |
| `gpui/src/features/arrange_panels.rs` | 1,074 | 0 | already takes counts as data |
| `gpui/src/panels/content.rs` | 1,376 | ~250–320 | real model list (rows/groups/plates), selection modes, export model list, supports/arrange counts |
| `gpui/src/widgets/orientation_panel.rs` | small | ~10–15 | active model name |
| `gpui-store/src/session.rs` | ~430 | ~10–15 | `rotation` demoted/documented; no schema bump required if kept |
| `gpui/scenarios/*.json` | 3 files | ~20–40 | keep `has_mesh`; add model count / active id; new two-model scenario |
| `gpui-voxl/src/lib.rs` | 450 | 0 (optional ~80) | already multi-model; optionally a typed `VoxlModel` accessor |

Total: **~1,700–2,200 changed lines across ~18 files**, of which ~500 are new
`gpui-core` scene code and its tests. The largest single file is `app.rs`
(single-model state removal), then `panels/content.rs` (the real list), then
`viewport/render.rs`.

### 5.2 Modules that gain or lose a field

| Module | Gains | Loses |
|---|---|---|
| `DragonFruit` | `scene: Scene`, `hovered: Option<ModelId>` | `mesh`, `translate`, `rotation`, `scale`, `selected: Cell<bool>`, `hovered: Cell<bool>`; `draft` becomes per-gesture |
| `Viewport` | `meshes: BTreeMap<ModelId, GpuMesh>`, per-model uniform/bind, `scene_revision` (optional) | `mesh: Option<GpuMesh>`, `mesh_ptr`, `mesh_matrix`, `model_uniform`, `model_bind` |
| `PanelStore` | `model_count`, `selected_model_count` (real), `active_model_name` (id-aware), `scene_min_z/max_z` | `mesh_min_z`, `mesh_center_z`, `mesh_center` (or they become scene aggregates) |
| `automation::Snapshot` | `model_count`, `active_model_id`, `selected_model_ids`, per-model transforms (optional) | keeps single `translate/rotation/scale` as active-model aliases |
| `gpui_store::Session` | nothing required | authority of `rotation` (kept as legacy field) |
| `gpui_voxl::Scene` | nothing | nothing |
| supports | `ModelId` on batches (step 8) | nothing |
| `Scene` (new) | all model/selection/plate state | — |

### 5.3 Tests, scenarios and gates that must change

- `gpui/crates/gpui-voxl/src/tests.rs:82-109` already asserts a multi-model
  header and `scene.models.len() == 1` for its fixture; add a two-model case
  (it already parses `Vec`).
- `gpui/crates/gpui-store/tests/store.rs` round-trips `Session` including
  `rotation`; keep the key and the test, add a note that it is legacy.
- `gpui/scenarios/new-scene.json`, `support-rail.json`, `app-bar.json`: keep the
  `has_mesh`/`selected` keys working (derive from `Scene`), add
  `model_count`/`active_model_id` to the new scenario, and assert the delete
  path empties the scene.
- New `--smoke` assertion: the two-model fixture logs two model lines (name,
  triangles, size, world translate) and the readback still paints a non-empty
  region. The current gate only prints one mesh (`app.rs:698-707`).
- New unit tests in `gpui-core`: selection single/toggle/add/prune; plate frames
  fixed/dynamic and front-left/center origins; membership from position with
  hint fallback; max height; per-model matrix centre invariance; `raycast_distance`
  nearest-hit ordering.
- Viewport tests are GPU-bound; the practical gate is the smoke run plus the
  scenario assertion that two distinct model ids are hoverable/pickable
  (automation can expose the id after a click). If synthetic clicks are too
  brittle, the scenario asserts `model_count` and the smoke log proves two
  uploads; picking is covered by the pure `raycast_distance` tests.
- `check:docs` will see this file; no published-doc changes are required.

### 5.4 What breaks if the change is done naively

- **The gizmo's single-transform contract**: `Drag` and `draft` assume one
  transform. Wiring N models without keying the draft produces the class of bug
  already seen in the source's support drags — a gesture committing the wrong
  model. Fix: `draft` is owned by the gesture, with base transforms captured at
  press.
- **The viewport's one-mesh upload**: keeping `mesh_ptr`/`mesh_matrix` as
  scalar fields means the second model either overwrites the first's buffer or
  draws the first model's geometry with the second's matrix. Fix: a map keyed by
  id and the `Arc` pointer; assert in the draw loop that the bind group and the
  vertex buffer belong to the same entry.
- **The session's single `rotation`**: writing it from the active model on exit
  while reading it into the first model on launch silently rotates one model.
  Fix: stop treating it as authoritative; document the legacy read.
- **The out-of-bounds stripe** is already a per-ModelUniform field, so it
  becomes correct for free once uniforms are per model — but the build-volume
  box's red aggregate needs an explicit `any` flag, or the box stays blue while
  a model is outside.
- **Picking by `bool`**: `stl::raycast` answers "hit", not "which". Looping
  models with `bool` picks the first model tested, not the nearest. The
  distance-returning variant is required in step 4, not deferred.

---

## 6. Recommendation: the smallest first slice

**Prove the model with two models loaded from one VOXL file, both drawn, and
exactly one selectable at a time.**

Scope of the slice (steps 1–4 plus the single-selection part of step 5):

1. `gpui-core::scene` with `ModelId`, `Model`, `Transform`, `Scene`,
   `Selection` (single mode only for now), plate math stubbed to one plate.
2. `scene::load` parses **every** `MODL` entry: each model's own MESH chunk
   (honouring `chunkIndex`, sharing `Arc<MeshData>` for duplicates), name,
   transform, visibility and colour. Reject only what cannot be resolved.
3. `DragonFruit` holds `scene: Scene`; `open_stl` still replaces the scene with
   one model (unchanged behaviour for STL).
4. `Viewport` holds one `GpuMesh` per model (keyed by `ModelId` + `Arc` pointer)
   and draws every visible model with its own `ModelUniform`/`bind`. Keep the
   existing footprint, mesh pipeline, clip plane and support pass; clip and
   layers derive from the scene's max height.
5. Picking returns the nearest model id via `raycast_distance`; a plain click
   sets `active` and the single-id selection. Hover is per model. The gizmo
   follows the active model only; a drag moves/rotates/scales that model only.
6. The Models panel lists every model as a row with the real count; clicking a
   row selects it. `PanelStore` counts (`model_count`,
   `selected_model_count`), `duplicate.active_model_name` and the Transform
   panel's position/rotation/scale read the active model.
7. Automation exposes `model_count` and `active_model_id`; `has_mesh` remains
   `!models.is_empty()` so every existing scenario compiles and passes. A
   new two-model scenario asserts count = 2 and that a click changes
   `active_model_id`.

Acceptance criteria for the slice:

- A two-model VOXL draws both models, each at its own transform, in one frame;
  the smoke log names both.
- Clicking either model makes it the only selected model; the gizmo sits at
  that model's centre; dragging moves only it, and releasing commits only its
  transform.
- Deleting the active model leaves the other model on the plate and selects
  it (or clears if none remain) instead of clearing the scene.
- The Models panel shows two rows, and the Arrange header reads "2 models".
- `has_mesh`, `selected`, `translate`/`rotation`/`scale` scenario keys still
  answer (as active-model aliases) so the three existing scenarios are green.
- `cargo check`, `cargo test`, the build, `--smoke`, `check:docs` all pass with
  one commit.

The slice **explicitly does not attempt**:

- Multi-selection: `add`/`toggle`, Shift-range, group selection, marquee.
- Group transform of a selection; the gizmo's group pivot; the Group rotation
  toggle's behaviour (it may render, but acts as a single-model toggle).
- Plates: no plate records, no `active_plate`, no cascade, no scenery dimming,
  no plate create/rename/remove. One plate at the origin remains.
- Arrange, duplicate, split-to-bodies, mirror, on-face, organic cut, repair,
  hollowing and any other engine that operates on a selection set.
- Per-model supports (the support set stays combined and regenerated from the
  active/scene mesh), per-model materials beyond the colour field, per-model
  classification, and multi-body 3MF bodies.
- VOXL save/round-trip, autosave, scene history/undo, and any change to
  `session.json` beyond demoting `rotation` to a legacy read.
- Camera framing over the full scene's bounds (focus still frames the active
  model), and the multi-model smoke pixel budget beyond "non-empty".

That slice is the smallest change that makes the next waves meaningful: once
two models render and one can be picked, selection, gizmo, panels, plates and
supports each have a real second case to migrate against instead of a boolean.

---

## 7. Open questions (for the implementation wave, not blockers)

1. **Id type.** Use the VOXL `id` string, or a monotonic `u64`? The viewport
   only needs hashing/ordering. The document needs stable ids for save; the
   in-memory list can keep a side map. Recommendation: `ModelId(u64)` in
   `gpui-core`, with `document_id: Option<String>` on `Model` for persistence
   and import dedupe.
2. **Scene revision.** With N models, walking the vector each frame to build the
   viewport's render list is cheap (tens of models, not thousands). Only add a
   revision counter if profiling shows it; the existing mesh-upload cache is the
   expensive part and it is keyed per model anyway.
3. **Support ownership.** The supports store is already keyed by `modelId`; the
   port's `SupportBatches` is not. Step 8 is the point to decide whether a batch
   carries one `model_id` or becomes a per-model map. The source's per-model
   proxy layers suggest the map.
4. **Plate rendering.** The viewport currently draws one rectangular plate and
   ignores the source's cascade/ghost plates. Plate data can exist before plate
   rendering; do not block the scene model on it.
5. **Session `rotation`.** Whether to write zeros or the first model's rotation
   when one model exists. Either is compatible with the current read; zeros are
   the clearer statement that it is legacy.
