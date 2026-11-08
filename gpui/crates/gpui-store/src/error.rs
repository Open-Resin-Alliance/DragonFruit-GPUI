//! Every failure the store can report, typed.

use std::io;
use std::path::PathBuf;

use thiserror::Error;

/// One of the JSON documents the store owns. Used in errors and in
/// [`crate::Store::is_read_only`] so a caller can name the file it is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StoreFile {
    Settings,
    Session,
    SupportPresets,
}

impl StoreFile {
    /// The file name inside the data directory.
    pub const fn file_name(self) -> &'static str {
        match self {
            Self::Settings => "settings.json",
            Self::Session => "session.json",
            Self::SupportPresets => "support-presets.json",
        }
    }

    /// The `schemaVersion` this build writes for the document.
    pub const fn schema_version(self) -> u32 {
        match self {
            Self::Settings => crate::settings::SETTINGS_SCHEMA_VERSION,
            Self::Session => crate::session::SESSION_SCHEMA_VERSION,
            Self::SupportPresets => crate::presets::SUPPORT_PRESETS_SCHEMA_VERSION,
        }
    }

    pub const fn all() -> [Self; 3] {
        [Self::Settings, Self::Session, Self::SupportPresets]
    }
}

impl std::fmt::Display for StoreFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.file_name())
    }
}

/// Any store operation that can fail.
#[derive(Debug, Error)]
pub enum StoreError {
    /// `dirs::data_dir()` had no answer on this platform.
    #[error("no platform data directory is known")]
    NoDataDir,

    #[error("could not create {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("could not write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("could not replace {path} with its new contents: {source}")]
    Rename {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("could not serialise {file}: {source}")]
    Serialize {
        file: StoreFile,
        #[source]
        source: serde_json::Error,
    },

    /// The file was written by a newer build; this one must not overwrite it.
    #[error("{file} uses schema version {found}, newer than this build's {supported}")]
    NewerSchema {
        file: StoreFile,
        found: u32,
        supported: u32,
    },

    #[error("could not start the debounced settings writer: {source}")]
    DebouncedWorker {
        #[source]
        source: io::Error,
    },

    /// The background writer thread is gone; the save never reached the queue.
    #[error("the debounced settings writer has stopped")]
    DebouncedWorkerStopped,
}
