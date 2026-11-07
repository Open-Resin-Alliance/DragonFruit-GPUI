//! The non-dismissible arrange blocking overlay — `SceneFileModals.tsx`.

use gpui::{Div, SharedString};
use gpui_ui::ui::{BlockingProgress, blocking_overlay};

use super::format::{grouped};

// ── Arrange blocking overlay ────────────────────────────────────────────────

/// `showArrangeBlockingOverlay` — `BlockingOverlay` from `SceneFileModals.tsx`.
///
/// The non-dismissible arrange sheet: the translated title and detail lines,
/// `Elapsed: {elapsed}` in the accent line and `Processing N model(s)` as the
/// footnote. The progress bar is the indeterminate sweep (`progress={null}`).
pub fn arrange_blocking_overlay(
    title: SharedString,
    details: &[SharedString],
    elapsed: SharedString,
    model_count: usize,
) -> Div {
    let footnote: SharedString = if model_count == 1 {
        "Processing 1 model".into()
    } else {
        format!("Processing {} models", grouped(model_count as f64)).into()
    };
    blocking_overlay(
        title,
        details,
        Some(format!("Elapsed: {elapsed}").into()),
        Some(footnote),
        BlockingProgress::Indeterminate,
        None::<Div>,
    )
}
