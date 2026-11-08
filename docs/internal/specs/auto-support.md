# Auto Support

The Auto Supports panel (`Auto Supports (Beta)`), its settings dialog, its preset
strip and footer, the panel's preset row + Auto-Lift toggle, the run diagnostics
and the three `StructuredDialogModal`s the panel owns. Plus the preset store
(`src/supports/Settings/autoSupportPresets.ts`) that all of it reads and writes.

Note on paths: the ticket says `src/components/autoSupport/`; the code actually
lives in **`src/components/controls/autoSupport/`** with the panel itself at
`src/components/controls/AutoSupportPanel.tsx`. Everything else in this document
uses the real paths.

## Components in scope

| Component | File |
|---|---|
| `AutoSupportPanel` (panel card, run button, counts, forest report, settings portal, discard + replace dialogs) | `src/components/controls/AutoSupportPanel.tsx` |
| `AutoSupportSettingsBody` (dialog body: cards, grid, footer mount) | `src/components/controls/autoSupport/AutoSupportSettingsBody.tsx` |
| `AutoSupportPresetSelector` (dialog strip), `AutoSupportSettingsFooterActions` (dialog footer), `selectAutoSupportPreset`, `deleteActiveAutoSupportPreset`, `useAutoSupportDialogChanges` | `src/components/controls/autoSupport/AutoSupportPresets.tsx` |
| `AutoSupportPresetSelect` + `autoSupportPresetOptions` (panel dropdown) | `src/components/controls/autoSupport/AutoSupportPresetSelect.tsx` |
| `AutoSupportPresetRow` (dropdown + Auto-Lift toggle) | `src/components/controls/autoSupport/AutoSupportPresetRow.tsx` |
| `AutoSupportRunDiagnostics` (Sizing Debug disclosure, forest report button) | `src/components/controls/autoSupport/AutoSupportRunDiagnostics.tsx` |
| `AutoSupportSizingTierField`, `useSupportStudioPresets`, `sizingTierName` | `src/components/controls/autoSupport/AutoSupportSizingTierField.tsx` |
| Catalogue: sections, knobs, toggles, hints, tier hints, locked hint | `src/components/controls/autoSupport/autoSupportPanelTabs.ts` |
| Preset store (built-ins, storage, CRUD, commit, import/export) | `src/supports/Settings/autoSupportPresets.ts` |
| Built-in name translation, `NO_ACTIVE_PRESET_LABEL`, copy-name | `src/supports/Settings/autoSupportPresetMessages.ts` |
| Settings block type, defaults, constraints, normalizer, diagnostics keys | `src/supports/autoSupport/settings.ts` |
| Tests that encode the contract | `src/supports/__tests__/autoSupportPanel.test.ts`, `src/supports/__tests__/autoSupportPresets.test.ts` |

Referenced but out of scope: `src/components/atoms/{Button,Card,IconButton,PanelCollapseToggle}.tsx`,
`src/components/ui/{StructuredDialogModal,SelectDropdown,Tooltip}.tsx`,
`src/components/settings/profileFormAtoms.tsx` (`LabeledNumberInput`,
`LabeledToggleInput`, `FieldHelpTooltip`), `src/components/layout/FloatingPanelStack.tsx`
(`useFloatingPanelCollapse`), `src/hooks/useLocaleFormat.ts`,
`src/hooks/useOutsideDismiss.ts`, `src/hotkeys/useEscapeToClose.ts`,
`src/volumeAnalysis/Islands/useIslands.ts`, `src/supports/autoSupport/{autoPlace,
autoPlaceWorkerClient,types}.ts`, `src/supports/Settings/{state,presets}.ts`,
`src/supports/state.ts`, `src/supports/history/supportEditHistory.ts`,
`src/supports/history/supportHistoryLabels.ts`, `src/utils/unitFormatting.ts`,
`src/features/slicing/tauri/nativeSlicerBridge.ts`, `src/utils/tauriRuntime.ts`.

---

## Surface: module-level run state — busy flag, progress, scan-driving flag

- **Source**: `src/components/controls/AutoSupportPanel.tsx:32-70`
- **Renders**: nothing. Two tiny observable stores plus one plain mutable flag.
- **Layout & styling**: n/a.
- **States**:
  - `_autoSupportBusy: boolean`, module-scope `false`. `setAutoSupportBusy(v)` only
    acts when `v` changes, and **on going false it also clears progress**:
    `setAutoSupportProgress(null)` — comment: `// The progress belongs to the run, so it goes when the run does.`
  - `_autoSupportProgress: { phase: string; done: number; total: number } | null`.
  - `autoSupportDrivingScan: boolean` — exported as a mutable binding
    (`export let`), **not** subscribable. Comment: `Set to true while auto-support is driving its own scan, so the native island-scan overlay can be suppressed.`
- **Data**: module globals; `Set<() => void>` listener sets per store.
- **Callbacks**: none. Exports `getAutoSupportBusy`, `subscribeAutoSupportBusy`,
  `getAutoSupportProgress`, `subscribeAutoSupportProgress`, `setAutoSupportProgress`
  (public), `autoSupportDrivingScan`.
- **Text**: none.
- **Port target**: `gpui/src/features/auto_support_settings.rs` — the module already
  holds the dialog's process-wide `LazyLock<Mutex<DialogState>>`; busy/progress belong
  beside it (or on `DragonFruit`). `gpui/src/panels/content.rs::supports_body` does not
  read either today.

## Surface: `AutoSupportPanel` — the floating panel card

- **Source**: `src/components/controls/AutoSupportPanel.tsx:104-130` (props/state),
  `:297-384` (card)
- **Renders**: a `Card` whose header is `[chevron toggle] "Auto Supports (Beta)"` with
  a gear on the right; while expanded, a `px-2.5 pb-3 space-y-2.5` column holding
  the run button, the island-count card, the preset row, the optional run
  diagnostics, and the "load a model" hint.
- **Layout & styling**:
  - `Card` = `div` with `cn('ui-panel rounded-lg shadow-lg border')` and inline
    `borderColor: 'var(--border-subtle)'` (`src/components/atoms/Card.tsx`).
  - `CardHeader` = `div` `data-panel-drag-handle="true"`,
    `relative px-2.5 py-2.5 flex items-center`; left group `flex items-center gap-2.5 pr-8`;
    right group absolutely placed `absolute right-2.5 top-1/2 -translate-y-1/2 flex items-center`.
  - Title: `h3` `text-sm font-semibold`, `style={{ color: 'var(--text-strong)' }}`.
  - Gear: `IconButton onClick={openSettings} className="!p-1.5" title={…}` with
    `<Settings className="h-3.5 w-3.5" style={{ color: 'var(--text-muted)' }} />`.
    `IconButton` defaults are `variant="solid" size="md"` → `ui-button
    ui-button-secondary !p-2`, so `!p-1.5` is the only override.
  - Body container: `px-2.5 pb-3 space-y-2.5` (10px sides/bottom, 10px vertical gaps).
  - Run button: `Button variant="secondary" size="auto"`,
    `className="w-full !h-8 text-[11px] disabled:opacity-50"`,
    `style={{ borderColor: 'var(--accent)', background: 'color-mix(in srgb, var(--accent), var(--surface-0) 86%)', color: 'var(--accent)' }}`.
    (CSS `color-mix(A, B 86%)` = `A*0.14 + B*0.86`.)
  - Counts card: `div` `rounded-md border p-2` with
    `style={AUTO_SUPPORT_SECTION_CARD}` = `{ borderColor: 'var(--border-subtle)', background: 'var(--surface-1)' }`
    (`autoSupportPanelTabs.ts:23`), inner `grid grid-cols-3 gap-2 text-center`.
    Per stat: label `text-[10px] font-semibold uppercase tracking-wide`
    (`var(--text-muted)`), value `text-sm font-bold` with
    `color: s.id === 'total' ? 'var(--accent)' : 'var(--text-strong)'`.
  - Hint: `text-[10px] italic text-center`, `var(--text-muted)`.
- **States**:
  - Collapsed/expanded via `useFloatingPanelCollapse(true)` — default expanded; the
    body is inside `{expanded && …}`.
  - Run button label: `busy ? "Running…" : "Generate Supports"`.
  - Run button disabled when `!canRun`, `canRun = hasGeometry && !!activeModelId && !busy && !islands.scanning` (`:295`).
  - Counts render `'…'` while `islands.scanning`, else the length.
  - `!hasGeometry` → the hint `Load a model and scan for islands.` renders below the
    preset row. Note it is `!hasGeometry`, **not** empty-counts.
  - `debugMode` (panel session state, default `false`) → renders
    `AutoSupportRunDiagnostics`.
- **Data**: reads
  - props `islands: UseIslandsReturn`, `hasGeometry`, `activeModelId?`, `autoLift`,
    `onAutoLiftChange`, `onBeforeRun?`, `getCavityGeometry?`;
  - from `@/supports/Settings/state`: `getSettings`, `subscribeToSettings`;
  - from the preset store: `subscribeToAutoSupportPresets`,
    `getAutoSupportPresetsSnapshot`, `getAutoSupportPresetsServerSnapshot`,
    `getActiveAutoSupportPresetId` (each through its own `useSyncExternalStore`);
  - panel state: `expanded`, `busy`, `showSettings`, `showDiscardSettingsDialog`,
    `showReplaceDialog`, `debugMode`, `sizingDebug`, `showForestReport`,
    `forestReport`, `draft` (only consumed by the settings dialog).
  - Mutates: `draft` on preset select / on open; the store on run (via
    `runAutoPlaceInWorker` → `commitAutoPlacePlan`), support entities on Replace;
    `sizingDebug`/`forestReport` from `result.analytics`.
- **Callbacks**:
  - Collapse chevron → `setExpanded(!expanded)`; expanding calls
    `stack.requestPanelExpand(panelId)` (see `useFloatingPanelCollapse`).
  - Gear → `openSettings()`: `setDraft(getSettings().autoSupport); setShowSettings(true)`.
  - Run → `void handleRun()` (`:270-294`), see below.
  - Preset select → `selectAutoSupportPreset(id, setDraft)`.
  - Auto-Lift → `onAutoLiftChange(!autoLift)` (the app's one lift flag, shared with
    Prepare → Transform → Lift).
- **Text** (verbatim msgids): `Auto Supports (Beta)` · `Auto-support settings` (gear
  title) · `Scan for islands if needed, then place automatic supports on this model`
  (run title) · `Generate Supports` · `Running…` · `Voxel` · `Minima` · `Total` ·
  `Load a model and scan for islands.` · `Collapse card` / `Expand card`
  (`PanelCollapseToggle`).
