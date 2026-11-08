# Settings D — modal shell, sidebar, Backups, UVTools, Logging, Updates, About, Sponsors

## Components in scope
| Component | File |
|---|---|
| `SettingsModal` (shell) | `src/components/settings/SettingsModal.tsx` |
| `SettingsSidebarTab`, `TabDescription` | `src/components/settings/SettingsModal.tsx` |
| tab registry (`tabMeta`, `sidebarTopTabs`, `sidebarBottomTabs`) | `src/components/settings/SettingsModal.tsx:1209-1315` |
| About tab body (inline JSX, no component) | `src/components/settings/SettingsModal.tsx:1583-1773` |
| `LocalBackupsSettingsTab` — the live **Backups** tab | `src/components/settings/LocalBackupsSettingsTab.tsx` |
| `BackupsSettingsTab` — **dead code**, see gotcha 1 | `src/components/settings/BackupsSettingsTab.tsx` |
| `UvToolsSettingsTab` | `src/components/settings/UvToolsSettingsTab.tsx` |
| `uvToolsPreferences` | `src/components/settings/uvToolsPreferences.ts` |
| `LoggingSettingsTab`, `LogLineRow`, `getSavedLogLevel`, `saveLogLevel` | `src/components/settings/LoggingSettingsTab.tsx` |
| `UpdatesSettingsTab` | `src/features/updater/UpdatesSettingsTab.tsx` |
| `UpdateCheckerSection` + its nine state bodies | `src/features/updater/UpdateCheckerSection.tsx` |
| `useUpdateChecker` | `src/features/updater/useUpdateChecker.ts` |
| `updateBridge` | `src/features/updater/updateBridge.ts` |
| `updaterMessages` | `src/features/updater/updaterMessages.ts` |
| `debugForceSession` | `src/features/updater/debugForceSession.ts` |
| `SponsorsCarousel`, `CachedAvatar`, `getSponsorRank`, `sponsorInitials` | `src/components/settings/SponsorsCarousel.tsx` |
| fallback data | `src/components/settings/sponsors.json`, `src/components/settings/contributors.json` |
| modal animations / shared CSS | `src/app/globals.css:673-735` |

Shared components this area *uses* but does not own — port them elsewhere and move on:
`StructuredDialogModal` (`src/components/ui/StructuredDialogModal.tsx`), `Tooltip`
(`src/components/ui/Tooltip.tsx`), `NumberInput`, and the `@/components/atoms`
set (`Button`, `IconButton`, `SegmentedControl`, `SettingRow`, `Spinner`,
`ColorSwatchInput`). `useEscapeToClose` (`src/hotkeys/useEscapeToClose.ts`).

Mount site: `src/components/layout/TopBar.tsx:1192-1195` —
`<SettingsModal isOpen={isSettingsOpen} onClose={…setSettingsInitialTab('general')} initialTab={settingsInitialTab} … />`.
`initialTab` is how other surfaces deep-link (e.g. "About" from the app menu).

---

## Surface: `SettingsModal` — the modal shell

- **Source**: `src/components/settings/SettingsModal.tsx:345` (component), render `:1349-1816`, dialogs `:1817-2092`.
- **Renders**: a fixed full-viewport backdrop containing one card
  `header / [sidebar | content] / footer`, plus up to five nested
  `StructuredDialogModal`s painted after the card.

### Layout & styling

Backdrop (`:1349-1353`):

```
className="fixed inset-0 z-50 flex items-stretch justify-center bg-black/60 backdrop-blur-sm p-5 ui-modal-backdrop-enter"
onMouseDown={(e) => { if (e.target === e.currentTarget) handleCancel(); }}
```

`.ui-modal-backdrop-enter` = `animation: uiModalBackdropIn 140ms ease-out`
(`globals.css:728-730`); `uiModalBackdropIn` is opacity 0→1.

Card (`:1355-1361`):

```
className="w-full max-w-[72rem] h-full flex flex-col rounded-2xl shadow-2xl border overflow-hidden ui-modal-panel-enter"
style={{ background: 'var(--surface-0)', borderColor: 'var(--border-strong)',
         boxShadow: '0 26px 64px rgba(0, 0, 0, 0.46)' }}
```

`.ui-modal-panel-enter` = `animation: uiModalPanelIn 180ms ease-out; transform-origin: center`
(`globals.css:732-735`); `uiModalPanelIn` is
`opacity 0→1, translateY(8px) scale(0.985) → translateY(0) scale(1)`.
`max-w-[72rem]` = 1152px. `rounded-2xl` = 16px. `p-5` = 20px backdrop padding.
`bg-black/60` = `#000` at 60 % — the source uses **/60**, not the /55, /58 seen
in other modals of the app.

Header (`:1363-1389`): `className="flex items-center justify-between px-4 py-3"`
with `borderBottom: '1px solid var(--border-subtle)'`. Left cluster
`flex items-center gap-2.5`:

- icon badge `inline-flex h-9 w-9 items-center justify-center rounded-lg border`,
  inline `borderColor: 'var(--border-subtle)'`,
  `background: 'linear-gradient(135deg, color-mix(in srgb, var(--accent), var(--surface-1) 84%), color-mix(in srgb, var(--accent-secondary), var(--surface-1) 90%))'`,
  child `<Settings2 className="h-4.5 w-4.5" style={{ color: 'var(--accent)' }} />`
  (`h-4.5` = 18px; Tailwind v4 arbitrary-ish half-step from the `4.5` scale).
- title `<h2 className="text-base font-semibold" style={{ color: 'var(--text-strong)' }}>Settings</h2>`
- subtitle `<p className="text-xs" style={{ color: 'var(--text-muted)' }}>…</p>`

Right: `<IconButton variant="solid" size="sm" aria-label="Close" onClick={handleCancel}><X className="h-4 w-4" /></IconButton>`.

Body (`:1391`): `className="flex-1 min-h-0 flex"`.

Sidebar column (`:1393-1397`):

```
className="w-72 min-h-0 p-2.5"
style={{ borderRight: '1px solid var(--border-subtle)',
         background: 'linear-gradient(180deg, color-mix(in srgb, var(--surface-1), transparent 6%), color-mix(in srgb, var(--accent-secondary), var(--surface-1) 96%))' }}
```

`w-72` = 288px; `p-2.5` = 10px. The gradient's second leg is a 4 % wash of
`--accent-secondary` over `--surface-1` (so it reads as a very slight green tint
at the bottom).

Sidebar scroller (`:1402-1404`): `className="h-full min-h-0 overflow-y-auto custom-scrollbar flex flex-col"`,
containing top group `className="space-y-1"` and bottom group
`className="mt-auto space-y-1 pt-3"`. Source comment (kept verbatim, `:1398-1401`):

> ```
> {/* No `pr-1` on the scroller: the column's 2.5 already insets the tab
>     cards, and a second inset here stacked on top of the 10px scrollbar
>     gutter, so the cards sat nearly twice as far from the right edge as
>     from the left. */}
> ```

Content column (`:1427-1432`): either

- `usesInternalTabScrollLayout` (About, Hotkeys, Updates):
  `className="flex-1 min-h-0 min-w-0 flex flex-col p-4"`, or
- everything else: `className="flex-1 min-h-0 overflow-y-auto custom-scrollbar p-4"`.

Inside, `<div key={activeTab} className="animate-[settingsTabIn_180ms_ease-out]">`
(plus `flex-1 min-h-0 flex flex-col` on the internal-scroll branch). The
`key={activeTab}` is what re-runs the entry animation on tab change;
`settingsTabIn` = `opacity 0→1, translateY(6px)→0` over 180 ms ease-out
(`globals.css:673-681`).

Footer (`:1775-1815`):

```
className="px-3 py-3 flex items-center justify-between gap-2"
style={{ borderTop: '1px solid var(--border-subtle)',
         background: 'color-mix(in srgb, var(--surface-1), transparent 10%)' }}
```

- Restore Defaults: `Button variant="secondary" size="auto"` with
  `className="!h-10 !px-3.5 !py-0 text-sm gap-1.5 whitespace-nowrap"`,
  `style={accentSecondaryActionStyle92}` and `<RotateCcw className="h-4 w-4 shrink-0" />`,
  label `Restore Defaults`.
- Right cluster `flex items-center gap-2`:
  - Cancel: `Button variant="secondary" size="auto"`,
    `className="!h-10 !px-4 !py-0 text-sm"`, `style={{ color: 'var(--text-muted)' }}`.
  - Apply: `Button variant="primary" size="auto"`,
    `className="!h-10 !px-4 !py-0 text-sm gap-1.5 whitespace-nowrap"`,
    `style={{ background: 'color-mix(in srgb, var(--accent), var(--surface-0) 16%)',
              borderColor: 'color-mix(in srgb, var(--accent), white 10%)' }}`,
    `<Check className="h-4 w-4 shrink-0" />`, label `Apply`.

`accentSecondaryActionStyle92` (`:507-511` in the shell, and defined for the
light theme at `:512-516`) resolves to:

```
{ color: 'color-mix(in srgb, #4f8a08, var(--text-strong) 30%)',            // light
  borderColor: 'color-mix(in srgb, #6aa20d, var(--border-subtle) 34%)',
  background: 'color-mix(in srgb, #6aa20d, var(--surface-1) 80%)' }
```
for `isLightTheme`, otherwise `var(--accent-secondary)` / the 42 % border mix /
the 92 % surface-1 mix. The `LocalBackupsSettingsTab` variant instead uses the
CSS custom properties `--accent-secondary-action-color`,
`--accent-secondary-action-border`, `--accent-secondary-action-bg-92`
(`globals.css:36-39`, light overrides `:102-105`, `:143-146`) — same colours,
one indirection.

### States

1. **Closed** — `if (!isOpen) return null;` (`:1185`). Nothing renders. The
   component *instance* however stays mounted by the parent, so all drafts,
   `activeTab` and dialog flags survive a close.
2. **Opening** — effect `:393-400`: when `isOpen` transitions false→true it sets
   `activeTab = initialTab ?? 'general'`, clears `showReloadPrompt`, and
   snapshots the enabled experiment ids. Second effect `:1055-1067` resets every
   draft from storage on the next animation frame and, on cleanup, restores the
   saved theme preview unless the draft was committed.
3. **Escape** — `useEscapeToClose(isOpen, …)` `:1158-1174`, cascade in order:
   theme-delete confirm → theme-rename dialog → theme-save confirm → `handleCancel()`.
   Source comment: *"Nested dialogs rendered through StructuredDialogModal
   register their own Escape handler and take precedence; the cascade here is
   the settings shell's own dismissal when nothing is layered above it."*
4. **Cancel / backdrop / X / Escape** — `handleCancel` (`:876-888`): closes the
   four shell dialogs, clears theme-name mode state, clears
   `didCommitThemeDraftRef`, restores the saved theme preview, calls
   `resetDraftFromProps()`, then `requestClose()`.
5. **Apply** — `handleApply` (`:968-1128`): pushes every draft out, persists the
   bridge-backed ones, marks the theme draft committed, then `requestClose()`.
   It does **not** itself close the modal; `requestClose` does.
6. **Experiments-changed gate** — `requestClose` (`:402-416`) compares the live
   enabled-experiment id list against the open-time snapshot; if it differs it
   sets `showReloadPrompt` instead of closing. Only the reload dialog's
   `Not Now` closes.
7. **Restore Defaults confirm** — `Restore Defaults` sets
   `showRestoreDefaultsConfirm`; the dialog (`z-[70]`, `max-w-md`) confirms into
   `applyRestoreDefaultsToDraft()` (`:890-926`), which writes the literals
   `DEFAULT_*` into the drafts (including `DEFAULT_UVTOOLS_SETTINGS`); nothing is
   written until Apply.
8. **Theme dialogs** (`showThemeSaveConfirm`, `showThemeDeleteConfirm`,
   `showThemeRenameDialog` at `z-[72..74]`) — owned by the UI & Theme tab; name
   them, do not port here.

### Data

- `activeTab: SettingsTabKey` (`:383`), `previousTabRef` (`:418`) — the tab the
  user came from, consumed only by the Experiments disclaimer exit.
- One draft per setting the shell owns. For *this* area the relevant drafts are
  `draftUvToolsSettings` (`:486`, seeded from `getSavedUvToolsSettings()`),
  `draftLogLevel` (`:487`, from `getSavedLogLevel()`) and `updateChannel`
  (`:488`, **not** a draft: loaded from the backend on mount by
  `getUpdateChannel().then(setUpdateChannel)` `:519-521`).
- Build identity constants (`:150-166`): `DRAGONFRUIT_VERSION`
  (`process.env.NEXT_PUBLIC_APP_VERSION ?? '0.0.0'`), `DRAGONFRUIT_BUILD_CHANNEL`
  (`NEXT_PUBLIC_BUILD_CHANNEL ?? 'mainline'`, trimmed and lowercased),
  `DRAGONFRUIT_GIT_COMMIT`, `DRAGONFRUIT_GIT_REF`,
  `BUILD_OS_LABELS = { darwin: 'macOS', win32: 'Windows', linux: 'Linux' }`,
  `DRAGONFRUIT_BUILD_OS`, `DRAGONFRUIT_BUILD_ARCH`,
  derived `DRAGONFRUIT_BUILD_PLATFORM` (`"macOS/arm64"`),
  derived `DRAGONFRUIT_GIT_BUILD_LABEL` (`"dev @ 62e80c79b · macOS/arm64"`),
  `ORA_LOGO_DARK_URL = '/dragonfruit_assets/branding/open_resin_alliance_logo_darkmode.png'`,
  `DRAGONFRUIT_REPO_URL = 'https://github.com/Open-Resin-Alliance/DragonFruit'`.
- `isLightTheme` (`:503` + effect `:1070-1103`) is derived from
  `document.documentElement[data-theme]` with a `prefers-color-scheme` fallback
  and a `MutationObserver` on `data-theme`.

### Callbacks

