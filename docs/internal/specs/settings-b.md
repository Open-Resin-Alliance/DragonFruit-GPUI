# Settings — Mesh, Slicing, 3D Mouse, UI & Theme

Scope: `src/components/settings/` — the four sidebar tabs `Mesh`, `Slicing` (source
component `PerformanceSettingsTab`), `3D Mouse` (`SpaceMouseSettingsTab`) and
`UI & Theme` (`UISettingsTab`), plus everything they render:
`meshSettings/MeshShaderPreviewSlot.tsx`, `meshSettings/MeshShaderPreviewCanvas.tsx`,
`features/organicCut/OrganicCutColorsSection.tsx`, and the preference modules
`themeCustomizations.ts`, `performancePreferences.ts`, `spacemousePreferences.ts`.
The modal shell that hosts them (`SettingsModal.tsx`) is documented only for the
draft/apply/theme-preview plumbing these four tabs depend on.

## Components in scope

| Component | File |
|---|---|
| `SettingsModal` (shell wiring only) | `src/components/settings/SettingsModal.tsx` |
| `MeshSettingsTab` | `src/components/settings/MeshSettingsTab.tsx` |
| `MeshShaderPreviewSlot` | `src/components/settings/meshSettings/MeshShaderPreviewSlot.tsx` |
| `MeshShaderPreviewCanvas` (+ `ZUpPreviewCamera`, `CameraHeadlight`, `BuiltinPreviewMesh`, `StlPreviewMesh`, `PreviewContent`) | `src/components/settings/meshSettings/MeshShaderPreviewCanvas.tsx` |
| `PerformanceSettingsTab` (sidebar label **Slicing**) | `src/components/settings/PerformanceSettingsTab.tsx` |
| `SpaceMouseSettingsTab` | `src/components/settings/SpaceMouseSettingsTab.tsx` |
| `UISettingsTab` | `src/components/settings/UISettingsTab.tsx` |
| `OrganicCutColorsSection` (rendered by UI & Theme › Cutting Tool) | `src/features/organicCut/OrganicCutColorsSection.tsx` |
| theme model + persistence + applying CSS variables | `src/components/settings/themeCustomizations.ts` |
| slicing prefs persistence | `src/components/settings/performancePreferences.ts` |
| 3D mouse prefs persistence | `src/components/settings/spacemousePreferences.ts` |
| shader option tables (`MESH_SHADER_OPTIONS`, `MATCAP_OPTIONS`, `MESH_SHADER_TYPES`) | `src/features/shaders/mesh/types.ts` |
| shared atoms used by all four: `SettingRow`, `SegmentedControl`, `Select`, `SelectDropdown`, `ColorSwatchInput`, `Button`, `Input` | `src/components/atoms/*.tsx`, `src/components/ui/SelectDropdown.tsx` |
| preview model manifest (empty today) | `public/mesh-preview-models/models.json` |

## Sidebar metadata for these four tabs

From `SettingsModal.tsx:1209-1311` (`tabMeta`), rendered by `SettingsSidebarTab`
(`SettingsModal.tsx:232`):

| key | label (msgid) | description (msgid) | icon | tone |
|---|---|---|---|---|
| `mesh` | `Mesh` | `Shader, rendering options, and selection behavior` | `Grid3x3` | primary |
| `performance` | `Slicing` | `PNG compression and engine metadata` | `MonitorCog` | primary |
| `ui` | `UI & Theme` | `Theme and custom UI token customization` | `Palette` | primary |
| `spacemouse` | `3D Mouse` | `3D mouse navigation controls` | `Gamepad2` | primary |

Sidebar order (`sidebarTopTabs`, `SettingsModal.tsx:1314`):
`general, camera, workspaces, multiPlate, mesh, performance, spacemouse, ui, hotkeys`
then the secondary group after `mt-auto`.

Every one of these four tabs is mounted inside the shared content column:

```tsx
<div className={usesInternalTabScrollLayout ? 'flex-1 min-h-0 min-w-0 flex flex-col p-4' : 'flex-1 min-h-0 overflow-y-auto custom-scrollbar p-4'}>
  <div key={activeTab} className={... 'animate-[settingsTabIn_180ms_ease-out]'}>
```

`usesInternalTabScrollLayout` is `isAboutTab || activeTab === 'hotkeys' || activeTab === 'updates'`
— so **all four of these tabs use the outer scrolling column**, not an internal
scroller, and the tab body is re-created on tab switch (`key={activeTab}`), which
resets per-tab local state such as `UISettingsTab`'s `themeTab`, `MeshSettingsTab`'s
`previewModel` / `activeColorIndex`, and the preview hover/selected flags.

---

## Surface: SettingsModal — shell plumbing these four tabs depend on

- **Source**: `src/components/settings/SettingsModal.tsx:345` (component), `:388-402` (open reset), `:1090-1119` (draft reseed + live theme preview), `:1116-1157` (light-theme detection), `:1158-1171` (Escape cascade), `:876-889` (cancel), `:890-967` (restore defaults to draft), `:968-1085` (apply)
- **Renders**: full-screen backdrop `fixed inset-0 z-50 flex items-stretch justify-center bg-black/60 backdrop-blur-sm p-5 ui-modal-backdrop-enter`; card `w-full max-w-[72rem] h-full flex flex-col rounded-2xl shadow-2xl border overflow-hidden ui-modal-panel-enter` on `var(--surface-0)` with `borderColor: var(--border-strong)` and `boxShadow: '0 26px 64px rgba(0, 0, 0, 0.46)'`. Sidebar `w-72 min-h-0 p-2.5` with a vertical gradient background; footer with `Restore Defaults` / `Cancel` / `Apply`.
- **Layout & styling**: not re-specified here; the port's `gpui/src/features/settings/mod.rs` already owns `settings_modal` (`:369`), `header` (`:447`), `sidebar` (`:509`), `rail_tab` (`:544`), `content` (`:605`) and `footer` (`:663`).
- **States**:
  - Closed → `if (!isOpen) return null;` (`:1185`). The component stays mounted while closed; `activeTab` is reset to `initialTab ?? 'general'` on each open (`:388-402`).
  - Open → draft state is (re)seeded from props and from localStorage on the frame after open, by `resetDraftFromProps` (`:545-616`) inside a `requestAnimationFrame` (`:1090-1101`).
  - Closing without commit → the effect cleanup calls `restoreSavedThemePreview()` unless `didCommitThemeDraftRef.current` (`:1097-1100`), i.e. theme edits are rolled back on Cancel/backdrop/Escape.
  - `Restore Defaults` → opens the `Restore Defaults?` confirm (`StructuredDialogModal`, `z-[70]`); confirming runs `applyRestoreDefaultsToDraft` (`:890-967`), which writes the `DEFAULT_*` constants into the drafts **without** touching storage.
- **Data**: drafts live in `SettingsModal` local state: `draftMeshColor`, `draftShaderType`, `draftMatcapVariant`, `draftFlatUseVertexColors`, `draftAmbientIntensity`, `draftDirectionalIntensity`, `draftMaterialRoughness`, `draftBakedAoIntensity`, `draftXrayOpacity`, `draftHeatmapMinAngle`, `draftHeatmapMaxAngle`, `draftHeatmapColors`, `draftHoverTintStrength`, `draftSelectedTintStrength` (`:446-459`); `draftThemePreference`, `draftThemePreset`, `draftThemeColors`, `draftThemeProfiles`, `draftCustomThemeName` (`:477-489`); `draftSpaceMouseSettings` (`:495`); `draftSlicingPerformanceSettings`, `draftSlicingThumbnailRenderSettings` (`:496-497`).
- **Callbacks**:
  - `Apply` → `handleApply` (`:968-1085`): pushes every mesh draft through its `on*Change` prop, then `applyThemePreference(draftThemePreference)`, `applyThemeCustomColors(draftThemeColors)` (`:984-985`), `saveSpaceMouseSettings(draftSpaceMouseSettings)`, `saveSlicingPerformanceSettings(draftSlicingPerformanceSettings)`, `onSlicingThumbnailRenderSettingsChange(draftSlicingThumbnailRenderSettings)`, and finally writes `THEME_STORAGE_KEY`, `THEME_PRESET_STORAGE_KEY`, `THEME_COLORS_STORAGE_KEY`, `THEME_CUSTOM_PROFILES_STORAGE_KEY` directly, sets `didCommitThemeDraftRef.current = true`, then `requestClose()`.
  - `Cancel` → `handleCancel` (`:876-889`): closes any nested confirm, `restoreSavedThemePreview()`, `resetDraftFromProps()`, `requestClose()`.
  - Escape → `useEscapeToClose` cascade delete-confirm → rename-dialog → save-confirm → `handleCancel` (`:1158-1171`).
- **Text**: `Settings` / `Customize DragonFruit behavior, visuals, and controls.` / footer `Restore Defaults`, `Cancel`, `Apply`.
- **Port target**: `gpui/src/features/settings/mod.rs:369-703`. **Existing deviation to keep in mind:** the port's `Apply` and `Cancel` both only close the modal — the draft is the live state and the app persists it debounced (`mod.rs:1-30`). There is no commit/rollback, and therefore **no live theme preview / restore-on-cancel**. Any implementer adding theme work must decide this explicitly; see Open questions.

### Theme live-preview effect (critical for UI & Theme and Mesh)

```tsx
useEffect(() => {
  if (!isOpen) return;
  applyThemePreference(draftThemePreference);
  applyThemeCustomColors(draftThemeColors);
}, [draftThemeColors, draftThemePreference, isOpen]);
```

`SettingsModal.tsx:1107-1112`. Every keystroke in a theme hex field applies CSS
variables to `document.documentElement` immediately. `restoreSavedThemePreview`
(`:625-628`) re-applies `getSavedThemePreference()` / `getSavedThemeCustomColors()`.

`applyThemeCustomColors` also publishes the mesh highlight colours:

```ts
export type ThemeMeshHighlightColors = { selection: string; hover: string };
export function subscribeToThemeMeshHighlightColors(listener: () => void): () => void
export function getThemeMeshHighlightColors(): ThemeMeshHighlightColors
```

with the source comment: *"The 3D viewport is not a DOM consumer — it cannot read these off the CSS variables the rest of the theme is applied through — so it subscribes here instead."* (`themeCustomizations.ts:600-625`).

### Mesh draft ↔ theme coupling

- `draftThemeProfileColors = getThemeProfile(draftThemePreset, draftThemeProfiles).colors` (`SettingsModal.tsx:508-515`).
- `handleMeshSelectionColorChange = (c) => handleThemeColorChange('meshSelectionColor', c)` and the same for hover (`:619-624`) — the Mesh tab's Selection & Hover colours are **theme colours**, not appearance overrides.
- `handleDraftHeatmapColorChange` copies the array and writes one index (`:633-639`).
- `showPngCompressionControls = outputFormatUsesPngLayers(activeOutputFormat ?? undefined)` (`:506`).

