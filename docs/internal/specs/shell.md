# Shell, mode routing, overlay order and hotkeys

Area owner: the window frame and the global input layer. Everything the user
sees that is not inside the 3D viewport, the panel bodies, or a feature modal:
the app bar, the floating panel stack, the empty-scene card, the first-run
wizard, the per-mode panel routing, the paint-order map, and every keyboard
binding.

Read this with `docs/internal/gpui-port-handoff.md` (decisions 1-7) open.

## Components in scope
| Component | File |
|---|---|
| Home (the whole app page) | `src/app/page.tsx` (12126 lines) |
| RootLayout (providers, html/body) | `src/app/layout.tsx` |
| EditorLayout (shell div) | `src/components/templates/EditorLayout.tsx` |
| TopBar | `src/components/layout/TopBar.tsx` (1242) |
| TopBar interpolated strings | `src/components/layout/topBarMessages.ts` |
| FloatingPanelStack (+ hooks, item) | `src/components/layout/FloatingPanelStack.tsx` (1051) |
| Floating layout storage | `src/components/layout/floatingLayoutPreferences.ts` |
| EmptySceneState | `src/components/layout/EmptySceneState.tsx` (774) |
| FirstRunOnboarding | `src/components/layout/FirstRunOnboarding.tsx` (338) |
| OnboardingPrinterLibrary | `src/components/layout/OnboardingPrinterLibrary.tsx` (429) |
| App bar / panel / shell CSS | `src/app/globals.css` (`.ui-topbar*`, `.ui-shell`, `.ui-empty-state`, `.ui-onboarding-*`) |
| Hotkey config (binding table) | `src/hotkeys/hotkeyConfig.ts` (330) |
| Hotkey store (state + resolution) | `src/hotkeys/hotkeyStore.ts` |
| Hotkey provider (persisted config) | `src/hotkeys/HotkeyContext.tsx` |
| Hotkey registry manager (listeners) | `src/hotkeys/HotkeyRegistryManager.tsx` |
| Hotkey → keycap labels | `src/hotkeys/hotkeyLabels.ts` |
| Escape → top-most dialog | `src/hotkeys/useEscapeToClose.ts` |
| Rising-edge hooks | `useUndoRedoHotkeys.ts`, `usePresetHotkeys.ts`, `usePrepareTransformHotkeys.ts`, `useCameraProjectionHotkey.ts`, `useCameraFocusHotkey.ts`, `useInteriorViewHotkey.ts`, `useOrganicCutHotkeys.ts` (all `src/hotkeys/`) |
| Delete dispatch + registry | `src/features/delete/useDeleteHotkey.ts`, `src/features/delete/deleteRegistry.ts` |
| Test that pins the dock behaviour | `src/components/layout/__tests__/panelDock.dom-test.tsx` |

Deliberately *not* specified here (name-and-move-on): `PreparePanelStack`,
`AnalysisPanelStack`, `ExportPanelStack`, `PrintingPanelStack`,
`SharedPanelStack` panel bodies (`src/components/organisms/panels/`), the
`SettingsModal` / `ProfileSettingsModal` internals, `PrintingModals`,
`SceneFileModals`, `ModifierModals`, `MeshRepairModals`, `DiagnosticsModals`,
`NotificationStack` toast bodies, `ToolRail` internals
(`src/components/controls/ToolRail.tsx` + `toolRailEntries.ts`), and
`SceneOverlays` (three.js children). The shell *calls* all of them; their own
specs own their contents.

---

## Surface: RootLayout — providers and document frame
- **Source**: `src/app/layout.tsx:12-70`
- **Renders**: `<html lang="en" suppressHydrationWarning>` → `<body>` → `I18nClientProvider` → `HotkeyProvider` → seven sibling chrome components → children.
- **Layout & styling**: `html` and `body` both carry the inline style
  `background: 'var(--background, #0b0f14)'`, `color: 'var(--foreground, #e6ebf2)'`.
  `body` also has `className={`${geistSans.variable} ${geistMono.variable} antialiased`}`,
  where the two fonts are `Geist({ variable: '--font-geist-sans', subsets: ['latin'] })`
  and `Geist_Mono({ variable: '--font-geist-mono', subsets: ['latin'] })`.
  `metadata = { title: "DragonFruit", description: "DragonFruit by Open Resin Alliance" }`,
  wrapped in `// i18n-ignore-start: static HTML metadata, rendered before any locale is known; product and org names`
  / `// i18n-ignore-end`.
- **States**: none — the tree is static. Both `html` and `body` are
  `suppressHydrationWarning` because the theme attribute is written client-side.
- **Data**: no props except `children`.
- **Callbacks**: none.
- **Text**: none beyond the metadata above.
- **Port target**: `gpui/src/main.rs` (app/window creation) and
  `gpui/src/app.rs`. There is no provider tree in the port; the equivalents are
  `gpui_store` (settings/session), `gpui_locale::t!`, and the theme in
  `gpui_core::theme`. The seven chrome siblings have these jobs:
  | sibling | job | port status |
  |---|---|---|
  | `HotkeyRegistryManager` | installs the global key listeners (see the hotkeys section) | **missing** — `app.rs` matches keys inline |
  | `ExperimentsNativeSync` | pushes localStorage experiment flags to Tauri | not ported |
  | `AppLogger` | log sink | not ported |
  | `RendererCrashDiagnostics` | WebGL context-loss reporting | N/A (native renderer) |
  | `DevIndicatorPosition` | Next.js dev-indicator offset | N/A |
  | `ScrollLockedInputs` | wheel-does-not-scroll-number-inputs guard | partially in `gpui_ui::number_field` |
  | `I18nClientProvider` | Lingui locale provider | `gpui_locale` |

## Surface: EditorLayout — the shell div
- **Source**: `src/components/templates/EditorLayout.tsx:14-20`
- **Renders**: one `<div>` holding every child as an absolute/overlay sibling.
  ```tsx
  <div className="ui-shell relative h-screen w-screen overflow-hidden" data-no-window-drag="true">
    {children}
  </div>
  ```
- **Layout & styling**: `relative h-screen w-screen overflow-hidden`; `.ui-shell`
  (`src/app/globals.css:439-442`) adds `background: var(--background)` and
  `color: var(--foreground)`. `.ui-shell` is **unlayered CSS**, so it beats
  Tailwind utilities in the same element; nothing here conflicts.
- **States**: none.
- **Data**: `children` only.
- **Callbacks**: none. `data-no-window-drag="true"` opts the whole subtree out of
  the app-bar window-drag gesture (see the TopBar surface) — the panel stack's
  own opt-out is a *different* attribute, `data-no-drag` (see gotcha 40).
- **Port target**: `gpui/src/app.rs` `impl Render for DragonFruit::render` — the
  root `div()` at `app.rs:1699` is the shell. It has no `.ui-shell` fill; the
  window background is set where the window is created in `main.rs`.

## Surface: TopBar — the frameless title bar and workflow stepper
- **Source**: `src/components/layout/TopBar.tsx:704-1242` (render), state/effects
  `:160-703`. CSS: `globals.css:158-312` (`.ui-topbar`, `.ui-topbar::before`,
  `.ui-topbar-blur`, `.ui-topbar-transparent`, `.ui-topbar-blur-transparent`,
  `.ui-topbar-monitor-active/paused`).