| Element | Handler | Observable result |
|---|---|---|
| sidebar tab | `handleSelectTab(tab)` (`:419`) | `previousTabRef = activeTab`; `activeTab = tab` |
| backdrop `onMouseDown` (target === currentTarget) | `handleCancel` | drafts reset, theme preview restored, `requestClose()` |
| header X | `handleCancel` | same |
| footer Cancel | `handleCancel` | same |
| footer Apply | `handleApply` | every draft written/persisted, `requestClose()` |
| footer Restore Defaults | `handleRestoreDefaults` → `setShowRestoreDefaultsConfirm(true)` | dialog |
| restore dialog `Keep Current` / close / backdrop | `setShowRestoreDefaultsConfirm(false)` | dialog closes |
| restore dialog `Restore Defaults` | `applyRestoreDefaultsToDraft()` + close | drafts become `DEFAULT_*` |
| reload dialog `Not Now` | clear flag + `onClose()` | modal closes, experiments unchecked until restart |
| reload dialog `Reload Now` | `window.location.reload()` | app restarts |
| Escape | cascade above | innermost open dialog closes, else the modal |

### Text (verbatim, msgid = the string)

| Where | Text |
|---|---|
| header title | `Settings` |
| header subtitle | `Customize DragonFruit behavior, visuals, and controls.` |
| close button | aria-label `Close` |
| footer | `Restore Defaults`, `Cancel`, `Apply` |
| restore dialog | title `Restore Defaults?`; subtitle `This resets settings in this dialog to their default values.`; body `You can still review the changes before saving. Nothing is written until you click ` + bold `Apply` + `.`; buttons `Keep Current`, `Restore Defaults`; aria-label `Confirm restore defaults`, close aria `Close restore defaults confirmation` |
| reload dialog | title `Apply Experiments`; subtitle `Experiment changes take effect after a restart.`; body `Reload DragonFruit now to apply your experiment changes? Any unsaved changes to the current scene will be lost.`; buttons `Not Now`, `Reload Now`; aria `Apply experiments`, close aria `Close restart prompt` |

No `msg` wrapper on the shell strings: they are plain JSX literals (not
extracted). Only `tabMeta` labels/descriptions, and the About strings that go
through `<Trans>`, are in the catalogue.

### Port target

`gpui/src/features/settings/mod.rs` — already holds the whole shell:
`settings_modal` (`:369-447`), `header` (`:451`), `sidebar` (`:514`),
`rail_tab`, `content` (tab dispatch + `internal_scroll` match),
`footer` (`:626`), `restore_defaults_dialog`, `close`, and the
`SettingsTab` enum with `ALL`/`TOP`/`BOTTOM` and `meta()`.
Geometry constants live in `gpui/src/features/settings/defaults.rs:4-6`:
`SIDEBAR_WIDTH = 288.0` (`w-72`), `CARD_MAX_WIDTH = 1152.0` (`max-w-[72rem]`),
`BACKDROP_PADDING = 20.0` (`p-5`), `CONTROL_WIDTH = 144.0` (`w-36`).

Known deviations already taken (do not re-fix): Apply/Cancel only close the
modal (the store owns persistence); the active tab's 1 px inset `boxShadow` ring
is omitted; backdrop blur, `z-index` stacking and CSS transitions have no GPUI
equivalent; the experiments-changed reload prompt is **not** ported.

---

## Surface: `SettingsSidebarTab` — one sidebar entry

- **Source**: `src/components/settings/SettingsModal.tsx:232-293`. Doc comment
  (`:225-231`) quoted in full because it explains the compactness:

  > ```
  > * One settings sidebar entry: icon, label and its description. The description
  > * stays visible; the compactness comes from dropping the icon's own 24px box,
  > * `leading-tight` and `px-2.5 py-2`, which matches the inset of the setting rows
  > * beside it, and the icon keeps the full glyph size instead of sitting in a box
  > * that padded it away from the card edge.
  > ```

- **Renders**: a full-width `<button type="button">` → `flex items-center gap-2.5`
  → 20 px icon + (label over description).
- **Layout & styling**:

  ```
  className="w-full rounded-sm border px-2.5 py-1.5 text-left transition-all duration-150"
  ```
  active:
  ```
  borderColor: `color-mix(in srgb, ${tabColor}, var(--border-subtle) 35%)`
  background:  `color-mix(in srgb, ${tabColor}, var(--surface-0) 84%)`
  boxShadow:   `0 0 0 1px color-mix(in srgb, ${tabColor}, transparent 76%) inset`
  ```
  inactive:
  ```
  borderColor: 'var(--border-subtle)'
  background:  'var(--surface-1)'
  ```
  `rounded-sm` = 4px, `px-2.5 py-1.5` = 10px / 6px.

  Icon: `<Icon className="h-5 w-5 shrink-0" style={{ color: active ? tabColor : 'var(--text-muted)' }} />`.
  Label: `<span className="block text-[15px] font-semibold leading-tight" style={{ color: 'var(--text-strong)' }}>`.
  Description: `TabDescription` — `block min-w-0 flex-1 truncate text-xs leading-tight`
  on `var(--text-muted)`.
  `tabColor = meta.tone === 'secondary' ? 'var(--accent-secondary)' : 'var(--accent)'`.

- **States**: default / active (above) / hover — none of its own, only
  `transition-all duration-150`; the sidebar cards have no hover style.
- **Data**: `meta` from `tabMeta`, `active` from `activeTab === tabId`.
- **Callbacks**: `onClick={() => onSelect(tabId)}` → `handleSelectTab`.
- **Text**: from `tabMeta`.
- **Port target**: `mod.rs::rail_tab` (exists). Note the Rust port uses
  4px gap (`space-y-1`) and reproduces border/fill but not the inset ring.

### Sidebar registry — order, labels, descriptions, icons, tone

`sidebarTopTabs` (`:1314`) and `sidebarBottomTabs` (`:1315`), verbatim:

```ts
const sidebarTopTabs: SettingsTabKey[] = ['general', 'camera', 'workspaces', 'multiPlate', 'mesh', 'performance', 'spacemouse', 'ui', 'hotkeys'];
const sidebarBottomTabs: SettingsTabKey[] = ['plugins', 'experiments', 'sceneAutosave', 'backups', 'uvtools', 'logging', 'updates', 'about'];
```

`tabMeta` (`:1209-1313`, richer than `SettingsTabKey` order — `camera` sits
before `mesh` in the key union, the arrays rule the display order):

| key | label | description | lucide icon | tone |
|---|---|---|---|---|
| `general` | `General` | `Workspace behavior and panel layout` | `Settings2` | primary |
| `camera` | `Camera` | `Projection and navigation behavior` | `Camera` | primary |
| `workspaces` | `Workspace` | `Build volume bounds, origin, and 3D view resolution` | `MonitorCog` | primary |
| `multiPlate` | `Multi-Plate` | `Plate behaviour across several beds` | `SquareStack` | primary |
| `mesh` | `Mesh` | `Shader, rendering options, and selection behavior` | `Grid3x3` | primary |
| `performance` | **`Slicing`** | `PNG compression and engine metadata` | `MonitorCog` | primary |
| `spacemouse` | **`3D Mouse`** | `3D mouse navigation controls` | `Gamepad2` | primary |
| `ui` | `UI & Theme` | `Theme and custom UI token customization` | `Palette` | primary |
| `hotkeys` | `Hotkeys` | `Keyboard bindings and presets` | `Keyboard` | primary |
| `plugins` | `Plugins` | `Load vendor profile plugins` | `Plug` | secondary |
| `experiments` | `Experiments` | `Early-access and experimental features` | `FlaskConical` | secondary |
| `sceneAutosave` | `Scene Autosave` | `Autosave and crash recovery behavior` | `HardDrive` | secondary |
| `backups` | `Backups` | `Local on-disk backup snapshots` | `ArchiveRestore` | secondary |
| `uvtools` | `UVTools` | `Send sliced files to UVTools for analysis` | `ExternalLink` | secondary |
| `logging` | `Logging` | `Log file location and verbosity` | `ScrollText` | secondary |
| `updates` | `Updates` | `Check for new versions and manage channels` | `CloudDownload` | secondary |
| `about` | `About` | `Version info and project details` | `Info` | secondary |

- **Port target**: `SettingsTab`/`meta()` in `mod.rs` (exists, same 17 entries,
  same split 9 top / 8 bottom, icon slugs identical). Note the Rust
  `Multi-Plate` slug key is `multi-plate` and `slicing`; the top/bottom arrays
  are `SettingsTab::TOP` (9) and `SettingsTab::BOTTOM` (8).

---

## Surface: `TabDescription` — clipped-description tooltip

- **Source**: `SettingsModal.tsx:295-343`. Doc comment `:272-294` (quoted in
  full, because two invariants depend on it):

  > ```
  > * A sidebar description line that only offers a tooltip when it is actually
  > * clipped. Translated descriptions run much longer than the English ones — the
  > * Spanish "Comportamiento del espacio de trabajo y disposición de los paneles"
  > * is more than twice "Workspace behavior and panel layout" — so the truncation
  > * that never triggered in English hides most of the sentence in other locales.
  > *
  > * Two traps this deliberately avoids:
  > *
  > * 1. Measuring the truncating span itself. `Tooltip` renders no wrapper at all
  > *    when its content is falsy, so a measurement taken on the span reacts to
  > *    the very wrapper it decides to add — the element measures, gets wrapped,
  > *    measures differently, gets unwrapped. The reference box here is the outer
  > *    span, whose width is set by the sidebar and never by this decision, and
  > *    the text is measured through a Range, which reports the full laid-out
  > *    width regardless of what clips it.
  > * 2. Measuring once. A late webfont changes text metrics after the first pass,
  > *    and the outer box never resizes, so a ResizeObserver alone would keep a
  > *    stale answer forever. Hence the `fonts.ready` re-measure.
  > ```

- **States**: `isClipped=false` → `Tooltip content={null}` (renders nothing);
  `isClipped=true` → tooltip with the description, `fullWidth maxWidth={280}`.
  Clip test: `textWidth > outer.clientWidth + 1` (the 1 px slack absorbs
  sub-pixel rounding).
- **Port target**: `mod.rs::rail_tab` uses `.truncate()` with no tooltip — a
  deliberate simplification to record, not to invent hover machinery for.

---

## Surface: `LocalBackupsSettingsTab` — the **Backups** tab

- **Source**: `src/components/settings/LocalBackupsSettingsTab.tsx` (1360 lines);
  rendered at `SettingsModal.tsx:1564` as `{activeTab === 'backups' && <LocalBackupsSettingsTab />}`.

### Renders

`<div className="space-y-3">` with, in order:

1. **Local On-Disk Backups** card (`:711-800`)
2. **Backup Management** card (`:802-1000`) containing three inner cards:
   Quick actions, Automation, Manage Backups
3. Delete-confirm dialog, Restore-confirm dialog (both `StructuredDialogModal`)
4. Snapshot-content modal (custom `fixed inset-0 z-[90]` overlay, `max-w-6xl h-[820px] max-h-[94vh]`)
5. A message strip at the bottom when `message.kind !== 'idle'`

The non-desktop early return (`:700-707`):

```jsx
<div className="rounded-md border p-3 text-xs" style={{ borderColor: 'var(--border-subtle)', background: 'var(--surface-1)', color: 'var(--text-muted)' }}>
  <Trans>Local on-disk backups are currently available in the DragonFruit desktop build only.</Trans>
</div>
```

### Layout & styling

Section 1 — `className="relative rounded-md border p-3"`, inline
`borderColor: 'var(--border-subtle)'`, `background: 'var(--surface-1)'`.
Header: icon chip `inline-flex h-8 w-8 items-center justify-center rounded-md border`
with `background: 'color-mix(in srgb, var(--surface-2), transparent 8%)'` and
`<HardDrive className="h-4 w-4" style={{ color: 'var(--accent)' }} />`;
title `text-xs font-semibold` on `--text-strong` — `Local On-Disk Backups`;
description `mt-0.5 text-xs leading-snug` on `--text-muted` —
`Automatically saves snapshots of your settings and profiles to a folder on your computer.`

Loading branch: `mt-2 text-xs inline-flex items-center gap-1.5` + `<Spinner size="sm" />` +
`Loading backup status…`.

Loaded branch: `mt-2 grid gap-2 sm:grid-cols-2` of four tiles, each
`rounded-md border px-2.5 py-2` on `--surface-0`; tile label
`text-[11px] uppercase tracking-wide font-semibold` on `--text-muted`; value
`mt-1 text-xs font-medium` (or `text-xs` for dates) on `--text-strong`:

| label | value |
|---|---|
| `Default directory` | `defaultDirectory || <Trans>Resolving...</Trans>` |
| `Selected directory` | `selectedDirectory || <Trans>Not selected</Trans>` |
| `Last backup on disk` | `stateUpdatedAt ? new Date(stateUpdatedAt).toLocaleString() : <Trans>Never</Trans>` |
| `Last local sync` | `lastLocalSyncAt ? new Date(lastLocalSyncAt).toLocaleString() : <Trans>Never</Trans>` |

Action row (`:765-800`): grid `sm:grid-cols-2`, or `sm:grid-cols-3` when
**not** using the default directory. Buttons are all
`Button size="auto" className="!h-9 !px-3 !py-0 text-sm gap-1.5 disabled:opacity-60"`,
`disabled={busy !== 'none'}`:

- `Use Default Path` (only when `!isUsingDefault && defaultDirectory`) —
  `variant="secondary"`, `<CheckCircle2 className="h-4 w-4" style={{ color: 'var(--text-muted)' }} />`.
- `Choose Folder` — `variant="tinted-accent"`; spinner while
  `busy === 'choose-directory'`, else `<FolderOpen className="h-4 w-4" />`.
- `Open Folder` — `variant="secondary"` with
  `borderColor: 'color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 45%)'`,
  `background: 'color-mix(in srgb, var(--accent-secondary), var(--surface-1) 86%)'`,
  `color: 'var(--accent-secondary)'`; `disabled={!selectedDirectory || busy !== 'none'}`;
  spinner while `busy === 'reveal'`.

`isUsingDefault = !defaultDirectory || selectedDirectory.trim().toLowerCase() === defaultDirectory.trim().toLowerCase()`.

Section 2 header: icon chip with `<RefreshCcw className="h-4 w-4" style={{color:'var(--accent)'}} />`,
title `Backup Management`, description
`Save snapshots to your local backup folder, then review and restore any point-in-time snapshot in-app.`

Inner cards are `mt-2 rounded-md border p-2.5` on `--surface-0` with a
`text-[11px] uppercase tracking-wide font-semibold` group label + a `text-xs`
`mt-0.5` line.

*Quick actions* — label `Quick actions`, line `Create a snapshot now and refresh history.`,
then `mt-2 grid gap-2 sm:grid-cols-2`:

