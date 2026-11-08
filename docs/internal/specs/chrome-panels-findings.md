# Chrome + panels — audit findings

Area: `gpui/src/chrome/**` (`app_bar.rs`, `rail.rs`, `editor_menu.rs`) and
`gpui/src/panels/**` (`list.rs`, `content.rs`). Specs: `chrome.md`,
`panels.md`, `controls.md`. `chrome/overlays.rs` belongs to `scene-overlays.md`
and `panels/empty_scene.rs` to that spec too, so neither is judged here.

Method: every surface in `chrome.md`/`panels.md` was compared against the port
file by file, with the React source re-read where a value was in doubt
(`TopBar.tsx`, `ViewTypeDropdown.tsx`, `SelectDropdown.tsx`, `ToolRail.tsx`,
`ModelManagerPanel.tsx`, `FloatingPanelStack.tsx`, `globals.css`). Edits were
syntax-checked by `rustfmt --check` (parse only; the tree is not
rustfmt-formatted). No `cargo` was run.

Verdicts: `correct` · `deliberate` (a `PORT DEVIATIONS` note in the file says why)
· `wrong` (fixed) · `absent` (spec surface missing, reported as new work) ·
`dead` (rendered, cannot do anything).

## App bar — `gpui/src/chrome/app_bar.rs`

