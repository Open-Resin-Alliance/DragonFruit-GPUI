//! Atomic scene-file writes — `scene_file_begin_atomic_write`,
//! `scene_file_commit_atomic`, `scene_file_discard_atomic_temp`.
//!
//! Ported from `src-tauri/src/main.rs` lines 220-346 and 2957-3000. A save writes
//! to a sibling temp (`<target>.tmp-<pid>-<seq>`), fsyncs it, then renames it over
//! the target, so a reader sees either the previous good file or the new one —
//! never a truncated prefix. The temp is a *sibling* because `fs::rename` is only
//! atomic within a volume and the system temp may be on another one.
//!
//! The discard command is gated on [`is_scene_commit_temp_path`]: it can only ever
//! delete a name this allocator could have produced, never an arbitrary path a
//! caller supplies.

use std::path::{Path, PathBuf};

use crate::BackendResult;

/// Marker identifying a scene-file commit temp. Must stay byte-identical: the
/// discard command's gate parses it.
const SCENE_COMMIT_TEMP_MARKER: &str = ".tmp-";

/// Rename retries, in milliseconds.
///
/// A Windows concession: a rename can transiently fail with
/// `ERROR_ACCESS_DENIED`/`ERROR_SHARING_VIOLATION` while an antivirus scanner or
/// an Explorer thumbnailer holds the destination open.
const SCENE_COMMIT_RENAME_BACKOFF_MS: [u64; 3] = [50, 150, 300];

/// Crash residue can be minutes old for a large scene; anything younger than this
/// may still be an in-flight write from a second instance.
pub const SCENE_COMMIT_TEMP_SWEEP_MIN_AGE_SECONDS: u64 = 30 * 60;

/// Allocate the staging path for an atomic commit, as a sibling of the target.
///
/// pid + a process-wide counter rather than a nanotime stamp (which the other
/// allocators use): Windows `SystemTime` granularity lets two concurrent
/// allocations land in the same tick, and the consumer opens with
/// `truncate(true)` — two in-flight saves sharing a name would let one overwrite
/// the other's staging bytes and then rename the wrong content over the user's
/// project.
pub fn sibling_commit_temp_path(target: &Path) -> BackendResult<PathBuf> {
    static COMMIT_TEMP_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    let file_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            format!(
                "Cannot stage an atomic commit for '{}': no UTF-8 file name",
                target.display()
            )
        })?;

    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    if !parent.as_os_str().is_empty() {
        std::fs::create_dir_all(parent).map_err(|err| {
            format!(
                "Failed creating destination directory '{}': {err}",
                parent.display()
            )
        })?;
    }

    Ok(parent.join(format!(
        "{file_name}{SCENE_COMMIT_TEMP_MARKER}{}-{}",
        std::process::id(),
        COMMIT_TEMP_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
    )))
}

/// Whether `path` is a `<target>.tmp-<pid>-<seq>` name.
pub fn is_scene_commit_temp_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| {
            let Some((_, suffix)) = name.rsplit_once(SCENE_COMMIT_TEMP_MARKER) else {
                return false;
            };
            let mut parts = suffix.split('-');
            let pid_ok = parts
                .next()
                .map(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
                .unwrap_or(false);
            let seq_ok = parts
                .next()
                .map(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
                .unwrap_or(false);
            pid_ok && seq_ok && parts.next().is_none()
        })
        .unwrap_or(false)
}

/// The pid segment of a commit-temp name.
pub fn scene_commit_temp_pid(path: &Path) -> Option<u32> {
    let name = path.file_name().and_then(|n| n.to_str())?;
    let (_, suffix) = name.rsplit_once(SCENE_COMMIT_TEMP_MARKER)?;
    suffix.split('-').next()?.parse::<u32>().ok()
}

/// `scene_file_begin_atomic_write`: the sibling staging path for `target_path`.
pub fn scene_file_begin_atomic_write(target_path: &str) -> BackendResult<String> {
    let trimmed = target_path.trim();
    if trimmed.is_empty() {
        return Err("scene_file_begin_atomic_write requires a non-empty target path".into());
    }
    let temp = sibling_commit_temp_path(Path::new(trimmed))?;
    Ok(temp.to_string_lossy().to_string())
}

/// `scene_file_commit_atomic`: fsync the temp, then rename it over the target.
///
/// On terminal failure the temp is removed and the **previous good target is left
/// untouched**.
pub fn scene_file_commit_atomic(temp_path: &str, target_path: &str) -> BackendResult<()> {
    let temp = temp_path.trim();
    let target = target_path.trim();
    if temp.is_empty() || target.is_empty() {
        return Err("scene_file_commit_atomic requires non-empty temp and target paths".into());
    }
    commit_scene_file_atomic(Path::new(temp), Path::new(target))
}

