//! Scene-file, material, mesh-repair and diagnostics dialogs.
//!
//! Sources, one section per file:
//!
//! * `src/components/organisms/modals/SceneFileModals.tsx` — the unsaved-changes
//!   prompts, the scene-import placement and hollowing decisions, the plugin
//!   import warning, the save-choice sheet, the save-error sheet and the
//!   arrange blocking overlay.
//! * `src/components/modals/SceneMaterialReassignModal.tsx`
//! * `src/components/modals/ScenePrinterMismatchModal.tsx` — the
//!   printer/scene conflict dialog.
//! * `src/components/controls/ModelManagerPanel.tsx` — the plate-trash dialog.
//! * `src/components/modals/ManifoldWarningModal.tsx`
//! * `src/components/modals/TetrahedralRemeshConfirmModal.tsx`
//! * `src/components/modals/ShrinkWrapConfirmModal.tsx`
//! * `src/components/organisms/modals/MeshRepairModals.tsx` — the damaged-model
//!   dialog and the manual repair sheet.
//! * `src/components/organisms/modals/DiagnosticsModals.tsx` and the three
//!   dialogs it mounts: `DiagnosticsModal.tsx`, `HistoryDebugModal.tsx`,
//!   `SliceMetricsDebugModal.tsx`.
//!
//! Every dialog is a pure function of its data plus its callbacks; the caller
//! owns the open flag, the selection state and the async work, exactly as the
//! React components take their props. Each entry point is documented with the
//! source component it reproduces.
//!
//! # PORT DEVIATIONS
//!
//! * **z-index** — `z-[70]`, `z-[120]`, `z-[130]`, `z-[140]`, `z-[220]` have no
//!   GPUI counterpart. A dialog is painted where the caller places it, so the
//!   caller decides the stacking order.
//! * **Portals** — `StructuredDialogModal` renders through `createPortal` at
//!   the source; GPUI has no portal, so the backdrop is an `absolute` layer
//!   sized to its parent. It must be mounted in a full-size container.
//! * **`backdrop-blur-sm` / `backdrop-blur-[1px]`** — omitted; the translucent
//!   fill is kept.
//! * **`grid`** — GPUI has no grid layout. `grid-cols-2` / `grid-cols-4` /
//!   `auto-cols-fr` become wrapping flex rows whose children share the line
//!   with `flex_1`; the column count is therefore implicit in the available
//!   width instead of fixed.
//! * **`<table>`** — the material and benchmark tables are rows of divs. The
//!   source's `border-collapse` and `scope` attributes have no GPUI effect;
//!   the per-row top border and the header fill are reproduced.
//! * **`<input type="checkbox">`** — the plugin warning's "Do not remind
//!   again" box is drawn as a 14px bordered square with a `check` glyph when
//!   checked (`accentColor: #f59e0b` has no GPUI equivalent).
//! * **Transitions / animation** — `transition-colors`, the repair bar's
//!   `transition-[width] duration-300` and the spinner's SMIL rotation are
//!   static; every element is drawn in its settled state.
//! * **`title` / `aria-*`** — GPUI widgets carry no tooltip or accessible-name
//!   attribute, so the `title=` strings (full model names, byte explanations)
//!   are dropped rather than faked.
//! * **`letter-spacing`** — `tracking-wide` is not applied.
//! * **`tabular-nums`** — no font-feature control in GPUI; the digits are
//!   drawn in the regular figures of the loaded font.
//! * **`custom-scrollbar`** — the 10px accent-thumb scrollbar is not ported;
//!   the scroll containers are, with GPUI's own scrollbar.
//! * **Locale formatting** — `useLocaleFormat` formats numbers for the active
//!   Lingui locale. The port formats with `.` decimals and `,` thousands
//!   separators (the `en` catalog) and does not re-render on a locale switch.
//! * **`<svg><polyline>`** — the diagnostics sparklines are painted with
//!   `PathBuilder` on a `canvas` at the source's 2px round-capped stroke.
//! * **`conic-gradient`** — the slice-metrics pie is painted as filled wedges
//!   on a `canvas` (same colours, same percentages, same 80px ring).
//! * **`navigator` / `performance.memory`** — the diagnostics dialog's browser
//!   probes (user agent, `deviceMemory`, JS heap) are caller-supplied fields;
//!   the port does not invent them. `None` renders the source's `N/A`.
//! * **`navigator.clipboard`** — the slice-metrics "Copy" button calls the
//!   caller's `on_copy`; the port does not own a clipboard.
//! * **Escape** — `useEscapeToClose` is a window listener at the source. GPUI
//!   key handling lives in `main.rs`, so the caller decides whether Escape
//!   dismisses; the dialogs only expose their close callback.
//! * **Backdrop clicks** — the source's `onBackdropClick` is a `mousedown`
//!   handler on the backdrop. The GPUI backdrop is a plain layer with no hit
//!   handler, so a dismissal that a dialog offers only through its backdrop
//!   (the two unsaved-changes prompts, the import prompts, the repair sheet)
//!   stays with the caller; the close tile and the actions are wired.
//! * **Select trigger height** — `SceneMaterialReassignModal` passes
//!   `selectClassName="w-full h-[32px] px-2 pr-8 text-xs"`; `ui::select_dropdown`
//!   pins the trigger at the source's 36px `.ui-input` height, so the rows are
//!   4px taller than the source's.
//! * **`font-mono`** — the save-error detail block is set in the UI face; no
//!   monospace family is registered in this port.
//! * **`sticky top-0`** — the slice-metrics header scrolls with the panel; GPUI
//!   has no sticky positioning in this port.
//! * **Not ported here** — the sub-modals `SceneFileModals` mounts from other
//!   files (`ModelSupportsModal`, `ObsoleteVoxlVersionModal`,
//!   `ScenePluginMissingModal`, `SceneAutosaveRecoveryModal`,
//!   `ZipFilePickerModal`) and the ones `MeshRepairModals` mounts
//!   (`MeshRepairConfirmModal`, `MeshRepairReportModal`). They belong to their
//!   own source files and are separate ports.

