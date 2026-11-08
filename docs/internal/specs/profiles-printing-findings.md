# Profiles + printing — audit findings

Scope: `gpui/src/features/printer_profiles/**`, `gpui/src/features/printer_picker.rs`,
`gpui/src/features/printing_preview.rs`. Specs: `profiles.md`, `printing.md`.

Legend: **correct** = matches the spec; **deliberate** = differs and a
`PORT DEVIATIONS` entry (module doc or inline comment) in the port justifies it;
**wrong** = differs with no justification (fixed here where the fix was
contained); **absent** = the spec has it, the port has nothing (new work — not
attempted); **dead** = rendered but cannot do anything.

Everything below was diffed against the spec text and, where the spec was
ambiguous, against the React source it cites. No build was run (the parent gates
cargo); the harness (`gpui/scripts/scenario.mjs`) ships no profiles/printing
scenario, so nothing here was executed.

---

## `printing_preview.rs` — `PrintingPreviewPane` (`printing.md`)

| Spec surface | Verdict | Why / fix |
|---|---|---|
| Pane split (`absolute right-0 w-1/2`, top bar → bottom, surface-0) | correct | `printing_preview` reproduces the geometry (`RAIL_COLUMN_WIDTH = 56`, `TOPBAR_HEIGHT`, `w(relative(0.5))`). |
| Rail column (`relative z-20 h-full border-r px-0 pt-1.5`, `surface-1` 94%) | correct | `SURFACE_1_94`, `RAIL_PADDING_TOP`. `z-20` has no GPUI counterpart (documented). |
| `LayerSlider` call-site props (`min 1`, `max max(1,total)`, `step 1`, clamped `value`) | correct | Same clamping; `overlays::layer_slider(selected, None, …)`. |
| `crossSectionEnabled` / `onToggleCrossSection` not passed | deliberate | The port passes `state.cross_section` so the rail matches the scene's live cross-section; justified in `printing_preview`'s doc ("reads the cross-section switch from `DragonFruit::cross_section`, so the pane and the scene clip stay in step"). |
| Content column (`p-3 flex flex-col gap-2`) | correct | `COLUMN_PADDING`/`COLUMN_GAP`. |
| Heading `Layer Preview` (uppercase, tracking-wide, msgid) | deliberate | Documented deviation: no letter-spacing in GPUI, literal `"LAYER PREVIEW"`; msgid gap recorded. |
| Progress line `Layer n/total` | correct (format) / deliberate (i18n) | Format matches `formatLayerProgress`; the msgid gap is documented in the same deviation block. |
| Preview box (`rounded-lg border p-2 centred`, `surface-1` 94%, `containerType: size`) | deliberate | Box and border are ported; the aspect box is painted rather than laid out (documented: no `cqw`/`min()`). |
| Aspect box (`width: min(100cqw, 100cqh * ratio)`) | deliberate | `paint_aspect_border` draws the dashed rect at the fitted, centred rect (documented). |
| Empty state (dashed border + `No preview available yet.`) | deliberate | Dashes emitted as subpaths, 3px/3px; square corners (documented). Text exact. |
| GPU scrub preview (`PrintingLayerGpuPreview`) | deliberate (documented) | three.js cannot be hosted; with layers and no PNG the box is empty — stated in `PORT DEVIATIONS`. |
| PNG layer (SVG/`<img>`, decode fade, held URL) | deliberate (documented) | One resolved path drawn `ObjectFit::Contain`, settled (documented). |
| Settled hi-res canvas | deliberate (documented) | Not modelled (documented). |
| Wheel zoom / pointer pan / cursor states | deliberate (documented) | Not ported (documented). |
| Mirror / de-mirror transforms | deliberate (documented) | Not applied (documented). |
| `printingCurrentHeightMm` / `slicingHeightMm` | deliberate (documented) | Pane does not take them (documented). |
| Manager state machine (two layer numbers, 180 ms settle, PNG decode nonce, multi-tank composite) | absent | The pane takes a single `PrintingPreviewState`; the manager lives outside this file and is not ported (spec: "a port that renders the pane without porting the manager's state machine will look right at layer 1 and be wrong everywhere else"). New work — see list. |

