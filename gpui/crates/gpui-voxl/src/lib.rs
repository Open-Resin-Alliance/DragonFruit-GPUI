//! VOXL scene-container reader.
//!
//! Ports the binary half of `src/features/scene/voxl/codec-v2.ts` — the
//! `parseVoxlBinaryV2` entry point — into a dependency-light Rust crate. The
//! React module also knows how to assemble a `VoxlDocument` (mesh references,
//! modifier snapshots, material profiles); this crate only splits the container
//! into its chunks and exposes the JSON payloads verbatim. The document-level
//! semantics stay in the caller.
//!
//! # Container layout
//!
//! ```text
//! [16-byte header] [chunk directory: 20 bytes * count] [chunk payloads]
//! ```
//!
//! Header (16 bytes, little-endian):
//!
//! | bytes | field |
//! |-------|-------|
//! | 0..4  | magic `"VOXL"` |
//! | 4..8  | version (`u32`; `2` or `3`) |
//! | 8..12 | chunk count (`u32`) |
//! | 12..16| reserved |
//!
//! The writer (`codec-v2.ts:10-30`) stores the version as a `u16` followed by a
//! `u16` flags word that is always `0`, so the `u32` read is byte-identical.
//!
//! Directory entry (20 bytes):
//!
//! | bytes | field |
//! |-------|-------|
//! | 0..4  | chunk type (4 ASCII bytes) |
//! | 4..8  | compression code (`u32`) |
//! | 8..12 | payload offset from the start of the file (`u32`) |
//! | 12..16| stored (compressed) length (`u32`) |
//! | 16..20| original (uncompressed) length (`u32`) |
//!
//! The compression slot is the source of the one real-world wrinkle: the
//! spec says `1` means zlib, while the writer stores a `u16` per-type index in
//! the low half and the `u16` code in the high half, so a zlib chunk with
//! index `0` reads as `65536`. The reader therefore inflates a code of `1` or
//! any code whose high half is `1`, treats a zero high half as raw (the low
//! half is then just the chunk ordinal), and reports any other compression as
//! an error rather than passing the bytes through.
//!
//! # Chunk types
//!
//! `META`, `SCNE`, `MODL`, `SUPP` and `EXTD` are JSON; `MESH` is opaque mesh
//! bytes. `HSRC`, `CAVT`, `PSRC`, `BODY` and `MATP` (the VOXL 3.1/3.4
//! snapshot chunks) are ignored here, and unknown types are ignored too — the
//! React reader tolerates additive chunks, so this one must as well.
//!
//! For `version == 3` the writer drops MESH chunks shared by duplicate
//! models, leaving the duplicates to point at the owner through
//! `meshRef.chunkIndex` inside `MODL` (`codec-v2.ts:74-78`). MESH chunks are
//! therefore collected in directory order, and that order is already the
//! deduplicated model order.

use std::io::Read;
use std::path::Path;

use serde_json::Value;

/// A parsed VOXL container.
///
/// The JSON values are handed back as written: `meta` is the `META` object,
/// `scene` the `SCNE` scene state, `models` the entries of the `MODL` array,
/// `supports` the `SUPP` support document and `extensions` the `EXTD` object.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    /// Container version: `2` (one MESH per model) or `3` (shared MESH
    /// chunks). The source calls this the "compat floor" (`codec-v2.ts:66-81`).
    pub version: u32,
    pub meta: Option<Value>,
    pub scene: Option<Value>,
    pub models: Vec<Value>,
    /// MESH payloads, decompressed, in directory order (model order).
    pub meshes: Vec<Vec<u8>>,
    pub supports: Option<Value>,
    pub extensions: Option<Value>,
}