- **Renders**: a fragment of two siblings:
  1. `<div className={`ui-topbar-blur ${hideWorkflowControls ? 'ui-topbar-blur-transparent' : ''}`} aria-hidden="true" />`
  2. `<div className={`ui-topbar fixed top-0 left-0 right-0 z-50 flex items-center relative ${hideWorkflowControls ? 'justify-between ui-topbar-transparent' : ''}`} onMouseDownCapture={handleTopBarPointerDown}>`
     containing, left to right: the left cluster (logo button, divider, printer
     badge, monitor button), the app menu (a `ContextMenu` rendered as a child at
     the button's position), two hidden `<input type="file">`, the printer quick
     menu (when open), the centred workflow stepper, the right cluster
     (`ViewTypeDropdown`, overhang toggle, settings button, native window
     controls), and finally the re-slice warning dialog, `SettingsModal` and
     `ProfileSettingsModal`.
- **Layout & styling** (quoted):
  - `.ui-topbar`: `position: relative; isolation: isolate; height: var(--topbar-height); background: transparent; padding: 3px 4px 9px 4px; gap: var(--space-3);`.
    `--topbar-height: 52px` (`globals.css:61`).
  - `.ui-topbar::before` is the frosted surface: `position: absolute; inset: 0; z-index: -1; pointer-events: none;` with two stacked
    `linear-gradient`s — a horizontal
    `color-mix(in srgb, var(--topbar-accent), transparent 93%) 0% → 97% → 94%`
    tint and a vertical
    `color-mix(in srgb, var(--surface-0), transparent 6%) 0% → 10% 50% → 24% 100%`
    ramp — plus `backdrop-filter: blur(24px)` and a bottom-edge `mask-image`
    fade (`1 0% … 1 86% … .52 92% … .12 97% … transparent 100%`).
  - `.ui-topbar-blur`: `position: fixed; top:0; left:0; right:0; height: var(--topbar-height); z-index: 30; pointer-events: none;`
    with `backdrop-filter: blur(24px)` and its own, longer mask fade
    (`1 60% → .82 75% → .44 88% → .14 96% → transparent`).
    Source comment, `globals.css:205-211`, verbatim:
    `/* Backdrop blur layer behind the topbar (z-30) so the Transform Toolbar (z-40) and modals (z-50+) float crisp above the frosted glass. */`
  - `.ui-topbar::before, .ui-topbar-blur { transition: opacity 150ms ease; }`;
    `.ui-topbar-transparent::before` and `.ui-topbar-blur-transparent` set
    `opacity: 0; backdrop-filter: none;`.
  - Left cluster: `flex flex-1 max-w-[430px] items-center gap-2.5 pl-0 pr-4 py-1.5 transition-opacity`, plus
    `opacity-45 pointer-events-none` when `topbarActionsDisabled`; carries
    `data-no-window-drag="false"` and `aria-disabled`.
  - Logo button: `inline-flex h-10 w-10 items-center justify-center rounded-md transition-colors`, inline
    `background: isAppMenuOpen ? 'color-mix(in srgb, var(--accent), transparent 80%)' : 'transparent'`,
    `data-no-window-drag="true"`. The image is
    `src="/dragonfruit_assets/branding/simple_icon.svg"`, `className="h-7 w-7 object-contain"`,
    and in light theme gets `filter: 'drop-shadow(0 1px 3px rgba(0,0,0,0.35))'`.
  - Divider: `className="h-6 w-px mx-0.5 shrink-0"`, `style={{ background: 'color-mix(in srgb, var(--border-subtle), transparent 24%)' }}`.
  - Printer badge button: `group inline-flex h-10 max-w-[300px] items-center gap-2 rounded-md px-2 transition-colors`,
    `background: 'transparent'`. Thumbnail well:
    `inline-flex h-8 w-8 items-center justify-center overflow-hidden rounded-sm shrink-0`, background
    `'color-mix(in srgb, var(--surface-1), transparent 6%)'`, and
    `display: 'none'` when `hideBadges`. Falls back to `<Printer className="h-4 w-4" style={{ color: 'var(--text-muted)' }} />`.
    Label stack `min-w-0 flex flex-col items-start leading-none gap-[2px]`:
    top line `text-[11px] uppercase tracking-[0.11em]` (muted) — or
    `truncate text-[11px] tracking-[0.01em]` in fleet order; bottom line
    `truncate text-[12px] font-semibold` in `var(--text-strong)`. Chevron:
    `h-3.5 w-3.5 ml-auto shrink-0 transition-transform`, `rotate-180` when the
    quick menu is open, colour `color-mix(in srgb, var(--text-muted), white 8%)`.
  - Monitor button: `group inline-flex h-10 items-center gap-1.5 rounded-md px-2 transition-colors`, tone
    `monitorButtonTone = monitorButtonOffline ? '#f87171' : 'var(--text-strong)'`, icon
    `<Activity>` + `text-[11px] font-semibold`, both carrying
    `ui-topbar-monitor-paused` / `ui-topbar-monitor-active` when relevant.
  - Workflow step button:
    `group relative flex cursor-pointer items-center gap-1.5 rounded-[5.5px] border px-1.5 py-1.5 transition-all duration-200 flex-1 min-w-[90px] max-w-[190px] h-[32px]`
    + active `shadow-[0_6px_16px_rgba(0,0,0,0.25)]` / idle `hover:-translate-y-[1px] hover:shadow-[0_6px_14px_rgba(0,0,0,0.18)]`
    + `opacity-45` when `visuallyDimmed` + `cursor-not-allowed hover:translate-y-0 hover:shadow-none` when disabled
    + `grayscale saturate-0` when `printingLocked`.
    Inline style, active: `{ borderColor: 'color-mix(in srgb, var(--accent), white 8%)', background: 'color-mix(in srgb, var(--accent), var(--surface-0) 84%)' }`;
    otherwise `borderColor` is `color-mix(in srgb, var(--border-subtle), black 30%)` when printing-locked
    else `var(--border-subtle)`, and `background` is
    `color-mix(in srgb, var(--surface-2), black 12%)` when printing-locked else
    `color-mix(in srgb, var(--surface-1), transparent 4%)`.
    Icon chip: `inline-flex h-5 w-5 items-center justify-center rounded-full text-[11px] font-bold`,
    active `{ color: 'var(--accent)', background: 'transparent' }`, idle
    `{ color: 'var(--text-muted)', background: 'var(--surface-2)' }`,
    `display: 'none'` when `hideBadges`. Label:
    `absolute left-1/2 -translate-x-1/2 whitespace-nowrap text-center text-xs font-bold leading-none tracking-[0.01em]`,
    colour `active ? 'var(--text-strong)' : 'var(--text-strong)'` (identical both
    ways — see gotcha 8). Lock icon on the Printing step:
    `h-3 w-3 ml-auto flex-shrink-0`, `color: var(--text-muted)`, only when
    `printingLocked && !hideBadges`.
  - Right cluster: `flex flex-1 max-w-[430px] items-center justify-end gap-2 pr-[3px]`; the inner
    button group is `flex items-center gap-2 transition-opacity` plus
    `opacity-45 pointer-events-none` while busy. `ViewTypeDropdown` gets
    `iconOnly={!showViewModeLabel}` and `title="View mode"`. Overhang toggle is
    a `<Button variant={overhangViewActive ? 'primary' : 'secondary'} className="!p-2">`
    with `<AlertDiamondIcon className="w-4 h-4" />`. Settings is
    `<Button variant="secondary" className="!p-2">` with `<Settings className="w-4 h-4" />`.
  - Native window controls (`isDesktopWindow` only): `ml-1 flex items-center gap-1`, three
    `inline-flex h-8 w-8 items-center justify-center rounded-[var(--radius-sm)] border transition-colors`
    buttons. Dark theme colours — minimize `#f4bf4f`
    (`borderColor: color-mix(...,55%)`, `background: color-mix(...,86%)`,
    `color: color-mix(...,16%)`), maximize `#40c463` (same three mixes),
    close `#ff6b6b` (`...55%`, `...88%`, `...18%`). Light theme swaps to
    `#c8920a` / `#1a7a3a` / `#c0160a` with `...35%`, `...50%` mixes and
    `color: 'var(--text-strong)'`.
  - App menu position: `{ x: rect.left, y: rect.bottom + 6 }` of the logo button.
  - Printer quick menu: `fixed z-[121] w-[280px] rounded-lg border p-1.5 shadow-xl backdrop-blur-sm`,
  `borderColor: 'var(--border-subtle)'`, `background: 'color-mix(in srgb, var(--surface-0), #000 10%)'`,
  `left/top` = `{ rect.left, rect.bottom + 6 }` of the badge. Heading row
  `mb-1 px-2 py-1 text-[10px] font-semibold uppercase tracking-wide` (muted).
  List `max-h-[260px] overflow-y-auto custom-scrollbar space-y-0.5`. Row:
  `flex w-full items-center gap-2 rounded-md border px-2 py-1.5 text-left text-[12px] transition-colors`,
  offline `{ borderColor: color-mix(border-subtle, black 18%), background: color-mix(surface-1, black 8%), opacity: 0.55 }`,
  active `{ borderColor: color-mix(accent, border-subtle 30%), background: color-mix(accent, surface-1 89%), color: var(--text-strong) }`,
  otherwise `{ borderColor: var(--border-subtle), background: var(--surface-1), color: var(--text-muted) }`.
  Active badge: `text-[10px] rounded-full border px-1.5 py-0.5` with
  `accent-secondary` mixes. Footer row separated by `mt-1 border-t pt-1`.
- **States** — every branch:
  - `topbarActionsDisabled = isSlicingBusy`: dims the left cluster, the stepper
    wrapper and the right button group to `opacity-45`; disables every button;
    step tooltip becomes `Slicing in progress. Topbar actions are temporarily disabled.`
  - `hideWorkflowControls` (true while `onboardingMounted && wizardActive`):
    the divider, printer badge and the whole stepper are **not rendered**
    (`{!hideWorkflowControls && …}`), `ViewTypeDropdown` + overhang toggle are
    hidden, and the bar becomes `justify-between ui-topbar-transparent`
    (+ the blur layer gets `ui-topbar-blur-transparent`).
  - Step `locked` per step (see Data). Locked → `disabled` + `aria-disabled`,
    `opacity-45`, tooltip `Load a model in Prepare to unlock this stage`, or for
    Printing `Run slicing in Export to unlock Printing preview`.
  - `printingLocked = item.mode === 'printing' && locked` additionally adds
    `grayscale saturate-0`, the darker border/background mixes and the lock icon.
  - Step `active = mode === item.mode`: accent border/background + drop shadow.
  - Printer badge: `hasTopbarFleetUnits` (more than one unit with an IP) →
    click toggles the quick menu; otherwise click opens
    `requestOpenProfileSettings('printer')`, which — while
    `warnBeforeProfileSettingsOpen` — opens the re-slice warning first.
  - Printer label order: `topbarUsesFleetLabelOrder = networkFleet.length > 1`
    switches the two-line label from `Printer` / profile name to
    profile name / active printer name.
  - Thumbnail failure: `printerThumbnailFailed` → `activePrinterThumbnailSrc`
    becomes `undefined` → the `<Printer>` icon is shown.
  - Badges hidden when the stepper is narrower than 4×120px, from a
    `ResizeObserver` on the stepper container: `perButton = (entry.contentRect.width - 3 * 4) / 4`,
    `setHideBadges(perButton < 120)`, and the view-mode label uses hysteresis —
    `perButton >= 155` to appear, `>= 130` to stay:
    `setShowViewModeLabel((previous) => (previous ? perButton >= 130 : perButton >= 155));`
  - `isLightTheme` toggles the logo drop-shadow and the window-control palette;
    it tracks `document.documentElement[data-theme]` through a `MutationObserver`
    plus the `prefers-color-scheme` media query.
  - `isDesktopWindow` is detected once by protocol (`tauri:`, `file:`), hostname
    `tauri.localhost`, or presence of `window.__TAURI_INTERNALS__`; window
    controls render only when true.
- **Data**: props (`TopBarProps`, `:52-146`) — style knobs passed straight to
  `SettingsModal` (`meshColor` … `slicingThumbnailRenderSettings`), `mode`,
  `hasModels`, `hasPrintingData`, `viewType`, `overhangViewActive`,
  `heatmapColors`, `isSlicingBusy`, the scene callbacks (`onNewScene`,
  `onSaveScene`, `onSaveSceneAs`, `onOpenScene`, `onLoadMeshChange`,
  `onImportSceneChange`, `onCloseProgram`), monitor props
  (`showMonitorButton`, `monitorButtonActive`, `monitorButtonPaused`,
  `monitorButtonOffline`, `printerReachabilityByDeviceId`, `onOpenMonitor`),
  `warnBeforeProfileSettingsOpen`, `hideWorkflowControls`.
  Local state (`:160-190`): `isSettingsOpen`, `settingsInitialTab` (default
  `'general'`), `isProfileModalOpen`, `profileModalTab` (`'printer'`),
  `profileModalOpenPrinterLibraryToken`,
  `profileModalOpenNetworkSettingsToken`,
  `profileModalOpenMaterialAntiAliasingToken`,
  `showProfileChangeWarning`, `isDesktopWindow`,
  `isDesktopWindowMaximized`, `isLightTheme`, `printerThumbnailFailed`,
  `isAppMenuOpen`, `appMenuPosition`, `isPrinterQuickMenuOpen`,
  `printerQuickMenuPosition`. Store reads: `subscribeToProfileStore` via
  `useSyncExternalStore` → `getActivePrinterProfile`.
  Effects: apply saved theme + `applyThemeCustomColors(getSavedThemeCustomColors())`
  on mount and observe `data-theme`; `hydrateProfilesFromStorage()` once;
  listen for `OPEN_PROFILE_SETTINGS_MODAL_EVENT` and `OPEN_SETTINGS_MODAL_EVENT`
  (see Callbacks); listen for `mousedown` + `app-hotkey-keydown` while the
  printer quick menu is open.
- **Callbacks**:
  | element | handler | result |
  |---|---|---|
  | logo button | toggles `isAppMenuOpen` | opens/closes the `ContextMenu` at `{left, bottom+6}` |
  | root `onMouseDownCapture` | `handleTopBarPointerDown` | `getCurrentWindow().startDragging()` — see the guardrail below |
  | printer badge | fleet? toggle quick menu : `requestOpenProfileSettings('printer')` | opens the fleet popover, or the profile modal (or the re-slice warning) |
  | monitor button | `onOpenMonitor?.()` | `setPrintingMonitorModalOpen(true)` in `page.tsx` |
  | workflow step | `onModeChange(item.mode)` (if not disabled) | `handleModeChange` in `page.tsx` — see Mode routing |
  | `ViewTypeDropdown` | `onViewTypeChange` | `scene.setShaderType` |
  | overhang toggle | `onOverhangViewChange(!overhangViewActive)` | `toggleOverhangView` |
  | settings button | `setIsSettingsOpen(true)` | renders `SettingsModal` |
  | window buttons | `handleDesktopWindowMinimize` / `handleDesktopWindowToggleMaximize` / `handleDesktopWindowClose` | `@tauri-apps/api/window` calls, dynamic import, failures swallowed |
  | app menu rows | `handleAppMenuSelect(id)` | see the table in Text |
  | quick-menu rows | `handleSelectFleetUnitFromQuickMenu(device.id)` | `selectPrinterNetworkDevice(profileId, deviceId)`, menu closes |
  | quick-menu footer | `handleOpenPrinterManagerFromQuickMenu` | closes menu, `requestOpenProfileSettings('printer')` |
  | re-slice warning | `Continue` → `openProfileSettings(profileModalTab)`; `Keep current profiles` → close | opens `ProfileSettingsModal` |
  | `SettingsModal` onClose | `setIsSettingsOpen(false); setSettingsInitialTab('general')` | reset to the general tab |
  | `ProfileSettingsModal` onClose | `dispatchProfileSettingsModalOpenChange(false)` | notifies listeners |
- **Text** (verbatim, msgids where `msg`/`msg({…})` is used):

  Workflow steps (`steps`, `:580-618`):
  | mode | label | hint (tooltip when unlocked) | locked when |
  |---|---|---|---|
  | `prepare` | `Prepare` | `Arrange model and transforms` | never |
  | `support` | `Support` | `Build and tune supports` | `!hasModels` |
  | `export` | `Export` | `Finalize and export output` | `!hasModels` |
  | `printing` | `Printing` | `Inspect sliced layers before printing` | `!hasModels \|\| !hasPrintingData` |

  Step icons: `FolderOpen`, `Anchor`, `Upload`, `Printer`, all
  `className="h-3 w-3" strokeWidth={2.5}`.

  App menu (`appMenuEntries`, `:620-673`) — rendered by `ContextMenu`, so the
  row chrome is the menu's:
  | id | label | icon | disabled when | `startsGroup` |
  |---|---|---|---|---|
  | `new-scene` | `New scene` | `FilePlus2` | busy, no `onNewScene`, or `!hasModels` | — |
  | `save-scene` | `Save scene` | `Save` | busy or no `onSaveScene` | `true` |
  | `save-scene-as` | `Save scene as…` | `SaveAll` | busy or no `onSaveSceneAs` | — |
  | `open-scene` | `Open scene…` | `FolderOpen` | busy or no `onOpenScene` | — |
  | `import-mesh` | `Import mesh…` | `Upload` | busy or no `onLoadMeshChange` | `true` |
  | `import-scene` | `Import scene…` | `FolderInput` | busy or no `onImportSceneChange` | — |
  | `close-program` | `Close program` | `Power` | never | `true` |

  `import-mesh` clicks `document.getElementById('topbar-mesh-input')`;
  `import-scene` clicks `#topbar-scene-input`. Menu `title` is the literal
  `"DragonFruit"` (i18n-ignore: product name); `ariaLabel` is
  `msg({ message: 'DragonFruit app menu', comment: '"DragonFruit" is the product name and should stay untranslated/unchanged.' })`.

  Other strings:
  | context | source | text |
  |---|---|---|
  | logo `title` | `msg({message:'DragonFruit menu', comment:'"DragonFruit" is the product name and should stay untranslated/unchanged; only "menu" needs translating.'})` | `DragonFruit menu` |
  | logo `aria-label` | `msg({message:'Open DragonFruit menu', …})` | `Open DragonFruit menu` |
  | printer label top (fleet) | `msg\`Select profile\`` | `Select profile` |
  | printer label top (single) | `msg({message:'Printer', comment:'Static field-label shown above the printer name in the topbar badge (like "Printer: <name>"), not an instruction. …'})` | `Printer` |
  | printer label bottom (fleet) | `msg\`No active printer\`` | `No active printer` |
  | printer label bottom (single) | `msg\`Select printer\`` | `Select printer` |
  | printer `title` (fleet) | `formatTopbarPrinterTitle` → `Printer profile: ${profileName} • Active printer: ${printerName}` | as shown |
  | printer `title` (single) | `msg\`Printer profile: ${name}\`` / `msg\`Select printer profile\`` | — |
  | printer `aria-label` (fleet) | `formatTopbarPrinterAriaLabel` → `Printer profile ${profileName}, active printer ${printerName}` | — |
  | printer thumbnail `alt` | `msg\`Selected printer\`` | `Selected printer` |
  | monitor label | offline `msg\`Offline\`` else `msg({message:'Monitor', comment:'Topbar button that opens the live print-progress monitoring view …'})` | `Offline` / `Monitor` |
  | monitor `title`/`aria-label` | `msg\`Selected printer is offline\`` / `msg\`Open printer monitor\`` | — |
  | quick menu `aria-label` | `msg({message:'Fleet quick switch', comment:'"Fleet" means the set of physical network printers discovered under the current printer profile, not a literal fleet of vehicles.'})` | `Fleet quick switch` |
  | quick menu heading | `msg({message:'Fleet units', comment:'Section heading above the list of network printers belonging to the current profile. "Units" = individual printers.'})` | `Fleet units` |
  | unit name fallback | `msg({message:`Printer ${device.id}`, …})` | `Printer {id}` |
  | unit row second line | `msg\`No IP\`` / `msg\`Offline\`` / `msg\`Online\`` joined by ` • ` | — |
  | active badge | `msg({message:'Active', comment:'Badge next to the currently-selected printer in the fleet list …'})` | `Active` |
  | quick menu footer | `msg({message:'Show manager', comment:'Opens the printer manager screen (printer profile settings), reached from this quick-switch popover. "Manager" refers to that screen.'})` | `Show manager` |
  | view mode dropdown title | `msg\`View mode\`` | `View mode` |
  | overhang toggle title/aria | `msg({message:'Overhangs view: On', comment:'Tooltip for a toggle button showing its current state, format "Feature name: state". …'})` / `msg\`Overhangs view: Off\`` | — |
  | settings title/aria | `msg\`Settings\`` | `Settings` |
  | window controls group `aria-label` | `msg\`Window controls\`` | `Window controls` |
  | minimize / maximize / restore / close titles+arias | `msg\`Minimize\``, `msg\`Minimize window\``, `msg\`Restore\``, `msg\`Restore window\``, `msg\`Maximize\``, `msg\`Maximize window\``, `msg\`Close\``, `msg\`Close window\`` | — |
  | re-slice warning `ariaLabel` | `msg\`Changing printer profile requires re-slice\`` | — |
  | re-slice warning title | `msg({message:'Re-slice required after profile change', comment:'"Slice"/"re-slice" is the 3D-printing step …'})` | `Re-slice required after profile change` |
  | re-slice warning subtitle | `msg\`Changing print settings invalidates the current sliced file.\`` | — |
  | warning body | `msg\`You can continue to adjust profiles, but you’ll be prompted to re-slice before printing with the updated settings.\`` | (note the curly apostrophe) |
  | warning buttons | `msg({message:'Keep current profiles', comment:'Cancels the pending profile change …'})`, `msg({message:'Continue', comment:'Confirms proceeding with the profile change despite the re-slice warning above …'})` | `Keep current profiles` / `Continue` |
  | warning close aria | `msg\`Close warning\`` | — |

  Hidden inputs: `#topbar-mesh-input` `accept=".stl,.obj,.3mf,.zip"` `multiple`;
  `#topbar-scene-input` `accept={sceneFileAccept}` (plugin-derived scene
  extensions) — **no** `multiple`. Both `className="hidden"`.

  `topBarMessages.ts` module comment (quote, it explains *why* the strings live
  outside the component):
  > Module level on purpose: React Compiler renames locals inside components
  > (`profileName` -> `profileName_0`) before the Lingui macro derives the message
  > id, so an interpolation written inside `TopBar` gets an id the compiled
  > catalogue does not contain and falls back to the raw source string in
  > production builds.

- **Port target**: `gpui/src/chrome/app_bar.rs` (`app_bar`, `AppBarState`,
  `PrinterBadge`, `FleetUnit`, `MonitorButton`, `MESH_SHADER_OPTIONS`). It is
  already a close port — re-read its header, which lists 22 numbered PORT
  DEVIATIONS; the ones that matter for parity are: no backdrop blur/mask (1, 2),
  no transitions (3), chevron swaps instead of rotating (4), no hover lift (5),
  no letter-spacing (6), no `grayscale` filter (7), monitor colours pinned to the
  animation's 0% frames (8), menus never flip upward (9), no native tooltips
  (10), `hideWorkflowControls` not modelled (12), dark-only window-control tints
  (13), `showViewModeLabel` uses only the 130px steady-state threshold (14), the
  printer badge always renders the empty-store label (15), `hasPrintingData` is
  always false so Printing stays locked (16). The re-slice warning has no
  counterpart (22).

## Surface: FloatingPanelStack — the docked/floating panel container
- **Source**: `src/components/layout/FloatingPanelStack.tsx:1-1051`; storage in
  `src/components/layout/floatingLayoutPreferences.ts`.
- **Renders**: a `FloatingPanelStackContext.Provider` around
  `<div ref className="absolute right-0 top-[var(--topbar-height)] bottom-0 z-10 pointer-events-none" style={{ left: 0 }}>`
  holding, in order: the left dock (`data-dock="left"`), the right dock
  (`data-dock="right"`), the floating panels, the drop-preview box (while
  dragging), and one `ContextMenu` for the panel right-click menu.
- **Layout & styling**:
  - root: `absolute right-0 top-[var(--topbar-height)] bottom-0 z-10 pointer-events-none`, `style={{ left: 0 }}`.
  - each dock: `absolute top-0 bottom-0 flex flex-col pointer-events-none` with
    inline `{ left: leftInsetPx | right: rightInsetPx, width: leftWidth|rightWidth,
    padding: PANEL_MARGIN, gap: PANEL_GAP, alignItems: 'flex-start'|'flex-end',
    overflowY: drag ? 'visible' : 'auto', overflowX: drag ? 'visible' : 'hidden' }`.
  - dock edge handle: `absolute inset-y-0 cursor-col-resize pointer-events-auto`, `{ right: 0, width: 6 }` (left dock) / `{ left: 0, width: 6 }` (right dock).
  - panel item (`FloatingPanelItem`): `className="pointer-events-auto"` with inline
    ```js
    display: hidden ? 'none' : undefined,
    position: docked ? 'relative' : 'absolute',
    left: docked ? undefined : position?.x ?? 0,
    top: docked ? undefined : position?.y ?? 0,
    width: panelWidth,
    flex: docked ? '0 0 auto' : undefined,
    zIndex: isDragging ? 100 : 20 + index,
    cursor: isDragging ? 'grabbing' : 'default',
    transform: dragOffset ? `translate(${dragOffset.x}px, ${dragOffset.y}px)` : undefined,
    pointerEvents: isDragging ? 'none' : 'auto',
    borderRadius: 12,
    ```
    inner wrapper `className="w-full [&>*]:!w-full"`.
  - drop preview: `absolute pointer-events-none rounded-lg border-2 border-dashed` with
    `left: dockLeft + PANEL_MARGIN`, `top: dropTarget.dockTop + dropTarget.slotTop`,
    `width: dockWidthPx - PANEL_MARGIN * 2`, `height: dropTarget.height`,
    `borderColor: 'color-mix(in srgb, var(--accent), white 12%)'`,
    `background: 'color-mix(in srgb, var(--accent), transparent 88%)'`,
    `boxShadow: '0 0 0 4px color-mix(in srgb, var(--accent), transparent 82%)'`,
    `zIndex: 120`.
  - constants (quote the block, `:124-148`):
    ```js
    const PANEL_MARGIN = 6;
    const PANEL_GAP = 6;
    const DEFAULT_PANEL_WIDTH = 320;
    const DEFAULT_PANEL_HEIGHT = 150;
    const MIN_DOCK_WIDTH = 200;
    const MAX_DOCK_WIDTH = 520;
    const COLLAPSED_PANEL_HEIGHT = 48;
    const PANEL_WIDTH_OVERRIDES: Record<string, number> = { 'transform-debug-overlay': 420 };
    const RESIZABLE_PANELS = new Set(['prepare-models', 'support-models']);
    const LOCKED_PANELS: Record<string, 'always' | 'rail-column'> = {
      'prepare-models': 'rail-column',
      'support-models': 'rail-column',
    };
    ```
  - `DEFAULT_PANEL_DOCKS` (`:150-177`) — 25 ids, quoted verbatim in the Data
    section below.
- **States**:
  - `hidden` panel (`style.display === 'none'` on the child) → `display: 'none'`,
    still in the dock list and still holding its placement.
  - `docked` vs floating: `position: relative` + `flex: '0 0 auto'` in a dock
    (the dock's `alignItems` places it), vs `position: absolute` at
    `clampPosition(...)`.
  - `isDragging` → `zIndex: 100`, `cursor: 'grabbing'`, `pointerEvents: 'none'`,
    `transform: translate(...)`.
  - `dropTarget` non-null → the dashed preview is painted.
  - `dockResize` active → dock width follows the pointer (clamped 200-520).
  - `persistLayout` (from `isFloatingLayoutPersistenceEnabled()`) → writes
    `{ version: 6, panels, docks }` to `localStorage` on every layout change;
    `false` means nothing is written and nothing is read at first render.
  - Auto-collapse: after every size/registry/viewport change, each dock whose
    content exceeds `viewportSize.height - PANEL_MARGIN * 2` collapses
    `autoCollapse` panels from the last ordering position upward
    (`candidates … .reverse()`), subtracting `height - COLLAPSED_PANEL_HEIGHT`
    each time.
  - Right-click gesture guard: a right-button press that moves more than
    `dx*dx + dy*dy > 36` px² is treated as a drag, not a context menu
    (`event.preventDefault(); event.stopPropagation()`).
- **Data**:
  - `DEFAULT_PANEL_DOCKS` — verbatim (`:150-177`):
    ```js
    const DEFAULT_PANEL_DOCKS: Record<string, PanelDock> = {
      'prepare-models': 'left',
      'prepare-debug-primitives': 'left',
      'support-models': 'left',
      'analysis-scan-card': 'left',
      'analysis-island-list': 'left',
      'analysis-workflow': 'left',
      'export-main': 'left',
      'curve-settings': 'left',
      'prepare-transform-controls': 'right',
      'prepare-orientation': 'right',
      'prepare-smoothing-settings': 'right',
      'prepare-arrange-panel': 'right',
      'prepare-duplicate-panel': 'right',
      'prepare-organic-cut-panel': 'right',
      'analysis-overlay-controls': 'right',
      'analysis-territory-voxel': 'right',
      'analysis-island-voxel': 'right',
      'analysis-volumes': 'right',
      'support-islands': 'right',
      'support-settings': 'right',
      'support-auto': 'right',
      'support-hollowing': 'right',
      'support-hole-punch': 'right',
      'export-slicing': 'right',
      'transform-debug-overlay': 'right',
    };
    ```
  - Panel ids come from the child's React `key`, or `panel-${index}` when it has
    none (`flattenPanelChildren`). `Fragments` are flattened recursively with
    prefix `panel-${index}`. **`PrintingPanelStack`'s `PrintingPanel` has no
    `key`**, so in Printing mode the only panel id is `panel-0`, which is not in
    `DEFAULT_PANEL_DOCKS` → dock `right`, order 0.
  - Props: `children`, `leftInsetPx = 0`, `rightInsetPx = 0`, `railIsColumn = true`.
  - State: `layout` (`PanelLayoutState`), `viewportSize`, `panelSizeVersion`,
    `registryVersion`, `drag`, `dropTarget`, `dockResize`, `windowContextMenu`,
    `persistLayout`.
  - Context value: `{ registerCollapsiblePanel, requestPanelExpand, beginDockResize }`.
    Public hooks: `useFloatingPanelDockResize()`, `useFloatingPanelCollapse(defaultExpanded = true, { autoCollapse = true })`.
- **Callbacks**:
  | trigger | handler | result |
  |---|---|---|
  | pointer-down on `[data-panel-drag-handle="true"]` (button 0, not locked, not on `button, input, select, textarea, label, a, [role="button"], [data-no-drag], [data-no-drag="true"], .react-colorful`) | `handlePointerDown` | starts a drag, `event.preventDefault()` |
  | window `pointermove` | drag tracking | moves after `dx²+dy² >= 16`; sets `dropTarget` when **more than half the panel's width** overlaps a dock |
  | window `pointerup` | drag end | `placePanel(previous, id, dropTarget.dock, dropTarget.index)` or, with no target, float at the clamped pointer position |
  | pointer-down on a dock edge handle | `setDockResize({side, startX, startWidth: parentElement.getBoundingClientRect().width})` | drags the dock width; panels in `RESIZABLE_PANELS` take `max(baseWidth, width - 2*PANEL_MARGIN)` |
  | `contextmenu` on a panel | `handlePanelContextMenu` | `windowContextMenu = { panelId, x, y }` |
  | menu `Reset Window` | `resetSingleWindow(panelId)` | `placePanel(previous, panelId, DEFAULT_PANEL_DOCKS[panelId] ?? 'right', Number.MAX_SAFE_INTEGER)` |
  | menu `Reset Layout` | `resetAllWindows()` | `setLayout({panels:{},docks:{}})`, bumps `panelSizeVersion`, removes both storage keys |
  | child collapse toggle expanding | `requestPanelExpand(panelId)` | collapses other `autoCollapse` panels (bottom-up) until the dock fits |
  | `ResizeObserver` on the container | viewport measure | `{ width: window.innerWidth, height: window.innerHeight - element.offsetTop }` |
  | `FLOATING_LAYOUT_DEBUG_REQUEST_EVENT` | debug dump | calls `detail.onResult(snapshot)` with `{version: 2, capturedAt, persistenceEnabled, storageKey, panelIds, positions, placements, docks}` |
- **Text**:
  | context | msgid | text |
  |---|---|---|
  | menu title | `msg({message:'Window', comment:'Heading of the floating panel right-click menu that resets panel positions.'})` | `Window` |
  | menu aria | `msg({message:'Window context menu', …})` | `Window context menu` |
  | row 1 | `msg\`Reset Window\`` | `Reset Window` |
  | row 2 | `msg\`Reset Layout\`` | `Reset Layout` |

  Icons: `RotateCcw`, `LayoutPanelTop`.
- **Port target**: `gpui/src/panels/list.rs` (`PanelDef`, `PanelDock`,
  `PanelStackInsets`, `panel_stack`, `panel_stack_with`, `PanelLayout` in
  `gpui_core::panel_layout`). Its header already records the deviations that
  matter: persistence moved to `%APPDATA%/DragonFruit/floating-panel-layout.json`
  (no v5 migration), no pointer-events-none (root registers no hitbox), z-index
  becomes paint order, panel ids collapsed to `models`/`supports`/`export`/
  `printing`/`info`, `RESIZABLE_PANELS`/`LOCKED_PANELS` mapped onto those ids,
  hidden panels absent rather than `display:none`, and `leftInsetPx`/`rightInsetPx`
  derived by `PanelStackInsets::for_panels` because the port's stack has no
  `mode`. The dock-drag/float behaviour is pinned by
  `src/components/layout/__tests__/panelDock.dom-test.tsx` (drag `a` to the left
  dock, assert persistence, remount, assert it is still left).

## Surface: floatingLayoutPreferences — the persisted shell preferences
- **Source**: `src/components/layout/floatingLayoutPreferences.ts:1-115`
- **Renders**: nothing (module of constants and localStorage accessors).
- **Layout & styling**: n/a.
- **States**: each getter treats an absent key as the default:
  | key | default | meaning |
  |---|---|---|
  | `lumenslicer:floating-panel-layout:v6` | absent → `{panels:{},docks:{}}` | placement per panel + dock widths |
  | `lumenslicer:floating-panel-layout:v5` | read once, migrated | legacy absolute positions only |
  | `app-floating-layout-persistence` | `true` | `raw !== 'false'` |
  | `app-models-panel-visible` | `true` | `raw !== 'false'` |
  | `app-tool-layout` | `'horizontal'` | `=== 'vertical' ? 'vertical' : 'horizontal'` |
  Events: `lumenslicer:floating-layout-persistence-changed`,
  `lumenslicer:floating-layout-debug-request`,
  `lumenslicer:tool-layout-changed`.
- **Data**: `ToolLayout = 'vertical' | 'horizontal'`; `FloatingLayoutDebugSnapshot`
  (`version: 2`); `FloatingPanelPosition = {x,y}`.
- **Callbacks**: `setFloatingLayoutPersistenceEnabled`, `clearSavedFloatingLayout`,
  `setModelsPanelVisibleEnabled`, `setToolLayout` (writes + dispatches).
- **Text**: none.
- **Port target**: `gpui_core::panel_layout::PanelLayout`
  (`set_persistence_enabled`, `set_collapsed`); the rail's layout lives in
  `gpui/src/chrome/rail.rs` (rail-local, defaulting to `Vertical`).

## Surface: EmptySceneState — the empty-plate card
- **Source**: `src/components/layout/EmptySceneState.tsx:1-774`; CSS
  `.ui-empty-state` (`globals.css:563-…`).
- **Renders**: `absolute inset-0 top-[var(--topbar-height)] z-30 flex items-center justify-center pointer-events-none`
  → `div.ui-empty-state.pointer-events-auto` containing, top to bottom: the
  version chip, `<h1 className="ui-empty-title">` (a random tagline), a `<p className="ui-empty-text">`
  subtitle (max-width 560, centred), then one of three bodies — loading,
  first-time onboarding, or the normal load/import card — and two hidden
  file inputs.
- **Layout & styling**:
  - root: `inset-0` sets `top: 0`, then `top-[var(--topbar-height)]` overrides
    it, so the card starts under the bar.
  - version chip:
    `mb-2 inline-flex cursor-pointer items-center rounded-full border px-3.5 py-1 text-[13px] font-bold tabular-nums transition-colors`;
    copied → `{ color: '#2d8a4e', borderColor: '#2d8a4e', background: 'rgba(45,138,78,0.12)', boxShadow: '0 0 6px rgba(45,138,78,0.3)' }`;
    beta → light `{ color:'#9a3412', borderColor: color-mix(#f97316, var(--border-subtle) 40%), background: color-mix(#f97316, var(--surface-1) 90%) }`,
    dark `{ color:'#fdba74', … background: color-mix(#f97316, transparent 94%), boxShadow: '0 0 6px color-mix(#f97316, transparent 84%)' }`;
    default → `{ color: 'var(--accent)', borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 40%)', background: 'color-mix(in srgb, var(--accent), transparent 94%)', boxShadow: '0 0 6px color-mix(in srgb, var(--accent), transparent 84%)' }`.
  - loading card: `rounded-lg border px-4 py-4 text-left`, border
    `color-mix(in srgb, var(--accent), var(--border-subtle) 65%)`, background
    `color-mix(in srgb, var(--surface-1), transparent 8%)`, label
    `text-sm font-semibold`, detail `mt-1 text-[11px]`, then
    `<ProgressBar indeterminate size="sm" className="mt-3" />`.
  - first-time card: `rounded-lg border p-3 text-left`, border
    `color-mix(in srgb, var(--border-subtle), transparent 8%)`, background
    `color-mix(in srgb, var(--surface-1), transparent 6%)`; eyebrow
    `mb-2 text-[10px] font-semibold uppercase tracking-wide`; two
    `group rounded-md border px-3 py-3 text-left transition-colors` buttons —
    primary uses `background: 'var(--primary-button-surface)'`, border
    `color-mix(in srgb, var(--primary-button-surface), white 16%)`, text
    `var(--accent-contrast)`; secondary uses `--secondary-button-surface` /
    `--accent-secondary-contrast` with `white 16%` / `black 18%` mixes.
  - load/import grid: `grid gap-2 w-full` + `grid-cols-2` when
    `onImportSceneChange` exists else `grid-cols-1`; two
    `group cursor-pointer rounded-md border px-3 py-3 text-center transition-colors`
    buttons using the same two surface tokens.
  - recent files card: `mt-2 rounded-lg border p-3 text-left`; header row
    `flex items-center justify-between text-[10px]`; `Clear` button
    `rounded px-1.5 py-0.5 font-semibold uppercase tracking-wide transition-colors hover:bg-white/10`;
    count pill `inline-flex min-w-5 items-center justify-center rounded-full px-1.5 py-0.5 text-[9px] font-semibold`
    with `background: color-mix(in srgb, var(--surface-2), transparent 26%)`,
    `border: 1px solid color-mix(in srgb, var(--border-subtle), transparent 20%)`.
    Empty slot `mt-1.5 flex h-[38px] items-center justify-center rounded-md text-[10px]`.
    List `mt-1.5 grid grid-cols-2 gap-1.5` over
    `recentOpenedFiles.slice().reverse().slice(0, 8)`. Each row
    `flex w-full items-center justify-between gap-2 rounded-md border px-2 py-1.5 text-[10px] text-left transition-colors disabled:cursor-not-allowed`,
    `opacity: isBusy ? 0.65 : 1`. Kind badge
    `inline-flex items-center rounded px-1 py-0.5 text-[9px] font-semibold uppercase tracking-wide`
    coloured `#fb923c`/`#a78bfa` (`#c2410c`/`#6d28d9` in light theme) with
    `color-mix(kindAccent, var(--surface-0) 88%)` fill (84 % light) and a
    `1px solid color-mix(kindAccent, var(--border-subtle) 46%)` border (38 % light).
    Name `block max-w-[132px] truncate text-[10px] leading-tight`; period
    `block text-[9px]`; trailing `shrink-0 text-[9px]`.
  - drop zone: `mt-2 flex min-h-[64px] items-center justify-center rounded-md border border-dashed px-4 py-3 text-center transition-colors`;
    border `isDropActive ? (isDropUnsupported ? 'var(--danger)' : 'var(--accent)') : 'var(--border-subtle)'`;
    background `isDropActive ? (unsupported ? 'color-mix(in srgb, var(--danger), var(--surface-0) 88%)' : 'color-mix(in srgb, var(--accent), var(--surface-0) 90%)') : 'color-mix(in srgb, var(--surface-1), transparent 16%)'`.
  - inputs: `#empty-state-stl-file-input` `accept=".stl,.obj,.3mf,.zip" multiple`;
    `#empty-state-scene-file-input` `accept={sceneFileAccept} multiple`, rendered
    only when `onImportSceneChange`.
- **States**:
  - `isLoading` → loading card (label `loadingLabel ?? 'Importing your file…'`,
    detail `loadingDetail ?? 'Please hang tight while we prepare your scene.'`).
  - `shouldShowFirstTimeOnboarding = showFirstTimeOnboarding && !isLoading` →
    the two-button "Get started" card.
  - else → load/import grid + recent files + drop zone.
  - `isDropActive` / `isDropUnsupported` (local) drive the drop-zone colours;
    cleared by a 220 ms auto-clear timer (`scheduleDropAutoClear`), by
    `dragleave` (unless the related target is still inside), by `drop`,
    `dragend`, window `blur`, and a visibility change away from `visible`.
    An unsupported drop sets `isDropUnsupported` for 1600 ms.
  - per-row `isBusy` (`reopeningEntryId === entry.id`) and `reopenError`.
  - Version chip: `copied` (2 s), `IS_BETA_BUILD`.
- **Data**: props (`:26-43`): `onFileChange`, `onLoadMeshClick`,
  `onImportSceneChange`, `onImportSceneClick`, `onDropMeshFiles`,
  `recentOpenedFiles = []`, `onReopenRecentFile`, `onClearRecentFiles`,
  `isLoading = false`, `loadingLabel`, `loadingDetail`,
  `showFirstTimeOnboarding = false`, `onAddPrinter`, `onUseWithoutPrinter`.
  Module constants: `BUILD_CHANNEL`/`APP_VERSION`/`GIT_COMMIT`/`GIT_REF` from
  `process.env.NEXT_PUBLIC_*`, `IS_BETA_BUILD`, `FULL_BUILD_LABEL`,
  `MESH_DROP_EXTENSIONS = ['.stl', '.obj', '.3mf']`. Local state:
  `taglineDescriptor` (chosen once per mount, see Text), `copied`,
  `isDropActive`, `isDropUnsupported`, `reopeningEntryId`, `reopenError`,
  `isLightTheme`.
- **Callbacks**:
  | element | handler | result |
  |---|---|---|
  | version chip (click / Enter / Space) | `handleCopyBuildInfo` | `navigator.clipboard.writeText(\`DragonFruit ${APP_VERSION} (${BUILD_CHANNEL})${FULL_BUILD_LABEL ? \` — ${FULL_BUILD_LABEL}\` : ''}\`)`, `copied` for 2 s |
  | `Load Mesh` | `triggerMeshPicker` | `onLoadMeshClick()` else clicks `#empty-state-stl-file-input` |
  | `Import Scene` | `triggerScenePicker` | `onImportSceneClick()` else clicks `#empty-state-scene-file-input` |
  | `Add printer` | `onAddPrinter` | `openProfileSettingsModal('printer', { openPrinterLibrary: true })` |
  | `Use without printer` | `onUseWithoutPrinter` | `setAllowPrepareWithoutPrinter(true)` |
  | recent row | `handleReopenRecentFile(entry.id)` | `onReopenRecentFile`; `false`/throw → `Could not reopen this file from cache.` |
  | `Clear` | `onClearRecentFiles` | `scene.clearRecentOpenedFiles` |
  | drop zone | drag handlers | see States |
- **Text** (verbatim):
  - Chip: `formatVersionChip` → beta `Version ${version} — Beta`, else `Version ${version}`; copied `✓ ${_('Copied!')}`. Tooltip `Click to copy build info` (+ `\n${FULL_BUILD_LABEL}` when present).
  - Subtitle: `Bring in a mesh or scene to start preparing, analyzing, supporting, and exporting your print.`
  - Loading: `Importing your file…`, `Please hang tight while we prepare your scene.`
  - Onboarding: `Get started`, `Add printer`, `Open printer library and add one now.`, `Use without printer`, `Keep going without a printer. You can add one later.`, `Add or switch printer anytime from the top bar.`
  - Load/import: `Load Mesh` (extensions line `MESH_DROP_EXTENSIONS.join(' • ')`), `Import Scene` (labels from `useSceneFileExtensionLabels()`, each lower-cased with a leading dot, joined with ` • `).
  - Recent files: `Recent files`, `Clear`, `No recent files`, per-row `Reopen ${entry.name}` (title), kind badges `Scene` / `Mesh` (both with translation comments), `last opened just now` / `last opened ${period} ago`, `loading…`, `cached`.
  - Drop zone: `Unsupported file type` / `Drag & Drop Area` (comment `'Heading of the dashed file target in the empty-scene dialog.'`), `Unsupported format detected.`
  - Relative time: `formatRecentOpenedAt` returns `null` under 60 s, else `msg\`${deltaMin} min\``, `msg\`${deltaHours} h\``, `msg\`${deltaDays} d\``, else `new Date(openedAt).toLocaleDateString()`.
  - `TAGLINES: MessageDescriptor[]` — 35 entries, drawn once per mount with
    `useState(() => TAGLINES[Math.floor(Math.random() * TAGLINES.length)])`:
    ```
    Ready for your next adventure?
    The build plate awaits.
    What are we printing today?
    Time to make something real.
    Every great print starts with an empty scene.
    Nothing to see here. Yet.
    Your models called. They want to be sliced.
    Suspiciously empty in here.
    This scene is aggressively unoccupied.
    Zero polygons. Infinite potential.
    The resin is ready. Are you?
    Currently displaying nothing at maximum fidelity.
    Idle hands do no slicing.
    The build volume misses you.
    Drag a file in. The platform is judging you.
    Do or do not. There is no try... oh wait, just click Load mesh.
    In space, no one can hear you slice.
    You shall not pass... until you load a model.
    I am the one who slices.
    With great resin comes great responsibility.
    Why so empty?
    We're gonna need a bigger build plate.
    One does not simply import a mesh... or actually, you just click a button.
    I'll be back. (Drop a file and I really will be.)
    It's a trap! ...just kidding, drop your STL.
    The spice must flow. The resin must cure.
    Elementary, my dear user — load a model.
    Roads? Where we're printing, we don't need roads.
    Waiting for first layer adhesion... to reality.
    No supports needed for an empty scene.
    Layer 0 of 0. Living dangerously.
    Anti-aliasing: nothing to anti-alias.
    Join the Open Resin Alliance!
    Not all slicers are created equal. This one's free!
    ```
    (Commented groups in the source: Adventurous, Snarky / witty, Movie /
    pop-culture references — each with the allusion spelled out in the
    translation comment — Engineering/maker flavour, Open Resin Alliance,
    Origin story.)
- **Port target**: `gpui/src/panels/empty_scene.rs` (`empty_scene_state`,
  `model_stats_card`, `EmptySceneState` in `gpui_core::empty_state`) plus
  `gpui/src/widgets/notifications.rs` for the toast side. Check
  `gpui_core::empty_state` for which of the three bodies exist; the tagline list
  and the version chip are the parts a port is most likely to omit.

## Surface: FirstRunOnboarding — the first-run wizard
- **Source**: `src/components/layout/FirstRunOnboarding.tsx:1-338`; CSS
  `.ui-onboarding-*` (`globals.css:789-908`).
- **Renders**: `div.ui-onboarding-backdrop.fixed.inset-0.z-40.overflow-hidden` (role `dialog`,
  `aria-modal`, `aria-label="DragonFruit first-time setup"`) → a blurred, 7 %-opacity
  brand watermark `<img>` → a centring flex box → `div.ui-onboarding-panel` with
  a progress bar strip (hidden on the printer step), then the current step body.
- **Layout & styling**:
  - backdrop style:
    ```js
    background:
      'radial-gradient(ellipse at 50% 30%, color-mix(in srgb, var(--accent), var(--background) 90%) 0%, ' +
      'color-mix(in srgb, var(--accent-secondary), var(--background) 93%) 58%, var(--background) 100%)',
    ```
    plus `ui-onboarding-reveal` while dissolving (750 ms, `mask-image` radial).
  - watermark: `height/width: 'min(90vh, 110vw)'`, `opacity: 0.07`,
    `filter: 'blur(50px)'`, centred, `pointerEvents: 'none'`.
  - panel: `ui-onboarding-panel flex w-full flex-col overflow-hidden rounded-xl border shadow-2xl`,
    `background: 'var(--surface-0)'`, `borderColor: 'var(--border-strong)'`,
    `height: isLibraryExpanded ? 'min(86vh, 980px)' : 'min(440px, 86vh)'`,
    `maxWidth: isLibraryExpanded ? 'min(1040px, 94vw)' : '768px'`,
    `transition: 'height 120ms ease-out, max-width 120ms ease-out'`. The source
    comment, verbatim:
    > Fixed height (not min-height) so the browser animates the stretch: a
    > content-driven min-height can't transition. The library clips its own
    > content while the box grows, so the reveal reads as a stretch.
  - progress strip: `shrink-0 px-6 pt-5` → `flex items-center gap-1.5` of four
    `h-1 flex-1 rounded-full transition-colors duration-300` bars, filled
    `var(--accent)` for `i <= stepIndex`, else
    `color-mix(in srgb, var(--surface-2), var(--border-subtle) 45%)`.
  - step body: `key={step}` so each step remounts and re-animates; classes
    `ui-onboarding-fade` (library / done) or `ui-onboarding-step-${direction}`
    (`forward` / `backward`), plus `overflow-hidden p-0` when expanded else
    `overflow-y-auto px-6 py-6`.
- **States**: `STEPS = ['welcome','theme','printer','done']`; `step`,
  `direction`, `themePreference`, `revealing`. `isLibraryExpanded = step === 'printer'`.
  Escape is registered with **no handler** (`useEscapeToClose(true, undefined)`)
  so the wizard swallows Escape.
- **Data**: prop `onExit`. Module helper `applyBuiltInThemePreference` writes
  `THEME_STORAGE_KEY`, `THEME_PRESET_STORAGE_KEY` (`dragonfruit-light` /
  `dragonfruit-dark`), `THEME_COLORS_STORAGE_KEY` (`JSON.stringify(getThemeProfile(preset).colors)`)
  then calls `applyThemePreference` + `applyThemeCustomColors`.
- **Callbacks**: `goToStep`, `handlePrinterAdded` (→ `done`), `startReveal`
  (750 ms, or 0 ms under `prefers-reduced-motion: reduce`, then `onExit()`),
  `handleThemeSelect`.
- **Text**: `Welcome!`, `Get Started`, `Choose your theme`,
  `Pick what fits your workspace. You can change it anytime in Settings.`,
  `Dark`, `Low-glare surfaces for late-night printing.`, `Light`,
  `Bright surfaces for well-lit workspaces.`, `Back`, `Continue`,
  `You're all set!`, `Your printer is ready. Drag a model in to start preparing your first print.`,
  `Start using DragonFruit`, and the two welcome paragraphs:
  `DragonFruit is a free, open-source slicer for resin 3D printing. Load a model, orient it, add supports, and slice it for your printer.`
  and `From the community, for the community. Free to use, open to contribute, and shaped by the people who print with it.`
  The theme cards' icon chips are `<IconChip size="lg">` with
  `background: 'var(--surface-2)', borderColor: 'var(--border-subtle)'`; the
  selected card is outlined `var(--accent)` with background
  `color-mix(in srgb, var(--accent), var(--surface-1) 95%)`.
- **Port target**: none — `app_bar.rs` deviation 12 records that the GPUI app has
  no onboarding; the printer library substitute is
  `features::printer_picker`. If onboarding is wanted, this file plus
  `OnboardingPrinterLibrary` is the whole feature.

## Surface: OnboardingPrinterLibrary — the wizard's printer step
- **Source**: `src/components/layout/OnboardingPrinterLibrary.tsx:1-429`
- **Renders**: `flex h-full min-h-0 flex-col` → header (back button, icon chip,
  `Printer Library` + subtitle), body
  `grid min-h-0 flex-1 grid-cols-[220px_minmax(0,1fr)] overflow-hidden`
  (left: search + manufacturer list; right: preset grid), footer (selection
  label + `Add printer`), and `<PrinterVariantPickerModal>` when a preset with
  variants is chosen.
- **Layout & styling**: header `flex shrink-0 items-center justify-between border-b px-4 py-3`
  with `background: color-mix(in srgb, var(--surface-1), transparent 8%)`;
  sidebar `flex min-h-0 flex-col border-r` (same tint); search icon absolutely
  placed `left-3 top-1/2 … -translate-y-1/2`; input
  `className="ui-input h-8 w-full text-xs" style={{ paddingLeft: '2.5rem', paddingRight: '0.625rem' }}`;
  manufacturer buttons `w-full rounded-md border px-2.5 py-2 text-left text-sm font-semibold`,
  selected `{ borderColor: color-mix(accent-secondary, border-subtle 35%), background: color-mix(accent-secondary, surface-1 88%), color: var(--text-strong) }`,
  else `{ borderColor: var(--border-subtle), background: var(--surface-1), color: var(--text-muted) }`;
  preset card `rounded-lg border p-2.5 text-left transition-[background-color,box-shadow] duration-150`,
  selected `background: color-mix(in srgb, var(--accent), var(--surface-1) 86%)` +
  `boxShadow: 'inset 0 0 0 1.5px color-mix(in srgb, var(--accent), transparent 40%)'`;
  image well `relative flex h-[132px] items-center justify-center overflow-hidden rounded-md border`
  with `background: '#2b3039'`; grid
  `grid-cols-[repeat(auto-fill,minmax(176px,1fr))] gap-2.5`; family divider is
  two `h-px flex-1` gradient rules around
  `shrink-0 text-sm font-semibold tracking-[0.08em]`; footer
  `flex shrink-0 items-center justify-between gap-3 border-t px-3 py-3` with a
  `<Button variant="secondary" size="auto" className="inline-flex items-center gap-1.5 !h-8 !px-3 !py-0 text-sm rounded-md">`.
- **States**: `search` non-empty → flat grid + `isSearching`; empty result →
  `No printers match your search.`; `selectedPresetId` toggles; a preset with
  `modelVariants?.length || modelVariantDetectPath` opens the variant picker on
  click instead of selecting; a preset already added gets a check badge;
  `platformBadge` and `bitDepthLabel` overlays (`8 Bit` green, `3 Bit` red, other
  → `accent-secondary`); footer button is inert (`onClick` undefined +
  `aria-disabled`) until a preset is selected.
- **Data**: `presets = getLibraryPrinterPresets()`, `manufacturers` (sorted,
  `generic` last), `filteredPresets` (manufacturer filter OR free-text over
  name/manufacturer/family), `groupedPresets` (by `family || 'Other'`, insertion
  order), `addedOfficialPresetIds` from the profile store.
- **Callbacks**: `handleAdd` (`addPrinterProfileFromPreset` +
  `setActivePrinterProfile` + `onAdded()`), `handleSelectVariant(...,
  networkContext)` (also `upsertPrinterNetworkDevice(..., { select: true })` with
  `statusText: 'Connected'`), `onBack`.
- **Text**: `Printer Library`, `Choose an official printer preset to add.`,
  `Search printers`, `No printers match your search.`, `Choose a printer to add`,
  `Add printer`, `Back`, `Back to setup step`. Bit-depth badge tooltip is
  `preset.bitDepth?.description || \`${bitDepthLabel} display\``.
- **Port target**: `gpui/src/features/printer_picker.rs` (library sheet +
  `Initial Setup` dialog) and `features/printer_profiles/list.rs`.

---

## Z-index / paint-order map

The React app is one stacking context (`.ui-shell`, `position: relative`); every
overlay below is a sibling inside it. Values are the literal Tailwind classes.
GPUI has no z-index — the equivalent is child order in `app.rs::render`, which
already records the same sequence in comments.

| z | layer | source |
|---|---|---|
| `z-0` | scene wrapper (`absolute inset-0 flex`) holding `#scene-root` and, in Printing, `PrintingPreviewPane` | `page.tsx:11000ish` |
| `z-10` | `FloatingPanelStack` container | `FloatingPanelStack.tsx:946` |
| `z-10` | `CrossSliderPanel` wrapper (`absolute right-0`, `top calc(var(--topbar-height)+6px)`, `bottom 6`, `width 48`) | `page.tsx` (`scene.models.length > 0 && scene.mode !== 'printing'`) |
| `zIndex: 20 + index` | each panel inside the stack | `FloatingPanelStack.tsx:371` |
| `z-20` | `PrintingPreviewPane` inner frame | `PrintingPreviewPane.tsx:100` |
| `z-30` | `EmptySceneState` | `EmptySceneState.tsx:458` |
| `z-30` | `.ui-topbar-blur` (fixed strip) | `globals.css:212-218` |
| `z-30` | `ModelStatsCard` wrapper (`absolute bottom-1 left-1`) | `page.tsx:~11435` |
| `z-40` | `ToolRail` (column or bar) | `ToolRail.tsx:506-508` |
| `z-40` | Prepare drag-and-drop overlay | `page.tsx:11054` |
| `z-40` | `FirstRunOnboarding` backdrop | `FirstRunOnboarding.tsx:95` |
| `z-50` | `TopBar` root | `TopBar.tsx:705` |
| `z-50` | `SettingsModal` backdrop | `SettingsModal.tsx:1350` |
| `z-50` | `ProfileSettingsModal` backdrop | `ProfileSettingsModal.tsx:3913` |
| `z-50` | `IslandsPanel` loading wash (inside a panel) | `IslandsPanel.tsx:158` |
| `z-[70]` | `DiagnosticsModal`, `HistoryDebugModal`, network-settings sheet | `DiagnosticsModal.tsx:414`, `HistoryDebugModal.tsx:138`, `ProfileSettingsModal.tsx:6424` |
| `z-[80]` | `RenderProfilerOverlay`, `ModelSupportsModal` | `RenderProfilerOverlay.tsx:398`, `ModelSupportsModal.tsx:123` |
| `zIndex: 100` | the panel being dragged | `FloatingPanelStack.tsx:371` |
| `z-[100]` | `StructuredDialogModal` default | `StructuredDialogModal.tsx:37` |
| `zIndex: 120` | panel drop preview | `FloatingPanelStack.tsx:1026` |
| `z-[120]` | `BlockingOverlay` default; export overlay; arrange overlay; Auto Support discard dialog | `BlockingOverlay.tsx:49`, `page.tsx` (`Exporting…`), `SceneFileModals.tsx:699`, `AutoSupportPanel.tsx:510` |
| `z-[121]` | printer quick menu; islands-scan overlay; modifier-apply overlay; printing upload dialog | `TopBar.tsx:867`, `page.tsx` (`Analyzing Model Islands & Minima`), `ModifierModals.tsx:212`, `PrintingModals.tsx:945` |
| `z-[122]` | auto-support generation overlay | `page.tsx` |
| `z-[123]` | orientation overlay | `page.tsx` |
| `125` | `ToastViewport` for history action, scene import, export success/error, orientation, leaf-fanning | `NotificationStack.tsx`, `page.tsx` |
| `126` | `ToastViewport` for save/autosave and print-monitor errors | `NotificationStack.tsx` |
| `127` | `ToastViewport` for the plate-locked and new-input-device toasts | `page.tsx` |
| `z-[130]` | every `ContextMenu` root (app menu, panel menu, editor menu) | `ContextMenu.tsx:232` |
| `z-[130]` | `SystemNotificationStack` | `SystemNotificationStack.tsx:198` |
| `z-[130]` | `StructuredDialogModal` at the `z-[130]` call sites (destructive transform, manifold warning, re-slice, material reassign, slice completed, remesh/shrink-wrap confirms, plate delete) | the `zIndexClassName="z-[130]"` callers |
| `z-[131]` | `ContextMenu` submenu (flyout) | `ContextMenu.tsx:186` |
| `z-[140]` | printing monitor modal | `PrintingModals.tsx:1134` |
| `z-[150]` | `UvToolsLaunchingModal` | `UvToolsLaunchingModal.tsx:35` |
| `z-[170]` | printing monitor debug panels (print-monitor + RTSP) | `PrintingModals.tsx:2306,2578` |
| `z-[220]` | re-slice warning in the bar; unsaved-changes / new-scene / save-choice / plugin-warning dialogs; mesh-repair modals; surfacing progress; autosave recovery | `TopBar.tsx:1159`, `SceneFileModals.tsx`, `MeshRepairModals.tsx:259`, `SurfacingProgressModal.tsx:102`, `SceneAutosaveRecoveryModal.tsx:80` |
| `z-[230]` | `ZipFilePickerModal`, `MeshRepairReportModal` | `ZipFilePickerModal.tsx:197`, `MeshRepairReportModal.tsx:269` |
| `z-[9999]` | `Tooltip`, `MouseTooltip`, `SelectDropdown` menu | `Tooltip.tsx:193`, `MouseTooltip.tsx:68`, `SelectDropdown.tsx:363` |

Element order inside the shell (the actual paint order at equal z, and the order
`app.rs` must keep):

```
EditorLayout (.ui-shell)
├─ TopBar (fragment)
│   ├─ .ui-topbar-blur            (fixed, z-30)
│   └─ .ui-topbar                 (z-50, but position: relative — see gotcha 1)
├─ GlobalUpdateIndicator
├─ FloatingPanelStack  (z-10; panels 20+; drag 100; drop 120)
├─ CrossSliderPanel wrapper (z-10, printing excluded)
├─ scene wrapper (z-0)
│   ├─ EmptySceneState (z-30, when the scene is empty)
│   ├─ prepare drop overlay (z-40)
│   ├─ SceneCanvas (+ overlays, cursors, place-on-face, cut, mirror mounts)
│   ├─ SnapAngleReadout / RotationHintTooltip (prepare, models present)
│   ├─ ModelStatsCard (z-30)
│   ├─ import BlockingOverlay (z-50)
│   └─ PrintingPreviewPane (printing only, z-20)
├─ ToolRail (z-40, models present, prepare|support)
├─ EditorContextMenu (portal, z-130)
├─ OrganicCutOverlay
├─ RenderProfilerOverlay (z-80)
├─ DiagnosticsModals (z-70 pages)
├─ PrintingModals (z-120 … z-220)
├─ SceneFileModals (z-120 / z-220)
├─ SceneMaterialReassignModal (z-130)
├─ ModifierModals (z-121 blocking overlay)
├─ ManifoldWarningModal (z-130)
├─ TetrahedralRemeshConfirmModal + SurfacingProgressModal (z-130 / z-220)
├─ ShrinkWrapConfirmModal + SurfacingProgressModal (z-130 / z-220)
├─ MeshRepairModals (z-220)
├─ NotificationStack (toasts 125/126)
├─ SystemNotificationStack (z-130)
├─ islands scan / auto-support / orientation / export BlockingOverlays (121/122/123/120)
├─ printer-conflict + plate-trash StructuredDialogs
├─ plate-locked + new-device + leaf-fanning toasts (125-127)
└─ FirstRunOnboarding (z-40)
```

---

## Mode routing

`SupportMode = 'prepare' | 'analysis' | 'support' | 'export' | 'printing'`
(`src/supports/types.ts:9`). The mode lives in `useSceneCollectionManager`:
`const [mode, setMode] = useState<SupportMode>('prepare')` — **not persisted**, so
every launch starts in Prepare.

`handleModeChange` (`page.tsx:7463-7479`), the only writer the bar calls:

```js
if (scene.models.length === 0 && nextMode !== 'prepare') { scene.setMode('prepare'); return; }
if (nextMode === 'printing' && !hasSlicedPlate) return;
if (nextMode === 'printing' && isActiveSliceStale) { setShowPrintingResliceModal(true); return; }
scene.setMode(nextMode);
```

Forced transitions back to `prepare` (all in `page.tsx`):
| guard | line | effect |
|---|---|---|
| models reach 0 | `:7289-7294` | `setMode('prepare')` + `slicedPlates.clear()` |
| models reach 0 (second guard) | `:8139-8143` | `setMode('prepare')` |
| the active slice went stale while in Printing | `:7316-7325` | `setMode('prepare')` + open the re-slice modal (deferred while the profile modal is open) |
| undo/redo restores a history origin | `:8049-8058` | `scene.setMode(origin.appMode)` when it differs |
| "Go to hollow tool" from the unapplied-modifier modal | `:8187-8196` | `setMode('support')` + `setTransformModeWithMirrorFinalize('hollowing')` |
| slice intents | `:3051, 3065, 3069, 3105, 3182` | `setMode('printing')` after a successful slice |
| onboarding `Ctrl+Shift+O` | `:7508-7514` | opens the wizard, does not change the mode |

Per-mode routing table (what the shell renders for each mode):

| mode | panel stack | canvas | rail | extra |
|---|---|---|---|---|
| `prepare` | `PreparePanelStack` (panels `prepare-models`, `prepare-transform-controls`, `prepare-orientation`, `prepare-smoothing-settings`, `prepare-organic-cut-panel`, `prepare-arrange-panel`, `prepare-duplicate-panel`) | full width | `buildPrepareToolRailEntries` | drop overlay enabled |
| `analysis` | `AnalysisPanelStack` (`analysis-scan-card`, `analysis-workflow`, `analysis-volumes`, `analysis-island-list`, `analysis-overlay-controls`, `analysis-island-voxel`, `analysis-territory-voxel`) | full width | — (no rail entry point) | no TopBar step — see gotcha 5 |
| `support` | inline fragment: `ModelsPanel` (`support-models`), `support-settings` (`SupportSidebar`), `support-auto` (experiment-gated), `support-islands`, plus `support-hollowing` + `support-hole-punch` while `transformMgr.transformMode === 'hollowing'` | full width | `buildSupportToolRailEntries` | panels are all mounted; the ones the selected tool does not own are `display: none` |
| `export` | `ExportPanelStack` (`export-main`, `export-slicing`) | full width | — | entering export selects every visible model and may open the unapplied-modifier modal |
| `printing` | `PrintingPanelStack` (`PrintingPanel`, **no key** → id `panel-0`) | **half width**, `w-1/2 border-r` (`borderColor: var(--border-subtle)`), with `PrintingPreviewPane` beside it | — | `CrossSliderPanel` hidden; the layer scrubber lives in the preview pane |

Support-mode panel selection: `const [supportRailMode, setSupportRailMode] = useState<SupportRailMode>('manual')`
(`SupportRailMode = 'auto' | 'manual' | 'islands'`). Each of the three panels
wraps in a `div` with `style={{ display: <selected && transformMode !== 'hollowing' ? undefined : 'none' }}`;
Hollowing replaces all three. Tool memory: `prepareToolMemoryRef` remembers the
last Prepare tool in `PREPARE_TOOL_MODES`, `supportToolMemoryRef` remembers
`{ hollowing, panelMode }`, and the workspace-transition effect
(`:8069-8092`) restores them when the mode changes.

Shell insets derived from mode:
```js
leftInsetPx={scene.models.length > 0 && (scene.mode === 'prepare' || scene.mode === 'support') && toolLayout === 'vertical' ? TOOL_RAIL_WIDTH_PX : 0}
rightInsetPx={scene.models.length > 0 && scene.mode !== 'printing' ? 48 : 0}
railIsColumn={toolLayout === 'vertical'}
```
`TOOL_RAIL_WIDTH_PX = TILE_SIZE_PX + 12` (`ToolRail.tsx:19`); the column is
`w-[54px]` at `left-1.5`, i.e. 54 + 12 = 66 px.

Other mode couplings a porter must reproduce:
- `trackSupportCollectionsInHome = scene.mode !== 'support'` (`:2184`).
- `shouldCalculateSupportAndRaftVolumes = scene.mode === 'export' || (scene.mode === 'printing' && !printingArtifact)` (`:3373`).
- `effectiveSelectionHighlightMode`: `'none'` in printing, `'spotlight'` while the
  support spotlight hold is down, else `'tint'` (`:8390-8394`).
- hover/selected tint strengths forced to `0` in printing (`:8409-8415`); the
  canvas active/selected ids are `null`/`[]` in printing (`:8417-8430`).
- `isTransitioningOutOfPrinting` keeps the cross-section alive for one frame
  (`:8396`).
- Support picking is disabled while Hollowing owns the canvas:
  `blockSupportPlacement={supports.isPlacementHardDisabled || transformMgr.transformMode === 'hollowing'}`.
- The island overlays come from `islandsPoc` in `support` and from `islands`
  when `islands.overlayEnabled` elsewhere.
- `EmptySceneState` renders when `showEmptyStatePanel = scene.models.length === 0 || holdEmptyStateSceneImportUi`;
  the import overlay only when `scene.models.length > 0 && importOverlayState.active && !holdEmptyStateSceneImportUi`.
- Hollowing hides the support group in the viewport:
  `const hidden = (scene.mode === 'prepare' || scene.mode === 'support') && transformMgr.transformMode === 'hollowing'; if (supportsRef.current) supportsRef.current.visible = !hidden;` (`:1238-1241`).

---

## Keyboard shortcuts

### The binding table (`DEFAULT_KEYBINDINGS`, `src/hotkeys/hotkeyConfig.ts:24-236`)

| category | action | key | modifier | description (source string) |
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
| PRESETS | SLOT_1 … SLOT_6 | `1` … `6` | — | Pinned Slot 1 … Pinned Slot 6 |
| GLOBAL | DELETE | `Backspace` | — | Delete selected item |
| GLOBAL | UNDO | `z` | `ctrl` | Undo last action |
| GLOBAL | REDO | `z` | `ctrl+shift` | Redo last action |
| GLOBAL | SAVE | `s` | `ctrl` | Save active scene |
| GLOBAL | SAVE_AS | `s` | `ctrl+shift` | Save active scene as… |
| DEBUG | DIAGNOSTICS | `d` | `ctrl+shift` | Toggle diagnostics modal |
| DEBUG | RENDER_PROFILER | `p` | `ctrl+shift` | Toggle render profiler overlay |
| DEBUG | FORCE_UPDATE | `u` | `ctrl+shift` | Force mock update notification |
| DEBUG | HISTORY | `c` | `ctrl+shift` | Toggle history debug |
| DEBUG | TRANSFORM | `x` | `ctrl+shift` | Toggle transform debug overlay |
| DEBUG | SLICE_METRICS | `a` | `ctrl+shift` | Toggle slice metrics debug |
| DEBUG | PRINT_MONITOR | `n` | `ctrl+shift` | Toggle printing monitor debug |
| DEBUG | PRINT_RTSP | `m` | `ctrl+shift` | Toggle printing monitor RTSP debug |
| DEBUG | TOGGLE_CAPS | `k` | `ctrl+shift` | Toggle cross section caps debug panel |
| DEBUG | RE_RUN_ONBOARDING | `o` | `ctrl+shift` | Re-run the first-run onboarding wizard |
| NAVIGATION | LAYER_UP | `arrowup` | — | Navigate layer up |
| NAVIGATION | LAYER_DOWN | `arrowdown` | — | Navigate layer down |
| HOLE_PUNCH | SELECT_ALL | `a` | `ctrl` | Select all hole punches / items |
| CUT | TOGGLE_PREVIEW | `b` | — | Show or hide the cut preview while the Cut tool is open |

Comment on `GLOBAL.DELETE`, verbatim (`hotkeyConfig.ts:196-199`):
> `Delete` is always available as a fixed secondary delete key (see
> useDeleteHotkey); this configurable binding defaults to `Backspace`.

Fixed, non-configurable keys (they do **not** come from the table):

| key | owner | behaviour |
|---|---|---|
| `Delete` | `useDeleteHotkey` | fires when `!ctrlKey && !metaKey && !altKey && !shiftKey`, regardless of `GLOBAL.DELETE` |
| `Escape` | `useEscapeToClose` | top-most registered dialog; `stopImmediatePropagation()` so nothing else acts |
| `Cmd+,` (mac only: `metaKey && !ctrl && !shift && !alt && key === ','`) | `HotkeyRegistryManager` | `openSettingsModal()`, `preventDefault()` |
| `ArrowUp` / `ArrowDown`, and `W`/`S` **in printing** | `page.tsx:7203-7230` | layer ±1, but read from the raw event (`altKey \|\| ctrlKey \|\| metaKey` → ignore); the `NAVIGATION.*` bindings are **not** consulted |
| `Escape` in Prepare | `usePrepareTransformHotkeys` | leaves the `mirror` tool (`setTransformMode('select')`) |
| `Escape` in Support | `useSupportInteractionManager` | clears the support selection and hover state |
| `Alt` down/up | `useSupportInteractionManager` | branch-placement tracking (reads `key`/`code`, not the config) |

### Resolution semantics (`hotkeyStore.ts`)

1. `pressKey(key, modifiers)` lower-cases the key and adds it to `activeKeys`.
   If any *other* modifier was held **and** `hasConfiguredComboFor(key, …)` finds
   a binding with a `modifier` whose full key set is now down, the key is added
   to `comboKeys`; otherwise it is removed from `comboKeys`. Comment, verbatim:
   > Only a real configured combination claims the key. Holding another
   > modifier while pressing a hold action (Alt+Q, for example) must not
   > suppress the bare binding: Q is not part of an Alt+Q shortcut.
2. `isActionActiveSync(category, action)`:
   - every key of `getRequiredKeys(binding)` must be in `activeKeys`
     (normalising `control → ctrl`, `altgraph → alt`, `command|meta|os → meta`,
     and mapping a configured `ctrl` onto `getPrimaryModifierKey()` —
     `meta` on macOS, `ctrl` elsewhere);
   - a **bare** binding is rejected while its key is in `comboKeys`
     (`if (!targetBinding.modifier && state.comboKeys.has(normalizeKey(targetBinding.key))) return false;`).
     Comment, verbatim:
     > A key pressed as part of a modified combination stays owned by it until it
     > is released. Without this, releasing Control before the A of Ctrl+A dropped
     > the overlap suppression above and made the bare A binding
     > (CANVAS.TOOL_ARRANGE) match, so a select-all opened Arrange.
   - a binding is suppressed when any *other* binding matches with a strict key
     superset (specificity ranking);
   - `useActionActive(category, action)` is the React subscription; its
     server snapshot is `false`.
3. `matchesConfiguredHotkeyDown(e, binding)` (`hotkeyConfig.ts:280-303`):
   expected modifiers come from `parseModifier(binding.modifier)`, plus the
   modifier named by the binding's own key (`alt`/`ctrl`/`shift`/`meta`
   bindings add that modifier to the expected set); the event's modifier set must
   **equal** the expected set (`setEquals`); the base key matches when the
   binding is `alt` → `e.altKey`, else `normalizeKeyName(e.key) === binding.key`.
   This is *exact* matching — `Ctrl+S` does not match a bare `s`, and `Ctrl+Shift+S`
   does not match `Ctrl+S`.
4. `matchesConfiguredHotkeyUp(e, binding)` releases on the bound key **or any
   required modifier** being released. That is what ends a hold action when the
   user lets go of Ctrl first.

### The listener pipeline (`HotkeyRegistryManager.tsx`)

- `setupHotkeyListeners()` adds `keydown` and `keyup` on `window` **with
  `{ capture: true }`**, plus `blur` (`clearKeys()`).
- `keydown`: if `dispatchSuspended` → return; if the mac settings shortcut →
  open Settings; **if `isTextInput(e.target)` → return** (input, textarea,
  select, `isContentEditable`, or inside `[contenteditable="true"]`);
  `preventDefault()` for the browser conflicts:
  ```js
  (isPrimaryModifier && ['s','a','c','v','z','y'].includes(key))
  || ['delete','backspace','arrowup','arrowdown'].includes(key)
  || (e.shiftKey && isPrimaryModifier && ['d','c','x','a','n','m','k','p'].includes(key))
  ```
  (comment: `p` is there because `Ctrl+Shift+P` is the profiler and the platform
  reads it as Print); then `pressKey(...)`, then dispatch
  `new CustomEvent('app-hotkey-keydown', { detail: { key, code, repeat, ctrlKey, metaKey, shiftKey, altKey } })`.
- `keyup`: `releaseKey(e.key)` then `app-hotkey-keyup` with
  `{ key, code, ctrlKey, metaKey, shiftKey, altKey }` (no `repeat`).
- `suspendHotkeyDispatch()` / `resumeHotkeyDispatch()` mute the whole pipeline
  and clear the store in both directions. Comment, verbatim:
  > While Settings is recording a new binding, the app must stay deaf: this
  > manager registers its capture listener before the recorder's, so by the time
  > the recorder calls `stopPropagation` the `app-hotkey-*` events have already
  > been dispatched and acted upon (Escape closing the Settings modal, Delete
  > deleting the selection, Cmd+, reopening Settings…).
- The module **monkey-patches `EventTarget.prototype.addEventListener`** to
  `console.error` when anything outside an allow-list registers a
  `keydown`/`keyup` listener on `window`/`document`, with the message
  `Forbidden keydown/keyup event listener registered on "…" from "…". Please use HotkeyRegistryManager or hotkeyStore. See docs/reference/hotkeys.md`.
  The allow-list covers `hotkeyStore.ts`, `HotkeyRegistryManager.tsx`, the
  module's own chunk, test files, `node_modules`, `node:internal`,
  `async_hooks`, extension origins, `next-devtools`, `webpack`, `hot-dev-client`.
  Listeners marked `__isHotkeySystemInternal` bypass it.

### Consumers (all rising-edge, never level-triggered)

| hook | binding(s) | reaction |
|---|---|---|
| `useUndoRedoHotkeys` (`page.tsx:10606`) | `GLOBAL.UNDO`, `GLOBAL.REDO` | `redo()` checked **first** (Redo is a strict superset of Undo); else `undo()` |
| `useDeleteHotkey` (`page.tsx:8110`) | `GLOBAL.DELETE` + fixed `Delete` | `triggerDelete()` → the highest-`priority` handler in the delete registry that says it can delete |
| `useCameraProjectionHotkey` (`page.tsx:8111`) | `CAMERA.TOGGLE_PROJECTION` | `toggleCameraProjectionMode()` |
| `useInteriorViewHotkey` (`page.tsx:8120`) | `CAMERA.INTERIOR_VIEW` | `setInteriorView(v => !v)`, gated on `hasCavityGeometry` |
| `usePrepareTransformHotkeys` (`page.tsx:8124`) | `CANVAS.*` | only when `appMode === 'prepare' && hasModels`; `TOOL_ARRANGE` while already arranging calls `onArrangeAll()` instead of re-selecting; Escape leaves `mirror` |
| `usePresetHotkeys` (in `SupportSidebar`) | `PRESETS.SLOT_1..6` | `setActivePreset(getPresetForPinnedSlot(slot).id)` |
| `useOrganicCutHotkeys` | delete registry, priority `200` | deletes the selected waypoint while the Cut tool is active |
| `useOrganicCutPreviewHotkey` | `CUT.TOGGLE_PREVIEW` | toggles `panelState.showPreview`, gated on `enabled` |
| `useCameraFocusHotkey` (in `CameraFocusHotkeyController`) | `CAMERA.FOCUS_PICK` | frames the hovered point or the best model |
| `page.tsx:6628-6636` | `DEBUG.DIAGNOSTICS`, `HISTORY`, `TRANSFORM`, `SLICE_METRICS`, `PRINT_MONITOR`, `PRINT_RTSP`, `RENDER_PROFILER` | toggles the corresponding overlay/modal |
| `page.tsx:7508` | `DEBUG.RE_RUN_ONBOARDING` | `setWizardActive(true)`, `setAllowPrepareWithoutPrinter(false)` |
| `page.tsx:9732-9743` | `GLOBAL.SAVE_AS` | `handleTopBarSaveSceneAs()` when a model exists |
| `SceneCanvas.tsx:6624` | `DEBUG.TOGGLE_CAPS` | toggles the cross-section caps debug |
| support controllers (`Curves/useCurveHotkey.ts`, `SupportPrimitives/Joint/useJointCreationHotkey.ts`, `SupportTypes/**/useBracePlacement.ts`, `useBranchPlacement.ts`, `useKickstandPlacement.ts`, `SupportSidebar.tsx`) | `SUPPORTS.*` | placement modes and the auto-bracing generate |

`GLOBAL.SAVE` (`Ctrl+S`) has **no consumer in the sampled code** — saving goes
through `GLOBAL.SAVE_AS` and the bar/`EditorContextMenu`; see Open questions.

### Escape ordering (`useEscapeToClose.ts`)

A module-level `openDialogs: EscapeEntry[]` stack plus one capture listener on
`window` for `app-hotkey-keydown`. On Escape the **last** entry is taken,
`stopImmediatePropagation()` is called, and its handler (or nothing) runs.
Comment, verbatim:
> A registration without a handler swallows Escape instead of closing — that is
> how a deliberately non-dismissible modal keeps the key from reaching whatever
> is behind it.

Registered this way (all through `useEscapeToClose(open, onClose?)`): the profile
change warning, `FirstRunOnboarding` (`true, undefined`), the four blocking
overlays in `page.tsx` (islands scan, auto-support, orientation, export — all
`undefined`), `AutoSupportPanel`'s settings dialog, the diagnostics / history /
supports / hierarchy modals, `UvToolsLaunchingModal` (`undefined`),
`ZipFilePickerModal`, `MeshRepairModals` (undefined while repairing),
`ModifierModals`' overlay, all of `PrintingModals`' five dialogs,
`SceneFileModals`' prompts, `PrinterVariantPickerModal`,
`MeshRepairReportModal`, `SceneAutosaveRecoveryModal` (`true, undefined`),
`BackupsSettingsTab`'s two, `ExperimentsSettingsTab`, `HotkeysSettingsTab`'s
recorder.

### Labels (`hotkeyLabels.ts`)

`toModifierLabel(modifier, primaryModifierLabel)`: `ctrl → primaryModifierLabel`
(`Cmd` on macOS, `Ctrl` elsewhere), `shift → 'Shift'`, `alt → 'Alt'`,
`meta → primaryModifierLabel === 'Cmd' ? 'Cmd' : 'Meta'`.
`toKeyLabel(key)`: `' '` → `'Space'` first (a one-character key would be
upper-cased into a blank keycap), single characters upper-cased, everything else
verbatim. `getBindingTokens` = modifier tokens in config order, then the key.
Module comment, verbatim:
> `hotkeyStore` maps a configured `ctrl` onto the platform's primary modifier,
> which is Cmd on macOS. Labelling it "Ctrl" there would advertise a shortcut
> that does not exist.

### Persistence

`HotkeyProvider` writes the whole config to `localStorage['app-hotkeys-config']`
after the first read. On load it runs `stripStaleActions(defaults, stored)` —
dropping categories/actions that no longer exist and **taking every
`description` from the defaults** (the user owns only `key` and `modifier`) —
then `mergeBindings(defaults, cleaned)` which keeps default actions the stored
tree does not know about. `resetCategories(categories)` restores whole
categories. The Settings table groups categories as `GLOBAL → General`,
`CAMERA → Camera`, `CANVAS → Canvas Tools`, … (`HotkeysSettingsTab.tsx:22-25`,
another area).

---

## Deliberate decisions and gotchas

1. **The app bar is not `position: fixed`, despite its class list.** `TopBar.tsx:705`
   writes `` `ui-topbar fixed top-0 left-0 right-0 z-50 flex items-center relative …` ``
   — `fixed` **and** `relative` on the same element, and `.ui-topbar`
   (`globals.css:158-161`) is *unlayered* CSS declaring `position: relative`.
   Unlayered rules beat layered `@layer utilities`, so the bar is a 52 px
   in-flow relative strip; `top-0/left-0/right-0` do nothing. Everything else
   positions itself with `top-[var(--topbar-height)]` for exactly this reason.
   The genuinely fixed layer is `.ui-topbar-blur` (`z-index: 30`). Do not "fix"
   this by making GPUI's bar float — the source's scene sits *under* the strip.
2. **Two frosted layers, two masks.** `.ui-topbar::before` carries the surface +
   blur for the bar's own box; `.ui-topbar-blur` is a separate fixed strip with a
   longer fade (60 % vs 86 % opaque). Both go fully transparent during onboarding
   (`ui-topbar-transparent` / `ui-topbar-blur-transparent`). GPUI has neither
   backdrop blur nor mask: `app_bar.rs` deviations 1 and 2.
3. **`z-index: -1` on the bar's `::before`.** The pseudo-element must paint under
   the bar's children but over the scene; `isolation: isolate` on `.ui-topbar`
   keeps that negative index inside the bar's stacking context instead of
   pushing the surface behind the whole shell. A port that draws the surface as a
   normal child paints over the controls.
4. **`step: 1..4` in the `steps` array is dead data.** The number is never read;
   only `icon`, `label`, `hint`, `mode`, `locked` are. Keep the icons, drop the
   number.
5. **`analysis` is a real mode with no way in from the bar.** `SupportMode`
   includes `'analysis'`, `HISTORY_APP_MODES` includes it, and the panel routing
   has an `AnalysisPanelStack` branch — but the bar renders only four steps
   (`prepare`, `support`, `export`, `printing`) and `handleModeChange` is the only
   writer the bar calls. `analysis` is reachable only through history restore
   (`restoreHistoryOriginRef`, `:8049-8058`). The port must not invent a fifth
   step, and must not drop the branch.
6. **`EmptySceneState`'s `z-30` is scoped to the `z-0` scene wrapper.** The card
   paints above the canvas but below the bar; the bar's blur strip is a *sibling*
   at `z-30` in the shell, and the shell's own order decides between them (the
   card is inside the earlier `z-0` subtree, so the blur strip wins). Getting this
   wrong hides the empty-state card behind the frosted strip.