---

## Surface: MeshSettingsTab — "Mesh" tab body

- **Source**: `src/components/settings/MeshSettingsTab.tsx:80` (component), `:186` (return)
- **Renders**: root `<div className="space-y-3">` containing exactly two `<section className="rounded-md border p-3" style={{ background: 'var(--surface-1)', borderColor: 'var(--border-subtle)' }}>` blocks: **Shader & Preview** (`:189`) and **Selection & Hover** (`:484`).
- **Layout & styling**: sections `rounded-md` (6px) `p-3` (12px) on `--surface-1` with `--border-subtle`. Header block in each: `flex items-start gap-2` (+ `mb-3` in the first section), 32px tile `inline-flex h-8 w-8 items-center justify-center rounded-md border shrink-0` with `borderColor: var(--border-subtle)`, `background: color-mix(in srgb, var(--surface-2), transparent 8%)` and a `h-4 w-4` lucide icon in `var(--accent)` (`Layers` in section 1, `MousePointer2` in section 2); title `text-xs font-semibold` in `--text-strong`; description `text-xs mt-0.5` in `--text-muted`. Inner cards are `rounded-md border px-2.5 py-1.5` (or `p-2` for the selector cards) on `var(--surface-0)` with `--border-subtle`.
- **States**: no loading/empty/error branches. Conditional *rendering options* (see below), one conditional swatch strip, and the stateful preview badge.
- **Data** (all props, no store access):
  `configuredShaderType`, `onConfiguredShaderTypeChange`, `matcapVariant`, `onMatcapVariantChange`, `flatUseVertexColors`, `onFlatUseVertexColorsChange`, `meshColor`, `onMeshColorChange`, `ambientIntensity`, `onAmbientIntensityChange`, `directionalIntensity`, `onDirectionalIntensityChange`, `materialRoughness`, `onMaterialRoughnessChange`, `bakedAoIntensity`, `onBakedAoIntensityChange`, `xrayOpacity`, `onXrayOpacityChange`, `heatmapMinAngle`/`onHeatmapMinAngleChange`, `heatmapMaxAngle`/`onHeatmapMaxAngleChange`, `heatmapColors: string[]` + `onHeatmapColorChange(index, color)`, `meshSelectionColor`/`onMeshSelectionColorChange`, `meshHoverColor`/`onMeshHoverColorChange`, `hoverTintStrength`/`onHoverTintStrengthChange`, `selectedTintStrength`/`onSelectedTintStrengthChange`, `defaultMeshSelectionColor`, `defaultMeshHoverColor`.
  Local state: `previewModel` (default `'knot'`), `stlPreviewModels` (`[]`), `activeColorIndex` (`0`), `isPreviewHovered` (`false`), `isPreviewSelected` (`false`) (`:118-122`).
  Header comment on props (`:52-56`): *"Selection/hover tint, owned by the active theme — these are `ThemeCustomColors.meshSelectionColor` / `meshHoverColor` mirrored into this tab, not separate appearance overrides."*
- **Callbacks**: see each sub-surface.
- **Text**: see sub-surfaces; all visible copy is quoted there.
- **Port target**: `gpui/src/features/settings/mesh.rs:5` (`mesh_tab`) and `:483` (`heatmap_swatch`), with the tables in `theme_colors.rs` and the control kit in `widgets.rs`.

### Derived values (compute these exactly)

```ts
const totalLight = ambientIntensity + directionalIntensity;                    // :145
const lightness  = Math.min(4, Math.max(0, totalLight));                        // :146
const contrast   = totalLight > 0 ? directionalIntensity / totalLight : 0.5;    // :147

const showLighting  = configuredShaderType === 'soft_clay' || configuredShaderType === 'xray';  // :151
const showRoughness = configuredShaderType === 'soft_clay' || configuredShaderType === 'xray';  // :152
// The baked occlusion only reaches the model through the Standard material.
const showBakedAo   = configuredShaderType === 'soft_clay';                     // :154
const hasRenderingOptions =
  configuredShaderType === 'matcap' ||
  configuredShaderType === 'flat_unlit' ||
  showRoughness || showBakedAo || showLighting ||
  configuredShaderType === 'overhang_heatmap';                                  // :155-161
```

`activeHexColor = activeColorIndex === 0 ? meshColor : heatmapColors[activeColorIndex - 1]` (`:163`);
`onActiveHexChange(c)` writes `onMeshColorChange(c)` at index 0 else `onHeatmapColorChange(index-1, c)` (`:164-168`).

---

## Surface: Mesh › Shader & Preview — selectors + preview + colour picker

- **Source**: `MeshSettingsTab.tsx:189-345`
- **Renders**: section header (above), then `grid grid-cols-2 gap-2 mb-2` of two selector cards, then `grid grid-cols-2 gap-2` of the preview tile and the colour tile.

Selector cards (`:213`, `:233`): `rounded-md border p-2` on `var(--surface-0)`, label `text-xs font-medium block mb-1` in `var(--text-muted)`, then a `Select` with `className="w-full !h-8"`.

- **Shader Type** — `value={configuredShaderType}`, options from `MESH_SHADER_OPTIONS` rendered with `_(opt.label)`:
  | value | label (msgid) | translator comment |
  |---|---|---|
  | `soft_clay` | `Standard` | `View mode: matte surface lit by the scene lights. The default view.` |
  | `overhang_heatmap` | `Overhangs` | `View mode colouring surfaces by how steeply they overhang, which is where supports are needed.` |
  | `opaque_wire_mesh` | `Mesh` | `View mode: mesh edges drawn over a solid surface, so back edges stay hidden.` |
  | `wireframe` | `Wireframe` | `View mode: only the mesh edges are drawn, so the surface between them shows through.` |
  | `xray` | `X-Ray` | `View mode: semi-transparent surface revealing the interior.` |
  | `normal_debug` | `Normals` | `View mode that colours the surface by its normal vector, for debugging. "Normal" is the geometry term, not the opposite of "unusual".` |
- **Preview Model** — `value={previewModel}`, fixed options `Cube` (`msg`Cube``), `Sphere` (`msg`Sphere``), `Knot` (`msg({ message: 'Knot', comment: 'A torus-knot preview shape.' })`), followed by `stlPreviewModels.map(...)` with `value={'stl:/mesh-preview-models/' + m.file}` and `label=m.label`.

Fetch (`:124-141`): `fetch('/mesh-preview-models/models.json', { cache: 'no-store' })`; the result must be `ok` and `Array.isArray(data.models)`, else the state stays `[]`; all errors swallowed (`// ignore`).

- **Preview tile**: `rounded-md border overflow-hidden` with `style={{ borderColor: 'var(--border-subtle)', background: 'var(--surface-0)', aspectRatio: '10 / 7' }}` wrapping `MeshShaderPreviewSlot` (`:246-266`), fed `hoverTintStrength={0.5}` and `selectedTintStrength={0.7}` — **hard-coded, not the user's tint strengths**.
- **Colour tile**: `rounded-md border p-2 flex flex-col gap-1.5`, same `aspectRatio: '10 / 7'` (`:269-270`). Row 1: `flex items-center gap-1.5` with label `text-xs font-medium whitespace-nowrap shrink-0` (`Mesh Color` when index 0, else `Heatmap {index}`), then `Input type="text"` `className="flex-1 !h-7 min-w-0"` `placeholder="#a3a3a3"` bound to `activeHexColor` → `onActiveHexChange`.
  Row 2: `flex-1 min-h-0 rounded overflow-hidden` on `var(--surface-2)` holding `HexColorPicker` from `react-colorful` at `style={{ width: '100%', height: '100%' }}`.
  Row 3 (only when `configuredShaderType === 'overhang_heatmap'`, `:297-325`): `flex items-center gap-1 pt-0.5` —
  swatch 0 `onClick={() => setActiveColorIndex(0)}`, `className="h-5 w-5 rounded border transition-all"`, `backgroundColor: meshColor`, `borderColor: activeColorIndex === 0 ? 'var(--text-strong)' : 'var(--border-strong)'`, `boxShadow: activeColorIndex === 0 ? '0 0 0 1px var(--accent)' : 'none'`, `title={_(msg`Mesh Color`)}`;
  then `<div className="w-px h-4 mx-0.5 shrink-0" style={{ background: 'var(--border-subtle)' }} />`;
  then one button per heatmap colour: `flex-1 h-5 rounded border transition-all`, `backgroundColor: color`, the same active border/boxShadow recipe against `activeColorIndex === idx + 1`, `title={formatHeatmapColorTitle(_, idx + 1)}` → `Heatmap Color {index}` (1-based).
- **Text**: `Shader & Preview`; `Pick the shader type to configure here, and tune the mesh color. The camera dropdown chooses what the viewport renders.`; `Shader Type`; `Preview Model`; `Mesh Color`; `Heatmap {index}` (msg comment: *"Short label of one overhang heatmap color stop, numbered from 1."*); `Heatmap Color {index}`.
- **Port target**: `mesh.rs:5-160` has the section, both selects (`SHADER_OPTIONS`, `PREVIEW_MODEL_OPTIONS`, `PREVIEW_MODEL_VALUES` from `defaults.rs`), a placeholder preview painted as `mix(mesh_color, SURFACE_0, 0.35)` labelled with the shader name, and the colour card with a fake saturation panel plus the `heatmap_swatch` strip. Missing: the real 3D preview, the picker, and the initial preview model (state stores an index; the source default is `knot`).

---

## Surface: Mesh › Rendering Options

- **Source**: `MeshSettingsTab.tsx:327-480`
- **Renders**: only when `hasRenderingOptions`. A divider label, then a `grid grid-cols-2 gap-2 mt-2` of option cards.
- **Layout & styling**: divider row `flex items-center gap-2 mt-3`: two `h-px flex-1` rules in `var(--border-subtle)` around `span.flex items-center gap-1.5 text-xs font-semibold shrink-0` in `var(--text-muted)` containing `SlidersHorizontal h-3 w-3` and the text `Rendering Options`. Cards: `rounded-md border px-2.5 py-1.5` on `var(--surface-0)`.
- **States / conditions** (each card appears only for its shader):
  | condition | card |
  |---|---|
  | `configuredShaderType === 'matcap'` | **Matcap Style** `Select` (`w-full !h-8`) over `MATCAP_OPTIONS`: `Neutral`, `Cool`, `Warm` (msg comments: *"Matcap lighting preset: neither warm nor cool."*, *"…with a cool (blueish) tint."*, *"…with a warm (orange) tint."*) |
  | `configuredShaderType === 'flat_unlit'` | `SettingRow bordered className="col-span-2"` label `Vertex Colors`, description `Sample per-vertex color data when available.`, control `SegmentedControl` `OFF`/`ON` (`on` ⇔ `flatUseVertexColors`) |
  | `showRoughness` | **Roughness** slider `min="0.0" max="1.0" step="0.05"` |
  | `showLighting` | **Lightness** slider `min="0.2" max="3.0" step="0.05"` |
  | `showLighting` | **Contrast** slider `min="0.05" max="0.95" step="0.01"` |
  | `showBakedAo` | **Ambient Occlusion** slider `min="0.0" max="2.0" step="0.05"` |
  | `configuredShaderType === 'xray'` | **X-Ray Opacity** slider `min="0.02" max="0.85" step="0.01"` |
  | `configuredShaderType === 'overhang_heatmap'` | **Min Overhang** and **Max Overhang** sliders, both `min="0" max="90" step="1"` |
