//! VOXL scene import.
//!
//! `gpui-voxl` splits the container into chunks and hands back the JSON
//! documents plus the decompressed `MESH` payloads. This module turns every
//! model the `MODL` array names into a model record the app can draw:
//!
//! * the `MESH` payload is a whole STL file (`codec-v2.ts`: `mimeType ??
//!   'model/stl'`, "MESH chunk is a whole STL payload"), so it goes through
//!   the same loader as a dropped `.stl`;
//! * each model keeps its own `MODL` name, transform and visibility;
//! * a V3 model whose `mesh.mode` is `embedded-chunk` points at its owner's
//!   payload through `mesh.chunkIndex`; the payload is parsed once and the
//!   `Arc<MeshData>` is shared, so the viewport uploads it once.
//!
//! The container itself is still handed back verbatim for the support panel:
//! `SUPP` carries the support document. Chunks this build does not consume
//! (`HSRC`, `CAVT`, `PSRC`, `BODY`, `MATP`) stay in the container untouched.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, bail};
use gpui_core::stl::{self, MeshData};
use gpui_voxl::Scene;

mod placement;
pub use placement::placement;

/// One model of a VOXL scene, ready for the app's `Scene`.
pub struct ImportedModel {
    /// `MODL[i].name`, falling back to the file name.
    pub name: String,
    /// The model's geometry. Duplicates of a V3 chunk share one `Arc`.
    pub mesh: Arc<MeshData>,
    /// The model's world translate: `MODL[i].transform.position`, `[x, y, z]`
    /// in millimetres, plus the recentring the `MESH` payload needs (zero for
    /// the bbox-centred payload the exporter writes).
    pub translate: [f32; 3],
    /// `MODL[i].transform.rotation`, `[x, y, z]` Euler radians about the fixed
    /// world axes — `ModelTransform::rotation` (`Euler('ZYX')` in the scene).
    pub rotation: [f32; 3],
    /// `MODL[i].transform.scale`, `[x, y, z]`.
    pub scale: [f32; 3],
    /// `MODL[i].visible`; a document without the field shows the model.
    pub visible: bool,
}

/// A VOXL scene as the app's model list.
pub struct Imported {
    /// The container as parsed, for the support panel (`SUPP`, `MODL`).
    pub scene: Scene,
    /// Every model whose mesh could be resolved, in `MODL` order.
    pub models: Vec<ImportedModel>,
}

