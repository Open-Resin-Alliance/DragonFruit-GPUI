# Printer & Material Profiles

The printer/materials modal: one large modal that owns the printer library, the
printer editor, the material library, the material editor, the initial-setup
(variant) picker, network/fleet settings, and the Multi-Tank tank assignment
pane. Everything in this area is a child of a single `ProfileSettingsModal`
instance mounted by `TopBar` (`src/components/layout/TopBar.tsx:1231`);
`openProfileSettingsModal(tab, options)` (`src/components/settings/profileModalEvents.ts:17`)
is the only supported way to open it, and it is fire-and-forget: it dispatches
`dragonfruit:open-profile-settings-modal`, `TopBar` listens, bumps a token, and
passes the token down.

**Note on port targets:** there is no `gpui/` tree in this worktree (verified: no
`gpui` directory anywhere under the repo root, and `docs/internal/gpui-port-handoff.md`
does not exist). The "Port target" lines below are therefore proposed module paths
under the layout the rest of the port uses (`gpui/src/features/<area>/…`), not
existing code. Nothing under `gpui/src/features/` could be inspected for this spec.

## Components in scope

| Component | File |
|---|---|
| `ProfileSettingsModal` (main modal + 13 nested dialogs + `RemoteMaterialEditDialog` + `AutoTrimmedImage`) | `src/components/settings/ProfileSettingsModal.tsx` (7 493 lines) |
| `PrinterVariantPickerModal` ("Initial Setup") | `src/components/printers/PrinterVariantPickerModal.tsx` (720 lines) |
| `SetupMethodChooser`, `SetupModeButton`, `ManualIpEntryCard` | `src/components/printers/SetupMethodChooser.tsx` (161 lines) |
| `FleetManagement` (rendered *by* `ProfileSettingsModal` as the Network Settings surface) | `src/components/settings/FleetManagement.tsx` (458 lines) |

### Dependencies (other areas — name only, do not re-spec here)

| Dependency | Where | What this area needs from it |
|---|---|---|
| `profileFormAtoms.tsx` (158 KB) | `src/components/settings/profileFormAtoms.tsx` | `MaterialProfileFormSections` (`:993`), `MaterialAntiAliasingSection` (`:1253`), `ReplacementMaterialEditorShell` (`:2913`), `PluginLocalMaterialSettingsSections` (`:2478`), `MaterialProfileIdentitySection` (`:832`), `LabeledInput`/`LabeledNumberInput`/`LabeledSelectInput`/`LabeledToggleInput`/`LabeledResinFamilySelect`/`LabeledCurrencySelect`, `MODIFIED_FIELD_STYLE` (`:545`), `DefaultOptionChip` (`:551`), `RESIN_FAMILY_OPTIONS` (`:48`), `RESIN_FAMILY_COLOR` (`:57`), `CURRENCY_OPTIONS` (`:66`), `clampNonNegativeNumber` (`:75`), types `MaterialDraft` (`:36`) and `LocalSettingsByOutputDraft` (`:37`). |
| `profileStore.ts` | `src/features/profiles/profileStore.ts` | Types `PrinterProfile` (`:153`), `PrinterPreset` (`:80`), `PrinterNetworkDevice` (`:48`), `MaterialProfile` (`:348`), `PrinterMaterialMode`, `PrinterWebcamRotationDeg`, `DEFAULT_MATERIAL_ANTI_ALIASING_SETTINGS` (`:246`); plus ~30 action functions enumerated in "Data" below. |
| `tankMaterialCompatibility.ts` | `src/features/profiles/tankMaterialCompatibility.ts` | `tankMaterialShape`, `tankMaterialValueMatches`, `tankMaterialsDisagree`. |
| `remoteMaterialUiUtils.ts` | `src/features/plugins/remoteMaterialUiUtils.ts` | `RemoteMaterialProfile`, `RemoteMaterialEditDraft`, `buildRemoteMaterialChips`, `buildSortedRemoteMaterialDraftEntries`, `buildBasicRemoteMaterialSections`, `buildAdvancedRemoteMaterialSections`, `formatRemoteMaterialFieldLabel`, `isLikelyNumericRemoteMaterialField`. |
| `pluginRegistry.ts` | `src/features/plugins/pluginRegistry.ts` | `getProfileNetworkUiAdapter(mode)`, `getDefaultProfileNetworkUiAdapter()`, `getAvailableProfileNetworkModes()`, `getRuntimeMaterialPresets(presetId?)`, `getProfileLocalMaterialSettingsAdapter(output, mode)`. |
| `complexPluginContracts.ts` | `src/features/plugins/complexPluginContracts.ts:49-97` | `RemoteMaterialSettingsAdapter` + `PluginNetworkUiAdapterContract` (the adapter shape the network/material UI is driven by). |
| formats registry | `src/features/slicing/formats/registry.ts` | `getAvailableOutputFormatOptions`, `getAvailableFormatVersionOptions`, `outputFormatSupportsMultiTank`, `getAvailableSettingsModeOptions`, `resolveOutputFormatVersion`, `resolveOutputSettingsMode`. |
| sideloaded file types | `src/features/plugins/sideloadedFileTypeRegistry.ts` | `getSideloadedOutputFormatsSnapshot`, `subscribeSideloadedFileTypes`. |
| printer reachability | `src/features/network/printerReachabilityStore.ts` | `getPrinterReachabilitySnapshot`, `getPrinterReachabilityServerSnapshot`, `subscribeToPrinterReachability` → `Record<deviceId, boolean \| null>`. |
| native dialogs | `src/features/slicing/tauri/nativeSlicerBridge.ts` | `pickOpenFilesWithNativeDialog('bundle', false)`, `readPrintArtifactBytesFromPath(path)`, `savePrintArtifactWithNativeDialog(bytes, suggestedFilename)`. |
| plugin network bridge | `src/utils/pluginNetworkBridge.ts` | `pluginNetworkFetch({ pluginId, operation, host, … })` → `fetch`-like `Response`. |
| atoms | `src/components/atoms/*` | `Button` (variants `secondary`, `tinted-danger`, `tinted-warning`, `tinted-accent`, `surface`, …), `IconButton` (`variant="surface"`, `size="sm"\|"md"`), `IconChip`, `Spinner`, `ProgressBar` (`indeterminate`, `size="xs"`), `FadeText`. |
| `SelectDropdown` | `src/components/ui/SelectDropdown.tsx` | Option flags `heading`, `divider`, `action`, `icon`, `rightContent`, `tone`; trigger props `leadingDisplay`, `selectedDisplay`, `hideSelectedText`, `selectedDisplayAlignment`, `selectedTextAlign`, `selectStyle`, `selectClassName`. |
| `StructuredDialogModal` | `src/components/ui/StructuredDialogModal.tsx` | Portal dialog with header icon/title/subtitle, `actions` grid footer, own `useEscapeToClose`. |
| `Tooltip` | `src/components/ui/Tooltip.tsx` | `content`, `wrapperClassName`. |
| `useEscapeToClose` | `src/hotkeys/useEscapeToClose.ts` | `useEscapeToClose(active, onEscape)`. |

---

## Surface: modal shell — `ProfileSettingsModal` root

- **Source**: `src/components/settings/ProfileSettingsModal.tsx:3911-3953` (backdrop/panel/body), header `:3925-3950`.
- **Renders**: full-screen backdrop; a centred panel with a header (icon tile + title + close), a scrolling body, then all nested dialogs as siblings inside the panel.
- **Layout & styling**:
  - Backdrop: `"fixed inset-0 z-50 flex items-stretch justify-center bg-black/58 backdrop-blur-sm p-5 ui-modal-backdrop-enter"`. Backdrop `onMouseDown` closes only when `event.target === event.currentTarget`, via `requestClose` (not `onClose`).
  - Panel: `` `w-full max-w-[1120px] flex flex-col rounded-xl border shadow-2xl overflow-hidden ui-modal-panel-enter ${hasPrinters ? 'h-full' : 'self-center h-[700px] max-h-[94vh]'}` `` with inline `background: 'var(--surface-0)'`, `borderColor: 'var(--border-strong)'`, `boxShadow: '0 26px 54px rgba(0,0,0,0.48)'`.
  - Header bar: `"flex items-center justify-between px-3 py-3 border-b"`, `borderColor: 'var(--border-subtle)'`, `background: 'color-mix(in srgb, var(--surface-1), transparent 8%)'`. Icon tile `"inline-flex h-8 w-8 items-center justify-center rounded-md border"` with `background: 'linear-gradient(135deg, color-mix(in srgb, var(--accent), var(--surface-1) 86%), color-mix(in srgb, var(--accent-secondary), var(--surface-1) 90%))'`, `Box` icon `w-4 h-4` in `var(--accent)`.
  - Title `h2`: `"text-base font-semibold"`, `color: var(--text-strong)`.
  - Close: `IconButton variant="surface" size="sm"` with `X` `w-4 h-4`, `aria-label="Close"`.
  - Body: `` `px-3 py-3 custom-scrollbar ${hasPrinters ? 'flex-1 min-h-0 overflow-y-auto overflow-x-hidden flex' : 'flex-1 min-h-0 overflow-hidden flex'}` `` wrapping `` `flex flex-col gap-3 ${hasPrinters ? 'w-full min-h-full' : 'w-full h-full min-h-0'}` ``.
- **States**: `!isOpen` → `return null` (`:3907`). Two panel geometries: with printers the panel fills the backdrop (`h-full`); with zero printers it is a fixed `700px` card, `max-h-[94vh]`, vertically centred (`self-center`), and the body does not scroll.
- **Data**: reads `useSyncExternalStore(subscribeToProfileStore, getProfileStoreSnapshot, getProfileStoreServerSnapshot)` into `profileState`; `isOpen`, `onClose`, `initialTab = 'printer'`, `openPrinterLibraryToken = 0`, `openNetworkSettingsToken = 0`, `openMaterialAntiAliasingToken = 0`.
- **Callbacks**: close (X and backdrop) → `requestClose()`.
- **Text**: `Printer & Material Profiles`. `Close`.
- **Port target**: `gpui/src/features/profiles/mod.rs` → `ProfileSettingsModal` view; `gpui/src/features/profiles/state.rs` for the `ProfileSettingsState` mirroring the ~45 `useState`s listed under "Data" of the panes. What exists: nothing in this worktree.

### Close gating (shared by X, backdrop and Escape)

`requestClose` (`:2189-2203`):

1. If `unresolvedTankMismatch` (Multi-Tank, two assigned tanks disagree) → `setTankMismatchPair(unresolvedTankMismatch)` and **return without closing**. Comment: *"A mismatch has to be resolved before the profile editor can be left: the values decide the layer stack, so closing would hide a broken setup."*
2. Else if `multiTankMaterialPicker && assignedTankCount === 0` → open the "Tanks are unassigned" dialog, return.
3. Else `onClose()`.

Comment above it: *"One assigned tank is enough to be useful — a profile is allowed to carry tanks nobody uses yet. Only leaving with nothing assigned at all is worth interrupting."*

### Escape unwinding

`useEscapeToClose(isOpen, …)` (`:2205-2270`) closes **one** nested layer per press, in this exact order, and only falls through to `requestClose()` when none was open:

1. `tankMismatchPair` → clear
2. `showUnassignedTanksWarning` → false
3. `deleteConfirmTarget` → null
4. `showOfficialMaterialLockDialog` → false
5. `showOfficialLockDialog` → false + `officialLockedProfileId = null`
6. `showPrinterUpdateDiffModal` → false
7. `isEditFleetUnitModalOpen` → false
8. `isRemoteMaterialEditDialogOpen` → false (blocked while `isSavingRemoteMaterialEdit`)
9. `isNetworkSettingsOpen` → false
10. `variantChooserPreset` → null
11. `showPresetPicker` → false
12. `showMaterialPresetPicker` → false
13. `isMaterialEditorOpen` → false
14. `isCreateMaterialOpen` → false
15. `isEditingPrinter` → false
16. otherwise `requestClose()`

Nested modals (initial setup, structured dialogs, fleet) each register **their own** `useEscapeToClose`, so the innermost handles the key first.

---

## Surface: custom-profile safety banner

- **Source**: `:3954-3967`.
- **Renders**: one warning strip above the 3D Printer rail.
- **Layout & styling**: `"rounded-md border px-3 py-2 text-xs flex items-start gap-2"`; inline `borderColor: 'color-mix(in srgb, #d97706, var(--border-subtle) 30%)'`, `background: 'color-mix(in srgb, #d97706, var(--surface-1) 92%)'`, `color: 'var(--text-muted)'`; `AlertTriangle` `w-4 h-4 mt-0.5 shrink-0` in `#d97706`.
- **States**: shown only when `isCustomSelectedPrinter` = `selectedPrinter?.isCustom && !selectedPrinter?.isOfficial` (`:3909`).
- **Data**: `selectedPrinter`.
- **Text**: `Safety warning:` (bold, `var(--text-strong)`) followed verbatim by `Custom, non-official profiles may increase the risk of print failure and can potentially damage the machine or cause personal injury. Verify all settings carefully before printing.`
- **Port target**: `gpui/src/features/profiles/printer_rail.rs`.

---

## Surface: empty state — "Welcome to Printer Profiles"

- **Source**: `:3969-4006`.
- **Renders**: a single centred card filling the body when there are no printer profiles.
- **Layout & styling**: `"rounded-md border flex-1 h-full min-h-0 flex items-center justify-center px-4 py-10"`; `borderColor: 'var(--border-subtle)'`, `background: 'linear-gradient(180deg, color-mix(in srgb, var(--surface-2), transparent 4%), color-mix(in srgb, var(--surface-2), black 8%))'`. Inner column `"text-center max-w-[520px]"`. Icon circle `"inline-flex h-14 w-14 items-center justify-center rounded-full border mb-3"` with `borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 22%)'`, `background: 'color-mix(in srgb, var(--accent), var(--surface-1) 88%)'`, `Printer` `w-6 h-6` accent.
- **States**: only when `!hasPrinters` (`profileState.printerProfiles.length === 0`).
- **Callbacks**: "Add printer" → `handleAddPrinter()` → clears `selectedLibraryPresetIds`, opens the Printer Library.
- **Text**: heading `Welcome to Printer Profiles` (`"text-2xl font-semibold"`); body `Add your first printer from the library to unlock a tailored materials list and printer-specific defaults.`; button label `Add printer` (`<Trans>`), `"mt-5 !h-10 !px-4 !py-0 text-sm inline-flex items-center justify-center gap-1.5 rounded-md"`, style `accentSecondaryActionStyle92`, `Plus w-4 h-4`.
- **Port target**: `gpui/src/features/profiles/empty_state.rs`.

---

## Surface: "3D Printer" rail — printer cards, reorder, fleet switch