7. **The empty-state tagline is chosen once per mount and never re-rolled.**
   `useState(() => TAGLINES[Math.floor(Math.random() * TAGLINES.length)])` — a
   re-render keeps the same line; a remount picks a new one. 35 lines, all with
   translation comments; a port that ships one of them is not 1:1.
8. **`style={{ color: active ? 'var(--text-strong)' : 'var(--text-strong)' }}`**
   on the step label (`TopBar.tsx:1030-1034`) is identical on both branches.
   Reproduce one constant; do not invent a "muted idle" step.
9. **`const nativeDisabled = disabled;`** is a pass-through. The real states are
   `locked`, `disabled = locked || topbarActionsDisabled`,
   `visuallyDimmed = topbarActionsDisabled || locked`, `printingLocked`.
   `printingLocked` alone adds `grayscale saturate-0` on top of the same
   `opacity-45`.
10. **`hideBadges` and `showViewModeLabel` come from one measurement with two
    thresholds.** `perButton = (contentRect.width - 3 * gapPx) / 4` with
    `gapPx = 4`; badges hide below 120 px; the view-mode label *appears* at
    155 px and only *stays* at 130 px. The `stepsContainerRef` effect is
    re-armed on `[hideWorkflowControls]` because the container unmounts during
    onboarding — the source comment says measuring a detached node "pinned the
    icons hidden".