/// Read a VOXL container and resolve every model it names. Never panics on
/// malformed input: the container reader bounds-checks the directory and this
/// module only uses fallible JSON access. A model that cannot be resolved is
/// skipped; a file where nothing resolves is an error.
pub fn load(path: &Path) -> Result<Imported> {
    let scene = gpui_voxl::read(path).with_context(|| format!("reading {}", path.display()))?;
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());

    if scene.models.is_empty() {
        bail!("{file_name} contains no models");
    }

    // A V3 duplicate points at its owner's chunk through `mesh.chunkIndex`;
    // the payloads are exposed in directory order, which is the deduplicated
    // owner order. Parse each chunk once so duplicates share the `Arc` and the
    // viewport uploads the geometry once.
    let mut chunks: BTreeMap<u64, Arc<MeshData>> = BTreeMap::new();
    let mut next_owner_chunk = 0u64;
    let mut models = Vec::new();
    // A model whose payload is broken is skipped like an unresolvable one, but
    // its error is kept: a file where nothing resolves should say why.
    let mut first_error = None;
    for model in scene.models.iter() {
        let mesh = match resolve_model(
            model,
            &scene,
            &mut chunks,
            &mut next_owner_chunk,
            &file_name,
        ) {
            Ok(Some(mesh)) => mesh,
            Ok(None) => continue,
            Err(error) => {
                first_error.get_or_insert(error);
                continue;
            }
        };
        let name = model
            .get("name")
            .and_then(|name| name.as_str())
            .filter(|name| !name.trim().is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| file_name.clone());

        let transform = model.get("transform");
        let axis = |group: &str, key: &str| {
            transform
                .and_then(|transform| transform.get(group))
                .and_then(|group| group.get(key))
                .and_then(|value| value.as_f64())
                .unwrap_or(0.0) as f32
        };
        let stored = [
            finite_or(axis("position", "x"), 0.0),
            finite_or(axis("position", "y"), 0.0),
            finite_or(axis("position", "z"), 0.0),
        ];
        // A `MESH` chunk is written bbox-centred: `ExportManager
        // ::exportModelAsEmbeddedBinaryStlBytes` moves the mesh by
        // `-geometry.center` before exporting it (`ExportManager.ts`, around
        // line 220), so `MODL.transform.position` is expressed against a mesh
        // whose bbox centre is the local origin — the frame `useStlGeometry.ts`
        // centres an imported geometry into as well. `placement` targets the
        // *seated* frame (bbox bottom at `z = 0`), which differs by half the
        // height, so that lift is taken back off before the stored position
        // is added; otherwise a model the source already seated on the plate
        // would rise by half its height a second time on every load. For a
        // centred payload the recentring is zero and the stored position
        // applies unchanged.
        let seated = placement(&mesh, [0.0, 0.0]);
        let recentre = [seated[0], seated[1], seated[2] - mesh.size[2] * 0.5];
        let translate = [
            stored[0] + recentre[0],
            stored[1] + recentre[1],
            stored[2] + recentre[2],
        ];
        let rotation = [
            finite_or(axis("rotation", "x"), 0.0),
            finite_or(axis("rotation", "y"), 0.0),
            finite_or(axis("rotation", "z"), 0.0),
        ];
        // Real scenes author the scale uniformly, but the document carries
        // three components and the scene keeps all three.
        let scale = [
            uniform_scale(axis("scale", "x")),
            uniform_scale(axis("scale", "y")),
            uniform_scale(axis("scale", "z")),
        ];
        let visible = model
            .get("visible")
            .and_then(|visible| visible.as_bool())
            .unwrap_or(true);

        models.push(ImportedModel {
            name,
            mesh,
            translate,
            rotation,
            scale,
            visible,
        });
    }

    if models.is_empty() {
        if let Some(error) = first_error {
            return Err(error);
        }
        bail!("{file_name} contains no model with a resolvable MESH chunk");
    }

    Ok(Imported { scene, models })
}

