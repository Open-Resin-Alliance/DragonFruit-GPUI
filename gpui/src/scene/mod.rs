//! VOXL scene import.
//!
//! `gpui-voxl` splits the container into chunks and hands back the JSON
//! documents plus the decompressed `MESH` payloads. This module turns the
//! first model into the port's single-model view:
//!
//! * the `MESH` payload is a whole STL file (`codec-v2.ts`: `mimeType ??
//!   'model/stl'`, "MESH chunk is a whole STL payload"), so it goes through
//!   the same loader as a dropped `.stl`;
//! * `MODL[0].transform` becomes the viewport's translate / rotate_z / scale.
//!
//! The rest of the `Scene` is kept verbatim on the entity for the support
//! panel: `SUPP` carries the support document, and the remaining models are
//! not rendered by this build's single-mesh viewport.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, bail};
use gpui_core::stl::{self, MeshData};
use gpui_voxl::Scene;

/// A VOXL scene whose first model is ready for the single-model viewport.
pub struct Imported {
    /// The container as parsed, for the support panel (`SUPP`, `MODL`).
    pub scene: Scene,
    pub mesh: MeshData,
    /// `MODL[0].transform.position`, `[x, y, z]` in millimetres.
    pub translate: [f32; 3],
    /// `MODL[0].transform.rotation.z` — the viewport only carries a Z angle.
    pub rotate_z: f32,
    /// `MODL[0].transform.scale.x` — the viewport carries one uniform scale.
    pub scale: f32,
}

/// Read a VOXL container and collapse its first model into the port's mesh
/// view. Never panics on malformed input: the container reader bounds-checks
/// the directory and this module only uses fallible JSON access.
pub fn load(path: &Path) -> Result<Imported> {
    let scene = gpui_voxl::read(path).with_context(|| format!("reading {}", path.display()))?;
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());

    let model = scene
        .models
        .first()
        .with_context(|| format!("{file_name} contains no models"))?;

    let mesh_ref = model.get("mesh");
    let mesh_mode = mesh_ref
        .and_then(|mesh| mesh.get("mode"))
        .and_then(|mode| mode.as_str());
    match mesh_mode {
        // V2 files and V3 owners write `embedded-chunk`; a missing field is
        // treated the same way because older writers always had a chunk per
        // model.
        Some("embedded-chunk") | None => {}
        Some(other) => bail!("first model's mesh is '{other}', not an embedded chunk"),
    }

    // A V3 duplicate points at its owner's chunk through `mesh.chunkIndex`.
    // `gpui-voxl` exposes the decompressed payloads in directory order, which
    // is the deduplicated owner order; the first model owning index 0 is the
    // first payload. Anything else cannot be located without the chunk index
    // from the directory, so refuse rather than show the wrong mesh.
    if let Some(chunk_index) = mesh_ref
        .and_then(|mesh| mesh.get("chunkIndex"))
        .and_then(|index| index.as_u64())
    {
        if chunk_index != 0 {
            bail!("first model reuses mesh chunk {chunk_index}, which this view cannot resolve");
        }
    }

    let payload = scene
        .meshes
        .first()
        .with_context(|| format!("{file_name} has no MESH chunk"))?;

    let name = model
        .get("name")
        .and_then(|name| name.as_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(&file_name);
    let mesh = parse_stl_payload(payload, name)?;

    let transform = model.get("transform");
    let axis = |group: &str, key: &str| {
        transform
            .and_then(|transform| transform.get(group))
            .and_then(|group| group.get(key))
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0) as f32
    };
    let translate = [
        finite_or(axis("position", "x"), 0.0),
        finite_or(axis("position", "y"), 0.0),
        finite_or(axis("position", "z"), 0.0),
    ];
    let rotate_z = finite_or(axis("rotation", "z"), 0.0);
    // The viewport has a single uniform scale; the source's per-axis scale is
    // [x, y, z] and real scenes author it uniformly.
    let scale = uniform_scale(axis("scale", "x"));

    Ok(Imported {
        scene,
        mesh,
        translate,
        rotate_z,
        scale,
    })
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