11. **The window drag is opt-out, not opt-in, and the opt-out is a *different*
    attribute from the panel stack's.** `handleTopBarPointerDown` walks from the
    event target up to the bar and bails on
    `'button, a, input, select, [role="button"], [data-no-window-drag="true"]'`.
    Source comment, verbatim:
    > The desktop window drag keeps its own opt-out. `data-no-drag` belongs to the
    > floating panel stack (see FloatingPanelStack.isDragBlockedByTarget); the
    > two are not interchangeable.
    It also bails when the pointer is outside the bar's own rect: "Guardrail: only
    allow drag starts from the visible topbar strip itself. This prevents nested
    modals/menus rendered under TopBar from dragging the app window."
12. **Panel ids are React `key`s, and one panel has none.** `flattenPanelChildren`
    uses `String(child.key)` or `panel-${index}`; fragments recurse with the
    fragment's index as prefix. `PrintingPanel` is rendered without a `key`, so
    its placement is keyed `panel-0` — not in `DEFAULT_PANEL_DOCKS`, therefore
    docked right by `defaultPlacement`. Renaming it changes the user's saved
    layout.
13. **Hidden panels stay mounted and only lose their box.** Support mode renders
    every panel and hides the unused ones with `style.display = 'none'`; the
    stack maps that to `display: 'none'` on the item and keeps them out of the
    *drop maths* only, never out of the dock order — "They must leave the dock's
    flow too, or their gap pushes every panel below them down." The reason the
    panels stay mounted is in `page.tsx`: "the window layout profiles resolve
    against the set of mounted panels, so unmounting one would move every panel
    anchored to it."
