# Chrome — app bar, tool rail, menus, shell

Everything that frames the scene: the shell template, the top app bar (workflow
steps, logo menu, printer badge and fleet quick menu, monitor button, camera /
view-mode dropdown, Overhangs toggle, Settings, caption buttons), the tool rail
and its entries, the shared menu widget used by all of them, the tooltip, the
canvas context menu, and the floating-panel stack that docks and floats every
panel over the canvas.

The Rust side of this area already exists and is close to the source:
`gpui/src/chrome/{app_bar,rail,editor_menu}.rs`, `gpui-core/src/panel_layout.rs`,
`gpui-ui/src/widgets/{menu,tooltip,select,select_menu,dialog}.rs`. Each
"Port target" section below states what is there and what is still missing, so
a follow-up agent closes gaps rather than re-deriving them.

---

## Components in scope

| Component | File |
|---|---|
| `EditorLayout` (shell template) | `src/components/templates/EditorLayout.tsx` |
| Page composition / layer order | `src/app/page.tsx:10649-12126` |
| `TopBar` | `src/components/layout/TopBar.tsx` |
| `topBarMessages` (interpolated msgids) | `src/components/layout/topBarMessages.ts` |
| `.ui-topbar*` / `.ui-shell` / `.ui-button*` / `.ui-input` CSS | `src/app/globals.css:39-490, 439-442` |
| `ToolRail` + hand-drawn icons | `src/components/controls/ToolRail.tsx` |
| `buildPrepareToolRailEntries` / `buildSupportToolRailEntries` | `src/components/controls/toolRailEntries.ts` |
| `Tooltip` | `src/components/ui/Tooltip.tsx` |
| `ContextMenu` (all menus) | `src/components/ui/ContextMenu.tsx` |
| `EditorContextMenu` | `src/components/ui/EditorContextMenu.tsx` |
| `SelectDropdown` (view-mode trigger/menu) | `src/components/ui/SelectDropdown.tsx` |
| `ViewTypeDropdown` | `src/components/controls/ViewTypeDropdown.tsx` |
| `MESH_SHADER_OPTIONS` | `src/features/shaders/mesh/types.ts:42-49` |
| `FloatingPanelStack` + `useFloatingPanel*` hooks | `src/components/layout/FloatingPanelStack.tsx` |
| Layout preferences + storage keys | `src/components/layout/floatingLayoutPreferences.ts` |
| `StructuredDialogModal` (re-slice warning) | `src/components/ui/StructuredDialogModal.tsx` |
| `Button` atom, `AlertDiamondIcon` | `src/components/atoms/Button.tsx`, `src/components/atoms/AlertDiamondIcon.tsx` |
| Dismissal: `useOutsideDismiss`, `useEscapeToClose` | `src/hooks/useOutsideDismiss.ts`, `src/hotkeys/useEscapeToClose.ts` |

Deliberately out of scope, in the same folder: `EmptySceneState.tsx` (empty-scene
area, ported as `gpui/src/panels/empty_scene.rs`), `FirstRunOnboarding.tsx` and
`OnboardingPrinterLibrary.tsx` (onboarding area — not ported, see gotcha 24).
`GlobalUpdateIndicator` (`src/features/updater/`), `NotificationStack`,
`SystemNotificationStack` and the modals belong to their own areas.

---

## Surface: `EditorLayout` — the shell

- **Source**: `src/components/templates/EditorLayout.tsx:12-17`
- **Renders**: one full-viewport div containing all overlay siblings.
- **Layout & styling**:
  ```jsx
  <div className="ui-shell relative h-screen w-screen overflow-hidden" data-no-window-drag="true">
  ```
  `.ui-shell` (`src/app/globals.css:439-442`) is
  `background: var(--background); color: var(--foreground);`. The component's own
  doc comment: "it owns the page layout skeleton (size, overflow, window-drag
  opt-out) and hosts the TopBar / panel stacks / scene / modals / toasts as
  children."
- **States**: none. No props besides `children`.
- **Data**: none.
- **Callbacks**: none.
- **Text**: none.
- **Port target**: the root element of `gpui/src/app.rs`'s render (the entity's
  top-level `div`). There is no module named for it; keep the meaning — a
  viewport-sized, clipped, opaque-background stack whose children are
  absolutely positioned layers.

### Layer order inside the shell (`src/app/page.tsx`)

Everything is a sibling inside `EditorLayout`; ordering is the z-order contract.

| # | Layer | Anchor | z |
|---|---|---|---|
| 1 | `TopBar` | `page.tsx:10651` | `z-50` (`.ui-topbar`) |
| 2 | `GlobalUpdateIndicator` | `page.tsx:10713` | own area |
| 3 | `FloatingPanelStack` (all docks + floats + their right-click menu) | `page.tsx:10717-10989` | `z-10` container |
| 4 | Right-edge `CrossSliderPanel` wrapper, only when `models.length > 0 && mode !== 'printing'`: `className="absolute right-0 z-10 pointer-events-auto"`, `style={{ top: 'calc(var(--topbar-height) + 6px)', bottom: 6, width: 48 }}` | `page.tsx:10991-10995` | `z-10` |
| 5 | Canvas: `<div className="absolute inset-0 z-0 flex">`, inside it `#scene-root` (`relative h-full w-full`, or `w-1/2 border-r` in printing mode) with `SceneCanvas` + `SceneOverlays`, and `PrintingPreviewPane` in printing mode | `page.tsx:11020-11425` | `z-0` |
| 6 | `ToolRail` | `page.tsx:11439` | `z-40` |
| 7 | `EditorContextMenu`, organic-cut overlay, profiler overlay, all modal groups | `page.tsx:11473+` | own areas |
| 8 | `NotificationStack`, `SystemNotificationStack`, blocking overlays, toasts (`zIndex` 125/127), `FirstRunOnboarding` | `page.tsx:11890-12122` | — |

Consequence a porter must respect: the canvas is **under** the app bar and under
the panels (which is why the bar needs a frosted surface and why the panel stack
starts at `top: var(--topbar-height)`), and the rail is **above** the panels
(`z-40` vs `z-10`) so a panel dragged to the left edge slides under it.

---

## Surface: `TopBar` — the app bar

- **Source**: `src/components/layout/TopBar.tsx` (1242 lines; the component body
  is `:118-1242`).
- **Renders**: two siblings — an `aria-hidden` blur strip and the bar itself,
  which is a three-column flex row (left group / workflow steps / right group)
  plus every popup it owns (app menu, fleet quick menu, two hidden file inputs,
  the re-slice warning dialog, and mounts of `SettingsModal` and
  `ProfileSettingsModal`).

### Bar root

- **Layout & styling** (`TopBar.tsx:703-706`):
  ```jsx
  <div className={`ui-topbar-blur ${hideWorkflowControls ? 'ui-topbar-blur-transparent' : ''}`} aria-hidden="true" />
  <div
    className={`ui-topbar fixed top-0 left-0 right-0 z-50 flex items-center relative ${hideWorkflowControls ? 'justify-between ui-topbar-transparent' : ''}`}
    onMouseDownCapture={handleTopBarPointerDown}
  >
  ```
  `.ui-topbar` (`globals.css:158-166`):
  ```css
  .ui-topbar {
    position: relative;
    isolation: isolate;
    height: var(--topbar-height);   /* 52px */
    background: transparent;
    padding: 3px 4px 9px 4px;
    gap: var(--space-3);            /* 12px */
  }
  ```
  `.ui-topbar::before` (`:170-208`) is the visible surface: two stacked
  gradients plus `backdrop-filter: blur(24px)` and a bottom-fading
  `mask-image`; it is `z-index: -1`, `pointer-events: none`. The accent ramp is
  ```css
  linear-gradient(90deg,
    color-mix(in srgb, var(--topbar-accent), transparent 93%) 0%,
    color-mix(in srgb, var(--topbar-accent), transparent 97%) 45%,
    color-mix(in srgb, var(--topbar-accent), transparent 94%) 100%)
  ```
  over the surface ramp
  ```css
  linear-gradient(to bottom,
    color-mix(in srgb, var(--surface-0), transparent 6%) 0%,
    color-mix(in srgb, var(--surface-0), transparent 10%) 50%,
    color-mix(in srgb, var(--surface-0), transparent 24%) 100%)
  ```
  with `mask-image: linear-gradient(to bottom, rgba(0,0,0,1) 0%, rgba(0,0,0,1) 86%, rgba(0,0,0,0.52) 92%, rgba(0,0,0,0.12) 97%, transparent 100%)`.
  `.ui-topbar-blur` (`:212-241`) is the second, *behind* the bar (`z-index: 30`,
  `pointer-events: none`): plain `backdrop-filter: blur(24px)` with a different
  mask (`1` to 60%, `0.82` @75%, `0.44` @88%, `0.14` @96%, transparent @100%).
  Both fade out over `150ms ease` (`:244-247`); `.ui-topbar-transparent::before`
  and `.ui-topbar-blur-transparent` zero their opacity and drop the
  `backdrop-filter` (`:249-259`).
- **States**:
  - default;
  - **onboarding** (`hideWorkflowControls`, `page.tsx:10695` →
    `onboardingMounted && wizardActive`): the bar adds `justify-between
    ui-topbar-transparent` and the blur strip adds `ui-topbar-blur-transparent`,
    i.e. both surfaces fade to nothing and the three groups spread to the edges
    because the centre column is not rendered at all;
  - **actions disabled** (`topbarActionsDisabled`, `:186`, = `isSlicingBusy`):
    each group dims itself (`opacity-45`, left group also `pointer-events-none`)
    and every control inside is `disabled`.
