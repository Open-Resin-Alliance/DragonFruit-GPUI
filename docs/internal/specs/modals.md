# Modals

Every dialog in the editor: the shared dialog shell, the 17 dialogs under
`src/components/modals/`, and the five modal "organisms" under
`src/components/organisms/modals/` that the page mounts once and drives with props.
Each surface records its trigger, its exact markup and styling, its state branches, and
what each button does to the store.

## Components in scope

| Component | File |
|---|---|
| `DiagnosticsModal` | `src/components/modals/DiagnosticsModal.tsx` |
| `HistoryDebugModal` | `src/components/modals/HistoryDebugModal.tsx` |
| `IslandHierarchyModal` | `src/components/modals/IslandHierarchyModal.tsx` |
| `ModelSupportsModal` | `src/components/modals/ModelSupportsModal.tsx` |
| `SceneMaterialReassignModal` | `src/components/modals/SceneMaterialReassignModal.tsx` |
| `ScenePluginMissingModal` | `src/components/modals/ScenePluginMissingModal.tsx` |
| `ScenePrinterMismatchModal` | `src/components/modals/ScenePrinterMismatchModal.tsx` |
| `ZipFilePickerModal` | `src/components/modals/ZipFilePickerModal.tsx` |
| `GpuPerformanceWarningModal` | `src/components/modals/GpuPerformanceWarningModal.tsx` |
| `SliceCompletedModal` | `src/components/modals/SliceCompletedModal.tsx` |
| `UvToolsLaunchingModal` | `src/components/modals/UvToolsLaunchingModal.tsx` |
| `PrintingResliceModal` | `src/components/modals/PrintingResliceModal.tsx` |
| `ManifoldWarningModal` | `src/components/modals/ManifoldWarningModal.tsx` |
| `DestructiveTransformModal` | `src/components/modals/DestructiveTransformModal.tsx` |
| `TetrahedralRemeshConfirmModal` | `src/components/modals/TetrahedralRemeshConfirmModal.tsx` |
| `ShrinkWrapConfirmModal` | `src/components/modals/ShrinkWrapConfirmModal.tsx` |
| `ObsoleteVoxlVersionModal` | `src/components/modals/ObsoleteVoxlVersionModal.tsx` |
| benchmark event protocol | `src/components/modals/diagnosticsBenchmarkEvents.ts` |
| `DiagnosticsModals` | `src/components/organisms/modals/DiagnosticsModals.tsx` |
| `SceneFileModals` | `src/components/organisms/modals/SceneFileModals.tsx` |
| `ModifierModals` | `src/components/organisms/modals/ModifierModals.tsx` |
| `MeshRepairModals` | `src/components/organisms/modals/MeshRepairModals.tsx` |
| `PrintingModals` | `src/components/organisms/modals/PrintingModals.tsx` |
| dom test | `src/components/organisms/modals/__tests__/meshRepairModals.dom-test.tsx` |

Referenced but **out of scope** (other areas own them; name only): `StructuredDialogModal`
(`src/components/ui/StructuredDialogModal.tsx`), `SelectDropdown` (`src/components/ui/SelectDropdown.tsx`),
`MeshRepairConfirmModal` / `MeshRepairReportModal` (`src/components/scene/`),
`SceneAutosaveRecoveryModal` (`src/components/scene/`), `SurfacingProgressModal` (`src/components/scene/`),
`SliceMetricsDebugModal` (`src/features/slicing/components/`), `RtspRelayCanvasPlayer`
(`src/components/monitoring/`), and the atoms `Button`, `IconButton`, `IconChip`, `ProgressBar`,
`Spinner`, `BlockingOverlay` (`src/components/atoms/`).

---

## Shared foundation

The surfaces below are written against these shared pieces; a porter who gets them wrong gets all
twenty-two surfaces wrong in the same way.

### Surface: `StructuredDialogModal` — the standard dialog shell

- **Source**: `src/components/ui/StructuredDialogModal.tsx:24`
- **Renders**: `createPortal` into `document.body` of a backdrop div > panel div > header (icon slot,
  title, subtitle, close X) + body div (children, then an actions row).
- **Layout & styling (verbatim)**:
  - backdrop: `` `fixed inset-0 ${zIndexClassName} flex items-center justify-center bg-black/55 backdrop-blur-sm px-3` ``
  - panel: `` `w-full ${maxWidthClassName} overflow-hidden rounded-xl border shadow-2xl ${panelClassName}` ``
    with inline `background: 'var(--surface-0)'`, `borderColor: 'var(--border-subtle)'`,
    `boxShadow: '0 24px 46px rgba(0,0,0,0.42)'`, `role="dialog"`, `aria-modal="true"`, `aria-label={ariaLabel}`.
  - header: `flex items-center justify-between gap-4 border-b px-5 py-4`, inline `borderColor: 'var(--border-subtle)'`.
  - icon slot: `inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-md border`, inline style
    `ICON_TONE_STYLES[iconTone]` (`iconTone` defaults to `'warning'`).
  - title: `text-base font-semibold leading-tight`, inline `color: 'var(--text-strong)'`.
  - subtitle: `mt-0.5 text-[11px] leading-snug`, inline `color: 'var(--text-muted)'`.
  - close: `IconButton variant="surface" size="md"` containing `<X className="w-4 h-4" />`.
  - body: `bodyClassName` default `'space-y-4 p-5'`; actions wrapper `actionsClassName`
    default `'grid grid-flow-col auto-cols-fr gap-2 pt-1'` (equal-width buttons in one row).
  - Defaults: `zIndexClassName='z-[100]'`, `maxWidthClassName='max-w-lg'`, `closeAriaLabel='Close dialog'`.
- **States**:
  - closed: `if (!open) return null` — nothing is portalled and the Escape registration is inactive.
  - close affordance: the X is rendered only when `onClose` is defined. A dialog that passes only
    `onBackdropClick` has **no X**.
  - `closeDisabled`: X is disabled and Escape is *swallowed* rather than closing
    (`escapeClose = closeDisabled ? undefined : (onClose ?? onBackdropClick)`).
- **Closing semantics**: backdrop `onMouseDown` fires only when `event.target === event.currentTarget`;
  it calls `onBackdropClick` if present, else `onClose?.()`. Escape closes via `useEscapeToClose`.
- **Text**: none of its own except the default `closeAriaLabel` `'Close dialog'`.
- **Port target**: a shared dialog primitive. `gpui/src/features/` does not exist in this worktree
  (there is no `gpui/` directory, and `docs/internal/gpui-port-handoff.md` is absent), so **nothing
  exists yet**; proposed `gpui/src/components/structured_dialog.rs`.

### `ICON_TONE_STYLES` — icon slot tones

- **Source**: `src/components/atoms/iconTone.ts:12`
- `warning`: `borderColor: 'color-mix(in srgb, #d97706, var(--border-subtle) 50%)'`,
  `background: 'color-mix(in srgb, #d97706, var(--surface-1) 85%)'`, `color: '#d97706'`.
- `danger`: `borderColor: 'color-mix(in srgb, #ef4444, var(--border-subtle) 55%)'`,
  `background: 'color-mix(in srgb, #ef4444, var(--surface-1) 88%)'`, `color: 'var(--danger)'`.
- `accent`: `borderColor: 'color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 45%)'`,
  `background: 'color-mix(in srgb, var(--accent-secondary), var(--surface-1) 90%)'`,
  `color: 'var(--accent-secondary)'`.
- `neutral`: `borderColor: 'var(--border-subtle)'`, `background: 'var(--surface-1)'`,
  `color: 'var(--text-muted)'`.
- Note the mismatch that trips porters: `iconTone="accent"` is **`--accent-secondary`**, not `--accent`.

### Escape stack

- **Source**: `src/hotkeys/useEscapeToClose.ts:1`
- Each open dialog pushes an entry; `Escape` reaches only the **last** entry (the last mounted = the
  top-most visually), and that entry calls `event.stopImmediatePropagation()` so nothing behind it
  (another dialog, the canvas tools) reacts to the same press.
- An entry registered with **no handler swallows Escape**: this is how non-dismissible dialogs keep
  the key from leaking to the surface behind. Several surfaces below rely on it.

### Button / IconButton / ProgressBar / BlockingOverlay contract

- `Button` variants used here: `secondary`, `tinted-accent`, `tinted-danger`; plus inline-styled
  buttons that pass no variant (`Button` defaults to `variant="secondary"`, `size="md"` →
  `'!h-9 px-3 text-xs'`).
- `IconButton` variants used: `surface` (bordered tile, `size` md = `h-9 w-9`, sm = `h-8 w-8`),
  `solid` (`ui-button` tile), plus `!p-1.5` overrides for the small inline icon buttons.
- `ProgressBar` `size` maps to track heights `xs: h-1.5`, `sm: h-2`, `md: h-2.5`; `indeterminate` uses
  `ui-loading-track` + `ui-loading-indicator` with
  `background: 'linear-gradient(90deg, var(--accent), #ff79c6)'`.
- `BlockingOverlay` is a `role="status" aria-live="polite" aria-busy="true"` sheet with **no close
  affordance**; panel `background: 'color-mix(in srgb, var(--surface-0), black 10%)'`, default width
  `w-[min(520px,92vw)]`, default `position="absolute"`.

### Z-index ladder actually used by these dialogs

| z | Surfaces |
|---|---|
| `z-50` | `IslandHierarchyModal` |
| `z-[70]` | `DiagnosticsModal`, `HistoryDebugModal` |
| `z-[80]` | `ModelSupportsModal` |
| `z-[100]` | `StructuredDialogModal` default — the three modifier-reset dialogs and the unsaved-changes dialogs when they do not override it |
| `z-[120]` | arrange `BlockingOverlay`, printing target picker |
| `z-[121]` | printing upload dialog, modifier-apply `BlockingOverlay` |
| `z-[130]` | GPU warning, printer mismatch, plugin missing, material reassign, printing reslice, obsolete VOXL, slice completed, tetrahedral/shrink-wrap confirms, manifold warning |
| `z-[140]` | printer monitor |
| `z-[150]` | UVTools launching |
| `z-[170]` | monitor debug overlay, RTSP debug overlay |
| `z-[220]` | scene import placement/hollowing prompts, unsaved-changes dialogs, plugin-import warning, save choice, manual mesh repair, monitor pending confirmation, pre-slice print confirm |
| `z-[230]` | ZIP file picker |

Note the two dialogs that override the shell default: `UnsavedChangesDialog` passes `z-[220]`, and the
modifier dialogs keep `z-[100]`.

---

## Dialogs: `src/components/modals/`

### Surface: `GpuPerformanceWarningModal` — one-shot WebView2 GPU warning

- **Source**: `src/components/modals/GpuPerformanceWarningModal.tsx:24`
- **Trigger**: self-starting. Mounted unconditionally in `src/app/page.tsx:11763` as
  `<GpuPerformanceWarningModal />`. On mount: bail unless `isWindowsWebview()`
  (`/windows/i.test(navigator.userAgent)`); bail if `localStorage.getItem('df.gpu-performance-warning.v1.dismissed') === '1'`
  (the constant is `DISMISS_KEY = 'df.gpu-performance-warning.v1.dismissed'`, comment:
  `/** Persisted only when the user asks not to see this again. */`); read `readGpuRendererName()`;
  bail when `classifyGpuRenderer(name)` is neither `'integrated'` nor `'software'`.
- **Renders**: `StructuredDialogModal`, title + warning body + a numbered how-to card + an optional
  detected-renderer card.
- **Layout & styling**: `zIndexClassName="z-[130]"`, `maxWidthClassName` default `max-w-lg`,
  `iconTone="warning"`, `icon={<AlertTriangle className="h-4 w-4" />}`, default body/actions.
  - body paragraph: `text-sm leading-relaxed`, inline `color: 'var(--text-muted)'`.
  - how-to card: `rounded-lg border p-3.5`, inline `borderColor: 'var(--border-subtle)'`,
    `background: 'var(--surface-1)'`; eyebrow `text-[10px] font-semibold uppercase tracking-wide`;
    list `mt-2 list-decimal space-y-1.5 pl-4 text-sm leading-relaxed`; trailing note
    `mt-2 text-[11px] leading-snug`.
  - renderer card (only when a renderer string was found): `rounded-md border px-3 py-2`, eyebrow
    `text-[10px] font-semibold uppercase tracking-wide`, value `mt-0.5 font-mono text-[11px] leading-snug break-words`
    with `color: 'var(--text-strong)'`.
- **States**: open/closed as above; renderer card present vs absent; no loading/error state.
- **Data**: own `useState` `open`, `renderer`; reads `localStorage`, `navigator.userAgent`, and a
  throwaway WebGL canvas.
- **Callbacks**: `dismiss` = `setOpen(false)`; `dismissForever` = write `'1'` to `localStorage`
  (failures ignored: `// Ignore storage failures; the dialog still closes for this session.`) then close.
  Both the X and the backdrop call `dismiss`.
- **Text**: title/aria `Graphics performance warning`; subtitle `DragonFruit may run slowly`;
  body `Windows is rendering DragonFruit on an integrated or software GPU. If this computer also has a discrete graphics card, Windows may not be using it — the viewport can feel much slower than it should.`;
  eyebrow `Use the faster GPU`; steps `Open **Settings → System → Display → Graphics**.`,
  `Add `msedgewebview2.exe` as a **Desktop app**.`, `Set it to **High performance**, then restart DragonFruit.`;
  note `This setting applies to every app that uses WebView2.`; renderer eyebrow `Detected renderer`;
  buttons `Don't show again` and `Got it`. Either button closes; only "Don't show again" persists.
- **Port target**: `gpui/src/features/app_shell/gpu_warning_modal.rs` — nothing exists yet.

### Surface: `ScenePrinterMismatchModal` — scene written for another printer

- **Source**: `src/components/modals/ScenePrinterMismatchModal.tsx:56`
- **Trigger**: `SceneFileModals` renders it whenever `scene.printerMismatch` is set
  (`src/components/organisms/modals/SceneFileModals.tsx:360`), after `useSceneCollectionManager`
  raised the prompt during import.
- **Renders**: `StructuredDialogModal` with a variant-selected title/subtitle/actions and a body of
  conditional paragraphs.
- **Layout & styling**: `iconTone="warning"`, `icon={<AlertTriangle className="h-4 w-4" />}`,
  `zIndexClassName="z-[130]"`, `closeAriaLabel={_(msg`Close modal`)}`, `onClose={onKeep}`,
  `onBackdropClick={onKeep}` (so the X and backdrop both mean "keep"), default max width `max-w-lg`.
  Body prose paragraphs are all `text-sm leading-relaxed` with `color: 'var(--text-muted)'`;
  emphasised printer names are `<strong className="text-sm font-medium" style={{ color: 'var(--text-strong)' }}>`.
- **States** — computed booleans:
  - `pluginMissing = Boolean(missingPluginName?.trim())`
  - `losesMaterialAssignments = sceneUsesMultiTankMaterials && currentMaterialMode !== 'multiTank'` (comment:
    `// The scene was authored with per-tank material assignments and the profile` /
    `// selected now has one material: keeping it throws those assignments away.`)
  - `lacksRequiredMaterials = !pluginMissing && !currentHasRequiredMaterials` (`currentHasRequiredMaterials` defaults `true`)
  - `buildVolumesMatch` = all three axes equal within `Math.abs(left - right) < 1e-6` (`sameAxis`).
  - `recordedLabel = recordedPrinterName?.trim() || _(msg`the printer this scene was built for`)`.
  - Body branches: `pluginMissing` → two fixed paragraphs; otherwise `lacksRequiredMaterials`
    paragraph, then `losesMaterialAssignments` paragraph, then — only `if (!buildVolumesMatch)` —
    four paragraphs including the `currentIsSmaller` / `willAddPrinter` variants.
- **Actions** (three mutually exclusive sets):
  1. `pluginMissing`: optional `Manage Plugins` (`variant="secondary"`, only when `onManagePlugins`),
     then `Import Geometry` (`variant="tinted-accent"`, class
     `inline-flex items-center justify-center gap-1.5`).
  2. `lacksRequiredMaterials`: `Load without materials` (secondary) + `Switch to {recordedLabel}` (tinted-accent).
  3. otherwise: `Keep {currentPrinterName}` (secondary) + `Switch to {recordedLabel}` (tinted-accent).
- **Data**: props only — `isOpen`, `recordedPrinterName?`, `recordedBuildVolumeMm {width,depth,height}`,
  `currentPrinterName`, `currentBuildVolumeMm`, `willAddPrinter`, `currentIsSmaller`,
  `currentHasRequiredMaterials?`, `sceneUsesMultiTankMaterials?`, `currentMaterialMode?`,
  `missingPluginName?`, `onManagePlugins?`, `onSwitch`, `onKeep`.
  In `SceneFileModals` these come from `scene.printerMismatch`; `willAddPrinter` is
  `scene.printerMismatch.installedProfileId === null`; `onManagePlugins` is passed only when
  `printerMismatch.missingPlugin` exists and resolves `'keep'` then calls `openSettingsModal('plugins')`.
- **Callbacks**: `onSwitch` → `scene.resolvePrinterMismatch('switch')`; `onKeep` → `'keep'`.
  Close/backdrop/X → `'keep'`.
- **Text** (verbatim): titles `Printer plugin not installed` / `Built for Multi-Material` /
  `Built for another printer`; subtitles `Only the geometry can be imported` /
  `The selected printer is missing the scene's materials` / `This scene is made for a multi-material printer` /
  `This scene may not fit the one selected` / `Its plates are spaced for that printer`.
  Plugin-missing body: `This scene was saved for {recordedLabel}, whose printer and material profiles come from the {missingPluginName} plugin. That plugin is not installed, so those profiles cannot be restored.`
  and `Only the geometry will be imported. Install the plugin, then reopen the scene, to restore its printer and materials.`
  Missing-materials body: `{currentPrinterName} does not have the materials this scene uses. Load the geometry without them, or switch to {recordedLabel} to restore them.`
  Multi-tank body: `Its materials are split across several tanks, which the printer selected now cannot print. The assignments stay with the scene, hidden while this printer is selected — switch to a multi-material printer to print them.`
  The four `buildVolumesMatch === false` paragraphs read (with `{w} × {d} × {h} mm` interpolations):
  `{recordedLabel} has a build volume of …`, then either
  `The printer selected now, {currentPrinterName}, has a build volume of …. A plate packed for the larger machine will not fit.`
  or `… and this scene was laid out for the other one.`, then either
  `It is not in your printer library, so switching adds {recordedLabel} from this scene.` or
  `Switch to {recordedLabel} before importing?`.
- **Port target**: `gpui/src/features/scene_io/printer_mismatch_modal.rs` — nothing exists yet.

### Surface: `ScenePluginMissingModal` — scene's printer plugin absent

- **Source**: `src/components/modals/ScenePluginMissingModal.tsx:22`
- **Trigger**: `SceneFileModals` always mounts it with `prompt={scene.missingPrinterPluginPrompt}`
  (`SceneFileModals.tsx:346`); open iff the prompt is non-null.
- **Renders**: `StructuredDialogModal` with two paragraphs.
- **Layout & styling**: `iconTone="warning"`, `zIndexClassName="z-[130]"`,
  **`maxWidthClassName="max-w-lg"` (explicit, though it equals the default)**,
  `closeAriaLabel={_(msg`Close modal`)}`, `onClose={onImportGeometry}`,
  `onBackdropClick={onImportGeometry}`. Paragraphs `text-sm leading-relaxed`,
  `color: 'var(--text-muted)'`.
- **States**: open iff `Boolean(prompt)`; printer name falls back to
  `_(msg`the printer this scene was built for`)`; plugin name is `{prompt?.pluginName}` (undefined-safe).
- **Callbacks**: `Manage Plugins` → `onManagePlugins` (caller resolves `'manage'` then opens the
  plugins settings tab); `Import Geometry`, X, backdrop, Escape → `onImportGeometry`
  (resolves `'geometry'`).
- **Text**: aria/title `Printer plugin not installed`; subtitle `Only the geometry can be imported`;
  paragraph 1 `This scene was saved for {printerName}, whose printer and material profiles come from the {pluginName} plugin. That plugin is not installed, so those profiles cannot be restored.`;
  paragraph 2 `Install the plugin, then reopen the scene, to restore its printer and materials.`;
  buttons `Manage Plugins`, `Import Geometry`.
- **Port target**: same module as the mismatch modal (`printer_mismatch_modal.rs`).

### Surface: `SceneMaterialReassignModal` — remap materials after a printer switch

- **Source**: `src/components/modals/SceneMaterialReassignModal.tsx:47`
- **Trigger**: page-level. `src/app/page.tsx:11716` renders it with
  `open={Boolean(materialReassignPrompt)}`; the prompt is set by the effect at `page.tsx:990-1094`
  when the active printer changes and the scene references materials that do not resolve on the new
  printer (`mode: 'reassign'`) or resolve but are not assigned to tanks (`mode: 'tankMatch'`).