14. **Dock width follows the panels, not the drag.** `dockWidth()` is
    `max(widestPanel + 2*PANEL_MARGIN)` clamped to `[200, 520]`, and a stored
    `docks.leftWidth` is *only* honoured when the dock holds a `RESIZABLE_PANELS`
    member. The drag handler writes both the panel width and the dock width, and
    the comment says why: "a panel keeps that width when it is dragged out to
    float. The drag can only widen: a panel never goes below its natural width."
15. **Drop targeting measures overlap, not the pointer.** "Snap on how much of
    the dragged panel is over the dock, not on where the pointer is: a wide dock
    would otherwise hold the panel until the pointer had travelled its whole
    width." The threshold is `> draggedWidth / 2`, and `dockIndexAt` walks the
    *visible* panels comparing `clientY` against each one's vertical midpoint.
16. **A right-drag is not a context menu.** `FloatingPanelItem` records the
    right-button gesture and suppresses `contextmenu` when the pointer travelled
    more than 6 px (`dx*dx + dy*dy > 36`) — the browser still fires `contextmenu`
    after a right drag.
17. **Dock resize reads the rendered width, not the state.**
    `startWidth: event.currentTarget.parentElement?.getBoundingClientRect().width`,
    with the comment "a closure captured before the last commit would start the
    drag from a stale value and jump."