#![allow(dead_code)]

// ── Submodules ──────────────────────────────────────────────────────────────
//
// Shared chrome lives in `chrome`; the dialogs keep one module each.

mod arrange_overlay;
mod chrome;
mod diagnostics;
mod format;
mod history_debug;
mod material_reassign;
mod mesh_repair;
mod plugin_warning;
mod printer_mismatch;
mod save;
mod scene_import;
mod slice_metrics;
mod unsaved_changes;

// ── Re-exports ──────────────────────────────────────────────────────────────

pub use arrange_overlay::arrange_blocking_overlay;
pub use diagnostics::{
    BenchmarkPhase, BenchmarkPhaseStats, BenchmarkRunState, BenchmarkStressProfile,
    DiagnosticsBenchmarkResult, DiagnosticsSceneFacts, DiagnosticsStats, SupportDiagnosticsStats,
    diagnostics_modal,
};
pub use history_debug::{
    HistoryDebugEvent, HistoryDebugKind, HistoryDebugSnapshot, history_debug_modal,
};
pub use material_reassign::{
    MaterialReassignMode, MaterialReassignOption, MaterialReassignRow, MaterialTankRow,
    scene_material_reassign_modal,
};
pub use mesh_repair::{
    RepairProgressState, damaged_model_modal, manifold_warning_modal, plate_delete_modal,
    repair_mesh_modal, shrink_wrap_confirm_modal, tetrahedral_remesh_confirm_modal,
};
pub use plugin_warning::plugin_import_warning_modal;
pub use printer_mismatch::{BuildVolumeMm, ScenePrinterMismatch, scene_printer_mismatch_modal};
pub use save::{SceneSaveError, scene_save_choice_modal, scene_save_error_modal};
pub use scene_import::{scene_import_hollowing_modal, scene_import_placement_modal};
pub use slice_metrics::{
    NativePerfMetrics, NativeRuntimeMetrics, SliceBenchmark, SliceCopyState, SliceJobConfig,
    SliceMetricsConfig, SliceNativePerf, slice_metrics_debug_modal,
};
pub use unsaved_changes::{close_unsaved_changes_modal, new_scene_unsaved_changes_modal};