- **Renders**: `StructuredDialogModal` > intro paragraphs > one of three tables (tank-match table,
  reassign table of `SelectDropdown`s, or a read-only "N/A" table plus hint).
- **Layout & styling**: `iconTone="warning"`, `icon={<Palette className="h-4 w-4" />}`,
  `zIndexClassName="z-[130]"`, `maxWidthClassName="max-w-xl"`,
  `closeAriaLabel={_(msg`Close material reassignment`)}`, `onClose`/`onBackdropClick` = `onKeepPrevious`.
  - intro block: `space-y-1.5 text-sm leading-relaxed`, `color: 'var(--text-muted)'`.
  - tables: wrapper `overflow-hidden rounded-md border` + `borderColor: 'var(--border-subtle)'`;
    `table` = `w-full border-collapse text-left text-xs`; `thead` inline
    `background: 'var(--surface-1)', color: 'var(--text-muted)'`; header cells `p-2 font-medium`
    (plus `break-words` in the two name headers); body rows `border-t` with
    `borderColor: 'var(--border-subtle)'`; row label cell `p-2 font-normal`.
  - swatch: `h-3 w-3 rounded-[3px]`, inline `background: row.color`,
    `boxShadow: 'inset 0 0 0 1px rgba(0,0,0,0.3)'`, `aria-hidden="true"`.
  - reassign dropdown cell: `<SelectDropdown className="w-full" selectClassName="w-full h-[32px] px-2 pr-8 text-xs" ariaLabel={row.name} …>`.
  - hint line: `text-xs leading-relaxed`, inline `color: '#f59e0b'` (a raw hex, not a token).
- **States**:
  - `isTankMatch = mode === 'tankMatch'`.
  - Title/aria: `Assign scene materials to tanks` when tank-match, else `Reassign scene materials`.
  - Subtitle: tank-match → `The new printer shares these materials but its tanks are not assigned`;
    else `canReassign` → `The new printer has different material profiles`; else
    `The new printer has no matching materials`.
  - Actions: always `Keep {previousPrinterName}` (secondary); plus
    `Load without materials` (secondary) only when `!canReassign && !(isTankMatch && canAssignTanks)`;
    plus `Reassign materials` (tinted-accent) when `canReassign`; plus
    `Assign to matching tanks` (tinted-accent) when `isTankMatch && canAssignTanks`.
  - Body: tank-match table when `isTankMatch`; editable table when `canReassign`; otherwise the
    N/A table + two hint paragraphs (second uses `#f59e0b`).
  - Missing/unavailable wording depends on `rows.length === 1`
    (`This material is not available on {newPrinterName}.`) vs plural
    (`None of these materials are available on {newPrinterName}.`).
- **Data / state**: local `mapping: Record<string, string>`, initialised to `options[0]?.id ?? ''`
  for every row, and **re-initialised by an effect on every `[open, options, rows]` change while open**.
  `handleApply` builds `Map<rowId, SceneMaterialReassignOption>` containing only rows whose current
  selection resolves to an option in `options` (a row pointing at nothing is silently dropped).
- **Callbacks**: `onApply(map)`, `onUnassign()`, `onAssignTanks?.()`, `onKeepPrevious()`; X/backdrop →
  `onKeepPrevious`. In `page.tsx`: apply → `scene.reassignSceneMaterials(map)`; unassign →
  `scene.reassignSceneMaterials(new Map(), true)`; assign tanks → `updatePrinterProfile(newPrinterId, { tankMaterials })`;
  keep previous → `setActivePrinterProfile(prompt.previous.id)`.
- **Text** (verbatim): headers `Scene material`, `Use on this printer`; tank rows `Tank {row.tankNumber}`;
  unassigned cell `N/A`; intro `This scene names materials from {previousPrinterName}.`,
  `{newPrinterName} shares them, but its tanks are not assigned. Assign each material to the matching tank, or keep the previous printer.`,
  `Choose which materials on {newPrinterName} the scene should use.`;
  fallback paragraph `The new printer has no matching materials assigned to its tanks. Load without materials, or keep the previous printer to preserve them.`;
  hint `You can share materials with printers that use the same material settings and output format in Material Settings.`
- **Port target**: `gpui/src/features/profiles/material_reassign_modal.rs` — nothing exists yet.

### Surface: `ZipFilePickerModal` — choose files inside an imported archive

- **Source**: `src/components/modals/ZipFilePickerModal.tsx:86`
- **Trigger**: `SceneFileModals` renders it when `zipPickerState` is non-null; the state is set by
  `useImportExportManager` (`src/features/import-export/useImportExportManager.ts:252`) when a ZIP is
  dropped/opened and more than one candidate is found. A promise resolver
  (`zipPickerResolveRef`) is answered by confirm/cancel.
- **Renders**: **hand-written** backdrop/panel (it does *not* use `StructuredDialogModal`): header with
  `FileArchive` chip, title, "found … for …" line, close X; body with instructions, an archive info
  card, quick-select pill row, a scrollable file list of checkbox rows, and a footer with
  Cancel + Import.
- **Layout & styling**: backdrop
  `fixed inset-0 z-[230] flex items-center justify-center bg-black/55 backdrop-blur-sm px-3`,
  backdrop `onMouseDown` closes when `target === currentTarget`. Panel
  `w-full max-w-lg overflow-hidden rounded-xl border shadow-2xl` with
  `background: var(--surface-0)`, `borderColor: var(--border-subtle)`,
  `boxShadow: '0 24px 46px rgba(0,0,0,0.42)'`, `role="dialog"`, `aria-modal="true"`,
  `aria-label="Select files from ZIP archive"`. Header `flex items-center justify-between gap-4 border-b px-5 py-4`.
  Icon chip `inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-md border` with
  `borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 55%)'`,
  `background: 'color-mix(in srgb, var(--accent), var(--surface-1) 88%)'`, `color: 'var(--accent)'`.
  Body `space-y-4 p-5`. Archive card `rounded-lg border px-3 py-2.5` with
  `background: 'color-mix(in srgb, var(--surface-1), var(--surface-0) 30%)'`.
  Quick-select pills: `inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-[10px] font-semibold tracking-wide transition-colors`.
  List: `max-h-72 overflow-y-auto rounded-lg border` with
  `background: 'color-mix(in srgb, var(--surface-1), var(--surface-0) 40%)'`; rows
  `flex cursor-pointer items-center gap-3 border-b px-3 py-2.5 transition-colors`, selected row
  `background: 'color-mix(in srgb, var(--accent), var(--surface-0) 94%)'`; extension badge
  `shrink-0 rounded px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wide`; name
  `min-w-0 flex-1 truncate text-xs`; size `shrink-0 text-[10px] tabular-nums`.
  Footer `flex shrink-0 items-center justify-end gap-2 pt-1`.
- **States**:
  - Default selection: `category !== 'mixed'` → **every** file; `mixed` → only files whose lowercase
    extension is in `MESH_EXTS = {'.stl','.obj','.3mf'}` (when `defaultSelectionCategory === 'mesh'`)
    or in `SCENE_EXTS` (from `SCENE_FILE_EXTENSIONS`). Re-derived via `useMemo` and re-applied by an
    effect on `[defaultSelectedIndices]`.
  - Theme: `isLightTheme` is computed from `documentElement` classes `dragonfruit-light` /
    `dragonfruit-dark`, `data-theme="light"`, or `prefers-color-scheme: light`, and kept live with a
    `MutationObserver` (`attributeFilter: ['class','data-theme']`) plus a media-query listener.
  - Pill "ALL": selected styling when `allSelected` (`selectedIndices.size === files.length`).
  - Per-extension pills: `allOfTypeSelected` controls border/colour; `hasAnySelected` controls the
    background mix (`84%` in light theme, `88%` in dark).
  - Import button: disabled and styled muted when `selectedIndices.size === 0`
    (`opacity: 0.65`, `cursor: 'not-allowed'`, `aria-disabled`), and its label switches between
    `Import`, `Import 1 file`, `Import N files`.
  - Empty list is not handled — the list is whatever the caller passed.
- **Data**: props `zipName`, `files: File[]`, `category`, `defaultSelectionCategory`, `onConfirm`,
  `onCancel`. Local `selectedIndices: Set<number>`.
- **Callbacks**: row checkbox → `toggleFile(index)`; `ALL` pill → `toggleAll()` (all or none);
  extension pill → `toggleExtension(indices)` (if all of that type are selected, deselect them all,
  else select them all); `Import …` → `handleConfirm` → `onConfirm(files.filter(selected))`, and
  **does nothing when nothing is selected**; `Cancel`, X, backdrop, Escape → `onCancel`.
  In `SceneFileModals`, cancel answers the waiting promise with `[]`.
- **Text** (all hardcoded English, **not** i18n): title `Select Files from ZIP`; header line
  `DragonFruit found {mesh files|scene files|mesh and scene files} in this archive for {Load Mesh|Import Scene|Load Mesh / Import Scene}`;
  instructions `Choose which files to import from this archive. {All files are selected by default.|Matching mesh files are selected by default.|Matching scene files are selected by default.}`;
  `Archive`; `{n} of {total} selected`; `Quick select`; `ALL`; `Cancel`; `Import`/`Import N file(s)`;
  titles `Select all files`/`Deselect all files`/`Select all {EXT} files`/`Deselect all {EXT} files`;
  close aria-label `Cancel ZIP import`.
- **Deliberate details**: extension colours are per-theme maps —
  dark `{'.stl':'#60a5fa','.3mf':'#34d399','.obj':'#a78bfa','.voxl':'#baf72e','.lys':'#f59e0b'}`,
  light `{'.stl':'#1d4ed8','.3mf':'#047857','.obj':'#6d28d9','.voxl':'#3f6212','.lys':'#b45309'}`,
  fallback `var(--accent)`. Group order is first-seen, not alphabetical.
  `formatFileSize`: base-1024, `decimals = value >= 100 ? 0 : value >= 10 ? 1 : 2`, units
  `['B','KB','MB','GB','TB']`, and non-positive/NaN renders `0 B`.
- **Port target**: `gpui/src/features/scene_io/zip_file_picker.rs` — nothing exists yet.

### Surface: `SliceCompletedModal` — slice finished

- **Source**: `src/components/modals/SliceCompletedModal.tsx:28`
- **Trigger**: `PrintingModals` (`PrintingModals.tsx:460`) with `isOpen={showSliceCompletedModal}`.
  The page sets it in an effect (`page.tsx:3198-3209`) only when
  `completedSliceIntent === 'file' && completedSaveDestinationPath` and a finite slicing time exists;
  `filePath` is `completedSaveDirectory || completedSaveDestinationPath` (a batch names its folder).
- **Renders**: `StructuredDialogModal` + a file-location card, an optional slicing-time card, and an
  optional inline error card.
- **Layout & styling**: `iconTone="accent"`, `icon={<CheckCircle2 className="h-4 w-4" />}`,
  `zIndexClassName="z-[130]"`, `maxWidthClassName="max-w-md"`,
  `closeAriaLabel={_(msg`Close slicing finished modal`)}`, X and backdrop both `onClose`.
  Cards: `rounded-lg border p-3`, `borderColor: 'var(--border-subtle)'`,
  `background: 'var(--surface-1)'`; eyebrows `text-[11px] uppercase tracking-wide`; path text
  `text-sm font-mono break-all`; time `text-sm font-semibold mt-1`.
  Error card: `rounded-md border px-2.5 py-2 text-[11px] leading-snug` with
  `borderColor: 'color-mix(in srgb, var(--danger), var(--border-subtle) 45%)'`,
  `background: 'color-mix(in srgb, var(--danger), var(--surface-1) 92%)'`,
  `color: 'var(--danger)'`.
- **States**: `filePath === null` → shows `—` and both action buttons disabled; `slicingTimeMs === null`
  → the time card is omitted entirely; `openDirectoryError` → error card; leaving the modal clears it
  (effect on `isOpen`).
- **Actions**: when `onOpenInUvTools` is provided (i.e. UVTools enabled):
  `Open in UVTools` (tinted-accent, `ExternalLink w-4 h-4`) and `Open Directory` (tinted-accent,
  `FolderOpen w-4 h-4`), both with class `inline-flex items-center justify-center gap-1.5 disabled:opacity-45`.
  Otherwise `Close` (secondary) + `Open Directory`.
- **Data**: props `isOpen`, `onClose`, `filePath`, `slicingTimeMs`, `onOpenInUvTools?`.
  `formatTime`: null → `—`; `< 1000` → `fmt.quantity(ms, 0, 'ms')`; else
  `fmt.quantity(ms / 1000, 2, 's')`.
- **Callbacks**: `handleOpenDirectory` dynamically imports `@tauri-apps/api/core` and awaits
  `invoke('reveal_in_file_manager', { path: filePath })`; on failure stores
  `error.message` (or `Failed to open directory.`) and `console.error('Failed to open directory:', error)`.
  `onOpenInUvTools(filePath)` is only called when `filePath` is non-null; in `PrintingModals` it calls
  `launchExternalProcess(resolveUvToolsExecutablePath(getSavedUvToolsSettings()), fp)` and
  `console.warn('[UVTools] Failed to launch from completed dialog:', err)` on failure.
- **Text**: title `Slicing Finished`; aria `Slicing completed`; labels `File Location`,
  `Slicing Time`; buttons `Open in UVTools`, `Open Directory`, `Close`.
- **Port target**: `gpui/src/features/slicing/slice_completed_modal.rs` — nothing exists yet.

### Surface: `UvToolsLaunchingModal` — non-dismissible UVTools launcher

- **Source**: `src/components/modals/UvToolsLaunchingModal.tsx:23`
- **Trigger**: `PrintingModals.tsx:473` with `isOpen={uvToolsLaunchingPath !== null}`. The page sets
  `uvToolsLaunchingPath` when a slice finishes with intent `'uvtools'` and clears it after 5 s
  (`setTimeout(() => setUvToolsLaunchingPath(null), 5000)`, both on success and on failure).
- **Renders**: raw portal (no `StructuredDialogModal`): header with `ExternalLink` chip, indeterminate
  progress bar, caption, optional file card, and a spinner + "Please wait…" row.
- **Layout & styling**: `fixed inset-0 z-[150] flex items-center justify-center bg-black/55 backdrop-blur-sm px-3`,
  `role="dialog" aria-modal="true"`, and **`onPointerDown={(e) => e.stopPropagation()}`** on the
  backdrop. Panel `w-full max-w-md overflow-hidden rounded-xl border shadow-2xl` with
  `background: var(--surface-0)`, `borderColor: var(--border-subtle)`, box-shadow as elsewhere.
  Header `flex items-center gap-2.5 border-b px-4 py-3`; eyebrow `text-xs uppercase tracking-wide`;
  title `text-base font-semibold`. Body `p-4 space-y-3`; caption `text-xs text-center`.
  File card `rounded-md border p-2.5` (`var(--surface-1)`), path `text-xs font-mono break-all`,
  `title={filePath}`.
- **States**: renders only when `isOpen` and `typeof document !== 'undefined'`; the file card appears
  only when `filePath` is truthy. **No close affordance at all** — Escape is registered with
  `undefined` and therefore swallowed (`// Registering without a close handler swallows Escape instead of dismissing.`).
- **Callbacks**: none. The caller clears the path.
- **Text**: `External Tool`, `Opening in UVTools`, `Launching UVTools to inspect the sliced file…`,
  `File`, `Please wait…`; aria-label `Opening in UVTools`.
- **Port target**: `gpui/src/features/slicing/uvtools_launching_modal.rs`.

### Surface: `PrintingResliceModal` — scene modified, slice stale

- **Source**: `src/components/modals/PrintingResliceModal.tsx:21`
- **Trigger**: `PrintingModals.tsx:604` with `isOpen={showPrintingResliceModal}`. Set at
  `page.tsx:7308` inside a `PROFILE_SETTINGS_MODAL_OPEN_CHANGE_EVENT` handler after returning the app
  to `prepare` mode — i.e. changing the printer profile after slicing.
- **Renders**: `StructuredDialogModal`, one paragraph, Back / Re-Slice Now.
- **Layout & styling**: `iconTone="warning"`, `icon={<AlertTriangle className="h-4 w-4" />}`,
  `zIndexClassName="z-[130]"`, `closeAriaLabel={_(msg`Close modal`)}`, X and backdrop = `onCancel`.
- **States**: open/closed only.
- **Callbacks**: `onCancel` → `setShowPrintingResliceModal(false)` then
  `scene.setMode(modeBeforePrintingRef.current)` (restore the mode the user came from);
  `onResliceNow` → close, `shouldReturnToPrintingAfterSliceRef.current = true`,
  `setShouldAutoSliceOnExportEntry(true)`, `scene.setMode('export')`.
- **Text**: aria/title `Scene Modified`; subtitle `Please re-slice before printing`; body
  `Your model has been modified (geometry, position, rotation, scale, or supports changed). The current slice is no longer valid.`;
  buttons `Back`, `Re-Slice Now`.
- **Port target**: `gpui/src/features/printing/reslice_modal.rs`.

### Surface: `ManifoldWarningModal` — mesh is not a valid solid

- **Source**: `src/components/modals/ManifoldWarningModal.tsx:15`
- **Trigger**: `page.tsx:11755` with `isOpen={showManifoldWarning}`; set at `page.tsx:888` when the
  first model flagged non-manifold is found (`setManifoldWarningModelId(flagged[0].id)`).
- **Renders**: `StructuredDialogModal`, three paragraphs.
- **Layout & styling**: `iconTone="warning"`, `zIndexClassName="z-[130]"`,
  **`onBackdropClick={() => {}}`** and no `onClose` → no X, backdrop clicks do nothing. Comment kept:
  `// No close-X and backdrop clicks are ignored: the user leaves through` / `// Dismiss or Repair Now.`
- **States**: open/closed only.
- **Callbacks**: `Dismiss` → `setShowManifoldWarning(false)`; `Repair Now` → close, then (comment
  `// Straight into the repair: the Repair Mesh dialog opens already running.`)
  `setAutoStartRepair(true); setManualRepairModelIds([manifoldWarningModelId])`.
- **Text**: aria/title `Mesh Issue detected`; subtitle `This mesh isn't a valid solid`; body:
  `Parts of this mesh are not closed — edges are left open, faces cross through each other, or surfaces do not quite meet. Slicing and support placement expect a sealed solid, so defects can reach the print without showing up here first.`,
  `The red-striped areas in the viewport mark the parts that failed validation.`,
  `You can dismiss this and keep working, or repair the mesh now.`;
  buttons `Dismiss`, `Repair Now` (with `Wrench h-3.5 w-3.5`).
- **Port target**: `gpui/src/features/mesh_health/manifold_warning_modal.rs`.

### Surface: `DestructiveTransformModal` — transform would drop supports

- **Source**: `src/components/modals/DestructiveTransformModal.tsx:15`
- **Trigger**: `ModifierModals` renders it with `isOpen={pendingDestructiveTransform !== null}`;
  `page.tsx:2628` and `page.tsx:2660` set `pendingDestructiveTransform` when a transform is requested
  on models that already carry supports.
- **Renders**: `StructuredDialogModal` with three field cards, one explainer paragraph, and a
  two-column action row.
- **Layout & styling**: `iconTone="warning"`, `zIndexClassName="z-[130]"`,
  `closeAriaLabel={_(msg`Close warning modal`)}`, X and backdrop = `onCancel`;
  **`bodyClassName="p-4 space-y-3"`**, **`actionsClassName="grid grid-cols-2 gap-2 pt-1"`**.
  Cards: `rounded-md border px-3 py-2`, `borderColor: 'var(--border-subtle)'`,
  `background: 'var(--surface-1)'`; label `text-[11px] uppercase tracking-wide`; value
  `text-sm font-semibold` (the count also `tabular-nums`).
- **States**: `modelName === null` → shows `Unknown Model`; otherwise the name. `supportCount` is
  rendered with `toLocaleString()`. No disabled/loading state.
- **Actions**: `Cancel` (secondary) and `Delete & Continue` (`variant="tinted-danger"`,
  `Trash2 w-3.5 h-3.5`).
- **Callbacks**: `onCancel` → `handleCancelDestructiveTransform`; `onConfirm` →
  `handleConfirmDestructiveTransform` (deletes every support of the models, resets the gizmo, then
  runs the deferred action stored in `pendingDestructiveTransformContinueRef`).
- **Text**: aria/title `Destructive Transform`; subtitle `Supports will be deleted before continuing`;
  field labels `Operation`, `Model` (msgid with `context: 'Dialog field'`), `Supports Detected`;
  paragraph `This transform invalidates existing supports. If you continue, all supports for this model will be deleted.`;
  buttons `Cancel`, `Delete & Continue`.
