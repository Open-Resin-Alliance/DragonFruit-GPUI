//! Behaviour the storage guarantees, exercised against a real temporary
//! directory.

use std::fs;
use std::path::Path;

use gpui_store::{
    CameraPose, CameraProjection, FloatPoint, HotkeyBinding, LogLevel, MeshShaderType,
    PanelDock, PanelPlacement, SessionLayout, Settings, Store, StoreError, StoreFile,
    SupportPreset, SupportPresets, ToolLayout,
};

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = Store::at(dir.path().to_path_buf());
    (dir, store)
}

fn names(dir: &Path) -> Vec<String> {
    let mut entries: Vec<String> = fs::read_dir(dir)
        .expect("read dir")
        .map(|entry| entry.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    entries
}

#[test]
fn defaults_when_the_files_are_missing() {
    let (_dir, store) = store();

    assert_eq!(store.load_settings(), Settings::default());
    assert_eq!(store.load_session(), gpui_store::Session::default());
    assert_eq!(store.load_support_presets(), SupportPresets::default());

    assert!(!store.is_read_only(StoreFile::Settings));
    assert!(!store.is_read_only(StoreFile::Session));
    assert!(!store.is_read_only(StoreFile::SupportPresets));
}

#[test]
fn settings_round_trip() {
    let (_dir, store) = store();

    let mut settings = Settings::default();
    settings.general.ui_scale = 1.25;
    settings.general.language = "de".to_owned();
    settings.general.tool_layout = ToolLayout::Vertical;
    settings.general.import_defaults.auto_repair = true;
    settings.camera.fov = 65.0;
    settings.camera.projection = CameraProjection::Perspective;
    settings.camera.trackpad.pan_acceleration = 3.5;
    settings.workspaces.higher_contrast_model_edges = true;
    settings.multi_plate.fixed_plate_columns = 5;
    settings.mesh.shader_type = MeshShaderType::Xray;
    settings.mesh.mesh_color = "#ff0000".to_owned();
    settings.performance.png_compression_strategy =
        gpui_store::PngCompressionStrategy::Smallest;
    settings.space_mouse.translation_sensitivity = 3.1;
    settings.plugins.allow_sideloaded_complex_plugins = true;
    settings.scene_autosave.debounce_ms = 60_000;
    settings.backups.auto_sync_minutes = 30;
    settings.uvtools.enabled = true;
    settings.ui.theme = gpui_store::ThemePreference::Light;
    settings.logging.min_level = LogLevel::Debug;
    settings.hotkeys.bindings.insert(
        "GLOBAL".to_owned(),
        [(
            "OPEN_SETTINGS".to_owned(),
            HotkeyBinding {
                key: "k".to_owned(),
                modifier: Some("cmd".to_owned()),
                description: "Open settings".to_owned(),
            },
        )]
        .into_iter()
        .collect(),
    );

    store.save_settings(&settings).expect("save");
    assert_eq!(store.load_settings(), settings);
}

#[test]
fn session_round_trip() {
    let (_dir, store) = store();

    let mut layout = SessionLayout::default();
    layout.panels.insert(
        "models".to_owned(),
        PanelPlacement {
            dock: PanelDock::Left,
            order: 1,
            width: 340.0,
            float: None,
        },
    );
    layout.panels.insert(
        "export".to_owned(),
        PanelPlacement {
            dock: PanelDock::Float,
            order: 0,
            width: 280.0,
            float: Some(FloatPoint { x: 24.0, y: 96.0 }),
        },
    );
    layout.left_dock_width = Some(340.0);

    let session = gpui_store::Session {
        last_file: Some("C:/models/part.voxl".into()),
        camera: Some(CameraPose {
            yaw: 1.0,
            pitch: 0.4,
            distance: 512.0,
            target: [1.0, 2.0, 3.0],
            fov_y: 0.8,
            orthographic: false,
        }),
        active_tool: 3,
        panel_layout: layout,
        ..gpui_store::Session::default()
    };

    store.save_session(&session).expect("save");
    assert_eq!(store.load_session(), session);
}

#[test]
fn support_presets_round_trip() {
    let (_dir, store) = store();

    let presets = SupportPresets {
        active_preset_id: Some("my-preset".to_owned()),
        presets: vec![SupportPreset {
            id: "my-preset".to_owned(),
            name: "My Preset".to_owned(),
            pinned_slot: Some(2),
            settings: serde_json::json!({ "tip": { "contactDiameterMm": 0.4 } }),
            ..SupportPreset::default()
        }],
        ..SupportPresets::default()
    };

    store.save_support_presets(&presets).expect("save");
    assert_eq!(store.load_support_presets(), presets);
}

#[test]
fn a_corrupt_file_is_renamed_and_defaults_are_returned() {
    let (dir, store) = store();
    fs::write(store.path(StoreFile::Settings), b"{ not json").expect("write corrupt");

    assert_eq!(store.load_settings(), Settings::default());

    let entries = names(dir.path());
    assert!(
        entries.iter().any(|name| name.starts_with("settings.json.corrupt-")),
        "corrupt file was not kept aside: {entries:?}"
    );
    assert!(
        !entries.iter().any(|name| name.as_str() == "settings.json"),
        "corrupt file is still in the way: {entries:?}"
    );

    // The quarantined copy is the user's original bytes, untouched.
    let quarantined = entries
        .iter()
        .find(|name| name.starts_with("settings.json.corrupt-"))
        .expect("quarantined file");
    assert_eq!(
        fs::read(dir.path().join(quarantined)).expect("read quarantine"),
        b"{ not json"
    );

    // And the store is writable again.
    store.save_settings(&Settings::default()).expect("save after quarantine");
}

#[test]
fn a_newer_schema_versions_file_is_never_overwritten() {
    let (dir, store) = store();
    let path = store.path(StoreFile::Settings);
    let newer = br#"{"schemaVersion": 7, "general": {"uiScale": 2.5}}"#;
    fs::write(&path, newer).expect("write newer");

    assert_eq!(store.load_settings(), Settings::default());
    assert!(store.is_read_only(StoreFile::Settings));
    assert_eq!(store.read_only_version(StoreFile::Settings), Some(7));

    let error = store.save_settings(&Settings::default()).expect_err("must refuse");
    assert!(matches!(
        error,
        StoreError::NewerSchema {
            file: StoreFile::Settings,
            found: 7,
            supported: 1,
        }
    ));

    assert_eq!(fs::read(&path).expect("file still there"), newer);
    assert_eq!(
        names(dir.path()),
        vec!["settings.json".to_owned()],
        "a refused save must not leave a .tmp or a rewrite behind"
    );
}

#[test]
fn an_older_file_loads_with_defaults_for_missing_fields() {
    let (_dir, store) = store();
    fs::write(
        store.path(StoreFile::Settings),
        br#"{"general": {"uiScale": 1.5}, "camera": {"fov": 70}}"#,
    )
    .expect("write older");

    let settings = store.load_settings();
    assert_eq!(settings.general.ui_scale, 1.5);
    assert_eq!(settings.camera.fov, 70.0);
    // Fields the old file never had are the source's defaults.
    assert_eq!(settings.general.language, "en");
    assert_eq!(settings.camera.trackpad.pan_acceleration, 2.0);
    assert_eq!(settings.mesh.ambient_intensity, 0.28);
    assert_eq!(settings.schema_version, 1);
}

#[test]
fn an_unknown_enum_value_falls_back_instead_of_failing_the_file() {
    let (_dir, store) = store();
    fs::write(
        store.path(StoreFile::Settings),
        br#"{"camera": {"projection": "isometric"}, "logging": {"minLevel": "verbose"}}"#,
    )
    .expect("write");

    let settings = store.load_settings();
    assert_eq!(settings.camera.projection, CameraProjection::Orthographic);
    assert_eq!(settings.logging.min_level, LogLevel::Info);
}

#[test]
fn an_atomic_write_leaves_no_temp_file() {
    let (dir, store) = store();
    store.save_settings(&Settings::default()).expect("save");
    store
        .save_session(&gpui_store::Session::default())
        .expect("save");
    store
        .save_support_presets(&SupportPresets::default())
        .expect("save");

    let leftovers: Vec<String> = names(dir.path())
        .into_iter()
        .filter(|name| name.ends_with(".tmp") || name.contains(".tmp."))
        .collect();
    assert!(leftovers.is_empty(), "temp files left behind: {leftovers:?}");

    assert_eq!(
        names(dir.path()),
        vec![
            "session.json".to_owned(),
            "settings.json".to_owned(),
            "support-presets.json".to_owned(),
        ]
    );
}

#[test]
fn debounced_save_waits_and_writes_the_newest_settings() {
    let (_dir, store) = store();

    let mut first = Settings::default();
    first.general.ui_scale = 1.1;
    let mut second = Settings::default();
    second.general.ui_scale = 1.75;

    store.save_settings_debounced(&first).expect("queue first");
    // Nothing has been written yet: the worker holds the edit for one interval.
    assert!(
        !store.path(StoreFile::Settings).exists(),
        "the debounced path wrote before its interval elapsed"
    );

    store.save_settings_debounced(&second).expect("queue second");
    store.flush_debounced().expect("flush");

    assert_eq!(store.load_settings().general.ui_scale, 1.75);
}

#[test]
fn a_dropped_store_still_writes_its_pending_settings() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().to_path_buf();

    {
        let store = Store::at(root.clone());
        let mut settings = Settings::default();
        settings.general.ui_scale = 1.5;
        store.save_settings_debounced(&settings).expect("queue");
    }

    // The dropped sender ends the worker; it commits the pending value.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let store = Store::at(root.clone());
        if store.path(StoreFile::Settings).exists()
            && store.load_settings().general.ui_scale == 1.5
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "pending debounced settings were lost when the store dropped"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}
