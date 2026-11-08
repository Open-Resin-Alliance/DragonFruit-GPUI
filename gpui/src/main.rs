//! DragonFruit's GPUI frontend: the editor shell around the 3D viewport.
//!
//! # PORT DEVIATIONS
//!
//! Diffed against `src/app/page.tsx`, `src/components/layout/TopBar.tsx`,
//! `src/components/layout/FloatingPanelStack.tsx`, `src/hotkeys/hotkeyConfig.ts`
//! and the hotkey hooks. Everything below is a gap the shell still carries.
//!
//! **Not ported (components the shell used to invent or omit)**
//! * `EmptySceneState` — with no model the source shows the empty-scene panel
//!   (load/import buttons, recents, onboarding). The shell renders the bare
//!   viewport instead; the only way to load a model is the app menu's
//!   `Import mesh…`, which the source also offers.
//! * `ModelStatsCard` — the bottom-left stats card (dimensions, volume, print
//!   time, resin) is not ported. The old bottom-left status text was invented
//!   and is gone; the source has no status line.
//! * `PrintingPreviewPane` — printing mode is a two-column split in the source
//!   (scene left, layer preview right). The shell keeps the full-width scene.
//! * `NotificationStack` / `SystemNotificationStack` — only the scene-import
//!   error toast is reproduced (see below); save, history, export, orientation
//!   and system toasts have no trigger in this build.
//! * `PrintingModals`, `SceneFileModals`, `SceneMaterialReassignModal`,
//!   `ModifierModals`, `ManifoldWarningModal`, `TetrahedralRemeshConfirmModal`,
//!   `ShrinkWrapConfirmModal`, `MeshRepairModals`, the two `StructuredDialogModal`s
//!   (printer/scene conflict, plate trash), `DiagnosticsModals`,
//!   `FirstRunOnboarding`, `EditorContextMenu`, `OrganicCutOverlay`,
//!   `RenderProfilerOverlay` — none are ported.
//! * The Auto Supports settings dialog (`AutoSupportSettingsBody`, a 1120px
//!   full-screen dialog) is not ported. The small modal that used to live here
//!   was invented and has been removed; `panel_content`'s gear still sets
//!   `show_support_settings`, which nothing renders.
//! * Prepare's tool panels (`TransformControls`, `AutoRotationPanel`,
//!   `MeshSmoothingSettingsPanel`, `OrganicCutPanel`, `ArrangePanel`,
//!   `DuplicatePanel`) and Support's `IslandsPanel`, `SupportSidebar`,
//!   `HollowingPanel`, `HolePunchPanel` are not mounted. `transform_panel.rs`
//!   exists but its callbacks take no `cx`, so the shell cannot apply them.
//! * Export's `SlicingPanel` is not mounted (no slicer).
//!
//! **Panel set**
//! * Models is shown only in Prepare/Support, matching `PreparePanelStack` /
//!   Support's stack; Export and Printing have no Models panel.
//! * The panel id is `models` in both modes. The source uses
//!   `prepare-models` / `support-models` with independent placement, but the
//!   rail's Models toggle (`rail.rs`) is hard-wired to the `models` id here.
//! * Support's right-hand panel follows the rail tool: only `Automatic`
//!   (index 2) has a ported body (`Auto Supports (Beta)`). The other tools
//!   leave the dock empty because their panels are not ported.
//! * `panel_stack` derives the rail / layer-slider insets from the panel set
//!   (`PanelStackInsets::for_panels`). The source also zeroes the left inset
//!   when the rail is horizontal; the rail's layout lives in `rail.rs` and is
//!   not readable here, so the column width is always assumed.
//!
//! **Hotkeys** (`DEFAULT_KEYBINDINGS`, edge-triggered like `useActionActive`)
//! * Bound: `CAMERA.TOGGLE_PROJECTION` (o), `CAMERA.FOCUS_PICK` (f),
//!   `CANVAS.TOOL_SELECT` (q), `TOOL_MODIFY` / `TOOL_MODIFY_LOCAL` (m /
//!   shift+m), `TOOL_SMOOTH` (s), `TOOL_ARRANGE` (a), `TOOL_DUPLICATE` (d) —
//!   the canvas tools only in Prepare with a model, as `usePrepareTransformHotkeys`
//!   gates them — `GLOBAL.DELETE` (Backspace, plus the fixed Delete key),
//!   `NAVIGATION.LAYER_UP` / `LAYER_DOWN` (Up/Down, and W/S in printing).
//! * Escape closes the menus and the settings dialog (`useEscapeToClose`).
//! * Not bound because their features are absent: `SUPPORTS.*` hold actions,
//!   `PRESETS.SLOT_1..6` (no camera presets), `GLOBAL.UNDO/REDO/SAVE/SAVE_AS`
//!   (no history or scene files), `DEBUG.*`, `HOLE_PUNCH.*`, `CUT.*`,
//!   `CAMERA.INTERIOR_VIEW` (the flag lives in `rail.rs`),
//!   `CANVAS.SELECT_ALL/COPY/PASTE`, `TOOL_MODIFY_LOCAL`'s local-space flag
//!   (no field), and `TOOL_ARRANGE`'s "arrange all when already active".
//! * The previous 1–4 mode switch and `R` reset were invented; the registry
//!   has no mode-switch binding (1–6 are camera presets), so mode changes are
//!   click-only.
//! * `FOCUS_PICK` has no hovered-point raycast, so it frames the whole model
//!   (`Camera::fit`'s distance) rather than the point under the cursor.
//! * Hotkeys are suppressed while the settings dialog is open. The source
//!   suppresses only when the event target is editable; GPUI exposes no
//!   "focused input" query, so the whole dialog is treated as owning the keys.
//!
//! **Drag and drop**
//! * External file drops are wired for Prepare only, matching
//!   `handlePrepareDrop`. Supported names are `.stl`, `.obj`, `.3mf`, `.voxl`,
//!   `.lys` (`PREPARE_DROP_EXTENSIONS`); the overlay strings and colours come
//!   from `page.tsx`.
//! * Only STL and VOXL can load (`stl.rs`, `scene/`). OBJ/3MF importers are not
//!   ported, so those drops are accepted by the overlay and then do nothing.
//! * `.lys` has no importer and raises the source's
//!   `Unsupported scene file: {name}` toast (`reportUnsupportedSceneFile`).
//! * A VOXL model carries a full XYZ rotation; the port's viewport has a Z
//!   angle only, so `rotation.x`/`.y` are dropped when a scene is opened.
//! * The source tracks drag depth and hides the overlay when the pointer leaves
//!   `#scene-root`; GPUI dispatches `on_drag_move` for the whole window, so the
//!   overlay follows the canvas bounds only.
//! * `backdrop-filter: blur(1px)` has no GPUI equivalent.
//!
//! **Layers / printing**
//! * `layer` is a layer index in `0..=numLayers`, with `numLayers =
//!   ceil(bboxHeight / 0.05mm)` — `useSlicingManager`'s uncontrolled 50µm
//!   default (`layerHeightMicron = 50`). The slider is `min 0, max numLayers,
//!   step 1`, as `CrossSliderPanel` passes it.
//! * The cross-section clip is the source's upper plane
//!   (`zMin + layer*layerHeight + 1e-6`, none at layer 0 or at the top layer)
//!   and is gated by `cross_section`, in every mode, as `sceneClipStore` does.
//!   The lower plane has no viewport parameter (`viewport.rs` takes one
//!   `clip_z`), so the lower thumb is display-only.
//! * The layer slider is hidden in Printing, as `page.tsx` hides
//!   `CrossSliderPanel` there; the source's printing scrubber lives in the
//!   unported `PrintingPreviewPane`.
//! * Printing is locked by `app_bar` because `has_printing_data` is `false`
//!   (no slicing pipeline), exactly as a fresh source session locks it; the
//!   printing panel and clip branch are therefore unreachable until a slicer
//!   exists.
//! * `hasSlicedPlate` / `isActiveSliceStale` guards from `handleModeChange`
//!   cannot be evaluated; the shell enforces only the "no models → Prepare"
//!   guard.
//!
//! **Text / chrome**
//! * Geist is installed per-user by `fonts::ensure_geist` (the old
//!   `FR_PRIVATE` registration was not guaranteed to reach DirectWrite), and
//!   text renders with `TextRenderingMode::Subpixel`; the window background is
//!   opaque.
//! * CSS transitions, keyframes and backdrop blur have no GPUI equivalent.

mod app;
mod chrome;
mod features;
mod panels;
mod widgets;

pub use crate::app::Toast;
pub(crate) use crate::app::{DragonFruit, Mode};

use gpui::{
    App, Bounds, TextRenderingMode, TitlebarOptions, WindowBounds, WindowOptions, prelude::*, px,
    size,
};
use gpui_core::{fonts, log, window};

fn main() {
    fonts::ensure_geist();
    gpui::application().run(|cx: &mut App| {
        // Subpixel needs an opaque window background, which is the default
        // (`window.rs` never sets `background_appearance`).
        cx.set_text_rendering_mode(TextRenderingMode::Subpixel);
        let bounds = Bounds::centered(None, size(px(1280.), px(800.)), cx);
        let options = window::with_wgpu_renderer(WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("DragonFruit".into()),
                appears_transparent: true,
                traffic_light_position: None,
            }),
            ..Default::default()
        });
        log::log("gpui: opening window");
        cx.open_window(options, |_, cx| cx.new(DragonFruit::new))
            .expect("a window");
        cx.activate(true);
    });
}
