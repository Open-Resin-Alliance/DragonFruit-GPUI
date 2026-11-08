# GPUI store

`gpui/crates/gpui-store` is the port's persistence layer. It replaces the
webview storage the React app used — `window.localStorage` for preferences and
the settings modules under `src/components/settings/` — because a Rust process
cannot read WebView2's `localStorage`. Tauri's backend and the GPUI app share
one data directory instead.

## Where the files live

The root is `dirs::data_dir()` joined with `APP_DIR_NAME`, the Tauri identifier
from `src-tauri/tauri.conf.json`. Tauri's `app_data_dir()` resolves to the same
place, so the backend's backups, sideloaded plugins and update channel stay in
one directory.

| Platform | Directory |
|---|---|
| Windows | `%APPDATA%\org.openresinalliance.dragonfruit` |
| macOS | `~/Library/Application Support/org.openresinalliance.dragonfruit` |
| Linux | `$XDG_DATA_HOME/org.openresinalliance.dragonfruit` (default `~/.local/share/org.openresinalliance.dragonfruit`) |

`Store::open()` creates the directory and uses the platform default;
`Store::at(path)` points a store at an explicit directory for tests.
`Store::path(file)` returns one document's absolute path. The directory is
shared deliberately: the React app's `localStorage` was invisible to Rust, but
the backend already wrote under this identifier, so backups and plugin state do
not move.

## The three documents

`StoreFile` names them, and each carries its own `schemaVersion`.

| File | Type | Holds |
|---|---|---|
| `settings.json` | `Settings` | User preferences, grouped the way Settings tabs group them: `general` (UI scale, language, layout persistence, import defaults, support-placement help), `camera` (feel preset, FOV, projection, trackpad), `workspaces`, `multiPlate`, `mesh` (shader, matcap, colours), `performance`, `spaceMouse`, `plugins`, `sceneAutosave`, `backups`, `uvtools`, `ui` (theme preference and overrides), `hotkeys`, `logging` |
| `session.json` | `Session` | What to reopen with: `lastFile`, the camera pose (`yaw`, `pitch`, `distance`, `target`, `fovY`, `orthographic`), the Prepare `activeTool` index, and `panelLayout` — layout version 6, panel id → placement (`dock`, `order`, `width`, float position) plus left/right dock widths |
| `support-presets.json` | `SupportPresets` | The user's support presets and `activePresetId`. Only user presets are stored: the built-ins (`detail`, `structure`, `anchor`) are code in `src/supports/Settings/presets.ts` and are re-derived on every launch. A preset's `settings` tree stays raw JSON because the supports crate owns its shape |

Every field is `#[serde(default)]`. A file written before a field existed still
loads, and an unknown enum value falls back to its default rather than failing
the whole document — the same tolerance the React `normalize*` helpers had.
`SessionLayout` deliberately keeps placements for panels that are not visible
this session, because the React layout document did.

The schema constants are `SETTINGS_SCHEMA_VERSION`, `SESSION_SCHEMA_VERSION`
and `SUPPORT_PRESETS_SCHEMA_VERSION`; `StoreFile::schema_version()` returns the
right one, so the loader and writer cannot disagree.

## The four rules

### 1. Atomic

A save writes `<file>.tmp`, calls `sync_all`, closes the handle and renames it
over the target. The handle is closed before the rename because Windows refuses
to replace an open file. A failed rename removes the temp file, so no `.tmp`
debris is left.

**Why.** A settings or preset file is not reconstructible: a crash, power loss
or kill in the middle of a write would leave a half-document that the next
launch cannot parse, and the user's configuration would be gone. Rename is the
only step that changes the visible file, so the next launch sees the old
document or the new one, never a mixture.

### 2. Debounced

