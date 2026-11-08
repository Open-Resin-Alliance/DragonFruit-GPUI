//! Sideloaded `.dfplugin` packages — inspect, list, remove, asset reads, and the
//! allow switch.
//!
//! Ported from `src-tauri/src/plugin_sideload.rs` lines 1-560 and 700-825. The
//! package format, trust model and verification live in
//! `dragonfruit-plugin-package`; this module is the host's on-disk store under
//! `<app data>/plugins-sideloaded/` plus the two commands' policy.
//!
//! ## Deferred: install and import
//!
//! `sideloaded_plugin_install` and `sideloaded_plugin_import_file` stay in
//! `src-tauri` for now:
//!
//! * install also calls `plugin_sideload_encoder::register_installed_encoders`,
//!   which registers a package's encoder formats with the slicing engine
//!   (`src-tauri/src/plugin_sideload_encoder.rs`, 781 lines). Installing without
//!   that hook would leave a plugin listed but unusable, so it is deferred with the
//!   registry rather than half-ported.
//! * import runs the plugin in a sidecar process over shared memory
//!   (`src-tauri/src/plugin_sideload_runtime.rs`, 1292 lines).
//!
//! Both are pure file/process work with no context beyond this module's root, so
//! nothing new is needed on [`AppContext`] to finish them.
//!
//! `sideloaded_plugin_pick` is the file dialog, which the GPUI app opens itself.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

use dragonfruit_plugin_package as package;
use serde::{Deserialize, Serialize};

use crate::context::AppContext;
use crate::BackendResult;

const DIR_NAME: &str = "plugins-sideloaded";
const SETTINGS_FILE: &str = "settings.json";
const MANIFEST_FILE: &str = "manifest.json";
const STATE_FILE: &str = "state.json";
const PAYLOAD_DIR: &str = "payload";
/// One preset image; the renderer receives it as a data URL.
const MAX_ASSET_BYTES: u64 = 2_500_000;

/// Serializes installs/removals: two concurrent imports of the same id would
/// otherwise interleave in the same directory. Process-wide on purpose — the
/// directory is process-wide.
fn dir_lock() -> &'static Mutex<()> {
    static LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
    &LOCK
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    #[serde(default)]
    allow_sideloaded_complex_plugins: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            allow_sideloaded_complex_plugins: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstalledState {
    publisher_fingerprint: String,
    imported_at: String,
    version: String,
    sealed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SideloadedInspect {
    pub manifest: package::Manifest,
    pub publisher_fingerprint: String,
    pub sealed: bool,
    /// True when a package with this id is already installed with the same
    /// publisher; an install would replace it.
    pub replaces_existing: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledSideloadedPlugin {
    pub manifest: package::Manifest,
    pub publisher_fingerprint: String,
    pub imported_at: String,
    pub sealed: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SideloadedAsset {
    pub mime: String,
    pub data_url: String,
}

/// The plugin store's root, created if needed.
pub fn root_dir(ctx: &impl AppContext) -> BackendResult<PathBuf> {
    let base = ctx.data_dir()?;
    let dir = base.join(DIR_NAME);
    std::fs::create_dir_all(&dir)
        .map_err(|err| format!("Failed creating '{}': {err}", dir.display()))?;
    Ok(dir)
}

fn read_settings(root: &Path) -> Settings {
    let path = root.join(SETTINGS_FILE);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str::<Settings>(&text).ok())
        .unwrap_or_default()
}

fn write_settings(root: &Path, settings: &Settings) -> BackendResult<()> {
    let path = root.join(SETTINGS_FILE);
    let text = serde_json::to_string_pretty(settings)
        .map_err(|err| format!("Failed serializing plugin settings: {err}"))?;
    write_private_file(&path, text.as_bytes())
}

/// Writes a file that only the current user should read.
///
/// On Unix that means mode 0600; on Windows the inherited ACL of the user's app data
/// dir applies.
fn write_private_file(path: &Path, bytes: &[u8]) -> BackendResult<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|err| format!("Failed writing '{}': {err}", path.display()))?;
    file.write_all(bytes)
        .map_err(|err| format!("Failed writing '{}': {err}", path.display()))?;
    Ok(())
}