| # | Surface (spec anchor) | Verdict | Fix / reason |
|---|---|---|---|
| 1 | Bar root height/padding (`chrome.md` gotcha 2) | deliberate | Port is 48px with symmetric `pt/pb 6`; the in-file comment says the port draws no bottom fade, so the controls keep their 36px content box. Source is 52/`3 4 9 4`. |
| 2 | `.ui-topbar::before` + `.ui-topbar-blur` gradients | deliberate | Three-stop CSS ramps split at the middle stop; `backdrop-filter` and `mask-image` are deviations 1-2. Stop alphas verified against `globals.css:170-241` (`f0/e6/c2`, `12/08/0f`). |
| 3 | Onboarding state (`hideWorkflowControls`) | deliberate | Deviation 12: no onboarding in the port. |
| 4 | `topbarActionsDisabled` dimming of both groups | deliberate | Deviation 24 (added): GPUI has no inherited `pointer-events`, so each control disables itself and swallows its press. |
| 5 | Window drag root (`TopBar.tsx:353-402`) | **wrong → fixed** | The source's `interactiveSelector` matches `button` *whether or not it is disabled*, so a press on a control never starts a window drag. The port attached `stop_propagation` only on the enabled branch: the Monitor button (no handler at all), disabled Settings/Overhangs buttons, the disabled view-mode trigger and locked workflow steps all dragged the window. Every control now swallows its press unconditionally (`step_button`, `toolbar_button`, `view_type_dropdown`, monitor button). |
| 6 | Logo/app-menu button | **wrong → fixed** | `rounded-md` is `var(--radius-md)` = **10px** (`globals.css:42`), not the utility default 6px; the port used `rounded_6`. Now `rounded(px(ROUNDED_MD))`. Size 40, 28px icon, `color-mix(accent, transparent 80%)` open tint (`0xec2a7733`) are right. |
| 7 | Left divider | correct | `color-mix(border-subtle, transparent 24%)` = `0x272a33c2`. |
| 8 | Printer badge — label logic | correct | `uses_fleet_label_order` / `fleet_printer_name` / `label_top` / `label_bottom` / `has_fleet_units` match `TopBar.tsx:460-511`. |
| 9 | Printer badge — top label case | **wrong → fixed** | Non-fleet top line carries `uppercase` (`TopBar.tsx:776-782`); the port rendered "Printer" as written. Now uppercased in that branch (fleet branch keeps the profile name, `truncate`). |
| 10 | Printer badge — thumbnail tile under `hideBadges` | **wrong → fixed** | `TopBar.tsx:768` spreads `{ display: 'none' }` on the 32px tile; the port always drew it, so a narrow window showed a tile the source hides. |
| 11 | Printer badge — chevron | **wrong → fixed** | `ml-auto` was missing (`TopBar.tsx:788`), so the chevron did not hold the right edge at `max-w-[300px]`. Colour `color-mix(text-muted, white 8%)` was already right (`mix(TEXT_MUTED, WHITE, 0.92)`). |
| 12 | Printer badge — radii | **wrong → fixed** | Button `rounded-md` 6→10; thumbnail tile `rounded-sm` 4→6 (`globals.css:41`). |
| 13 | Printer badge — `title`/`aria-label` strings | absent | Deviation 10. The interpolated msgids (`topBarMessages.ts`) are not drawn at all. |
| 14 | Printer badge click → fleet menu vs profile flow | correct | `hasTopbarFleetUnits` chooses the quick menu; otherwise `printer_profiles::begin()` + `show_printer_profiles`. |
| 15 | Fleet quick menu — surface/rows/recipes | correct | 280px `menu_surface`, `FLEET UNITS` heading (widget uppercases), offline/active/other recipes all match `TopBar.tsx:864-936` (`82/92`, `70/11`, badge `55/08`). |
| 16 | Fleet quick menu — "Show manager" destination | **wrong → fixed** | `handleOpenPrinterManagerFromQuickMenu` runs `requestOpenProfileSettings('printer')`; the port opened the **app Settings modal** (`show_settings = true`). Now opens the Printer & Material Profiles modal, as the badge's non-fleet path does. |
| 17 | Fleet quick menu — row selection | dead | The row sets `DragonFruit::select_fleet_unit`, which **nothing reads** (grep across `src/`). Unreachable anyway: `features::printer_picker::badge()` returns an empty fleet. New work — see below. |
| 18 | Monitor button — geometry | correct | 40px, `gap 6`, `px 8`, 14px glyph, 11px label. |
| 19 | Monitor button — tone precedence | **wrong → fixed** | `ui-topbar-monitor-paused`/`-active` are applied regardless of `offline` and their animated `color` beats the inline tone (`globals.css:297-305`), so paused wins over offline. The port tested `offline` first. Order is now paused → active → offline → `--text-strong`. |
| 20 | Monitor button — visibility / destination | dead | `app.rs` passes `monitor: Default::default()` (never visible) and `show_monitor` is read by nothing. Cross-area: needs a print-monitor view plus reachability/paused/active producers. |
| 21 | Workflow steps — geometry, recipes, lock, shadows | correct | 32px, `flex-1 min-w 90 max-w 190`, `rounded 5.5`, `px/py 6`, gap 6; active fill `mix(accent, surface-0, .16)` (16% accent), active border `mix(accent, white, .92)`, printing-locked `mix(surface-2, black, .88)` / `mix(border-subtle, black, .70)`, inactive `0x1a1b21f5` (surface-1 at 96%) all check out against `chrome.md` gotcha 8. |
| 22 | Workflow steps — label colour | correct | The source's conditional is dead (both branches `--text-strong`); the port renders one colour. |
| 23 | Workflow steps — `title` hint ladder | absent | `chrome.md` "Tooltip text" (four-priority hint). Deviation 10 says a tooltip needs a wrapper; `ui::mouse_tooltip` follows the pointer and needs none, so this is a closable gap, not a platform limit (note amended in the file). |
| 24 | Workflow steps — `hidden`/`hideBadges` measurement | deliberate | Deviation 14: per-frame `perButton` from `window_width`, 130px steady state, no hysteresis. |
| 25 | Step icons `strokeWidth={2.5}` | deliberate | Deviation 19 (generated SVGs bake 2). |
| 26 | View-mode trigger — geometry | correct | `!h-9`, `!pl-2 !pr-7` / `!h-9 !w-9 !p-0`, 13px/`line-height 1.3` from unlayered `.ui-input`, 18px leading well, `right-1.5` chevron, centred camera in icon-only mode. |
| 27 | View-mode menu — surface/rows/icons | correct | `!w-64`, 4px radius, accent 82% border, `mix(surface-0, black, .94)` fill, two-layer shadow, `text-[0.95em]` labels, selected `mix(accent, surface-0, .14)`, icon colour accent/text-strong/text-muted (`SelectDropdown.tsx:462-494`). |
| 28 | View-mode menu — backdrop blur | deliberate | Deviation 23 (added): `blur(6px) saturate(112%)` has no counterpart. |
| 29 | View-mode — `rotate-180` chevron, icon-only leading icon | deliberate | Deviations 4 and 20. |
| 30 | View-mode menu flip/clamp | deliberate | Deviation 9 (menus open downwards; no measure pass). |
| 31 | Overhangs toggle | **wrong → fixed (disabled state)** | Same `.ui-button:disabled` bug as Settings: the source swaps to `color-mix(surface-1, black 8%)` / `color-mix(border-subtle, black 10%)` / `color-mix(text-muted, surface-2 18%)` with `opacity: 1` (`controls.md` "Cursor + disabled"); the port faded to `opacity .55`. |
| 32 | Overhangs toggle — effect | dead | `overhang_view` is stored and mirrored into `AppBarState`; nothing consumes it, so the scene never changes. Needs the viewport/shader pass. |
| 33 | Settings button | correct | Opens `features::settings::settings_modal`; 36×34 box, `RADIUS_SM`. |
| 34 | Caption buttons | correct | 32×32, `radius-sm`, dark tint triples (`55%` border, `86%`/`88%` surface, 16%/18% text), 16px glyphs (`TopBar.tsx:1087-1148`), live during a slice (outside the dim wrapper). Light-theme tints are deviation 13. |
| 35 | App menu — rows/order/groups/disabled | correct | All seven ids in `appMenuEntries` order with `startsGroup` on save/import-mesh/close; `new-scene` disabled without models; rows with no handler render disabled, exactly the source's `!onX` checks. |
| 36 | App menu — placement/heading/width | correct | 192px at `(4, 49)` — the logo's `rect.bottom + 6`. |
| 37 | App menu — Save/Open/Import scene | absent | Disabled because `page.tsx` handlers have no port counterpart (scene file I/O). |
| 38 | App menu — dismissal | deliberate | Deviation 11: `app.rs` clears the flags on outside press/Escape. |
| 39 | Re-slice warning dialog | deliberate | Deviation 22 (no profile switching). |
| 40 | Theme bootstrap / profile hydration | deliberate | Deviation 13 (dark-only). |

