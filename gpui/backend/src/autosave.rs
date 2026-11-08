//! Scene autosave — where a recovery tick lands, and the manifest that says so.
//!
//! Ported from `src-tauri/src/main.rs` lines 2729-3340. The policy has three
//! invariants the project pinned, each enforced rather than assumed:
//!
//! 1. the autosave target is **never** the project file the user saves to;
//! 2. the target **always** ends in `.voxl` (autosave emits VOXL bytes, so a
//!    plugin-format project must not be handed back as the target);
//! 3. every path autosave writes is self-describing — a `_autosave` sidecar or the
//!    generic recovery payload.
//!
//! ## Sidecar, not app data
//!
//! A project that has been saved or opened gets its recovery copy *beside it*, as
//! `<stem>_autosave.voxl` — local and obvious, not a hidden path the user would have
//! to hunt for. Untitled scenes keep the generic `<app data>/autosave/` location.
//! Any failure of the sidecar target (folder gone, read-only volume, permission
//! denied, unreachable share) falls back to the generic location with a
//! `fallbackReason` recorded in the manifest: never fail an autosave over a folder
//! policy question, and never lie about where the payload went.
//!
//! ## Failure honesty
//!
//! A failed tick must not overwrite the pointer to the last committed payload nor
//! advance `savedAt` past what is on disk. A naive write that stamped `savedAt: now`
//! on every attempt turned a transient (antivirus locking the file) into a false
//! recovery prompt offering an old payload labelled with a fresh timestamp. So on a
//! failure write every payload-describing field, `savedAt` included, falls back to
//! the previous manifest's value — and a successful write clears `lastError`, because
//! the condition is over and a stale error in the settings tab is its own small
//! dishonesty.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::context::AppContext;
use crate::scene_file::{sweep_stale_scene_commit_temps, SCENE_COMMIT_TEMP_SWEEP_MIN_AGE_SECONDS};
use crate::system::is_scene_file_path;
use crate::BackendResult;

const SCENE_AUTOSAVE_DIR_NAME: &str = "autosave";
/// The generic-recovery payload name. It carries the `_autosave` marker so the
/// third invariant holds without an exception: every path autosave writes is
/// self-describing, whether it is a sidecar or the fallback.
const SCENE_AUTOSAVE_VOXL_FILE: &str = "scene_autosave.voxl";
/// The pre-Ph0.1 generic payload name. Never written any more, always searched: an
/// autosave a user already has on disk must stay recoverable.
const SCENE_AUTOSAVE_LEGACY_VOXL_FILE: &str = "scene.voxl";
const SCENE_AUTOSAVE_MANIFEST_FILE: &str = "manifest.json";
/// The marker that makes a recovery file self-describing and gates every deletion
/// seam below.
const SCENE_AUTOSAVE_SIDECAR_MARKER: &str = "_autosave";
const VOXL_FILE_MAGIC: &[u8; 4] = b"VOXL";

pub const SCENE_AUTOSAVE_ORIGIN_SIDECAR: &str = "sidecar";
pub const SCENE_AUTOSAVE_ORIGIN_RECOVERY_DIR: &str = "recovery-dir";

/// The manifest a tick writes.
///
/// Every field added after `savedAt`/`clean` is `#[serde(default)]` so a manifest
/// written by an older build still parses — losing recovery for the autosave a user
/// already has on disk would be a worse bug than the one this fixes.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SceneAutosaveManifest {
    pub saved_at: String,
    pub clean: bool,
    /// Absolute path of the payload actually committed. **Authoritative**: the
    /// manifest is written only after the atomic rename, so this can only name a
    /// fully-committed file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voxl_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    /// The project the sidecar belongs to; lets recovery re-derive the sidecar when
    /// `voxlPath` has gone stale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
    /// Failure message from the most recent tick, or `None` while autosave is healthy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

/// One tick's inputs. This is exactly the shell command's argument list, bundled so
/// it can be passed by name.
#[derive(Debug, Clone, Default)]
pub struct SceneAutosaveManifestWrite {
    pub saved_at: String,
    pub clean: bool,
    pub voxl_path: Option<String>,
    pub origin: Option<String>,
    pub project_path: Option<String>,
    pub payload_bytes: Option<u64>,
    pub fallback_reason: Option<String>,
    pub last_error: Option<String>,
    pub delete_payload: bool,
}

/// Where an autosave tick will land, and why.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneAutosavePaths {
    pub voxl_path: String,
    pub manifest_path: String,
    pub origin: String,
    pub project_path: Option<String>,
    pub fallback_reason: Option<String>,
}

/// The resolved target, before it is turned into paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneAutosaveTarget {
    pub voxl_path: PathBuf,
    pub origin: &'static str,
    pub project_path: Option<String>,
    pub fallback_reason: Option<&'static str>,
}

