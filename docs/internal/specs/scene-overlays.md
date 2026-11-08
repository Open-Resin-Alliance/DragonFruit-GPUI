# Scene overlays — empty scene, view cube, layer slider, plate widgets, toasts

Porting specification for the 2D overlays that sit on and around the 3D viewport, written
from the TypeScript sources read in full. Every class list, inline style value and source
comment below is quoted from the file it names.

Scope note: the gizmo **maths** (camera tween model, quarter-turn axes, `stabilizeDirection`)
is described here only as far as the widget needs it to be drawn and hit-tested; the tween
itself belongs to the camera controller area.

## Components in scope

| Component | File |
|---|---|
| `Toast`, `ToastViewport` | `src/components/atoms/Toast.tsx` |
| `NotificationStack` | `src/components/organisms/NotificationStack.tsx` |
| Plate-locked / new-device / leaf-fanning toasts (inline) | `src/app/page.tsx:12062-12120` |
| Marquee mode toast (inline) | `src/components/scene/SceneCanvas/SceneCanvas.tsx:8557-8569` |
| `EmptySceneState` | `src/components/layout/EmptySceneState.tsx` |
| `LayerSlider` | `src/components/controls/LayerSlider.tsx` |
| `CrossSliderPanel` | `src/components/controls/CrossSliderPanel.tsx` |
| `ZUpGizmoHelper` | `src/components/scene/SceneCanvas/ZUpGizmoHelper.tsx` |
| `ZUpGizmoViewcube` | `src/components/scene/SceneCanvas/ZUpGizmoViewcube.tsx` |
| `plateWidgetDraw.ts` (widget colours, rounded-rect, icon stroking) | `src/components/scene/SceneCanvas/plateWidgetDraw.ts` |
| `PlateNameLabel` | `src/components/scene/SceneCanvas/PlateNameLabel.tsx` |
| `PlateSideButtons` | `src/components/scene/SceneCanvas/PlateSideButtons.tsx` |
| Plate widget anchors/scale (caller) | `src/components/scene/SceneCanvas/SceneEnvironment.tsx:1374-1382` |
| Toast state machine (timings) | `src/features/notifications/useEditorToasts.ts` |

Supporting atoms read but not owned by this spec: `Card` (`src/components/atoms/Card.tsx`),
`Spinner` (`src/components/atoms/Spinner.tsx`), `ProgressBar`
(`src/components/atoms/ProgressBar.tsx`), `Tooltip` (`src/components/ui/Tooltip.tsx`),
`fitFontToWidth` (`src/utils/canvasTextFit.ts`), `useOutsideDismiss`
(`src/hooks/useOutsideDismiss.ts`).

---

## Surface: `Toast` — the app's single toast chip

- **Source**: `src/components/atoms/Toast.tsx:4-102`
- **Renders**: one `<div>` chip, border + padding + text, tone colours inline.
- **Layout & styling**:
  - Base classes, verbatim: `'border px-4 py-2 text-sm font-semibold shadow-lg'`.
  - `shape` adds `'rounded-full'` (`pill`, default) or `'rounded-lg'` (`rounded`).
  - `pulse` adds `'motion-safe:animate-pulse'`.
  - Tone table (`toastToneStyle`, inline styles, `color` is `var(--text-strong)` for every tone):
    | tone | `borderColor` | `background` |
    |---|---|---|
    | `neutral` | `var(--border-subtle)` | `var(--surface-0)` |
    | `info` | `color-mix(in srgb, #60a5fa, var(--border-subtle) 50%)` | `color-mix(in srgb, #60a5fa, var(--surface-0) 90%)` |
    | `success` | `color-mix(in srgb, #22c55e, var(--border-subtle) 50%)` | `color-mix(in srgb, #22c55e, var(--surface-0) 90%)` |
    | `warning` | `color-mix(in srgb, #f59e0b, var(--border-subtle) 50%)` | `color-mix(in srgb, #f59e0b, var(--surface-0) 90%)` |
    | `error` | `color-mix(in srgb, #ef4444, var(--border-subtle) 50%)` | `color-mix(in srgb, #ef4444, var(--surface-0) 90%)` |
  - Merge order is fixed: `{...toastToneStyle[tone], ...animationStyle, ...style}`, so a
    caller-supplied `style` overrides both the tone and the animation. The marquee toast and
    the plate-lock toast both rely on this.
- **States**:
  - `animated = false` (default): `shown = visible`; no transform, no opacity, no transition.
  - `animated = true`: a local `entered` flag starts `false` and is flipped in a
    `requestAnimationFrame`; `shown = entered && visible`. The animation style is
    `opacity: shown ? 1 : 0`, `transform: translateY(shown ? '0px' : '${enterOffsetPx}px')`,
    `transition: 'opacity 220ms ease, transform 220ms ease'`. `enterOffsetPx` defaults to `8`.
  - `pulse` is a second, independent animated state (`motion-safe` only).
- **Data**: props only — `tone`, `shape`, `animated`, `visible`, `enterOffsetPx`, `pulse`,
  plus all `HTMLAttributes<HTMLDivElement>` (`role`, `tabIndex`, `onClick`, `onKeyDown`).
  No store.
- **Callbacks**: none of its own; it forwards `...props`.
- **Text**: none; children only.
- **Port target**: `gpui/crates/gpui-app/src/features/toast/mod.rs` (new). No `gpui/`
  directory exists in this worktree — see "Port targets" at the end.

## Surface: `ToastViewport` — the centred fixed strip a toast lives in

- **Source**: `src/components/atoms/Toast.tsx:105-156`
- **Renders**: one `position: fixed` full-width flex row, horizontally centred.
- **Layout & styling**:
  - Classes: `'pointer-events-none fixed inset-x-0 flex justify-center px-3'`.
  - `offset` resolution (`resolveOffsetValue`): a finite number becomes `` `${n}px` ``; a
    non-empty trimmed string is used verbatim; otherwise the fallback is `'1rem'` for
    `top-center` and `'1.25rem'` for `bottom-center`.
  - `position` writes `{ top: offsetValue }` for `top-center` and `{ bottom: offsetValue }`
    for `bottom-center` (default). `zIndex` is applied only when not `null`.
  - `pointer-events-none` on the viewport means the **toast** must opt into pointer events
    itself (`.pointer-events-auto`) if it is clickable. `NotificationStack` does exactly that
    for the scene-import toast and the plate-lock toast does too.
- **States**: two positions only; no visible/hidden state of its own — the caller
  conditionally mounts it.
- **Data**: props only.
- **Callbacks**: none.
- **Text**: none.
- **Port target**: same module as `Toast`; a `ToastViewport` is a positioned `div` in GPUI.

## Surface: `NotificationStack` — the editor's bottom-centre toast stack

- **Source**: `src/components/organisms/NotificationStack.tsx:1-241`
- **Renders**: a fragment of up to eight `ToastViewport`+`Toast` pairs, each independently
  gated on its own `visible` / record prop. They are **not** stacked in one container: each
  has its own `zIndex` and `offset`, and they overlap unless the code raises the offset.
- **Layout & styling** (per toast, in source order):

  | # | Gate | `ToastViewport` | `Toast` shape | Children |
  |---|---|---|---|---|
  | 1 | `isSaveToastVisible` | `zIndex={126} offset="1.25rem"` | `tone="info"`, `animated`, `visible={isSaveToastAnimatedVisible}`, `className="flex items-center gap-2"` | `<Spinner size="md" icon={RefreshCw} />` + `Saving…` or `Autosaving…` |
  | 2 | `historyActionToast` | `zIndex={125} offset="1.25rem"` | `tone = direction === 'undo' ? 'warning' : 'info'`, `animated`, `visible={isHistoryActionToastVisible}`, `className="flex items-center gap-2"` | `Undo2` (`undo`) or `Redo2` (`redo`), both `className="h-4 w-4 motion-safe:animate-pulse"`, + `_(historyActionToast.label)` |
  | 3 | `printingMonitorErrorToast` | `zIndex={126}`, `offset={(historyActionToast \|\| sceneImportReport \|\| orientationToast) ? '4.5rem' : '1.25rem'}` | `tone="error"`, `animated`, `visible={isPrintingMonitorErrorToastVisible}`, `className="flex items-center gap-2"` | `AlertTriangle h-4 w-4 motion-safe:animate-pulse` + `.text` |
  | 4 | `sceneImportReport` | `zIndex={125} offset="1.25rem"` | tone: `error`→`error`, `warning`→`warning`, else `success`; `animated`; `visible={isSceneImportToastVisible}`; `className` = `flex items-center gap-2 ` + (`clickAction === 'openMeshRepairReport'` ? `'pointer-events-auto cursor-pointer select-none'` : `''`); `role`/`tabIndex` set to `'button'`/`0` only when clickable | icon `AlertTriangle` for `error` **and** `warning`, `CheckCircle2` otherwise; + `.text` |
  | 5 | `exportSuccessToast` | `zIndex={125} offset="1.25rem"` | `tone="success"`, `animated`, `visible={isExportSuccessToastVisible}`, `flex items-center gap-2` | `CheckCircle2 h-4 w-4` + `Saved to: {path}` |
  | 6 | `exportErrorToast` | `zIndex={125} offset="1.25rem"` | `tone="error"`, `animated`, `visible={isExportErrorToastVisible}`, `flex items-center gap-2` | `AlertTriangle` + `.text` |
  | 7 | `orientationToast` | `zIndex={125}`, `offset={(historyActionToast \|\| sceneImportReport) ? '4.5rem' : '1.25rem'}` | `tone = status === 'applied' ? 'success' : 'info'`, `animated`, `visible={isOrientationToastVisible}`, `flex items-center gap-2` | `CheckCircle2` (applied) or `Info`; text `Successfully oriented {modelName}` or `Already optimal for this goal.` |

- **States**: every toast is controlled by two booleans — *mounted* (record non-null) and
  *visible* (drives the fade). Both must be honoured; see the timings table below.
- **Data**: props only (`NotificationStackProps`). No store reads inside.
- **Callbacks**:
  - toast 4: `onClick` and `onKeyDown` (`Enter`/`Space`, `preventDefault`) both call
    `onOpenMeshRepairReport()`, **only** when `sceneImportReport?.clickAction ===
    'openMeshRepairReport'`. Every other toast is inert.
- **Text** (verbatim, with msgid):
  - `Saving…` (`msg\`Saving…\``), `Autosaving…` (`msg\`Autosaving…\``).
  - `Already optimal for this goal.` — module-level `const ALREADY_OPTIMAL = msg\`…\``.
  - `Successfully oriented {modelName}` — msgid carries the comment
    `'{modelName} is the display name of the model that was reoriented.'`.
  - `Saved to: {path}` — comment `'{path} is where the exported file was written.'`.
  - Toast 2's label and toasts 3/4/6's text are data (`MessageDescriptor` / plain strings).
- **Port target**: `gpui/crates/gpui-app/src/features/toast/stack.rs`.

### Timings the stack's `visible` flags are driven by

`src/features/notifications/useEditorToasts.ts` unless stated otherwise. These are the
contract for when a rust toast must start and stop:

| Toast | Show | Fade (`visible=false`) | Unmount | Source |
|---|---|---|---|---|
| save | on `isSceneSaveInProgress \|\| (isAutosaving && !isPreSliceSceneSaveInProgress)`; `animatedVisible` flips on the **next rAF** | `remaining = max(0, 2000 - elapsed)`, then `TOAST_ANIMATION_MS = 220` later unmount | same effect | `useEditorToasts.ts:186-243` |
| history action | on `subscribeHistoryOperations` | 1400 ms | 1800 ms | `page.tsx:6059-6067` |
| printing monitor error | `setPrintingMonitorError` | 2200 ms | 2600 ms | `useEditorToasts.ts:120-140` |
| scene import | report becomes non-null | `durationMs ?? 4200`, fade at `max(0, duration - 400)` | report clears | `useEditorToasts.ts:246-273` |
| export success | `handleExportSuccess` | 3800 ms | — | `useEditorToasts.ts:275-283` |
| export error | `showOperationError` | 4500 ms | — | `useEditorToasts.ts:285-293` |
| orientation | `showOrientationToast` | 6000 ms | — | `useEditorToasts.ts:295-303` |
| plate locked | `notifyPlateLocked` | 2600 ms | `useTrailingMount` (+240 ms) | `page.tsx:861-871` |
| new input device | `handleNewDeviceDetected` | 9000 ms | — | `page.tsx:6084-6091` |