/// Resolve one `MODL` entry's geometry, parsing the payload the first time a
/// chunk is used. `Ok(None)` means the entry names no chunk this build can
/// read; `Err` carries a payload that exists but failed to parse.
fn resolve_model(
    model: &serde_json::Value,
    scene: &Scene,
    chunks: &mut BTreeMap<u64, Arc<MeshData>>,
    next_owner_chunk: &mut u64,
    file_name: &str,
) -> Result<Option<Arc<MeshData>>> {
    let mesh_ref = model.get("mesh");
    let mesh_mode = mesh_ref
        .and_then(|mesh| mesh.get("mode"))
        .and_then(|mode| mode.as_str());
    match mesh_mode {
        // V2 files and V3 owners write `embedded-chunk`; a missing field is
        // treated the same way because older writers always had a chunk per
        // model.
        Some("embedded-chunk") | None => {}
        Some(_) => return Ok(None),
    }

    // A V2 model owns the next chunk in model order; a V3 duplicate names its
    // owner's chunk explicitly. The owner ordinal advances on every owning
    // entry (even one that fails to parse), which is what the payload array's
    // deduplicated order is.
    let chunk_index = match mesh_ref
        .and_then(|mesh| mesh.get("chunkIndex"))
        .and_then(|index| index.as_u64())
    {
        Some(index) => index,
        None => {
            let index = *next_owner_chunk;
            *next_owner_chunk += 1;
            index
        }
    };
    if let Some(mesh) = chunks.get(&chunk_index) {
        return Ok(Some(mesh.clone()));
    }

    let Some(payload) = scene.meshes.get(chunk_index as usize) else {
        return Ok(None);
    };
    let name = model
        .get("name")
        .and_then(|name| name.as_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(file_name);
    let mesh = Arc::new(parse_stl_payload(payload, name)?);
    chunks.insert(chunk_index, mesh.clone());
    Ok(Some(mesh))
}

/// Parse a `MESH` payload through the STL loader.
///
/// `stl::load_stl` takes a path, and `stl.rs` is outside this module's scope
/// this wave, so the payload is staged in a uniquely named temp file, loaded,
/// and removed. The file name is replaced with the model's name afterwards.
fn parse_stl_payload(payload: &[u8], name: &str) -> Result<MeshData> {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "dragonfruit-voxl-{}-{}.stl",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, payload)
        .with_context(|| format!("staging MESH payload at {}", path.display()))?;
    let loaded = stl::load_stl(&path);
    let _ = std::fs::remove_file(&path);

    let mut mesh = loaded.with_context(|| format!("parsing {name}'s MESH payload"))?;
    mesh.name = name.to_owned();
    Ok(mesh)
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

fn uniform_scale(value: f32) -> f32 {
    if value.is_finite() && value != 0.0 {
        value
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-triangle binary STL, enough for the loader to return real
    /// geometry with a bounding box.
    fn binary_stl(size: f32) -> Vec<u8> {
        let mut bytes = vec![0u8; 80];
        bytes.extend_from_slice(&1u32.to_le_bytes());
        // The facet normal, three vertices, then the u16 attribute word.
        for value in [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, size, 0.0, 0.0, 0.0, size, 0.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes
    }

    /// Assembles a raw (uncompressed) container the way `codec-v2.ts` lays one
    /// out: header, one 20-byte directory entry per chunk, then the payloads.
    fn container(chunks: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
        let mut directory = Vec::with_capacity(chunks.len() * 20);
        let mut bodies = Vec::new();
        let mut offset = 16 + chunks.len() * 20;
        for (kind, payload) in chunks {
            directory.extend_from_slice(kind);
            directory.extend_from_slice(&0u32.to_le_bytes());
            directory.extend_from_slice(&(offset as u32).to_le_bytes());
            directory.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            directory.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            offset += payload.len();
            bodies.extend_from_slice(payload);
        }
        let mut bytes = Vec::with_capacity(offset);
        bytes.extend_from_slice(b"VOXL");
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&(chunks.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&directory);
        bytes.extend_from_slice(&bodies);
        bytes
    }

    /// The slice's core loader promise: every `MODL` entry becomes a model,
    /// with its own name, transform and visibility.
    #[test]
    fn loads_every_model_in_a_voxl() {
        let modl = br#"[
            {
                "id": "a",
                "name": "first",
                "mesh": {"mode": "embedded-chunk", "chunkIndex": 0},
                "transform": {
                    "position": {"x": 0, "y": 0, "z": 5},
                    "rotation": {"x": 0, "y": 0, "z": 0.25},
                    "scale": {"x": 1, "y": 1, "z": 1}
                }
            },
            {
                "id": "b",
                "name": "second",
                "visible": false,
                "mesh": {"mode": "embedded-chunk", "chunkIndex": 1},
                "transform": {"position": {"x": 40, "y": 0, "z": 5}}
            }
        ]"#;
        let bytes = container(&[
            (*b"MODL", modl.to_vec()),
            (*b"MESH", binary_stl(10.0)),
            (*b"MESH", binary_stl(10.0)),
        ]);
        let path = std::env::temp_dir().join(format!(
            "dragonfruit-multi-{}-{}.voxl",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::write(&path, &bytes).expect("write fixture");
        let imported = load(&path).expect("two-model container parses");
        let _ = std::fs::remove_file(&path);

        assert_eq!(imported.models.len(), 2);
        assert_eq!(imported.models[0].name, "first");
        assert_eq!(imported.models[1].name, "second");
        assert!(!imported.models[1].visible);
        // Model 2 keeps its own 40 mm offset from model 1.
        let offset = imported.models[1].translate[0] - imported.models[0].translate[0];
        assert!((offset - 40.0).abs() < 1e-3, "translate offset {offset}");
        assert!((imported.models[0].rotation[2] - 0.25).abs() < 1e-6);
        assert!(!Arc::ptr_eq(
            &imported.models[0].mesh,
            &imported.models[1].mesh
        ));
    }
}