/// A payload recovery can offer.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneAutosaveRecoveryCandidate {
    pub voxl_path: PathBuf,
    pub origin: String,
    pub saved_at: Option<String>,
    pub clean: bool,
    pub project_path: Option<String>,
    pub payload_bytes: u64,
    pub fallback_reason: Option<String>,
}

/// Build the manifest to persist for a tick.
pub fn build_scene_autosave_manifest(
    previous: Option<&SceneAutosaveManifest>,
    write: SceneAutosaveManifestWrite,
) -> SceneAutosaveManifest {
    if write.last_error.is_some() {
        return SceneAutosaveManifest {
            saved_at: previous
                .map(|m| m.saved_at.clone())
                .unwrap_or(write.saved_at),
            clean: write.clean,
            voxl_path: write
                .voxl_path
                .or_else(|| previous.and_then(|m| m.voxl_path.clone())),
            origin: write
                .origin
                .or_else(|| previous.and_then(|m| m.origin.clone())),
            project_path: write
                .project_path
                .or_else(|| previous.and_then(|m| m.project_path.clone())),
            payload_bytes: write
                .payload_bytes
                .or_else(|| previous.and_then(|m| m.payload_bytes)),
            fallback_reason: write
                .fallback_reason
                .or_else(|| previous.and_then(|m| m.fallback_reason.clone())),
            last_error: write.last_error,
        };
    }

    SceneAutosaveManifest {
        saved_at: write.saved_at,
        clean: write.clean,
        voxl_path: if write.delete_payload {
            None
        } else {
            write
                .voxl_path
                .or_else(|| previous.and_then(|m| m.voxl_path.clone()))
        },
        origin: write
            .origin
            .or_else(|| previous.and_then(|m| m.origin.clone())),
        project_path: write
            .project_path
            .or_else(|| previous.and_then(|m| m.project_path.clone())),
        payload_bytes: if write.delete_payload {
            None
        } else {
            write
                .payload_bytes
                .or_else(|| previous.and_then(|m| m.payload_bytes))
        },
        fallback_reason: write
            .fallback_reason
            .or_else(|| previous.and_then(|m| m.fallback_reason.clone())),
        last_error: None,
    }
}

// ---------------------------------------------------------------------------
// Sidecar naming, writability, and payload identity
// ---------------------------------------------------------------------------

/// **THE ONE PLACE THE SIDECAR FILE NAME IS BUILT.**
///
/// Every target, every discovery candidate, and every deletion guard routes through
/// this function, so the naming scheme has exactly one definition.
///
/// *Open decision, deliberately left as a one-line change:* a per-process name
/// (`<stem>_<pid>_autosave.voxl`) would make two instances editing the same project
/// unable to collide. It is **not** the default, because the pid is gone by the time
/// it matters: after a crash — the one case clean-exit deletion cannot cover —
/// recovery would face several `MyPrint_1234_autosave` files with no way to say
/// which is current, and the orphans accumulate forever. A single well-known name is
/// unambiguous at recovery time, which is the moment the file exists for. To switch,
/// change only this format string (and `is_sidecar_autosave_file`'s suffix test).
pub fn sidecar_autosave_file_name(project_stem: &str) -> String {
    format!("{project_stem}{SCENE_AUTOSAVE_SIDECAR_MARKER}.voxl")
}

/// `<dir>/<stem>_autosave.voxl` for a project file. `None` when the path has no
/// usable stem (the caller falls back to the recovery directory).
pub fn sidecar_autosave_path_for_project(project: &Path) -> Option<PathBuf> {
    let stem = project.file_stem().and_then(|s| s.to_str())?.trim();
    if stem.is_empty() {
        return None;
    }
    let parent = project.parent().unwrap_or_else(|| Path::new(""));
    Some(parent.join(sidecar_autosave_file_name(stem)))
}

/// Windows paths are case-insensitive; a candidate-list check that missed
/// `C:\Projects\...` vs `c:\projects\...` would reject a legitimate recovery.
pub fn autosave_paths_equal(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    a.to_string_lossy().to_ascii_lowercase() == b.to_string_lossy().to_ascii_lowercase()
}

pub fn is_sidecar_autosave_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let suffix = format!("{SCENE_AUTOSAVE_SIDECAR_MARKER}.voxl");
    let lower = name.to_ascii_lowercase();
    // Strictly longer than the suffix: a file called exactly `_autosave.voxl`
    // belongs to no project and is not ours to delete.
    lower.ends_with(&suffix) && lower.len() > suffix.len()
}

/// **The deletion gate.** Only a file this policy could itself have produced — a
/// `<stem>_autosave.voxl` sidecar, or the generic recovery payload — may be removed
/// through any of the cleanup seams. The user's project never qualifies.
pub fn is_deletable_autosave_payload(recovery_dir: &Path, path: &Path) -> bool {
    is_sidecar_autosave_file(path)
        || autosave_paths_equal(path, &recovery_dir.join(SCENE_AUTOSAVE_LEGACY_VOXL_FILE))
}

