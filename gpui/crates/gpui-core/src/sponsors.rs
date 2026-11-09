//! Sponsor avatars — `SponsorsCarousel.tsx`'s `CachedAvatar` (`:200-366`) and
//! `toHighResUrl` (`:150-177`).
//!
//! The source paints each sponsor's Open Collective avatar and keeps it in an
//! in-memory map plus `localStorage`, because the S3 bucket re-streams the PNG
//! progressively on every open. It fetches the bytes through the Tauri command
//! `fetch_sponsor_avatar` (`src-tauri/src/main.rs:4054`) — a Rust `reqwest`
//! call with a host allowlist — or the same-origin `/api/sponsor-avatar`
//! proxy on the web (`src/app/api/sponsor-avatar/route.ts`).
//!
//! This port has neither a WebView nor a Next.js server, and GPUI's own HTTP
//! seam (`App::http_client`) is a `NullHttpClient` in this build (it fails with
//! "No HttpClient available"), so `img(uri)` cannot load a remote avatar.
//! The bytes are fetched with `ureq` on GPUI's background executor instead,
//! decoded through `gpui::Image::to_image_data`, and cached as a
//! `RenderImage` — `img(Arc<RenderImage>)` is the render path that needs no I/O
//! at paint time.
//!
//! Fetch failures are terminal for the session and surface as
//! [`AvatarState::Failed`], which the About tab paints as the source's
//! monogram. (The source instead leaves the circle blank: it falls back to the
//! bare URL for a direct `<img>` load and hides the image after it errors.) A
//! cold remount clears the failures, mirroring the source's retry on every
//! carousel mount.
//!
//! PORT DEVIATIONS
//! - **No `localStorage`.** The decoded `RenderImage` lives in this process
//!   only; every launch fetches the avatars once.
//! - **`ureq`, not `reqwest`.** `src-tauri` uses `reqwest`; `ureq` reaches the
//!   same three hosts with rustls and no async runtime, which keeps the GPUI
//!   blob small. It is the one dependency this feature adds
//!   (`gpui-core/Cargo.toml`).

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, PoisonError};
use std::time::Duration;

use gpui::{App, Image, ImageExt as _, ImageFormat, RenderImage, WeakEntity, prelude::*};

/// `sponsors.json`'s `image` field, keyed by the sponsor name the About tab
/// renders. Eleven of the eighteen fallback sponsors ship an avatar; the other
/// seven have `image: null` and paint the monogram branch.
const FALLBACK_AVATARS: [(&str, &str); 11] = [
    (
        "KCast",
        "https://opencollective-production.s3.us-west-1.amazonaws.com/account-avatar/9d3d5d53-be83-4892-a5e9-7b81735ced93/IMG_6181.jpg",
    ),
    (
        "Siraya Tech",
        "https://opencollective-production.s3.us-west-1.amazonaws.com/account-avatar/9d2748f9-cbf9-4f1f-a456-14bdf6a03680/siraya-logo-mid.jpg",
    ),
    (
        "Atlas 3DSS",
        "https://opencollective-production.s3.us-west-1.amazonaws.com/account-avatar/f479abfb-3b76-4f3f-87c1-9e6f7bfe29b8/_AtlasLogo_Large.png",
    ),
    (
        "Gobbledygook Games",
        "https://opencollective-production.s3.us-west-1.amazonaws.com/account-avatar/3b53c2e8-cfb3-4aad-aa05-a3b9c470ecce/GGGAvatar3.png",
    ),
    (
        "Concepts 3D",
        "https://opencollective-production.s3.us-west-1.amazonaws.com/account-avatar/e3e2aacc-4bb2-44d3-9504-551de34e9af7/Untitled%20design%20(1).png",
    ),
    (
        "Kryz Slaughter",
        "https://opencollective-production.s3.us-west-1.amazonaws.com/account-avatar/5a9cb541-0082-4a40-8e0e-e9e3905b205b/judiciar-2.jpg",
    ),
    (
        "Rob Skinner",
        "https://opencollective-production.s3.us-west-1.amazonaws.com/account-avatar/24d313ae-8019-42b5-bd80-6f4fe279d494/IMG20260829104728.jpg",
    ),
    (
        "Cunabula",
        "https://opencollective-production.s3.us-west-1.amazonaws.com/account-avatar/17078813-f0be-4bda-9c18-ee637e971fd3/CP_Icon_250.png",
    ),
    (
        "Viznoman",
        "https://opencollective-production.s3.us-west-1.amazonaws.com/account-avatar/b023bcba-1115-4154-aa3c-8991af9ebc6f/Smug%20man%20face%20icon.jpg",
    ),
    (
        "Tom",
        "https://opencollective-production.s3.us-west-1.amazonaws.com/account-avatar/2d59b7e9-9f09-4df5-bc98-0b06a0b3fcb0/IMG_2626.jpg",
    ),
    (
        "Sin",
        "https://opencollective-production.s3.us-west-1.amazonaws.com/account-avatar/4ae6c055-8efc-4f2f-8390-1848c1305111/HSQI5561.jpg",
    ),
];

