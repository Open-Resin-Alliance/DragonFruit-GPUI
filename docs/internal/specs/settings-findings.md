# Settings — audit findings

Area: `gpui/src/features/settings/**` against `settings-a.md` … `settings-d.md`.
Every verdict below is per surface; "fix" names the spec line and the React file the
value came from. Nothing in `app.rs`, the ledger or another area's files was touched.

## Cross-cutting

| Surface | Verdict | Note / fix |
|---|---|---|
| Radii across every tab | `wrong` → fixed | The port had documented "Tailwind v4 radii" (4/6/8) in `mod.rs`. The app overrides the Tailwind variables in `globals.css:41-43` (`--radius-sm: 6px`, `-md: 10px`, `-lg: 14px`), and `gpui-core/theme.rs:49-52` already carried the generated values. Every `rounded-md` drawn at 6px is now `ROUNDED_MD` (10) and every `rounded-lg` drawn at 8px is `ROUNDED_LG` (14): 30 call sites over `about`, `autosave`, `backups`, `experiments`, `hotkeys`, `logging`, `mesh`, `multi_plate`, `plugins`, `slicing`, `spacemouse`, `ui_theme`, `updates`, `mod`, `workspace`. `mod.rs`'s module doc was corrected. |
| `color-mix` direction | `wrong` → fixed | `mix(a,b,w)` is `a*w + b*(1-w)`; CSS `color-mix(in srgb, A, B N%)` is `A*(1-N) + B*N`. Several sites passed `N` where `1-N` was needed. Fixed: segmented active label (`widgets.rs`), checkbox checked fill, message-strip borders, Apply button fill (`SettingsModal.tsx:1804`), the Up-to-date chip (`UpdateCheckerSection.tsx:132-133`), the release-channel cards + amber callout (`UpdatesSettingsTab.tsx`), the plugin allowance warning box (`PluginsSettingsTab.tsx`), and five logging mixes (level rows, filter trigger, filter menu row, info callout). |
| i18n | `wrong` (not fixed) | The port has `gpui-locale` (catalogs, `t!`, `UnitFormat`) and the settings msgids exist in `catalog_data.rs`, but **no** settings string goes through it: every label/section/description is a raw literal. A module-wide translation sweep is new work (and the source itself leaves several section headings untranslated — `settings-a.md` decision 14). |
| Language switcher | `dead` (not fixed) | `general.rs` writes `draft.language` → `gpui_store`, and `gpui_locale::set_locale` is **never called anywhere in `gpui/src`** (grep: only `fields.rs` reads `current_catalog`). Choosing a language changes nothing on screen. Fix needs the i18n sweep above plus a `set_locale(…)` call on hydrate/apply, i.e. a whole-module change, not a value fix. |
| Floating-layout persistence flag | `wrong` (partly fixed) | `general.rs` only wrote the store field; `PanelLayout::set_persistence_enabled` (the flag that actually gates `floating-panel-layout.json`) had no caller. The toggle now calls it. **Cross-file blocker:** the startup sync (`layout.set_persistence_enabled(settings.general.floating_layout_persistence)` right after `hydrate`) belongs in `app.rs:406` — a saved `OFF` is ignored until the user toggles it once in that session. |
| "Reset saved window layout" | `dead` → fixed | The button set the persistence flag back to `true` and never cleared a layout, while its own copy promises to forget stored positions. Now calls `this.layout.reset()` (forget placements + delete the saved document = `clearSavedFloatingLayout()`). |
| Rail description tooltip | `deliberate` | `SettingsSidebarTab`'s `TabDescription` only offers a tooltip when the text is clipped, measured through a `Range` + `fonts.ready` (`SettingsModal.tsx:272-343`). GPUI runs no measure pass here; the line truncates. Recorded in the `mod.rs` deviation list. |
| Second scroll position | `correct` | About/Hotkeys/Updates are the internal-scroll tabs, matching `usesInternalTabScrollLayout`. |

## settings-a — General, Camera, Workspace, Multi-Plate