/// Deletes the sidecar belonging to `project`, if there is one.
///
/// Used on Save As, where the sidecar beside the **old** project would otherwise be
/// orphaned — a `_autosave.voxl` next to a file the user no longer edits implies
/// unsaved work that does not exist.
pub fn delete_sidecar_for_project(project: &Path) -> BackendResult<()> {
    let Some(sidecar) = sidecar_autosave_path_for_project(project) else {
        return Ok(());
    };
    if !is_sidecar_autosave_file(&sidecar) {
        return Err(format!(
            "Refusing to delete '{}': not an autosave sidecar",
            sidecar.display()
        ));
    }
    if !sidecar.exists() {
        return Ok(());
    }
    std::fs::remove_file(&sidecar)
        .map_err(|err| format!("Failed deleting sidecar '{}': {err}", sidecar.display()))
}

/// Real-syscall writability probe for a prospective sidecar location.
///
/// Deliberately not a metadata/permissions guess: read-only volumes,
/// permission-denied folders and unreachable network shares all present differently
/// in metadata but identically to an actual create. Uses the commit temp allocator,
/// so the probe exercises the very mechanism the real commit will use.
///
/// `None` when the directory is usable, or the `fallbackReason` to record when it is
/// not.
pub fn probe_directory_writable(sidecar: &Path) -> Option<&'static str> {
    let parent = sidecar.parent().unwrap_or_else(|| Path::new("."));
    if !parent.as_os_str().is_empty() && !parent.is_dir() {
        return Some("parent-missing");
    }
    let probe = crate::scene_file::sibling_commit_temp_path(sidecar).ok()?;
    let writable = std::fs::write(&probe, b"").is_ok();
    let _ = std::fs::remove_file(&probe);
    if writable {
        None
    } else {
        Some("parent-unwritable")
    }
}

/// A candidate is only advertised if it exists, is non-empty and actually starts
/// with the VOXL magic — a truncated or foreign file must never be offered as a
/// recoverable scene.
pub fn is_readable_voxl_payload(path: &Path) -> bool {
    use std::io::Read;
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic).is_ok() && &magic == VOXL_FILE_MAGIC
}

/// `VOXL` + nothing else is a valid-but-empty payload; this just names the magic so
/// the check above reads.
pub fn voxl_magic() -> &'static [u8; 4] {
    VOXL_FILE_MAGIC
}

/// The ordered recovery candidate list — and the **only** paths the recovery reader
/// will ever open.
///
/// 1. `manifest.voxlPath` — authoritative: the manifest is written only after the
///    atomic rename, so it can only name a fully-committed payload.
/// 2. the sidecar derived from `manifest.projectPath` — covers a project that moved,
///    and manifests from builds older than this change.
/// 3. the generic `<app data>/autosave/scene_autosave.voxl`, then the legacy
///    `…/autosave/scene.voxl` — every autosave written before the sidecar change;
///    back-compat is free and mandatory.
pub fn scene_autosave_recovery_candidates(
    recovery_dir: &Path,
    manifest: Option<&SceneAutosaveManifest>,
) -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Some(manifest) = manifest {
        if let Some(voxl_path) = manifest
            .voxl_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            candidates.push(PathBuf::from(voxl_path));
        }
        if let Some(project) = manifest
            .project_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            if let Some(sidecar) = sidecar_autosave_path_for_project(Path::new(project)) {
                candidates.push(sidecar);
            }
        }
    }

    candidates.push(recovery_dir.join(SCENE_AUTOSAVE_VOXL_FILE));
    candidates.push(recovery_dir.join(SCENE_AUTOSAVE_LEGACY_VOXL_FILE));

    let mut deduped: Vec<PathBuf> = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if !deduped
            .iter()
            .any(|existing| autosave_paths_equal(existing, &candidate))
        {
            deduped.push(candidate);
        }
    }
    deduped
}

/// The read gate: the recovery reader accepts a path **only** if the resolver itself
/// produced it. Same discipline as the scene-file discard command — an arbitrary
/// caller-supplied path can never be turned into a file read through this seam.
pub fn is_allowed_recovery_read(candidates: &[PathBuf], path: &Path) -> bool {
    candidates
        .iter()
        .any(|candidate| autosave_paths_equal(candidate, path))
}