- **Slider card shape**: `rounded-md border px-2.5 py-1.5`; header `flex items-center justify-between text-xs mb-1.5` with label `font-medium` in `--text-muted` and value `font-semibold tabular-nums` in `--text-strong` formatted `fmt.fixed(value, 2)`; track `<input type="range" className="ui-range w-full">`.
- **Callbacks** (semantics matter):
  - Roughness → `onMaterialRoughnessChange(parseFloat(value))`.
  - Lightness → `onLightnessChange(next)` = `onAmbientIntensityChange((1 - c) * next)`, `onDirectionalIntensityChange(c * next)` where `c` is the **current** `contrast` (`:169-173`).
  - Contrast → `onContrastChange(next)` = `onAmbientIntensityChange((1 - next) * t)`, `onDirectionalIntensityChange(next * t)` where `t` is the **current** `lightness` (`:175-179`).
  - Ambient Occlusion → `onBakedAoIntensityChange(parseFloat(...))`.
  - X-Ray Opacity → `onXrayOpacityChange(...)`.
  - Min Overhang → `onHeatmapMinAngleChange(Math.min(parseFloat(value), heatmapMaxAngle))`.
  - Max Overhang → `onHeatmapMaxAngleChange(Math.max(parseFloat(value), heatmapMinAngle))`.
- **Text**: `Rendering Options`, `Matcap Style`, `Vertex Colors`, `Sample per-vertex color data when available.`, `Roughness`, `Lightness`, `Contrast`, `Ambient Occlusion`, `X-Ray Opacity`, `Min Overhang` (`{heatmapMinAngle}°` — raw number, no formatting), `Max Overhang` (`{heatmapMaxAngle}°`), `OFF`, `ON`.
- **Port target**: `mesh.rs:160-260` has the divider (`divider_label("sliders-horizontal", "Rendering Options")`) and `slider_field` cards for Roughness / Lightness / Contrast / AO / X-Ray / Min+Max Overhang. Missing: the Matcap Style card, the Vertex Colors `SettingRow`, and any drag handling (`widgets.rs:642` `slider_field` is display-only).

---

## Surface: Mesh › Selection & Hover

- **Source**: `MeshSettingsTab.tsx:483-663`
- **Renders**: header (icon `MousePointer2`, title `Selection & Hover`, description below), then `grid grid-cols-2 gap-2`: left column `min-w-0 space-y-2` with two cards; right column `relative min-w-0` with an absolutely-positioned preview card.
- **Layout & styling**: description text (verbatim, `<Trans>` spans lines):
  `How selected and hovered models are emphasized throughout the app. These two colors come from the current theme — editing them here changes the theme, exactly like UI & Theme › Mesh Highlights.`
- **Card 1 — Colors** (`:510-554`): `rounded-md border px-2.5 py-1.5` on `var(--surface-0)`.
  - Header `flex items-center justify-between mb-2`: `div.text-xs.font-semibold` `Colors`; a `Button variant="secondary" size="auto"` with `className="!h-6 !w-6 !p-0 inline-flex shrink-0 items-center justify-center rounded-md"`, `title={_(msg`Reset colors to default`)}`, `aria-label={_(msg`Reset selection and hover colors to default`)}`, child `RotateCcw h-3 w-3`.
  - Body `grid gap-2 sm:grid-cols-2`, two fields, each `space-y-1`: label `text-xs font-medium` in `--text-muted` (`Selection Color` / `Hover Color`), then `flex items-center gap-2` with `ColorSwatchInput className="h-8 w-10"` and `input type="text" className="ui-input h-8 w-[7.5rem] min-w-0"` `placeholder="#ec2a77"`.
  - Callbacks: swatch and text both write `onMeshSelectionColorChange` / `onMeshHoverColorChange` on every change (no local draft). Reset → `handleResetColors` = `onMeshSelectionColorChange(defaultMeshSelectionColor)` + `onMeshHoverColorChange(defaultMeshHoverColor)` (`:181-184`), where the defaults are the **selected theme profile's** colours (`draftThemeProfileColors`), not the app defaults.
- **Card 2 — Tint Intensity** (`:562-590`): `rounded-md border px-2.5 py-1.5`; title `text-xs font-semibold mb-2` `Tint Intensity`; `space-y-2` of two rows. Each row: `flex items-center justify-between text-xs mb-1` with `span` in `--text-muted` (`Hover` / `Selected`, both `msg({ message: 'Hover'/'Selected', context: 'Tint intensity' })`) and a `tabular-nums` value in `--text-strong` formatted `fmt.fixed(x, 2)`; then `input type="range" min="0" max="1" step="0.01" className="ui-range w-full"`. Values are `hoverTintStrength` / `selectedTintStrength`.
- **Preview card** (`:596-660`): the parent is `relative min-w-0`; the card is `absolute inset-0 rounded-lg border p-2 flex flex-col gap-2` on `var(--surface-0)` with `var(--border-subtle)`. Source comment:
  ```tsx
  {/* The preview fills the row the Colors / Tint Intensity stack sets. It is
      positioned absolutely so its own height never stretches that row: a
      taller preview would otherwise leave the left column short and ragged. */}
  ```
  - Header `flex items-center justify-between gap-2`: `Selection Preview` (`text-xs font-semibold`, `--text-strong`) and a pill `inline-flex items-center rounded-full border px-2 py-0.5 text-[11px] font-semibold transition-colors` whose style is:
    - selected → `{ color: 'var(--text-strong)', borderColor: 'color-mix(in srgb, var(--accent), white 12%)', background: 'color-mix(in srgb, var(--accent), transparent 22%)' }`, label `selected`;
    - else hovered → `{ color: 'var(--text-strong)', borderColor: 'var(--border-strong)', background: 'var(--surface-2)' }`, label `hovered`;
    - else → `{ color: 'var(--text-muted)', borderColor: 'var(--border-subtle)', background: 'transparent' }`, label `idle`;
    labels are `msg({ message: 'selected'|'hovered'|'idle', context: 'Preview state' })`.
  - `MeshShaderPreviewCanvas` with constants: `shaderType="soft_clay"`, `matcapVariant="neutral"`, `flatUseVertexColors={true}`, `useVertexColors={false}`, `meshColor="#a3a3a3"`, `materialRoughness={0.55}`, `previewModel="knot"`, `ambientIntensity={0.28}`, `directionalIntensity={1.12}`, `xrayOpacity={0.25}`, `heatmapMinAngle={0}`, `heatmapMaxAngle={45}`, `hoverTintColor={meshHoverColor}`, `selectedTintColor={meshSelectionColor}`, `hoverTintStrength`/`selectedTintStrength` from props, `isSelected`/`isHovered` from local state, `onHoverChange={setIsPreviewHovered}`, `onPress={() => setIsPreviewSelected(prev => !prev)}`, `onCanvasPress={() => setIsPreviewSelected(false)}`.
- **Text**: `Selection & Hover`, the description above, `Colors`, `Tint Intensity`, `Hover`, `Selected`, `Selection Preview`, `selected`, `hovered`, `idle`, `Selection Color`, `Hover Color`, `Mesh Color` (tooltip), `Reset colors to default`, `Reset selection and hover colors to default`.
- **Port target**: `mesh.rs:270-482` (`selection_card`, `tint_card`, `preview_card`) plus `color_field` (`ui_theme.rs:411`). Deviations: the reset writes `gpui_store::ThemeColors::default()` (`mesh.rs:305-332`) instead of the selected profile's colours; the swatch click cycles a palette via `next_theme_color` (`ui_theme.rs:466`) instead of opening a picker; the preview is a flat tinted rectangle (`mix(color, SURFACE_0, strength)`) with a badge, not a 3D canvas.

---

## Surface: MeshShaderPreviewSlot — 1:1 wrapper

- **Source**: `src/components/settings/meshSettings/MeshShaderPreviewSlot.tsx:14`
- **Renders**: `<div className="w-full h-full relative">` around `MeshShaderPreviewCanvas`.
- **Layout & styling**: nothing of its own; passes every prop through (`heatmapColors?` optional).
- **States**: none.
- **Port target**: no equivalent needed; fold into the preview element in `mesh.rs`.

## Surface: MeshShaderPreviewCanvas — the 3D shader preview

- **Source**: `src/components/settings/meshSettings/MeshShaderPreviewCanvas.tsx:472` (exported component), sub-parts at `:16` `ZUpPreviewCamera`, `:32` `CameraHeadlight`, `:51` `applyUniformVertexColor`, `:63` `normalizeGeometryToUnitSize`, `:78` `BuiltinPreviewMesh`, `:223` `StlPreviewMesh`, `:369` `PreviewContent`.
- **Renders**: `<div className="w-full h-full relative">` containing a react-three-fiber `<Canvas>`.
- **Layout & styling**: container `w-full h-full relative`. Canvas props:
  ```tsx
  gl={{ alpha: true, antialias: true }}
  camera={{ position: [0, -cameraDistance, 0], fov: 35 }}
  dpr={[1, 2]}
  onPointerMissed={() => { onHoverChange?.(false); onCanvasPress?.(); }}
  ```
  with `const cameraDistance = previewModel === 'knot' ? 7.2 : 5.6;`
