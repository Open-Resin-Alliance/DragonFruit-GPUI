//! The store itself: one data directory, three documents, and the guards that
//! keep a save from destroying what a newer build wrote.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::debounce::{SettingsWriter, lock};
use crate::error::{StoreError, StoreFile};
use crate::file::{document_path, read_document, write_atomic};
use crate::presets::{SUPPORT_PRESETS_SCHEMA_VERSION, SupportPresets};
use crate::session::{SESSION_SCHEMA_VERSION, Session};
use crate::settings::{SETTINGS_SCHEMA_VERSION, Settings};

/// The Tauri identifier from `src-tauri/tauri.conf.json`. `app_data_dir()`
/// resolves to `dirs::data_dir()/this`, which is where the backend's backups
/// and plugin state already live.
pub const APP_DIR_NAME: &str = "org.openresinalliance.dragonfruit";

/// Owns the data directory and the debounced settings writer.
pub struct Store {
    root: PathBuf,
    // `0` means "writable"; any other value is the newer `schemaVersion` that
    // was found, and every save of that document is refused.
    settings_read_only: AtomicU32,
    session_read_only: AtomicU32,
    presets_read_only: AtomicU32,
    debouncer: Mutex<Option<SettingsWriter>>,
}

impl Store {
    /// Open the platform data directory, creating it if needed.
    pub fn open() -> Result<Self, StoreError> {
        let root = Self::default_root()?;
        std::fs::create_dir_all(&root).map_err(|source| StoreError::CreateDir {
            path: root.clone(),
            source,
        })?;
        Ok(Self::at(root))
    }

    /// `<data dir>/org.openresinalliance.dragonfruit`.
    pub fn default_root() -> Result<PathBuf, StoreError> {
        dirs::data_dir()
            .map(|base| base.join(APP_DIR_NAME))
            .ok_or(StoreError::NoDataDir)
    }

    /// A store at an explicit directory, for tests. The directory is created on
    /// the first save, not here.
    pub fn at(root: PathBuf) -> Self {
        Self {
            root,
            settings_read_only: AtomicU32::new(0),
            session_read_only: AtomicU32::new(0),
            presets_read_only: AtomicU32::new(0),
            debouncer: Mutex::new(None),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The absolute path of one document.
    pub fn path(&self, file: StoreFile) -> PathBuf {
        document_path(&self.root, file)
    }

    /// True when the file on disk was written by a newer build and this one
    /// must not overwrite it. Set by the matching `load_*` call.
    pub fn is_read_only(&self, file: StoreFile) -> bool {
        self.read_only_version(file).is_some()
    }

    /// The newer `schemaVersion` found on disk, if any.
    pub fn read_only_version(&self, file: StoreFile) -> Option<u32> {
        match self.flag(file).load(Ordering::Relaxed) {
            0 => None,
            version => Some(version),
        }
    }

    fn flag(&self, file: StoreFile) -> &AtomicU32 {
        match file {
            StoreFile::Settings => &self.settings_read_only,
            StoreFile::Session => &self.session_read_only,
            StoreFile::SupportPresets => &self.presets_read_only,
        }
    }

    fn guard(&self, file: StoreFile) -> Result<(), StoreError> {
        match self.read_only_version(file) {
            None => Ok(()),
            Some(found) => Err(StoreError::NewerSchema {
                file,
                found,
                supported: file.schema_version(),
            }),
        }
    }

    // --- Settings --------------------------------------------------------

    /// Settings, or the defaults when the file is absent, unreadable, corrupt
    /// (the corrupt copy is renamed aside) or newer than this build.
    pub fn load_settings(&self) -> Settings {
        let mut settings: Settings = read_document(
            &self.root,
            StoreFile::Settings,
            &self.settings_read_only,
        )
        .unwrap_or_default();
        settings.schema_version = SETTINGS_SCHEMA_VERSION;
        settings
    }

    /// Write settings atomically. Fails rather than downgrading a file written
    /// by a newer build.
    pub fn save_settings(&self, settings: &Settings) -> Result<(), StoreError> {
        self.guard(StoreFile::Settings)?;
        let mut stamped = settings.clone();
        stamped.schema_version = SETTINGS_SCHEMA_VERSION;
        write_atomic(&self.root, StoreFile::Settings, &stamped)
    }

    /// Queue settings for the debounced writer. Returns as soon as the value is
    /// in the channel: no file is opened on the calling thread.
    ///
    /// Every edit can call this; the newest value is written 500 ms after the
    /// last call ([`crate::DEBOUNCE_INTERVAL`]).
    pub fn save_settings_debounced(&self, settings: &Settings) -> Result<(), StoreError> {
        self.guard(StoreFile::Settings)?;
        let mut slot = lock(&self.debouncer);
        if slot.is_none() {
            *slot = Some(SettingsWriter::spawn(self.root.clone())?);
        }
        let writer = slot.as_ref().expect("writer was just installed");
        let mut stamped = settings.clone();
        stamped.schema_version = SETTINGS_SCHEMA_VERSION;
        writer.enqueue(stamped)
    }

    /// Wait for the queued settings write to finish. `Ok(())` when nothing is
    /// queued. This is the only blocking call in the debounced path.
    pub fn flush_debounced(&self) -> Result<(), StoreError> {
        let slot = lock(&self.debouncer);
        match slot.as_ref() {
            Some(writer) => writer.flush(),
            None => Ok(()),
        }
    }

    /// Take the last background write failure, if any. A debounced save cannot
    /// return its IO error at call time; this is where it surfaces.
    pub fn take_debounced_error(&self) -> Option<StoreError> {
        let slot = lock(&self.debouncer);
        slot.as_ref().and_then(SettingsWriter::take_error)
    }

    // --- Session ---------------------------------------------------------

    /// Last session, or the defaults when there is none to read.
    pub fn load_session(&self) -> Session {
        let mut session: Session =
            read_document(&self.root, StoreFile::Session, &self.session_read_only)
                .unwrap_or_default();
        session.schema_version = SESSION_SCHEMA_VERSION;
        session
    }

    pub fn save_session(&self, session: &Session) -> Result<(), StoreError> {
        self.guard(StoreFile::Session)?;
        let mut stamped = session.clone();
        stamped.schema_version = SESSION_SCHEMA_VERSION;
        write_atomic(&self.root, StoreFile::Session, &stamped)
    }

    // --- Support presets -------------------------------------------------

    /// User presets, or an empty collection when there are none to read.
    pub fn load_support_presets(&self) -> SupportPresets {
        let mut presets: SupportPresets = read_document(
            &self.root,
            StoreFile::SupportPresets,
            &self.presets_read_only,
        )
        .unwrap_or_default();
        presets.schema_version = SUPPORT_PRESETS_SCHEMA_VERSION;
        presets
    }

    pub fn save_support_presets(&self, presets: &SupportPresets) -> Result<(), StoreError> {
        self.guard(StoreFile::SupportPresets)?;
        let mut stamped = presets.clone();
        stamped.schema_version = SUPPORT_PRESETS_SCHEMA_VERSION;
        write_atomic(&self.root, StoreFile::SupportPresets, &stamped)
    }
}
