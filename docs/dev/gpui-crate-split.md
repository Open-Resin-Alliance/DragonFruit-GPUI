# GPUI crate split

The port starts as one binary crate. That makes every edit rebuild the whole
12k-line frontend and blocks any future hot-reload. The target is a workspace
of small crates, one per GUI system, so editing a panel rebuilds that panel and
the final link only.

## Dependency graph

```
gpui-app (bin)          window, render composition, main
├── gpui-app-bar
├── gpui-rail
├── gpui-panels
├── gpui-panel-content
├── gpui-overlays
├── gpui-settings
└── gpui-ui             atoms
    └── gpui-core       theme, icons, camera, stl, viewport, state
```

- `gpui-core` owns `DragonFruit`, `Mode`, `PanelLayout` (state only), the
  camera, the STL loader, the wgpu viewport, the generated theme and the icon
  set, plus `assets/icons` and `assets/fonts`.
- `gpui-ui` owns the atoms. It depends on `gpui-core` for tokens and icons.
- Each feature crate owns exactly one module's renderer and depends on
  `gpui-core` + `gpui-ui`. Feature crates never depend on each other.
- `gpui-app` is the only crate that implements `Render for DragonFruit` and
  composes the tree.

Module map:

| Crate | Modules |
|---|---|
| `gpui-core` | `theme`, `icons`, `camera`, `stl`, `viewport`, `window`, `fonts`, state (`DragonFruit`, `Mode`, `PanelLayout`) |
| `gpui-ui` | `ui`, `panel_fields` |
| `gpui-app-bar` | `app_bar`, `view_type` |
| `gpui-rail` | `rail` |
| `gpui-panels` | `panels` |
| `gpui-panel-content` | `panel_content`, `support_panels`, `prepare_stack`, `transform_panel`, `arrange_panels`, `smoothing_panel`, `organic_cut_panel` |
| `gpui-overlays` | `overlays`, `plate_widgets` |
| `gpui-settings` | `settings` |
| `gpui-app` | `main` |

## Rules that keep the graph acyclic

- **State lives in `gpui-core`.** A feature that needs a new field on
  `DragonFruit` adds it there. A feature that needs its own render state keeps
  it in the feature crate (thread-locals are acceptable for view-only state,
  as the rail's layout already does).
- **Content is passed in, not imported.** `gpui-panels` renders panel shells;
  the app passes `AnyElement` bodies built by `gpui-panel-content`.
- **No feature crate names another feature crate.**

## Migration steps

1. Create the workspace at `gpui/Cargo.toml` with `members = ["crates/*"]` and
   keep the binary package at the root.
2. Move `theme.rs`, `icons.rs`, `camera.rs`, `stl.rs`, `viewport.rs`,
   `window.rs` and the `DragonFruit`/`Mode`/`PanelLayout` definitions into
   `crates/gpui-core`; move `assets/` with them.
3. Move `ui.rs` into `crates/gpui-ui`.
4. Move each feature module into its own crate; replace `crate::` paths with
   `gpui_core::` / `gpui_ui::`.
5. Point `main.rs` at the crates; delete the old modules.
6. Verify with `cargo check` and a smoke run.

## Incremental-build target

After the split, editing `gpui-panel-content` should rebuild only that crate
plus the binary. The viewport (wgpu) and the theme stay untouched, which is
where most of the current rebuild time goes.