Two extra behaviours of the printing-monitor toast, quoted from
`normalizePrintingMonitorErrorMessage`: a message containing
`'tainted canvases may not be exported'` (lowercased compare) is replaced with
`'Unable to export this webcam frame directly. Retrying through the secure snapshot proxy.'`;
and a repeat of the same normalised message within **1500 ms** is dropped entirely.

`useTrailingMount(visible, tailMs = 240)` (`useEditorToasts.ts:44-56`) keeps a toast mounted
`tailMs` after `visible` goes false. Its doc comment:

> "A toast whose presence and whose `visible` flag are the same boolean is torn out of the
> tree in the commit that hides it, so its fade-out never gets to run and it blinks away."

## Surface: page-level toasts — plate locked, new 3D mouse, leaf fanning

- **Source**: `src/app/page.tsx:12062-12120`
- **Renders**: three more `ToastViewport` + `Toast` pairs, siblings of `NotificationStack`.

  1. **Plate locked** — `{plateLockedNoticeMounted && …}`, `ToastViewport zIndex={127}
     offset="1.25rem"`, `Toast tone="warning" shape="rounded" animated
     visible={plateLockedNoticeVisible} className="flex items-center max-w-sm
     pointer-events-auto"`. Body:
     ```tsx
     <span className="flex-1 text-center leading-snug">
       <span className="inline-flex items-center gap-1.5">
         <Lock className="h-3.5 w-3.5 flex-shrink-0" />
         {formatPlateLockedNotice(_, plateLockedNoticeLabel)}
       </span>
       <br />
       <span style={{ fontWeight: 400, opacity: 0.8 }}>{_(msg`Unlock it to add or move models.`)}</span>
     </span>
     ```
     Text: `{plateName} is locked.` (comment: `'{plateName} is the locked build plate, named
     as the Models panel names it: its own name, or "Plate 1".'`) and
     `Unlock it to add or move models.` The comment above the padlock explains placement:
     > "The padlock leads the first line rather than standing beside the block: centred on
     > two lines of text it read as belonging to neither."
     `plateName` resolution: `plate?.name.trim() || plateNumberPlaceholder(index + 1, _)`,
     where `plateNumberPlaceholder` is `msg\`Plate ${index}\`` (`src/features/scene/plates/plateMessages.ts:18`).

  2. **New input device** — `{newDeviceToast && …}`, `ToastViewport zIndex={127}
     offset="1.25rem"`, `Toast tone="warning" shape="rounded" animated
     visible={isNewDeviceToastVisible} className="flex items-center gap-3 max-w-sm
     pointer-events-auto"`. Body: `Gamepad2 h-4 w-4 flex-shrink-0`, then
     `New input device detected.` + `<br/>` + a `fontWeight 400, opacity 0.8` span reading
     `Go to Settings → 3D Mouse to configure or block it.`, then a Dismiss button:
     `className="flex-shrink-0 rounded px-2 py-0.5 text-[11px] font-semibold"`,
     `style={{ background: 'color-mix(in srgb, #f59e0b, transparent 80%)', color:
     'var(--text-strong)' }}`, text `Dismiss`, `onClick` sets `isNewDeviceToastVisible`
     false (it does **not** clear `newDeviceToast`).

  3. **Leaf fanning** — `{sproutParentingLockHeld && …}`, `ToastViewport zIndex={125}
     offset="1.25rem"`, `Toast tone="info" visible={true} className="flex items-center
     gap-2"` (no `animated`). Text is `stage === 'awaitingSproutTip'`
     ? `Leaf Fanning Active: Click model to sprout leaf`
     : `Leaf Fanning: Click a support shaft to lock anchor knot`.

- **Port target**: `gpui/crates/gpui-app/src/features/toast/stack.rs` (same file as
  `NotificationStack`; they are siblings in the shell).

## Surface: marquee mode toast — the only toast inside the 3D viewport

- **Source**: `src/components/scene/SceneCanvas/SceneCanvas.tsx:8557-8569`, label built at
  `SceneCanvas.tsx:3511-3528`
- **Renders**: `{marqueeSelection && (<div className="pointer-events-none absolute bottom-6
  left-1/2 z-50 -translate-x-1/2"><Toast …>{marqueeModeLabel}</Toast></div>)}` — a DOM
  child of the viewport container, not inside the r3f tree.
- **Layout & styling**: container is `pointer-events-none absolute bottom-6 left-1/2 z-50
  -translate-x-1/2`. The toast keeps `tone="neutral"` (so `--border-subtle` /
  `--surface-0`) and then overrides two properties inline:
  ```tsx
  borderColor: `color-mix(in srgb, ${marqueeHue}, var(--border-subtle) 40%)`,
  color: `color-mix(in srgb, ${marqueeHue}, var(--text-strong) 40%)`,
  ```
  `marqueeHue` is `marqueeMode === 'crossing' ? 'var(--accent)' : 'var(--accent-secondary)'`.
  **Note the mix orientation**: CSS `color-mix(A, B 40%)` is 40 % of `B` in `A`, so the hue
  dominates and the theme token is the minority — the same ratio idiom is used everywhere in
  this codebase, always with the *theme token* carrying the percentage.
- **States**: only mounted while a marquee drag is live; no animation.
- **Data**: `marqueeSelection` (drag rect), `marqueeMode` from `marqueeModeForDrag(start,
  current)`.
- **Callbacks**: none.
- **Text**:
  - `Selection Mode: Contact (Right to Left)` — comment: "Shown while dragging a selection
    rectangle right to left, which takes every object the rectangle touches. Mirrors the CAD
    convention; keep it on one short line."
  - `Selection Mode: Enclosed (Left to Right)` — comment: "…left to right, which takes only
    the objects the rectangle fully encloses…"
- **Port target**: `gpui/crates/gpui-app/src/features/scene/marquee_overlay.rs` (this spec
  only owns the toast, not the marquee rect).

---

## Surface: `EmptySceneState` — the full-viewport empty-scene card

- **Source**: `src/components/layout/EmptySceneState.tsx` (774 lines; every branch below)
- **Renders**: a centred card over the viewport with a build chip, a random tagline, a
  subtitle, and then exactly one of three bodies (loading / first-time onboarding / actions),
  followed by two hidden file inputs.
- **Layout & styling**:
  - Outer: `className="absolute inset-0 top-[var(--topbar-height)] z-30 flex items-center
    justify-center pointer-events-none"`.
  - Card: `className="ui-empty-state pointer-events-auto"`. From `src/app/globals.css:563`:
    ```css
    .ui-empty-state {
      background: color-mix(in srgb, var(--surface-0), transparent 6%);
      border: 1px solid color-mix(in srgb, var(--border-subtle), transparent 10%);
      border-radius: var(--radius-lg);
      padding: 20px;
      max-width: 560px;
      width: min(90vw, 560px);
      text-align: center;
      box-shadow: 0 16px 34px rgba(0, 0, 0, 0.22);
    }
    ```
  - Build chip: `<span role="button" tabIndex={0}>`, classes
    `"mb-2 inline-flex cursor-pointer items-center rounded-full border px-3.5 py-1 text-[13px]
    font-bold tabular-nums transition-colors"`. Three style variants, in priority order:
    - copied: `color: '#2d8a4e'`, `borderColor: '#2d8a4e'`,
      `background: 'rgba(45,138,78,0.12)'`, `boxShadow: '0 0 6px rgba(45,138,78,0.3)'`.
    - beta: `color: isLightTheme ? '#9a3412' : '#fdba74'`,
      `borderColor: 'color-mix(in srgb, #f97316, var(--border-subtle) 40%)'`,
      `background: isLightTheme ? 'color-mix(in srgb, #f97316, var(--surface-1) 90%)' :
      'color-mix(in srgb, #f97316, transparent 94%)'`,
      `boxShadow: isLightTheme ? 'none' : '0 0 6px color-mix(in srgb, #f97316, transparent 84%)'`.
    - normal: `color: 'var(--accent)'`,
      `borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 40%)'`,
      `background: 'color-mix(in srgb, var(--accent), transparent 94%)'`,
      `boxShadow: '0 0 6px color-mix(in srgb, var(--accent), transparent 84%)'`.
  - Title: `<h1 className="ui-empty-title" suppressHydrationWarning>` —
    `color: var(--text-strong); font-size: clamp(20px, 3vw, 28px); line-height:
    var(--line-height-tight); font-weight: 600; max-width: 460px;` centred,
    `margin-bottom: var(--space-3)`.
  - Subtitle: `<p className="ui-empty-text" style={{ maxWidth: 560, marginLeft: 'auto',
    marginRight: 'auto' }}>` — `color: var(--text-muted); font-size: var(--font-size-md);
    margin-bottom: var(--space-5)`.
  - Loading card: `"rounded-lg border px-4 py-4 text-left"`,
    `borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 65%)'`,
    `background: 'color-mix(in srgb, var(--surface-1), transparent 8%)'`. Title
    `text-sm font-semibold` in `var(--text-strong)`; detail `mt-1 text-[11px]` in
    `var(--text-muted)`; then `<ProgressBar indeterminate size="sm" className="mt-3" />`.
  - Onboarding card: `"rounded-lg border p-3 text-left"`,
    `borderColor: 'color-mix(in srgb, var(--border-subtle), transparent 8%)'`,
    `background: 'color-mix(in srgb, var(--surface-1), transparent 6%)'`; header
    `"mb-2 text-[10px] font-semibold uppercase tracking-wide"` in `var(--text-muted)`;
    `"grid gap-2 grid-cols-1"`.
    - Add-printer button: `"group rounded-md border px-3 py-3 text-left transition-colors"`,
      `background: 'var(--primary-button-surface)'`,
      `borderColor: 'color-mix(in srgb, var(--primary-button-surface), white 16%)'`,
      `color: 'var(--accent-contrast)'`. Title row
      `"mb-1 inline-flex items-center gap-1.5 text-sm font-semibold"` with
      `<Printer className="w-4 h-4" />`; sub-line `text-[11px]` in
      `color-mix(in srgb, var(--accent-contrast), black 16%)`.
    - Use-without-printer button: same shape with
      `background: 'var(--secondary-button-surface)'`,
      `borderColor: 'color-mix(in srgb, var(--secondary-button-surface), white 16%)'`,
      `color: 'var(--accent-secondary-contrast)'`, `<Wrench className="w-4 h-4" />`, sub-line
      in `color-mix(in srgb, var(--accent-secondary-contrast), black 18%)`.
    - Footer: `"mt-2 text-[11px]"` in `var(--text-muted)`.
  - Actions body:
    - Button grid: `grid gap-2 w-full` + (`onImportSceneChange ? 'grid-cols-2' :
      'grid-cols-1'`). Both buttons `"group cursor-pointer rounded-md border px-3 py-3
      text-center transition-colors"`; Load Mesh uses primary tokens, Import Scene secondary
      (same colour maths as the onboarding buttons). Title rows are
      `"mb-1 inline-flex items-center justify-center gap-1.5 text-sm font-semibold w-full"`.
    - Recent-files card: `"mt-2 rounded-lg border p-3 text-left"` with the same
      border/background as the onboarding card. Header
      `"flex items-center justify-between text-[10px]"`; `Clear` button
      `"rounded px-1.5 py-0.5 font-semibold uppercase tracking-wide transition-colors
      hover:bg-white/10"`; count pill `"inline-flex min-w-5 items-center justify-center
      rounded-full px-1.5 py-0.5 text-[9px] font-semibold"` with
      `background: 'color-mix(in srgb, var(--surface-2), transparent 26%)'`,
      `border: '1px solid color-mix(in srgb, var(--border-subtle), transparent 20%)'`.
    - Empty list: `"mt-1.5 flex h-[38px] items-center justify-center rounded-md text-[10px]"`.
    - List: `"mt-1.5 grid grid-cols-2 gap-1.5"`, entries
      `entry.kind === 'scene'` badge colour `isLightTheme ? '#c2410c' : '#fb923c'`, mesh
      `isLightTheme ? '#6d28d9' : '#a78bfa'`; badge background
      `` `color-mix(in srgb, ${kindAccent}, var(--surface-0) ${isLightTheme ? '84%' : '88%'})` ``,
      border `` `1px solid color-mix(in srgb, ${kindAccent}, var(--border-subtle) ${isLightTheme ? '38%' : '46%'})` ``.
      Row button: `"flex w-full items-center justify-between gap-2 rounded-md border px-2 py-1.5
      text-[10px] text-left transition-colors disabled:cursor-not-allowed"`, `color:
      'var(--text-strong)'`, `borderColor: 'color-mix(in srgb, var(--border-subtle),
      transparent 14%)'`, `background: 'color-mix(in srgb, var(--surface-1), transparent 8%)'`,
      `opacity: isBusy ? 0.65 : 1`. Name `"block max-w-[132px] truncate text-[10px]
      leading-tight"`; period `"block text-[9px]"` in `var(--text-muted)`.
    - Error line: `"mt-1 text-[10px]"` in `var(--danger)`.
    - Drop zone: `"mt-2 flex min-h-[64px] items-center justify-center rounded-md border
      border-dashed px-4 py-3 text-center transition-colors"` with
      `borderColor: isDropActive ? (isDropUnsupported ? 'var(--danger)' : 'var(--accent)') :
      'var(--border-subtle)'` and
      `background: isDropActive ? (isDropUnsupported ?
      'color-mix(in srgb, var(--danger), var(--surface-0) 88%)' :
      'color-mix(in srgb, var(--accent), var(--surface-0) 90%)') :
      'color-mix(in srgb, var(--surface-1), transparent 16%)'`. Icon
      `<Upload className="h-4 w-4 shrink-0" style={{ color: isDropUnsupported ? 'var(--danger)'
      : 'var(--accent)' }} />`; heading `"truncate text-sm font-semibold"` in
      `var(--text-strong)`; the unsupported sub-line `"mt-1 text-[11px]"` in `var(--danger)`.
  - Hidden inputs: `#empty-state-stl-file-input` with `accept=".stl,.obj,.3mf,.zip"`, and
    `#empty-state-scene-file-input` with `accept={sceneFileAccept}`, both `multiple`,
    `className="hidden"`.