- `Backup Now` — `variant="tinted-accent"`, `!h-9 !px-3 !py-0 text-sm gap-1.5 disabled:opacity-60`,
  `disabled={busy !== 'none' || !selectedDirectory}`, spinner on `busy === 'sync'`,
  else `<UploadCloud className="h-4 w-4" />`.
- `Refresh History` — `variant="secondary"`, same size classes,
  `disabled={busy !== 'none' || historyLoading || !selectedDirectory}`,
  spinner on `historyLoading`, else `<RefreshCcw className="h-4 w-4" />`.

*Automation* — label `Automation`, then `mt-2 grid gap-2` of two `SettingRow`s
(`bordered`, `surface="raised"`):

- `Enable automatic backups` / `Automatically write snapshots to local disk on an interval.`
  with a `SegmentedControl` `label={_(msg\`Enable automatic backups\`)}` whose options are
  `[{ value: 'off', label: <Trans>OFF</Trans> }, { value: 'on', label: <Trans>ON</Trans> }]`,
  `value={autoSyncEnabled ? 'on' : 'off'}`.
- `Sync interval` / `Minutes between automatic local backups.`, `disabled={!autoSyncEnabled}`,
  a `NumberInput` `min={1} max={240} step={1}`, `className="ui-input h-[34px] w-[120px] pl-2.5 pr-5 py-1.5 text-sm"`,
  clamped by the handler to `Math.max(1, Math.min(240, Math.round(next)))`, plus a
  trailing `<span className="text-xs" style={{ color: 'var(--text-muted)' }}>min</span>`.

Then `mt-2 text-xs` on `--text-muted`:
`Last local sync: {lastLocalSyncAt ? new Date(lastLocalSyncAt).toLocaleString() : _(msg\`never\`)}`.

*Manage Backups* — header row `flex items-center justify-between gap-2` with label
`Manage Backups`, line `View, restore, or delete older snapshots from disk.`, and a
`Refresh` button (`variant="secondary"`, `!h-8 !px-2.5 !py-0 text-xs gap-1.5 disabled:opacity-60`,
`disabled={historyLoading || !selectedDirectory}`, `<RefreshCcw className="h-3.5 w-3.5" />`).
List container: `mt-2 max-h-64 overflow-auto custom-scrollbar`.

Each row: `<li className="flex items-center justify-between gap-2 rounded-md border px-2.5 py-2">`
on `--surface-1`:

- main clickable area `min-w-0 flex-1 text-left` → `truncate text-xs font-medium`
  on `--text-strong` = `formatHistoryDate(item)`, then `text-xs` on
  `--text-muted` = `ID {item.id}`;
- two 28 px icon buttons (`h-7 w-7`, `rounded border`): `Eye` in
  `accentSecondaryActionStyle92` with `title={_(msg\`View snapshot\`)}`, `Trash2`
  in `dangerActionStyle92` with `title={_(msg\`Delete snapshot\`)}`.

`dangerActionStyle92` = `{ color: 'var(--danger)', borderColor: 'color-mix(in srgb, #ef4444, var(--border-subtle) 45%)', background: 'color-mix(in srgb, #ef4444, var(--surface-1) 86%)' }`.

Empty list text (`text-xs` on `--text-muted`, `px-3 py-3`):
- `hasAnySync` → `No history snapshots found in this folder.`
- otherwise → `No snapshots yet. Run "Backup Now" to create your first local snapshot.`

Message strip (`:1350-1358`): `rounded-md border px-3 py-2 text-xs`, border
`message.kind === 'error' ? 'color-mix(in srgb, #ef4444, var(--border-subtle) 40%)' : 'color-mix(in srgb, #22c55e, var(--border-subtle) 40%)'`,
background `var(--surface-1)`, colour `#fca5a5` (error) / `#86efac` (success).

### Dialogs and the snapshot modal

**Delete** (`:990-1010`): `StructuredDialogModal`, `open={Boolean(confirmDeleteId)}`,
aria-label `Confirm delete snapshot`, title `Delete Backup Snapshot`,
subtitle `This action cannot be undone.`, icon `Trash2`, `iconTone="danger"`,
`zIndexClassName="z-[100]"`, close aria `Cancel delete`; buttons `Cancel`
(secondary, md) and `Delete` (`tinted-danger`, md, `gap-1.5`, `Trash2 h-3.5 w-3.5`,
`disabled={busy !== 'none'}`). Body: when an id is pending,
`formatSnapshotDeletionWarning(_, new Date(Number(confirmDeleteId)).toLocaleString())`
= `Backup snapshot from {timestamp} will be permanently removed from disk.`;
otherwise `This backup snapshot will be permanently removed from disk.`

**Restore** (`:1012-1040`): aria-label `Confirm restore snapshot`, title
`Restore Snapshot`, subtitle `Snapshot from {new Date(Number(confirmRestoreId)).toLocaleString()}`,
icon `AlertTriangle`, `iconTone="warning"`, close aria `Cancel restore`; buttons
`Cancel` and `Yes, restore` (secondary, md, `accentSecondaryActionStyle92`,
`ArchiveRestore h-3.5 w-3.5`). Body: `This will overwrite your current app settings and profiles with the data from this snapshot, then reload the app. This action cannot be undone.`

**Snapshot content modal** (`:1054-1330`): custom overlay
`fixed inset-0 z-[90] flex items-center justify-center bg-black/60 backdrop-blur-sm p-4 ui-modal-backdrop-enter`,
`role="dialog" aria-modal="true"`, backdrop mousedown on itself closes. Card:
`w-full max-w-6xl h-[820px] max-h-[94vh] flex flex-col rounded-xl border shadow-2xl overflow-hidden ui-modal-panel-enter`
with `borderColor: 'var(--border-strong)'`, `background: 'var(--surface-0)'`.
Header row `flex items-center justify-between gap-2 px-4 py-3` on
`background: 'color-mix(in srgb, var(--surface-1), transparent 8%)'`:
`<h4 className="text-sm font-semibold">Backup Snapshot Content</h4>` and a
subtitle `Snapshot {new Date(Number(selectedHistoryId)).toLocaleString()}` (or
`Loading snapshot...`), plus `Restore to App` (secondary, `!h-8 !px-2.5 !py-0 text-xs gap-1.5 disabled:opacity-60`,
`ArchiveRestore`) and an `IconButton` `aria-label={_(msg\`Close snapshot details\`)}` with `X`.

Body grid `h-full min-h-0 grid gap-3 lg:grid-cols-[190px_minmax(0,1fr)]`:
left `<aside className="h-full min-h-0 rounded-lg border p-2 overflow-auto custom-scrollbar">`
on `--surface-1`, with four tab buttons
(`w-full text-left rounded-md px-2 py-1.5 text-xs font-medium transition-colors`):
`Overview`, `Local Storage`, `Profiles`, `Raw JSON`. Active tab:
`background: 'color-mix(in srgb, var(--accent-secondary), var(--surface-1) 88%)'`,
`color: 'var(--accent-secondary)'`,
`border: '1px solid color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 40%)'`;
inactive: `color: 'var(--text-muted)'`, `border: '1px solid transparent'`.

Panels:

- **Overview** — `grid gap-2 sm:grid-cols-2 lg:grid-cols-4` of fact tiles
  (`rounded-md border px-2.5 py-2`, `--surface-0`); labels `Snapshot ID`,
  `Document Updated`, `Client ID`, `LocalStorage Keys`; values the id,
  `new Date(selectedHistoryDocument.updatedAt).toLocaleString()`,
  `snapshot.clientId`, and the key count.
- **Local Storage** — grid `lg:grid-cols-[minmax(220px,30%)_minmax(0,1fr)]`.
  Left: key list (`mb-1 last:mb-0 w-full rounded-md border px-2 py-1.5 text-left text-xs`);
  selected key gets
  `borderColor: 'color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 35%)'`,
  `background: 'color-mix(in srgb, var(--accent-secondary), var(--surface-1) 92%)'`,
  `color: 'var(--accent-secondary)'`. Right: label (uppercase 11 px) + a `<pre>`
  with `borderColor: 'color-mix(in srgb, #3f4451, var(--border-subtle) 35%)'`,
  `background: '#282c34'`, `color: '#abb2bf'`, showing
  `renderHighlightedJson(stringifyReadable(value))`. Empty states:
  `No LocalStorage keys found.`, `Select a key`, `Select a LocalStorage key from the left to view its value.`
- **Profiles** — two-column grid `lg:grid-cols-[minmax(230px,32%)_minmax(0,1fr)]`.
  Left: `Printer Profiles ({count})`, each printer a button showing
  `name ?? id`, `manufacturer ?? _(msg\`Unknown manufacturer\`)`,
  `formatMaterialCountLabel(_, materialCount)` and an `ACTIVE` badge when
  `activePrinterProfileId === printer.id`; selection style
  `borderColor color-mix(accent-secondary, border-subtle 35%)`,
  `background color-mix(accent-secondary, surface-1 94%)` / `94 %` on surface-0 for
  material tiles. Right: detail card with name, `manufacturer • networkSupport`,
  `Build: {w} × {d} × {h} mm`, `Display: {rx} × {ry}` + `({outputFormat})`,
  then `Materials for this printer ({n})` list showing brand • resinFamily and
  `Layer {h} mm • Normal {n}s • Bottom {n}s × {n}`. Empty states:
  `No printer profiles.`, `Select a printer profile to view details.`,
  `No materials linked to this printer.`, `Unknown brand`.
  If `parseProfilesSnapshot` returns null the raw JSON `<pre>` is shown instead.
- **Raw JSON** — full highlighted `<pre>` with the same dark palette, `maxHeight`
  not set (flex-1).

JSON syntax highlighting colours (`getTokenColor`): key `#f472b6`, string
`#a5b4fc`, number `#facc15`, boolean `#22d3ee`, null `#94a3b8`.

### States

- `!desktopAvailable` → single-card fallback text (above).
- `loadingStatus` → `Loading backup status…` spinner row; otherwise the four tiles.
- `busy` ∈ `'none' | 'choose-directory' | 'sync' | 'restore' | 'reveal'` disables
  every action button and swaps the corresponding icon for a spinner.
- history: empty (two texts), loading (`Refresh` spinner), populated list.
- snapshot modal: not loaded (`Loading snapshot content…` spinner), loaded.
- message: `idle` (strip hidden) / `success` (green) / `error` (red).
- Confirm dialogs only open from a row / the modal header.

### Data

Persisted keys (all `localStorage`, all owned by this tab):

| key | meaning | default |
|---|---|---|
| `dragonfruit-local-backups:auto-sync-enabled` | written `'true'`/`'false'` | `true` (`!== 'false'`) |
| `dragonfruit-local-backups:auto-sync-minutes` | clamped 1–240 | `15` |
| `dragonfruit-local-backups:client-id` | uuid v4 | created on first snapshot |
| `dragonfruit-local-backups:last-sync-at` | ISO timestamp | — |
| `dragonfruit-local-backups:directory` | chosen folder | — (removed when empty) |

`KNOWN_LOCAL_STORAGE_KEYS` (`:145-162`) plus the prefix rules
(`dragonfruit-`, `app-`, `lumenslicer:`) decide what `collectSnapshot` copies.

Backend commands (`invokeDesktop`, all Tauri, all guarded by
`'__TAURI_INTERNALS__' in window`):

| command | args | result |
|---|---|---|
| `local_backup_default_directory` | — | default path (`<app data>/backups`) |
| `local_backup_read_state` | `{ directoryPath }` | `{ documentJson, updatedAt }` |
| `local_backup_sync` | `{ directoryPath, documentJson }` | `{ syncedAt, historyId, statePath, historyPath }` |
| `local_backup_list_history` | `{ directoryPath }` | `LocalBackupHistoryEntry[]` |
| `local_backup_read_history_item` | `{ directoryPath, id }` | `{ documentJson }` |
| `local_backup_delete_history_item` | `{ directoryPath, id }` | `boolean` |
| `local_backup_restore_history_item` | `{ directoryPath, id }` | — |
| `local_backup_pick_directory` | `{ currentPath }` | chosen path |
| `reveal_in_file_manager` | `{ path }` | — |

Snapshot shape written:

```json
{ "source": "dragonfruit", "schemaVersion": 1, "updatedAt": "<iso>",
  "snapshot": { "version": 1, "updatedAt": "<iso>", "clientId": "<uuid>",
                "localStorage": { "<key>": "<raw string>" }, "profiles": { … } } }
```

`profiles` is `getProfileStoreSnapshot()` passed through untouched.

### Callbacks

| Element | Handler | Result |
|---|---|---|
| `Choose Folder` | `handleChooseDirectory` | native picker → `selectedDirectory`, reload state+history, success message |
| `Use Default Path` | `handleUseDefaultDirectory` | sets default path, reloads, message |
| `Open Folder` | `handleRevealDirectory` | `reveal_in_file_manager` |
| `Backup Now` | `runSync()` | collects snapshot, `local_backup_sync`, writes `LAST_SYNC_AT_KEY`, message `Backup saved to local disk.`, refreshes history |
| `Refresh History` / `Refresh` | `loadHistory()` | replaces `historyItems` |
| row body / `Eye` | `handleViewHistory(id)` | opens the snapshot modal, loads the document, defaults tab to `overview` |
| `Trash2` | `setConfirmDeleteId(id)` | delete dialog |
| delete dialog `Delete` | `handleDeleteHistory(id)` | `local_backup_delete_history_item`, closes the modal if it was that id, refreshes, message `Backup snapshot deleted.` |
| modal header `Restore to App` | `setConfirmRestoreId(selectedHistoryId)` | restore dialog |
| restore dialog `Yes, restore` | `handleRestoreHistory(id)` | restores on disk; if the modal holds that snapshot it calls `applySnapshotToLocalApp(snapshot)` → writes every key, sets `LAST_SYNC_AT_KEY`, `window.location.reload()` |
| modal `X` / backdrop / Escape | close modal | — |

Effects:

- init (`:611-651`): if desktop, read default dir, then
  `nextDirectory = selectedDirectory || default`, `loadState` + `loadHistory`.
