# GPUI backend map — the Tauri command surface

What every `#[tauri::command]` in `src-tauri/src/` does, what it needs from the
environment, and where it went in `gpui/crates/gpui-backend/`.

**131 commands**, declared across 12 files (17 modules count the non-command ones:
`mallinfo_shim`, `plugin_registry`, `plugin_sideload_encoder`,
`plugin_sideload_runtime`, `plugin_test_support`, `generated_builtin_plugins`,
`window_state`). `generate_handler!` in `main.rs` registers **132 entries** for
those 131 commands because `fetch_sponsor_avatar` is listed twice.

Per-declaring-file counts, which the sections below follow:

| File | Commands |
| --- | --- |
| `main.rs` | 58 |
| `mesh_repair.rs` | 43 |
| `mesh_refine.rs` | 1 |
| `mesh_minima.rs` | 5 |
| `overhang.rs` | 2 |
| `ao_vertex.rs` | 2 |
| `network.rs` | 2 |
| `experiments.rs` | 1 |
| `plugin_sideload.rs` | 9 |
| `updater_channel.rs` | 4 |
| `spacemouse/mod.rs` | 3 |
| `webview_watchdog.rs` | 1 |
| **Total** | **131** |

## Verdicts

| Verdict | Meaning | Count |
| --- | --- | --- |
| `pure` | Ported to `gpui-backend` as a function with no ambient dependency. | 89 |
| `context` | Ported, behind the `AppContext` trait: it resolved a directory from `app.path()` or drove the main window. | 20 |
| `ui` | Frontend-only: it opened an `rfd` file dialog. The GPUI app opens its own; nothing is ported. | 6 |
| `deferred` | Not ported, with the reason in the row. | 16 |
| **Ported** | `pure` + `context` | **109** |

## The four things a command actually needed from `AppHandle`

Reading all 131 handlers, the ambient dependency reduces to four — the whole
`AppContext` surface (`gpui-backend/src/context.rs`):

| `AppContext` member | Tauri equivalent | Commands needing it |
| --- | --- | --- |
| `data_dir()` | `app.path().app_data_dir()` | 5 autosave, 1 backup default, 2 update channel, 6 sideload = 14 |
| `log_dir()` | `app.path().app_log_dir()` | 3 (log viewer) |
| `emit(event, payload)` | `app.emit` / `window.emit` | 1 (scene handoff) |
| `window() → &dyn WindowHost` | `app.get_webview_window("main")` / `("splashscreen")` | 2 (focus, reveal) |

The two counting subtleties: one command can need two members (none do), and 12
commands took an `AppHandle` or a `Window` **solely** to `emit` progress —
`slice_solid_native_to_temp_path`, `run_island_scan_native` and the ten repair /
surfacing commands. Those are `pure` here because the engines' `*_with_progress`
entry points already take `Option<&mut dyn FnMut(&str, usize) -> bool>`; the GPUI
layer writes the event and reads its cancel flag inside that sink. Event names are
in the target column.

Two things are deliberately **not** on the trait:

* **A spawner.** Tauri's commands were `async` only to reach `spawn_blocking`.
  GPUI's `background_executor` is the same facility and already belongs to the app,
  so a spawner here would be a second threading policy with no caller. Every
  `gpui-backend` entry point is blocking and the UI decides what runs where.
* **Nothing for the deferred set.** Every deferred command needs a *dependency*
  (an extracted crate, or a plugin/updater subsystem), never more context. That is
  the short answer to "what `AppContext` surface do the deferred ones need": none.

## Commands

Declaring file, then registration order within it. `Target` is the
`gpui-backend` path for a ported command, or the reason for a deferral.

### `main.rs` (58)

