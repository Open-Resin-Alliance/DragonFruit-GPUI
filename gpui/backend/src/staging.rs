//! The mesh staging buffer — `stage_mesh_*`, `append_mesh_stage_chunk`,
//! `finish_mesh_stage_write`, `allocate_mesh_stage_path`.
//!
//! Ported from `src-tauri/src/main.rs` lines 470-560 and 1200-1580. The shape is
//! unchanged: a frontend uploads a triangle soup either fully in memory or in
//! chunks to a file, and every later operation (slicing, repair, island scan,
//! export) consumes it. What changed is ownership: `src-tauri` kept four
//! process-wide `OnceLock<Mutex<…>>`s, and here they are four fields of
//! [`MeshStaging`] so a test — or a second window — can have its own buffer.
//!
//! The staging format is a raw little-endian `f32` soup, 9 floats (3 vertices)
//! per triangle. That is the format every consumer in `repair`, `slicing` and
//! `export` reads.

use std::io::Write;
use std::sync::Mutex;

use dragonfruit_mesh_repair::IndexedMesh;
use serde::Serialize;

use crate::BackendResult;

/// Env var `src-tauri` honoured to relocate the staging directory, for machines
/// whose `%TEMP%` is small or on a slow volume.
const MESH_STAGE_DIR_ENV: &str = "DF_MESH_STAGE_DIR";
const STAGED_MESH_PREALLOC_MIN_BYTES: usize = 16 * 1024 * 1024;
const STAGED_MESH_PREALLOC_MAX_BYTES: usize = 1024 * 1024 * 1024;

/// An open, buffered writer for a chunked upload. Released as soon as the write
/// finishes so Windows lets the next reader open the file.
pub struct StageFileAppender {
    pub path: String,
    pub writer: std::io::BufWriter<std::fs::File>,
    pub len: u64,
}

/// Counters behind the staging acknowledgements, kept so the UI can report
/// upload throughput (they are what `StageMeshChunkAck` reports).
#[derive(Default, Clone, PartialEq)]
pub struct StageMeshStats {
    pub chunks_received: u64,
    pub append_ns_total: u64,
}

/// Reply to one chunk of a binary upload; field names are the frontend's
/// (`camelCase`) because the reply crossed the IPC as JSON.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StageMeshChunkAck {
    pub chunk_bytes: u64,
    pub total_bytes: u64,
    pub capacity_bytes: u64,
    pub reserve_grew: bool,
    pub chunks_received: u64,
    pub append_ns: u64,
    pub append_ns_total: u64,
}

/// The staging buffer, its counters, and the optional file a chunked upload is
/// being written to.
#[derive(Default)]
pub struct MeshStaging {
    mesh: Mutex<Option<Vec<u8>>>,
    stats: Mutex<StageMeshStats>,
    file_path: Mutex<Option<String>>,
    appender: Mutex<Option<StageFileAppender>>,
}

impl MeshStaging {
    pub fn new() -> Self {
        Self::default()
    }

    /// `stage_mesh_binary_start`: reset every buffer and pre-allocate for
    /// `total_bytes` (clamped to 16 MiB..1 GiB, as the shell did).
    pub fn stage_mesh_binary_start(&self, total_bytes: usize) -> BackendResult<()> {
        let reserve_bytes = normalize_staged_mesh_prealloc_bytes(total_bytes);
        *self
            .mesh
            .lock()
            .map_err(|e| format!("staged mesh lock poisoned: {e}"))? =
            Some(Vec::with_capacity(reserve_bytes));
        *self
            .stats
            .lock()
            .map_err(|e| format!("staged mesh stats lock poisoned: {e}"))? =
            StageMeshStats::default();
        *self
            .file_path
            .lock()
            .map_err(|e| format!("staged mesh file-path lock poisoned: {e}"))? = None;
        *self
            .appender
            .lock()
            .map_err(|e| format!("staged mesh file appender lock poisoned: {e}"))? = None;
        Ok(())
    }

