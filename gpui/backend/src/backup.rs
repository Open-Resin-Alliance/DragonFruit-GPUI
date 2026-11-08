//! Local backups — `local_backup_*` except the folder picker.
//!
//! Ported from `src-tauri/src/main.rs` lines 2225-2610. A backup directory is a
//! plain folder the user owns: `state.json` plus a `history/` of timestamped copies.
//! Everything here is ordinary file work; only the default directory needs the app's
//! data dir.
//!
//! `local_backup_pick_directory` is **not** here: a GPUI app opens its own folder
//! dialog (`rfd`), and this crate has no UI dependency.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::context::AppContext;
use crate::BackendResult;

const LOCAL_BACKUP_STATE_FILE_NAME: &str = "state.json";
const LOCAL_BACKUP_HISTORY_DIR_NAME: &str = "history";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalBackupStateResponse {
    pub document_json: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalBackupSyncResponse {
    pub synced_at: String,
    pub history_id: String,
    pub state_path: String,
    pub history_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalBackupHistoryEntry {
    pub id: String,
    pub path: String,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalBackupReadHistoryResponse {
    pub document_json: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalBackupRestoreResponse {
    pub synced_at: Option<String>,
}

/// Resolve the backup root: trimmed, and absolute (a relative path is resolved
/// against the process's working directory, which the UI cannot do for itself).
pub fn normalize_backup_directory_path(directory_path: &str) -> BackendResult<PathBuf> {
    let trimmed = directory_path.trim();
    if trimmed.is_empty() {
        return Err("Backup directory path is empty".to_string());
    }
    let mut path = PathBuf::from(trimmed);
    if !path.is_absolute() {
        path = std::env::current_dir()
            .map_err(|err| format!("Failed to resolve current directory: {err}"))?
            .join(path);
    }
    Ok(path)
}

fn local_backup_state_path(root: &Path) -> PathBuf {
    root.join(LOCAL_BACKUP_STATE_FILE_NAME)
}

fn local_backup_history_dir(root: &Path) -> PathBuf {
    root.join(LOCAL_BACKUP_HISTORY_DIR_NAME)
}

/// A history id is a 13-digit millisecond timestamp. Anything else in the folder is
/// not ours, and the fixed width is what makes `history/<id>.json` unforgeable.
fn is_valid_local_backup_history_id(id: &str) -> bool {
    id.len() == 13 && id.chars().all(|c| c.is_ascii_digit())
}

fn local_backup_history_file_path(root: &Path, id: &str) -> BackendResult<PathBuf> {
    if !is_valid_local_backup_history_id(id) {
        return Err("Invalid backup history identifier".to_string());
    }
    Ok(local_backup_history_dir(root).join(format!("{id}.json")))
}

fn ensure_local_backup_structure(root: &Path) -> BackendResult<()> {
    std::fs::create_dir_all(root).map_err(|err| {
        format!(
            "Failed creating backup directory '{}': {err}",
            root.display()
        )
    })?;
    std::fs::create_dir_all(local_backup_history_dir(root)).map_err(|err| {
        format!(
            "Failed creating backup history directory '{}': {err}",
            local_backup_history_dir(root).display()
        )
    })?;
    Ok(())
}

/// The document's own `updatedAt`, or the snapshot's — whichever the payload carries.
fn extract_backup_document_updated_at(raw: &str) -> Option<String> {
    let parsed = serde_json::from_str::<serde_json::Value>(raw).ok()?;
    parsed
        .get("updatedAt")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
        .or_else(|| {
            parsed
                .get("snapshot")
                .and_then(|snapshot| snapshot.get("updatedAt"))
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())
        })
}

/// `local_backup_default_directory`: `<app data>/backups`, created if needed.
pub fn local_backup_default_directory(ctx: &impl AppContext) -> BackendResult<String> {
    let directory = ctx.data_dir()?.join("backups");
    std::fs::create_dir_all(&directory).map_err(|err| {
        format!(
            "Failed creating default backup directory '{}': {err}",
            directory.display()
        )
    })?;
    Ok(directory.to_string_lossy().to_string())
}

/// `local_backup_read_state`: the current backup document, if the folder has one.
pub fn local_backup_read_state(directory_path: &str) -> BackendResult<LocalBackupStateResponse> {
    let root = normalize_backup_directory_path(directory_path)?;
    let state_path = local_backup_state_path(&root);

    match std::fs::read_to_string(&state_path) {
        Ok(content) => Ok(LocalBackupStateResponse {
            updated_at: extract_backup_document_updated_at(&content),
            document_json: Some(content),
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(LocalBackupStateResponse {
            updated_at: None,
            document_json: None,
        }),
        Err(error) => Err(format!(
            "Failed reading backup state file '{}': {error}",
            state_path.display()
        )),
    }
}

/// `local_backup_sync`: normalise and write the document to both `state.json` and a
/// new history entry.
///
/// The document is re-serialised pretty-printed first, so a backup is diffable and a
/// hand-edited file is regularised. The history id is the current time in
/// milliseconds, which is also the sort key.
pub fn local_backup_sync(
    directory_path: &str,
    document_json: &str,
) -> BackendResult<LocalBackupSyncResponse> {
    let root = normalize_backup_directory_path(directory_path)?;
    ensure_local_backup_structure(&root)?;

    let parsed = serde_json::from_str::<serde_json::Value>(document_json)
        .map_err(|err| format!("Invalid backup document JSON: {err}"))?;
    let normalized_document = serde_json::to_string_pretty(&parsed)
        .map_err(|err| format!("Failed formatting backup document JSON: {err}"))?;

    let synced_at = extract_backup_document_updated_at(document_json).unwrap_or_default();

    let history_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_string());

    let state_path = local_backup_state_path(&root);
    let history_path = local_backup_history_file_path(&root, &history_id)?;

    std::fs::write(&state_path, normalized_document.as_bytes()).map_err(|err| {
        format!(
            "Failed writing backup state file '{}': {err}",
            state_path.display()
        )
    })?;
    std::fs::write(&history_path, normalized_document.as_bytes()).map_err(|err| {
        format!(
            "Failed writing backup history file '{}': {err}",
            history_path.display()
        )
    })?;

    Ok(LocalBackupSyncResponse {
        synced_at,
        history_id,
        state_path: state_path.to_string_lossy().to_string(),
        history_path: history_path.to_string_lossy().to_string(),
    })
}

/// `local_backup_list_history`: every history entry, newest first.
pub fn local_backup_list_history(directory_path: &str) -> BackendResult<Vec<LocalBackupHistoryEntry>> {
    let root = normalize_backup_directory_path(directory_path)?;
    let history_dir = local_backup_history_dir(&root);

    if !history_dir.exists() {
        return Ok(Vec::new());
    }

    let read_dir = std::fs::read_dir(&history_dir).map_err(|err| {
        format!(
            "Failed listing backup history directory '{}': {err}",
            history_dir.display()
        )
    })?;

    let mut entries: Vec<LocalBackupHistoryEntry> = Vec::new();
    for dir_entry in read_dir.flatten() {
        let path = dir_entry.path();
        if !path.is_file() {
            continue;
        }
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
            .unwrap_or_default();
        if extension != "json" {
            continue;
        }
        let id = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        if !is_valid_local_backup_history_id(&id) {
            continue;
        }
        let updated_at = std::fs::read_to_string(&path)
            .ok()
            .and_then(|content| extract_backup_document_updated_at(&content));
        entries.push(LocalBackupHistoryEntry {
            id,
            path: path.to_string_lossy().to_string(),
            updated_at,
        });
    }

    entries.sort_by(|left, right| right.id.cmp(&left.id));
    Ok(entries)
}

/// `local_backup_read_history_item`.
pub fn local_backup_read_history_item(
    directory_path: &str,
    id: &str,
) -> BackendResult<LocalBackupReadHistoryResponse> {
    let root = normalize_backup_directory_path(directory_path)?;
    let history_path = local_backup_history_file_path(&root, id.trim())?;
    let document_json = std::fs::read_to_string(&history_path).map_err(|err| {
        format!(
            "Failed reading backup history file '{}': {err}",
            history_path.display()
        )
    })?;
    Ok(LocalBackupReadHistoryResponse { document_json })
}

/// `local_backup_delete_history_item`: `false` when it was already gone.
pub fn local_backup_delete_history_item(directory_path: &str, id: &str) -> BackendResult<bool> {
    let root = normalize_backup_directory_path(directory_path)?;
    let history_path = local_backup_history_file_path(&root, id.trim())?;
    match std::fs::remove_file(&history_path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!(
            "Failed deleting backup history file '{}': {error}",
            history_path.display()
        )),
    }
}

/// `local_backup_restore_history_item`: copy a history entry back over `state.json`.
pub fn local_backup_restore_history_item(
    directory_path: &str,
    id: &str,
) -> BackendResult<LocalBackupRestoreResponse> {
    let root = normalize_backup_directory_path(directory_path)?;
    ensure_local_backup_structure(&root)?;

    let history_path = local_backup_history_file_path(&root, id.trim())?;
    let state_path = local_backup_state_path(&root);

    let document_json = std::fs::read_to_string(&history_path).map_err(|err| {
        format!(
            "Failed reading backup history file '{}': {err}",
            history_path.display()
        )
    })?;
    std::fs::write(&state_path, document_json.as_bytes()).map_err(|err| {
        format!(
            "Failed restoring backup state file '{}': {err}",
            state_path.display()
        )
    })?;

    Ok(LocalBackupRestoreResponse {
        synced_at: extract_backup_document_updated_at(&document_json),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::HeadlessContext;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("df-backup-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    const DOCUMENT: &str = r#"{"updatedAt":"2026-01-01T00:00:00Z","snapshot":{"a":1}}"#;

    #[test]
    fn history_ids_are_thirteen_digits() {
        assert!(is_valid_local_backup_history_id("1767225600000"));
        assert!(!is_valid_local_backup_history_id("176722560000"));
        assert!(!is_valid_local_backup_history_id("17672256000000"));
        assert!(!is_valid_local_backup_history_id("17672256abcd"));
        assert!(!is_valid_local_backup_history_id(""));
    }

    #[test]
    fn sync_writes_state_and_a_history_entry_then_lists_it() {
        let dir = temp_dir("sync");
        let root = dir.to_string_lossy().to_string();

        let response = local_backup_sync(&root, DOCUMENT).expect("sync");
        assert_eq!(response.synced_at, "2026-01-01T00:00:00Z");
        assert_eq!(response.history_id.len(), 13);
        assert!(Path::new(&response.state_path).exists());
        assert!(Path::new(&response.history_path).exists());

        let state = local_backup_read_state(&root).expect("state");
        assert_eq!(state.updated_at.as_deref(), Some("2026-01-01T00:00:00Z"));
        let document = state.document_json.expect("document");
        // Pretty-printed, so a backup is diffable.
        assert!(document.contains('\n'), "{document}");
        assert!(document.contains("\"updatedAt\""));

        let history = local_backup_list_history(&root).expect("history");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, response.history_id);
        assert_eq!(history[0].updated_at.as_deref(), Some("2026-01-01T00:00:00Z"));

        let read = local_backup_read_history_item(&root, &response.history_id).expect("read");
        assert!(read.document_json.contains("\"snapshot\""));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn restore_replaces_the_state_and_delete_removes_the_entry() {
        let dir = temp_dir("restore");
        let root = dir.to_string_lossy().to_string();
        let first = local_backup_sync(&root, DOCUMENT).expect("sync one");

        std::thread::sleep(std::time::Duration::from_millis(5));
        let second = local_backup_sync(
            &root,
            r#"{"updatedAt":"2026-02-02T00:00:00Z","value":2}"#,
        )
        .expect("sync two");
        assert_ne!(first.history_id, second.history_id);

        let restored = local_backup_restore_history_item(&root, &first.history_id).expect("restore");
        assert_eq!(restored.synced_at.as_deref(), Some("2026-01-01T00:00:00Z"));
        assert_eq!(
            local_backup_read_state(&root)
                .expect("state")
                .updated_at
                .as_deref(),
            Some("2026-01-01T00:00:00Z")
        );

        assert!(local_backup_delete_history_item(&root, &first.history_id).expect("delete"));
        assert!(!local_backup_delete_history_item(&root, &first.history_id).expect("gone"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_empty_folder_reads_as_no_backup_and_lists_nothing() {
        let dir = temp_dir("empty");
        let root = dir.to_string_lossy().to_string();
        let state = local_backup_read_state(&root).expect("state");
        assert_eq!(state.document_json, None);
        assert_eq!(state.updated_at, None);
        assert!(local_backup_list_history(&root).expect("history").is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_bad_history_id_is_refused_before_touching_the_disk() {
        let dir = temp_dir("bad-id");
        let root = dir.to_string_lossy().to_string();
        local_backup_sync(&root, DOCUMENT).expect("sync");
        let err = local_backup_delete_history_item(&root, "../../state")
            .expect_err("path traversal must be refused");
        assert!(err.contains("Invalid backup history identifier"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn malformed_document_json_is_refused() {
        let dir = temp_dir("bad-json");
        let root = dir.to_string_lossy().to_string();
        let err = local_backup_sync(&root, "{not json").expect_err("refuse");
        assert!(err.contains("Invalid backup document JSON"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_empty_directory_path_is_refused() {
        assert!(local_backup_read_state("   ").is_err());
        assert!(normalize_backup_directory_path("").is_err());
    }

    #[test]
    fn the_default_directory_lives_under_app_data() {
        let data = temp_dir("default");
        let ctx = HeadlessContext::new(data.clone(), temp_dir("default-logs"));
        let directory = local_backup_default_directory(&ctx).expect("default");
        assert_eq!(PathBuf::from(&directory), data.join("backups"));
        assert!(Path::new(&directory).is_dir());
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn the_snapshot_updated_at_is_a_fallback() {
        assert_eq!(
            extract_backup_document_updated_at(r#"{"snapshot":{"updatedAt":"2026-03-03T00:00:00Z"}}"#)
                .as_deref(),
            Some("2026-03-03T00:00:00Z")
        );
        assert_eq!(extract_backup_document_updated_at("{}"), None);
    }
}
