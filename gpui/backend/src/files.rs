//! Print-file reads, temp-artifact housekeeping, and plain file writes.
//!
//! Ported from `src-tauri/src/main.rs` lines 100-112, 348-400, 3514-3900.
//!
//! These commands existed because the webview cannot read a `file://` path or
//! write outside the sandbox: the printing workspace streams a sliced container
//! back a chunk at a time, and the temp artifacts the slicer leaves in `%TEMP%`
//! need sweeping. In the GPUI app the same work happens in this process, so the
//! only reason they remain commands is that the *renderer* still asks for the
//! bytes.

use std::path::{Path, PathBuf};

use crate::BackendResult;

/// A fresh `dragonfruit-slice-<nanos>.<ext>` path in the system temp directory.
///
/// The nanos stamp is what the slicer names its output when the caller did not
/// choose one; `is_dragonfruit_temp_artifact` recognises the prefix, which is
/// what makes the sweeps and the delete command safe.
pub fn temp_artifact_path(extension: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let clean_ext = extension.trim_start_matches('.');
    path.push(format!("dragonfruit-slice-{stamp}.{clean_ext}"));
    path
}

/// Whether `path` is one of our temp artifacts: the slicer's file name, in the
/// temp directory. Both halves matter, so a user file that happens to be named
/// `dragonfruit-slice-1.print` somewhere else is never deleted.
pub fn is_dragonfruit_temp_artifact(path: &Path) -> bool {
    let file_name_ok = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with("dragonfruit-slice-"))
        .unwrap_or(false);
    let in_temp_dir = path.starts_with(std::env::temp_dir());
    file_name_ok && in_temp_dir
}

