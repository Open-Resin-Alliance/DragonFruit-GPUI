# Settings — Hotkeys, Plugins, Experiments, Scene Autosave

## Components in scope

| Component | File |
|---|---|
| `HotkeysSettingsTab` | `src/components/settings/HotkeysSettingsTab.tsx` |
| `PluginsSettingsTab` | `src/components/settings/PluginsSettingsTab.tsx` |
| `ExperimentsSettingsTab` | `src/components/settings/ExperimentsSettingsTab.tsx` |
| `SceneAutosaveSettingsTab` | `src/components/settings/SceneAutosaveSettingsTab.tsx` |

Supporting modules read for this spec (not edited, but a porter must not re-derive
them and must keep their contracts):

| Module | Why it matters here |
|---|---|
| `src/components/settings/SettingsModal.tsx` | mounts all four tabs, owns the sidebar, the Experiments "Apply Experiments" reload prompt and the `onExit` hand-off |
| `src/hotkeys/hotkeyConfig.ts` | `HotkeyBinding`, `DEFAULT_KEYBINDINGS` (the defaults the Hotkeys tab renders and resets to) |
| `src/hotkeys/HotkeyContext.tsx` | `useHotkeyConfig()`; localStorage persistence and stale-action stripping |
| `src/hotkeys/hotkeyLabels.ts` | `getBindingTokens`, `toKeyLabel`, `toModifierLabel` |
| `src/hotkeys/HotkeyRegistryManager.tsx` | `suspendHotkeyDispatch` / `resumeHotkeyDispatch` |
| `src/hotkeys/useEscapeToClose.ts` | Escape stack used by the disclaimer and every dialog |
| `src/hooks/usePlatformModifier.ts` | `Cmd` on macOS, `Ctrl` elsewhere |
| `src/features/delete/useDeleteHotkey.ts` | `SECONDARY_DELETE_KEY = 'Delete'` |
| `src/config/experiments.json` | the experiment manifest |
| `src/features/experiments/experimentsRegistry.ts` | enable/disable, promotion semantics, subscriptions |
| `src/features/experiments/experimentMessages.ts` | translated experiment names/descriptions |
| `src/components/settings/sceneAutosavePreferences.ts` | autosave defaults, clamping, storage key, change event |
| `src/features/plugins/sideloadedPlugins.ts` | the `.dfplugin` host bridge |
| `src/features/plugins/pluginManifestClient.ts` | manifest fetch transport (native command vs Next route) |
| `src/features/plugins/githubPluginManifest.ts` | allowlist, liability gate, manifest sanitiser |
| `src/features/profiles/profileStore.ts` | `getInstalledPlugins`, `installPluginFromManifest`, `uninstallPlugin`, `markPrinterProfilesMissingPlugin` |
| `src/features/plugins/repositoryLabel.ts` | `owner/repo` label for repository links |
| `src/components/atoms/*`, `src/components/ui/StructuredDialogModal.tsx`, `src/components/ui/NumberInput.tsx`, `src/components/ui/Tooltip.tsx` | the controls every surface in this area is built from |
| `src/app/globals.css` | `.ui-input`, `.ui-segmented-option`, `.ui-button*`, `settingsTabIn` |

---

# Surface: SettingsModal — how these four tabs are mounted

- **Source**: `src/components/settings/SettingsModal.tsx:1252-1315` (tab metadata and
  sidebar order), `:1547-1563` (the four tab bodies), `:1430-1432` (the content
  column), `:391-430` (open/reset and `previousTabRef`), `:1854-1888` (the
  "Apply Experiments" reload prompt).
- **Renders**: a full-screen fixed backdrop holding a 72rem-wide card: header,
  `w-72` sidebar rail, content column, footer.
- **Layout & styling**:
  - backdrop: `fixed inset-0 z-50 flex items-stretch justify-center bg-black/60 backdrop-blur-sm p-5 ui-modal-backdrop-enter`, backdrop `onMouseDown` closes only when `e.target === e.currentTarget`.
  - card: `w-full max-w-[72rem] h-full flex flex-col rounded-2xl shadow-2xl border overflow-hidden ui-modal-panel-enter`, `background: var(--surface-0)`, `borderColor: var(--border-strong)`, `boxShadow: 0 26px 64px rgba(0, 0, 0, 0.46)`.
  - header: `flex items-center justify-between px-4 py-3`, `borderBottom: 1px solid var(--border-subtle)`.
  - sidebar rail: `w-72 min-h-0 p-2.5`, `borderRight: 1px solid var(--border-subtle)`, background
    `linear-gradient(180deg, color-mix(in srgb, var(--surface-1), transparent 6%), color-mix(in srgb, var(--accent-secondary), var(--surface-1) 96%))`.
  - content column: `usesInternalTabScrollLayout` = `isAboutTab || activeTab === 'hotkeys' || activeTab === 'updates'`.
    When true: `flex-1 min-h-0 min-w-0 flex flex-col p-4`, and the inner keyed div is
    `animate-[settingsTabIn_180ms_ease-out] flex-1 min-h-0 flex flex-col`.
    Otherwise: `flex-1 min-h-0 overflow-y-auto custom-scrollbar p-4`, inner div
    `animate-[settingsTabIn_180ms_ease-out]`. **Only Hotkeys uses internal scroll in
    this area**; Plugins, Experiments and Scene Autosave scroll the column itself.
  - `settingsTabIn`: `0%` opacity 0 + `translateY(6px)`, `100%` opacity 1 + `translateY(0)`.
- **Tab metadata relevant here** (label / description / icon / tone):
  - `hotkeys`: "Hotkeys" / "Keyboard bindings and presets" / `Keyboard` / `primary`
  - `plugins`: "Plugins" / "Load vendor profile plugins" / `Plug` / `secondary`
  - `experiments`: "Experiments" / "Early-access and experimental features" / `FlaskConical` / `secondary`
  - `sceneAutosave`: "Scene Autosave" / "Autosave and crash recovery behavior" / `HardDrive` / `secondary`
  - `sidebarTopTabs = ['general','camera','workspaces','multiPlate','mesh','performance','spacemouse','ui','hotkeys']`;
    `sidebarBottomTabs = ['plugins','experiments','sceneAutosave','backups','uvtools','logging','updates','about']`.
- **States**:
  - The modal stays mounted while closed; `activeTab` resets to `initialTab ?? 'general'` each time `isOpen` flips false→true, and `showReloadPrompt` resets then too.
  - `previousTabRef` tracks the tab active *before* the current selection so the Experiments disclaimer can return there.
- **Callbacks**:
  - `handleSelectTab(tab)` sets `previousTabRef.current = activeTab` then `setActiveTab(tab)`.
  - `requestClose()` compares `getEnabledExperimentIds().sort().join(',')` with the snapshot taken at open; if they differ it opens the reload prompt instead of closing.
  - `onExit` passed to Experiments: `setActiveTab(previousTabRef.current === 'experiments' ? 'general' : previousTabRef.current)`.
  - "Apply Experiments" dialog: ariaLabel `"Apply experiments"`, title `"Apply Experiments"`, subtitle `"Experiment changes take effect after a restart."`, icon `RotateCcw`, `iconTone="warning"`, `zIndexClassName="z-[80]"`, `maxWidthClassName="max-w-md"`, `closeAriaLabel="Close restart prompt"`; body text `"Reload DragonFruit now to apply your experiment changes? Any unsaved changes to the current scene will be lost."`; actions `Not Now` (secondary, closes prompt then modal) and `Reload Now` (tinted-accent, `window.location.reload()`).
- **Port target**: `gpui/src/features/settings/mod.rs` already owns `settings_modal`,
  `content`, the `SettingsTab` enum, the sidebar and the footer; `hotkeys` is
  already in the internal-scroll set (`SettingsTab::About | Hotkeys | Updates`).
  The reload prompt is **not** ported.
- **Note**: `src/components/settings/settingsModalEvents.ts` exposes
  `openSettingsModal(tab?)` dispatching `dragonfruit:open-settings-modal` with
  `{ tab }`; that is the deep-link path that makes the Experiments `onExit`
  fallback to General necessary.

---

# Surface: HotkeysSettingsTab — keyboard binding editor

- **Source**: `src/components/settings/HotkeysSettingsTab.tsx:1-429`; the row and
  token are `HotkeyRow` (`:337-408`) and `KbdToken` (`:410-429`).
- **Renders**: a two-column grid of four section cards (Global, Prepare, Supports,
  Support Presets), each card a header plus a stack of binding rows; a fixed
  recording overlay while a row is being captured.

## Card grid

- **Layout & styling**:
  - root: `h-full min-h-0 flex flex-col gap-2`
  - scroller: `flex-1 min-h-0 overflow-y-auto custom-scrollbar pr-1`
  - stack: `space-y-2.5`
  - each row of cards: `grid gap-2.5 lg:grid-cols-2`; an absent right-hand card is
    `<div />`, so a lone card still occupies half the row.
  - section card: `rounded-lg border p-2.5 h-full` with
    `borderColor: var(--border-subtle)`, `background: var(--surface-1)`.
  - header row: `flex items-start gap-2`.
  - icon tile: `inline-flex h-8 w-8 items-center justify-center rounded-md border shrink-0`,
    `borderColor: var(--border-subtle)`,
    `background: color-mix(in srgb, var(--surface-2), transparent 8%)`; the
    `Keyboard` icon is `h-4 w-4` with `color: var(--accent)`.
  - title: `text-sm font-semibold`, `color: var(--text-strong)`.
  - description: `text-[11px]`, `color: var(--text-muted)`.
  - reset button: `IconButton` with `className="!h-7 !w-7 !p-0 shrink-0 rounded-md"`,
    `title="Reset section to default shortcuts"`,
    `aria-label={\`Reset ${section.title} section to default shortcuts\`}`,
    child `RotateCcw` at `h-3.5 w-3.5`.
  - body: `mt-2 space-y-2`; per category group `space-y-1`.
  - category sub-label (only when the section has more than one category):
    `text-[11px] font-semibold uppercase tracking-wide`, `color: var(--text-muted)`.
- **Sections** (`SECTION_GROUPS`, order preserved):

  | id | title | description | categories |
  |---|---|---|---|
  | `global` | Global | General and camera shortcuts available across all workspaces. | `['GLOBAL','CAMERA']` |
  | `scene` | Prepare | Canvas tool switching and scene arrangement shortcuts. | `['CANVAS']` |
  | `supports` | Supports | Support authoring workflow shortcuts. | `['SUPPORTS']` |
  | `presets` | Support Presets | Quick-apply support preset shortcuts. | `['PRESETS']` |

  Category labels: `GLOBAL`→"General", `CAMERA`→"Camera", `CANVAS`→"Canvas Tools",
  `SUPPORTS`→"Supports", `PRESETS`→"Presets"; unknown falls back to the raw key.
- **States**: a section with no entries is dropped
  (`filter((section) => section.categories.some((c) => c.entries.length > 0))`), and a
  category absent from `config` is dropped from the section.
- **Data**: `useHotkeyConfig()` → `{ config, updateHotkey, resetCategories }`.
  `config` is `HotkeyConfig = Record<category, Record<action, HotkeyBinding>>` merged
  over `DEFAULT_KEYBINDINGS` and persisted at localStorage key `app-hotkeys-config`.
- **Callbacks**: the reset button calls
  `resetCategories(section.categories.map((c) => c.category))` — one call resetting
  every category the card displays.
- **Text**: the four titles and descriptions above; `Keyboard`, `RotateCcw` icons.

## Row labels

Rows are rendered from `Object.entries(categoryConfig)`:
- every category except `PRESETS`: `label = binding.description`.
- `PRESETS`: `label = PINNED_SLOT_LABELS[action] ?? action`, i.e. `Slot 1` … `Slot 6`.