## Tool rail — `gpui/src/chrome/rail.rs`

| # | Surface | Verdict | Fix / reason |
|---|---|---|---|
| 41 | Rail container, both layouts | correct | Column `left 6 / top 52 / bottom 0 / w 54`, `gap 6`, `py 6`; bar `top 56`, centred row, `px 6`. `overflow-*` omitted and documented (tooltips are in-tree). |
| 42 | Tile recipes (`railEntryStyles`) | correct | lit `mix(hue, surface-0, .22)` / `mix(hue, white, .86)` / inset `0x42`, off `mix(hue, surface-1, .10)` / `mix(hue, border-subtle, .12)`; `invert_lit` flips the recipe only. |
| 43 | Disabled tile | correct | `SURFACE_1`/`BORDER_SUBTLE`/`TEXT_MUTED` at `opacity .45` (gotcha 10). |
| 44 | Hover `brightness(1.1)` | deliberate | Documented substitution (colours brightened). |
| 45 | Fold animation / transitions | deliberate | Documented as absent. |
| 46 | Tooltip | correct | 140px, 8px gap, right/below by layout, `rgba(24,24,24,.98)`, accent border, 4px radius, both shadows (`0x70`, `0x47`). No flip/clamp and hover-only are documented. |
| 47 | Layout context menu | correct | `Tool layout`, 192px, checked Vertical/Horizontal, right mouse-down. Backdrop-based dismissal is documented. |
| 48 | Prepare entries (8 + Models) | correct | Order, labels, hints, icons, group separations match `toolRailEntries.ts:26-100`; `onHover` omitted (dead in the source per gotcha 15). |
| 49 | Support entries (3 + Hollow + Interior + View Mode) | correct | Order/separations/`active` rules match `toolRailEntries.ts:145-231`; `viewMode` shows the current mode's icon. |
| 50 | Models tile → Models panel | **wrong → fixed** | The tile only collapsed the card, leaving a 48px header (documented deviation) — the source hides the whole card (`display: none`, `ModelManagerPanel.tsx:376-378`). `models_shown()` now lets `panel_stack` drop the card from the dock, so nothing is left behind and the stored placement/width survive. |
| 51 | Call-site gates (models > 0, prepare/support) | **wrong doc → fixed** | The module doc claimed `main.rs` renders the rail unconditionally; `app.rs:1898` applies the source's exact gate. Doc corrected (the `main.rs`→`app.rs` paint-order note too). |
| 52 | Rail layout default / persistence | deliberate | Kept `Vertical` and unpersisted, now with an accurate note; the insets that depend on it follow `layout_is_column()` (see #62). The Settings "Toolbar position" control writes `gpui_store.general.tool_layout`, which nothing reads back — see new work. |
| 53 | Interior View availability gate | absent | `hasCavityGeometry` has no port field: the tile is always enabled with the enabled hint (documented). The disabled recipe is ported and unreachable. |
| 54 | Interior View / View Mode toggles | dead | Both only flip rail-local `interior_view` / `view_lines`; no consumer (scene, hollowing, support drawing). Needs renderer/viewport support. |
| 55 | Prepare tool memory across modes | deliberate | Documented: `active_tool` is Prepare-only; the source's shared `transformMode` and the gizmo check are cross-file. |

## Editor context menu — `gpui/src/chrome/editor_menu.rs`

| # | Surface | Verdict | Fix / reason |
|---|---|---|---|
| 56 | Rows, labels, icons, groups | correct | Matches `MENU_ITEMS` (`editor_menu.rs` unit test pins the list, including the three flyouts). |
| 57 | Heading (`Editor` / `Supports`) | absent | Omitted with a comment: the title needs the mode, and `editor_menu` is built from `(position, entity)`. Closable by passing the mode from `app.rs`. |
| 58 | Flyout side / clamping / dismissal | deliberate | Documented (`FlyoutSide::Right`, `snap_to_window_with_margin(8)`, parent-owned dismissal). |
| 59 | Disabled rows | deliberate | Documented: the source derives `disabledActions` from scene state the builder cannot read. |
| 60 | Row actions | dead | Only `delete` acts. Cut/copy/paste, repair/remesh/shrink-wrap, support split/merge/scan and geometry designation close the menu and do nothing (each arm documented). Needs the scene graph / repair pipeline. |
| 61 | Organic-cut items (`Add waypoint here`, `Delete waypoint`) | absent | Caller-added in `page.tsx`; the organic-cut tool is not ported. |

## Panel stack — `gpui/src/panels/list.rs`

| # | Surface | Verdict | Fix / reason |
|---|---|---|---|
| 62 | `leftInsetPx` / `rightInsetPx` / `railIsColumn` | **wrong → fixed** | `page.tsx:10718` passes 66 **only while the rail is a column**; the port hardcoded `rail_is_column: true` and a 66px left inset, so switching the rail to the bar left a 66px gap and kept the Models card drag-locked. `panel_stack` now reads `rail::layout_is_column()`. |
| 63 | Dock geometry, margins, gaps, resize strip | correct | `PANEL_MARGIN`/`PANEL_GAP` 6, `padding 6`, 6px `cursor-col-resize` strip, left/right alignment, overflow per drag. |
| 64 | Dock resize semantics | correct | Writes the panels' widths (`width - 12`, never below base) and lets the dock follow; clamp 200-520. |
| 65 | Panel drag / drop / float clamp / right-gesture threshold | correct | 4px threshold, `LOCKED_PANELS` via `is_locked`, drop index by midpoint, snap on the panel's overlap, 6px right-button threshold, `clampPosition` to 6. |
| 66 | Drop preview | correct | `mix(accent, white, .88)` 2px dashed, `0x1f` fill, 4px `0x2e` ring, 14px radius. |
| 67 | `Window` context menu | correct | `Reset Window` / `Reset Layout`, heading `Window`, 192px. Measure-free clamping documented. |
| 68 | Collapse toggle | correct | 30px `.ui-button` tile, 12px chevron, accent when expanded / muted when collapsed. |
| 69 | Collapsibility per panel | **wrong → fixed** | `collapsible={modelsPanelCollapsible}` is `toolLayout === 'horizontal'` (`page.tsx:10731`), and `expanded` is a *fallback*: the column's Models card has **no** chevron. The port always drew one. The leading toggle is now omitted (and the card pinned expanded) for the Models panel in the column layout. |
| 70 | Non-collapsible header `py-4` | absent | `ui::card_header_with` fixes the header padding at 10px, so the source's `py-4` branch is unreachable from this file. Reported for `gpui-ui`. |
| 71 | Hidden panels (`display: none` semantics) | deliberate | Documented: a panel not passed has no placement in the frame; its stored placement survives. The Models card now uses exactly this path for the rail toggle (#50). |
| 72 | Persistence / no localStorage | deliberate | Documented (`floating-panel-layout.json`, no v5 migration, no debug-snapshot event). |
| 73 | Auto-compact / z-order / pointer-events model | deliberate | Documented in the header and implemented in `gpui-core/panel_layout.rs`. |

## Models panel body — `gpui/src/panels/content.rs`

| # | Surface | Verdict | Fix / reason |
|---|---|---|---|
| 74 | Header plus button | correct | `IconButton sm` (32 tile, 14px `plus`, `!p-0`), tooltip "Add models to the plate", opens the native picker (`open_stl`). No count in the header (gotcha 33). |
| 75 | Empty state | correct | "No models loaded", `text-xs`, centred, `py-2`, italic, muted — unreachable in the source app as well (gotcha 2). |
| 76 | Model row geometry/selection | correct | `px-2 py-1 rounded border gap-2`, selected `mix(accent, surface-1, .08)` fill and `mix(accent, border-subtle, .60)` border. |
| 77 | Row name tooltip | correct | Full-name tooltip on a truncated, 14px medium `--text-strong` label. |
| 78 | Row delete | **dead → fixed** | Trash ran nothing. Now runs `DragonFruit::delete_model` — the same path as the `Delete` key and the canvas menu — and its press no longer selects the row (`ghost_icon_action` stops propagation, like the source's `onDelete` branch). |
| 79 | Row info button | dead | No details dialog (`ModelSupportsModal`, other area). Tooltip shows the polygon clause only (`MeshData` has no size). |
| 80 | Row eye button | dead | No per-model visibility; the label is always "Hide". |
| 81 | Rename / context menu / groups / plates / bodies / multi-select | absent | Single-mesh model: `body_row`, `folder_row`, `models_context_menu` are ported but `#[allow(dead_code)]` and unreachable. |
| 82 | Card max height + list scroll | absent | Gotcha 30 and `overflow-y-auto` need a fixed card height; with one row the cap is unreachable, and a scroll container without a height collapses (documented deviation 14). |
| 83 | Card radius | correct | `ui::card()` paints 14px, which **is** `rounded-lg` here (`--radius-lg: 14px` at `globals.css:43`). `controls.md` gotcha 3 ("8px") contradicts `panels.md`; the CSS resolves it in `panels.md`'s favour. |

## Controls surfaces these files consume

| # | Surface | Verdict | Fix / reason |
|---|---|---|---|
| 84 | `.ui-button` + disabled recipe (`controls.md` "Cursor + disabled") | **wrong → fixed** | Settings/Overhangs disabled state used `opacity .55`. Replaced with the global `button:disabled` recipe (surface/border/text mixes, `opacity: 1`, `cursor: not-allowed`) in `toolbar_button`. The view-mode trigger keeps its own `disabled:opacity-55`, which `SelectDropdown.tsx` really does set. |
| 85 | `ContextMenu` surface/rows | correct (usage) | Widget-owned (`gpui-ui`); the bar and panel menus pass width, heading and `startsGroup` as the source does. |
| 86 | `Card`/`CardHeader` | deliberate | `ui::card` + `card_header_with`; the only gap is the `py-4` branch (#70). |
| 87 | `SelectDropdown` widget itself | deliberate | Deviation 17: `ui::select_dropdown` lacks `selectClassName`/`menuClassName`/`chevronClassName`/`hideChevron`/`selectedDisplay`, so the bar keeps its own trigger. The menu's own geometry is reproduced in-file (#27). |
| 88 | `Tooltip` widget (cursor / trigger anchored) | deliberate | Rail uses fixed 140px trigger-anchored geometry (#46); no viewport clamp or flip (`chrome.md` "Tooltip" last paragraph). |
| 89 | `floatingLayoutPreferences` keys/events | deliberate | Documented in `list.rs`: file-backed layout at `floating-panel-layout.json`, no `TOOL_LAYOUT_EVENT`, no debug-snapshot event. |
| 90 | `useFloatingPanelDockResize` / `useFloatingPanelCollapse` hooks | correct | The dock resize is `layout.dock_resize` (#64); collapse is the `state.collapsed` map with `request_panel_expand` before expanding (`collapse_toggle`, #68). |
| 91 | `StructuredDialogModal` (re-slice warning, plate delete) | absent | Neither dialog has a trigger in this area: no profile switching (#39) and no build plates (#81). |
| 92 | `ModelsPanel` wrapper / Prepare + Support stacks | deliberate | The port mounts one `models` panel for both modes, documented in `list.rs` ("Panel ids"); the surrounding tool panels are other areas. |
| 93 | `ExportPanelStack` / `PrintingPanelStack` composition | n/a — other area | The port mounts a single `export-main` / `printing` panel from `app.rs`; the plate-scope derivation belongs to those specs. |
| 94 | TopBar event buses (`OPEN_SETTINGS_MODAL_EVENT`, `OPEN_PROFILE_SETTINGS_MODAL_EVENT`) | deliberate | GPUI has no DOM event bus; the bar writes `show_settings` / `show_printer_profiles` on the entity directly, and `settings_tab` is the parent's. |
| 95 | Hidden file inputs (accept lists) | deliberate | Native dialogs instead (`open_stl`: STL + VOXL filters), since `.obj`/`.3mf`/`.zip` have no loader in this build. |
| 96 | `.ui-topbar-mode-idle` / `-active` | n/a | Referenced by no component in either tree (`chrome.md` open question 7). |

## Changes made (file → cited authority)

- `chrome/app_bar.rs`
  - every control swallows its press: `TopBar.tsx:379-394` `interactiveSelector`
    contains `button` unconditionally (#5).
  - logo/monitor/printer `rounded-md` → 10px, thumbnail tile `rounded-sm` → 6px:
    `globals.css:41-43` (#6, #12).
  - printer top label `uppercase` off-fleet, `ml-auto` chevron, `hideBadges`
    drops the thumbnail tile: `TopBar.tsx:768-790` (#9-#11).
  - "Show manager" → printer profiles, not Settings:
    `handleOpenPrinterManagerFromQuickMenu` (#16).
  - monitor tone order: `globals.css:297-305` (#19).
  - `.ui-button:disabled` recipe: `globals.css` "Cursor + disabled" (#31, #84).
  - deviations 10, 23, 24 amended/added so the remaining gaps are stated
    accurately.
- `chrome/rail.rs`
  - `models_shown()` / `layout_is_column()` exposed; the Models toggle now hides
    the card through the panel stack instead of collapsing it
    (`ModelManagerPanel.tsx:376-378`, `page.tsx:8595`) (#50).
  - stale `main.rs` call-site and paint-order notes corrected (#51); layout
    deviation note rewritten (#52, #62).
- `panels/list.rs`
  - insets and `rail_is_column` follow the rail layout (`page.tsx:10718`,
    `FloatingPanelStack.tsx` `LOCKED_PANELS`) (#62).
  - the Models card is dropped from the dock while the rail hides it (#50, #71).
  - collapse chevron only when `collapsible` (`page.tsx:10731`) and the card
    pinned expanded otherwise (#69); deviation bullets updated (#69, #70, #72).
- `panels/content.rs`
  - model-row delete wired to `delete_model`; `ghost_icon` split into inert and
    acting forms so the press still stops at the button
    (`ModelManagerPanel.tsx:594-821`) (#78); deviations 10, 14, 15 updated.

## New work (not attempted: needs other files/areas)

1. **Shader switching.** `view_type` (and the Overhangs flag) never reach the
   viewport, so the view-mode dropdown and the Overhangs toggle are `dead`.
   Needs the mesh shader pass in the renderer (`app.rs` + backend).
2. **Printer fleet.** `printer_picker::badge()` returns no fleet/thumbnail, so
   the quick menu is unreachable and `select_fleet_unit` has no consumer. Needs
   network-device selection in the printer store plus a reachability producer.
3. **Print monitor.** `MonitorButton::visible` is always false and
   `show_monitor` is unread. Needs the monitor view (printing area) and the
   paused/active/offline producers.
4. **Bar tooltips.** The four workflow-step hints, the monitor's
   `Open printer monitor` / `Selected printer is offline`, and the printer
   badge's title/aria are absent (#13, #23). `ui::mouse_tooltip` makes this
   closable without changing the flex geometry.
5. **Scene file I/O.** `Save scene`, `Save scene as…`, `Open scene…`,
   `Import scene…` render disabled because nothing handles them (#37).
6. **Editor menu actions.** Cut/copy/paste, repair/remesh/shrink-wrap,
   support split/merge/scan, geometry designation (#60), and the organic-cut
   waypoint items (#61). The menu heading needs the mode passed in (#57).
7. **Interior view / support view mode.** Both rail toggles only change their own
   tile (#54); `interiorViewAvailable` needs a cavity flag (#53).
8. **Tool layout wiring.** `gpui_store.general.tool_layout` (e.g. "Toolbar
   position" in `features/settings/general.rs:143-171`) is written but never read
   by the rail, and neither the rail layout nor `models_visible` is persisted
   (`app-tool-layout` / `app-models-panel-visible` in the source). Needs a
   settings snapshot at rail build time (and a decision on the default, which the
   source sets to `horizontal`; flipping it will move the `automation.rs` rail
   rects).
9. **Models panel depth.** Rename, group/plate structure, body drawer, panel
   context menu and multi-select (#81) need a real model list in the scene,
   `splitBodies`, plates and a selection model with `single|toggle|add` + range.
   The card's viewport-derived `maxHeight` and the list's scroll come with it
   (#82).
10. **Non-collapsible card header padding.** `ui::card_header_with` needs a
    `py-4` variant for #70 (`gpui-ui/src/widgets/card.rs`).
11. **`SharedPanelStack` (Ctrl+Shift+X debug overlay)** — no counterpart at all
    (`panels.md`); id `transform-debug-overlay`, 420px override already reserved
    in `panel_layout.rs`.
12. **Analysis panel stack** — the six analysis cards and their order have no
    Rust module (`panels.md`), and `PanelKind` has no analysis entries.