- **Port target**: `gpui/src/features/transform/destructive_transform_modal.rs`.

### Surface: `TetrahedralRemeshConfirmModal` — confirm a remesh

- **Source**: `src/components/modals/TetrahedralRemeshConfirmModal.tsx:20`
- **Trigger**: `page.tsx:11765` with `isOpen={tetSurfaceConfirmModelId !== null}`.
- **Renders**: `StructuredDialogModal`, one paragraph + one warning box.
- **Layout & styling**: `iconTone="accent"`, `icon={<Pyramid className="h-4 w-4" />}`,
  `zIndexClassName="z-[130]"`, **`onBackdropClick={() => {}}`** and no `onClose` → no X, backdrop inert.
  Comment kept: `// No close-X and backdrop clicks are ignored: the user leaves through Cancel or Remesh.`
  Warning box: `rounded-md border px-3 py-2` with
  `borderColor: 'color-mix(in srgb, #d97706, var(--border-subtle) 40%)'`,
  `background: 'color-mix(in srgb, #d97706, var(--surface-1) 92%)'`; icon
  `AlertTriangle mt-0.5 h-3.5 w-3.5 shrink-0` in `#d97706`; text `text-xs leading-relaxed`.
- **States**: open/closed only.
- **Callbacks**: `Cancel` → `setTetSurfaceConfirmModelId(null)`; `Remesh` →
  `scene.tetSurfaceModelInPlace(targetId, TET_SURFACE_DEFAULT_OPTIONS, …)` with token + progress
  callbacks, then `SurfacingProgressModal` (out of scope) takes over.
- **Text**: aria `Remesh confirmation`; title `Remesh this model?`; subtitle `Rebuilds the surface`;
  body `This rebuilds the model's surface, keeping fine detail.`; warning
  `The rebuilt surface will replace this model in your current scene. Large models take longer, and some models cannot be rebuilt at all and are left untouched. You can undo this afterwards.`;
  buttons `Cancel`, `Remesh`.
- **Port target**: `gpui/src/features/mesh_health/remesh_confirm_modal.rs`.

### Surface: `ShrinkWrapConfirmModal` — confirm a shrink-wrap

- **Source**: `src/components/modals/ShrinkWrapConfirmModal.tsx:20`
- **Trigger**: `page.tsx` renders it with `isOpen={shrinkWrapConfirmModelId !== null}`, right after the
  tetrahedral confirm and its progress modal.
- **Renders**: same shape as the tetrahedral confirm — paragraph + warning box.
- **Layout & styling**: identical classes; `iconTone="accent"`, `icon={<RefreshCw className="h-4 w-4" />}`,
  `zIndexClassName="z-[130]"`, `onBackdropClick={() => {}}`, no X, same `#d97706` box.
- **Callbacks**: `Cancel` → clear id; `Shrink-Wrap` →
  `scene.solidRemeshModelInPlace(targetId, SOLID_REMESH_DEFAULT_OPTIONS, …)`.
- **Text**: aria `Shrink-Wrap confirmation`; title `Shrink-Wrap this model?`; subtitle
  `Reworks the surface into one solid`; body
  `This merges overlapping shells, closes gaps and drops loose pieces, leaving one solid.`; warning
  `The rebuilt surface will replace this model in your current scene. A model that cannot be resolved is left untouched, and you can undo this afterwards.`;
  buttons `Cancel`, `Shrink-Wrap`.
- **Port target**: `gpui/src/features/mesh_health/shrinkwrap_confirm_modal.rs`.

### Surface: `ObsoleteVoxlVersionModal` — VOXL V1 scene refused

- **Source**: `src/components/modals/ObsoleteVoxlVersionModal.tsx:22`
- **Trigger**: `SceneFileModals.tsx:337` renders it when `scene.obsoleteVoxlScene` is set
  (`ObsoleteVoxlScenePrompt = { fileName, detected: 'v1-json' | 'v1-binary' }`), which the scene
  manager raises during load.
- **Renders**: `StructuredDialogModal`, three paragraphs, single OK action.
- **Layout & styling**: `iconTone="warning"`, `zIndexClassName="z-[130]"`,
  `closeAriaLabel={_(msg`Close modal`)}`, X and backdrop = `onDismiss`.
- **States**: `detected === 'v1-json'` → `This file is a VOXL V1 JSON scene.`; else
  `This file is a VOXL V1 binary container.`
- **Callbacks**: OK / X / backdrop / Escape → `scene.dismissObsoleteVoxlScene()`.
- **Text**: aria `Unsupported scene version`; title `Can't open this scene`; subtitle
  `Saved by an obsolete DragonFruit version`; paragraph 1
  `{fileName} was saved in the VOXL V1 format, which this version no longer reads.`; recovery
  `To recover it, open the scene in an older version of DragonFruit and save it again — the re-saved scene will open here.`;
  button `OK`.
- **Port target**: `gpui/src/features/scene_io/obsolete_voxl_modal.rs`.

### Surface: `ModelSupportsModal` — read-only support inventory

- **Source**: `src/components/modals/ModelSupportsModal.tsx:74`
- **Trigger**: `SceneFileModals.tsx:226` with `isOpen={supportsInfoModelId !== null}`; set by
  `handleOpenModelSupportsInfo(modelId)` (`page.tsx:5689`), wired to a model-row action.
- **Renders**: hand-written backdrop/panel: header (title + model name, close X), scrollable body with
  a total card, a 2-column stat grid (one cell per support group), and an expandable ID list per group.
- **Layout & styling**: backdrop `fixed inset-0 z-[80] flex items-center justify-center bg-black/55 backdrop-blur-sm px-3`
  with `onMouseDown` backdrop close. Panel `w-full max-w-2xl max-h-[84vh] overflow-hidden rounded-xl border shadow-2xl`,
  `background: var(--surface-0)`. Header `flex items-center justify-between border-b px-4 py-3`;
  close `IconButton variant="surface" size="sm"`.
  Body `p-4 space-y-3 max-h-[68vh] overflow-y-auto custom-scrollbar`.
  Total card: `rounded-md border px-3 py-2` with
  `borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 58%)'`,
  `background: 'color-mix(in srgb, var(--accent), var(--surface-1) 92%)'`.
  Stat grid `grid grid-cols-2 gap-2`, cells `rounded-md border px-3 py-2` on `var(--surface-1)`.
  ID section `rounded-md border p-2` (`var(--surface-1)`); each group `rounded border` on
  `var(--surface-0)`; group header is a full-width `button` `flex items-center gap-2 px-2 py-1.5` with
  a `ChevronRight`/`ChevronDown w-3.5 h-3.5`; IDs are chips
  `rounded border px-1.5 py-0.5 text-[11px] font-mono` with
  `background: 'color-mix(in srgb, var(--surface-1), black 6%)'` and `title={id}`.
- **States**: all groups are **collapsed on open** (effect sets every key to `true` on
  `[isOpen, model?.id]`); clicking a group header toggles it. Empty group, while expanded, prints
  `No {groupLabel} for this model.` with `groupLabel.toLowerCase()`. `model === null` → subtitle
  `No model selected` and empty groups.
- **Data**: reads the support store through `useSyncExternalStore(subscribeSupportState, getSupportSnapshot)`;
  groups come from `getSupportsForModel(snapshot, modelId)` for every key in
  `MODEL_ID_COLLECTION_KEYS`, plus knots resolved through
  `modelIdOfParentShaft(snapshot, item.parentShaftId) === modelId` (comment:
  `// Which model a knot belongs to is which model its parent shaft belongs to.` /
  `// parentShaftId is a SEGMENT id, so the owner is resolved through the` /
  `// segment rather than by looking the id up in a collection of entities.`).
  IDs are sorted `a.localeCompare(b)`.
  `totalSupportEntities` sums **`MODEL_ID_COLLECTION_KEYS` only** — knots are excluded
  (`// Knots are excluded: they are attachments on a support, not supports.`).
  Row order: `Roots`, then each `SUPPORT_STATE_TYPES` descriptor label, then `Knots`.
- **Callbacks**: group header → toggle collapse; close X and backdrop → `onClose`
  (`setSupportsInfoModelId(null)`). No mutation.
- **Text**: aria/title `Model Supports`; close aria `Close supports modal`; `Total`;
  count message `{count, plural, one {# support entity} other {# support entities}}` (with comment
  `Total number of support entities on the model.`); empty message `No {groupLabel} for this model.`
  (comment explains the label is lowercased by the caller); section header `Support IDs by type`.
- **Port target**: `gpui/src/features/supports/model_supports_modal.rs`.

### Surface: `IslandHierarchyModal` — island parent/child tree

- **Source**: `src/components/modals/IslandHierarchyModal.tsx:43`
- **Trigger**: `IslandListCard` header button (`src/components/controls/IslandListCard.tsx:93`)
  sets `showHierarchyModal`; the modal is rendered inside the card at line 269.
- **Renders**: hand-written backdrop/panel: header, scrollable body of recursive tree nodes with
  connecting lines, and a legend footer.
- **Layout & styling**: backdrop `fixed inset-0 z-50 flex items-center justify-center bg-black/55 backdrop-blur-sm px-3`
  (backdrop mousedown closes). Panel
  `w-full max-w-3xl max-h-[84vh] flex flex-col rounded-xl border shadow-2xl` on `var(--surface-0)`.
  Body `flex-1 overflow-y-auto custom-scrollbar px-4 py-3`; tree wrapper `space-y-2.5`.
  Footer `px-3 py-3 border-t` with `background: 'var(--surface-1)'`, text `text-[11px] space-y-1`,
  first line `font-semibold` in `var(--text-strong)`.
  Node row: `flex items-center gap-2 p-2 rounded-md transition-colors relative border` with inline
  `paddingLeft: `${level * 32 + 8}px``, `background: 'var(--surface-1)'`,
  `borderColor: 'color-mix(in srgb, var(--border-subtle), transparent 20%)'`.
  Expand button: `h-6 w-6 inline-flex items-center justify-center rounded border transition-colors z-10`
  with `background: 'var(--surface-2)'`; leaf nodes render a `w-6` spacer instead.
  Connectors (only for `level > 0`): horizontal `absolute top-7 border-t` at
  `left: ${(level - 1) * 32 + 16}px`, `width: '24px'`; vertical `absolute top-0 bottom-0 border-l` at
  the same left with `height: '28px'`; child container adds a `border-l` at
  `left: ${level * 32 + 16}px`, `top: 0`, `bottom: '12px'`.
  Island id `font-mono text-sm font-bold` in `var(--accent)`; layer range `text-[11px] font-mono`;
  volume `text-sm`; child badge `text-[10px] px-2 py-0.5 rounded` on `var(--surface-2)`.
- **States**: no islands → centered `No islands found.` (`text-center py-10 text-sm`). Nodes start
  expanded (`useState(true)`); a node with children gets the chevron button, a leaf gets a spacer.
- **Data**: props `islands: Island[]`, `isOpen`, `onClose`, `layerHeightMm`, `zOffsetMm`.
  Tree built by grouping `islands.filter(i => !i.parentId)` as roots and attaching
  `islands.filter(i => i.parentId === island.id)` recursively.
  Per node: `layerRange = L{firstLayer}-{lastLayer}`, `layerCount = lastLayer - firstLayer + 1`,
  `volume = fmt.quantity(volumeMm3 ?? 0, 2, 'mm3')`, `maxArea = fmt.quantity(maxAreaMm2 ?? 0, 2, 'mm2')`,
  `worldMaxAreaLayer = Math.round(maxAreaLayer + (zOffsetMm / layerHeightMm))`.
- **Callbacks**: node chevron → toggle local expand; close X and backdrop → `onClose`.
- **Text**: title `Island Hierarchy`; subtitle `Parent-child merges across detected islands`;
  close aria `Close`; empty `No islands found.`; legend heading `Legend`; legend lines
  `• Root islands have no parent (independent start)`,
  `• Child islands merge upward into a parent node`,
  `• Volume shows each island’s individual contribution` (note the curly apostrophe);
  interpolated messages: `(# layer)` / `(# layers)`, `# child` / `# children`,
  `Max area: {area} at L{layer}`.
- **Port target**: `gpui/src/features/islands/island_hierarchy_modal.rs`.

### Surface: benchmark event protocol (`diagnosticsBenchmarkEvents.ts`)

- **Source**: `src/components/modals/diagnosticsBenchmarkEvents.ts:1`
- **Contents**: two window event names —
  `DIAGNOSTICS_BENCHMARK_REQUEST_EVENT = 'dragonfruit:diagnostics-benchmark-request'` and
  `DIAGNOSTICS_BENCHMARK_PROGRESS_EVENT = 'dragonfruit:diagnostics-benchmark-progress'` — plus the
  payload types: `DiagnosticsBenchmarkPhaseName = 'slow' | 'medium' | 'fast'`,
  `DiagnosticsBenchmarkStressProfile = 'quick' | 'standard' | 'torture'`,
  `DiagnosticsBenchmarkStats { sampleCount, durationMs, fpsAvg, fpsMin, fpsMax, frameTimeAvgMs, frameTimeP95Ms, frameTimeMaxMs }`,
  `DiagnosticsBenchmarkPhaseResult { phase, stats }`,
  `DiagnosticsBenchmarkResult { requestId, stressProfile, startedAtIso, finishedAtIso, totalDurationMs, projectionMode: 'orthographic'|'perspective', cameraFeelPreset: 'raw'|'precise'|'balanced'|'fast', phases[], overall }`,
  `DiagnosticsBenchmarkRequestDetail { requestId, stressProfile }`,
  `DiagnosticsBenchmarkProgressDetail { requestId, status: 'started'|'phase-complete'|'completed'|'error', phase?, message?, result? }`.
- **Port target**: `gpui/src/features/diagnostics/benchmark_events.rs` (the request is consumed by the
  renderer, not by the dialog).

### Surface: `DiagnosticsModal` — runtime telemetry + orbit benchmark

- **Source**: `src/components/modals/DiagnosticsModal.tsx:206` (props type at 23, helpers at 95–190)
- **Trigger**: `DiagnosticsModals` with `isOpen={isDiagnosticsOpen}`; toggled by the debug hotkey
  `Ctrl+Shift+D` (`DEBUG.DIAGNOSTICS` in `src/hotkeys/hotkeyConfig.ts:176`, description
  `'Toggle diagnostics modal'`). The modal's own subtitle states the binding:
  `Runtime telemetry for DragonFruit (toggle with Ctrl+Shift+D)`.
- **Renders**: hand-written backdrop/panel: header (title + subtitle + close X), then a scroll area
  containing: an 8-cell KPI grid, the benchmark card, two sparkline cards, scene/system stat pair,
  support/picking stat pair.
- **Layout & styling**: backdrop
  `fixed inset-0 z-[70] flex items-center justify-center bg-black/60 backdrop-blur-sm px-4`
  (**`bg-black/60` here, not `/55`**), backdrop `onMouseDown` closes. Panel
  `w-full max-w-5xl max-h-[88vh] overflow-hidden rounded-xl border shadow-2xl` with
  `background: var(--surface-0)`, `borderColor: var(--border-subtle)`,
  `boxShadow: '0 28px 64px rgba(0,0,0,0.48)'`. Header `flex items-center justify-between px-4 py-3 border-b`.
  Scroll body `max-h-[calc(88vh-58px)] overflow-y-auto custom-scrollbar p-4 space-y-4`.
  KPI grid `grid grid-cols-2 md:grid-cols-4 gap-3`; each KPI `rounded-lg border p-3` on `var(--surface-1)`
  with label `text-[11px]` and value `text-xl font-semibold` in `var(--text-strong)`.
  Benchmark card `rounded-lg border p-3` on `var(--surface-1)`; its profile `SelectDropdown` uses
  `className="space-y-0"`, `selectClassName="!h-9 min-w-[118px] text-[12px]"`,
  `selectStyle={{ borderColor: 'var(--border-subtle)', background: 'var(--surface-0)', color: 'var(--text-strong)' }}`.
  Run button `h-9 rounded-md border px-3 text-[12px] font-semibold transition-colors disabled:opacity-45`;
  idle style `borderColor: 'color-mix(in srgb, var(--accent), white 10%)'`,
  `background: 'color-mix(in srgb, var(--accent), var(--surface-0) 76%)'`,
  `color: 'var(--accent-contrast)'`; running style `var(--border-subtle)` / `var(--surface-2)` / `var(--text-muted)`.
  Result tiles inside the benchmark card: `rounded-md border px-2.5 py-2` on `var(--surface-0)`;
  phase table `w-full text-[11px]` with `thead` on `var(--surface-0)` and rows on `var(--surface-1)`
  separated by `borderTop: '1px solid var(--border-subtle)'`.
  Sparkline cards live in `grid grid-cols-1 xl:grid-cols-2 gap-4`; each `rounded-lg border p-3`;
  the SVG is `viewBox="0 0 380 88"`, class `w-full h-24`, `preserveAspectRatio="none"`, `aria-hidden`,
  stroke width 2, `strokeLinecap/Linejoin: 'round'`. FPS line stroke `var(--accent)`;
  frame-time line stroke `var(--accent-secondary)`.
  The two stat pairs use `grid grid-cols-1 md:grid-cols-2 gap-4`, each cell `rounded-lg border p-3`
  with rows `space-y-1 text-[12px]` in `var(--text-muted)` and values in `var(--text-strong)`.
- **States / behaviour**:
  - Sampling runs only while open, via `requestAnimationFrame`. Every frame pushes the frame delta into
    `frameTimeHistory` (capped at `MAX_GRAPH_POINTS`). Every `SAMPLE_INTERVAL_MS = 250` ms it computes
    `fps = frameCounter * 1000 / elapsed`,
    `avgFrameTimeMs = elapsed / frameCounter`, and
    `cpuUsageEstimate = Math.min(100, Math.max(0, (avgFrameTimeMs / 16.67) * 100))`,
    pushes one FPS sample, and updates state (including `webglMode` and heap). Window is
    `GRAPH_WINDOW_SECONDS = 120`, so `MAX_GRAPH_POINTS = 480`.
  - `webglMode`: first `<canvas>` in the document — `'WebGL2'` if `getContext('webgl2')`, else
    `'WebGL1'` if `getContext('webgl')`, else `'Unknown'`.
  - Heap figures come from the non-standard `performance.memory` (`usedJSHeapSize`,
    `totalJSHeapSize`, `jsHeapSizeLimit`), `null` when absent; all three render `N/A` then.
  - `heapUsageRatio` is `used / max(1, total)` or `null`.
  - Benchmark: idle message `No benchmark run yet.`; run states `Queued`, `Starting`,
    `{PHASE} complete`, `Done`, `Failed`, with messages `Requesting benchmark run…`,
    `Starting benchmark…`, `Benchmark complete.`, `Benchmark failed.`. Error text replaces the message
    and is coloured `var(--danger)`; a run appends `` ` • ${phaseLabel}` ``.
  - A progress event whose `requestId` differs from the current one is ignored
    (`if (prev.requestId && prev.requestId !== detail.requestId) return prev;`).
  - Run button label: `Running…` while `isRunning`, else `Run Benchmark`; disabled while running.
    Profile select is disabled while running.
  - `benchmarkResult` block (rendered only after a completed run): three tiles
    (`Overall Avg` with FPS + frame time, `Overall P95` with `Projection: {mode}`, `Run Duration`
    with seconds and `Camera feel: {preset} • Stress: {profile}`) and the phase table with headers
    `Phase`, `Avg FPS`, `Min FPS`, `Avg FT`, `P95 FT`, `Max FT`.
  - FPS sparkline max is `Math.max(60, ...fpsHistory, 1)`; frame-time max is
    `Math.max(24, ...frameTimeHistory, 1)`; both min 0.
- **Data**: props `isOpen`, `onClose`, `appMode`, `cameraProjectionMode`, `modelCount`,
  `visibleModelCount`, `selectedModelCount`, `totalPolygons`, `selectedPolygons`.
  Internal `stats` (`fps`, `frameTimeMs`, `cpuUsageEstimate`, histories, heap triple, `webglMode`,
  `supportStats`), benchmark state.
  `computeSupportDiagnostics()` reads the support snapshot and `getPickingDiagnosticsSnapshot()`:
  for every `SUPPORT_TYPES` descriptor it walks the collection; descriptors with
  `hasSegments === false` add `implicitSegmentCount(descriptor)` segments instead
  (comment: `// Every type's segments and joints. A brace has no segments but draws one` /
  `// shaft, so it counts as one each.`); it collects unique joint ids from
  `segment.topJoint?.id` / `segment.bottomJoint?.id`;
  `estimatedRenderablePrimitives = segmentCount + uniqueJoints + knots + roots + leaves + braces`.
  Picking figures sum `registrationsByCategory.support/segment/joint/knot/raft` for the support-related
  total and pass through `totalRegistrations`, `visiblePickObjects`, `cachedPickObjects`,
  `avgPickDurationMs`, `lastPickDurationMs`, `avgSyncDurationMs`, `picksPerSecond`.