- **Port target**: `gpui/src/panels/content.rs::supports_body` (exists). It already
  renders the run button, the three counts, the preset row and the hint; the file's
  own header lists the gaps: no busy flag (`Running…`), no `hasGeometry`/model gating,
  no island scanner (Voxel/Minima hardcoded `0`, Total = generated count), no
  diagnostics, no debug mode. The header gear lives in
  `gpui/src/features/tool_panels.rs::supports_settings_gear` chained in `app.rs:1923`
  (source has it inside the card header — same result, different owner).

### `handleRun` / `doRun` — the run state machine (read this literally)

- **Source**: `:152-268`
- `handleRun`:
  1. `if (!activeModelId || busy) return;`
  2. `if (onBeforeRun && !(await onBeforeRun())) return;` — resolves unapplied
     hollowing/hole punches *before* generating; a `false` **abandons the run**.
  3. Existing supports? Scans `getSnapshot().trunks` then `.branches` for any entry
     whose `modelId === activeModelId`. Any hit → `setShowReplaceDialog(true)` and
     **stop** (no run yet).
  4. Otherwise `doRun(false)`.
- `doRun(true)` (Replace): capture `before = captureSupportEditSnapshot()`; call
  `removeSupportEntitiesForModel(activeModelId)`; **only if the return value > 0** push
  one history entry `pushSupportEditHistory(REPLACE_SUPPORTS_LABEL, before, captureSupportEditSnapshot())`
  (`REPLACE_SUPPORTS_LABEL` = `msg({ message: 'Replace supports', context: 'Undo history entry' })`).
  Then `requestAnimationFrame(() => { setBusy(true); setAutoSupportBusy(true);
  requestAnimationFrame(() => setTimeout(() => { … }, 0)); })` — the double rAF exists
  so React flushes the removal and `filteredIslands` is fresh.
- `doRun(false)` (Add): `setBusy(true); setAutoSupportBusy(true);` immediately, then:
  - If `filteredIslands.length === 0 && voxelIslands.length === 0 && minimaIslands.length === 0`
    → `pendingRef.current = true; autoSupportDrivingScan = true; void islands.onRunScan(); return;`
  - Else one rAF + `setTimeout(0)`, then if `list.length > 0 && getSettings().autoSupport.enabled`
    → `runAutoSupports(list).finally(clear busy)`; else clear busy immediately.
- Deferred run effect (`:176-193`), deps `[islands.scanning, islands.filteredIslands, activeModelId, runAutoSupports]`:
  if `pendingRef.current && !islands.scanning` → clear pending, `autoSupportDrivingScan = false`,
  then run (or clear busy when the list is empty or `enabled` is off).
  `islands.filteredIslands` is in the deps on purpose: after a scan it is a new array.
- `runAutoSupports` (`:158-174`): `runAutoPlaceInWorker(list, activeModelId, getSettings().autoSupport, setAutoSupportProgress, getCavityGeometry?.(activeModelId) ?? null)`;
  on success copies `result.analytics?.sizingDebug` and `.forestReport` into panel state;
  errors are `console.error('[AutoSupport] runAutoPlace failed:', e)` only — **no toast,
  no dialog**.
- **Gotcha**: the run reads `getSettings().autoSupport.enabled` twice, live, never the
  draft — an unsaved dialog edit cannot affect a run.

## Surface: `AutoSupportPresetRow` — dropdown + Auto-Lift, one row

- **Source**: `src/components/controls/autoSupport/AutoSupportPresetRow.tsx:71-130`
- **Renders**: `div.flex.gap-1.5` with the preset select in `min-w-0 flex-1` and the
  Auto-Lift `Button` in `min-w-0 flex-1`.
- **Layout & styling**:
  - Row: `flex gap-1.5` (6px).
  - Toggle `Button variant="secondary" size="auto"`, `className="min-w-0 flex-1 !h-8 px-2 text-[11px]"`,
    `aria-pressed={autoLift}`, label wrapped in `span.truncate`.
  - `ON_STYLE` (`:44`): `{ borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 30%)',
    background: 'color-mix(in srgb, var(--accent), var(--surface-1) 85%)', color: 'var(--text-strong)' }`.
  - `OFF_STYLE` (`:54`): `{ background: 'var(--surface-1)' }` — border/text stay the
    secondary variant's.
- **States**: on/off only; the label states it (`Auto-Lift ON` / `Auto-Lift OFF`).
- **Data**: props only — `presets`, `activeId`, `onSelect`, `activeHint?`, `autoLift`,
  `onAutoLiftChange`. Store-free by construction.
- **Callbacks**: `onAutoLiftChange(!autoLift)`; select behaviour is in
  `AutoSupportPresetSelect`.
- **Text**: `Auto-Lift ON`, `Auto-Lift OFF`, title
  `Lift the selected models to this distance so a support can stand under them; off returns them to the height they were at.`