18. **The panel layout is `version: 6` and the v5 migration is lossy on purpose.**
    v5 stored absolute positions for *moved* panels only; the loader projects each
    into a dock by its centre (`< 40 %` width → left, `> 60 %` → right, else
    float). "Keep the intent: a panel parked near an edge joins that dock,
    anything else floats in place."
19. **`requestPanelExpand` and the auto-compact pass share the arithmetic but not
    the trigger.** Expanding a panel from its header chevron collapses *other*
    `autoCollapse` panels bottom-up until the dock fits; the same loop runs after
    every size/registry/viewport change. Both use
    `COLLAPSED_PANEL_HEIGHT = 48` as an estimate — the collapsed panel is 48 px
    tall in the model, not a measured height.
20. **The panel right-click menu has exactly two rows.** `Reset Window` /
    `Reset Layout` under the heading `Window`; there are no "float / dock left /
    dock right" entries — `list.rs` records that the previous GPUI port invented
    them.
21. **Panel `zIndex` is `20 + index` where `index` is the index within its own
    list** (`leftPanels.map`, `rightPanels.map`, `floatingPanels.map`), so a left
    and a right panel can share a z. Only the dragging panel (100) and the drop
    preview (120) escape the ties.
22. **`usePresetHotkeys` subscribes to two stores with a snapshot that never
    changes** (`() => null`). That is deliberate: it lives in `SupportSidebar`,
    which holds the anatomy preview canvas, and re-rendering it on every preset
    keypress was a hitch. Read the rising edge inside the callback instead.
