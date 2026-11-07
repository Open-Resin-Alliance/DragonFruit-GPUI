//! Lucide icons, generated into `assets/icons` by
//! `scripts/generate-gpui-icons.mjs` from the app's own imports.

use gpui::{IntoElement, Styled, px, rgb, svg};

/// Render an icon by its lucide slug (`chevron-down`, `folder-open`, ...).
pub fn icon(name: &str, size: f32, color: u32) -> impl IntoElement {
    let path = format!("{}/assets/icons/{name}.svg", env!("CARGO_MANIFEST_DIR"));
    svg()
        .external_path(path)
        .w(px(size))
        .h(px(size))
        .text_color(rgb(color))
}