| Surface | Verdict | Note / fix |
|---|---|---|
| General › Language | `correct` (behaviour) | Row, description, `w-36` select with leading glyph. Selector itself is dead — see cross-cutting. |
| General › UI Scale | `deliberate` | Presets, `Custom` raw string, `isCustomScale` derivation, 25–400 clamp, `mt-2.5` custom row all match. Immediate-save → draft is the module-level "store owns persistence" deviation. Percent labels are hard-coded English (`fmt.quantity` in the source) — part of the i18n gap. |
| General › Toolbar | `correct` | Top/Left, icon swap, draft write. |
| General › Floating Windows | `wrong`/`dead` → fixed | Both rows above. |
| General › Support Tooltips | `correct` | OFF/ON, description with the nested quotes, draft write (source writes immediately — covered by the module deviation). |
| General › Import Defaults | `correct` | Line force-enables roots and hides the roots row; wall only for `solid`; defaults from the store; row spacing `mt-3`/`mt-2`. |
| `ScrollableNumberField` | `wrong` → fixed | Steppers never showed the source's disabled state at `<= min` / `>= max`. Now disabled (dimmed, no handler, clamped). |
| `NumberInput` / `Select` / `SettingRow` | `correct` | 12px stepper tiles, `right-0.5`, 10px icons, `hover:bg-white/10`, per-keystroke commit; setting-row padding/gaps. |
| Camera › scope + global branch | `correct` | Ortho/Perspective, FOV slider only under global+perspective, 15–120. |
| Camera › FOV read-out | `wrong` → fixed | `{perspectiveFov}°` is the stored value; the port re-rounded it to an integer. |
| Camera › workspace branch | `deliberate` | Four workspaces (no `printing`), `fullWidth` picker, per-workspace label/hint. `workspace_tab` is draft state rather than the source's transient local state — commented in place. |
| Camera › trackpad acceleration | `wrong` → fixed | Two defects: (1) `fmt.quantity(v, 2, 'times')` renders `2.00x` (msgid `{value}x`, `unitFormatting.ts:43`) but the port printed `2.00×`; now via `UnitFormat`/`Unit::Times`. (2) The three sliders were drawn as bordered cards; the source's `space-y-0.5` groups sit inside the already-bordered panel (`CameraSettingsTab.tsx:352-398`). Added `slider_group` (borderless) and used it. Row B + panel were also flush against each other; they now share the `space-y-1.5` stack. |
| Camera › Higher Contrast Model Edges | `correct` (absent by flag) | `MODEL_EDGE_OVERLAY_ENABLED = false`; nothing renders, the field is kept. |
| Workspace › 3D View | `correct` except one radius | Build-volume summary card was `rounded-md` drawn at 6px → fixed to 10. Printer-managed branch absent (no profile store) — recorded deviation. Number clamps (`max(10/320/200, round)`) match. SAT rows nest three deep. Grid `mt-2`. |
| Multi-Plate | `wrong` → fixed | Card is `rounded-lg` (14px), drawn at 8px. ON-before-OFF order, reversed index, `fixedPlateColumns` clamp, `w-[120px]` field all correct. Immediate-write vs draft is the module deviation. |

## settings-b — Mesh, Slicing, 3D Mouse, UI & Theme

