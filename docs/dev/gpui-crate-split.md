# GPUI crate and module split

The port is a workspace with two library crates and one binary. Everything the
GUI shares goes in a crate; everything that composes the window into a tree
stays in the binary, grouped by area.

## Layout

```
gpui/                       the binary: window bootstrap + the render tree
├── src/main.rs             module declarations, App, main()
├── src/app.rs              DragonFruit: state, actions, `Render`
├── src/chrome/             app_bar, rail, overlays        (the window frame)
├── src/panels/             list, content, empty_scene      (the panel column)
├── src/features/           settings, scene_modals, support_panels, tool_panels,
│                           transform_panel, arrange_panels, smoothing_panel,
│                           organic_cut_panel, auto_support_settings, printing_preview
├── src/widgets/            plate_widgets, prepare_stack, notifications
└── crates/
    ├── gpui-core/          theme, icons, fonts, camera, stl, viewport, cube,
    │                       window, log, state, support_state, empty_state,
    │                       panel_layout
    └── gpui-ui/            the widget library: ui (buttons, cards, controls,
                            feedback), panel_fields
```

`gpui-core` owns the generated theme (`theme.rs`, emitted by
`scripts/generate-gpui-theme.py`), the icon set and every value the port has
pinned against the React sources. `gpui-ui` owns presentation only: it depends
on `gpui-core` for tokens and never on the binary.

## Why the feature modules are not crates

`Render` is gpui's trait, so only the crate that defines `DragonFruit` can
implement it — and that impl composes every feature. A feature crate would have
to name `DragonFruit` to take a `WeakEntity<DragonFruit>`, which makes the
dependency cyclic the moment the binary calls it.

The escape hatch is a wrapper entity: move `DragonFruit` into a `gpui-app`
crate and have the binary mount a `Root` that owns it, then features depend on
`gpui-app` for the type and the app depends on the features for the tree
fragments. Nothing in the current tree needs that, and the directory split
gives the same editing isolation for now.

## One widget, one implementation

The React tree re-implements the same card header, section title, toggle and
modal shell across dozens of components; the port must not inherit that.

- **A shared element is defined once, in `gpui-ui`.** If two modules need a card
  header, a labelled number row, a segmented control or a dialog shell, the
  widget lives in `gpui-ui` and the modules call it. Copying the recipe into a
  feature module is the bug, not the shortcut.
- **Feature modules own composition, not chrome.** A feature decides *what* is
  in a panel; `gpui-ui` decides how a panel and its controls look.
- **Values come from `gpui-core`.** Radii are `theme::ROUNDED_*`, cards are
  `panel_layout::CARD_RADIUS`, spacing and colours are the same constants the
  React sources use. A literal that duplicates a token is a defect.
- **A module file that outgrows its area becomes a directory**, with one
  submodule per view plus one module for the chrome they share.

## Incremental builds

`gpui-core`'s viewport and theme are the slowest part of a rebuild. Editing a
panel touches `gpui/src/features/**` and the binary alone; editing a widget
touches `gpui-ui` and the binary. Run `cargo check` from `gpui/`; use
`CARGO_TARGET_DIR=target-check` for builds so a running app's lock on
`target/debug` does not block them.