23. **Undo/redo is checked Redo-first.** "Redo is a strict superset of Undo (adds
    Shift by default), so it is checked first; `isActionActiveSync` also
    suppresses Undo whenever Redo is the active match." A port that checks Undo
    first turns every Ctrl+Shift+Z into a plain undo.
24. **`isActionActiveSync` is not a key test.** It normalises modifiers across
    platforms, ranks overlapping bindings by specificity, and refuses a bare
    binding whose key is currently owned by a combination. Everything that
    dispatches actions goes through it; only Escape/Alt handling and the arrow
    keys bypass it. If the port reduces it to `key == "a"`, `Ctrl+A` opens
    Arrange (the regression `hotkeyStore.test.ts:173-186` pins).
25. **`Ctrl+S` is preventDefault'ed but nothing consumes `GLOBAL.SAVE`.**
    Handling exists only for `SAVE_AS` (`page.tsx:9732`). The browser's save
    dialog is suppressed; the app does nothing. See Open questions.
26. **The mac settings shortcut is checked before the text-input guard.**
    `Cmd+,` opens Settings even with focus in a text field; every other binding
    returns early inside one. The mac check also requires `!ctrl && !shift && !alt`.
27. **`preventDefault` is unconditional for `delete`/`backspace`/`arrowup`/`arrowdown`.**
    That is what keeps Backspace from navigating back and the arrows from
    scrolling, in every mode, even with no models.
