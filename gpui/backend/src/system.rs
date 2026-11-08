//! Shell integration — engine version, launch arguments, file manager, external
//! processes, and the two window commands.
//!
//! Ported from `src-tauri/src/main.rs` lines 3342-3670 and 3895-4040.
//!
//! The window commands (`focus_main_window_command`, `reveal_main_window_command`)
//! and `notify_launch_scene_handoff` are the only entry points here that need an
//! [`AppContext`]; they keep the Tauri behaviour exactly — show before focusing, do
//! not call `set_focus` on the reveal path (Windows' focus-stealing prevention
//! plays an error sound when `SetForegroundWindow` comes from a process that does
//! not own the foreground), and close the splash after the main window is visible so
//! no desktop frame shows between them.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::context::AppContext;
use crate::{BackendResult, BUILTIN_PLUGIN_SCENE_FILE_EXTENSIONS};

/// A file the user chose (or the shell handed over at launch).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickedOpenFile {
    pub path: String,
    pub name: String,
}

/// Payload of the `dragonfruit://scene-file-handoff` event.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneFileHandoffPayload {
    pub paths: Vec<String>,
    pub source: String,
}

/// `get_slicer_engine_version`: the engine's own version string, so the UI can
/// show which slicer produced an artifact.
pub fn get_slicer_engine_version() -> &'static str {
    dragonfruit_slicing_engine::ENGINE_VERSION
}

/// Whether `path` names a scene file: `.voxl`, or any format a built-in fileType
/// plugin declares.
pub fn is_scene_file_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let normalized = ext.trim().trim_start_matches('.');
            normalized.eq_ignore_ascii_case("voxl")
                || BUILTIN_PLUGIN_SCENE_FILE_EXTENSIONS
                    .iter()
                    .any(|&s| normalized.eq_ignore_ascii_case(s))
        })
        .unwrap_or(false)
}

/// Scene files among `args`, skipping `argv[0]`.
///
/// This is how a shell "open with" arrives: the OS appends the chosen files to the
/// command line. Anything that is not an existing scene file is ignored rather than
/// reported — launchers add their own switches.
pub fn collect_scene_file_paths_from_args(args: &[String]) -> Vec<String> {
    let mut files: Vec<String> = Vec::new();
    for arg in args.iter().skip(1) {
        let trimmed = arg.trim();
        if trimmed.is_empty() {
            continue;
        }
        let path = PathBuf::from(trimmed);
        if !path.is_file() {
            continue;
        }
        if !is_scene_file_path(&path) {
            continue;
        }
        files.push(path.to_string_lossy().to_string());
    }
    files
}

/// `get_launch_scene_files`: the scene files this process was launched with.
pub fn get_launch_scene_files() -> Vec<PickedOpenFile> {
    collect_scene_file_paths_from_args(&std::env::args().collect::<Vec<_>>())
        .into_iter()
        .map(|path_text| {
            let path = PathBuf::from(&path_text);
            PickedOpenFile {
                name: path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("file")
                    .to_string(),
                path: path_text,
            }
        })
        .collect()
}

/// `notify_launch_scene_handoff`: push the launch files at the UI.
///
/// A second instance is the common case (the shell's single-instance plugin
/// forwarded its argv), so this is a command the UI calls once at startup rather
/// than something the backend decides on its own.
pub fn notify_launch_scene_handoff(ctx: &impl AppContext) -> BackendResult<()> {
    let args = std::env::args().collect::<Vec<_>>();
    emit_scene_file_handoff(ctx, &args, "primary-launch");
    Ok(())
}

/// Emit the handoff event for the scene files among `args`, if there are any.
pub fn emit_scene_file_handoff(ctx: &impl AppContext, args: &[String], source: &str) {
    let paths = collect_scene_file_paths_from_args(args);
    if paths.is_empty() {
        return;
    }
    ctx.emit(
        "dragonfruit://scene-file-handoff",
        serde_json::to_value(SceneFileHandoffPayload {
            paths,
            source: source.to_string(),
        })
        .unwrap_or(serde_json::Value::Null),
    );
}