/// The hosts the web proxy (`src/app/api/sponsor-avatar/route.ts:3-8`) and the
/// Tauri command (`main.rs:4064-4069`) accept. A URL outside this list is
/// never fetched.
const ALLOWED_HOSTS: [&str; 4] = [
    "opencollective-production.s3.us-west-1.amazonaws.com",
    "opencollective-production.s3-us-west-1.amazonaws.com",
    "images.opencollective.com",
    "avatars.githubusercontent.com",
];

/// One sponsor avatar, as the About tab needs it.
#[derive(Clone)]
pub enum AvatarState {
    /// The fetch is in flight: the source paints its `Spinner size="lg"`.
    Loading,
    Ready(Arc<RenderImage>),
    /// The fetch or the decode failed: the tile paints the monogram.
    Failed,
}

static CACHE: LazyLock<Mutex<HashMap<&'static str, AvatarState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn cache() -> std::sync::MutexGuard<'static, HashMap<&'static str, AvatarState>> {
    CACHE.lock().unwrap_or_else(PoisonError::into_inner)
}

fn set(url: &'static str, state: AvatarState) {
    cache().insert(url, state);
}

/// The avatar `sponsors.json` ships for `name`, if any.
pub fn avatar_for(name: &str) -> Option<&'static str> {
    FALLBACK_AVATARS
        .iter()
        .find(|(sponsor, _)| *sponsor == name)
        .map(|(_, url)| *url)
}

/// What to paint for `url` right now, if the About tab has asked for it yet.
pub fn state(url: &'static str) -> Option<AvatarState> {
    cache().get(url).cloned()
}

/// Forget every failure so a fresh mount retries, like `CachedAvatar` does when
/// the carousel remounts. In-flight and loaded avatars are left alone.
pub fn reset_failed() {
    cache().retain(|_, state| !matches!(state, AvatarState::Failed));
}

/// Start the fetch for `url` unless it has already started. The tab paints
/// [`AvatarState::Loading`] until the decode lands, then repaints `view`.
pub fn ensure_loaded<T: 'static>(url: &'static str, cx: &mut App, view: WeakEntity<T>) {
    if cache().contains_key(url) {
        return;
    }
    let Some(host) = host_of(url) else {
        set(url, AvatarState::Failed);
        return;
    };
    if !ALLOWED_HOSTS.contains(&host) {
        crate::log::log(format!("sponsor avatar: refused host {host}"));
        set(url, AvatarState::Failed);
        return;
    }

    set(url, AvatarState::Loading);
    let target = high_res(url);
    let renderer = cx.svg_renderer();
    cx.spawn(async move |cx| {
        let result = cx
            .background_executor()
            .spawn(async move {
                let bytes = fetch(&target)?;
                let format = format_of(&bytes)
                    .ok_or_else(|| "the response is not a PNG, JPEG, GIF or WebP".to_owned())?;
                Image::from_bytes(format, bytes)
                    .to_image_data(renderer)
                    .map_err(|error| error.to_string())
            })
            .await;
        match result {
            Ok(image) => set(url, AvatarState::Ready(image)),
            Err(error) => {
                crate::log::log(format!("sponsor avatar: {url}: {error}"));
                set(url, AvatarState::Failed);
            }
        }
        let _ = view.update(cx, |_, cx| cx.notify());
    })
    .detach();
}

/// `toHighResUrl` (`SponsorsCarousel.tsx:150-177`): ask the avatar hosts for a
/// 512px image. S3 ignores the parameters (the source says so in place) but
/// keeps returning the object; the Open Collective image CDN honours them.
fn high_res(url: &str) -> String {
    let Some(host) = host_of(url) else {
        return url.to_owned();
    };
    let mut out = url.to_owned();
    if host.contains("images.opencollective.com") {
        if !url.contains("height=") && !url.contains("width=") {
            push_query(&mut out, "height=512&width=512");
        }
    } else if host.contains("githubusercontent") {
        if !url.contains("s=") {
            push_query(&mut out, "s=512");
        }
    } else if host.contains("s3") && !url.contains("height=") {
        push_query(&mut out, "height=512&width=512");
    }
    out
}