`PINNED_SLOT_LABELS` maps `SLOT_1`…`SLOT_6` to `Slot 1`…`Slot 6`.

The `GLOBAL/DELETE` row also gets a fixed secondary keycap:
`secondaryToken = category === 'GLOBAL' && action === 'DELETE' ? toKeyLabel(SECONDARY_DELETE_KEY) : undefined`,
i.e. `Delete`.

Default bindings the tab renders (from `DEFAULT_KEYBINDINGS`, in declaration order):

| Category | Action | key | modifier | description |
|---|---|---|---|---|
| SUPPORTS | JOINT_CREATION | `j` | — | Hold to enter Joint Creation Mode |
| SUPPORTS | CURVE_MODE | `c` | — | Hold to create curved segment when moving joint |
| SUPPORTS | BRANCH_PLACEMENT | `Alt` | — | Hold to enter Branch Placement Mode |
| SUPPORTS | LEAF_PLACEMENT | `Alt` | `ctrl` | Hold to enter Leaf Placement Mode |
| SUPPORTS | KICKSTAND_PLACEMENT | `Control` | — | Hold to enter Kickstand Placement Mode |
| SUPPORTS | TEMP_SPOTLIGHT_HOLD | `p` | — | Hold to temporarily enable Spotlight highlight in Support mode |
| SUPPORTS | FORCE_PLACE_SUPPORT | `q` | — | Hold to force placing support |
| SUPPORTS | AUTO_BRACING | `g` | — | Generate auto bracing while the Bracing settings page is open |
| SUPPORTS | SPROUTED_PARENTING_LOCK | `w` | — | Hold to enter Sprouted Leaf Fanning Mode |
| SUPPORTS | TOGGLE_CONE_KNOT_SELECTION | `e` | — | Press to toggle selection between support cone and parent base knot |
| CAMERA | FOCUS_PICK | `f` | — | Press to focus hovered point on selected model, else snap to best visible model |
| CAMERA | TOGGLE_PROJECTION | `o` | — | Toggle camera projection (Orthographic / Perspective) |
| CAMERA | INTERIOR_VIEW | `x` | — | Toggle Interior View Mode |
| CANVAS | TOOL_SELECT | `q` | — | Switch canvas tool to Drag |
| CANVAS | TOOL_MODIFY | `m` | — | Switch canvas tool to Transform |
| CANVAS | TOOL_MODIFY_LOCAL | `m` | `shift` | Switch canvas tool to Transform with the gizmo in the model's own axes |
| CANVAS | TOOL_SMOOTH | `s` | — | Switch canvas tool to Brush |
| CANVAS | TOOL_ARRANGE | `a` | — | Switch canvas tool to Arrange, or arrange all when it is already active |
| CANVAS | TOOL_DUPLICATE | `d` | — | Switch canvas tool to Duplicate |
| CANVAS | SELECT_ALL | `a` | `ctrl` | Select all visible models |
| CANVAS | COPY | `c` | `ctrl` | Copy selected model(s) |
| CANVAS | PASTE | `v` | `ctrl` | Paste copied model(s) |
| PRESETS | SLOT_1…SLOT_6 | `1`…`6` | — | Pinned Slot 1 … Pinned Slot 6 (row label is `Slot N`) |
| GLOBAL | DELETE | `Backspace` | — | Delete selected item |
| GLOBAL | UNDO | `z` | `ctrl` | Undo last action |
| GLOBAL | REDO | `z` | `ctrl+shift` | Redo last action |
| GLOBAL | SAVE | `s` | `ctrl` | Save active scene |
| GLOBAL | SAVE_AS | `s` | `ctrl+shift` | Save active scene as… |

`DEFAULT_KEYBINDINGS` additionally declares `DEBUG`, `NAVIGATION`, `HOLE_PUNCH` and
`CUT` categories (10 + 4 + 1 + 1 actions). The Hotkeys tab never shows them, but
they exist in `config` and are persisted.

## HotkeyRow

- **Source**: `HotkeysSettingsTab.tsx:337-408`.
- **Renders**: `<div>` with the label on the left and, on the right, a clickable
  binding button, plus the optional secondary keycap.
- **Layout & styling**:
  - row: `flex w-full items-center justify-between gap-2 rounded-md border px-2 py-1.5 transition-colors`
  - recording style: `borderColor: color-mix(in srgb, var(--accent), var(--border-subtle) 35%)`,
    `background: color-mix(in srgb, var(--accent), var(--surface-1) 84%)`,
    `color: var(--text-strong)`
  - idle style: `borderColor: var(--border-subtle)`,
    `background: color-mix(in srgb, var(--surface-2), transparent 10%)`,
    `color: var(--text-strong)`
  - label: `min-w-0 text-[11px] truncate`, `color: var(--text-strong)`, `title={label}`
  - right cluster: `inline-flex items-center justify-end gap-1`
  - binding button: `inline-flex min-w-[92px] items-center justify-center gap-1 rounded-md px-1 py-0.5 transition-colors hover:brightness-110`
  - recording placeholder: `text-[11px] font-medium`, `color: var(--text-muted)`, text `Press keys…`
  - secondary block: `inline-flex items-center gap-1` with `title="Always available"`;
    a `text-[11px]` `/` in `--text-muted` with `aria-hidden`, then a muted `KbdToken`.
- **States**: default = keycaps; recording = `Press keys…`; hovering the binding
  button brightens it.
- **Callbacks**: binding button `onClick` stops propagation, then either `onCancel()`
  or `onRecord()`; button `title` is `Cancel` while recording, else `Click to change`.
- **Token rendering**: `getBindingTokens(binding, primaryModifierLabel)` produced
  tokens; each token is a `KbdToken` keyed `${binding.description}-${token}`.

## KbdToken

- **Source**: `HotkeysSettingsTab.tsx:410-429`.
- `<kbd className="inline-flex min-w-[20px] items-center justify-center rounded border px-1 py-1 font-mono text-[11px]">`
- muted (`secondary`): `borderColor: var(--border-subtle)`,
  `background: color-mix(in srgb, var(--surface-2), transparent 40%)`,
  `color: var(--text-muted)`.
- normal: `borderColor: color-mix(in srgb, var(--accent), var(--border-subtle) 50%)`,
  `background: color-mix(in srgb, var(--surface-2), transparent 4%)`,
  `color: var(--text-strong)`.

## Label derivation (`hotkeyLabels.ts`)

- `toKeyLabel(key)`: `' '` → `Space` (checked first); single character → upper-cased;
  otherwise the key unchanged. So `Backspace` stays `Backspace`, `arrowup` stays
  `arrowup`, `Alt`/`Control` stay as written.
- `toModifierLabel(modifier, primary)`: `ctrl` → the platform primary (`Cmd`/`Ctrl`),
  `shift` → `Shift`, `alt` → `Alt`, `meta` → `Cmd` when primary is `Cmd` else `Meta`;
  anything else passes through.
- `getBindingTokens(binding, primary)`: modifiers in declared order, then the key.
  `modifier` is a `+`-separated string, e.g. `ctrl+shift`.
- The source comment above `toModifierLabel` is the reason: "hotkeyStore maps a
  configured `ctrl` onto the platform's primary modifier, which is Cmd on macOS.
  Labelling it \"Ctrl\" there would advertise a shortcut that does not exist."

## Recording overlay

- **Layout & styling**: `fixed inset-0 z-50 flex items-center justify-center bg-black/55 backdrop-blur-sm px-4`;
  backdrop `onMouseDown` cancels only when `event.target === event.currentTarget`.
  Panel `w-full max-w-md rounded-xl border p-5 shadow-2xl`,
  `borderColor: var(--border-strong)`, `background: var(--surface-0)`.
  Heading block `flex items-center gap-1.5`; `<h4 className="text-base font-semibold">` in
  `--text-strong`; body `mt-2 text-sm` in `--text-muted`; hint `mt-1 text-xs` in
  `--text-muted`; actions `mt-4 flex justify-end`.
  Cancel: `Button variant="secondary" size="auto" className="!h-8 !px-3 !py-0 text-xs rounded-md"`.
- **Text**: `Recording Shortcut`; `Press the new key combination now.`;
  `Press <strong>Esc</strong> or click outside to cancel.`; `Cancel`.
- **States**: rendered only while `recordingKey !== null`.

## Recording algorithm (per key press)

The `useEffect` at `HotkeysSettingsTab.tsx:57-137` is the contract:

1. Runs only while `recordingKey` is set. First statement: `suspendHotkeyDispatch()`.
   The source comment: "Silence the app while recording: otherwise the key being
   captured also reaches its normal handlers — Escape would close Settings instead of
   cancelling here."
2. `let pressedNonModifier = false;`
3. `handleKeyDown` (registered with capture `true` on `window`):
   - `e.preventDefault()`, `e.stopPropagation()`
   - `if (e.repeat) return;`
   - `Escape` → `setRecordingKey(null)` and return.
   - collect modifiers in this exact order: `ctrl` (`e.ctrlKey`), `shift`, `alt`, `meta`.
   - if `e.key` is one of `Control|Shift|Alt|Meta` → return (no commit).
   - `pressedNonModifier = true`
   - `finalKey = normalizeRecordedKey(e.key)` (`' '` → `Space`; one character →
     lower-cased; else unchanged).
   - `updateHotkey(category, action, { key: finalKey, modifier: modifiers.length ? modifiers.join('+') : undefined, description: config[category][action].description })`
   - `setRecordingKey(null)`
4. `handleKeyUp`:
   - `if (pressedNonModifier) return;`
   - only `Control|Shift|Alt|Meta` continue; they also `preventDefault` + `stopPropagation`.
   - `targetKey = keyMap[e.key]` (identity map for the four names).
   - modifiers are re-collected from the event **excluding the released key**
     (`if (e.ctrlKey && e.key !== 'Control')` etc.).
   - commit as above and `setRecordingKey(null)`.
5. Both handlers get `(handler as any).__isHotkeySystemInternal = true`.
6. Cleanup removes both listeners and calls `resumeHotkeyDispatch()`.
7. Effect deps: `[recordingKey, config, updateHotkey]`.

- **Data mutated**: `updateHotkey` writes `config[category][action]`; the provider
  then JSON-stringifies the whole config into localStorage `app-hotkeys-config`.
- **Port target**: `gpui/src/features/settings/hotkeys.rs` already has `key_label`,
  `modifier_label`, `binding_tokens`, `kbd_token`, `hotkey_row`, `hotkey_section`,
  `category_label`, `pinned_slot_label`, `hotkeys_tab`, `hotkeys_overlay`;
  `HotkeyState` in `state.rs` holds `recording: Option<(&'static str, &'static str)>`
  and `overrides: HashMap<(&'static str, &'static str), HotkeyBinding>`; defaults in
  `defaults.rs::HOTKEY_DEFAULTS` (33 rows, the five displayed categories only).
  **Missing**: any key capture (the overlay only cancels), the
  empty-section filter, and the four hidden categories of `DEFAULT_KEYBINDINGS`.

---

# Surface: PluginsSettingsTab — plugin + profile-pack manager

- **Source**: `src/components/settings/PluginsSettingsTab.tsx:1-1536`.
  Component starts at `:218`; installed-row assembly `:622-670`; row renderer
  `:670-765`; main return `:767`.
- **Renders**: a vertical `space-y-3` stack: (1) Plugins card with the profile-pack
  installer and the desktop-only sideload import row; (2) the installed-plugins card
  grouped into three tiers; (3) a dev-only "Plugin Creation Studio" link; (4) nine
  dialogs.

## Module-level constants (must be ported verbatim)