- **Callbacks**: close X and backdrop → `onClose`; Escape → `onClose` (`useEscapeToClose(isOpen, onClose)`);
  `Run Benchmark` → dispatch
  `new CustomEvent(DIAGNOSTICS_BENCHMARK_REQUEST_EVENT, { detail: { requestId, stressProfile } })`
  with `requestId = \`benchmark-${Date.now()}-${Math.random().toString(36).slice(2, 8)}\``.
  The modal also listens for `DIAGNOSTICS_BENCHMARK_PROGRESS_EVENT` while open.
- **Text** (all hardcoded English, **not** i18n): title `Diagnostics`; subtitle
  `Runtime telemetry for DragonFruit (toggle with Ctrl+Shift+D)`; close aria `Close diagnostics`;
  KPI labels `FPS`, `Frame Time`, `Used JS Heap`, `Heap Usage`, `CPU Usage (est.)`,
  `Support Segments`, `Support Pickers`, `Picking Avg`;
  benchmark block `Automated Orbit Benchmark`,
  `Runs slow, medium, and fast full 3D sweep passes around the current model and records frame timing.`,
  select aria `Benchmark stress profile`, options `Quick`/`Standard`/`Torture`;
  `FPS History` + `Last {120}s ({250}ms samples)`; `Frame Time History (ms)` + `Last {120}s frame time`;
  `Scene Stats` rows `Mode:`, `Rendering Mode:`, `Graphics API:`, `Models:`, `Visible Models:`,
  `Selected Models:`, `Total Triangles:`, `Selected Triangles:`; `System` rows
  `Logical CPU Cores:`, `Device Memory:`, `Total JS Heap:`, `JS Heap Limit:`, `User Agent:`;
  `Support Stats` rows derived from `SUPPORT_COLLECTION_KEYS` (capitalised key) plus `Segments:`,
  `Joints:`, `Estimated Render Primitives:`; `GPU Picking Stats` rows `Total Registered:`,
  `Support Registered:`, `Visible Pick Objects:`, `Cached Pick Objects:`, `Pick Avg:`, `Pick Last:`,
  `Sync Cache Avg:`, `Pick Rate:`. Number formats: `fmt.fixed(fps,1)`, `fmt.quantity(ms,2,'ms')`,
  `fmt.percent(x)` at one decimal, `fmt.number(...)` for integers, `fmt.quantity(x,3,'ms')` for pick times,
  `perSecond` at one decimal, bytes as GB/MB/KB/B with 2/2/1/0 decimals.
- **Port target**: `gpui/src/features/diagnostics/diagnostics_modal.rs` — `gpui/` does not exist yet.

### Surface: `HistoryDebugModal` — undo/redo event log

- **Source**: `src/components/modals/HistoryDebugModal.tsx:38`
- **Trigger**: `DiagnosticsModals` with `isOpen={isHistoryDebugOpen}`; debug hotkey `Ctrl+Shift+C`
  (`DEBUG.HISTORY`, description `'Toggle history debug'`). The header prints `Hotkey: Ctrl+Shift+C`.
- **Renders**: hand-written backdrop/panel: header, a chip/action toolbar row, then the event list.
- **Layout & styling**: backdrop `fixed inset-0 z-[70] flex items-center justify-center bg-black/60 backdrop-blur-sm px-4`
  with backdrop close. Panel `w-full max-w-5xl max-h-[88vh] overflow-hidden rounded-xl border shadow-2xl`
  with `background: var(--surface-0)`, `boxShadow: '0 28px 64px rgba(0,0,0,0.48)'`.
  Header `flex items-center justify-between border-b px-4 py-3`.
  Toolbar `flex flex-wrap items-center gap-2 border-b px-4 py-3`; chips are
  `rounded-full border px-2 py-1 text-[11px] font-semibold` in `var(--text-muted)`;
  toolbar buttons are `h-8 inline-flex items-center justify-center rounded-md border px-3 text-xs font-semibold transition-colors`.
  List `max-h-[60vh] overflow-y-auto custom-scrollbar px-4 py-3`; rows `space-y-1.5`, each row
  `relative pl-7`; timeline rail `absolute left-[10px] top-5 bottom-[-10px] w-px` with
  `background: 'color-mix(in srgb, var(--border-subtle), transparent 10%)'` (skipped on the last row);
  dot `absolute left-0 top-2.5 h-5 w-5 rounded-full border` on `var(--surface-0)` containing an
  `absolute left-1/2 top-1/2 h-2.5 w-2.5 -translate-x-1/2 -translate-y-1/2 rounded-full` of the kind colour;
  card `cursor-pointer rounded-lg border px-2.5 py-1.5 transition-colors` with
  `background: 'color-mix(in srgb, var(--surface-1), black 8%)'`.
- **States**:
  - Events are shown **newest first** (`[...historyDebugEvents].reverse()`).
  - Empty: `No history events yet.` in a `rounded-lg border p-3 text-xs` card.
  - Per-kind visuals (`getHistoryDebugEventVisual`): `push` → dot `var(--accent)`, row border
    `color-mix(in srgb, var(--accent), var(--border-subtle) 68%)`; `undo` → `#fbbf24`, border `…72%)`;
    `redo` → `#60a5fa`, border `…72%)`; `undo-empty` / `redo-empty` → `var(--text-muted)`, border
    `var(--border-subtle)`; `undo-handler-missing` / `redo-handler-missing` / `undo-declined` /
    `redo-declined` → `#fb7185`, border `…70%)`; `clear-history` / `clear-debug-log` / default →
    `#a78bfa`, border `…72%)`. Badge background is always `color-mix(in srgb, {colour}, var(--surface-0) 90%)`
    (grey variant: `color-mix(in srgb, var(--surface-2), var(--surface-0) 45%)`).
  - Selected preview event: its card border becomes
    `color-mix(in srgb, var(--accent), var(--border-subtle) 40%)` and a `Preview` pill appears
    (`rounded-full border px-1.5 py-0 text-[10px] font-semibold`, accent).
  - `Cancel Preview` button appears only when `isPreviewActive` (amber styling:
    `borderColor: 'color-mix(in srgb, #fbbf24, var(--border-subtle) 55%)'`,
    `background: 'color-mix(in srgb, #fbbf24, var(--surface-1) 92%)'`).
  - Event with neither `actionType` nor `actionDescription` renders `No action payload attached`.
- **Data**: props `isOpen`, `onClose`, `historyDebugEvents: HistoryDebugEvent[]`,
  `historyStackCounts {undo, redo}`, `historyStackBytes {undo, redo, total}`, `sceneSnapshotBytes`,
  `selectedPreviewEventId`, `isPreviewActive`, `onJumpToEvent`, `onCancelPreview`,
  `onClearEventLog`, `onClearUndoRedoStacks`, `onClearAll`.
  Helpers: `formatBytes` → `{n} B` / `{(n/1024).toFixed(1)} KB` / `{(n/1048576).toFixed(1)} MB`
  (comment `/** Bytes as B / KB / MB — a debug readout, so one decimal is plenty. */`);
  timestamp via `new Date(ts).toLocaleTimeString()` with a `String(ts)` catch;
  kind label via `kind.replaceAll('-', ' ').replace(/\b\w/g, (m) => m.toUpperCase())` —
  i.e. "undo-empty" → "Undo Empty".
- **Callbacks**: card click or `Enter`/`Space` (keyboard, `preventDefault`) → `onJumpToEvent(event)`;
  `Cancel Preview` → `onCancelPreview`; `Clear Event Log` → `onClearEventLog`;
  `Clear Undo/Redo Stacks` → `onClearUndoRedoStacks`; `Clear All` → `onClearAll`;
  close X / backdrop / Escape → `onClose`.
  In `DiagnosticsModals`: `onClearEventLog` → `clearHistoryDebugEvents()`;
  `onClearUndoRedoStacks` → `clearHistory()`; `onClearAll` → both, in that order.
- **Text** (hardcoded English, not i18n): title `History Debug Log`; subtitle `Hotkey: Ctrl+Shift+C`;
  close aria `Close history debug`; chips `Undo Stack: {n}`, `Redo Stack: {n}`, `Events: {n}`,
  `Payloads: {bytes}`, `Snapshots: {bytes}`; the two memory chips carry `title` tooltips:
  `"Payload bytes on the undo + redo stacks. Mesh snapshots are not in the payloads — they live in the scene registry, counted separately."`
  and `"Geometry held by the scene-snapshot registry, each mesh counted ONCE — snapshots share meshes, so a move costs nothing here."`;
  buttons `Cancel Preview`, `Clear Event Log`, `Clear Undo/Redo Stacks`, `Clear All`;
  row chips `Size: {bytes} · Undo: {n}`, `Redo: {n}`, `Jump`; badge `Preview`; missing payload
  `No action payload attached`; empty `No history events yet.`
- **Port target**: `gpui/src/features/history/history_debug_modal.rs`.

---

## Organisms: `src/components/organisms/modals/`

These are thin composition layers the page mounts once (`src/app/page.tsx:11488–11895`). They own
almost no state of their own; the exceptions are `MeshRepairModals` (repair run state) and the
per-dialog `SceneMaterialReassignModal` mapping (which lives in that dialog).

### Surface: `DiagnosticsModals` — diagnostics trio

- **Source**: `src/components/organisms/modals/DiagnosticsModals.tsx:35`
- **Trigger**: rendered unconditionally by the page with all-open flags as props.
- **Renders**: a fragment of three dialogs — `DiagnosticsModal`, `HistoryDebugModal`, and the
  out-of-scope `SliceMetricsDebugModal`.
- **Data mapping** (exact):
  - `DiagnosticsModal`: `appMode={scene.mode}`,
    `cameraProjectionMode={getSavedCameraProjectionSettings().mode}`,
    `modelCount={scene.models.length}`,
    `visibleModelCount={scene.models.filter((m) => m.visible).length}`,
    `selectedModelCount={scene.selectedModelIds.length}`, plus `totalPolygons` / `selectedPolygons` props.
  - `HistoryDebugModal`: all history props forwarded; `onClose` → `setIsHistoryDebugOpen(false)`.
  - `SliceMetricsDebugModal`: `benchmark={printingSlicingBenchmark}`,
    `outputName={printingArtifact?.outputName ?? null}`, `outputSizeLabel={printingOutputSizeLabel}`.
- **Callbacks**: each `onClose` only flips its own flag. No business logic.
- **Text**: none of its own.
- **Port target**: fold into the page/root view's modal group; proposed
  `gpui/src/features/diagnostics/mod.rs`. Nothing exists yet.

### Surface: `DiagnosticsModals` — hotkey plumbing (page side, for the porter)

- `isDiagnosticsOpen`, `isHistoryDebugOpen`, `isSliceMetricsDebugOpen` are plain state
  (`page.tsx:1638–1641`); the debug hotkeys toggle them and, for the history log, the page also
  refreshes counts/bytes on open (`page.tsx:6552–6578`) and mirrors the flag into
  `setHistoryDebugDetailsEnabled(isHistoryDebugOpen)` — i.e. the history *store* starts recording
  detail only while the panel is open.

### Surface: `SceneFileModals` — scene I/O dialogs

- **Source**: `src/components/organisms/modals/SceneFileModals.tsx:149` (props at 19; shared
  `UnsavedChangesDialog` at 100)
- **Trigger**: rendered unconditionally at `page.tsx:11676`; each child opens on its own flag or on a
  prompt object owned by `useSceneCollectionManager` / the import-export manager.
- **Renders** (in order): save-error dialog, `ModelSupportsModal`, scene-import placement prompt,
  `ObsoleteVoxlVersionModal`, `ScenePluginMissingModal`, `ScenePrinterMismatchModal`,
  scene-import hollowing prompt, `SceneAutosaveRecoveryModal`, plugin-import warning, `ZipFilePickerModal`,
  the close-program unsaved-changes dialog, the new-scene unsaved-changes dialog, the save-choice
  dialog, and the arrange `BlockingOverlay`.
- **Escape registrations** (top of the component, comment:
  `// Escape mirrors each dialog's backdrop click; the arrange overlay is a` /
  `// blocking progress state, so it swallows the key instead.`):
  - placement prompt → `scene.resolveSceneImportPlacementPrompt('load_as_is')`
  - hollowing prompt → `scene.resolveSceneImportHollowingPrompt('bake')`
  - plugin-import warning → `handleCancelPluginImportWarning`
  - save choice → `resolveSceneSaveChoice('cancel')`
  - arrange overlay → `undefined` (swallow)
- **Text**: none of its own beyond the sub-surfaces below.

#### Sub-surface: save error

- **Source**: `SceneFileModals.tsx:196`
- `StructuredDialogModal` with `open={sceneSaveError !== null}`, `onClose={() => dismissSceneSaveError?.()}`,
  `ariaLabel={_(msg`Save Error`)}`, `icon={<AlertTriangle className="h-5 w-5 text-amber-400" />}`,
  title `{sceneSaveError?.title ?? 'Save error'}`, and a single action `OK`
  (tinted-accent, `inline-flex items-center justify-center gap-1.5`; the label is a **bare `OK` string, not `Trans`**).
  No subtitle, no explicit close label → the X uses the shell default `Close dialog`.
  Body: `space-y-2 text-xs`; message `font-medium text-sm` in `var(--text-strong)`; optional detail in
  `<pre className="mt-2 overflow-x-auto rounded bg-black/30 p-2 text-[11px] font-mono whitespace-pre-wrap">`.
  All buttons and X call `dismissSceneSaveError?.()`.

#### Sub-surface: scene import placement prompt ("Scene may be off-plate")

- **Source**: `SceneFileModals.tsx:219`
- Hand-written shell; open iff `scene.sceneImportPlacementPrompt` is set
  (`{source, fileName, modelCount, offPlateModelCount}`).
- Backdrop `fixed inset-0 z-[220] flex items-center justify-center bg-black/55 backdrop-blur-sm px-3`;
  mousedown on the backdrop resolves `'load_as_is'`. Panel `w-full max-w-lg overflow-hidden rounded-xl border shadow-2xl`
  on `var(--surface-0)`; `role="dialog" aria-modal="true"`,
  `aria-label={_(msg`Scene import placement decision`)}`.
- Header `flex items-center justify-between gap-4 border-b px-5 py-4`; icon chip is `IconChip size="lg"`
  with `icon={LayoutGrid}` and inline accent styling
  (`borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 45%)'`,
  `background: 'color-mix(in srgb, var(--accent), var(--surface-1) 88%)'`); title
  `Scene may be off-plate`; subtitle `Choose how to place imported models.`;
  close `IconButton variant="surface" size="md"` aria-label `Close scene import placement prompt`.
- Body `space-y-4 p-5`; info card `rounded-md border px-3 py-2` on `var(--surface-1)` with eyebrow
  `Imported scene`, file name `text-sm font-semibold truncate` (`title` = full name), and count line
  `{offPlateModelCount.toLocaleString()} of {modelCount.toLocaleString()} model{s} appear outside the build plate.`
  (the `s` is a conditional on `modelCount === 1`).
- Explainer paragraph: `**Auto-Arrange** will reposition imported models onto free space on the plate.`
  then a block `**Load As-Is** keeps scene coordinates exactly as stored in the file.`
- Actions `grid grid-cols-2 gap-2 pt-1`: `Load As-Is` (secondary) → `'load_as_is'`;
  `Auto-Arrange` (tinted-accent) → `'auto_arrange'`.

#### Sub-surface: obsolete VOXL version

- Renders `ObsoleteVoxlVersionModal` (see above) when `scene.obsoleteVoxlScene` is set, with
  `isOpen` (always true), `fileName`, `detected`, `onDismiss={scene.dismissObsoleteVoxlScene}`.

#### Sub-surface: printer plugin missing

- Always mounted: `<ScenePluginMissingModal prompt={scene.missingPrinterPluginPrompt} …>`.
  `onImportGeometry` → `scene.resolveMissingPrinterPluginChoice('geometry')`;
  `onManagePlugins` → `scene.resolveMissingPrinterPluginChoice('manage')` **then**
  `openSettingsModal('plugins')` (`src/components/settings/settingsModalEvents`).

#### Sub-surface: printer mismatch

- Renders `ScenePrinterMismatchModal` when `scene.printerMismatch` is set, `isOpen` always true, with
  the prop mapping listed under that surface. `onSwitch`/`onKeep` →
  `scene.resolvePrinterMismatch('switch' | 'keep')`; the "switch" path may add the printer from the
  scene bundle or recreate it from `bundle.printer.officialPresetId`.

#### Sub-surface: scene import hollowing prompt ("Scene was hollowed before export")

- **Source**: `SceneFileModals.tsx:374`
- Open iff `scene.sceneImportHollowingPrompt` (`{source, fileName, modelCount, hollowedModelCount}`).
  Same hand-written shell, `z-[220]`, `max-w-lg`, `aria-label={_(msg`Scene import hollowing decision`)}`;
  backdrop mousedown resolves `'bake'`.
- Icon chip `IconChip size="lg" icon={CircleDashed}` with the same accent styling; title
  `Scene was hollowed before export`; subtitle `Choose whether to keep the hollow.`;
  close aria `Close scene import hollowing prompt` → resolves `'bake'`.
- Info card eyebrow `Imported scene`, file name, then a plural line built with Lingui `<Plural>`:
  `{hollowedModelCount} of ` + one of `# model carries an interior cavity.` / `# models carry an interior cavity.`
  (the PO extractor renders this as
  `"{0} of {1, plural, one {# model carries an interior cavity.} other {# models carry an interior cavity.}}"`).
- Explainer: `**Keep Hollow** builds the stored cavity into each model, so it prints hollow exactly as authored.`
  and a second block `**Import Solid** loads the outer shape only. The cavity is kept, so you can apply it later from the Hollowing panel.`
- Actions `grid grid-cols-2 gap-2 pt-1`: `Import Solid` (secondary) → `'keep_solid'`;
  `Keep Hollow` (tinted-accent) → `'bake'`.

#### Sub-surface: autosave recovery

- Renders the out-of-scope `SceneAutosaveRecoveryModal` when `autosaveRecovery`
  (`{savedAt, voxlPath, origin}`) is non-null, forwarding `onRestore={handleAutosaveRestore}` and
  `onDiscard={handleAutosaveDiscard}`.

#### Sub-surface: plugin import experimental warning

- **Source**: `SceneFileModals.tsx:461`
- Open iff `showPluginImportWarningModal`; `StructuredDialogModal` with `open` (always),
  `zIndexClassName="z-[220]"`, `ariaLabel={_(msg`Plugin import experimental warning`)}`,
  title `formatExperimentalTitle(pluginImportWarningTitle, _)` → `{feature} is Experimental` when a
  title was supplied, else `Plugin Import is Experimental`; subtitle `This feature is still under development.`;
  `icon={<AlertTriangle className="h-4 w-4" />}`, `iconTone="warning"`,
  `closeAriaLabel={_(msg`Close plugin import warning`)}`, X and backdrop → `handleCancelPluginImportWarning`.
- Body: paragraph `{pluginImportWarningBody ?? 'Geometry, support placement, and transforms can import differently across scene variants, so unforeseen results are still possible.'}`
  (`text-xs leading-relaxed`, muted). Then a footer row
  `flex flex-wrap items-center justify-between gap-3 pt-1` containing:
  - a `<label className="inline-flex items-center gap-2 text-xs select-none">` with a checkbox
    (`h-3.5 w-3.5 rounded border`, `accentColor: '#f59e0b'`) bound to `pluginImportWarningSkipFuture`
    and the label `Do not remind again`;
  - buttons `Cancel` (secondary) and `Continue` (secondary with an amber inline style:
    `borderColor: 'color-mix(in srgb, #f59e0b, var(--border-subtle) 45%)'`,
    `background: 'color-mix(in srgb, #f59e0b, var(--surface-1) 86%)'`, `color: '#fde68a'`).
  Note: these buttons are **inside the children block**, so the shell's `actions` slot is unused and
  the default equal-width grid does not apply.
- Semantics: the page awaits this dialog as a promise (`page.tsx:1900`); `Continue` resolves `true`,
  `Cancel`/X/backdrop/Escape resolves `false`.

#### Sub-surface: ZIP file picker

- Renders `ZipFilePickerModal` when `zipPickerState` is set; `onConfirm(selected)` clears the ref and
  state then resolves the waiting promise with the selected files; `onCancel` does the same with `[]`.

#### Sub-surface: `UnsavedChangesDialog` (shared shell, two instances)