/// `current` must be at least `required`, comparing dotted numeric versions
/// component by component and ignoring any pre-release suffix.
pub fn version_at_least(current: &str, required: &str) -> bool {
    fn parts(version: &str) -> Vec<u64> {
        version
            .split('-')
            .next()
            .unwrap_or("")
            .split('.')
            .map(|part| part.trim().parse::<u64>().unwrap_or(0))
            .collect()
    }
    let current = parts(current);
    let required = parts(required);
    for index in 0..current.len().max(required.len()) {
        let left = current.get(index).copied().unwrap_or(0);
        let right = required.get(index).copied().unwrap_or(0);
        if left != right {
            return left > right;
        }
    }
    true
}

fn installed_dir(root: &Path, id: &str) -> PathBuf {
    root.join(id)
}

fn read_installed(dir: &Path) -> Option<InstalledSideloadedPlugin> {
    let manifest_bytes = std::fs::read(dir.join(MANIFEST_FILE)).ok()?;
    let manifest: package::Manifest = serde_json::from_slice(&manifest_bytes).ok()?;
    let state: InstalledState = std::fs::read_to_string(dir.join(STATE_FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())?;
    Some(InstalledSideloadedPlugin {
        manifest,
        publisher_fingerprint: state.publisher_fingerprint,
        imported_at: state.imported_at,
        sealed: state.sealed,
    })
}

/// Every installed package, sorted by display name.
pub fn list_installed(root: &Path) -> Vec<InstalledSideloadedPlugin> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return out;
    };
    for entry in entries.flatten() {
        if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }
        if let Some(plugin) = read_installed(&entry.path()) {
            out.push(plugin);
        }
    }
    out.sort_by(|left, right| left.manifest.name.cmp(&right.manifest.name));
    out
}

/// Read and validate a package without installing it.
///
/// Refuses two things beyond the package's own verification: a package whose id is
/// installed under a *different* publisher key (removing it first is the only way
/// through, which is what stops a package silently taking over another's id), and one
/// that needs a newer app than this build.
///
/// `app_version` is the running application's version. The shell used its own
/// `CARGO_PKG_VERSION`; this crate is not the application, so the caller passes it.
fn inspect_file(
    path: &Path,
    root: &Path,
    app_version: &str,
) -> BackendResult<(Vec<u8>, package::Inspected, bool)> {
    let bytes = std::fs::read(path)
        .map_err(|err| format!("Failed reading '{}': {err}", path.display()))?;
    let inspected = package::inspect(&bytes).map_err(|err| err.to_string())?;

    let existing = installed_dir(root, &inspected.manifest.id);
    let replaces_existing = match read_installed(&existing) {
        Some(installed) => {
            if installed.publisher_fingerprint != inspected.publisher_fingerprint {
                return Err(format!(
                    "'{}' is already installed from a different publisher key. Remove it before importing a package signed by another key.",
                    inspected.manifest.id
                ));
            }
            true
        }
        None => false,
    };

    if !version_at_least(app_version, &inspected.manifest.min_app_version) {
        return Err(format!(
            "This package needs DragonFruit {} or newer; this build is {}.",
            inspected.manifest.min_app_version, app_version
        ));
    }

    Ok((bytes, inspected, replaces_existing))
}

/// `sideloaded_plugin_allowance`: whether complex sideloaded plugins are allowed.
pub fn sideloaded_plugin_allowance(ctx: &impl AppContext) -> BackendResult<bool> {
    let root = root_dir(ctx)?;
    Ok(read_settings(&root).allow_sideloaded_complex_plugins)
}

/// `sideloaded_plugin_set_allowance`.
pub fn sideloaded_plugin_set_allowance(ctx: &impl AppContext, allow: bool) -> BackendResult<bool> {
    let root = root_dir(ctx)?;
    let mut settings = read_settings(&root);
    settings.allow_sideloaded_complex_plugins = allow;
    write_settings(&root, &settings)?;
    Ok(allow)
}

/// `sideloaded_plugin_inspect`: what a package file is, and whether installing it
/// would replace something.
pub fn sideloaded_plugin_inspect(
    ctx: &impl AppContext,
    path: &str,
    app_version: &str,
) -> BackendResult<SideloadedInspect> {
    let root = root_dir(ctx)?;
    let (_bytes, inspected, replaces_existing) =
        inspect_file(Path::new(path), &root, app_version)?;
    Ok(SideloadedInspect {
        manifest: inspected.manifest,
        publisher_fingerprint: inspected.publisher_fingerprint,
        sealed: inspected.sealed,
        replaces_existing,
    })
}

