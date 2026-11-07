# GPUI port parity

The GPUI frontend (`gpui/`) is a port of the Tauri/React GUI, not a redesign.
This page is the checklist: every component in the React app has exactly one
GPUI counterpart, and "done" means the counterpart exists and its values trace
back to the source.

## Rules

- **Source of truth is the React component and `globals.css`.** Every size,
  colour, radius and state comes from a class string or a token. No invented
  values.
- **Tokens are generated.** `gpui/src/theme.rs` is emitted from the `:root`
  block of `src/app/globals.css` by `scripts/generate-gpui-theme.py`. Do not
  edit it by hand.
- **Icons are generated.** `gpui/assets/icons/` is emitted from the app's own
  `lucide-react` imports by `scripts/generate-gpui-icons.mjs`; the slug is the
  lucide module name, so an icon in the source maps to the same file here.
- **Styling uses `declarative-gpui`'s `ui!` macro** with the source's Tailwind
  classes translated to tokens (`px-2.5` → `px_2p5`, `rounded-[5.5px]` →
  `rounded_5p5`, `text-[11px]` → `text_11`). Theme colours are passed as
  `bg = rgb(theme::SURFACE_0)` so the same token drives both apps.
- **Fonts:** Geist Sans/Mono are converted from the app's `next/font` woff2
  files to TTF and registered at startup; the root sets `font_family("Geist")`
  at 14px / 1.45, matching `body` in `globals.css`.

The app's own atom contract is `docs/dev/ui-atoms.md`: it defines the set, the
size/geometry rules and the tone table (`src/components/atoms/iconTone.ts`).
The GPUI port follows that document rather than re-deriving the atoms.

## No approximations

A value that is not in the source is a bug. Each module carries a
`// PORT DEVIATIONS` block at the top: it lists the source files it was
diffed against and every place the platform cannot match (no backdrop blur,
no CSS transitions, no portals, no CSS transforms), with the native
substitute. A module without that block has not been audited.

## Components

| GPUI | Source | Status |
|---|---|---|
| `theme.rs` | `src/app/globals.css` `:root` | generated |
| `icons.rs` + `assets/icons` | every `lucide-react` import | generated |
| `ui::button` | `atoms/Button.tsx` + `.ui-button*` | ported |
| `ui::icon_button` | `atoms/IconButton.tsx` | ported |
| `ui::card` / `card_header` | `atoms/Card.tsx` | ported |
| `ui::toggle` | `atoms/Toggle.tsx` | ported |
| `ui::menu_item` | `ui/ContextMenu.tsx` | ported |
| `ui::spinner` | `atoms/Spinner.tsx` | ported |
| `ui::modal_shell` | `ui/StructuredDialogModal.tsx` | ported |
| `ui::segmented`, `select`, `input`, `toast`, `tooltip`, `progress`, `setting_row` | the matching atoms/ui files | in progress |
| `app_bar` | `layout/TopBar.tsx` | in progress |
| `rail` | `controls/ToolRail.tsx`, `controls/toolRailEntries.ts` | in progress |
| `panels` | `layout/FloatingPanelStack.tsx` | in progress |
| `overlays::layer_slider` | `controls/LayerSlider.tsx` | in progress |
| `overlays::view_cube` | `SceneCanvas` view cube | in progress |
| `panel_content` | `controls/ModelManagerPanel.tsx`, `controls/AutoSupportPanel.tsx`, `ExportPanelStack.tsx`, `PrintingPanelStack.tsx` | ported |
| `support_panels` | `IslandsPanel`, `IslandListCard`, `AutoSupportSettingsBody`, `HollowingPanel`, `HolePunchPanel` | ported |
| `transform_panel`, `arrange_panels`, `smoothing_panel`, `organic_cut_panel` | the matching Prepare panels | ported |
| `prepare_stack` | `PreparePanelStack.tsx` | ported |
| `plate_widgets` | `PlateNameLabel.tsx`, `PlateSideButtons.tsx` | ported |
| `empty_scene` | `EmptySceneState.tsx`, `ModelStatsCard.tsx` | ported |
| `printing_preview` | `PrintingPreviewPane.tsx` | ported |
| `notifications` | `NotificationStack.tsx`, `SystemNotificationStack.tsx` | ported |
| `auto_support_settings` | `AutoSupportSettingsBody.tsx` | ported |
| `scene_modals` | the scene/dialog modals | in progress |
| `tool_panels` | adapter mounting the Prepare/Support tool panels | in progress |
| `settings modal` | `settings/SettingsModal.tsx` | pending |
| `context menus` | `ui/ContextMenu.tsx` | pending |

## Deliberate gaps

These are web-platform features GPUI does not provide; each has a stated
substitute rather than an accidental omission.

| Web | GPUI substitute |
|---|---|
| `backdrop-filter: blur(24px)` | layered translucent gradients (no blur) |
| CSS transitions | instant state changes (GPUI has no CSS tweening) |
| `box-shadow` from Tailwind | GPUI `box_shadow` where available |
| CSS transforms/skew | none; layout is axis-aligned |
| Portals | absolute children of the root |
| `<input type="file">` | native file dialog (`rfd`) |
| three.js view cube | GPUI-drawn isometric cube with the same click targets |