- persist `autoSyncEnabled` / `autoSyncMinutes` / `selectedDirectory` on change.
- auto-sync timer (`:686-699`): when desktop, enabled and a directory exists,
  `window.setInterval(() => void runSync(), autoSyncMinutes * 60 * 1000)`.
  **No immediate backup on mount** — the first automatic snapshot happens one
  full interval after the tab is first opened (and the interval is recreated
  whenever `selectedDirectory`, the enabled flag or the minutes change).

### Text (verbatim)

`Local On-Disk Backups`, `Automatically saves snapshots of your settings and profiles to a folder on your computer.`,
`Loading backup status…`, `Default directory`, `Resolving...`, `Selected directory`,
`Not selected`, `Last backup on disk`, `Last local sync`, `Never`,
`Use Default Path`, `Choose Folder`, `Open Folder`, `Backup Management`,
`Save snapshots to your local backup folder, then review and restore any point-in-time snapshot in-app.`,
`Quick actions`, `Create a snapshot now and refresh history.`, `Backup Now`,
`Refresh History`, `Automation`, `Enable automatic backups`,
`Automatically write snapshots to local disk on an interval.`, `OFF`, `ON`,
`Sync interval`, `Minutes between automatic local backups.`, `min`,
`Last local sync: {…}` / msgid `never`, `Manage Backups`,
`View, restore, or delete older snapshots from disk.`, `Refresh`,
`No history snapshots found in this folder.`,
`No snapshots yet. Run "Backup Now" to create your first local snapshot.`,
`ID {item.id}`, tooltips `View snapshot` / `Delete snapshot`,
`Delete Backup Snapshot`, `This action cannot be undone.`, `Cancel delete`,
`Cancel`, `Delete`, `Backup snapshot from {timestamp} will be permanently removed from disk.`,
`This backup snapshot will be permanently removed from disk.`,
`Confirm restore snapshot`, `Restore Snapshot`, `Snapshot from {…}`, `Cancel restore`, `Yes, restore`,
`This will overwrite your current app settings and profiles with the data from this snapshot, then reload the app. This action cannot be undone.`,
`Backup Snapshot Content`, `Snapshot {…}`, `Loading snapshot...`, `Restore to App`,
`Close snapshot details`, `Loading snapshot content…`, `Overview`, `Local Storage`,
`Profiles`, `Raw JSON`, `Snapshot ID`, `Document Updated`, `Client ID`,
`LocalStorage Keys`, `No LocalStorage keys found.`, `Select a key`,
`Select a LocalStorage key from the left to view its value.`,
`Printer Profiles ({count})`, `No printer profiles.`, `Unknown manufacturer`, `ACTIVE`,
`Select a printer profile to view details.`, `Build: {w} × {d} × {h} mm`,
`Display: {rx} × {ry}`, `Materials for this printer ({count})`,
`No materials linked to this printer.`, `Unknown brand`,
`Layer {h} mm • Normal {n}s • Bottom {n}s × {n}`,
`Failed to load local backup history.`, `Choose a local backup directory before syncing.`,
`Local backup sync failed.`, `Local backup directory updated.`,
`Failed to choose local backup directory.`, `Using default local backup directory.`,
`Failed to open folder in file manager.`, `Backup snapshot is malformed.`,
`Failed to load backup snapshot.`, `Failed to delete backup snapshot.`,
`Snapshot restored to current local backup state.`, `Failed to restore backup snapshot.`,
`Failed to initialize local backups.`, `Backup saved to local disk.`,
`Backup snapshot deleted.`, `Local on-disk backups are currently available in the DragonFruit desktop build only.`

### Port target

`gpui/src/features/settings/backups.rs` — `backups_tab` (`:5`) and
`backups_overlay` (`:322`) exist: both cards, the four tiles
(`DEFAULT DIRECTORY` / `SELECTED DIRECTORY` / `LAST BACKUP ON DISK` /
`LAST LOCAL SYNC` all `Resolving...`/`Never` where the bridge is absent), the
three inner cards, the automation rows, the history list box and the two confirm
dialogs. State: `BackupsState` (`state.rs:510-517`) with `selected_directory`,
`auto_sync_enabled`, `auto_sync_minutes`, `confirm_delete_id`,
`confirm_restore_id`, `message`. The Rust tab deliberately has no history rows
(no bridge), so the empty-state and the dialogs' open-only-from-a-row path are
represented but unreachable; wiring the `local_backup_*` commands (they exist in
`gpui/backend/src/backup.rs`) is the remaining work, together with the snapshot
content modal, which is **not** ported at all.

---

## Surface: `BackupsSettingsTab` — **dead, do not port**

- **Source**: `src/components/settings/BackupsSettingsTab.tsx` (1673 lines).
- **Evidence it is dead**: nothing in `src/` imports it. The only references are
  its own definition (`:428`) and `.po` source references for its strings. The
  sidebar's `backups` entry mounts `LocalBackupsSettingsTab`.
- **What it is**: the earlier *GitHub private-repository* backup tab — a
  Next.js-route-backed (web) implementation calling
  `/api/backups/github/auth/{start,status,callback,logout}`,
  `/api/backups/github/sync`, `/api/backups/github/repo/ensure`, and
  `/api/backups/github/history[/{id}]`, with a popup OAuth flow, a
  repo-name picker, conflict/force-push handling and the same snapshot viewer.
  `StatusResponse` shape at `:39-58`, storage keys `dragonfruit-backups:*`
  (`:105-113`), `DEFAULT_BACKUP_REPO_NAME = 'dragonfruit-backups'`.
- **Port target**: none. Delete-equivalent (leave the TS file alone; do not
  build Rust for it). If a future ticket revives GitHub backup it is a *server*
  feature, not a settings tab.

---

## Surface: `UvToolsSettingsTab` — UVTools integration

- **Source**: `src/components/settings/UvToolsSettingsTab.tsx` (full file);
  rendered at `SettingsModal.tsx:1565-1570` with
  `uvToolsSettings={draftUvToolsSettings}` /
  `onUvToolsSettingsChange={setDraftUvToolsSettings}`.
- **Renders**: `div className="space-y-3"` → one
  `<section className="rounded-md border p-3">`, header row, the enable row, and
  (only when enabled) the path card.

### Layout & styling

Section: `background: 'var(--surface-1)'`, `borderColor: 'var(--border-subtle)'`.

Header: `flex items-start gap-2`; icon chip `inline-flex h-8 w-8 items-center justify-center rounded-md border`
with `background: 'color-mix(in srgb, var(--surface-2), transparent 8%)'` and
`<ExternalLink className="h-4 w-4" style={{ color: 'var(--accent)' }} />`;
`<h3 className="text-xs font-semibold">UVTools Integration</h3>`;
`<p className="text-xs mt-0.5">` — both on `--text-strong` / `--text-muted`.

Enable row: `SettingRow bordered className="mt-3"`, label
`Enable UVTools Integration`, description `Adds a “Send to UVTools” option in the slicing panel.`
(typographic quotes in the source), control
`SegmentedControl label={_(msg\`Enable UVTools Integration\`)}` with
`[{value:'off',label:'OFF'},{value:'on',label:'ON'}]`,
`value={uvToolsSettings.enabled ? 'on' : 'off'}`.

Path card (only when enabled): `mt-2 rounded-md border p-2.5` on
`borderColor: 'var(--border-subtle)'`, `background: 'var(--surface-0)'`:

- `SettingRow` label `UVTools Executable Path`, description
  `formatExecutablePathHint(_, executableLabel)` =
  `Use auto-discover or enter the path to ${executableLabel} manually.`
- Button `variant="secondary" size="auto"`,
  `className="!h-9 !px-3 !py-0 text-sm inline-flex items-center gap-1.5 whitespace-nowrap disabled:opacity-50 transition-all duration-700"`,
  `disabled={discoveryBusy}`, style by state:
  `showFoundGlow` → `{ background: 'color-mix(in srgb, var(--accent-secondary), var(--surface-1) 88%)' }`,
  `showNotFound` → `{ background: 'color-mix(in srgb, var(--danger), var(--surface-1) 90%)' }`,
  else `{}`. Content: `discoveryBusy` → `<Spinner size="md" className="shrink-0" />` +
  `Scanning…`; `showFoundGlow` → `<CheckCircle2 className="h-4 w-4 shrink-0" style={{ color: 'var(--accent-secondary)' }} />` +
  `Found!`; otherwise `<Search className="h-4 w-4 shrink-0" />` + `Auto-Discover`.
- Path input: `w-full rounded-md border px-2.5 py-1.5 text-xs font-mono`,
  `borderColor: 'var(--border-subtle)'`, `background: 'var(--surface-1)'`,
  `color: 'var(--text-strong)'`, `spellCheck={false}`,
  placeholder `formatExecutablePathPlaceholder(_, executableLabel)` =
  `Select or type the path to ${executableLabel}`.

### States

- `enabled === false` → only the header and the OFF/ON row.
- `enabled === true` → path card.
- `discoveryBusy` → scanning spinner + disabled button.
- `showFoundGlow` (5 s, `FOUND_GLOW_DURATION_MS = 5000`) → green wash + `Found!`.
- `showNotFound` (same 5 s) → danger wash; label stays `Auto-Discover`
  (the failure signal is colour only).
- Timers cleared on unmount.

`executableLabel` is `'UVtools.app'` on macOS, `'UVTools.exe'` otherwise —
resolved in an effect (`detectPlatform() === 'mac'`) so SSR and first client
render agree; comment at `:47-49`: *"Resolved in an effect so the server-rendered
markup and the first client render agree; macOS points at the `.app` bundle, not
at an executable."*

### Data

`UvToolsSettings = { enabled: boolean; customPath: string }`
(`uvToolsPreferences.ts:3-6`), storage key `'app-uvtools-settings'`, event
`'app-uvtools-settings-changed'`, defaults `{ enabled: false, customPath: '' }`,
cached snapshot read-through in `getSavedUvToolsSettings`, and
`subscribeToUvToolsSettings` (storage + custom event). `normalizeUvToolsSettings`
coerces `enabled === true` and a string `customPath`.

Auto-discovery candidates (`uvToolsPreferences.ts:96-110`):
Windows `C:\Program Files\UVtools\UVTools.exe`,
`C:\Program Files (x86)\UVtools\UVTools.exe`; macOS
`/Applications/UVtools.app`, `~/Applications/UVtools.app`; Linux `[]`
(source comment: *"Linux has no candidates yet — see issue #604."*).
`autoDiscoverUvToolsPath()` invokes `discover_uvtools_path { candidates }`
and returns `null` on any error (browser mode).

### Callbacks

| Element | Handler | Result |
|---|---|---|
| OFF/ON | `onUvToolsSettingsChange({ ...settings, enabled: next === 'on' })` | draft only (Apply persists) |
| `Auto-Discover` | `handleAutoDiscover` | resets glow/not-found + timers, `autoDiscoverUvToolsPath()`; on path → writes `customPath`, `showFoundGlow` 5 s; on `null`/throw → `showNotFound` 5 s |
| path input | `onChange` | writes `customPath` |

### Text

Header title `UVTools Integration`; description
`Automatically open sliced print files in UVTools for further inspection and repair. After slicing, the file is sent directly to UVTools for analysis.`
(msgid = that string, `<Trans>`); then
`Enable UVTools Integration`, `Adds a “Send to UVTools” option in the slicing panel.`,
`UVTools Executable Path`, `Use auto-discover or enter the path to {executableLabel} manually.`,
`Select or type the path to {executableLabel}`, `Scanning…`, `Found!`, `Auto-Discover`,
`OFF`, `ON`.

### Port target

`gpui/src/features/settings/uvtools.rs::uvtools_tab` (exists, 1:1 for the enabled
and disabled layouts, segmented control writes `draft.uvtools.enabled`,
`Auto-Discover` inert, `show_found_glow`/`show_not_found`/`discovery_busy` styled
via `mix(...)`), state `UvToolsState` (`state.rs:547-553`). Discovery is
available in `gpui/backend/src/system.rs::discover_uvtools_path`; wiring it and
the 5 s timers is the remaining work.

---

## Surface: `LoggingSettingsTab` — log file, level, live viewer

- **Source**: `src/components/settings/LoggingSettingsTab.tsx` (610 lines);
  rendered at `SettingsModal.tsx:1571-1576` with `logLevel={draftLogLevel}` /
  `onLogLevelChange={setDraftLogLevel}`.
- **Renders**: `div className="flex flex-col gap-3.5"` with three sections —
  **Log File**, **Minimum Log Level**, the **Live Log** viewer — and an info
  callout.

### Layout & styling

All three sections: `rounded-xl border p-4` on
`{ borderColor: 'var(--border-subtle)', background: 'var(--surface-1)' }`
(the viewer uses `overflow-hidden` and no `p-4`).
Section headers are identical: `flex items-start gap-2 mb-3` (the viewer's header
is a toolbar instead), icon chip `inline-flex h-8 w-8 items-center justify-center rounded-md border shrink-0`
with `background: 'color-mix(in srgb, var(--surface-2), transparent 8%)'`,
accent icon, `text-xs font-semibold` title on `--text-strong`,
`text-xs mt-0.5` description on `--text-muted`.

**Log File** path box: `rounded-lg border flex items-stretch overflow-hidden`,
`borderColor: 'var(--border-subtle)'`, `background: 'var(--surface-2)'`;
left `flex-1 px-3 py-2.5 min-w-0`; right `flex border-l` with 12 px buttons,
each `inline-flex items-center justify-center ... transition-colors duration-150`.

Path cell states:
- `logPathError` → `text-xs` `#f87171`
- `logPath === null` → spinner `size="xs"` + `Resolving path…` on `--text-muted`
- resolved → `block text-xs font-mono truncate select-all` on `--text-strong`,
  `title={logPath}`, text = `resolveLogDirPath()` (`appLogDir()` + `dir separator`
  + `dragonfruit.log`, with exactly one separator enforced).

Buttons (each `title`): `Copy path` (`ClipboardCopy`; `Check` and
`color: var(--accent)` while copied), `Open in text editor` (`ExternalLink`,
spinner while opening), `Reveal in file manager` (`FolderOpen`, spinner while
revealing), `Delete log file` (`Trash2`, `color: '#f87171'` normally,
`var(--text-muted)` while deleting). The last three render only when
`isTauri`. Error lines: `mt-2 text-xs` `#f87171` from `revealError`, `openError`,
`deleteError`.

