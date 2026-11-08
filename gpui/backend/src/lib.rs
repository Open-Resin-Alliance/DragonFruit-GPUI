//! DragonFruit's command surface, without Tauri.
//!
//! Every operation the desktop shell exposed through `#[tauri::command]` lives
//! in `src-tauri/src/`: 131 registered handlers over ~17.9k lines. The GPUI port
//! calls the same work as ordinary Rust instead of serializing JSON across an
//! IPC boundary, so this crate is the adapter layer: it keeps each command's
//! contract (arguments, result shape, guards, error text) and delegates the work
//! to the `rust/` engines.
//!
//! # What moved, and what did not
//!
//! `docs/internal/gpui-backend-map.md` lists all 131 commands with a verdict.
//! "Ported" means the logic is here; "deferred" means it stayed behind with a
//! stated reason. In outline:
//!
//! * Ported, no ambient dependency — staging, mesh load/repair/hollow/punch/cut,
//!   surfacing, slicing, export, print-file reads, temp-artifact sweeps, scene
//!   file commits, network fetches, filesystem/system helpers, log-level prefs,
//!   experiment gates.
//! * Ported, needs an [`AppContext`] — anything that resolved `app_data_dir` /
//!   `app_log_dir` or touched a window: autosave, local backup defaults, the
//!   update-channel preference, sideloaded-plugin storage, the log viewer, and
//!   the window focus/reveal commands.
//! * Deferred — see the map. The island/overhang scanners and the SpaceMouse
//!   navlib bridge are deferred because their algorithms live in `src-tauri`
//!   rather than in a `rust/` crate; the file dialogs because a GPUI app opens
//!   its own; the updater because it is `tauri-plugin-updater`; the plugin
//!   network host, encoder registry and webview watchdog likewise belong to the
//!   Tauri webview shell.
//!
//! # Everything is synchronous
//!
//! The Tauri commands were `async`, but only because each one wrapped blocking
//! work in `tauri::async_runtime::spawn_blocking`. That is not observable
//! behaviour — it is a threading policy — and GPUI already owns one
//! (`cx.background_executor().spawn`). So the entry points here are blocking and
//! the caller decides what runs where. The one internal pool that is kept is the
//! slicer's named rayon pool ([`slicing`]), because its thread count is a
//! measured performance contract, not a scheduling detail.
//!
//! Process-wide state that Tauri kept in `static`s (the staging buffer, the
//! cancel flags, the mesh registry, the organic-cut preview buffers) is owned
//! state here — [`MeshStaging`], [`repair::CancelRegistry`],
//! [`mesh_io::MeshRegistry`], [`organic_cut::OrganicCutState`] — so a headless
//! test can have its own and two windows could ever have separate ones.

pub mod ao;
pub mod autosave;
pub mod backup;
pub mod context;
pub mod experiments;
pub mod export;
pub mod files;
pub mod islands;
pub mod log_prefs;
pub mod mesh_io;
pub mod net;
pub mod organic_cut;
pub mod plugins;
pub mod repair;
pub mod scene_file;
pub mod slicing;
pub mod staging;
pub mod system;
pub mod updater;

pub use context::{AppContext, HeadlessContext, NoWindow, WindowHost};
pub use experiments::Experiments;
pub use mesh_io::MeshRegistry;
pub use organic_cut::OrganicCutState;
pub use repair::{CancelRegistry, HollowPreview, PunchPreview};
pub use slicing::Slicer;
pub use staging::MeshStaging;

/// Every command's error type. The Tauri surface used `Result<T, String>` and the
/// frontend matched on those strings, so the text is part of the contract.
pub type BackendResult<T> = std::result::Result<T, String>;

/// Scene file extensions contributed by built-in fileType plugins, without a
/// leading dot.
///
/// Copied from `src-tauri/src/generated_builtin_plugins.rs`'s
/// `GENERATED_BUILTIN_PLUGIN_SCENE_FILE_EXTENSIONS`, which
/// `scripts/generate-plugin-registry.mjs` regenerates. Kept as a literal rather
/// than a build-time read because the GPUI crate does not own that generator;
/// when the generated list changes this constant must change with it.
pub const BUILTIN_PLUGIN_SCENE_FILE_EXTENSIONS: &[&str] = &["chitubox", "ctp", "lys"];