    /// `stage_mesh_binary_set`: replace the buffer with one complete payload.
    pub fn stage_mesh_binary_set(&self, bytes: &[u8]) -> BackendResult<StageMeshChunkAck> {
        let reserve_bytes = normalize_staged_mesh_prealloc_bytes(bytes.len());
        let append_start = std::time::Instant::now();
        let mut staged = Vec::with_capacity(reserve_bytes);
        staged.extend_from_slice(bytes);
        let append_ns = append_start.elapsed().as_nanos().min(u64::MAX as u128) as u64;
        let capacity_bytes = staged.capacity() as u64;
        let total_bytes = staged.len() as u64;

        *self
            .mesh
            .lock()
            .map_err(|e| format!("staged mesh lock poisoned: {e}"))? = Some(staged);
        *self
            .stats
            .lock()
            .map_err(|e| format!("staged mesh stats lock poisoned: {e}"))? = StageMeshStats {
            chunks_received: 1,
            append_ns_total: append_ns,
        };
        *self
            .file_path
            .lock()
            .map_err(|e| format!("staged mesh file-path lock poisoned: {e}"))? = None;
        *self
            .appender
            .lock()
            .map_err(|e| format!("staged mesh file appender lock poisoned: {e}"))? = None;

        Ok(StageMeshChunkAck {
            chunk_bytes: total_bytes,
            total_bytes,
            capacity_bytes,
            reserve_grew: capacity_bytes > reserve_bytes as u64,
            chunks_received: 1,
            append_ns,
            append_ns_total: append_ns,
        })
    }

    /// `stage_mesh_binary_chunk`: append one chunk to the in-memory buffer.
    pub fn stage_mesh_binary_chunk(&self, bytes: &[u8]) -> BackendResult<StageMeshChunkAck> {
        let mut lock = self
            .mesh
            .lock()
            .map_err(|e| format!("staged mesh lock poisoned: {e}"))?;
        let vec = lock
            .as_mut()
            .ok_or("Staged mesh not started. Call stage_mesh_binary_start first")?;

        let chunk_bytes = bytes.len() as u64;
        let capacity_before = vec.capacity();
        let append_start = std::time::Instant::now();
        vec.extend_from_slice(bytes);
        let append_ns = append_start.elapsed().as_nanos().min(u64::MAX as u128) as u64;
        let capacity_after = vec.capacity();
        let total_bytes = vec.len() as u64;
        drop(lock);

        let mut stats = self
            .stats
            .lock()
            .map_err(|e| format!("staged mesh stats lock poisoned: {e}"))?;
        stats.chunks_received = stats.chunks_received.saturating_add(1);
        stats.append_ns_total = stats.append_ns_total.saturating_add(append_ns);

        Ok(StageMeshChunkAck {
            chunk_bytes,
            total_bytes,
            capacity_bytes: capacity_after as u64,
            reserve_grew: capacity_after > capacity_before,
            chunks_received: stats.chunks_received,
            append_ns,
            append_ns_total: stats.append_ns_total,
        })
    }

    /// `stage_mesh_file_path`: point the buffer at a file already on disk and
    /// drop anything in memory. Returns the file's length.
    pub fn stage_mesh_file_path(&self, mesh_file_path: &str) -> BackendResult<u64> {
        let trimmed = mesh_file_path.trim();
        if trimmed.is_empty() {
            return Err("stage_mesh_file_path requires a non-empty path".into());
        }
        let path = std::path::PathBuf::from(trimmed);
        if !path.exists() {
            return Err(format!(
                "Mesh stage file does not exist: {}",
                path.display()
            ));
        }

        self.flush_appender_for(trimmed)?;

        let len = std::fs::metadata(&path)
            .map_err(|e| format!("Failed reading mesh stage file metadata: {e}"))?
            .len();

        *self
            .mesh
            .lock()
            .map_err(|e| format!("staged mesh lock poisoned: {e}"))? = None;
        *self
            .file_path
            .lock()
            .map_err(|e| format!("staged mesh file-path lock poisoned: {e}"))? =
            Some(path.to_string_lossy().to_string());
        *self
            .stats
            .lock()
            .map_err(|e| format!("staged mesh stats lock poisoned: {e}"))? = StageMeshStats {
            chunks_received: 1,
            append_ns_total: 0,
        };

        Ok(len)
    }