/// Delete artifacts older than `max_age_seconds`; returns how many went.
pub fn sweep_stale_temp_artifacts(max_age_seconds: u64) -> u32 {
    let mut removed = 0u32;
    let temp_dir = std::env::temp_dir();
    let cutoff = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(max_age_seconds))
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

    let entries = match std::fs::read_dir(&temp_dir) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !is_dragonfruit_temp_artifact(&path) {
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

/// Delete every artifact, regardless of age.
pub fn sweep_all_temp_artifacts() -> u32 {
    let mut removed = 0u32;
    let temp_dir = std::env::temp_dir();
    let entries = match std::fs::read_dir(&temp_dir) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !is_dragonfruit_temp_artifact(&path) {
            continue;
        }
        if std::fs::remove_file(&path).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// `cleanup_stale_print_temp_files`. The shell clamped the age to at least 60 s
/// so a caller cannot sweep a directory an in-flight slice is writing into.
pub fn cleanup_stale_print_temp_files(max_age_seconds: u64) -> u32 {
    sweep_stale_temp_artifacts(max_age_seconds.max(60))
}

/// `cleanup_all_print_temp_files`.
pub fn cleanup_all_print_temp_files() -> u32 {
    sweep_all_temp_artifacts()
}

/// A print file's bytes, whole.
pub fn read_print_file_bytes(source_path: &str) -> BackendResult<Vec<u8>> {
    let source = PathBuf::from(source_path.trim());
    if !source.exists() {
        return Err("Source print file no longer exists on disk".to_string());
    }
    std::fs::read(&source).map_err(|err| format!("Failed reading print file: {err}"))
}

/// A print file's length, without reading it.
pub fn read_print_file_size(source_path: &str) -> BackendResult<u64> {
    let source = PathBuf::from(source_path.trim());
    if !source.exists() {
        return Err("Source print file no longer exists on disk".to_string());
    }
    std::fs::metadata(&source)
        .map(|meta| meta.len())
        .map_err(|err| format!("Failed reading print file metadata: {err}"))
}

/// One chunk of a print file, capped at 8 MiB per call.
pub fn read_print_file_chunk(source_path: &str, offset: u64, length: u64) -> BackendResult<Vec<u8>> {
    const MAX_CHUNK_BYTES: usize = 8 * 1024 * 1024;

    let source = PathBuf::from(source_path.trim());
    if !source.exists() {
        return Err("Source print file no longer exists on disk".to_string());
    }

    let mut file = std::fs::File::open(&source)
        .map_err(|err| format!("Failed opening print file: {err}"))?;
    let file_len = file
        .metadata()
        .map_err(|err| format!("Failed reading print file metadata: {err}"))?
        .len();

    if offset >= file_len {
        return Ok(Vec::new());
    }

    let remaining = file_len - offset;
    let requested = length.max(1).min(MAX_CHUNK_BYTES as u64).min(remaining) as usize;

    use std::io::{Read, Seek, SeekFrom};
    file.seek(SeekFrom::Start(offset))
        .map_err(|err| format!("Failed seeking print file chunk: {err}"))?;
    let mut chunk = vec![0u8; requested];
    file.read_exact(&mut chunk)
        .map_err(|err| format!("Failed reading print file chunk: {err}"))?;
    Ok(chunk)
}

/// One layer preview PNG out of a sliced container, decoded by the engine.
pub fn read_print_layer_png(
    source_path: &str,
    layer_number: u32,
    format_hint: &str,
) -> BackendResult<Vec<u8>> {
    if layer_number == 0 {
        return Err("Layer number must be >= 1".to_string());
    }
    let source = PathBuf::from(source_path.trim());
    if !source.exists() {
        return Err("Source print file no longer exists on disk".to_string());
    }
    dragonfruit_slicing_engine::engine::read_layer_preview_png_by_format_hint(
        &source,
        layer_number,
        format_hint,
    )
    .map_err(|err| err.to_string())
}

/// A run of layer previews in one call, packed as
/// `[u32 LE length][bytes]` per layer in the order asked for.
///
/// The printing workspace wants the layers either side of the one on screen; one
/// call and every core beats one round trip per layer. An empty entry is a layer
/// that would not decode.
pub fn read_print_layer_pngs(
    source_path: &str,
    layer_numbers: &[u32],
    format_hint: &str,
) -> BackendResult<Vec<u8>> {
    if layer_numbers.is_empty() {
        return Ok(Vec::new());
    }
    let source = PathBuf::from(source_path.trim());
    if !source.exists() {
        return Err("Source print file no longer exists on disk".to_string());
    }
    if layer_numbers.iter().any(|number| *number == 0) {
        return Err("Layer number must be >= 1".to_string());
    }

    use rayon::prelude::*;
    let decoded: Vec<Vec<u8>> = layer_numbers
        .par_iter()
        .map(|layer_number| {
            dragonfruit_slicing_engine::engine::read_layer_preview_png_by_format_hint(
                &source,
                *layer_number,
                format_hint,
            )
            .unwrap_or_default()
        })
        .collect();

    let mut packed: Vec<u8> =
        Vec::with_capacity(decoded.iter().map(|bytes| bytes.len() + 4).sum());
    for bytes in decoded {
        packed.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        packed.extend_from_slice(&bytes);
    }
    Ok(packed)
}

/// Which of these paths already exist, in order.
///
/// For a caller about to write files over them: a batch of plate outputs lands in
/// a folder the user picked, and that folder may already hold files of the same
/// names from an earlier run. The shell wrapped this in `Result` only because the
/// blocking join could fail; there is nothing to fail here.
pub fn existing_paths(paths: &[String]) -> Vec<bool> {
    paths
        .iter()
        .map(|path| Path::new(path.trim()).exists())
        .collect()
}

/// Delete one slicer temp artifact. Refuses anything that is not one.
pub fn delete_print_temp_file(source_path: &str) -> BackendResult<bool> {
    let source = PathBuf::from(source_path.trim());
    if !source.exists() {
        return Ok(false);
    }
    if !is_dragonfruit_temp_artifact(&source) {
        return Err("Refusing to delete non-DragonFruit temp artifact path".to_string());
    }
    std::fs::remove_file(&source).map_err(|err| format!("Failed deleting temp artifact: {err}"))?;
    Ok(true)
}

/// `write_bytes_to_path`: create the parent directory and write the bytes.
///
/// The renderer used this to hand a produced file to a destination the user had
/// already chosen (the picker ran separately), so it takes the path, not a dialog.
pub fn write_bytes_to_path(destination_path: &str, bytes: &[u8]) -> BackendResult<String> {
    let trimmed_destination = destination_path.trim();
    if trimmed_destination.is_empty() {
        return Err("Destination path is empty".to_string());
    }
    let destination = PathBuf::from(trimmed_destination);
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("Failed creating destination folder: {err}"))?;
    }
    std::fs::write(&destination, bytes).map_err(|err| format!("Failed writing file bytes: {err}"))?;
    Ok(destination.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn our_artifact_is_recognised_and_a_lookalike_is_not() {
        let artifact = temp_artifact_path("print");
        assert!(is_dragonfruit_temp_artifact(&artifact));
        assert_eq!(
            artifact.extension().and_then(|e| e.to_str()),
            Some("print")
        );

        let elsewhere = PathBuf::from(if cfg!(windows) {
            "C:/Users/public/dragonfruit-slice-1.print"
        } else {
            "/var/tmp/dragonfruit-slice-1.print"
        });
        assert!(!is_dragonfruit_temp_artifact(&elsewhere));
        assert!(!is_dragonfruit_temp_artifact(&std::env::temp_dir().join("notes.txt")));
    }

    #[test]
    fn delete_refuses_a_non_artifact() {
        let dir = std::env::temp_dir().join(format!("df-files-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let file = dir.join("user-file.print");
        std::fs::write(&file, b"x").expect("write");

        let err = delete_print_temp_file(&file.to_string_lossy()).expect_err("refuse");
        assert!(err.contains("Refusing to delete"), "{err}");
        assert!(file.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stale_sweep_respects_the_age_gate() {
        let artifact = temp_artifact_path("sweeptest");
        std::fs::write(&artifact, b"x").expect("write");
        // One hour is newer than a one-day cutoff, so the fresh file survives.
        sweep_stale_temp_artifacts(24 * 60 * 60);
        assert!(artifact.exists());
        // Let the mtime settle below the zero-age cutoff before sweeping it.
        std::thread::sleep(std::time::Duration::from_millis(20));
        let removed = sweep_stale_temp_artifacts(0);
        assert!(removed >= 1);
        assert!(!artifact.exists());
    }

    #[test]
    fn chunk_reads_clamp_and_stop_at_eof() {
        let dir = std::env::temp_dir().join(format!("df-chunks-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let file = dir.join("blob.bin");
        let payload: Vec<u8> = (0..100u8).collect();
        std::fs::write(&file, &payload).expect("write");
        let path = file.to_string_lossy().to_string();

        assert_eq!(read_print_file_size(&path).expect("size"), 100);
        assert_eq!(read_print_file_bytes(&path).expect("bytes"), payload);
        assert_eq!(
            read_print_file_chunk(&path, 90, 1_000).expect("chunk"),
            payload[90..].to_vec()
        );
        assert!(read_print_file_chunk(&path, 100, 10).expect("past end").is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_is_reported_not_silently_empty() {
        let err = read_print_file_bytes("definitely/not/here.print").expect_err("missing");
        assert!(err.contains("no longer exists"), "{err}");
    }

    #[test]
    fn write_bytes_creates_the_parent_and_existing_paths_answers_in_order() {
        let dir = std::env::temp_dir().join(format!("df-write-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let target = dir.join("nested").join("out.print");

        let written = write_bytes_to_path(&target.to_string_lossy(), b"payload").expect("write");
        assert_eq!(written, target.to_string_lossy());
        assert_eq!(std::fs::read(&target).expect("read"), b"payload");

        let answers = existing_paths(&[
            target.to_string_lossy().to_string(),
            dir.join("absent").to_string_lossy().to_string(),
        ]);
        assert_eq!(answers, vec![true, false]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_destination_is_refused() {
        let err = write_bytes_to_path("   ", b"x").expect_err("refuse");
        assert!(err.contains("Destination path is empty"), "{err}");
    }
}
