//! The slice of `tauri::AppHandle` that commands actually used.
//!
//! Reading the 131 registered handlers, the ambient dependency is small: a data
//! directory (`app.path().app_data_dir()`), a log directory
//! (`app.path().app_log_dir()`), a way to push an event at the UI
//! (`app.emit` / `window.emit`), and the main window's visibility and focus.
//! Everything else that looked like context — progress sinks, cancel flags, the
//! mesh registry — is explicit state the caller owns instead, which is why the
//! vast majority of the surface ported with no context at all.
//!
//! ## Why there is no `spawn` on this trait
//!
//! Tauri's commands were `async` only to reach `spawn_blocking`. GPUI's
//! `background_executor` is the same facility and already belongs to the app, so
//! a spawner here would be a second threading policy with no caller. The ported
//! functions are blocking; the GPUI layer decides their executor.

use std::path::PathBuf;
use std::sync::Mutex;

use serde_json::Value;

use crate::BackendResult;

/// Where the app's files live, and where its events go.
pub trait AppContext {
    /// `app.path().app_data_dir()`. Created by the caller if it does not exist;
    /// the commands that use it create their own subdirectories.
    fn data_dir(&self) -> BackendResult<PathBuf>;

    /// `app.path().app_log_dir()`.
    fn log_dir(&self) -> BackendResult<PathBuf>;

    /// Push an event at the UI. `app.emit` and `window.emit` were the same call
    /// for every ported command, so there is one method; a command that needs to
    /// name its window can put it in the payload.
    fn emit(&self, event: &str, payload: Value);

    /// The main window, for the three commands that showed or focused it.
    fn window(&self) -> &dyn WindowHost;
}

/// The main window operations `focus_main_window_command` and
/// `reveal_main_window_command` performed.
///
/// `src-tauri` treated a missing window as visible / not minimized / not
/// focused, so a host with no window can answer those that way and make the
/// show-and-focus sequence a no-op.
pub trait WindowHost {
    fn is_visible(&self) -> bool;
    fn show(&self);
    fn is_minimized(&self) -> bool;
    fn unminimize(&self);
    fn is_focused(&self) -> bool;
    fn set_focus(&self);
    /// Close the startup splash, if the build has one.
    fn close_splash(&self);
}

/// A context with no window and no events: what a headless test wants, and a
/// reasonable fallback for a command that only needs directories.
#[derive(Debug, Default)]
pub struct HeadlessContext {
    pub data_dir: Option<PathBuf>,
    pub log_dir: Option<PathBuf>,
    events: Mutex<Vec<(String, Value)>>,
    window: NoWindow,
}

impl HeadlessContext {
    pub fn new(data_dir: PathBuf, log_dir: PathBuf) -> Self {
        Self {
            data_dir: Some(data_dir),
            log_dir: Some(log_dir),
            events: Mutex::new(Vec::new()),
            window: NoWindow,
        }
    }

    /// Every event emitted so far, in order.
    pub fn events(&self) -> Vec<(String, Value)> {
        self.events.lock().expect("event log lock").clone()
    }
}

impl AppContext for HeadlessContext {
    fn data_dir(&self) -> BackendResult<PathBuf> {
        self.data_dir
            .clone()
            .ok_or_else(|| "Failed resolving app data directory: not configured".to_string())
    }

    fn log_dir(&self) -> BackendResult<PathBuf> {
        self.log_dir
            .clone()
            .ok_or_else(|| "Failed to resolve log dir: not configured".to_string())
    }

    fn emit(&self, event: &str, payload: Value) {
        self.events
            .lock()
            .expect("event log lock")
            .push((event.to_string(), payload));
    }

    fn window(&self) -> &dyn WindowHost {
        &self.window
    }
}

/// A window host that reports the defaults `src-tauri` substituted for a missing
/// window, and does nothing.
#[derive(Debug, Default)]
pub struct NoWindow;

impl WindowHost for NoWindow {
    fn is_visible(&self) -> bool {
        true
    }
    fn show(&self) {}
    fn is_minimized(&self) -> bool {
        false
    }
    fn unminimize(&self) {}
    fn is_focused(&self) -> bool {
        false
    }
    fn set_focus(&self) {}
    fn close_splash(&self) {}
}