- **Scene graph (in `PreviewContent`)**:
  - `ZUpPreviewCamera`: sets `camera.up.set(0,0,1)`, `camera.position.set(0, -distance, 0)`, `camera.lookAt(0,0,0)`, `updateProjectionMatrix()` in an effect keyed on `[camera, distance]`.
  - `OrbitControls` `enablePan={false} enableZoom={false} enableRotate autoRotate autoRotateSpeed={0.6} enableDamping dampingFactor={0.08} rotateSpeed={0.9}`.
  - Lights: `<ambientLight intensity={ambientIntensity} />`, `<directionalLight position={[0, 0, 12]} intensity={directionalIntensity} />`, `<directionalLight position={[0, 0, -12]} intensity={directionalIntensity * 0.15} />`, `<hemisphereLight args={['#ffffff', '#444444', ambientIntensity * 0.6]} />`, `<CameraHeadlight intensity={1.0} />` (a `pointLight` with `intensity`, `decay={0}`, `distance={0}`, `color="#ffffff"` whose position is copied from the camera every frame).
  - Geometry (memoised on `[shape]`, then `computeVertexNormals()`, then `applyUniformVertexColor`):
    - `cube` → `new THREE.BoxGeometry(1.6, 1.6, 1.6, 1, 1, 1)`
    - `knot` → `new THREE.TorusKnotGeometry(1, 0.35, 160, 24)`
    - default/sphere → `new THREE.SphereGeometry(1.1, 48, 32)`
    `builtinShape` maps `'sphere'→sphere`, `'knot'→knot`, **everything else → cube** (`:407-409`).
  - STL models: `previewModel.startsWith('stl:')` → `url = previewModel.slice('stl:'.length)`, loaded with `useLoader(STLLoader, url)`, cloned, `computeVertexNormals()`, then `normalizeGeometryToUnitSize` (`computeBoundingBox`, `center()`, `scale(1.5 / maxDim)` where `maxDim = max(size.x,size.y,size.z)`; returns unchanged if `maxDim` is not finite or `<= 0`).
  - `opaque_wire_mesh` renders a `<group>` of two meshes over the same geometry: the first uses `MeshShaderMaterial` with `shaderType={'soft_clay'}` (**forced**, not the selected type), the second `renderOrder={1}` with `OpaqueWireOverlayMaterial` — so back edges stay hidden.
  - Every other type uses one `<mesh>` with `MeshShaderMaterial shaderType={shaderType}`.
  - All meshes get `onPointerOver` → `stopPropagation()` + `onHoverChange?.(true)`, `onPointerOut` → `stopPropagation()` + `onHoverChange?.(false)`, `onClick` → `stopPropagation()` + `onPress?.()`.
  - `clippingPlanes={[]}` on every material.
- **Data**: props listed above; `useVertexColors` defaults `true` in the component signature but the Mesh tab's Selection preview passes `false`.
- **Port target**: `gpui/src/features/settings/mesh.rs` currently paints a flat colour block for the shader preview and a tinted rectangle for the selection preview. The real target is the existing wgpu viewport stack (`gpui/crates/gpui-core/viewport/*`) rendering a second, small pass with this exact camera/light setup.

---

## Surface: PerformanceSettingsTab — sidebar "Slicing" tab body

- **Source**: `src/components/settings/PerformanceSettingsTab.tsx:27` (component), `:64` (return)
- **Renders**: root `<div className="space-y-3">` with four `<section className="rounded-md border p-3" style={{ background: 'var(--surface-1)', borderColor: 'var(--border-subtle)' }}>` blocks: **Slicing Engine** (`:68`), **Thumbnail Rendering** (`:109`), **PNG Compression** (`:185`, conditional), **Temp File Cleanup** (`:239`).
- **Layout & styling**: same section/header recipe as Mesh (`flex items-start gap-2`, 32px tile `h-8 w-8 rounded-md border` with `color-mix(in srgb, var(--surface-2), transparent 8%)`, `h-4 w-4` icon in `--accent`, `h3.text-xs.font-semibold`, `p.text-xs.mt-0.5`). Icons: `Cpu`, `ImageIcon`, `Zap`, `Trash2`. Inner card `mt-3 rounded-md border p-2.5` on `var(--surface-0)` (`space-y-2` for the thumbnail card).
- **States**:
  - Engine version `engineVersion: string | null`, starts `null`; `null` renders `-`.
  - PNG section only when `showPngCompressionControls` (prop, default `true`) — the modal passes `outputFormatUsesPngLayers(activeOutputFormat)` (`:506`), which is `true` when there is no active format, and `true` when the format is unknown to the built-in registry (`src/features/slicing/formats/registry.ts:79-85`).
  - Cleanup buttons: no loading state; success/failure surfaces via `alert(...)`.
- **Data**: props `settings: SlicingPerformanceSettings`, `onChange`, `thumbnailSettings: SlicingThumbnailRenderSettings`, `onThumbnailSettingsChange`, `showPngCompressionControls?`.
  `patch(partial)` = `onChange({ ...settings, ...partial })` (`:35-37`); `patchThumbnailSettings(partial)` = `{ ...thumbnailSettings, ...partial }` (`:39-45`).
- **Callbacks**:
  - Mount effect (`:48-58`): `const { invoke } = await import('@tauri-apps/api/core'); const v = await invoke<string>('get_slicer_engine_version');` — accepted only if `typeof v === 'string' && v.trim()`; stored trimmed. Errors swallowed. Source comment: *"Null until the native side answers; outside Tauri it never does."*
- **Text**: `Slicing Engine`, `Native Rust slicing via the DragonFruit engine.`, `Crate`, `dragonfruit-slicing-engine`, `Version`, `-`.
- **Port target**: `gpui/src/features/settings/slicing.rs:5` (`slicing_tab`) — the engine section exists with `Version` hard-coded `"-"` and no invoke.

### Slicing › Thumbnail Rendering

- **Source**: `PerformanceSettingsTab.tsx:109-182`
- **Renders**: header (icon `ImageIcon`, title `Thumbnail Rendering`, description `Configure what appears in generated export/slice thumbnails.`), then `mt-3 rounded-md border p-2.5 space-y-2` holding three `SettingRow`s (default density `compact`, `surface` default `sunken` — they are **not** `bordered`; the parent card is the border).
- Rows:
  | label (msgid) | description (msgid) | control |
  |---|---|---|
  | `Background gradient` | `Scene mood overlay in thumbnail` | `SegmentedControl` `OFF`/`ON`, `includeGradient` |
  | `Build plate` | `Render build plate in thumbnail` | `SegmentedControl` `OFF`/`ON`, `includeBuildPlate` |
  | `Grid` | `Render build grid in thumbnail` | `SegmentedControl` `OFF`/`ON`, `includeGrid` |
  Each `SegmentedControl` also gets `label={_(msgid)}` (same string as the row label, for `aria-label`), options `[{ value: 'off', label: _(msg`OFF`) }, { value: 'on', label: _(msg`ON`) }]`, `value={flag ? 'on' : 'off'}`, `onChange={(next) => patchThumbnailSettings({ flag: next === 'on' })}`.
- **Port target**: `slicing.rs:59-130` — present (`settings-thumbnail-gradient|plate|grid`, `segmented` with `["OFF","ON"]`).

### Slicing › PNG Compression

- **Source**: `PerformanceSettingsTab.tsx:184-237`
- **Renders**: header (icon `Zap`, title `PNG Compression`, description `Enable or disable PNG compression for PNG-based formats.`), `mt-3 rounded-md border p-2.5` with one `SettingRow` label `Compression`, description `Auto adapts by AA level, Off is fastest, On favors smaller PNG files`, control `SegmentedControl` `Auto` / `Off` / `On`.
- **Data mapping** (`:59-63`):
  ```ts
  const pngCompressionMode = settings.pngCompressionStrategy === 'auto' ? 'auto'
    : settings.pngCompressionStrategy === 'fastest' ? 'off' : 'on';
  ```
- **Callback** (`:225-232`): `auto → 'auto'`, `off → 'fastest'`, `on → 'balanced'`. Note `'smallest'` and `'optimal'` are valid stored values that all display as `On`, and choosing `On` collapses them to `'balanced'`.
- **Text**: `PNG Compression`, description above, `Compression`, `Auto adapts by AA level, Off is fastest, On favors smaller PNG files`, `Auto`, `Off`, `On`.
- **Port target**: `slicing.rs:130-175` — present; mapping in `state.rs` (`png_mode_index` / `png_mode_strategy`) matches.

### Slicing › Temp File Cleanup

- **Source**: `PerformanceSettingsTab.tsx:238-318`
- **Renders**: header (icon `Trash2`, title `Temp File Cleanup`, description `Free disk space by removing temporary slice files.`), then `mt-3 space-y-2` with two full-width buttons.
- **Layout & styling**:
  - Stale: `className="w-full rounded-md border p-2.5 text-left transition-all hover:border-[var(--accent)] hover:bg-[color-mix(in_srgb,var(--accent),var(--surface-0)_92%)]"`, `style={{ borderColor: 'var(--border-subtle)', background: 'var(--surface-0)' }}`. Title `text-xs font-semibold` `--text-strong`: `Clean Stale Files`; subline `text-xs mt-0.5` `--text-muted`: `Remove temp files older than 1 hour`.
  - All: same shape, hover `hover:border-red-500/50 hover:bg-red-500/5`. Title `Clean All Files`; subline `Emergency cleanup: delete all temp slices`.
- **Callbacks**:
  - Stale: `const removed = await cleanupStalePrintTempArtifacts(60 * 60); alert(\`Cleaned ${removed} temp file(s) older than 1 hour.\`);` — catch logs `[Cleanup] Failed:` and `alert('Cleanup failed. See console for details.')`.
  - All: `if (!confirm('Delete ALL temporary slice files? This cannot be undone.')) return;` then `const removed = await cleanupAllPrintTempArtifacts(); alert(\`Cleaned ${removed} temp file(s).\`);` same catch.
  - Bridge: `cleanup_stale_print_temp_files { maxAgeSeconds: Math.max(60, Math.floor(age)) }` and `cleanup_all_print_temp_files`; outside Tauri both return `0` without invoking (`nativeSlicerBridge.ts:1056-1073`).
- **Text**: `Temp File Cleanup`, `Free disk space by removing temporary slice files.`, `Clean Stale Files`, `Remove temp files older than 1 hour`, `Clean All Files`, `Emergency cleanup: delete all temp slices`, `Delete ALL temporary slice files? This cannot be undone.`, `Cleaned {n} temp file(s) older than 1 hour.`, `Cleaned {n} temp file(s).`, `Cleanup failed. See console for details.`
- **Port target**: `slicing.rs:175-240` (`cleanup_button` at `:203`); buttons render with hover fills `mix(ACCENT, SURFACE_0, 0.08)` / `mix(DANGER, SURFACE_0, 0.05)` but only `cx.notify()` — not wired to the backend, and no confirm/result dialog.

---

## Surface: SpaceMouseSettingsTab — "3D Mouse" tab body