| # | Command | What it does | Needs beyond its arguments | Verdict | Target / reason |
| --- | --- | --- | --- | --- | --- |
| 1 | `scene_file_begin_atomic_write` | Allocates `<target>.tmp-<pid>-<seq>` beside the target for an atomic save. | — | `pure` | `scene_file::scene_file_begin_atomic_write` |
| 2 | `scene_file_commit_atomic` | fsyncs the temp, then renames it over the target with three bounded retries. | — | `pure` | `scene_file::scene_file_commit_atomic` |
| 3 | `scene_file_discard_atomic_temp` | Deletes an abandoned commit temp, gated on the temp-name shape. | — | `pure` | `scene_file::scene_file_discard_atomic_temp` |
| 4 | `export_mesh_file` | Converts a raw triangle staging file into binary STL or 3MF. | The staging appender, which must be flushed and released first. | `pure` | `export::export_mesh_file` (+ `write_binary_stl`, `write_3mf`) |
| 5 | `stage_mesh_binary_start` | Resets the staging buffer and pre-allocates 16 MiB..1 GiB for an in-memory upload. | — | `pure` | `staging::MeshStaging::stage_mesh_binary_start` |
| 6 | `allocate_mesh_stage_path` | Allocates a fresh `dragonfruit-mesh-stage-<nanos>.bin` in the staging directory. | Env `DF_MESH_STAGE_DIR` (optional). | `pure` | `staging::allocate_mesh_stage_path` |
| 7 | `append_mesh_stage_chunk` | Appends a raw chunk to a staging file; refuses two interleaved writers. | Raw IPC body + `x-mesh-stage-path` (percent-encoded) + `x-mesh-stage-offset` headers. | `pure` | `staging::MeshStaging::append_mesh_stage_chunk` — the adapter decodes the headers into `(path, bytes, is_first_chunk)` |
| 8 | `finish_mesh_stage_write` | Flushes and releases the appender for a path; returns bytes written. | — | `pure` | `staging::MeshStaging::finish_mesh_stage_write` |
| 9 | `stage_mesh_file_path` | Points the staging buffer at a file already on disk; returns its length. | — | `pure` | `staging::MeshStaging::stage_mesh_file_path` |
| 10 | `stage_mesh_binary_set` | Replaces the buffer with one complete payload; returns an upload ack. | Raw IPC body. | `pure` | `staging::MeshStaging::stage_mesh_binary_set` |
| 11 | `stage_mesh_binary_chunk` | Appends one chunk to the in-memory buffer; returns an upload ack. | Raw IPC body. | `pure` | `staging::MeshStaging::stage_mesh_binary_chunk` |
| 12 | `slice_solid_native_to_temp_path` | Slices the staged mesh to a file through `dragonfruit-slicing-engine`, with perf counters. | `Window` for `window.emit("slicer://progress")`; the slicer's rayon pool and cancel flag. | `pure` | `slicing::Slicer::slice_solid_native_to_temp_path` — progress becomes a `SliceSink`; pool and 8 ms throttle kept |
| 13 | `cancel_slicing` | Flips the slicer's cancel flag. | Shared `Arc<AtomicBool>`. | `pure` | `slicing::Slicer::cancel_slicing` |
| 14 | `run_island_scan_native` | Streaming island scan over the staged soup, reshaped into the overlay's `ScanResults` (RLE labels, first/last hit, comp base/top). | `Window` for `window.emit("islandscan://progress")`. | `pure` | `islands::run_island_scan` — progress becomes `Option<&(dyn Fn(u32,u32) + Sync)>` |
| 15 | `local_backup_default_directory` | `<app data>/backups`, created if needed. | `app.path().app_data_dir()`. | `context` | `backup::local_backup_default_directory` — needs `AppContext::data_dir` |
| 16 | `local_backup_pick_directory` | Folder picker for the backup root. | `rfd` folder dialog. | `ui` | Dropped; the GPUI app opens its own folder dialog |
| 17 | `local_backup_read_state` | Reads `state.json` from the backup folder. | — | `pure` | `backup::local_backup_read_state` |
| 18 | `local_backup_sync` | Pretty-prints the document into `state.json` plus a timestamped `history/<id>.json`. | — | `pure` | `backup::local_backup_sync` |
| 19 | `local_backup_list_history` | Lists history entries, newest first. | — | `pure` | `backup::local_backup_list_history` |
| 20 | `local_backup_read_history_item` | Reads one history entry. | — | `pure` | `backup::local_backup_read_history_item` |
| 21 | `local_backup_delete_history_item` | Deletes one history entry; `false` when already gone. | — | `pure` | `backup::local_backup_delete_history_item` |
| 22 | `local_backup_restore_history_item` | Copies a history entry back over `state.json`. | — | `pure` | `backup::local_backup_restore_history_item` |
| 23 | `scene_autosave_get_paths` | Resolves where an autosave tick lands (sidecar or generic) and why; sweeps stale commit temps. | `app.path().app_data_dir()`. | `context` | `autosave::scene_autosave_get_paths` — needs `AppContext::data_dir` |
| 24 | `scene_autosave_write_manifest` | Writes the recovery manifest after a payload commit; optionally deletes the previous payload. | App data dir. | `context` | `autosave::scene_autosave_write_manifest` — the ten arguments become `SceneAutosaveManifestWrite` |
| 25 | `scene_autosave_delete_sidecar` | Deletes the sidecar for a project (Save As calls it on the old path). | — | `pure` | `autosave::scene_autosave_delete_sidecar` |
| 26 | `scene_autosave_read_manifest` | Reads the manifest, if any. | App data dir. | `context` | `autosave::scene_autosave_read_manifest` |
| 27 | `scene_autosave_resolve_recovery` | Finds a recoverable payload (manifest → sidecar → generic → legacy), VOXL magic checked. | App data dir. | `context` | `autosave::scene_autosave_resolve_recovery` |
| 28 | `scene_autosave_read_voxl_bytes` | Reads a recovery payload, gated on the resolver's own candidate list. | App data dir. | `context` | `autosave::scene_autosave_read_voxl_bytes` |
| 29 | `save_print_file` | Save dialog, then writes the supplied bytes. | `rfd` save dialog. | `ui` | Dropped; the app opens its own dialog and calls `files::write_bytes_to_path` |
| 30 | `save_print_file_from_path` | Save dialog, then copies an existing file. | `rfd` save dialog. | `ui` | Dropped; the copy is `std::fs::copy` in the app |
| 31 | `pick_save_path` | Save dialog with filters; returns the chosen path. | `rfd` save dialog. | `ui` | Dropped |
| 32 | `write_bytes_to_path` | Creates the parent directory and writes bytes to a chosen path. | — | `pure` | `files::write_bytes_to_path` |
| 33 | `pick_open_files` | Open dialog with mesh/scene filters; returns paths and names. | `rfd` open dialog; the built-in plugin extension list. | `ui` | Dropped; `system::is_scene_file_path` and `BUILTIN_PLUGIN_SCENE_FILE_EXTENSIONS` build the filters |
| 34 | `get_launch_scene_files` | Scene files this process was launched with ("open with"). | `std::env::args`. | `pure` | `system::get_launch_scene_files` |
| 35 | `get_slicer_engine_version` | The slicing engine's version string. | — | `pure` | `system::get_slicer_engine_version` |
| 36 | `notify_launch_scene_handoff` | Emits `dragonfruit://scene-file-handoff` for the launch files. | `AppHandle` to `emit`. | `context` | `system::notify_launch_scene_handoff` — needs `AppContext::emit` |
| 37 | `focus_main_window_command` | Shows, un-minimizes and focuses the main window. | `AppHandle` → `get_webview_window("main")`. | `context` | `system::focus_main_window_command` — needs `AppContext::window` |
| 38 | `reveal_main_window_command` | Shows the main window **without** `set_focus` (Windows' focus-stealing sound) and closes the splash. | `AppHandle` → main and splash windows. | `context` | `system::reveal_main_window_command` — needs `WindowHost::show` / `close_splash` |
| 39 | `read_print_file_bytes` | Reads a sliced container whole. | — | `pure` | `files::read_print_file_bytes` |
| 40 | `read_print_file_size` | Reads a container's length. | — | `pure` | `files::read_print_file_size` |
| 41 | `read_print_file_chunk` | Reads one chunk, capped at 8 MiB. | — | `pure` | `files::read_print_file_chunk` |
| 42 | `read_print_layer_png` | One layer preview PNG, decoded by the engine. | — | `pure` | `files::read_print_layer_png` |
| 43 | `read_print_layer_pngs` | A run of layer previews decoded in parallel, packed as `[u32 len][bytes]`. | `rayon`. | `pure` | `files::read_print_layer_pngs` |
| 44 | `existing_paths` | Which of these paths exist, in order. | — | `pure` | `files::existing_paths` (no `Result`: nothing can fail) |
| 45 | `delete_print_temp_file` | Deletes one slicer temp artifact, refusing anything else. | — | `pure` | `files::delete_print_temp_file` |
| 46 | `cleanup_stale_print_temp_files` | Sweeps temp artifacts older than N seconds (floor 60 s). | — | `pure` | `files::cleanup_stale_print_temp_files` |
| 47 | `cleanup_all_print_temp_files` | Sweeps every temp artifact. | — | `pure` | `files::cleanup_all_print_temp_files` |
| 48 | `reveal_in_file_manager` | Opens the containing folder with the file selected (`explorer /select,` / `open -R` / `xdg-open`). | — | `pure` | `system::reveal_in_file_manager` |
| 49 | `discover_uvtools_path` | Finds UVTools at a candidate path (with `~/` expansion) or on PATH. | — | `pure` | `system::discover_uvtools_path` |
| 50 | `open_external_url` | Hands a URL to the default browser. | — | `pure` | `system::open_external_url` |
| 51 | `launch_external_process` | Starts an executable with a file argument; on macOS via `open -n -a … --args`. | — | `pure` | `system::launch_external_process` |
| 52 | `fetch_sponsor_avatar` | Fetches a sponsors avatar through Rust (CORS bypass) as a `data:` URL; https + four-host allowlist. | `reqwest`. Registered **twice** in `generate_handler!`, which is why 131 commands produce 132 registration entries. | `pure` | `net::fetch_sponsor_avatar` |
| 53 | `fetch_github_plugin_resource` | Fetches a plugin manifest or preset from GitHub; returns status + base64 body; https + three-host allowlist. | `reqwest`. | `pure` | `net::fetch_github_plugin_resource` |
| 54 | `fetch_external_text` | Fetches the sponsors list as text; https + opencollective hosts only. | `reqwest`. | `pure` | `net::fetch_external_text` |
| 55 | `set_log_level_pref` | Validates the level, applies it with `log::set_max_level`, persists it. | The env-derived preference path (deliberately not the app resolver). | `pure` | `log_prefs::set_log_level_pref` |
| 56 | `read_log_tail` | Last N lines of `dragonfruit.log` for the settings viewer; clamped 10..10 000. | `app.path().app_log_dir()`. | `context` | `log_prefs::read_log_tail` — needs `AppContext::log_dir` |
| 57 | `open_log_file` | Opens the log in the OS text viewer. | App log dir. | `context` | `log_prefs::open_log_file` |
| 58 | `delete_log_file` | Deletes the log; a missing file is success. | App log dir. | `context` | `log_prefs::delete_log_file` |

### `mesh_repair.rs` (43)

| # | Command | What it does | Needs beyond its arguments | Verdict | Target / reason |
| --- | --- | --- | --- | --- | --- |
| 59 | `mesh_analyze_from_path` | Loads a mesh file and returns its `MeshAnalysis` JSON. | — | `pure` | `repair::mesh_analyze_from_path` |
| 60 | `mesh_repair_from_path` | Loads, repairs, replaces the staged mesh, returns the health report. | `AppHandle` for `mesh-repair-progress`. | `pure` | `repair::mesh_repair_from_path` — sink + `CancelRegistry` |
| 61 | `mesh_repair_parts_from_path` | Repairs a plate object by object; stages the joined result with a per-object verdict. | `AppHandle` for `mesh-repair-progress`. | `pure` | `repair::mesh_repair_parts_from_path` |
| 62 | `mesh_load_from_path` | Loads and stages a mesh, repairing nothing. | — | `pure` | `repair::mesh_load_from_path` |
| 63 | `mesh_repair_cancel` | Stops the repair carrying a token at its next stage boundary. | Token → flag map. | `pure` | `repair::CancelRegistry::cancel` |
| 64 | `mesh_repair_staged` | Repairs the staged soup in place. | `AppHandle` for `mesh-repair-progress`. | `pure` | `repair::mesh_repair_staged` |
| 65 | `mesh_classify_staged` | Model/support classifier only; reorders the staged geometry. | — | `pure` | `repair::mesh_classify_staged` |
| 66 | `mesh_analyze_staged` | Analyses the staged soup without modifying it, for a health check before repairing. | — | `pure` | `repair::mesh_analyze_staged` |
| 67 | `mesh_hollow_staged` | Voxel hollowing of the staged mesh; stashes the cavity mesh. | — | `pure` | `repair::mesh_hollow_staged` |
| 68 | `mesh_shrinkwrap_staged` | Resamples the staged mesh from a signed distance field. | `AppHandle` for `mesh-shrinkwrap-progress`. | `pure` | `repair::mesh_shrinkwrap_staged` |
| 69 | `mesh_tet_surface_staged` | Resamples by tetrahedralising the model's own points. | `AppHandle` for `mesh-tet-surface-progress`. | `pure` | `repair::mesh_tet_surface_staged` |
| 70 | `mesh_solid_remesh_staged` | Rebuilds the surface by resolving the mesh into a solid. | `AppHandle` for `mesh-solid-remesh-progress`. | `pure` | `repair::mesh_solid_remesh_staged` |
| 71 | `mesh_hollow_preview_capture_staged_source` | Captures the staged mesh as the non-mutating preview source. | Preview buffers. | `pure` | `repair::mesh_hollow_preview_capture_staged_source` |
| 72 | `mesh_hollow_preview_from_captured_source` | Hollows the captured source; stashes positions, cavity, infill and removed/blocked voxels. | Cached `HollowSession`. | `pure` | `repair::mesh_hollow_preview_from_captured_source` |
| 73 | `mesh_hollow_apply_from_captured_source` | Commits the preview's hollowing to the staged mesh. | Cached `HollowSession`. | `pure` | `repair::mesh_hollow_apply_from_captured_source` |
| 74 | `mesh_hollow_preview_read_positions` | Raw preview positions (LE `f32`). | Preview buffers. | `pure` | `repair::mesh_hollow_preview_read_positions` |
| 75 | `mesh_hollow_preview_read_infill_positions` | Raw preview infill positions. | Preview buffers. | `pure` | `repair::mesh_hollow_preview_read_infill_positions` |
| 76 | `mesh_hollow_preview_read_removed_voxel_centers` | Raw removed-voxel centres. | Preview buffers. | `pure` | `repair::mesh_hollow_preview_read_removed_voxel_centers` |
| 77 | `mesh_hollow_preview_read_removed_voxel_indices` | Raw removed-voxel grid indices. | Preview buffers. | `pure` | `repair::mesh_hollow_preview_read_removed_voxel_indices` |
| 78 | `mesh_hollow_preview_read_blocked_voxel_centers` | Raw blocked-voxel centres. | Preview buffers. | `pure` | `repair::mesh_hollow_preview_read_blocked_voxel_centers` |
| 79 | `mesh_hollow_preview_read_blocked_voxel_indices` | Raw blocked-voxel grid indices. | Preview buffers. | `pure` | `repair::mesh_hollow_preview_read_blocked_voxel_indices` |
| 80 | `mesh_hollow_preview_select_removed_voxels_in_polygon` | Lasso selection of removed voxels by projected screen point. | Cached session. | `pure` | `repair::mesh_hollow_preview_select_removed_voxels_in_polygon` |
| 81 | `mesh_hollow_preview_read_cavity_positions` | Raw preview cavity mesh. | Preview buffers. | `pure` | `repair::mesh_hollow_preview_read_cavity_positions` |
| 82 | `mesh_hollow_staged_read_cavity_positions` | Raw cavity mesh from the last staged hollow. | Preview buffers. | `pure` | `repair::mesh_hollow_staged_read_cavity_positions` |
| 83 | `mesh_punch_staged` | Applies cylindrical hole punches to the staged mesh. | — | `pure` | `repair::mesh_punch_staged` |
| 84 | `mesh_punch_capture_staged_source` | Captures the staged mesh for repeated punch previews. | Preview buffers. | `pure` | `repair::mesh_punch_capture_staged_source` |
| 85 | `mesh_punch_from_captured_source` | Punches the captured source without mutating the staged mesh. | Preview buffers. | `pure` | `repair::mesh_punch_from_captured_source` |
| 86 | `mesh_punch_read_positions` | Raw punch preview positions. | Preview buffers. | `pure` | `repair::mesh_punch_read_positions` |
| 87 | `mesh_organic_cut_staged` | Cuts the staged mesh; stashes every part and stages the first. | Cut buffers. | `pure` | `organic_cut::mesh_organic_cut_staged` |
| 88 | `mesh_organic_cut_capture_staged_source` | Captures the staged mesh as the cut source; clears parts and the solver. | Cut buffers. | `pure` | `organic_cut::mesh_organic_cut_capture_staged_source` |
| 89 | `mesh_organic_cut_from_captured_source` | Cuts the captured source without mutating the staged mesh. | Cut buffers. | `pure` | `organic_cut::mesh_organic_cut_from_captured_source` |
| 90 | `mesh_organic_cut_read_part` | Raw `f32` soup of part `index`. | Cut buffers. | `pure` | `organic_cut::mesh_organic_cut_read_part` |
| 91 | `mesh_organic_cut_geodesic_loop` | Surface-following seam through the waypoints; returns `{"pointCount":N}`. | Cached `GeodesicSolver`. | `pure` | `organic_cut::mesh_organic_cut_geodesic_loop` |
| 92 | `mesh_organic_cut_geodesic_loop_bytes` | The same loop as raw bytes (one round trip per drag frame). | Cached solver. | `pure` | `organic_cut::mesh_organic_cut_geodesic_loop_bytes` |
| 93 | `mesh_organic_cut_read_geodesic` | Raw last loop polyline. | Cut buffers. | `pure` | `organic_cut::mesh_organic_cut_read_geodesic` |
| 94 | `mesh_organic_cut_membrane_preview` | Builds the cutter surface for the loop (and the tenon) for preview. | Cut buffers. | `pure` | `organic_cut::mesh_organic_cut_membrane_preview` |
| 95 | `mesh_organic_cut_plane_tenon_preview` | The registration tenon a flat cut would place, framed on the plane. | Cut buffers. | `pure` | `organic_cut::mesh_organic_cut_plane_tenon_preview` |
| 96 | `mesh_organic_cut_read_tenon` | Raw last tenon+mortise soup. | Cut buffers. | `pure` | `organic_cut::mesh_organic_cut_read_tenon` |
| 97 | `mesh_organic_cut_read_membrane` | Raw last membrane soup. | Cut buffers. | `pure` | `organic_cut::mesh_organic_cut_read_membrane` |
| 98 | `mesh_repair_read_positions` | Raw staged positions for the renderer to hydrate. | — | `pure` | `repair::mesh_repair_read_positions` |
| 99 | `load_mesh_file` | Parses `stl`/`obj`/`3mf` into a DFMX payload; classifies single bodies; decimates an oversized STL first. | — | `pure` | `mesh_io::load_mesh_file` |
| 100 | `load_mesh_bytes` | The same for a byte source (a VOXL's chunk, a file out of a zip). | `x-mesh-classify` / `x-mesh-refine` headers. | `pure` | `mesh_io::load_mesh_bytes` — headers become bools |
| 101 | `load_mesh_range` | Reads and (optionally) inflates a range inside a file, then parses it — a VOXL's embedded mesh. Registers the single body for the AO bake. | Mesh registry. | `pure` | `mesh_io::load_mesh_range` |

### `mesh_refine.rs` (1)

| # | Command | What it does | Needs beyond its arguments | Verdict | Target / reason |
| --- | --- | --- | --- | --- | --- |
| 102 | `refine_mesh_soup` | Splits coarse faces on geometry an importer built outside the native loaders (the LYS case). | — | `pure` | `repair::refine_mesh_soup` |

### `mesh_minima.rs` (5)

| # | Command | What it does | Needs beyond its arguments | Verdict | Target / reason |
| --- | --- | --- | --- | --- | --- |
| 103 | `scan_mesh_minima` | Local vertical minima from a world-space soup. | — | `deferred` | The algorithm lives in `src-tauri/src/mesh_minima.rs`, not a `rust/` crate: `scan_minima_internal` with its sonar thread-local, `snap_to_surface`, `is_point_inside_mesh`, `load_and_transform_mesh` and `scan_islands_from_mesh` have nothing to delegate to. Porting means extracting them into a crate first. Nothing needed on `AppContext` — the two `AppHandle` uses are `emit`, which is a sink. |
| 104 | `scan_mesh_minima_from_path` | The same from a file, after the frontend-parity centring and scene matrix. | — | `deferred` | The algorithm lives in `src-tauri/src/mesh_minima.rs`, not a `rust/` crate: `scan_minima_internal` with its sonar thread-local, `snap_to_surface`, `is_point_inside_mesh`, `load_and_transform_mesh` and `scan_islands_from_mesh` have nothing to delegate to. Porting means extracting them into a crate first. Nothing needed on `AppContext` — the two `AppHandle` uses are `emit`, which is a sink. |
| 105 | `scan_voxel_islands_from_path` | Voxel islands with per-island contact voxels. | — | `deferred` | The algorithm lives in `src-tauri/src/mesh_minima.rs`, not a `rust/` crate: `scan_minima_internal` with its sonar thread-local, `snap_to_surface`, `is_point_inside_mesh`, `load_and_transform_mesh` and `scan_islands_from_mesh` have nothing to delegate to. Porting means extracting them into a crate first. Nothing needed on `AppContext` — the two `AppHandle` uses are `emit`, which is a sink. |
| 106 | `scan_islands_from_path` | Combined scan: voxel islands + minima islands + overhang islands + stability. | `AppHandle` for progress. | `deferred` | The algorithm lives in `src-tauri/src/mesh_minima.rs`, not a `rust/` crate: `scan_minima_internal` with its sonar thread-local, `snap_to_surface`, `is_point_inside_mesh`, `load_and_transform_mesh` and `scan_islands_from_mesh` have nothing to delegate to. Porting means extracting them into a crate first. Nothing needed on `AppContext` — the two `AppHandle` uses are `emit`, which is a sink. |
| 107 | `scan_islands_staged` | The same combined scan against the staged soup. | `AppHandle` for progress. | `deferred` | The algorithm lives in `src-tauri/src/mesh_minima.rs`, not a `rust/` crate: `scan_minima_internal` with its sonar thread-local, `snap_to_surface`, `is_point_inside_mesh`, `load_and_transform_mesh` and `scan_islands_from_mesh` have nothing to delegate to. Porting means extracting them into a crate first. Nothing needed on `AppContext` — the two `AppHandle` uses are `emit`, which is a sink. |

### `overhang.rs` (2)

| # | Command | What it does | Needs beyond its arguments | Verdict | Target / reason |
| --- | --- | --- | --- | --- | --- |
| 108 | `scan_overhangs` | Classifies overhang regions from a world-space soup and logs the topple report. | — | `deferred` | The classifier is `src-tauri/src/overhang.rs` (2.4k lines: region clustering, projected footprint masks, drag-moment ranking, stability report). No crate to call. Nothing needed on `AppContext`. |
| 109 | `scan_overhangs_staged` | The same, consuming the staged soup. | — | `deferred` | The classifier is `src-tauri/src/overhang.rs` (2.4k lines: region clustering, projected footprint masks, drag-moment ranking, stability report). No crate to call. Nothing needed on `AppContext`. |

### `ao_vertex.rs` (2)

| # | Command | What it does | Needs beyond its arguments | Verdict | Target / reason |
| --- | --- | --- | --- | --- | --- |
| 110 | `ao_available` | Discrete-GPU gate for the bake. | — | `pure` | `ao::ao_available` |
| 111 | `bake_vertex_occlusion` | Per-vertex ambient occlusion, by mesh token or by soup. | GPU adapter; the loader's mesh registry. | `pure` | `ao::bake_vertex_occlusion` / `ao::bake_vertex_occlusion_soup` |

### `network.rs` (2)

| # | Command | What it does | Needs beyond its arguments | Verdict | Target / reason |
| --- | --- | --- | --- | --- | --- |
| 112 | `plugin_network_request` | Dispatches a plugin's network request through the plugin host (built-in plugins, then sideloaded ones). | The plugin registry and the sideloaded runtime's process transport. | `deferred` | `plugin_registry::dispatch_network_request` routes into `plugin_sideload_runtime` (1.3k lines of sidecar IPC) and the generated built-in plugin modules. Deferred with the plugin host, not for lack of context. |
| 113 | `ensure_rtsp_relay` | Native RTSP relay status, starting it if needed. | — | `pure` | `net::ensure_rtsp_relay` (delegates to `dragonfruit-rtsp-relay`) |

### `experiments.rs` (1)

| # | Command | What it does | Needs beyond its arguments | Verdict | Target / reason |
| --- | --- | --- | --- | --- | --- |
| 114 | `set_experiment_overrides` | Replaces the user's experiment override map. | `State<ExperimentsState>`. | `pure` | `experiments::Experiments::set_overrides` — the map is owned state now |

### `plugin_sideload.rs` (9)

| # | Command | What it does | Needs beyond its arguments | Verdict | Target / reason |
| --- | --- | --- | --- | --- | --- |
| 115 | `sideloaded_plugin_allowance` | Whether complex sideloaded plugins are allowed. | App data dir (`plugins-sideloaded/settings.json`). | `context` | `plugins::sideloaded_plugin_allowance` |
| 116 | `sideloaded_plugin_set_allowance` | Sets that switch. | App data dir. | `context` | `plugins::sideloaded_plugin_set_allowance` |
| 117 | `sideloaded_plugin_pick` | `.dfplugin` file picker. | `rfd` dialog. | `ui` | Dropped |
| 118 | `sideloaded_plugin_inspect` | Verifies a package and reports its manifest, publisher, and whether it replaces an install. | App data dir; the running app's version for the `minAppVersion` check. | `context` | `plugins::sideloaded_plugin_inspect` — takes `app_version` explicitly, because this crate's `CARGO_PKG_VERSION` is not the application's |
| 119 | `sideloaded_plugin_install` | Verifies, unseals, extracts and atomically installs a package, then registers its encoder formats. | App data dir, install lock, **and** `plugin_sideload_encoder::register_installed_encoders`. | `deferred` | The install itself is portable, but it ends by registering the package's encoder formats with the slicing engine — `src-tauri/src/plugin_sideload_encoder.rs`, 781 lines, which must be ported first or the plugin installs unusable. No new `AppContext` member. |
| 120 | `sideloaded_plugin_import_file` | Runs a sideloaded importer over a dropped file and returns the scene container it wrote. | App data dir **and** the runtime's shared-memory sidecar transport. | `deferred` | `src-tauri/src/plugin_sideload_runtime.rs` (1.3k lines: process spawn, shm ring, protocol). No new `AppContext` member — it takes the plugin root and raw bytes. |
| 121 | `sideloaded_plugin_asset` | One preset image from a package as a `data:` URL, confined to `payload/assets/`, images only, ≤2.5 MB. | App data dir. | `context` | `plugins::sideloaded_plugin_asset` |
| 122 | `sideloaded_plugin_list` | Every installed package, by display name. | App data dir. | `context` | `plugins::sideloaded_plugin_list` |
| 123 | `sideloaded_plugin_remove` | Deletes an installed package's directory. | App data dir, install lock. | `context` | `plugins::sideloaded_plugin_remove` |

### `updater_channel.rs` (4)

| # | Command | What it does | Needs beyond its arguments | Verdict | Target / reason |
| --- | --- | --- | --- | --- | --- |
| 124 | `check_updates` | Builds the plugin updater against the channel's endpoint and caches the result. | `tauri-plugin-updater` state via `app_handle.updater_builder()`. | `deferred` | The command *is* the plugin: endpoint selection, signature verification, `Update` handle caching. `updater.rs` records the endpoints for whoever writes the replacement. Linux is a special case (Flatpak updates itself). |
| 125 | `perform_update` | Downloads and installs the cached update, reporting progress through an IPC channel. | `tauri-plugin-updater`; `tauri::ipc::Channel`. | `deferred` | Same dependency; it also launches the platform installer and exits the app. |
| 126 | `get_saved_update_channel` | Reads `stable`/`dev` from the app data dir, defaulting to `stable`. | App data dir. | `context` | `updater::get_saved_update_channel` |
| 127 | `save_update_channel` | Validates and writes the channel preference. | App data dir. | `context` | `updater::save_update_channel` |

### `spacemouse/mod.rs` (3)

| # | Command | What it does | Needs beyond its arguments | Verdict | Target / reason |
| --- | --- | --- | --- | --- | --- |
| 128 | `spacemouse_native_start` | Starts the navlib bridge and returns the driver path it loaded. | The 3Dconnexion navlib FFI. | `deferred` | The bridge is `src-tauri/src/spacemouse/` — deliberately isolated platform FFI (`navlib_sys.rs`, per-OS load), not a `rust/` crate. Porting means extracting it into one so both shells can use it. No context needed. |
| 129 | `spacemouse_native_stop` | Tears the bridge down. | The 3Dconnexion navlib FFI. | `deferred` | The bridge is `src-tauri/src/spacemouse/` — deliberately isolated platform FFI (`navlib_sys.rs`, per-OS load), not a `rust/` crate. Porting means extracting it into one so both shells can use it. No context needed. |
| 130 | `spacemouse_native_sync` | One frame of the JS↔navlib camera bridge. | The 3Dconnexion navlib FFI. | `deferred` | The bridge is `src-tauri/src/spacemouse/` — deliberately isolated platform FFI (`navlib_sys.rs`, per-OS load), not a `rust/` crate. Porting means extracting it into one so both shells can use it. No context needed. |

### `webview_watchdog.rs` (1)

| # | Command | What it does | Needs beyond its arguments | Verdict | Target / reason |
| --- | --- | --- | --- | --- | --- |
| 131 | `webview_heartbeat` | Records a webview liveness ping and dismisses the reload dialog if it is open. | `AppHandle`, `State<WebviewLiveness>`, and a macOS `NSAlert` sheet. | `deferred` | The watchdog exists to detect a wedged *webview*. GPUI has none; the state machine plus its `objc2` sheet have no consumer. No context needed. |

## How completeness was checked

The rows were generated from the source, not typed: every file under
`src-tauri/src` was scanned for `#[tauri::command]` followed by a `fn`, giving 131
names, and each name appears exactly once above (the generator asserts the count).
To re-check:

```sh
grep -rn -A2 '#\[tauri::command\]' src-tauri/src --include=*.rs \
  | grep -oE 'fn [a-z0-9_]+' | sort -u | wc -l   # 131
```

Registration is the second check: `generate_handler!` in `main.rs` lists 132
entries, and `fetch_sponsor_avatar` is the only name there twice.

## Behaviour that deliberately did not change

* **Error strings are the contract.** The renderer matched on them, so every ported
  function reproduces them verbatim — including the shell's odd ones: the solid
  remesh parser says `"invalid shrink-wrap options JSON"`, and a plain load failure
  says `"…task panicked"` because that is what the shell's `spawn_blocking` join
  error path said.
* **Guards are kept.** The commit-temp name check on discard; the VOXL magic check
  and candidate list on recovery reads; the `dragonfruit-slice-` prefix *and* temp
  dir check on delete; the `_autosave` marker on every autosave deletion; the
  `payload/assets/` confinement and image-type check on plugin assets; the `https`
  + host allowlists on all three fetches; the corner-count check on a mesh token;
  the 13-digit history id; the 8 MiB chunk cap; the 60 s sweep floor; the
  10..10 000 log-tail clamp.
* **Serialized shapes are kept**, including the ones that are not camelCase:
  `GithubPluginResource` stays `status` / `content_type` / `body_base64` (the shell's
  struct had no `rename_all`), and the organic-cut preview objects stay hand-built
  JSON with the same keys (`triangleCount`, `tenonKind`, `tenonFits`,
  `tenonFrame.{anchor,axis,u,v,tip,depth,maxTiltRad,halfDiagMm}`).
* **Process-wide state became owned state**, so a headless test can have its own:
  the staging buffer (`MeshStaging`), cancel flags (`CancelRegistry`), the AO mesh
  registry (`MeshRegistry`), the hollow and punch previews (`HollowPreview`,
  `PunchPreview`) and the cut buffers (`OrganicCutState`). The one `static` kept is
  the slicer's named rayon pool — its thread count and thread names are a measured
  performance contract — and the plugin store's install lock, because the directory
  is process-wide.

## Tests

`cargo test -p gpui-backend` (or `cargo test --manifest-path
gpui/crates/gpui-backend/Cargo.toml` while the crate is standalone). The suite
drives the ported behaviour without a window or a GPU:

* staging round trips, chunked file uploads, the interleaved-writer refusal, the
  pre-allocation clamp;
* atomic scene commit / discard, the discard name gate, temp-name uniqueness;
* STL and 3MF output (header, triangle count, ZIP members, the model XML), and the
  "unsupported format leaves the stage alone" case;
* print-file reads, the chunk clamp, the temp-artifact sweep age gate, the delete
  refusal on a non-artifact, `existing_paths` ordering;
* DFMX header/version/body layout, the zlib body round trip, unknown chunk
  compression, mesh-token registration and the corner-count guard, the registry cap;
* repair: staged analysis, `refine_mesh_soup`'s output layout, classify, hollow
  preview lifecycle, punch preview lifecycle, cancel tokens, missing-file errors;
* organic cut: empty previews, malformed request fallback, capture resets, the
  flat-tenon empty answers;
* island scan geometry derivation on a 10 mm cube;
* autosave: the three invariants over every project-path shape, the sidecar
  fallback reasons, the deletion gate, manifest round trip and tolerance for a
  short manifest, failure honesty, discard, and the recovery read gate;
* backup: id validation, sync/list/read/restore, traversal refusal;
* sideload: version comparison, asset-path sanitisation, mime mapping, allowance
  round trip, installed-package listing and asset data URL;
* updater channel, log preferences, experiments gating, network allowlists,
  UVTools discovery, launch-argument filtering.
