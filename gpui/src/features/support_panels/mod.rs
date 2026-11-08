//! Support-mode panel bodies.
//!
//! Sources: `src/components/controls/IslandsPanel.tsx`,
//! `src/components/controls/IslandListCard.tsx`,
//! `src/components/controls/autoSupport/AutoSupportSettingsBody.tsx` (through
//! `AutoSupportPresets.tsx` and the `autoSupportPanelTabs.ts` catalogue),
//! `src/features/hollowing/HollowingPanel.tsx` and
//! `src/features/hole-punching/HolePunchPanel.tsx`.
//!
//! The Auto Supports *floating* panel is `panel_content.rs::supports_body`; this
//! module is its settings-dialog body, the three tool panels beside it and the
//! island surfaces. Only bodies are ported: the card, its header, its collapse
//! chevron and the panel placement belong to `panels.rs`.
//!
//! # Entry points
//!
//! * [`islands_body`] plus [`island_scan_settings_dialog`] — the dialog is a
//!   window-level body, mounted while `IslandsModel::settings_open` is set.
//! * [`island_list_body`].
//! * [`support_settings_body`], whose preset strip and knobs are backed by
//!   [`studio_presets`] (`support-presets.json`).
//! * [`hollowing_body`] and [`hole_punch_body`].
//!
//! Each takes a cheap, cloneable state handle (`IslandsPanelState`,
//! `IslandListState`, `HollowingState`, `HolePunchState` — all
//! `Rc<RefCell<..>>`, like `panels::PanelLayout`) plus the app's
//! `WeakEntity<DragonFruit>`, and returns the body only. `support_settings_body`
//! is the exception: its values live in `support-presets.json`
//! ([`studio_presets`]), not in a per-panel handle.
//!
//! # PORT DEVIATIONS
//!
//! ## Shared
//!
//! 1. A body cannot reach its own card header: `PanelDef::header_right` exists,
//!    but the caller builds the `PanelDef`. Each body therefore renders its
//!    header control (the Islands scan-settings gear, the Hollowing mode badge,
//!    the island list's Hierarchy button) at the top of the body, right-aligned.
//!    The one control that does use the real slot is the Auto Supports gear
//!    ([`crate::features::tool_panels::supports_settings_gear`]), chained onto
//!    that panel's `PanelDef` by `app.rs`.
//! 2. GPUI has no CSS transitions, `backdrop-blur`, `letter-spacing`,
//!    `tabular-nums` or `filter: grayscale()`: hover fills apply instantly, the
//!    backdrop blur is omitted, tracked labels are not tracked, numerals are not
//!    tabular, and the disabled hole-punch card keeps its opacity but not its
//!    grey cast.
//! 3. `space-y-*` / `space-x-*` are a flex `gap` on the same container.
//! 4. Radii come from `gpui_core::theme::ROUNDED_*`: `globals.css` overrides
//!    `--radius-sm`/`md`/`lg` to 6/10/14px, so `rounded-sm`/`md`/`lg` are
//!    6/10/14, and `rounded-xl` keeps v4's 12px. Bare `rounded` has no token
//!    and stays 4px.
//! 5. The `.ui-panel` cascade: `.ui-panel .ui-button` / `.ui-input` paint at
//!    `--font-size-md` (14px), so the sources' `text-[10px]` / `text-[11px]` on
//!    those controls never apply inside a card. Raw `<button>`s (the island
//!    layer toggles, the nav arrows) keep their class sizes.
//! 6. No text entry in this slice: `NumberInput`, `ScrollableNumberField` and
//!    the island search box render their value or placeholder without a caret.
//!    The steppers (and, on the dialog fields, the two 12px chevrons) are the
//!    way to change a value.
//! 7. No range inputs: the scan dialog's two `input[type=range]` sliders render
//!    as stepper fields at the same min/max/step.
//! 8. No portals: `island_scan_settings_dialog` is a window-level body the shell
//!    mounts; `ui::dialog_modal`'s backdrop is a plain div, not a fixed layer.
//! 9. No timers: `applySettings`'s 50 ms "Recalculating…" pass is not
//!    reproduced — Apply copies the draft immediately.
//! 10. `ui::select_dropdown`'s trigger is the atom's 36px `.ui-input`; the
//!     sources' `!h-8` (32px) overrides are not applied.
//! 11. `.ui-checkbox`'s 3px accent ring and its rotated-border tick are replaced
//!     by the `check` glyph; the ring is not drawn.
//!
//! ## Islands — `IslandsPanel.tsx`
//!
//! 12. No island scanner: `scanning`, the three totals and the island labels are
//!     the shell's to fill in. The Scan button renders the source's enabled and
//!     disabled recipes but starts nothing, and the `Recalculating…` overlay only
//!     shows while the shell sets `applying_settings`.
//! 13. The body's `maxHeight: calc(100vh - …)` scroll cap is not applied: the
//!     card grows to its content in this shell, so a scroll container would
//!     collapse.
//! 14. `fmt.quantity(draftPxMm, 2, 'mmPerPx')` has no locale formatter here; the
//!     value prints as `0.05 mm/px`.
//!
//! ## Island list — `IslandListCard.tsx`
//!
//! 15. `IslandHierarchyModal` is not ported; the Hierarchy button sets
//!     `show_hierarchy`, which the shell may read.
//! 16. The search box has no text entry, so `search` stays whatever the shell
//!     seeds (the filter itself is ported).
//! 17. The card's `clientWidth <= 286` compact-header measurement is not
//!     reproduced; the full Hierarchy button always renders.
//!
//! ## Support settings — `AutoSupportSettingsBody.tsx`
//!
//! 18. Only the sections this panel is named for are ported: the preset strip,
//!     Density & Sizing, Stability and the footer. Detection, Distribution,
//!     Post-processing, Diagnostics and Advanced (calibration) are not rendered.
//! 19. The preset strip is the studio collection (`detail` / `structure` /
//!     `anchor` plus the user's own) persisted in `support-presets.json` by
//!     [`studio_presets`], which the app hydrates on the first render. Selecting
//!     a record is written at once, so the selection survives a restart.
//!     `Rename` / `Export` / `Import` are still inert (no text entry, no Tauri
//!     file bridge), and the selector's `New preset…` footer entry does not
//!     exist.
//! 19a. The source locks every policy field while a built-in is active
//!     (`presetLocked`). That lock is reproduced; `Duplicate` is the escape, as
//!     at the source, and it is what makes an edit persistable.
//! 20. No `AutoSupportSizingTierField` in this panel: the sizing band it edits
//!     lives in the record's `settings.autoSupport.sizingPreset`, which the
//!     dialog's own tier field (not ported here) reads.
//! 21. `overhangSelfSupportAngleDeg` is wired to `DragonFruit::support_angle`,
//!     the value `stl::overhang_points` classifies with. The panel's other knobs
//!     live in the active record: nothing in the app consumes them yet, and
//!     `DragonFruit::support_density` (a 0.05–1.0 count multiplier) is a
//!     different quantity from `areaPerSupportMm2`, so it is not bound here.
//! 22. Save merges the draft into the active custom record's
//!     `settings.autoSupport` and writes the document; a staged edit that is
//!     never saved is dropped, where the source also persists the live settings
//!     continuously.
//!
//! ## Hollowing — `HollowingPanel.tsx`
//!
//! 23. No hollowing engine: Apply, Remove Hollowing, Solid Areas and the
//!     voxel-edit Done/Clear only flip the panel's own flags.
//! 24. `HollowMode::ShellOpenFace` is unreachable from the panel (the source's
//!     Mode card offers Cavity and Infill only), but its Open Face card is
//!     ported because the shell can put the state there.
//!
//! ## Hole punching — `HolePunchPanel.tsx`
//!
//! 25. No punching engine: Apply and Reset only flip the panel's own flags.

// The shell has not mounted these bodies yet, so every handle and body is
// unreachable from `main` today. The lint is silenced here rather than on each
// item: once `panels.rs` builds the support stack it goes away on its own.
#![allow(dead_code)]

mod shared;

/// [`shared::hint`], for the panel adapters outside this module tree (the Auto
/// Supports header gear [`crate::features::tool_panels`] builds).
pub(crate) use shared::hint;

pub mod hollowing;
pub mod hole_punch;
pub mod island_list;
pub mod island_scan_settings;
pub mod islands;
pub mod studio_presets;
pub mod support_settings;

pub use hollowing::*;
pub use hole_punch::*;
pub use island_list::*;
pub use island_scan_settings::*;
pub use islands::*;
pub use support_settings::*;