| Name | Value / behaviour |
|---|---|
| `PLUGIN_RELOAD_SECONDS` | `15`. Comment: "Seconds the prompt waits before it reloads on its own. The plugin's code is loaded by the webview and the engine at start, so nothing here is optional: the dialog has one outcome and no dismiss." |
| `reloadCountdownLabel(seconds)` | `msg\`Reload Now (${seconds}s)\`` — module scope, "Interpolated messages live at module scope: React Compiler renames interpolated locals inside components, which desyncs the message id from the catalog." |
| `passphrasePlaceholder(pluginName)` | `msg\`Passphrase for ${pluginName}\`` |
| `BUILTIN_ATHENA_REPOSITORY_URL` | `'https://github.com/Open-Resin-Alliance/DragonFruit'` |
| `SIDELOAD_CAPABILITY_MESSAGES` | `slicerEncoder`: `Slicer encoder`; `networkOperations`: `Network operations`; `fileType`: `File-type import` |
| `SIDELOAD_CAPABILITY_SHORT_MESSAGES` | `slicerEncoder`: `Encoder`; `networkOperations`: `Network`; `fileType`: `Import` (each with `context: 'plugin capability'`) |
| `reloadDragonFruit()` | `window.location.reload()` |
| `openExternalUrl(url)` | `invoke('open_external_url', { url })`, falling back to `window.open(url, '_blank')` |

`truncateRepositoryUrl(url)` (`:157-174`) is **dead code** in this file — it is
never referenced. Do not port it.

## Plugin card

- **Layout & styling**: `<section className="relative rounded-md border p-3">` with
  `borderColor: var(--border-subtle)`, `background: var(--surface-1)`.
  Header: `flex items-start gap-2.5`; icon tile as in the Hotkeys card
  (`h-8 w-8`, `border`, `color-mix(in srgb, var(--surface-2), transparent 8%)`) with
  `Plug` `h-4 w-4` in `var(--accent)`.
  Title `text-xs font-semibold`; description `mt-0.5 text-xs leading-snug` in
  `var(--text-muted)`; the text block is `min-w-0 flex-1 pr-10` so the cog does not
  overlap it.
  Rows container `mt-2 grid gap-2`.
- **Allowance cog** (desktop only): `IconButton variant="surface" size="sm" tone="warning"`
  with `className="absolute right-3 top-3 hover:brightness-110"`,
  `aria-label` and `title` both `Sideloaded packages`, child `PackageOpen h-4 w-4`.
- **Strings**: title `Plugins`; description `Profile packs add presets and material
  profiles. A sideloaded package adds code and needs a publisher you trust.`
- **Install row** (`SettingRow bordered`):
  - label `Install profile pack`; description `Adds presets and materials from a
    repository. Contains no code.`
  - control cluster: `<div className="flex items-center gap-2" style={{ width: 'min(420px, 46vw)' }}>`
  - `Input type="text"` with `className="h-[34px] min-w-0 flex-1 text-sm"`,
    `aria-label="Plugin repository URL"`, placeholder
    `isDevRuntime ? 'df://debug_plugin_official' : _(msg\`https://github.com/owner/repo\`)`;
    `onKeyDown` Enter triggers install.
  - `Button variant="tinted-accent" className="inline-flex shrink-0 items-center gap-1.5"`
    `disabled={isInstalling}`; content `Spinner size="sm"` + `Installing…` while
    installing, else `Download h-3.5 w-3.5` + `Install`.
- **Sideload import row** — rendered only when `isDesktop && allowSideload`:
  label `Import package`; description `Checks the publisher signature, then unseals
  and installs the .dfplugin file.`; `Button variant="secondary"` with
  `className="inline-flex shrink-0 items-center gap-1.5"`, `disabled={sideloadBusy}`,
  `Spinner`/`PackagePlus h-3.5 w-3.5`, label `Choose file…`.
- **Status line** (`status.kind !== 'idle'`): `<p className="mt-2 text-xs">` coloured
  `#fca5a5` for error, `#86efac` for success; text is `status.message`.
