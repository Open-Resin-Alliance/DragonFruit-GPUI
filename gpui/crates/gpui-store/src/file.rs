//! Reading and writing one JSON document atomically, with corrupt-file
//! quarantine and the newer-schema guard.
//!
//! The rules mirror the persistence story the React app never had: every write
//! is a rename, every unreadable file is moved aside instead of being
//! overwritten, and a document from a newer build is left exactly as it is.

use std::fs::{self, File};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::{StoreError, StoreFile};

/// The only field read before the full parse: a document from a newer build may
/// change the shape of fields this build knows, so the version has to be read
/// without deserialising the rest.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionProbe {
    #[serde(default)]
    schema_version: Option<u32>,
}

/// Look up a document path in the store root.
pub(crate) fn document_path(root: &Path, file: StoreFile) -> PathBuf {
    root.join(file.file_name())
}

/// The temporary name a write uses before it is renamed over the target.
fn temp_path(root: &Path, file: StoreFile) -> PathBuf {
    root.join(format!("{}.tmp", file.file_name()))
}

/// Read `file`, or `None` when it is absent, was quarantined, or was written by
/// a newer schema (in which case `read_only` records the version found).
pub(crate) fn read_document<T: DeserializeOwned>(
    root: &Path,
    file: StoreFile,
    read_only: &AtomicU32,
) -> Option<T> {
    let path = document_path(root, file);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        // Missing is the normal first run. Any other read failure also falls
        // back to defaults: a load must never write, so nothing is clobbered.
        Err(_) => return None,
    };

    let probe: VersionProbe = match serde_json::from_slice(&bytes) {
        Ok(probe) => probe,
        Err(_) => {
            let _ = quarantine(&path);
            return None;
        }
    };

    if let Some(version) = probe.schema_version {
        if version > file.schema_version() {
            // A newer build owns this file. Return defaults and remember why,
            // so every save of it is refused instead of downgrading the data.
            read_only.store(version, Ordering::Relaxed);
            return None;
        }
    }

    match serde_json::from_slice(&bytes) {
        Ok(value) => Some(value),
        Err(_) => {
            let _ = quarantine(&path);
            None
        }
    }
}

/// Write `value` to `<file>.tmp`, then rename it over `file`. The caller's
/// directory is created if needed; a failed rename removes the temp file so no
/// `.tmp` is left behind.
pub(crate) fn write_atomic<T: Serialize>(
    root: &Path,
    file: StoreFile,
    value: &T,
) -> Result<(), StoreError> {
    fs::create_dir_all(root).map_err(|source| StoreError::CreateDir {
        path: root.to_path_buf(),
        source,
    })?;

    let path = document_path(root, file);
    let temp = temp_path(root, file);

    let mut bytes = serde_json::to_vec_pretty(value).map_err(|source| StoreError::Serialize {
        file,
        source,
    })?;
    // A trailing newline keeps the files diff-friendly and editor-friendly.
    bytes.push(b'\n');

    let mut handle = File::create(&temp).map_err(|source| StoreError::Write {
        path: temp.clone(),
        source,
    })?;
    handle
        .write_all(&bytes)
        .and_then(|()| handle.sync_all())
        .map_err(|source| StoreError::Write {
            path: temp.clone(),
            source,
        })?;
    // Close before the rename: Windows refuses to replace an open file.
    drop(handle);

    fs::rename(&temp, &path).map_err(|source| {
        let _ = fs::remove_file(&temp);
        StoreError::Rename {
            path: path.clone(),
            source,
        }
    })
}

/// Move an unreadable file aside as `<file>.corrupt-<unix-ts>`. Never deletes.
fn quarantine(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    let name = path.file_name()?.to_string_lossy().to_string();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);

    for attempt in 0..100 {
        let suffix = if attempt == 0 {
            format!("{name}.corrupt-{timestamp}")
        } else {
            format!("{name}.corrupt-{timestamp}-{attempt}")
        };
        let target = parent.join(suffix);
        if target.exists() {
            continue;
        }
        return match fs::rename(path, &target) {
            Ok(()) => Some(target),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(_) => None,
        };
    }
    None
}