- **Source**: `SceneFileModals.tsx:100`
- `StructuredDialogModal` with `ariaLabel={_(msg`Unsaved changes`)}`, title `Unsaved Scene Changes`,
  `iconTone="warning"`, `icon={<AlertTriangle className="h-4 w-4" />}`, `zIndexClassName="z-[220]"`,
  `closeAriaLabel={_(msg`Close unsaved changes modal`)}`, `closeDisabled={busy}`.
  `onClose` and `onBackdropClick` are both guarded: `if (busy) return; onCancel();`
  → with `busy` set, the X is disabled **and** Escape is swallowed (shell rule).
- Actions (default equal-width grid): `Discard Changes` (`variant="tinted-danger"`,
  `Trash2 w-3.5 h-3.5`, `w-full inline-flex items-center justify-center gap-1.5`, `disabled={busy}`)
  and `{confirmLabel}` (`variant="secondary"`, `w-full`, `disabled={busy}`).
- **There is no Cancel button**: cancelling is the X, the backdrop, or Escape.
- Instance A (close program), `open={showCloseUnsavedChangesModal}`:
  `busy={closeUnsavedChangesBusy !== 'none'}`, `confirmLabel={_(msg`Save & Close`)}`,
  `onCancel` → `setShowCloseUnsavedChangesModal(false)`, `onDiscard` → `handleDiscardAndCloseProgram`,
  `onSave` → `handleSaveAndCloseProgram`. Body: when `hasUnsavedSceneChanges`
  `You’re about to close DragonFruit with unsaved scene changes.` else `Close DragonFruit now?`,
  followed by `**Please ensure you have saved any important work.**`
  (body paragraphs `text-xs leading-relaxed`, muted).
  Subtitle: `hasUnsavedSceneChanges ? _(msg`You have unsaved edits in this scene.`) : _(msg`This scene is already saved.`)`.
- Instance B (new scene), `open={showNewSceneUnsavedChangesModal}`:
  `busy={newSceneBusy !== 'none'}`, `confirmLabel={_(msg`Save & New Scene`)}`,
  `onCancel` → `setShowNewSceneUnsavedChangesModal(false)`, `onDiscard` → `handleDiscardAndNewScene`,
  `onSave` → `handleSaveAndNewScene`. Body: `You’re about to start a new scene with unsaved edits in this one.`
  else `Start a new scene now?`, then the same bold save reminder. Same subtitle rule.
- Trigger for both: `page.tsx:5076` (close request) and `page.tsx:5128` (new scene) check
  `hasUnsavedSceneChangesRef.current` first; both are also raised from the OS close-request listener
  (`page.tsx:5214`).

#### Sub-surface: save choice ("Save Loaded Scene")

- **Source**: `SceneFileModals.tsx:600`
- Open iff `showSceneSaveChoiceModal`; hand-written shell `z-[220]`, `max-w-lg`,
  `aria-label={_(msg`Save scene options`)}`; backdrop mousedown → `resolveSceneSaveChoice('cancel')`.
- Icon chip `IconChip size="lg" icon={CheckCircle2}` with green styling
  (`borderColor: 'color-mix(in srgb, #22c55e, var(--border-subtle) 55%)'`,
  `background: 'color-mix(in srgb, #22c55e, var(--surface-1) 90%)'`,
  `color: 'color-mix(in srgb, #22c55e, var(--text-strong) 18%)'`).
- Title `Save Loaded Scene`; subtitle ``Choose where Ctrl+S should save this imported `.voxl` scene.``;
  close aria `Close save scene options` → `'cancel'`.
- Body `space-y-3.5 p-5`; info card `rounded-lg border px-3 py-2.5` with
  `background: 'color-mix(in srgb, var(--surface-1), black 8%)'`, eyebrow `Loaded file`,
  name `{sceneSaveChoiceFileName ?? 'Loaded scene'}` (`title` = name), path
  `{sceneSaveChoicePath ?? 'Original file path unavailable (overwrite disabled)'}` (`title` = path).
- Buttons `grid grid-cols-1 sm:grid-cols-2 gap-2 pt-0.5`:
  `Save as New Scene` (**bare string, not `Trans`**) → `'save_as'`; `Overwrite Loaded Scene`
  (tinted-accent, `whitespace-nowrap inline-flex items-center justify-center gap-1.5`,
  `disabled={!sceneSaveChoicePath}`) → `'overwrite'`.
- Trigger: `page.tsx:1954` sets path + open and awaits a promise
  (`'overwrite' | 'save_as' | 'cancel'`).

#### Sub-surface: arrange blocking overlay

- When `showArrangeBlockingOverlay`, renders `BlockingOverlay` with `zIndexClassName="z-[120]"`,
  `title={_(arrangeOverlayContent.title)}`,
  `details={arrangeOverlayContent.detailLines.map((line) => _(line))}`,
  `elapsed={formatArrangeElapsed(arrangeOverlayElapsedLabel, _)}` → `Elapsed: {elapsed}`,
  `footnote={formatArrangeFootnote(arrangeOverlayModelCount ?? 0, _)}` →
  `Processing {count, plural, one {# model} other {# models}}`,
  `progress={null}` (indeterminate) and `progressLabel={_(arrangeOverlayContent.title)}`.
  Non-dismissible; Escape is swallowed (registration above).

- **Port target**: `gpui/src/features/scene_io/scene_file_dialogs.rs` (one module per group of the
  above is acceptable; the escape ordering must be preserved). Nothing exists yet.

### Surface: `ModifierModals` — unapplied modifiers, resets, destructive transform, apply overlay

- **Source**: `src/components/organisms/modals/ModifierModals.tsx:42`
- **Trigger**: rendered at `page.tsx:11732`; each dialog opens on its own prop:
  `showUnappliedHolePunchModal`, `pendingModifierResetAction !== null`,
  `pendingBlockerResetState !== null`, `pendingDestructiveTransform !== null`,
  `showModifierApplyBlockingOverlay`.
- **Renders**: three `StructuredDialogModal`s, `DestructiveTransformModal`, and a `BlockingOverlay`.
- Escape: `useEscapeToClose(showModifierApplyBlockingOverlay, undefined)` — comment:
  `// A blocking progress overlay: swallow Escape rather than let it through.` The three
  `StructuredDialogModal`s register their own Escape through their `onClose`.
- Derived: `isHollowingReset = pendingModifierResetAction === 'hollowing' || pendingModifierResetAction === 'clear_hollowing'`
  with comment `// Two of the three reset actions undo hollowing; only \`hole_punch\` is about` /
  `// holes alone, so the confirmation reads off that rather than one action.`

#### Sub-surface: unapplied changes prompt

- **Source**: `ModifierModals.tsx:67`
- `open={showUnappliedHolePunchModal}`, **`ariaLabel="Unapplied model changes"`** (hardcoded English),
  title/subtitle from `unappliedModifierPrompt`, `icon={<AlertTriangle className="h-4 w-4" />}`,
  `iconTone="warning"`, `closeAriaLabel="Close"`, `onClose={() => unappliedHolePunchResolveRef.current?.('skip')}`.
- Body `space-y-2`: each of `unappliedModifierPrompt.paragraphs` as
  `<p className="text-xs leading-relaxed" style={{ color: 'var(--text-muted)' }}>`.
- Actions (in this order): `Continue Without` (secondary) → `'skip'`;
  `Go to Hollow Tool` (tinted-accent, `inline-flex items-center justify-center gap-1.5`) → `'goto'`;
  `Apply to All` (tinted-accent, same class) → `'apply'`, rendered **only when
  `unappliedModifierPrompt.showApplyAll`**.
- Copy is produced by the page (`page.tsx:8224`) in two purposes:
  - purpose `'support'`-prompt (`supports`): title `Unapplied Changes`; subtitle is one whole sentence —
    `Some models have unapplied hole punches and hollowing` / `Some models have unapplied hole punches` /
    `Some models have unapplied hollowing`; paragraph 1 explains that supports are generated against the
    mesh as it stands and holes/hollowing bake later; paragraph 2 is
    `Apply the holes to every visible model, or open the Hollow tool to review and apply them per model.`
    (when holes exist) or `Open the Hollow tool to apply the hollowing, or continue and supports will follow the current mesh.`
    `showApplyAll = hasHoles`.
  - purpose `'export'`: title `Unapplied Holes`; subtitle `Some models have unapplied hole punches`;
    paragraphs `One or more models have hole punches that haven’t been applied. Hole punches must be baked into the geometry before slicing or they will not appear in the output.`
    and `Apply to All bakes the holes into every visible model when applicable. Or open the Hollow tool to review and apply holes individually.`;
    `showApplyAll: true`.
- Semantics: the dialog only reports the choice; the page performs it. `'goto'` switches to support mode
  and the hollowing tool and answers a waiting caller `false`; `'apply'` starts the bake-all queue and
  answers when the queue drains; `'skip'` answers `true`. The export flow opens the same dialog with no
  caller to answer.

#### Sub-surface: modifier reset confirmation

- **Source**: `ModifierModals.tsx:109`
- `open={pendingModifierResetAction !== null}`, `ariaLabel="Confirm modifier reset"`,
  `closeAriaLabel="Close reset confirmation"`, `iconTone="warning"`,
  `onClose={() => setPendingModifierResetAction(null)}`.
- Title: `Remove Hollowing?` when `isHollowingReset`, else `Remove All Holes?`.
  Subtitle (always): `This action can't be undone`.
- Body `space-y-2`, two paragraphs (`text-xs leading-relaxed`, muted):
  1. `Are you sure you want to remove hollowing from this model?` / `Are you sure you want to remove all hole punches from this model?`
  2. by action — `clear_hollowing` → `The cavity and all holes will be removed.`;
     `hollowing` → `Your model will return to its solid version.`;
     otherwise (`hole_punch`) → `All holes on this model will be removed.`
- Actions: `Cancel` (secondary) → clear the pending action; `Remove Hollowing` / `Remove All Holes`
  (tinted-danger, `Trash2 w-3.5 h-3.5`) → `handleConfirmModifierReset`, which clears the action then
  dispatches to `handleResetHollowing()` or the hole-punch reset (`page.tsx:10297`).

#### Sub-surface: solid-area (blocker) reset confirmation

- **Source**: `ModifierModals.tsx:152`
- `open={pendingBlockerResetState !== null}`, `ariaLabel="Confirm solid area reset"`,
  title `Reset Solid Areas?`, subtitle `Solid areas will be lost`, `iconTone="warning"`,
  `closeAriaLabel="Close solid area reset confirmation"`,
  `onClose={() => setPendingBlockerResetState(null)}`.
- Body: `Changing the voxel resolution or shell thickness will clear all solid areas.` and
  `You will need to mark them again after the change.`
- Actions: `Cancel` (secondary); `Reset Solid Areas` (tinted-danger, `Trash2 w-3.5 h-3.5`) →
  `handleConfirmBlockerReset`, which clears the state and re-applies the deferred hollowing settings
  change (`page.tsx:10313`).

#### Sub-surface: destructive transform

- Renders `DestructiveTransformModal` with `isOpen={pendingDestructiveTransform !== null}`,
  `modelName={pendingDestructiveTransform?.modelName ?? null}`,
  `supportCount={pendingDestructiveTransform?.supportCount ?? 0}`,
  `operationLabel={pendingDestructiveTransform?.operationLabel ?? 'Transform'}`,
  `onCancel={handleCancelDestructiveTransform}`, `onConfirm={handleConfirmDestructiveTransform}`.

#### Sub-surface: modifier-apply progress overlay

- When `showModifierApplyBlockingOverlay`, `BlockingOverlay` with `zIndexClassName="z-[121]"`,
  `title={modifierApplyOverlayContent.title}`, `details={modifierApplyOverlayContent.detailLines}`,
  ``elapsed={`Elapsed: ${modifierApplyOverlayElapsedLabel}`}``,
  `footnote={modifierApplyProcessingLabel}`, `progress={null}`,
  `progressLabel={modifierApplyOverlayContent.title}`.

- **Port target**: `gpui/src/features/mesh_modifiers/modifier_dialogs.rs`. Nothing exists yet.

### Surface: `MeshRepairModals` — damaged-mesh dialog, repair confirm, manual repair run, reports

- **Source**: `src/components/organisms/modals/MeshRepairModals.tsx:122` (props at 108)
- **Trigger**: rendered at `page.tsx:11881`. Opens are driven by
  `showDamagedModelDialog`, `scene.meshRepairConfirmPrompt`, `manualRepairModelIds`,
  `scene.meshRepairReports`.
- **Renders**: the damaged-mesh `StructuredDialogModal`, the out-of-scope `MeshRepairConfirmModal`,
  a hand-written manual-repair dialog, and the out-of-scope `MeshRepairReportModal`.
- Escape: `useEscapeToClose(manualRepairModelIds !== null, isManualRepairing ? undefined : () => setManualRepairModelIds(null))`
  — comment: `// Escape mirrors the backdrop click: it cannot leave mid-repair.`
- Own state: `manualRepair: ManualRepairState | null`, `repairToken: string | null`,
  `cancelling: boolean`, `tick: number`.
  `ManualRepairState = { startedAt, modelName, modelIndex, modelCount, stage, stages }`.

#### Sub-surface: "Mesh quality too low"

- `open={showDamagedModelDialog}`, `ariaLabel={_(msg`Mesh boolean operation failed`)}`,
  title `Mesh quality too low`, subtitle `Boolean operation requires a manifold mesh`,
  **`iconTone="danger"`**, `icon={<AlertTriangle className="h-4 w-4" />}`, **`closeDisabled`** with
  `onClose={() => setShowDamagedModelDialog(false)}` → the X is rendered but disabled and Escape is
  swallowed. Single action `Got it` (tinted-accent) closing the dialog.
- Body `space-y-2`, two paragraphs: `This mesh contains too many self-intersecting triangles and non-manifold edges for boolean operations to succeed. The import repair pass reduced but could not fully resolve all issues in the geometry.`
  and `We recommend repairing the mesh in a dedicated 3D modeling program such as **Blender** or **Netfabb** before importing it into DragonFruit.`
- Raised from `useHolePunchManager` (`:1237`) and `useHollowingManager` (`:306`) when a voxel boolean
  removes zero voxels.

#### Sub-surface: import repair confirm

- Renders `MeshRepairConfirmModal` when `scene.meshRepairConfirmPrompt` is set
  (`{fileName, analysis, canRepairParts}`), wiring the four choices to
  `scene.resolveMeshRepairConfirmPrompt('repair' | 'repair_parts' | 'load_as_is' | 'cancel_import')`.

#### Sub-surface: manual "Repair Mesh" dialog (hand-written shell)

- **Source**: `MeshRepairModals.tsx:246`
- Open iff `manualRepairModelIds` is non-null **and** at least one id still exists in `scene.models`
  (otherwise it renders nothing). `repairModels = scene.models.filter(m => manualRepairModelIds.includes(m.id))`;
  `repairModelNames` = their names joined by `', '`; `isMultiple = repairModels.length > 1`.
- Backdrop `fixed inset-0 z-[220] flex items-center justify-center bg-black/55 backdrop-blur-sm px-3`;
  mousedown closes only when `target === currentTarget && !isManualRepairing`.
  Panel `w-full max-w-lg overflow-hidden rounded-xl border shadow-2xl` on `var(--surface-0)`;
  `role="dialog" aria-modal="true"`,
  `aria-label={autoStartRepair ? _(msg`Repairing Mesh`) : _(msg`Repair Mesh`)}`.
- Header `flex items-center justify-between gap-4 border-b px-5 py-4`; icon `IconChip size="lg" icon={AlertTriangle}`
  with amber inline styling (`#d97706` mixes at 45%/88%); title is
  `autoStartRepair ? "Repairing Mesh" : formatRepairMeshesTitle(_, count)` →
  `Repair Mesh` / `Repair # Meshes`; subtitle `autoStartRepair ? "Fixing geometry issues." : "Fixes geometry issues before you keep working."`.
  Close `IconButton variant="surface" size="md"` aria-label `Close repair mesh dialog`,
  `disabled={isManualRepairing}`.
- Body `space-y-4 p-5`:
  1. model card `rounded-md border px-3 py-2` on `var(--surface-1)`; eyebrow
     `Models` (msg context `'Dialog field'`) when `isMultiple` else `Model`; value
     `text-sm font-semibold truncate` with `title={repairModelNames}`.
  2. `manualRepair ? <RepairProgressPanel …/> : <p>Repairs holes, broken surfaces, and other mesh problems.</p>`.
  3. amber warning box (`#d97706` 40%/92% mixes) with `AlertTriangle mt-0.5 h-3.5 w-3.5 shrink-0` and the
     sentence `The repaired result will replace this model in your current scene. …` (plural form
     "results … these models" when `isMultiple`).
  4. Action row: class is `autoStartRepair || isManualRepairing ? 'pt-1' : 'grid grid-cols-2 gap-2 pt-1'`
     — i.e. one full-width button while a run is under way.
     - Left/secondary button (`w-full inline-flex items-center justify-center gap-1.5 disabled:opacity-60`,
       `disabled={cancelling}`): label is `Stopping…` (with `Spinner size="sm"`) while `cancelling`,
       else `Stop` while `isManualRepairing`, else `Keep Original`. Click: if repairing →
       `if (!repairToken) return;` set `cancelling` and `void cancelMeshRepair(repairToken)`
       (comment: `// Stops at the next stage boundary, so the mesh it leaves is` /
       `// one the pipeline finished a pass on.`); otherwise `setManualRepairModelIds(null)`.
     - Right button `Repair` (`variant="tinted-accent"`, `Wrench h-3.5 w-3.5`): rendered
       **only when `!autoStartRepair && !isManualRepairing`** (comment: `// Once the run is under way Stop is the only thing to press: a second button` /
       `// announcing the repair it is already doing, disabled, is noise beside it.`).
- `RepairProgressPanel`: model position line (`mx-1.5 truncate text-[11px]`) shown only when
  `modelCount > 1 && modelName`, formatted `Model {position} of {count}: {modelName}`; a row with the
  stage label (`mx-1.5 min-w-0 truncate text-[11px] font-medium`, `var(--text-strong)`) and elapsed
  (`shrink-0 text-[11px] tabular-nums`); track `mx-1.5 h-1.5 overflow-hidden rounded-full` on
  `var(--surface-2)` with `role="progressbar"`, `aria-label={_(msg`Repair progress`)}`; fill
  `h-full rounded-full transition-[width] duration-300 ease-out`, `background: '#d97706'`.
  Stage label: cancelling → `Stopping at the end of this step…`, else
  `translateRepairStage(state.stage?.name, _)`.
- Progress fraction (quote the comment): `// The stage count is not known in advance: the pipeline branches on what it finds in` /
  `// the mesh, so a percentage of the stages would be invented. A saturating function of the` /
  `// count always moves and never claims the work is nearly done, which is the honest` /
  `// thing to show when the total is unknown.`
  ```ts
  const modelFraction = state.stage ? 0.1 + 0.8 * (1 - Math.exp(-state.stages.length / 6)) : 0.06;
  return (state.modelIndex + modelFraction) / Math.max(1, state.modelCount);
  ```
- Elapsed: `formatElapsed` → `m:ss.s` with `` `${minutes}:${seconds.toFixed(1).padStart(4, '0')}` ``
  (e.g. `0:04.2`); a 100 ms interval updates `tick` only while `isManualRepairing`.
- Run start: `runRepair()` filters `manualRepairModelIds` to ids that still exist, returns early when
  empty, then `setIsManualRepairing(true)`, resets `cancelling`, seeds the state, and calls
  `scene.repairModelsInPlace(ids, { onToken, onModelStart, onProgress })`; `finally` clears
  `isManualRepairing`, `manualRepairModelIds`, the state, the token and `cancelling`.
- Auto-start: an effect with a ref key `manualRepairModelIds.join('\0')` runs `runRepair()` once when
  `autoStartRepair` and ids are present (comment: `// Repair Now: the model arrives with the flag already set, so the repair starts here rather` /
  `// than waiting for a click on a Repair button the user has effectively already pressed.`).
  A second effect clears `autoStartRepair` whenever `manualRepairModelIds === null`
  (comment: `// Every way out of the dialog clears the flag, so the next repair opened by hand waits.`).
- Triggers (page): model-row repair button → `handleRepairModel(modelId)` sets
  `autoStartRepair=false` and the id list (whole selection when the row is part of a multi-selection);
  `ManifoldWarningModal`'s `Repair Now` sets `autoStartRepair=true` with the flagged model.
- **Port target**: `gpui/src/features/mesh_health/repair_dialogs.rs`. Nothing exists yet.

### Surface: `PrintingModals` — slicing, upload and printer-monitor dialogs

- **Source**: `src/components/organisms/modals/PrintingModals.tsx:261` (props at 47 — 150 props, all
  page state; helpers at 216–258)