- **Data**: `getInstalledPlugins()` is called **on every render** (comment:
  "Read fresh list on every render; avoids stale memo behavior when external store
  updates don't change snapshot identity but still emit notifications."). The store
  subscription is
  `useSyncExternalStore(subscribeToProfileStore, getProfileStoreSnapshot, getProfileStoreServerSnapshot)`.
- **Callbacks**:
  - cog → `setAllowanceDialogOpen(true)`
  - install → `handleInstall()`
  - import → `handlePickSideloadPackage()`

## Install flow (profile pack from GitHub)

`handleInstall` (`:469-512`):

1. `repoUrl.trim()`; empty → status error `Please enter a GitHub repository URL.` and return.
2. `setIsInstalling(true)`, clear status and both pending-preview states.
3. `runInstallRequest(trimmed)` → `requestPluginManifest({ repoUrl, allowUnverifiedInstall: false, acknowledgeLiabilityWarning: false })`.
4. Accept only when `status ∈ 200..299 && payload.ok && payload.manifest`; otherwise
   throw an `Error` carrying `payload` (`err.payload = payload`), message
   `payload.error || 'Unable to install plugin from GitHub repository.'`.
   Source comment: "2xx is not enough on its own: a parsed-but-rejected manifest also fails the install."
5. `normalizePluginManifest(payload.manifest)`; null → `Plugin manifest is missing required fields.`
6. Open the preview with
   `trust: payload.repoAllowlisted === false ? 'unverified-user-approved' : 'allowlisted'`,
   `liabilityAcceptedAt` = now when unverified, `originUrl = payload.rawManifestUrl`.
7. On error: if `payload.requiresLiabilityWarning` → open the liability dialog with
   `{ repoUrl, unverifiedRepo, allowlistRules }` and set status error
   `This repository is not on the allowlist. Review and acknowledge the warning to proceed.`;
   otherwise status error = `error.message` or `Plugin installation failed.`
8. `finally setIsInstalling(false)`.

`normalizePluginManifest` (`:198-215`) requires non-empty trimmed `id`, `name`,
`version`; `schemaVersion` defaults to `1` when not finite; `printerPresets` and
`materialTemplates` default to `[]` when not arrays.

`handleConfirmLiabilityInstall` retries `runInstallRequest(repoUrl, { allowUnverifiedInstall: true, acknowledgeLiabilityWarning: true })`
and opens the preview with `trust: 'unverified-user-approved'` and
`liabilityAcceptedAt` = now.

`handleInstallFromPreview` (`:555-578`) calls
`installPluginFromManifest(manifest, repoUrl, { manifestSha256, installTrust, liabilityAcceptedAt })`,
then `highlightInstalledRow(\`profile:${manifest.id}\`)`, clears the URL field and
closes the preview. Errors go to `status` (`Plugin installation failed.` fallback).

## Installed rows

Assembled at `:622-660` into `InstalledPluginRow`:

```ts
type InstalledPluginRow = {
  key: string; id: string; name: string; version: string; description?: string;
  source: 'builtin' | 'github' | 'sideloaded'; link?: string; publisher?: string;
  signingKeyFingerprint?: string; capabilities?: SideloadedCapability[];
  unverified: boolean; acceptedAt?: string; remove?: () => void;
};
```

Profile-store rows (all except ids present in `sideloadPlugins`):
- `key = \`profile:${manifest.id}\``; `source = plugin.source === 'builtin' ? 'builtin' : 'github'`.
- `link = manifest.homepage || (isBuiltin ? BUILTIN_ATHENA_REPOSITORY_URL : plugin.sourceUrl)`.
- `description = manifest.description`.
- `unverified = !isBuiltin && plugin.installTrust === 'unverified-user-approved'`.
- `acceptedAt = plugin.liabilityAcceptedAt`.
- `remove` only for non-builtin: `() => handleUninstall(id, name)`.

Sideloaded rows: `key = \`sideloaded:${manifest.id}\``, `source: 'sideloaded'`,
`description = manifest.vendor.name`, `publisher = manifest.vendor.name`,
`signingKeyFingerprint = plugin.publisherFingerprint`,
`capabilities = manifest.capabilities`, `unverified: false`,
`remove: () => setPendingRemoveSideload(plugin)`.

Then `installedRows.sort((l, r) => l.name.localeCompare(r.name))` and group in this
fixed order — `sideloaded`, `github`, `builtin` — dropping empty groups. Source
comment: "Packages first, then GitHub packs, then what ships with the app — each
tier separated by a hairline, and an empty tier contributes nothing."

Dedupe comment: "A sideloaded package with presets also registers a profile-store
entry, so the printer library sees them. That entry must not become a second row
here: the host's record is the one that can be enabled, removed or re-imported."

## Installed list section

- `<section className="rounded-md border p-3">`, `--border-subtle` / `--surface-1`.
- Header `flex items-start gap-2.5`; icon tile `PackageCheck h-4 w-4` in `var(--accent)`.
- Title `Installed plugins` (`text-xs font-semibold`); subtitle is a `<Plural>`:
  `# plugin in this profile.` / `# plugins in this profile.` with `value={installedRows.length}`.
- Groups: separator when `index > 0`: `<div className="my-2 h-px" style={{ background: 'var(--border-subtle)' }} aria-hidden />`.
  First group `<ul className="mt-2 space-y-1.5">`, later groups
  `<ul className="space-y-1.5">`.

## Installed row

- `<li className="flex items-center justify-between gap-2 rounded-md border px-2.5 py-2">`.
- Recently-installed glow (3 s), style object:
  - `borderColor`: `color-mix(in srgb, #86efac, var(--border-subtle) 28%)` vs `var(--border-subtle)`
  - `background`: `color-mix(in srgb, #86efac, var(--surface-0) 88%)` vs `var(--surface-0)`
  - `boxShadow` (only while highlighted):
    `inset 0 0 0 1px color-mix(in srgb, #86efac, transparent 45%), inset 0 0 14px color-mix(in srgb, #86efac, transparent 72%), 0 0 16px color-mix(in srgb, #86efac, transparent 62%)`
  - `transition: 'box-shadow 600ms ease, border-color 600ms ease, background 600ms ease'`.
  - Source comment: "The fill, not just the outline: a ring alone read as a focus state."
- Left block `min-w-0`; title line `flex flex-wrap items-center gap-1.5`:
  - `source === 'github'` → `Github h-3.5 w-3.5` in `var(--text-muted)`
  - `source === 'sideloaded'` → `PackagePlus h-3.5 w-3.5` in `var(--text-muted)`
  - `row.link` → `<button className="inline-flex items-center gap-1 truncate text-sm font-semibold underline underline-offset-2" style={{ color: 'var(--accent)', background: 'none', border: 'none', cursor: 'pointer', padding: 0 }} title={row.link}>` with the name and `ExternalLink h-3 w-3`; otherwise a plain
    `<span className="truncate text-sm font-semibold">` in `--text-strong`.
  - `row.unverified` → pill `inline-flex items-center rounded-full border px-1.5 py-0.5 text-[11px]`,
    `borderColor: color-mix(in srgb, #d97706, var(--border-subtle) 35%)`,
    `color: color-mix(in srgb, #d97706, var(--text-strong) 18%)`,
    `background: color-mix(in srgb, #d97706, var(--surface-2) 88%)`; text
    `Unverified source`.
  - optional description: `mt-0.5 text-xs leading-snug` in `--text-muted`.
- Right cluster: `flex shrink-0 items-center gap-1.5 self-center`.
  - Info: `Tooltip content` `Plugin details`, wrapper `flex items-center`; `IconButton
    variant="ghost" size="xs" aria-label="Plugin details"` → `InfoIcon h-4 w-4` in
    `var(--text-muted)`.
  - builtin: `Tooltip content="Built-in plugin"` → `ShieldCheck h-4 w-4` in
    `builtinShieldIconColor` (`#86efac` in dark; a light-theme mix computed from
    `data-theme` and `prefers-color-scheme`, see below).
  - builtin **and** `isOraHostedRepository(row.link)`: `Tooltip content="Officially verified by ORA"`
    → `CheckCircle2 h-4 w-4` in `color-mix(in srgb, #d97706, var(--text-strong) 18%)`.
  - `row.remove`: `Tooltip content="Remove plugin"` → `IconButton variant="ghost" size="xs"`
    with `className="hover:brightness-125"`, child `Trash2 h-4 w-4` in `#fca5a5`.
- **States** (row): highlighted / normal; builtin vs github vs sideloaded; link vs
  plain name; verified-ORA only for builtin whose link host is `github.com` under
  `/open-resin-alliance/`.

### Theme detection for the builtin shield

`isLightTheme` is computed by reading `document.documentElement`'s `data-theme`
attribute (`light`/`dark`), falling back to `window.matchMedia('(prefers-color-scheme: light)')`,
and kept in sync with a `MutationObserver` on `attributes, attributeFilter: ['data-theme']`
plus a media-query `change` listener (legacy `addListener` fallback). Then

```ts
const builtinShieldIconColor = isLightTheme
  ? 'color-mix(in srgb, #4f8a08, var(--text-strong) 28%)'
  : '#86efac';
```

## Reload-required flow

- `reloadRequired` set to `true` after a successful sideload install or removal.
- Effect: on `reloadRequired`, reset `reloadSecondsLeft = 15` and start a 1 s
  interval decrementing to a floor of 0; cleared when `reloadRequired` goes false.
- Effect: when `reloadRequired && reloadSecondsLeft <= 0` → `reloadDragonFruit()`.
- Dialog: `ariaLabel="Reload required"`, title `Reload Required`, subtitle
  `The plugin list has changed, DragonFruit has to reload.`, icon `RotateCcw`,
  `iconTone="warning"`, `zIndexClassName="z-[200]"`, `maxWidthClassName="max-w-md"`,
  `closeAriaLabel="Reload DragonFruit now"`, `onClose`/`onBackdropClick` both
  `reloadDragonFruit` (no dismiss).
- Action button: `Button variant="tinted-accent" size="md" className="relative isolate gap-1.5 overflow-hidden"`
  with `style={{ color: 'var(--accent-contrast)' }}`; inside, a draining fill
  `<span aria-hidden className="pointer-events-none absolute inset-y-0 left-0 -z-10 transition-[width] duration-1000 ease-linear" style={{ width: \`${(reloadSecondsLeft / PLUGIN_RELOAD_SECONDS) * 100}%\`, background: 'color-mix(in srgb, var(--button-tint, var(--accent)), #000 62%)' }} />`,
  then `RotateCcw h-3.5 w-3.5` and `reloadCountdownLabel(seconds)`.
  Source comment: "The button is the countdown: the fill drains over the remaining seconds, so the wait is visible without a second progress element." and "The fill is dark enough to sit under the label: the accent text the tinted variant uses is unreadable on a bright accent bar."
- Body text: `Reloading loads an installed plugin's code, or releases a removed one's formats, network mode and file types. Unsaved changes to the current scene are lost.`

## Sideloaded packages (`getSideloadAllowance`)

- On desktop mount: `Promise.all([getSideloadAllowance(), listSideloadedPlugins()])`,
  then `refreshSideloadedContributions()`; failure sets `status` error `String(error)`.
- `handleSideloadAllowanceChange(next)`: optimistic `setAllowSideload(next)`, clears
  status, awaits `setSideloadAllowance(next)` → `listSideloadedPlugins()` →
  `refreshSideloadedContributions()`; on error reverts and sets status.
- `handlePickSideloadPackage()`: `pickSideloadPackage()` → null means cancel; else
  `inspectSideloadPackage(path)`, clear passphrase, `setPendingSideload({ path, info })`.
- `handleConfirmSideloadInstall()`: `installSideloadPackage(path, sealed ? sideloadPassphrase : undefined)`
  then refresh list + contributions, close both dialogs, highlight
  `sideloaded:${installed.manifest.id}`, `setReloadRequired(true)`. A failure on a
  sealed package sets `passphraseError`; otherwise `status` error.
- `handleConfirmRemoveSideload()` — collect `presetIds` from
  `manifest.printerPresets[].presetId` **before** removing (comment: "Collect the
  presets before the package (and its manifest) is gone: the profiles built from
  them are what stays behind, greyed out until it returns."), then
  `removeSideloadedPlugin(id)`, refresh list + contributions,
  `markPrinterProfilesMissingPlugin(id, name, presetIds)`, `setReloadRequired(true)`.

## Dialog inventory

All use `StructuredDialogModal` (portal to `document.body`), whose shared shell is:
backdrop `fixed inset-0 ${zIndexClassName} flex items-center justify-center bg-black/55 backdrop-blur-sm px-3`
with backdrop-mousedown close; panel
`w-full ${maxWidthClassName} overflow-hidden rounded-xl border shadow-2xl`,
`background: var(--surface-0)`, `borderColor: var(--border-subtle)`,
`boxShadow: 0 24px 46px rgba(0,0,0,0.42)`; header
`flex items-center justify-between gap-4 border-b px-5 py-4`; icon tile
`inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-md border` +
`ICON_TONE_STYLES[iconTone]`; title `text-base font-semibold leading-tight`; subtitle
`mt-0.5 text-[11px] leading-snug`; close `IconButton variant="surface" size="md"`
with `X w-4 h-4`; body default `space-y-4 p-5`; actions default
`grid grid-flow-col auto-cols-fr gap-2 pt-1`.

`ICON_TONE_STYLES`:

| tone | borderColor | background | color |
|---|---|---|---|
| `warning` | `color-mix(in srgb, #d97706, var(--border-subtle) 50%)` | `color-mix(in srgb, #d97706, var(--surface-1) 85%)` | `#d97706` |
| `danger` | `color-mix(in srgb, #ef4444, var(--border-subtle) 55%)` | `color-mix(in srgb, #ef4444, var(--surface-1) 88%)` | `var(--danger)` |
| `accent` | `color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 45%)` | `color-mix(in srgb, var(--accent-secondary), var(--surface-1) 90%)` | `var(--accent-secondary)` |
| `neutral` | `var(--border-subtle)` | `var(--surface-1)` | `var(--text-muted)` |

Escape: `useEscapeToClose(open, closeDisabled ? undefined : (onClose ?? onBackdropClick))`
— a disabled close swallows Escape.

### 1. Install an unverified plugin (liability gate)

`open={pendingLiabilityInstall !== null}`, ariaLabel `Install unverified plugin`,
title `Install an unverified plugin?`, subtitle `owner/name` from
`unverifiedRepo` (empty halves render as empty strings), icon `AlertTriangle`,
`iconTone="warning"`, `maxWidthClassName="max-w-xl"`, `zIndexClassName="z-[130]"`,
`closeAriaLabel="Close modal"`, `closeDisabled={isInstalling}`, `onClose` and
`onBackdropClick` clear the pending state.

Body: `space-y-1.5` with
- `DragonFruit cannot confirm who published this repository.` (`text-sm leading-relaxed`, `--text-strong`)
- `A profile pack is data only — presets and material templates — so installing it cannot add code. The values it declares are still yours to check.` (`text-sm leading-relaxed`, `--text-muted`)
- when `allowlistRules.length > 0`: a box `rounded-md border px-3 py-2` on `--surface-1`
  with label `Trusted sources` (`text-[11px] font-semibold uppercase tracking-wide`)
  and the rules joined with `, ` (`mt-0.5 text-xs break-all`).

Actions: `Cancel` (secondary, clears state) and
`Install anyway` (tinted-warning, `inline-flex items-center justify-center gap-1.5`,
spinner while installing).

### 2. Review plugin before install

`open={pendingInstallPreview !== null}`, ariaLabel `Review plugin before install`,
title `Install this plugin?`, subtitle = manifest name, icon `Plug`, `iconTone="accent"`,
`maxWidthClassName="max-w-2xl"`, `zIndexClassName="z-[130]"`, `closeAriaLabel="Close modal"`,
`closeDisabled={isInstalling}`, `bodyClassName="space-y-3 p-5"`.

Body, in order:
1. Badge row `flex flex-wrap items-center gap-1.5`:
   - version pill `text-[11px] rounded-full border px-1.5 py-0.5` (`--border-subtle`,
     `--text-muted`) with `v{version}`.
   - trust pill (`text-[11px] rounded-full border px-1.5 py-0.5 font-semibold`):
     allowlisted → border `color-mix(in srgb, #86efac, var(--border-subtle) 45%)`,
     `color: #86efac`, background `color-mix(in srgb, #86efac, var(--surface-2) 92%)`,
     text `Allowlisted source`; unverified → border `color-mix(in srgb, #d97706, var(--border-subtle) 45%)`,
     `color: color-mix(in srgb, #d97706, var(--text-strong) 18%)`,
     background `color-mix(in srgb, #d97706, var(--surface-2) 88%)`, text
     `Unverified source`.
2. Two-column grid `grid grid-cols-1 sm:grid-cols-2 gap-3`:
   - **Identity** card (`rounded-md border p-3`, `--surface-1`): label `Identity`;
     rows `mt-1.5 space-y-1 text-xs` (`--text-muted`): `ID:` **bold key** + value;
     `Author:` when present; `Contents:` + `<Plural>` `# printer preset(s)` `, ` +
     `<Plural>` `# material template(s)`.
   - **Origin** card: label `Origin`; `Repository:` + `repoUrl`; `Manifest:` +
     `originUrl` when present; `SHA-256:` + monospaced value.
3. Optional **Description** card: label `Description` and the manifest description.

Actions: `Cancel` and `Install` (tinted-accent, spinner + `Installing…` while busy,
`onClick={handleInstallFromPreview}`).

### 3. Remove plugin (profile pack)

ariaLabel `Remove plugin`, title `Remove Plugin?`, subtitle = plugin name, icon
`Trash2`, `iconTone="danger"`, `zIndexClassName="z-[130]"`, `closeAriaLabel="Close modal"`.
Body: `This removes the plugin from your profile library.`
Actions: `Cancel`, `Remove plugin` (tinted-danger, `Trash2 h-3.5 w-3.5`,
`onClick={handleConfirmUninstall}` → `uninstallPlugin(id)`).

### 4. Package passphrase

`open={passphraseOpen && pendingSideload !== null}`, ariaLabel `Package passphrase`,
title `Enter the passphrase`, subtitle = manifest name, icon `LockKeyhole`,
`iconTone="neutral"`, `maxWidthClassName="max-w-md"`, `zIndexClassName="z-[131]"`,
`closeAriaLabel="Close modal"`, `closeDisabled={sideloadBusy}`,
`onClose`/`onBackdropClick` = `closePassphraseDialog` (clears open flag, passphrase, error).

Body: explanatory `text-sm leading-relaxed` (`--text-muted`):
`The package is sealed. Enter the key your vendor gave you; it is kept locally so the plugin can load later.`
then `Input autoFocus id="sideload-passphrase" type="password" autoComplete="off" className="h-10 w-full"`
with `value`, `onChange`, Enter-to-submit when non-empty and not busy, placeholder
`_(passphrasePlaceholder(name))`, `aria-label="Package passphrase"`.
Optional error `<p className="text-xs leading-relaxed" style={{ color: '#fca5a5' }}>`.
Actions: `Cancel` (disabled while busy) and `Unseal and install`
(tinted-accent, disabled while busy **or** when `sideloadPassphrase.trim().length === 0`).
Source comment: "Sealed packages finish in a second, focused step so the passphrase is not a field a user can scroll past on a long review sheet."

### 5. Sideloaded packages (allowance switch)

`open={allowanceDialogOpen}`, ariaLabel `Sideloaded packages`, title
`allowSideload ? 'Sideloading is on' : 'Allow untrusted plugins?'`, subtitle
`_(msg\`Sideloaded packages\`)`, icon `ShieldAlert`, `iconTone="warning"`,
`maxWidthClassName="max-w-lg"`, `zIndexClassName="z-[130]"`, `closeAriaLabel="Close modal"`.

Body: if `allowSideload` → one paragraph `Packages you import run with your
permissions and can read any file you can read, open network connections, and crash
the app if they misbehave. Disabling this stops new imports; packages already
installed stay on disk.`; else `space-y-1.5` with `A sideloaded package is a native
program, not a profile pack.` (`--text-strong`) and `Once imported it runs with your
permissions: it can read any file you can read, open network connections, and crash
the app if it misbehaves.` (`--text-muted`).
Then a warning box: `rounded-md border px-3 py-2`,
`borderColor: color-mix(in srgb, #d97706, var(--border-subtle) 40%)`,
`background: color-mix(in srgb, #d97706, var(--surface-1) 92%)`; a `flex items-start gap-2`
with `AlertTriangle mt-0.5 h-3.5 w-3.5 shrink-0` in `#d97706` and
`Signatures prove who built the package, not that it is safe. Only enable this for
a publisher you trust.` (`text-xs leading-relaxed`, `--text-muted`).

Actions: `Cancel`; `Disable Sideloading` (tinted-danger) or `Enable Sideloading`
(tinted-warning), both with `ShieldAlert h-3.5 w-3.5`, both closing the dialog after
`handleSideloadAllowanceChange(!allowSideload)`.

### 6. Install sideloaded plugin (review)

`open={pendingSideload !== null}`, ariaLabel `Install sideloaded plugin`, title
`Install this plugin?`, subtitle = manifest name, icon `PackagePlus`,
`iconTone="accent"`, `maxWidthClassName="max-w-xl"`, `zIndexClassName="z-[130]"`,
`closeAriaLabel="Close modal"`, `closeDisabled={sideloadBusy}`,
`bodyClassName="space-y-3 p-5"`.

Body:
1. Badge row `flex flex-wrap items-center gap-1.5`: version pill `v{version}`; one pill
   per capability (`text-[11px] rounded-full border px-1.5 py-0.5`, `--border-subtle`,
   `--text-strong`) showing `SIDELOAD_CAPABILITY_MESSAGES[capability]`.
2. `ReviewFact` table: container `overflow-hidden rounded-md border` on `--surface-1`;
   each fact is `flex items-baseline justify-between gap-3 border-b px-3 py-2 last:border-b-0`
   (or `border-b px-3 py-2 last:border-b-0` when `stacked`), label
   `shrink-0 text-[11px] font-semibold uppercase tracking-wide` in `--text-muted`,
   value `text-xs` in `--text-strong` (plus `font-mono break-all` when `mono`).
   Rows:
   - `Publisher` → `manifest.vendor.name`
   - `Signing key` (mono) → `shortFingerprint(publisherFingerprint)` wrapped in a
     `Tooltip` (`content` = the full fingerprint in a `break-all` span, `maxWidth={320}`,
     `wrapperClassName="inline-flex"`)
   - `Package ID` (mono) → `manifest.id`
   - `Requires` → `DragonFruit {minAppVersion} or newer`
   - `Sealed` → `Yes` / `No`
   - `Output formats` (mono), only when `manifest.formats.length > 0`: per format,
     when `fileExtensionFromVersion` and `formatVersions.length > 0`, join that
     format's versions as `ext (label)` / `ext` (id trimmed, `.` prefixed once,
     label trimmed, empty entries dropped); otherwise join `format.extensions` with
     spaces. Formats are joined with `, `.
     Source comment: "With `fileExtensionFromVersion` each version is an output extension of its own, so the review lists every file the package can write (`.exf` and `.exm`), not just the canonical one."
3. `replacesExisting` → `<p className="text-xs" style={{ color: 'color-mix(in srgb, #d97706, var(--text-strong) 18%)' }}>` `An older install with this ID and publisher is replaced.`
4. Closing paragraph: `This package runs native code with your user account. It can read your files and open network connections. Only install packages from a vendor you trust.`

Actions: `Cancel` (disabled while busy); if `info.sealed` → `Continue`
(tinted-accent, `LockKeyhole h-3.5 w-3.5`, clears passphrase + error then opens the
passphrase dialog), else `Install` (tinted-accent, spinner while busy).

### 7. Remove sideloaded plugin

ariaLabel `Remove sideloaded plugin`, title `Remove this plugin?`, subtitle = manifest
name, icon `Trash2`, `iconTone="danger"`, `zIndexClassName="z-[130]"`,
`closeAriaLabel="Close modal"`.
Body: `The package, its extracted payload and its stored key are deleted. Re-importing needs the passphrase again.`
Actions: `Cancel`, `Remove plugin` (tinted-danger, `Trash2 h-3.5 w-3.5`).

### 8. Plugin details

`open={detailsRow !== null}`, ariaLabel `Plugin details`, title = row name, subtitle =
row description, icon `InfoIcon`, `iconTone="accent"`, `maxWidthClassName="max-w-md"`,
`zIndexClassName="z-[140]"`, `closeAriaLabel="Close modal"`; `onClose`/`onBackdropClick`
clear `detailsRow`, `copiedSigningKey` and `capabilitiesExpanded`.

Body: a `ReviewFact` table (`overflow-hidden rounded-md border`, `--surface-1`):
- `Package ID` (mono)
- `Source` → `Built-in` / `GitHub` / `Sideloaded`
- `Version` → `v{version}`
- `Publisher` when present
- `Signing key` when present → `shortFingerprint` plus a copy `IconButton`
  (`variant="ghost" size="xs" aria-label="Copy signing key"`), tooltip `Copied` or
  `Copy signing key`. On click: `navigator.clipboard.writeText(full)`, set
  `copiedSigningKey`, clear after 2000 ms; a denied clipboard is swallowed
  (comment: "A denied clipboard leaves the full value visible in the tooltip.").
  Icon is `Check h-3.5 w-3.5` in `var(--success)` once copied, else `Copy h-3.5 w-3.5`
  in `--text-muted`.
- `Liability accepted` when `acceptedAt`
- `Repository` when `link` → `repositoryLabel(link)` + `ExternalLink h-3 w-3`, styled
  as a link (`color: var(--accent)`, no border/background/padding, `cursor: pointer`),
  `title={link}`, opens externally.
- `Capabilities` when non-empty; collapsed (`stacked` false) shows short pills
  (`rounded-full border px-1.5 py-0.5 text-[11px]`, `--border-subtle`,
  `--text-muted`) each inside a `Tooltip` with the full capability name, then a
  `ChevronDown` `IconButton` (`aria-label="Show all capabilities"`). Expanded
  (`stacked` true) shows a `flex flex-col items-start gap-1` of full names plus a
  `Less` button (`ChevronUp h-3.5 w-3.5`, `aria-label="Show fewer capabilities"`,
  `text-[11px]`, muted, no chrome).
Actions: single `Close` (secondary).

## Dev-only link

`isDevRuntime = process.env.NODE_ENV === 'development'`. When true, a centred
`<button className="text-xs underline-offset-2 hover:underline">` reading
`Plugin Creation Studio`, colour `var(--text-faint, var(--text-muted))`, opens
`PluginStudioModal` (`src/components/settings/PluginStudioModal.tsx`, a separate
area — dev tooling). A second effect force-closes the studio whenever
`isDevRuntime` is false.

## Port target — Plugins

`gpui/src/features/settings/plugins.rs` exists with `plugins_tab` and
`plugins_overlay` (allowance dialog only). `PluginsState` in `state.rs` already
carries `repo_url`, `allowance_dialog_open`, `allow_sideload`, `details_row`,
`capabilities_expanded`, `copied_signing_key`, `reload_required`,
`reload_seconds_left`, `status`; `write_store` persists only
`allow_sideloaded_complex_plugins`.

What the current port does **not** have, and this spec requires:
- a profile store / installed-plugin list (`installed_count = 0`, a hard-coded
  "No plugins installed in this profile." line — the source's empty list renders no
  rows at all, not this line);
- GitHub manifest fetch and the install/preview/liability dialogs;
- sideload list, pick/inspect/install/remove and the passphrase dialog;
- details dialog, reload-required dialog and countdown;
- the dev studio link (deliberately omitted in a release build; fine).
`plugins_overlay` currently toggles `allow_sideload` locally instead of calling a
host command.

---

# Surface: ExperimentsSettingsTab — experiment toggles behind a disclaimer gate

- **Source**: `src/components/settings/ExperimentsSettingsTab.tsx:1-260`.
  `ExperimentsDisclaimer` at `:22-117`; `disclaimerSeenThisLaunch` at `:123`;
  component at `:125`; the early `return` that renders the gate at `:159-175`.
- **Renders**: either the full-viewport disclaimer gate (portalled to `document.body`),
  or the tab body: an intro card, one row per experiment with an ON/OFF control, and
  a restart notice.
- **Props**: `{ onExit: () => void }`.

## Module state

```ts
// Module-level so the disclaimer is shown until acknowledged with I understand;
// Take me back keeps it for next entry, so it can reappear within the same launch.
let disclaimerSeenThisLaunch = false;
```

`getInitialEnabledState()` builds `Record<id, boolean>` from
`getExperimentDefinitions()` + `isExperimentEnabled(id)`.

## The gate

- **Render path**: `if (showDisclaimer) return typeof document === 'undefined' ? null : createPortal(<ExperimentsDisclaimer …/>, document.body)`.
  The source comment on the component explains the portal: "ORA disclaimer shown as
  a gate before the Experiments tab content. Rendered via a portal to document.body
  so it escapes the settings panel's transient entry animation transform (which
  would otherwise offset a fixed overlay)."
