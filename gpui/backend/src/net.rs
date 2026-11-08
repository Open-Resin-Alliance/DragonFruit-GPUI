//! Network fetches the renderer cannot do itself — `fetch_sponsor_avatar`,
//! `fetch_external_text`, `fetch_github_plugin_resource`.
//!
//! Ported from `src-tauri/src/main.rs` lines 4040-4190. They exist because the
//! Tauri webview has no CORS grant for these hosts, and the desktop build serves
//! only static assets (so the Next.js `/api/*` routes do not exist in production).
//!
//! The allowlists are the security boundary and are copied verbatim: each command
//! accepts `https` only, and only from the hosts it was written for. Anything else
//! is refused here rather than fetched.
//!
//! These are `reqwest::blocking` calls, unlike the shell's `async` ones — every
//! entry point in this crate is synchronous (see the crate docs).

use std::time::Duration;

use base64::Engine as _;
use serde::Serialize;

use crate::BackendResult;

/// Hosts whose avatars the sponsors panel may display. Mirrors the allowlist in
/// `src/app/api/sponsor-avatar/route.ts`.
const SPONSOR_AVATAR_HOSTS: &[&str] = &[
    "opencollective-production.s3.us-west-1.amazonaws.com",
    "opencollective-production.s3-us-west-1.amazonaws.com",
    "images.opencollective.com",
    "avatars.githubusercontent.com",
];

/// Hosts the plugin manifest resolver may reach.
const GITHUB_PLUGIN_HOSTS: &[&str] = &["github.com", "api.github.com", "raw.githubusercontent.com"];

/// Hosts the sponsors list may reach.
const EXTERNAL_TEXT_HOSTS: &[&str] = &[
    "opencollective.com",
    "www.opencollective.com",
    "api.opencollective.com",
];

/// Parse and gate a URL: `https` only, and only `allowed`.
fn checked_url(raw: &str, allowed: &[&str]) -> BackendResult<url::Url> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("URL is empty".to_string());
    }
    let parsed = url::Url::parse(trimmed).map_err(|e| format!("Invalid url: {e}"))?;
    if parsed.scheme() != "https" {
        return Err("Only https URLs are allowed".to_string());
    }
    let host = parsed.host_str().ok_or("Missing host")?.to_ascii_lowercase();
    if !allowed.contains(&host.as_str()) {
        return Err(format!("Host not allowed: {host}"));
    }
    Ok(parsed)
}

/// A plugin manifest resource, fetched native-side.
///
/// Field names are **snake_case**: the shell's struct had no `rename_all`, so that
/// is the shape the renderer's resolver already reads. Non-2xx responses come back
/// with their status rather than an error, so the resolver can report the HTTP
/// failure itself.
#[derive(Serialize)]
pub struct GithubPluginResource {
    pub status: u16,
    pub content_type: String,
    pub body_base64: String,
}

/// `fetch_sponsor_avatar`: a sponsors avatar as a `data:` URL, so the UI can render
/// it without further network access.
pub fn fetch_sponsor_avatar(url: &str) -> BackendResult<String> {
    let parsed = checked_url(url, SPONSOR_AVATAR_HOSTS)?;

    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(parsed)
        .header("Accept", "image/*,*/*")
        .send()
        .map_err(|e| format!("Fetch failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Upstream {}", resp.status()));
    }
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/png")
        .to_string();
    let bytes = resp.bytes().map_err(|e| format!("Read failed: {e}"))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{content_type};base64,{b64}"))
}

/// `fetch_github_plugin_resource`: fetch a plugin manifest, preset pack or preset
/// image from GitHub.
pub fn fetch_github_plugin_resource(url: &str) -> BackendResult<GithubPluginResource> {
    let parsed = checked_url(url, GITHUB_PLUGIN_HOSTS)?;

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Client build failed: {e}"))?;
    let resp = client
        .get(parsed)
        .header("Accept", "application/json, */*")
        .header("User-Agent", "DragonFruit")
        .send()
        .map_err(|e| format!("Fetch failed: {e}"))?;

    let status = resp.status().as_u16();
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = resp.bytes().map_err(|e| format!("Read failed: {e}"))?;

    Ok(GithubPluginResource {
        status,
        content_type,
        body_base64: base64::engine::general_purpose::STANDARD.encode(&bytes),
    })
}

/// `fetch_external_text`: the sponsors list, as text.
pub fn fetch_external_text(url: &str) -> BackendResult<String> {
    let parsed = checked_url(url, EXTERNAL_TEXT_HOSTS)?;

    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(parsed)
        .header("Accept", "application/json,*/*")
        .send()
        .map_err(|e| format!("Fetch failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Upstream {}", resp.status()));
    }
    resp.text().map_err(|e| format!("Read failed: {e}"))
}

/// `ensure_rtsp_relay`: the native RTSP relay's status, starting it if needed.
///
/// The relay itself is `dragonfruit-rtsp-relay`; the shell command was the whole
/// adapter (network.rs). Ported verbatim.
pub use dragonfruit_rtsp_relay::RelayStatusResponse;

pub fn ensure_rtsp_relay(rtsp_url: Option<&str>) -> RelayStatusResponse {
    dragonfruit_rtsp_relay::ensure_rtsp_relay_status(rtsp_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_https_from_the_named_hosts_is_accepted() {
        assert!(checked_url("https://api.opencollective.com/x", EXTERNAL_TEXT_HOSTS).is_ok());
        // Plain http.
        let err = checked_url("http://api.opencollective.com/x", EXTERNAL_TEXT_HOSTS)
            .expect_err("http");
        assert!(err.contains("Only https"), "{err}");
        // A host that is not on the list, including a lookalike suffix.
        let err = checked_url("https://api.opencollective.com.evil.test/x", EXTERNAL_TEXT_HOSTS)
            .expect_err("host");
        assert!(err.contains("Host not allowed"), "{err}");
        // Case-insensitive host matching.
        assert!(checked_url("https://API.GitHub.com/x", GITHUB_PLUGIN_HOSTS).is_ok());
        // Empty.
        assert!(checked_url("   ", GITHUB_PLUGIN_HOSTS).is_err());
    }

    #[test]
    fn each_command_keeps_its_own_allowlist() {
        // A GitHub avatar host is not a sponsor-feed host, and vice versa.
        assert!(checked_url(
            "https://avatars.githubusercontent.com/u/1",
            SPONSOR_AVATAR_HOSTS
        )
        .is_ok());
        assert!(checked_url(
            "https://avatars.githubusercontent.com/u/1",
            EXTERNAL_TEXT_HOSTS
        )
        .is_err());
    }
}
