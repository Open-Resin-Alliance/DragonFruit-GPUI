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
//! * [`support_settings_body`].
//! * [`hollowing_body`] and [`hole_punch_body`].
//!
//! Each takes a cheap, cloneable state handle (`IslandsPanelState`,
//! `IslandListState`, `SupportSettingsState`, `HollowingState`,
//! `HolePunchState` — all `Rc<RefCell<..>>`, like `panels::PanelLayout`) plus the
//! app's `WeakEntity<DragonFruit>`, and returns the body only.
//!
//! # PORT DEVIATIONS
//!
//! ## Shared
//!
//! 1. `panels.rs::card_header` has no right slot, so each panel's header control
//!    (the Islands scan-settings gear, the Hollowing mode badge, the island
//!    list's Hierarchy button) is rendered at the top of the body, right-aligned,
//!    where the header slot would have put it.
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
//! 19. No preset store: `Rename` / `Export` / `Import` are inert, and the
//!     selector's `New preset…` footer entry does not exist. The three built-ins
//!     (`Light` / `Medium` / `Heavy`) are selectable and copy their
//!     `areaPerSupportMm2` into the draft, and `Duplicate` turns the selection
//!     into a custom profile in the panel's own state — the source's way out of
//!     a locked built-in — but the copy has no name and is not persisted.
//! 19a. The source locks every policy field while a built-in is active
//!     (`presetLocked`). That lock is reproduced; `Duplicate` is the escape, as
//!     at the source.
//! 20. No `AutoSupportSizingTierField`: the sizing band reads Support Studio's
//!     preset collection, which the app does not have.
//! 21. `overhangSelfSupportAngleDeg` is wired to `DragonFruit::support_angle`,
//!     the value `stl::overhang_points` classifies with. The dialog's other
//!     knobs live in the draft: nothing in the app consumes them yet, and
//!     `DragonFruit::support_density` (a 0.05–1.0 count multiplier) is a
//!     different quantity from `areaPerSupportMm2`, so it is not bound here.
//! 22. Save writes the draft over the baseline and clears the modified marker;
//!     there is no settings store to persist to.
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

pub mod hollowing;
pub mod hole_punch;
pub mod island_list;
pub mod island_scan_settings;
pub mod islands;
pub mod support_settings;

pub use hollowing::*;
pub use hole_punch::*;
pub use island_list::*;
pub use island_scan_settings::*;
pub use islands::*;
pub use support_settings::*;