- **Trigger**: rendered at `page.tsx:11513`.
- **Renders**, in order: `SliceCompletedModal`, `UvToolsLaunchingModal`, the monitor confirmation
  dialog, `PrintingResliceModal`, the pre-slice print-readiness dialog, the printing target picker,
  the upload dialog, the printer monitor, the monitor debug overlay (`Ctrl+Shift+N`) and the RTSP
  debug overlay (`Ctrl+Shift+M`).

#### Frame: Escape registrations (exact, in order)

```ts
useEscapeToClose(Boolean(printingMonitorPendingConfirmation), () => setPrintingMonitorPendingConfirmation(null));
useEscapeToClose(preSlicePrintConfirmOpen, cancelPreSlicePrintConfirm);
useEscapeToClose(printingTargetPickerOpen, printingSendBusy ? undefined : cancelPrintingTargetPicker);
useEscapeToClose(
  printingUploadDialogOpen,
  (printingUploadDialogStage === 'failed' || printingUploadDialogStage === 'started' || printingUploadDialogStage === 'ready')
    && !printingSendBusy
    && !printingPrintNowBusy
    ? () => setPrintingUploadDialogOpen(false)
    : undefined,
);
useEscapeToClose(printingMonitorModalOpen, () => setPrintingMonitorModalOpen(false));
```
Comment kept: `// Escape closes the top-most of these dialogs; the upload dialog only while` /
`// its Close button is available, and never while a send is in flight.`

Two cancel helpers:
- `cancelPreSlicePrintConfirm`: closes, then answers `preSlicePrintConfirmResolverRef.current(false)`
  and nulls the ref.
- `cancelPrintingTargetPicker`: closes; when `isPreSliceTargetPicker` answers the pre-slice resolver
  with `null`; always resets `setPrintingTargetPickerMode('post-slice')`.

#### Wiring: slice completed and UVTools

- `SliceCompletedModal` gets `isOpen={showSliceCompletedModal}`,
  `onClose={() => setShowSliceCompletedModal(false)}`, `filePath={sliceCompletedModalData.filePath}`,
  `slicingTimeMs={sliceCompletedModalData.slicingTimeMs}`, and
  `onOpenInUvTools` **only when `getSavedUvToolsSettings().enabled`** — in which case it launches
  `resolveUvToolsExecutablePath(settings)` with the path and logs
  `'[UVTools] Failed to launch from completed dialog:'` on failure.
- `UvToolsLaunchingModal` gets `isOpen={uvToolsLaunchingPath !== null}` and `filePath={uvToolsLaunchingPath}`.

#### Sub-surface: monitor confirmation (control / plate actions)

- **Source**: `PrintingModals.tsx:478`
- Open iff `printingMonitorPendingConfirmation`
  (`{kind:'control', action:'cancel'|'emergency-stop'}` or
  `{kind:'plate', action:'start'|'delete', plateId, plateName}`).
- `StructuredDialogModal` with `open`, `zIndexClassName="z-[220]"`,
  `iconTone="warning"`, `icon={<AlertTriangle className="h-4 w-4" />}`,
  `closeAriaLabel={_(msg`Close monitor confirmation modal`)}`, X and backdrop clear the confirmation,
  **`bodyClassName="p-4 space-y-3"`**, no `actions` prop.
- Title / subtitle / aria by case:

  | kind + action | title | subtitle | aria |
  |---|---|---|---|
  | control + cancel | `Cancel Print Job` | `This action cannot be undone.` | `Confirm cancel print` |
  | control + emergency-stop | `Emergency Stop` | `This will immediately halt the printer.` | `Confirm emergency stop` |
  | plate + start | `Start Recent Print File` | `Start this recent file on the selected printer now?` | `Confirm start recent file` |
  | plate + delete | `Delete Recent Print File` | `This will remove the file from the printer.` | `Confirm delete recent file` |

- Body: for `kind === 'plate'` a file card (`rounded-md border px-3 py-2`, eyebrow `File`, value
  `` `#${plateId} • ${plateName}` `` truncated, `title` = same string). Then always a printer card
  (eyebrow `Printer`, value `monitoringDevice?.displayName || hostName || ipAddress || _(msg`Selected printer`)`).
  Then a paragraph: control+cancel → `Canceling will stop the current print job and clear queued progress for this plate.`;
  control+emergency-stop → `Emergency Stop is for immediate intervention and should be used only when necessary.`;
  plate+start → `The selected plate will begin printing immediately on this machine.`;
  plate+delete → `Deleted files cannot be restored from this monitor.`
- Buttons `grid grid-cols-2 gap-2 pt-1`: dismiss label is `Keep File` for `kind==='plate'`, else
  `Keep Printing` (secondary, `w-full`); confirm label is `Confirm Start` / `Confirm Delete` (plate)
  or `Confirm Cancel` / `Confirm Emergency Stop` (control).
  Confirm button uses inline styles only (no variant): plate+start → green
  (`#22c55e` mixes 45% border / 84% bg / `color: 'color-mix(in srgb, #22c55e, var(--text-strong) 25%)'`);
  plate+delete → red (`#ef4444` 40% / 78% / 25%); control+cancel → amber
  (`#f59e0b` 45% / 86% / 20%); control+emergency-stop → the same red pair as delete.
- On confirm: clears the pending object first, then
  `executePrintingMonitorControlAction(action)` for control, or
  `executeStartMonitorRecentPlate(plateId)` / `executeDeleteMonitorRecentPlate(plateId)` for plates.
  Dismiss does nothing else.

#### Wiring: printing reslice

- `PrintingResliceModal` with `isOpen={showPrintingResliceModal}`;
  `onCancel` → close then `scene.setMode(modeBeforePrintingRef.current)`;
  `onResliceNow` → close, `shouldReturnToPrintingAfterSliceRef.current = true`,
  `setShouldAutoSliceOnExportEntry(true)`, `scene.setMode('export')`.

#### Sub-surface: pre-slice print readiness ("Safety Check")

- **Source**: `PrintingModals.tsx:618`
- Open iff `preSlicePrintConfirmOpen`; raised at the end of the pre-slice gate when the user chose
  `intent === 'print'` (`page.tsx:4287`) and awaited as a promise.
- `StructuredDialogModal` with `open`, `zIndexClassName="z-[220]"`,
  `ariaLabel={_(msg`Print readiness confirmation`)}`, `iconTone="warning"`,
  `icon={<AlertTriangle className="h-4 w-4" />}`,
  `closeAriaLabel={_(msg`Close print readiness confirmation`)}`, X/backdrop → `cancelPreSlicePrintConfirm`,
  **`actionsClassName="flex items-center justify-end gap-2 pt-1"`** — but note the buttons are written
  inside the children block, so the `actions` slot is unused and that class has no effect.
- Title is a fragment: an eyebrow `<span className="block text-[11px] font-normal uppercase tracking-wide" …>Safety Check</span>`
  above `Confirm printer is ready to print`.
- Body: `Please verify before continuing:` then a checklist card
  (`rounded-md border p-3 space-y-2` on `var(--surface-1)`) of three rows, each
  `flex items-start gap-2 text-xs` with a `CheckCircle2 mt-0.5 h-3.5 w-3.5 shrink-0` tinted
  `color: 'color-mix(in srgb, #22c55e, var(--text-strong) 18%)'`:
  1. `Build plate and resin vat are properly seated and secured.`
  2. `Resin is mixed, sufficient for the print, and at operating temperature.`
  3. `Build plate is clean and clear, and the printer cover is fully closed.`
- Action row `flex items-center justify-end gap-2 pt-1`: `Cancel` (secondary) → `cancelPreSlicePrintConfirm`;
  `Continue to Slicing` (tinted-accent, `gap-1.5`) → close, then
  `preSlicePrintConfirmResolverRef.current(true)` and null the ref.

#### Sub-surface: printing target picker (fleet upload)

- **Source**: `PrintingModals.tsx:680`
- Open iff `printingTargetPickerOpen`. Triggered from:
  the pre-slice gate when more than one reachable printer exists or a remote material must be chosen
  (`page.tsx:4257`, awaited); `handleSendToPrinter` when the selected material's layer height does not
  match (`page.tsx:5562`); and `PrintingPanelStack` (`src/components/organisms/panels/PrintingPanelStack.tsx:72`).
- Markup: **`absolute inset-0 z-[120]`** (not `fixed`), backdrop
  `flex items-center justify-center bg-black/55 backdrop-blur-sm px-4`; panel
  `w-full max-w-3xl overflow-hidden rounded-xl border shadow-2xl` on `var(--surface-0)`,
  `role="dialog" aria-modal="true"`, `aria-label={_(msg`Choose printer`)}`.
  Header `border-b px-4 py-3` with eyebrow `Pre-Slice Targeting` or `Fleet Upload` and title
  `Choose target before slicing` or `Choose target printer`; **no close X**.
  Body `p-4 space-y-3.5`.
- Layout: two columns only when `requiresRemoteMaterialSelectionForUpload`:
  `` `grid gap-3 md:items-start ${requiresRemoteMaterialSelectionForUpload ? 'md:grid-cols-2' : 'md:grid-cols-1'}` ``.
  Each column card is `rounded-md border px-3 py-2.5 min-h-[360px]` on `var(--surface-1)`, with a
  `text-[11px] mb-2` label and an inner `max-h-[318px] overflow-y-auto custom-scrollbar pr-1 space-y-2`.
- Printer rows (`printableConnectedPrinterFleet`), `button` `relative w-full rounded-lg border px-3 py-2.5 pr-9 text-left`:
  offline → `borderColor: 'color-mix(in srgb, var(--border-subtle), black 18%)'`,
  `background: 'color-mix(in srgb, var(--surface-1), black 8%)'`, `opacity: 0.55`, disabled;
  selected → accent border `…28%)` and `background: 'color-mix(in srgb, var(--accent), var(--surface-1) 89%)'`;
  otherwise `var(--border-subtle)` / `color-mix(in srgb, var(--surface-1), black 3%)`.
  Row content: name `text-[15px] font-semibold leading-tight` (`displayName || hostName || ipAddress`),
  second line `` `${device.ipAddress} • ${isOffline ? 'Offline' : 'Online'}` ``. Offline rows show an
  absolute `Offline` tag; selected rows show an absolute green `CheckCircle2 h-4 w-4` chip
  (aria-label `Selected printer`, `title="Selected"`).
  Selecting writes `setPrintingTargetDeviceId(device.id)` and, when a profile is active,
  `selectPrinterNetworkDevice(activePrinterProfile.id, device.id)`.
- Material column (only when required): label `Target material` (pre-slice) or
  `Target material (matching sliced layer height)`; loading → `Loading materials from selected printer…`;
  empty → `printingTargetMaterialError ?? _(msg`No matching material profile found on this printer.`)`.
  Groups render an optional uppercase group label and rows `relative w-full rounded-md border px-2.5 py-2 pr-9 text-left`
  (selected = accent `…32%` / `…90%`); name `text-[13px] font-medium truncate` with `title`;
  optional layer height `fmt.quantity(layerHeightMm, 3, 'mm')`; selected shows the same green check chip
  (aria `Selected material`). Selecting writes `setPrintingTargetMaterialId(id)` and
  `upsertPrinterNetworkDevice(activePrinterProfile.id, { id, ipAddress, selectedMaterialId, selectedMaterialName, selectedMaterialLayerHeightMm }, { select: true })`.
- Extra lines: when `requiresRemoteMaterialSelectionForUpload && !isPreSliceTargetPicker`,
  `Target layer height: {fmt.quantity(slicedLayerHeightMm, 3, 'mm')}`; a material error line when
  options exist; and `Selected printer is offline. Choose an online printer to continue.` when the
  chosen device is offline.
- Actions `flex items-center justify-end gap-2 pt-1`: `Cancel` (secondary, `disabled={printingSendBusy}`)
  → `cancelPrintingTargetPicker`; primary `Continue to Slicing` (pre-slice) or
  `Upload to Selected Printer` (tinted-accent, `gap-1.5`). Primary is disabled when
  `printingSendBusy || isPrintingTargetMaterialsLoading || !printingTargetDevice ||
  (requiresRemoteMaterialSelectionForUpload && !printingTargetMaterialId) ||
  printerReachabilityByDeviceId[printingTargetDevice.id] === false`.
  On click: close the picker; if pre-slice, answer the resolver with
  `{ deviceId, materialId: requiresRemoteMaterialSelectionForUpload ? printingTargetMaterialId : undefined }`,
  null the ref and reset mode to `'post-slice'`; otherwise reset mode and
  `void performSendToPrinter(printingTargetDevice, materialId?)`.

#### Sub-surface: printing upload dialog

- **Source**: `PrintingModals.tsx:944`
- Open iff `printingUploadDialogOpen`; set by the send path (`page.tsx:5339`) with stage `'uploading'`.
- Markup: **`absolute inset-0 z-[121]`** backdrop `flex items-center justify-center bg-black/55 backdrop-blur-sm px-4`;
  panel `w-full max-w-xl overflow-hidden rounded-xl border shadow-2xl` on `var(--surface-0)`,
  `role="dialog" aria-modal="true" aria-live="polite"`,
  `aria-label={_(msg`Printer upload status`)}`; **no close X** (closing is the Close button / Escape when allowed).
- Header: eyebrow `Post-Processing`; title `Upload to {activeNetworkUiAdapter?.displayName ?? _(msg`Printer`)}`;
  filename `printingArtifact?.outputName ?? _(msg`Preparing artifact`)` (`text-xs truncate`).
- Body `p-4 space-y-3`:
  - a 2-column grid of three cards (`rounded-md border px-3 py-2` on `var(--surface-1)`, label
    `text-[10px] uppercase tracking-wide`): `Stage` = `printingDialogStageLabel`;
    `Target printer` = `displayName || hostName || ipAddress || _(msg`Pending`)` (truncated, `title`);
    `Plate` (msg with comment `'The file slot on the printer a print is uploaded to.'`) =
    `printingReadyPlateId ? \`#${id}\` : _(msg`Pending`)`.
  - status line (`text-xs min-h-[18px]`) = `printingSendStatusText ?? _(msg`Preparing upload pipeline…`)`.
  - stage `'started'`: note card `Print started. Use **Monitor** in the top bar to view live progress and webcam.`
  - stage `'uploading'` with telemetry: three small cards `Speed` / `Remaining` / `Transferred`
    (values with `fontVariantNumeric: 'tabular-nums'`).
  - progress: when `printingDialogIsIndeterminate` (stage `'processing'`) → `ProgressBar indeterminate`
    + `Processing on {backend}… elapsed {elapsed}`; otherwise a raw track
    (`h-2.5 w-full rounded-full border overflow-hidden`,
    `background: 'color-mix(in srgb, var(--surface-2), black 20%)'`) with a fill
    `h-full rounded-full transition-[width] duration-200 ease-out`, width
    `` `${printingDialogProgressPercent.toFixed(2)}%` ``, gradient by stage — failed
    `linear-gradient(90deg, #ef4444, #f97316)`, started
    `linear-gradient(90deg, #60a5fa, #22d3ee)`, else
    `linear-gradient(90deg, var(--accent), color-mix(in srgb, var(--accent), #ffffff 28%))`.
  - footer row: left caption `Waiting for metadata readiness` (stage processing) else `Transfer progress`;
    right percentage (`font-semibold`, `var(--text-strong)`) = `—` when indeterminate, else
    `fmt.quantity(pct, 0, 'percent')`.
  - action row `pt-1 flex items-center justify-end gap-2`:
    - `Close` (secondary) for stages `failed` / `started` / `ready`, disabled while
      `printingSendBusy || printingPrintNowBusy`.
    - `Retry Upload` (tinted-accent, `gap-1.5`) on `failed` → `void handleSendToPrinter()`, disabled when
      `printingSendBusy || printingPrintNowBusy || !canSendToPrinter`.
    - `Start Print` / `Starting print…` (tinted-accent) on `ready` → `handlePrintNow`, disabled when
      `!canPrintNow || printingPrintNowBusy || printingSendBusy`.
    - `Open Monitor` (tinted-accent) on `started` → `openPrintingMonitorForTargetDevice(printingTargetDevice?.id ?? null)`.
- Stage labels (`page.tsx` memo, when no explicit stage text): `Uploading to printer`, `Processing on device`,
  `Ready to print`, `Starting print`, `Print started`, `Upload failed`, fallback `Processing`.

#### Sub-surface: printer monitor (`z-[140]`)

- **Source**: `PrintingModals.tsx:1133`
- Open iff `printingMonitorModalOpen`; opened by the top-bar Monitor button
  (`page.tsx:10713` → `setPrintingMonitorModalOpen(true)`), by `Open Monitor` in the upload dialog, and by
  `openPrintingMonitorForTargetDevice(deviceId)` (`usePrintingMonitorManager.ts:2694`, which sets the device
  and forces `'detail'` view).
- Markup: wrapper `fixed inset-0 z-[140] flex items-center justify-center p-4` with `role="presentation"`,
  and the backdrop is a real `<button className="absolute inset-0 bg-black/55" aria-label={_(msg`Close printer monitor`)}>`.
  Panel `` `relative z-[1] ${printingMonitorModalWidthClass} max-h-[88vh] overflow-auto rounded-xl border shadow-2xl` ``
  with `borderColor: 'var(--border-subtle)'`,
  `background: 'color-mix(in srgb, var(--surface-0), #000 10%)'`,
  `role="dialog" aria-modal="true" aria-label={_(msg`Printer monitor`)}`.
- Header `flex items-center justify-between border-b px-4 py-3`:
  - Dashboard mode: a static chip (`inline-flex items-center gap-2 px-1.5 py-1`) with a 7×7 `LayoutGrid`
    tile (`background: 'color-mix(in srgb, #baf72e, var(--surface-1) 90%)'`,
    `border: '1px solid color-mix(in srgb, #baf72e, var(--border-subtle) 45%)'`,
    `color: 'var(--accent-secondary)'`) and two lines: `Monitoring Dashboard`
    (`text-[10px] tracking-[0.01em]`) and `Fleet Status Overview` (`text-[11px] font-semibold`).
  - Detail mode with more than one selectable device: a dropdown trigger
    (`group inline-flex items-center gap-2 rounded-md px-1.5 py-1 text-sm font-semibold`, transparent bg)
    with a 7×7 printer-thumbnail tile (`color-mix(in srgb, var(--surface-1), transparent 6%)`), the
    top label (`printingMonitorHeaderTopLabel`, either `truncate text-[10px] tracking-[0.01em]` or
    `text-[9px] uppercase tracking-[0.11em]` when `printingMonitorHeaderUsesFleetLabelOrder` is false),
    the bottom label (`text-[11px] font-semibold`), and a `ChevronDown h-3.5 w-3.5` that rotates 180°
    when open. With one device it renders as a plain (non-clickable) row.
    Menu: `absolute left-0 top-full z-20 mt-2 w-[min(360px,82vw)] rounded-lg border p-1.5 shadow-xl` on
    `color-mix(in srgb, var(--surface-0), #000 8%)`; rows `w-full rounded-md border px-2.5 py-2 text-left`
    (offline: `color-mix(in srgb, var(--border-subtle), black 18%)` border,
    `color-mix(in srgb, var(--surface-1), black 8%)` bg, `opacity: 0.55`, disabled; selected: accent
    `…35%` / `…90%`; else `var(--border-subtle)` / `var(--surface-1)`). Selecting →
    `setPrintingMonitorDeviceId(device.id)` and closes the menu. Name falls back through
    `displayName → hostName → ipAddress → formatFallbackPrinterName(_, id)` (`Printer {id}`).
  - Right side: a fleet toggle `Button variant="secondary" size="sm" className="gap-1"` labelled
    `Detail View` / `Dashboard View` (rendered only when `hasPrintingMonitorFleet`; it also closes the
    printer menu) with `LayoutGrid w-3.5 h-3.5` and a `title` (`Switch to detailed single-printer view`
    / `Switch to dashboard view for all fleet printers`), then the close
    `IconButton variant="solid" size="sm"` aria-label `Close printer monitor`, `title="Close monitor"`.
- Body selection order: `printingMonitorViewMode === 'dashboard'` → dashboard;
  else `shouldShowPrintingMonitorSlowResponseCard` → slow-response card;
  else `isPrintingMonitorSelectedPrinterOffline` → offline card;
  else the detail view.

##### Monitor body: dashboard

- Wrapper `p-5`; with devices, a scroll box `overflow-y-auto custom-scrollbar pr-1` with inline
  `height: 'clamp(34rem, 66vh, 42rem)'` containing
  `grid gap-3 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 auto-rows-max content-start`.