- **Source**: section `:4008-4494`; header `:4009-4068`; card grid `:4073`; printer card factory `renderPrinterRailCard` `:1240-1385`; preset-less card body `:4140-4330`; fleet cards `:4091-4140`; empty-fleet block `:4357-4375`; footer action row `:4377-4490`.
- **Renders**: a bordered section with a title row (title + subtitle, then Move-left / Move-right / Add-printer buttons on the right), a 5-column grid of printer cards (or fleet-device cards), and beneath it a second bordered strip of profile-level actions.
- **Layout & styling**:
  - Section: `"rounded-md border overflow-hidden shrink-0"`, `borderColor: var(--border-subtle)`, `background: 'linear-gradient(180deg, color-mix(in srgb, var(--surface-1), transparent 8%), var(--surface-1))'`.
  - Header: `"p-3 border-b"`; inner `"flex items-start justify-between gap-2"`. `h3` `"text-sm font-semibold flex items-center gap-2"` with `Box w-4 h-4`; subtitle `"text-xs mt-0.5"` muted.
  - Reorder buttons: `Button variant="secondary" size="auto"` `"!h-8 !w-8 !px-0 !py-0 inline-flex items-center justify-center rounded-md disabled:opacity-45"`, style `{ color: 'var(--text-muted)', borderColor: 'var(--border-subtle)', background: 'var(--surface-1)' }`, `aria-label`/`title` `"Move selected printer left"` / `"Move selected printer right"`, icons `ChevronLeft`/`ChevronRight` `w-3.5 h-3.5`.
  - Add printer: `"!h-8 !px-3 !py-0 text-sm inline-flex items-center justify-center gap-1 rounded-md shrink-0"`, style `shouldShowFleetSwitchAction ? accentSecondaryActionStyle92 : accentSecondaryActionStyle93`.
  - Grid: `` `grid grid-cols-5 gap-2.5 pb-1 ${shouldConstrainPrinterRailHeight ? 'max-h-[392px] overflow-y-auto pr-1' : ''}` ``; `shouldConstrainPrinterRailHeight = printerRailEntryCount > 8`. Grid `onDragOver` prevents default; `onDrop` on the grid moves the dragged profile to the **end** (`moveDraggedPrinter(printerDragId, null)`).
  - Card (shared factory): outer `div` `` `min-w-0 w-full rounded-md border p-2 transition-all duration-150 ${draggable ? (dragging ? 'opacity-60' : 'cursor-grab active:cursor-grabbing') : ''}` `` with `active ? activeStyles : inactiveStyles`. Active styles for both card kinds: `{ borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 28%)', background: 'color-mix(in srgb, var(--accent), var(--surface-1) 88%)' }`; inactive: `{ borderColor: 'var(--border-subtle)', background: 'var(--surface-2)' }`.
  - Image well: `"h-[128px] min-h-[128px] max-h-[128px] shrink-0 rounded-md border overflow-hidden relative"`, inline `borderColor: 'var(--border-subtle)'`, `background: printerImageWellBackground` (`isLightTheme ? 'color-mix(in srgb, var(--surface-2), white 20%)' : '#1c2027'`), plus explicit `height/minHeight/maxHeight: 128`. Image wrapper `absolute inset-1` (profiles, `object-contain`, `AutoTrimmedImage`) or `inset-0` (`object-cover`, plain `<img>`, fleet units).
  - Missing image fallback: `"absolute inset-0 flex items-center justify-center px-2"`, colour `var(--text-muted)`, inner `"flex flex-col items-center justify-center gap-1 text-[11px] text-center leading-tight"`; content is `Printer`/`ImagePlus` `w-5 h-5 mx-auto mb-1` plus `Generic` / `No image` (profiles) or `Online` / `Offline` (fleet).
  - Title block under the well: `"mt-0.5 flex items-center justify-between gap-1.5"`; name `"text-xs leading-snug font-semibold truncate min-w-0"` in `var(--text-strong)`; subtitle `"text-xs truncate"` in `var(--text-muted)`; optional `footer` node on the right.
  - Fleet empty block: `"mt-2 rounded-md border px-3 py-5 text-center"` with `borderColor: var(--border-subtle)`, `background: 'color-mix(in srgb, var(--surface-2), transparent 8%)'`.
  - Footer action strip: `"mt-2.5 rounded-md border p-2"` with `background: 'color-mix(in srgb, var(--surface-2), transparent 8%)'`, inner `"flex flex-wrap items-center gap-2"`. All action buttons `"!h-8 !px-3 !py-0 text-sm inline-flex items-center justify-center gap-1 rounded-md"` (icons `w-3.5 h-3.5`).
- **States**:
  - Section title: `shouldRenderFleetRail ? `${selectedPrinter?.name ?? 'Printer'} Fleet` : '3D Printer'`.
  - Subtitle: fleet → `Showing connected devices for ${selectedPrinter?.name ?? 'selected profile'}.`; profiles → `Each printer can store its own image and has a dedicated set of compatible material profiles.`
  - Move-left disabled when `!selectedPrinter || selectedPrinterRailIndex <= 0`; Move-right disabled when `!selectedPrinter || index < 0 || index >= printerProfiles.length - 1`.
  - Rail mode `printerRailViewMode: 'profiles' | 'fleet'`; `shouldRenderFleetRail = selectedPrinterSupportsNetworkSettings && printerRailViewMode === 'fleet'`. Two effects keep it sane: no network support → force `'profiles'`; zero fleet devices → force `'profiles'` (`:1150-1168`).
  - Profile card `muted` when `printer.missingPlugin` is set: outer `aria-disabled`, image well `"opacity-60 grayscale"`, both buttons `cursor-not-allowed` and no click/drag; wrapped in `Tooltip` with `The "<plugin>" plugin is missing.` (`missingPluginMessage`, `:350`).
  - Profile card badges: `resolvedCardBadgeText = hasOfficialUpdate ? 'UPDATE' : (printer.isCustom ? 'CUSTOM' : platformBadge?.text)`. UPDATE uses `linear-gradient(135deg, #22c55e, #15803d)`; CUSTOM uses `linear-gradient(135deg, #ef4444, #b91c1c)`; platform badge uses `linear-gradient(135deg, color-mix(in srgb, ${color || '#0ea5e9'}, white 14%), color-mix(in srgb, ${color || '#0ea5e9'}, black 18%))`. Badge box: `"pointer-events-none absolute top-1 right-1 z-10 inline-flex h-[18px] min-w-[44px] items-center justify-center whitespace-nowrap rounded-md px-1.5 text-[9px] font-bold leading-none"`, white text, `letterSpacing: '0.04em'`, inner `span` `"relative top-[0.5px]"`.
  - Bit-depth corner badge (bottom-right): only when `bitDepthBits != null && bitDepthBits !== 8`, label `` `${bits} Bit` ``; 3-bit uses red gradient pair (`#ef4444` mixed with `#111827` 56% / `#0b1220` 72%) and border `color-mix(in srgb, #ef4444, white 18%)`; otherwise accent-secondary gradient (`52% / 68%`) and border `color-mix(in srgb, var(--accent-secondary), white 20%)`. `title` = `printer.bitDepth?.description || `${label} display``.
  - Network overlay (top-left circle, profiles): shown when `isNetworkConnected || isNetworkOffline`; `"pointer-events-none absolute top-1 left-1 z-10 inline-flex h-7 w-7 items-center justify-center rounded-full border"`; connected `{ borderColor: 'color-mix(in srgb, #22c55e, white 12%)', background: 'color-mix(in srgb, #22c55e, #0f172a 40%)', color: '#dcfce7' }` with `Wifi`; offline `{ borderColor: 'color-mix(in srgb, var(--border-subtle), #ef4444 18%)', background: 'color-mix(in srgb, #ef4444, #0f172a 78%)', color: '#fecaca' }` with `WifiOff`. `title`: `Connected to ${hostName || ip}` or `Printer not connected`.
  - Fleet card overlay/badge: same circle shape, `Wifi`/`WifiOff`; top-right badge text `Online` / `Limited` / `Offline` where `statusLabel = online ? 'Online' : device.connected ? 'Limited' : 'Offline'`, `online = device.connected === true && reachable`, `reachable = printerReachabilityByDeviceId[device.id] !== false`. Green gradient when online, red when not.
  - Card `footer` (right of the title): missing plugin → small `tinted-danger` delete button `"!h-7 !w-7 !px-0 !py-0"`, `aria-label` `Delete printer`, `Trash2 w-3.5 h-3.5`. Else if `printer.networkSupport` → a network button whose label is the fleet count (`"grid h-full w-full place-items-center text-[12px] font-semibold leading-none tabular-nums"`) when `fleetCount > 0`, otherwise `Wifi w-3.5 h-3.5`; `aria-label`/`title` `Open fleet view (${fleetCount})` / `Set up network` (or `Switch to fleet view (${fleetCount})`); active style `{ color: 'var(--text-strong)', borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 42%)', background: 'color-mix(in srgb, var(--accent), var(--surface-1) 90%)' }`.
  - Footer strip, fleet mode: `Return to Printers` (→ `'profiles'`), `Manage Fleet` (`Search` icon, opens Network Settings), `Edit Unit` (`Edit3`, disabled without an active managed device).
  - Footer strip, profile mode: `Show Fleet` / `Network Settings` (only when `selectedPrinterSupportsNetworkSettings`; label `regularNetworkActionLabel = shouldShowFleetSwitchAction ? 'Show Fleet' : 'Network Settings'`, icon `LayoutGrid` vs `Search`); `Update Printer` (only when `selectedPrinterUpdate`, `Download`, title `Update v<cur> to v<latest>`); `Edit`; `Import`; `Export`; `Remove` (`ml-auto`, danger colour unless `!hasPrinters`).
- **Data**: `profileState.printerProfiles`, `printerRailViewMode`, `printerDragId`, `manualBuildDimensionsByPrinterId`, `officialPrinterUpdateIds`, `printerReachabilityByDeviceId`, `activeManagedNetworkPrinter`, `managedNetworkPrinters`, `selectedPrinter`, `selectedPrinterRailIndex`, `shouldConstrainPrinterRailHeight`, `isLightTheme`. Writes: `movePrinterProfile`, `setActivePrinterProfile` (via `handlePickPrinter`), `setSelectedPrinterId`, `setIsEditingPrinter`, `setDeleteConfirmTarget`, store-level fleet actions through `handlePickPrinter` + `setPrinterRailViewMode` + `setIsAddingNetworkPrinter`.
- **Callbacks**:
  - Card click (profiles) → `handlePickPrinter(printer.id)` (`:3305-3326`): resets `materialSelectionInitializedRef`, sets `selectedPrinterId`, `isEditingPrinter=false`, calls `setActivePrinterProfile`, then restores a material: `activeMaterialProfileIdByPrinterId[id]` → `activeMaterialProfileId` → first material; sets `selectedMaterialId` and, if found, `selectedManufacturer` (`brand || 'Default'`, trimmed) and `selectedResinFamily`.
  - Card double-click → `handlePickPrinter` + `setIsEditingPrinter(true)`.
  - Drag start → `dataTransfer.effectAllowed = 'move'`, `setData('text/plain', id)`, `setPrinterDragId(id)`; drag end clears; per-card drag-over prevents default; per-card drop calls `moveDraggedPrinter(printerDragId, printer.id)` (insert before the hovered card), `stopPropagation`, then clears.
  - Move left/right → `moveSelectedPrinterInRail(±1)` → `movePrinterProfile(id, beforeId)`.
  - `Add printer` → `handleAddPrinter` (opens library).
  - `Show Fleet` → `setPrinterRailViewMode('fleet')` when a fleet exists, else `handleOpenNetworkSettings()`.
  - `Edit` → `setIsEditingPrinter(true)`.
  - `Import` → `handleImportSelectedPrinterBundle`; `Export` → `handleExportSelectedPrinterBundle`.
  - `Remove` → `requestDeleteSelectedPrinter` → `deleteConfirmTarget = {kind:'printer', id, name}`.
  - `Update Printer` → `setShowPrinterUpdateDiffModal(true)`.
  - Fleet card click → `handleSelectManagedPrinter(device)`; double-click → select + `setIsEditingPrinter(true)`.
- **Text**: `3D Printer`; `Add printer`; `Move selected printer left`; `Move selected printer right`; `Return to Printers`; `Manage Fleet`; `Edit Unit`; `Show Fleet`; `Network Settings`; `Update Printer`; `Edit`; `Import`; `Export`; `Remove`; `No fleet devices saved for this printer profile yet.`; `Add First Device`; `Generic`; `No image`; `Online`; `Limited`; `Offline`; `CUSTOM`; `UPDATE`; `<n> Bit`; `Delete printer` (`msg`); `The "<plugin>" plugin is missing.`; `Update v{cur} to v{latest}` (attribute, untranslated template).
- **Port target**: `gpui/src/features/profiles/printer_rail.rs` (`PrinterRail`, `PrinterCard`), `gpui/src/features/profiles/fleet_rail.rs`.

---

## Surface: Multi-Tank pane

- **Source**: `:4497-4625`; tank tile grid `:4514-4615`; empty hint `:4618-4622`.
- **Renders**: a section titled "Multi-Tank" with a `FlaskConical` icon and a grid of tank tiles, each a `Tank N` label (with an inter-tank link glyph between consecutive assigned tanks) above a `SelectDropdown` of materials.
- **Layout & styling**: section `"rounded-md border overflow-hidden flex flex-col shrink-0"` (same gradient background as the printer section); header `"p-3 border-b"`; grid `"grid grid-cols-2 md:grid-cols-3 xl:grid-cols-4 gap-2.5"`; tile wrapper `"space-y-1 block"`; label `"ui-label font-medium inline-flex w-full justify-center text-center relative"`.
  - Link glyph: absolutely centred SVG `width={200} height={64} viewBox="-100 -32 200 64"` with two paths `d="M -20 0 H -70 A 6 6 0 0 0 -76 6 V 9"` and `d="M 20 0 H 70 A 6 6 0 0 1 76 6 V 9"`, `strokeWidth={2}`, `strokeLinecap="round"`, `stroke` = `linkColor` (`var(--danger)` mismatch, `var(--success)` match).
  - Mismatch affordance: a real `<button>` `"inline-flex h-6 w-6 shrink-0 cursor-pointer items-center justify-center rounded-md hover:brightness-125 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2"`, `outlineColor: 'var(--danger)'`, `OctagonAlert h-6 w-6`, `aria-label` = the tooltip text; match state is a non-interactive `<span role="img">` with `Link2 h-6 w-6`.
  - Dropdown: `selectClassName="w-full h-[36px] px-2.5 pr-10 leading-tight text-sm"`, `selectedDisplayAlignment="center"`, `selectedTextAlign="center"`; unassigned tank gets `leadingDisplay` = `AlertTriangle h-3.5 w-3.5` in `var(--danger)`, `hideSelectedText`, and `selectedDisplay` = `<span style={{ color: isTank ? 'var(--danger)' : 'var(--text-muted)' }}>—</span>`, plus `selectStyle={{ color: 'var(--danger)' }}`. Assigned tank shows the material's label.
- **States**: rendered only when `hasPrinters && selectedPrinter && multiTankMaterialPicker` (`:680`: `outputFormatSupportsMultiTank(outputFormat) && materialMode === 'multiTank'`). The grid always renders `Math.max(4, selectedPrinter.tankCount ?? 2)` tiles; tiles at index ≥ `tankCount` are disabled spares (`isTank=false`). Link glyph only when both neighbours are assigned. Hint when `printerMaterials.length === 0`.
- **Data**: `selectedPrinter.tankCount`, `selectedPrinter.tankMaterials` (array by tank index, `''` = unassigned), `tankAssignedMaterials`, `tankMaterialOptions` (built `:1055-1110`).
- **Callbacks**: `onChange(value)` → clones `tankMaterials` to `tankCount` length, sets `next[index] = value`, `updatePrinterProfile(id, { tankMaterials: next })` (value `''` = Unassign Tank). Mismatch glyph click → `setTankMismatchPair({ leftId, rightId })`.
- **Tank option list construction** (reproduce exactly): group `printerMaterials` by `brand` (`'Default'` when blank), sort brands with `localeCompare`, emit a `heading` option `{ value: `__brand__:${brand}`, label: brand, heading: true }`, then materials sorted by `name.localeCompare`, labelled `familyLabel && !nameStartsWithFamily ? `${familyLabel} ${material.name}` : material.name` where `familyLabel` comes from `RESIN_FAMILY_OPTIONS`; if `material.color`, add a swatch `icon` `<span className="h-3.5 w-3.5 rounded-[4px]">` with `background: color` and `boxShadow: 'inset 0 0 0 1px rgba(0, 0, 0, 0.35), 0 0 0 1px var(--border-subtle)'`. Finally a divider option (`{ value: '__unassign-rule__', label: '', divider: true }`) then `{ value: '', label: 'Unassign Tank', icon: <Trash2 class="h-3.5 w-3.5"/>, tone: 'danger', action: true }`.
- **Text**: `Multi-Tank`; `Tank 1`…`Tank N`; `Unassign Tank`; `Add a material profile to assign it to a tank.`; tooltips `Tank materials have compatible settings.` (`msg`, comment *"Tooltip on the green link between two Multi-Tank tank labels whose materials agree."*) and `Tank materials have incompatible settings. Please click to resolve mismatches.` (comment *"Tooltip on the warning between two Multi-Tank tank labels whose materials would print different layer stacks."*).
- **Port target**: `gpui/src/features/profiles/multi_tank.rs`.

---

## Surface: material pane header + Material Settings list