    /// `append_mesh_stage_chunk`.
    ///
    /// The Tauri command took the bytes as a raw IPC body and the destination
    /// path from a percent-encoded `x-mesh-stage-path` header (HTTP header values
    /// are visible-ASCII, so a path with an umlaut could not ride verbatim).
    /// There is no header here: the caller passes the decoded path.
    /// `is_first_chunk` is what the shell derived from `x-mesh-stage-offset == 0`.
    pub fn append_mesh_stage_chunk(
        &self,
        path_text: &str,
        bytes: &[u8],
        is_first_chunk: bool,
    ) -> BackendResult<u64> {
        let path = std::path::PathBuf::from(path_text);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("Failed creating mesh stage directory: {err}"))?;
        }

        let mut appender_lock = self
            .appender
            .lock()
            .map_err(|e| format!("staged mesh file appender lock poisoned: {e}"))?;

        // Contention backstop (from the shell): one appender, so a mid-sequence
        // chunk arriving for a *different* path means two chunked writes are
        // interleaving. Warning loudly beats two silently corrupt files.
        if let Some(existing) = appender_lock.as_ref() {
            if !is_first_chunk && existing.path != path_text {
                return Err(format!(
                    "concurrent mesh-stage write detected: '{}' is mid-write while a chunk arrived for '{}'",
                    existing.path, path_text
                ));
            }
        }

        let needs_new_appender = match appender_lock.as_ref() {
            Some(existing) => is_first_chunk || existing.path != path_text,
            None => true,
        };

        if needs_new_appender {
            if let Some(existing) = appender_lock.as_mut() {
                let _ = existing.writer.flush();
            }
            let file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&path)
                .map_err(|err| format!("Failed opening mesh stage file for write: {err}"))?;
            *appender_lock = Some(StageFileAppender {
                path: path_text.to_string(),
                writer: std::io::BufWriter::with_capacity(4 * 1024 * 1024, file),
                len: 0,
            });
        }

        let appender = appender_lock
            .as_mut()
            .ok_or("Failed initializing mesh stage file appender")?;
        appender
            .writer
            .write_all(bytes)
            .map_err(|err| format!("Failed appending mesh stage bytes: {err}"))?;
        appender
            .writer
            .flush()
            .map_err(|err| format!("Failed flushing mesh stage bytes: {err}"))?;
        appender.len = appender.len.saturating_add(bytes.len() as u64);

        Ok(appender.len)
    }

    /// `finish_mesh_stage_write`: flush and release the appender for `path`,
    /// returning how many bytes this process wrote. `0` when no appender is open
    /// for that path (already closed, or never opened).
    pub fn finish_mesh_stage_write(&self, path: &str) -> BackendResult<u64> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return Err("finish_mesh_stage_write requires a non-empty path".into());
        }

        let mut appender_lock = self
            .appender
            .lock()
            .map_err(|e| format!("staged mesh file appender lock poisoned: {e}"))?;
        if let Some(appender) = appender_lock.as_mut() {
            if appender.path == trimmed {
                appender
                    .writer
                    .flush()
                    .map_err(|e| format!("Failed flushing mesh stage file writer: {e}"))?;
                let len = appender.len;
                *appender_lock = None;
                return Ok(len);
            }
        }
        Ok(0)
    }

    /// Flush and release the appender if it is writing to `path`.
    fn flush_appender_for(&self, path: &str) -> BackendResult<()> {
        let mut lock = self
            .appender
            .lock()
            .map_err(|e| format!("staged mesh file appender lock poisoned: {e}"))?;
        let matches = lock.as_ref().map_or(false, |a| a.path == path);
        if matches {
            if let Some(appender) = lock.as_mut() {
                appender
                    .writer
                    .flush()
                    .map_err(|e| format!("Failed flushing staging appender: {e}"))?;
            }
            *lock = None;
        }
        Ok(())
    }

    /// Flush and release the appender if it is writing to `path`, without
    /// touching a poisoned lock elsewhere. Used by `export_mesh_file`, which must
    /// see every buffered byte before it reads the file back.
    pub fn release_appender_for(&self, path: &str) -> BackendResult<()> {
        self.flush_appender_for(path.trim())
    }

    /// The current buffer's bytes: the in-memory soup when there is one,
    /// otherwise the staged file's contents.
    ///
    /// This is `src-tauri`'s `read_staging_bytes`, shared by every repair
    /// command.
    pub fn read_staging_bytes(&self) -> BackendResult<Vec<u8>> {
        if let Some(bytes) = self
            .mesh
            .lock()
            .map_err(|e| format!("staged mesh lock poisoned: {e}"))?
            .clone()
        {
            return Ok(bytes);
        }

        {
            let mut appender_lock = self
                .appender
                .lock()
                .map_err(|e| format!("staged mesh file appender lock poisoned: {e}"))?;
            if let Some(appender) = appender_lock.as_mut() {
                appender
                    .writer
                    .flush()
                    .map_err(|e| format!("flush staged mesh appender: {e}"))?;
            }
        }

        let path = self
            .file_path
            .lock()
            .map_err(|e| format!("staged mesh file-path lock poisoned: {e}"))?
            .clone();
        match path {
            Some(p) => std::fs::read(&p).map_err(|e| format!("read staged mesh file '{p}': {e}")),
            None => Err(
                "No staged mesh buffer — call stage_mesh_* or mesh_repair_from_path first".into(),
            ),
        }
    }

    /// Replace the staging buffer with `mesh`, in memory.
    ///
    /// This is `src-tauri`'s `replace_staging_with_mesh`: every repair pass hands
    /// its result back here so the next command — and the viewport — sees it.
    pub fn replace_with_mesh(&self, mesh: &IndexedMesh) -> BackendResult<()> {
        let soup = mesh.to_triangle_soup();
        let bytes: Vec<u8> = bytemuck::cast_slice::<f32, u8>(&soup).to_vec();

        *self
            .appender
            .lock()
            .map_err(|e| format!("staged mesh file appender lock poisoned: {e}"))? = None;
        *self
            .file_path
            .lock()
            .map_err(|e| format!("staged mesh file-path lock poisoned: {e}"))? = None;
        *self
            .stats
            .lock()
            .map_err(|e| format!("staged mesh stats lock poisoned: {e}"))? = StageMeshStats {
            chunks_received: 1,
            append_ns_total: 0,
        };
        *self
            .mesh
            .lock()
            .map_err(|e| format!("staged mesh lock poisoned: {e}"))? = Some(bytes);
        Ok(())
    }

    /// Take the staged soup for slicing, releasing it.
    ///
    /// This is the first half of `slice_solid_native_to_temp_path`: an in-memory
    /// buffer is taken as-is; otherwise the staged file is flushed, read and
    /// **deleted** (a slice consumes its source, and the file can be gigabytes).
    pub fn take_staged_bytes_for_slicing(&self) -> BackendResult<Vec<u8>> {
        if let Some(bytes) = self
            .mesh
            .lock()
            .map_err(|e| format!("staged mesh lock poisoned: {e}"))?
            .take()
        {
            return Ok(bytes);
        }

        {
            let mut appender_lock = self
                .appender
                .lock()
                .map_err(|e| format!("staged mesh file appender lock poisoned: {e}"))?;
            if let Some(appender) = appender_lock.as_mut() {
                appender
                    .writer
                    .flush()
                    .map_err(|e| format!("Failed flushing mesh stage file writer: {e}"))?;
            }
        }

        let staged_path = self
            .file_path
            .lock()
            .map_err(|e| format!("staged mesh file-path lock poisoned: {e}"))?
            .take()
            .ok_or(
                "No staged mesh binary or file path — call stage_mesh_binary_* or stage_mesh_file_path first",
            )?;

        let path = std::path::PathBuf::from(staged_path.trim());
        if !path.exists() {
            return Err(format!(
                "Staged mesh file no longer exists: {}",
                path.display()
            ));
        }
        let bytes = std::fs::read(&path)
            .map_err(|e| format!("Failed reading staged mesh file '{}': {e}", path.display()))?;
        let _ = std::fs::remove_file(&path);
        *self
            .appender
            .lock()
            .map_err(|e| format!("staged mesh file appender lock poisoned: {e}"))? = None;
        Ok(bytes)
    }

    /// Take the in-memory buffer only, without touching the file fallback.
    ///
    /// Used by `run_island_scan`, whose shell command required an in-memory
    /// upload (`stage_mesh_binary_*`) and errored otherwise.
    pub fn take_memory_buffer(&self) -> BackendResult<Option<Vec<u8>>> {
        Ok(self
            .mesh
            .lock()
            .map_err(|e| format!("staged mesh lock poisoned: {e}"))?
            .take())
    }

    pub fn stats(&self) -> BackendResult<StageMeshStats> {
        Ok(self
            .stats
            .lock()
            .map_err(|e| format!("staged mesh stats lock poisoned: {e}"))?
            .clone())
    }
}