- Card (`dashboardMonitorDevices`): `group w-full rounded-lg border overflow-hidden transition-shape hover:shadow-sm text-left`
  on `var(--surface-1)`; `cursor: isOffline ? 'not-allowed' : 'pointer'`; `role="button"` and
  `tabIndex` 0 only when online; `title`/`aria-label` = `formatPrinterOffline(_, display)` (`{printer} is offline`)
  or `formatOpenDetailedMonitor(_, display)` (`Open detailed monitor for {printer}`). Click or
  `Enter`/`Space` sets the device and switches to `'detail'`.
- Card thumb: with `imageDataUrl`, a `relative h-28 overflow-hidden` box on
  `linear-gradient(135deg, color-mix(in srgb, var(--surface-2), black 30%), var(--surface-1))` with an
  `<img class="h-full w-full object-cover">` (offline filter
  `grayscale(100%) sepia(0.25) brightness(0.94)`; on error the image is hidden via
  `e.currentTarget.style.display = 'none'`). Without an image, the same box plus a
  `Printer h-8 w-8 opacity-40`. Online cards show a hover-only overlay
  (`pointer-events-none absolute inset-0 … opacity-0 … group-hover:opacity-100`,
  `background: 'color-mix(in srgb, #000, transparent 55%)'`) with the pill
  `Click to show Detailed View` (`rounded-md border px-2 py-1 text-[10px] font-semibold tracking-wide`,
  border `color-mix(in srgb, #baf72e, var(--border-subtle) 55%)`, `color: '#d9ff8f'`,
  `background: 'color-mix(in srgb, #1f2937, transparent 35%)'`).
- Card body `p-3 space-y-2`: name `text-[13px] font-semibold leading-tight` (truncated, `title`);
  IP line `text-[10px]` = `device.ipAddress || _(msg`No IP`)`; status pill
  `inline-flex h-6 items-center rounded-full border px-2.5 text-[10px] font-semibold whitespace-nowrap`
  labelled `Offline` / `Paused` / `Printing` / `Idle` with the colour quartet
  (offline `#ef4444` / text `#fecaca`; paused `#f59e0b` / `#fde68a`; printing `brandColor = '#baf72e'`;
  idle `idleColor = '#60a5fa'` / `#bfdbfe`) and surface mixes at 90/90/92/88%.
  State text line `text-[11px] leading-tight` = `isOffline ? 'Offline' : (snapshot?.stateText?.trim() || 'Status unavailable')`.
- Progress block (always rendered, `space-y-2 min-h-[34px]`): track `h-2.5 w-full rounded-full border`
  on `color-mix(in srgb, var(--surface-2), black 25%)`; fill width
  `` `${(displayProgressPct ?? 0).toFixed(1)}%` ``; fill background — paused
  `linear-gradient(90deg, #f59e0b, color-mix(in srgb, #f59e0b, #fde68a 35%))`, printing
  `linear-gradient(90deg, #baf72e, color-mix(in srgb, #baf72e, #52cc80 50%))`, else
  `color-mix(in srgb, var(--text-muted), transparent 78%)`. Caption row: left
  `formatLayerProgress(_, displayLayerText)` → `Layer {layers}`, right percentage
  (`fmt.quantity(pct, 0, 'percent')`) or `-`, coloured `#fde68a` / `#baf72e` / muted.
- Progress maths: `progressPct = clamp0-100(((max(0, currentLayer - 1)) / totalLayers) * 100)` when both
  are known; `totalLayers` is null unless `> 0`; `displayLayerText` is
  `` `${cur}/${total}` `` or `${cur}` or `—`, and `-/-` when there is no active print. Paused counts as
  an active print for display, and "printing" excludes paused.
- Empty fleet: a `rounded-lg border p-6 text-center` panel with `Printer h-8 w-8 mx-auto mb-2 opacity-40`,
  `No printers available`, `No networked printers with valid IP addresses were found in this fleet`.

##### Monitor body: slow response

- `p-4` > frame `h-[min(62vh,520px)] rounded-xl border` on `color-mix(in srgb, var(--surface-1), #000 4%)`
  > centered card `max-w-md w-full rounded-xl border px-5 py-5 text-center` with
  `borderColor: 'color-mix(in srgb, #f59e0b, var(--border-subtle) 56%)'`,
  `background: 'color-mix(in srgb, #78350f, var(--surface-1) 72%)'`; an 11×11 icon tile with a
  `Spinner size="lg" icon={RefreshCw}`; heading `Printer is responding slowly`; text
  `formatReconnectGrace(_, sec)` → `We will keep trying to reconnect for another {sec}s. If reconnection fails, please verify the network configuration and confirm the printer is online.`;
  an indeterminate `ProgressBar` in a `mt-4 mx-auto w-[78%]` box.

##### Monitor body: selected printer offline

- Same frame geometry; card border
  `color-mix(in srgb, #f87171, var(--border-subtle) 56%)`, background
  `color-mix(in srgb, #7f1d1d, var(--surface-1) 72%)`; red `AlertTriangle h-5 w-5` tile; heading
  `This machine is currently offline`; text `Reconnect this printer in Network Settings, or choose a different online printer from the selector above.`;
  button `Open Network Settings` (secondary) → close the monitor then
  `openProfileSettingsModal('printer', { openNetworkSettings: true })`.

##### Monitor body: detail view

- Wrapper class:
  `` `p-4 grid grid-cols-1 items-start ${printingMonitorDetailWebcamExpanded ? 'gap-y-3 lg:gap-x-0' : 'gap-3'} ${printingMonitorUsesTwoColumnDetailLayout ? 'lg:items-stretch lg:[grid-template-columns:var(--printing-monitor-detail-columns)]' : ''}` ``
  with a CSS variable `--printing-monitor-detail-columns` set inline to
  `'minmax(0,1fr)'` when the webcam is expanded, else `'minmax(340px,1fr) minmax(420px,1fr)'`.
- Left column (`<section>` ref `printingMonitorLeftColumnRef`) renders only when
  `!printingMonitorDetailWebcamExpanded`; class
  `grid gap-3 grid-rows-[auto_1fr] overflow-hidden transition-[opacity,transform] duration-140 ease-out motion-reduce:transition-none opacity-100 translate-y-0`.
  - **Print files panel**: `w-full min-w-0 max-w-full overflow-hidden rounded-md border p-2` on
    `color-mix(in srgb, var(--surface-1), #000 4%)`. Header row
    `grid min-h-[34px] items-center gap-2 px-1` with columns
    `grid-cols-[1fr_auto]` when a print is active (label left, refresh right) or
    `grid-cols-[1fr_auto_1fr]` otherwise (label, Local/USB segment, refresh). Label is
    `Print Details` when active, else `Print Files`.
    Segment control (only when no active print): `relative inline-flex h-9 w-[132px] items-center rounded-lg border p-1 justify-self-center overflow-hidden`
    on `color-mix(in srgb, var(--surface-1), #000 12%)`,
    `boxShadow: 'inset 0 1px 0 color-mix(in srgb, #ffffff, transparent 94%)'`,
    `aria-label={_(msg`Print file source`)}`; sliding thumb `absolute bottom-1 left-1 top-1 rounded-md border`
    with `width: 'calc(50% - 4px)'` and `transform: printingMonitorPlatesStoragePath === '/usb/' ? 'translateX(100%)' : 'translateX(0)'`,
    accent border/bg mixes 32%/78%; two buttons `Local` (msg context `'Printer storage'`) and `USB`,
    each `relative z-[1] inline-flex h-7 min-w-0 flex-1 items-center justify-center rounded-md px-2.5 text-[11px] font-semibold tracking-[0.02em]`,
    coloured strong/muted by the current path, `title` `Show print files from local storage` /
    `Show print files from USB storage`, calling `handlePrintingMonitorStoragePathChange('/local/'|'/usb/')`.
    Refresh `IconButton` (`!p-1.5`) → `void refreshPrintingMonitorRecentPlates()`, disabled while
    `printingMonitorAnyActionBusy || isPrintingMonitorRecentPlatesLoading`; shows
    `Spinner size="sm" icon={RefreshCw}` while loading else `RefreshCw w-3.5 h-3.5`; `title`/aria
    `Refresh print files`.
  - Files viewport: `mt-1.5 w-full min-w-0 max-w-full rounded-md border overflow-hidden` on
    `color-mix(in srgb, var(--surface-1), #000 6%)`, inner `h-[clamp(220px,30vh,320px)] w-full`.
    - With an active print and a thumbnail URL: `relative h-full w-full overflow-hidden` with a loading
      overlay (`absolute inset-0 … text-[11px]`, 74%-wide indeterminate `ProgressBar` + `Loading thumbnail…`)
      and `<img class="absolute inset-0 h-full w-full object-contain object-center transition-opacity duration-150">`
      (`opacity: isPrintingMonitorThumbnailLoaded ? 1 : 0`, `loading="eager"`, `decoding="async"`,
      `fetchPriority="high"`, alt `Active print thumbnail`).
    - Otherwise the recent-plate list: each row `w-full min-w-0 overflow-hidden rounded-md border px-2 py-1.5`
      on `var(--surface-1)`; left block truncated four lines —
      `` `#${plateId} • ${plate.name}` `` (`text-[11px]`, strong),
      `plate.materialProfileName ?? _(msg`Material profile unavailable`)`,
      `Est. {time} • {material}` and `Area Σ {total} • Min {min} • Max {max}`
      (all `text-[10px]`, built through `formatPlateEstimate` / `formatPlateAreaStats`,
      with `—` for null numbers from `formatPrintingMonitor*` helpers);
      right block `flex w-[56px] shrink-0 items-center justify-end gap-1` with
      play (`#22c55e` 45%/86%/25% inline colours, `title` `Start plate #{id}`, disabled when
      `printingMonitorAnyActionBusy || printingMonitorHasActivePrint`) and
      delete (`#ef4444` 40%/78% + `color: '#fecaca'`, `title` `Delete plate #{id}`, disabled when
      `printingMonitorAnyActionBusy`); both call `event.stopPropagation()` then
      `void handleStartMonitorRecentPlate(id)` / `void handleDeleteMonitorRecentPlate(id)`.
    - Empty: centered `text-[11px]` block — `Loading recent print files…` while loading, else
      `printingMonitorRecentPlatesError`, else `No Files Found` (`font-semibold`, strong) plus a
      `Refresh` `Button variant="secondary" size="auto"` with
      `!h-8 !px-3 !py-0 !text-[11px] !font-semibold gap-1`.
  - **Status/control panel**: `rounded-md border p-3 space-y-3` on
    `color-mix(in srgb, var(--surface-1), #000 4%)`. Top row (`flex items-center justify-between gap-2 text-xs`):
    left `printingMonitorSnapshot?.stateText ?? _(msg`Polling printer status…`)`; right
    `Busy…` when a status request is in flight within the slow-response grace,
    else `Live`/`Idle` by `isPrintingMonitorPolling`.
    With an active print: a `h-2` track on `color-mix(in srgb, var(--surface-2), black 20%)` with a
    `linear-gradient(90deg, #60a5fa, #22d3ee)` fill at
    `` `${(printingMonitorDisplayProgressPct ?? 0).toFixed(2)}%` ``, then
    `Progress {pct,1,'percent'}`; otherwise `No active print.`
    A `grid grid-cols-2 gap-2 text-[11px]` of three cells (`rounded-md border px-2.5 py-2` on
    `var(--surface-1)`): `Layer:` + `{cur}/{total}` or `{cur}` or `—`; `Material:` +
    `printingMonitorDisplayMaterialProfile`; and a `col-span-2 truncate` `Job:` cell showing
    `snapshot?.jobName ?? '—'` when active (never otherwise).
    Controls `grid grid-cols-2 gap-2`:
    - Pause/Resume `Button` with inline styles only: enabled accent pair
      (`…45%` border, `…87%` bg, `var(--text-strong)`), disabled grey pair
      (`var(--border-subtle)`, `color-mix(in srgb, var(--surface-2), black 8%)`, `var(--text-muted)`, `opacity: 0.55`).
      Label: while animating, a `Spinner size="sm" icon={RefreshCw}` + one of
      `Resuming…` / `Pausing…` (chosen by `printingMonitorControlPendingAction === 'resume'`, then
      `snapshot?.isPaused && !isPrintingMonitorIsPauseTransition`, else pausing); otherwise
      `Resume` when paused else `Pause`. Click → `handlePrintingMonitorControlAction(paused ? 'resume' : 'pause')`.
    - Cancel `Button` (amber enabled pair `#f59e0b` 48%/88% + `color: '#fde68a'`; same disabled pair),
      label `Canceling…` with spinner while animating else `Cancel`; click →
      `handlePrintingMonitorControlAction('cancel')`.
    - Emergency Stop `Button className="col-span-2"` (red `#ef4444` 40%/78% + `color: '#fee2e2'`),
      label `Stopping…` while `printingMonitorControlPendingAction === 'emergency-stop' || printingMonitorActionBusy === 'emergency-stop'`,
      else `Emergency Stop`; click → `handlePrintingMonitorControlAction('emergency-stop')`.
  - **Webcam panel** (`<section>` ref `printingMonitorWebcamSectionRef`, rendered only when
    `printingMonitorHasCamera`): class
    `` `rounded-md border p-2 flex flex-col min-h-0 overflow-hidden self-stretch h-[min(62vh,520px)] lg:h-full transition-opacity duration-150 ease-out motion-reduce:transition-none ${printingMonitorDetailWebcamExpanded ? 'opacity-100' : 'opacity-[0.985]'}` ``.
    Header `grid min-h-[34px] grid-cols-[1fr_auto] items-center gap-2 px-1` with label `Webcam` and two
    `IconButton`s (`!p-1.5`): maximise/minimise (only when `printingMonitorCanExpandWebcam`, toggles
    `setPrintingMonitorWebcamExpanded`, title/aria `Collapse webcam view` / `Expand webcam view`) and
    save snapshot (`Download w-3.5 h-3.5` or `Spinner size="sm" icon={RefreshCw}`, disabled when
    `isPrintingMonitorWebcamSnapshotSaving || !printingMonitorWebcamUrl || !isPrintingMonitorWebcamLoaded`,
    title/aria `Save webcam snapshot`).
    With a URL and a load error: an error card
    (`rounded-md border p-4 … h-full` on `color-mix(in srgb, var(--surface-1), #000 7%)`) with a 12×12
    circular danger icon, `printingMonitorWebcamDisplayPresentation.title` / `.description`, and buttons
    `Reset stream slot` (only when `printingMonitorWebcamCanResetStreamSlot`, title
    `Ask the printer to disable any stale webcam stream before retrying`, label `Resetting stream…`
    while busy) and `Retry`, both `variant="secondary" size="auto" className="!h-8 px-2.5 text-[10px]"`.
    With a URL and no error: a viewport div (ref `printingMonitorWebcamViewportRef`) with
    `relative rounded-md border overflow-hidden h-full max-h-full max-w-full` on
    `color-mix(in srgb, var(--surface-1), #000 6%)`, a loading overlay until
    `isPrintingMonitorWebcamLoaded` (indeterminate bar + `Loading camera feed…`), and the feed:
    - relay mode (`printingMonitorWebcamUsesRelayWs`) → `RtspRelayCanvasPlayer` with
      `className="block h-full w-full object-contain transition-opacity duration-150"`,
      `opacity: loaded ? 1 : 0`, `transform: monitorWebcamTransform`, origin center.
      `onLoaded(ratio)` cancels the readiness check, resets the auto-retry count/timeout, normalises the
      aspect ratio (`normalizePrintingMonitorWebcamAspectRatio`, accepted range **0.45…2.4**, skipped when
      unchanged within `0.001`), marks loaded and clears the error.
      `onError(message)` cancels the readiness check and logs
      `'[Monitor/Webcam] rtsp-relay playback issue'`; if the message contains
      `'did not deliver any video data in time'` or `'websocket disconnected'` **and** the retry count is
      below `DEFAULT_RELAY_AUTORETRY_LIMIT`, it increments the count, sets the error text
      `` `Webcam stream stalled. Retrying (${attempt}/${DEFAULT_RELAY_AUTORETRY_LIMIT})…` `` and re-triggers
      after `DEFAULT_RELAY_AUTORETRY_DELAY_MS`; otherwise it stores the raw message.
    - else an `<img>` MJPEG feed (`alt="Printer webcam preview"`, same opacity/transform), `onLoad` →
      `schedulePrintingMonitorMjpegReadinessCheck(currentTarget)`, `onError` → cancel check, mark
      unloaded, error `The webcam image could not be loaded.`; `loading="eager"`, `decoding="async"`,
      `fetchPriority="high"`.
    The feed box gets an `aspectRatio` style only when `monitorWebcamDisplayAspectRatio != null`
    (`width: '100%', height: 'auto'` + `aspectRatio`), else `width/height: '100%'`.
    Without a URL: a status card (`h-full` on the same tint) with a tone-driven 12×12 circular icon
    (warning `#d97706`, error `var(--danger)`, neutral `RefreshCw` in muted) plus
    `printingMonitorWebcamStatusPresentation.title` / `.description` and the same two buttons.

##### Monitor debug overlay (`Ctrl+Shift+N`)

- Rendered inside the monitor panel when `isPrintingMonitorDebugOpen`; wrapped in
  `{/* i18n-ignore-start: monitor debug overlay (Ctrl+Shift+N), raw SDCP for developers */}` … `end`.
- Container `pointer-events-none fixed right-4 top-[5.25rem] z-[170] w-[min(760px,94vw)]`; inner panel
  `pointer-events-auto rounded-lg border p-2.5 font-mono text-[10px] leading-tight shadow-xl` on
  `color-mix(in srgb, var(--surface-0), black 14%)`, `fontSize: '10px'`.
- Header: `Monitor Debug Overlay (Ctrl+Shift+N)` (`text-xs font-semibold`, `fontFamily: 'var(--font-geist-mono)'`),
  a `Copy JSON` / `Copied` / `Copy Failed` button (text tracks `printingMonitorDebugCopyState`; title
  `Copy monitor debug bundle`) and `Close`.
- A 2-column key/value grid: `Printer`, `Device host` (IP or `n/a`), `Reachability`
  (`unknown`/`online`/`offline`), `Offline gate` (`true`/`false`).
- `Channel payloads`: `lg:grid-cols-3` of panels from `printingMonitorDebugPanels`; each panel is a
  `rounded-md border overflow-hidden` on `color-mix(in srgb, var(--surface-2), #000 8%)` with a header
  (channel name + `statusText`, red `#fca5a5` when `hasError`), a `<pre class="max-h-56 overflow-auto custom-scrollbar p-2 text-[10px] leading-[1.35]">`
  of `panel.json`, and a footer with `requestedAt ?? 'not requested'`.
- `Manual SDCP commands` (grid `sm:grid-cols-2 lg:grid-cols-3`), each a button
  `rounded-md border px-2 py-1 text-left text-[10px]` on `color-mix(in srgb, var(--surface-2), #000 6%)`:
  `Cmd 386 / Enable video stream` (disabled unless `printingMonitoringAdapter.operations?.webcamEnable`),
  `Cmd 386 / Disable video stream` (`webcamDisable`), `Cmd 387 / Enable timelapse`
  (`timelapseEnable`), `Cmd 387 / Disable timelapse` (`timelapseDisable`), `Cmd 320 / Fetch task history IDs`
  and `Cmd 321 / Fetch task detail records` (both disabled unless `printingMonitoringAdapter.pluginId === 'sdcp-v3'`).
  All are additionally disabled while `printingMonitorAnyActionBusy`. The task commands call
  `executePrintingMonitorSdcpDebugCommand({ operation: 'sdcp/task/history/list' | 'sdcp/task/details', label, channel: 'taskHistory' | 'taskDetails' })`.
  Status line below: `printingMonitorActionStatus ?? 'Use these commands to manually toggle SDCP device features.'`
- `Last SDCP response JSON`: header row with `operation` (`'No response yet'` when null) and an
  `HTTP {status} • transport-ok|transport-error • command-ok|command-error` summary (`'waiting'` when null),
  then a `<pre class="mt-1 max-h-40 overflow-auto custom-scrollbar whitespace-pre-wrap break-words …">`
  of `JSON.stringify({ httpStatus, httpOk, commandOk, error, payload }, null, 2)` or
  `'Click a command to inspect the response JSON.'`.
- Footer `Toggle: Ctrl+Shift+N`.

##### RTSP debug overlay (`Ctrl+Shift+M`)