- **Layout & styling**: `fixed inset-0 z-[220] flex items-center justify-center bg-black/55 backdrop-blur-sm px-3`;
  backdrop `onMouseDown` calls `onExit()` only when `event.target === event.currentTarget`.
  Panel `w-full max-w-lg overflow-hidden rounded-xl border shadow-2xl`,
  `background: var(--surface-0)`, `borderColor: var(--border-subtle)`,
  `boxShadow: 0 24px 46px rgba(0,0,0,0.42)`, `role="dialog"`, `aria-modal="true"`,
  `aria-label="Experiments disclaimer"`.
  Header `flex items-center gap-4 border-b px-5 py-4` (`--border-subtle`), inner
  `flex min-w-0 items-center gap-3`, `IconChip size="lg" tone="warning" icon={AlertTriangle}`;
  title `text-base font-semibold leading-tight`; subtitle `mt-0.5 text-[11px] leading-snug`.
  Body `space-y-4 p-5`; paragraphs `text-xs leading-relaxed` in `--text-muted`, the
  third `style={{ color: '#fca5a5', fontWeight: 600 }}`; button grid
  `grid grid-cols-2 gap-2 pt-1` with both buttons `size="md" className="w-full gap-1.5"`.
- **Text**: `Experimental Features`; `Use at your own risk.`;
  `Experiments are early-access features still under active development. They are provided "as is" without any warranty, and without guarantee of correctness, reliability, or compatibility.`;
  `The Open Resin Alliance and its contributors accept no responsibility or liability for any loss, damage, or data loss arising from the use of experimental features.`;
  `Enable and use them at your own risk.`; `Take me back!`; `I understand`.
- **Callbacks**:
  - `Take me back!`: `variant="secondary"` → `setShowDisclaimer(false); onExit()` — does
    **not** set `disclaimerSeenThisLaunch`.
  - `I understand`: `variant="tinted-warning"` → `disclaimerSeenThisLaunch = true; setShowDisclaimer(false)`.
  - Escape: `useEscapeToClose(true, onExit)` — always registered while the gate renders.
- **State**: shown on mount whenever `!disclaimerSeenThisLaunch`; closing the tab and
  re-entering shows it again until acknowledged.

## Tab body