- **Source**: section `:4627-5133`; header `:4636-4725`; remote list `:4740-4800`; offline card `:4800-4830`; connect card `:4830-4860`; three-column list `:4864-5020`; action row `:5022-5133`.
- **Renders**: a section with a `FlaskConical` title, a description paragraph that changes by mode, an optional "update available" line, header action buttons, then either a remote-material list / a blocking empty state / a three-column browser (Manufacturer | Material Type | Profile), and finally an action row.
- **Layout & styling**: section `"rounded-md border overflow-hidden flex flex-col min-h-[320px] flex-1 shrink-0"` + same gradient. Header `"p-3 border-b"` with `"flex items-start justify-between gap-2"`. Description `"text-xs mt-0.5"` muted; printer name inside it is a `<span style="color: var(--text-strong)">`. Update line `"text-xs mt-1"` in `var(--accent-secondary)`. Header buttons `"!h-8 !px-3 !py-0 text-sm …"`; refresh is square (`!h-8 !w-8 !p-0`).
  - Three-column browser container `"rounded-md border overflow-hidden flex flex-col flex-1 min-h-0"` with `background: var(--surface-2)`; grid `"grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,1.25fr)] flex-1 min-h-0 border-b"`; each column `"border-r min-h-0 flex flex-col"`; column header `"px-2.5 py-2 text-sm font-semibold uppercase tracking-wide"` muted; list `"flex-1 min-h-0 overflow-y-auto custom-scrollbar p-1.5 space-y-1"`.
  - Column rows: `"w-full rounded-md border px-2.5 py-2 text-left text-sm"`. Manufacturer active: `{ borderColor: 'color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 28%)', background: 'color-mix(in srgb, var(--accent-secondary), var(--surface-1) 90%)', color: 'var(--text-strong)' }`. Resin-type and profile active: `{ borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 30%)', background: 'color-mix(in srgb, var(--accent), var(--surface-1) 89%)', color: 'var(--text-strong)' }`. Inactive for all three: `{ borderColor: 'var(--border-subtle)', background: 'var(--surface-1)', color: 'var(--text-muted)' }`.
  - Profile row content: name uses `FadeText` inside `"flex min-w-0 flex-1 items-center gap-1.5 font-semibold"`; right side `"inline-flex shrink-0 items-center gap-1.5 whitespace-nowrap tabular-nums"` showing the colour swatch (same 3.5×3.5 swatch) and `` `${Math.round(material.layerHeightMm * 1000)} μm` ``.
  - Profile row leading icons: owned-and-shared → `Crown h-3.5 w-3.5` in `#fbbf24` wrapped in a tooltip `Shared with <machine names joined by ", ">`; owned-by-another → `Link2 h-3.5 w-3.5` in `var(--accent-secondary)` with tooltip `Owned by <name>`; locked → `Lock w-3.5 w-3.5`.
  - Remote row: `"w-full rounded-md border px-2.5 py-2 text-left"`; active `{ borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 30%)', background: 'color-mix(in srgb, var(--accent), var(--surface-1) 89%)', color: 'var(--text-strong)' }`; inactive `{ borderColor: 'var(--border-subtle)', background: 'var(--surface-1)', color: 'var(--text-muted)' }`. Locked remote materials show `Lock w-3.5 h-3.5` in `#fbbf24`. Chips: `"text-[11px] rounded-full border px-1.5 py-0.5 whitespace-nowrap"` with `borderColor: var(--border-subtle)`, `background: var(--surface-2)`, muted text.
  - Action row: `"rounded-md border p-2"` with `background: 'color-mix(in srgb, var(--surface-2), transparent 8%)'`, inner `"flex flex-wrap items-center gap-2"`; Delete button carries `ml-auto` and a `Tooltip wrapperClassName="ml-auto"`.
- **States** (choose in this order):
  1. `shouldUseRemoteOnDeviceMaterials` → remote list. True when the adapter supports remote materials, `networkConnection.connected`, a host/ip exists and `reachable !== false` (`:1120-1126`).
  2. `shouldShowRemoteMaterialSelectedPrinterOfflineState` → "Printer Not Connected" card.
  3. `shouldShowRemoteMaterialConnectInfo` → "Connect to a Machine" card.
  4. otherwise the local three-column browser + action row.
  - Description text per state: remote → `Connected <mode> profiles are loaded directly from <printer name>. Selection is read-only for now.`; offline → `Printer not connected. Reconnect the selected machine in Fleet Management, then refresh on-device materials.`; connect → `Connect to a machine to view on-device material profiles.`; local → `Profiles below are bound to <printer name> and follow the selected printer hardware.`
  - Remote list internals: loading first time → `Loading materials from printer…`; empty → `remoteMaterialsError || No on-device materials were returned by this <mode> host.`
  - Header actions remote: `Edit` (disabled when no selection; if `remoteMaterialEditingWipNotice`, the button is `disabled`, `opacity-35`, `pointer-events: none`, and a hover card shows `Work in progress` + the notice) and a square Refresh button (spinner while loading, disabled without a host, `title`/`aria-label` `Refresh remote materials`).
  - Header actions local: `Library` (accent-tinted: `color: var(--accent)`, `borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 42%)'`, `background: 'color-mix(in srgb, var(--accent), var(--surface-1) 92%)'`) and `New` (`accentSecondaryActionStyle93`).
  - Action row buttons (order): `Update Material` (only when `selectedMaterialUpdate`), `Edit`, `Duplicate`, `Import`, `Export`, `Lock`/`Unlock` (only when selected and not official), `Delete` (disabled when none selected, `printerMaterials.length <= 1`, or the material is shared; disabled colour `var(--text-muted)`, enabled `var(--danger)`).
- **Data**: `selectedPrinter`, `printerMaterials`, `availableManufacturers`, `selectedManufacturerValue`, `availableResinTypes`, `selectedResinFamilyValue`, `filteredMaterialProfiles`, `selectedMaterial`, `selectedMaterialIsShared`, `selectedMaterialOwnerName`, `selectedMaterialUpdate`, `remoteMaterials`, `isLoadingRemoteMaterials`, `remoteMaterialsError`, `selectedRemoteMaterialId`, `selectedRemoteMaterial`, `isSavingRemoteMaterialEdit`.
- **Callbacks**: manufacturer click → `setSelectedManufacturer(m)` + `setSelectedResinFamily(null)` + `setSelectedMaterialId(null)`; resin-type click → `setSelectedResinFamily(t)` + `setSelectedMaterialId(null)`; profile row click → `setSelectedMaterialId(id)` and, unless Multi-Tank, `setActiveMaterialProfile(id)`; double-click → same plus either the lock dialog (official or locked) or `setIsMaterialEditorOpen(true)`; remote row click → `handleSelectRemoteMaterial`; remote double-click → `openRemoteMaterialEditDialogForMaterial`; `Edit` (local) → `openSelectedMaterialEditor`; `Duplicate` → `handleDuplicateSelectedMaterial`; `Import`/`Export` → bundle handlers; `Lock`/`Unlock` → `handleToggleMaterialLock` (no-op on official); `Delete` → `requestDeleteSelectedMaterial`; `Update Material` → `handleApplySelectedMaterialOfficialUpdate`; `Library` → `handleOpenMaterialLibrary`; `New` → `handleAddMaterial`; `Refresh` → `loadRemoteMaterials()`; `Edit` (remote) → `openRemoteMaterialEditDialog`.
- **Text**: `Material Settings`; `Manufacturer`; `Material Type`; `Profile`; `Library`; `New`; `Edit`; `Duplicate` (title `Duplicate this material profile as a new editable copy`); `Import`; `Export`; `Lock` (title `Lock this material profile to prevent accidental edits`); `Unlock` (title `Unlock this material profile`); `Delete`; `Update Material` (title `Update v{cur} to v{latest}`); `Update available for selected material profile (v{cur} → v{latest}).`; `Printer Not Connected`; `Open Fleet Management`; `Connect to a Machine`; `Connect Now`; `Loading materials from printer…`; `Refresh remote materials`; `Work in progress`; `Owned by {ownerName}`; `Shared with {machineNames}`; `This material is locked. Unlock it or make a custom copy before applying settings.`; `A network printer is configured, but it is not responding right now. Reconnect it in Fleet Management, then refresh to load on-device materials.`
- **Port target**: `gpui/src/features/profiles/material_pane.rs`, `gpui/src/features/profiles/remote_materials.rs`.

---

## Surface: Printer Editor dialog ("Printer Profile Settings")

- **Source**: `:5523-5966`; official banner `:5549-5574`; Identity `:5576-5660`; Build Volume `:5662-5770`; Display `:5772-5800`; Output `:5802-5960`; footer `:5947-5965`.
- **Renders**: a second-level modal (`z-[60]`, `max-w-[960px]`, `max-h-[88vh]`) with a header (title + `name • manufacturer`), a scrolling body of four bordered cards, and a footer.
- **Layout & styling**: backdrop `"fixed inset-0 z-[60] flex items-center justify-center bg-black/55 p-4 ui-modal-backdrop-enter"`; panel `"w-full max-w-[960px] max-h-[88vh] rounded-xl border shadow-2xl ui-modal-panel-enter flex flex-col"` + `borderColor: var(--border-strong)`, `background: var(--surface-0)`. Header `"flex items-center justify-between px-4 py-3 border-b"`. Body `"p-3 space-y-3 overflow-y-auto custom-scrollbar flex-1 flex flex-col"`. Cards `"rounded-md border p-3"` with `background: 'var(--surface-2)'`; card titles `"ui-meta font-semibold uppercase tracking-wide mb-2"`. Field grids: Identity `"grid grid-cols-1 md:grid-cols-[minmax(0,1fr)_220px] gap-3 md:items-stretch"`; Build volume `"grid grid-cols-1 md:grid-cols-3 gap-2.5"`, margins `"mt-2.5 grid grid-cols-2 md:grid-cols-4 gap-2.5"`; Display/Output `"grid grid-cols-1 md:grid-cols-3 gap-2.5"`. Footer `"px-3 py-3 border-t flex items-center justify-between gap-2 shrink-0"` with `"Changes are applied immediately."` on the left and a `Done` pill on the right.
- **States**:
  - `isSelectedPrinterOfficial` (`selectedPrinter.isOfficial === true`) → banner card shown (`borderColor: 'color-mix(in srgb, #d97706, var(--border-subtle) 36%)'`, `background: 'color-mix(in srgb, #d97706, var(--surface-1) 92%)'`) and these fields disabled: Printer Name, Manufacturer, build-dimension auto toggle, Build Width/Depth/Height, all four margins, Resolution X/Y, Bit Depth, Pixel Size X/Y, Anti-Aliasing, Mirror X/Y, image upload. **Not** disabled for official profiles: User Notes, Output Format, Network Support, Webcam Support/Rotation, Format Version, Settings Mode, Material Mode, Amount of Tanks.
  - Build-dimension card has its own toggle: `role="switch"`, `aria-checked={selectedBuildDimensionMode === 'auto'}`, label `Auto` when auto else `Manual`, style `accentSecondaryActionStyle92` when auto. Comment on the card: `Uses Resolution × Pixel Size. Non-destructive: switching back restores previous manual width/depth.`
  - Webcam Rotation select renders only when `hasCamera !== false`. Format Version / Settings Mode render only when their option list is non-empty; when the profile is official, the option matching the preset default gets `rightContent: <DefaultOptionChip />`, and the trigger gets `selectStyle={MODIFIED_FIELD_STYLE}` when the current value differs from that default.
  - Material Mode + Amount of Tanks render only when `supportsMultiTankFormat`. Amount of Tanks renders only when `materialMode === 'multiTank'`, `step={1} precision={0} min={2} max={16}`.
  - Image well: `role="button"`/`tabIndex=0` only when empty and not official; empty text `Click to add a profile image` (or `No preview image` for official); clear button (`IconButton variant="surface" size="sm"`, `aria-label="Clear profile image"`, `Trash2` in `#fca5a5`) only when an image exists and the profile is not official.
- **Data**: `selectedPrinter` (live), `officialFieldDefaults`, `officialFieldsModified`, `selectedFormatVersionOptions`, `selectedResolvedFormatVersion`, `selectedSettingsModeOptions`, `selectedResolvedSettingsMode`, `selectedBuildDimensionMode`, `selectedPrinterSafetyMargins`, `selectedPrinterUpdate`.
- **Callbacks**: every field writes straight to the store — there is no Save. `updatePrinterProfile(id, { … })` per field; display fields go through `handlePrinterDisplayChange` (which re-resolves `formatVersion`/`settingsMode` and, in auto mode, recomputes width/depth); pixel size through `handlePrinterPixelSizeChange(axis, value)`; bit depth through `handlePrinterBitDepthChange` (`bits = max(1, round(value))`, preserving `description`); the auto/manual toggle through `setBuildDimensionMode`; image upload through `triggerImageUpload(id)` → hidden file input → `normalizeUploadedPrinterImageDataUrl` → `updatePrinterProfile({ imageDataUrl })`; clear image → `updatePrinterProfile({ imageDataUrl: undefined })`; Network Support `None` also closes network settings (`setIsNetworkSettingsOpen(false)`, `setIsAddingNetworkPrinter(false)`); `Create Custom Copy` → `handleDuplicateSelectedPrinterAsCustom()` (duplicate + pick + `setIsEditingPrinter(true)`); `Done`/X/backdrop → `setIsEditingPrinter(false)`.
- **Text**: `Printer Profile Settings`; subtitle `{name} • {manufacturer || 'Generic'}`; `Official Profile — Edits Limited`; `Only some fields are editable here; everything else unlocks with a custom copy. Custom copies may increase the risk of print failure or injury.`; `Create Custom Copy`; `Identity`; `Printer Name`; `Manufacturer`; `User Notes`; `Upload profile image`; `Click to add a profile image`; `No preview image`; `Clear profile image`; `Build Volume`; `Auto-Calculate Width/Depth`; `Uses Resolution × Pixel Size. Non-destructive: switching back restores previous manual width/depth.`; `Auto`; `Manual`; `Build Width (mm)`; `Build Depth (mm)`; `Build Height (mm)`; `Front Margin (mm)`; `Back Margin (mm)`; `Left Margin (mm)`; `Right Margin (mm)`; `Display`; `Resolution X (px)`; `Resolution Y (px)`; `Bit Depth`; `Pixel Size X (μm)`; `Pixel Size Y (μm)`; `Anti-Aliasing`; `Output`; `Output Format`; `Network Support`; `Webcam Support`; `Webcam Rotation`; `Format Version`; `Settings Mode`; `Mirror X`; `Mirror Y`; `Material Mode`; `Single Material`; `Multi-Tank`; `Amount of Tanks`; `Changes are applied immediately.`; `Done`; `Close printer editor`. (`Webcam Rotation` options are literal `0°`, `90°`, `180°`, `270°`, not translated.)
- **Port target**: `gpui/src/features/profiles/printer_editor.rs`.

---

## Surface: Material Editor dialog

- **Source**: `:5410-5522`; header `:5415-5434`; body `:5436-5466`; footer `:5468-5520`.
- **Renders**: `z-[60]`, `max-w-[920px]`, `max-h-[88vh]`; header `Material Profile Settings` (or `Edit <EXT> Material Profile` in replacement mode) + `name • brand`, body = either `ReplacementMaterialEditorShell` **or** the three stacked sections, footer = Delete Material / Cancel / Save Material.
- **Layout & styling**: panel `"w-full max-w-[920px] max-h-[88vh] rounded-xl border shadow-2xl ui-modal-panel-enter flex flex-col"`; body `"p-3 space-y-3 overflow-y-auto custom-scrollbar flex-1 flex flex-col"`; footer `"px-3 py-3 border-t flex items-center justify-between gap-2 shrink-0"`; footer buttons are pills (`rounded-full`), Delete is `"!h-8 !px-3 !py-0 text-sm inline-flex items-center gap-1 rounded-full disabled:opacity-45"`, Cancel `"text-xs rounded-full"`, Save `accentSecondaryActionStyle92`.
- **States**:
  - `usePluginLocalSettingsAsReplacement` (= `Boolean(selectedLocalMaterialSettingsAdapter?.replacesDefaultMaterialSettings)`) → `ReplacementMaterialEditorShell` with `tabs={replacementMaterialEditorTabs}`, `activeTabId={materialEditorTab}`, `activeTabStyle={accentSecondaryActionStyle92}`, draft/draft-setter, `printerDitherBitDepth`, `printerPanelBitDepth`, `outputFormat`, `settingsMode`, `adapter`, `localSettingsByOutput`, `printerProfiles`, `shareablePrinterIds={editShareablePrinterIds}`, `ownerPrinterId={selectedMaterial?.printerProfileId}`.
  - Otherwise stacked: `MaterialProfileFormSections` → `MaterialAntiAliasingSection` → `PluginLocalMaterialSettingsSections` (with `replacementMode={false}`), all fed `editMaterialDraft` / `editMaterialLocalSettingsByOutput`.
  - Delete disabled when `!selectedMaterial || printerMaterials.length <= 1 || selectedMaterialIsShared`; tooltip explains ownership when shared.