- **Source**: `src/components/settings/SpaceMouseSettingsTab.tsx:18` (component), `:51` (return)
- **Renders**: root `<div className="space-y-3">` with: section **3D Mouse Input** (`:52`), conditionally section **Motion Tuning** (`:188`), conditionally section **Axis Inversion** (`:301`), then an always-present trademark strip (`:353`).
- **Layout & styling**: sections identical to the other tabs (`rounded-md border p-3`, `--surface-1`, `--border-subtle`, 32px accent tile). Icons `MousePointer2`, `SlidersHorizontal`, `ArrowLeftRight`. Inner cards `mt-2 rounded-md border p-2.5` on `var(--surface-0)`.
- **States**:
  - `navlibActive` via `useSyncExternalStore(subscribeNativeSpaceMouseActive, getNativeSpaceMouseActive, () => false)`. Source comment: *"When the native 3DxWare/navlib bridge is driving the camera, the 3Dconnexion driver computes all motion itself — the app never applies per-axis sensitivity or a deadzone — so the Motion Tuning controls below are inert. Hide them while navlib is active (tuning lives in the 3Dconnexion control panel instead)."* When true, the **Pivot mode card, the whole Motion Tuning section and the whole Axis Inversion section are not rendered**.
  - Devices: `candidatePads.length === 0` → muted paragraph; else a device list. Per device: `isBlocked = settings.blockedDeviceIds.includes(pad.id)`; `isNamed = NAMED_3D_MOUSE.test(pad.id)`.
- **Data**:
  - Props `settings: SpaceMouseSettings`, `onChange: (partial: Partial<SpaceMouseSettings>) => void`. The parent wraps it in `handleSpaceMouseChange` = `setDraftSpaceMouseSettings(prev => normalizeSpaceMouseSettings({ ...prev, ...partial }))` (`SettingsModal.tsx:1174-1176`) — **every** change is normalised (clamped) before it lands.
  - `candidatePads` polled: `refresh()` immediately, then `window.setInterval(refresh, 800)`, plus `gamepadconnected` / `gamepaddisconnected` listeners; all cleaned up on unmount (`:33-46`). `getCandidateGamepads()` = `navigator.getGamepads()` filtered to `pad.axes.length >= 6` (`SpaceMouseController.tsx:47-50`).
  - `NAMED_3D_MOUSE = /spacemouse|3dconnexion|space navigator|spacepilot/i` (`SpaceMouseController.tsx:35`).
- **Port target**: `gpui/src/features/settings/spacemouse.rs:5` — all three sections and the strip exist; `navlibActive` is hard-wired false and `candidatePads` is always empty, so the empty state is what renders.

### 3D Mouse › 3D Mouse Input

- **Source**: `SpaceMouseSettingsTab.tsx:52-183`
- **Renders**: header (title `3D Mouse Input`, description `6-DoF navigation for pan, orbit, zoom, and roll using compatible 3D mouse devices.`), then `SettingRow bordered className="mt-3"` label `Enable 3D Mouse`, description `Uses browser gamepad input from supported 3D mouse devices.`, control `SegmentedControl` `OFF`/`ON` (`on` ⇔ `settings.enabled`), `onChange={(next) => onChange({ enabled: next === 'on' })}`.
- Then the **Detected devices** card: `mt-2 rounded-md border p-2.5` on `var(--surface-0)`.
  - Header `flex items-center gap-2 mb-2`: `Gamepad2 h-4 w-4` coloured `var(--accent)` when `candidatePads.length > 0` else `var(--text-muted)`; `div.text-xs.font-semibold` `Detected devices`.
  - Empty state: `<p className="text-xs">` in `--text-muted`: `No compatible input devices detected. Ensure your 3D mouse driver is installed.`
  - List: `flex flex-col gap-1.5`; each row `flex items-start gap-2 rounded-md border p-2` with
    ```tsx
    borderColor: isBlocked
      ? 'color-mix(in srgb, #f87171, var(--border-subtle) 60%)'
      : isNamed
        ? 'color-mix(in srgb, var(--accent), var(--border-subtle) 60%)'
        : 'color-mix(in srgb, #fb923c, var(--border-subtle) 60%)',
    background: isBlocked
      ? 'color-mix(in srgb, #f87171, var(--surface-1) 94%)'
      : 'var(--surface-1)',
    ```
    icon `Ban h-3 w-3` in `#f87171` when blocked, else `CheckCircle2 h-3 w-3` in `var(--accent)` when named / `#fb923c` when unnamed; device id `text-xs font-semibold truncate` in `--text-strong`; subline `text-[11px] mt-0.5 pl-4.5` in `--text-muted`:
    `{pad.axes.length} axes · {isBlocked ? 'blocked' : isNamed ? 'recognized 3D mouse' : 'unrecognized — used as fallback'}`.
    Block toggle button: `className="flex-shrink-0 rounded border px-2 py-1 text-[11px] font-semibold uppercase tracking-wide transition-colors"`,
    blocked → `{ borderColor: 'var(--border-subtle)', color: 'var(--text-muted)', background: 'var(--surface-2)' }` label `Unblock`;
    else → `{ borderColor: 'color-mix(in srgb, #f87171, var(--border-subtle) 50%)', color: '#f87171', background: 'color-mix(in srgb, #f87171, var(--surface-0) 92%)' }` label `Block`.
    Click: blocked → `settings.blockedDeviceIds.filter(id => id !== pad.id)`; else `[...settings.blockedDeviceIds, pad.id]`; then `onChange({ blockedDeviceIds: next })`.
- **Pivot mode card** (only `!navlibActive`, `:165-181`): `mt-2 rounded-md border p-2.5` on `--surface-0`; `space-y-1`; label `text-xs font-semibold` `--text-strong` `Pivot mode`; `<p className="text-xs">` `--text-muted` `Auto picks selected/single/nearest model center. Camera Raycast picks the model center nearest your current view beam.`; `Select className="w-full !h-8"` with raw `<option value="auto">Auto</option>` and `<option value="camera-ray">Camera Raycast</option>` — **not** msg-wrapped, no `_()`.
- **Port target**: `spacemouse.rs:5-118` (section + devices + pivot). The port always renders the empty device list and the pivot card.

### 3D Mouse › Motion Tuning

- **Source**: `SpaceMouseSettingsTab.tsx:187-291`
- **Renders** (only `!navlibActive`): header (icon `SlidersHorizontal`, title `Motion Tuning`, description `Adjust sensitivity and movement behavior for translation, rotation, zoom, and deadzone.`), then `mt-2 grid grid-cols-2 gap-2` of four slider blocks, then `SettingRow bordered className="mt-2"` for the dominant-axis toggle.
- Each slider block: `space-y-0.5`; `label.text-xs.flex.justify-between` (`color: var(--text-muted)`) with `<span>Label</span>` and `<span style={{ color: 'var(--text-strong)' }}>{fmt.fixed(value, 2)}</span>`; then `<input type="range" className="ui-range w-full">`.
  | label | field | min | max | step |
  |---|---|---|---|---|
  | `Translation` | `translationSensitivity` | `0.1` | `4` | `0.05` |
  | `Rotation` | `rotationSensitivity` | `0.1` | `4` | `0.05` |
  | `Zoom` | `zoomSensitivity` | `0.1` | `4` | `0.05` |
  | `Deadzone` | `deadzone` | `0` | `0.3` | `0.01` |
  All write `onChange({ field: parseFloat(e.target.value) })`.
- `Dominant axis mode` row: label `Dominant axis mode`, description `Prioritize strongest axis per translation/rotation group for steadier input.`, `SegmentedControl` `OFF`/`ON` ⇔ `settings.dominantAxis`.
- **Port target**: `spacemouse.rs:120-190` — present, with `slider_field` (display-only) and the `setting_row` toggle.

### 3D Mouse › Axis Inversion

- **Source**: `SpaceMouseSettingsTab.tsx:300-350`
- **Renders** (only `!navlibActive`): header (icon `ArrowLeftRight`, title `Axis Inversion`, description `Flip the direction of individual pan, zoom, pitch, yaw, and roll axes.`), then `mt-2 grid grid-cols-3 gap-2 text-xs` of six checkbox tiles in this order:
  `invertTx` `Invert X pan`, `invertTy` `Invert Y pan`, `invertTz` `Invert zoom`, `invertRx` `Invert pitch`, `invertRy` `Invert yaw`, `invertRz` `Invert roll`.
- Tile: `<label className="inline-flex items-center gap-2 rounded border px-2 py-1.5" style={{ borderColor: 'var(--border-subtle)', background: 'var(--surface-0)' }}>` with `<input type="checkbox" checked={!!settings[key]} onChange={(e) => onChange({ [key]: e.target.checked })} className="ui-checkbox !w-3.5 !h-3.5" />` and `<span style={{ color: 'var(--text-muted)' }}>{label}</span>`.
- **Port target**: `spacemouse.rs:195-250` — present as clickable rows with `checkbox(checked, true)`.

### 3D Mouse › trademark strip

- **Source**: `SpaceMouseSettingsTab.tsx:353-360`
- **Renders**: `div.rounded-md border px-3 py-2` with `borderColor: 'color-mix(in srgb, var(--border-subtle), var(--accent) 20%)'`, `background: 'color-mix(in srgb, var(--surface-1), var(--accent) 4%)'`, containing `p.text-[11px].leading-snug` in `--text-muted`:
  `Supports 3Dconnexion SpaceMouse devices. “SpaceMouse” is a trademark of 3Dconnexion.`
  (Note the curly quotes `“ ”` and the em dash in the devices subline — copy them verbatim.)
- **Port target**: `spacemouse.rs:252-270`.

---

## Surface: UISettingsTab — "UI & Theme" tab body