**Minimum Log Level**: `flex flex-col gap-1.5` of five buttons
(`flex items-center gap-3 rounded-lg border px-3 py-2 text-left transition-all duration-150`):
active →
```
borderColor: `color-mix(in srgb, ${color}, var(--border-subtle) 40%)`
background:  `color-mix(in srgb, ${color}, var(--surface-0) 90%)`
boxShadow:   `0 0 0 1px color-mix(in srgb, ${color}, transparent 78%) inset`
```
inactive → `{ borderColor: 'var(--border-subtle)', background: 'var(--surface-2)' }`.
Content: a 8 px dot `inline-flex h-2 w-2 rounded-full flex-shrink-0` with
`background: color`, `boxShadow: active ? \`0 0 6px ${color}\` : 'none'`;
then `text-[12px] font-semibold` label on `--text-strong` over `text-xs`
description on `--text-muted`; then `<Check className="h-3.5 w-3.5" style={{ color }} />`
when active.

`LEVEL_META` (`:131-138`):

| value | label | colour | description |
|---|---|---|---|
| `error` | `Error` | `#f87171` | `Only critical failures` |
| `warn` | `Warn` | `#fb923c` | `Warnings and errors` |
| `info` | `Info` | `#4ade80` | `Normal operational messages (recommended)` |
| `debug` | `Debug` | `#60a5fa` | `Verbose internal state` |
| `trace` | `Trace` | `#a78bfa` | `Maximum verbosity — may impact performance` |
| `unknown` | `?` | `#6b7280` | (empty) |

**Live Log viewer**: toolbar `flex items-center gap-2 px-3 py-2 border-b`,
`borderColor: 'var(--border-subtle)'`, `background: 'var(--surface-2)'`:

- live dot `inline-block h-1.5 w-1.5 rounded-full` — `#4ade80` with
  `boxShadow: '0 0 5px #4ade80'` when live, else `#6b7280`, no glow;
  `text-xs font-semibold` `Live Log`; `text-[11px]` counter —
  `{visibleLines.length} line{s}`, plus ` ({n} filtered)` when the filter hides
  any.
- noise toggle: `inline-flex items-center gap-1.5 rounded-md border px-2 py-1 text-[11px]`;
  label `noise hidden` / `noise shown`; colour `var(--text-muted)` when hiding,
  `#fb923c` + `background: 'color-mix(in srgb, #fb923c, var(--surface-0) 88%)'`
  when showing; titles `Show low-level transport noise (tungstenite, reqwest…)` /
  `Hide transport noise`.
- filter dropdown trigger: `inline-flex items-center gap-1.5 rounded-md border px-2 py-1 text-xs`,
  `borderColor: color-mix(in srgb, ${filterColor}, var(--border-subtle) 50%)`,
  `background: color-mix(in srgb, ${filterColor}, var(--surface-0) 88%)`,
  a 6 px dot, the label, `<ChevronDown className="h-3 w-3 opacity-60" />`.
  Menu: `absolute right-0 top-full mt-1 z-20 rounded-lg border shadow-lg overflow-hidden min-w-[100px]`,
  `background: 'var(--surface-2)'`; rows
  `flex items-center gap-2 w-full px-3 py-1.5 text-xs text-left transition-colors duration-100`,
  active background `color-mix(in srgb, ${color}, var(--surface-0) 85%)`,
  active row shows `<Check className="ml-auto h-3 w-3" style={{ color }} />`.
- live toggle (Pause/Play, `h-3 w-3`, `title` `Pause polling` / `Resume polling`),
- manual refresh (`RefreshCcw`), rendered only when `!isLive`, `title="Refresh now"`,
- clear viewer (`Trash2`, `title="Clear viewer"`).
  All icon buttons: `inline-flex items-center justify-center rounded-md border px-2 py-1`,
  `borderColor: 'var(--border-subtle)'`, `color: 'var(--text-muted)'`.

Output pane: `overflow-y-auto font-mono text-xs leading-snug`,
`style={{ height: 320, background: 'var(--surface-0, #0e0e0e)', padding: '8px 12px' }}`
(note the CSS fallback `#0e0e0e`).

`LogLineRow` (`:583-610`): `flex gap-2 min-w-0 hover:bg-white/[0.03] rounded px-0.5 -mx-0.5`;
level column `flex-shrink-0 text-[11px] font-bold tracking-wide pt-px`,
`minWidth: 38`, `userSelect: 'none'`, text = `unknown → '?'`, else
`level.toUpperCase().padEnd(5)` (so `INFO ` keeps a 5-char column);
timestamp `text-[11px] pt-px` `#4b5563` (only if parsed);
target `text-[11px] pt-px truncate max-w-[140px]` `#6b7280` (only if parsed);
message `flex-1 min-w-0 break-all` coloured `var(--danger)` for error,
`color-mix(in srgb, #f59e0b, var(--text-strong) 20%)` for warn, else
`var(--text-strong)`; falls back to `line.raw` when the message is empty.

Viewer empty/error states: `loadError` → `#f87171`; `!isTauri` →
`Log viewer only available in the desktop build.` (`#6b7280`);
no visible lines → `No log entries yet.` when `lines.length === 0`, else
`All lines filtered out.` (`#4b5563`).

Info callout: `rounded-lg border px-3 py-2.5 flex items-start gap-2.5`,
`borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 60%)'`,
`background: 'color-mix(in srgb, var(--accent), var(--surface-0) 94%)'`,
`<Info className="h-3.5 w-3.5 mt-0.5 flex-shrink-0" style={{ color: 'var(--accent)' }} />`,
`text-xs leading-snug` on `--text-muted`: `The minimum log level applies ` **`immediately`** ` and is also saved for next launch. The viewer filter and noise suppression are local-only — they do not affect what is written to disk.`

### States

- level list: one active at a time; clicking a row writes it immediately
  (`onLogLevelChange`), which the shell persists only on Apply.
- viewer: live (2 s poll) / paused; filter `trace` default; noise hidden default;
  copied flag (2 s); loading / error / empty.
- when the file is deleted the viewer clears (`setLines([])`) but the path stays.

### Data

- `LOG_LEVEL_STORAGE_KEY = 'dragonfruit-logging:min-level'`,
  `DEFAULT_LOG_LEVEL = 'info'`.
- `getSavedLogLevel()` validates against
  `['error','warn','info','debug','trace']`.