- **Port target**: `gpui/src/panels/content.rs::supports_body` (present). It builds the
  row by hand: `preset_select_trigger` + a `ButtonSpec` for the lift, with
  `mix(ACCENT, SURFACE_1, 0.15)` / `mix(ACCENT, BORDER_SUBTLE, 0.70)` for ON (CSS
  direction honoured) and `SURFACE_1` hover for OFF. Flag storage:
  `auto_support_settings::{auto_lift, set_auto_lift}` (PORT DEVIATION 13 in that
  module's header); the Transform panel's own copy is a known duplicate (handoff
  "Known open items").

## Surface: `AutoSupportPresetSelect` + `autoSupportPresetOptions`

- **Source**: `src/components/controls/autoSupport/AutoSupportPresetSelect.tsx:49-100`
- **Renders**: one `SelectDropdown` — no label, no adornment.
- **Layout & styling**:
  `className="space-y-0"`, `selectClassName="w-full !h-8 leading-tight text-[12px]"`,
  `selectStyle={{ background: 'var(--surface-1)', ...CENTERED_SELECT_PADDING }}`,
  `menuClassName="max-w-[26rem]"`. `CENTERED_SELECT_PADDING` =
  `{ paddingLeft: 0, paddingRight: 26, textAlign: 'center' }` — the 26px is the
  chevron's `right-3` (12px) + 14px icon, so the label centres in the space left of
  the chevron.
- **States**:
  - Option rows: `label` = `translateAutoSupportPresetName(preset, _)`;
    `rightContent` = `Built-in` or `Custom`; `value` = preset id.
  - `activeId === null` → a **disabled** first row `{ value: '', label: NO_ACTIVE_PRESET_LABEL,
    disabled: true, rightContent: '' }` prepended, because the trigger renders the
    selected option's label and would otherwise be empty.
  - `activeHint` present (built-ins only) → trigger `title` = that hint; otherwise
    `Apply this preset to the auto-support settings`.
  - Open state: `SelectDropdown` tints the selected row
    (`color-mix(in srgb, var(--accent), var(--surface-0) 86%)`) and rotates the
    chevron; the menu is a portal at `fixed z-[9999]`, `rounded-[4px]`,
    `background: color-mix(in srgb, var(--surface-0), black 6%)`, `backdrop-filter: blur(6px) saturate(112%)`,
    rows `px-3 py-2 text-sm`, `rightContent` at `ml-auto` `text-[10px]` muted.
- **Data**: props `presets`, `activeId`, `onSelect`, `activeHint?`.
- **Callbacks**: `onChange(id) → onSelect(id)`. The panel wires it to
  `selectAutoSupportPreset(id, setDraft)`.
- **Text**: `Auto-support preset` (aria) · `Apply this preset to the auto-support settings` ·
  `Built-in` · `Custom` · `Custom — not a preset` (`NO_ACTIVE_PRESET_LABEL`, comment:
  `Shown in a half-width preset dropdown when no preset is selected. Keep it short; "no preset" alone is enough.`).
- **Port target**: `gpui/src/panels/content.rs` (`preset_select_trigger` + the
  `context_menu_sized` list). The port currently marks the selected row with a tick
  instead of the source's tint, and takes no `!h-8` (36px trigger). It does not show
  the `activeHint` per-row generic split exactly as the source (it puts the hint in a
  `hint()` wrapper).

## Surface: the panel's island counts

- **Source**: `AutoSupportPanel.tsx:334-352`
- **Renders**: three centred stats — `Voxel`, `Minima`, `Total`.
- **Layout & styling**: as stated under the panel: `rounded-md border p-2` +
  `AUTO_SUPPORT_SECTION_CARD`, `grid grid-cols-3 gap-2 text-center`.
- **States**: `islands.scanning` → each value is `'…'`.
- **Data**: `islands.voxelIslands.length`, `islands.minimaIslands.length`,
  `islands.filteredIslands.length`.
- **Callbacks**: none.
- **Text**: `Voxel`, `Minima`, `Total`.
- **Port target**: `panels/content.rs::island_stat` (present, but counts are
  placeholders — `Voxel`/`Minima` are `"0"`, `Total` is the generated support count,
  and `…` never renders).

## Surface: `AutoSupportRunDiagnostics` — last run, on the panel

- **Source**: `src/components/controls/autoSupport/AutoSupportRunDiagnostics.tsx:32-160`
- **Renders**: nothing when both props are null; otherwise up to two stacked blocks.
- **Layout & styling**:
  - Sizing block: `div.rounded-md.border` + `AUTO_SUPPORT_SECTION_CARD`; header is a
    `button` `w-full flex items-center justify-between px-2.5 py-2 text-[10px] font-semibold uppercase tracking-wide`,
    colour `var(--text-muted)`, with a `ChevronDown` `w-3 h-3 transition-transform`,
    `transform: showSizingDebug ? 'rotate(180deg)' : 'rotate(0deg)'`.
  - Disclosure body: `px-2.5 pb-2 space-y-1 text-[10px] tabular-nums`, muted; each row
    `flex justify-between` with the value in `var(--text-strong)`. The first row carries
    `border-t pt-1.5` with `borderColor: 'var(--border-subtle)'`; the "Shaft Ø range"
    row carries `borderTop: '1px solid var(--border-subtle)', paddingTop: 2, marginTop: 2`.
  - Report button: `w-full rounded-md border px-2.5 py-2 text-[10px] font-semibold uppercase tracking-wide flex items-center justify-between`,
    `style={{ borderColor: 'var(--border-subtle)', background: 'var(--surface-1)', color: 'var(--text-muted)' }}`;
    tally span `text-[9px] normal-case tracking-normal`.
- **States**: `sizingDebug` null → block absent; `forestReport` null → button absent.
  `showSizingDebug` is local, default `false` (disclosed, never open).
- **Data**: props `sizingDebug: SizingDebugInfo | null`, `forestReport: ForestReport | null`,
  `onShowForestReport`. Numbers go through `useLocaleFormat()` →
  `fmt.quantity(value, fractionDigits, unit)` and `fmt.fixed(value, digits)`.
- **Callbacks**: header button toggles `showSizingDebug`; report button calls
  `onShowForestReport` (the panel opens the Forest Report modal).
- **Text** (rows in order, verbatim):
  `Sizing Debug` (header, title `The inputs and factors the last run sized the supports with`),
  `Model volume` (`fmt.quantity(mm3/1000, 1, 'cm3')`), `Est. weight` (`estimatedWeightG`, 1, `'g'`),
  `Candidates` (`totalCandidates`), `Model size` (`modelSizeMm`, 0, `'mm'`),
  `Weight / support` (`weightPerSupportG`, 2, `'g'`), `Load share` (`loadShareG`, 2, `'g'`),
  `Sizing factors` → `×{fmt.fixed(sizeFactor,2)} size · ×{fmt.fixed(loadFactor,2)} load`,
  `Avg island area` (`avgIslandAreaMm2`, 2, `'mm2'`), `Standalone trunks` (`standaloneHosts`),
  `Grid infill trunks` (`gridInfillHosts`),
  `Shaft Ø range` → `{fmt.fixed(min,2)}–{fmt.quantity(max,2,'mm')}`,
  `Tip Ø range` → `{fmt.fixed(min,2)}–{fmt.quantity(max,2,'mm')}`.
  Report button: `Show Forest Report`, tally `{hostCount}H {leafCount}L {branchCount}B · {trees.length} trees`,
  title `Every placed support with its size and fan-out groups`.
- **Port target**: none — `panels/content.rs` declares `PORT GAP: no debug mode, so
  AutoSupportRunDiagnostics never renders`. It needs a new module (suggested
  `gpui/src/features/auto_support_settings.rs` or a sibling `auto_support_panel.rs`).

## Surface: Forest Report modal

- **Source**: `AutoSupportPanel.tsx:387-431`
- **Renders**: `StructuredDialogModal` titled `Forest Report`, body = one bordered box
  holding a `<pre>`.
- **Layout & styling**: `maxWidthClassName="max-w-[72rem]"`, `iconTone="neutral"`;
  body wrapper `rounded-md border overflow-hidden` + `AUTO_SUPPORT_SECTION_CARD`;
  `<pre>` = `px-3 py-2 text-[10px] leading-relaxed whitespace-pre-wrap break-words tabular-nums overflow-y-auto`,
  `style={{ color: 'var(--text-muted)', maxHeight: '70vh' }}`.
  Modal chrome (from `StructuredDialogModal`): backdrop `fixed inset-0 z-[100] flex items-center justify-center bg-black/55 backdrop-blur-sm px-3`;
  panel `w-full max-w-… overflow-hidden rounded-xl border shadow-2xl`, `background: var(--surface-0)`,
  `borderColor: var(--border-subtle)`, `boxShadow: '0 24px 46px rgba(0,0,0,0.42)'`; header
  `flex items-center justify-between gap-4 border-b px-5 py-4`; body `space-y-4 p-5`;
  actions `grid grid-flow-col auto-cols-fr gap-2 pt-1` (buttons share the width equally).
- **States**: only renders the `<pre>` when `forestReport` is non-null.
- **Data**: `forestReport` from the last run; text from `forestReportToText(forestReport)`
  (`src/supports/autoSupport/autoPlace.ts:2730`) — a plain-text dump with sections
  `FOREST REPORT`, `SCAN`, `ORPHANS CULLED`, `PLACEMENT DIAGNOSTICS`, then the tally line.
- **Callbacks**: `Copy` → `void navigator.clipboard?.writeText(forestReportToText(forestReport))`
  (optional chaining: a browser without the API silently does nothing, no toast).
  `Close` / ✕ / backdrop / Escape → `setShowForestReport(false)`.
- **Text**: aria `Forest Report` · title `Forest Report` · subtitle
  `Every placed support with its size and fan-out groups` · `Copy`
  (title `Copy the whole report to the clipboard`) · `Close` (title `Close the report`).
- **Port target**: none.

## Surface: the settings dialog (portal) — header + body host

- **Source**: `AutoSupportPanel.tsx:433-502`
- **Renders**: `createPortal` into `document.body`: full-screen backdrop with a
  1120px panel: header row (gradient icon tile, `Auto Supports (Beta) Settings` +
  one-line subtitle, close ✕) and then `AutoSupportSettingsBody`.
- **Layout & styling**:
  - Backdrop: `fixed inset-0 z-[60] flex items-stretch justify-center bg-black/58 backdrop-blur-sm p-5 ui-modal-backdrop-enter`;
    `onMouseDown` closes **only** when `event.target === event.currentTarget`.
  - Panel: `w-full max-w-[1120px] h-full flex flex-col rounded-xl border shadow-2xl overflow-hidden ui-modal-panel-enter`,
    `style={{ background: 'var(--surface-0)', borderColor: 'var(--border-strong)' }}`,
    `role="dialog" aria-modal="true"`.
  - Header: `flex items-center justify-between gap-4 px-4 py-3 shrink-0`,
    `borderBottom: '1px solid var(--border-subtle)'`; left `flex min-w-0 items-center gap-2.5`;
    icon tile `inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border`,
    `borderColor: var(--border-subtle)`,
    `background: 'linear-gradient(135deg, color-mix(in srgb, var(--accent), var(--surface-1) 84%), color-mix(in srgb, var(--accent-secondary), var(--surface-1) 90%))'`,
    with `Settings2` `h-4 w-4` in `var(--accent)`.
  - Title `text-base font-semibold` `var(--text-strong)`; subtitle
    `mt-0.5 text-xs leading-snug` `var(--text-muted)`.
  - Close: `IconButton variant="solid" size="sm"` (`h-8 w-8 !p-0`),
    `className="inline-flex shrink-0 items-center justify-center leading-none !p-0"`,
    `X` `h-4 w-4`.
- **States**: `showSettings` only. Escape closes it through
  `useEscapeToClose(showSettings, closeSettings)` (`:150`) — the same path as ✕ and the
  backdrop, i.e. the dirty check applies.
- **Data**: `draft` + `setDraft` (local `useState` seeded from `getSettings().autoSupport`
  at mount, re-seeded on `openSettings`), `presetLocked` derived as
  `presets.find(p => p.id === activePresetId)?.isBuiltIn === true`.
- **Callbacks**: close → `closeSettings` (dirty ⇒ opens the discard dialog instead);
  body props:
  - `debugSimpleRender={supportSettings.debugSimpleSupportRender}`,
    `onToggleDebugSimpleRender={() => updateDebugSimpleSupportRender(!debugSimpleRender)}`;
  - `diagnostics={{ debugSupportOriginColors, debugSkipAutoBracing }}` from the store;
  - `onToggleDiagnostic={(key, enabled) => { updateAutoSupportDiagnostic(key, enabled);
    setDraft(current => ({ ...current, [key]: enabled })); }}` — comment:
    `Applied at once, so closing the dialog does not ask whether to discard a view switch. The draft follows, or a later Save would write the value the toggle just replaced.`
  - `debugMode` + `onToggleDebugMode` (panel session state).
- **Text**: `Auto Supports (Beta) Settings` · `Detected surfaces, density and sizing, stability, and the saved presets a run follows` ·
  aria `Auto-support settings` · close aria `Close dialog` · close title
  `Close without applying the edits made in this dialog`.
- **Port target**: `gpui/src/features/auto_support_settings.rs::auto_support_settings(open, entity)`
  (present) + `DragonFruit::show_support_settings` in
  `gpui/src/app.rs:269-272, 1672-1676, 2034`. That module's header documents 13 port
  deviations — read them before touching it.

## Surface: `AutoSupportSettingsBody` — the dialog body

- **Source**: `src/components/controls/autoSupport/AutoSupportSettingsBody.tsx:275-355`
- **Renders**: a scrolling column: preset selector card → 2-column grid of five
  policy `FieldCard`s → `DiagnosticsCard` → warning-toned calibration card
  (`lg:col-span-2`) ; then the pinned footer.
- **Layout & styling**:
  - Root: `flex-1 min-h-0 flex flex-col`.
  - Scroll region: `flex-1 min-h-0 overflow-y-auto overflow-x-hidden custom-scrollbar p-4 space-y-3`.
  - Grid: `grid grid-cols-1 lg:grid-cols-2 gap-3` — comment: `The cards stretch to their row's height — the anti-aliasing section's idiom (profileFormAtoms.tsx): a plain grid, whose items stretch by default, so a short card leaves no dead space beside a tall one. Six cards fill three rows exactly; the disclosure below spans both columns.`
  - `FieldCard`: `section.rounded-xl border p-3` with `FIELD_CARD_STYLE` =
    `{ borderColor: 'var(--border-subtle)', background: 'var(--surface-2)' }`; header
    `ui-meta font-semibold uppercase tracking-wide` with `title={section.hint}`;
    optional subtitle `mt-0.5 text-xs leading-snug` `var(--text-muted)`;
    fields `mt-2 grid grid-cols-2 gap-2`.
  - Calibration card: `section.lg:col-span-2.rounded-xl.border.p-3`,
    `borderColor: 'color-mix(in srgb, #d97706, var(--border-subtle) 36%)'`,
    `background: 'color-mix(in srgb, #d97706, var(--surface-1) 92%)'`; header same
    `ui-meta` line; fields in `mt-2` → `grid grid-cols-2 gap-2`.
- **States**: `presetLocked` disables every policy field (knobs, toggles, sizing tier)
  and swaps each field's help for `LOCKED_PROFILE_HINT`; the diagnostics card stays
  live (comment in the props type: `The diagnostics stay live: they are view switches rather than run policy, they are excluded from the preset's dirtiness, and needing one to read the scene has nothing to do with which preset is selected.`).
- **Data**: props only — `draft`, `setDraft`, `debugSimpleRender`,
  `onToggleDebugSimpleRender`, `diagnostics`, `onToggleDiagnostic`, `debugMode`,
  `onToggleDebugMode`, `presetLocked`. Store-free (tests render it to static markup).
- **Card order** (`AUTO_SUPPORT_POLICY_SECTIONS`): Detection, Distribution, Density &
  Sizing, Stability, Post-processing; then Diagnostics; then
  `Advanced (calibration)`.
- **Text**: headers `Detection` / `Distribution` / `Density & Sizing` / `Stability` /
  `Post-processing` / `Diagnostics` / `Advanced (calibration)`; subtitles
  `What needs support, and how close is too close` · `Where contacts land, and how far members fan` ·
  `How many supports, and how thick` · `Held against toppling and peel` · `The passes after placement`.
- **Port target**: `features/auto_support_settings.rs` (`field_card`, `two_cards`,
  `field_grid`, `advanced_card`, `diagnostics_card`). Known deviations there: no
  `lg:` breakpoint (always two columns — the 1120px cap is wider than 1024px), no
  `tracking-wide`, headers uppercased in the string, and `grid-cols-2` emulated with
  flex rows.

### The field controls inside the body

- `NumberField` (`:68-113`): `div.space-y-1` wrapping `LabeledNumberInput` with
  `label`, `unit` (omitted when empty string: `knob.unit || undefined`),
  `helpText`, `title`, `disabled`, `value={draft[knob.key]}`, `step`, and
  `onChange={v => setDraft(cur => ({ ...cur, [knob.key]: normalizeFieldValue(knob.min, knob.max, knob.step, v) }))}`.
  Locked: wrapped in `<Tooltip content={help} fullWidth wrapperClassName="w-full [&>*]:w-full">`
  (comment: `The app's own tooltip, on the whole field: a locked control is dead to the pointer (hence disabled:pointer-events-none on it), so the wrapper is what receives the hover. [&>*]:w-full is load-bearing: the wrapper is a flex container, so without it the field's label would be a content-sized flex item and the input would shrink to its intrinsic width.`).
- `LabeledNumberInput` itself (`profileFormAtoms.tsx:302`): label row
  `space-y-1 block` + `ui-label font-medium inline-flex items-center gap-1.5` with a
  `FieldHelpTooltip` ⓘ; input `ui-input w-full h-[36px] pl-2.5 pr-6 leading-tight text-sm no-spinners`,
  `pr-14` when a unit renders, `pr-20` for a tag; unit `span` at
  `absolute inset-y-0 right-6 z-10 text-[11px] font-semibold` muted; two carets in a
  `absolute inset-y-0 right-1 z-20 flex w-4 flex-col` column, each `h-3 w-3`;
  ArrowUp/ArrowDown nudge, Enter blurs (commits), `-` is refused while typing.
  `disabled` adds `opacity-55 cursor-not-allowed` + `disabled:pointer-events-none`.
  Caret aria labels: `Increase {label} ({unit})` / `Decrease …`.
- `ToggleField` (`:116-147`): `div.space-y-1` + `LabeledToggleInput` with
  `checked={draft[toggle.key]}`, `onChange={next => setDraft(cur => ({ ...cur, [toggle.key]: next }))}`.
  `LabeledToggleInput` (`profileFormAtoms.tsx:629`) renders `label.space-y-1.block`,
  the same label row + ⓘ, then a `button role="switch" aria-checked` styled
  `ui-input w-full h-[36px] px-2.5 leading-tight text-sm inline-flex items-center justify-between`;
  inside: text `Enabled` / `Disabled` and a `h-5 w-9 rounded-full p-0.5` track with a
  `h-4 w-4 rounded-full bg-white` thumb (`translate-x-4` when on). Checked colours:
  border `color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 36%)`,
  background `color-mix(in srgb, var(--accent-secondary), var(--surface-1) 90%)`,
  text `var(--text-strong)`; unchecked: `var(--border-subtle)` / `var(--surface-1)` /
  `var(--text-muted)`; disabled adds `color-mix(in srgb, var(--surface-2), black 8%)`.
- `normalizeFieldValue` (`:60`): `Math.min(max, Math.max(min, raw))` then
  `Number(clamped.toFixed(decimalsForStep(step)))`, `decimalsForStep = step < 0.1 ? 2 : step < 1 ? 1 : 0`.
  Comment: `The settings store clamps to AUTO_SUPPORT_CONSTRAINTS on write; this keeps a typed number from showing as one value and landing as another.`

### `FieldCard` field order (the render order is load-bearing)

Per section the body renders **toggles first, then the sizing tier (density only),
then knobs** (`:296-305`):

| Section | Toggles | Sizing tier | Knobs (in order) |
|---|---|---|---|
| detection | `Enabled`, `Prioritize Dual` | — | `Min Island Size`, `Merge Radius` |
| distribution | — | — | `Fan Reach`, `Fan Angle` |
| density | — | `Sizing Tier` (`col-span-2`) | `Support Density`, `Support Size`, `Grid Threshold`, `Flat Boost`, `Slope Relax`, `Suction Scale` |
| stability | `Stabilization Anchors`, `Minima Reinforcement` | — | `Self-Support Angle` |
| postProcessing | — | — | `Branches per Column`, `Coverage Target` |

## Surface: `NumberField` / `ToggleField` labels, bounds and hints (catalogue)

- **Source**: `src/components/controls/autoSupport/autoSupportPanelTabs.ts:143-241`
- **Renders**: nothing itself; the tables the body maps over.
- **Data** — `KNOBS` (`:143`), in table order:

| key | section | label | min | max | step | unit | hint (msgid) |
|---|---|---|---|---|---|---|---|
| `minIslandAreaMm2` | detection | Min Island Size | 0.01 | 2 | 0.01 | `mm²` | `Skip detected areas smaller than this — tiny specks rarely need supports` |
| `tipInfluenceRadiusMm` | detection | Merge Radius | 0.1 | 10 | 0.1 | `mm` | `A candidate within this 3D distance of an existing support merges into it instead of starting a new trunk` |
| `leafFanRadiusMm` | distribution | Fan Reach | 2 | 15 | 0.5 | `mm` | `Max horizontal distance a fan-out leaf may span from a trunk shaft` |
| `leafFanMaxAngleDeg` | distribution | Fan Angle | 20 | 80 | 5 | `°` | `Max angle from vertical for fan-out leaves` |
| `areaPerSupportMm2` | density | Support Density | 1 | 30 | 0.5 | `mm²` | `Projected area each support carries — smaller = more, tighter supports (grid spacing ≈ √value)` |
| `sizeScale` | density | Support Size | 0.5 | 2 | 0.05 | `×` | `Master multiplier over the preset sizing bands — thicker or thinner everywhere` |
| `gridAreaThresholdMm2` | density | Grid Threshold | 5 | 200 | 5 | `mm²` | `Flat regions at/above this area get a full grid; smaller regions get a single support` |
| `flatDensityBoost` | density | Flat Boost | 0.5 | 1 | 0.05 | `×` | `Grid spacing on flat ceilings — lower = denser supports on anchor surfaces (0.7 = ~2× the supports)` |
| `slopeRelaxFactor` | density | Slope Relax | 1 | 2 | 0.1 | `×` | `Grid spacing on slopes at the self-support angle — higher = sparser` |
| `suctionAreaExponent` | density | Suction Scale | 0 | 0.4 | 0.05 | `''` | `How strongly flat density grows with region area — large shallow ceilings carry more peel. 0 = off` |
| `overhangSelfSupportAngleDeg` | stability | Self-Support Angle | 20 | 75 | 5 | `°` | `Surfaces flatter than this angle get supports (resin standard: 45°). Higher = fewer, mostly on the steepest parts.` |
| `maxAttachmentsPerTrunk` | postProcessing | Branches per Column | 2 | 50 | 1 | `''` | `Max branches + leaves one trunk may carry before new trunks are started — the cap on chunk consolidation` |
| `coverageTargetPercent` | postProcessing | Coverage Target | 75 | 100 | 5 | `%` | `How much of each region's footprint the grid must cover before gap-filling stops` |

  `TOGGLES` (`:193`): `enabled` (detection) `Enabled` / `Generate supports automatically on scan`;
  `prioritizeIntersection` (detection) `Prioritize Dual` / `Islands found by BOTH the slice and mesh scans are placed first (they are the most certain)`;
  `stabilizationEnabled` (stability) `Stabilization Anchors` / `Add contacts along the edge or corner a pose bears on, so a leaning part is held against toppling and peel (default on)`;
  `minimaReinforcementEnabled` (stability) `Minima Reinforcement` / `Ring each mesh minima with contacts on its own flank, so a section starts on a base instead of a needle (default on)`;
  `debugSupportOriginColors` (debug) `Origin Colors` / `Debug: color supports by origin — stump (red), overhang (orange), island (blue), standalone (purple), reinforcement (teal)`;
  `debugSkipAutoBracing` (debug) `No Brace` / `Debug: skip automatic bracing for this run`.

  `ADVANCED_CALIBRATION_KNOBS` (`:181`) — bounds come from
  `AUTO_SUPPORT_CONSTRAINTS`, unit `''`:

| key | label | min/max/step | hint |
|---|---|---|---|
| `tipContactMarginScale` | Tip Fit Margin | 0.4 / 1 / 0.05 | `How much of the free width a contact tip may occupy. The default, 0.9, keeps the disc just inside the feature it lands on; lower values shrink every tip on a feature narrower than the band tip.` |
| `memberHostShaftRatio` | Member / Host Ratio | 0.5 / 1 / 0.05 | `Least diameter a branch or leaf takes as a fraction of the trunk it grows from. The default, 0.7, keeps a hosted member a step below its host instead of a needle beside it.` |
| `modelSizeFactorCap` | Model Size Cap | 1 / 2 / 0.05 | `Ceiling of the size factor derived from the model's bounding-box diagonal. The default, 1.45, bounds how much a large part's shafts thicken.` |
| `modelLoadFactorCap` | Model Load Cap | 1 / 2 / 0.05 | `Ceiling of the load factor derived from resin mass per support. The default, 1.3, bounds how much a heavy part's shafts thicken.` |
| `heightFactorCap` | Height Factor Cap | 1 / 2 / 0.05 | `Ceiling of the height factor for a tall column, whose buckling load falls with height. The default, 1.35, bounds how much a tall support thickens.` |

  `ADVANCED_CALIBRATION_TOGGLE` (`:207`): `modelScaleEnabled`, section `debug`,
  label `Model-Scale Sizing`, hint
  `Master switch for the run-level size, load and height factors. Off pins all three to ×1, so sizing is the active band plus the local terms alone (default: on).`
- **Section definitions**: `POLICY_SECTIONS` (`:45`) with `subtitle` + long-form
  `hint` used as the card header's `title`; `AUTO_SUPPORT_ADVANCED_SECTION`
  (`label: 'Advanced (calibration)'`, `hint: 'The sizing constants the engine ships with. They are stored per preset.'`);
  `AUTO_SUPPORT_SECTIONS = [...POLICY_SECTIONS, ADVANCED]`;
  `AUTO_SUPPORT_POLICY_SECTIONS = POLICY_SECTIONS`.
- **Derived**: `groupBySection` (`:212`) produces `KNOBS_BY_SECTION` / `TOGGLES_BY_SECTION`
  with **every** section key present (`Record<AutoSupportSectionKey, T[]>`);
  `DIAGNOSTIC_TOGGLES = TOGGLES_BY_SECTION.debug.filter(key ∈ DIAGNOSTIC_AUTO_SUPPORT_KEYS)`.
- **Text** (extra): `TIER_HINTS` = `{ light: 'Sparse supports — the detail sizing band', medium: 'Balanced supports — the structure sizing band', heavy: 'Dense supports — the anchor sizing band' }`;
  `SIZING_TIER_FIELD` = `{ label: 'Sizing Tier', hint: 'The Support Studio preset whose tip, shaft and root numbers size this run. Editing that preset changes what auto-support prints — Duplicate it first if you only want to change the run.' }`;
  `LOCKED_PROFILE_HINT` = `This is a built-in profile, so its fields are fixed. Duplicate it and edit the copy — custom settings belong in a custom profile.`
- **Port target**: `features/auto_support_settings.rs` (`POLICY_SECTIONS`, `KNOBS`,
  `CALIBRATION_KNOBS`, `TOGGLES`, `CALIBRATION_TOGGLE`, `DIAGNOSTIC_TOGGLES`,
  `SectionKey`, `KnobKey`, `ToggleKey`). **Gap**: the port's `CALIBRATION_KNOBS` sets
  `section: SectionKey::Detection` for the first entry (a copy/paste artefact); the
  calibration group is rendered by `advanced_card` regardless, so it is cosmetic.

## Surface: `DiagnosticsCard` — debug switches + the two session switches

- **Source**: `src/components/controls/autoSupport/AutoSupportSettingsBody.tsx:170-227`
- **Renders**: a `FieldCard`-shaped `section.rounded-xl.border.p-3` with header
  `Diagnostics`, then `mt-2 grid grid-cols-2 gap-2` holding, in order:
  `DIAGNOSTIC_TOGGLES` (Origin Colors, No Brace), then `Simplified`, then `Debug Mode`.
- **Layout & styling**: same `FIELD_CARD_STYLE` as the policy cards (surface-2); the
  header has no `title` attribute (only the policy cards' headers do).
- **States**: none of the four honours `presetLocked` — deliberate.
- **Data**: `diagnostics` come from the **store** (`supportSettings.autoSupport`), never
  from the draft; `debugSimpleRender` is the top-level `SupportSettings.debugSimpleSupportRender`;
  `debugMode` is panel session state.
- **Callbacks**: `onToggleDiagnostic(key, next)` (writes store immediately **and**
  mirrors into the draft); `onToggleDebugSimpleRender()` →
  `updateDebugSimpleSupportRender(!debugSimpleRender)` (store write, *not* mirrored into
  the draft); `onToggleDebugMode()` (panel `setDebugMode(cur => !cur)`).
- **Text**: `Diagnostics` · `Origin Colors` · `No Brace` · `Simplified`
  (hint `Debug: simplified support render — contact disks/cones plus line vectors instead of full shafts`) ·
  `Debug Mode` (hint `Debug: show the last run's sizing factors and forest report in the panel`).
  For the two store-backed toggles, `title` = `helpText` = the catalogue hint.
- **Port target**: `features/auto_support_settings.rs::diagnostics_card` (present). Two
  session switches there are local statics dressed as `ToggleDef`s (`SIMPLIFIED_TOGGLE`,
  `DEBUG_MODE_TOGGLE`, reusing diagnostic keys) — cosmetic, but note their `key` fields
  lie.

## Surface: `AutoSupportSizingTierField` — Support Studio preset as the sizing band

- **Source**: `src/components/controls/autoSupport/AutoSupportSizingTierField.tsx:88-140`
- **Renders**: `div.col-span-2.space-y-1` with a label row and one `SelectDropdown`.
- **Layout & styling**: label row
  `ui-label font-medium inline-flex items-center gap-1.5` + `FieldHelpTooltip`;
  select `className="space-y-0"`,
  `selectClassName="w-full !h-8 pl-2.5 pr-10 leading-tight text-[12px]"` (left-aligned —
  no `CENTERED_SELECT_PADDING` here).
- **States**:
  - Options = `useSupportStudioPresets()` mapped to `{ value: preset.id, label: preset.name,
    rightContent: FACTORY_TIER_IDS.has(id) ? 'Built-in' : 'Custom' }` where
    `FACTORY_TIER_IDS = new Set([DETAIL_PRESET.id, STRUCTURE_PRESET.id, ANCHOR_PRESET.id])`
    = `{detail, structure, anchor}`.
  - `disabled` → dropdown disabled (`SelectDropdown` adds
    `disabled:pointer-events-none disabled:opacity-55 disabled:cursor-not-allowed`), the
    trigger gets **no** `title` (so only the app tooltip shows), and the whole field is
    wrapped in `<Tooltip content={help} fullWidth wrapperClassName="w-full [&>*]:w-full">`
    with `help = LOCKED_PROFILE_HINT`.
- **Data**: `draft.sizingPreset` (an **open id**: any Support Studio preset, possibly a
  deleted one); `onChange(id) → setDraft(cur => ({ ...cur, sizingPreset: id }))`.
  The store's `select` never writes the Support Studio preset.
- **Snapshot caching** (important): `getPresetList()` builds a fresh array each call,
  which `useSyncExternalStore` reads as a change every render — "an endless render loop".
  The module caches one list per notification (`studioPresetsSnapshot`) and registers
  `subscribeToPresets(() => { studioPresetsSnapshot = null; })` **once, at module load**.
- **`sizingTierName(id, presets)`**: the named preset's `name`, else the `structure`
  preset's name (`sizingTierName` falls back to `'structure'` — the same fallback the
  engine uses), else the raw id.
- **Text**: `Sizing Tier` (label and aria), the hint above.
- **Port target**: `features/auto_support_settings.rs::sizing_tier_field` (present,
  **closed set** `SizingTier::{Detail, Structure, Anchor}` — PORT DEVIATION 9: a custom
  Support Studio preset cannot be listed until `sizingPreset` becomes an open id; the
  port reads `support-presets.json` for the factory bands).

## Surface: `AutoSupportPresetSelector` — the dialog's preset strip

- **Source**: `src/components/controls/autoSupport/AutoSupportPresets.tsx:257-470`
- **Renders**: `section.rounded-xl.border.p-3` (border `var(--border-subtle)`,
  background `var(--surface-2)`) containing one `div.flex.items-center.gap-2`:
  the selector (`min-w-0 flex-1`), then `Rename`, `Duplicate`, `Export`, `Import`
  buttons, then a hidden `<input type="file">`; below (when set) an error banner.
- **Layout & styling**:
  - Selector: `SelectDropdown` `className="space-y-0"`,
    `selectClassName="w-full !h-8 pl-2.5 pr-10 leading-tight text-[12px]"`,
    `menuClassName="max-w-[26rem]"`; **no** `selectStyle` (so the trigger keeps the
    `ui-input` look, unlike the panel's surface-1 fill).
  - Strip buttons: `Button variant="secondary" size="auto"` + `STRIP_BUTTON` =
    `'!h-8 !px-2.5 !py-0 text-[11px] inline-flex items-center gap-1 disabled:opacity-45 disabled:cursor-not-allowed'`;
    each carries a `h-3.5 w-3.5` icon (`PenLine`, `Copy`, `Download`, `Upload`).
  - Error banner: `mt-2 rounded-md border px-2 py-1.5`,
    `borderColor: 'color-mix(in srgb, var(--danger), var(--border-subtle) 45%)'`,
    `background: 'color-mix(in srgb, var(--danger), var(--surface-1) 92%)'`,
    `color: 'var(--danger)'`, `role="alert"`, inner `span.text-[10px].leading-snug.break-words`.
- **States**:
  - Options: each preset → `{ value, label: translateAutoSupportPresetName(preset,_) + (preset.id === activeId && modified ? ' *' : ''), icon: preset.id === activeId ? <Check className="h-3.5 w-3.5"/> : undefined, rightContent: \`${sizingTierName(preset.settings.sizingPreset, supportStudioPresets)} · ${preset.isBuiltIn ? 'Built-in' : 'Custom'}\` }`.
  - `activeId === null` → prepend `{ value: '', label: NO_ACTIVE_PRESET_LABEL, disabled: true }`.
  - `modified = useAutoSupportDialogChanges(draft)` → the `*` suffix on the active
    row plus trigger `aria-label` `Auto-support preset, modified` and title
    `The settings no longer match this preset. Save overwrites it; Reset reloads it.`;
    clean → aria `Auto-support preset`, title
    `The saved run policy the settings below are. Selecting one applies it immediately; the rest of the dialog stays staged until you save.`
  - `Rename`: `disabled={!activePreset || isBuiltIn}`; title isBuiltIn →
    `A built-in's name is translated and cannot be renamed`, else `Rename the selected preset`.
  - `Duplicate`: `disabled={!activeId}`.
  - `Export`: `disabled={!activeId}`.
  - `Import`: always enabled.
  - Error banner after a failed import/export.
  - Dropdown footer action: `menuFooterAction={{ label: 'New preset…', icon: <Plus/>,
    tone: 'accent', onClick: () => setNameDialog({ mode: 'create', name: 'New Preset' }) }}`.
- **Data** (`useAutoSupportPresetState`, `:216-243`): `presets` (snapshot),
  `activeId`, `dirty`, `live` each via their own `useSyncExternalStore`;
  `activePreset` memoised via `getAutoSupportPreset(activeId)`. Local state:
  `nameDialog: { mode: 'create' | 'rename'; name: string } | null`, `errorMessage`.
  Comment on why four subscriptions: `React Compiler treats a bare call to an imported function as pure and evaluates it once, which would otherwise freeze both at whatever they were when the component mounted — a selection that never updates and a footer whose Save never enables.`
- **Callbacks**:
  - select → `selectAutoSupportPreset(id, setDraft)` → `setActiveAutoSupportPreset(id)`
    **then** `setDraft(getSettings().autoSupport)` (applies immediately; the draft
    follows so the fields show what was applied).
  - Rename → opens the name dialog in `rename` mode. The strip opens the dialog only
    when `activePreset && !activePreset.isBuiltIn`, so the disabled state and the
    handler agree.
  - Duplicate → `duplicateAutoSupportPreset(activeId, formatAutoSupportPresetCopyName(activePreset, _))`
    (copy name = `"{shown name} copy"`, msgid `` `${presetName} copy` ``).
  - Export → `exportPresetToFile(activeId)` (see below), errors → banner.
  - Import → Tauri: `importFromNativeDialog()`; otherwise `importInputRef.current?.click()`.
  - `input onChange` → read `files[0]`, reset `event.target.value = ''`, `file.text().then(applyImportedText)`.
  - `applyImportedText(text)` → `importAutoSupportPresetFromJson(text)`; success clears
    the error and `setDraft(getSettings().autoSupport)`; failure sets `errorMessage`.
- **Export/import mechanics**:
  - `exportPresetToFile` (`:171`): `getAutoSupportPreset(id)` (bail if absent) →
    `exportAutoSupportPresetToJson(id)`; filename =
    `preset.name.trim().toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '')`
    (empty → `auto-support-preset`) + `.dragonfruit-auto-support.json`.
    Tauri: `savePrintArtifactWithNativeDialog(bytes, fileName)`; an error whose message
    contains `cancel` is swallowed, anything else rethrows.
    Browser: `Blob` + `URL.createObjectURL` + a temp `<a download>` clicked and removed,
    then `revokeObjectURL`.
  - `importFromNativeDialog`: `pickOpenFilesWithNativeDialog('bundle', false)` →
    `picked[0]?.path?.trim()` → `readPrintArtifactBytesFromPath(path)` →
    `new TextDecoder().decode(bytes)` → `applyImportedText`; `cancel` swallowed.
- **Text**: `Auto-support preset` · `Auto-support preset, modified` · the two titles
  above · `Rename` · `Duplicate` (`Copy the selected preset under a new name`) ·
  `Export` (`Export the selected preset as a JSON file`) · `Import`
  (`Import a preset from a JSON file, and apply it`) · `New preset…` (menu footer) ·
  `New Preset` (placeholder/starting name) · `Auto-support preset file` (hidden input aria).
- **Port target**: `features/auto_support_settings.rs::preset_strip` (present). Gaps from
  that module's header: **Export/Import are inert** (PORT DEVIATION 3: needs the Tauri
  bridge and a JSON reader; the error banner is therefore never rendered), and there is
  no text entry, so `Create`/`Rename` use the name the dialog opened with (PORT
  DEVIATION 2).

## Surface: the name dialog (Create / Rename)

- **Source**: `AutoSupportPresets.tsx:405-470`
- **Renders**: `StructuredDialogModal` with a labelled text input; two buttons.
- **Layout & styling**: `icon={<PenLine className="h-4 w-4" />}`, `iconTone="accent"`;
  body `div.space-y-2` with label
  `block text-xs font-semibold uppercase tracking-wide` (`var(--text-muted)`) and
  `input.ui-input.h-9.w-full.text-xs` (so 36px tall, matching the fields).
  `Create`/`Save Name` button carries `style={SAVE_STYLE}` and `Check` icon.
- **States**: `nameDialog == null` → closed. Title/aria/subtitle/action label all switch
  on `mode`. `Create` disabled while the trimmed name is empty
  (`disabled={(nameDialog?.name.trim().length ?? 0) === 0}`, plus `disabled:opacity-40`).
- **Data**: `nameDialog.name`; initial `New Preset` (create) or `activePreset.name` (rename).
- **Callbacks**:
  - `confirmNameDialog`: bail on empty trim; `create` → `createAutoSupportPreset(name)`;
    `rename` → only if `activePreset && !activePreset.isBuiltIn` →
    `renameAutoSupportPreset(activePreset.id, name)`; then `setNameDialog(null)` and
    `setDraft(getSettings().autoSupport)`.
  - `Enter` in the input calls the same: `event.stopPropagation(); confirmNameDialog();`
    (stopPropagation keeps the dialog's Escape/backdrop handlers out of it).
  - `Cancel` / ✕ / backdrop / Escape → `setNameDialog(null)`.
- **Text**: aria `Create preset` / `Rename Preset`; title `New Preset` / `Rename Preset`;
  subtitle create `The current settings are saved under this name, and it becomes the selected preset.`,
  rename `The selected preset is renamed; its settings are not touched.`; field label and
  aria `Preset name`; primary `Create` (title `Create the preset and select it`) /
  `Save Name` (title `Rename the selected preset`); `Cancel` (title `Leave the collection as it is`).
- **Port target**: `features/auto_support_settings.rs::name_dialog` (present; no text
  entry — see above).

## Surface: `AutoSupportSettingsFooterActions` — Delete / Reset / Save

- **Source**: `src/components/controls/autoSupport/AutoSupportPresets.tsx:556-663`
- **Renders**: a pinned footer bar (`flex items-center justify-between gap-3 px-4 py-3 shrink-0`)
  with `Delete` at the left and `Reset` + `Save` at the right; plus the delete
  confirmation modal.
- **Layout & styling**: footer
  `borderTop: '1px solid var(--border-subtle)'`,
  `background: 'color-mix(in srgb, var(--surface-1), transparent 10%)'`.
  - `Delete`: `Button variant="tinted-danger" size="md"`,
    `className="inline-flex items-center justify-center gap-1.5 disabled:cursor-not-allowed"`,
    `Trash2` `h-3.5 w-3.5`.
  - `Reset`: `Button variant="secondary" size="auto"` + `FOOTER_BUTTON` =
    `'inline-flex items-center gap-1.5 !h-9 px-3 text-[12px] disabled:opacity-45 disabled:cursor-not-allowed'`,
    `RotateCcw` icon.
  - `Save`: `size="md"`, `className="inline-flex items-center justify-center gap-1.5 disabled:cursor-not-allowed"`,
    `style={saved ? SAVED_STYLE : (!hasChanges || isBuiltIn ? undefined : SAVE_STYLE)}`.
    `SAVE_STYLE` = `{ borderColor: 'var(--accent-secondary-action-border)',
    background: 'var(--accent-secondary-action-bg-92)',
    color: 'var(--accent-secondary-action-color)' }`;
    `SAVED_STYLE` = `{ borderColor: 'color-mix(in srgb, var(--success), var(--border-subtle) 40%)',
    background: 'color-mix(in srgb, var(--success), var(--surface-1) 85%)',
    color: 'var(--success)' }`.
- **States**:
  - `Delete` disabled when `!activePreset || isBuiltIn`; title `Built-in presets cannot be deleted` /
    `Delete the selected preset`.
  - `Reset` disabled when `!hasChanges`; title
    `Discard the edits made in this dialog and reload the selected preset`.
  - `Save` disabled when `!hasChanges || isBuiltIn`; title, in priority order:
    `Saved` → `A built-in preset cannot be saved over — Duplicate it to make it yours` →
    `dirty` ? `Write these settings, and overwrite the selected preset with them` :
    `Write these settings to the auto-support settings`.
  - `saved` (2s ack): label becomes `Saved!` with a `Check` `h-3.5 w-3.5`, style swaps to
    `SAVED_STYLE`. Timer cleared on unmount.
  - `hasChanges = useAutoSupportDialogChanges(draft)` = `dirty || policyDiffers(draft, live)`.
- **Data**: `activePreset`, `dirty`, `live` (subscribed), `draft`; local `pendingDelete`, `saved`.
- **Callbacks**:
  - `Delete` → `setPendingDelete(true)` → modal `Delete preset` with title
    `` `Delete "${preset.name}"?` `` (module-scope `formatDeletePresetTitle`, msgid
    `Delete "{presetName}"?`), subtitle `This cannot be undone.`, `iconTone="danger"`.
    Confirm → `deleteActiveAutoSupportPreset(setDraft)` + close; the helper deletes the
    active id and then **selects `'medium'`** (`setActiveAutoSupportPreset('medium')`
    through `selectAutoSupportPreset`). Cancel → close.
    Body text: `The preset is removed from the list. The current settings are left as they are.`
  - `Reset` → `resetToActivePreset(); setDraft(getSettings().autoSupport);`
  - `Save` → `handleSave`: `commitAutoSupportSettings(draft)` (one call: writes the live
    settings *and* the active preset when they differ) → `setDraft(getSettings().autoSupport)`
    → `setSaved(true)` + 2000 ms timer. **The dialog stays open** —
    comment: `Save is the dialog's commit — it writes the settings and the active preset when the draft has drifted from it … It leaves the dialog open and acknowledges with the button itself, because the preset list is behind the dialog`.
  - Delete confirm button style:
    `{ borderColor: 'color-mix(in srgb, var(--danger), var(--border-subtle) 45%)',
    background: 'color-mix(in srgb, var(--danger), var(--surface-1) 88%)',
    color: 'var(--danger)' }`.
- **Text**: `Delete` · `Reset` · `Save` · `Saved!` · `Saved` · `Cancel` (title
  `Keep the preset`) · `Delete preset` (aria; confirm title `Delete preset`).
- **Port target**: `features/auto_support_settings.rs::{footer, footer_delete_button,
  footer_reset_button, footer_save_button, delete_dialog}` (present).

## Surface: discard-changes dialog

- **Source**: `AutoSupportPanel.tsx:504-545`
- **Renders**: `StructuredDialogModal` with a two-sentence body and two buttons.
- **Layout & styling**: `iconTone="warning"`, `zIndexClassName="z-[120]"` (above the
  settings portal's `z-[60]`); body `p.text-[11px].leading-relaxed` muted.
- **States**: only open when `showDiscardSettingsDialog`.
- **Data**: none beyond the flag.
- **Callbacks**: `Keep Editing` → close (back to the settings dialog);
  `Discard` → close **and** `setDraft(getSettings().autoSupport)` **and**
  `setShowSettings(false)`. `onClose` / `onBackdropClick` / Escape → close only.
- **Text**: aria `Discard auto-support settings changes` · title `Discard Changes?` ·
  subtitle `The edits made in this dialog have not been saved.` · body
  `Closing now leaves the auto-support settings as they are. Save in the dialog's footer writes them.` ·
  `Keep Editing` (`Go back to the settings dialog`) · `Discard` (`Close the dialog and discard the edits`);
  Discard's tinted-danger style as above.
- **Port target**: none — `features/auto_support_settings.rs` has a `dialog_cancel_button`
  but no discard confirmation; its docs do not claim one.

## Surface: replace-existing-supports dialog

- **Source**: `AutoSupportPanel.tsx:547-590`
- **Renders**: `StructuredDialogModal` with a bordered explainer and three actions.
- **Layout & styling**: `iconTone="neutral"`; body
  `div.rounded-md.border.p-3` + `AUTO_SUPPORT_SECTION_CARD`, inner
  `p.text-[11px].leading-relaxed` muted.
- **States**: open only when `showReplaceDialog`.
- **Data**: none beyond the flag; the pending run is re-derived from `handleRun`'s decision.
- **Callbacks**: `Cancel` / ✕ / backdrop / Escape → close, nothing runs.
  `Add to existing` → close + `doRun(false)`. `Replace all` (variant `tinted-accent`,
  `className="inline-flex items-center justify-center gap-1.5"`) → close + `doRun(true)`.
- **Text**: aria/title `Existing Supports Detected` · subtitle
  `This model already has supports. How would you like to proceed?` · body
  `You can replace all existing supports with auto-placed ones, or incorporate your existing supports and fill in the gaps.` ·
  `Cancel` (`Keep the existing supports and do nothing`) ·
  `Add to existing` (`Keep the existing supports and place the new ones around them`) ·
  `Replace all` (`Delete this model's existing supports and place the new ones`).
- **Port target**: none.

---

## `src/supports/Settings/autoSupportPresets.ts` — the preset store

- **Source**: whole file (722 lines).
- **Shape**: module-scope mutable singleton, read once at import, persisted to
  `localStorage`, observed by `Set<listener>`.
- **Types**:
  - `AutoSupportPresetSettings = AutoSupportSettings & Record<string, unknown>` —
    "the payload IS the `autoSupport` settings block, whole — including keys this build
    does not know, so a preset written by a newer version still round-trips".
  - `AutoSupportPreset = { id, name, isBuiltIn, settings, updatedAt? }` — a built-in's
    stored `name` is never displayed (translated by id).
  - `AutoSupportPresetDocument = { kind: 'dragonfruit-auto-support-preset';
    formatVersion: 1; exportedAt: ISO string; preset: { name; settings } }`.
- **Storage keys**: `AUTO_SUPPORT_PRESETS_STORAGE_KEY = 'auto-support-presets-v1'`
  (whole collection as `{ byId, allIds }`), `AUTO_SUPPORT_ACTIVE_PRESET_ID_STORAGE_KEY = 'auto-support-active-preset-id-v1'`
  (a bare id string). Writes: `writeStoredState()` writes both keys; when nothing is
  active it **removes** the active-id key (`removeItem`).
- **Built-ins** (`:89`), each spread over `createDefaultAutoSupportSettings()`:
  - `light`: `areaPerSupportMm2: 16`, `sizingPreset: 'detail'`
  - `medium`: plain defaults (`areaPerSupportMm2: 10`, `sizingPreset: 'structure'`)
  - `heavy`: `areaPerSupportMm2: 5`, `sizingPreset: 'anchor'`
  Names are English placeholders `Light`/`Medium`/`Heavy`. They must equal
  `DETAIL_PRESET/STRUCTURE_PRESET/ANCHOR_PRESET.settings.autoSupport`
  (`presets.ts:202,295`; asserted by the store test). `BUILT_IN_IDS` = `['light','medium','heavy']`;
  `BUILT_IN_BY_ID` is the static reserved-id lookup.
- **`KNOWN_AUTO_SUPPORT_KEYS`** = `Object.keys(createDefaultAutoSupportSettings())`;
  **`POLICY_AUTO_SUPPORT_KEYS`** = those minus `DIAGNOSTIC_AUTO_SUPPORT_KEYS`.
- **`readStoredState()`** (`:176`) — load repair, run once:
  built-ins seeded from the factory records; then for each stored record by id:
  a reserved id adopts **only** `settings` (and a finite `updatedAt`) and never the
  stored name/id/`isBuiltIn`; a non-reserved record needs a plain-object record, keeps
  its id, `sanitizePresetName(record.name)` (trim; empty → `Auto-Support Preset`) and
  falls back to the default block when `settings` is not a plain object. `allIds` =
  `[...BUILT_IN_IDS]` then the stored order (skipping built-ins, duplicates, unknown
  ids) then anything present in `byId` but missing from the order. `activeId` is the
  stored id **only if it names a loaded preset**, else `null`
  (comment: `No preset is active until the user picks one. Inferring it from the live settings … would claim the user chose a preset they never picked`).
- **Snapshots**: `snapshot = state.allIds.map(id => state.byId[id])`, rebuilt in
  `notify()` before listeners run (one reference per state).
  `SERVER_SNAPSHOT` is built-ins only (SSR / no storage).
- **`isAutoSupportPresetDirty()`** (`:379`): no active preset → `false`; else
  `autoSupportPolicyDiffers(normalizeAutoSupportSettings(preset.settings), normalizeAutoSupportSettings(getAutoSupportSettings()))`
  — comparison is by `JSON.stringify` **per key**, policy keys only, both sides normalised.
- **Mutations**:
  - `applyPresetSettings(preset)`: `updateAutoSupportSettings(preset.settings)` (the
    settings store normalises/clamps, dropping unknown keys) then
    `saveSettingsToLocalStorage()`, so a restart does not show preset A over preset B's block.
  - `setActiveAutoSupportPreset(id|null)`: unknown id → `console.warn` and **no change**;
    else set active, apply, write, notify.
  - `resetToActivePreset()`: re-apply the active preset, notify (does **not** re-write storage).
  - `createAutoSupportPreset(name)`: new id `asp-<base36 Date.now()>-<5 random base36>`,
    unique name, `settings = snapshotPresetSettings(getAutoSupportSettings())`,
    `isBuiltIn:false`, `updatedAt: Date.now()`, **activated**.
  - `saveAutoSupportPreset(id)`: missing → warn; built-in → warn and return (never writes);
    else replace `settings` + `updatedAt`, write, notify.
  - `commitAutoSupportSettings(draft)`: `updateAutoSupportSettings(draft)` **then**
    `if (activeId && isAutoSupportPresetDirty()) saveAutoSupportPreset(activeId)` —
    the dirty test is deliberately read *after* the write.
  - `renameAutoSupportPreset(id, name)`: missing → warn + `null`; built-in → warn + `null`;
    else `{...preset, name: ensureUniquePresetName(name, id), updatedAt: Date.now()}`.
  - `duplicateAutoSupportPreset(id, copyName?)`: missing → `null`; else inserts a
    non-built-in copy named `ensureUniquePresetName(copyName ?? "<source> copy")`,
    block = `snapshotPresetSettings(source.settings)`, **activated**, and the live block
    is deliberately **not** re-applied.
  - `deleteAutoSupportPreset(id)`: missing/built-in → warn + no-op; else remove from
    `byId`/`allIds`, `activeId = null` if it was the deleted one, write, notify.
  - `restoreAutoSupportFactoryDefaults()`: resets the three built-ins to the factory
    records; user presets and the live block untouched. **No UI calls it** (the footer's
    Reset is `resetToActivePreset`).
  - `ensureUniquePresetName(name, excludeId?)`: case-insensitive, `" (2)"`, `" (3)"`, …
  - `snapshotPresetSettings(settings)`: `structuredClone(migrateLegacySizingPreset(settings))`
    — the one write path where the `sizingBand`→`sizingPreset` migration runs.
- **Import/export**:
  - `exportAutoSupportPresetToJson(id)`: throws `Unknown auto-support preset: ${id}.` for an
    unknown id; otherwise `JSON.stringify(doc, null, 2)` with `exportedAt: new Date().toISOString()`.
  - `importAutoSupportPresetFromJson(jsonText)`: throws, in this order, with these exact messages:
    - `Invalid auto-support preset file: not valid JSON.`
    - `Invalid auto-support preset file: expected a JSON object.` (null, array, scalar)
    - `Invalid auto-support preset file: unsupported kind.` (missing or wrong `kind`)
    - `Invalid auto-support preset file: unsupported format version ${formatVersion}.`
      (string-interpolated — `'1'` prints `1`)
    - `Invalid auto-support preset file: missing preset.`
    - `Invalid auto-support preset file: missing preset name.` (non-string or blank)
    - `Invalid auto-support preset file: settings must be an object.`
    On success: new id, unique name, `isBuiltIn:false`, settings =
    `snapshotPresetSettings({ ...createDefaultAutoSupportSettings(), ...migrateLegacySizingPreset(payload.settings) })`,
    inserted **and activated**, then `applyPresetSettings(preset)` again. Nothing is
    added when a rejection throws.
- **Text**: only the preset default name `Auto-Support Preset` and the import errors
  (developer-facing `console.warn`/`Error` strings, not `msg`s).
- **Port target**: `gpui/src/features/auto_support_settings.rs` — the same store modelled
  as `LazyLock<Mutex<DialogState>>` with `Preset`, `built_in_presets()`,
  `unique_preset_name`, `DocumentState`, and its own JSON document
  (`auto-support-presets.json`, `schemaVersion: 1`, atomic tmp+rename, corrupt-file
  quarantine). Its PORT DEVIATION 11 explains why it does not use the three
  `gpui_store` documents; the handoff's "Known open items" says to promote it to a
  fourth `gpui-store` document later.

## Architecture — how the pieces connect

```
useIslands (page.tsx)  ──props──▶  AutoSupportPanel
                                     │  draft (useState)
                                     │  run ─▶ runAutoPlaceInWorker ─▶ commitAutoPlacePlan
                                     │           └─ setAutoSupportProgress ──▶ page BlockingOverlay
                                     ├─▶ AutoSupportPresetRow ─▶ AutoSupportPresetSelect
                                     ├─▶ AutoSupportRunDiagnostics ─▶ Forest Report modal
                                     ├─▶ settings portal ─▶ AutoSupportSettingsBody
                                     │                        ├─ AutoSupportPresetSelector
                                     │                        ├─ FieldCard ×5 (+ NumberField/ToggleField/AutoSupportSizingTierField)
                                     │                        ├─ DiagnosticsCard
                                     │                        ├─ calibration card
                                     │                        └─ AutoSupportSettingsFooterActions
                                     ├─▶ Discard Changes modal
                                     └─▶ Existing Supports Detected modal

autoSupportPresets store ──(subscribe/notify)──▶ panel + dialog
        └─ writes ─▶ Settings/state (live `autoSupport`) ──▶ subscribeToSettings ──▶ panel/dialog
        └─ localStorage: auto-support-presets-v1, auto-support-active-preset-id-v1
Settings/state also persists the live block in the support-settings document.

cross-area dependency: Support Studio presets (src/supports/Settings/presets.ts) are
read only by AutoSupportSizingTierField for the tier list and names.
```

The page also consumes this module for the full-screen modal: `page.tsx:270` imports
`getAutoSupportBusy/subscribeAutoSupportBusy/autoSupportDrivingScan/getAutoSupportProgress/subscribeAutoSupportProgress`,
subscribes at `:7438-7439`, blocks Escape at `:7455-7456`, suppresses the island-scan
overlay while `autoSupportDrivingScan` (`:11911`) and renders
`BlockingOverlay title="Generating Supports"` at `z-[122]` with detail
`islandsPoc.scanning ? 'Scanning islands & minima…' : 'Placing and bracing supports…'`,
footnote `Processing 1 model`, and a progress bar fed by either the island scan's
progress or `autoSupportProgress`.

---

## Deliberate decisions and gotchas

1. **Only the panel's run reads `getSettings()`, never the draft.** A run uses the
   applied policy; the dialog's unsaved edits cannot leak into it. Any port that runs
   from the dialog draft is wrong.

2. **`presets`/`activeId`/`dirty`/`live` are four separate `useSyncExternalStore`
   subscriptions on purpose.** The source comment:
   `React Compiler treats a bare call to an imported function as pure and evaluates it once, which would freeze the selector at whatever the store held when the panel mounted.` Do not "simplify" to one subscription that returns an object — that also changes re-render timing.

3. **The active preset id is a store fact, never inferred.** `readStoredState` leaves
   `activeId: null` until the user picks one:
   `Inferring it from the live settings — as the trunk preset store does — would claim the user chose a preset they never picked, and the first knob edit would then read as a silent rewrite of it.`

4. **Selecting a preset applies it immediately.** The store's contract — `setActive…`
   calls `applyPresetSettings` — so the dialog copies the applied block into its draft
   instead of staging a selection:
   `Selecting a preset applies it to the live settings. That is the store's contract, so it cannot be staged in the dialog's draft like a knob edit — instead the applied block is copied back into the draft.`

5. **`isAutoSupportPresetDirty()` normalises both sides before comparing.** Raw
   comparison marked every preset dirty the moment a key was added or renamed
   (`sizingPreset → sizingBand`). Unknown imported keys must not dirty a preset either,
   because the settings store cannot apply them.

6. **Diagnostics are not policy.** `debugSupportOriginColors` and `debugSkipAutoBracing`
   (a) apply the instant they are toggled, (b) are excluded from every dirty check, and
   (c) stay enabled while a built-in is selected. `debugSimpleSupportRender` and
   `debugMode` are not even `autoSupport` keys — the first is top-level
   `SupportSettings`, the second panel session state.

7. **The dialog's Save does not close the dialog.** It acknowledges on the button for
   2000 ms (`Saved!` + success tint). A port that closes on save breaks the design
   rationale (`the preset list behind the dialog is where a save shows`).

8. **`Save` is disabled for a built-in even though the store refuses it anyway.** Two
   guards exist; the store's refusal is the real one and its message is a `console.warn`.

9. **Delete falls back to `medium`.** The store's contract is "nothing selected";
   `deleteActiveAutoSupportPreset` then calls `selectAutoSupportPreset('medium', …)`,
   which *applies* the Medium block. This is UI policy, not store policy.

10. **Duplicate moves the selection but does not re-apply the block**, so a staged edit
    survives. The copy therefore reads modified — `The copy then reads as modified, which is the truth`.

11. **`commitAutoSupportSettings` reads the dirty flag after writing.** The header says:
    `The "do they differ" test is read after the write, on purpose. It used to be read from the pre-write block, which is answerable only for the drift the live settings had before the draft landed — never for the draft itself.` Swapping the order is the bug it fixes.

12. **A built-in's stored record may carry saved settings but never identity.**
    `readStoredState` adopts only `settings`/`updatedAt` for reserved ids. Practically:
    `saveAutoSupportPreset` refuses a built-in, so the factory block survives every
    load; `restoreAutoSupportFactoryDefaults` exists for the case a stored record does
    carry a user block.

13. **The dialog's numeric fields and the store disagree on two bounds.**
    `autoSupportPanelTabs.KNOBS` gives `minIslandAreaMm2` `max 2` and
    `leafFanRadiusMm` `min 2`, while `AUTO_SUPPORT_CONSTRAINTS` clamps to `max 10` and
    `min MIN_LEAF_FAN_RADIUS_MM = 8`. `normalizeFieldValue` uses the *knob's* range, so a
    typed `3` for Fan Reach displays as `3`, and the settings store then clamps it to `8`
    on write — the field and the applied value disagree until the next read.
    Reproduce the source exactly or fix both sides deliberately; do not silently pick one.

14. **`color-mix` direction.** `color-mix(in srgb, A, B 86%)` is `A*0.14 + B*0.86`. Every
    inline style here is written in the CSS direction; the Rust port must invert into
    `mix(a, b, w) = a*w + b*(1-w)` (handoff decision 3). Example already in the port:
    `mix(ACCENT, SURFACE_0, 0.14)` for `color-mix(accent, surface-0 86%)`.

15. **`AUTO_SUPPORT_SECTION_CARD` is the panel's surface-1 inset card**, whereas
    `FIELD_CARD_STYLE` is the dialog's surface-2 field card. Two different tokens for
    two different greys; swapping them is invisible in a diff and obvious on screen.

16. **The calibration card's warning tint is amber `#d97706`, not a CSS variable.**
    `color-mix(in srgb, #d97706, var(--border-subtle) 36%)` and
    `color-mix(in srgb, #d97706, var(--surface-1) 92%)`. There is deliberately **no**
    callout text — the test asserts the dialog does not contain `measured`,
    `Reset to measured defaults` or `Calibration, not preferences`.

17. **The panel's dropdown has no modified star; the dialog's does.** `*` rides on the
    active row's label in the dialog only, together with `aria-label="Auto-support preset, modified"`.

18. **The panel's preset `onSelect` writes the panel's `draft`, which nothing renders.**
    `selectAutoSupportPreset(id, setDraft)` mutates panel state that only
    `openSettings` re-reads (and it re-seeds from the store anyway). Harmless, but a
    port should not conclude the panel's draft is a preview.

19. **`activeHint` exists only for built-ins.** `TIER_HINTS[activePresetId]` is
    `undefined` for a custom preset, so the trigger falls back to the generic
    `Apply this preset to the auto-support settings`.

20. **The panel's hint condition is `!hasGeometry`, and the counts' ellipsis is
    `islands.scanning`.** Two unrelated conditions; the port currently conflates them
    (`when(count == 0)` for the hint).

21. **The run has three busy flags to keep in step**: local `busy` (button label), the
    module `_autoSupportBusy` (page overlay + Escape block), and `autoSupportDrivingScan`
    (suppresses the island-scan overlay while the auto run owns the scan). All three are
    set in `doRun`, cleared in the `.finally`s and in the deferred effect. Missing the
    `if (!pendingRef.current)` guard in the Replace path leaves the overlay up after a
    scan-then-place run.

22. **History has exactly one entry per Replace, and only if something was removed**
    (`if (removeSupportEntitiesForModel(activeModelId) > 0)`), with label
    `Replace supports`. The "Add to existing" path pushes none of its own.

23. **`getCavityGeometry` must be a getter prop, not a captured geometry** — the comment:
    `Resolve the model's cavity (void boundary) at run time. Read after onBeforeRun has applied any pending hollowing, so it must be a getter rather than a prop value captured when the panel rendered.`

24. **`onBeforeRun` returning false abandons the run silently** — no toast, and `busy`
    was never set, so the panel just stays idle.

25. **Sizing tier is derived, not stored data.** `sizingPreset` is an open id resolved
    against Support Studio every run; a deleted id falls back to the `structure` band
    rather than failing. `sizingTierName` mirrors that fallback for row labels.

26. **The cache in `useSupportStudioPresets` is load-bearing.** `getPresetList()` returns
    a new array each call; without the per-notification cache the control render-loops.

27. **Export filename slug**: lowercase, every run of non-`[a-z0-9]` → `-`, edges trimmed,
    empty → `auto-support-preset`, extension `.dragonfruit-auto-support.json`. Cancels
    (`message.toLowerCase().includes('cancel')`) are not errors.

28. **The preset format is unversioned inside the payload but versioned in the document.**
    `kind` + `formatVersion` are hard-rejected if unknown — no migration attempt
    (`guessing at a format this build does not know is how a preset silently becomes the wrong policy`). Unknown *setting* keys, by contrast, are preserved.

29. **`structuredClone` on every snapshot** — presets never share a settings object with
    the live block or with each other (asserted by the store test).

30. **The port's existing modules are the authority for what is already done.** Read
    `gpui/src/features/auto_support_settings.rs`'s header (13 deviations) and
    `gpui/src/panels/content.rs:598-800` (the body's PORT GAPs) before writing code;
    do not re-derive what those comments already record.

---

## Open questions

1. **No island scanner in the port.** The panel's counts and the scan-then-place run
   (`pendingRef` + `onRunScan`) depend on `useIslands`, which has no GPUI equivalent
   listed in the handoff. Until it exists, `Voxel`/`Minima` cannot be real and the
   scan-drive branch has nothing to call.

2. **No auto-support engine in the port.** `runAutoPlaceInWorker`, `SizingDebugInfo`,
   `ForestReport` and `forestReportToText` are TS-only; `panels/content.rs` calls its
   run button a "placeholder generator (W6 replaces it with the real `autoSupport`
   engine)". The diagnostics/forest surfaces therefore have no data source yet.

3. **Export/Import need a native file bridge.** The source uses
   `savePrintArtifactWithNativeDialog` / `pickOpenFilesWithNativeDialog` /
   `readPrintArtifactBytesFromPath`; the port has no equivalent wired for this dialog
   (PORT DEVIATION 3). The JSON format itself is fully specified above and in
   `docs/reference/auto-support-preset-format.md`.

4. **Text entry in the name dialog.** The source uses a real `<input>` with Enter-to-confirm;
   the port renders a static value (PORT DEVIATION 2) even though `gpui-ui` ships
   `text_field.rs`/`EntityInputHandler`. Unclear whether the omission was a scope choice
   or a gap at the time it was written.

5. **`restoreAutoSupportFactoryDefaults` has no caller.** Whether the port should expose
   it at all (e.g. in a future menu) is undecided; the dialog deliberately has no
   factory-restore action (the test asserts `Restore factory presets` is absent).

6. **Which bounds are canonical** — the catalogue's field ranges or
   `AUTO_SUPPORT_CONSTRAINTS` (gotcha 13). The source ships both; the port must choose
   and say so, because the field's displayed value and the applied value can differ for
   `Fan Reach` and `Min Island Size`.

7. **Auto-Lift ownership.** The handoff's "Known open items" records that
   `tool_panels::PanelStore.transform.auto_lift` still keeps its own copy and should
   delegate to `auto_support_settings::{auto_lift, set_auto_lift}`; the source's flag is
   per-model and lives in the scene document, which the port does not have.
