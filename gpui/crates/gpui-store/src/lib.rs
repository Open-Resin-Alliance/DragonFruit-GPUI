//! Persistent state for the GPUI port: settings, the last session, and user
//! support presets, stored as JSON under the Tauri app-data directory.
//!
//! Ported from the persistence the React app keeps in `window.localStorage`
//! and IndexedDB — the per-feature preference modules under
//! `src/components/settings/` (`uiScalePreference.ts`,
//! `cameraFeelPreferences.ts`, `cameraFovPreferences.ts`,
//! `cameraProjectionPreferences.ts`, `cameraTrackpadPreferences.ts`,
//! `performancePreferences.ts`, `spaceMousePreferences.ts`,
//! `sceneAutosavePreferences.ts`, `multiPlatePreferences.ts`,
//! `view3dPreferences.ts`, `workspaceCameraPreferences.ts`,
//! `uvToolsPreferences.ts`, `supportPlacementPreferences.ts`,
//! `themeCustomizations.ts`), `src/features/scene/importDefaultsPreferences.ts`,
//! `src/features/scene/useSceneCollectionManager.ts`'s mesh appearance key,
//! `src/hotkeys/HotkeyContext.tsx`,
//! `src/components/settings/LocalBackupsSettingsTab.tsx`,
//! `src/components/settings/LoggingSettingsTab.tsx` and
//! `src/supports/Settings/presets.ts` — plus the panel layout that
//! `gpui/crates/gpui-core/src/panel_layout.rs` already serialises.
//!
//! The root is `dirs::data_dir()/org.openresinalliance.dragonfruit`: the
//! identifier from `src-tauri/tauri.conf.json`, which is also what the Tauri
//! backend's `app_data_dir()` resolves to, so backups and plugin state stay in
//! the directory the backend already writes.
//!
//! What makes this storage rather than a JSON dump:
//!
//! - writes go to `<file>.tmp` and are renamed over the target, so a crash
//!   leaves either the old file or the new one, never a half-written one;
//! - an unparseable file is renamed `<file>.corrupt-<unix-ts>` and defaults
//!   are returned — user data is moved aside, never deleted;
//! - a `schemaVersion` above this crate's own is loaded read-only, and every
//!   save of that file fails instead of clobbering a newer build's data;
//! - [`Store::save_settings_debounced`] hands the write to a background thread
//!   that coalesces edits for [`DEBOUNCE_INTERVAL`], so the caller never pays
//!   for disk IO on the frame it happened in.

pub mod debounce;
pub mod error;
mod file;
pub mod presets;
pub mod session;
pub mod settings;
mod store;

pub use debounce::DEBOUNCE_INTERVAL;
pub use error::{StoreError, StoreFile};
pub use presets::{SupportPreset, SupportPresets, SUPPORT_PRESETS_SCHEMA_VERSION};
pub use session::{CameraPose, FloatPoint, PanelDock, PanelPlacement, Session, SessionLayout, SESSION_SCHEMA_VERSION};
pub use settings::{
    BackupsSettings, CameraFeel, CameraFeelPreset, CameraModeDefaults, CameraProjection,
    CameraScopeMode, CameraSettings, CameraTrackpadModifierKey, CameraTrackpadPrimaryAction,
    CameraTrackpadSettings, ExperimentsSettings, GeneralSettings, HotkeyBinding, HotkeySettings,
    ImportDefaults, LogLevel, LoggingSettings, MatcapVariant, MeshSettings, MeshShaderType,
    MultiPlateSettings, PerformanceSettings, PlateOrderingMode, PluginsSettings,
    PngCompressionStrategy, RaftBottomMode, SceneAutosaveSettings, SelectionHighlightDefaults,
    SelectionHighlightMode, Settings, SliceSatMode, SliceSatRender, SpaceMousePivotMode,
    SpaceMouseSettings, ThemeColors, ThemePreference, ThumbnailRenderSettings, ToolLayout,
    UiSettings, UpdateChannel, UpdateSettings, UvToolsSettings, View3dOriginMode, View3dSettings,
    WorkspaceCameraSettings, SETTINGS_SCHEMA_VERSION,
};
pub use store::{Store, APP_DIR_NAME};
