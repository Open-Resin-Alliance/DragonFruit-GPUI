//! Shared UI atoms and field widgets: buttons, cards, selects, sliders and the
//! settings-row primitives built from the generated theme tokens.

pub mod panel_fields;
pub mod widgets;

/// The widget library under its historical path, so `ui::card()` and
/// `ui::progress_bar_phase(...)` call sites keep resolving.
pub mod ui {
    pub use crate::widgets::*;
}