- **Source**: `src/components/settings/UISettingsTab.tsx:120` (component), `:260` (return)
- **Renders**: root `<div className="space-y-2.5">` with the **Theme** section, then a sub-tab strip, then three stacked panels (`display: none` toggling) for `General UI` / `Mesh Highlights` / `Cutting Tool`.
- **Layout & styling**: Theme section is `rounded-xl border p-2.5` on `--surface-1` (note `rounded-xl` = 12px here, unlike the other tabs' `rounded-md`) with `--border-subtle`. Header `mb-2 flex items-start gap-2`; 32px tile with `Palette` in `var(--accent)`; `h3.text-xs.font-semibold` `Theme`; `p.mt-0.5.text-xs.leading-snug` in `--text-muted`:
  `Preview is live. ` + `<span className="font-semibold">Apply</span>` + ` saves it.`
- **States**:
  - `themeTab: 'general' | 'mesh' | 'cut'`, default `general` (`:142`).
  - `isBuiltInThemePreset` switches the entire action row: built-in → `Import` + `Reset`; custom → `Save`, `Rename`, `Export`, `Import` + `Reset`, `Delete`.
  - `isThemeResetDirty` → enables `Reset`; `isCustomThemeDirty` → enables `Save`. Both are computed in `SettingsModal.tsx:1188-1207` and passed down.
  - `pendingPickerColors: Partial<Record<keyof ThemeCustomColors, string>>` (`:141`) — a per-key draft for the native colour picker.
- **Data**: props `themeProfiles: ThemeProfile[]`, `themePreset`, `onThemePresetChange`, `themePreference`, `onThemePreferenceChange`, `themeColors: ThemeCustomColors`, `onThemeColorChange(key, value)`, `isBuiltInThemePreset`, `isCustomThemeDirty`, `isThemeResetDirty`, `onCreateCustomThemeFromPreset`, `onRequestSaveCustomTheme`, `onRequestRenameCustomTheme`, `onRequestDeleteCustomTheme`, `onExportTheme`, `onImportTheme(file?)`, `onResetThemeColors`.
  Derived: `builtInProfiles = themeProfiles.filter(p => p.isBuiltIn)`, `customProfiles = themeProfiles.filter(p => !p.isBuiltIn)`;
  `themePresetOptions = [...builtIns.map(p => ({ value: p.id, label: p.name, rightContent: 'Built-in' })), ...customs.map(p => ({ value: p.id, label: p.name, rightContent: 'Custom' }))]`.
- **Text**: `Theme`, `Preview is live. Apply saves it.`, `Theme preset`, `Color scheme`, `Dark`, `Light`, `Built-in`, `Custom`, `+ New Theme`, `Import`, `Save`, `Rename`, `Export`, `Reset`, `Delete`, plus the button tooltips quoted below and the section/row copy in the sub-surfaces.
- **Port target**: `gpui/src/features/settings/ui_theme.rs:5` (`ui_tab`), `:319` (`theme_sub_tab`), `:351` (`color_row`), `:411` (`color_field`), `:466` (`next_theme_color`), with the tables in `theme_colors.rs`.

### UI & Theme › Theme section (preset, scheme, actions)

- **Source**: `UISettingsTab.tsx:263-435`
- **Renders**: header, then `grid gap-2 md:grid-cols-2` with two `rounded-lg border p-2.5` cards on `var(--surface-2)`, then a hidden `<input type="file" accept=".json,application/json" className="hidden">`, then `mt-2 flex flex-wrap items-center justify-between gap-2` with the action buttons.
- **Preset card** (`:295`): label `mb-1 block text-xs font-semibold uppercase tracking-wide` in `--text-muted` `Theme preset`; then
  ```tsx
  <SelectDropdown<ThemePreset>
    value={themePreset}
    options={themePresetOptions}
    onChange={(nextPreset) => onThemePresetChange(nextPreset)}
    selectClassName="!h-8 text-xs !leading-none"
    menuClassName="max-w-[28rem]"
    menuFooterDivider
    menuFooterAction={{ label: '+ New Theme', onClick: onCreateCustomThemeFromPreset, tone: 'accent' }}
  />
  ```
- **Scheme card** (`:307`): same card shape with `opacity: isBuiltInThemePreset ? 0.5 : 1`; label `Color scheme`; then a plain `Select` `value={themePreference}` `disabled={isBuiltInThemePreset}` with `<option value="dark">Dark</option>` and `<option value="light">Light</option>` (not msg-wrapped).
- **Action rows** (`:333-430`):
  - Built-in: left `flex flex-wrap items-center gap-1.5` with `Import` (`variant="secondary" size="auto" className="!h-8 !px-2.5 !py-0 text-xs"`); right `ml-auto ...` with `Reset`, `disabled={!isThemeResetDirty}`, `style={isThemeResetDirty ? undefined : mutedActionStyle92}`, `title={isThemeResetDirty ? 'Reset current theme edits to selected preset values' : 'No theme changes to reset'}`.
  - Custom: left `Save` (`style={isCustomThemeDirty ? accentSecondaryActionStyle92 : mutedActionStyle92}`, `disabled={!isCustomThemeDirty}`, `title={isCustomThemeDirty ? 'Save current custom theme changes' : 'No unsaved custom theme changes'}`), `Rename`, `Export`, `Import`; right `Reset` (as above) then `Delete` with `style={dangerActionStyle92}`.
  - Style objects (`:130-143`):
    ```ts
    const accentSecondaryActionStyle92 = {
      color: 'var(--accent-secondary-action-color)',
      borderColor: 'var(--accent-secondary-action-border)',
      background: 'var(--accent-secondary-action-bg-92)',
    };
    const dangerActionStyle92 = {
      color: 'var(--danger)',
      borderColor: 'color-mix(in srgb, var(--danger), var(--border-subtle) 40%)',
      background: 'color-mix(in srgb, var(--danger), var(--surface-1) 92%)',
    };
    const mutedActionStyle92 = {
      color: 'var(--text-muted)',
      borderColor: 'var(--border-subtle)',
      background: 'var(--surface-1)',
    };
    ```
- **Callbacks**:
  - Import: `handleImportTheme` (`:182-189`) = inside Tauri (`'__TAURI_INTERNALS__' in window`) call `onImportTheme()` (native picker); otherwise click the hidden file input. The file input's `onChange` clears `event.target.value = ''` before dispatching (`:172-177`). The parent's `handleImportTheme` (`SettingsModal.tsx:711-740`) parses the JSON, creates a custom profile, appends it to the draft list and switches the draft to it; on failure it `alert('Failed to import theme profile. ' + message)` unless the message contains `cancel`.
  - Save/Rename/Delete open `StructuredDialogModal`s owned by `SettingsModal` (`z-[72]` save, `z-[73]` delete, `z-[74]` create/rename). The create dialog adds: a Dark/Light `Base preset` segmented pair, and `Primary branding` / `Secondary branding` colour fields (`ColorSwatchInput className="h-8 w-9"` + text input `ui-input h-8 min-w-0 flex-1 text-xs`, placeholders `#ec2a77` / `#baf72e`); the confirm button is disabled while the name is empty.
  - `onThemePresetChange` → `handleThemePresetChange` (`SettingsModal.tsx:629-631`) → `setThemeDraftFromProfile(preset, draftThemeProfiles)` (`:526-533`): replaces `draftThemePreset`, `draftThemePreference`, `draftThemeColors`, `draftCustomThemeName` from the chosen profile at once.
  - `onResetThemeColors` → `handleResetThemeColors` (`:641-647`): `getThemeProfile(draftThemePreset, draftThemeProfiles)` and restores that profile's `colors`, `preference`, and name.
- **Port target**: `ui_theme.rs:5-175`. Existing deviations: preset select lists built-ins only and writes `state.theme_preset`; scheme select is always rendered at `opacity(0.5)` and disabled; `Import` renders disabled at `opacity(0.5)`; there is no Save/Rename/Export/Delete; `Reset` writes `gpui_store::ThemeColors::default()` instead of the selected profile's colours.

### UI & Theme › sub-tab strip

- **Source**: `UISettingsTab.tsx:428-449`
- **Renders**: `flex items-center gap-1.5 border-b pb-2` with `borderColor: 'var(--border-subtle)'`; three buttons: `General UI`, `Mesh Highlights`, `Cutting Tool` (plain string titles in the source's `as const` table).
- Button: `className="ui-button ui-button-secondary !h-7 !px-2.5 !py-0 text-xs rounded-md"`, active style `{ color: 'var(--accent-secondary)', borderColor: 'color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 42%)' }`, inactive `{ color: 'var(--text-muted)' }`, `onClick={() => setThemeTab(tab.id)}`.
- **Port target**: `ui_theme.rs:178-230` (`theme_sub_tab`; active border `mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.58)`, hover `SURFACE_2`).

### UI & Theme › General UI panel

- **Source**: `UISettingsTab.tsx:451-478`
- **Renders**: `<div style={{ display: themeTab === 'general' ? undefined : 'none' }}>` → `grid gap-2.5 xl:grid-cols-2` of six `section.rounded-xl.border.p-2.5` cards on `--surface-1`.
- Card: `mb-2` block with `h4.text-[12px].font-semibold` in `--text-strong` and `p.mt-0.5.text-[11px].leading-snug` in `--text-muted`; then `space-y-1.5` of colour rows.
- **Colour row** (`renderColorField`, `:221-258`):
  ```tsx
  <div key={row.key} className="rounded-md border px-2 py-1.5" title={row.description}
       style={{ borderColor: 'var(--border-subtle)', background: 'color-mix(in srgb, var(--surface-0), transparent 8%)' }}>
    <div className="grid grid-cols-[minmax(0,1fr)_10.75rem] items-center gap-2.5">
      <div className="min-w-0"><label className="block truncate text-xs font-semibold" style={{ color: 'var(--text-strong)' }}>{row.label}</label></div>
      <div className="flex min-w-0 items-center gap-1.5">
        <ColorSwatchInput value={getDisplayThemeColor(row.key)}
                          onChange={(next) => handleThemeColorPickerDraftChange(row.key, next)}
                          onBlur={() => commitThemeColorPickerChange(row.key)}
                          className="h-7 w-8" />
        <input type="text" value={getDisplayThemeColor(row.key)}
               onChange={(event) => { clearPendingThemeColorPicker(row.key); onThemeColorChange(row.key, event.target.value); }}
               className="ui-input h-7 min-w-0 flex-1 text-xs" placeholder={row.placeholder} />
      </div>
    </div>
  </div>
  ```
  - `getDisplayThemeColor(key)` = `pendingPickerColors[key] ?? themeColors[key]` (`:217-219`).
  - `handleThemeColorPickerDraftChange` writes the pending map (live swatch feedback, no store write); `commitThemeColorPickerChange` (`:206-215`) compares against `themeColors[key]` and calls `onThemeColorChange` on blur, then clears the pending entry.
- **Sections and rows** (`THEME_COLOR_SECTIONS`, `:24-86`), in order (label · description · placeholder):
  1. `Foundation surfaces` — `Core app backgrounds and panel surfaces.`
     `background` · `App background` · `Outer app frame and page background.` · `#0b0f14`
     `foreground` · `App foreground` · `Top-level body text fallback color.` · `#e6ebf2`
     `surface0` · `Surface 0` · `Main modal and panel base surface.` · `#111216`
     `surface1` · `Surface 1` · `Raised cards, tool panes, and section fills.` · `#1a1b21`
     `surface2` · `Surface 2` · `Secondary tiles and inset controls.` · `#23252e`
  2. `Content contrast` — `Typography, borders, and neutral UI signals.`
     `textStrong` · `Text strong` · `Primary headings and high-contrast labels.` · `#f8f8fb`
     `textMuted` · `Text muted` · `Supporting labels, hints, and metadata.` · `#c3c7cf`
     `indicator` · `Indicator` · `Neutral dots, markers, and status indicators.` · `#c3c7cf`
     `borderSubtle` · `Border subtle` · `Low-contrast panel and input outlines.` · `#272a33`
     `borderStrong` · `Border strong` · `Higher-contrast structural dividers.` · `#353944`
  3. `Primary brand accent` — `Primary action styling, highlights, and key brand colors.`
     `accent` · `Accent` · `Primary highlight, active icons, and focus color.` · `#ec2a77`
     `accentHover` · `Accent hover` · `Hover/pressed state for primary accent actions.` · `#d81d67`
     `primaryButtonSurface` · `Primary button surface` · `Filled primary buttons and pills.` · `#c11f61`
     `accentContrast` · `Accent contrast` · `Text/icons shown on primary accent fills.` · `#fff6ff`
     `topbarAccent` · `Top bar accent` · `Glow and accent wash used by the app bar.` · `#ec2a77`
  4. `Secondary brand accent` — `Secondary action styling, complementary highlights, and secondary brand colors.`
     `accentSecondary` · `Accent secondary` · `Secondary accent and approved-action color.` · `#baf72e`
     `accentSecondaryHover` · `Secondary hover` · `Hover/pressed state for green actions.` · `#a6df29`
     `secondaryButtonSurface` · `Secondary button surface` · `Filled secondary buttons and badges.` · `#9bcc26`
     `accentSecondaryContrast` · `Secondary contrast` · `Text/icons shown on green fills.` · `#182106`
  5. `Scene accents` — `3D view gradient treatment and scene chrome.`
     `sceneGradientRadial` · `3D radial glow` · `Radial color bloom in the scene backdrop.` · `#ff37aa`
     `sceneGradientLinearStart` · `3D gradient top` · `Top stop of the linear scene gradient.` · `#ff37aa`
     `sceneGradientLinearMid` · `3D gradient mid` · `Middle stop of the linear scene gradient.` · `#6f33ff`
  6. `Semantic status colors` — `Feedback colors for success and destructive actions.`
     `success` · `Success` · `Success states, confirmations, and healthy signals.` · `#2eb67d`
     `danger` · `Danger` · `Destructive actions, warnings, and errors.` · `#e45454`
- **Port target**: `theme_colors.rs:25-85` holds exactly these tables; `ui_theme.rs:232-290` renders the grid. Deviation: `color_row` prints `field.description` as a visible `text-[11px]` line under each row instead of using it as a `title` tooltip, and uses `grid_cols(2)` rather than the `10.75rem` second column.

### UI & Theme › Mesh Highlights panel

- **Source**: `UISettingsTab.tsx:481-498`
- **Renders**: one `section.rounded-xl.border.p-2.5` on `--surface-1`; title `h4.text-[12px].font-semibold` `Mesh Highlights`; description `p.mt-0.5.text-[11px].leading-snug`:
  `Selection and hover tint for models in the 3D view. Also editable from the Mesh tab, which mirrors these two theme colors.`
  Then `grid gap-1.5 md:grid-cols-2` of the two colour rows.
- Fields (`MESH_HIGHLIGHT_COLOR_FIELDS`, `:95-98`, with the source comment *"Mesh tint, on their own sub-tab: they are theme entries (so presets can set them individually) rather than part of the core UI token set."*):
  `meshSelectionColor` · `Selection color` · `Tint applied to selected models.` · `#ec2a77`
  `meshHoverColor` · `Hover color` · `Tint applied to hovered models.` · `#ec2a77`
- **Port target**: `theme_colors.rs:87-90`, rendered at `ui_theme.rs:292-330`.

### UI & Theme › Cutting Tool panel

- **Source**: `UISettingsTab.tsx:500-502` → `OrganicCutColorsSection`
- **Port target**: `ui_theme.rs:332-370` renders `CUT_COLOR_FIELDS` inline (16 rows) rather than mounting the organic-cut section; values come from `state.cut_colors`. **Note:** the source writes organic-cut colours straight to their own localStorage key on every change (`saveOrganicCutColors`), i.e. this panel bypasses the settings draft/Apply entirely.

## Surface: OrganicCutColorsSection — Cut tool colour panel

- **Source**: `src/features/organicCut/OrganicCutColorsSection.tsx:34`
- **Renders**: one `section.rounded-xl border p-2.5` on `--surface-1` with `h4` `Cut Tool Colors`, description `Seam, cut surface, tenon, mortise, and waypoint colors for the Cut tool.`, then `grid grid-cols-2 gap-1.5` of 16 colour rows.
- **Layout & styling**: identical row shape to `renderColorField` above (`rounded-md border px-2 py-1.5`, `title={_(hint)}`, `grid grid-cols-[minmax(0,1fr)_10.75rem] items-center gap-2.5`, `ColorSwatchInput className="h-7 w-8"`, text input `ui-input h-7 min-w-0 flex-1 text-[11px] font-mono` with `placeholder={DEFAULT_ORGANIC_CUT_COLORS[key]}`).
- **Fields** (`FIELDS`, in order — key · label · hint · default):
  `seam` · `Seam` · `The cut line you draw.` · `#00008b`
  `seamHover` · `Seam (hover)` · `The seam while the cursor is over it.` · `#c9752e`
  `seamInactive` · `Seam (other loops)` · `Loops of a multi-loop cut you are not editing.` · `#5e3418`
  `seamGlow` · `Seam glow` · `Halo around the hovered seam.` · `#ffe6cc`
  `cutSurface` · `Cut surface` · `The contour membrane and the flat cut plane.` · `#37ff7a`
  `tenonFront` · `Tenon (near faces)` · `The tenon's faces turned toward you.` · `#001e57`
  `tenonBack` · `Tenon (far faces)` · `Far faces, darker so the shape reads solid.` · `#8a4a08`
  `tenonEdge` · `Tenon edges` · `The tenon's silhouette lines.` · `#ff7a00`
  `mortiseFront` · `Mortise (near faces)` · `The hole carved in the other half.` · `#c07bff`
  `mortiseBack` · `Mortise (far faces)` · `Far faces, darker so the shape reads solid.` · `#4a2a7a`
  `mortiseEdge` · `Mortise edges` · `The mortise's silhouette lines.` · `#a24bff`
  `tenonHandle` · `Tenon handle` · `The dot you drag to slide the tenon.` · `#0091ff`
  `markerFirst` · `First waypoint` · `The point the loop starts from.` · `#37ff7a`
  `markerPoint` · `Waypoint` · `Every other point on the loop.` · `#ffd24a`
  `markerSelected` · `Waypoint (selected)` · `The point you clicked.` · `#0091ff`
  `markerDragging` · `Waypoint (dragging)` · `The point being dragged.` · `#35e3ff`
- **Data**: `useOrganicCutColors()` (subscribed snapshot); `set(key, value)` → `saveOrganicCutColors({ ...colors, [key]: value })` — immediate persistence, normalised to `#rrggbb` lowercase with the per-key default as fallback.
- **Callbacks**: every swatch and text input writes through `set` on each change.
- **Port target**: `theme_colors.rs` `CUT_COLOR_FIELDS` + the `ui_theme.rs` inline grid; `state.cut_colors` is draft state that no store group owns (`state.rs:913-922`).

---

## Dependencies on other areas (do not re-spec here)

- The four tabs use the shared atoms `SettingRow`, `SegmentedControl`, `Select`/`SelectDropdown`, `ColorSwatchInput`, `Button`, `Input` — their exact CSS lives in `src/components/atoms/*` and `src/app/globals.css` (`.ui-input`, `.ui-select`, `.ui-button*`, `.ui-segmented-option`, `.ui-checkbox`, `.ui-range`). The port's equivalents are `gpui/crates/gpui-ui/src/ui/widgets/*` plus the `widgets.rs` recipes (`section` `:337`, `section_header` `:349`, `icon_chip` `:381`, `inner_card` `:392`, `field_card` `:404`, `segmented` `:458`, `slider_field` `:642`).
- `Tooltip` is not used inside these four tab bodies (the sidebar description and About tab are the users).
- Theme colours reach the rest of the app through `--*` CSS variables set by `applyThemeCustomColors` (`themeCustomizations.ts:640-763`) and, for the viewport, through `subscribeToThemeMeshHighlightColors`.
- The Slicing tab's `showPngCompressionControls` depends on `features/slicing/formats/registry.ts` and the active printer profile passed from `TopBar.tsx:1226`.

---

## Deliberate decisions and gotchas

1. **`mix(a, b, w)` is `a*w + b*(1-w)`; CSS `color-mix(in srgb, A, B N%)` is `A*(1-N) + B*N`.** Every `color-mix` in this scope must be inverted when written in Rust. Cross-check each: `mix(BORDER_SUBTLE, ACCENT, 0.8)` = border 80% + accent 20% ✔; `mix(SURFACE_1, ACCENT, 0.96)` = surface 96% + accent 4% ✔; `mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.58)` = accent-secondary 58% + border 42% ✔ (`ui_theme.rs:335` vs source `color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 42%)`).
2. **Lightness/Contrast are not independent sliders.** They are a re-parameterisation of two stored intensities: `lightness = clamp(totalLight, 0, 4)`, `contrast = directional/total`. Moving one recomputes both intensities from the *other's current value*. Getting the order wrong (reading the new value of both) makes the pair drift.
3. **`hasRenderingOptions` includes `matcap` and `flat_unlit`, which the Shader Type dropdown does not offer.** `configuredShaderType` is whatever the viewport's camera dropdown selected (`MESH_SHADER_OPTIONS` omits two of the eight types in `MESH_SHADER_TYPES`). The source's `<Select value="matcap">` with no matching `<option>` renders blank while the rendering options still appear. The port keeps that value in `SettingsState::unlisted_shader` and clears it only on a real dropdown pick (`mesh.rs:60-77`) — preserve that.
4. **`showBakedAo` is `soft_clay` only**, per the source comment: *"The baked occlusion only reaches the model through the Standard material."* Do not extend it to `xray` even though `xray` shares `showLighting`/`showRoughness`.
5. **Min/Max Overhang are clamped against each other on write**, not on render: `Math.min(next, heatmapMaxAngle)` / `Math.max(next, heatmapMinAngle)`. The two stored values can therefore never cross, and a drag on Min stops dead at Max.
6. **The Mesh tab's preview panes deliberately ignore user tint strengths in one place**: the shader preview is hard-coded `hoverTintStrength={0.5}` / `selectedTintStrength={0.7}`, while the selection preview uses `meshSelectionColor` + the live `selectedTintStrength` and `meshHoverColor` + the live `hoverTintStrength`. The colour side is live; the shader preview's tint is not.
7. **The selection preview's badge and card are a three-state machine** (`selected` > `hovered` > `idle`), driven by `onHoverChange` from mesh pointer events and `onCanvasPress` from `onPointerMissed`. Clicking the model toggles selected; clicking empty canvas clears it (does not toggle).
8. **The preview card is absolutely positioned on purpose** (source comment quoted in the surface): `absolute inset-0` inside `relative min-w-0`. Replacing it with a flex child changes the left column's height.
9. **The Mesh reset button restores the *selected theme profile's* colours, not `DEFAULT_THEME_CUSTOM_COLORS`**: `defaultMeshSelectionColor`/`defaultMeshHoverColor` come from `getThemeProfile(draftThemePreset, draftThemeProfiles).colors`. The port's `mesh.rs` reset writes `ThemeColors::default()` — wrong for any non-default preset.
10. **The theme hex text input commits per keystroke while the swatch commits on blur.** The text input clears the pending picker entry and calls `onThemeColorChange` immediately; `ColorSwatchInput` writes `pendingPickerColors` and only commits on `onBlur` (and only if the value actually differs). Without the pending map, dragging a colour picker fights the controlled input.
11. **Every theme edit is applied to the document immediately and rolled back on close.** `applyThemePreference` sets `data-theme`; `applyThemeCustomColors` writes 23 CSS variables on `document.documentElement.style`. Cancel/backdrop/Escape re-applies the saved theme unless `didCommitThemeDraftRef` was set by Apply. Because the Mesh tab edits `meshSelectionColor`/`meshHoverColor` through the same draft, **mesh selection colours are also previewed live and rolled back**.
12. **`--accent-secondary-action-*` are theme-aware in dark mode but literal in light mode.** `:root` defines them as `color-mix(… var(--accent-secondary) …)`, so a custom secondary accent propagates; `:root[data-theme='light']` and the `prefers-color-scheme` block hard-code `#4f8a08` / `#6aa20d` mixes (`globals.css:36-39, 84-105, 140-147`). `SettingsModal` therefore computes its own light-theme variants for the footer (`isLightTheme` from `data-theme` plus a `MutationObserver`, `:1116-1157`), while `UISettingsTab` uses the CSS variables directly.
13. **`applyThemeCustomColors` normalises every value and derives three fallbacks**: `accentHover`/`primaryButtonSurface` fall back to `darkenHex(accent, 0.82)`, `accentSecondaryHover` to `darkenHex(secondary, 0.9)`, `secondaryButtonSurface` to `darkenHex(secondary, 0.84)`; `topbarAccent` falls back to the accent. It does **not** set `--accent-secondary-action-*` (the cascade computes them from `--accent-secondary`) and it sets no `--mesh-*` variable — the viewport gets those from the subscription.
14. **Preset switch is a four-field atomic write.** `setThemeDraftFromProfile` replaces preset, preference, colours *and* the custom name together; resetting only colours would leave a light preset with dark neutrals.
15. **`Reset` (UI tab) resets to the selected profile, and its dirty check is profile-relative**: `isThemeDraftDirty` compares `profile.preference` and each of the 26 colour keys against the draft. `isCustomThemeDirty` is the same comparison but returns `false` for any built-in preset.
16. **Button `size="auto"` emits no geometry** — every one of these buttons carries its own `!h-8 !px-2.5 !py-0 text-xs` (or `!h-6 !w-6 !p-0` for the mesh reset icon button). Do not substitute the default `md` shape.
17. **The sub-tabs are hidden, not unmounted**: all three panels stay in the tree with `display: none`. Any measurement or subscription inside `OrganicCutColorsSection` keeps running while hidden.
18. **The preview-model manifest is empty** (`public/mesh-preview-models/models.json` is `{"models": []}`), so the only reachable preview shapes are `Cube`, `Sphere`, `Knot`, and the default is `knot`. The STL path (`stl:/mesh-preview-models/<file>`) and its normalisation to unit size (`1.5 / maxDim`) exist but are unreachable until that file is populated.
19. **`opaque_wire_mesh` in the preview forces `shaderType={'soft_clay'}` on the base mesh** and draws the wire overlay as a second mesh with `renderOrder={1}` — the two-mesh group is what hides back edges.
20. **The preview camera is Z-up and looks from `-Y`**: `camera.up.set(0,0,1)`, `position (0, -distance, 0)`, `fov 35`, `dpr [1,2]`; `distance = 7.2` for the knot and `5.6` otherwise. `OrbitControls` auto-rotates at `0.6` with `dampingFactor 0.08`, pan and zoom disabled.
21. **Light model of the preview**: ambient + one key directional at `z=+12` + a fill at `z=-12` at `0.15×` + hemisphere `('#ffffff','#444444', ambient*0.6)` + a camera-attached point light of `intensity 1.0`, `decay 0`, `distance 0`. The headlight is what keeps the model visible when `ambientIntensity` is 0.
22. **PNG compression is a three-way UI over five stored strategies.** `'smallest'` and `'optimal'` render as `On` and are silently rewritten to `'balanced'` if the user re-picks On. `normalizeSlicingPerformanceSettings` accepts all five.
23. **`showPngCompressionControls` defaults to showing the section** when there is no active output format or the format is unrecognised by the built-in registry.
24. **The engine version is `'-'` until the native command answers**, and it never answers outside Tauri (`// Null until the native side answers; outside Tauri it never does.`). The `dragonfruit-slicing-engine` crate name is a literal (`SLICING_ENGINE_CRATE`), not msg-wrapped.
25. **Cleanup buttons use `alert`/`confirm`; "Clean All" is destructive and gated by a confirm with the exact text** `Delete ALL temporary slice files? This cannot be undone.` The stale variant clamps to 60s minimum *inside the bridge* (`Math.max(60, Math.floor(age))`).
26. **`navlibActive` removes three whole blocks**, not just disables them: the pivot card, the Motion Tuning section and the Axis Inversion section. Hiding only the sliders would leave the pivot select and six checkboxes that do nothing.
27. **`blockedDeviceIds` is the only 3D-mouse setting written from the device list**, and it is replaced (not mutated): `filter(...)` or `[...ids, id]`. Every change goes through `normalizeSpaceMouseSettings`, which drops non-string entries.
28. **The 3D mouse device list polls every 800ms** in addition to the connect/disconnect events; candidates require `axes.length >= 6`. `NAMED_3D_MOUSE` decides "recognized"; `KNOWN_NON_3D_MOUSE` (in the controller, not the tab) decides which fallback device the camera actually uses.
29. **Axis inversion labels are `Invert X pan`/`Invert Y pan`/`Invert zoom`/`Invert pitch`/`Invert yaw`/`Invert roll`, not "translate/rotate X/Y/Z"** — keyed `invertTx…invertRz` with the defaults `invertTy` and `invertRy` **true** and everything else false (`spacemousePreferences.ts:34-45`).
30. **Default sensitivities are `2.3` and deadzone `0.03`, but the sliders' ranges are `0.1–4` and `0–0.3`**; normalisation then clamps, and its `?? 1` / `?? 0.08` fallbacks only apply to fields absent after the defaults merge (the merge order is `DEFAULT_SPACEMOUSE_SETTINGS` first, then the partial).
31. **Organic cut colours bypass the settings draft.** `OrganicCutColorsSection` reads/writes its own localStorage key (`organic-cut-colors`) and dispatches its own event immediately; Apply/Cancel in the settings modal do not affect them.
32. **Theme preset ids are slugs (`dragonfruit-dark`, `concepts-3d`, …) while labels are names (`DragonFruit Dark`, `Concepts 3D`, …)**; custom ids are `custom:<slug>-<base36 time>-<random>`, and names are de-duplicated with a ` 2`, ` 3` suffix (`themeCustomizations.ts:330-375`).
33. **Preset colour tables are not derivable from the CSS variables.** `DRAGONFRUIT_LIGHT_THEME_COLORS`, `CONCEPTS_3D_THEME_COLORS` and `ATLAS_3DSS_THEME_COLORS` are full 26-key literals with their own rationale comments (light `success` darkened to `#146b46`; Atlas teal chrome at 40% lightness; Concepts' deeper blue model highlight `#487dd5`). Copy them verbatim; do not recompute.
34. **`--secondary-button-surface` differs between the CSS default (`#bdf435`) and `DEFAULT_THEME_CUSTOM_COLORS` (`#9bcc26`)** — the theme write wins as soon as any theme is applied.
35. **`select` values are strings even for enum-ish settings**; the source's `Select` synthesises a React change event over `SelectDropdown` (`atoms/Select.tsx`), so a port can treat every dropdown as "id → value".

---

## Open questions

1. **Does the GPUI port keep the draft/Apply model at all?** Today `Apply` and `Cancel` both just close the modal and the draft is persisted debounced (`mod.rs:1-30`). That makes "Preview is live. Apply saves it." false and removes the rollback behaviour (gotcha 11). Either the shell gains a real draft/commit split or the UI copy must change.
2. **Where does the shader preview render?** The source embeds a second three.js canvas with its own camera, lights and R3F event handling. The port has no multi-viewport path in the settings modal; options are a small offscreen wgpu pass reusing `gpui-core/viewport`, or dropping the 3D pane and keeping the colour block. Interaction (`onPointerMissed` → deselect) only exists with a real canvas.
3. **Colour pickers.** The source uses `react-colorful` (Mesh tab), native `input[type=color]` (swatch), and free-text hex fields. GPUI has none of these; the port currently cycles a palette on click (`next_theme_color`) and renders text fields read-only. A real hex/picker control is unbuilt.
4. **Theme import/export and custom theme profiles** need a file dialog, JSON schema validation (`kind`, `formatVersion`, `theme`), name de-duplication, and four confirm dialogs. None exist in the port; `Import` is rendered disabled.
5. **`get_slicer_engine_version`** — is the backend command ported? If yes, the version field can stop showing `-`; if not, leave it and document.
6. **Temp-file cleanup** needs `cleanup_stale_print_temp_files` / `cleanup_all_print_temp_files` plus a confirm dialog and a result surface; GPUI has no `alert`. Which feedback widget replaces it (`dialog`? the toast in `widgets/notifications.rs`?) is undecided.
7. **Gamepad input for the 3D mouse.** The tab depends on `navigator.getGamepads()`; a native GPUI build has no such API. If the port keeps the device list, it needs a platform 3D-mouse backend (and `blockedDeviceIds` then controls that backend, not Web Gamepad ids).
8. **Organic cut colours have no store group** (`state.rs:913-922` says so). Promote them to `gpui-store` (like the handoff's note for auto-support) or keep a settings-local map persisted under the settings document?
9. **The palette-cycle stand-in is in one shared place** (`ui_theme.rs:466` `next_theme_color`, used by both theme rows and the Mesh selection/hover fields). Decide whether that behaviour survives the colour picker or should be deleted when the picker lands.
10. **Is the light theme in scope?** The tab offers a `Dark`/`Light` scheme switch, but the port's palette is dark-only (`about.rs:72-73` notes the same for drop shadows). Two schemes mean a second full token table plus the light-specific `--accent-secondary-action-*` variants.