- **Data**: `selectedMaterial`, `editMaterialDraft`, `editMaterialLocalSettingsByOutput`, `materialEditorTab`, `replacementMaterialEditorTabs` (`:860-880`), `selectedLocalMaterialSettingsAdapter`, `printerDitherBitDepth` (2..7 clamp for <8-bit panels), `printerPanelBitDepth` (raw declared bits).
- **Callbacks**: open happens from the profile rows / `openSelectedMaterialEditor`; on open an effect (`:3280-3320`) seeds `editMaterialDraft` from `selectedMaterial` (including `color` — see gotchas) and `editMaterialLocalSettingsByOutput` via `mergeWithLocalSettingsDefaults(outputFormat, settingsMode, material.localSettingsByOutput)`; it also restores the last tab from `dragonfruit.materialSettings.activeTabByPrinterProfile.v1` (validated against `replacementMaterialEditorTabs`), falling back to `materialEditorInitialTabOverrideRef.current` then `replacementMaterialEditorDefaultTab`. Tab change → `handleMaterialEditorTabChange` (state + persist per printer). Save → `handleSaveMaterialEdits` (writes `name|selected.name`, `brand|'Default'`, `currencyCode.trim().toUpperCase()|'USD'`, plus the whole draft and local settings; closes). Cancel/X/backdrop → close. Delete Material → `requestDeleteSelectedMaterial()` **and** closes the editor.
- **Text**: `Material Profile Settings`; `Edit {EXT} Material Profile`; `{name} • {brand}`; `Delete Material`; `Cancel`; `Save Material`; `Close material editor`.
- **Port target**: `gpui/src/features/profiles/material_editor.rs`.

---

## Surface: Create Material dialog

- **Source**: `:6329-6421`.
- **Renders**: same shell as the editor but with the `newMaterialDraft` / `newMaterialLocalSettingsByOutput` pair and a footer of Cancel / Save Material.
- **Layout & styling**: identical classes to the material editor; footer is `"px-3 py-3 border-t flex items-center justify-end gap-2"` (no delete button).
- **States**: title `Create <EXT> Material Profile` in replacement mode, else `Create Material Profile`; subtitle is the printer name; body is the same replacement/stacked split as the editor with `ownerPrinterId={selectedPrinter?.id}` and `shareablePrinterIds={newShareablePrinterIds}`.
- **Data**: `newMaterialDraft` (seeded by `handleAddMaterial` / `handleApplyMaterialLibraryPreset`), `newMaterialLocalSettingsByOutput`.
- **Callbacks**: `Save Material` → `handleCreateMaterial` → `addMaterialProfile(printerId, { …newMaterialDraft, name: trimmed || `Material ${printerMaterials.length + 1}`, brand: trimmed || 'Default', localSettingsByOutput })`, then selects the new material's brand/family/id and calls `setActiveMaterialProfile(newId)`, closes. Cancel/X/backdrop → close.
- **Text**: `Create Material Profile`; `Create {EXT} Material Profile`; `Cancel`; `Save Material`; `Close create material dialog`.
- **Port target**: same module as the editor (`material_editor.rs`), `CreateMaterialDialog`.

---

## Surface: Printer Library dialog

- **Source**: `:5255-5398`; card factory `renderPresetLibraryCard` `:3765-3905`; footer `:5376-5397`.
- **Renders**: `z-[65]` overlay; panel `max-w-[1040px]`, height `min(90vh, 1066px)`; header with `Printer` icon tile, title/subtitle, close; body = a 220px manufacturer sidebar (search box on top, brand buttons) + a preset grid grouped by family; footer with selection count and the Add button.
- **Layout & styling**: backdrop `"fixed inset-0 z-[65] flex items-center justify-center bg-black/55 p-4 ui-modal-backdrop-enter"`; panel `"w-full max-w-[1040px] min-h-0 rounded-xl border shadow-2xl overflow-hidden ui-modal-panel-enter flex flex-col"` + `background: var(--surface-0)`, `borderColor: var(--border-strong)`, `height: 'min(90vh, 1066px)'`, `maxHeight: 'calc(100vh - 2rem)'`. Header icon tile background `'linear-gradient(135deg, color-mix(in srgb, var(--accent-secondary), var(--surface-1) 84%), color-mix(in srgb, var(--accent), var(--surface-1) 90%))'` with `Printer` in `var(--accent-secondary)`.
  - Body grid `"grid grid-cols-[220px_minmax(0,1fr)] grid-rows-[1fr] flex-1 min-h-0 overflow-hidden"`; sidebar `"border-r flex flex-col min-h-0"` with `background: 'color-mix(in srgb, var(--surface-1), transparent 8%)'`.
  - Search: `"ui-input w-full h-8 text-xs"` with `paddingLeft: '2.5rem'`, `paddingRight: '0.625rem'`, `Search` icon `"absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 pointer-events-none"`; placeholder `Search printers`.
  - Manufacturer buttons `"w-full rounded-md border px-2.5 py-2 text-left text-sm font-semibold"`; active `{ borderColor: 'color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 35%)', background: 'color-mix(in srgb, var(--accent-secondary), var(--surface-1) 88%)', color: 'var(--text-strong)' }`; inactive `{ borderColor: 'var(--border-subtle)', background: 'var(--surface-1)', color: 'var(--text-muted)' }`.
  - Cards: grid `"grid grid-cols-[repeat(auto-fill,minmax(176px,1fr))] gap-2.5"`. Card button `"rounded-md border p-2.5 text-left disabled:opacity-55 transition-[background-color,box-shadow,opacity] duration-150"`; background `isAlreadyAdded ? 'color-mix(in srgb, var(--accent-secondary), var(--surface-1) 93%)' : isSelected ? 'color-mix(in srgb, var(--accent), var(--surface-1) 86%)' : 'var(--surface-1)'`; box-shadow `isAlreadyAdded ? 'inset 0 0 0 1px color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 45%)' : isSelected ? 'inset 0 0 0 1.5px color-mix(in srgb, var(--accent), transparent 40%)' : 'inset 0 0 0 0 transparent'`. Image well `"h-[132px] rounded-md border overflow-hidden flex items-center justify-center relative"` with `background: '#2b3039'` (hard-coded, not a token). Text: name `"truncate text-[12px] font-semibold leading-tight"`; meta row `"mt-0.5 grid grid-cols-[minmax(0,1fr)_auto] items-center gap-2"`; the `Added` chip is always laid out but `visibility: isAlreadyAdded ? 'visible' : 'hidden'`.
  - Family group header when not searching: a centred label between two 1px gradient rules (`linear-gradient(90deg, transparent 0%, color-mix(in srgb, var(--border-subtle), transparent 52%) 18%, color-mix(in srgb, var(--text-muted), white 28%) 100%)`, mirrored on the right), label `"shrink-0 text-sm font-semibold tracking-[0.08em]"` muted.
  - Footer `"flex items-center justify-between gap-3 px-3 py-3 border-t shrink-0"`; count `"text-[12px]"`.
- **States**: open when `showPresetPicker`. Selected set `selectedLibraryPresetIds`. A card is `disabled` when already added (family: every variant id added; concrete: its id added). A card is "selected" (family: any variant id selected; concrete: its id). Clicking a family preset (`modelVariants.length > 0`) **or** any preset with `modelVariantDetectPath` opens the Initial Setup modal instead of toggling; otherwise it toggles the set. `isSearching = presetSearch.trim().length > 0` switches the layout from family groups to one flat grid. Manufacturer filter is ignored while searching (`manufacturerMatch = search.length > 0 || preset.manufacturer === selectedPresetManufacturer`). `presetManufacturers` lists non-generic manufacturers alphabetically, then "generic" ones last.
- **Data**: `libraryPrinterPresets` (`getLibraryPrinterPresets()`), `presetSearch`, `selectedPresetManufacturer`, `selectedLibraryPresetIds`, `addedOfficialPresetIds`, `groupedFilteredPrinterPresets`.
- **Callbacks**: search → `setPresetSearch`; manufacturer → `setSelectedPresetManufacturer`; card click → `handleToggle` (open chooser or toggle); `Add printers` → `handleAddSelectedPrinterPresets()` (adds every selected preset via `addPrinterProfileFromPreset`, picks the last one, closes, clears search/selection, resets manufacturer to the first); close/X/backdrop → `setShowPresetPicker(false)`.
- **Text**: `Printer Library`; `Choose an official printer preset to add.`; `Search printers`; `Added`; `{n} printer{s} selected` / `Select printers to add`; `Add printers` / `<Plural value=n one="Add # printer" other="Add # printers" />`; `Close printer library`.
- **Port target**: `gpui/src/features/profiles/printer_library.rs`.

---

## Surface: Initial Setup — `PrinterVariantPickerModal`

- **Source**: `src/components/printers/PrinterVariantPickerModal.tsx:1-718` (whole file); formatters `:24-92`; scan `:186-300`; connect+detect `:302-395`; render `:404-712`.
- **Renders**: `z-[65]` overlay, `max-w-[560px]`, `max-h-[88vh]`, `role="dialog" aria-modal="true" aria-label="Initial Setup"`; header (title `Initial Setup`, subtitle = `preset.libraryDisplayName ?? preset.name`, close X); a scrolling body whose content depends on `mode` (`'choose' | 'scan' | 'manual' | 'ip'`); a footer shown only when `mode !== 'choose'`.
- **Layout & styling**:
  - Backdrop `"ui-modal-backdrop-enter fixed inset-0 z-[65] flex items-center justify-center bg-black/55 p-4"`; panel `"ui-modal-panel-enter flex max-h-[88vh] w-full max-w-[560px] min-h-0 flex-col overflow-hidden rounded-xl border shadow-2xl"` + `borderColor: var(--border-strong)`, `background: var(--surface-0)`.
  - Header `"flex shrink-0 items-center justify-between gap-4 border-b px-4 py-3"`; title `"truncate text-sm font-semibold"`; subtitle `"ui-meta truncate"`.
  - Body `"custom-scrollbar min-h-0 flex-1 space-y-3 overflow-y-auto p-3"`.
  - Status card `"rounded-xl border px-3 py-2.5"` with `background: 'color-mix(in srgb, var(--surface-1), var(--surface-0) 30%)'`.
  - Device row / variant row: `"flex w-full items-center gap-3 rounded-xl border px-3 py-2.5 text-left transition-colors hover:border-[color-mix(in_srgb,var(--accent),var(--border-subtle)_45%)] disabled:opacity-55"` with `background: 'color-mix(in srgb, var(--surface-1), var(--surface-0) 40%)'`.
  - Device row icon tile `"inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-md border"` with `background: var(--surface-1)`, colour `var(--accent-secondary)`, `Printer h-4 w-4`; trailing `ChevronRight h-4 w-4` (or `Spinner` while connecting).
  - Detected-variant chip: `"inline-flex h-6 min-w-[46px] shrink-0 items-center justify-center rounded border px-1.5 text-[10px] font-bold leading-none"` with text `` `${bits}-bit` ``; 8-bit green gradient, 3-bit red gradient, otherwise accent-secondary gradient (same mixes as the library card badge).
  - Detected card in `'ip'` mode uses `borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 55%)'`, `background: 'color-mix(in srgb, var(--accent), var(--surface-1) 92%)'`.
  - Footer `"flex shrink-0 items-center justify-between gap-3 border-t px-2 py-2"`; primary action uses `ACCENT_ACTION_STYLE` (`color: var(--accent-secondary)`, border `color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 42%)`, background `color-mix(..., var(--surface-1) 92%)`).
  - `Configure Networking Later` is a bare text button: `"text-xs font-semibold underline-offset-2 hover:underline"` muted.
- **States**:
  - `networkAdapter` is non-null only when the preset has `modelVariantDetectPath` **and** the adapter exposes `discover`, `connect` and `printerData`. Initial `mode = networkAdapter ? 'choose' : 'manual'`.
  - `!networkAdapter && variants.length === 0` → `This printer has no setup options.`
  - `'choose'` → `SetupMethodChooser` (Auto Detect → `'scan'` + `runScan()`; Manual IP → `'ip'`).
  - `'scan'` status card precedence, in order: detected (name + `The printer reported this variant.` / variant name + optional `This model is already added.`) → connecting (`Spinner` + `Detecting model…`) → scanning (label + `ProgressBar indeterminate size="xs"`) → devices found (`{count, plural, one {# printer found} other {# printers found}}` + `Select a printer to detect its model.`) → otherwise `scanError ?? No printers found.` plus a `Scan again` button (`"!h-6 !px-2.5 !py-0 text-[11px] rounded-md"`).
  - Device list renders when `!detectedVariant && devices.length > 0`; each row shows `printerName || printerModel || hostName || ipAddress` and `ip[:port]`.
  - `detectError` line: `"text-[11px] leading-snug"` with hard-coded `#f87171`.
  - `Configure Networking Later` link appears only when `!detectedVariant && !connecting && devices.length === 0 && showConfigureLater`; `showConfigureLater` is set after the scan finishes or after a 25 s timer, whichever comes first (comment: *"If the scan has been going ~25s without a detection, offer to skip networking so the user isn't stuck waiting on a long subnet scan."*). Clicking it goes to `'manual'` when there are variants, otherwise calls `onSelect(preset.presetId)` (no network context).
  - `'ip'` → either the detected card (with `ManualIpEntryCard` hidden) or `ManualIpEntryCard` (`isConnecting={busy}`).
  - `'manual'` → one row per variant, disabled when already added, trailing `Added` text or chevron.
  - Footer hidden in `'choose'`; `SetupModeButton mode={mode === 'scan' ? 'auto' : 'manual'}` toggles between scan and ip (re-running the scan when switching to it); `Cancel` always; `Add printer` only when `showNetworkMode && detectedVariant && !detectedAlreadyAdded`.