No `wrong`/`dead` surfaces found in `printing_preview.rs`. The failing case is
coverage, not incorrectness: `PrintingPreviewState` carries one image path and
no scrub/settle state, so scrub-time behaviour is new work, not a bug here.

Out of scope for this ticket (spec surfaces that live elsewhere):
`PrintingPanel` / `PrintingPanelStack` (`gpui/src/panels/content.rs`) and
`LayerSlider`/tooltip (`gpui/src/chrome/overlays.rs`).

---

## `printer_profiles/mod.rs` — modal shell (`profiles.md`)

| Spec surface | Verdict | Why / fix |
|---|---|---|
| Shell (1120px panel, `Box` accent tile, title, close, scrolling body) | deliberate | `dialog_modal_full` + fixed 700px body; deviations 1–2 in the module doc explain both (shared shell; no window measurement). |
| `hasPrinters` geometry split (`h-full` vs 700px card) | deliberate | Deviation 2. |
| Custom-profile safety banner (`isCustom && !isOfficial`) | correct | Colours resolve to `color-mix(#d97706, border-subtle 30%)` / `(#d97706, surface-1 92%)`; `Safety warning:` bold + verbatim body. |
| Empty state (`Welcome to Printer Profiles`, icon circle, `Add printer`) | correct | Heading/body/button verbatim; `Add printer` is `list::add_printer_button` (which also resets the picker selection, the source's `handleAddPrinter`). |
| Delete confirmation (printer) | correct + **fixed** | Dialog matched; the confirm handler now closes the modal when the last profile goes, per `handleConfirmDelete` (`ProfileSettingsModal.tsx:3448-3468`, gotcha 9: "Deleting the last printer closes the modal"). Was: stayed open on the empty state. |
| Close gating (`requestClose`: unresolved tank mismatch blocks, unassigned tanks warn) | absent | `close()` just closes. Needs the tank-mismatch and unassigned-tanks dialogs first — new work (spec: "Close gating", gotcha 6). |
| Escape unwinding order | deliberate | Module doc documents the one-level-at-a-time grouping; the pane's internal order was fixed (see `materials.rs`). |

---

## `printer_profiles/list.rs` — "3D Printer" rail

| Spec surface | Verdict | Why / fix |
|---|---|---|
| Section shell, header title/subtitle, move-left/right, `Add printer` | correct | Title/subtitle/disabled rules match (`index <= 0`, `index >= len-1`). |
| 5-column grid → wrapping 190px cards | deliberate | Documented (no CSS grid); fits 5 per row at 1120px. |
| Rail height cap (`> 8` entries → `max-h-[392px]`, `pr-1`) | correct | `profiles.len() > 8`. (Fleet entry count does not exist here.) |
| Card active/inactive fills, `p-2`, image well 128px, fallback (`Printer`/`ImagePlus`, `Generic`/`No image`) | correct | Values match; artwork itself documented absent (deviation 4). |
| Badge text (`CUSTOM`, preset platform badge) | correct | `card_badge_text`; `UPDATE` not produced (`hasOfficialUpdate` unported, inline comment; model deviation 2). |
| Platform-badge gradient (`white 14%` / `black 18%`) | correct | `mix(color, WHITE, 0.86)` / `mix(color, BLACK, 0.82)`. |
| Bit-depth corner badge | **wrong → fixed** | `mix()` weights its first argument. The port mixed the chip colour *into* the dark colours (56%/72%) instead of the reverse, and used a 20% white border for 3-bit where the source uses 18%. Now: 3-bit `mix(DANGER, WHITE, 0.82)` border, `mix(DANGER,#111827,0.44)` → `mix(DANGER,#0b1220,0.28)`; otherwise accent-secondary 80% / 48% / 32%. Source: `ProfileSettingsModal.tsx:4224-4232`. |
| Network overlay: connected | **wrong → fixed** | Background was inverted (`mix(#22c55e,#0f172a,0.40)` = 40% green). Now 60% green / 40% dark, matching `color-mix(#22c55e, #0f172a 40%)`. |
| Network overlay: offline (`WifiOff`, red tile) | **wrong → fixed** | Was not rendered at all. Added `has_network_target && !connected` with the source's red border/bg/colour and the `Printer not connected` tooltip; connected now carries `Connected to <host|ip>` too. Source: `:4171-4172`, `:4238-4262`. |
| Card click → `handlePickPrinter` | correct | `model::select`. |
| Card double-click → select + edit | **wrong → fixed** | The old deviation claimed `ClickEvent` carries no click count — false (`materials.rs` and `chrome/overlays.rs` both read `down.click_count`). Handler now reads `click_count >= 2` → `model::open_editor`. Stale deviation removed. |
| Drag-to-reorder (card → before card, grid → end) | **wrong → fixed** | The old deviation claimed this GPUI build has no drag payload — false (`on_drag`/`on_drag_move`/`on_drop` exist; `chrome/overlays.rs` uses them). Added a `PrinterDrag(id)` payload: dropping on a card calls `model::move_profile(dragged, Some(target))`, dropping on the grid calls `move_profile(dragged, None)`. Deviation now only records the empty drag ghost. |
| Card `footer` (fleet count / `Set up network`, missing-plugin delete tile) | absent | Documented in deviation 3 (fleet and updates unported; `missing_plugin` never set). New work. |
| Missing-plugin muted card (`aria-disabled`, grayscale, tooltip) | absent | Same; nothing sets `missing_plugin`. |
| Fleet rail view, fleet cards, empty-fleet block, `Return to Printers`/`Manage Fleet`/`Edit Unit` | absent | Deviation 5 (network panes unported). New work. |
| `Show Fleet` / `Network Settings`, `Update Printer`, `Import`, `Export` band buttons | absent | Deviation 5 names the first two and Import/Export; `Update Printer` follows from the unported update diff. `Edit`/`Remove` present and wired. |

---

## `printer_profiles/materials.rs` — Multi-Tank + Material Settings

| Spec surface | Verdict | Why / fix |
|---|---|---|
| Section shells, gradient band, headers (`FlaskConical`, title, description) | correct | Local description only (remote descriptions need the remote surfaces). |
| Multi-Tank gate (`outputFormatSupportsMultiTank && materialMode === multiTank`) | **wrong → fixed** | Only `materialMode` was checked, so a hand-set `multiTank` showed the pane even though the editor hides Material Mode for the same format (its own deviation 3). `View::multi_tank` now `&& editor::supports_multi_tank(format)`; no shipped format declares `supportsMultiTank`, so with the built-in catalogue the pane stays hidden as in the source (`:680`, gotcha 18). |
| Tank grid `max(4, tankCount ?? 2)` tiles, disabled spares, `Tank N` labels | correct | — |
| Tank menu grouping (brand heading, name sort, family prefix) + `Unassign Tank` | correct | `tank_options` / `tank_option_label`; leading `—` row is deviation 7. |
| Colour swatch per tank option | absent | `select_dropdown` rows carry no icon; the source's 3.5×3.5 swatch is not rendered. New work (widget capability). |
| Inter-tank link / mismatch glyph | deliberate | Inline comment: the glyph leads the label because there is no absolute measurement pass. Tooltips exact (`TANK_MATERIALS_MATCH`/`MISMATCH`). |
| Mismatch glyph opens the resolution dialog | absent | Deviation 6: link stays a warning; the copy action and its `localSettingsByOutput` write are not ported. New work. |
| Unassigned tank (`AlertTriangle`, centred dash) | deliberate | Deviation 7 (`selectedDisplay`/`hideSelectedText` have no equivalent). |
| Empty hint `Add a material profile to assign it to a tank.` | correct | — |
| Three-column browser (Manufacturer / Material Type / Profile), filters, fallbacks | correct | Filter composition and resets match gotcha 28. |
| Manufacturer row selection tone | **wrong → fixed** | Source uses `--accent-secondary` for the manufacturer column and `--accent` for the other two (`:4870-4950`); the port used accent for all. `rail_row` now takes the accent per column. |
| Profile row: crown/link2/lock, swatch, `µm` | correct | `Shared with <names>` / `Owned by <name>` tooltips ported (deviation 11 notes the tooltip mechanism). |
| Row click / double-click (Multi-Tank owns the active material) | correct | `set_active` skipped when multi-tank; double-click opens the editor or the lock dialog. |
| Action bar `Edit` / `Duplicate` / `Lock`/`Unlock` / `Delete` order and guards | correct | Lock hidden for official; delete blocked on none / last / shared. |
| `Delete` colours | **wrong → fixed** | Was a dark-tinted fill with danger text; the source is `variant="secondary"` (`surface-1`/`border-subtle`) with `color: var(--danger)` (`:5117-5126`). |
| Disabled `Delete` tooltip (`Owned by <owner>`) | **wrong → fixed** | Added the `materialOwnedByMessage` tooltip on the shared case, in both the action bar and the editor footer. |
| `Import` / `Export` / `Library` / `Update Material` | absent | Deviations 3–4. New work. |
| Editor dialog (title, subtitle, delete/cancel/save, backdrop, 920px, scroll body) | correct + **fixed** | Title/subtitles/actions match; footer buttons are now `rounded-full` pills with the source's font sizes (`:5478-5520`) instead of 6px rectangles. |
| Create dialog (title, subtitle, Cancel/Save, right-aligned footer) | correct + **fixed** | Same pill fix (`:6404-6418`). |
| Lock dialog (official vs user copy, Unlock only for user locks, Make Custom Copy) | correct | Bodies/actions verbatim. |
| Delete dialog (material) | correct | — |
| Compatible Machines dialog (owner row, incompatible rows, checkbox writes) | correct | Ported from `MaterialProfileIdentitySection`, beyond the spec's summary but consistent with it. |
| Material anti-aliasing + plugin local-settings sections; replacement-mode tabs and `Edit <EXT> Material Profile` title | absent | Deviation 5. New work. |
| `escape()` order | **wrong → fixed** | Order was compat → editor → create → lock → delete; the source unwinds top-most first (delete `z-[76]`, lock `z-[75]`, editor/create `z-[60]`, compat `z-[120]`). Reordered, comment cites the z-stack. |

---

## `printer_profiles/model.rs` — the store this area reads

| Spec surface | Verdict | Why / fix |
|---|---|---|
| Field sets (`PrinterProfile`, `PrinterPreset`, `MaterialProfile`, AA settings, safety margins, display, network) | correct | Source order, `camelCase`, `default`; round-trip tests exist. |
| Persistence (`localStorage`, hydration) | deliberate | Deviation 1. |
| Plugin-contributed presets, `missingPlugin` marking, official update diff | deliberate | Deviation 2. |
| Plugin material adapters (`localSettingsByOutput`) | deliberate | Deviation 3 (raw JSON). |
| Id minting (`printer-custom-<n>` etc.) and two-store split | deliberate | Deviations 4–5. |
| Modal selection follows the store (`lastSyncedActivePrinterRef`) | **wrong → fixed** | Every read re-adopted the picker's active profile, so selecting a custom copy (which the picker does not know) was undone on the next frame — contradicting the module's own deviation 4. `Store::synced_active` now adopts only a *change*, like the source's ref guard (`ProfileSettingsModal.tsx:1216-1230`). |
| Duplicate material naming | **wrong → fixed** | Was `<name> Copy`; source: `<name> Custom` unless the name already contains `Custom` (`:3547-3549`, gotcha 14). Fixed in `duplicate_material`. Keeping `color` on the copy is now documented as a deviation (the source's field list drops it by omission). |
| `removePrinterProfile` bookkeeping | **wrong → fixed** | Cleared tank entries matching the *printer* id (tank entries hold material ids) and never stripped the removed printer from other materials' `compatiblePrinterProfileIds`. Now matches `profileStore.ts:2457-2478`. |
| Tank assignment (`assign_tank`), material removal cleanup, add/duplicate/lock/delete | correct | Matches the store calls the spec enumerates. |

---

## `printer_profiles/editor.rs` — Printer Profile Settings

| Spec surface | Verdict | Why / fix |
|---|---|---|
| Shell (`z-[60]`, 960px, 88vh, backdrop, header + subtitle, four cards, footer) | correct | Backdrop/geometry/footer copy (`Changes are applied immediately.`, `Done`). |
| Official banner + `Create Custom Copy` | correct | Text verbatim; the action duplicates as custom and keeps the editor open (`handleDuplicateSelectedPrinterAsCustom`). |
| Official disabled field set | correct | Name, manufacturer, build-mode toggle, width/depth/height, four margins, resolution, bit depth, pixel size, AA, mirror, image controls; notes/output/network/webcam/format/material mode stay enabled. |
| Identity + image well | deliberate | Upload/trim pipeline unported and the well inert (deviation 2); clear tile wired. |
| Build Volume: auto card (switch, `Auto`/`Manual`, non-destructive note), width/depth/height, four margins | correct | `set_build_dimension_mode` remembers and restores the manual pair (deviations exist only for input mechanics). |
| Display: resolution/bit depth/pixel size (clamped at the handler), AA toggle | correct | `max(1, round)` / `max(0.001, …)`; bit depth keeps the preset description; auto mode recomputes width/depth. |
| Output: format, network support (with `Unknown (<mode>)`), webcam support/rotation, mirror X/Y, material mode + tank count guard | correct | Handler re-resolves version/settings mode and network "None" records `network_support_disabled`. |
| Format Version / Settings Mode hidden | deliberate | Deviation 3: no shipped format declares versions/modes, so the source's `length > 0` guards stay closed. |
| Official `Default` chip / amber modified wash | deliberate | Deviation 5 (shared trigger has no `rightContent`/`selectStyle` seam). |
| Text entry via `text_input` / `number_on_blur` (commit timing, carets) | deliberate | Documented (deviation 1); same commit semantics as `LabeledInput`/`LabeledNumberInput`. |

---

## `printer_picker.rs` — Printer Library + Initial Setup

The header documents that this ports `OnboardingPrinterLibrary` (single-select)
and `PrinterVariantPickerModal`, plus `SetupMethodChooser`'s pieces.

| Spec surface | Verdict | Why / fix |
|---|---|---|
| Library shell: `Printer Library`, `Choose an official printer preset to add.`, 220px sidebar, `Search printers`, brand buttons, family-grouped cards, `Choose a printer to add` / `Add printer` footer | correct | Matches `OnboardingPrinterLibrary.tsx:280-440`; search/filter/group-order rules match `:86-130`. |
| Family rule, card selected state, `Check` chip, platform badge, 8-bit-excluded depth chip, title/manufacturer | correct | Card mixes and the depth chip are exact (`bit_depth_colors`). Selected ring replaced by an accent border (deviation 5). |
| Invented `MATERIAL` list under the library | **wrong → fixed** | Neither source component renders a material list; the port added one (with a height switch). Removed, along with its view fields and accessor. |
| Add closes the flow (`onAdded`) | **wrong → fixed** | Adding a printer left the picker open, so the modal underneath never showed the new profile. Library add, variant row and detected confirm now close `show_printer_picker` (the source's `onAdded`), returning to the modal with the new profile active. |
| Modal's multi-select Printer Library (`addedOfficialPresetIds`, disabled added cards, `N printers selected`, `Add printers`) | absent | This port ships the onboarding single-select variant; the `ProfileSettingsModal` library surface is new work. |
| Setup shell (`Initial Setup`, subtitle = family display name, 560px, footer) | correct | — |
| `mode === 'choose'` + `SetupMethodChooser` cards | absent | Deviation 2: no network adapter, so the flow starts in `manual` (the source's own fallback) and the chooser never renders. |
| Scan status precedence and copy (`Detecting model…`, scanning + indeterminate bar, `N printers found`, `Select a printer to detect its model.`, error + `Scan again`) | correct (copy/order) / dead-by-design | Renders exactly; with no adapter the scan never yields devices, documented in deviation 2. |
| Detected card headline/subline | **wrong → fixed** | Headline used the *library* name and the subline the stripped variant title; the source is `detectedDeviceName?.trim() || detectedVariant.name` and `detectedDeviceName ? detectedVariant.name : The printer reported this variant.` (`:391`, `:454-457`, `:590-593`). `PresetRow::raw_name` now feeds both. |
| `Configure Networking Later` (25 s fallback) | absent | Deviation 6. New work. |
| Device rows, `Detecting` spinner on the connecting row | correct | Icon tile is simplified to the glyph (cosmetic, not documented — noted, not fixed). |
| Manual-IP card (`ManualIpEntryCard`): box, placeholder, enabled/disabled Connect, spinner label | correct / dead-by-design | Connect cannot reach an adapter; the code says so (deviation 2). |
| Variant rows: depth chip, `libraryDisplayName ?? stripVariantSuffix(name)`, description, `Added`, disabled | correct | — |
| `SetupModeButton` (names the mode it switches to) + `Cancel` + `Add printer` gating | correct | `showNetworkMode` in practice is `mode === scan`. |
| Escape/definitions: setup closes back to the library, library closes the picker | correct | `escape()`/`dismiss(back_to_library)`. |

---

## New work (`absent` surfaces, not attempted)

1. **Fleet Management / Network Settings** — fleet rail mode and cards, empty-fleet block, `Manage Fleet`/`Show Fleet`/`Return to Printers`, `Edit Unit` dialog, card footer network tile and fleet count, discovery/connect/disconnect/remove handlers, progress phases, cached discoveries, the `192.168.999.999` debug sentinel.
2. **Remote on-device materials** — the three-way visibility split, remote list, `Printer Not Connected` / `Connect to a Machine` cards, refresh, persisted selection, WIP-gated `Remote Material Edit` dialog (`Z-[71]`, Basic/Advanced, Dynamic Wait).
3. **Libraries and bundle IO** — Material Library dialog + preset explainer, printer/material bundle Import/Export, `AutoTrimmedImage` trim + cache, the printer-image upload pipeline.
4. **Official updates** — `Update` badge, `Update Printer` band action and the field-diff dialog, `Update Material`, apply handlers.
5. **ProfileSettingsModal's own Printer Library** — multi-select, added-disabled cards, selection count and `Add printers` (the port ships the onboarding single-select library for the add flow).
6. **Multi-Tank dialogs and close gating** — the three-row mismatch table with `Use <material>` actions, the unassigned-tanks dialog, `requestClose` gating, their Escape entries.
7. **Material editor scope** — `MaterialAntiAliasingSection`, `PluginLocalMaterialSettingsSections`, replacement-mode tabs and the `Edit <EXT> Material Profile` title.
8. **Card states** — missing-plugin muted card + delete tile; fleet view; `UPDATE` badge.
9. **Initial Setup networking** — `SetupMethodChooser` (`choose` mode), real scan/connect/`printerData` protocol, the 25 s fallback, device icon tiles.
10. **Printing manager** — layer scrub state machine (selected vs displayed layer), 180 ms settle timer, PNG decode handoff, multi-tank composite, zoom/pan/mirror transforms; the pane currently takes one resolved image path.
11. **Tank menu swatches** — `select_dropdown` needs per-row icon support before the source's colour chip can render.

## Verification note

No compilation or harness run was performed (the parent gates cargo; no
profiles/printing scenario exists). Rust-API usage in the fixes was checked
against the vendored `bite-gp-authoring` sources (`on_drag`, `on_drop`,
`ClickEvent::down.click_count`) and against existing call sites in
`gpui/src/chrome/overlays.rs` and `gpui/src/features/settings/fields.rs`.