/// `sideloaded_plugin_list`.
pub fn sideloaded_plugin_list(ctx: &impl AppContext) -> BackendResult<Vec<InstalledSideloadedPlugin>> {
    let root = root_dir(ctx)?;
    Ok(list_installed(&root))
}

/// `sideloaded_plugin_remove`: delete an installed package's directory. A package
/// that is not installed is success.
pub fn sideloaded_plugin_remove(ctx: &impl AppContext, id: &str) -> BackendResult<()> {
    let root = root_dir(ctx)?;
    let _guard = dir_lock()
        .lock()
        .map_err(|_| "Plugin install lock poisoned.".to_string())?;
    let dir = installed_dir(&root, id);
    if !dir.exists() {
        return Ok(());
    }
    std::fs::remove_dir_all(&dir)
        .map_err(|err| format!("Failed removing '{}': {err}", dir.display()))
}

/// `sideloaded_plugin_asset`: one image out of an installed package's
/// `payload/assets/`, as a data URL.
///
/// Paths are relative to `assets/`; nothing outside that directory is reachable and
/// only image types are served.
pub fn sideloaded_plugin_asset(
    ctx: &impl AppContext,
    id: &str,
    asset_path: &str,
) -> BackendResult<SideloadedAsset> {
    let root = root_dir(ctx)?;
    read_asset(&root, id, asset_path)
}