- **States** (the body branch is a three-way `isLoading ? … : shouldShowFirstTimeOnboarding ?
  … : …`):
  - `isLoading` — loading card. `loadingLabel` defaults to `Importing your file…`,
    `loadingDetail` to `Please hang tight while we prepare your scene.`.
  - `showFirstTimeOnboarding && !isLoading` (`shouldShowFirstTimeOnboarding`) — onboarding card.
  - otherwise — the actions body.
  - Drop zone sub-states: idle, `isDropActive` (accent), `isDropActive && isDropUnsupported`
    (danger, heading becomes `Unsupported file type`, sub-line appears).
  - Recent rows: idle; `isBusy` (opacity 0.65, right label `loading…`);
    disabled (`disabled={isBusy || isLoading || !onReopenRecentFile}`).
  - Build chip: copied (green, for 2000 ms), beta, normal.
  - `reopenError` — red line under the recent list.
- **Data**: `useLingui()` `_`; `useLocaleFormat()` `fmt`; `useSceneFileExtensions()`,
  `useSceneFileExtensionLabels()`, `useSceneFileInputAccept()` from the plugin registry;
  local state `copied`, `isDropActive`, `isDropUnsupported`, `reopeningEntryId`,
  `reopenError`, `isLightTheme`, and a **one-shot random tagline**:
  `React.useState(() => TAGLINES[Math.floor(Math.random() * TAGLINES.length)])`.
  `isLightTheme` is recomputed by a `MutationObserver` on `document.documentElement`
  (`['class', 'data-theme']`) plus a `matchMedia('(prefers-color-scheme: light)')` listener,
  with the rule: light if the `dragonfruit-light` class is present, or
  `data-theme === 'light'`, or the OS prefers light **and** `dragonfruit-dark` is absent.
- **Callbacks**:
  - Build chip `onClick` / `onKeyDown` (`Enter`, `' '`) → `handleCopyBuildInfo`: writes
    `` `DragonFruit ${APP_VERSION} (${BUILD_CHANNEL})${FULL_BUILD_LABEL ? ` — ${FULL_BUILD_LABEL}` : ''}` ``
    to the clipboard, sets `copied` for 2000 ms; a clipboard failure is swallowed
    (`catch { /* clipboard unavailable */ }`).
  - Add-printer button → `onAddPrinter`; Use-without-printer → `onUseWithoutPrinter`.
  - Load Mesh → `triggerMeshPicker`: `onLoadMeshClick()` if provided, else clicks
    `#empty-state-stl-file-input`. Import Scene → `triggerScenePicker`, same fallback with
    `#empty-state-scene-file-input`.
  - File inputs `onChange` → `onFileChange` / `onImportSceneChange`.
  - `Clear` → `onClearRecentFiles` (rendered only when the prop exists **and** the list is
    non-empty).
  - Recent row → `handleReopenRecentFile(entry.id)`: clears `reopenError`, sets
    `reopeningEntryId`, awaits `onReopenRecentFile(id)`; `false` **or** a thrown error sets
    `Could not reopen this file from cache.`; finally clears `reopeningEntryId`.
  - Drag: `onDragOver` / `onDragEnter` / `onDragLeave` / `onDrop` on the drop zone (see
    gotchas for the exact protocol).