fn push_query(url: &mut String, params: &str) {
    url.push(if url.contains('?') { '&' } else { '?' });
    url.push_str(params);
}

/// The host of an `https` URL, without the port. `None` for anything else, so
/// the allowlist is never bypassed by a different scheme or by userinfo.
fn host_of(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://")?;
    let end = rest
        .find(|c: char| c == '/' || c == '?' || c == '#')
        .unwrap_or(rest.len());
    let authority = &rest[..end];
    if authority.contains('@') {
        return None;
    }
    let host = authority.split(':').next().unwrap_or(authority);
    if host.is_empty() { None } else { Some(host) }
}

/// One agent for every avatar: the connection pool and the TLS root store are
/// shared instead of being rebuilt per avatar.
static AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .new_agent()
});

fn fetch(target: &str) -> Result<Vec<u8>, String> {
    // `fetch_sponsor_avatar`'s request (`main.rs:4070-4085`): an image `Accept`
    // header and a bounded read. The timeout keeps a hung S3 request from
    // leaving the tile on its spinner.
    let mut response = AGENT
        .get(target)
        .header("Accept", "image/*,*/*")
        .call()
        .map_err(|error| error.to_string())?;
    response
        .body_mut()
        .read_to_vec()
        .map_err(|error| error.to_string())
}

/// The decoded format, from the bytes rather than the URL: the S3 objects mix
/// `.png` and `.jpg`, and the CDN can answer a `.png` URL with a JPEG.
fn format_of(bytes: &[u8]) -> Option<ImageFormat> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(ImageFormat::Png)
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(ImageFormat::Jpeg)
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some(ImageFormat::Gif)
    } else if bytes.len() > 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP".as_slice()
    {
        Some(ImageFormat::Webp)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_fallback_image_is_fetchable() {
        for (name, url) in FALLBACK_AVATARS {
            let host = host_of(url).unwrap_or_else(|| panic!("{name}: not https: {url}"));
            assert!(
                ALLOWED_HOSTS.contains(&host),
                "{name}: host {host} is not on the allowlist"
            );
            assert_eq!(avatar_for(name), Some(url));
        }
        assert_eq!(avatar_for("ben"), None);
    }

    #[test]
    fn high_res_matches_the_source_urls() {
        assert_eq!(
            high_res("https://images.opencollective.com/x/y.png"),
            "https://images.opencollective.com/x/y.png?height=512&width=512"
        );
        assert_eq!(
            high_res("https://avatars.githubusercontent.com/u/1?s=64"),
            "https://avatars.githubusercontent.com/u/1?s=64"
        );
        assert_eq!(
            high_res("https://avatars.githubusercontent.com/u/1"),
            "https://avatars.githubusercontent.com/u/1?s=512"
        );
        assert_eq!(
            high_res("https://opencollective-production.s3.us-west-1.amazonaws.com/a/b.png"),
            "https://opencollective-production.s3.us-west-1.amazonaws.com/a/b.png?height=512&width=512"
        );
    }

    #[test]
    fn hosts_are_read_without_ports_or_userinfo() {
        assert_eq!(
            host_of("https://images.opencollective.com:443/a.png"),
            Some("images.opencollective.com")
        );
        assert_eq!(host_of("https://evil.example/x"), Some("evil.example"));
        assert_eq!(host_of("http://images.opencollective.com/x"), None);
        assert_eq!(host_of("https://user@images.opencollective.com/x"), None);
        assert_eq!(host_of("https:///x"), None);
    }

    #[test]
    fn formats_are_sniffed_from_bytes() {
        assert_eq!(format_of(b"\x89PNG\r\n\x1a\nrest"), Some(ImageFormat::Png));
        assert_eq!(format_of(b"\xFF\xD8\xFF\xE0"), Some(ImageFormat::Jpeg));
        assert_eq!(format_of(b"GIF89a...."), Some(ImageFormat::Gif));
        assert_eq!(format_of(b"RIFF\0\0\0\0WEBPmore"), Some(ImageFormat::Webp));
        assert_eq!(format_of(b"<svg></svg>"), None);
    }
}