/// Show, un-minimize and focus the main window.
fn focus_main_window(ctx: &impl AppContext) {
    let window = ctx.window();
    if !window.is_visible() {
        window.show();
    }
    if window.is_minimized() {
        window.unminimize();
    }
    if !window.is_focused() {
        window.set_focus();
    }
}

/// `focus_main_window_command`.
pub fn focus_main_window_command(ctx: &impl AppContext) -> BackendResult<()> {
    focus_main_window(ctx);
    Ok(())
}

/// `reveal_main_window_command`: show the main window **without** taking focus, and
/// close the splash.
///
/// Used at startup to avoid Windows' focus-stealing prevention, which plays an error
/// sound when `SetForegroundWindow` comes from a process that does not currently own
/// the foreground. Showing before closing the splash leaves no frame of desktop
/// between them; geometry was applied at window creation, so showing causes no
/// resize flash.
pub fn reveal_main_window_command(ctx: &impl AppContext) -> BackendResult<()> {
    let window = ctx.window();
    if !window.is_visible() {
        window.show();
    }
    window.close_splash();
    Ok(())
}

/// `reveal_in_file_manager`: open the folder containing `path`, with the file
/// selected where the platform allows it.
pub fn reveal_in_file_manager(path: &str) -> BackendResult<()> {
    let path = path.trim().to_string();
    if path.is_empty() {
        return Err("Path is empty".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", &path])
            .spawn()
            .map_err(|e| format!("Failed to open Explorer: {e}"))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| format!("Failed to reveal in Finder: {e}"))?;
    }

    #[cfg(target_os = "linux")]
    {
        let p = Path::new(&path);
        let dir = p.parent().unwrap_or(p);
        std::process::Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map_err(|e| format!("Failed to open file manager: {e}"))?;
    }

    Ok(())
}

/// `open_external_url`: hand a URL to the default browser.
pub fn open_external_url(url: &str) -> BackendResult<()> {
    let url = url.trim().to_string();
    if url.is_empty() {
        return Err("URL is empty".to_string());
    }

    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("cmd")
        .args(["/c", "start", &url])
        .spawn();
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(&url).spawn();
    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("xdg-open").arg(&url).spawn();

    result.map_err(|e| format!("Failed to open URL in browser: {e}"))?;
    Ok(())
}

/// `launch_external_process`: start `exe_path`, passing `file_arg`.
///
/// A macOS `.app` bundle is a directory and has to be started through `open -a`. The
/// file goes in as a *command-line argument*, not as a document to open: UVtools
/// declares no document types, so it ignores the open-document event that
/// `open -a <app> <file>` sends. `-n` starts a new instance, since one already
/// running would never see the arguments.
pub fn launch_external_process(exe_path: &str, file_arg: &str) -> BackendResult<()> {
    let exe_path = exe_path.trim().to_string();
    let file_arg = file_arg.trim().to_string();

    if exe_path.is_empty() {
        return Err("Executable path is empty".to_string());
    }

    #[cfg(target_os = "macos")]
    if Path::new(&exe_path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("app"))
    {
        let mut command = std::process::Command::new("open");
        command.arg("-n").arg("-a").arg(&exe_path);
        if !file_arg.is_empty() {
            command.arg("--args").arg(&file_arg);
        }
        command
            .spawn()
            .map_err(|e| format!("Failed to launch external process: {e}"))?;
        return Ok(());
    }

    std::process::Command::new(&exe_path)
        .arg(&file_arg)
        .spawn()
        .map_err(|e| format!("Failed to launch external process: {e}"))?;
    Ok(())
}

/// Executable names to look for on PATH.
///
/// Everywhere but Windows, UVTools ships the same plain binary — `UVtools` inside
/// the macOS bundle, `UVtools` in the Linux zip — that users sometimes symlink onto
/// PATH in lowercase.
#[cfg(target_os = "windows")]
const UVTOOLS_PATH_EXECUTABLES: &[&str] = &["UVTools.exe"];
#[cfg(not(target_os = "windows"))]
const UVTOOLS_PATH_EXECUTABLES: &[&str] = &["UVtools", "uvtools"];