`save_settings_debounced` pushes a settings value into an unbounded channel; a
background thread named `gpui-store-debounce` keeps only the newest value and
writes it once the channel has been quiet for `DEBOUNCE_INTERVAL` (500 ms). The
calling thread never opens, writes or renames a file. `flush_debounced` waits
for the queued write, which is what shutdown and tests use. Dropping the `Store`
with a write still queued makes the worker write that last value before it
exits. A background write failure surfaces through `take_debounced_error`,
because a debounced save cannot return its IO error at call time.

**Why.** Settings change at input speed: dragging a slider, resizing a panel or
picking a colour emits many updates per second. A synchronous file write on the
render thread turns that into a visible stall. Coalescing means one write per
quiet interval, and the last value always wins. Only settings are debounced;
session and preset saves are explicit and direct.

### 3. Versioned

Every document has a `schemaVersion`. The loader rewrites `0` (a file from
before the crate versioned the document) to the current version. A file whose
version is *higher* than this build's is loaded as defaults, the version is
remembered per file, and every save of that document fails with `NewerSchema`
instead of downgrading it.

**Why.** Users run mixed builds — a beta beside a release, or a rolled-back
install. Without the guard an older build would open a newer file, ignore the
fields it does not know, and write the reduced document back, silently deleting
the newer build's data. The read-only state is per file, so one newer document
does not make the other two unwritable.

### 4. Corrupt recovery

A document that does not parse is renamed `<file>.corrupt-<unix-ts>` (with a
numeric suffix if that name is taken) and defaults are returned. The same
happens when the version probe fails. A read error — a missing file, or one the
process may not read — also returns defaults, and a load never writes, so
nothing is clobbered.

**Why.** An unparseable file is still the user's data and often the only copy of
it. Deleting it or overwriting it on the next save destroys both the
configuration and the evidence of what went wrong. Moving it aside lets the app
start with defaults while leaving the broken file available for recovery.

## API

```rust
pub fn open() -> Result<Store, StoreError>;
pub fn at(root: PathBuf) -> Store;

pub fn load_settings(&self) -> Settings;
pub fn save_settings(&self, settings: &Settings) -> Result<(), StoreError>;
pub fn save_settings_debounced(&self, settings: &Settings) -> Result<(), StoreError>;
pub fn flush_debounced(&self) -> Result<(), StoreError>;
pub fn take_debounced_error(&self) -> Option<StoreError>;

pub fn load_session(&self) -> Session;
pub fn save_session(&self, session: &Session) -> Result<(), StoreError>;

pub fn load_support_presets(&self) -> SupportPresets;
pub fn save_support_presets(&self, presets: &SupportPresets) -> Result<(), StoreError>;

pub fn is_read_only(&self, file: StoreFile) -> bool;
```

Loads return defaults rather than `Result`: being unable to read a preference
file is a normal first run or a recovered corruption, not an error the caller
can act on. Saves return `Result` because a failed write is actionable.

## Constraints

- **No dependencies on the UI or the supports crate.** `SupportPreset.settings`
  is `serde_json::Value`; the crate that owns the support tree validates it.
- **JSON is pretty-printed with a trailing newline.** The files are
  diff-friendly and hand-editable; they are not a cache.
- **Built-ins never round-trip.** A built-in preset is code, not a record, and
  is not written to `support-presets.json`.
- **One writer per document.** Atomic writes and the per-file read-only flag are
  the concurrency boundary; a second process writing the same file is outside
  the contract.

## Sources

- `gpui/crates/gpui-store/src/lib.rs` — the crate contract and module map.
- `gpui/crates/gpui-store/src/file.rs` — atomic write, version probe,
  quarantine.
- `gpui/crates/gpui-store/src/store.rs` — `Store`, `APP_DIR_NAME`, per-file
  read-only guards.
- `gpui/crates/gpui-store/src/debounce.rs` — the background settings writer.
- `gpui/crates/gpui-store/src/settings.rs`,
  `gpui/crates/gpui-store/src/session.rs`,
  `gpui/crates/gpui-store/src/presets.rs` — the three documents.
- `src-tauri/tauri.conf.json` — the identifier the directory is named after.
- `docs/dev/data-storage.md` — the React-era keys these documents replace.