28. **`y` is in the primary-modifier preventDefault list but has no binding.**
    Leftover from a Windows-style redo; harmless, but a port that adds `Ctrl+Y`
    as redo invents behaviour.
29. **The store is cleared on window `blur`, not on visibility change.** A
    webview that loses and regains the window with a key held would otherwise
    keep that key latched.
30. **`deleteRegistry` priority decides the action, not registration order.**
    The Cut tool registers at `ORGANIC_CUT_DELETE_PRIORITY = 200` — "Above the
    support-interaction handler (100) so that while a seam is being drawn, Delete
    edits the seam and can never fall through to deleting the model itself" —
    and the select-all handler at `20`. A port that keeps a single delete
    callback loses that ordering.
31. **The Cut tool's delete claim covers the whole session, even with no waypoint
    selected**: "a Delete that fell through to the model would be destructive."
32. **The wizard swallows Escape** (`useEscapeToClose(true, undefined)`) and its
    "Start using DragonFruit" button does not close it directly — it runs a
    750 ms radial-mask reveal (0 ms under `prefers-reduced-motion`) and *then*
    calls `onExit`.
33. **The wizard is latched, not derived.** `wizardActive` goes true once
    `onboardingMounted && !hasPrinterProfiles` and stays true until `onExit`, so
    the finalisation step can play *after* a printer was just added (which makes
    `hasPrinterProfiles` true). A port that derives visibility from "no printers"
    closes the wizard mid-flow.
34. **`allowPrepareWithoutPrinter` is a session-only override** cleared as soon
    as a printer profile exists (`:8366-8370`), so "Use without printer" does not
    hide the onboarding prompt forever.
35. **`onboardingMounted` exists to avoid hydration mismatch.** The wizard and
    `hideWorkflowControls` are gated on a mount effect, not on the store, so the
    server and first client render agree.
36. **`ProfileSettingsModal` is opened by an event, not by props**:
    `OPEN_PROFILE_SETTINGS_MODAL_EVENT` carries
    `{ tab, openPrinterLibrary, openNetworkSettings, openMaterialAntiAliasing }`
    and each "openX" flag bumps a counter token the modal reads, so the same
    request twice in a row still re-opens the sub-view. `OPEN_SETTINGS_MODAL_EVENT`
    carries `{ tab }`, whitelisted against 17 tab keys, else `'general'`.
37. **The re-slice warning gates on `Boolean(printingArtifact && !isActiveSliceStale)`**
    — only when there is a valid slice to invalidate.
38. **`isSlicingBusy` freezes navigation as well as dimming the bar** —
    `disabled = locked || topbarActionsDisabled`.
39. **`EditorLayout` carries `data-no-window-drag="true"` at the root**, so the
    whole app is opted out of window dragging; the bar re-enables it per control
    by walking up from the event target inside its own handler.
40. **`data-no-drag` vs `data-no-window-drag`** — see gotcha 11. The panel stack
    checks `data-no-drag`, the bar checks `data-no-window-drag`; the bar's left
    cluster deliberately sets `data-no-window-drag="false"`, a no-op for the
    `[data-no-window-drag="true"]` selector, but it documents intent.

---

## Open questions

1. **`GLOBAL.SAVE` has no handler in the sampled call sites.** `Ctrl+S` is
   preventDefault'ed and `SAVE_AS` is consumed, but nothing reads `GLOBAL.SAVE`.
   Either a consumer lives outside `src/app` / `src/hotkeys` (a plugin?) or the
   app only ever saves through the save-as path. The port needs a decision: bind
   `Ctrl+S` to the existing save flow (as the description promises) or leave it
   inert to match the source.
2. **`HOLE_PUNCH.SELECT_ALL` (`Ctrl+A`)** collides with `CANVAS.SELECT_ALL`.
   `isActionActiveSync`'s specificity ranking treats them as equals (same key
   set), so both report active. Which one acts depends on each consumer's mode
   guard; the port must decide whether the hole-punch selection replaces or
   supplements the model select-all.
3. **`SUPPORTS.LEAF_PLACEMENT` is `Alt` + `ctrl`** while `SUPPORTS.BRANCH_PLACEMENT`
   is bare `Alt`. The store's `SELF_MODIFIER_FLAG` table is what keeps `Shift`
   from counting as "another modifier" for `Shift` itself; a port with a simpler
   "any modifier held" rule makes both placement modes fire at once (the test at
   `hotkeyStore.test.ts:89-96` pins the correct behaviour).
4. **`DEBUG.FORCE_UPDATE` (`Ctrl+Shift+U`) has no consumer in the sampled
   files.** Presumably `GlobalUpdateIndicator` / `UpdateCheckerSection`; those
   listen to `app-hotkey-keydown` directly. The port has no updater yet, so this
   row is unported by construction.
5. **The `analysis` mode's entry point is unknown** (see gotcha 5). If it is
   dead in the source, the port may drop `AnalysisPanelStack`; if it is entered
   by history restore, the port must keep it.
6. **Preset slots depend on `getPresetForPinnedSlot` returning a preset**; a slot
   with no pinned preset does nothing (no feedback). Whether the port should
   toast "slot empty" is a product decision the source does not make.
7. **The panel layout's `docks` widths survive a printer/profile change** but the
   panel *set* does not (Export has one panel, Support has five). Nothing prunes
   placements for panels that are not currently rendered, so the saved document
   accumulates ids forever. A port may want a prune; the source does not have one.
8. **`isLightTheme` in `TopBar` and `EmptySceneState` use different detectors.**
   `TopBar` reads `document.documentElement[data-theme]`; `EmptySceneState` also
   checks the `dragonfruit-light` / `dragonfruit-dark` classes and the media
   query. Either is fine for a dark-only port, but a light-theme port must not
   pick one and assume it covers both.
9. **`ViewTypeDropdown`'s `iconOnly` threshold and the bar's own measurement are
   coupled** (`perButton >= 155/130`). The GPUI port derives `perButton` from the
   window width each frame and uses only 130; the source's asymmetry exists
   because the trigger's own width feeds the measurement. Whether the port keeps
   the asymmetry depends on `ui::select_dropdown`'s ability to change width
   mid-render.
10. **Nothing persists the app mode.** `mode` is `useState('prepare')` in the
    scene manager, so a restore always lands in Prepare even when the session was
    saved in Printing. If the port persists the mode (the handoff says session
    persistence is ours), that is a *deviation*; record it rather than "fixing"
    it silently.