- **Text** (verbatim; msgids identical unless noted):
  - Build chip: `Click to copy build info` (tooltip; when `FULL_BUILD_LABEL` is set the
    tooltip is `` `${_(msg\`Click to copy build info\`)}\n${FULL_BUILD_LABEL}` `` rendered in
    a `whitespace-pre-line` span), `Copied!` (rendered as `` `✓ ${…}` ``),
    `Version ${version}` and `Version ${version} — Beta`.
  - Subtitle: `Bring in a mesh or scene to start preparing, analyzing, supporting, and
    exporting your print.`
  - Loading: `Importing your file…`, `Please hang tight while we prepare your scene.`
  - Onboarding: `Get started`, `Add printer`, `Open printer library and add one now.`,
    `Use without printer`, `Keep going without a printer. You can add one later.`,
    `Add or switch printer anytime from the top bar.`
  - Actions: `Load Mesh`, `Import Scene`, `Recent files`, `Clear`, `No recent files`,
    `Reopen ${entry.name}` (row `title`), `Scene` (badge, comment "File type badge on the
    recent files list. Noun — labels a saved scene file (.voxl, .lys)."), `Mesh` (badge,
    comment "…labels a mesh file (.stl, .obj, .3mf)."), `last opened just now`,
    `last opened ${period} ago`, `loading…` (comment "Shown in place of file size while a
    recent file is being reopened. Keep it short."), `cached` (comment "Shown in place of
    file size when the file is in cache but its size is unknown. Adjective/state label."),
    `Drag & Drop Area` (comment "Heading of the dashed file target in the empty-scene
    dialog."), `Unsupported file type`, `Unsupported format detected.`
  - Period strings: `${deltaMin} min`, `${deltaHours} h`, `${deltaDays} d`.
  - `Could not reopen this file from cache.`
  - Taglines — the list is `TAGLINES` (`EmptySceneState.tsx:75-108`), all `msg` descriptors,
    with translator comments on the pop-culture ones:
    `Ready for your next adventure?`, `The build plate awaits.`,
    `What are we printing today?`, `Time to make something real.`,
    `Every great print starts with an empty scene.`, `Nothing to see here. Yet.`,
    `Your models called. They want to be sliced.`, `Suspiciously empty in here.`,
    `This scene is aggressively unoccupied.`, `Zero polygons. Infinite potential.`,
    `The resin is ready. Are you?`, `Currently displaying nothing at maximum fidelity.`,
    `Idle hands do no slicing.` (comment: Adaptation of the English proverb "Idle hands are
    the devil's workshop".), `The build volume misses you.`,
    `Drag a file in. The platform is judging you.`,
    `Do or do not. There is no try... oh wait, just click Load mesh.`,
    `In space, no one can hear you slice.`,
    `You shall not pass... until you load a model.`,
    `I am the one who slices.`, `With great resin comes great responsibility.`,
    `Why so empty?`, `We're gonna need a bigger build plate.`,
    `One does not simply import a mesh... or actually, you just click a button.`,
    `I'll be back. (Drop a file and I really will be.)`,
    `It's a trap! ...just kidding, drop your STL.`,
    `The spice must flow. The resin must cure.`,
    `Elementary, my dear user — load a model.`,
    `Roads? Where we're printing, we don't need roads.`,
    `Waiting for first layer adhesion... to reality.`,
    `No supports needed for an empty scene.`,
    `Layer 0 of 0. Living dangerously.`,
    `Anti-aliasing: nothing to anti-alias.`,
    `Join the Open Resin Alliance!`,
    `Not all slicers are created equal. This one's free!`.
- **Port target**: `gpui/crates/gpui-app/src/features/scene/empty_state.rs`.

---

## Surface: `LayerSlider` — the vertical layer/cross-section rail

- **Source**: `src/components/controls/LayerSlider.tsx` (1087 lines)
- **Renders**: a vertical rail with an optional header block, one or two thumbs on a shared
  track, an optional value field below, and a click-to-type popover per thumb. Three layouts
  from the same component: standalone floating panel, `docked`, and `embedded` (with a
  `minimal rail` variant when `embedded && docked`).
- **Layout & styling**:
  - Derived flags: `isMinimalRail = embedded && docked`;
    `isCompactMinimalRail = isMinimalRail && compactMinimalRail`.
  - Outer wrapper:
    ```tsx
    docked
      ? `relative z-10 select-none ${className ?? ''}`
      : `absolute right-3 top-1/2 -translate-y-1/2 z-10 select-none ${className ?? ''}`
    ```
  - Inner card, three shapes:
    ```tsx
    embedded
      ? isCompactMinimalRail
        ? `${expandToContainer ? 'h-full min-h-0 flex flex-col' : ''} mx-auto w-[34px] pt-1 pb-0`
        : `${expandToContainer ? 'h-full min-h-0 flex flex-col' : ''} w-full rounded-lg ${isMinimalRail ? 'px-0 py-1.5' : 'px-1 py-1'}`
      : 'ui-panel w-44 rounded-lg px-2.5 py-2.5 shadow-lg'
    ```
    Non-embedded inline style: `background: 'color-mix(in srgb, var(--surface-0),
    transparent 10%)'`, `borderColor: 'var(--border-subtle)'`. (`.ui-panel` itself supplies
    `background: var(--surface-0); border: 1px solid var(--border-subtle); font-size:
    var(--font-size-sm)` — the inline style only overrides background and border colour.)
  - Rail box (`railClassName`), used **twice** — once as the `Tooltip`'s
    `wrapperClassName` and once on the `div` that takes the pointer:
    ```
    `relative mx-auto ${embedded ? (expandToContainer ? 'flex-1 h-full min-h-[300px]' : 'h-[46vh]') : 'h-[56vh]'} ${embedded ? (isMinimalRail ? (isCompactMinimalRail ? 'w-4' : 'w-5') : 'w-7') : 'w-10'} cursor-pointer`
    ```
  - Header (`!isMinimalRail` only): a row `"flex items-center justify-between"` with the
    `Layer` caption (`${embedded ? 'text-[10px]' : 'text-[11px]'} font-semibold uppercase
    tracking-wide`, colour `var(--text-muted)`) and the rounded value
    (`"text-xs font-semibold tabular-nums"`, `var(--text-strong)`). Under it the height chip
    `"mt-0.5 inline-flex items-center rounded-md border px-1.5 py-0.5 text-[10px] tabular-nums"`
    with `color: var(--text-muted)`,
    `borderColor: color-mix(in srgb, var(--border-subtle), transparent 25%)`,
    `background: color-mix(in srgb, var(--surface-1), transparent 12%)`; and, only when
    `maxHeightMm` is a number **and** not embedded, `Max {formatMm(maxHeightMm)}` in
    `"text-[10px] tabular-nums"`.
  - Track: `"absolute left-1/2 top-0 h-full w-1.5 -translate-x-1/2 rounded-full"`,
    `background: 'color-mix(in srgb, var(--surface-2), black 8%)'`,
    `border: '1px solid color-mix(in srgb, var(--border-subtle), transparent 40%)'`.
  - Fill: `"absolute left-1/2 -translate-x-1/2 w-1.5 rounded-full"`,
    `bottom: `${lowerPercent ?? 0}%``, `height: `${percent - (lowerPercent ?? 0)}%``,
    background `linear-gradient(180deg, color-mix(in srgb, var(--accent), white 14%),
    var(--accent))` when cross-section is enabled, else the same gradient over
    `var(--text-muted)`; `boxShadow` `0 0 10px color-mix(in srgb, var(--accent),
    transparent 65%)` or `none`; `opacity` `1` or `0.45`.
  - Thumb (`h-[9px] w-[24px] rounded-full border`, `transition-[transform,background,box-shadow]
    duration-150`, `scale-105` while dragging else `scale-100`):
    `borderColor: `color-mix(in srgb, white, ${thumbColor} 20%)``,
    `background: `linear-gradient(90deg, color-mix(in srgb, ${thumbColor}, white 20%),
    ${thumbColor}, color-mix(in srgb, ${thumbColor}, white 20%))``,
    `boxShadow` two variants (dragging: `0 0 0 2px color-mix(in srgb, ${thumbColor},
    transparent 65%), 0 6px 14px rgba(0,0,0,0.38)`; idle: `… transparent 80%), 0 4px 10px
    rgba(0,0,0,0.35)`). The **lower** thumb additionally carries `opacity: 0.75`; the upper
    does not.
    `thumbColor = crossSectionEnabled ? 'var(--accent)' : 'var(--text-muted)'`.
  - Value badges: upper, non-minimal →
    `"absolute left-full ml-2 top-1/2 -translate-y-1/2 whitespace-nowrap rounded border
    px-1.5 py-0.5 text-[10px] shadow tabular-nums pointer-events-none"` with
    `borderColor: var(--border-subtle)`,
    `background: color-mix(in srgb, var(--surface-0), transparent 12%)`,
    `color: var(--text-strong)`, text `formatMm(displayHeightMm)`. Upper, minimal rail →
    `absolute left-1/2 -translate-x-1/2 whitespace-nowrap ${minimalRailBadgeClass}
    pointer-events-none ${shouldPlaceCurrentBadgeBelowThumb ? 'top-3' : '-top-5'}`, text the
    bare rounded layer number, style `railCurrentBadgeStyle`. Lower mirrors this with
    `top-3` always in minimal mode and `right-full mr-2` otherwise.
  - Minimal-rail end badges: `max` at the top inside `"flex items-center justify-center mb-1"`,
    `min` at the bottom inside `"mt-1 flex items-center justify-center gap-1.5"`, both using
    `railBadgeClass` / `minimalRailBadgeClass` and `railBadgeStyle`:
    ```tsx
    const railBadgeClass = 'inline-flex items-center rounded-md border px-1 py-0.5 text-[9px] font-semibold tabular-nums';
    const railBadgeStyle = {
      color: 'var(--text-muted)',
      borderColor: 'color-mix(in srgb, var(--border-subtle), transparent 15%)',
      background: 'color-mix(in srgb, var(--surface-1), transparent 10%)',
    };
    const railCurrentBadgeStyle = {
      color: 'var(--text-strong)',
      borderColor: 'color-mix(in srgb, var(--border-subtle), transparent 10%)',
      background: 'color-mix(in srgb, var(--surface-1), transparent 4%)',
    };
    ```
    compact variant: `'inline-flex items-center rounded-md border px-0.5 py-0 text-[8px]
    font-semibold tabular-nums'`.
  - Non-minimal `min`/`max` captions: `max` floats above the rail
    (`"absolute left-1/2 -translate-x-1/2 -top-5 text-[10px] tabular-nums"`), `min` sits
    below (`"mt-0 text-center text-[10px] tabular-nums"`), both `var(--text-muted)`.
  - Value field (only `showValue && !isMinimalRail`):
    `"mt-2 w-full rounded border px-1.5 py-1 text-center text-xs shadow tabular-nums
    focus:outline-none transition-colors"`. Normal: `borderColor: 'var(--border-subtle)'`,
    `background: 'color-mix(in srgb, var(--surface-0), transparent 10%)'`,
    `color: 'var(--text-strong)'`. Error: `borderColor: 'var(--danger)'`,
    `background: 'color-mix(in srgb, #ef4444, var(--surface-0) 88%)'`,
    `color: 'var(--danger)'`.
  - Edit popover: `"absolute left-full top-1/2 -translate-y-1/2 z-[200] flex flex-col gap-1.5
    rounded-lg border p-2"` (opening right, `marginLeft: '10px'`) or
    `"absolute right-full top-1/2 -translate-y-1/2 z-[200] flex flex-col gap-1.5 rounded-lg
    border p-2"` (opening left, `marginRight: '10px'`), plus `minWidth: '136px'`,
    `background: 'var(--surface-0)'`, `borderColor: 'var(--border-subtle)'`,
    `boxShadow: '0 8px 24px rgba(0,0,0,0.45)'`.
    - Mode buttons (`Layer #` / `mm`, only when `layerHeightMm > 0`): `"flex-1 rounded px-1.5
      py-0.5 text-[10px] font-medium transition-colors"`; active
      `{ background: 'var(--secondary-button-surface)', color: 'var(--accent-secondary-contrast)' }`,
      inactive `{ background: 'color-mix(in srgb, var(--surface-2), transparent 20%)', color:
      'var(--text-muted)' }`.
    - Input: `"w-full rounded border px-1.5 py-1 text-center text-[11px] tabular-nums
      focus:outline-none selection:bg-transparent selection:text-[var(--accent)]"` with
      `borderColor: 'var(--border-subtle)'`,
      `background: 'color-mix(in srgb, var(--surface-1), transparent 10%)'`,
      `color: 'var(--text-strong)'`; `inputMode="decimal"`; placeholder
      `editMode === 'layer' ? _(msg\`Layer #\`) : _(msg\`Height mm\`)`.
  - Shift indicator: `"absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2
    pointer-events-none"` wrapping `<svg width="52" height="52" viewBox="0 0 52 52"
    className="drop-shadow-lg">` with four arc paths, all `stroke="#3b82f6" strokeWidth="2.5"
    strokeLinecap="round" fill="none"`:
    `d="M 9.5 14.5 A 24 24 0 0 0 2 26"`, `d="M 9.5 37.5 A 24 24 0 0 1 2 26"` (outer r=24),
    `d="M 14.0 17.0 A 17 17 0 0 0 9 26"`, `d="M 14.0 35.0 A 17 17 0 0 1 9 26"` (inner r=17).
- **States**:
  - cross-section on/off — accent vs muted thumb and fill, `opacity 0.45` fill, and the
    tooltip's last clause.
  - dragging (`isDraggingThumb`) — thumb `scale-105` and the wide shadow ring.
  - shift held (`isShiftHeld`) — precision arcs drawn.
  - glide (`easesPosition`) — `top` / `bottom` / `height` transition
    `170ms cubic-bezier(0.22, 1, 0.36, 1)`; otherwise `none`.
  - thumb edit open (`editingThumb`), with `editMode` `layer` | `mm`.
  - value-field error (`showError`), auto-cleared after 1000 ms.
  - edge badges hidden at the ends: `hideUpperFloatingBadge = percent <= 1e-6 || percent >=
    100 - 1e-6` (same for the lower badge).
  - thumb labels: `shouldPlaceCurrentBadgeBelowThumb = isMinimalRail && percent >= 96`.
- **Data**: props only. Display values fall back through live scrub state:
  ```tsx
  const displayValue = liveValue ?? value;
  const displayLowerValue = liveLowerValue ?? lowerValue;
  ```
  with `displayHeightMm` recomputed from `displayValue * layerHeightMm` clamped to
  `[0, maxHeightMm]` only while `liveValue != null`; otherwise the owner's `currentHeightMm`
  is used. Same for the lower thumb with `lowerCurrentHeightMm`.
  `percent = Math.min(100, Math.max(0, ((displayValue - min) / Math.max(1, (max - min))) * 100))`.
- **Callbacks**:
  - `onChange(next)` on every accepted move (wheel, arrow keys, drag, value field, popover
    commit). Always `Math.max(clamp(snap(raw)), lowerValueRef.current)` — the upper thumb can
    never cross the lower one.
  - `onLowerChange(next)` — `Math.min(clamp(snap(raw)), valueRef.current)`.
  - `onScrubStart()` once on left-press that actually begins a drag; `onScrubEnd()` on
    mouse-up / window blur, **and** on a press that is refused (bare track click).
  - `onToggleCrossSection()` on double-click of the rail (only when provided;
    `preventDefault` + `stopPropagation`).
  - `onChange` / `onLowerChange` fire **during** the scrub too — the slider does not buffer
    them; see gotchas for why that is safe.
- **Text** / msgids: `Layer`, `Layer #`, `mm` (literal, not translated), `Height mm`,
  `Max {mm}`, and the minimal-rail tooltip from `formatMinimalRailTitle`:
  - cross-section on: `Layer ${layer} • ${height} • Right-click a handle to type an exact
    value • Double-click to turn the cross-section off`
  - off: `… • Double-click to turn the cross-section back on`
  - no toggle: `Layer ${layer} • ${height} • Right-click a handle to type an exact value`
  All three are `msg` with interpolations; the tooltip body is a
  `<span className="whitespace-pre-line">` because the string uses `•` separators.
- **Port target**: `gpui/crates/gpui-app/src/features/scene/layer_slider.rs`.

## Surface: `CrossSliderPanel` — the 48 px right-edge host for the rail

- **Source**: `src/components/controls/CrossSliderPanel.tsx` (1-60)
- **Renders**: `<Card className="h-full flex flex-col">` → padding wrapper → fixed-height rail
  area → one `LayerSlider` in embedded/docked/compact mode.
- **Layout & styling**: `Card` contributes `ui-panel rounded-lg shadow-lg border` plus
  `borderColor: var(--border-subtle)`. Wrapper: `"px-0 py-1 min-h-0 flex-1 flex flex-col"`;
  rail area: `"flex-1 min-h-[220px] overflow-visible"`.
- **States**: none of its own; it forwards `crossSectionEnabled` etc.
- **Data**: props only.
- **Callbacks**: `handleLayerChange` rounds before forwarding:
  `onLayerIndexChange(Math.round(nextValue))`.
- **Text**: none.
- **Port target**: same module as `LayerSlider` (it is a 20-line wrapper), or
  `gpui/.../scene/cross_slider_panel.rs`.

### The `LayerSlider` call it makes (fixed props)

```tsx
<LayerSlider
  min={0} max={maxLayers} step={1}
  value={layerIndex} onChange={handleLayerChange}
  onScrubStart={onScrubStart} onScrubEnd={onScrubEnd}
  currentHeightMm={currentHeightMm} maxHeightMm={maxHeightMm}
  showValue={true}
  lowerValue={lowerLayerIndex} onLowerChange={onLowerLayerIndexChange}
  lowerCurrentHeightMm={lowerCurrentHeightMm}
  crossSectionEnabled={crossSectionEnabled} onToggleCrossSection={onToggleCrossSection}
  layerHeightMm={layerHeightMm}
  compactMinimalRail docked embedded expandToContainer
  className="h-full"
/>
```

The host at `src/app/page.tsx:10993-10999` is:
```tsx
<div className="absolute right-0 z-10 pointer-events-auto"
     style={{ top: 'calc(var(--topbar-height) + 6px)', bottom: 6, width: 48 }}>
```
rendered only when `scene.models.length > 0 && scene.mode !== 'printing'`. Scrub start/end
call `handleSceneLayerScrubStart()` + `slicing.beginScrub()` and the matching `…End` +
`slicing.endScrub()`.

---

## Surface: `ZUpGizmoHelper` — the HUD container for the view cube

- **Source**: `src/components/scene/SceneCanvas/ZUpGizmoHelper.tsx` (501 lines)
- **Renders**: a drei `Hud` with its own orthographic camera; inside it, a group positioned
  by `alignment`/`margin` holding (a) the children (the cube), (b) four screen-aligned
  quarter-turn arrows that fade with how face-on the view is, and (c) an always-visible home
  chip on the bottom-right diagonal.
- **Layout & styling** — anchor maths, quoted:
  ```tsx
  const x = alignment.endsWith('-center') ? 0
    : alignment.endsWith('-left') ? -size.width / 2 + marginX
    : size.width / 2 - marginX;
  const y = alignment.startsWith('center-') ? 0
    : alignment.startsWith('top-') ? size.height / 2 - marginY
    : -size.height / 2 + marginY;
  ```
  Groups are `position={[x, y, 0]}`; the arrow and home groups additionally use
  `scale={[60, 60, 60]}`, matching `ZUpGizmoViewcube`'s own `scale={[60,60,60]}`.
  - Arrow geometry: `ARROW_DISTANCE = 0.74`, `ARROW_WIDTH = 0.26`, `ARROW_HEIGHT = 0.15`,
    a flat `BufferGeometry` triangle with vertices
    `[0, H*0.5, 0, -W*0.5, -H*0.5, 0, W*0.5, -H*0.5, 0]`, apex +Y.
  - Arrows: `up` at `[0, 0.74, 0]` `rotation={Math.PI}`; `down` at `[0, -0.74, 0]`
    `rotation={0}`; `left` at `[-0.74, 0, 0]` `rotation={-Math.PI/2}`; `right` at
    `[0.74, 0, 0]` `rotation={Math.PI/2}`. Material: `meshBasicMaterial color={accentColor}
    transparent opacity={(hover ? 1 : 0.9) * fade} side={DoubleSide}`. Hover scales the mesh
    to `1.18`.
  - Home chip: `HOME_OFFSET = 0.93`, `HOME_SIZE = 0.47`, position
    `[HOME_OFFSET, -HOME_OFFSET, 0]`, plane `[0.47, 0.47]`, `meshBasicMaterial map
    transparent opacity={hover ? 1 : 0.9} depthWrite={false} side={DoubleSide}`, hover scale
    `1.15`. Its texture is a 64×64 canvas: `roundRect(0.5, 0.5, 63, 63, 16)` filled with
    `chipColor` and stroked 1 px with `chipBorderColor`, then lucide's `house` icon
    rasterised at 50 px (viewBox 24) with **closed** sub-paths filled in `accentColor` and
    open ones (the door) filled in `chipColor`.
  - Defaults: `accentColor = '#baf72e'`, `chipColor = '#20242c'`,
    `chipBorderColor = '#3a3f4b'`, `alignment = 'bottom-right'`, `margin = [80, 80]`,
    `renderPriority = 1`.
- **States**:
  - arrows visible only when `arrowFade > 0`; fade computed every frame from the camera's
    world direction:
    ```tsx
    const faceOn = Math.max(|dir.x|, |dir.y|, |dir.z|);
    const rawFade = clamp((faceOn - ARROW_FADE_ZERO_COS) / max(1e-6, ARROW_FADE_FULL_COS - ARROW_FADE_ZERO_COS), 0, 1);
    const fade = rawFade <= 0.02 ? 0 : rawFade >= 0.98 ? 1 : rawFade * rawFade * (3 - 2 * rawFade);
    ```
    with `ARROW_FADE_FULL_DEGREES = 6`, `ARROW_FADE_ZERO_DEGREES = 20`
    (`ARROW_FADE_FULL_COS = cos(6°)`, `ARROW_FADE_ZERO_COS = cos(20°)`). State only updates
    when `|fade - previous| > 0.01`.
  - camera tween in flight (`animating`), snapped when `q1.angleTo(q2) < 0.01` with
    `turnRate = 4 * Math.PI` rad/s.
  - home chip hover; arrow hover.
- **Data**: `useThree` → `size`, `camera`, `controls`, `invalidate`. Refs for the tween
  (`gizmoRef`, `virtualCam`, `animating`, `radius`, `focusPoint`, `savedControlsEnabled`,
  `arrowFadeRef`). The gizmo mirror is
  `matrix.copy(mainCamera.matrix).invert(); gizmoRef.current.quaternion.setFromRotationMatrix(matrix)`.
- **Callbacks**:
  - Child cube calls `tweenCamera(direction)` (through the context) → starts the tween,
    disabling the scene controls for the duration and restoring them after.
  - `quarterTurn(dir)` rotates the camera offset 90° about the camera's own right/up axis,
    then calls `tweenCamera`.
  - Arrow mesh `onClick` → `quarterTurn(direction)`; `pointerOver/Out` → hover.
  - Home chip `onClick` → `onHome`.
  - `onUpdate?.()` each animated frame; `onTarget?.()` resolves the focus point.
- **Text**: none. (Label text is the viewcube's.)
- **Port target**: `gpui/crates/gpui-app/src/features/scene/view_cube/helper.rs`.

## Surface: `ZUpGizmoViewcube` — the rotating labelled cube

- **Source**: `src/components/scene/SceneCanvas/ZUpGizmoViewcube.tsx` (1-200)
- **Renders**: `<group scale={[60,60,60]} rotation={[0, 0, -Math.PI/2]}>` containing one
  six-material box, twelve edge pads, eight corner pads.
- **Layout & styling**:
  - `GIZMO_Z_ROTATION = -Math.PI / 2`; all `position` vectors are
    `new Vector3(...).multiplyScalar(0.38)`.
  - Face canvas: 128×128, `fillStyle = color` (`#1f2937` from the caller), `strokeStyle =
    strokeColor` (`--accent-secondary`), `strokeRect(0, 0, 128, 128)`; label is
    `faces[index].toUpperCase()` fitted with `fitFontToWidth(context, font, label, 112)`
    (`FACE_LABEL_MAX_WIDTH = 112`), centred at `(64, 64)` with `textAlign: 'center'`,
    `textBaseline: 'middle'`, `fillStyle = textColor`.
  - Per-face rotation, quoted:
    ```ts
    const needsRotation = [-Math.PI / 2, Math.PI / 2, Math.PI, 0, -Math.PI / 2, -Math.PI / 2][index];
    ```
    applied about the canvas centre before filling the text.
  - Face material: `meshBasicMaterial map={texture} map-anisotropy={gl.capabilities.getMaxAnisotropy() || 1}
    attach={`material-${index}`} color={hover ? hoverColor : 'white'} transparent opacity={opacity}`.
  - Edge/corner pads: `mesh scale={1.01}`, `meshBasicMaterial color={hover ? hoverColor : 'white'}
    transparent opacity={0.6} visible={hover}`, `boxGeometry args={dimensions}`.
    Edge dimensions are `0.5` along the axis that is `0` in their direction vector and `0.25`
    along the other two; corner pads are `[0.25, 0.25, 0.25]`.
  - Defaults when the caller omits them: `colors = { bg: '#f0f0f0', hover: '#999', text:
    'black', stroke: 'black' }`; `opacity = 1`; `font = '20px Inter var, Arial, sans-serif'`;
    `faces = ['Front', 'Back', 'Right', 'Left', 'Top', 'Bottom']` (marked
    `// i18n-ignore: fallback only; SceneCanvas passes the translated face labels.`).
- **States**: face hover (`hover === index` → `color = hoverColor`); pad hover (`visible`,
  `color = hoverColor`).
- **Data**: context `{ tweenCamera, quarterTurn }`; local `hover` index (faces) / boolean
  (pads).
- **Callbacks**:
  - `handlePointerMove` on the box → `setHover(Math.floor(e.faceIndex! / 2))` (two triangles
    per face).
  - Box `onClick` → `tweenCamera(rotateTweenTarget(e.face!.normal))`, where
    `rotateTweenTarget(v) = v.clone().applyAxisAngle(Z_AXIS, GIZMO_Z_ROTATION)`.
  - Pad `onClick` → `tweenCamera(rotateTweenTarget(position))`.
  - Both `stopPropagation()` on every pointer event.
  - A caller-supplied `onClick` **replaces** the built-in camera tween on both the box and
    the pads (`onClick={props.onClick || handleClick}`), so a caller passing `onClick`
    disables the cube's navigation — the current caller does not.
- **Text**: `Front`, `Back`, `Right`, `Left`, `Top`, `Bottom` — supplied by
  `SceneCanvas`'s `gizmoFaceLabels` (`SceneCanvas.tsx:1920-1931`), each with a translator
  comment; `Front`'s comment notes it is "shared with the build plate's front-edge marker".
  Face order is fixed: "Face order is fixed by the box geometry: +X, -X, +Y, -Y, +Z, -Z."
- **Port target**: `gpui/crates/gpui-app/src/features/scene/view_cube/cube.rs`.

### How `SceneCanvas` mounts the pair

`SceneCanvas.tsx:8202-8221`:
```tsx
{!thumbnailCaptureActive && cameraInteractionCycleEnabled && (
  <ZUpGizmoHelper
    alignment="bottom-right"
    margin={mode === 'printing' ? [82, 82] : [nonPrintingViewCubeRightMargin, 82]}
    accentColor={gizmoColors.accent}
    chipColor={gizmoColors.chip}
    chipBorderColor={gizmoColors.chipBorder}
    onHome={resetCameraHome}
  >
    <ZUpGizmoViewcube
      font="600 24px Inter, system-ui, sans-serif"
      faces={gizmoFaceLabels}
      color={gizmoColors.face}
      textColor={gizmoColors.text}
      strokeColor={gizmoColors.accent}
      hoverColor={gizmoColors.accent}
      opacity={0.75}
    />
  </ZUpGizmoHelper>
)}
```
Margins (`SceneCanvas.tsx:377-384, 1008-1009`):
```ts
const FLOATING_PANEL_RIGHT_INSET_PX = 12;
const VIEW_CUBE_HALF_EXTENT_PX = 30;
const VIEW_CUBE_PANEL_GAP_PX = 42;
const nonPrintingViewCubeRightMargin = 48 + FLOATING_PANEL_RIGHT_INSET_PX + VIEW_CUBE_PANEL_GAP_PX + VIEW_CUBE_HALF_EXTENT_PX; // = 132
```
with the source comment: "Drei GizmoHelper positions by gizmo center, not right edge.
GizmoViewcube renders at scale [60,60,60] on a unit box, so half-extent is 30px." and
"Bottom margin is 72px to cube center; cube half-height is 30px, giving 42px gap from
bottom. Match this gap on the right side so the cube feels equally spaced from panel and
screen bottom." — note the comment says 72 while the code passes **82**; the 82 is the
value to port.

Colours are re-derived from the document's CSS variables on every theme change
(`SceneCanvas.tsx:2556-2630`): `accent = --accent-secondary` (fallback `#baf72e`),
`face = --surface-1` (fallback `#1f2937`), `text = --text-strong` (fallback `#f8fafc`),
`chip = --surface-0` (fallback `#20242c`), `chipBorder = --border-subtle` (fallback
`#3a3f4b`), each passed through `new THREE.Color().setStyle(raw).getStyle()` so the values
are normalised CSS colours; a `MutationObserver` on `['class', 'style', 'data-theme']`
re-runs it.

---

## Surface: `plateWidgetDraw` — shared painting kit for the plate widgets

- **Source**: `src/components/scene/SceneCanvas/plateWidgetDraw.ts` (1-180)
- **Renders**: nothing; exports the CSS→world conversion, the theme colour set, the icon
  stroker and the rounded-rect path.
- **Layout & styling**:
  - `CSS_PIXELS_PER_WORLD_UNIT = 40` — quoted comment:
    > "drei's `Html` sizes one CSS pixel of its content as 1/40 of a world unit at its
    > default `distanceFactor` of 10 — the same ratio it scales its occlusion mesh by —
    > multiplied by the element's own `scale`. A canvas plane for the same widget has to
    > keep that conversion, or the widget changes size on the plate."
  - `widgetWorldPerCssPixel(labelScale) = labelScale / 40`.
  - `PlateWidgetColors`: `accent` (`--accent`, the plate's name and the locked wash),
    `textStrong` (`--text-strong`, locked icon), `textMuted` (`--text-muted`, quiet buttons
    and an unnamed plate), `surface0` (`--surface-0`, quiet panel fill), `surface1`
    (`--surface-1`, what the locked wash sits on).
  - `FALLBACK_COLORS` (dark theme): `accent: '#ec2a77'`, `textStrong: '#f8f8fb'`,
    `textMuted: '#c3c7cf'`, `surface0: '#111216'`, `surface1: '#1a1b21'`.
  - `roundedRectPath(x, y, width, height, radius)` clamps `r = max(0, min(radius, width*0.5,
    height*0.5))` and builds a `Path2D` with four `arcTo` corners.
  - `drawPlateWidgetIcon(ctx, icon, {x, y, size}, color)`: scales `size / 24`, sets
    `lineWidth = 2`, `lineCap = 'round'`, `lineJoin = 'round'` and strokes each node,
    translating `path` (`new Path2D(attrs.d)`) and `rect` (via `roundedRectPath` with
    `rx ?? ry ?? 0`) only.
  - `PLATE_WIDGET_USER_DATA = { thumbnailCaptureExclude: true, thumbnailCaptureExcludeReason:
    'plate-widget' }` with the comment: "The widgets never appear in an exported thumbnail:
    as DOM they were not in the WebGL frame a thumbnail is rendered from, and becoming meshes
    must not bake them into it."
- **States**: colours re-read on theme change — `usePlateWidgetColors` installs a
  `MutationObserver` on `document.documentElement` with
  `attributeFilter: ['class', 'style', 'data-theme']` and re-reads via
  `getComputedStyle(document.documentElement).getPropertyValue(...).trim() || fallback`.
- **Port target**: `gpui/crates/gpui-app/src/features/scene/plate_widgets/paint.rs`.

## Surface: `PlateNameLabel` — the editable plate name on the plate

- **Source**: `src/components/scene/SceneCanvas/PlateNameLabel.tsx` (470 lines)
- **Renders**: a `group` at `position`/`facingRotation` holding **one textured plane** (either
  the name + pencil, or — while editing — the field box with draft, selection and caret), plus
  an invisible drei `Html` that carries the real `<input>` the keyboard talks to.
- **Layout & styling** — the CSS-pixel constants, quoted:
  ```ts
  const LABEL_FONT = '700 36px Arial';
  const LABEL_LINE_HEIGHT_CSS = 45;
  const LABEL_PADDING_CSS = 4;
  const LABEL_GAP_CSS = 8;
  const PENCIL_BUTTON_CSS = 36;
  const PENCIL_ICON_CSS = 20;
  const PENCIL_RADIUS_CSS = 5.5;
  const LABEL_MAX_TEXT_CSS = 512;
  const TEXTURE_SCALE = 4;
  const EDITOR_PADDING_X_CSS = 8;
  const EDITOR_PADDING_Y_CSS = 4;
  const EDITOR_BORDER_CSS = 1;
  const EDITOR_RADIUS_CSS = 5.5;
  const EDITOR_MIN_CHARS = 6;
  const EDITOR_CARET_CSS = 2;
  const CARET_BLINK_MS = 530;
  ```
  - Display texture: canvas
    `width = ceil((LABEL_PADDING_CSS + textWidthCss + LABEL_GAP_CSS + PENCIL_BUTTON_CSS + LABEL_PADDING_CSS) * TEXTURE_SCALE)`,
    `height = ceil(LABEL_LINE_HEIGHT_CSS * TEXTURE_SCALE)`, then `ctx.scale(4, 4)`. Font is
    `fitFontToWidth(measure, LABEL_FONT, text, LABEL_MAX_TEXT_CSS)`. Text at
    `(4, 45/2 + 2)` with `textAlign: 'left'`, `textBaseline: 'middle'`, colour
    `name ? colors.accent : colors.textMuted`. The pencil chip is a rounded rect at
    `x = 4 + textWidth + 8`, `y = (45 - 36)/2`, inset by `border/2` with `border = 1`,
    filled `colors.surface0` at `globalAlpha = 0.55` and stroked `colors.textMuted` at
    `lineWidth = 1`; the lucide pencil is drawn 20 px wide, centred, in `colors.textMuted`.
    Hover overlay: `globalCompositeOperation = 'source-atop'`, `globalAlpha = 0.15`,
    `fillStyle = '#ffffff'` over the whole label.
  - Editor texture: `boxWidthCss = ceil(max(6, (draft || placeholder).length + 2) * charWidth)`
    where `charWidth` is the width of `'0'` in `LABEL_FONT`;
    `boxHeightCss = 45 + 4*2 = 53`; `textXCss = 1 + 8 = 9`. Box filled with `colors.surface0`
    at `globalAlpha = 0.9`; text font fitted to `textWidthCss - charCss`; selection wash
    `colors.accent` at `globalAlpha = 0.3` from `y = EDITOR_PADDING_Y_CSS` with height
    `LABEL_LINE_HEIGHT_CSS`; text colour `draft ? colors.textStrong : colors.textMuted` at
    `(9, boxHeightCss/2 + 2)`; caret `colors.textStrong`, width 2, height 45, at
    `textXCss + widthTo(selection.start)` when `caretOn && collapsed`; the **border is
    stroked last** in `colors.accent`.
  - Plane: `renderOrder={22}`, `userData={PLATE_WIDGET_USER_DATA}`,
    `planeGeometry args={[label.widthCss * worldPerCssPixel, label.heightCss * worldPerCssPixel]}`,
    `meshBasicMaterial map transparent depthWrite={false} side={THREE.DoubleSide}
    toneMapped={false}`. Position inside the group:
    `[ (w/2) * worldPerCssPixel, (h/2) * worldPerCssPixel, 0 ]`.
  - Hidden input wrapper: `<Html position={position} transform rotation={[0, 0,
    facingRotation]} scale={labelScale} pointerEvents="none" style={{ opacity: 0 }}>`.
- **States**: display (named → accent text; unnamed → placeholder in `textMuted`); hover
  (`useCursor(hovered)` sets the pointer cursor; texture brightens 15 %); editing; each of
  the editor's selection/caret states.
- **Data**: props (`name`, `placeholder`, `editTitle`, `emptyTitle`, `position`,
  `facingRotation = 0`, `labelScale = 5`, `onCommit`) + local `draft` (`null` = not editing),
  `hovered`, `selection {start,end}`, `caretOn`, and three refs (`inputRef`,
  `keepEditingRef`, `tearingDownRef`). Colours from `usePlateWidgetColors()`.
- **Callbacks**:
  - Plane `onPointerOver` → `setHovered(true)`; `onPointerOut` → `setHovered(false)`.
  - Plane `onPointerDown` → `stopPropagation`; while editing, sets `keepEditingRef` and
    re-focuses the input in a `setTimeout(…, 0)` (see gotchas).
  - Plane `onClick` → opens the editor when not already editing (`openEditor()` sets
    `tearingDownRef = false` and `draft = name`).
  - Input `onChange` → `setDraft(value)`, `readSelection()`, `setCaretOn(true)`.
    `onSelect`/`onKeyUp`/`onFocus` → `readSelection()`.
  - Input `onKeyDown` → **always** `stopPropagation()`; `Enter` → `commitDraft()`;
    `Escape` → `tearingDownRef = true; setDraft(null)` (cancel without commit).
  - Input `onBlur` → `commitDraft()` unless `keepEditingRef` or `tearingDownRef`.
  - `commitDraft()` trims, tears down (`tearingDownRef = true; setDraft(null)`) and calls
    `onCommit(next)` **only when `next !== name`**.
- **Text**: `editTitle` and `emptyTitle` reach the input as `aria-label`
  (`aria-label={name ? editTitle : emptyTitle}`); `placeholder` reaches it as the input's
  placeholder and is also what the display texture renders while the name is empty. The
  values come from `SceneCanvas.tsx:1826-1828`: placeholder `Plate 1` (comment: "What the
  build plate's name widget shows while the plate has no name of its own…"),
  `Rename build plate`, `Give this build plate a name`. The widget itself renders **no**
  visible string of its own — every pixel is the caller's text.
- **Port target**: `gpui/crates/gpui-app/src/features/scene/plate_widgets/name_label.rs`.

## Surface: `PlateSideButtons` — the add/lock/arrange column and the bin

- **Source**: `src/components/scene/SceneCanvas/PlateSideButtons.tsx` (326 lines)
- **Renders**: one group at `columnAnchor` with three square buttons stacked in CSS pixels,
  and one group at `clearAnchor` with the bin.
- **Layout & styling**:
  ```ts
  const BUTTON_SIZE_CSS = 104;
  const BUTTON_ICON_CSS = 56;
  const BUTTON_RADIUS_CSS = 5.5;
  const BUTTON_BORDER_CSS = 1;
  const COLUMN_GAP_CSS = 12;
  const TEXTURE_SCALE = 4;
  const DANGER_COLOR = '#ef4444';
  const columnStepCss = BUTTON_SIZE_CSS + COLUMN_GAP_CSS; // 116
  ```
  - Column, in order: `onAdd` (`plusIcon`) at `cssY = 0`; `onToggleLock` (`lockIcon` when
    `locked`, else `lockOpenIcon`) at `116`; `onArrangePlate` (`layoutGridIcon`) at `232`
    and inert when `arrangeDisabled`. The bin (`trash2Icon`) is at `cssY = -104` — quoted:
    "The bin's anchor is its bottom-left corner, so its top-left corner is one button's
    height *above* it in CSS pixels, i.e. forward along the plate."
  - Each button mesh: `renderOrder={22}`, `userData={PLATE_WIDGET_USER_DATA}`,
    `planeGeometry args={[sizeWorld, sizeWorld]}`, `meshBasicMaterial map transparent
    depthWrite={false} side={THREE.DoubleSide} toneMapped={false}`,
    `position={[(cssX + 52) * w, -(cssY + 52) * w, 0]}`.
  - Panel painting (`paintButtonPanel`), quoted logic:
    - `opacity = disabled ? 0.55 : 1` — "`disabledButtonStyle` carries `opacity: 0.55`,
      which rides on every part of it."
    - `red = destructive && hovered && !disabled` — "The bin only turns red while it can act;
      a locked plate's stays quiet and grey."
    - Panel fill: `globalAlpha = opacity * (locked || red ? 1 : 0.45)`, `fillStyle = locked ?
      colors.surface1 : colors.surface0`; then if `locked || red`:
      `globalAlpha = opacity * (locked ? 0.15 : 0.12)`, `fillStyle = locked ? colors.accent :
      DANGER_COLOR`.
    - Border + icon tint: `red ? DANGER_COLOR : locked ? colors.accent : colors.textMuted`,
      at `globalAlpha = opacity * (disabled ? 0.4 : red ? 0.6 : locked ? 0.7 : 0.45)`,
      `lineWidth = 1`.
    - Icon: 56 px centred — `x = y = (104 - 56)/2 = 24` — coloured
      `red ? DANGER_COLOR : locked ? colors.textStrong : colors.textMuted`.
    - Hover lift (`hover:brightness-110`): `source-atop`, `globalAlpha = 0.08`,
      `fillStyle = '#ffffff'` over the panel, **skipped** when `red`.
- **States**: quiet; locked (surface1 + 0.15 accent wash, accent border/icon); destructive
  hover (red wash 0.12, red border, 0.6 alpha); disabled (`!onClick`) — 0.55 panel alpha,
  0.4 border/icon alpha, no hover lift, and the cursor stays default
  (`useCursor(hovered && !disabled)`).
- **Data**: props only; local `hovered` per button.
- **Callbacks**: `onClick` on each plane → `onAdd` / `onToggleLock` / `onArrangePlate` /
  `onClearPlate`; every pointer event `stopPropagation()`s so the plate beneath never sees
  the press (quoted: "The DOM buttons swallowed the pointer so the plate they hang beside
  never saw the press; stopping it here keeps a pick on a button a pick on it alone.").
  An inert button passes `undefined` for `onClick` (`arrangeDisabled ? undefined :
  onArrangePlate`, `clearDisabled ? undefined : onClearPlate`, and `onAdd` directly).
- **Text**: none is rendered. The props `addLabel`, `addComingSoonTitle`, `lockTitle`,
  `unlockTitle`, `arrangeTitle`, `arrangeDisabledTitle`, `clearTitle`, `clearDisabledTitle`
  exist only because `SceneEnvironment` builds them; quoted from the file header:
  > "The wording strings below (`addLabel` and the `*Title`s) were the DOM buttons'
  > `aria-label`s and tooltips. A mesh can carry neither, so they are no longer read; they
  > stay in the contract because `SceneEnvironment` builds its props from this type and the
  > page still translates and passes them."
  Caller values (`SceneCanvas.tsx:1858-1916`): `Add plate`, `Coming Soon!`,
  `Lock build plate`, `Unlock build plate`, `Clear build plate`,
  `Unlock the plate to clear it`, `Auto arrange the plate`, `Unlock the plate to arrange it`.
- **Port target**: `gpui/crates/gpui-app/src/features/scene/plate_widgets/side_buttons.rs`.

### Where the widgets are placed (`SceneEnvironment.tsx:1374-1382`)

```tsx
widgets: {
  scale: plateWidgetScale,
  // The plate's left edge (X0 in the front-left origin mode) and its back edge.
  nameLabelPosition: [resolvedOriginMinX, resolvedOriginMinY + depth, plateLogoZ + 0.2],
  // Two anchors in one component: the column hangs from the plate's back edge, the
  // bin stands on its front edge (the plate's front is its smallest Y).
  columnAnchor: [resolvedOriginMinX + width + buildPlateOversizeEachSideMm + plateWidgetClearanceMm, resolvedOriginMinY + depth, plateLogoZ + 0.2],
  clearAnchor: [resolvedOriginMinX + width + buildPlateOversizeEachSideMm + plateWidgetClearanceMm, resolvedOriginMinY, plateLogoZ + 0.2],
}
```
with `plateWidgetClearanceMm = 1`, `buildPlateOversizeEachSideMm = 3`, `plateLogoZ = 0.12`
(so widget z = 0.32) and:
```ts
// Widget size follows the plate, so a 100mm printer's buttons are not the same
// physical size as a 300mm one's. 5 is what a 200mm plate has always used; the
// clamp keeps a very small plate legible and a very large one from dominating.
const plateWidgetScale = Math.min(14, Math.max(3, width / 40));
```
`PlateNameLabel` receives `labelScale={shared.widgets.scale * 1.5}`; `PlateSideButtons`
receives `labelScale={shared.widgets.scale}`.

Gating (`SceneEnvironment.tsx:1406-1450`): `nameLabelFor` returns undefined unless the
widget strings and the commit callback exist; `sideButtonsFor` returns undefined when
`!allowMultiPlate`, `!shouldShowBuildPlate`, `!showPlateWidgets`, when the plate is not the
active one, or when any of its wording/callback props is missing.

---

## Deliberate decisions and gotchas

1. **`Toast` merges tone last-to-first, so a caller's `style` beats the tone.** The marquee
   toast passes `tone="neutral"` and then overrides `borderColor`/`color`. A port that
   applies tone after style loses the marquee's hue entirely.
2. **The `entered` rAF in `Toast` is not decoration.** Quoted: "A transition needs a starting
   value to move away from, and a toast that is mounted with `visible` already true has
   none… Running that toggle on the next frame instead gives the element one paint in the off
   state first, which is what makes the fade-and-rise happen on the way in." Without it, only
   toasts whose guard and `visible` flag changed in different ticks animate.
3. **Two booleans per toast, not one.** *Mounted* and *visible* are separate; the unmount is
   always later than the fade (`220 ms` for the save toast, `400 ms` for scene import,
   `400 ms` after the history fade, …). Collapsing them makes toasts blink away — that is the
   exact bug `useTrailingMount` was written for.
4. **`NotificationStack` toasts overlap by design and are separated by bumping `offset`.** The
   printing-monitor toast moves to `'4.5rem'` when any of history/import/orientation is
   present; the orientation toast moves to `'4.5rem'` when history or import is present. The
   conditions are **not** symmetric — orientation ignores the printing-monitor toast, and
   export toasts are ignored by everyone. Copy the conditions verbatim.
5. **Toast 4 maps `warning` to `AlertTriangle`, not a warning glyph.** `error` and `warning`
   share the icon; `success` gets `CheckCircle2`. The tone is still `warning`, so the border
   is amber.
6. **The scene-import toast is the only clickable one in `NotificationStack`.** `role`,
   `tabIndex`, `pointer-events-auto`, `cursor-pointer` and `select-none` are all conditional
   on `clickAction === 'openMeshRepairReport'`; without the class the toast is inside a
   `pointer-events-none` viewport and unclickable.
7. **`EmptySceneState` picks its tagline once, at mount, with `Math.random()`.** `useState(() =>
   TAGLINES[…])` — not a `useMemo`, and `suppressHydrationWarning` is on the `<h1>` because
   the pick differs between server and client render. A port must pick once per empty-state
   mount and keep it through re-renders.
8. **`isLikelyFileDrag` returns `true` for *any* `DataTransfer`.** The final line is a bare
   `return true`, with the comment "Some desktop runtimes (including certain Tauri/webview
   combos) expose file payloads late in the drag lifecycle; allow the drag to proceed
   optimistically." Do not "fix" this into `false`; the optimistic branch is what makes
   external drags show the highlight at all.
9. **The drop zone clears itself on a 220 ms timer, refreshed on every drag event.** Quoted:
   "Some drag-exit paths in desktop webviews can miss `dragleave`. Keep this short timeout
   refreshed while dragging over the zone so stale highlights clear quickly." A rejected drop
   gets its own 1600 ms timer. Window `drop`, `dragend`, `blur`, and
   `document.visibilitychange` (when not visible) also clear it.
10. **`handleDragOver` sets `dropEffect` per support state.** `unsupported` → `'none'`
    (and `isDropUnsupported`), otherwise `'copy'`. `dragenter` never sets `dropEffect` and
    only *sets* `isDropUnsupported` for `supported`/`unsupported` — `unknown` leaves the
    previous value alone.
11. **Support detection reads names out of three places**: `dataTransfer.files`,
    `item.getAsFile()`, and `webkitGetAsEntry().name`, plus the first non-empty line of
    `text/uri-list` and `text/plain` (with `file://` decoding and both separators normalised).
    `isSupportedDropName` accepts `.stl/.obj/.3mf` (`MESH_DROP_EXTENSIONS`), any registered
    scene extension, and `isSideloadedFileName(name)`. Zero names found → `'unknown'`, and
    `'unknown'` is treated as acceptable (not unsupported).
12. **`formatRecentOpenedAt` returns `null` for "just now"** (under 60 s, or a non-finite or
    negative delta) and the caller renders `last opened just now`; otherwise it wraps the
    period in `last opened ${period} ago`. Periods are `msg` ids so translators control the
    space between number and unit — the source says so explicitly.
13. **`formatBytes` rounds asymmetrically**: `value >= 10 ? Math.round(value) : Math.round(value
    * 10) / 10`, after successive `/1024` divisions, starting from `bytes / 1024` (so bytes
    under 1024 print with unit `B` and 0 fractional digits). A missing/negative/non-finite
    size returns `null`, and the caller prints `cached`.
14. **`Recent files` are reversed before slicing.** `recentOpenedFiles.slice().reverse().slice(0,
    8)` — the *last* array element is displayed first, so the array is oldest-first in the
    store. The count badge shows `Math.min(length, 8)`.
15. **The scene-import button exists only when `onImportSceneChange` is set**, and the grid
    switches `grid-cols-2`/`grid-cols-1` on the same condition. `onImportSceneClick` alone is
    not enough.
16. **`triggerMeshPicker`/`triggerScenePicker` fall back to `document.getElementById`.** When
    the `…Click` prop is absent they click the hidden input by id; that is the only path that
    makes the inputs work at all, so a port must keep the fallback or require the click prop.
17. **`LayerSlider`'s `railClassName` is applied twice** — as `Tooltip.wrapperClassName` and
    on the `div`. The tooltip wrapper therefore inherits `cursor-pointer`, the width and the
    height, which is what makes the hover target the full rail rather than a text node.
18. **A bare track click is deliberately *refused*.** Without `allowTrackClickJump`, a press
    more than `grabThreshold` from a thumb calls `setIsDraggingThumb(false)`,
    `dragShiftModeRef = false`, `setIsShiftHeld(false)`, `onScrubEnd?.()` and returns — and
    `onScrubStart` was already fired before the check, so the pair is balanced. Inside the
    window between two thumbs (but outside both cores) the press starts a **window drag**
    that moves both thumbs by the same delta.
19. **Thumb hit thresholds are normalised to the rail height**, not absolute px:
    ```ts
    const normalizedPerPx = 1 / Math.max(1, railHeightPx);
    const coreThreshold = (9 / 2) * normalizedPerPx;
    const grabThreshold = ((9 / 2) + 1.5) * normalizedPerPx;
    ```
    with the comment "Visual thumb height is 9px. Keep the true handle core tight, with only a
    small comfort padding for easier grabbing without stealing the window area." Port the
    ratio, not a pixel constant.
20. **Shift is latched for the whole drag.** `dragShiftModeRef` is seeded from the press and
    then only ever set to `true` during the drag (`if (ev.shiftKey && !dragShiftModeRef.current)`);
    releasing shift mid-drag does **not** restore full speed. Precision is `delta * 0.1` —
    a 10× reduction, applied to the *distance from the current value*, not to the pointer
    delta.
21. **The wheel direction is inverted** — `const dir: 1 | -1 = e.deltaY > 0 ? -1 : 1` — with
    the comment "Inverted direction: wheel down moves to lower layers, wheel up moves to
    higher layers." Shift makes the step `(step || 1) * 0.1`. The listener is attached
    natively with `{ passive: false }` so `preventDefault` works; React's synthetic wheel
    could not.
22. **Only jumps glide.** Quoted:
    > "Only a jump glides. A wheel or arrow step lands where it is: eased, every step of a
    > scroll restarts the glide, and the thumb trails the layer the viewport already shows."
    The test is `Math.abs(value - glide.value) > 2 * (step || 1)`, computed *during render*
    with a `setGlide` side effect (a legal React pattern here, not a mistake), and
    `easesPosition = glide.eased && !isDraggingThumb`. `LayerSlider.dom-test.tsx` asserts both
    halves: a wheel step leaves `style.top === '49%'` with `transition: 'none'`, and a
    programmatic jump to 10 leaves `transition` matching `/^top 170ms/`.
23. **A live scrub draws the slider's own value, not the owner's.** Quoted: "While a scrub is
    live the owner may defer its own state (the scene publishes the clip imperatively
    instead), so the slider draws the value it just sent." Hence `liveValue`/`liveLowerValue`
    are set only while `scrubActiveRef.current`, and cleared on `settleDrag`. Heights are
    recomputed from the live value (`displayValue * layerHeightMm` clamped) only in that
    state; otherwise the owner's `currentHeightMm` is authoritative.
24. **`emitChange` refuses no-op writes**: it returns early when
    `next === valueRef.current`, and it also clamps up to `lowerValueRef.current`. The lower
    emitter is the mirror (`Math.min(…, valueRef.current)`), so the two thumbs can meet but
    never cross.
25. **The edit popover side is chosen from a viewport probe**, not from the component's own
    space: `clampToViewport({ x: rect.right + 10, y: rect.top }, { width: THUMB_EDIT_POPOVER_WIDTH, height: 0 })`
    and `setEditPopoverSide(overflowRight ? 'left' : 'right')`. `THUMB_EDIT_POPOVER_WIDTH =
    156` is documented as "min-width 136px + 10px offset + margin". Quoted rationale: "Open
    the popover towards whichever side of the rail actually has room — sliders docked at the
    right screen edge would otherwise clip it off-screen."
26. **Right-click near a thumb opens the editor; right-click anywhere else is only
    suppressed.** `onPointerDown` handles `button === 2` (opens the popover when within
    `grabThreshold` of the nearer thumb, with `preventDefault` + `stopPropagation`), and the
    container's `onContextMenu` always does `preventDefault` + `stopPropagation`. Left press is
    the only drag; `button !== 0` returns.
27. **Double-click toggles the cross-section, and only when `onToggleCrossSection` exists.**
    `e.preventDefault()` + `e.stopPropagation()` first. It is the second half of the
    minimal-rail tooltip contract, and it is *not* an alternative to the drag — a fast
    drag-and-release can produce one.
28. **`commitThumbEdit` accepts only `>= 0` and non-NaN**, parses through `fmt.parse`
    (locale-aware), and converts `mm → layer` only when `editMode === 'mm' && lhMm > 0`,
    rounding with `Math.round(parsed / lhMm)`. `switchEditMode` refuses to switch when
    `layerHeightMm` is missing or `<= 0` — and the mode buttons do not render at all in that
    case.
29. **The value field forces `max` and shows an error for 1000 ms.** Typing a number above
    `max` sets the field to `max`, emits `max`, and sets `showError`; the error style is
    removed by a timeout that is cleared and re-created on every keystroke. Blur restores
    `String(Math.round(value))` when the field is empty or non-numeric. `Enter` only blurs.
30. **The caret blink is 530 ms and the caret hides on a non-collapsed selection** — the
    browser's own behaviour, reproduced by hand because the field is painted, not a real
    focused input. A fresh keystroke (`onChange`) resets `caretOn` to `true`.
31. **`ZUpGizmoHelper` ignores drei's `ratio` prop.** Only `alignment`, `margin`,
    `renderPriority`, `onHome`, `onUpdate`, `onTarget` and children are destructured; the
    cube's size is fixed by `scale={[60,60,60]}` and the HUD camera at
    `position={[0, 0, 200]}`. Porting `ratio` would change the widget's pixel size.
32. **Two source comments contradict the code; the code wins.** `SceneCanvas.tsx:382` says
    "Bottom margin is 72px to cube center" while the call passes `82`; and
    `ZUpGizmoHelper.tsx`'s `HOME_OFFSET` comment computes the corner sweep as
    "30 × 1.73 ≈ 52px" for a chip at `0.93` — the numbers are the authored ones, both are
    deliberate tuning, neither matches a naive re-derivation.
33. **`stabilizeDirection` exists because a pole is a singularity for `OrbitControls`.**
    Quoted: "Tilt such a direction a fraction of a degree off the pole toward the side that
    puts `polarScreenUp` up on screen, so the whole tween (and the state it lands in) is
    well-defined. Visually it is still a top/bottom view." `polarTilt = 0.0087` (≈0.5°),
    `polarScreenUp = (0, 1, 0)`, and the sign is `upDot > 0 ? -1 : 1`. Removing it makes
    TOP/BOTTOM snaps roll instead of animate.
34. **Controls are disabled for the tween and restored on completion *and* on unmount.**
    Quoted: "Never leave OrbitControls disabled if this unmounts mid-tween (thumbnail capture
    does exactly that)." `savedControlsEnabled` is a one-shot latch (`=== null` check).
35. **The quarter-turn arrows are outside the rotating group**, so they stay screen-aligned:
    "the top arrow is always 'turn up from here'. They fade out as the view leaves a face — a
    quarter turn is meaningless from an arbitrary angle. Fully faded frames render nothing
    (not `visible`), so the faded arrows cannot be hit." The fade is a smoothstep
    (`rawFade² · (3 − 2·rawFade)`) with hard snaps at 0.02 and 0.98.
36. **`quarterTurn` rotates the *offset*, not the camera**, about the camera's own world-axis
    columns 0 and 1: `up` → `applyAxisAngle(right, -π/2)`, `down` → `+π/2`, `right` →
    `applyAxisAngle(up, +π/2)`, `left` → `-π/2`. Signs are easy to invert; they are correct as
    written.
37. **The viewcube's face order and per-face rotations are coupled to the Z-up box.** Quoted:
    "In Y-up Drei the top/bottom faces (+Y/-Y) are indices 4/5 and need no rotation. In Z-up
    the top/bottom faces are +Z/-Z which map to different box face indices, so the rotation
    values below have been adjusted accordingly." Rotations:
    `[-π/2, π/2, π, 0, -π/2, -π/2]`. Changing `GIZMO_Z_ROTATION` without changing this array
    scrambles the labels.
38. **The label font is centred on the em box, then nudged +2 px.** Quoted: "`middle` centres
    the em box, and capitals sit above its middle, so dead-centre reads high — the same nudge,
    scaled to this font, that the plate's FRONT marker uses." Same nudge in the editor text.
39. **Every label font is shrunk to fit, never wrapped.** `fitFontToWidth` computes
    `max(8, floor(size * maxWidth / width))` and rewrites the px size in the shorthand. This is
    what keeps `IZQUIERDA` inside a 112 px face and a long plate name inside 512 px. It
    **replaces the size in the font string**, so the port needs a font-size-scaled text pass,
    not a clipped/ellipsised one.
40. **Edge/corner pads are invisible until hovered** (`visible={hover}`, `opacity={0.6}`) and
    are scaled `1.01` so they sit a hair proud of the cube — both are needed for them to
    answer picks at all.
41. **Plate widget planes are in the depth buffer on purpose.** Quoted from
    `plateWidgetDraw.ts`: "DOM always paints over the WebGL canvas, so a widget written in DOM
    drew on top of the models that should hide it, however far behind it they stood. A plane
    is in the depth buffer like the rest of the plate's decals." Also `renderOrder={22}` and
    `depthWrite={false}` — the first sorts the widgets, the second stops them writing depth
    they would then z-fight with.
42. **`PLATE_WIDGET_USER_DATA` must survive the port**, with
    `thumbnailCaptureExcludeReason: 'plate-widget'`: the export thumbnail renderer excludes
    anything carrying `thumbnailCaptureExclude`.
43. **The plate name plane and the side buttons grow in opposite Y directions.** The name
    plane is centred at `+ (h/2)·world` in Y (its anchor is the bottom-left corner and text
    reads toward +Y); the buttons are centred at `-(cssY + 52)·world`, i.e. CSS `y` runs
    *backwards* along the plate's plane ("CSS pixels run down the canvas, which the plate's
    plane runs backwards in"). Copying the sign from one widget to the other mirrors a column
    off the plate.
44. **The plate-name editor is a real `<input>` that is never drawn.** Quoted: "A real
    `<input>` is still the thing the keyboard talks to; it is simply kept out of sight, and the
    plane draws what it holds." The `Html` wrapper is `pointerEvents="none"` and `opacity: 0`
    so the plane decides what a press means.
45. **`keepEditingRef` + `setTimeout(0)` exists because the canvas steals focus.** Quoted:
    "The press goes to the canvas, which takes the focus the editor's input needs. Hand it
    back once the browser has finished moving it — the blur in between is this press, not the
    field being left, so it commits nothing." Two separate refs guard the two distinct blur
    cases (`keepEditingRef` for the press, `tearingDownRef` for unmount/Escape).
46. **The input stops propagation on `keydown` unconditionally** — "the app's single-letter
    hotkeys must not fire while a name is being typed."
47. **The editor's border is stroked last so the selection wash and caret sit inside it**, and
    every box is stroked on its own centre inset by half a border ("as a CSS border box
    does"). Getting this wrong shifts every panel by a pixel at 4× texture scale.
48. **`PlateSideButtons`' wording props are dead weight that must stay in the type.** Quoted
    above; `SceneEnvironment` builds them and `page.tsx` still translates them. Deleting them
    breaks the call site's type, not just an unused variable.
49. **`usePlateWidgetColors` observes `class`, `style` and `data-theme`** — the app's theme
    switch sets a class, not a data attribute, on some paths. Missing any of the three leaves
    a light-theme scene painting dark tokens onto its plates.
50. **The plate widget scale is `clamp(3, 14, width / 40)`** — a physical size derived from the
    plate, not a screen size. The name label gets `1.5×` the buttons' scale, which is a
    separate multiplier at the call site, not a constant inside the widget.
51. **All plate widget text arrives pre-translated.** Quoted: "This runs inside the r3f
    reconciler, where the i18n provider is out of scope, so every string arrives already
    translated." Same rule for the view cube's `faces`. A port must resolve the strings in the
    outer view and pass them down, never look up a catalog inside the 3D module.
52. **`SceneCanvas` hides both the gizmo and the cube during thumbnail capture**
    (`!thumbnailCaptureActive && cameraInteractionCycleEnabled`), and the home reset comes
    from `resetCameraHome` — a camera-controller concern, not a widget one.

---

## Open questions

1. **Port targets are proposals.** This worktree has no `gpui/` directory (`gpui` is absent,
   `docs/internal/gpui-port-handoff.md` is absent), so the module paths in each *Port target*
   follow the one convention that does exist — `gpui/crates/gpui-backend/…` from
   `docs/internal/gpui-backend-map.md` — and nothing can be confirmed to "already exist
   there".
2. **GPUI has no DOM, so five mechanisms need a decided replacement**:
   (a) the `MutationObserver` theme watchers in `usePlateWidgetColors` and
   `SceneCanvas`'s gizmo-colour effect; (b) the per-frame canvas texture rebuilds in
   `PlateNameLabel` (a new `CanvasTexture` on every keystroke, blink tick and hover change);
   (c) `getComputedStyle`-based `color-mix` resolution, which the marquee toast and every
   tone table depend on; (d) `clampToViewport` for the popover side; (e) the hidden `<input>`
   that the native keyboard talks to. None of these has a source answer.
3. **`document.visibilityState`, `window.blur` and the `webkitGetAsEntry` drag probes** are
   WebView specifics. What the native drag-and-drop API reports for external files before
   `drop` is unknown, so `isLikelyFileDrag`'s "optimistically true" fallback may have no
   analogue.
4. **`fmt.parse` / `fmt.field` / `fmt.unit` / `fmt.number`** come from `useLocaleFormat`, whose
   catalogue is `src/locales/*.po`. The slider's `mm` field accepts locale decimal separators;
   whether the Rust i18n layer exposes an equivalent parse is outside this area.
5. **The recent-files list, the slicing layer state and the plate list** (`RecentOpenedFileEntry`,
   `slicing.layerIndex`/`numLayers`/`layerHeightMm`, `scene.plates`) belong to other specs.
   This document only fixes the shapes these components consume.
6. **`showFirstTimeOnboarding` is computed as `!hasActivePrinterProfile &&
   !allowPrepareWithoutPrinter`** (`page.tsx:11048`); the store that holds those two is not in
   this scope, and the onboarding card's buttons call page-level handlers that are.
7. **`Toast`'s `pulse` and the history toast's `motion-safe:animate-pulse` icons** have no
   stated reduced-motion contract beyond the Tailwind `motion-safe:` prefix — the port needs
   a decision on honouring the OS reduce-motion setting for both.