- Rendered **outside** the monitor panel (still inside the organism's fragment) when
  `isPrintingMonitorRtspDebugOpen`, also inside `i18n-ignore` markers. Toggled by the debug hotkey
  `Ctrl+Shift+M` but only while the monitor is open (`page.tsx:6659`).
- Container `pointer-events-none fixed left-4 top-[5.25rem] z-[170] w-[min(620px,94vw)]`; panel identical
  to the monitor overlay.
- Header `RTSP Debug Overlay (Ctrl+Shift+M)` + `Close`.
- Key/value grid: `Mode` (`printingMonitorRtspDebugSummary.title`), `Source RTSP`,
  `Relay base`, `Final webcam URL` (each `n/a` when absent), `Transport path`
  (`RTSP relay websocket` / `Direct webcam URL` / `Unavailable`), `UDP source port`
  (`relayDebugTransport.serverPort`), `UDP destination port` (`clientPort`), then a spanning note
  `Source is the printer/server RTP port; destination is the DragonFruit/client RTP port.`,
  `Reclaim session`, `Reclaim status`, `Webcam status` (`webcamDisplayPresentation.title`, tooltip = its description).
- Footer block: the summary description, `Current feed note: {description}`, optional
  `Last Transport header: {…}`, then `Toggle: Ctrl+Shift+M`.

- **Port target**: `gpui/src/features/printing/printing_dialogs.rs` (the monitor wants its own
  `printer_monitor.rs`; the two debug overlays are developer surfaces and can be deferred, but their
  data contracts are listed above). Nothing exists yet.

### Surface: `meshRepairModals.dom-test.tsx` — behavioural contract test

- **Source**: `src/components/organisms/modals/__tests__/meshRepairModals.dom-test.tsx:58`
- **What it asserts** (render it as a test contract, not a UI surface):
  - `'a manual repair offers Stop alone while it runs'`: with `isManualRepairing: true` and
    `manualRepairModelIds: ['m1']`, some button's text is exactly `'Stop'`, and **no button label
    contains `'Repair'`**.
  - `'a manual repair offers both actions before it starts'`: with defaults, labels include both
    `'Keep Original'` and `'Repair'`.
- Fixture shape it depends on: `scene.models = [{ id: 'm1', name: 'plate.stl' }]`,
  `meshRepairReports: []`, `dismissMeshRepairReports`, `repairModelsInPlace: async () => true`.
  Rendering goes through `withProviders` and queries `document.body` because the dialog portals.
- **Port target**: equivalent Rust UI tests on the repair dialog's button set; the invariant is
  "Stop is the only action while a run is in flight".

---

## Deliberate decisions and gotchas

1. **`iconTone="accent"` is `--accent-secondary`, not `--accent`.** Reading the tone name as "the
   accent colour" gives the wrong hue on the slice-completed, remesh and shrink-wrap dialogs.
2. **A dialog with no `onClose` has no X.** `StructuredDialogModal` renders the close button only when
   `onClose` exists, yet Escape still resolves through `onClose ?? onBackdropClick`. The confirms that
   are meant to be answered explicitly pass **`onBackdropClick={() => {}}` and no `onClose`**
   (`TetrahedralRemeshConfirmModal`, `ShrinkWrapConfirmModal`, `ManifoldWarningModal`) so the backdrop
   is inert and there is no X — but note the Escape path then calls that empty handler and therefore
   *consumes* the key without closing. That is the intended "you must pick a button" behaviour.
3. **`closeDisabled` also swallows Escape.** In `UnsavedChangesDialog` and the damaged-mesh dialog the
   busy/disabled state turns Escape into a no-op rather than a cancel. Do not implement
   "Esc still closes" there.
4. **Dialogs that pass only `onBackdropClick` rely on the shell's fallback ordering.**
   `onBackdropClick` wins for backdrop clicks, while Escape prefers `onClose`. Where the two differ
   (`ScenePrinterMismatchModal` passes both, same handler) keep them identical.
5. **Escape ordering is mount order, last wins.** `useEscapeToClose` pushes onto a stack and the
   top-most entry `stopImmediatePropagation()`s. Nested/nearby dialogs must register in the same
   order the DOM mounts them, or one press closes the wrong one (or two).
6. **Several dialogs are hand-written, not `StructuredDialogModal`** — `ZipFilePickerModal`,
   `ModelSupportsModal`, `IslandHierarchyModal`, `DiagnosticsModal`, `HistoryDebugModal`,
   `UvToolsLaunchingModal`, the manual repair dialog, and the three import/save prompts inside
   `SceneFileModals`. They repeat the panel recipe by hand
   (`rounded-xl border shadow-2xl` + `var(--surface-0)` + `var(--border-subtle)` +
   `0 24px 46px rgba(0,0,0,0.42)`), and the two debug/telemetry modals use a different shadow
   (`0 28px 64px rgba(0,0,0,0.48)`) and a darker backdrop (`bg-black/60`). Do not unify them silently.
7. **`absolute` vs `fixed` is load-bearing for three printing surfaces.** The target picker
   (`z-[120]`) and upload dialog (`z-[121]`) are `absolute inset-0` — they anchor to the printing
   workspace, not the viewport — while every other dialog is `fixed`. The upload dialog sits above the
   picker by exactly one z-step so it can cover it without unmounting it.
8. **The printing upload dialog's `Close` is conditional.** It appears only for stages
   `failed | started | ready`, and Escape is registered with `undefined` (swallow) for every other
   stage and while a send or print-start is in flight. There is no X.
9. **`Upload to {printer}` chooses its name from the *adapter*,** `activeNetworkUiAdapter?.displayName`,
   while the "Target printer" card uses the *device* (`displayName || hostName || ipAddress`). The two
   can read differently.
10. **The progress bar is hand-rolled inside the upload dialog** (not `<ProgressBar>`), with three
    gradients and `toFixed(2)` widths. The indeterminate branch *is* `<ProgressBar indeterminate>`.
    Reproducing the determinate branch with the atom changes the gradient
    (`var(--accent) → #ff79c6` in the atom).
11. **`printingDialogIsIndeterminate` is only `stage === 'processing'`** — the only stage whose
    percentage is meaningless.
12. **`Start Print` is a separate call from uploading.** `ready` + `Start Print` → `handlePrintNow`
    (plugin `operations.start` with `plateId`); on success the page closes the dialog and opens the
    monitor, so the `started` branch of this dialog is mostly seen when the dialog was already open.
13. **The monitor's backdrop is a real `<button>`**, not a mousedown handler on the wrapper — so it is
    focusable and labelled `Close printer monitor`. It covers the full viewport at `z-[140]`.
14. **The monitor panel paints its own background**: `color-mix(in srgb, var(--surface-0), #000 10%)`,
    i.e. darker than a normal dialog.
15. **The printer menu only exists with more than one device**; with one device the same header is a
    static row. `printingMonitorHeaderUsesFleetLabelOrder` flips the top label's typography between a
    truncated 10 px line and a `text-[9px] uppercase tracking-[0.11em]` line.
16. **Dashboard progress is `(currentLayer - 1) / totalLayers`**, not `currentLayer / totalLayers`, and
    is clamped 0–100. Paused counts as an active print, and both paused and printing are false when the
    device is offline or disconnected (`device.connected !== true`).
17. **The dashboard progress bar is always rendered** (`/* Progress Bar (always rendered to keep card
    heights consistent) */`) even with no print — it renders an empty track at 0 %.
18. **Thumbnail/preview failures hide the `<img>` by mutating its style** (`display = 'none'`) rather
    than changing state. That is why the card falls back to the gradient with no logo.
19. **The relay webcam retries only two error classes** (`did not deliver any video data in time`,
    `websocket disconnected`) and only up to `DEFAULT_RELAY_AUTORETRY_LIMIT`, with a delay of
    `DEFAULT_RELAY_AUTORETRY_DELAY_MS`; other errors go straight to the error card. The retry counter
    is reset on every successful load. Aspect ratios outside 0.45–2.4 are rejected as pathological.
20. **`Cancel` on a print does not call the printer immediately** — it opens the monitor confirmation
    (`kind: 'control'`, `action: 'cancel'`) first; the same is true for emergency stop, plate start and
    plate delete. Every destructive printer action is two-step.
21. **`printingMonitorPendingConfirmation` is cleared before the action fires**, and the button styles
    are computed from the pending object before clearing.
22. **Placeholder card labels use hardcoded English** (`Pending`, `N/A`, `—`, `Unknown Model`,
    `Transform`, `Loaded scene`, `Original file path unavailable (overwrite disabled)`). They are not
    i18n and should be ported as literal constants, not msgids.
23. **`Save as New Scene` and `OK` (save error) are bare strings**; the sibling buttons in the same rows
    use `<Trans>`. Do not "fix" one without checking the locale catalogue.
24. **`UnsavedChangesDialog` has no Cancel button.** Cancel is the X, the backdrop, or Escape, and all
    three are disabled while `busy`.
25. **The two unsaved-changes dialogs differ only in copy and primary label**; both are `z-[220]` and
    both reuse the same subtitle rule based on `hasUnsavedSceneChanges` (so a saved scene reads
    `This scene is already saved.` above a body asking "Close DragonFruit now?").
26. **Several "dialogs" are `BlockingOverlay`s, not dialogs**: arrange (`z-[120]`), modifier apply
    (`z-[121]`). They announce `role="status"`, have no close affordance, and swallow Escape via a
    no-handler registration.
27. **`SceneMaterialReassignModal` resets its mapping whenever `options` or `rows` change identity**
    while open, defaulting every row to `options[0]`. If a porter memoises wrongly, a user's selections
    silently reset on any store update.
28. **Apply drops rows with no resolvable option.** The map it hands back contains only rows whose
    selected id is present in `options`; a row whose default was `''` disappears from the map.
29. **In tank-match mode the primary action can be `Assign to matching tanks` instead of
    `Reassign materials`**, and `Load without materials` is suppressed precisely when
    `isTankMatch && canAssignTanks` — the condition is `!canReassign && !(isTankMatch && canAssignTanks)`.
30. **`ScenePrinterMismatchModal` treats "close" as "keep".** X, backdrop and Escape all resolve
    `'keep'`, which for a missing plugin means geometry-only import and, when materials are missing,
    clears imported material assignments. There is no neutral dismiss.
31. **`lacksRequiredMaterials` is gated on `!pluginMissing`** — the missing-plugin variant never shows
    the materials paragraph, and its actions collapse to `Import Geometry` (+ optional `Manage Plugins`).
32. **Build-volume comparison uses a 1e-6 epsilon per axis**, not rounded mm, so near-identical
    volumes count as different. `buildVolumesMatch === false` is what gates the four volume paragraphs.
33. **`ScenePluginMissingModal` and the mismatch modal's plugin branch say the same thing twice** in the
    app (both can be raised). Keep both; they are raised by different flows (pre-load vs post-load).
34. **ZIP picker default selection is category-driven**, not "everything": a `mixed` archive preselects
    only mesh files or only scene files depending on `defaultSelectionCategory`, and the caption says so.
    An extension pill toggles *all* files of that extension: if all are selected, it clears them; else
    it selects them — it never partially inverts.
35. **The ZIP picker's strings are entirely hardcoded English** (title, header, captions, `Select all`
    titles, footer). Its i18n debt is intentional in the source; a port should keep the same literal
    text until the area is internationalised.
36. **The ZIP picker keeps its own theme detection** (document classes + `data-theme` + media query)
    to choose between two extension colour maps and two mix percentages. A native port has its own
    theme source and should map those two palettes, not copy the media-query logic.
37. **`formatFileSize` uses base 1024 with decimal thresholds (100 → 0 decimals, 10 → 1, else 2)** and
    returns `0 B` for non-positive input. It is separate from `useLocaleFormat`'s quantity formatting
    in intent (only the unit suffixes are localised).
38. **`ModelSupportsModal`'s total excludes knots** while the per-type grid lists them. The counts
    therefore do not add up to the total — that is deliberate
    (`// Knots are excluded: they are attachments on a support, not supports.`).
39. **Knots are attributed to a model through `parentShaftId`, which is a segment id**, resolved by
    `modelIdOfParentShaft`. Looking the id up in a knots/supports collection finds nothing.
40. **All support groups start collapsed on every open and on model change** (effect on
    `[isOpen, model?.id]`), and IDs are sorted with `localeCompare`, not by insertion.
41. **`IslandHierarchyModal` computes tree depth positions arithmetically** (32 px per level, connectors
    at `(level-1)*32 + 16` and node padding at `level*32 + 8`). Levels are 0-based, and level 0 has no
    connector; the child rail is drawn by the parent, offset by one level.
42. **`worldMaxAreaLayer` adds the z-offset divided by layer height and rounds**, matching how the island
    list card shows the same figure. Do not print the raw `maxAreaLayer` in the modal.
43. **`TetrahedralRemeshConfirmModal` and `ShrinkWrapConfirmModal` look identical but are different
    passes** (`tetSurfaceModelInPlace` vs `solidRemeshModelInPlace`), each with its own progress modal
    and its own phase translator. They must not be merged into one parameterised dialog.
44. **`DiagnosticsModal`'s benchmark is a window-event protocol**, not a store: the dialog emits
    `dragonfruit:diagnostics-benchmark-request` and only reacts to
    `dragonfruit:diagnostics-benchmark-progress`. In a single-process port this becomes a direct
    command + progress channel, but the `requestId` correlation and "ignore events for another request"
    rule must survive.
45. **`DiagnosticsModal`'s sampling has three magic constants** (250 ms window, 120 s history, 60 FPS
    floor / 24 ms floor on the graph axes) and a hardcoded `16.67` ms frame budget for the CPU estimate.
    The CPU figure is an estimate and is labelled as such (`CPU Usage (est.)`).
46. **`HistoryDebugModal` reverses the log for display** and shows a `Size:` fragment only when
    `payloadBytes !== undefined`. The two memory chips carry tooltips that explain why payload bytes and
    snapshot bytes are counted separately — those tooltips are part of the contract, not decoration.
47. **`DiagnosticsModals` only flips flags**; the history store is told to record details by the page
    (`setHistoryDebugDetailsEnabled(isHistoryDebugOpen)`), so a port that opens the panel without
    enabling detail recording will show an empty log.
48. **The manual repair dialog's Cancel button changes meaning three ways** (`Keep Original` → `Stop` →
    `Stopping…`) and `Stop` is a *cooperative* cancel: it sets `cancelling` and calls
    `cancelMeshRepair(token)`; the pipeline stops at the next stage boundary. Quote kept in source:
    `// Stops at the next stage boundary, so the mesh it leaves is one the pipeline finished a pass on.`
49. **The repair progress fraction is deliberately non-linear** (`0.1 + 0.8 * (1 - exp(-stages/6))`,
    or `0.06` before the first stage) because the stage count is unknown in advance. Do not replace it
    with `done/total`; there is no total.
50. **`autoStartRepair` is consumed once per id-set** (ref keyed on `ids.join('\0')`) and cleared
    whenever the id list becomes null. Both halves are needed: without the ref, re-renders restart the
    repair; without the clear, the next manual open auto-starts.
51. **`Repair Now` from the manifold warning reuses the manual repair dialog with `autoStartRepair`**,
    which also hides the `Repair` button and the two-column action grid.
52. **`DestructiveTransformModal` is reachable both before and after support deletion**; the page
    stores the continuation in `pendingDestructiveTransformContinueRef` and runs it after the delete,
    so the dialog itself never performs the transform.
53. **`ModifierModals`' reset dialog title depends on `isHollowingReset`**, which covers two of the
    three actions (`hollowing`, `clear_hollowing`) — `hole_punch` gets the holes wording. The *second*
    paragraph, however, is three-way (including the `clear_hollowing` sentence).
54. **Every interpolated Lingui message in these files is defined at module scope**, with comments such
    as `// Module level on purpose: React Compiler renames locals inside components` /
    `// before the Lingui macro derives the message id.` and
    `// Static ICU patterns in module-level formatters (see page.tsx): inline` /
    `// interpolation loses its placeholder names to the React Compiler in production.`
    A port that keeps translation keys stable must preserve the resulting msgids (including the
    `{0}`/`{1}` argument names shown in the `.po` files) rather than regenerating them.
55. **`SceneFileModals` passes `onManagePlugins` to the mismatch modal that first resolves `'keep'`** —
    the mismatch question is answered before navigating, otherwise the prompt would still be open
    behind the settings dialog.
56. **The plugin-import warning is awaited as a boolean.** Continuing resolves `true`; *every* other
    exit (Cancel, X, backdrop, Escape) resolves `false`. Its checkbox writes through
    `setPluginImportWarningSkipFuture` immediately, so cancelling after ticking still persists "don't
    remind again".
57. **"Do not remind again" is a checkbox inside the body**, so the shell's default actions grid does
    not apply — the actions are hand-placed in a `justify-between` row with the checkbox on the left.
58. **The pre-slice readiness dialog's buttons live in `children`**, so its `actionsClassName` prop is
    dead. Porting the class alone reproduces nothing; the row is `flex items-center justify-end gap-2 pt-1`.
59. **`PrintingResliceModal`'s Back returns to the *previous* mode** (`modeBeforePrintingRef`), while
    Re-Slice Now navigates to `export` and sets `shouldReturnToPrintingAfterSliceRef` so the app returns
    to printing afterwards. Cancel is not "stay here".
60. **`SliceCompletedModal` is only raised for the `'file'` intent** and only when a slicing time is
    finite; batches show the *directory*. `'upload'`, `'print'`, `'uvtools'` and `'preview'` navigate
    instead.
61. **The UVTools launching modal cannot be dismissed and is dismissed by the caller** after 5 seconds
    regardless of launch success or failure; `onPointerDown` stop-propagation prevents the backdrop
    click from reaching the editor underneath.
62. **`GpuPerformanceWarningModal` is self-triggering and storage-backed.** It never re-checks, never
    shows on non-Windows, and fails open on storage errors (`// Storage can be unavailable; showing the
    warning is still correct.`). "Don't show again" is the only thing persisted;
    "Got it" is session-only.
63. **`classifyGpuRenderer` order matters**: software first, then discrete, then integrated, then
    unknown. Integrated names contain discrete vendor names (`Intel Arc` is discrete, `Intel UHD
    Graphics` is not; `Radeon RX` is discrete, `Radeon Graphics` is an APU). A reordered matcher
    mislabels real machines.

## Open questions

1. **No GPUI tree exists in this worktree** — there is no `gpui/` directory and
   `docs/internal/gpui-port-handoff.md` is absent, so every "Port target" above is a proposal. Module
   names should be reconciled with whatever layout the port adopts.
2. **Tauri/native calls.** `SliceCompletedModal` invokes `reveal_in_file_manager`; `PrintingModals`
   launches UVTools through `launchExternalProcess` and resolves its executable path. Both need native
   equivalents; the port cannot keep the `import('@tauri-apps/api/core')` lazy import.
3. **Printer I/O is plugin-owned.** `executePrintingMonitorControlAction`, `executePrintingMonitorFeatureToggle`,
   `executePrintingMonitorSdcpDebugCommand`, `executeStartMonitorRecentPlate`,
   `executeDeleteMonitorRecentPlate`, `performSendToPrinter`, `handlePrintNow` and the whole
   `printingTargetMaterial*` data set come from `usePrintingMonitorManager` and the plugin network
   adapters (`ProfileNetworkUiAdapter`, `PluginMonitoringUiAdapterContract`). Their contracts are not
   defined here; the dialogs only render their outputs.
4. **Relay playback** depends on `RtspRelayCanvasPlayer` and `printingMonitorRelayDebugTransport`
   (client/server RTP ports, claim status), which belong to the RTSP relay area. The RTSP debug overlay
   is a pure readout of that state.
5. **WebGL/JS-specific diagnostics** (`performance.memory`, `WEBGL_debug_renderer_info`, `requestAnimationFrame`
   timing, `navigator.hardwareConcurrency`, `navigator.deviceMemory`, `navigator.userAgent`) have no
   direct GPUI equivalent. The port needs a decision: map to platform APIs, or render `N/A` for the
   fields it cannot source (the source already renders `N/A` when a value is absent).
6. **Benchmark producer.** Nothing in this area executes the orbit benchmark; the dialog only emits the
   request event. The consuming side (`dragonfruit:diagnostics-benchmark-request`) is out of scope and
   must be found before the benchmark card can work.
7. **`useLocaleFormat` / `UnitFormat.quantity(value, decimals, unit)`** drives numbers everywhere
   (`ms`, `s`, `mm`, `mm3`, `mm2`, `percent`, `fps`, `perSecond`, `B/KB/MB/GB/TB`). The port must decide
   the Rust equivalent (unit suffix localisation included) before any of these dialogs can be matched
   exactly.
8. **Translations.** `.po` files key messages off exact source strings including JSX placeholders
   (`<0>`, `{0}`), and several strings deliberately stay English. Whether the port reuses these msgids
   or restarts the catalogue is a policy decision this spec cannot settle.
9. **`SceneMaterialReassignModal` is rendered by the page, not by `SceneFileModals`**, although it is
   conceptually scene I/O. Its placement should be confirmed against the port's own grouping.
10. **Focus trapping.** The React dialogs do not trap focus beyond the escape stack and
    `aria-modal`; a native port will get focus behaviour for free (or wrong) depending on the framework.
    No source contract exists beyond the `aria-*` attributes quoted above.