/// The commit itself, split out so the autosave path can use the same code.
pub fn commit_scene_file_atomic(temp: &Path, target: &Path) -> BackendResult<()> {
    if !temp.exists() {
        return Err(format!(
            "Atomic commit staging file is missing: {}",
            temp.display()
        ));
    }

    // fsync the payload BEFORE the rename, or a power loss can leave the rename
    // durable while the bytes it points at are not. Opened for write rather than
    // read-only (the obvious choice on unix) because Windows
    // `FlushFileBuffers` requires write access to the handle.
    let sync_result = std::fs::OpenOptions::new()
        .write(true)
        .open(temp)
        .and_then(|file| file.sync_all());
    if let Err(err) = sync_result {
        let _ = std::fs::remove_file(temp);
        return Err(format!(
            "Failed flushing '{}' to disk before commit: {err}",
            temp.display()
        ));
    }

    let mut last_error: Option<String> = None;
    for (attempt, backoff_ms) in SCENE_COMMIT_RENAME_BACKOFF_MS.iter().enumerate() {
        match std::fs::rename(temp, target) {
            Ok(()) => {
                // Best-effort directory fsync so the rename itself is durable. No
                // portable Windows equivalent (a directory cannot be opened as a
                // file), where NTFS metadata journalling covers it.
                #[cfg(unix)]
                {
                    if let Some(parent) = target.parent() {
                        if let Ok(dir) = std::fs::File::open(parent) {
                            let _ = dir.sync_all();
                        }
                    }
                }
                return Ok(());
            }
            Err(err) => {
                last_error = Some(err.to_string());
                if attempt + 1 < SCENE_COMMIT_RENAME_BACKOFF_MS.len() {
                    std::thread::sleep(std::time::Duration::from_millis(*backoff_ms));
                }
            }
        }
    }

    let _ = std::fs::remove_file(temp);
    Err(format!(
        "Failed committing '{}' over '{}' after {} attempts: {}",
        temp.display(),
        target.display(),
        SCENE_COMMIT_RENAME_BACKOFF_MS.len(),
        last_error.unwrap_or_else(|| "unknown error".to_string()),
    ))
}

/// `scene_file_discard_atomic_temp`: remove an abandoned staging file, gated on
/// [`is_scene_commit_temp_path`] so no arbitrary path can be deleted here.
pub fn scene_file_discard_atomic_temp(temp_path: &str) -> BackendResult<()> {
    let temp = temp_path.trim();
    if temp.is_empty() {
        return Err("scene_file_discard_atomic_temp requires a non-empty path".into());
    }
    let path = Path::new(temp);
    if !is_scene_commit_temp_path(path) {
        return Err(format!(
            "Refusing to discard '{temp}': not a scene-file commit temp"
        ));
    }
    if path.exists() {
        std::fs::remove_file(path)
            .map_err(|err| format!("Failed discarding commit temp '{temp}': {err}"))?;
    }
    Ok(())
}

/// Remove commit temps abandoned by a crashed write, in `dir`.
///
/// Conservative by construction, because these live in the user's own project
/// folder: only names [`is_scene_commit_temp_path`] recognises, never this
/// process's own temps (they may be in flight — the discard command owns those),
/// and only past an age gate so another instance's in-flight write is never
/// deleted out from under it.
pub fn sweep_stale_scene_commit_temps(dir: &Path, min_age_seconds: u64) -> u32 {
    let cutoff = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(min_age_seconds))
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
    let current_pid = std::process::id();

    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };

    let mut removed = 0u32;
    for entry in entries.flatten() {
        let path = entry.path();
        if !is_scene_commit_temp_path(&path) {
            continue;
        }
        if scene_commit_temp_pid(&path) == Some(current_pid) {
            continue;
        }
        let stale = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .map(|modified| modified <= cutoff)
            .unwrap_or(false);
        if stale && std::fs::remove_file(&path).is_ok() {
            removed += 1;
        }
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "df-scene-file-{label}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn commit_replaces_the_target() {
        let dir = temp_dir("commit");
        let target = dir.join("scene.voxl");
        std::fs::write(&target, b"old").expect("seed");

        let temp = scene_file_begin_atomic_write(&target.to_string_lossy()).expect("begin");
        std::fs::write(&temp, b"new").expect("stage");
        scene_file_commit_atomic(&temp, &target.to_string_lossy()).expect("commit");

        assert_eq!(std::fs::read(&target).expect("read"), b"new");
        assert!(!Path::new(&temp).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn commit_without_a_staging_file_does_not_touch_the_target() {
        let dir = temp_dir("missing-temp");
        let target = dir.join("scene.voxl");
        std::fs::write(&target, b"old").expect("seed");
        let err = commit_scene_file_atomic(&dir.join("nope.tmp-1-2"), &target).expect_err("refuse");
        assert!(err.contains("staging file is missing"), "{err}");
        assert_eq!(std::fs::read(&target).expect("read"), b"old");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn discard_is_gated_on_the_temp_name_shape() {
        let dir = temp_dir("discard");
        let project = dir.join("scene.voxl");
        std::fs::write(&project, b"keep me").expect("seed");

        let err = scene_file_discard_atomic_temp(&project.to_string_lossy())
            .expect_err("must refuse the project");
        assert!(err.contains("not a scene-file commit temp"), "{err}");
        assert!(project.exists());

        let temp = sibling_commit_temp_path(&project).expect("temp");
        std::fs::write(&temp, b"junk").expect("stage");
        assert!(is_scene_commit_temp_path(&temp));
        scene_file_discard_atomic_temp(&temp.to_string_lossy()).expect("discard");
        assert!(!temp.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn temp_names_are_unique_and_siblings() {
        let dir = temp_dir("unique");
        let target = dir.join("scene.voxl");
        let a = sibling_commit_temp_path(&target).expect("a");
        let b = sibling_commit_temp_path(&target).expect("b");
        assert_ne!(a, b);
        assert_eq!(a.parent(), Some(dir.as_path()));
        assert_eq!(scene_commit_temp_pid(&a), Some(std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_foreign_name_is_not_a_commit_temp() {
        assert!(!is_scene_commit_temp_path(Path::new("scene.voxl")));
        assert!(!is_scene_commit_temp_path(Path::new("scene.voxl.tmp-x-1")));
        assert!(!is_scene_commit_temp_path(Path::new("scene.voxl.tmp-1-2-3")));
        assert!(is_scene_commit_temp_path(Path::new("scene.voxl.tmp-1-2")));
    }
}