| Surface | Verdict | Note / fix |
|---|---|---|
| Mesh › Shader & Preview | `wrong` → fixed | Selector cards are `rounded-md border p-2` with a `w-full !h-8` select; the port used the `px-2.5 py-1.5` card and pinned the selects to `w-36`. Added `selector_card` (p-2) and full-width selects. Preview tile + colour card radii → 10. |
| Mesh › colour hex box | `dead` → fixed | The `activeHexColor` box was a static label. It is now the source's `<Input>`: editable, placeholder `#a3a3a3`, writes `meshColor` at index 0 and `heatmapColors[index-1]` otherwise on every keystroke. |
| Mesh › Rendering Options | `deliberate` (display-only) | Roughness/Lightness/Contrast/AO/X-Ray/Min+Max Overhang present with the source's ranges and `fixed(2)` read-outs; no drag handler (module deviation). **`absent`: Matcap Style select and the `Vertex Colors` row** — `hasRenderingOptions` includes `matcap`/`flat_unlit` at the source; both cards are missing here, so a stored `matcap`/`flat_unlit` shows no options at all. |
| Mesh › Selection & Hover | `correct` + 2 deviations | Cards, badge state machine, hover/click wiring, tint read-outs match. Preview card realigned to `rounded-lg p-2` (it was the `px-2.5 py-1.5` card). Tint sliders are now borderless groups. Reset-to-selected-profile is impossible without per-preset colour tables → commented in place (`deliberate`). |
| Slicing | `wrong` → fixed | Cleanup tiles are `rounded-md` (drawn at 6px). Engine `-`, thumbnail rows, PNG three-way mapping all correct. Cleanup buttons inert and no confirm/result — module deviation. |
| 3D Mouse | `wrong` → fixed | Devices/pivot cards and the trademark strip are `rounded-md` (drawn at 6px). Motion Tuning sliders were bordered cards; they are the source's `space-y-0.5` groups. Axis tile radii (`rounded` = 4px) were already right. `navlibActive`/`candidatePads` hard-wired to the source's empty state — recorded deviation. |
| UI & Theme › Theme section | `wrong` → fixed | Preset/scheme cards are `rounded-lg` (14px, drawn at 8px); the preset select fills its card (`!h-8 text-xs`), not a 144px control; the subtitle now bolds `Apply`; `upper_label` now upper-cases (the source's `uppercase` class — GPUI has none). Reset target deviation documented. `+ New Theme`, `Save/Rename/Export/Delete` remain **absent** (custom profiles are unbuilt). |
| UI & Theme › sub-tabs | `wrong` → fixed | Pills are `rounded-md` (drawn at 6px); active colour/hover already matched. |
| UI & Theme › colour rows | `wrong` → fixed | Row is `rounded-md` (drawn at 6px). Description-under-row instead of a `title` tooltip documented. Cut panel writes draft state where the source writes `organic-cut-colors` immediately — see "new work". |

## settings-c — Hotkeys, Plugins, Experiments, Scene Autosave

| Surface | Verdict | Note / fix |
|---|---|---|
| Hotkeys › card grid / rows | `wrong` → fixed | Section card `rounded-lg` (14), rows + binding button + section reset `rounded-md` (10) — all drawn one step small. Pairing, category sub-labels, `PINNED_SLOT_LABELS`, the `Delete` secondary keycap, token colours and recording row style match. |
| Hotkeys › recording overlay | `wrong` → fixed (partly) | The panel tells the user "or click outside to cancel" and nothing cancelled. The backdrop now cancels on mousedown and the panel stops propagation. Still not ported: key capture (needs the hotkey store's global dispatch) and Escape-to-cancel (Escape reaches the modal's own handler and closes Settings) — both now stated in the `mod.rs` deviation list. |
| Hotkeys › empty-section filter | `absent` | The source drops a section with no entries; the port renders every card unconditionally. Unreachable with the static default table, but the filter is missing. |
| Plugins › card | `good` except two | Removed an invented `No plugins installed in this profile.` line — the source renders an empty list as *nothing* (`settings-c.md` gotcha 27). Allowance warning box radius + mixes fixed. Install/import buttons inert, no list rows, no dialogs — module deviation (no host bridge, no profile store). |
| Experiments › body | `wrong` → fixed | Intro card, rows and restart notice were `rounded-md` at 6px. Toggle writes the draft map correctly. |
| Experiments › disclaimer gate | `wrong` → fixed | `Take me back!` only hid the gate and left the user on the Experiments tab; the source calls `onExit()` — the previous tab (General when that was Experiments). Added `SettingsState::prev_tab` (recorded by the rail's click; view-only, never persisted) and a shared `leave_disclaimer`, also wired to the backdrop mousedown with a stop-propagating panel. `I understand` remains the only acknowledgement. |
| Experiments › persistence shape | `wrong` (not fixed) | `write_store` saves plain booleans where the source stores `true` / `'off-when-default-on'`, so a future default promotion would not reach users who opted out. Needs a three-state field in `gpui-store` — outside my files. |
| Scene Autosave | `wrong` → fixed | Both cards were `rounded-md` at 6px. The four rows, clamping (`15–900 s`, `1–60 min`, `cap >= max(debounce, cooldown)`), disabled-while-off state and the bridge-absent "desktop build" status match. Message strip + manifest/paths resolution remain unported (no bridge) — reported. |

## settings-d — shell, Backups, UVTools, Logging, Updates, About

| Surface | Verdict | Note / fix |
|---|---|---|
| Shell › header/footer/sidebar | `wrong` → fixed (2 values) | Header icon tile is `rounded-lg` (14px, drawn at 8px). Apply's fill is `color-mix(accent, surface-0 16%)` = **84 % accent** (`SettingsModal.tsx:1804`); the port drew it at 16 %, i.e. a dark button. Everything else (72rem, 288px rail, 10px rail padding, rail gradient, entry `p-4`, footer mix) matches. |
| Shell › draft/Apply/Cancel | `deliberate` | Store-backed draft, both buttons close — documented in the module note. The source's Apply-vs-Cancel split is the one open decision of the whole port. |
| Shell › tab metadata | `correct` | 17 entries, 9/8 split, labels, descriptions, icon slugs, primary/secondary tone. |
| Backups | `wrong` → fixed (radii) | Both cards were `rounded-md` at 6px. No history rows (no bridge) → both confirmations unreachable, snapshot-content modal **absent**; module note already records it. Interpolated id where the source formats a locale timestamp — recorded deviation. |
| UVTools | `correct` | Enabled/path branches, quotes in the description copy, discovery states' fills (`0.12`/`0.10` shares) all match; discovery is inert and the 5s timers are absent — recorded deviation. |
| Logging | `wrong` → fixed | Path box, level rows, filter menu and info callout are `rounded-lg` (drawn at 8px); the five toolbar chips are `rounded-md` (drawn at 6px). Five mixes were inverted (level active border/fill, filter trigger fill, filter menu active row, callout border) — the active level row was 90 % colour instead of 10 %. The path cell showed an invented `<only available in desktop build>` string; the source's own `logPath === null` state is a spinner + `Resolving path…`, which is what renders now. Viewer remains the source's "desktop build" line — recorded deviation. |
| Updates | `wrong` → fixed | Card is `rounded-lg` (14px, drawn at 8px); the Up-to-date chip, the active channel card and the amber callout all had inverted mixes (the chip was 82 % surface-1 instead of 18 % success; the active card 85 % surface-1 instead of 15 % accent). The release-channel gear is now hidden on Linux (`isLinux` prop). Only the up-to-date body is ported — the other eight `UpdateCheckerSection` states are `absent`. |
| About | `wrong` → fixed (radii) | Team card, person cards and the sponsor viewport are `rounded-md` (10), the sponsor card `rounded-lg` (14). Sorting/grouping, tone mixes, hero gradient, ORA footer mixes and the version/build pills match. Clipboard, URL openers, avatars, marquee and tooltips are recorded deviations. |

## New work (`absent` — not attempted)

1. **i18n sweep for the whole settings module**, incl. `set_locale` on hydrate/apply so the language switcher stops being dead.
2. **Mesh: Matcap Style card + Vertex Colors row** (`MeshSettingsTab.tsx:327-480`), and `hasRenderingOptions` extended to `matcap`/`flat_unlit`.
3. **UI & Theme custom profiles**: `+ New Theme` footer action, Save/Rename/Export/Delete + their four dialogs, import/export, per-preset colour tables (which would also fix Mesh/Theme "Reset"), `isThemeResetDirty`/`isCustomThemeDirty`.
4. **Mesh 3D preview + real colour picker** (needs the wgpu preview pass and a picker control).
5. **Hotkey key capture + dispatch suspension**; empty-section filter.
6. **Plugins**: profile store / installed rows / three-tier grouping, GitHub manifest install + liability/preview dialogs, sideload list + passphrase + removal, details + reload-required countdown, dev studio link.
7. **Experiments**: `off-when-default-on` persistence (needs `gpui-store`), the modal's "Apply Experiments" reload prompt.
8. **Backups**: `local_backup_*` wiring, history rows, snapshot-content modal, message strip.
9. **Logging**: real log path, 2 s poll + parsers, noise filter, per-line rendering, level persistence to the backend.
10. **Updates**: the eight remaining `UpdateCheckerSection` states, `check_updates`/`perform_update`, relaunch, dev allow-same-version shortcut.
11. **Slicing**: engine-version invoke, temp-file cleanup bridge + confirm/result surface.
12. **UVTools**: `discover_uvtools_path` + the 5 s found/not-found timers.
13. **3D Mouse**: a native 3D-mouse backend (device list, `blockedDeviceIds`, navlib-active gating).
14. **Cut tool colours**: they bypass the settings draft in the source (own storage key + event); the port keeps them in draft state that nothing persists.
15. **`ScrollableNumberField` wheel stepping / Arrow keys** and range-drag for every slider (module deviation today).

## Blockers outside my files

- `app.rs` never pushes `general.floating_layout_persistence` into `PanelLayout` at startup (see cross-cutting).
- `gpui-store` needs the three-state experiment value and (for §3) custom theme profiles; `gpui-store` has no cut-colour group.
- `gpui-ui`'s `dialog_modal_full`/`modal_backdrop` has no backdrop-dismiss handler, so no settings dialog closes on a backdrop click (the experiments/hotkeys overlays I could fix locally are fixed).