- `saveLogLevel(level)` writes localStorage **and**, in Tauri, invokes
  `set_log_level_pref { level }` (comment: *"Persist to disk so Rust reads the
  correct level on next startup."*).
- Polling: `invoke('read_log_tail', { lines: MAX_LINES })`, `MAX_LINES = 2000`,
  `POLL_INTERVAL_MS = 2000`, split on `\n`, drop empties, parse each line.
- Parsers: `BRACKET_RE = /^\[(\d{4}-\d{2}-\d{2})\]\[(\d{2}:\d{2}:\d{2})\]\[(\w+)\]\[([^\]]*)\]\s*(.*)/`
  (tauri-plugin-log v2 fern format; timestamp becomes `"YYYY-MM-DD HH:MM:SS"`),
  `ISO_RE = /^(\d{4}-\d{2}-\d{2}T[\d:.]+Z?)\s+(\w+)\s+\[([^\]]*)\]\s*(.*)/`,
  else `LEVEL_PATTERN = /\b(ERROR|WARN(?:ING)?|INFO|DEBUG|TRACE)\b/i` scans the
  whole line and the message stays the raw line.
- `LEVEL_RANK`: error 0, warn 1, info 2, debug 3, trace 4, unknown 5; a line is
  visible when `LEVEL_RANK[level] <= LEVEL_RANK[viewerFilter]` — so `unknown`
  lines are always visible.
- `NOISE_TARGETS` (exact match on `target`): `tungstenite::client`,
  `tungstenite::handshake::client`, `tungstenite::handshake::server`,
  `reqwest::connect`, `reqwest::async_impl::client`, `hyper::client`,
  `hyper_util::client`, `rustls`, `h2`.
- Auto-scroll: after each `lines` change, if `autoScrollRef.current` the pane
  jumps to `scrollHeight`; scrolling within 40 px of the bottom re-enables it.

### Callbacks

| Element | Handler | Result |
|---|---|---|
| level row | `onLogLevelChange(value)` | draft level |
| Copy path | `handleCopy` | clipboard, `copied` 2 s |
| Open in text editor | `invoke('open_log_file')` | OS viewer |
| Reveal in file manager | `invoke('reveal_in_file_manager', { path: logPath })` | file manager |
| Delete log file | `invoke('delete_log_file')` | file removed, view cleared |
| noise toggle | `setHideNoise(v => !v)` | local only |
| filter row | `setViewerFilter(v)` + close menu | local only |
| Pause/Play | `setIsLive` | starts/stops the 2 s interval |
| Refresh now | `fetchLines` | one poll |
| Clear viewer | `setLines([])` | view cleared only |

### Port target

`gpui/src/features/settings/logging.rs::logging_tab` (exists; log path renders the
source's non-desktop placeholder, copy is inert, viewer shows `0 lines`, level
rows and the filter menu are wired through `draft.logging.*`), state
`LoggingState` (`state.rs:581-587`) plus `LOG_LEVEL_OPTIONS` in `defaults.rs`.
Backend exists in `gpui/backend/src/log_prefs.rs` (`set_log_level_pref`,
`read_log_tail`, `open_log_file`, `delete_log_file`). Remaining: the real path,
the poll + parse pipeline, noise filtering and per-line rendering.

---

## Surface: `UpdatesSettingsTab` — updates card + release-channel dialog

- **Source**: `src/features/updater/UpdatesSettingsTab.tsx` (full file);
  rendered at `SettingsModal.tsx:1577-1582` with `channel={updateChannel}` /
  `onChannelChange={setUpdateChannel}`.
- **Renders**: `div className="flex h-full min-h-0 flex-col space-y-3"` → one
  `section` card holding the checker, plus the `Release Channel` dialog.

### Layout & styling

Card: `relative flex min-h-0 flex-1 flex-col rounded-lg border p-3`,
`{ borderColor: 'var(--border-subtle)', background: 'var(--surface-1)' }`.
Gear: `IconButton variant="surface" size="sm" tone="accent"`,
`className="absolute right-3 top-3 hover:brightness-110"`,
`aria-label="Release channel settings"`, `title="Release channel settings"`,
`<Settings className="h-4 w-4" />`; **rendered only when `!isLinux`** (Linux
updates through Flatpak, so there is nothing to choose).
Inner: `flex min-h-0 flex-1 flex-col` → `<UpdateCheckerSection className="flex min-h-0 flex-1 flex-col" />`.

Channel dialog: `StructuredDialogModal`, `open={showChannelSettings}`,
aria-label `Release channel settings`, title `Release Channel`, subtitle
`Choose which update feed to check for new versions.`, icon
`<Settings className="h-5 w-5" style={{ color: 'var(--accent)' }} />`,
`iconTone="neutral"`, backdrop click closes. Actions: `Cancel` (secondary, md)
and one conditional button:

```
style = pendingChannel === channel
  ? { borderColor: 'var(--border-subtle)', background: 'var(--surface-1)',
      color: 'var(--text-muted)', opacity: 0.6 }
  : isPendingDevSwitch
    ? { borderColor: 'color-mix(in srgb, #f59e0b, var(--border-subtle) 45%)',
        background: 'color-mix(in srgb, #f59e0b, var(--surface-1) 86%)', color: '#f59e0b' }
    : { borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 45%)',
        background: 'color-mix(in srgb, var(--accent), var(--surface-1) 86%)', color: 'var(--accent)' }
```
`disabled={pendingChannel === channel}`; label `Switch to Previews` with
`<FlaskConical className="w-3.5 h-3.5" />` when `isPendingDevSwitch`, else `Apply`.

Body: `div className="grid gap-2.5" role="tablist" aria-label="Release channel"`
with two `Button variant="secondary" size="auto" role="tab"
aria-selected={…} className="flex flex-col items-center justify-center gap-1.5 px-2 py-6 text-center min-h-[64px]"`:

- `Stable Builds` (`<ShieldCheck className="h-3.5 w-3.5 shrink-0" />`),
  subtitle `Recommended · Production`
- `Development Previews` (`<FlaskConical … />`), subtitle
  `Early access · May be unstable`

Active card style (`activeChannelStyle`, `:11-15`):
```
borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 30%)'
background:  'color-mix(in srgb, var(--accent), var(--surface-1) 85%)'
color:       'var(--text-strong)'
```
Title span `inline-flex items-center gap-1.5 text-sm font-semibold leading-none`;
subtitle span `text-xs font-normal leading-tight` on `--text-muted`.

Warning callout, only when `isPendingDevSwitch`:
`mt-3 rounded-md border px-3 py-2.5 flex gap-2.5`,
`borderColor: 'color-mix(in srgb, #f59e0b, var(--border-subtle) 40%)'`,
`background: 'color-mix(in srgb, #f59e0b, var(--surface-1) 90%)'`,
`<AlertTriangle className="h-4 w-4 shrink-0 mt-0.5" style={{ color: '#f59e0b' }} />`,
body `text-xs leading-relaxed` on `--text-muted`:
**`Development Previews`** ` are built from the ` **`dev`** ` branch for early testing. Expect bugs and breaking changes — switch back to Stable Builds at any time.`

### States

| condition | rendering |
|---|---|
| `isLinux` | no gear button (Flatpak) |
| dialog closed | only the card |
| `pendingChannel === channel` | Apply disabled, dimmed 60 % |
| `pendingChannel === 'dev'`, `channel !== 'dev'` | Apply = amber `Switch to Previews` + warning callout |
| other pending change | Apply = accent `Apply` |

Dialog open effect: `setPendingChannel(channel)` each time it opens.
Apply: if unchanged just closes; otherwise `commitChannel(pending)` →
`onChannelChange(newChannel)` **and** `void setUpdateChannel(newChannel)` (bridge
write) → close.

### Text

`Release channel settings` (aria/title), `Release Channel`,
`Choose which update feed to check for new versions.`, `Cancel`, `Apply`,
`Switch to Previews`, `Stable Builds`, `Recommended · Production`,
`Development Previews`, `Early access · May be unstable`,
the amber warning paragraph above.

### Port target

`gpui/src/features/settings/updates.rs::updates_tab` (`:5`) and
`updates_overlay` (`:121-326`) exist: card + gear + the whole channel dialog
(cards, active style, amber callout, disabled/dimmed Apply, warning variant).
State `UpdatesState` (`state.rs:636-640`) with `channel`, `pending_channel`,
`show_channel_settings`; enums `UpdateChannel`, `StoreUpdateChannel`.
The Rust body currently paints the *up-to-date* state as a static stand-in
because no check backend is wired (see open questions).

---

## Surface: `UpdateCheckerSection` — the nine update states

- **Source**: `src/features/updater/UpdateCheckerSection.tsx` (584 lines).
- **Renders**: a bare `<div className={className}>` that picks exactly one state
  body from `effectiveState`.

### State → body

| status | component | shape |
|---|---|---|
| `external` (Linux) | `ExternalState` | `w-full rounded-md border p-2.5` on `var(--surface-0)`; chip `Package` accent; title `Managed by Flatpak`; line `DragonFruit for Linux ships as a Flatpak, so updates are installed outside the app.`; right button `Releases` (`ExternalLink h-3.5 w-3.5`, `rounded-md border px-2.5 py-1.5 text-xs`, `--surface-2`); below it `<code>` `flatpak update org.openresinalliance.dragonfruit` (`mt-2 block rounded-md border px-2 py-1.5 text-[11px]`, `--surface-1`, `--text-muted`) |
| `idle` | `IdleState` | `grid grid-cols-2 gap-2`; `Check for Updates` (secondary md, `RotateCcw h-3.5 w-3.5`) and `Install` (secondary md, `disabled`, `opacity-50 cursor-not-allowed`, `title={_(msg\`No update available\`)}`, `Download h-3.5 w-3.5`) |
| `checking` | `CheckingState` | centered `flex flex-1 flex-col items-center justify-center gap-4 py-16`; 48 px chip (`h-12 w-12 rounded-xl border`, border `color-mix(in srgb, var(--accent), var(--border-subtle) 40%)`, bg `color-mix(in srgb, var(--accent), var(--surface-1) 82%)`) with `<Spinner size="xl" />`; `text-lg font-bold` `Checking for Updates…`; `text-sm mt-1.5 font-medium` `Querying GitHub releases` |
| `up-to-date` | `UpToDateState` | same layout, `gap-8`; chip tinted with `var(--success)` and `<Check className="h-6 w-6" />`; `Up to date` in `var(--success)`; `You’re on the latest version`; button `Check Again` (`variant="secondary" size="auto" className="!h-10 px-8 text-sm …"`, `RotateCcw h-4 w-4`) |
| `available` | `AvailableState` | see below |
| `downloading` | `DownloadingState` | spinner chip; `Downloading update`; progress line `formatUpdateDownloadedBytes(...)`; `w-full max-w-sm` `ProgressBar` |
| `installing` | `InstallingState` | spinner chip; `Installing Update…`; `The update is being installed. DragonFruit will restart automatically.` |
| `installed` | `InstalledState` | `w-full rounded-md border p-2.5` with accent-secondary border/background; chip `CloudOff`; `Update installed. DragonFruit will restart.` |
| `error` | `ErrorState` | `w-full rounded-md border p-2.5`, `#b91c1c` mixes; chip `CloudOff` on `#ef4444`; `Update failed`; the real message; right `Retry` button (`RotateCcw h-3 w-3`) calling `checkForUpdates` |

`AvailableState` (`:160-260`): column `flex min-h-0 flex-1 flex-col space-y-2`;
32 px chip `h-8 w-8 rounded-md border` with accent-secondary mixes and
`<Download className="h-4 w-4" />`; `text-base font-bold` `Update Available in
var(--accent-secondary)`; subtitle `text-xs mt-1 font-medium` —
`info.version === info.currentVersion ? formatUpdateVersionChip(...) : formatUpdateVersionReady(...)`
(i.e. `Version {v}` vs `Version {v} • update ready`).

Release notes block when `info.body`: `flex-1 min-h-0 overflow-y-auto custom-scrollbar rounded-md border px-3 py-3 text-xs leading-relaxed`,
`--border-subtle` / `--surface-0` / `--text-muted`, `<ReactMarkdown>` inside a
`prose prose-invert max-w-none` wrapper whose utility overrides map headings,
paragraphs, lists, links (`prose-a:text-[var(--accent)]`), bold, inline code
(bordered chip on `--surface-1`) and `<pre>`.

Button row `grid grid-cols-3 gap-2`:
`Check for Updates` (secondary md), `View on GitHub` (secondary md with
accent-secondary overrides, opens
`https://github.com/Open-Resin-Alliance/DragonFruit/releases/tag/v${info.version}`),
`Install` (`variant="tinted-accent"`, `Download h-3.5 w-3.5`) → opens the
`Update & Restart?` dialog.

`ProgressBar` (`:66-80`): track `w-full h-1.5 rounded-full overflow-hidden` with
`background: 'color-mix(in srgb, var(--surface-2), transparent 20%)'`; fill
`h-full rounded-full transition-all duration-200 ease-out`,
`width: ${Math.min(pct, 100)}%`,
`background: 'linear-gradient(90deg, var(--accent), var(--accent-secondary))'`.

`Update & Restart?` dialog: aria-label `Confirm update and restart`, title
`Update & Restart?`, subtitle `formatUpdateReadyToInstall(info.version, _)` =
`Version {v} is ready to install`, icon `Download`, `iconTone="warning"`;
buttons `Cancel` and `Update & Restart`. Body paragraphs:
`DragonFruit will ` **`download, verify and install`** ` the update, then restart automatically.`;
`Please ` **`save your scene and any unsaved progress`** ` before continuing.`;
a bordered warning line `pt-3 mt-1 border-t text-sm font-semibold`,
colour `color-mix(in srgb, var(--danger), white 32%)`: **`Unsaved changes may be lost.`**
The confirm handler: `await window.__df_flushAutosave?.()`, then a 400 ms wait,
then `onDownload()`.

### Behaviour

- Mount effect (`:474-476`): `checkForUpdates()` **on every mount**, i.e. every
  time the Updates tab is opened.
- `useUpdateChecker` (`useUpdateChecker.ts`):
  - state machine above; `state` is forced to `{ status: 'external' }` on Linux.
  - `handleCheck`: `fetchUpdateInfo(channel, allowSame)`; `null` →
    `up-to-date` with an empty `info` (comment: *"Couldn't reach the update
    server or no release exists yet. Show \"up to date\" with a muted note
    instead of silently going back to idle, so the user knows something
    happened."*); otherwise enrich `body`/`date` from
    `https://api.github.com/repos/Open-Resin-Alliance/DragonFruit/releases/tags/v{version}`
    when the feed carries none, then `available` + write
    `dragonfruit-updater-last-check = Date.now()`.
  - auto-check (`:137-158`): once the channel is loaded and
    `Date.now() - lastCheck >= 24h` (or never), call `handleCheck`. Not run on
    Linux.
  - `handleDownloadAndInstall`: `downloading` → `downloadAndInstall(onProgress)`
    → `installed`; any throw → `error` with `err.message` (comment: *"Show what
    actually broke. A generic \"check your connection\" hides failures that have
    nothing to do with the network."*).
  - `dismiss()` → `idle`.
- Dev shortcut (`:447-472`): a `window` listener on `app-hotkey-keydown`; if
  detail is `{ key: 'u', ctrlKey: true, shiftKey: true }` it calls
  `enableAllowSameVersionForSession()` and force-fetches with
  `allowSameVersion=true`, storing the result in `debugForceAvailable`, which
  **overrides** the hook's state. `debugForceSession.ts` is a module-level
  boolean cleared only by reload, with `enableAllowSameVersionForSession`,
  `isAllowSameVersionEnabled`, `clearAllowSameVersion` and no-op back-compat
  shims.

### Data / bridge

`updateBridge.ts`: `UpdateInfo = { version, currentVersion, body?, date? }`,
`DownloadProgress = { contentLength, downloaded }`, `UpdateChannel = 'stable' | 'dev'`,
`LINUX_RELEASES_URL = 'https://github.com/Open-Resin-Alliance/DragonFruit/releases'`,
`FLATPAK_APP_ID = 'org.openresinalliance.dragonfruit'`,
`updatesAreExternal() = detectPlatform() === 'linux'`,
`getUpdateChannel()` → `invoke('get_saved_update_channel')` (default `'stable'`
on error or in browser), `setUpdateChannel(channel)` →
`invoke('save_update_channel', { channel })`,
`fetchUpdateInfo(channel?, allowSameVersion)` → `invoke('check_updates', { channel, allow_same_version, allowSameVersion })`
— note the payload sends **both** snake_case and camelCase keys
(`:as unknown as Record<string, unknown>`), because the command's argument
deserialization is name-sensitive; returns `null` when
`!result.updateAvailable` or on any error.
`downloadAndInstall(onProgress)` builds a Tauri `Channel<PerformUpdateProgress>`
(`{ downloadedBytes, totalBytes, phase }`), invokes `perform_update { onChunk }`
and then `relaunch()` from `@tauri-apps/plugin-process`; the comment
(*"`perform_update` takes a `Channel`. Invoking without it fails argument
deserialization before the command body runs at all, so the channel is what
makes the call reach Rust — not just what reports progress."*) must survive the
port.

`updaterMessages.ts` formatters (module-level so React Compiler cannot rename the
placeholders, per its header comment): `formatUpdateProgressPct(pct)` →
`{pct}%`; `formatUpdateReleaseLine(date)` → `Release {date}`;
`formatUpdateVersionChip(v)` → `Version {v}`;
`formatUpdateDownloadedBytes(d, t, pct)` → `{downloaded} / {total} ({pct}%)`;
`formatUpdateVersionReady(v)` → `Version {v} • update ready`;
`formatUpdateReadyToInstall(v)` → `Version {v} is ready to install`.

### Port target

`gpui/src/features/settings/updates.rs` — only the up-to-date body is ported
(statically), the channel dialog is complete. `UpdateCheckerSection` as a state
machine is **not** ported: there is no checker, no `check_updates` /
`perform_update` implementation (`gpui/backend/src/updater.rs` has only
`get_saved_update_channel`, `save_update_channel`, `endpoint_for_channel`,
`STABLE_ENDPOINT` / `DEV_ENDPOINT`), and no relaunch. Porting the remaining
eight bodies is mechanical once those exist.

---

## Surface: About tab — hero, Team & Credits, sponsors, ORA footer

- **Source**: inline JSX at `src/components/settings/SettingsModal.tsx:1583-1773`;
  data from `contributors.json` (imported `:8`) and `SponsorsCarousel`
  (`:9`, `:1724`).
- **Renders** (root `flex h-full min-h-0 flex-col gap-3.5`, `:1584`):

  1. scroller `flex-1 min-h-0 overflow-y-auto custom-scrollbar pr-1` →
     `space-y-3.5 pb-2` containing **hero** and **Team & Credits**;
  2. `<SponsorsCarousel />` (its own card, pinned below the scroller);
  3. the ORA footer card (pinned).

  This three-part split is why the tab uses the internal-scroll layout.

### 1. Hero (`:1587-1645`)

`rounded-xl p-4`, background
`linear-gradient(145deg, color-mix(in srgb, var(--accent), var(--surface-0) 95%), color-mix(in srgb, var(--accent-secondary), var(--surface-0) 94%))`.

- Wordmark row `relative flex items-center justify-center`:
  `<img src="/dragonfruit_assets/branding/text_logo.svg" alt="DragonFruit" className="h-9 w-auto object-contain" style={isLightTheme ? { filter: 'drop-shadow(0 1px 3px rgba(0,0,0,0.35))' } : undefined} />`
  (the SVG viewBox is `0 0 1772 304`, so `h-9` ⇒ ~209.8 × 36).
- Corner badge `absolute right-0 top-0 inline-flex shrink-0 rounded-full px-2.5 py-0.5 text-[12px] font-semibold`,
  `color: '#ffffff'`,
  `background: 'linear-gradient(135deg, #3b0764 0%, #991b1b 50%, #9a3412 100%)'`,
  text `An Open Resin Alliance Project`.
- Pills row `mt-3 flex flex-col items-center gap-2` →
  `flex flex-wrap items-center justify-center gap-2.5`:
  - **version chip** wrapped in `Tooltip content={<span className="whitespace-pre-line">Click to copy build info{label ? `\n${label}` : ''}</span>}`;
    the chip is a `<span role="button" tabIndex={0} onClick={handleCopyBuildInfo} onKeyDown={Enter/Space → handleCopyBuildInfo}>`
    with `inline-flex cursor-pointer items-center rounded-full border px-2.5 py-0.5 text-[12px] font-semibold tabular-nums transition-colors`;
    normal: colour `var(--text-strong)`, border
    `color-mix(in srgb, var(--border-subtle), white 8%)`, background
    `color-mix(in srgb, var(--surface-1), transparent 8%)`; copied: colour and
    border `#2d8a4e`, background `rgba(45,138,78,0.1)`, text `✓ Copied!`
    (2 s) instead of `Version {DRAGONFRUIT_VERSION}`.
  - **build-status pill** `inline-flex rounded-full border px-2.5 py-0.5 text-xs font-semibold`
    with `buildStatusStyle`:
    `buildStatusLabel` = `'BETA VERSION'` when the channel contains `beta`,
    `'Mainline Build'` when it equals `mainline`, else `'{CHANNEL} Build'`
    (upper-cased). Dark-theme legs: beta →
    `{ color: '#fdba74', borderColor: 'color-mix(in srgb, #f97316, var(--border-subtle) 16%)', background: 'color-mix(in srgb, #f97316, transparent 96%)', textShadow: '0 0 4px color-mix(in srgb, #fb923c, transparent 66%)', boxShadow: '0 0 0 1px color-mix(in srgb, #f97316, transparent 62%), 0 0 10px color-mix(in srgb, #fb923c, transparent 74%)' }`;
    otherwise `{ color: 'var(--text-strong)', borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 40%)', background: 'color-mix(in srgb, var(--accent), transparent 84%)' }`.
    Light legs (used when `isLightTheme`): beta
    `{ color: '#9a3412', borderColor: 'color-mix(in srgb, #ea580c, var(--border-subtle) 30%)', background: 'color-mix(in srgb, #fed7aa, var(--surface-0) 14%)' }`.

Copy payload (`:427`):
`DragonFruit ${VERSION} (${CHANNEL})${GIT_BUILD_LABEL ? ` — ${GIT_BUILD_LABEL}` : ''}`.

### 2. Team & Credits (`:1647-1718`)

`rounded-md border p-3` on `{ borderColor: 'var(--border-subtle)', background: 'var(--surface-1)' }`.
Header `<h5 className="text-xs font-semibold uppercase tracking-wide">Team & Credits</h5>`
on `--text-muted`. Then `mt-2.5 space-y-2`.

Sorting (`:1654-1662`): tone rank `founder → 0`, `accent → 1`, everything else
`2`, then `a.name.localeCompare(b.name)` **within** a tone.

Grouping: iterate `['founder','accent','secondary']`; for each non-empty group
emit (for index > 0 first) a divider
`my-2.5 h-px rounded-full` with
`background: 'linear-gradient(90deg, transparent 0%, var(--border-subtle) 22%, var(--border-subtle) 78%, transparent 100%)'`,
then `grid grid-cols-2 gap-2` of person cards.

Person card: `rounded-md border px-3 py-2.5`, inline
`borderColor: color-mix(in srgb, ${toneVar}, var(--border-subtle) 45%)`,
`background: color-mix(in srgb, ${toneVar}, var(--surface-0) ${bgMix})` where
`toneVar` = `#d4a017` (founder) / `var(--accent)` (accent) / `var(--accent-secondary)` (secondary),
and `bgMix` = `94%` / `90%` / `93%`. Inside
`flex items-center justify-between gap-2 text-sm`:
name `font-semibold truncate` on `--text-strong`; then the affiliation button
`inline-flex shrink-0 items-center gap-0.5 font-normal hover:underline` on
`--text-muted` with `<Github className="h-3 w-3" />` and the affiliation text.

Affiliation click (`:1700-1708`): dynamic `import('@tauri-apps/api/core')` →
`invoke('open_external_url', { url: \`https://github.com/${person.affiliation}\` })`,
errors swallowed. **The JSON `role` field is never rendered** — "Founding
Member", "Head Maintainer", "Contributor", "Maintainer" do not appear in the UI.

### 3. ORA footer (`:1726-1771`)

`flex items-center gap-4 rounded-xl border px-4 py-3`,
`borderColor: 'color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 52%)'`,
`background: 'color-mix(in srgb, var(--accent-secondary), var(--surface-0) 94%)'`:

- `<img src={ORA_LOGO_DARK_URL} alt="Open Resin Alliance" className="h-14 w-auto object-contain shrink-0" style={isLightTheme ? { filter: 'drop-shadow(0 1px 4px rgba(0,0,0,0.3))' } : undefined} />`
  (square PNG ⇒ 56 × 56);
- middle `min-w-0 flex-1 space-y-2 text-center`:
  - repo row `flex items-center justify-center gap-2 text-[12px]`:
    `<Github className="h-3.5 w-3.5 shrink-0" style={{ color: 'var(--accent)' }} />`,
    a button `inline-flex items-center gap-1 underline underline-offset-2 font-mono tracking-tighter`
    on `var(--accent)` labelled `Open-Resin-Alliance/DragonFruit` with
    `<ExternalLink className="h-3 w-3" />`; click → `invoke('open_external_url', { url: DRAGONFRUIT_REPO_URL })`,
    fallback `window.open(url, '_blank')`.
  - licence row `flex items-center justify-center gap-2 text-[12px]` on `--text-strong`:
    `<ScrollText className="h-3.5 w-3.5 shrink-0" style={{ color: 'var(--accent)' }} />`
    + `<span className="font-mono tracking-tighter">AGPL-3.0-or-later</span>`;
- `<img src="/dragonfruit_assets/branding/simple_icon.svg" alt="" aria-hidden="true" className="h-10 w-auto object-contain shrink-0" />`
  (40 × 40).

### Port target

`gpui/src/features/settings/about.rs::about_tab` (`:373-752`) — complete: hero,
badge, both pills, the sort + tone groups + divider, the ORA footer, the
compile-time-anchored branding images (`branding_path`), `sponsors_card`, and
the internal scroller. Deviations already recorded in its module doc (two-stop
gradients, no clipboard, no URL opener, no `drop-shadow`, no `tracking`,
`copy` pill stuck in its un-copied state). Data constants in `defaults.rs`:
`DRAGONFRUIT_VERSION = env!("CARGO_PKG_VERSION")`, `DRAGONFRUIT_BUILD_CHANNEL = "mainline"`,
`DRAGONFRUIT_REPO_URL`, `CONTRIBUTORS`.

---

## Surface: `SponsorsCarousel` — sponsor marquee

- **Source**: `src/components/settings/SponsorsCarousel.tsx` (653 lines);
  mounted at `SettingsModal.tsx:1724` (inside About) and nowhere else.
- **Renders**: `shrink-0 rounded-lg border px-3 py-2` card on
  `{ borderColor: 'var(--border-subtle)', background: 'var(--surface-1)' }`
  with `aria-label={_(msg\`Sponsors\`)}`; header row; then one of loading /
  empty / marquee.

### Header (`:494-514`)

`flex items-center justify-between gap-2`; left
`flex items-center gap-1.5 text-[11px] font-semibold uppercase tracking-wide`
on `--text-muted`: `<Heart className="h-3 w-3" style={{ color: 'var(--accent-secondary)' }} aria-hidden="true" />`,
`Sponsors`, then a nested span `normal-case tracking-normal font-normal`
`{ color: 'var(--text-muted)', opacity: 0.85 }` with `via Open Collective`.
Right: button `inline-flex items-center gap-1 text-[10px] font-medium hover:underline`
on `var(--accent)`, label `Sponsor Us` + `<Heart className="h-3 w-3" aria-hidden="true" />`,
click → `openExternal(OPENCOLLECTIVE_URL)`.

### Body states

- **loading** (`sponsors === null`): `mt-2 flex items-center justify-center gap-2 rounded-md border px-3 py-6`,
  `borderColor: 'var(--border-subtle)'`,
  `background: 'color-mix(in srgb, var(--surface-1), transparent 20%)'`,
  `color: 'var(--text-muted)'`; `<Spinner size="lg" />` + `text-[11px]`
  `Loading sponsors…`.
- **empty** (`sponsors.length === 0`): dashed card
  `mt-2 flex items-center gap-2 rounded-md border border-dashed px-3 py-2`,
  `borderColor: 'color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 40%)'`,
  `background: 'color-mix(in srgb, var(--accent-secondary), var(--surface-0) 94%)'`;
  24 px heart chip `inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-full`
  with `background: 'color-mix(in srgb, var(--accent-secondary), var(--surface-1) 85%)'`,
  `color: 'var(--accent-secondary)'`; `text-[11px] font-medium leading-none`
  `No sponsors yet — be the first!`; `text-[10px] leading-tight`
  `Your name spins here when you back us.`; button `Sponsor`
  (`variant="secondary" size="auto"`, `!h-7 shrink-0 !px-2.5 text-[11px]`,
  style `{ background: 'color-mix(in srgb, var(--accent), var(--surface-0) 12%)', borderColor: 'color-mix(in srgb, var(--accent), white 10%)', color: 'white' }`,
  `<Heart className="h-3 w-3" />`).
- **populated**: viewport `relative mt-2 overflow-hidden rounded-md` on
  `background: 'color-mix(in srgb, var(--surface-0), transparent 20%)'`, with two
  absolute edge fades `pointer-events-none absolute inset-y-0 left-0|right-0 z-10 w-6`,
  `linear-gradient(90deg, var(--surface-1) 0%, transparent 100%)` and
  `linear-gradient(270deg, …)`. Track:
  `sponsors-marquee-track sponsors-marquee-track--animated flex items-center gap-3 py-2 will-change-transform`
  with `aria-live="polite"`.

### Marquee CSS (`:620-650`, inline `<style>`)

```css
.sponsors-marquee-track--animated {
  width: max-content;
  animation: sponsors-marquee 34s linear infinite;
}
.sponsors-marquee-track--animated:hover,
.sponsors-marquee-track--animated:focus-within {
  animation-play-state: paused;
}
@keyframes sponsors-marquee {
  /* 34s loop: 3s dwell at list start, 28s scroll, 3s dwell at rotation end.
     100% rests at -50% so the loop restart (second copy aligned) is invisible. */
  0% { transform: translateX(0); }
  8.8% { transform: translateX(0); }
  91.2% { transform: translateX(-50%); }
  100% { transform: translateX(-50%); }
}
@media (prefers-reduced-motion: reduce) {
  .sponsors-marquee-track--animated { animation: none; width: auto; }
}
```

Each sponsor is rendered **twice** (`[...sponsors, ...sponsors]`); the duplicate
copy gets `aria-hidden` and `tabIndex={-1}`; every `idx % sponsors.length === 0`
tile also gets `ml-6`. Tile = `<Tooltip content={label} maxWidth={220}>` around a
button `inline-flex shrink-0 items-center justify-center py-1 transition-opacity hover:opacity-80`,
`aria-label={formatOpenSponsorProfile(_, label)}` = `{name} — open sponsor profile`,
click → `openExternal(s.profile ?? s.website ?? OPENCOLLECTIVE_URL)`.

Tile content: `s.image` → `<CachedAvatar src profile alt="" className="h-12 w-12 rounded-full object-cover shrink-0" totalAmountDonated />`;
else the monogram span `inline-flex shrink-0 items-center justify-center rounded-full`
with `width: '3rem'`, `boxSizing: 'content-box'`, `height: '3rem'`, `padding: '3px'`,
`background: rank.gradient ?? undefined`, `boxShadow: rank.glow !== 'none' ? rank.glow : undefined`,
`borderRadius: '9999px'`; inner span `inline-flex h-full w-full items-center justify-center rounded-full border`
with `background: 'var(--surface-1)'`,
`color: rank.gradient ? rank.borderColor : 'var(--accent)'`,
`borderColor: 'transparent'`; initials at
`font-semibold leading-none tracking-wide` + `text-[15px]` for ≥ 2 letters or
`text-[19px]` for 1.

### Rank rings (`getSponsorRank`, `:27-45`)

Thresholds on lifetime `totalAmountDonated`; source comment kept:

> ```
> // Donation-rank ring colors — Siraya Tech ($200) → gold, next tiers silver/bronze.
> // Thresholds are totalAmountDonated from Open Collective (lifetime total, not monthly).
> // Adjust thresholds if sponsor amounts shift; gold >=100 (Siraya 200, WickedGrey 100), silver >=30, bronze >=10.
> ```

| total | tier | borderColor | gradient (conic, `from 0deg at 50% 50%`) | glow |
|---|---|---|---|---|
| `null`/undefined | `supporter` | `var(--border-subtle)` | none | `none` |
| ≥ 100 | `gold` | `#D4AF37` | `#BF953F 0, #FCF6BA 45, #B38728 90, #FBF5B7 135, #AA771C 180, #BF953F 225, #FCF6BA 270, #B38728 315, #BF953F 360` | `0 0 6px rgba(212,175,55,0.35), 0 0 12px rgba(212,175,55,0.18), inset 0 1px 1px rgba(255,255,255,0.5)` |
| ≥ 30 | `silver` | `#C0C0C0` | `#71706E 0, #E8E8E8 45, #A8A8A8 90, #F5F5F5 135, #9B9B9B 180, #71706E 225, #E8E8E8 270, #A8A8A8 315, #71706E 360` | `0 0 6px rgba(192,192,192,0.3), inset 0 1px 1px rgba(255,255,255,0.4)` |
| ≥ 10 | `bronze` | `#CD7F32` | `#6B3A1F 0, #CD7F32 45, #E9BB83 90, #8C4A2A 135, #5C3317 180, #6B3A1F 225, #CD7F32 270, #E9BB83 315, #6B3A1F 360` | `0 0 6px rgba(205,127,50,0.25), inset 0 1px 1px rgba(255,255,255,0.3)` |
| < 10 | `supporter` | `var(--border-subtle)` | none | `none` |

`sponsorInitials(name)` (`:58-67`): split camelCase with
`/([a-z0-9])([A-Z])/g → '$1 $2'`, split on `/[^\p{L}\p{N}]+/u`, take the first
grapheme of each word, keep letters only, then the first and last letter (or the
single one), upper-cased; `'?'` when nothing survives. Comment:
*"first letter of the first and last word, with camelCase split so
\"WickedGrey\" → WG and \"Kryz Slaughter\" → KS. A lone word keeps one letter
(\"ben\" → B), so the mark stays a monogram."*

### `CachedAvatar` (`:200-370`)

- `toHighResUrl(src)`: `images.opencollective.com` gets `height=512&width=512`
  when neither exists; `githubusercontent` gets `s=512`; `s3` gets
  `height=512&width=512` (harmless) and never the proxy; everything else returns
  `src`. Comments explain the S3-vs-proxy preference.
- Cache keys: `avatarCache` (module `Map`) and
  `localStorage['sponsor-avatar:' + highResSrc]`; hydrated in a
  `useLayoutEffect` before paint.
- Fetch chain: Tauri `invoke('fetch_sponsor_avatar', { url })` → must return a
  `data:` URL; else same-origin `/api/sponsor-avatar?url=…`; else direct
  `fetch(highResSrc, { cache: 'force-cache', mode: 'cors' })`; else
  `fetch(src, …)`; converts the blob with `FileReader.readAsDataURL` and caches
  it. Need for the Rust path (comment): *"Tauri production: Next.js proxy not
  available, S3 CORS blocks fetch(). Use Rust (reqwest) to bypass WebView CORS
  and return a data URL."*
- Pre-load ring: same 3rem + 3px content-box circle (comment: *"Always reserve
  the 3px ring gutter so ring-less avatars stay the same 54px height — otherwise
  their label sits 6px higher."*) with `<Spinner size="lg" />`, `aria-label={_(msg\`Loading avatar\`)}`.
- Loaded: outer ring span + inner `relative inline-flex h-full w-full overflow-hidden rounded-full`
  on `var(--surface-1)` with `transform: translateZ(0)`, `isolation: isolate`;
  a 512 × 512 `div` with `backgroundImage: url(...)`, `backgroundSize: 'cover'`,
  `transform: translate(-50%, -50%) scale(0.09375)` (512 × 0.09375 = 48 px inside
  the 48 px content box), `willChange: 'transform'`; a hidden `<img>` is used
  only for `onLoad`/`onError` → `setLoaded(true)`.

### Data flow

1. Initial state = `readCachedSponsors()` if present (`localStorage`
   `'dragonfruit:sponsors-cache-v1'`, `{ sponsors, fetchedAt }`), else `null`.
2. Fetch both collectives —
   `https://opencollective.com/openresinalliance/members/all.json` and
   `https://opencollective.com/dragonfruit-slicer/members/all.json` — via Tauri
   `invoke('fetch_external_text', { url })` first, else `fetch(..., { cache: 'no-store' })`.
3. `mapOCMemberToSponsor` keeps only `role === 'BACKER'` with a non-empty trimmed
   name, projecting `{ name, profile, image, website, role, tier, totalAmountDonated }`.
4. Merge successful lists → `dedupeSponsors` (key `(profile ?? name).toLowerCase()`)
   → sort by `totalAmountDonated` desc, then `name.localeCompare` → `setSponsors`
   + `writeCachedSponsors`.
5. If **both** sources fail and nothing is cached, fall back to
   `sponsors.json` (18 entries, amounts 900 … 1).
6. Refresh: `setInterval(fetchSponsors, 5 * 60 * 1000)`, on `window focus`, and
   on `document.visibilitychange → visible`; all cleared on unmount with an
   `AbortController`.

`SPONSORS_CACHE_TTL_MS = 1000 * 60 * 60 * 6` is declared and **never used** — the
cache is written with a `fetchedAt` but read without expiry (stale is preferred
to baked-in, comment at `:113`).

`sponsors.json` (fallback, order = file order): KCast 900, Siraya Tech 600,
Atlas 3DSS 300, ben 190, Gobbledygook Games 115, WickedGrey 100, jacobi 50,
Concepts 3D 45, Kryz Slaughter 45, Rob Skinner 45, Runklestiltskin 30, DCent 20,
foors 20, Cunabula 15, Viznoman 15, Tom 10, MachineMenace 5, Sin 1. `ben`,
`WickedGrey`, `jacobi`, `Runklestiltskin`, `DCent`, `foors`, `MachineMenace` have
`image: null` → monogram; the rest carry Open Collective S3 URLs.

### Port target

`gpui/src/features/settings/about.rs` — `sponsor_initials`, `sponsor_rank`,
`sponsor_tile`, `sponsors_card` (`SPONSORS` in `defaults.rs` = name/profile/amount
only). Deliberate: no fetch (fallback list only, so loading/empty states are
unreachable), no avatars (all monograms), no marquee (single-copy horizontal
scroller), ring rendered as a 135° two-stop linear of the conic's first stops,
tooltips omitted (no portal). Improving any of these needs an HTTP client and a
clip-aware popover.

---

## Deliberate decisions and gotchas

1. **`BackupsSettingsTab.tsx` is dead code.** Nothing imports it; the Backups
   sidebar entry mounts `LocalBackupsSettingsTab`. Do not port it, do not
   translate its strings (they are still in the `.po` files, stale). Verify
   before deleting anything: `grep -r "settings/BackupsSettingsTab"` returns only
   its own file.
2. **The shell's `ActiveTabIcon` is computed and never used** (`:1318`):
   `const ActiveTabIcon = tabMeta[activeTab].icon;` is dead. `activeTabColor`
   next to it is also unused inside the shell (the sidebar tab derives its own
   colour from `meta.tone`). Do not invent a header icon from it.
3. **`performance` renders under the label `Slicing`; `spacemouse` under
   `3D Mouse`; `workspaces` under `Workspace`** (singular). Tab key ≠ label.
4. **The sidebar is two groups, not one list**, and the second group is pinned
   with `mt-auto` inside the same scroller. On a short window the bottom eight
   (Plugins … About) still appear after the top nine, but any spare height goes
   *between* the groups.
5. **Tab entry animation depends on `key={activeTab}`**, not on a transition —
   re-rendering the same tab does not replay it.
6. **The modal stays mounted while closed**; the source comment at `:384-388`
   explains that `activeTab` would otherwise persist, hence the reset on open.
   Port consequence: opening settings must always land on `general` unless the
   caller deep-links.
7. **Apply must read every draft.** Source comment on the `useCallback` dep list
   (`:1129-1132`):
   > ```
   > // Every draft this callback reads belongs in this list. One omission and
   > // Apply pushes whatever the draft happened to be when the callback was last
   > // rebuilt, which presents as a control that saves once and then reverts.
   > ```
8. **Closing can be blocked by the experiments reload prompt** (`requestClose`).
   Cancel, X, backdrop and Escape all funnel through it; `onClose` alone does not
   check.
9. **`handleCancel` restores the *saved* theme preview**, not the draft reset
   only: `applyThemePreference(getSavedThemePreference())` +
   `applyThemeCustomColors(getSavedThemeCustomColors())`, then
   `resetDraftFromProps()`. Order matters, because `resetDraftFromProps` re-reads
   storage.
10. **`applyRestoreDefaultsToDraft` deliberately excludes**
    `draftLogLevel` and the update channel (Restore Defaults does not touch log
    level or channel), and it does *not* reset `draftThemeProfiles` (custom
    themes are kept). It sets `draftThemePreference = 'dark'` and
    `draftThemePreset = 'dragonfruit-dark'` literally.
11. **`color-mix(in srgb, A, B N%)` means `A*(1-N) + B*N`.** Several call sites
    read backwards if you assume A is the dominant colour — e.g.
    `color-mix(in srgb, var(--accent), var(--surface-0) 84%)` on the active
    sidebar tab is 16 % accent. This inversion has already produced four visible
    bugs (handoff decision 3).
12. **Backups: the auto-sync interval is not a scheduler.** `setInterval(…,
    minutes * 60_000)` lives in an effect keyed on `selectedDirectory`,
    `autoSyncEnabled`, `autoSyncMinutes`, so changing the interval restarts the
    countdown and the first backup only lands one full interval later.
13. **Backups: restore reloads the window** (`applySnapshotToLocalApp` ends with
    `window.location.reload()`), and only when the snapshot modal's own document
    is the one restored; otherwise it just says
    `Snapshot restored to current local backup state.`
14. **Backups: snapshot ids are epoch-millisecond strings**, used as labels via
    `new Date(Number(id)).toLocaleString()` in three places (delete body,
    restore subtitle, modal subtitle). `formatHistoryDate` prefers the entry's
    `updatedAt`, then the numeric id, then the raw id.
15. **Backups: `collectSnapshot` is a whitelist, not a dump.**
    `KNOWN_LOCAL_STORAGE_KEYS` or a `dragonfruit-` / `app-` / `lumenslicer:`
    prefix. Adding a new setting key outside those prefixes silently stops it
    being backed up.
16. **Backups: empty-history copy differs by `hasAnySync`**, which is
    `Boolean(stateUpdatedAt || lastLocalSyncAt)` — i.e. "we have backed up
    before" vs "never".
17. **The backups snapshot viewer is a second, hand-rolled modal**, not
    `StructuredDialogModal`: `z-[90]`, `max-w-6xl`, `h-[820px]`,
    `max-h-[94vh]`, `role="dialog" aria-modal="true"`, `ui-modal-backdrop-enter` /
    `ui-modal-panel-enter`, and its own Escape handler via
    `useEscapeToClose(showSnapshotModal, …)`.
18. **UVTools: the found/not-found signals are colour + a 5 s timer only.** The
    label stays `Auto-Discover` when nothing was found; only `Found!` replaces it.
    `transition-all duration-700` is what makes the wash fade back.
19. **UVTools is draft-only**: toggling ON/OFF and typing a path never persist
    until Apply (`saveUvToolsSettings(draftUvToolsSettings)`), unlike the ~15 s
    auto-persisted camera feel settings.
20. **Logging: level vs viewer filter are different things.** The file level
    defaults `'info'`; the viewer filter defaults `'trace'` (show everything) and
    is never persisted. Comment in the callout says exactly that.
21. **Logging: `unknown` ranks 5**, so unparsed lines pass every filter
    (`<= 5` always true).
22. **Logging: parser order matters** — bracket form, then ISO, then a keyword
    scan that keeps the whole line as the message.
23. **Logging: delete clears the view but keeps the path**, and the 2 s poll
    continues (so a recreated log file reappears).
24. **Logging: auto-scroll is user-intent based**, re-enabled only within 40 px
    of the bottom; do not force-scroll on every poll.
25. **Updates: the check runs on every mount of the tab** *in addition to* the
    24 h auto-check, so opening the tab always re-checks (and rewrites
    `dragonfruit-updater-last-check`).
26. **Updates: `fetchUpdateInfo` sends the same value twice**
    (`allow_same_version` and `allowSameVersion`) because Tauri arg
    deserialization is name-sensitive; `perform_update` must receive a `Channel`
    or the command fails before its body runs (both comments quoted above).
27. **Updates: a `null` result renders "up to date"**, not an error, by design.
28. **Updates: Linux has no gear and no checker** — `updatesAreExternal()` is
    `detectPlatform() === 'linux'`, and `useUpdateChecker` forces
    `{ status: 'external' }`.
29. **Updates: switching *to* dev is a different action** — amber button
    `Switch to Previews`, amber callout, and the Apply button is disabled
    (`opacity 0.6`) whenever the pending channel equals the current one.
30. **Ctrl+Shift+U is a live dev shortcut**, not a hidden setting: it flips a
    module-level `allowSameVersionUntilReload` flag, and its forced result
    (`debugForceAvailable`) overrides the hook state. Decide explicitly whether
    the port keeps it; if it does, it needs a reload-reset story.
31. **About: the three blocks scroll differently.** Hero + credits scroll; the
    sponsors card and the ORA footer are pinned siblings. That is why the tab is
    in `usesInternalTabScrollLayout` together with Hotkeys and Updates.
32. **About: `role` in `contributors.json` is never displayed** — only `name`
    and `affiliation`, grouped and coloured by `tone`.
33. **About: sorting is tone-rank then `localeCompare`**, not file order:
    `Aaron Baca` sorts before `AJRubenstein`, `tforver…` style lowercase names
    interleave by locale rules (`tforik55` before `Tim`).
34. **About: the version chip copies a build string, not just the version** —
    `DragonFruit {v} ({channel}) — {ref @ commit · OS/arch}`, and the copied
    state is a local 2 s flag, not persisted.
35. **About: `buildStatusLabel` casing is inconsistent on purpose**:
    `BETA VERSION` (upper), `Mainline Build` (title), `{CHANNEL} Build` (upper
    channel).
36. **Sponsors: the list is duplicated for the marquee**, and only the second
    copy is `aria-hidden` + `tabIndex={-1}`; `ml-6` on each copy's first tile is
    what makes the loop restart invisible.
37. **Sponsors: `8.8%`/`91.2%` are dwell, not easing** — the track is at
    `translateX(0)` for the first 3 s and at `-50%` for the last 3 s of the 34 s
    cycle.
38. **Sponsors: the 3 px ring gutter is always reserved** (`box-sizing:
    content-box` + `padding: 3px`), so ringed and ring-less tiles are both 54 px;
    removing the padding for ring-less tiles shifts every label 6 px.
39. **Sponsors: only `role === 'BACKER'` survives the Open Collective mapping**;
    admins/contributors on the collective page are dropped.
40. **Sponsors: `SPONSORS_CACHE_TTL_MS` is unused** and `readCachedSponsors`
    accepts an empty array (`cached.length >= 0` is always true), so a cached
    empty list is preferred over the baked-in fallback.
41. **Sponsors: 512 px source scaled by `0.09375`** — that factor is 48/512, tied
    to `3rem`; changing the tile size means changing the scale.
42. **`prefers-reduced-motion` kills the marquee** and switches the track to
    `width: auto`; a GPUI port needs its own respect for the OS setting.

---

## Open questions

1. **No update backend.** `gpui/backend/src/updater.rs` implements only channel
   persistence + feed endpoints (`STABLE_ENDPOINT =
   https://open-resin-alliance.github.io/DragonFruit/latest.json`,
   `DEV_ENDPOINT = …latest-dev.json`). `check_updates`, `perform_update`, the
   download progress `Channel`, signature verification and `relaunch` are
   unimplemented; `updates_tab` therefore paints a static "Up to date". Who owns
   the Tauri-updater parity work?
2. **Clipboard.** The About version chip, the Logging "Copy path" tile and the
   theme export all need a clipboard. GPUI has one (`App::write_to_clipboard`?),
   but nothing in this slice uses it, and `about.rs`/`logging.rs` currently ship
   inert controls. Decide whether to wire it or keep documenting the inert state.
3. **`open_external_url`** lives in the excluded `gpui-backend` crate, so every
   link in this area (contributors, repo, sponsors, Releases) is static text in
   the port. Same owner question as (1).
4. **Sponsors need HTTP.** Fetching the two Open Collective JSON feeds and
   `fetch_sponsor_avatar` exist in `gpui/backend/src/net.rs`
   (`fetch_external_text`, `fetch_sponsor_avatar`, host allow-lists), but the
   settings crate has no client. Port the fetch/merge/dedupe/sort pipeline, or
   keep the baked-in list permanently?
5. **`localStorage` → store.** This area reads/writes five backup keys, one
   logging key and three sponsor caches directly. The Rust store
   (`gpui-store`) already models `BackupsSettings`, `LoggingSettings`,
   `UpdateSettings`; the sponsor caches and `dragonfruit-updater-last-check`
   have no home. Do they move into the store documents or get deleted with the
   fetch pipeline?
6. **Date formatting.** Delete/restore labels and every "Last …" tile use
   `toLocaleString()`. `gpui-locale` has a verified number formatter; no date
   formatter is mentioned in the handoff. Needed for parity on 5 surfaces.
7. **The snapshot content modal is a large unported surface** (four tabs, JSON
   highlighter, profile drill-down). Does it belong to this wave or to a
   dedicated "backup browser" ticket? Without it, restore can only be triggered
   from a history row, which the port does not render.
8. **Should `BackupsSettingsTab.tsx` be deleted** from the repo (with its `.po`
   references regenerated), or left as documentation of the abandoned GitHub
   backup feature?
9. **The Ctrl+Shift+U debug shortcut** (allow same-version install) — port,
   drop, or move behind a `--debug` flag?
10. **Log viewer polling owner.** A 2 s `read_log_tail` timer inside a GPUI
    entity needs a `Timer`/`cx.spawn` task tied to the modal's lifetime; it
    must not run while the modal is closed. The Rust tab renders the placeholder
    today, so this is unbuilt and unmeasured.