/// Everything that can go wrong reading a VOXL container.
#[derive(Debug, thiserror::Error)]
pub enum VoxlError {
    #[error("not a VOXL container: bad magic")]
    NotVoxl,
    #[error("unsupported VOXL version {0} (expected 2 or 3)")]
    UnsupportedVersion(u32),
    #[error("truncated VOXL container")]
    Truncated,
    #[error("bad VOXL chunk: {0}")]
    BadChunk(&'static str),
    #[error("invalid JSON in a VOXL chunk: {0}")]
    Json(#[from] serde_json::Error),
    #[error("zlib inflate failed: {0}")]
    Inflate(String),
    /// Reading the file itself failed. Not part of the container format, but
    /// `read` needs somewhere to put a filesystem error.
    #[error("reading VOXL file failed: {0}")]
    Io(#[from] std::io::Error),
}

const MAGIC: [u8; 4] = *b"VOXL";
const HEADER_LEN: usize = 16;
const DIR_ENTRY_LEN: usize = 20;
/// The React reader rejects files with more than 10 000 chunks
/// (`codec-v2.ts:1233`); the same ceiling bounds the directory arithmetic and
/// catches corrupted counts.
const MAX_CHUNKS: u32 = 10_000;
const COMPRESSION_NONE: u32 = 0;

/// Reads and parses a `.voxl` scene file.
pub fn read(path: &Path) -> Result<Scene, VoxlError> {
    read_bytes(&std::fs::read(path)?)
}

/// Parses a VOXL container already held in memory.
pub fn read_bytes(bytes: &[u8]) -> Result<Scene, VoxlError> {
    if bytes.len() < MAGIC.len() {
        return Err(VoxlError::Truncated);
    }
    if !bytes.starts_with(&MAGIC) {
        return Err(VoxlError::NotVoxl);
    }
    if bytes.len() < HEADER_LEN {
        return Err(VoxlError::Truncated);
    }

    let version = u32::from_le_bytes(read_u32(bytes, 4));
    match version {
        2 | 3 => {}
        other => return Err(VoxlError::UnsupportedVersion(other)),
    }

    let chunk_count = u32::from_le_bytes(read_u32(bytes, 8));
    if chunk_count > MAX_CHUNKS {
        return Err(VoxlError::BadChunk("unreasonable chunk count"));
    }
    let chunk_count = chunk_count as usize;
    let dir_end = HEADER_LEN + chunk_count * DIR_ENTRY_LEN;
    if bytes.len() < dir_end {
        return Err(VoxlError::Truncated);
    }

    let mut entries = Vec::with_capacity(chunk_count);
    for i in 0..chunk_count {
        let base = HEADER_LEN + i * DIR_ENTRY_LEN;
        let kind = [
            bytes[base],
            bytes[base + 1],
            bytes[base + 2],
            bytes[base + 3],
        ];
        let compression = u32::from_le_bytes(read_u32(bytes, base + 4));
        let offset = u32::from_le_bytes(read_u32(bytes, base + 8)) as usize;
        let compressed_len = u32::from_le_bytes(read_u32(bytes, base + 12)) as usize;
        let original_len = u32::from_le_bytes(read_u32(bytes, base + 16)) as usize;

        let end = offset
            .checked_add(compressed_len)
            .ok_or(VoxlError::Truncated)?;
        if end > bytes.len() {
            return Err(VoxlError::Truncated);
        }

        entries.push(Entry {
            kind,
            compression,
            offset,
            compressed_len,
            original_len,
        });
    }

    let mut scene = Scene {
        version,
        meta: None,
        scene: None,
        models: Vec::new(),
        meshes: Vec::new(),
        supports: None,
        extensions: None,
    };

    for entry in &entries {
        let payload = entry.decode(bytes)?;
        match &entry.kind {
            b"META" => scene.meta = Some(serde_json::from_slice(&payload)?),
            b"SCNE" => scene.scene = Some(serde_json::from_slice(&payload)?),
            b"MODL" => {
                // `MODL` is a JSON array of model entries (`codec-v2.ts:1298`).
                // A single object is accepted as a one-model list so a hand-made
                // file still opens.
                match serde_json::from_slice::<Value>(&payload)? {
                    Value::Array(models) => scene.models.extend(models),
                    other => scene.models.push(other),
                }
            }
            b"SUPP" => scene.supports = Some(serde_json::from_slice(&payload)?),
            b"EXTD" => scene.extensions = Some(serde_json::from_slice(&payload)?),
            b"MESH" => scene.meshes.push(payload),
            b"HSRC" | b"CAVT" | b"PSRC" | b"BODY" | b"MATP" | b"ORIG" => {
                // These carry modifier snapshots, split bodies, material
                // profiles and the original-file bytes; all document-layer
                // state the container reader intentionally leaves alone.
                note_ignored(&entry.kind);
            }
            _ => note_ignored(&entry.kind),
        }
    }

    Ok(scene)
}

fn read_u32(bytes: &[u8], at: usize) -> [u8; 4] {
    [bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]
}

#[cfg(debug_assertions)]
fn note_ignored(kind: &[u8; 4]) {
    eprintln!(
        "gpui-voxl: ignoring '{}' chunk",
        String::from_utf8_lossy(kind)
    );
}

#[cfg(not(debug_assertions))]
fn note_ignored(_kind: &[u8; 4]) {}

struct Entry {
    kind: [u8; 4],
    /// The directory's compression word: `1` per the spec, `index | code << 16`
    /// from the real writer. A zero high half means raw bytes; a high half of
    /// `1` means zlib.
    compression: u32,
    offset: usize,
    compressed_len: usize,
    original_len: usize,
}

impl Entry {
    fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>, VoxlError> {
        let raw = &bytes[self.offset..self.offset + self.compressed_len];
        let decoded = match self.compression {
            COMPRESSION_NONE => raw.to_vec(),
            // The spec's zlib code.
            1 => inflate(raw, &self.kind)?,
            // The writer stores `index | compression << 16`, so a zlib payload
            // reads as 65536, 65537, … and the low half is the chunk's ordinal.
            code if code >> 16 == 1 => inflate(raw, &self.kind)?,
            // High half zero with a non-zero low half is the same writer layout
            // for a chunk it stored raw because zlib did not shrink it
            // (`codec-v2.ts:885-888`); the value is just the chunk index.
            code if code >> 16 == 0 => raw.to_vec(),
            code => {
                return Err(VoxlError::Inflate(format!(
                    "{} chunk: unsupported compression code {code}",
                    String::from_utf8_lossy(&self.kind)
                )));
            }
        };

        if self.original_len != 0 && decoded.len() != self.original_len {
            return Err(VoxlError::BadChunk("chunk size mismatch"));
        }
        Ok(decoded)
    }
}

fn inflate(raw: &[u8], kind: &[u8; 4]) -> Result<Vec<u8>, VoxlError> {
    let mut decoded = Vec::new();
    flate2::read::ZlibDecoder::new(raw)
        .read_to_end(&mut decoded)
        .map_err(|err| {
            VoxlError::Inflate(format!("{} chunk: {err}", String::from_utf8_lossy(kind)))
        })?;
    Ok(decoded)
}

#[cfg(test)]
mod tests;