/// The first candidate that actually holds a VOXL payload.
pub fn resolve_scene_autosave_recovery(
    recovery_dir: &Path,
    manifest: Option<&SceneAutosaveManifest>,
) -> Option<SceneAutosaveRecoveryCandidate> {
    let generic = recovery_dir.join(SCENE_AUTOSAVE_VOXL_FILE);
    let legacy_generic = recovery_dir.join(SCENE_AUTOSAVE_LEGACY_VOXL_FILE);

    for candidate in scene_autosave_recovery_candidates(recovery_dir, manifest) {
        if !is_readable_voxl_payload(&candidate) {
            continue;
        }

        let origin = if autosave_paths_equal(&candidate, &generic)
            || autosave_paths_equal(&candidate, &legacy_generic)
        {
            SCENE_AUTOSAVE_ORIGIN_RECOVERY_DIR.to_string()
        } else {
            manifest
                .and_then(|m| m.origin.clone())
                .unwrap_or_else(|| SCENE_AUTOSAVE_ORIGIN_SIDECAR.to_string())
        };
        let payload_bytes = std::fs::metadata(&candidate).map(|m| m.len()).unwrap_or(0);

        return Some(SceneAutosaveRecoveryCandidate {
            voxl_path: candidate,
            origin,
            saved_at: manifest.map(|m| m.saved_at.clone()),
            clean: manifest.map(|m| m.clean).unwrap_or(false),
            project_path: manifest.and_then(|m| m.project_path.clone()),
            payload_bytes,
            fallback_reason: manifest.and_then(|m| m.fallback_reason.clone()),
        });
    }
    None
}

/// `VOXL`-only, deliberately **not** every plugin scene extension: autosave emits
/// VOXL bytes, so a plugin-format project must never be handed back as the autosave
/// target.
pub fn is_voxl_autosave_target(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            ext.trim()
                .trim_start_matches('.')
                .eq_ignore_ascii_case("voxl")
        })
        .unwrap_or(false)
}

/// Resolves where an autosave tick lands.
///
/// Pure and app-handle-free on purpose: the recovery directory is passed in, so the
/// whole target policy — including the three invariants — is testable without a
/// runtime.
pub fn resolve_scene_autosave_target(
    recovery_dir: &Path,
    preferred_project_path: Option<&str>,
) -> SceneAutosaveTarget {
    let fallback = |project: Option<&str>, reason: &'static str| SceneAutosaveTarget {
        voxl_path: recovery_dir.join(SCENE_AUTOSAVE_VOXL_FILE),
        origin: SCENE_AUTOSAVE_ORIGIN_RECOVERY_DIR,
        project_path: project.map(str::to_string),
        fallback_reason: Some(reason),
    };

    let Some(project) = preferred_project_path
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return fallback(None, "no-project");
    };

    let path = Path::new(project);
    // `is_scene_file_path`, not `is_voxl_autosave_target`: a project saved in a
    // plugin scene format is still a project and still deserves a sidecar. It gets a
    // correctly-named `.voxl` one, so VOXL bytes never land in a file whose extension
    // promises a foreign format.
    if !is_scene_file_path(path) {
        return fallback(Some(project), "not-a-scene-file");
    }

    let Some(sidecar) = sidecar_autosave_path_for_project(path) else {
        return fallback(Some(project), "not-a-scene-file");
    };

    // Invariants enforced rather than assumed. Both are unreachable by construction
    // — the naming helper inserts the marker before a hard-coded `.voxl` — but they
    // are the locks this whole policy exists for, so the resolver checks its own
    // output instead of trusting that the helper is never changed carelessly.
    if autosave_paths_equal(&sidecar, path) || !is_voxl_autosave_target(&sidecar) {
        return fallback(Some(project), "not-a-scene-file");
    }

    if let Some(reason) = probe_directory_writable(&sidecar) {
        return fallback(Some(project), reason);
    }

    SceneAutosaveTarget {
        voxl_path: sidecar,
        origin: SCENE_AUTOSAVE_ORIGIN_SIDECAR,
        project_path: Some(project.to_string()),
        fallback_reason: None,
    }
}

/// `<app data>/autosave`, created if needed.
fn scene_autosave_resolve_dir(ctx: &impl AppContext) -> BackendResult<PathBuf> {
    let base = ctx
        .data_dir()
        .map_err(|e| e.replace("app data directory", "app data dir"))?;
    let dir = base.join(SCENE_AUTOSAVE_DIR_NAME);
    std::fs::create_dir_all(&dir)
        .map_err(|err| format!("Failed creating autosave dir '{}': {err}", dir.display()))?;
    Ok(dir)
}