- **Data**: props `preset: PrinterPreset`, `variants: PrinterPreset[]`, `addedPresetIds: ReadonlySet<string>`, `onSelect(variantPresetId, networkContext?)`, `onClose`. State `mode`, `manualIp`, `scanning`, `devices`, `scanError`, `busy`, `connectingHost`, `detectedVariant`, `detectedDeviceName`, `detectError`, `showConfigureLater`. Refs `pendingNetworkRef` (host/port/printerName/mode for the created profile) and `scanAbortRef`. Escape closes.
- **Callbacks / network protocol** (adapter ops via `pluginNetworkFetch`):
  - `runScan()` fires the local-hostname pass and the progressive subnet pass **concurrently** (`Promise.all`); local pass: `operation=adapter.operations.discover`, `mode=preset.networkSupport`, `scanScope: 'local-hostnames'`, `networkFilter: preset.networkFilter ?? undefined`, `ports: [80, 8080]`. Subnet pass loops `scanScope: 'subnet'`, `progressive: true`, `batchStart`, `batchSize: 96`, same `networkFilter`/`ports`. Devices are deduped by `ipAddress`; the subnet loop stops as soon as one device is found (comment: *"ARP priority puts the likely printer in the first batch — stop scanning once a matching device is found so discovery completes in a couple of seconds instead of a full subnet sweep."*), when aborted, on `done`, or when `nextBatchStart <= batchStart`. Errors set `scanError` (`Network scan failed.` or the payload's `error`); zero results set `No {presetName} printers found on the network.`
  - `connectAndDetect(host, port?)`: aborts the scan, then `operations.connect` with `host`, `port`, `networkFilter`; requires `connected === true` (otherwise `statusText` or `Couldn't connect to {host}.`); then `operations.printerData` with `host`, `port`, `path: preset.modelVariantDetectPath`; requires `ok === true` (otherwise `error` or `Could not read printer data.`); reads `bitdepth` and `printerName`. Variant resolution: `variants.length > 0 ? matchPrinterVariantByBitDepth(variants, bits| null)`; no match with variants → `Detected a {bitsLabel} screen that doesn't match any known variant of {presetName}.` (bitsLabel `'?'` when null, otherwise `` `${bits}-bit` ``); presets without variants are "detected" as themselves. On success stores `pendingNetworkRef = { host: resolvedHost, port: resolvedPort, printerName: reported ?? undefined, mode: preset.networkSupport }`.
  - `handleConfirmDetected()` → `onSelect(detectedVariant.presetId, pendingNetworkRef.current)`.
  - `handleSelectVariant(variant)` → `onSelect(variant.presetId)` (no network context).
  - `SetupModeButton` label logic: shows the *other* mode.
- **Text** (all via `msg`): `Initial Setup`; `Manual`; `Auto`; `Close`; `Cancel`; `Add printer`; `Added`; `Detecting model…`; `The printer reported this variant.`; `This model is already added.`; `Select a printer to detect its model.`; `No printers found.`; `Scan again`; `Configure Networking Later`; `Network scan failed.`; `Could not read printer data.`; `This printer has no setup options.`; `Detected: {variantName}` (comment *"Confirmation that the printer model variant was auto-detected over the network. {variantName} is the variant display name."*); `Scanning your network for {presetName} printers…` (comment *"Auto-detection is scanning the network for a matching printer. {presetName} is the printer family name."*); `{count, plural, one {# printer found} other {# printers found}}` (comment *"How many matching printers the network scan discovered. {count} is a positive integer."*); `Detected a {bitsLabel} screen that doesn't match any known variant of {presetName}.`; `Couldn't connect to {host}.` (comment *"Network connect failed for the given printer host/IP. {host} is the address."*); `No {presetName} printers found on the network.`
- **Port target**: `gpui/src/features/profiles/initial_setup.rs` (`VariantPickerModal`), plus a `network_discovery` module for the scan/connect/probe protocol.

---

## Surface: `SetupMethodChooser`, `SetupModeButton`, `ManualIpEntryCard`

- **Source**: `src/components/printers/SetupMethodChooser.tsx:9-161`.
- **Renders**: (a) two side-by-side method cards; (b) a single pill button that flips Auto/Manual; (c) a bordered card with an IP text field and an inline Connect button.
- **Layout & styling**:
  - Chooser container `"grid grid-cols-2 gap-2"`. Each card: `"flex items-center gap-3 rounded-xl border p-3 text-left transition-colors hover:border-[color-mix(in_srgb,var(--accent),var(--border-subtle)_45%)]"`, `borderColor: var(--border-subtle)`, `background: var(--surface-1)`. Icon tile `"inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-md border"` with `borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 55%)'`, `background: 'color-mix(in srgb, var(--accent), var(--surface-1) 88%)'`, `color: var(--accent)`. Title `"block truncate text-sm font-semibold"`; description `"mt-0.5 block truncate text-[11px] leading-snug"` muted.
  - `SetupModeButton`: `Button variant="secondary" size="auto"` `"!h-8 !px-3 !py-0 text-xs rounded-full"`.
  - `ManualIpEntryCard`: `"rounded-xl border p-3.5"` + `borderColor: var(--border-subtle)`, `background: var(--surface-1)`; heading `"text-xs font-semibold uppercase tracking-wide"` muted; row `"mt-2 flex items-center gap-2"`; input `"ui-input h-[34px] min-w-0 flex-1 px-2.5 py-1.5 font-mono text-sm tabular-nums"` with `placeholder="192.168.1.100"`, `inputMode="decimal"`, `autoComplete="off"`, `spellCheck={false}`; button `"!h-[34px] !px-3 !py-0 shrink-0 text-xs inline-flex items-center gap-1 rounded-full disabled:opacity-45"`, colour `var(--accent-secondary)`, `Wifi` icon or `Spinner`, disabled while connecting or when the value is blank.
- **States**: `SetupModeButton` label = `mode === 'auto' ? 'Manual' : 'Auto'` — it names the mode you switch **to**, not the current one. `ManualIpEntryCard` shows `Connecting…` + spinner while `isConnecting`; Enter in the field triggers `onConnect`.
- **Callbacks**: `onAutoDetect()`, `onManualIp()`, `onSwitch()`, `onChange(value)`, `onConnect()`.
- **Text**: `Auto Detect`; `Find your printer on the network`; `Manual IP Entry`; `Enter your printer's IP address`; `Connect` (or the `connectLabel` override); `Connecting…`; `Manual`; `Auto`.
- **Port target**: `gpui/src/features/profiles/setup_shared.rs` — shared by Initial Setup and Fleet Management; keep it shared, the source comment says why: *"Shared printer setup pieces used by both the Initial Setup dialog and the Network Settings modal, so the 'Auto Detect / Manual IP Entry' flows look identical everywhere."*

---

## Surface: Network Settings / Fleet Management

- **Source**: `src/components/settings/FleetManagement.tsx:1-458`; rendered from `ProfileSettingsModal.tsx:6423-6472` inside a `z-[70]` overlay.
- **Renders**: `max-w-[560px]`, `max-h-[92vh]` panel: header (title `Fleet Management` when >1 managed printer else `Network Settings`, subtitle = printer name, close X); body = managed-printer card (list + Add/Done Adding footer) when any are saved, then the add-printer flow; footer Auto/Manual toggle + Cancel, only while the add flow is in `scan`/`manual`.
- **Layout & styling**: panel `"flex max-h-[92vh] w-full max-w-[560px] min-h-0 flex-col overflow-hidden rounded-xl border shadow-2xl ui-modal-panel-enter"`. Body `"custom-scrollbar min-h-0 flex-1 space-y-3 overflow-y-auto p-3"`. Managed card `"rounded-xl border p-3.5"` with `background: var(--surface-1)`; its list `"mt-2.5 space-y-2 max-h-[328px] overflow-y-auto custom-scrollbar pr-1"`; each device `"relative rounded-md border px-3 py-2.5 pl-10"` with `background: var(--surface-0)` and a left gradient glow strip `"absolute inset-y-0.5 left-0.5 w-14 rounded-md pointer-events-none"` (green/red 5-stop gradient), plus a `Check`/`X` glyph `h-[18px] w-[18px] strokeWidth={3}` in the card background colour (a mask trick) — `aria-label` `Printer online`/`Printer offline`, `title` `Online`/`Offline`.
  - Per-device buttons: `Select` (`"!h-7 !px-2.5 !py-0 text-xs rounded-full"`, hidden on the active device), refresh/connect (`"inline-flex h-7 w-7 … rounded-full border … disabled:opacity-45"`, colour `var(--accent-secondary)`), disconnect (same shape, colour `var(--text-strong)`/muted, disabled unless connected), remove (`Trash2`, danger, only when not connected).
  - Discovery status card `"rounded-xl border px-3 py-2.5"` with `background: 'color-mix(in srgb, var(--surface-1), var(--surface-0) 30%)'`; indeterminate bar markup uses the global classes `ui-loading-track h-1.5 w-full rounded-full` + `ui-loading-indicator` with `background: 'linear-gradient(90deg, color-mix(in srgb, var(--accent-secondary), var(--surface-0) 30%), var(--accent))'`.
  - Discovered row `"flex items-center gap-3 rounded-md border px-3 py-2.5"` + `background: var(--surface-0)`; list `"mt-2 space-y-1.5 max-h-[180px] overflow-y-auto custom-scrollbar pr-1"`.
  - Count badge `"rounded-full border px-2 py-0.5 text-[10px] font-semibold tabular-nums"`.
  - Section headings `h5`: `"text-xs font-semibold uppercase tracking-wide"` muted. "Other Printers" separator: `"my-1.5 border-t pt-1"` + `"text-[10px] uppercase tracking-wide"`.
- **States**: `hasMultiplePrinters = managedPrinters.length > 1` decides the title; the active managed printer is hoisted to the top and the rest fall under the `Other Printers` rule. `isOnline = device.connected && printerReachabilityByDeviceId?.[device.id] !== false`. Add-flow internal `method: 'scan' | 'manual' | null`, reset to `null` whenever `showAddPrinterFlow` changes. Scanning state shows `Scanning your local network…` + phase label (`networkScanPhaseLabel || Scanning network…`); idle shows `<n> printers found` (raw template, not the ICU formatter) or `No printers found.` plus `Scan again`. Discovered entries that match a saved device by IP and are connected show a green `Connected` label instead of a Connect button.
- **Data**: all props from the modal (see the modal's render block `:6423-6472`) — `managedPrinters`, `printerReachabilityByDeviceId`, `activePrinterId`, `showAddPrinterFlow`, `onEnterAddPrinterFlow`/`onExitAddPrinterFlow`, `onRunDiscovery`, `isNetworkScanning`, `networkScanPhaseLabel`, `discoveredPrinters`, `isNetworkConnecting`, `onConnectDiscovered`, `onSelectManagedPrinter`, `onReconnectManagedPrinter`, `onDisconnectManagedPrinter`, `onRemoveManagedPrinter`, `networkIpAddress`, `onNetworkIpAddressChange`, `onConnectManual`, `activePrinterSummary`, `onClose`.
- **Callbacks**: all delegated to the modal's handlers (see "Network handlers" below). `Add printer` → `onEnterAddPrinterFlow`; `Done Adding` → `onExitAddPrinterFlow`; footer toggle → local `setMethod` + `onRunDiscovery`.
- **Text**: `Fleet Management`; `Network Settings`; `Managed Printers`; `{saved, plural, one {# saved} other {# saved}} • {online, plural, one {# online} other {# online}}` (comment *"Counter above the managed printer list: how many printers are saved and how many are reachable."*); `Your fleet for this profile.`; `Primary printer assigned to this profile.`; `No printers saved yet. Add one below to start.`; `Other Printers`; `Select`; `Refresh`; `Connect`; `Disconnect`; `Remove saved printer`; `Remove`; `Add printer`; `Done Adding`; `Scanning your local network…`; `Scanning network…`; `{n} printers found`; `No printers found.`; `Scan again`; `Discovered Printers`; `{count, plural, one {# found} other {# found}}` (comment *"Badge counting the printers the network scan turned up."*); `Connected`; `Connecting…`; `Cancel`; `Online`; `Offline`; `Printer online`; `Printer offline`; `Close network settings`.
- **Port target**: `gpui/src/features/profiles/fleet_management.rs`.

### Network handlers in the modal (the behaviour FleetManagement drives)

- `handleRunNetworkDiscovery` (`:2400-2700`): guarded by `discoveryInFlightRef`; monotonic `discoveryRunIdRef` so a stale run cannot write state. Progress script: `8%` `Resolving friendly .local hostnames…`; connect the configured host first if `networkIpAddress` is non-empty and seed it as `` `${printerId}-configured-host` ``; then the local-hostname pass (`scanScope: 'local-hostnames'`, `localHostnames` = adapter defaults + a slugified `` `${printer.name}.local` `` + the configured host if it ends in `.local`, `ports: [80, 8080]`, `debugNetworkFilter: true`); `44%`/`42%` verify cached discoveries one by one via `connect`; `56%` start the progressive subnet loop (`batchSize: 96`, `probeTimeoutMs: 1200`, `subnetConcurrency: 84`, `excludeHosts`/`seedIps` = local discoveries) computing `Math.round(56 + ratio*42)` clamped to `[56, 98]` with phase `` `Scanning local subnet… ${scanned}/${total} endpoints` ``; finish at `100` / `Scan complete`. Final message is one of three exact templates (`Found N <mode> device(s) (resolved X .local hostnames, scanned Y subnet hosts / Z endpoints).`, `No <mode> devices found (…)`, `No local IPv4 subnet detected by the scanner. Try entering printer IP and scanning again.`). On error: message + `Scan failed` + 100%. After 500 ms the progress resets to 0/''.
- `handleConnectNetworkPrinter({host, closeOnSuccess, preferredName})` (`:2702-2900`): empty host → message `Enter a printer IP address or host first.` and a failed `updatePrinterNetworkConnectionStatus` with `Missing printer host/IP.`. Host `192.168.999.999` is a **debug sentinel**: seeds two fake devices (`Debug Dummy Athena A` / `B`, IPs `192.168.999.999` and `192.168.999.998`) via `upsertPrinterNetworkDevice`, then reports `Debug mode: seeded 2 dummy printers (Athena A + Athena B).`. Otherwise calls `connect`, then on success resolves a display name with this precedence: payload `printerName`/`hostName` (if not generic and not equal to the host/ip) → `preferredName` option → existing known device `displayName`/`hostName` → discovered entry name → `resolveNetworkDiscoveryDisplayName(...)` → host. It then `upsertPrinterNetworkDevice(..., { select: true })`, `updatePrinterNetworkSettings`, `updatePrinterNetworkConnectionStatus`, sets the message `Connected to <name>`, exits add mode, optionally closes the pane. Failure writes `connected: false` with `statusText`.
- `handleSelectManagedPrinter` / `handleDisconnectManagedPrinter` / `handleRemoveManagedPrinter` (`:2902-2935`): store calls + messages `Selected <name>`, `Disconnected <name>`, `Removed <name> from this profile fleet.`; removing clears `networkIpAddress` when it matched.
- Remote materials: `loadRemoteMaterials({background})` (`:2320-2430`) — no adapter/host → error `Connect to a <mode> printer to load on-device materials.`; uses the cache keyed `` `${pluginId}::${printerId}::${host}` ``; `pluginNetworkFetch({operation: adapter.operations.materials, host})`; filters to entries with string `id` and `name`; picks selection `readRemoteMaterialByPrinter()[printerId] || persistedRemoteMaterialIdForSelectedPrinter || current`, else the first unlocked, else the first; writes `updatePrinterNetworkConnectionStatus` with the resolved process values. `handleSelectRemoteMaterial` persists `dragonfruit.network.remoteMaterialByPrinter.v1[printerId] = materialId` and updates the connection status. `handleSaveRemoteMaterialEdits` POSTs `operations.materialsEdit` with numeric `profileId` and `denormalizeEditDraftForBackend(draft)`, requires `response.ok && payload.ok === true`, then sets the message `` `${mode} profile updated. Refreshing materials…` `` and reloads.

---

## Surface: Remote Material Edit dialog

- **Source**: `src/components/settings/ProfileSettingsModal.tsx:6852-7093` (`RemoteMaterialEditDialog`), mounted `:6474-6489`.
- **Renders**: `z-[71]` overlay, `max-w-[920px]`, `max-h-[88vh]`; header `Edit <mode> Material Profile` + `<name> • Profile ID <id>`; body with a Basic/Advanced tab pair and section cards of fields; sticky footer.
- **Layout & styling**: backdrop `"fixed inset-0 z-[71] flex items-center justify-center bg-black/55 p-4 ui-modal-backdrop-enter"` (backdrop click closes only when `!isSaving`); panel `"w-full max-w-[920px] max-h-[88vh] rounded-xl border shadow-2xl overflow-hidden flex flex-col ui-modal-panel-enter"`. Tabs row `"flex items-center gap-1.5 border-b pb-2"`; active tab style `{ color: 'var(--accent-secondary)', borderColor: 'color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 42%)' }`, inactive `{ color: 'var(--text-muted)' }`, both `"!h-7 !px-2.5 !py-0 text-xs rounded-md"`. Section cards `"rounded-md border p-3"` + `background: var(--surface-2)`; section headers `"ui-meta font-semibold uppercase tracking-wide mb-2 flex items-center justify-between gap-2"`; fields `"grid grid-cols-1 md:grid-cols-2 gap-2.5"`. Footer `"px-3 py-3 border-t flex items-center justify-between gap-2 shrink-0 sticky bottom-0"` with `background: 'color-mix(in srgb, var(--surface-0), transparent 4%)'`, `backdropFilter: 'blur(8px)'`.
- **States**: Basic tab uses `basicSections`; numeric-looking entries become `LabeledNumberInput`, others `LabeledInput`; `SupportBeforeWait`/`BeforeWait` are disabled while the adapter reports Dynamic Wait enabled, with help text `Controlled by Dynamic Wait. Disable Dynamic Wait in Advanced settings to edit this value.`; `SupportLayerNumber`/`TransitionalLayer` coerce to non-negative integers. A `Dynamic Wait active` pill (`"text-[11px] rounded-full border px-2 py-0.5 normal-case tracking-normal"`, amber) appears on the `timing` section. Advanced tab uses `advancedSections`; empty → `No additional advanced controls were found for this profile.` Save shows `Spinner` + `Saving…` and is disabled while saving.
- **Data**: `remoteMaterialEditDraft` (a `Record<string,string>`), `remoteMaterialEditTab`, `isSavingRemoteMaterialEdit`, `selectedRemoteMaterial`, `selectedNetworkModeLabel`, `basicRemoteMaterialSections`, `advancedRemoteMaterialSections`, `remoteMaterialPrimaryFieldByKey`, `isRemoteMaterialDynamicWaitEnabledState`.
- **Callbacks**: `onDraftChange` (functional setter), `onEditTabChange`, `onClose`, `onSave`; field labels come from `primaryFieldByKey.get(key)?.label ?? formatRemoteMaterialFieldLabel(key)` in Basic and always from `formatRemoteMaterialFieldLabel(key)` in Advanced.
- **Text**: `Edit {mode} Material Profile`; `{name} • Profile ID {id}`; `Basic`; `Advanced`; `Dynamic Wait active` (title `Dynamic Wait is controlling Wait Before Print fields`); `No additional advanced controls were found for this profile.`; `Applies to {mode} profile on the printer (all scalar parameters from this profile).`; `Cancel`; `Saving…`; `Save to {mode}`; `Close {mode} edit dialog`.
- **Port target**: `gpui/src/features/profiles/remote_material_editor.rs`.

---

## Surface: Printer update diff dialog

- **Source**: `:6491-6570`; diff computation `:1370-1525`.
- **Renders**: `z-[74]`, `max-w-[860px]`, `max-h-[88vh]`; header `Official Printer Update` + `name • v<cur> → v<latest>`; a note strip; a 3-column table (`Field` / `Current` / `Update`) or an "no changes" note; footer Cancel + Apply Update.
- **Layout & styling**: backdrop `"fixed inset-0 z-[74] flex items-center justify-center bg-black/60 p-4 ui-modal-backdrop-enter"` (note `black/60`, not `/55`); panel `"w-full max-w-[860px] max-h-[88vh] rounded-xl border shadow-2xl ui-modal-panel-enter flex flex-col overflow-hidden"`. Note strip `"rounded-md border px-3 py-2 text-xs"` with `background: var(--surface-1)`. Table grid `"grid grid-cols-[170px_minmax(0,1fr)_minmax(0,1fr)]"`; header row `"border-b text-[11px] font-semibold uppercase tracking-wide"`; cells `"px-2.5 py-2"` with hard `borderRight: '1px solid var(--border-subtle)'`; "Update" cell colour `var(--accent-secondary)`; rows separated with `divide-y divide-[var(--border-subtle)]`.
- **States**: opens only when `selectedPrinterUpdate` exists (an effect closes it when the update disappears). Empty diff → `No profile field-level changes were detected, but the official version marker will still advance to v<latest>.`
- **Diff algorithm** (reproduce): build a "next" object from the matching `PrinterPreset` (see `:1383-1410`) and a "current" object from the profile, then recursively walk both, sorting keys, printing leaf paths with `toFieldLabel` (`camelCase` split, `_`/`-` → space, first char upper-cased, segments joined by ` › `); values via `toDisplayString` (`null` → `—`; boolean → `Enabled`/`Disabled`; number → its string or `—`; trimmed string or `—`; objects → `JSON.stringify`); rows are emitted only when the two display strings differ. `networkFilter`/`platformBadge`/`pixelSize`/`bitDepth`/`buildVolumeMm`/`display.*` therefore appear as nested rows such as `Display › Resolution X`.
- **Data**: `selectedPrinterPreset`, `selectedPrinterUpdate` (`{printerProfileId, currentVersion, latestVersion, …}`), `selectedPrinterUpdateDiffItems`.
- **Callbacks**: `Cancel`/X/backdrop → close; `Apply Update` → `handleApplySelectedPrinterOfficialUpdate()` (`applyOfficialPrinterProfileUpdate(id)`) then close.
- **Text**: `Official Printer Update`; `{name} • v{cur} → v{latest}`; `Review the incoming changes before applying this official profile update.`; `Field`; `Current`; `Update`; `No profile field-level changes were detected, but the official version marker will still advance to v{latest}.`; `Cancel`; `Apply Update`; `Close update preview`.
- **Port target**: `gpui/src/features/profiles/official_update.rs`.

---

## Surface: structured dialogs (five)

All use `StructuredDialogModal`; all are portalled to `document.body` and each registers its own Escape handler.

### 1. Official printer profile locked
- **Source**: `:6572-6618`. `zIndexClassName="z-[75]"`, `ariaLabel="Official profile locked"`.
- **Text**: title `Official Profile Locked`; subtitle `Official slicer profiles can't be edited directly.`; icon `Lock h-4 w-4`, tone warning; body `For safety reasons, official slicer profiles cannot be modified directly.` / `Choose Make Custom Copy to duplicate and edit safely.` / `Warning: Custom, non-official profiles may increase print-failure risk and can potentially damage the machine or cause personal injury.`; actions `Cancel` and `Make Custom Copy` (`Lock` icon); `closeAriaLabel="Close official profile lock dialog"`.
- **Callbacks**: `Make Custom Copy` → `handleDuplicateOfficialProfile()` (duplicate `officialLockedProfileId` as custom, pick it, close dialog, `setIsEditingPrinter(true)`).
- **Note**: nothing in the rendered JSX calls `showOfficialProfileDialog` — the open path is defined (`:3474`) but not wired to any visible control.

### 2. Material profile locked (official or user-locked)
- **Source**: `:6620-6683`. `zIndexClassName="z-[75]"`, `ariaLabel="Material profile locked"`, open = `showOfficialMaterialLockDialog && Boolean(selectedMaterial)`.
- **Text**: title `Official Profile Locked` when `materialLockDialogIsOfficial`, else `Profile Locked`; subtitles `Official material profiles can't be edited directly.` / `This material profile is locked to prevent accidental edits.`; body (official) `Official material profiles cannot be edited directly.` / `Choose Make Custom Copy to duplicate and adjust exposure settings safely.` / `Warning: Custom exposure settings may affect print quality and could damage the machine or cause personal injury.`; body (user lock) `This material profile is locked, so its settings are read-only.` / `Choose Unlock to edit it directly, or Make Custom Copy to duplicate it as a new editable profile.`; actions `Cancel`, `Unlock` (only when not official), `Make Custom Copy`.
- **Callbacks**: `Unlock` → `handleToggleMaterialLock()` + close; `Make Custom Copy` → `handleDuplicateMaterialAsCustom()` (duplicate with `Official …` cleared, `locked: false`, select it, close dialog, open the editor).

### 3. Tank mismatch
- **Source**: `:6685-6771`. `zIndexClassName="z-[100]"`, `maxWidthClassName="max-w-xl"`, open when `tankMismatchMaterials && tankMismatchShapes`.
- **Text**: title `Tank material settings do not match` (`<Trans>`); subtitle `<left full name> · <right full name>`; icon `OctagonAlert`; body intro `Layer height, bottom layer count and transition layers must match across tanks. Choose which material supplies all three.`; table headers `Setting`, then both full material names; row labels `Layer height (mm)`, `Bottom layer count`, `Transition layers`; locked footnote `Locked materials cannot be changed. Unlock or make a custom copy before applying settings.`
- **Layout & styling**: table `"w-full border-collapse text-left text-xs"` inside `"overflow-hidden rounded-md border"`; `thead` `background: var(--surface-1)`, muted; cells `p-2`; each value row gets `tankMismatchRowStyle(left, right)` → `{ borderColor: 'var(--border-subtle)', color: tankMaterialValueMatches(left, right) ? 'var(--success)' : 'var(--danger)' }`.
- **Callbacks**: two actions labelled `Use <full material name>` (`tankMaterialSettingsMessage`); left action copies the **left** material's layer stack onto the right, right action copies the right onto the left; each is disabled when its **target** material is locked (`materialIsLocked` = `locked === true || officialTemplateId` non-empty) with title `This material is locked. Unlock it or make a custom copy before applying settings.` `applyTankMaterialSettings(source, target)` writes `layerHeightMm`, `bottomLayerCount` and the transition-layer key (first present of `traLayers`/`transitionLayers`/`transitionLayerCount` in the target's settings, else in the source's, else `transitionLayerCount`) into the target's `localSettingsByOutput[outputFormat]`, then clears the mismatch pair.
- **Full material name** (`fullMaterialName`, `:148-160`): trim name/brand; look up the resin-family label; if the name does not already start with the family label, prefix it (`"Standard Blue"`); if the brand is non-empty and the result does not already start with it, prefix the brand too.

### 4. Unassigned tanks
- **Source**: `:6773-6805`. `zIndexClassName="z-[100]"`, `maxWidthClassName="max-w-md"`, `ariaLabel="Unassigned tanks"`.
- **Text**: title `Tanks are unassigned`; subtitle = printer name; body `No tank has a material assigned yet, so a Multi-Tank print would not know which resin it is holding. Assign at least one tank, or close anyway.`; actions `Keep editing` and `Close anyway` (`variant="tinted-danger"`).
- **Callbacks**: `Close anyway` clears the flag and calls `onClose()` directly, bypassing `requestClose`.

### 5. Delete confirmation
- **Source**: `:6807-6850`. `zIndexClassName="z-[76]"`, `ariaLabel="Confirm delete"`.
- **Text**: title `Delete Printer Profile` / `Delete Material Profile`; subtitle `This action cannot be undone.`; body (printer) `Delete <strong>name</strong> and remove all material profiles linked to it?`; body (material) `Delete material profile <strong>name</strong>?`; actions `Cancel` and `Delete` (`Trash2`, `tinted-danger`).
- **Callbacks**: `handleConfirmDelete` (`:3448-3468`) → printer: `removePrinterProfile(id)`, and if the store now has zero profiles also `onClose()` (comment: *"Removing the last printer profile should surface the first-run onboarding (shown whenever there are no printer profiles) — close the modal rather than leaving its empty "Welcome" state open."*); material: `removeMaterialProfile(id)`.

---

## Surface: Material Library dialog (+ explainer)

- **Source**: `:5968-6327`; explainer portal `:6268-6322`; row factory `:6090-6180`.
- **Renders**: `z-[65]`, `max-w-[1040px]`, same geometry as the Printer Library. Header `FlaskConical` tile, title `Material Library for <manufacturer> <name>`, subtitle, `Info` and `X` buttons. Body = 220px sidebar (search, a collapsible brand node with layer-height children) + a preset list grouped by resin family; footer with count + Add button.
- **Layout & styling**:
  - Sidebar brand node is a disclosure button: `"w-full flex items-center justify-between gap-2 rounded-md border px-2 py-1.5 text-left text-sm font-semibold transition-colors"`; expanded `{ borderColor: 'transparent', background: 'transparent', color: 'var(--accent-secondary)' }`, collapsed `{ borderColor: 'var(--border-subtle)', background: 'var(--surface-1)', color: 'var(--text-muted)' }`; chevron `ChevronDown w-3 h-3` rotated `-rotate-90` when collapsed; count pill `"shrink-0 text-[11px] font-bold tabular-nums px-1 py-0.5 rounded"` with `background: var(--surface-2)`.
  - Layer-height children sit in `"ml-4 pl-4 border-l space-y-1"` whose border is `var(--accent)` when a height is selected, else `var(--border-subtle)`; each is `"w-full flex items-center justify-between gap-2 rounded-md border px-2 py-1 text-left text-xs font-medium transition-colors"`, label `` `${Math.round(h * 1000)}μm` ``, trailing count `"text-[11px] tabular-nums"`; active `{ borderColor: 'transparent', background: 'transparent', color: 'var(--accent)' }`, inactive `{ borderColor: 'var(--border-subtle)', background: 'var(--surface-0)', color: 'var(--text-muted)' }`.
  - Preset rows: `"w-full rounded-md border text-left transition-colors overflow-hidden disabled:opacity-60"`; colours keyed off the resin family colour `RESIN_FAMILY_COLOR[resinFamily] ?? '#94a3b8'`: added → `borderColor: color-mix(…, var(--border-subtle) 30%)`, `background: color-mix(…, var(--surface-1) 88%)`; selected → `borderColor: familyColor`, `background: color-mix(…, var(--surface-1) 82%)`; default → `borderColor: color-mix(…, transparent 62%)`, `background: color-mix(…, var(--surface-1) 92%)`. Row inner `"flex-1 min-w-0 px-2.5 py-1.5"`; name `"text-sm font-semibold truncate"` with the `Siraya Tech - ` prefix stripped by regex `/^Siraya Tech\s*-\s*/i`; family chip `"text-[11px] rounded-full border px-1.5 py-0.5 font-medium whitespace-nowrap"`; selected check is a filled circle with `var(--accent)` background and `#0a0f0a` glyph; `Added` chip `"text-[11px] rounded-full border px-1.5 py-0.5 font-semibold whitespace-nowrap"`.
  - Group header: `"flex items-center gap-2 mb-1.5 px-0.5"` with a `w-1.5 h-1.5` dot and an `"text-[11px] font-bold uppercase tracking-widest"` label, both in the family colour.
  - Empty state: `Frown w-16 h-16` (`strokeWidth={1.5}`) + `"text-sm font-semibold"` title + `"mt-1 text-xs leading-snug max-w-[340px]"` body.
  - Explainer portal: `createPortal` to `document.body`, `"fixed inset-0 z-[220] flex items-center justify-center bg-black/55 backdrop-blur-sm px-3"`, `role="dialog" aria-modal="true" aria-label="Material presets disclaimer"`; card `"w-full max-w-lg overflow-hidden rounded-xl border shadow-2xl"` with `background: var(--surface-0)` and `boxShadow: '0 24px 46px rgba(0,0,0,0.42)'`; `IconChip size="lg"` with accent tint; two-button row `"grid grid-cols-2 gap-2 pt-1"`.
- **States**: open when `showMaterialPresetPicker`. Brand filter: the sidebar only ever exposes `materialPresetBrands[0]` (see gotchas). Layer-height filter is applied only when not searching. Searching switches to a flat `"grid grid-cols-1 lg:grid-cols-2 gap-1.5"` list. A row is disabled when its `templateId` is already present in `addedOfficialMaterialTemplateIds` (official templates only). Empty preset list → the `No presets here yet` block. The explainer opens automatically the first time the picker opens unless `localStorage['dragonfruit.materialLibraryExplainerDismissed'] === 'true'`; `Do not show again` is a toggle (writes nothing until `Got it` is pressed).
- **Data**: `availableMaterialPresets` (`getRuntimeMaterialPresets(officialPresetId)`), `materialPresetSearch`, `selectedMaterialPresetBrand`, `selectedMaterialPresetLayerHeight`, `isMaterialBrandExpanded`, `selectedLibraryMaterialKeys`, `showMaterialLibraryExplainer`, `dontShowMaterialLibraryExplainerAgain`.
- **Callbacks**: `Info` → reopen explainer; row click → toggle the key (`templateId || `${brand}::${name}``); `Add Materials` → `handleAddSelectedMaterialPresets()` (adds each selected preset via `addMaterialProfile` with the modal's defaults, selects the last, closes, clears); brand toggle → expand/collapse; layer-height row → `setSelectedMaterialPresetLayerHeight(h)`; explainer `Got it` → optionally persist the dismissal key, close; backdrop/X/close → close.
- **Text**: `Material Library` + ` for {manufacturer} {name}`; `Choose an official material preset to add.`; `Search materials`; `Added`; `No presets here yet`; `We searched high and low, but there are no material presets for {printer name} yet. We're working on adding some soon.`; `{n} material{s} selected` / `Select materials to add`; `Add {n} Material{s}` / `Add Materials`; `About material presets`; `Close material library`; `Heads up about presets`; `Good baselines, still calibrate.`; `All material presets are good baselines and generally tested on hardware, but we still recommend individual calibration for the best results. Exposure, lift and temperature can vary by machine, resin batch and environment.`; `Do not show again`; `Got it`; `Material presets disclaimer`.
- **Port target**: `gpui/src/features/profiles/material_library.rs`.

---

## Surface: Edit Unit dialog (fleet device nickname/thumbnail)

- **Source**: `:5136-5253`.
- **Renders**: `z-[72]`, `max-w-[760px]`; header `Edit Unit` + subtitle; body two cards side by side (`grid-cols-1 md:grid-cols-[minmax(0,1fr)_340px] gap-3`): "Unit Identity" (Nickname, read-only IP, reset note) and "Card Thumbnail" (220px preview, Upload Image / Clear); footer Reset Unit ← → Cancel + Save Unit.
- **Layout & styling**: backdrop `"fixed inset-0 z-[72] flex items-center justify-center bg-black/55 p-4 ui-modal-backdrop-enter"`; panel `"w-full max-w-[760px] rounded-xl border shadow-2xl overflow-hidden ui-modal-panel-enter"`. Cards `"rounded-md border p-2.5"` with `background: 'color-mix(in srgb, var(--surface-1), transparent 5%)'`; card titles `"ui-meta font-semibold uppercase tracking-wide mb-2"`. Note strip `"mt-2 rounded-md border px-2 py-1.5 text-xs"` with `background: 'color-mix(in srgb, var(--surface-2), transparent 6%)'`. Thumbnail well `"h-[220px] w-full rounded-md border overflow-hidden flex items-center justify-center"` with `background: printerImageWellBackground`; preview uses `AutoTrimmedImage` `object-cover`.
- **States**: `editingFleetUnit` must exist. Nickname field editable; IP field `disabled`. Clear button disabled without an image (`color: var(--danger)` when present, muted otherwise).
- **Data**: `editingFleetUnitId`, `editingFleetUnitNickname`, `editingFleetUnitImageDataUrl`, `editingFleetUnit`, hidden `fleetUnitImageUploadInputRef`.
- **Callbacks**: open via `handleOpenEditFleetUnitModal` (target = active device or first fleet device). `Upload Image` → click hidden input → `handleFleetUnitImageUploadChange` → `normalizeUploadedPrinterImageDataUrl`; `Clear` → null; `Reset Unit` → `handleResetFleetUnitDraft` (nickname back to `hostName || ipAddress || 'Printer'`, image null); `Save Unit` → `handleSaveFleetUnitEdits` (writes `upsertPrinterNetworkDevice(id, { id, ipAddress, displayName: nickname.trim() || hostName || ipAddress || 'Printer', imageDataUrl: draft?.trim() ?? '' })`, closes).
- **Text**: `Edit Unit`; `Customize nickname and card thumbnail for this fleet unit in DragonFruit.`; `Unit Identity`; `Nickname`; `IP Address`; `Reset will clear custom nickname + thumbnail and fall back to the device hostname/IP.`; `Card Thumbnail`; `No custom image`; `Upload Image`; `Clear`; `Reset Unit`; `Cancel`; `Save Unit`; `Close Edit Unit`.
- **Port target**: `gpui/src/features/profiles/fleet_unit_editor.rs`.

---

## `AutoTrimmedImage` and image normalisation

- **Source**: `:7096-7493` (cache helpers `:7101-7150`, trim maths `:7160-7250`, `normalizeUploadedPrinterImageDataUrl` `:7250-7298`, component `:7300-7493`).
- **Renders**: a `relative h-full w-full min-h-0 overflow-hidden` box with a spinner overlay while decoding, then an absolutely positioned `<img>`.
- **Layout & styling**: overlay `"absolute inset-0 z-[1] flex items-center justify-center"` with `background: 'color-mix(in srgb, #151923, transparent 32%)'` and `Spinner size="md"` in `var(--accent-secondary)`; image `"absolute inset-0 … transition-opacity duration-150 opacity-100"` plus the caller's `className`.
- **Trim algorithm** (must be ported exactly; it is the visual contract for preset thumbnails):
  1. Candidate sources: the trimmed `src`; if it starts with `/api/profile-assets/`, add the path with that prefix stripped; if it starts with `/plugins/`, add `/api/profile-assets/<path after the leading slash>`. Dedupe, keep order.
  2. Return a cached result if `src` or the candidate is in the memory cache.
  3. Decode; if width/height are 0 or no 2D context → cache `src→candidate` and `candidate→candidate`, show the candidate.
  4. `resolveUniformBackgroundColor`: read the four corners; abort (no background) if any corner alpha `< 220`; abort unless every corner is within `24` per-channel delta of corner 0; else average the corners.
  5. A pixel is background when `alpha <= 8`, or when a background colour exists, `alpha >= 220` and `max(|Δr|,|Δg|,|Δb|) <= 20`.
  6. Bounding box of non-background pixels; if empty, fall back to the candidate.
  7. Padding `pad = max(2, round(max(trimmedW, trimmedH) * 0.04))`, clamped to the image.
  8. If the padded box covers `>= 99%` of both dimensions, no trim is needed → candidate.
  9. Otherwise draw the sub-rectangle onto a canvas and `toDataURL('image/png')`; cache under both `src` and `candidate`.
  10. After all candidates fail, cache `src→src` and show `src`.
- **Cache**: in-memory `Map` capped at 48 entries (FIFO eviction via `keys().next()`), persisted to `localStorage['dragonfruit.trimmedImageCache.v1']` as an array of `[key, value]` pairs, **only** for non-`data:` sources whose trimmed value is `<= 350_000` characters, last 48 entries. Hydration happens once, lazily, on first use; corrupted payloads are ignored.
- **`normalizeUploadedPrinterImageDataUrl`**: same corner/background/bbox/pad logic (thresholds identical), returns a PNG data URL; on any failure returns the input unchanged. Used for both the printer image upload and the fleet-unit thumbnail.
- **Port target**: `gpui/src/features/profiles/trimmed_image.rs` — cache + trim as pure functions over decoded RGBA, with a small `gpui` view wrapper.

---

## Deliberate decisions and gotchas

1. **Interpolated Lingui messages are module-scope functions, never inline.** `tankMaterialSettingsMessage`, `materialOwnedByMessage`, `materialSharedWithMessage`, `missingPluginMessage`, and the six formatters in `PrinterVariantPickerModal` (`formatDetectedVariantLabel`, `formatScanningLabel`, `formatDevicesFoundLabel`, `formatUnmatchedBitDepthError`, `formatConnectError`, `formatNoDevicesFoundMessage`) exist because of the React Compiler, and the source says so twice: *"Module scope so React Compiler does not rename the interpolated material name."* and *"Interpolated messages MUST stay at module scope (see AGENTS.md): React Compiler renames interpolated locals inside component bodies, desyncing Lingui message ids in production builds."* The port has no compiler, but the **msgid strings** (e.g. `Use {materialName}`, `Detected: {variantName}`, `Couldn't connect to {host}.`) are the catalog keys and must be kept verbatim, including the translator `comment:` text reproduced above.

2. **`tankMaterialSettingsMessage` msgid is `Use {materialName}`**, so the two mismatch buttons read `Use Standard Blue` / `Use Standard Grey`. The tooltips on the tank labels are the *only* explanation of the mismatch glyph's meaning; the mismatching glyph is a real button while the matching glyph is a `role="img"` span. Do not make both buttons.

3. **`unresolvedTankMismatch`'s comment is wrong and the code is right.** The comment says *"The first adjacent assigned pair whose layer stacks disagree, in tank order."*, but the implementation is a full nested loop over all assigned pairs (`for leftIndex … for rightIndex from leftIndex+1`). The first *pair*, not the first *adjacent* pair, wins. Port the code, not the comment.

4. **Tank disagreement is about three numbers only.** `tankMaterialsDisagree` compares `layerHeightMm`, `bottomLayerCount` and the transition-layer count. The transition key is resolved by trying `traLayers`, `transitionLayers`, `transitionLayerCount` **in that order**, first in the format-keyed settings bucket (`.lys` *and* `lys` are both probed), then in the other material's bucket; the default write key is `transitionLayerCount`. `tankMaterialValueMatches` uses `abs(diff) < 1e-6`.

5. **`applyTankMaterialSettings` writes only those three values** — it does not copy exposure, lift or any AA settings, and it does not copy `localSettingsByOutput` wholesale; it merges one key into the target's existing bucket.

6. **Closing is blocked by an unresolved mismatch, not just warned.** `requestClose` re-opens the mismatch dialog and returns. The unassigned-tanks dialog is *not* a block: `Close anyway` calls `onClose()` directly.

7. **Two different "unassigned" thresholds.** The dialog only appears when **zero** tanks are assigned; the mismatch check runs from **two** assigned tanks upward.

8. **`hasPrinters` changes the whole panel geometry.** With printers: `h-full` + scrolling body. Without: fixed `700px` card, `max-h-[94vh]`, non-scrolling body. The empty state itself is `flex-1 h-full min-h-0`.

9. **Deleting the last printer closes the modal** so first-run onboarding can appear. Deleting a material never does.

10. **Material rows act differently in Multi-Tank.** Comment: *"In Multi-Tank the row is picked for the Edit/Duplicate buttons, not as the job's material: the tanks own that."* Concretely: `onClick` always sets `selectedMaterialId` but calls `setActiveMaterialProfile` **only when `!multiTankMaterialPicker`** (both on click and double-click).

11. **Material deletion is blocked in two ways**: `printerMaterials.length <= 1` (never delete the last material) and `selectedMaterialIsShared` (`material.printerProfileId !== selectedPrinter.id`). The disabled tooltip in that case is `Owned by <owner name>`.

12. **Material "lock" has two independent sources**: `officialTemplateId` (non-empty ⇒ official ⇒ always locked, cannot be unlocked) and `locked === true` (user toggle, only for non-official). `materialIsLocked` in the modal is `locked === true || Boolean(officialTemplateId?.trim())`. The Lock/Unlock button is hidden for official profiles; the lock dialog offers Unlock only for user locks.

13. **The material editor seeds `color` explicitly, and the comment explains why:** *"The colour is part of the material meta: without it here, an edit that changed it saved nothing and the profile reopened on the old value."* Keep `color: selectedMaterial.color` in the draft seed.

14. **Duplicating a material names it `<name> Custom` unless the name already contains `Custom`** (`selectedMaterial.name.includes('Custom')` — case-sensitive, anywhere in the string). The duplicate clears `officialTemplateId`/`officialTemplateVersion` and sets `locked: false`.

15. **`mergeWithLocalSettingsDefaults` lowercases the format keys and fills in adapter defaults for missing fields**; `resolveDefaultLocalSettingsForOutput` returns `{}` when no adapter exists and otherwise `{ [output.toLowerCase()]: { …field defaults } }`.

16. **Two spellings of the format key are accepted** everywhere (`.lys` and `lys`): `applyTankMaterialSettings`, `tankMaterialCompatibility.transitionLayersFor`, and the default-settings merge all probe both.

17. **`supportsMultiTankFormat` depends on the sideloaded-format snapshot, not just the string.** The code forces the dependency with `void sideloadedFormats;` because *"The plugin format registry is loaded after the profile store on a cold start, so this has to recompute when that snapshot arrives instead of pinning the answer to the extension string alone."* Same trick in `selectedFormatVersionOptions`.

18. **`multiTankMaterialPicker` needs both conditions**: the format supports multi-tank **and** `materialMode === 'multiTank'`. Switching `materialMode` to `multiTank` also seeds `tankCount: selectedPrinter.tankCount ?? 2`; switching to `single` does not clear `tankMaterials`.

19. **Auto build dimensions are `round(px) * max(0.001, pixel)` / 1000, rounded to 3 decimals** (`computeBuildDimensionMm`). Only **width and depth** are auto-derived; height is always manual. Switching auto → manual restores the remembered `{width, depth}` from `manualBuildDimensionsByPrinterId` when present, otherwise only flips the mode (leaving the auto-computed values in place).

20. **`handlePrinterDisplayChange` always re-resolves both `formatVersion` and `settingsMode`** after merging the partial display, because changing `outputFormat` invalidates both. It also recomputes width/depth in auto mode using the *new* resolution.

21. **Pixel size is clamped to `>= 0.001`; bit depth to `max(1, round(v))`; resolution to `max(1, round(v))`** at the handler level (the number inputs also clamp, but the handlers are the contract).

22. **The dither bit depth and the panel bit depth are two different numbers.** `printerDitherBitDepth` = `null` for ≥8-bit panels, otherwise `clamp(round(bits), 2, 7)`; `printerPanelBitDepth` = the raw rounded declared depth (or `null`). Comment: *"Raw declared depth: lets the AA section tell an 8-bit-or-deeper panel (where dithering is forced off) apart from a printer that declares nothing."*

23. **Replacement-mode tabs are derived, not authored**: plugin tab declarations sorted by `order` (defaults `(index+1)*10`), then two synthetic tabs appended — `{ id: 'meta', title: 'Meta', order: 1000 }` and `{ id: 'anti-aliasing', title: 'Anti-Aliasing', order: 1010 }`. Default tab = the first plugin tab; the modal label is the output format uppercased with a leading `.` stripped (`'.lys'` → `LYS`).

24. **The last-used material-editor tab is persisted per printer** in `localStorage['dragonfruit.materialSettings.activeTabByPrinterProfile.v1']`, validated against the current tab list, and an explicit "open Anti-Aliasing" request (`openMaterialAntiAliasingToken`) wins over it via `materialEditorInitialTabOverrideRef`. That request opens the lock dialog instead when the material is official or locked.

25. **The token plumbing is one-shot.** `openPrinterLibraryToken` / `openNetworkSettingsToken` / `openMaterialAntiAliasingToken` are compared against `lastHandled*Ref` values so re-renders cannot reopen a dialog; they are only honoured on the open transition (`wasOpenRef`) and only for the matching tab (`initialTab === 'printer'` / `'material'`). Opening the printer library and network settings in the same request: network wins (`setShowPresetPicker(shouldOpenPrinterLibrary && !shouldOpenNetworkSettings)`).

26. **The active printer can change outside the modal** and the modal follows it: an effect on `profileState.activePrinterProfileId` re-selects the printer and restores its material (`activeMaterialProfileIdByPrinterId` → global `activeMaterialProfileId` → first), guarded by `lastSyncedActivePrinterRef`. Comment: *"The active printer can change outside this modal (for example "Keep" on the scene-material prompt). Keep the local selection following the store."*

27. **Material auto-selection is stateful across renders.** `materialSelectionInitializedRef` distinguishes "first selection after open/printer switch" (restore remembered material, and do **not** write it back to the store) from later reconciliations (fall back to the first filtered material and `setActiveMaterialProfile` it).

28. **Three-column material browser filters compose**: manufacturer list = distinct `brand || 'Default'` of **this printer's** materials; resin types = families present under the selected manufacturer; profiles = both filters. Selecting a manufacturer resets the family *and* the profile id; selecting a family resets the profile id. The `selectedMaterial` memo falls back to the first filtered profile whenever the remembered id is not in the filtered set.

29. **`materialSharingPrinters` filters to the same format and settings mode**: `printer.display.outputFormat === owner.display.outputFormat && (settingsMode ?? '') === (owner.settingsMode ?? '')`, plus the owner itself. This list becomes both the "Compatible Machines" checkbox set and the `shareablePrinterIds` passed to the form sections.

30. **Official field highlighting is per-field, and its basis is the preset, not the store defaults.** `officialFieldDefaults` comes from the matching `PrinterPreset`; `formatVersion`/`settingsMode` are compared through their **resolved** values with the first available option as the default, because an absent value means "first option". Fields without a highlight prop (`Webcam Rotation`, `Format Version`, `Settings Mode`) use either `highlight` (LabeledSelectInput) or `MODIFIED_FIELD_STYLE` (SelectDropdown) — two different mechanisms for the same idea.

31. **`LabeledSelectInput` vs `SelectDropdown`**: Output Format, Network Support, Webcam Rotation and Material Mode use `LabeledSelectInput` (which takes `defaultValue`/`highlight`); Format Version and Settings Mode use `SelectDropdown` directly (so they can put a `DefaultOptionChip` in `rightContent`).

32. **`WEBCAM_ROTATION_OPTIONS` values are strings** (`'0' | '90' | '180' | '270'` with degree labels) and the handler coerces anything else to `0`. The profile field is `0 | 90 | 180 | 270`.

33. **`selectedPrinterNetworkModeOptions` appends an `Unknown (<mode>)` option** when the profile's mode is not registered, so a profile from an uninstalled plugin still renders. Comment on the memo it wraps: *"A package installed in this session registers its mode after this modal mounted; the profile snapshot moves with the registry, so it is the dep that makes the list current. An empty dep list pinned the list at mount and showed "Unknown (<mode>)" for a mode that was registered a second later."*

34. **Choosing `None (Local only)` hides the network UI immediately**: the handler lowercases the value, stores `undefined` for empty, and calls `setIsNetworkSettingsOpen(false)` + `setIsAddingNetworkPrinter(false)`. Note `PrinterProfile.networkSupportDisabled` exists in the store because an absent `networkSupport` means "inherit the preset" — the modal does not set that flag.

35. **Light theme gets hand-tuned green accent constants.** `accentSecondaryActionColor/BorderColor/Background92/Background93` are computed from `isLightTheme` and mix `#4f8a08`/`#6aa20d` with tokens; dark theme uses `var(--accent-secondary)`. The two derived style objects (`accentSecondaryActionStyle92/93`) are the "secondary/positive action" look reused by ~15 buttons. `isLightTheme` reads `data-theme` on `<html>`, falls back to `prefers-color-scheme`, and watches both a `MutationObserver` (`attributeFilter: ['data-theme']`) and the media query.

36. **`printerImageWellBackground` is a literal `#1c2027` in dark mode** and a white-tinted `--surface-2` mix in light mode. The library card image well is a *different* literal: `#2b3039`. Preset images are `object-contain`; fleet-unit images are `object-cover`. Profile cards use `AutoTrimmedImage`; fleet cards use a plain `<img>` (`useTrimmedImage: false`).

37. **The reorder buttons and the drag-and-drop use the same store call** but different anchors: arrows call `movePrinterProfile(id, neighbourId)` (before the neighbour, or before the next-next item when moving right), drag-drop calls `movePrinterProfile(draggedId, hoveredId)`, and dropping on empty grid space moves to the end.

38. **Rail height is capped at 8 entries** (`printerRailEntryCount > 8` → `max-h-[392px] overflow-y-auto pr-1`); fleet mode counts fleet devices, profile mode counts profiles.

39. **`renderPresetLibraryCard`'s 8-bit branch is unreachable.** `bitDepthLabel` is `null` for 8-bit (`bitDepthBits != null && bitDepthBits !== 8 ? … : null`), yet the badge style still branches on `bitDepthBits === 8`, and dead `title` fallbacks remain. The rail card has the same 8-bit exclusion; only the variant picker styles 8-bit (green). Do not "fix" the exclusion: 8-bit cards intentionally carry no depth badge.

40. **`renderPrinterRailCard` renders the image inside a `<button>` and the title row inside a *second* `<button>`**, both wired to the same `onClick`; `onDoubleClick` sits on the outer `div`. `muted` cards pass `undefined` handlers rather than `disabled`, so they stay inert but keyboard-reachable (`aria-disabled`).

41. **The Material Library sidebar shows only `materialPresetBrands[0]`.** The brand node label falls back to the literal `Siraya Tech` when the list is empty, and both the brand count and the layer-height counts are computed against `materialPresetBrands[0]` rather than `selectedMaterialPresetBrand`. Combined with the layer-height filter's `selectedMaterialPresetBrand` check, a profile with multiple brands can only browse the first brand's presets. Port the current behaviour or fix it deliberately — do not silently "improve" it, because the auto-selected brand effect (`selectedMaterialPresetBrand ?? brands[0]`) assumes this.

42. **Material library order comes from the family grouping**: `RESIN_FAMILY_OPTIONS` label lookup, falling back to `'Other'`, groups sorted by label with `localeCompare` (not by the option order).

43. **The printer library groups by `family`, preserving manifest order** — `familyOrder` records first appearance, and the sort uses that index (defaulting to `999`), so it is *not* alphabetical. Empty family → `'Other'`.

44. **`addedOfficialPresetIds` is derived from profiles, not from presets**: for every official profile it takes `profile.officialPresetId` (trimmed) or, failing that, strips the `printer-default-` prefix from the id (`resolveOfficialPresetIdFromProfile`, `:332-340`). A family preset counts as "added" only when **every** variant id is present; family selection is "any variant selected".

45. **The Initial Setup modal is opened for two different reasons**: a preset with `modelVariants`, or a preset with `modelVariantDetectPath` (network-detectable, e.g. Athena). Comment: *"Variant families AND network-detect presets (e.g. Athena) open the Initial Setup flow."* In the second case `variants` may be empty and detection resolves to the preset itself.

46. **Network-resolved variants create a connected profile immediately; manual variants just join the multi-select set.** `handleResolveVariant` (`:3350-3390`): with `networkContext` → `addPrinterProfileFromPreset`, `upsertPrinterNetworkDevice(newId, { ipAddress: host, port, connected: true, mode, hostName: displayName, lastCheckedAt: now, statusText: 'Connected', displayName }, { select: true })` where `displayName = printerName?.trim() || host`, pick it, close everything. Without → toggle the id in `selectedLibraryPresetIds` and close the chooser.

47. **The variant picker deliberately runs two scans concurrently and stops the subnet pass on first hit.** Comments: *"Run the fast local/mDNS pass and the progressive subnet scan concurrently. Athena printers don't advertise mDNS, so the subnet scan is what actually finds them; the local pass adds any Bonjour-advertising devices without delaying the subnet scan."* and *"ARP priority puts the likely printer in the first batch — stop scanning once a matching device is found so discovery completes in a couple of seconds instead of a full subnet sweep."* The 25 s `Configure Networking Later` fallback exists because the subnet sweep can be slow.

48. **The connect response's resolved host/port, not the input, is what gets stored.** `connectAndDetect` prefers `payload.ipAddress`/`payload.port` and only falls back to the requested host/port.

49. **`matchPrinterVariantByBitDepth(variants, bits)` is only consulted when the preset declares variants**; a preset with no variants but a detect path is "detected as itself" and the networking is the point (comment in source).

50. **The debug sentinel host `192.168.999.999` seeds two fake fleet devices** and a fake connection status. It is reachable from the real UI by typing that IP. Keep it if the port is meant to be testable the same way; otherwise remove it deliberately.

51. **The modal keeps two "discovered" lists.** `discoveredPrinters` is the live list; `cachedDiscoveredPrinters` survives printer switches (`setCachedDiscoveredPrinters(discoveredPrinters)` on printer change, then `setDiscoveredPrinters([])`), and the next scan merges `[...discoveredPrinters, ...cachedDiscoveredPrinters]` and re-verifies each cached entry with a `connect` call before keeping it.

52. **Discovery progress phases are strings, not enums, and are shown verbatim** (`Resolving friendly .local hostnames…`, `Scanning local subnet…`, `Verifying previously discovered printers…`, `Scan complete`, `Scan failed`). The percentage is driven by an explicit script (8 → 42/44 → 56 → 56..98 → 100), not by measurement.

53. **`handleRunNetworkDiscovery` logs extensively under `[NetworkSettings][AutoScan][…]`** with `console.info`. Every request logs its `networkFilter`/`modelHint` resolution. Not UI, but the port's network layer will need equivalent diagnostics hooks if parity matters.

54. **`selectedPrinterModelHint` is a fuzzy classifier** over `networkFilter + name + preset name + preset family` (`/\bathena\s*(ii|2)\b/` or `athena2` → `'athena-2'`; `athena` → `'athena'`; else `undefined`) and is passed to every connect/discover call. `selectedPrinterNetworkFilterHint` tries, in order: profile `networkFilter` → preset `networkFilter` → preset matched by resolution+pixel size → by resolution → by name+manufacturer → by name → by name containment → by family containment → `''`.

55. **Remote-material visibility is three-way, not on/off**: `shouldUseRemoteOnDeviceMaterials` (live), `shouldShowRemoteMaterialSelectedPrinterOfflineState` (configured target but unreachable), `shouldShowRemoteMaterialConnectInfo` (adapter supports remote materials but nothing configured). The local three-column browser is only the fallback when none of them applies. The header description mirrors this three-way split.

56. **A remote printer with no host sets an error and clears the list** rather than silently doing nothing: `Connect to a <mode> printer to load on-device materials.`

57. **Remote material selection persists in three places**: `localStorage['dragonfruit.network.remoteMaterialByPrinter.v1'][printerId]`, the store (`updatePrinterNetworkConnectionStatus({ selectedMaterialId, selectedMaterialName, selectedMaterialLayerHeightMm, selectedMaterialNormalExposureSec, selectedMaterialBottomExposureSec, selectedMaterialBottomLayerCount })`), and the in-memory cache entry's `selectedMaterialId`. The initial value is resolved by `persistedRemoteMaterialIdForSelectedPrinter` with a four-step fallback (connection → active fleet device → host-matched fleet device → any device with a selection).

58. **Remote material refresh is background-first.** The effect seeds the cached list, then calls `loadRemoteMaterials({ background: true })`, which does **not** clear a previous error. User-initiated refresh clears it.

59. **Remote edit is gated by a plugin-declared WIP notice** (`remoteMaterialEditingWipNotice`): the Edit button is rendered but made inert with a custom hover card (not a `Tooltip`), and double-click on a row is disabled. `openRemoteMaterialEditDialog` also early-returns when the notice is set.

60. **`RemoteMaterialEditDialog` is defined inside the same file but is a separate component with its own props** — it is not a closure over modal state (`:6852-7093`). Port it as its own view keyed on a props/state struct.

61. **The printer update diff compares only 18 top-level/nested fields**, listed explicitly in `nextComparable`/`currentComparable`; fields absent from that object (e.g. `notes`, `safetyMarginMm`, `tankCount`, `webcamRotationDeg`) are *not* diffed even though the update would overwrite them. `officialPresetVersion` is set to `latestVersion` in "next", so the version bump itself shows up as a row.

62. **`applyOfficialPrinterProfileUpdate` / `applyOfficialMaterialProfileUpdate` take an id, not an object**; the dialog closes immediately after calling them (fire-and-forget, no await, no spinner).

63. **Printer bundle import/export shapes** (these are the on-disk contract):
    - Export printer: `{ version: 1, exportedAt: ISO, printer: PrinterProfile, materials: MaterialProfile[] }`, filename `<safe-name>-bundle.json` where `safe-name = name.replace(/[^a-z0-9-_]+/gi, '_').toLowerCase()` (fallback `printer-profile`), pretty-printed with 2 spaces, saved with `savePrintArtifactWithNativeDialog`.
    - Import printer: pick via `pickOpenFilesWithNativeDialog('bundle', false)`, read bytes, `JSON.parse`, hand the whole payload to `importPrinterBundle(payload)`.
    - Export material: `{ version: 1, exportedAt, material }`, filename `<safe-name>-bundle.json` (fallback `material-profile`).
    - Import material: reads `payload.material`, rejects non-objects with `Error('[ProfileSettingsModal] Invalid material bundle payload')`, then coerces every field with explicit defaults (`currencyCode` upper-cased, `bottomLayerCount` `max(1, round(v))`, `minimumAaAlphaPercent` clamped 0..100, `localSettingsByOutput` falling back to `resolveDefaultLocalSettingsForOutput`).
    - Cancel detection is string-based on every call: if the error message contains `"cancel"` (case-insensitive), it is swallowed.

64. **`FleetManagement` renders an unreachable branch.** The `No printers saved yet. Add one below to start.` line sits inside `{managedPrinters.length > 0 && ( … {managedPrinters.length === 0 ? … } )}`, so it can never show. The modal also passes `showAddPrinterFlow={isAddingNetworkPrinter || managedPrinters.length === 0}`, so the add flow is forced whenever the fleet is empty.

65. **`FleetManagement`'s own count string is duplicated**: the idle discovery status uses raw template interpolation `` `${discoveredPrinters.length} printer${…} found` `` while the "Discovered Printers" heading uses the ICU formatter `{count, plural, one {# found} other {# found}}`. The first is untranslated in practice.

66. **`FleetManagement` external actions are fire-and-forget `void handle…()` promises** — the modal owns scanning/connecting state, so the component never awaits.

67. **`ManualIpEntryCard`/`SetupModeButton` are shared by two call sites** (Initial Setup and Fleet Management) — keep one implementation; their `connectLabel` override exists for reuse but is currently never passed.

68. **Several unused locals survived**: `selectedPrinterNetworkSupportMode` (`:1349`), the `networkDiscoveryEnabled` dependency in `handleRunNetworkDiscovery`'s dep array (the body never reads it), `replacementMaterialEditorDefaultTab` in `handleAddMaterial`'s deps, the FleetManagement wrapper `<div>`'s `onMouseDown` (the component renders its own panel, not a backdrop), and `showOfficialProfileDialog`'s caller. Port the behaviour, drop the dead locals.

69. **Z-index stack (must be preserved as a stack, not a set):** shell `z-50` < editors `z-[60]` < libraries/variant chooser `z-[65]` < network `z-[70]` < remote edit `z-[71]` < edit unit `z-[72]` < update diff `z-[74]` < lock dialogs `z-[75]` < delete `z-[76]` < tank mismatch/unassigned `z-[100]` < compatible machines (inside the form atoms) `z-[120]` < material explainer portal `z-[220]`.

70. **Every second-level modal uses the same backdrop recipe** (`fixed inset-0 z-[N] flex items-center justify-center bg-black/55 p-4 ui-modal-backdrop-enter`, panel `rounded-xl border shadow-2xl … ui-modal-panel-enter`) and closes on backdrop `onMouseDown` only. The one exception is the update diff (`bg-black/60`, `z-[74]`).

71. **Backdrop dismissal is disabled while saving in the remote-material dialog only** — `if (event.target === event.currentTarget && !isSaving) onClose()`.

72. **`Edit Unit`'s thumbnail preview uses `object-cover`**, matching the fleet card it edits — but the printer profile image uses `object-contain` everywhere. Mixing them makes the editor preview look different from the card. Keep the split.

73. **Escape is not enough to dismiss the modal shell** when a mismatch exists: `requestClose` refuses, so Escape appears to "do nothing" on the top layer once the mismatch dialog is closed. That is intended (`requestClose` re-opens the mismatch dialog on the next press).

74. **The modal mounts its hidden file inputs once at the panel level** (`:5095-5113`), so both upload flows need a `ref` target chosen by state (`uploadTargetPrinterId`) rather than per-card inputs.

---

## Open questions

1. **No `gpui/` tree in this worktree.** All port targets above are proposed paths; there is no existing Rust module to check for conflicts or shared atoms (buttons, dialogs, tooltips) that a previous area may already have added. The main agent should reconcile `profiles/*` with whatever `gpui/src/features/` actually contains.

2. **`ProfileStore` actions are not specified here.** `addPrinterProfileFromPreset`, `duplicatePrinterProfileAsCustom`, `importPrinterBundle`, `applyOfficial*Update`, `getOfficial*Updates`, `getLibraryPrinterPresets`, `getPrinterPresetVariants`, `getAvailablePrinterPresets`, `movePrinterProfile`, `addMaterialProfile`, `updateMaterialProfile`, `setMaterialProfileLocked`, `upsertPrinterNetworkDevice`, `selectPrinterNetworkDevice`, `disconnectPrinterNetworkDevice`, `removePrinterNetworkDevice`, `updatePrinterNetworkSettings`, `updatePrinterNetworkConnectionStatus` — their semantics (id generation, sanitising, migration, persistence, cross-profile material sharing side effects) live in `src/features/profiles/profileStore.ts` and belong to that area's spec. This spec depends on their documented behaviour; the modal's correctness (e.g. `tankMaterials` being trimmed to `tankCount`, `networkSupportDisabled`) is partly enforced there.

3. **Plugin-provided data shapes.** `getRuntimeMaterialPresets` returns `MaterialPreset` (`templateId`, `profileVersion`, `localSettingsByOutput`, `antiAliasingSettings`, …) and `getProfileLocalMaterialSettingsAdapter` returns `replacesDefaultMaterialSettings`, `tabs`, `fields[].defaultValue`, `numericFields`. Their full contracts are in `pluginRegistry.ts`/the plugins SDK docs and were only read at the call sites.

4. **Native file dialogs.** The bundle import/export flows assume `pickOpenFilesWithNativeDialog('bundle', false)` filters to bundle JSON and that `savePrintArtifactWithNativeDialog` shows a save dialog. The port has its own dialog policy (`gpui-backend` treats `rfd` commands as `ui`/dropped), so the filter list and default directory need a decision.

5. **Translating the leftover literals.** Many strings in this area are plain JSX text (`Printer & Material Profiles`, `Safety warning:`, `Edit Unit`, `Printer Library`, `Material Settings`, table headers, `Changes are applied immediately.`) while neighbours go through `msg`/`Trans`. The `.po` catalogs contain only the `msg`/`Trans` ones. The port needs one convention; this spec marks which strings are already localised (msgid = the English string shown) and which are not.

6. **`FleetManagement`'s title switch** (`Fleet Management` vs `Network Settings`) is driven by the device count, not by what the user asked for — so "Network Settings" with one device and "Fleet Management" with two. If the GPUI shell exposes a single "Network" entry, this heuristic may need replacing.

7. **Remote material editing is declared "work in progress" by plugins**, so the write path (`materialsEdit` + `denormalizeEditDraftForBackend`) may never be exercised in practice. Whether the port implements it is a product call.

8. **`AutoTrimmedImage`'s cache is a `localStorage` side channel** keyed by URL, capped at 350 000 chars per entry. In a native port the same trim can run offline at asset-load time; whether to keep a persisted cache (and where) is undecided.
