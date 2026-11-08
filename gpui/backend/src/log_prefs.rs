//! Log level preference and the log viewer.
//!
//! Ported from `src-tauri/src/main.rs` lines 4190-4364.
//!
//! The preference file's location is deliberately computed from raw environment
//! variables rather than the app's path resolver: it has to be readable before the
//! app (and its resolver) is initialised, because it decides the level the very
//! first records are written at. The log-file commands resolve `app_log_dir` and so
//! need an [`AppContext`].

use std::path::PathBuf;

use crate::context::AppContext;
use crate::BackendResult;

const APP_ID: &str = "org.openresinalliance.dragonfruit";
const PREF_FILE: &str = "loglevel";
const LOG_FILE_NAME: &str = "dragonfruit.log";
const VALID_LEVELS: &[&str] = &["error", "warn", "info", "debug", "trace"];

/// Where the log-level preference lives.
pub fn resolve_log_level_pref_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        PathBuf::from(appdata).join(APP_ID).join(PREF_FILE)
    }
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_default();
        PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join(APP_ID)
            .join(PREF_FILE)
    }
    #[cfg(target_os = "linux")]
    {
        let base = std::env::var("XDG_DATA_HOME")
            .unwrap_or_else(|_| format!("{}/.local/share", std::env::var("HOME").unwrap_or_default()));
        PathBuf::from(base).join(APP_ID).join(PREF_FILE)
    }
}

/// The persisted level, defaulting to `Info` for a missing or unrecognised value.
pub fn read_log_level_pref() -> log::LevelFilter {
    let content = std::fs::read_to_string(resolve_log_level_pref_path()).unwrap_or_default();
    match content.trim() {
        "error" => log::LevelFilter::Error,
        "warn" => log::LevelFilter::Warn,
        "info" => log::LevelFilter::Info,
        "debug" => log::LevelFilter::Debug,
        "trace" => log::LevelFilter::Trace,
        _ => log::LevelFilter::Info,
    }
}

fn filter_for(level: &str) -> log::LevelFilter {
    match level {
        "error" => log::LevelFilter::Error,
        "warn" => log::LevelFilter::Warn,
        "debug" => log::LevelFilter::Debug,
        "trace" => log::LevelFilter::Trace,
        _ => log::LevelFilter::Info,
    }
}

/// `set_log_level_pref`: validate, apply immediately, then persist.
///
/// `log::set_max_level` is atomic and takes effect for every subsequent record, so
/// no restart is needed.
pub fn set_log_level_pref(level: &str) -> BackendResult<()> {
    let level = level.trim().to_lowercase();
    if !VALID_LEVELS.contains(&level.as_str()) {
        return Err(format!("Invalid log level: {level}"));
    }
    log::set_max_level(filter_for(&level));

    let path = resolve_log_level_pref_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create config dir: {e}"))?;
    }
    std::fs::write(&path, level.as_bytes()).map_err(|e| format!("Failed to write log level pref: {e}"))
}

fn log_file_path(ctx: &impl AppContext) -> BackendResult<PathBuf> {
    Ok(ctx.log_dir().map_err(|e| e.replace("not configured", "Failed to resolve log dir"))?
        .join(LOG_FILE_NAME))
}

/// `read_log_tail`: the last `lines` lines for the live viewer in Settings.
///
/// An empty string when the file does not exist yet — a fresh install has no log,
/// and that is not an error. The count is clamped to 10..10 000 so the viewer cannot
/// ask for the whole file.
pub fn read_log_tail(ctx: &impl AppContext, lines: usize) -> BackendResult<String> {
    let log_file = log_file_path(ctx)?;
    match std::fs::read_to_string(&log_file) {
        Ok(content) => {
            let max = lines.clamp(10, 10_000);
            let all: Vec<&str> = content.lines().collect();
            let tail = if all.len() > max {
                &all[all.len() - max..]
            } else {
                &all[..]
            };
            Ok(tail.join("\n"))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(format!("Failed to read log file: {e}")),
    }
}

/// `open_log_file`: open the log in the OS default text viewer.
pub fn open_log_file(ctx: &impl AppContext) -> BackendResult<()> {
    let log_file = log_file_path(ctx)?;
    if !log_file.exists() {
        return Err("Log file does not exist yet.".to_string());
    }
    let path_str = log_file.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("notepad.exe")
            .arg(&path_str)
            .spawn()
            .map_err(|e| format!("Failed to open log file: {e}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-t", &path_str])
            .spawn()
            .map_err(|e| format!("Failed to open log file: {e}"))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path_str)
            .spawn()
            .map_err(|e| format!("Failed to open log file: {e}"))?;
    }
    Ok(())
}

/// `delete_log_file`: remove the log so it starts fresh on the next write.
///
/// A missing file is success: the caller wanted it gone, and it is.
pub fn delete_log_file(ctx: &impl AppContext) -> BackendResult<()> {
    let log_file = log_file_path(ctx)?;
    match std::fs::remove_file(&log_file) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Failed to delete log file: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::HeadlessContext;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("df-log-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    #[test]
    fn every_level_maps_to_its_filter_and_junk_falls_back_to_info() {
        assert_eq!(filter_for("error"), log::LevelFilter::Error);
        assert_eq!(filter_for("warn"), log::LevelFilter::Warn);
        assert_eq!(filter_for("debug"), log::LevelFilter::Debug);
        assert_eq!(filter_for("trace"), log::LevelFilter::Trace);
        assert_eq!(filter_for("nonsense"), log::LevelFilter::Info);
    }

    #[test]
    fn an_unknown_level_is_refused_before_anything_is_persisted() {
        let err = set_log_level_pref("verbose").expect_err("refuse");
        assert!(err.contains("Invalid log level: verbose"), "{err}");
    }

    #[test]
    fn the_preference_path_ends_with_the_app_id_and_file_name() {
        let path = resolve_log_level_pref_path();
        assert_eq!(path.file_name().and_then(|n| n.to_str()), Some(PREF_FILE));
        assert!(path.to_string_lossy().contains(APP_ID));
    }

    #[test]
    fn a_missing_log_reads_as_empty_and_deletes_as_success() {
        let ctx = HeadlessContext::new(temp_dir("data"), temp_dir("logs"));
        assert_eq!(read_log_tail(&ctx, 100).expect("tail"), "");
        delete_log_file(&ctx).expect("delete absent");
        assert!(open_log_file(&ctx).is_err(), "nothing to open");
    }

    #[test]
    fn the_tail_is_clamped_and_ordered() {
        let logs = temp_dir("tail");
        let ctx = HeadlessContext::new(temp_dir("tail-data"), logs.clone());
        let lines: Vec<String> = (0..50).map(|i| format!("line {i}")).collect();
        std::fs::write(logs.join(LOG_FILE_NAME), lines.join("\n")).expect("write");

        let tail = read_log_tail(&ctx, 1).expect("tail");
        let produced: Vec<&str> = tail.lines().collect();
        // Clamped up to 10, and the last line is still last.
        assert_eq!(produced.len(), 10);
        assert_eq!(produced.last().copied(), Some("line 49"));

        let all = read_log_tail(&ctx, 10_000).expect("all");
        assert_eq!(all.lines().count(), 50);
        delete_log_file(&ctx).expect("delete");
        assert!(!logs.join(LOG_FILE_NAME).exists());
        let _ = std::fs::remove_dir_all(&logs);
    }
}