- **Layout & styling**: root `space-y-3`.
- **Intro card**: `rounded-md border p-3`,
  `borderColor: var(--border-subtle)`,
  `background: color-mix(in srgb, var(--surface-1), transparent 6%)`;
  header `flex items-start gap-2`; icon tile
  `inline-flex h-8 w-8 items-center justify-center rounded-md border shrink-0` with
  `--border-subtle` + `color-mix(in srgb, var(--surface-2), transparent 8%)`, icon
  `FlaskConical h-4 w-4` in `var(--accent-secondary)`; `<h3 className="text-xs font-semibold">`
  `Experiments`; description `mt-0.5 text-xs leading-snug` in `--text-muted`:
  `Early-access features that are still in testing. They may change or disappear without notice.`
- **Empty state** (`experiments.length === 0`): `rounded-md border px-3 py-2 text-xs`,
  `--border-subtle` / `--surface-0` / `--text-muted`, text
  `No experiments are available in this build.`
- **Row**: `rounded-md border px-2.5 py-2 flex items-center justify-between gap-3`,
  `--border-subtle` / `--surface-1`. Left `min-w-0 flex-1`: name
  `text-xs font-semibold` in `--text-strong`; description `mt-0.5 text-xs leading-snug`
  in `--text-muted`. Right: `SegmentedControl label={name} className="shrink-0"
  value={enabled ? 'on' : 'off'} options=[OFF, ON]`.
  The two option labels use `msg({ message: 'OFF' | 'ON', comment: 'Toggle state on a narrow uppercase button in the Experiments tab. Keep it short — the button is 92px wide.' })`.
- **Restart notice**: `rounded-md border px-3 py-2 text-xs` on `--surface-1`,
  `--text-muted`: `Experiment changes apply after you restart DragonFruit or reload the window.`
- **Data**:
  - `enabledState` initialised from the registry; `enabled = enabledState[id] ?? experiment.defaultEnabled`.
  - subscription: `subscribeToExperiments(() => setEnabledState(getInitialEnabledState()))`,
    unsubscribed on unmount.
  - names/descriptions: `translateExperimentName/Description(experiment, _)`; unknown
    ids fall back to the manifest strings.
- **Callbacks**: `SegmentedControl onChange` → `handleToggle(id, next === 'on')` →
  `setExperimentEnabled(id, enabled)` (persists + dispatches).
- **Manifest** (`src/config/experiments.json`, version 1):

  | id | name (manifest) | description (manifest) | defaultEnabled | gatedPlugins |
  |---|---|---|---|---|
  | `chitubox-import` | Chitubox File Import | Enable importing .chitubox project files. Lacks comprehensive testing and may not work with all files. | false | `["chitubox-import"]` |
  | `auto-supports` | Auto Supports | Automatic support generation from model analysis. Early access. | true | — |
  | `auto-rotation` | Auto Orientation | Orientation suggestions that minimize support contact area. Early access. | true | — |

  Translated copy (`experimentMessages.ts`) overrides `chitubox-import`,
  `auto-supports` names and the `chitubox-import`/`auto-supports` descriptions;
  `auto-rotation`'s description has **no** descriptor and uses the manifest string.

## Experiment persistence (behaviour the tab depends on)

- Storage key `dragonfruit-experiments-enabled`; change event `dragonfruit-experiments-changed`.
- Saved values are `true` or `'off-when-default-on'`; a plain `false` is dropped on read.
- `setExperimentEnabled`: matching the default **deletes** the entry; deviating stores
  `true` (opt-in) or the opt-out marker. Source comment: "Matching the manifest
  default: the manifest rules on its own, so drop any saved override. A later
  promotion (default `false` → `true`) then reaches this user instead of being pinned
  by a stale choice."
- The tab shows effective state only; it never reads the override record directly.

## Port target — Experiments

`gpui/src/features/settings/experiments.rs` exists: `experiments_tab` (intro card,
three rows, footer note) and `experiments_overlay` (the gate). `ExperimentsState`
in `state.rs` has `disclaimer_seen`, `show_disclaimer`, `enabled: HashMap<&'static str, bool>`;
`EXPERIMENTS` in `defaults.rs` mirrors the three manifest rows.
Gaps versus the source: no empty state, no translated copy indirection, and
`write_store` persists plain booleans (cleared then only non-default values) rather
than the `off-when-default-on` marker — a promotion would then not reach users who
opted out.

---

# Surface: SceneAutosaveSettingsTab — autosave timing, recovery and status

- **Source**: `src/components/settings/SceneAutosaveSettingsTab.tsx:1-420`.
- **Renders**: three stacked sections — Scene Autosave (four setting rows), Crash
  Recovery (one row), Autosave Status (bridge-backed diagnostics) — plus a status
  message strip.
- **Root**: `<div className="space-y-3">`.

## Section 1 — Scene Autosave

- **Layout & styling**: `rounded-md border p-3`, `--border-subtle` / `--surface-1`.
  Header `flex items-start gap-2.5`; icon tile as usual with `HardDrive h-4 w-4` in
  `var(--accent)`; `<h3 className="text-xs font-semibold">` `Scene Autosave`;
  description `mt-0.5 text-xs leading-snug`:
  `Automatically saves a crash-recovery .voxl scene in the background and offers restore on next launch.`
  Rows container `mt-2 grid gap-2`.
- **Rows** (all `SettingRow bordered`):
  1. label `Enable scene autosave`, description `Creates recovery snapshots while editing.`,
     control `SegmentedControl label="Enable scene autosave"` with `OFF`/`ON`,
     `value={settings.enabled ? 'on' : 'off'}` → `setSettings(prev => ({...prev, enabled: next === 'on'}))`.
  2. label `Autosave idle delay`, description `Wait this long after edits before autosaving.`,
     `disabled={!settings.enabled}`; `NumberInput min={15} max={900} step={5}`
     `value={Math.round(debounceMs / 1000)}`, `className="ui-input h-[34px] w-[120px] pl-2.5 pr-5 py-1.5 text-sm"`,
     `disabled`; unit `<span className="text-xs" style={{ color: 'var(--text-muted)' }}>{_(msg({ message: 'sec', comment: 'Unit suffix after a seconds input box. Keep it abbreviated.' }))}</span>`.
     `onChange`: ignore non-finite; `nextSeconds = clamp(round(next), 15, 900)`;
     `debounceMs = nextSeconds * 1000`; `capMs = max(prev.capMs, nextSeconds * 1000)`.
  3. label `Autosave cooldown`, description `After an autosave, defer further automatic saves until this interval ends.`,
     `value={Math.round(cooldownMs / 1000)}`; same min/max/step/class/disabled;
     `cooldownMs = nextSeconds * 1000`; `capMs = max(prev.capMs, nextSeconds * 1000)`.
  4. label `Maximum autosave interval`, description `For continuous edits, save at least this often.`,
     `value={Math.round(capMs / 60_000)}`, `min={1} max={60} step={1}`, unit `min`;
     `capMs = max(nextMinutes * 60_000, prev.debounceMs, prev.cooldownMs)`.
- **States**: rows 2–4 are disabled (opacity 0.68 on the row from `SettingRow`, controls
  also `disabled`) while `enabled === false`.
- **NumberInput details** (shared component): `type="text"`, `inputMode="decimal"`,
  locale-formatted with up to 2 decimals, a regex guard on typing
  (`sign` omitted when `min >= 0`, `[.,]` separator, ≤2 decimals), wheel stepping via
  `wheelStepDirection`, stepper buttons `absolute inset-y-0 right-0.5 flex w-4 flex-col items-center justify-center gap-0.5`
  each `inline-flex h-3 w-3 items-center justify-center rounded hover:bg-white/10 disabled:opacity-50`
  with `aria-label` `Increase value` / `Decrease value` and `tabIndex={-1}`.
- **Persistence**: `React.useEffect(() => { saveSceneAutosaveSettings(settings); }, [settings])`
  — fires on mount too.

## Section 2 — Crash Recovery

- `rounded-md border p-3` on `--surface-1`; header `flex items-start gap-2`;
  `IconChip size="md" tone="warning" icon={AlertTriangle}`; `<h4 className="text-sm font-semibold">`
  `Crash Recovery`; description `text-xs mt-0.5`:
  `Show restore/discard prompt if a dirty autosave is detected at startup.`
- One `SettingRow bordered className="mt-2"` label `Recovery prompt on startup`,
  no description, `SegmentedControl` `OFF`/`ON` bound to `recoveryPromptEnabled`.

## Section 3 — Autosave Status

- `rounded-md border p-3` on `--surface-1`; header `flex items-start gap-2`. The icon
  tile here is the **shrink** variant
  (`inline-flex h-8 w-8 items-center justify-center rounded-md border shrink-0`) with
  `HardDrive h-4 w-4` in `var(--accent)`; `<h4 className="text-sm font-semibold">`
  `Autosave Status`; description `text-xs mt-0.5`:
  `Desktop-only diagnostics for autosave files and recovery state.`
- **Refresh button**: `Button variant="secondary" size="auto"`,
  `className="!h-8 !px-2.5 !py-0 text-xs inline-flex items-center gap-1.5 disabled:opacity-60 shrink-0"`,
  `disabled={!desktopAvailable || busy !== 'none'}`; content is
  `Spinner size="sm"` while `busy === 'refresh' || loadingStatus`, else
  `RefreshCw h-3.5 w-3.5`; label `Refresh`.
- **Body states** (exclusive, in this order):
  1. `!desktopAvailable` → `mt-2 text-xs` in `--text-muted`:
     `Autosave files are available in the DragonFruit desktop build.`
  2. `loadingStatus` → `mt-2 text-xs inline-flex items-center gap-1.5`, `Spinner size="sm"`,
     `Loading autosave status…`.
  3. Otherwise a `mt-2 grid gap-2 sm:grid-cols-2` of tiles
     (`rounded-md border px-2.5 py-2`, `--border-subtle` / `--surface-0`), each with a
     label `text-[11px] uppercase tracking-wide font-semibold` in `--text-muted` and a
     value `mt-1 text-xs` in `--text-strong`:
     - `Recovery state`: `manifest == null` → `No autosave manifest found`;
       `manifest.clean` → `Clean (no pending recovery)`; else `Recovery available`.
     - `Last autosave timestamp`: `manifest?.savedAt` →
       `new Date(savedAt).toLocaleString(i18n.locale)`; else
       `_(msg({ message: 'Unknown', comment: 'Placeholder when the autosave timestamp could not be read.' }))`.
     - When `manifest?.lastError`: a `sm:col-span-2` tile with literal classes
       `rounded-md border px-2.5 py-2 sm:col-span-2 border-amber-500/40 bg-amber-500/10`;
       label class `text-[11px] uppercase tracking-wide font-semibold text-amber-400`,
       text `Standing Autosave Error`; value `mt-1 text-xs text-amber-200` = `manifest.lastError`.
     - `Autosave scene file` (`sm:col-span-2`): `resolvedAutosavePath ?? _(msg({ message: 'Unavailable', comment: 'Placeholder when the autosave file path could not be resolved.' }))`
       in `mt-1 text-xs break-all`; then a note `mt-1.5 text-xs leading-snug` in `--text-muted`:
       if `resolvedAutosaveOrigin === 'sidecar'` →
       `_(msg({ message: 'A recovery copy is written beside each saved project, as <name>_autosave.voxl. It uses roughly as much disk as the project itself.', comment: '<name> is a literal placeholder in the filename, not markup — leave it as-is.' }))`;
       else `Recovery copies are written to the app data folder above.`; when
       `resolvedFallbackReason` append `' ' + formatFallbackNotice(_, reason)`.