/// Expand a leading `~/` against HOME. Candidate paths come from the frontend,
/// which has no way to resolve the home directory itself.
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}

/// A candidate counts as UVTools when it is an executable file or, on macOS, an
/// `.app` bundle — which is a directory, so `is_file()` alone rejects it.
pub fn is_uvtools_install(path: &Path) -> bool {
    if path.is_file() {
        return true;
    }
    cfg!(target_os = "macos")
        && path.is_dir()
        && path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("app"))
}

/// `discover_uvtools_path`: the first candidate the user's settings name, else the
/// first `UVtools` on PATH (for installs outside a standard location).
pub fn discover_uvtools_path(candidates: &[String]) -> Option<String> {
    for candidate in candidates {
        let p = expand_tilde(candidate);
        if is_uvtools_install(&p) {
            return Some(p.to_string_lossy().to_string());
        }
    }

    if let Ok(path_env) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_env) {
            for name in UVTOOLS_PATH_EXECUTABLES {
                let exec_candidate = dir.join(name);
                if exec_candidate.is_file() {
                    return Some(exec_candidate.to_string_lossy().to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("df-system-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    #[test]
    fn scene_file_detection_covers_voxl_and_plugin_formats() {
        assert!(is_scene_file_path(Path::new("model.voxl")));
        assert!(is_scene_file_path(Path::new("model.VOXL")));
        assert!(is_scene_file_path(Path::new("model.chitubox")));
        assert!(is_scene_file_path(Path::new("model.lyS")));
        assert!(!is_scene_file_path(Path::new("model.stl")));
        assert!(!is_scene_file_path(Path::new("model")));
    }

    #[test]
    fn launch_arguments_skip_argv_zero_and_non_scene_files() {
        let dir = temp_dir("args");
        let scene = dir.join("part.voxl");
        let mesh = dir.join("part.stl");
        std::fs::write(&scene, b"VOXL").expect("scene");
        std::fs::write(&mesh, b"solid").expect("mesh");
        let absent = dir.join("gone.voxl");

        let args: Vec<String> = [
            "dragonfruit.exe".to_string(),
            scene.to_string_lossy().to_string(),
            mesh.to_string_lossy().to_string(),
            absent.to_string_lossy().to_string(),
            "  ".to_string(),
        ]
        .to_vec();

        let found = collect_scene_file_paths_from_args(&args);
        assert_eq!(found, vec![scene.to_string_lossy().to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_empty_path_is_refused_by_the_shell_helpers() {
        assert!(reveal_in_file_manager("  ").is_err());
        assert!(open_external_url("").is_err());
        assert!(launch_external_process("", "x").is_err());
    }

    #[test]
    fn uvtools_is_found_at_a_candidate_path_before_path_lookup() {
        let dir = temp_dir("uvtools");
        let exe = dir.join(if cfg!(windows) { "UVTools.exe" } else { "UVtools" });
        std::fs::write(&exe, b"#!/bin/sh\n").expect("exe");

        let found = discover_uvtools_path(&[exe.to_string_lossy().to_string()]);
        assert_eq!(found, Some(exe.to_string_lossy().to_string()));

        let absent = dir.join("nope").to_string_lossy().to_string();
        // A candidate that does not exist falls through to PATH; the answer may be
        // Some (a machine with UVtools installed) or None, but never the bad match.
        assert_ne!(discover_uvtools_path(&[absent.clone()]), Some(absent));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tilde_expansion_uses_home_when_it_is_set() {
        // The test machine may not have HOME (Windows uses USERPROFILE); both
        // branches must be total.
        let expanded = expand_tilde("~/uvtools");
        assert!(expanded.to_string_lossy().ends_with("uvtools"));
        assert_eq!(expand_tilde("/absolute/uvtools"), PathBuf::from("/absolute/uvtools"));
    }
}