/// `allocate_mesh_stage_path`: a fresh staging file in the staging directory.
pub fn allocate_mesh_stage_path() -> BackendResult<String> {
    let mut dir = resolve_mesh_stage_directory();
    std::fs::create_dir_all(&dir).map_err(|e| {
        format!(
            "Failed creating mesh stage directory '{}': {e}",
            dir.display()
        )
    })?;

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    dir.push(format!("dragonfruit-mesh-stage-{stamp}.bin"));
    Ok(dir.to_string_lossy().to_string())
}

/// `DF_MESH_STAGE_DIR` when set, else the system temp directory.
pub fn resolve_mesh_stage_directory() -> std::path::PathBuf {
    std::env::var(MESH_STAGE_DIR_ENV)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

fn normalize_staged_mesh_prealloc_bytes(total_bytes_hint: usize) -> usize {
    total_bytes_hint
        .max(STAGED_MESH_PREALLOC_MIN_BYTES)
        .min(STAGED_MESH_PREALLOC_MAX_BYTES)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn soup() -> Vec<u8> {
        let mut v = Vec::new();
        for value in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            v.extend_from_slice(&value.to_le_bytes());
        }
        v
    }

    #[test]
    fn in_memory_round_trip() {
        let staging = MeshStaging::new();
        staging.stage_mesh_binary_start(0).expect("start");
        let ack = staging.stage_mesh_binary_chunk(&soup()).expect("chunk");
        assert_eq!(ack.chunks_received, 1);
        assert_eq!(ack.total_bytes, 36);
        // The pre-allocation floor is 16 MiB, so the buffer never re-grew.
        assert!(!ack.reserve_grew);
        assert_eq!(staging.read_staging_bytes().expect("read"), soup());
    }

    #[test]
    fn single_shot_set_replaces_the_buffer() {
        let staging = MeshStaging::new();
        staging.stage_mesh_binary_start(0).expect("start");
        staging.stage_mesh_binary_chunk(&soup()).expect("chunk");
        let ack = staging.stage_mesh_binary_set(&soup()).expect("set");
        assert_eq!(ack.chunks_received, 1);
        assert_eq!(staging.read_staging_bytes().expect("read"), soup());
    }

    #[test]
    fn chunk_before_start_is_an_error() {
        let staging = MeshStaging::new();
        let err = staging
            .stage_mesh_binary_chunk(&soup())
            .expect_err("must refuse");
        assert!(err.contains("stage_mesh_binary_start"), "{err}");
    }

    #[test]
    fn file_staging_appends_and_finishes() {
        let dir = std::env::temp_dir().join(format!(
            "df-staging-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("stage.bin");
        let path_text = path.to_string_lossy().to_string();

        let staging = MeshStaging::new();
        let first = soup();
        let second = soup();
        staging
            .append_mesh_stage_chunk(&path_text, &first, true)
            .expect("first chunk");
        let len = staging
            .append_mesh_stage_chunk(&path_text, &second, false)
            .expect("second chunk");
        assert_eq!(len, 72);
        assert_eq!(staging.finish_mesh_stage_write(&path_text).expect("finish"), 72);
        assert_eq!(staging.finish_mesh_stage_write(&path_text).expect("again"), 0);

        assert_eq!(std::fs::read(&path).expect("file").len(), 72);
        staging.stage_mesh_file_path(&path_text).expect("adopt");
        assert_eq!(staging.read_staging_bytes().expect("read").len(), 72);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn interleaved_writers_are_refused() {
        let dir = std::env::temp_dir().join(format!(
            "df-staging-interleave-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let a = dir.join("a.bin").to_string_lossy().to_string();
        let b = dir.join("b.bin").to_string_lossy().to_string();

        let staging = MeshStaging::new();
        staging.append_mesh_stage_chunk(&a, &soup(), true).expect("a");
        let err = staging
            .append_mesh_stage_chunk(&b, &soup(), false)
            .expect_err("must refuse");
        assert!(err.contains("concurrent mesh-stage write"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn prealloc_is_clamped() {
        assert_eq!(normalize_staged_mesh_prealloc_bytes(0), 16 * 1024 * 1024);
        assert_eq!(
            normalize_staged_mesh_prealloc_bytes(usize::MAX / 2),
            1024 * 1024 * 1024
        );
    }
}