- **Action buttons**: `mt-2 grid gap-2 sm:grid-cols-2`:
  - `Mark Recovery Clean`: `Button variant="tinted-danger" size="auto"`,
    `className="!h-9 !px-3 !py-0 text-sm inline-flex items-center justify-center gap-1.5 disabled:opacity-60"`,
    `disabled={!desktopAvailable || busy !== 'none'}`; `Spinner size="md"` while
    `busy === 'mark-clean'`, else `AlertTriangle h-4 w-4`.
  - `Open Autosave Location`: `Button variant="tinted-accent" size="auto"`, same
    className, `disabled={!desktopAvailable || !resolvedAutosavePath || busy !== 'none'}`;
    `Spinner size="md"` while `busy === 'reveal'`, else `HardDrive h-4 w-4`.
- **Data / resolution**:
  ```ts
  const resolvedAutosavePath = manifest?.voxlPath ?? paths?.voxlPath ?? null;
  const resolvedAutosaveOrigin = manifest?.origin ?? paths?.origin ?? null;
  const resolvedFallbackReason = manifest?.fallbackReason ?? paths?.fallbackReason ?? null;
  ```
  With the source comment: "The manifest wins over the path probe.
  `scene_autosave_get_paths` is called here without a project, so it can only ever
  answer with the generic fallback — while the last tick may well have written a
  sidecar beside the open project. The manifest records the payload that was actually
  committed, so it is the honest answer to \"where is my autosave\"."
- **Types**:
  ```ts
  type AutosavePaths = { voxlPath: string; manifestPath: string; origin: string;
                         projectPath: string | null; fallbackReason: string | null };
  type AutosaveManifest = { savedAt: string; clean: boolean; voxlPath?: string | null;
                            origin?: string | null; projectPath?: string | null;
                            fallbackReason?: string | null; lastError?: string | null };
  ```
- **Commands** (desktop only, all via dynamic `import('@tauri-apps/api/core')`):
  - `scene_autosave_get_paths` + `scene_autosave_read_manifest` in `Promise.all` on
    mount and on Refresh; failure → message error
    `error.message` or `Failed to load autosave status.`
  - Refresh success message `Autosave status refreshed.`
  - Mark clean: `scene_autosave_write_manifest` with
    `{ savedAt: manifest?.savedAt ?? new Date().toISOString(), clean: true, deletePayload: true }`,
    then reload status, success message `Autosave recovery state marked clean.`,
    error `error.message` or `Failed to mark autosave as clean.`
    Source comment: "This is the settings-level explicit discard, so it deletes the
    payload as well as clearing the flag — the same rule as the recovery prompt's
    Discard. A `_autosave.voxl` the user has just declared stale should not stay on
    disk next to their project."
  - Reveal: `reveal_in_file_manager` with `{ path: resolvedAutosavePath }`; error
    `error.message` or `Failed to open autosave location.`
- **Busy state**: `busy: 'none' | 'refresh' | 'mark-clean' | 'reveal'`; each handler
  sets it before and resets to `'none'` in `finally`.
- **desktopAvailable**: `useState(() => isDesktopRuntime())`, where
  `isDesktopRuntime()` is `typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window`.
  Computed once; never re-evaluated.

## Message strip

- Rendered when `message.kind !== 'idle'`:
  `rounded-md border px-3 py-2 text-xs`, `background: var(--surface-1)`,
  `borderColor: message.kind === 'error' ? 'color-mix(in srgb, #ef4444, var(--border-subtle) 40%)' : 'color-mix(in srgb, #22c55e, var(--border-subtle) 40%)'`,
  `color: message.kind === 'error' ? '#fca5a5' : '#86efac'`; text `message.text`.
- **States**: `idle` never renders; `success` green; `error` red. `loadStatus()` clears
  to idle before each attempt.

## `sceneAutosavePreferences.ts` (persistence contract)

- Key `dragonfruit-scene-autosave:settings-v1`; change event
  `dragonfruit://scene-autosave-settings-changed`.
- Defaults: `enabled: true`, `recoveryPromptEnabled: true`, `debounceMs: 45_000`,
  `cooldownMs: 180_000`, `capMs: 5 * 60_000`.
- `normalizeSceneAutosaveSettings`: `debounceMs` and `cooldownMs` = `clamp(round(v), 15_000, 15 * 60_000)`
  (default when not finite); `capMs` = `clamp(round(v), 60_000, 60 * 60_000)` then
  `Math.max(capMs, debounceMs, cooldownMs)`; `enabled`/`recoveryPromptEnabled` default
  to `true` unless exactly `false` (`value?.enabled !== false`).
- `saveSceneAutosaveSettings(partial)` merges over the snapshot, normalises, writes only
  when the serialized value changed, dispatches the change event.
- `getSceneAutosaveSettingsSnapshot()` caches by raw string; SSR returns defaults.
- `subscribeToSceneAutosaveSettings` listens to the custom event and `storage` (only
  for this key).

## Port target — Scene Autosave

`gpui/src/features/settings/autosave.rs` exists: `autosave_tab` renders the four
setting rows (segmented + three `number_with_unit(number_input(…), "sec" | "min")`),
the Crash Recovery section and an Autosave Status section that is permanently in the
"desktop build" state with disabled Refresh / Mark Recovery Clean / Open Autosave
Location. `AutosaveState` in `state.rs` holds `enabled`,
`recovery_prompt_enabled`, `debounce_ms`, `cooldown_ms`, `cap_ms`; `write_store`
writes all five. `rows.rs::number_with_unit` is the unit-suffix wrapper.
Gaps: no host bridge (paths/manifest), no `manifest`-wins resolution, no message
strip — the source's status grid, error tile and action wiring are absent.

---

# Deliberate decisions and gotchas

1. **Hotkeys is the only tab in this area with internal scroll.** `usesInternalTabScrollLayout`
   is true for About, Hotkeys and Updates; the tab supplies
   `h-full min-h-0 flex flex-col` and its own `overflow-y-auto` column. Plugins,
   Experiments and Scene Autosave must *not* add an inner scroller — the modal column
   already scrolls them.
2. **The preset row label is not the binding description.** For `PRESETS` the row uses
   `PINNED_SLOT_LABELS[action]` ("Slot 1"), while the stored `description` is
   "Pinned Slot 1". A porter who renders `binding.description` for every category
   will show the wrong text for all six presets.
3. **Descriptions are app-owned, not user data.** `stripStaleActions` rewrites every
   stored binding's `description` from the current defaults, and the recording path
   passes the current `description` through. Comment: "The key and modifier are the
   user's; the description is the app's own wording for that action, and it is never
   editable in the UI."
4. **`Ctrl` is displayed as the platform primary.** `ctrl` → `Cmd` on macOS, `Ctrl`
   elsewhere; `meta` → `Cmd` only when the primary is `Cmd`, else `Meta`. Do not print
   the stored modifier verbatim.
5. **Hotkey dispatch is suspended for the whole recording session,** restored only in
   the effect cleanup. Without it, Escape closes Settings and every captured letter
   also fires its tool shortcut. This is why the handlers are registered with capture
   `true` and set `__isHotkeySystemInternal`.
6. **Modifier-only bindings commit on key *up*, ordinary keys on key *down*.**
   `pressedNonModifier` latches on the first non-modifier keydown and permanently
   disables the key-up branch for that recording, so `Ctrl+Shift` pressed as a chord
   does not also commit `Shift` on release.
7. **Modifier order is fixed** `ctrl, shift, alt, meta`, joined with `+`. This is the
   on-disk string format (`'ctrl+shift'`) and must match what `parseModifier`/the
   runtime matcher accepts.
8. **Recorded single characters are lower-cased; Space becomes the literal `Space`.**
   `normalizeRecordedKey` returns `'Space'` for `' '` and `key.toLowerCase()` for any
   single character — so a recorded `Z` is stored as `z`, even though the default
   config for the same action may say `Ctrl+Z` (i.e. `z` + `ctrl`).
9. **`toKeyLabel` checks `' '` before the length check.** Comment: "Space first: it is
   one character long, so an upper-case pass would render it as a blank keycap and the
   branch below would never be reached."
10. **`Delete` is a fixed secondary delete key, not a configurable binding.**
    `SECONDARY_DELETE_KEY = 'Delete'`; `GLOBAL.DELETE` defaults to `Backspace`. The
    row renders `Delete` after a `/` separator with tooltip "Always available", and the
    `/` span is `aria-hidden`. `useDeleteHotkey` requires `Delete` with **no**
    modifiers to count.
11. **`resetCategories` resets whole categories, not the card's rows.** The "Global"
    card resets `GLOBAL` and `CAMERA`; the `HotkeyContext` comment still mentions
    `GLOBAL, CAMERA, ROTATION`, but no `ROTATION` category exists in
    `DEFAULT_KEYBINDINGS` — the comment is stale, the code maps the card's own
    categories. Do not add a `ROTATION` category on the strength of the comment.
12. **The Hotkeys tab never shows four whole categories.** `DEFAULT_KEYBINDINGS`
    declares `DEBUG` (10), `NAVIGATION` (4), `HOLE_PUNCH` (1) and `CUT` (1) as well;
    they are persisted and reset never touches them. The GPUI
    `HOTKEY_DEFAULTS` table (33 rows) drops them entirely — if the store is the only
    home for bindings, those four categories would silently vanish.
13. **Section cards are paired, not independent.** `sectionRows` chunks
    `configurableSections` in twos and an odd last section renders against an empty
    `<div />`, so a single remaining card occupies only half the width at `lg`.
14. **Plugins dedupes sideloaded ids out of the profile store.** A sideloaded package
    that contributes presets also registers in the profile store; the source skips
    that entry so there is exactly one row (`sideloaded:<id>`), because the host's
    record is the one that can be removed. Comment quoted at `:624-626`.
15. **Group order is fixed: sideloaded, then github, then builtin,** each group
    alphabetised (`localeCompare`) *before* grouping. Empty groups are dropped **and**
    the hairline separator only appears between surviving groups.
16. **A freshly installed row glows for 3 s and the glow is a fill, not a ring.**
    `highlightInstalledRow` stores `recentlyInstalledKey` and clears it after
    `3_000` ms; a pending timer is cleared on unmount. Comment: "The fill, not just
    the outline: a ring alone read as a focus state." The key format differs per tier:
    `profile:<id>` for GitHub/builtin, `sideloaded:<id>` for packages.
17. **2xx is not enough for a GitHub install.** `runInstallRequest` requires
    `status ∈ [200,299]`, `payload.ok`, and `payload.manifest`. Comment: "a
    parsed-but-rejected manifest also fails the install."
18. **The liability retry re-requests the manifest** with both
    `allowUnverifiedInstall: true` and `acknowledgeLiabilityWarning: true`; the
    unverified flag alone is refused by the resolver.
19. **`trust` is derived from `repoAllowlisted === false`, not from truthiness.**
    A payload without the field counts as allowlisted.
20. **The install preview's `liabilityAcceptedAt` is captured at preview time,** not
    at confirm time, and is only set for unverified installs.
21. **The reload dialog cannot be dismissed.** `onClose` and `onBackdropClick` both
    reload; `closeAriaLabel` reads "Reload DragonFruit now". After 15 s the countdown
    effect reloads on its own. Also note `setReloadRequired(true)` is never reset to
    false in this component — the app reloads.
22. **The countdown fill is dark on purpose.** The tinted-accent button's text colour
    is overridden to `var(--accent-contrast)` and the fill is
    `color-mix(in srgb, var(--button-tint, var(--accent)), #000 62%)`, because "the
    accent text the tinted variant uses is unreadable on a bright accent bar."
23. **Sideloaded preset ids are collected before removal.** The package directory (and
    its manifest) is deleted by `removeSideloadedPlugin`, so
    `markPrinterProfilesMissingPlugin(id, name, presetIds)` is called after with ids
    read from the in-memory manifest.