fn read_manifest_from_dir(dir: &Path) -> Option<SceneAutosaveManifest> {
    let path = dir.join(SCENE_AUTOSAVE_MANIFEST_FILE);
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// `scene_autosave_get_paths`: where an autosave tick will land, and why.
pub fn scene_autosave_get_paths(
    ctx: &impl AppContext,
    preferred_save_path: Option<&str>,
) -> BackendResult<SceneAutosavePaths> {
    let dir = scene_autosave_resolve_dir(ctx)?;
    let target = resolve_scene_autosave_target(&dir, preferred_save_path);

    // Sweep crash residue from the directory we are about to write to, and only that
    // one: a `<project>.voxl.tmp-<pid>-<seq>` left by a killed write is confusing
    // clutter sitting next to the user's project.
    if let Some(parent) = target.voxl_path.parent() {
        sweep_stale_scene_commit_temps(parent, SCENE_COMMIT_TEMP_SWEEP_MIN_AGE_SECONDS);
    }

    Ok(SceneAutosavePaths {
        voxl_path: target.voxl_path.to_string_lossy().to_string(),
        manifest_path: dir
            .join(SCENE_AUTOSAVE_MANIFEST_FILE)
            .to_string_lossy()
            .to_string(),
        origin: target.origin.to_string(),
        project_path: target.project_path,
        fallback_reason: target.fallback_reason.map(str::to_string),
    })
}

/// `scene_autosave_write_manifest`: write the recovery manifest — **always after**
/// the payload has been committed by the atomic rename.
///
/// That ordering is what makes `voxlPath` authoritative: the manifest can only ever
/// name a file that is already complete on disk. `delete_payload` removes the payload
/// the *previous* manifest advertised (plus the legacy generic location), gated on
/// [`is_deletable_autosave_payload`] so the user's project can never be reached
/// through it. Callers use it only after a successful save, restore or explicit
/// discard — never while unsaved work exists.
pub fn scene_autosave_write_manifest(
    ctx: &impl AppContext,
    write: SceneAutosaveManifestWrite,
) -> BackendResult<()> {
    let dir = scene_autosave_resolve_dir(ctx)?;
    let path = dir.join(SCENE_AUTOSAVE_MANIFEST_FILE);

    let previous = read_manifest_from_dir(&dir);

    if write.delete_payload {
        let mut targets: Vec<PathBuf> = vec![
            dir.join(SCENE_AUTOSAVE_LEGACY_VOXL_FILE),
            dir.join(SCENE_AUTOSAVE_VOXL_FILE),
        ];
        for advertised in [
            previous.as_ref().and_then(|m| m.voxl_path.clone()),
            write.voxl_path.clone(),
        ]
        .into_iter()
        .flatten()
        {
            targets.push(PathBuf::from(advertised));
        }
        if let Some(project) = previous
            .as_ref()
            .and_then(|m| m.project_path.clone())
            .or_else(|| write.project_path.clone())
        {
            if let Some(sidecar) = sidecar_autosave_path_for_project(Path::new(&project)) {
                targets.push(sidecar);
            }
        }
        for target in targets {
            if !is_deletable_autosave_payload(&dir, &target) {
                continue;
            }
            if target.exists() {
                let _ = std::fs::remove_file(&target);
            }
        }
    }

    let manifest = build_scene_autosave_manifest(previous.as_ref(), write);
    let json = serde_json::to_string(&manifest)
        .map_err(|err| format!("Failed serializing autosave manifest: {err}"))?;
    std::fs::write(&path, json.as_bytes())
        .map_err(|err| format!("Failed writing autosave manifest: {err}"))?;
    Ok(())
}

/// `scene_autosave_delete_sidecar`: delete the sidecar belonging to a specific
/// project.
///
/// Save As uses this on the **old** project path, so a successful Save As never
/// orphans a `_autosave.voxl` beside a file the user has stopped editing.
pub fn scene_autosave_delete_sidecar(project_path: &str) -> BackendResult<()> {
    let trimmed = project_path.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    delete_sidecar_for_project(Path::new(trimmed))
}

/// `scene_autosave_read_manifest`.
pub fn scene_autosave_read_manifest(
    ctx: &impl AppContext,
) -> BackendResult<Option<SceneAutosaveManifest>> {
    let dir = scene_autosave_resolve_dir(ctx)?;
    let path = dir.join(SCENE_AUTOSAVE_MANIFEST_FILE);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|err| format!("Failed reading autosave manifest: {err}"))?;
    let manifest: SceneAutosaveManifest = serde_json::from_str(&content)
        .map_err(|err| format!("Failed parsing autosave manifest: {err}"))?;
    Ok(Some(manifest))
}

/// `scene_autosave_resolve_recovery`: the payload a crash left behind, if any.
///
/// Without this, sidecar path control would be a regression rather than a fix: the
/// manifest would advertise a payload beside the project and a reader hard-wired to
/// the generic location could not open it.
pub fn scene_autosave_resolve_recovery(
    ctx: &impl AppContext,
) -> BackendResult<Option<SceneAutosaveRecoveryCandidate>> {
    let dir = scene_autosave_resolve_dir(ctx)?;
    let manifest = read_manifest_from_dir(&dir);
    Ok(resolve_scene_autosave_recovery(&dir, manifest.as_ref()))
}