- **Data**: props only (no store). `mode`, `hasModels`, `hasPrintingData`,
  `viewType`, `overhangViewActive`, `isSlicingBusy`, `showMonitorButton` +
  `monitorButton*`, `printerReachabilityByDeviceId`,
  `warnBeforeProfileSettingsOpen`, `hideWorkflowControls`; plus the mesh/view
  settings that are only forwarded to `SettingsModal` (see "Pass-through
  props").
- **Callbacks**: `onModeChange`, `onViewTypeChange`, `onOverhangViewChange`,
  `onNewScene`/`onSaveScene`/`onSaveSceneAs`/`onOpenScene`,
  `onLoadMeshChange`/`onImportSceneChange`, `onCloseProgram`, `onOpenMonitor`.
- **Port target**: `gpui/src/chrome/app_bar.rs` — `AppBarState` + `app_bar()`.
  Present and complete for every surface in this section, including the
  responsive step thresholds, the fleet menu, the caption buttons, the app menu
  and the view-mode menu. Open items are listed in "Gotchas" 1-8.

### Window drag (`handleTopBarPointerDown`, `:353-402`)

Only when `isDesktopWindow` and `event.button === 0`. Two guards, both quoted
because they are load-bearing:

```js
// Guardrail: only allow drag starts from the visible topbar strip itself.
// This prevents nested modals/menus rendered under TopBar from dragging the app window.
if (event.clientX < topbarRect.left || ... event.clientY > topbarRect.bottom) return;
```
```js
const interactiveSelector = [
  'button', 'a', 'input', 'select', '[role="button"]',
  // The desktop window drag keeps its own opt-out. `data-no-drag` belongs to
  // the floating panel stack (see FloatingPanelStack.isDragBlockedByTarget);
  // the two are not interchangeable.
  '[data-no-window-drag="true"]',
].join(',');
```
Then it walks up from the event target to the bar root and returns if any
ancestor matches. Otherwise `getCurrentWindow().startDragging()`.

`data-no-window-drag` appears explicitly on: the logo button, the printer
button, the monitor button, the Overhangs button, the Settings button (all
`="true"`), and on the left group as `="false"` (`TopBar.tsx:710`) — a
non-match that keeps the group draggable in its empty areas.

- **Port target**: `app_bar.rs` attaches `start_window_drag(window)` to the bar
  root's `on_mouse_down` and every control calls `cx.stop_propagation()`.
  `chrome/rail.rs` sets both `data-no-drag` semantics by construction (it never
  reaches the drag handler).

### Left group

- **Source**: `TopBar.tsx:708-833`
- **Layout & styling**:
  ```jsx
  <div className={`flex flex-1 max-w-[430px] items-center gap-2.5 pl-0 pr-4 py-1.5 transition-opacity ${topbarActionsDisabled ? 'opacity-45 pointer-events-none' : ''}`}
       data-no-window-drag="false" aria-disabled={topbarActionsDisabled}>
  ```

**Logo / app-menu button** (`:713-746`)
- `className="inline-flex h-10 w-10 items-center justify-center rounded-md transition-colors"`.
- inline `background`: `color-mix(in srgb, var(--accent), transparent 80%)` while
  `isAppMenuOpen`, else `transparent`.
- `<img src="/dragonfruit_assets/branding/simple_icon.svg" alt="DragonFruit" className="h-7 w-7 object-contain" draggable={false}>`;
  in light theme only, `style={{ filter: 'drop-shadow(0 1px 3px rgba(0,0,0,0.35))' }}`.
- `title=msg("DragonFruit menu")`, `aria-label=msg("Open DragonFruit menu")`,
  `data-no-window-drag="true"`, `disabled={topbarActionsDisabled}`.
- Click toggles the app menu (`openAppMenu` measures
  `getBoundingClientRect()` → `{ x: rect.left, y: rect.bottom + 6 }`).

**Divider** (`:749-751`): `<div className="h-6 w-px mx-0.5 shrink-0" aria-hidden="true" style={{ background: 'color-mix(in srgb, var(--border-subtle), transparent 24%)' }} />`.

**Printer badge** (`:752-804`) — see its own surface below.

**Monitor button** (`:807-832`), rendered only when `showMonitorButton`:
```jsx
className="group inline-flex h-10 items-center gap-1.5 rounded-md px-2 transition-colors"
style={{ background: 'transparent', color: monitorButtonTone }}
```
`<Activity className={h-3.5 w-3.5 ${monitorButtonAnimationClass}} />` and a
`text-[11px] font-semibold` span with the same animation class. Both coloured
`monitorButtonTone`:
- `monitorButtonOffline` → `'#f87171'`, label `Offline`, title/aria
  `Selected printer is offline`;
- otherwise `'var(--text-strong)'`, label `Monitor` (msgid comment: "Topbar
  button that opens the live print-progress monitoring view for the active
  printer."), title/aria `Open printer monitor`.

`monitorButtonAnimationClass` (`:512-514`):
`monitorButtonPaused ? 'ui-topbar-monitor-paused' : (monitorButtonActive ? 'ui-topbar-monitor-active' : '')`.
Those classes (`globals.css:297-305`) set `color` `#22c55e` / `#facc15` and run
`uiTopbarMonitorActivePulse` (3.8s ease-in-out infinite:
`#22c55e`/opacity .84 → `#85c59d`/opacity 1) or `uiTopbarMonitorPausedBlink`
(1s: `#facc15` + `drop-shadow(0 0 4px rgba(250,204,21,0.34))` → `#fb923c` +
`drop-shadow(0 0 12px rgba(251,146,60,0.82))`), disabled under
`prefers-reduced-motion`. Because the animation animates `color`, the paused /
active class **overrides** the inline `--text-strong` tone but not the offline
override (offline also carries `ui-topbar-monitor-paused` if paused — the class
is added regardless of offline).
Disabled when `topbarActionsDisabled || !onOpenMonitor`; click → `onOpenMonitor()`.

### Workflow steps (prepare / support / export / printing)

- **Source**: `TopBar.tsx:938-1048` (`steps` array at `:580-621`, render at
  `:966-1047`).
- **Renders**: a centred, flexible row of four 32px-high step buttons.
- **Layout & styling**:
  - outer: `<div className="flex min-w-0 flex-1 justify-center px-2">`
    (`:939`);
  - wrapper: `<div className={"relative w-full transition-opacity " + (topbarActionsDisabled ? 'opacity-45' : '')} aria-disabled=…>`;
  - row: `<div ref={stepsContainerRef} className="flex items-center justify-center gap-2">`;
  - button:
    ```jsx
    className={`group relative flex cursor-pointer items-center gap-1.5 rounded-[5.5px] border px-1.5 py-1.5 transition-all duration-200 flex-1 min-w-[90px] max-w-[190px] h-[32px] ${
      active ? 'shadow-[0_6px_16px_rgba(0,0,0,0.25)]'
             : 'hover:-translate-y-[1px] hover:shadow-[0_6px_14px_rgba(0,0,0,0.18)]'
    } ${visuallyDimmed ? 'opacity-45' : ''} ${disabled ? 'cursor-not-allowed hover:translate-y-0 hover:shadow-none' : ''} ${printingLocked ? 'grayscale saturate-0' : ''}`}
    ```
  - active inline style: `borderColor: 'color-mix(in srgb, var(--accent), white 8%)'`,
    `background: 'color-mix(in srgb, var(--accent), var(--surface-0) 84%)'`;
  - inactive inline style: `borderColor: printingLocked ? 'color-mix(in srgb, var(--border-subtle), black 30%)' : 'var(--border-subtle)'`,
    `background: printingLocked ? 'color-mix(in srgb, var(--surface-2), black 12%)' : 'color-mix(in srgb, var(--surface-1), transparent 4%)'`;
  - icon badge: `<span className="inline-flex h-5 w-5 items-center justify-center rounded-full text-[11px] font-bold">`
    with active `{ color: 'var(--accent)', background: 'transparent' }`, inactive
    `{ color: 'var(--text-muted)', background: 'var(--surface-2)' }`, plus
    `display: 'none'` when `hideBadges`. Icon: `<FolderOpen|Anchor|Upload|Printer className="h-3 w-3" strokeWidth={2.5} />`.
  - label: `<span className="absolute left-1/2 -translate-x-1/2 whitespace-nowrap text-center text-xs font-bold leading-none tracking-[0.01em]" style={{ color: active ? 'var(--text-strong)' : 'var(--text-strong)' }}>`
    — the conditional is dead, both branches are identical;
  - the lock: `{printingLocked && !hideBadges && <Lock className="h-3 w-3 ml-auto flex-shrink-0" style={{ color: 'var(--text-muted)' }} />}`.
- **States**, per step: `active = mode === item.mode`; `locked = item.locked`;
  `disabled = locked || topbarActionsDisabled`; `nativeDisabled = disabled`
  (the button really is `disabled`); `visuallyDimmed = topbarActionsDisabled || locked`;
  `printingLocked = item.mode === 'printing' && locked`.
- **Data** (the four steps, `:580-621`), in order:

  | mode | step | label (msgid) | hint (msgid) | icon | locked when |
  |---|---|---|---|---|---|
  | `prepare` | 1 | `Prepare` | `Arrange model and transforms` | `FolderOpen` | never |
  | `support` | 2 | `Support` | `Build and tune supports` | `Anchor` | `!hasModels` |
  | `export` | 3 | `Export` | `Finalize and export output` | `Upload` | `!hasModels` |
  | `printing` | 4 | `Printing` | `Inspect sliced layers before printing` | `Printer` | `!hasModels || !hasPrintingData` |

  `step` is carried in the array but unused in the render.
- **Callbacks**: `onClick` → `if (disabled) return; onModeChange(item.mode)`.
  The parent (`page.tsx:7463`) refuses any mode other than `prepare` when
  `models.length === 0`, forcing `prepare`.
- **Tooltip text** (`title`, `:1013-1021`), in priority order:
  1. `topbarActionsDisabled` → `Slicing in progress. Topbar actions are temporarily disabled.`
  2. `locked && mode === 'printing'` → `Run slicing in Export to unlock Printing preview`
  3. `locked` → `Load a model in Prepare to unlock this stage`
  4. otherwise `item.hint`.
- **Responsive measurement** (`:553-579`), quoted in full because the numbers
  are asymmetric on purpose:
  ```js
  // The view mode trigger shows the selected shader's name when the bar has room
  // and falls back to its icon when it does not. Its own width feeds the
  // measurement it is based on, so the thresholds are asymmetric: appearing
  // needs more room than staying does.
  const ro = new ResizeObserver(([entry]) => {
    const gapPx = 4;
    const perButton = (entry.contentRect.width - 3 * gapPx) / 4;
    setHideBadges(perButton < 120);
    setShowViewModeLabel((previous) => (previous ? perButton >= 130 : perButton >= 155));
  });
  ```
  Observed from the steps row: `contentRect.width` minus three 4px gaps, over
  four. The effect depends on `hideWorkflowControls` so the observer is
  re-attached when the row remounts ("…the steps container is unmounted while
  the workflow controls are hidden (onboarding), so re-observe it when they come
  back to avoid measuring a detached node (which pinned the icons hidden)").
  `hideBadges` also initialises from `window.innerWidth < 800`.

### Right group

- **Source**: `TopBar.tsx:1050-1149`
- **Layout & styling**:
  ```jsx
  <div className="flex flex-1 max-w-[430px] items-center justify-end gap-2 pr-[3px]">
    <div className={`flex items-center gap-2 transition-opacity ${topbarActionsDisabled ? 'opacity-45 pointer-events-none' : ''}`}>
  ```

**View-mode dropdown** (`:1053-1058`, only when `!hideWorkflowControls`):
```jsx
<ViewTypeDropdown value={viewType} onChange={onViewTypeChange}
  iconOnly={!showViewModeLabel} title={_(msg`View mode`)} />
```

**Overhangs toggle** (`:1059-1070`), atom `Button`:
```jsx
<Button type="button" variant={overhangViewActive ? 'primary' : 'secondary'} className="!p-2"
  onClick={() => onOverhangViewChange(!overhangViewActive)} disabled={topbarActionsDisabled}
  title={overhangViewActive ? _(msg`Overhangs view: On`) : _(msg`Overhangs view: Off`)}
  aria-label={same} data-no-window-drag="true">
  <AlertDiamondIcon className="w-4 h-4" />
</Button>
```
`title`/`aria-label` msgids carry the comments: On — "Tooltip for a toggle
button showing its current state, format \"Feature name: state\". The Overhangs
view colours surfaces by how steeply they overhang, which is where supports are
needed."; Off has no comment.

**Settings** (`:1071-1082`):
```jsx
<Button type="button" variant="secondary" className="!p-2"
  onClick={() => setIsSettingsOpen(true)} disabled={topbarActionsDisabled}
  title={_(msg`Settings`)} aria-label={_(msg`Settings`)} data-no-window-drag="true">
  <Settings className="w-4 h-4" />
</Button>
```
`Button` (`src/components/atoms/Button.tsx`) emits
`className="ui-button <variantClass> <sizeClass> <className>"` with
`variant="secondary"` → `ui-button-secondary`, default `size="md"` →
`!h-9 px-3 text-xs`, then the call site's `!p-2`. `.ui-button` is unlayered CSS
(`globals.css:348-360`: `border: 1px solid transparent; border-radius:
var(--radius-sm); padding: 5px 11px 7px 11px; font-size: var(--font-size-md);
font-weight: 600; line-height: 1; transition: background-color 140ms ease,
border-color 140ms ease, color 140ms ease`), so only the `!`-prefixed
`!h-9`/`!p-2` win: a 36px tall, 34px wide icon button (8+16+8+2 borders).
`.ui-button-secondary` is `background: var(--surface-1); color:
var(--text-strong); border-color: var(--border-subtle)` with hover
`background: var(--surface-2)`; `.ui-button-primary` is
`background: var(--primary-button-surface); color: var(--accent-contrast);
border-color: color-mix(in srgb, var(--primary-button-surface), white 14%)`,
hover `color-mix(in srgb, var(--primary-button-surface), black 12%)`.

**Caption buttons** (`:1087-1148`), only when `isDesktopWindow`:
```jsx
<div className="ml-1 flex items-center gap-1" aria-label={_(msg`Window controls`)}>
```
Three buttons, all
`className="inline-flex h-8 w-8 items-center justify-center rounded-[var(--radius-sm)] border transition-colors"`,
with light/dark inline styles:

| button | icon | dark `borderColor` / `background` / `color` | light `borderColor` / `background` / `color` |
|---|---|---|---|
| minimize (`handleDesktopWindowMinimize`) | `Minimize2` | `color-mix(#f4bf4f, var(--border-subtle) 55%)` / `color-mix(#f4bf4f, var(--surface-1) 86%)` / `color-mix(#f4bf4f, var(--text-strong) 16%)` | `color-mix(#c8920a, var(--border-subtle) 35%)` / `color-mix(#c8920a, var(--surface-1) 50%)` / `var(--text-strong)` |
| maximize / restore (`handleDesktopWindowToggleMaximize`) | `Square` when maximized else `Maximize2` | `color-mix(#40c463, var(--border-subtle) 55%)` / `color-mix(#40c463, var(--surface-1) 86%)` / `color-mix(#40c463, var(--text-strong) 16%)` | `color-mix(#1a7a3a, var(--border-subtle) 35%)` / `color-mix(#1a7a3a, var(--surface-1) 50%)` / `var(--text-strong)` |
| close (`handleDesktopWindowClose`) | `X` | `color-mix(#ff6b6b, var(--border-subtle) 55%)` / `color-mix(#ff6b6b, var(--surface-1) 88%)` / `color-mix(#ff6b6b, var(--text-strong) 18%)` | `color-mix(#c0160a, var(--border-subtle) 35%)` / `color-mix(#c0160a, var(--surface-1) 50%)` / `var(--text-strong)` |

Titles/aria: `Minimize`/`Minimize window`, `Restore`|`Maximize`/`Restore window`|`Maximize window`, `Close`/`Close window`.

`isDesktopWindow` is set by a mount effect (`:229-269`) that treats
`tauri:`/`file:`/`tauri.localhost`/`__TAURI_INTERNALS__` as desktop and then
awaits `getCurrentWindow().isMaximized()`.

### Surface: printer badge

- **Source**: `TopBar.tsx:752-804`; label logic `:460-511`.
- **Renders**: a 40px-tall badge: 32px thumbnail tile, two-line label,
  chevron.
- **Layout & styling**:
  - button: `className="group inline-flex h-10 max-w-[300px] items-center gap-2 rounded-md px-2 transition-colors"`, `style={{ background: 'transparent' }}`;
  - thumbnail tile: `className="inline-flex h-8 w-8 items-center justify-center overflow-hidden rounded-sm shrink-0"`, `style={{ background: 'color-mix(in srgb, var(--surface-1), transparent 6%)', ...(hideBadges ? { display: 'none' } : {}) }}`;
    inside: the profile's `imageDataUrl` as `<img className="h-full w-full object-contain">`
    (`alt` = profile name or `Selected printer`, `onError` sets
    `printerThumbnailFailed`, which is reset whenever the profile id or image
    changes), else `<Printer className="h-4 w-4" style={{ color: 'var(--text-muted)' }} />`;
  - label column: `className="min-w-0 flex flex-col items-start leading-none gap-[2px]"`;
  - top line: `className={topbarUsesFleetLabelOrder ? 'truncate text-[11px] tracking-[0.01em]' : 'text-[11px] uppercase tracking-[0.11em]'}`, `style={{ color: 'var(--text-muted)' }}`;
  - bottom line: `className="truncate text-[12px] font-semibold"`, `style={{ color: 'var(--text-strong)' }}`;
  - chevron: `<ChevronDown className={"h-3.5 w-3.5 ml-auto shrink-0 transition-transform " + (isPrinterQuickMenuOpen ? 'rotate-180' : '')} style={{ color: 'color-mix(in srgb, var(--text-muted), white 8%)' }} />`.
- **Data / derived text**: `profileState` from
  `useSyncExternalStore(subscribeToProfileStore, …)`; `activePrinterProfile`.
  - `topbarFleetPrinterName` = the fleet unit matching `activeNetworkDeviceId`,
    else the first `connected` one, else `fleet[0]`, rendered as
    `displayName || hostName || ipAddress || null`.
  - `topbarUsesFleetLabelOrder` = `networkFleet.length > 1`.
  - `topbarPrinterLabelTop` = fleet ? `profile.name ?? "Select profile"` :
    `msg("Printer")` (comment: "Static field-label shown above the printer name
    in the topbar badge (like \"Printer: <name>\"), not an instruction. Only
    shown when a profile has a single printer, so there is no fleet to pick
    from.").
  - `topbarPrinterLabelBottom` = fleet ? `fleetPrinterName ?? "No active printer"`
    : `profile.name ?? "Select printer"`.
  - `title` = fleet ? `formatTopbarPrinterTitle(profile, printer)` :
    profile ? `Printer profile: ${name}` : `Select printer profile`.
  - `aria-label` = fleet ? `formatTopbarPrinterAriaLabel(...)` : profile ?
    `Printer profile ${name}` : `Select printer profile`.
  `topBarMessages.ts` holds the two interpolated msgids, module-level on purpose:
  > "React Compiler renames locals inside components (`profileName` -> `profileName_0`) before the Lingui macro derives the message id, so an interpolation written inside `TopBar` gets an id the compiled catalogue does not contain and falls back to the raw source string in production builds."
  Messages:
  `Printer profile: {0} • Active printer: {1}` and
  `Printer profile {0}, active printer {1}` (comments distinguish *profile* =
  saved configuration from *active printer* = connected physical device).
- **States**: thumbnail present/absent, `hideBadges` (tile hidden),
  quick-menu open (chevron flipped, but background stays `transparent` — unlike
  the logo button there is no open-state tint), disabled
  (`topbarActionsDisabled`).
- **Callbacks**: click → if `hasTopbarFleetUnits` (`topbarFleetUnits.length > 1`,
  where `topbarFleetUnits` = `networkFleet` filtered to units with a non-blank
  `ipAddress`) toggle the quick menu; otherwise `requestOpenProfileSettings('printer')`.
- **Port target**: `app_bar::PrinterBadge` + the `printer_button` block. The
  badge's label rules (`uses_fleet_label_order`, `fleet_printer_name`,
  `label_top`, `label_bottom`, `has_fleet_units`) are already unit-tested there.
  The profile store is not ported, so the port renders the empty-store label and
  never takes the fleet branch (documented deviation 15).

### Surface: app menu (logo dropdown)

- **Source**: entries `TopBar.tsx:624-663`, dispatch `:665-699`, render `:835-847`.
- **Renders**: a `ContextMenu` at `{x: rect.left, y: rect.bottom + 6}` with
  `title="DragonFruit"`, `ariaLabel=msg("DragonFruit app menu")` and
  `dismissIgnoreRef={appMenuButtonRef}` (so the logo button's own click closes
  the menu instead of the dismiss handler eating it).
- **Entries** (id → label, icon, disabled, group):

  | id | label (msgid) | icon | disabled when | `startsGroup` |
  |---|---|---|---|---|
  | `new-scene` | `New scene` | `FilePlus2` | `topbarActionsDisabled \|\| !onNewScene \|\| !hasModels` | — |
  | `save-scene` | `Save scene` | `Save` | `topbarActionsDisabled \|\| !onSaveScene` | ✔ |
  | `save-scene-as` | `Save scene as…` | `SaveAll` | `topbarActionsDisabled \|\| !onSaveSceneAs` | — |
  | `open-scene` | `Open scene…` | `FolderOpen` | `topbarActionsDisabled \|\| !onOpenScene` | — |
  | `import-mesh` | `Import mesh…` | `Upload` | `topbarActionsDisabled \|\| !onLoadMeshChange` | ✔ |
  | `import-scene` | `Import scene…` | `FolderInput` | `topbarActionsDisabled \|\| !onImportSceneChange` | — |
  | `close-program` | `Close program` | `Power` | never | ✔ |

  Note `new-scene` is disabled when there are no models — "New scene" is not a
  way to clear an empty scene.
- **Callbacks**: `new-scene`→`onNewScene?.()`, `save-scene`→`onSaveScene?.()`,
  `save-scene-as`→`onSaveSceneAs?.()`, `open-scene`→`onOpenScene?.()`,
  `import-mesh`→`document.getElementById('topbar-mesh-input').click()`,
  `import-scene`→`…('topbar-scene-input').click()`,
  `close-program`→`handleCloseProgram()` (which prefers the `onCloseProgram`
  prop, else closes the Tauri window). Every selection also closes the menu
  (`ContextMenu` calls `onClose` after `onSelect`).
- **Port target**: `app_bar::app_menu`.

### Surface: printer fleet quick menu

- **Source**: `TopBar.tsx:864-936`; positioning/dismissal `:311-349`.
- **Renders**: a portal-free fixed popover (280px) listing fleet units, then a
  "Show manager" footer row.
- **Layout & styling**:
  ```jsx
  className="fixed z-[121] w-[280px] rounded-lg border p-1.5 shadow-xl backdrop-blur-sm"
  style={{ left, top, borderColor: 'var(--border-subtle)',
           background: 'color-mix(in srgb, var(--surface-0), #000 10%)' }}
  role="menu" aria-label={_(msg`Fleet quick switch`)}
  ```
  - heading: `className="mb-1 px-2 py-1 text-[10px] font-semibold uppercase tracking-wide"`, `color: var(--text-muted)`, text `Fleet units`;
  - list: `className="max-h-[260px] overflow-y-auto custom-scrollbar space-y-0.5"`;
  - row: `className="flex w-full items-center gap-2 rounded-md border px-2 py-1.5 text-left text-[12px] transition-colors"`, `role="menuitem"`, `title={device.ipAddress || undefined}`, with three style states:
    - offline: `borderColor: color-mix(in srgb, var(--border-subtle), black 18%)`, `background: color-mix(in srgb, var(--surface-1), black 8%)`, `opacity: 0.55`, and `disabled`;
    - active: `borderColor: color-mix(in srgb, var(--accent), var(--border-subtle) 30%)`, `background: color-mix(in srgb, var(--accent), var(--surface-1) 89%)`, `color: var(--text-strong)`;
    - other: `borderColor: var(--border-subtle)`, `background: var(--surface-1)`, `color: var(--text-muted)`;
  - row thumbnail: `className="inline-flex h-6 w-6 items-center justify-center overflow-hidden rounded-sm shrink-0"` with `background: color-mix(in srgb, var(--surface-1), transparent 6%)`, same `<img>`/`Printer` fallback as the badge (the *profile's* image, not a per-device one);
  - name: `className="block truncate font-semibold"`;
  - sub: `className="block truncate text-[10px]"`, `color: var(--text-muted)`, content `` `${device.ipAddress || _('No IP')} • ${isOffline ? _('Offline') : _('Online')}` `` (a literal `•` with spaces around it);
  - "Active" badge: `className="text-[10px] rounded-full border px-1.5 py-0.5"` with
    `borderColor: color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 45%)`,
    `color: var(--accent-secondary)`,
    `background: color-mix(in srgb, var(--accent-secondary), var(--surface-1) 92%)`;
  - footer: `<div className="mt-1 border-t pt-1" style={{ borderColor: 'var(--border-subtle)' }}>` with a
    `className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[12px] font-semibold transition-colors"`,
    `style={{ color: 'var(--accent-secondary)' }}`, `<ChevronDown className="h-3.5 w-3.5 rotate-[-90deg]" />`
    and the label `Show manager` (comment: "Opens the printer manager screen
    (printer profile settings), reached from this quick-switch popover.").
- **Data**: `topbarFleetUnits`, `activePrinterProfile.activeNetworkDeviceId`,
  `printerReachabilityByDeviceId` (`false` ⇒ offline; `null`/absent ⇒ online),
  `device.displayName || device.hostName || device.ipAddress || msg("Printer " + id)`
  (that fallback msgid's comment explains `{0}` is an internal id).
- **Callbacks**: row → `if (isOffline) return; handleSelectFleetUnitFromQuickMenu(id)`
  → `selectPrinterNetworkDevice(profile.id, deviceId)` then close. Footer →
  `handleOpenPrinterManagerFromQuickMenu` → close, then
  `requestOpenProfileSettings('printer')`.
- **Dismissal** (`:325-349`): a window `mousedown` listener closes it unless the
  target is inside `[data-printer-quick-menu="true"]` or inside the badge
  button; also closes on `app-hotkey-keydown` with `detail.key === 'Escape'`.
  This is the *manual* equivalent of `useOutsideDismiss` (no resize/scroll
  dismissal, no dialog-stack registration).
- **Port target**: `app_bar::printer_menu` (rows, offline/active recipes, footer).

### Surface: hidden file inputs

- **Source**: `TopBar.tsx:848-862`.
- `#topbar-mesh-input`: `type="file" accept=".stl,.obj,.3mf,.zip" multiple onChange={onLoadMeshChange} className="hidden"`.
- `#topbar-scene-input`: `type="file" accept={sceneFileAccept} onChange={onImportSceneChange} className="hidden"` (no `multiple`).
  `sceneFileAccept` comes from `useSceneFileInputAccept()`
  (`src/features/plugins/pluginFileTypeExtensions.ts:220-228`): built-in `.voxl`
  plus plugin-gated scene extensions, server-rendered as `.voxl,.zip` and
  corrected after hydration to avoid a mismatch.
- Nothing else in `TopBar` references them; only the app-menu rows click them.
- **Port target**: `app_bar.rs` opens the native file dialog directly
  (`on_import_scene` / `on_open_mesh`); the source's accept strings are the
  contract for what the dialog offers.

### Surface: re-slice warning dialog

- **Source**: `TopBar.tsx:1151-1190`; trigger `requestOpenProfileSettings`
  (`:527-535`).
- **Renders**: a `StructuredDialogModal` (single-paragraph body, two
  full-width buttons).
- **Props**: `open`, `ariaLabel=msg("Changing printer profile requires re-slice")`,
  `title=msg("Re-slice required after profile change")`,
  `subtitle=msg("Changing print settings invalidates the current sliced file.")`,
  `icon=<AlertTriangle className="h-4 w-4" />`, `iconTone="warning"`,
  `zIndexClassName="z-[220]"`, `closeAriaLabel=msg("Close warning")`,
  `onClose={() => setShowProfileChangeWarning(false)}`.
- **Layout & styling**: from `StructuredDialogModal` (below) plus
  `<p className="text-xs leading-relaxed" style={{ color: 'var(--text-muted)' }}>`
  for the body; `actions` are `secondary` `Keep current profiles` and
  `tinted-warning` `Continue`, both `size="md" className="w-full"` inside the
  default `grid grid-flow-col auto-cols-fr gap-2 pt-1`.
- **States**: shown only when `warnBeforeProfileSettingsOpen &&
  showProfileChangeWarning`; `warnBeforeProfileSettingsOpen` is
  `Boolean(printingArtifact && !isActiveSliceStale)` (`page.tsx:10711`), i.e.
  only when a *valid* slice exists.
- **Callbacks**: `Keep current profiles` → just close (profile untouched);
  `Continue` → close, then `openProfileSettings(profileModalTab)` (the tab the
  blocked request asked for). Escape also closes (`useEscapeToClose` inside the
  modal).
- **Text** (no comment on the body):
  - title `Re-slice required after profile change` — comment: "'Slice'/'re-slice'
    is the 3D-printing step that converts a model into printer instructions
    (G-code). This dialog warns that switching printer profile invalidates the
    already-sliced file."
  - subtitle `Changing print settings invalidates the current sliced file.`
  - body `You can continue to adjust profiles, but you’ll be prompted to re-slice before printing with the updated settings.`
    (note the typographic apostrophe in `you’ll`).
  - buttons `Keep current profiles` (comment: "Cancels the pending profile
    change and closes this warning dialog…"), `Continue` (comment: "Confirms
    proceeding with the profile change despite the re-slice warning above…").
- **Port target**: `app_bar.rs` deviation 22 — absent; there is no profile
  switching in the port yet. When there is, this is
  `gpui_ui::ui::dialog_modal(_full)` at `z` above the modals (`z-[220]`).

### Cross-cutting TopBar behaviour

- **Event buses it listens to** (all `window` custom events):
  - `OPEN_PROFILE_SETTINGS_MODAL_EVENT` (`:403-432`): detail
    `{ tab?, openPrinterLibrary?, openNetworkSettings?, openMaterialAntiAliasing? }`;
    sets the tab (`printer`|`material`, default `printer`), bumps a token per
    requested sub-panel, opens, and dispatches
    `dispatchProfileSettingsModalOpenChange(true)`.
  - `OPEN_SETTINGS_MODAL_EVENT` (`:434-457`): detail `{ tab? }`; the tab is
    accepted only if it is one of
    `general, camera, workspaces, multiPlate, mesh, performance, spacemouse, ui,
    hotkeys, plugins, experiments, sceneAutosave, backups, uvtools, logging,
    updates, about`, else `general`.
  - On close of `SettingsModal` the initial tab is reset to `general`;
    `ProfileSettingsModal` dispatches the open-change event with `false`.
- **Theme bootstrap** (`:194-227`): writes `data-theme` from
  `getSavedThemePreference()` (`dark`/`light`, removed otherwise), applies
  `applyThemeCustomColors(getSavedThemeCustomColors())`, and observes the
  attribute plus `prefers-color-scheme` to maintain `isLightTheme` (used only for
  the logo drop-shadow and the caption tints).
- **Profile hydration**: `hydrateProfilesFromStorage()` on mount.
- **Pass-through props**: all the mesh/shader/view settings and
  `slicingThumbnailRenderSettings` are handed straight to `SettingsModal`; they
  are the Settings area's contract, not the bar's. Same for
  `ProfileSettingsModal`'s `initialTab` + three open-tokens.

---

## Surface: `ToolRail` — the tool rail

- **Source**: `src/components/controls/ToolRail.tsx` (587 lines); constants at
  `:16` and `:19`, styles `:370-405`, component `:420-586`.
- **Renders**: a `role="toolbar"` container (`fixed`) holding one
  `Tooltip`-wrapped 54×54 tile per entry, plus a `ContextMenu` for the layout.
- **Layout & styling**:
  ```jsx
  className={`${layout === 'vertical'
    ? 'fixed left-1.5 top-[var(--topbar-height)] bottom-0 z-40 flex w-[54px] flex-col items-stretch gap-1.5 overflow-y-auto py-1.5 custom-scrollbar pointer-events-auto'
    : 'fixed left-1/2 top-[calc(var(--topbar-height)+4px)] z-40 flex -translate-x-1/2 flex-row items-stretch gap-1.5 overflow-x-auto px-1.5 custom-scrollbar pointer-events-auto'}${isFolding ? ' !overflow-visible' : ''}`}
  data-no-drag="true" data-no-window-drag="true" role="toolbar"
  aria-orientation={layout === 'vertical' ? 'vertical' : 'horizontal'}
  aria-label={_(msg`Editor tools`)}
  ```
  `TILE_SIZE_PX = 54`; `export const TOOL_RAIL_WIDTH_PX = TILE_SIZE_PX + 12;`
  (66px — the tile plus `left-1.5` on each side; the parent uses it as the panel
  stack's left inset).
- **Tile recipes** (`:370-405`), quoted:
  ```ts
  function railEntryStyles(hue: string) {
    return {
      off: {
        background: `color-mix(in srgb, ${hue} 10%, var(--surface-1))`,
        borderColor: `color-mix(in srgb, ${hue} 12%, var(--border-subtle))`,
        color: 'var(--text-muted)',
      },
      on: {
        background: `color-mix(in srgb, ${hue}, var(--surface-0) 78%)`,
        borderColor: `color-mix(in srgb, ${hue}, white 14%)`,
        color: 'var(--text-strong)',
        boxShadow: `0 0 0 1px color-mix(in srgb, ${hue}, transparent 74%) inset`,
      },
    };
  }
  const TOOL_STYLES = railEntryStyles('var(--accent)');
  const PANEL_STYLES = railEntryStyles('var(--accent-secondary)');
  const DISABLED_STYLES: React.CSSProperties = {
    background: 'var(--surface-1)',
    borderColor: 'var(--border-subtle)',
    color: 'var(--text-muted)',
    // Dimmed here rather than with an `opacity-*` class: this Tailwind's scale has
    // no 45, so the class would be dropped and the tile would only look muted.
    opacity: 0.45,
  };
  const TILE_BASE = 'flex h-[54px] items-center justify-center rounded-[5.5px] border transition-[color,background-color,border-color,filter,box-shadow] duration-150 hover:brightness-110';
  ```
  The comment above `TILE_BASE`: "Only colours and the hover brightness are
  transitioned. `transition-all` also watched width and transform, so the layout
  swap made the browser tween the entry's own box (a column entry is a scrollbar
  narrower than `w-[54px]`, and the scrollbar goes away in the bar) at the same
  time as the fold animation was driving transform — two engines animating one
  property is the glitch."
- **Per-entry geometry** (`:519-530`):
  - `lit = entry.invertLit === true ? !entry.active : entry.active`;
  - `iconColor = !lit ? 'var(--text-muted)' : entry.tone === 'panel' ? 'var(--accent-secondary)' : 'var(--accent)'`;
  - `separation = separated === 'below' ? (vertical ? ' mb-3' : ' mr-3') : separated === 'above' ? (vertical ? ' mt-3' : ' ml-3') : ''`;
  - `geometry = `${vertical ? 'w-full' : 'w-[54px] shrink-0'}${separation}`` — carried by the **tooltip wrapper**, so tile + hint are one flex child;
  - button: `className={`${TILE_BASE} w-full${isDisabled ? ' cursor-not-allowed' : ''}`}`,
    `style = isDisabled ? DISABLED_STYLES : lit ? styles.on : styles.off`,
    `aria-label={_(entry.label)}`, `aria-pressed={entry.active}`,
    `data-rail-entry="true"`, `<Icon className="h-7 w-7" style={{ color: isDisabled ? 'var(--text-muted)' : iconColor }} />`.
- **States**: `disabled` (inert, `disabled` attribute, muted recipe at
  `opacity: 0.45`), lit/unlit (`active` XOR `invertLit`), hover
  (`hover:brightness-110` on the whole tile), layout × `isFolding`
  (`!overflow-visible`).
- **Layout fold animation** (`:436-490`) — `useLayoutEffect` on `[layout]`:
  measures each `[data-rail-entry="true"]`, FLIP-animates from the previous
  rects with
  ```js
  { offset: 0,   transform: `translate(${dx}px, ${dy}px) rotate(0deg)`,     opacity: 0.85 }
  { offset: 0.5, transform: `translate(${dx*0.35}px, ${dy*0.35}px) rotate(${foldDegrees}deg)`, opacity: 0.4 }
  { offset: 1,   transform: 'translate(0px, 0px) rotate(0deg)',            opacity: 1 }
  ```
  duration 240ms, `delay: index * 8`, `easing: 'cubic-bezier(0.16, 1, 0.3, 1)'`,
  `fill: 'backwards'`, and `foldDegrees = layout === 'horizontal' ? 26 : -26`.
  Source comments to keep: "The rail is a scroll container, so `isFolding` lifts
  the clip for the length of the flight: a turned entry is wider than the 54px
  column and would otherwise be cut off against its own box." and "A lean, not a
  quarter turn: a tile rotated 90° reads as a diamond with sideways text."
  `isFolding` clears when every animation finishes.
- **Context menu** (`:493-501`, `:578-585`): right-click anywhere on a tile
  (`event.preventDefault()`, position = `clientX/clientY`) opens a `ContextMenu`
  with `title` and `ariaLabel` both `Tool layout` and two checked rows:
  `Vertical` / `Horizontal`. `onSelect` → `onLayoutChange(id === 'horizontal' ? 'horizontal' : 'vertical')`.
- **Tooltip per tile** (`:534-573`): `content` = `<span className="block text-center">{label}<span className="block" style={{color:'var(--text-muted)'}}>{hint}</span></span>`,
  `width={140}`, `anchor="trigger"`, `placement={vertical ? 'right' : 'below'}`,
  `offsetY={8}`, `wrapperClassName={geometry}`.
- **Data**: `entries` (from `toolRailEntries.ts`) and `layout` (caller-owned
  preference).
- **Callbacks**: tile `onClick={entry.onSelect}`; hover
  `onMouseEnter/onMouseLeave` → `entry.onHover?.(true|false)` (skipped when
  disabled) and `onBlur` → `entry.onHover?.(false)`.
- **Text**: `Editor tools` (aria), `Tool layout` (menu title/aria), `Vertical`,
  `Horizontal`.
- **Port target**: `gpui/src/chrome/rail.rs` — complete for both layouts,
  including the tile recipes, group gaps, the 140px tooltip, the right-click
  layout menu, and the Models toggle. Deviations are listed in the file header
  (no transitions/fold animation, no `brightness()` filter, no portals so no
  tooltip flip/clamp, no aria/roles/focus reveal, no backdrop blur, right-click
  opens on mouse-down). Known gaps against the call site: the rail renders
  unconditionally in `main.rs` (the source gates it on `models.length > 0`), the
  layout default is `Vertical` where the source defaults to `horizontal`, and
  the Prepare tool is not reset/restored per workspace when entering Support.

---

## Surface: `toolRailEntries` — what each mode lists

- **Source**: `src/components/controls/toolRailEntries.ts` (whole file).

### Prepare (`buildPrepareToolRailEntries`, `:26-100`)

Order, with `separated` and tone; every tool is `tone: 'tool'` and
`onSelect: () => onModeChange(mode)`:

| # | mode | label msgid (comment) | hint msgid | icon | separated |
|---|---|---|---|---|---|
| 0 | `models` *(optional)* | `Models` | `Show or hide the model list` | `Boxes` | `below` |
| 1 | `select` | `Drag` — "The tool that drags and places models on the plate. Deliberately 'Drag' rather than 'Arrange': the app uses 'Arrange' elsewhere for arranging the whole plate, and Spanish distinguishes the two." | `Move and place models on the plate` | `Hand` (lucide) | — |
| 2 | `transform` | `Transform` | `Move, rotate, and scale` | `Move3D` (lucide) | — |
| 3 | `placeOnFace` | `On-Face` — "Short for \"lay the model flat on a selected face\"; keep it terse so the rail entry stays one line." | `Orient flat against plate` | `PlaceOnFaceIcon` | — |
| 4 | `mirror` | `Mirror` | `Mirror across X, Y, or Z` | `FlipHorizontal2` (lucide) | — |
| 5 | `duplicate` | `Duplicate` | `Array copies across the plate` | `Copy` (lucide) | `above` |
| 6 | `arrange` | `Arrange` — "The tool that auto-packs the models onto the plate." | `Arrange models on the plate` | `LayoutGrid` (lucide) | — |
| 7 | `organicCut` | `Split` — "Splits the model along a drawn seam." | `Split the model along a drawn seam` | `CutSeamIcon` | `above` |
| 8 | `smoothing` | `Smooth` — "The tool brushes and smooths local surface regions." | `Sculpt and smooth surface` | `SmoothingSphereIcon` | — |

Source comment for the group boundaries: "Three groups: orient the model, lay
out copies, then edit its surface."

The Models entry only exists when `onToggleModelsPanel !== undefined` ("The
list's tile is a column affordance — the bar's caller passes no toggle, so no
tile is built for it."). Its shape:
`{ id: 'models', label, hint, icon: Boxes, active: modelsPanelVisible === true, invertLit: true, tone: 'panel', separated: 'below', onSelect: onToggleModelsPanel }`.

`onHover` is attached to **every tool** (`onHover: (entering) => onModeHover?.(entering ? tool.mode : null)`) but not to the Models entry.

### Support (`buildSupportToolRailEntries`, `:145-231`)

Order:

| # | id | label msgid (comment) | hint msgid (comment) | icon | separated | active |
|---|---|---|---|---|---|---|
| 0 | `models` *(optional)* | as Prepare | as Prepare | `Boxes` | `below` | `modelsPanelVisible === true`, `invertLit` |
| 1 | `islands` | `Islands` — "Short for the Islands panel, which scans the model for unsupported islands." | `Scan the model for unsupported islands` | `IslandPalmIcon` | — | `!hollowingActive && mode === 'islands'` |
| 2 | `manual` | `Manual` — "Short for the manual support settings panel (tip, shaft, roots, bracing)." | `Support settings and presets` | `PencilRuler` (lucide) | `above` | `!hollowingActive && mode === 'manual'` |
| 3 | `auto` | `Automatic` — "Short for the Auto Supports panel, which generates supports automatically." | `Auto support generation` | `AutoSupportIcon` | — | `!hollowingActive && mode === 'auto'` |
| 4 | `hollowing` | `Hollow` — "One word, like the other tool names: opens the hollowing tool (shell the model, add drainage holes)." | `Create cavity or open-face shell` | `HollowShellIcon` | `above` | `hollowingActive` |
| 5 | `interiorView` | `Interior View` — "Toggles looking inside a hollowed model instead of at its outside." | `interiorViewAvailable ? "Look inside the hollow, at the cavity and its drain holes" : "Hollow the model first to look inside it"` | `InteriorViewIcon` | — | `interiorView`, `disabled: !interiorViewAvailable` |
| 6 | `viewMode` | `View Mode` — "Toggles the support display between everything drawn solid and contact discs with line members." | `How the support forest is drawn` | `viewMode === 'lines' ? LineSupportIcon : SolidSupportIcon` | `above` | `viewMode === 'lines'` |

`viewMode`'s `onSelect` toggles: `onViewModeChange(viewMode === 'lines' ? 'full' : 'lines')`.
The icon of the *current* mode is the tile's icon ("so the rail answers 'how am
I looking at this?' at a glance").

Source header comments worth keeping verbatim:
> "A panel tool *selects* — its panel is the one shown, the others are hidden — and the model list is a panel toggle, because the list is mounted in both modes and only its visibility changes."
> "The labels are only ever read in a tile's tooltip, so they name the tool rather than fitting a box, and the comments give each one the context its translation needs."
> For Hollowing: "selecting it sets `transformMgr.transformMode` and shows the hollowing workflow. While it is active the panel tools are deselected but their mode is kept, so coming back lands on the panel the user left."

### Call site (`src/app/page.tsx:8595-8613`, `:11438-11470`)

- `modelsPanelShown = !showEmptySceneDialog && (modelsPanelVisible || toolLayout === 'horizontal')` — the bar always shows the list.
- `modelsRailToggle` is `{}` unless `toolLayout === 'vertical'`; then it is
  `{ modelsPanelVisible, onToggleModelsPanel }` which flips state **and** persists
  (`setModelsPanelVisibleEnabled(next)`).
- The rail itself renders only when
  `scene.models.length > 0 && (mode === 'prepare' || mode === 'support')`.
- Support's `onModeChange` also leaves Hollowing:
  ```js
  setSupportRailMode(nextMode);
  // Picking a panel tool leaves the Hollowing transform mode, so its
  // panel gives way to the selected one.
  if (transformMgr.transformMode === 'hollowing') setTransformModeWithMirrorFinalize('select');
  ```
- Prepare passes `mode: transformMgr.transformMode`, `onModeChange:
  setTransformModeWithMirrorFinalize`, `onModeHover: handleToolRailHover`.
- Support passes `viewMode: supportDisplaySettings.navigationDiscsOnly ? 'lines' : 'full'`,
  `onViewModeChange: (m) => updateNavigationDiscsOnly(m === 'lines')`,
  `interiorViewAvailable: hasCavityGeometry`,
  `interiorView` (page state), `onToggleInteriorView: () => setInteriorView(p => !p)`.

### The nine hand-drawn rail icons (`ToolRail.tsx:80-357`)

Each is an inline `<svg viewBox="0 0 24 24">` on the 24-grid with
`strokeWidth={2}`, `stroke="currentColor"`, `aria-hidden="true"`, and each has a
comment explaining its geometry. Names: `HollowShellIcon` (`:80`),
`InteriorViewIcon` (`:106`), `SolidSupportIcon` (`:139`), `LineSupportIcon`
(`:172`), `SmoothingSphereIcon` (`:215`), `CutSeamIcon` (`:246`),
`PlaceOnFaceIcon` (`:290`), `IslandPalmIcon` (`:306`), `AutoSupportIcon`
(`:342`).

Load-bearing details a porter must not "simplify":
- `HollowShellIcon` is filled `evenodd`, one path.
- `InteriorViewIcon` uses `strokeDasharray="0.01 3.585"` on the outer square —
  "The gap divides the outline's 71.7-unit perimeter into exactly 20, so the
  pattern closes with no seam where the path starts." — plus a filled inner rect
  `x=6.5 y=6.5 w=11 h=11 rx=1.4`.
- `SolidSupportIcon` and `LineSupportIcon` share
  `<g transform="translate(2.5 -1) scale(1.1)">` "so the pair stays one shape and
  the two tiles do not shift against each other as the mode changes".
- `LineSupportIcon`'s dashes are drawn as separate butt-capped paths, not a dash
  pattern: "a round cap adds half the stroke width to each end of a dash, and at
  these lengths that closes the gap and the shaft reads as one solid line again."
- `IslandPalmIcon` and `AutoSupportIcon` bake their scale into the path data
  instead of using `<g transform>`, because "a transform would scale the stroke
  with the geometry, so the group would have to set its own width to keep the
  rail's 2". `AutoSupportIcon` is lucide `bot` scaled 1.07 about the box centre.
- `CutSeamIcon` uses a `<mask id="cut-seam-mask">`; the mask id is global — fine
  in one document, a collision if two rails ever render.
- `PlaceOnFaceIcon` is documented as the replacement for lucide
  `ArrowDownToLine`, which "reads as 'download' or 'drop to bottom'".

- **Port target**: `rail.rs` keeps `id/label/hint/icon` in
  `PREPARE_TOOLS`/`SUPPORT_PANELS` const tables and loads the icons by slug from
  `gpui/assets/icons` (all nine are generated assets; `alert-diamond-custom` is
  the tenth, for the Overhangs toggle).

---

## Surface: `Tooltip`

- **Source**: `src/components/ui/Tooltip.tsx` (whole file).
- **Renders**: a wrapper `<span>` around one child plus, while shown, a
  `createPortal`-ed fixed box on `document.body`.
- **Layout & styling**:
  ```jsx
  className="fixed pointer-events-none z-[9999] rounded px-2 py-1.5 text-[11px] leading-tight font-medium shadow-lg"
  style={{ left: 0, top: 0, visibility: 'hidden',
           background: 'rgba(24, 24, 24, 0.98)',
           color: 'var(--text-strong, #e0e0e0)',
           border: '1px solid var(--accent, #baf72e)',
           width: width ?? 'max-content', maxWidth, whiteSpace: 'normal',
           textAlign: 'left',
           boxShadow: '0 6px 32px 0 rgba(0,0,0,0.44), 0 1.5px 8px 0 rgba(0,0,0,0.28)' }}
  ```
  `rounded` is Tailwind's bare 4px. Wrapper: `className={wrapperClassName ? `inline-flex ${wrapperClassName}` : 'inline-flex'}`,
  or with `fullWidth` `style={{ display: 'flex', width: '100%', minWidth: 0 }}`.
- **Defaults**: `offsetY = 28`, `maxWidth = 260`, `delayMs = 0`,
  `anchor = 'cursor'`, `placement = 'below'`, no `width`.
- **States / placement**:
  - `content` falsy → `return children` unwrapped (no span, no listeners);
  - `cursor` anchor: appears at `{clientX, clientY}`, follows `mousemove`, box
    centred under the pointer (`x = cursor.x - w/2`, `y = cursor.y + gap`); flips
    above if `overflowBottom`;
  - `trigger` anchor: box centred on the trigger, beside it for `placement:
    'right'` (`x = trigger.right + gap`, `y = centred`) or under it for
    `'below'` (`y = trigger.bottom + gap`); flips for
    `placement === 'right' ? overflowRight : overflowBottom`;
  - both: `clampToViewport(..., { margin: 4 })`, and a final
    `top < 4 ? 4 : top`;
  - `show = hovered && (anchor === 'trigger' || pos !== null)` — trigger mode
    needs no cursor, which is what lets keyboard focus (`onFocus`) reveal it;
  - placement is written **onto the element** (`el.style.left/top`,
    `visibility = 'visible'`) in a `useLayoutEffect`, not held in state:
    "measuring it and moving it is work on the DOM, and doing it here — before
    the browser paints — keeps the box from ever showing at a guessed spot,
    without spending a render on a position only the browser needs";
  - `delayMs` gates `hovered` behind a `setTimeout`; unmount clears the timer.
- **Why the portal** (comment, quoted): "`position: fixed` resolves against the
  nearest ancestor that carries a transform, filter or backdrop-filter — and
  inside the settings modal that is the panel itself, which also clips with
  overflow:hidden. Left in place the popover is positioned against the panel and
  then clipped away: present in the DOM, reported visible, and painted nowhere."
- **Data**: none; content is a node.
- **Callbacks**: none.
- **Port target**: `gpui_ui::ui::{tooltip, tooltip_with, tooltip_placed,
  mouse_tooltip, mouse_tooltip_default}` (`widgets/tooltip.rs`). `tooltip_with`
  takes the offset/max-width; `tooltip_placed` adds an anchor side; the
  cursor-following variants are the `mouse_tooltip*` pair. No viewport clamp and
  no flip (documented in the file), which is why the rail pins its tooltip to
  the trigger rather than the pointer.

---

## Surface: `ContextMenu` — the one menu widget

- **Source**: `src/components/ui/ContextMenu.tsx` (386 lines).
- **Renders**: `createPortal` into `document.body`: a `role="menu"` panel at the
  clamped position, a heading row, rows in `space-y-0.5`, separators, and
  optionally one open submenu flyout.
- **Layout & styling**:
  - root: `className={`fixed ${zIndexClassName} ${widthClassName} ${MENU_SURFACE}`}`
    with defaults `widthClassName = 'w-48'` (192px) and
    `zIndexClassName = 'z-[130]'`; `MENU_SURFACE = 'rounded-lg border p-1.5 shadow-xl backdrop-blur-sm'`;
    `MENU_SURFACE_STYLE = { borderColor: 'var(--border-subtle)', background: 'color-mix(in srgb, var(--surface-0), #000 10%)' }`;
    position `left/top = placement ?? position`;
  - heading: `className="mb-1 px-2 py-1 text-[10px] font-semibold uppercase tracking-wide"`, `color: var(--text-muted)`;
  - row (`MenuRow`): `className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] font-medium transition-colors"`,
    `role="menuitem"`, inline
    `color: disabled ? 'var(--text-muted)' : danger ? 'var(--danger)' : 'var(--text-strong)'`,
    `opacity: disabled ? 0.55 : 1`, `cursor: disabled ? 'not-allowed' : 'pointer'`;
    hover writes `style.background` imperatively — `danger ? 'color-mix(in srgb, var(--danger), var(--surface-1) 90%)' : 'color-mix(in srgb, var(--accent), var(--surface-1) 82%)'`
    — and mouse-leave resets it to `transparent`;
  - icon slot: `className={`inline-flex h-5 w-5 items-center justify-center ${iconBare && !checked ? '' : 'rounded border'}`}`,
    `borderColor: var(--border-subtle)`, `background: iconBare && !checked ? 'transparent' : 'var(--surface-1)'`,
    `opacity: disabled ? 0.8 : 1`, `color: checked && !disabled ? 'var(--accent)' : undefined`;
    content is `checked ? <Check className="h-3.5 w-3.5" /> : iconNode ?? (Icon ? <Icon className="h-3.5 w-3.5" /> : null)`;
  - trailing content sits after the label (`<span className={trailing ? 'flex-1' : undefined}>` on the label);
  - separator: `<div role="separator" className="my-1 h-px" style={{ background: 'var(--border-subtle)' }} />`,
    emitted when `entry.startsGroup && index > 0`;
  - flyout: `className={`absolute ${placement.up ? 'bottom-0' : 'top-0'} ${placement.left ? 'right-full mr-1' : 'left-full ml-1'} z-[131] ${widthClassName} ${MENU_SURFACE}`}`
    — same width as the parent, one z above it.
- **States**:
  - `position === null` → renders nothing;
  - submenu row: `hasPopup`, `aria-expanded`, chevron
    `<ChevronRight className={`h-4 w-4 shrink-0 opacity-60 transition-transform ${submenuPlacement.left && expanded ? 'rotate-180' : ''}`} />`,
    and the row is disabled only when **every** child is disabled
    (`children.every(c => c.disabled)`);
  - `danger` colour, `checked` tick, `trailing` node, `iconBare`.
- **Placement / dismissal**:
  - measured in a `useLayoutEffect` then `clampToViewport(position, size, { margin: 8 })`
    (comment: "Row heights follow the labels, so estimating them silently
    drifts as entries change");
  - flyout flips are measured against the window with the same 8px margin:
    `up = flyout.bottom > innerHeight - 8 && row.top - flyout.height >= 8`,
    `left = flyout.right > innerWidth - 8 && row.left - flyout.width >= 8`;
  - submenu opens on hover *or* click, closes `SUBMENU_CLOSE_DELAY_MS = 120`ms
    after leaving the row or the flyout;
  - `useOutsideDismiss(open, onClose, { ignoreRef: dismissIgnoreRef })` →
    Escape (through the dialog stack), outside `pointerdown`, `resize`, and
    `scroll` (capture-phase);
  - the root stops `pointerdown` propagation so clicks inside never reach the
    dismiss listener; `createPortal` menus are outside `ignoreRef`, hence the
    explicit stop.
- **Data**: `entries: ContextMenuEntry[]` — `{ id, label (node), icon?,
  iconNode?, iconBare?, trailing?, checked?, disabled?, danger?, startsGroup?,
  children? }`.
- **Callbacks**: `onSelect(id)` then `onClose()` for leaf rows; a flyout's id is
  never selected.
- **Port target**: `gpui_ui::ui::{ContextMenuEntry, context_menu,
  context_menu_sized, context_menu_placed, menu_surface, menu_title,
  FlyoutSide, flyout_side, menu_item}` (`widgets/menu.rs`). Entry builder covers
  `with_icon`, `bare_icon`, `checked`, `danger`, `disabled`, `starts_group`,
  `with_trailing`, `with_children`. Call sites pass `FlyoutSide` explicitly;
  `chrome/editor_menu.rs` uses `FlyoutSide::Right` as the source's pre-measure
  state, and a pure `flyout_side(row, menu, window)` function exists for callers
  that can measure.

---

## Surface: `SelectDropdown` — trigger + menu (as used by the view-mode dropdown)

- **Source**: `src/components/ui/SelectDropdown.tsx` (556 lines).
- **Renders**: `<label>` → optional label span → `relative` container →
  `<button role="combobox">` trigger → portal-ed `role="listbox"` menu.
- **Layout & styling (trigger)**:
  ```jsx
  className={`ui-input rounded-[4px] relative w-full h-[36px] px-2.5 py-0 pr-10 leading-none text-sm disabled:pointer-events-none disabled:opacity-55 disabled:cursor-not-allowed inline-flex items-center text-left ${selectClassName}`}
  style={{ ...selectStyle, ...(isOpen ? { borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 18%)',
                                          boxShadow: '0 0 0 1px color-mix(in srgb, var(--accent), transparent 72%) inset' } : undefined) }}
  ```
  Leading affordance: `<span className="mr-1.5 inline-flex h-[18px] w-[18px] shrink-0 items-center justify-center">`.
  Value span: `className={\`min-w-0 flex-1 truncate ${selectedTextAlign === 'center' ? 'text-center' : ''} ${(hideSelectedText && selectedDisplay) ? 'text-transparent' : ''}\`}`;
  an overlay `selectedDisplay` is `absolute top-1/2 -translate-y-1/2`,
  `left-1/2 -translate-x-1/2` when centred else `left-2.5`, with
  `pointer-events-none`.
  Chevron: `<ChevronDown className={\`pointer-events-none absolute ${chevronClassName} top-1/2 h-3.5 w-3.5 -translate-y-1/2 transition-transform ${isOpen ? 'rotate-180' : ''}\`}>`,
  default `chevronClassName = 'right-3'`.
- **Layout & styling (menu)**:
  ```jsx
  className={`fixed z-[9999] rounded-[4px] border shadow-xl ${menuClassName}`}
  style={{ top, left, width: minWidth, minWidth, maxHeight,
           visibility: 'hidden'|'visible',
           borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 82%)',
           background: 'color-mix(in srgb, var(--surface-0), black 6%)',
           backdropFilter: 'blur(6px) saturate(112%)',
           boxShadow: '0 14px 34px rgba(0, 0, 0, 0.46), 0 4px 12px rgba(0, 0, 0, 0.32)' }}
  ```
  Options: `className="group w-full px-3 py-2 text-left text-sm transition-colors inline-flex items-center gap-2 border-b last:border-b-0"`;
  separators between rows are
  `borderBottomColor: (last || next.heading || next.divider) ? 'transparent' : 'color-mix(in srgb, var(--border-subtle), transparent 16%)'`
  ("A group ends without a rule: the heading below is the separation, and two
  lines there read as a table.").
  Selected row: `background: color-mix(in srgb, var(--accent), var(--surface-0) 86%)`;
  hover: default `color-mix(in srgb, var(--surface-2), transparent 18%)`,
  accent `color-mix(in srgb, var(--accent-secondary), var(--surface-0) 92%)`,
  danger `color-mix(in srgb, var(--danger), transparent 88%)`;
  text: danger → `var(--danger)`, accent → `var(--accent-secondary)`,
  `(selected || hovered) ? 'var(--text-strong)' : 'var(--text-muted)'`.
  Headings: `px-3 pb-1 pt-2 text-[10px] font-semibold uppercase tracking-wide`,
  `color: var(--accent)`, `borderBottom: 1px solid color-mix(in srgb, var(--border-subtle), transparent 35%)`.
  Dividers: `border-t` with `borderColor: var(--border-subtle)`.
  Disabled option: `opacity: 0.55; cursor: not-allowed` and no hover/selected
  styling.
- **Placement**: `gap = 6`, `margin = 8`; menu width is forced to the trigger
  width first (so an `!w-64` override is measured correctly — "Measuring without
  the inline width was wrong on the first pass"); `menuAlign: 'right'` →
  `left = rect.right - measuredWidth`; clamp; open upwards when
  `belowSpace < Math.min(180, measuredHeight) && aboveSpace > belowSpace`;
  `maxHeight = Math.max(120, space)` on the chosen side.
- **States**: closed/open (trigger border + inset ring, chevron rotated, menu
  `visibility` hidden until measured), focused, disabled, option
  selected/hovered/disabled/heading/divider/action.
- **Callbacks**: trigger click toggles; `Enter`/`Space`/`ArrowDown` open;
  `Escape` closes; option click → `onChange(value)`, close, blur (twice — once
  immediately and once in `requestAnimationFrame`); footer action →
  `onClick()`, close.
- **Port target**: `gpui_ui::ui::{select_dropdown, select_dropdown_aligned,
  SelectOption, SelectOptionTone, SelectMenuAlign}` (`widgets/select.rs`,
  `select_menu.rs`). `SelectOption` supports `heading()`, `divider()`,
  `action()`, `with_icon`, `with_right`, `with_tone`, `disabled`.
  `select_dropdown_aligned` exists because the menu is forced to the trigger's
  width. What the widget does **not** expose — and `app_bar.rs` therefore
  re-implements — is `selectClassName`, `menuClassName`, `chevronClassName`,
  `hideChevron`, `selectedDisplay`, and the icon-only trigger.

### `ViewTypeDropdown`

- **Source**: `src/components/controls/ViewTypeDropdown.tsx` (whole file).
- **Renders**: a `pointer-events-auto` div wrapping `SelectDropdown`, or an
  icon-only variant of it.
- **Layout & styling / props to `SelectDropdown`**:
  - `className="space-y-0"`; `selectClassName`:
    `iconOnly ? '!h-9 !w-9 !p-0' : '!h-9 !py-1.5 !pl-2 !pr-7 text-xs'` plus
    `w-full` when `fullWidth` — with the comment "`!h-9` matches the adjacent
    topbar buttons (Button's `md` shape), so the trigger's top and bottom edges
    sit on the same lines as theirs";
  - `chevronClassName="right-1.5"`, `hideChevron={iconOnly}`, `menuAlign="right"`,
    `menuClassName="!w-64"`, `hideSelectedText={iconOnly}`,
    `selectedDisplayAlignment={iconOnly ? 'center' : 'left'}`,
    `selectedDisplay={iconOnly ? <Camera className="h-4 w-4" style={{ color: 'var(--text-strong)' }} /> : undefined}`;
  - `title`/`ariaLabel` = `resolvedTitle = title ?? (iconOnly ? `Camera view mode: ${currentLabel}` : 'View type')`
    — the caller passes `title={_('View mode')}`, so in practice both are
    `View mode`.
- **Icons per option** (`getViewTypeIcon`): `soft_clay`→`Paintbrush`,
  `overhang_heatmap`→`AlertDiamondIcon`, `opaque_wire_mesh`→`Layers3`,
  `wireframe`→`Grid3X3`, `xray`→`Eye`, `normal_debug`→`Scan`, default
  `AppWindow`; all `h-3.5 w-3.5`.
- **Options / labels** (`MESH_SHADER_OPTIONS`, `src/features/shaders/mesh/types.ts:42-49`),
  in this order:
  `soft_clay` → `Standard` ("matte surface lit by the scene lights. The default
  view."), `overhang_heatmap` → `Overhangs`, `opaque_wire_mesh` → `Mesh` ("mesh
  edges drawn over a solid surface, so back edges stay hidden"),
  `wireframe` → `Wireframe`, `xray` → `X-Ray`, `normal_debug` → `Normals`
  ("'Normal' is the geometry term, not the opposite of 'unusual'.").
- **Comment worth keeping** (icon-only trigger rationale): "Icon-only triggers
  carry the camera, since the mode's name does not fit. With room, the name and
  the chevron read as the dropdown on their own: the chevron sits close to the
  edge and the trigger's right padding holds the text off it, so it reads as
  label then chevron rather than label, gap, chevron, larger gap."
- **Port target**: `app_bar::view_type_dropdown` + `view_type_menu`
  (`app_bar.rs:972`, `:1112`), including the `!h-9`/`!p-0` trigger sizes, the
  256px (`!w-64`) right-aligned menu, the option-icon tinting, and the note that
  `.ui-input`'s unlayered 13px/6px radius beat the layered `text-sm` /
  `rounded-[4px]`.

---

## Surface: `EditorContextMenu` — the canvas right-click menu

- **Source**: `src/components/ui/EditorContextMenu.tsx` (whole file), rendered
  over `ContextMenu`.
- **Renders**: a `ContextMenu` with `title` (caller: `Editor` in Prepare,
  `Supports` in Support) and `ariaLabel = msg("Editor context menu")`.
- **Entries** (`MENU_ITEMS`, `:88-140`):

  | id | label (msgid, comment) | icon | group |
  |---|---|---|---|
  | `delete` | `Delete` | `Trash2` | — |
  | `cut` | `Cut` | `Scissors` | — |
  | `copy` | `Copy` | `Copy` | — |
  | `paste` | `Paste` | `ClipboardPaste` | — |
  | `repair-menu` (flyout) | `Repair` | `Wrench` | ✔ |
  | ↳ `repair` | `Auto-Repair` | `Wrench` | |
  | ↳ `remesh` | `Remesh` | `Pyramid` | |
  | ↳ `shrinkwrap` | `Shrink-Wrap` | `RefreshCw` | |
  | `supports-menu` (flyout) | `Supports` | `Blocks` | ✔ |
  | ↳ `split-supports` | `Split supports` — "detaches the generated support scaffolding at the clicked point. \"Supports\" = the temporary print scaffolding structures, not customer support." | `Split` | |
  | ↳ `merge-supports` | `Merge supports` — "re-attaches the support scaffolding back to the model." | `Link` | |
  | ↳ `scan-for-supports` | `Scan for Supports` | `Search` | |
  | `geometry-menu` (flyout) | `Geometry` | `Shapes` | — |
  | ↳ `mark-as-support-geometry` | `Mark as Support` | `LifeBuoy` | |
  | ↳ `mark-as-model-geometry` | `Mark as Model` | `Box` | |

  Commented-out (dead) rows kept in the source: `link-models`
  `Link Selected Models` / `Link`, `unlink-models` `Unlink Selected Models` /
  `Unlink`. The `EditorMenuAction` union also lists `show-repair-report`,
  `undo-repairs`, `supports-toggle-curve`, `supports-add-joint`,
  `assign-material:${string}` and the two organic-cut actions, which the caller
  adds/modifies as its own item lists.
- **States**: `disabled = disabledActions.includes(id)`; a flyout is disabled
  when all its children are. `checked`, `iconNode`, `iconBare` pass through for
  caller-built rows (material swatches). Organic-cut convenience items:
  `ORGANIC_CUT_ADD_WAYPOINT_ITEM` (`Add waypoint here`, `Plus`) and
  `ORGANIC_CUT_DELETE_WAYPOINT_ITEM` (`Delete waypoint`, `Trash2`).
- **Callbacks**: `onAction(id as EditorMenuAction)` then `onClose()`.
- **Port target**: `gpui/src/chrome/editor_menu.rs` — rows, labels, icons and
  groups match; only `delete` performs an action, all others are documented
  no-ops. Deviations in the header: heading omitted (entity state is unreadable
  while building the tree), flyouts always open right, clamping via
  `snap_to_window_with_margin(8)`, dismissal owned by `app.rs`, no disabled
  rows, no aria.

---

## Surface: `FloatingPanelStack` — the dock/float layout

- **Source**: `src/components/layout/FloatingPanelStack.tsx` (1051 lines;
  component `:420-1051`).
- **Renders**: an absolutely positioned, `pointer-events-none` container with a
  left dock column, a right dock column, the floating panels, a dashed drop
  preview while dragging, and a right-click menu.
- **Layout & styling**:
  - container: `className="absolute right-0 top-[var(--topbar-height)] bottom-0 z-10 pointer-events-none"`, `style={{ left: 0 }}`;
  - each dock: `className="absolute top-0 bottom-0 flex flex-col pointer-events-none"` with inline
    `{ left: leftInsetPx, width: leftWidth, padding: 6, gap: 6, alignItems: 'flex-start', overflowY: drag ? 'visible' : 'auto', overflowX: drag ? 'visible' : 'hidden' }`
    (right dock mirrors: `right: rightInsetPx`, `alignItems: 'flex-end'`);
  - each dock has a resize strip: `className="absolute inset-y-0 cursor-col-resize pointer-events-auto"`, `style={{ right: 0, width: 6 }}` (left) / `{ left: 0, width: 6 }` (right);
  - panel item: `className="pointer-events-auto"` with inline
    `{ display: hidden ? 'none' : undefined,
       position: docked ? 'relative' : 'absolute',
       left/top: floating coords, width: panelWidth,
       flex: docked ? '0 0 auto' : undefined,
       zIndex: isDragging ? 100 : 20 + index,
       cursor: isDragging ? 'grabbing' : 'default',
       transform: dragging ? `translate(${dx}px, ${dy}px)` : undefined,
       pointerEvents: isDragging ? 'none' : 'auto',
       borderRadius: 12 }`,
    `data-panel-id={id}`, and its children wrapped in
    `<div className="w-full [&>*]:!w-full">`;
  - drop preview: `className="absolute pointer-events-none rounded-lg border-2 border-dashed"` with
    `borderColor: color-mix(in srgb, var(--accent), white 12%)`,
    `background: color-mix(in srgb, var(--accent), transparent 88%)`,
    `boxShadow: 0 0 0 4px color-mix(in srgb, var(--accent), transparent 82%)`,
    `zIndex: 120`.
- **Constants**: `PANEL_MARGIN = 6`, `PANEL_GAP = 6`, `DEFAULT_PANEL_WIDTH = 320`,
  `DEFAULT_PANEL_HEIGHT = 150`, `MIN_DOCK_WIDTH = 200`, `MAX_DOCK_WIDTH = 520`,
  `COLLAPSED_PANEL_HEIGHT = 48`, `PANEL_WIDTH_OVERRIDES =
  { 'transform-debug-overlay': 420 }`, `RESIZABLE_PANELS = new Set(['prepare-models', 'support-models'])`,
  `LOCKED_PANELS = { 'prepare-models': 'rail-column', 'support-models': 'rail-column' }`.
- **Default docks** (`:161-192`) — the first-run placement, quoted in full:

  | panel id | dock |
  |---|---|
  | `prepare-models`, `prepare-debug-primitives`, `support-models`, `analysis-scan-card`, `analysis-island-list`, `analysis-workflow`, `export-main`, `curve-settings` | `left` |
  | `prepare-transform-controls`, `prepare-orientation`, `prepare-smoothing-settings`, `prepare-arrange-panel`, `prepare-duplicate-panel`, `prepare-organic-cut-panel`, `analysis-overlay-controls`, `analysis-territory-voxel`, `analysis-island-voxel`, `analysis-volumes`, `support-islands`, `support-settings`, `support-auto`, `support-hollowing`, `support-hole-punch`, `export-slicing`, `transform-debug-overlay` | `right` |
  | anything else | `right` (`DEFAULT_PANEL_DOCKS[id] ?? 'right'`) |

  Comment: "`always` is the shell chrome. The model list is only pinned while the
  tool rail is a column down the left edge."
- **Data**: `children` (flattened; a child's `key` becomes its panel id, a
  `React.Fragment` is flattened recursively with `${prefix}-${index}` fallbacks,
  and a child whose inline style has `display: 'none'` is recorded as `hidden`
  — comment: "They must leave the dock's flow too, or their gap pushes every
  panel below them down"); `leftInsetPx`, `rightInsetPx`, `railIsColumn`.
  Persistence: `localStorage['lumenslicer:floating-panel-layout:v6']`
  (`{ version: 6, panels, docks }`), legacy
  `…:v5` (absolute positions only) migrated once by centre position —
  `x + width/2 < innerWidth * 0.4` → left, `> 0.6` → right, else float — and
  `localStorage['app-floating-layout-persistence']` (absent ⇒ enabled).
- **Behaviour** (each is a numbered contract):
  1. **Dock widths**: `content = (widest panel || 320) + 2 * 6`; a stored width is
     used only if the dock holds a `RESIZABLE_PANELS` member and then only
     `Math.max(stored, content)`; always clamped to `[200, 520]`.
  2. **Dragging**: starts on the panel's `[data-panel-drag-handle="true"]`
     descendant, left button only, not on a locked panel, not on
     `button, input, select, textarea, label, a, [role="button"], [data-no-drag], [data-no-drag="true"], .react-colorful`;
     4px movement threshold (`dx*dx + dy*dy < 16` ⇒ not moved).
  3. **Drop target**: chosen by how much of the **dragged panel** overlaps a
     dock (`leftOverlap > draggedWidth / 2`), not by the pointer — comment:
     "Snap on how much of the dragged panel is over the dock, not on where the
     pointer is: a wide dock would otherwise hold the panel until the pointer
     had travelled its whole width." Insert index counts visible panels whose
     vertical midpoint is above the pointer.
  4. **Release outside a dock**: floats at the pointer, clamped by
     `clampPosition` to `[6, bounds - size - 6]`.
  5. **Dock resize**: dragging the strip sets the **panels'** own widths
     (`Math.max(baseWidth, width - 12)` on every resizable panel in that dock)
     and the dock follows them — comment: "so a panel keeps that width when it
     is dragged out to float. The drag can only widen: a panel never goes below
     its natural width."
  6. **Auto-compact**: after any size/registry/viewport change and while not
     dragging, for each dock whose total height (`Σ height + 6` per panel)
     exceeds `viewportHeight - 12`, collapse `expanded && autoCollapse` panels
     from the bottom up until it fits; each collapse frees
     `height - 48`. `requestPanelExpand(id)` runs the same loop *before*
     expanding, excluding the panel being expanded.
  7. **Context menu** (right-click on a panel, suppressed if the right button
     moved more than 6px — `dx*dx+dy*dy > 36`): `Reset Window` (`RotateCcw`,
     returns that panel to its default dock, ordered last) and `Reset Layout`
     (`LayoutPanelTop`, clears the layout state and both storage keys), with
     `title` `Window` and `ariaLabel` `Window context menu`.
  8. **Debug snapshot**: listens for
     `lumenslicer:floating-layout-debug-request` and answers with
     `{ version: 2, capturedAt, persistenceEnabled, storageKey, panelIds,
     positions, placements, docks }`.
  9. **Persistence toggle**: re-read on
     `lumenslicer:floating-layout-persistence-changed` and on `storage`; writes
     only when enabled.
- **Hooks exported for panels**: `useFloatingPanelDockResize()` → `beginDockResize`
  and `useFloatingPanelCollapse(defaultExpanded = true, { autoCollapse = true })`
  → `[expanded, setExpanded]`, where `setExpanded(true)` first calls
  `requestPanelExpand(panelId)`.
- **Port target**: `gpui-core/src/panel_layout.rs` (`PanelLayout`,
  `PanelPlacement`, `PanelDock`, `PanelKind`, `PanelStackInsets`,
  `MIN/MAX_DOCK_WIDTH`, `DEFAULT_PANEL_*`, `PANEL_MARGIN`, `PANEL_GAP`,
  `COLLAPSED_PANEL_HEIGHT`) plus `gpui/src/panels/list.rs::panel_stack` for the
  columns and `app.rs`'s `layout` field, `session_layout` /
  `apply_session_layout` for persistence. The port persists to its own
  `floating-panel-layout.json` (`state.panels`, `state.loaded`,
  `state.left_width`, `state.right_width`) and falls back to `session.json`
  only when that file is absent; `TOOL_RAIL_WIDTH` is hardcoded to 66 there and
  `PanelStackInsets` carries the source's three props.

---

## Surface: `floatingLayoutPreferences`

- **Source**: `src/components/layout/floatingLayoutPreferences.ts` (whole file).
- **Renders**: nothing — storage keys, types and accessors.
- **Constants / storage**:
  `FLOATING_LAYOUT_STORAGE_KEY = 'lumenslicer:floating-panel-layout:v6'`,
  `LEGACY_FLOATING_LAYOUT_STORAGE_KEY = …:v5`,
  `FLOATING_LAYOUT_PERSISTENCE_STORAGE_KEY = 'app-floating-layout-persistence'`,
  `FLOATING_LAYOUT_PERSISTENCE_EVENT = 'lumenslicer:floating-layout-persistence-changed'`,
  `FLOATING_LAYOUT_DEBUG_REQUEST_EVENT = 'lumenslicer:floating-layout-debug-request'`,
  `MODELS_PANEL_VISIBILITY_STORAGE_KEY = 'app-models-panel-visible'`,
  `TOOL_LAYOUT_STORAGE_KEY = 'app-tool-layout'`,
  `TOOL_LAYOUT_EVENT = 'lumenslicer:tool-layout-changed'`.
- **Semantics**:
  - `isFloatingLayoutPersistenceEnabled()`: absent ⇒ `true`, `'false'` ⇒ false;
  - `isModelsPanelVisibleEnabled()`: absent ⇒ `true`, `'false'` ⇒ hidden
    ("It starts visible, so an absent key means shown; hiding it is what gets
    remembered.");
  - `getToolLayout()`: `'vertical'` only for an exact `'vertical'` value,
    **default `'horizontal'`**;
  - `setToolLayout` writes and dispatches `TOOL_LAYOUT_EVENT` with
    `{ detail: { layout } }`;
  - `clearSavedFloatingLayout()` removes both layout keys.
  - `ToolLayout = 'vertical' | 'horizontal'` — "The horizontal form takes no
    room from the panel stack, so the stack's left inset follows this."
- **Port target**: `gpui-store`'s layout/session documents and
  `panel_layout.rs`'s own file. The port has **no** equivalent of the
  `TOOL_LAYOUT_EVENT` bus (rail layout is rail-local) and defaults the rail to
  `Vertical` — the source's default is `horizontal`; changing it requires the
  panel stack to take the rail layout as an inset.

---

## Surface: `StructuredDialogModal`

- **Source**: `src/components/ui/StructuredDialogModal.tsx` (whole file).
- **Renders**: a portal-ed backdrop + card: icon chip, title, subtitle, close
  button, body (`children`), action row.
- **Layout & styling**:
  - backdrop: `className={`fixed inset-0 ${zIndexClassName} flex items-center justify-center bg-black/55 backdrop-blur-sm px-3`}`
    with default `zIndexClassName = 'z-[100]'`; `onMouseDown` closes only when
    `event.target === event.currentTarget`;
  - card: `className={`w-full ${maxWidthClassName} overflow-hidden rounded-xl border shadow-2xl ${panelClassName}`}`
    with default `maxWidthClassName = 'max-w-lg'`, `role="dialog"`,
    `aria-modal="true"`, `style={{ background: 'var(--surface-0)', borderColor: 'var(--border-subtle)', boxShadow: '0 24px 46px rgba(0,0,0,0.42)' }}`;
  - header: `className="flex items-center justify-between gap-4 border-b px-5 py-4"`, `borderColor: var(--border-subtle)`;
  - icon chip: `className="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-md border"`, `style={ICON_TONE_STYLES[iconTone]}`;
  - title: `className="text-base font-semibold leading-tight"`, `color: var(--text-strong)`;
  - subtitle: `className="mt-0.5 text-[11px] leading-snug"`, `color: var(--text-muted)`;
  - body: default `bodyClassName = 'space-y-4 p-5'`;
  - actions: default `actionsClassName = 'grid grid-flow-col auto-cols-fr gap-2 pt-1'`.
- **States**: `open` false → null; `closeDisabled` ⇒ Escape does nothing (a
  registration without a handler "swallows Escape instead of closing"); no
  `onClose` ⇒ no close button.
- **Callbacks**: `onClose` (X and Escape), `onBackdropClick` (falls back to
  `onClose`).
- **Port target**: `gpui_ui::ui::dialog_modal` /
  `dialog_modal_full` / `modal_shell` (`widgets/dialog.rs`).

---

## Deliberate decisions and gotchas

1. **`.ui-topbar`'s `position: relative` beats the `fixed` utility.** The root
   carries both (`TopBar.tsx:705`). `globals.css` is unlayered and Tailwind's
   `fixed` lives in the utilities layer, so `.ui-topbar` wins and the bar is
   `relative` — in flow at the top of the shell. Nothing else moves because
   every sibling layer is `absolute`, which is why the bug is invisible. Render
   it as a top strip with `z` above the canvas; do not "fix" the class list.
   The port correctly uses `absolute top: 0, left: 0, right: 0`.
2. **Bar height arithmetic.** `.ui-topbar` is `height: 52px` with
   `padding: 3px 4px 9px 4px`, which leaves a 40px content box — exactly the
   `h-10` logo / printer / monitor buttons. The left group adds `py-1.5` (12px)
   to a 40px child and therefore overflows the content box by 12px, centred, so
   its buttons still land at y ≈ 3…43. `app_bar.rs` renders `h = px(48.)` with
   `pt_1p5 pb_1p5` (36px content) for 40px children. Same band within ~2px, but
   the painted surface stops 4px higher; if exact parity matters, the bar should
   be 52px tall with 3/9 padding.
3. **`topbarActionsDisabled` is only `isSlicingBusy`.** Every "disabled" visual
   in the bar derives from that one prop (`:186`), and it is a *soft* disable:
   groups add `opacity-45`, the left group additionally `pointer-events-none`,
   and each control sets its own `disabled`. The right group's dim wrapper does
   **not** disable the caption buttons outside it — window controls stay live
   during a slice.
4. **The step measurement is asymmetric on purpose** — appear at ≥155px per
   button, stay at ≥130px — because the view-mode label's own width feeds the
   measurement. A single threshold oscillates.
5. **`hideBadges` has two sources**: `window.innerWidth < 800` at mount and the
   observer's `perButton < 120`. The observer must be re-created when
   `hideWorkflowControls` flips, or it observes a detached node and pins the
   icons hidden forever.
6. **The step label's colour conditional is dead** — both branches are
   `var(--text-strong)`. Do not invent a differing active colour.
7. **`step` in the steps array is never read.** Order is the contract; the
   numbers 1-4 exist only as data.
8. **`color-mix(in srgb, A, B N%)` = `A*(1-N) + B*N`.** This is the single most
   common porting error in this area (see the handoff's decision 3). Examples
   that matter here: the badge chevron `color-mix(text-muted, white 8%)` keeps
   92% muted; step active `color-mix(accent, surface-0 84%)` keeps 16% accent;
   the fleet row `color-mix(accent, surface-1 89%)` keeps 11% accent; the tile
   `off` recipe's first argument carries its own inline weight
   (`color-mix(in srgb, ${hue} 10%, var(--surface-1))` = 10% hue).
9. **The rail's `invertLit` inverts the *recipe*, not `active`.**
   `aria-pressed` still reports `entry.active`. For Models: shown ⇒ unlit look;
   hidden ⇒ lit look.
10. **`DISABLED_STYLES` carries `opacity: 0.45` inline** because "this Tailwind's
    scale has no 45, so the class would be dropped". Use the same value for a
    disabled tile; do not assume an `opacity-45` utility exists for it.
11. **The tile's group gap goes on the tooltip wrapper, not the button**
    (`wrapperClassName={geometry}`) so the tile and its hint stay one flex child.
    Same for `w-full` in the column.
12. **Tooltip geometry in the rail is fixed, not measured**: `width={140}`,
    `anchor="trigger"`, `placement` right in the column and below in the bar,
    `offsetY={8}` ("8px clears the tile's own border"). Reproduce the numbers;
    do not size the box to the text.
13. **`TOOL_RAIL_WIDTH_PX = 54 + 12 = 66`** and the parent uses it as
    `leftInsetPx` only when `models.length > 0 && (mode === 'prepare' || 'support') && toolLayout === 'vertical'`
    (`page.tsx:10718`). The right inset is `models.length > 0 && mode !== 'printing' ? 48 : 0`
    (`:10719`) — the 48px is the `CrossSliderPanel` strip.
14. **The Models tile exists only in the column.** `modelsRailToggle` is `{}` in
    the bar, and the panel list is forced visible there
    (`modelsPanelShown = … || toolLayout === 'horizontal'`) because "there is no
    room beside it for the list to hide into and no tile to bring it back". The
    stored preference is *not* rewritten when the bar forces it visible.
15. **`handleToolRailHover` is effectively dead.** Prepare's builder attaches
    `onHover` to its eight tools; Support's attaches none; and the handler body
    only acts when the hovered mode is `'hollowing'`, which no Prepare entry is.
    It only warms the hollow-preview cache. Do not port it as behaviour.
16. **Support's panel tools and Hollowing are one selection.**
    `active: !hollowingActive && mode === railMode` for the three panels,
    `active: hollowingActive` for Hollow; and the call site resets
    `transformMode` to `'select'` when a panel tool is chosen while Hollowing is
    active.
17. **`Interior View` is disabled, not hidden**, when there is no cavity
    ("the tile greys out rather than hiding while there is nothing to look
    inside"), and its hint swaps to `Hollow the model first to look inside it`.
    The same gate is used by the `X` hotkey.
18. **View-mode's `onSelect` toggles from the *current* mode**, not from the
    tile's active flag: `onViewModeChange(viewMode === 'lines' ? 'full' : 'lines')`,
    and the *current* mode's icon is displayed.
19. **`ContextMenu` closes on real `pointerdown`, Escape, resize and
    scroll** — and the app menu passes `dismissIgnoreRef` (the logo button) so a
    second click on the button closes the menu through its own handler rather
    than being swallowed by the dismiss listener. The fleet quick menu has a
    hand-rolled equivalent that does *not* handle resize/scroll and *does* listen
    for Escape on `app-hotkey-keydown`; the two differ.
20. **Fleets are thresholded twice, differently**: the badge switches to
    profile/printer label order at `networkFleet.length > 1`, but the quick menu
    requires `topbarFleetUnits.length > 1` where units must have a non-blank
    `ipAddress`. A profile with two fleet entries, one without an IP, shows
    fleet labels and opens the profile modal instead of the quick menu.
21. **Drag/drop in the panel stack snaps on the panel's overlap, not the
    pointer**, and a right-click that has moved more than 6px is treated as a
    drag, not a menu gesture.
22. **Panel `hidden` uses `display: none` and must leave the dock's flow** —
    otherwise the `gap: 6` reserves its space and every panel below shifts.
23. **Resizing a dock widens the *panels*, and the dock follows.** A panel keeps
    that width when dragged out to float; the drag can never make a panel
    narrower than its base width.
24. **There is no onboarding in the port, so `hideWorkflowControls` is
    unmodelled** (`app_bar.rs` deviation 12). Anything that reads "the bar looks
    different at first run" is this flag: steps, divider, printer badge, view
    menu and Overhangs toggle all disappear and the remaining groups spread.
25. **`SettingsModal`/`ProfileSettingsModal` are mounted by the bar but owned by
    other areas.** The bar only owns the open flags, the initial tab, the
    three `open*Token` counters, and the `dispatchProfileSettingsModalOpenChange`
    notifications. Do not port those modal bodies here.
26. **The interpolated printer strings live in `topBarMessages.ts` for a
    compiler reason**, not for tidiness (see the quote in the printer-badge
    section). A port that inlines them will compile and lose the translations.
27. **`useEscapeToClose` is a stack, not a listener per dialog**: the topmost
    open dialog consumes Escape with `stopImmediatePropagation`, so a nested
    dialog closes before its parent and one press never closes two.
    `StructuredDialogModal` registers through it; `ContextMenu` and
    `SelectDropdown` go through `useOutsideDismiss`, which wraps the same hook.
    This is why the bar's fleet menu needs its own Escape listener — it does not
    use either.

---

## Open questions

1. **`Profile`/`Material` modal ownership.** The bar derives
   `warnBeforeProfileSettingsOpen` from `printingArtifact && !isActiveSliceStale`
   — i.e. "a valid slice exists for the current plate". The port has no slicing
   pipeline, so the warning (and the profile switch it guards) is unreachable.
   Whoever ports slicing must decide whether the guard lives on the bar or on
   the profile store.
2. **Fleet liveness.** `printerReachabilityByDeviceId` is supplied by the
   parent; the source never says who computes it or how often. The port's fleet
   rows are drawn from `PrinterBadge.fleet` with an `online` flag — the upstream
   producer is still a JS-side polling loop (`usePrintingMonitorManager` /
   profile store), which is outside this area.
3. **`getProfileStoreSnapshot`/`subscribeToProfileStore` are the only store the
   bar reads.** Whether the port's printer profiles (in
   `features/printer_profiles/`) should feed `AppBarState.printer` on a
   subscription or per frame is unspecified; today the badge is computed from a
   value passed into `app_bar()`.
4. **`SelectDropdown`'s `menuMount`/`action`/`heading`/`footer` features** are
   used by other areas but not by this one. `ui::select_dropdown` lacks the
   icon-only trigger knobs, so `app_bar.rs` re-implements the view-mode trigger;
   if that widget gains `leadingDisplay`/`selectedDisplay`/`hideChevron`, the
   bar's copy should be deleted rather than left to drift.
5. **The rail's fold animation** has no GPUI equivalent (no `Element::animate`
   in this fork's element builders, per `rail.rs`); whether the port wants a
   ticker-driven equivalent is a product decision, not a porting question.
6. **`custom-scrollbar`** (the thin accent scrollbar used by the rail, the fleet
   list and menu lists) is a `globals.css` class with no GPUI counterpart; the
   port uses the platform scrollbar. Confirm that is acceptable before painting
   pixel-parity baselines over those regions.
7. **`.ui-topbar-mode-idle` / `.ui-topbar-mode-active`** (`globals.css:261-267`)
   are referenced by no component in this scope — possibly dead, possibly used
   by a feature that injects the class. Nothing in `TopBar.tsx` sets them.