24. **A wrong passphrase goes to the field, not to the status line.** In
    `handleConfirmSideloadInstall`, `sealed` decides: `setPassphraseError` inside the
    passphrase dialog, otherwise `setStatus` on the card. The install button is
    disabled until `sideloadPassphrase.trim().length > 0`.
25. **The sideload import row is doubly gated**: `isDesktop && allowSideload`. The cog
    that toggles the allowance is gated only by `isDesktop`. `isDesktop` is set in an
    effect from `isTauriRuntime()`, so the first paint is the non-desktop state.
26. **The source's `PluginsSettingsTab` calls `getInstalledPlugins()` during render.**
    There is a long comment at `:616-618`: "Read fresh list on every render; avoids
    stale memo behavior when external store updates don't change snapshot identity but
    still emit notifications." Do not hoist it into a memo.
27. **The empty plugin list renders nothing under the header.** Only the header
    subtitle changes (`# plugins in this profile.`). The GPUI port's hard-coded "No
    plugins installed in this profile." line does not exist in the source.
28. **`isOraHostedRepository` is narrow.** It requires host `github.com` (case-insensitive)
    and a path starting with `/open-resin-alliance/`; the ORA check badge is builtin-only.
29. **`truncateRepositoryUrl` is dead code.** The details dialog uses
    `repositoryLabel` instead. Do not port the unused helper.
30. **The Experiments gate lives at module scope, not in React state.**
    `disclaimerSeenThisLaunch` survives tab unmount/remount within one launch, so
    "Take me back!" only re-shows the gate on the next entry — and "I understand"
    suppresses it for the rest of the launch. A port that stores this in the tab's
    state will show the gate on every visit.
31. **The gate is portalled to `document.body` on purpose.** Its own comment explains
    that the settings panel's entry animation transform would otherwise offset a
    fixed overlay. Any port that paints overlays inside the animated panel inherits
    that bug.
32. **`Take me back!` also navigates away** (`onExit`), and does **not** acknowledge.
    Escape is wired to the same exit via `useEscapeToClose(true, onExit)`; the backdrop
    does the same on `mousedown`, not `click`.
33. **The Experiments tab is the reason the modal has a reload prompt.** Toggling only
    writes storage; `SettingsModal.requestClose` compares the enabled-id snapshot and
    shows "Apply Experiments" instead of closing. The tab's own footer says changes
    apply "after you restart DragonFruit or reload the window."
34. **`enabledState ?? experiment.defaultEnabled` is the display fallback,** but the
    registry already resolves defaults; the `??` only covers an id missing from the
    snapshot.
35. **A saved `false` is not meaningful for experiments.** Only `true` and
    `'off-when-default-on'` are read; `false` means "follow the manifest default".
    Saving the default deletes the key. A plain-boolean port breaks the promotion
    contract documented in `docs/dev/experiments-framework.md`.
36. **Experiment copy for `auto-rotation` is untranslated.** `experimentMessages.ts`
    has no description entry for it, so it falls back to the manifest string
    ("Orientation suggestions that minimize support contact area. Early access.").
37. **Autosave serialises everything in milliseconds but edits it in seconds/minutes.**
    `Math.round` is used both ways; there is no sub-second precision and no sticky
    rounding. The three rows clamp independently (15–900 s, 15–900 s, 1–60 min).
38. **`capMs` is maintained as `>= max(debounceMs, cooldownMs)` in three places:**
    the two seconds rows (`capMs: Math.max(prev.capMs, nextSeconds * 1000)`), the
    minutes row (`max(nextMinutes * 60_000, prev.debounceMs, prev.cooldownMs)`), and
    `normalizeSceneAutosaveSettings`. A port that clamps one place and not the others
    will let the cap fall below the delay.
39. **The autosave settings effect writes on mount.** `saveSceneAutosaveSettings`
    normalises and may rewrite localStorage and dispatch
    `dragonfruit://scene-autosave-settings-changed` just by opening the tab — that is
    how out-of-range stored values get repaired.
40. **The manifest beats the path probe.** `resolvedAutosavePath/Origin/FallbackReason`
    read `manifest.*` first; `scene_autosave_get_paths` is called with no project and
    can only answer with the generic location. Quoted comment in the surface above.
41. **"Mark Recovery Clean" deletes the payload.** It sends
    `{ savedAt: manifest?.savedAt ?? now, clean: true, deletePayload: true }` — it is
    not just a flag clear, and `savedAt` is preserved from the existing manifest when
    there is one.
42. **`desktopAvailable` is captured once** in a `useState` initialiser; a port that
    re-checks the runtime per render will flash different copy.
43. **The status action buttons stay disabled for the whole busy window**
    (`busy !== 'none'`), not just for their own action; the Refresh button is also
    disabled while `loadingStatus`. `Open Autosave Location` additionally requires a
    resolved path.
44. **The refresh spinner condition is `busy === 'refresh' || loadingStatus`,** so the
    initial load also shows the spinner in the button, not only the body.
45. **`lastError` is the only amber surface in this tab** and uses literal Tailwind
    colour classes (`border-amber-500/40 bg-amber-500/10`, `text-amber-400`,
    `text-amber-200`) rather than theme variables — do not "tokenise" it away from the
    source's appearance.
46. **`formatFallbackNotice` is a module-level interpolated message on purpose.**
    Comment: "Interpolated messages live at module scope: React Compiler renames
    interpolated locals inside components, which desyncs the message id from the
    catalog." The reason value is passed through untranslated and the comment in the
    descriptor says so.
47. **The sidecar origin copy contains a literal `<name>` placeholder.** The
    descriptor comment says "<name> is a literal placeholder in the filename, not
    markup — leave it as-is." It must not be treated as a tag by a renderer.
48. **Every Plugins dialog is a separate `StructuredDialogModal` with its own z-index.**
    `z-[130]` for the working dialogs, `z-[131]` for the passphrase step above the
    review sheet, `z-[140]` for details, `z-[200]` for the reload prompt; the
    Experiments gate is `z-[220]`. The order matters: the passphrase dialog opens on
    top of the pinned review dialog.
49. **`StructuredDialogModal` only closes on backdrop `mousedown`,** and prefers
    `onBackdropClick` over `onClose`. Escape is routed through the shared
    `useEscapeToClose` stack, and `closeDisabled` *swallows* Escape rather than
    ignoring it.
50. **`ReviewFact`'s `stacked` flag changes the layout, not just the wrapping**:
    stacked uses `border-b px-3 py-2 last:border-b-0` and a left-aligned
    `mt-1.5 flex flex-wrap items-center gap-1 text-xs` value; unstacked is a
    `flex items-baseline justify-between` row with a right-aligned value. The
    capabilities fact flips it when expanded.
51. **The `SIDELOAD_CAPABILITY_MESSAGES` / `_SHORT_` split is intentional.** The short
    set is "One word each, for the narrow value column in the details modal; the full
    name stays in the tooltip and in the expanded list. Contexts keep these apart from
    verbs like \"import\" elsewhere in the app."
52. **The version pill prints `v` adjacent to a `<Trans>`,** e.g. `<Trans>v{version}</Trans>`
    — the `v` is part of the message, not decoration, so RTL/locales can reorder it.
53. **`Github` / `PackagePlus` icons mark the provenance tier, and a linked name
    replaces the plain name entirely** — there is no separate "open repository"
    button on the row; the name itself is the link (and carries `title={link}`).
54. **The builtin shield colour depends on the app theme** (`data-theme` attribute or
    `prefers-color-scheme`), observed live. Light uses
    `color-mix(in srgb, #4f8a08, var(--text-strong) 28%)`, dark `#86efac`.
55. **`status` is a discriminated pair, not a string:** `{ kind: 'idle'|'success'|'error', message }`.
    Idle renders nothing; the message is raw text (error strings from the host are not
    translated).
56. **The dev placeholder and the studio link are dev-only.** `placeholder={isDevRuntime ? 'df://debug_plugin_official' : _(msg\`https://github.com/owner/repo\`)}`;
    the studio link renders only when `isDevRuntime`, and an effect closes the studio
    if the dev flag is false.

---

# Open questions

1. **Plugin install transport.** `requestPluginManifest` uses
   `fetch_github_plugin_resource` in Tauri and the Next route
   `/api/plugins/github-manifest` on the web. The GPUI app has neither a Next server
   nor (per `gpui-backend-map.md`) a ported allowlist resolver; the manifest resolver
   (`githubPluginManifest.ts`: allowlist parsing, ref normalisation, asset inlining
   budget, debug manifests) has no GPUI counterpart. Where does the port get
   `ok / requiresLiabilityWarning / repoAllowlisted / manifestSha256`?
2. **Sideloaded install and import are deferred upstream.** `sideloaded_plugin_install`
   needs `plugin_sideload_encoder::register_installed_encoders` and
   `sideloaded_plugin_import_file` needs the sidecar runtime; both are marked
   `deferred` in the backend map. Should the port keep the allowance switch and list
   only, or wait for the encoder port?
3. **Where the profile store lives in GPUI.** The source's `profileStore` is a
   localStorage-backed registry with `getInstalledPlugins()`,
   `installPluginFromManifest`, `uninstallPlugin` and
   `markPrinterProfilesMissingPlugin`. The GPUI settings state only has
   `BUILTIN_PLUGINS` (one entry). Is there a `gpui-store` plugin registry planned, and
   where does `markPrinterProfilesMissingPlugin` (also used by the printer-profile
   area) belong?
4. **Hotkey dispatch does not exist in GPUI.** The recording overlay in
   `hotkeys.rs` can only cancel; there is no global key capture and no
   `suspendHotkeyDispatch` equivalent. Should the port wire recording once a hotkey
   store/keymap lands, and does the capture have to run before the app-level key
   handler (the source relies on capture-phase listeners)?
5. **Hidden hotkey categories.** `HOTKEY_DEFAULTS` in `defaults.rs` omits `DEBUG`,
   `NAVIGATION`, `HOLE_PUNCH` and `CUT`. Is that deliberate scope, and if the store is
   the only persistence layer, should those categories be carried anyway so they
   survive a settings round-trip?
6. **Experiments persistence shape.** GPUI stores `HashMap<String, bool>`; the source's
   promotion contract needs `true | 'off-when-default-on'`. Does `gpui_store` need a
   three-state enum, or does the port accept that a promotion re-enables opt-outs?
7. **`disclaimerSeenThisLaunch` lifetime.** It is process-lifetime in the source. In
   GPUI it is `ExperimentsState::disclaimer_seen`, which is part of the settings draft
   and therefore persists if the draft is saved. Should it be reset on every
   `hydrate`, or moved out of the persisted snapshot?
8. **Autosave status bridge.** The commands exist in `gpui-backend`
   (`autosave::scene_autosave_get_paths`, `read_manifest`, `write_manifest`,
   `resolve_recovery`, plus `AppContext::data_dir`). The tab is not wired to them.
   Does the GPUI settings entity have access to a `gpui-backend` autosave service, or
   does the status section stay inert until the autosave driver is ported?
9. **i18n.** Every string here is Lingui `msg`/`Trans` (with the interpolated ones at
   module scope). The GPUI slice's strings are hard-coded English and `gpui-locale`
   exists. Which catalog keys does the port use — the source msgids verbatim, or
   extracted slugs?
10. **`NumberInput` editing.** The GPUI port renders a non-editable number box
    (module note: "typing (and wheel stepping) needs a text-input entity this slice
    does not have"). The autosave rows depend on typing and stepper clicks. Is the
    text-input entity in scope for this area, or should the rows ship read-only?
11. **Clipboard.** The details dialog's "Copy signing key" and its 2 s "Copied"
    feedback need a clipboard; the About tab in GPUI could not reach one. Same
    question here.
12. **`process.env.NODE_ENV`.** `isDevRuntime` gates the studio link and the debug
    placeholder. What is the GPUI build's equivalent debug flag?