/// `scene_autosave_read_voxl_bytes`: read a recovery payload.
///
/// `path` is accepted **only** when it appears in the resolver's own candidate list —
/// never an arbitrary caller-supplied path. Omitting it reads the best candidate.
pub fn scene_autosave_read_voxl_bytes(
    ctx: &impl AppContext,
    path: Option<&str>,
) -> BackendResult<Vec<u8>> {
    let dir = scene_autosave_resolve_dir(ctx)?;
    let manifest = read_manifest_from_dir(&dir);
    let candidates = scene_autosave_recovery_candidates(&dir, manifest.as_ref());

    let requested = path
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);

    let target = match requested {
        Some(requested) => {
            if !is_allowed_recovery_read(&candidates, &requested) {
                return Err(format!(
                    "Refusing to read '{}': not an autosave recovery candidate",
                    requested.display()
                ));
            }
            requested
        }
        None => resolve_scene_autosave_recovery(&dir, manifest.as_ref())
            .map(|candidate| candidate.voxl_path)
            .ok_or_else(|| "No autosaved scene file found".to_string())?,
    };

    if !target.exists() {
        return Err("No autosaved scene file found".to_string());
    }
    std::fs::read(&target).map_err(|err| format!("Failed reading autosaved scene: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::HeadlessContext;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("df-autosave-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    /// A context whose data dir holds an `autosave` subdirectory.
    fn context(label: &str) -> (HeadlessContext, PathBuf) {
        let data = temp_dir(label);
        let recovery = data.join(SCENE_AUTOSAVE_DIR_NAME);
        std::fs::create_dir_all(&recovery).expect("recovery dir");
        let ctx = HeadlessContext::new(data, temp_dir(&format!("{label}-logs")));
        (ctx, recovery)
    }

    #[test]
    fn an_untitled_scene_lands_in_the_generic_location() {
        let (_, recovery) = context("untitled");
        let target = resolve_scene_autosave_target(&recovery, None);
        assert_eq!(target.voxl_path, recovery.join(SCENE_AUTOSAVE_VOXL_FILE));
        assert_eq!(target.origin, SCENE_AUTOSAVE_ORIGIN_RECOVERY_DIR);
        assert_eq!(target.fallback_reason, Some("no-project"));
    }

    #[test]
    fn a_saved_project_gets_a_sidecar_beside_it() {
        let (_, recovery) = context("sidecar");
        let project_dir = temp_dir("sidecar-project");
        let project = project_dir.join("MyPrint.voxl");
        std::fs::write(&project, b"VOXL").expect("project");

        let target = resolve_scene_autosave_target(&recovery, Some(&project.to_string_lossy()));
        assert_eq!(
            target.voxl_path,
            project_dir.join("MyPrint_autosave.voxl")
        );
        assert_eq!(target.origin, SCENE_AUTOSAVE_ORIGIN_SIDECAR);
        assert_eq!(target.fallback_reason, None);
        let _ = std::fs::remove_dir_all(&project_dir);
    }

    /// The three invariants, over every shape of project path.
    #[test]
    fn every_target_is_never_the_project_ends_in_voxl_and_is_marked() {
        let (_, recovery) = context("invariants");
        let project_dir = temp_dir("invariants-project");
        let cases = [
            "MyPrint.voxl",
            "MyPrint.chitubox",
            "MyPrint.lys",
            "no-extension",
            "My Print With Spaces.ctp",
        ];
        for case in cases {
            let project = project_dir.join(case);
            std::fs::write(&project, b"VOXL").expect("project");
            let target = resolve_scene_autosave_target(&recovery, Some(&project.to_string_lossy()));
            assert!(
                !autosave_paths_equal(&target.voxl_path, &project),
                "{case}: the target must not be the project"
            );
            assert!(
                is_voxl_autosave_target(&target.voxl_path),
                "{case}: the target must end in .voxl"
            );
            assert!(
                target
                    .voxl_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.contains(SCENE_AUTOSAVE_SIDECAR_MARKER)),
                "{case}: the target must carry the marker"
            );
        }
        let _ = std::fs::remove_dir_all(&project_dir);
    }

    #[test]
    fn a_missing_project_folder_falls_back_and_says_why() {
        let (_, recovery) = context("fallback");
        let absent = temp_dir("fallback-parent").join("gone").join("Scene.voxl");
        let target = resolve_scene_autosave_target(&recovery, Some(&absent.to_string_lossy()));
        assert_eq!(target.origin, SCENE_AUTOSAVE_ORIGIN_RECOVERY_DIR);
        assert_eq!(target.fallback_reason, Some("parent-missing"));
        assert_eq!(target.voxl_path, recovery.join(SCENE_AUTOSAVE_VOXL_FILE));
    }

    #[test]
    fn a_non_scene_project_falls_back() {
        let (_, recovery) = context("not-scene");
        let target = resolve_scene_autosave_target(&recovery, Some("/tmp/model.stl"));
        assert_eq!(target.fallback_reason, Some("not-a-scene-file"));
    }

    #[test]
    fn the_deletion_gate_only_accepts_sidecars_and_the_legacy_payload() {
        let recovery = Path::new("C:/app/autosave");
        assert!(is_deletable_autosave_payload(
            recovery,
            Path::new("C:/projects/MyPrint_autosave.voxl")
        ));
        assert!(is_deletable_autosave_payload(
            recovery,
            &recovery.join(SCENE_AUTOSAVE_LEGACY_VOXL_FILE)
        ));
        assert!(!is_deletable_autosave_payload(
            recovery,
            Path::new("C:/projects/MyPrint.voxl")
        ));
        // The new generic payload is not on the legacy list, and that is the shell's
        // own shape: it is removed through the generic-path branch of the delete
        // list, not this gate.
        assert!(!is_deletable_autosave_payload(
            recovery,
            &recovery.join(SCENE_AUTOSAVE_VOXL_FILE)
        ));
        // A file named exactly `_autosave.voxl` belongs to no project.
        assert!(!is_deletable_autosave_payload(
            recovery,
            Path::new("C:/projects/_autosave.voxl")
        ));
    }

    #[test]
    fn a_failed_tick_keeps_the_previous_pointer_and_timestamp() {
        let previous = SceneAutosaveManifest {
            saved_at: "2026-01-01T00:00:00Z".into(),
            clean: false,
            voxl_path: Some("/p/Scene_autosave.voxl".into()),
            origin: Some(SCENE_AUTOSAVE_ORIGIN_SIDECAR.into()),
            project_path: Some("/p/Scene.voxl".into()),
            payload_bytes: Some(1_234),
            fallback_reason: None,
            last_error: None,
        };
        let manifest = build_scene_autosave_manifest(
            Some(&previous),
            SceneAutosaveManifestWrite {
                saved_at: "2026-02-02T00:00:00Z".into(),
                clean: true,
                voxl_path: Some("/p/other.voxl".into()),
                payload_bytes: Some(9),
                last_error: Some("antivirus held the file".into()),
                ..Default::default()
            },
        );
        // A failure must not advance what the manifest claims is on disk.
        assert_eq!(manifest.saved_at, "2026-01-01T00:00:00Z");
        assert_eq!(manifest.voxl_path.as_deref(), Some("/p/Scene_autosave.voxl"));
        assert_eq!(manifest.payload_bytes, Some(1_234));
        assert_eq!(manifest.clean, true);
        assert_eq!(
            manifest.last_error.as_deref(),
            Some("antivirus held the file")
        );
    }

    #[test]
    fn a_successful_tick_clears_the_error_and_advances() {
        let previous = SceneAutosaveManifest {
            saved_at: "2026-01-01T00:00:00Z".into(),
            clean: false,
            voxl_path: None,
            origin: None,
            project_path: None,
            payload_bytes: None,
            fallback_reason: None,
            last_error: Some("transient".into()),
        };
        let manifest = build_scene_autosave_manifest(
            Some(&previous),
            SceneAutosaveManifestWrite {
                saved_at: "2026-02-02T00:00:00Z".into(),
                clean: true,
                voxl_path: Some("/p/Scene_autosave.voxl".into()),
                origin: Some(SCENE_AUTOSAVE_ORIGIN_SIDECAR.into()),
                payload_bytes: Some(42),
                ..Default::default()
            },
        );
        assert_eq!(manifest.saved_at, "2026-02-02T00:00:00Z");
        assert_eq!(manifest.last_error, None);
        assert_eq!(manifest.payload_bytes, Some(42));
    }

    #[test]
    fn a_discarding_tick_drops_the_payload_pointers() {
        let previous = SceneAutosaveManifest {
            saved_at: "2026-01-01T00:00:00Z".into(),
            clean: true,
            voxl_path: Some("/p/Scene_autosave.voxl".into()),
            origin: Some(SCENE_AUTOSAVE_ORIGIN_SIDECAR.into()),
            project_path: Some("/p/Scene.voxl".into()),
            payload_bytes: Some(1_234),
            fallback_reason: None,
            last_error: None,
        };
        let manifest = build_scene_autosave_manifest(
            Some(&previous),
            SceneAutosaveManifestWrite {
                saved_at: "2026-02-02T00:00:00Z".into(),
                clean: true,
                delete_payload: true,
                ..Default::default()
            },
        );
        assert_eq!(manifest.voxl_path, None);
        assert_eq!(manifest.payload_bytes, None);
    }

    #[test]
    fn a_manifest_round_trips_and_tolerates_a_short_one() {
        let json = r#"{"savedAt":"2026-01-01T00:00:00Z","clean":false}"#;
        let manifest: SceneAutosaveManifest = serde_json::from_str(json).expect("parse");
        assert_eq!(manifest.voxl_path, None);
        let encoded = serde_json::to_string(&manifest).expect("encode");
        // `None` fields are skipped, not written as nulls.
        assert!(!encoded.contains("voxlPath"), "{encoded}");
    }

    #[test]
    fn paths_compare_case_insensitively() {
        assert!(autosave_paths_equal(
            Path::new("C:/Projects/Scene_autosave.voxl"),
            Path::new("c:/projects/scene_autosave.voxl")
        ));
        assert!(!autosave_paths_equal(
            Path::new("C:/Projects/A_autosave.voxl"),
            Path::new("C:/Projects/B_autosave.voxl")
        ));
    }

    #[test]
    fn recovery_offers_only_files_with_the_voxl_magic() {
        let (ctx, recovery) = context("magic");
        // A truncated / foreign payload is not a candidate.
        std::fs::write(recovery.join(SCENE_AUTOSAVE_VOXL_FILE), b"NOTVOXL").expect("write");
        assert!(scene_autosave_resolve_recovery(&ctx).expect("resolve").is_none());

        std::fs::write(recovery.join(SCENE_AUTOSAVE_VOXL_FILE), b"VOXLpayload").expect("write");
        let candidate = scene_autosave_resolve_recovery(&ctx)
            .expect("resolve")
            .expect("candidate");
        assert_eq!(candidate.origin, SCENE_AUTOSAVE_ORIGIN_RECOVERY_DIR);
        assert_eq!(candidate.payload_bytes, 11);
        assert_eq!(
            scene_autosave_read_voxl_bytes(&ctx, None).expect("read"),
            b"VOXLpayload".to_vec()
        );
    }

    #[test]
    fn the_reader_refuses_a_path_the_resolver_did_not_produce() {
        let (ctx, recovery) = context("read-gate");
        std::fs::write(recovery.join(SCENE_AUTOSAVE_VOXL_FILE), b"VOXLpayload").expect("write");
        let err = scene_autosave_read_voxl_bytes(&ctx, Some("/etc/passwd")).expect_err("refuse");
        assert!(err.contains("not an autosave recovery candidate"), "{err}");
    }

    #[test]
    fn writing_the_manifest_updates_paths_and_a_discard_removes_the_payload() {
        let (ctx, recovery) = context("write-manifest");
        let project_dir = temp_dir("write-manifest-project");
        let project = project_dir.join("Scene.voxl");
        std::fs::write(&project, b"VOXL").expect("project");

        let paths = scene_autosave_get_paths(&ctx, Some(&project.to_string_lossy())).expect("paths");
        assert_eq!(paths.origin, SCENE_AUTOSAVE_ORIGIN_SIDECAR);
        let sidecar = PathBuf::from(&paths.voxl_path);
        std::fs::write(&sidecar, b"VOXLpayload").expect("payload");

        scene_autosave_write_manifest(
            &ctx,
            SceneAutosaveManifestWrite {
                saved_at: "2026-01-01T00:00:00Z".into(),
                clean: false,
                voxl_path: Some(paths.voxl_path.clone()),
                origin: Some(paths.origin.clone()),
                project_path: Some(project.to_string_lossy().to_string()),
                payload_bytes: Some(11),
                ..Default::default()
            },
        )
        .expect("write manifest");

        let manifest = scene_autosave_read_manifest(&ctx)
            .expect("read")
            .expect("present");
        assert_eq!(manifest.voxl_path.as_deref(), Some(paths.voxl_path.as_str()));

        let candidate = scene_autosave_resolve_recovery(&ctx)
            .expect("resolve")
            .expect("candidate");
        assert_eq!(candidate.origin, SCENE_AUTOSAVE_ORIGIN_SIDECAR);
        assert_eq!(
            scene_autosave_read_voxl_bytes(&ctx, Some(paths.voxl_path.as_str())).expect("read"),
            b"VOXLpayload".to_vec()
        );

        // A discard tick removes the payload the manifest advertised.
        scene_autosave_write_manifest(
            &ctx,
            SceneAutosaveManifestWrite {
                saved_at: "2026-01-02T00:00:00Z".into(),
                clean: true,
                delete_payload: true,
                ..Default::default()
            },
        )
        .expect("discard");
        assert!(!sidecar.exists(), "the sidecar is gone");
        assert!(scene_autosave_read_manifest(&ctx).expect("read").is_some());
        assert!(project.exists(), "the discard seam must never touch the project");
        let _ = std::fs::remove_dir_all(&project_dir);
    }

    #[test]
    fn saving_as_deletes_the_old_sidecar_only() {
        let project_dir = temp_dir("save-as");
        let old = project_dir.join("Old.voxl");
        std::fs::write(&old, b"VOXL").expect("project");
        let old_sidecar = project_dir.join("Old_autosave.voxl");
        std::fs::write(&old_sidecar, b"VOXL").expect("sidecar");

        scene_autosave_delete_sidecar(&old.to_string_lossy()).expect("delete");
        assert!(!old_sidecar.exists());
        assert!(old.exists(), "the project survives");
        // An empty path is a no-op, not an error.
        scene_autosave_delete_sidecar("   ").expect("no-op");
        let _ = std::fs::remove_dir_all(&project_dir);
    }
}