fn read_asset(root: &Path, id: &str, asset_path: &str) -> BackendResult<SideloadedAsset> {
    let relative = sanitize_asset_path(asset_path)?;
    let base = installed_dir(root, id).join(PAYLOAD_DIR).join("assets");
    let base = base
        .canonicalize()
        .map_err(|_| format!("plugin '{id}' has no assets"))?;
    let path = base.join(&relative);
    let path = path
        .canonicalize()
        .map_err(|_| format!("asset '{asset_path}' not found"))?;
    if !path.starts_with(&base) {
        return Err("asset path escapes the package".to_string());
    }
    let metadata = std::fs::metadata(&path).map_err(|err| err.to_string())?;
    if !metadata.is_file() || metadata.len() > MAX_ASSET_BYTES {
        return Err(format!(
            "asset '{asset_path}' is not a readable image under {MAX_ASSET_BYTES} bytes"
        ));
    }
    let mime = image_mime(&path)
        .ok_or_else(|| format!("asset '{asset_path}' is not an image type this build serves"))?;
    let bytes = std::fs::read(&path).map_err(|err| err.to_string())?;
    use base64::Engine as _;
    Ok(SideloadedAsset {
        mime: mime.to_string(),
        data_url: format!(
            "data:{mime};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        ),
    })
}

/// Relative segments only: no absolute path, no `.`/`..`, no hidden files.
pub fn sanitize_asset_path(input: &str) -> BackendResult<PathBuf> {
    let trimmed = input.trim().replace('\\', "/");
    if trimmed.is_empty() || trimmed.len() > 240 {
        return Err("asset path must be 1..240 characters".to_string());
    }
    let mut out = PathBuf::new();
    for segment in trimmed.split('/') {
        if segment.is_empty() || segment.starts_with('.') {
            return Err("asset path may not contain empty, hidden or dot segments".to_string());
        }
        out.push(segment);
    }
    Ok(out)
}

fn image_mime(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "avif" => "image/avif",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::HeadlessContext;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("df-plugins-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    fn context(label: &str) -> HeadlessContext {
        HeadlessContext::new(temp_dir(label), temp_dir(&format!("{label}-logs")))
    }

    #[test]
    fn version_comparison_accepts_equal_and_newer_builds() {
        assert!(version_at_least("0.3.0", "0.3.0"));
        assert!(version_at_least("0.4.0", "0.3.9"));
        assert!(version_at_least("1.0", "0.9.9"));
        assert!(!version_at_least("0.3.0", "0.4.0"));
        assert!(!version_at_least("0.3.0", "0.3.1"));
    }

    #[test]
    fn a_prerelease_suffix_is_ignored() {
        assert!(version_at_least("0.3.0-beta.4", "0.3.0"));
        assert!(!version_at_least("0.2.9-beta", "0.3.0"));
    }

    #[test]
    fn asset_paths_refuse_dot_segments_and_hidden_files() {
        assert!(sanitize_asset_path("presets/a.png").is_ok());
        assert!(sanitize_asset_path("../escape.png").is_err());
        assert!(sanitize_asset_path("presets/../../escape.png").is_err());
        assert!(sanitize_asset_path(".hidden.png").is_err());
        assert!(sanitize_asset_path("presets//a.png").is_err());
        assert!(sanitize_asset_path("").is_err());
        assert!(sanitize_asset_path(&"a".repeat(241)).is_err());
    }

    #[test]
    fn mime_types_are_recognised_by_extension() {
        assert_eq!(image_mime(Path::new("a.PNG")), Some("image/png"));
        assert_eq!(image_mime(Path::new("a.jpeg")), Some("image/jpeg"));
        assert_eq!(image_mime(Path::new("a.hex")), None);
        assert_eq!(image_mime(Path::new("a")), None);
    }

    #[test]
    fn the_allowance_defaults_to_off_and_round_trips() {
        let ctx = context("allowance");
        assert!(!sideloaded_plugin_allowance(&ctx).expect("default"));
        assert!(sideloaded_plugin_set_allowance(&ctx, true).expect("set"));
        assert!(sideloaded_plugin_allowance(&ctx).expect("read"));
        assert!(sideloaded_plugin_set_allowance(&ctx, false).expect("unset"));
        assert!(!sideloaded_plugin_allowance(&ctx).expect("read"));
    }

    #[test]
    fn an_empty_store_lists_nothing_and_removes_nothing() {
        let ctx = context("empty-store");
        assert!(sideloaded_plugin_list(&ctx).expect("list").is_empty());
        sideloaded_plugin_remove(&ctx, "not-installed").expect("no-op");
    }

    #[test]
    fn an_asset_outside_the_package_is_refused() {
        let ctx = context("asset");
        let err = sideloaded_plugin_asset(&ctx, "plugin.one", "a.png").expect_err("no package");
        assert!(err.contains("has no assets"), "{err}");
    }

    #[test]
    fn an_installed_package_lists_and_its_asset_round_trips() {
        let ctx = context("installed");
        let root = root_dir(&ctx).expect("root");
        let dir = root.join("plugin.one");
        std::fs::create_dir_all(dir.join(PAYLOAD_DIR).join("assets")).expect("assets");
        std::fs::write(
            dir.join(MANIFEST_FILE),
            serde_json::json!({
                "formatVersion": 1,
                "id": "plugin.one",
                "name": "One",
                "version": "1.0.0",
                "vendor": { "name": "V" },
                "minAppVersion": "0.1.0",
                "capabilities": [],
                "entry": { "kind": "process", "proto": 1 },
                "publisher": { "key": "abc" },
                "payload": { "sha256": "", "bytes": 0 },
                "formats": [],
                "fileTypes": [],
                "printerPresets": [],
                "materialTemplates": [],
                "jobMetadataPayloads": []
            })
            .to_string(),
        )
        .expect("manifest");
        std::fs::write(
            dir.join(STATE_FILE),
            serde_json::json!({
                "publisherFingerprint": "abc",
                "importedAt": "2026-01-01T00:00:00Z",
                "version": "1.0.0",
                "sealed": false
            })
            .to_string(),
        )
        .expect("state");
        std::fs::write(
            dir.join(PAYLOAD_DIR).join("assets").join("icon.png"),
            [0x89u8, b'P', b'N', b'G'],
        )
        .expect("asset");

        let listed = sideloaded_plugin_list(&ctx).expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].manifest.id, "plugin.one");
        assert!(!listed[0].sealed);

        let asset = sideloaded_plugin_asset(&ctx, "plugin.one", "icon.png").expect("asset");
        assert_eq!(asset.mime, "image/png");
        assert!(asset.data_url.starts_with("data:image/png;base64,"));

        sideloaded_plugin_remove(&ctx, "plugin.one").expect("remove");
        assert!(sideloaded_plugin_list(&ctx).expect("list").is_empty());
    }
}
