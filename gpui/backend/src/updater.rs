//! Release channel preference — `get_saved_update_channel`, `save_update_channel`.
//!
//! Ported from `src-tauri/src/updater_channel.rs` lines 355-399. The preference is
//! one text file in the app data dir; reading and writing it needs nothing from
//! `tauri-plugin-updater`.
//!
//! ## Deferred: `check_updates` and `perform_update`
//!
//! Both are `tauri-plugin-updater` calls: the first builds an updater from the
//! plugin's registered state (`app_handle.updater_builder()`), the second hands the
//! cached `Update` to `download_and_install`, which verifies the signature, launches
//! the platform installer and exits the app. That is the plugin's job, not an
//! adapter's, and it needs no `AppContext` surface beyond what is listed here — the
//! GPUI port needs an equivalent updater implementation, not more context.
//!
//! The endpoints and the Linux rule are recorded here so that port has one place to
//! read them: Linux ships only as a Flatpak, whose feed has no `linux-x86_64` key and
//! which cannot install over a running sandboxed app (`/app` is read-only), so the
//! shell skipped the check and let the Flatpak remote update it.

use crate::context::AppContext;
use crate::BackendResult;

const CHANNEL_FILE: &str = "update-channel.txt";
const STABLE_CHANNEL: &str = "stable";
const DEV_CHANNEL: &str = "dev";

/// The stable release feed.
pub const STABLE_ENDPOINT: &str = "https://open-resin-alliance.github.io/DragonFruit/latest.json";
/// The pre-release feed. Not published for Linux (Flatpak handles its own updates).
pub const DEV_ENDPOINT: &str =
    "https://open-resin-alliance.github.io/DragonFruit/latest-dev.json";

/// Which feed a channel name maps to. `"prerelease"` is accepted as a synonym for
/// `"dev"`, as the shell did.
pub fn endpoint_for_channel(channel: &str) -> &'static str {
    match channel {
        "dev" | "prerelease" => DEV_ENDPOINT,
        _ => STABLE_ENDPOINT,
    }
}

fn channel_path(ctx: &impl AppContext) -> BackendResult<std::path::PathBuf> {
    Ok(ctx.data_dir()?.join(CHANNEL_FILE))
}

/// `get_saved_update_channel`: the saved preference, or `"stable"`.
///
/// Anything in the file that is not exactly `stable` or `dev` reads as `stable` — a
/// hand-edited or truncated file must not select an unknown feed.
pub fn get_saved_update_channel(ctx: &impl AppContext) -> String {
    let Ok(path) = channel_path(ctx) else {
        return STABLE_CHANNEL.to_string();
    };
    if !path.exists() {
        return STABLE_CHANNEL.to_string();
    }
    std::fs::read_to_string(&path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| s == STABLE_CHANNEL || s == DEV_CHANNEL)
        .unwrap_or_else(|| STABLE_CHANNEL.to_string())
}

/// `save_update_channel`: validate and persist the preference.
pub fn save_update_channel(ctx: &impl AppContext, channel: &str) -> BackendResult<()> {
    if channel != STABLE_CHANNEL && channel != DEV_CHANNEL {
        return Err(format!(
            "Invalid channel '{channel}'. Must be 'stable' or 'dev'."
        ));
    }
    let path = channel_path(ctx)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create app data dir: {e}"))?;
    }
    std::fs::write(&path, channel)
        .map_err(|e| format!("Failed to write channel preference: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::HeadlessContext;
    use std::path::PathBuf;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("df-updater-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    fn context(label: &str) -> HeadlessContext {
        HeadlessContext::new(temp_dir(label), temp_dir(&format!("{label}-logs")))
    }

    #[test]
    fn the_default_channel_is_stable() {
        let ctx = context("default");
        assert_eq!(get_saved_update_channel(&ctx), "stable");
    }

    #[test]
    fn a_saved_channel_round_trips_and_junk_reads_as_stable() {
        let ctx = context("round-trip");
        save_update_channel(&ctx, "dev").expect("save");
        assert_eq!(get_saved_update_channel(&ctx), "dev");

        let path = ctx.data_dir().expect("data").join(CHANNEL_FILE);
        std::fs::write(&path, "nightly").expect("write junk");
        assert_eq!(get_saved_update_channel(&ctx), "stable");

        // A trailing newline is a text editor's doing, not a different channel.
        std::fs::write(&path, " dev\n").expect("write");
        assert_eq!(get_saved_update_channel(&ctx), "dev");
    }

    #[test]
    fn an_unknown_channel_is_refused() {
        let ctx = context("invalid");
        let err = save_update_channel(&ctx, "nightly").expect_err("refuse");
        assert!(err.contains("Invalid channel 'nightly'"), "{err}");
    }

    #[test]
    fn channels_map_to_their_feeds() {
        assert_eq!(endpoint_for_channel("dev"), DEV_ENDPOINT);
        assert_eq!(endpoint_for_channel("prerelease"), DEV_ENDPOINT);
        assert_eq!(endpoint_for_channel("stable"), STABLE_ENDPOINT);
        assert_eq!(endpoint_for_channel("anything"), STABLE_ENDPOINT);
    }
}
