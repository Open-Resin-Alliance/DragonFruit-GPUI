# Printing mode — preview pane, layer scrubber, print controls

Printing mode divides the window: the 3D viewport shrinks to `w-1/2` on the
left (`page.tsx:11024`), and this pane owns the right half as an absolutely
positioned, self-contained overlay. Nothing else in the window moves to make
room for it. The pane's only interactive control is the layer rail; its job is
to show one sliced layer, fast while the thumb moves and sharp when it stops.

## Components in scope

| Component | File |
|---|---|
| `PrintingPreviewPane` | `src/components/organisms/PrintingPreviewPane.tsx` |
| `LayerSlider` (compact minimal rail branch only) | `src/components/controls/LayerSlider.tsx` |
| `PrintingLayerGpuPreview` | `src/components/controls/PrintingLayerGpuPreview.tsx` |
| `CrossSectionStencilCap` (rendered inside the GPU preview) | `src/components/scene/CrossSectionStencilCap.tsx` |
| `Tooltip` (wraps the rail) | `src/components/ui/Tooltip.tsx` |
| `PrintingPanelStack` | `src/components/organisms/panels/PrintingPanelStack.tsx` |
| `PrintingPanel` (print controls / status) | `src/features/printing/components/PrintingPanel.tsx` |
| `usePrintingPreviewManager` (all pane behaviour) | `src/features/printing/usePrintingPreviewManager.ts` |

The pane itself is 272 lines and derives nothing: every value it shows arrives
as a prop from `usePrintingPreviewManager` or from `page.tsx`. **A port that
renders the pane without porting the manager's state machine will look right
at layer 1 and be wrong everywhere else.** The manager is therefore specified
here in full.

---

## Surface: PrintingPreviewPane — the printing-mode right column

- **Source**: `src/components/organisms/PrintingPreviewPane.tsx:95`
- **Renders**: one CSS grid, `56px` rail column + `1fr` content column. Content
  column is a flex column: heading, progress line, preview box.
- **Layout & styling** (`:95-98`):
  ```tsx
  className="absolute right-0 w-1/2 min-w-0 grid overflow-hidden"
  style={{ top: 'var(--topbar-height)', bottom: 0,
           gridTemplateColumns: '56px minmax(0, 1fr)', background: 'var(--surface-0)' }}
  ```
  Mounted only when `scene.mode === 'printing'` (`page.tsx:11407`), as a sibling
  of the scene column inside `.absolute inset-0 z-0 flex`.
- **States**: the pane has no branch of its own; it always renders. Every
  visual state lives in its children (rail position, preview layers, empty box).
- **Data**: its props, all of them traceable to `usePrintingPreviewManager` or
  `page.tsx`. See "Props contract" below.
- **Callbacks**: none of its own. Every event handler is a prop passed to the
  preview box or to `LayerSlider`.
- **Text**: see the heading and progress surfaces below.
- **Port target**: `gpui/src/features/printing_preview.rs::printing_preview`
  exists. Integration point: `gpui/src/app.rs:1815`
  (`.when(mode == Mode::Printing && has_models, |this| this.child(features::printing_preview::printing_preview(...)))`).
  The current port renders the rail via
  `gpui/src/chrome/overlays.rs::layer_slider` and a single static image via
  `PrintingPreviewState::preview_image`.

### Props contract (`PrintingPreviewPaneProps`, `:18-63`)

| Prop | Type / source | What it is |
|---|---|---|
| `printingPreviewTotalLayers` | `activeSlice?.totalLayers ?? 0` (`page.tsx:1660`) | layer count; `0` = plate not sliced |
| `printingSelectedLayer` | manager state | the committed layer |
| `handlePrintingLayerChange` | manager | slider `onChange` |
| `handlePrintingLayerScrubStart` / `…End` | manager | slider `onScrubStart` / `onScrubEnd` |
| `printingCurrentHeightMm` | `page.tsx:7195-7202` memo | cut height; `null` outside printing or with 0 layers |
| `slicingHeightMm` | `slicing.heightMm` | the sliced build height (mm) |
| `printingPreviewViewportRef` | manager ref | the preview box element; the hi-res canvas sizes from its `getBoundingClientRect()` |
| `printingPreviewCursor` | manager memo | `'default' \| 'zoom-in' \| 'grab' \| 'grabbing'` |
| `handlePrintingPreviewWheel` / `PointerDown` / `PointerMove` / `PointerEnd` | manager | zoom / pan |
| `printingPreviewTargetResolution` | manager memo | `{widthPx,heightPx,viewportWidth,viewportHeight}` or `null` |
| `activePrinterProfile` | profiles store | build volume, display mirror, `materialMode` |
| `printingPreviewVisualTransform` | manager memo | CSS transform string or `undefined` |
| `models` | `printingScrubPreviewModels` (`page.tsx:3220`) | `LoadedModel[]` of the active bed, moved to the origin |
| `supportDragGroupRef`, `supportRenderRefreshNonce` | `page.tsx:1244`, `:2162` | three.js support group + a nonce bumped when supports re-render |
| `printingPreviewScrubUpscaleTransform` | manager memo | `scale(2)` while scrubbing a loaded PNG, else `undefined` |
| `printingPreviewPngUrlForDisplay` | manager memo | object URL to draw |
| `isPrintingPngLoaded` | manager memo | that URL has been decoded |
| `selectedPrintingLayerPreviewUrl` | manager memo | URL for the *displayed* layer (may be `null` while loading) |
| `usePrintingSettledHiResCanvas` | manager memo | zoom > 1 and not scrubbing |
| `printingPreviewCanvasRef` | manager ref | the settled hi-res canvas |
| `isPrintingSettledCanvasReady` | manager state | canvas has been drawn |

### `printingCurrentHeightMm` (`page.tsx:7195-7202`)

```ts
if (scene.mode !== 'printing') return null;
if (printingPreviewTotalLayers <= 0) return null;
const clampedLayer = Math.max(1, Math.min(Math.max(1, printingPreviewTotalLayers), printingSelectedLayer));
const height = clampedLayer * crossSectionLayerHeightMm;
return Math.min(Math.max(height, 0), Math.max(slicing.heightMm, 0));
```
`crossSectionLayerHeightMm` = `slicedLayerHeightMm` (`page.tsx:3725`), the
layer height of the slice actually on disk. This same number is handed to
`SceneCanvas` as `indicatorPlaneZ` (`page.tsx:11326`) — the pink plane in the
3D view and the GPU cap in this pane must show the *same* height.

### `printingScrubPreviewModels` (`page.tsx:3220-3238`)

The models of the active bed, translated so that bed sits at the origin, because
the slice was taken that way:

```ts
const activeFrame = scene.plateFrames.find((frame) => frame.id === scene.activePlateId);
const onPlate = modelsOnPlate(scene.models, scene.activePlateId, scene.plateFrames);
if (!activeFrame || (activeFrame.dxMm === 0 && activeFrame.dyMm === 0)) return onPlate;
return onPlate.map((model) => ({ ...model,
  transform: { ...model.transform,
    position: model.transform.position.clone().add(new THREE.Vector3(-activeFrame.dxMm, -activeFrame.dyMm, 0)) } }));
```
Source comment: *"Handed the scene as it stands, it shows the bed that sits at
the origin — plate 1 — whatever bed the preview is really of, which is why
scrubbing showed the wrong model on plate 2."*

---

## Surface: rail column — holds the layer scrubber

- **Source**: `PrintingPreviewPane.tsx:99-121`
- **Renders**: a 56px-wide, full-height, right-bordered column containing one
  `LayerSlider`.
- **Layout & styling**:
  ```tsx
  className="relative z-20 h-full overflow-visible border-r px-0 pt-1.5 pb-0"
  style={{ borderColor: 'var(--border-subtle)',
           background: 'color-mix(in srgb, var(--surface-1), transparent 6%)' }}
  ```
  `overflow-visible` is load-bearing: the thumb edit popover and the floating
  value badges hang outside the track and must not be clipped.
- **States**: none.
- **Data**: `value` = `Math.max(1, Math.min(Math.max(1, total), selected))`.
- **Callbacks**: delegated to the slider.
- **Port target**: `printing_preview.rs` (rail) — constants
  `RAIL_COLUMN_WIDTH = 56.0`, `RAIL_PADDING_TOP = 6.0`, `SURFACE_1_94`.

### LayerSlider call site (`PrintingPreviewPane.tsx:103-121`)

```tsx
<LayerSlider
  min={1}
  max={Math.max(1, printingPreviewTotalLayers)}
  step={1}
  value={Math.max(1, Math.min(Math.max(1, printingPreviewTotalLayers), printingSelectedLayer))}
  onChange={handlePrintingLayerChange}
  onScrubStart={handlePrintingLayerScrubStart}
  onScrubEnd={handlePrintingLayerScrubEnd}
  allowTrackClickJump
  currentHeightMm={printingCurrentHeightMm ?? undefined}
  maxHeightMm={slicingHeightMm}
  showValue={true}
  compactMinimalRail
  dragBatchMode="raf"
  docked
  embedded
  expandToContainer
  className="mx-auto h-full"
/>
```

There is **no** `lowerValue`, **no** `onLowerChange`, **no**
`crossSectionEnabled`, **no** `onToggleCrossSection`, **no** `layerHeightMm`.
Those absences are each observable — see the gotchas.

---

## Surface: LayerSlider — the compact minimal rail

`LayerSlider` is a shared component with three presentation branches. The pane
passes `docked embedded compactMinimalRail expandToContainer showValue`, so
`isMinimalRail = embedded && docked === true`,
`isCompactMinimalRail = isMinimalRail && compactMinimalRail === true`, and
**every non-minimal branch is dead code at this call site**. Only the minimal
branch is specified below.

- **Source**: `src/components/controls/LayerSlider.tsx:63` (signature),
  `:688` (render root)
- **Renders**: `[max]` badge · rail *[track, accent progress fill, 9×24 thumb
  with value badge]* · `[min]` badge — stacked vertically, full height.
- **Layout & styling**

  Outer wrapper (`:689-695`), `docked` branch:
  ```
  className={`relative z-10 select-none ${className ?? ''}`}   // className="mx-auto h-full"
  ```
  Inner column (`:702-711`), `embedded && isCompactMinimalRail`:
  ```
  `${expandToContainer ? 'h-full min-h-0 flex flex-col' : ''} mx-auto w-[34px] pt-1 pb-0`
  ```
  `style={embedded ? undefined : {...}}` — embedded carries no style.

  Top badge row (`:739-745`): `flex items-center justify-center mb-1`, badge
  class `minimalRailBadgeClass` = `inline-flex items-center rounded-md border px-0.5 py-0 text-[8px] font-semibold tabular-nums`,
  style `railBadgeStyle` = `{ color: 'var(--text-muted)',
  borderColor: 'color-mix(in srgb, var(--border-subtle), transparent 15%)',
  background: 'color-mix(in srgb, var(--surface-1), transparent 10%)' }`.
  Content: `{max}`.

  Rail (`:686`, computed once and used for both the tooltip wrapper and the
  interactive div):
  ```
  `relative mx-auto ${embedded ? (expandToContainer ? 'flex-1 h-full min-h-[300px]' : 'h-[46vh]') : 'h-[56vh]'} ${embedded ? (isMinimalRail ? (isCompactMinimalRail ? 'w-4' : 'w-5') : 'w-7') : 'w-10'} cursor-pointer`
  ```
  → at this call site: `relative mx-auto flex-1 h-full min-h-[300px] w-4 cursor-pointer`.

  Track (`:785-792`): `absolute left-1/2 top-0 h-full w-1.5 -translate-x-1/2 rounded-full`,
  style `{ background: 'color-mix(in srgb, var(--surface-2), black 8%)',
  border: '1px solid color-mix(in srgb, var(--border-subtle), transparent 40%)' }`.

  Progress fill (`:794-812`): `absolute left-1/2 -translate-x-1/2 w-1.5 rounded-full`,
  style
  ```ts
  bottom: `${lowerPercent ?? 0}%`,
  height: `${percent - (lowerPercent ?? 0)}%`,
  transition: easesPosition
    ? 'bottom 170ms cubic-bezier(0.22, 1, 0.36, 1), height 170ms cubic-bezier(0.22, 1, 0.36, 1), background 200ms, box-shadow 200ms'
    : 'background 200ms, box-shadow 200ms',
  background: crossSectionEnabled
    ? 'linear-gradient(180deg, color-mix(in srgb, var(--accent), white 14%), var(--accent))'
    : 'linear-gradient(180deg, color-mix(in srgb, var(--text-muted), white 10%), var(--text-muted))',
  boxShadow: crossSectionEnabled ? '0 0 10px color-mix(in srgb, var(--accent), transparent 65%)' : 'none',
  opacity: crossSectionEnabled ? 1 : 0.45,
  ```
  `lowerPercent` is `null` here (no lower thumb): fill runs from `0%` to
  `percent%`.

  Upper thumb (`:912-…`): container `absolute left-1/2 -translate-x-1/2 -translate-y-1/2`,
  `top: ${100 - percent}%`, `transition: easesPosition ? 'top 170ms cubic-bezier(0.22, 1, 0.36, 1)' : 'none'`.
  Pill: `h-[9px] w-[24px] rounded-full border ${isDraggingThumb ? 'scale-105' : 'scale-100'} transition-[transform,background,box-shadow] duration-150`,
  style
  ```ts
  borderColor: `color-mix(in srgb, white, ${thumbColor} 20%)`,
  background: `linear-gradient(90deg, color-mix(in srgb, ${thumbColor}, white 20%), ${thumbColor}, color-mix(in srgb, ${thumbColor}, white 20%))`,
  boxShadow: isDraggingThumb
    ? `0 0 0 2px color-mix(in srgb, ${thumbColor}, transparent 65%), 0 6px 14px rgba(0,0,0,0.38)`
    : `0 0 0 2px color-mix(in srgb, ${thumbColor}, transparent 80%), 0 4px 10px rgba(0,0,0,0.35)`,
  ```
  `thumbColor = crossSectionEnabled ? 'var(--accent)' : 'var(--text-muted)'`
  → `var(--accent)` here (the pane never passes `crossSectionEnabled`, so its
  default `true` applies).

  Bottom badge row (`:1058-1066`): `mt-1 flex items-center justify-center gap-1.5`
  + the same `minimalRailBadgeClass` / `railBadgeStyle`, content `{min}`.

- **States**
  | State | Condition | Visible effect |
  |---|---|---|
  | rest | — | pill `scale-100`, `box-shadow` 80%-transparent ring |
  | dragging | `isDraggingThumb` | pill `scale-105`, tighter/brighter ring, 38% shadow |
  | shift held | `isShiftHeld` | four blue arcs (`#3b82f6`, strokeWidth 2.5, 52×52 svg, `drop-shadow-lg`) centred on the thumb |
  | value badge hidden at the ends | `percent <= 1e-6 \|\| percent >= 1 - 1e-6` | no floating layer-number badge |
  | value badge below the thumb | `percent >= 96` | badge at `top-3` instead of `-top-5` |
  | gliding | `glide.eased && !isDraggingThumb` | `bottom`/`height`/`top` animate at 170ms instead of snapping |
  | press near the thumb | within press thresholds | thumb drag |
  | press far from the thumb | `allowTrackClickJump` | jumps to the pressed height and drags from there |

  Shift arcs (`:1008-1049`), all `fill="none" stroke="#3b82f6" strokeWidth="2.5" strokeLinecap="round"`,
  outer radius 24 (`M 9.5 14.5 A 24 24 0 0 0 2 26` / `M 9.5 37.5 A 24 24 0 0 1 2 26`),
  inner radius 17 (`M 14.0 17.0 A 17 17 0 0 0 9 26` / `M 14.0 35.0 A 17 17 0 0 1 9 26`).

- **Data**: `value`, `min`, `max`, `step`; internal `liveValue` (what the
  slider sent during a live scrub), `isShiftHeld`, `isDraggingThumb`,
  `editingThumb`, `editMode`, `editRawValue`.

  Derived (`:637-686`):
  ```ts
  const displayValue = liveValue ?? value;
  const displayHeightMm = liveValue != null && typeof layerHeightMm === 'number'
    ? Math.min(Math.max(displayValue * layerHeightMm, 0),
               typeof maxHeightMm === 'number' ? maxHeightMm : Number.POSITIVE_INFINITY)
    : currentHeightMm;
  const percent = Math.min(100, Math.max(0, ((displayValue - min) / Math.max(1, (max - min))) * 100));
  const hideUpperFloatingBadge = percent <= EDGE_BADGE_HIDE_EPS || percent >= (100 - EDGE_BADGE_HIDE_EPS);  // 1e-6
  const shouldPlaceCurrentBadgeBelowThumb = isMinimalRail && percent >= 96;
  ```
  Note `percent` divides by `Math.max(1, max - min)`, not by the span.

- **Callbacks**
  | Element | Handler | Observable result |
  |---|---|---|
  | rail `onMouseDown` (left) | `onPointerDown` (`:312`) | `scrubActiveRef = true`; `onScrubStart?.()` → manager sets `isPrintingLayerScrubbing` + restarts the 180ms settle timer; window `mousemove`/`mouseup`/`blur` listeners; on `mouseup`/`blur` → `onScrubEnd?.()` |
  | rail `onMouseDown` (right) | same handler, `button === 2` branch | if the press is within the grab band of the thumb (±6px) → open the thumb editor; `preventDefault` + `stopPropagation` |
  | rail `onDoubleClick` | `if (!onToggleCrossSection) return;` (`:755`) | **no-op at this call site** — the pane passes no `onToggleCrossSection` |
  | rail `onContextMenu` | `e.preventDefault(); e.stopPropagation();` | suppresses the browser menu |
  | rail `onKeyDown` | `onKeyDown` (`:563`) | `ArrowUp`/`ArrowRight` → `onChange(value + step)`; `ArrowDown`/`ArrowLeft` → `onChange(value - step)`; both `preventDefault()` |
  | container `wheel` | native listener, `{ passive: false }` (`:545`) | `preventDefault()`; `dir = e.deltaY > 0 ? -1 : 1`; shift → `0.1 * step`, else `step` |
  | floating badge | — | `pointer-events-none`; the thumb itself is inside the rail's hit area |

- **Text**: `{max}` (top badge), `{min}` (bottom badge), the layer number
  (`Math.round(displayValue)`) in the floating badge. No i18n on the badges —
  they are numbers. The heading `msgid "Layer"` (`:715`) renders only in the
  non-minimal branch, so **it never appears in printing mode**.

- **Port target**: `gpui/src/chrome/overlays.rs::layer_slider` (exists;
  `:485`), called from `printing_preview.rs` as
  `overlays::layer_slider(selected, None, state.cross_section, 1.0, max_layer as f32, 1.0, entity)`.
  Constants already derived there: `THUMB_CORE_HALF = 4.5`,
  `THUMB_GRAB_HALF = 6.0`, `THUMB_HEIGHT = 9.0`, `THUMB_WIDTH = 24.0`,
  `TRACK_WIDTH = 6.0`, `RAIL_COLUMN_WIDTH = 34.0`, `RAIL_WIDTH = 16.0`,
  `RAIL_MIN_HEIGHT = 300.0`, `BADGE_PX = 2.0`, `BADGE_TEXT = 8.0`.

### Drag semantics (`:312-533`)

- Thumb choice, single-thumb case (this call site):
  ```ts
  if (Math.abs(clickInv - upperPos) > thumbGrabThreshold) {
    if (allowTrackClickJump) setByClientY(e.clientY, false);   // jump + drag
    else { /* ignore press, onScrubEnd() */ return; }
  }
  activeDraggingThumbRef.current = 'upper';
  ```
  `getThumbHitThresholds` (`:242`): `coreThreshold = (9 / 2) * (1 / max(1, railHeightPx))`,
  `grabThreshold = ((9 / 2) + 1.5) * (1 / max(1, railHeightPx))`.
- `setByClientY` (`:203`): `inv = 1 - (clientY - rect.top) / rect.height`, so
  bottom = `min`, top = `max`. With shift: `newPercent = currentPercent + (inv - currentPercent) * 0.1`
  — a tenth of the way toward the pointer, not a tenth-size step.
- `emitChange` (`:118`): `next = Math.max(clamp(snap(rawNext)), floor)` where
  `snap` rounds to `step`; returns early if unchanged. `scrubActiveRef` makes it
  also write `liveValue`.
- Shift latch (`:465`): once shift is pressed during a drag,
  `dragShiftModeRef` stays true until `mouseup`; releasing shift does not
  restore coarse mode mid-drag.
- Batch mode `'raf'` (`:441-462`): moves store `pendingClientY` and coalesce into
  one `requestAnimationFrame` flush per frame. `'immediate'` is the other branch
  and does not apply here.
- `settleDrag` (`:490-512`): idempotent; cancels the pending rAF, clears
  `isDraggingThumb` / shift / `scrubActiveRef`, clears `liveValue`, calls
  `onScrubEnd?.()`, removes the three window listeners.

### Glide (`:625-635`)

```ts
const [glide, setGlide] = React.useState({ value, lowerValue, eased: false });
if (glide.value !== value || glide.lowerValue !== lowerValue) {
  const near = 2 * (step || 1);
  const eased = Math.abs(value - glide.value) > near
    || Math.abs((lowerValue ?? 0) - (glide.lowerValue ?? 0)) > near;
  setGlide({ value, lowerValue, eased });
}
const easesPosition = glide.eased && !isDraggingThumb;
```
Source comment: *"Only a jump glides. A wheel or arrow step lands where it is:
eased, every step of a scroll restarts the glide, and the thumb trails the
layer the viewport already shows."* With `step = 1` a change of 2 or more
frames animates; ±1 snaps.

---

## Surface: rail tooltip — cursor-following, minimal mode only

- **Source**: `LayerSlider.tsx:747-750`
  ```tsx
  <Tooltip
    content={isMinimalRail ? <span className="whitespace-pre-line">{minimalRailTitle}</span> : undefined}
    wrapperClassName={railClassName}
  >
  ```
- **Renders**: a `Tooltip`-wrapped rail. `wrapperClassName` is the same string
  as the rail, so the wrapper span is `inline-flex relative mx-auto flex-1 h-full min-h-[300px] w-4 cursor-pointer`
  and the rail div repeats it.
- **Layout & styling** (`Tooltip.tsx`): popover is
  `fixed pointer-events-none z-[9999] rounded px-2 py-1.5 text-[11px] leading-tight font-medium shadow-lg`,
  inline `background: 'rgba(24, 24, 24, 0.98)'`,
  `color: 'var(--text-strong, #e0e0e0)'`,
  `border: '1px solid var(--accent, #baf72e)'`,
  `width: width ?? 'max-content'`, `maxWidth: 260`,
  `whiteSpace: 'normal'`, `textAlign: 'left'`,
  `boxShadow: '0 6px 32px 0 rgba(0,0,0,0.44), 0 1.5px 8px 0 rgba(0,0,0,0.28)'`.
  Portalled to `<body>`; placed in a `useLayoutEffect` and `visibility: hidden`
  until placed.
- **States**
  - hidden until `mouseenter` or focus (`delayMs` defaults to 0 → immediate).
  - `anchor` defaults to `'cursor'`, `offsetY = 28`: box centred under the
    pointer (`x = cursor.x - width/2`, `y = cursor.y + 28`), flipping above when
    it would overflow the bottom; clamped to the viewport with a 4px margin, and
    nudged to `top >= 4`.
  - follows the pointer while hovered (`handleMouseMove`).
  - hidden on `mouseleave` / `blur`.
- **Data**: `minimalRailTitle` (`:681-685`)
  ```ts
  const mmLabel = typeof displayHeightMm === 'number' ? formatMm(displayHeightMm) : '—';
  const crossSection = onToggleCrossSection ? (crossSectionEnabled ? 'on' : 'off') : 'none';
  return formatMinimalRailTitle(_, Math.round(displayValue), mmLabel, crossSection);
  ```
  At this call site `onToggleCrossSection` is undefined → `'none'`, so the
  no-double-click variant is always the one shown. `formatMm` = `fmt.unit('mm', fmt.number(mm, { maximumFractionDigits: 2 }))`,
  falling back to `fmt.unit('mm', '0')` for a non-finite value.
- **Callbacks**: none.
- **Text** (`LayerSlider.tsx:20-31`, module scope — React Compiler would rename
  interpolated locals):
  - `msgid "Layer {layer} • {height} • Right-click a handle to type an exact value"` (used here)
  - `msgid "Layer {layer} • {height} • Right-click a handle to type an exact value • Double-click to turn the cross-section off"`
  - `msgid "Layer {layer} • {height} • Right-click a handle to type an exact value • Double-click to turn the cross-section back on"`
  - `{height}` renders through `UNIT_MESSAGES.mm` = `msgid "{value} mm"`, so the
    full text reads e.g. `Layer 12 • 3.6 mm • Right-click a handle to type an exact value`.
- **Port target**: `overlays::layer_slider` draws a trigger-anchored box,
  right-aligned. The documented deviation (`overlays.rs:24-56`) is that the
  source follows the cursor; a faithful port needs a pointer-following overlay
  placed from the last mouse position.

---

## Surface: thumb edit popover — right-click on the thumb

- **Source**: `LayerSlider.tsx:312-345` (open) and `:965-1007` (render)
- **Renders**: a small anchored column: optional mode toggle row, then a text
  input. Opens on right-click within the grab band of a thumb.
- **Layout & styling** (`:675-680`, `:968-980`):
  ```
  'absolute left-full top-1/2 -translate-y-1/2 z-[200] flex flex-col gap-1.5 rounded-lg border p-2'   // popover side 'right'
  'absolute right-full top-1/2 -translate-y-1/2 z-[200] flex flex-col gap-1.5 rounded-lg border p-2'  // popover side 'left'
  ```
  plus `marginLeft: '10px'` / `marginRight: '10px'`, and inline
  `{ minWidth: '136px', background: 'var(--surface-0)',
     borderColor: 'var(--border-subtle)', boxShadow: '0 8px 24px rgba(0,0,0,0.45)' }`.
  Side is chosen by `clampToViewport({ x: rect.right + 10, y: rect.top }, { width: THUMB_EDIT_POPOVER_WIDTH /* 156 */, height: 0 })`:
  `overflowRight ? 'left' : 'right'`.
  Mode buttons (`:836-855`): `flex-1 rounded px-1.5 py-0.5 text-[10px] font-medium transition-colors`;
  selected → `{ background: 'var(--secondary-button-surface)', color: 'var(--accent-secondary-contrast)' }`,
  else `{ background: 'color-mix(in srgb, var(--surface-2), transparent 20%)', color: 'var(--text-muted)' }`.
  Input: `w-full rounded border px-1.5 py-1 text-center text-[11px] tabular-nums focus:outline-none selection:bg-transparent selection:text-[var(--accent)]`,
  style `{ borderColor: 'var(--border-subtle)',
  background: 'color-mix(in srgb, var(--surface-1), transparent 10%)',
  color: 'var(--text-strong)' }`.
- **States**: in this pane `layerHeightMm` is `undefined`, so the `Layer #` /
  `mm` toggle row is **not rendered** (`{layerHeightMm && layerHeightMm > 0 && … }`)
  and `switchEditMode` refuses to switch. `editMode` stays `'layer'`, the input
  placeholder is `msgid "Layer #"`, and typed values are read as layer numbers.
- **Data**: `editRawValue` (string), `editingThumb`.
- **Callbacks**
  | Element | Handler | Result |
  |---|---|---|
  | input `keydown` Enter | `commitThumbEdit()` + `preventDefault`, `stopPropagation` | parses `fmt.parse(editRawValue)`; if finite and `>= 0`, `Math.round(parsed)` → `emitChange` (or `emitLowerChange`) |
  | input `keydown` Escape | `setEditingThumb(null)` | closes without committing |
  | input `blur` | `commitThumbEdit()` | same commit |
  | outside pointer down | `useOutsideDismiss(editingThumb !== null, commitThumbEdit, { ignoreRef: editPopoverRef })` | commits |
  | Escape / resize / scroll | same hook | dismisses (commits) |
  | popover `mousedown` / `pointerdown` | `stopPropagation` | does not start a rail drag |
- **Port target**: **PORT GAP** in `overlays.rs` (documented at
  `overlays.rs:26-30`): the popover's open/mode/text state has no home in the
  stateless builder. Fixing it needs slider-editor fields on `DragonFruit`.

---

## Surface: preview box — the layer image viewport

- **Source**: `PrintingPreviewPane.tsx:132-267`
- **Renders**: a rounded bordered box, `p-2`, centred content, that emits
  wheel/pointer events and hosts (a) the aspect box with GPU + PNG + empty
  state, and (b) the settled hi-res canvas.
- **Layout & styling** (`:133-147`):
  ```tsx
  className="relative flex-1 min-h-0 min-w-0 rounded-lg border p-2 flex items-center justify-center overflow-hidden"
  style={{
    borderColor: 'var(--border-subtle)',
    background: 'color-mix(in srgb, var(--surface-1), transparent 6%)',
    cursor: printingPreviewCursor,
    touchAction: 'none',
    // The layer box below sizes itself from this one's content box.
    containerType: 'size',
  }}
  ```
  `ref={printingPreviewViewportRef}` — the settled canvas and the pan clamps
  measure *this* element. `containerType: 'size'` is what makes the child's
  `100cqw` / `100cqh` resolve to this box's content size.
- **States**: the box itself has none; the cursor is its only stateful style
  (`default` when there is nothing to look at, `zoom-in` at 1×, `grab`/`grabbing`
  when zoomed).
- **Callbacks**
  | Element | Handler | Observable result |
  |---|---|---|
  | `onWheel` | `handlePrintingPreviewWheel` | zoom, 1…32, anchored on the pointer; restarts the settle timer |
  | `onPointerDown` | `handlePrintingPreviewPointerDown` | only when zoomed in and left button: pointer capture, start pan drag, `isPrintingPreviewPanning = true` |
  | `onPointerMove` | `handlePrintingPreviewPointerMove` | while captured: pan = drag origin + delta, clamped |
  | `onPointerUp` / `onPointerCancel` | `handlePrintingPreviewPointerEnd` | releases capture, ends pan, restarts settle |

### Zoom and pan maths (`usePrintingPreviewManager.ts:444-536`)

```ts
if (printingPreviewTotalLayers <= 0) return;
event.preventDefault();
const previousZoom = printingPreviewZoomRef.current;
if (previousZoom <= 1.0001 && event.deltaY > 0) return;         // no zoom-out below 1
const factor = Math.exp(-event.deltaY * 0.0015);
const nextZoom = Math.max(1, Math.min(32, previousZoom * factor));
if (Math.abs(nextZoom - previousZoom) < 1e-5) return;
schedulePrintingPreviewSettle();
// pointer-anchored zoom, in the viewport's centre-relative space:
const pointerX = event.clientX - (viewportRect.left + viewportRect.width * 0.5);
const contentX = (pointerX - previousPan.x) / Math.max(1e-4, previousZoom);
const nextPan  = nextZoom <= 1.0001 ? { x: 0, y: 0 }
  : { x: pointerX - (contentX * nextZoom), y: pointerY - (contentY * nextZoom) };
```
`clampPrintingPreviewPan` (`:240`): at `zoom <= 1.0001` → `{0,0}`; otherwise
`maxPanX = ((zoom - 1) * width) * 0.5`, symmetric for Y, and the pan is clamped
to `±maxPanX` / `±maxPanY`.
`queuePrintingPreviewPan` (`:224`) coalesces onto one rAF and skips writes whose
delta is `< 0.05px`.

---

## Surface: aspect box — the fitted layer rectangle

- **Source**: `PrintingPreviewPane.tsx:150-171`
- **Renders**: a `position: relative` div with the plate/display aspect ratio,
  fitted inside the preview box, carrying the pan/zoom/de-mirror transform.
- **Layout & styling**:
  ```tsx
  className="block rounded relative"
  style={{
    aspectRatio: aspectRatio.toString(),
    // Fitted to whichever side runs out first. A width of 100% capped by
    // max-height keeps the full width and drops the ratio, which stretched the
    // GPU layer while the PNG over it stayed letterboxed at the right shape.
    width: `min(100cqw, 100cqh * ${aspectRatio})`,
    transform: printingPreviewVisualTransform || 'none',
    transformOrigin: 'center center',
    willChange: 'transform',
  }}
  ```
  Ratio source (`:151-157`):
  ```ts
  const aspectW = printingPreviewTargetResolution ? printingPreviewTargetResolution.viewportWidth
                 : activePrinterProfile?.buildVolumeMm?.width ?? 143;
  const aspectH = printingPreviewTargetResolution ? printingPreviewTargetResolution.viewportHeight
                 : activePrinterProfile?.buildVolumeMm?.depth ?? 89;
  const aspectRatio = aspectW / aspectH;
  ```
- **States**: always rendered when the pane renders.
- **Data**: `printingPreviewVisualTransform` (`usePrintingPreviewManager.ts:569-590`)
  is the concatenation, **in this order**, of:
  1. `translate(${pan.x}px, ${pan.y}px)` — only if `|x| > 0.01 || |y| > 0.01`
  2. `scale(${zoom})` — only if `|zoom - 1| > 1e-4`
  3. `printingPreviewDeMirrorTransform` = `scale(±1, ±1)` from
     `activePrinterProfile.display.mirrorX/mirrorY` (absent when both are false)
  4. `printingPreviewScrubUpscaleTransform` = `scale(2)` when
     `printingPreviewScrubQualityScale = 0.5` (i.e. scrubbing a loaded PNG at 1×)

  Order matters: de-mirror is applied *inside* the zoom, so a mirrored display
  zooms around its own centre and still reads left-to-right at 1×.
- **Port target**: `printing_preview.rs` — the aspect box cannot be laid out in
  GPUI (no `100cqw`/`100cqh`, no `min()`), so the current port paints the empty
  state's dashed border at the fitted rect instead (`paint_aspect_border`).

---

## Surface: GPU scrub preview — three.js stencil cap

- **Source**: `PrintingPreviewPane.tsx:174-201` (wrapper) ·
  `src/components/controls/PrintingLayerGpuPreview.tsx:66` (component)
- **Renders**: an `absolute inset-0` div (opacity 1, `pointer-events: none`)
  holding a react-three-fiber `<Canvas>` that draws only the **cross-section
  silhouette** at the clip height, one stencil cap per material colour.
- **Layout & styling**
  - wrapper (`:175-181`): `className="absolute inset-0 transition-opacity duration-100"`,
    `style={{ opacity: 1, pointerEvents: 'none' }}`.
  - preview (`PrintingLayerGpuPreview.tsx:170-175`, `:236`):
    `className="block w-full h-full rounded"`,
    `style={{ transform: printingPreviewScrubUpscaleTransform || 'none',
              transformOrigin: 'center center', willChange: 'transform' }}`.
  - the component's own root div: `{ position: 'relative', overflow: 'hidden', background: '#000', ...className/style }`.
  - Canvas: `frameloop="always"`, `dpr={[1, 1]}`, `orthographic`,
    `gl={{ antialias: false, stencil: true, alpha: false, powerPreference: 'high-performance' }}`,
    `onCreated={({ gl }) => { gl.localClippingEnabled = true; }}`,
    style `{ width: '100%', height: '100%', transform: `scale(${mirrorX ? -1 : 1}, ${mirrorY ? -1 : 1})` }`.
  - Camera: `position: [0, 0, 1000]`, `zoom: 1`, `near: 0.1`, `far: 2000`,
    `left/right/top/bottom = ±orthoSize.width/2 / ±orthoSize.height/2`, where
    `orthoSize` is the fitted frustum below.
- **States**
  - `clipZ == null` → no Canvas at all; the root div renders as a plain black
    box (`:170-176`). This is the "0 layers / not printing" state.
  - `clipZ != null` → Canvas with one `CrossSectionStencilCap` per colour group.
  - `multiMaterialMode` changes grouping only (see below), not the box.
- **Data**
  - `capGroups` (`:76-147`): single-material → one group
    `{ color: '#ffffff', entries: models.filter(m => m.visible).map(...) }`.
    Multi-tank → one group per colour; for a model with `splitBodies` each body
    is its own entry keyed `${model.id}:body:${index}` and coloured
    `bodyColors[index] || materialColorById.get(bodyMaterialProfileIds[index]) || model.color || '#ffffff'`;
    without `splitBodies`, the whole model is one entry coloured
    `model.color || '#ffffff'`. Resin colours come from
    `getProfileStoreSnapshot().materialProfiles`.
    Source comment: *"The saved colour array can be missing (a scene kept under a
    single-material printer before assignments became non-destructive); the
    body's material id still names the resin, so resolve it from the profile
    store."*
  - `frustumSize` (`:150-186`): base = `max(1, buildPlateWidthMm)` ×
    `max(1, buildPlateDepthMm)`; `viewportAspect = viewportWidthMm / viewportHeightMm`
    when both are positive, else `baseWidth / baseHeight`. If
    `safeViewportAspect >= baseAspect` → `{ width: baseHeight * aspect, height: baseHeight }`,
    else `{ width: baseWidth, height: baseWidth / aspect }`. So the plate fits
    inside the viewport with the viewport's aspect, never stretching the plate.
  - `planeWidthMm/planeHeightMm` = the frustum size.
  - `capEntryCount` feeds `DemandInvalidateOnChange` (unused in practice —
    `frameloop="always"`).
- **Callbacks**: none from the user. `DemandInvalidateOnChange` calls
  `invalidate()` whenever `clipZ`, `supportVersion`, `entryCount`, `mirrorX`,
  `mirrorY`, `viewportWidthMm` or `viewportHeightMm` change.
- **Text**: none.
- **Port target**: `printing_preview.rs` — **omitted by design**: three.js
  cannot be hosted in GPUI. See "Port deviations" at the end for what a
  replacement must reproduce.

### `CrossSectionStencilCap` (inside the Canvas, `PrintingLayerGpuPreview.tsx:205-232`)

Props passed at this call site: `entries`, `sourceObject={supportGroupRef?.current ?? null}`,
`sourceObjectVersion={supportVersion}`, `y={clipZ}`, `color={group.color}`,
`renderOrderOffset={groupIndex * 10}`, `planeWidthMm`, `planeHeightMm`,
`visible={true}`, `key={group.color}`.

It builds a stencil pass over the scene geometry, clips it at the plane `y`,
and draws a flat cap at that height (a true slice silhouette, not the geometry
below the plane). `renderOrderOffset` exists for a specific reason — quote it:

> *"Every group's stencil pass, cap and stencil clear must finish before the
> next group starts: with one shared render order the groups' stencil passes
> merge and the first cap paints all of their silhouettes."*

`sourceObject`/`sourceObjectVersion` make the cap re-collect the support group
whenever `supportRenderRefreshNonce` changes; supports are not part of `models`
and would otherwise be missing from the silhouette.

---

## Surface: PNG layer — the higher-quality image on top

- **Source**: `PrintingPreviewPane.tsx:204-237`
- **Renders**: an `absolute inset-0` div whose opacity follows the decode state;
  inside it either an `<svg>` (when the printer display geometry is known) or a
  plain `<img>`.
- **Layout & styling**
  ```tsx
  <div className="absolute inset-0 transition-opacity duration-150"
       style={{ opacity: isPrintingPngLoaded ? 1 : 0 }}>
  ```
  SVG path (`:211-227`):
  ```tsx
  <svg viewBox={`0 0 ${viewportWidth} ${viewportHeight}`}
       preserveAspectRatio="xMidYMid meet"
       className="block w-full h-full rounded"
       role="img" aria-label={formatLayerPreviewAlt(_, printingSelectedLayer)}>
    <image href={url} x={0} y={0} width={viewportWidth} height={viewportHeight}
           preserveAspectRatio="none" style={{ imageRendering: 'pixelated' }} />
  </svg>
  ```
  `<img>` path (`:228-235`): `className="block rounded w-full h-full object-contain"`,
  `style={{ imageRendering: 'pixelated' }}`, same `alt`.
  The SVG path exists so a 1-pixel-per-printer-pixel PNG lands exactly on the
  aspect box with no resampling; the box and the viewBox share a ratio, so
  `meet` is a no-op and `none` on the inner image prevents the browser from
  letterboxing the bitmap a second time.
- **States**
  - not rendered at all when `printingPreviewPngUrlForDisplay` is `null`.
  - `opacity 0` whenever `printingPngLoadedUrl` is not the selected URL.
    `isPrintingPngLoaded` (`:389-392`) returns `false` both for a *different*
    URL and for `null`, so this is the state during every handoff.
  - `opacity 1` once the image fired `load` **or** `error` (fail-open).
  - The GPU layer below is never unmounted; the PNG is simply painted over it,
    and the 150ms `transition-opacity` makes the handoff a fade rather than a cut.
- **Data**: `printingPreviewPngUrlForDisplay` =
  `selectedPrintingLayerPreviewUrl ?? printingPngLoadedUrl` (`:411-413`) — when
  the selected layer has no URL yet, the element keeps the last loaded one, so
  the node is never swapped for a blank one mid-fade.
- **Callbacks**: none.
- **Text**: `aria-label` = `msgid "Layer {layer} preview"`
  (`msg\`Layer ${layer} preview\``, `PrintingPreviewPane.tsx:57-59`), with the
  **selected** layer, not the displayed one.
- **Port target**: `printing_preview.rs` draws a single image with
  `object_fit(ObjectFit::Contain)`. The settle/decode layering is not modelled.

---

## Surface: empty state — dashed box with a message

- **Source**: `PrintingPreviewPane.tsx:239-252`
- **Condition**: `!selectedPrintingLayerPreviewUrl && printingPreviewTotalLayers === 0`
- **Renders**: a dashed rounded rectangle filling the aspect box, with one
  centred line of text.
- **Layout & styling**
  ```tsx
  className="absolute inset-0 rounded border border-dashed flex items-center justify-center text-xs"
  style={{
    borderColor: 'var(--border-subtle)',
    color: 'var(--text-muted)',
    // The box is flipped to undo a mirrored display; text has nothing to undo.
    transform: `scale(${mirrorX ? -1 : 1}, ${mirrorY ? -1 : 1})`,
  }}
  ```
- **States**: the only state is "no slice": the plate has zero layers and no
  preview URL. A plate with layers but no image yet shows **nothing** here (the
  GPU layer covers that case).
- **Callbacks**: none.
- **Text**: `msgid "No preview available yet."`
- **Port target**: `printing_preview.rs::paint_aspect_border` + a muted text
  child. The texture dash is 3px on / 3px off, `BORDER_SUBTLE`, 1px stroke.

---

## Surface: settled hi-res canvas — sharp re-draw when zoomed

- **Source**: `PrintingPreviewPane.tsx:257-266`
- **Condition**: `selectedPrintingLayerPreviewUrl && usePrintingSettledHiResCanvas`,
  i.e. a layer image exists, `printingPreviewZoom > 1.0001`, and no scrub is
  running.
- **Renders**: a `<canvas class="pointer-events-none absolute inset-0 block h-full w-full rounded transition-opacity duration-75">`
  over the whole preview box (not just the aspect box), with
  `imageRendering: 'pixelated'` and `opacity: isPrintingSettledCanvasReady ? 1 : 0`.
- **States**
  - not rendered when zoomed out or while scrubbing (the aspect box's CSS-scaled
    content is the visible layer).
  - `opacity 0` until the first successful draw.
  - `opacity 1` after `draw()` returns true; stays 1 for subsequent redraws
    (resize, pan, zoom) because `isPrintingSettledCanvasReady` is only cleared
    when the URL changes.
- **Data / drawing** (`usePrintingPreviewManager.ts:747-830`)
  ```ts
  const rect = viewport.getBoundingClientRect();
  const viewportWidth  = Math.max(1, Math.round(rect.width));
  const viewportHeight = Math.max(1, Math.round(rect.height));
  const dpr = Math.max(1, window.devicePixelRatio || 1);
  canvas.width  = Math.max(1, Math.round(viewportWidth  * dpr));
  canvas.height = Math.max(1, Math.round(viewportHeight * dpr));
  const logicalSourceWidth  = Math.max(1, targetViewportWidth  ?? image.naturalWidth);
  const logicalSourceHeight = Math.max(1, targetViewportHeight ?? image.naturalHeight);
  const baseScale = Math.min(viewportWidth / logicalSourceWidth, viewportHeight / logicalSourceHeight);
  const drawWidth  = logicalSourceWidth  * baseScale;
  const drawHeight = logicalSourceHeight * baseScale;
  ctx.setTransform(1,0,0,1,0,0); ctx.clearRect(0,0,canvas.width,canvas.height);
  ctx.scale(dpr, dpr); ctx.imageSmoothingEnabled = false;
  ctx.translate(viewportWidth * 0.5 + pan.x, viewportHeight * 0.5 + pan.y);
  ctx.scale(zoom, zoom);
  ctx.scale(mirrorScale.x, mirrorScale.y);
  ctx.drawImage(image, -drawWidth * 0.5, -drawHeight * 0.5, drawWidth, drawHeight);
  ```
  Redrawn on: `mirrorScale.x/y`, `pan.x/y`, `targetViewportWidth/Height`,
  `zoom`, `selectedPrintingLayerPreviewUrl`, `usePrintingSettledHiResCanvas`,
  and on every `ResizeObserver` callback for the viewport. Source comment:
  *"Drawn again whenever the pane changes size: the canvas keeps the pixel size
  it was given, and CSS stretches it to the new box until something redraws it."*
  A monotonically increasing `printingPreviewCanvasRenderNonceRef` discards
  stale `onload` results.
- **Callbacks**: none.
- **Port target**: nothing in `printing_preview.rs`. GPUI has no canvas here;
  the equivalent is drawing the image quad at the transformed rect with nearest
  sampling, which the port's viewport pipeline already does elsewhere.

---

## Surface: "Layer Preview" heading

- **Source**: `PrintingPreviewPane.tsx:125-127`
- **Layout & styling**: `text-xs font-semibold uppercase tracking-wide`,
  `color: var(--text-muted)`. `text-xs` = 12px/16px.
- **States**: none.
- **Text**: `msgid "Layer Preview"` (`<Trans>Layer Preview</Trans>`).
- **Port target**: `printing_preview.rs` — rendered as the literal
  `"LAYER PREVIEW"` because GPUI has no letter-spacing/uppercase; `HEADING_SIZE = 12.0`,
  `HEADING_LINE_HEIGHT = 16.0`. A real port needs the i18n msgid
  (`t!("Layer Preview")`), not a pre-uppercased literal.

## Surface: progress line — "Layer n/total"

- **Source**: `PrintingPreviewPane.tsx:128-130`
- **Renders**: one line of `text-[11px]` muted text.
- **Data**:
  ```ts
  formatLayerProgress(_, `${Math.max(1, Math.min(Math.max(1, total), selected))}/${Math.max(1, total)}`)
  ```
  `formatLayerProgress` (`:61-63`) is module scope:
  `translate(msg({ message: `Layer ${layers}`, comment: 'Print progress on a monitor card, e.g. "Layer 120/450".' }))`
  → `msgid "Layer {layers}"`.
- **States**: none — with 0 layers it reads `Layer 1/1`.
- **Text**: e.g. `Layer 120/450`.
- **Port target**: `printing_preview.rs` formats `format!("Layer {}/{}", selected.round(), max_layer)`
  with `PROGRESS_SIZE = 11.0`; the msgid needs to come from `gpui-locale`.

---

## Surface: PrintingPanel — the print controls and status card

- **Source**: `src/features/printing/components/PrintingPanel.tsx:36`; mounted by
  `PrintingPanelStack` (`src/components/organisms/panels/PrintingPanelStack.tsx:57`)
  inside `FloatingPanelStack` when `scene.mode === 'printing'` (`page.tsx:10929-10957`).
- **Renders**: `Card` (`ui-panel rounded-lg shadow-lg border`, `borderColor:
  var(--border-subtle)`) at `w-[22rem]`; header = collapse chevron + `Printing`
  title; body (when expanded) = printer/resin/estimate card, generated-file
  card, action column, optional status line.
- **Layout & styling**
  - Card: `className="w-[22rem]"`.
  - Header: `CardHeader` = `relative px-2.5 py-2.5 flex items-center`,
    `data-panel-drag-handle="true"`, left cluster `flex items-center gap-2.5 pr-8`;
    title `text-sm font-semibold`, `color: var(--text-strong)`.
  - Collapse chevron: `PanelCollapseToggle` (`IconButton`, `aria-expanded`,
    label `msgid "Collapse card"` / `msgid "Expand card"`), chevron
    `h-3 w-3`, colour `var(--accent)` expanded / `var(--text-muted)` collapsed.
  - Body: `className="px-3 pb-3 space-y-2.5"`.
  - Info cards: `rounded-md border p-2.5 space-y-1`,
    `style={{ borderColor: 'var(--border-subtle)', background: 'var(--surface-1)' }}`.
  - Labels: `text-xs`, `color: var(--text-muted)`; subsequent labels `mt-1`.
  - Values: `text-sm font-semibold`, `color: var(--text-strong)`; the generated
    file name is additionally `truncate` with `title={…}`.
  - Format/size line: `text-xs`, `color: var(--text-muted)`, content
    `{outputFormat ?? '—'} • {outputSizeLabel}`.
  - Action column: `grid grid-cols-1 gap-2`.
  - Buttons: `Button` atom → `ui-button` + variant class + `!h-9` etc.
    `variant="accent"` = `.ui-button-accent` (`background: var(--secondary-button-surface); color: var(--accent-secondary-contrast); border-color: color-mix(in srgb, var(--secondary-button-surface), white 14%)`, hover `color-mix(…, black 9%)`);
    `variant="secondary"` = `.ui-button-secondary` (`background: var(--surface-1); color: var(--text-strong); border-color: var(--border-subtle)`, hover `var(--surface-2)`).
    Base `.ui-button`: `border-radius: var(--radius-sm)`, `padding: 5px 11px 7px 11px`,
    `font-size: var(--font-size-md)`, `font-weight: 600`, `line-height: 1`,
    `transition: background-color 140ms ease, border-color 140ms ease, color 140ms ease`.
  - Saved-path row: `rounded-md border p-2.5 space-y-0.5`; path text
    `text-xs whitespace-nowrap overflow-hidden` with
    `fontFamily: 'var(--font-geist-mono)'` and a right-edge fade
    `maskImage/WebkitMaskImage: 'linear-gradient(to right, #000 0%, #000 calc(100% - 18px), transparent 100%)'`.
  - Reveal button: `inline-flex h-6 w-6 shrink-0 items-center justify-center rounded border transition-colors disabled:opacity-50`,
    `borderColor: 'var(--border-subtle)'`, `color: 'var(--text-muted)'`,
    `background: 'var(--surface-2)'`; shows `Spinner size="sm"` while working,
    else `FolderOpen` at `h-3.5 w-3.5`.
  - Status line: `text-xs rounded border px-2 py-1`,
    `style={{ borderColor: 'var(--border-subtle)', color: 'var(--text-muted)' }}`.
- **States**
  | State | Condition | Effect |
  |---|---|---|
  | collapsed | `isExpanded === false` (local `useFloatingPanelCollapse(true)`) | body not rendered |
  | fresh plate | `outputName === null` | file name `msgid "No generated file yet"`, format line `— • —` |
  | `sliceIntent !== 'file'` | — | accent "Export as …" button shown |
  | `sliceIntent === 'file'` | — | instead, the "Saved to" card with the path + reveal button |
  | `sliceIntent !== 'upload' && !== 'print'` | — | send row shown |
  | multiple targets | `showSendTargetPicker` (= `printableConnectedPrinterFleet.length > 1`) | send row gains the `ChevronDown` picker button |
  | sending | `sendBusy` | label becomes `msgid "Sending…"`; an extra cancel button appears (`X` icon; `RotateCcw` when not busy) |
  | UVTools | `canSendToUvTools && onSendToUvTools && savedFilePath` | "Send to UVTools" row |
  | reveal error | `revealSavedPathError !== null` | `text-[11px]`, `color: '#f87171'` |
  | status | `sendStatusText !== null` | status line |
  | disabled download | `canDownload === false` | accent button disabled, `title` = `msgid "Slice first to generate a print file"` |
  | disabled send | `!canSendToPrinter \|\| sendBusy` | send button disabled, `title` = `msgid "Requires connected printer with supported upload capability and a generated print file"` |
- **Data**: `outputName`, `outputFormat` (`.${ext}`, derived from `outputName`),
  `outputSizeLabel`, `printerName` (`activePrinterProfile?.name ?? 'No printer selected'`),
  `resinName` (`page.tsx:4122-4157`: `'Multi-Tank'` for a multi-tank printer,
  `'N/A'` for an offline remote material, the target's / profile's selected
  material name, else the local composite label, else `msgid "No resin selected"`),
  `estimatedPrintTimeLabel`, `estimatedVolumeLabel`, `canDownload`
  (`Boolean(printingArtifact)`), `canSendToPrinter`
  (`printingArtifact && activeNetworkUiAdapter && printableConnectedPrinterFleet.length > 0`),
  `sendBusy`, `sendStatusText`, `sendButtonLabel`
  (`formatUploadToLabel(_)` → `msgid "Upload to {printer}"`, printer name
  truncated to 24 chars + `…` when longer than 26; else `msgid "Send to Printer"`),
  `sliceIntent`, `savedFilePath`.
- **Callbacks**
  | Element | Handler | Observable result |
  |---|---|---|
  | collapse chevron | `setIsExpanded(prev => !prev)` | card collapses/expands |
  | Export button | `onDownload` → `handleDownloadPrintArtifact` (`page.tsx:4543`) | native save dialog on the artifact's temp path → byte fallback through the native save dialog → browser `<a download>` blob fallback |
  | Send button | `onSendToPrinter` → `handleSendToPrinter` (`page.tsx:5547`) | no fleet → status `msgid "No connected printer is available for upload."`; material mismatch → opens the post-slice target picker; else `performSendToPrinter` → upload dialog + progress + status text |
  | Cancel button | `onCancelSendToPrinter` → `page.tsx:5580` | sets the cancel flag, status/stage `msgid "Canceling upload…"`, and for the athena plugin calls `abortUpload()` |
  | `ChevronDown` | `onOpenSendTargetPicker` | `setPrintingTargetPickerMode('post-slice')` + opens the picker |
  | Reveal (folder) | `handleRevealSavedPath` | `invoke('reveal_in_file_manager', { path })`; on failure shows the error text |
  | Send to UVTools | `onSendToUvTools` (`PrintingPanelStack.tsx:78-84`) | `launchExternalProcess(resolveUvToolsExecutablePath(s), completedSaveDestinationPath)`; a rejection only logs `console.warn('[UVTools] Failed to launch from printing panel:', err)` |
- **Text** (verbatim, with msgid)
  | Rendered | msgid / note |
  |---|---|
  | `Printing` | `msgid "Printing"` + `msgctxt "Title of the panel with the sliced file and its print actions"` |
  | `Printer` | `msgid "Printer"` |
  | `Resin` | `msgid "Resin"` |
  | `Estimated print time` | `msgid "Estimated print time"` |
  | `Estimated volume` | `msgid "Estimated volume"` |
  | `Generated file` | `msgid "Generated file"` |
  | `No generated file yet` | `msgid "No generated file yet"` |
  | `Export as .ctb` | `msgid "Export as {format}"`, comment `'Button that saves the sliced file. {format} is its extension, e.g. ".ctb".'` |
  | `Export as file` | `msgid "Export as file"` (used when `outputFormat` is null) |
  | `Saved to` | `msgid "Saved to"` |
  | `Open file location` | `msgid "Open file location"` (title when a path exists) |
  | `Path unavailable` | `msgid "Path unavailable"` (title when it does not) |
  | `Open saved file location` | `msgid "Open saved file location"` (aria-label) |
  | `Send to Printer` | `msgid "Send to Printer"` |
  | `Sending…` | `msgid "Sending…"` |
  | `Cancel` | `msgid "Cancel"` |
  | `Cancel current upload` | `msgid "Cancel current upload"` (title/aria-label of the cancel button) |
  | `Choose upload target printer` | `msgid "Choose upload target printer"` (title + aria-label) |
  | `Send generated print file to selected printer` | title, picker variant |
  | `Send generated print file to connected printer` | title, plain variant |
  | `Requires connected printer with supported upload capability and a generated print file` | title when send is disabled |
  | `Download generated print file` | title, download enabled |
  | `Slice first to generate a print file` | title, download disabled |
  | `Send to UVTools` | `msgid "Send to UVTools"` |

  `estimatedPrintTimeLabel` (`page.tsx:3617-3638`):
  ```ts
  if (!activeMaterialProfile || printingPreviewTotalLayers <= 0) return '—';
  const bottomLayers = Math.max(0, Math.min(total, Math.round(activeMaterialProfile.bottomLayerCount)));
  const normalLayers = Math.max(0, total - bottomLayers);
  const liftSec    = liftSpeedMmMin > 0 ? (liftDistanceMm / liftSpeedMmMin) * 60 : 0;
  const retractSec = retractSpeedMmMin > 0 ? (liftDistanceMm / retractSpeedMmMin) * 60 : 0;
  const travelSecPerLayer = Math.max(0, liftSec + retractSec);
  const totalSec = bottomLayers * (bottomExposureSec + travelSecPerLayer)
                 + normalLayers * (normalExposureSec + travelSecPerLayer);
  return formatApproxPrintTimeLabel(_, totalSec);
  ```
  `formatApproxPrintTimeLabel` (`page.tsx:518-526`): hours > 0 →
  `msgid "~{hours} h {mins} min"`, else `msgid "~{mins} min"`.

- **Port target**: `gpui/src/panels/content.rs::printing_body` (exists, `:1214`)
  — currently a static empty-state: every value is the source's empty label and
  both actions are `.disabled()`. Comments there list what is missing
  (no slicer, printer or profile state; no send-target picker; no busy state;
  UVTools needs a saved path).

---

## Behaviour: the manager's state machine

This is the part a porter cannot infer from markup. Everything below is
`src/features/printing/usePrintingPreviewManager.ts`.

### Two layer numbers, deliberately

| State | Meaning | Who sees it |
|---|---|---|
| `printingSelectedLayer` | the committed scrub value | the slider's `value`, the progress line, `printingCurrentHeightMm` (and therefore the GPU cap's `clipZ`) |
| `printingDisplayedLayer` | the layer whose PNG is being shown | `selectedPrintingLayerPreviewUrl` |

During a scrub the two diverge: the slider keeps its own `liveValue`, the pane
keeps showing the **displayed** layer's image, while the GPU cap follows the
*selected* layer through `printingCurrentHeightMm`.

### `handlePrintingLayerChange` (`:655-696`)

```ts
if (!Number.isFinite(nextLayer)) return;
const clamped = clampPrintingLayer(nextLayer);   // round, then clamp to [1, max(1,total)]

if (isPrintingLayerScrubbing) {
  if ((pendingRef.current ?? selectedRef.current) === clamped) return;
  pendingRef.current = clamped;
  if (rafRef.current !== null) return;
  rafRef.current = requestAnimationFrame(() => {
    rafRef.current = null;
    flushPendingLayer({ syncDisplayedLayer: false });   // updates selected only
  });
  return;
}
// not scrubbing: cancel any pending rAF, clear it, then set both, synchronously.
```

### `handlePrintingLayerScrubStart` (`:698`) / `…ScrubEnd` (`:703-737`)

Start: `setIsPrintingLayerScrubbing(true)` + `schedulePrintingPreviewSettle()`.
End: cancel the pending rAF, flush the pending layer to **both** selected and
displayed, `setIsPrintingLayerScrubbing(false)`, then set `printingDisplayedLayer`
to the clamped target. Source comment:

> *"Switch display target to the released layer immediately. If that layer PNG
> is not loaded yet, UI falls back to cross-section preview instead of showing
> stale PNG from the previously displayed layer."*

### Settle timer (`:209-222`)

```ts
setPrintingPreviewSettled(false)  // and the ref
clearTimeout(previous)
timeout = setTimeout(() => { settledRef = true; setIsPrintingPreviewSettled(true); }, 180)
```
Restarted by: scrub start, scrub end, every zoom step, every pan move, every
pointer-end, and by the effect that runs when `selectedPrintingLayerPreviewUrl`
resolves (`:640-650`).

### PNG decode tracking (`:415-442`)

For each new `selectedPrintingLayerPreviewUrl`:
```ts
const loadNonce = ++printingPreviewLoadNonceRef.current;
const image = new Image(); image.decoding = 'async';
image.onload  = () => { if (nonce stale) return; setPrintingPngLoadedUrl(targetUrl); };
image.onerror = () => { if (nonce stale) return; setPrintingPngLoadedUrl(targetUrl); };  // fail-open
image.src = targetUrl;
```
No URL → `setPrintingPngLoadedUrl(null)`.

`selectedPrintingLayerPreviewUrl` (`:380-387`):
```ts
if (printingDisplayedLayer < 1) return null;
if (multiTankPreview?.layer === printingDisplayedLayer) return multiTankPreview.url;
// While a multi-tank composite is being built, the white single-image mask is
// not shown: it would flash an uncoloured layer first.
if (multiTankArtifactPath) return null;
return layerPreviewUrlAt(printingDisplayedLayer);
```

### `shouldShowScrubPreview` (`:397-408`)

```ts
isPrintingLayerScrubbing || !isPrintingPreviewSettled || !selectedPrintingLayerPreviewUrl || !isPrintingPngLoaded
```
Inside the manager it is used only to compute `isPrintingPreviewLowResActive`
(it is also forwarded to the diagnostics panel as a debug field,
`page.tsx:10975`). It does **not** gate the GPU layer — that is always mounted
while `totalLayers > 0`; it decides whether the scrub-upscale trick applies.

### Cursor (`:592-600`)

```ts
if (!selectedPrintingLayerPreviewUrl) return 'default';
if (printingPreviewZoom > 1.0001) return isPrintingPreviewPanning ? 'grabbing' : 'grab';
return 'zoom-in';
```

### Mode transitions (`:603-650`)

Entering printing resets: `isPrintingSettledCanvasReady = false`,
`settledRef = false`, `isPrintingPreviewSettled = false`, `zoom = 1`,
`pan = {0,0}`, `isPrintingPreviewPanning = false`, drag ref `null`, settle timer
cleared. Hiding the pane (`mode !== 'printing'`) additionally sets
`isPrintingLayerScrubbing = false` and `printingDisplayedLayer = 1`.
Leaving also clears `isSceneLayerScrubbing`. And on entering, `page.tsx:7261-7266`
sets `printingSelectedLayer = printingDisplayedLayer = 1` and
`printingSelectedLayerRef.current = 1`.
A change of `selectedPrintingLayerPreviewUrl` → `setIsPrintingSettledCanvasReady(false)` (`:652`).

### Multi-tank composite (`:273-378`)

When `activePrinterProfile.materialMode === 'multiTank'` and the artifact has a
`nativeTempPath`, the layer image is composed from `layer_%05d_tank_%d.png`
members of the zip:

- skipped entirely while `isPrintingLayerScrubbing || !isPrintingPreviewSettled`
  (source comment: *"While the pointer is moving the GPU cross-section is the
  preview; composing images for every intermediate layer is what made scrubbing
  stutter. The settled layer is composed once, when the scrub ends."*)
- zip is loaded once and cached per path (`multiTankZipRef`);
- tank PNGs are opaque grayscale masks: the **red channel is coverage**, so each
  tank's pixels become `(tint.r, tint.g, tint.b, coverage)`, then the tanks are
  drawn **in tank order** over one canvas;
- tint = the tank's material colour from `getProfileStoreSnapshot().materialProfiles`,
  else `TANK_PREVIEW_COLOURS = ['#60a5fa', '#facc15', '#f87171', '#34d399', '#c084fc', '#fb923c']`
  indexed by tank;
- result is a PNG object URL, cached in a `Map<layer, url>` capped at **12**
  entries (oldest evicted and revoked, never the one just inserted);
- any failure → `setMultiTankPreview(null)` (falls back to nothing, not to the
  white mask).

### `printingPreviewTargetResolution` (`:106-137`)

```ts
const printerWidth  = Math.max(1, Math.round(activePrinterProfile?.display?.resolutionX ?? 0));
const printerHeight = Math.max(1, Math.round(activePrinterProfile?.display?.resolutionY ?? 0));
const pixelSizeX = Math.max(0.0001, Number(activePrinterProfile?.pixelSize?.x ?? 1));
const pixelSizeY = Math.max(0.0001, Number(activePrinterProfile?.pixelSize?.y ?? 1));
const hasPrintableArtifact = (printingArtifact?.outputName ?? '').trim().length > 0;
if (!hasPrintableArtifact || printerWidth <= 0 || printerHeight <= 0) return null;
return { widthPx: printerWidth, heightPx: printerHeight,
         viewportWidth: printerWidth * pixelSizeX, viewportHeight: printerHeight * pixelSizeY };
```
`null` is the gate for three things: the aspect box falls back to build-volume
dimensions, the PNG is drawn as a plain `<img>` instead of an SVG, and the GPU
preview loses its explicit viewport aspect.

---

## Deliberate decisions and gotchas

1. **The GPU layer is always mounted while `totalLayers > 0`** — its wrapper is
   `opacity: 1` with `pointer-events: none`, never unmounted. Comment at
   `:173`: *"Fast scrub preview: keep mounted to avoid first-use GPU warmup
   hitch."* Do not "optimise" it away when the PNG is loaded.
2. **The PNG element keeps the last loaded URL when the selected one is gone.**
   `printingPreviewPngUrlForDisplay` (`:411`) is `selected ?? lastLoaded`, but
   `isPrintingPngLoaded` (`:389`) is false as soon as the selected URL differs —
   including when it is `null` — so the PNG layer sits at `opacity: 0` for the
   whole handoff. What survives is the *node and its 150ms fade*, not a visible
   stale image; the GPU cap is what the user sees until the new bitmap decodes.
   Comment at `:204`: *"PNG layer on top (held briefly during scrub handoff to
   avoid flash)."* A port that unmounts the PNG on every layer change will flash.
3. **"Settled" is a 180ms timer, not "scrub ended".** `isPrintingLayerScrubbing`
   is only true while the thumb is held, but wheel-zoom and pan also need the
   debounce. `schedulePrintingPreviewSettle` (restarting a 180ms timeout) is
   called from scrub start, scrub end, every zoom step, every pan move, every
   pointer-end, and when a layer URL resolves. Both flags feed the PNG loader
   and the multi-tank compositor.
4. **`LayerSlider` gets a `value` the pane already clamped, and clamps again.**
   `emitChange` (`:118`) re-clamps and snaps, and returns early when the value is
   unchanged — which is why the hotkey handler can pass `selected + 1` at the top
   layer without any store write.
5. **`percent` divides by `Math.max(1, max - min)`**, while the value maths
   divides by `Math.max(1e-6, max - min)`. With one layer `percent` is 0, so the
   floating layer-number badge is hidden (`hideUpperFloatingBadge`) even though
   the thumb sits at the bottom — the badge is hidden at **both** ends by design
   (`EDGE_BADGE_HIDE_EPS = 1e-6`).
6. **`slicingHeightMm` (`maxHeightMm`) has no visible effect in printing mode.**
   In the minimal rail it only participates in `displayHeightMm` when
   `layerHeightMm` is a number, and the pane never passes `layerHeightMm`. The
   tooltip's mm label therefore comes from `currentHeightMm`, i.e. from
   `printingCurrentHeightMm`. The real height cap lives in that memo
   (`Math.min(…, Math.max(slicing.heightMm, 0))`).
7. **The pane does not pass `crossSectionEnabled` or `onToggleCrossSection`.**
   Consequences: the rail's thumb/fill are always accent-coloured, and
   double-clicking the rail does nothing (`onDoubleClick` returns immediately).
   The tooltip's third clause (cross-section toggle) is therefore never shown.
   The Rust port *does* pass `state.cross_section`, so a double-click there would
   diverge unless the toggle is also wired.
8. **The thumb edit popover is reachable even in the minimal rail.** Right-click
   within `(9/2 + 1.5)` px of the thumb core opens it. In printing mode it is a
   plain layer-number field, because the mode toggle row is gated on
   `layerHeightMm` — which the pane does not supply.
9. **`glide` only animates jumps.** `near = 2 * step`; a ±1 change snaps, ≥2
   frames at 170ms. Quote: *"A wheel or arrow step lands where it is: eased,
   every step of a scroll restarts the glide, and the thumb trails the layer the
   viewport already shows."*
10. **The scroll direction on the rail is inverted** relative to the browser:
    `dir = e.deltaY > 0 ? -1 : 1`, and the rail's native listener is
    `{ passive: false }` with `preventDefault()`. In printing mode the page must
    not scroll.
11. **Two wheel surfaces, one wheel.** The preview box's `onWheel` zooms the
    image (1…32, `exp(-deltaY * 0.0015)`); the rail's wheel steps the layer. They
    are different elements, so no arbitration is needed — but the preview box's
    wheel **does not** `preventDefault()` when `totalLayers <= 0` (it returns
    first), which is harmless only because the pane never scrolls.
12. **Zoom cannot go below 1, and scrolling down at 1× is ignored**
    (`if (previousZoom <= 1.0001 && event.deltaY > 0) return;`). Pan is clamped
    to `±(zoom - 1) * viewport / 2`, so the image can never be dragged off its box.
13. **The aspect box is sized by container query units on purpose.** The quoted
    comment at `:164-166` explains the failure mode of the obvious
    `width: 100%; max-height: 100%` reading: the GPU layer stretched while the
    PNG stayed letterboxed. `containerType: 'size'` on the preview box is
    required for `100cqw`/`100cqh` to resolve at all — without it the box
    collapses.
14. **Mirroring is applied in two different places.** The GPU preview mirrors its
    Canvas (`scale(±1, ±1)`); the aspect box counter-mirrors with
    `printingPreviewDeMirrorTransform` so the *PNG* reads correctly; the settled
    canvas applies `printingPreviewMirrorScale` itself; the empty state's text is
    counter-mirrored again inside the aspect box. A port that mirrors once will
    flip the PNG.
15. **The `<svg>`/`<img>` split is a real branch, not a preference.** With a
    known printer resolution the PNG is drawn as an SVG `<image>` with the
    printer's exact geometry (`viewBox` in millimetres), `preserveAspectRatio="none"`
    on the image and `imageRendering: 'pixelated'`. Without it, `object-contain`
    letterboxes the raw bitmap. Both paths use `alt`/`aria-label` with the
    **selected** layer while the box shows the **displayed** one.
16. **`supportRenderRefreshNonce` is a prop of the GPU preview only.** It becomes
    `CrossSectionStencilCap.sourceObjectVersion`, which makes the cap re-collect
    the support group. Supports are not in `models`, so without this the
    silhouette loses its supports after a support edit.
17. **Multi-material grouping has a fallback chain, in order:**
    `bodyColors[i]` → material colour resolved from `bodyMaterialProfileIds[i]`
    → `model.color` → `'#ffffff'`; single-material mode ignores all of it and
    draws one white group.
18. **The GPU camera sits on +Z at 1000mm looking at the origin**, `near 0.1`,
    `far 2000`, `localClippingEnabled = true`. The frustum is fitted so the plate
    is *contained* by the viewport aspect, never stretched: if the viewport is
    wider than the plate the frustum widens; if taller it grows in height. This
    is the same rule as the CSS aspect box, and both must agree or the GPU cap
    and the PNG will not line up.
19. **`dpr={[1, 1]}` and `antialias: false` are intentional for the scrub
    preview** — it is a fast silhouette, not a final render.
20. **Printing mode resets the layer selection to 1 on entry and restores the
    pre-printing slicing layer index on exit** (`page.tsx:7261-7274`, comment:
    *"Restore general slider state so printing scrub position does not leak
    across modes."*). It also force-syncs `slicing.setLayerIndex` to the selected
    printing layer in a layout effect (`page.tsx:7361-7375`, comment: *"Keep 3D
    cross-section in lock-step with selected PNG layer. Use 1-based layer index
    here so layer 1 still produces a real cut plane."*).
21. **`W`/`S` are layer keys only in printing mode** (`page.tsx:7209-7220`), in
    addition to `ArrowUp`/`ArrowDown`, and only when alt/ctrl/meta are not held.
    The events arrive on `window` as `app-hotkey-keydown` `CustomEvent`s with
    `detail: { key, code, altKey, ctrlKey, metaKey, … }`, dispatched by
    `src/hotkeys/HotkeyRegistryManager.tsx:183`.
22. **Deleting the last model while in printing mode kicks the app back to
    prepare and clears the sliced plates** (`page.tsx:7290-7293`).
23. **`formatLayerPreviewAlt` / `formatLayerProgress` are module-scope
    functions.** Comment at `PrintingPreviewPane.tsx:55-56`: *"Interpolating
    messages live at module level so React Compiler cannot rename their
    placeholders (see AGENTS.md)."* The same rule produced
    `formatApproxPrintTimeLabel` and `formatUploadToLabel` in `page.tsx:509-550`.
24. **`Card`'s `ui-panel` class rewrites its own buttons**: `.ui-panel .ui-button`
    gets `font-size: var(--font-size-md)` and `padding: 5px 11px 7px 11px`
    (`globals.css:457-479`). Do not re-derive button padding from `!h-9`.
25. **`PrintingPanelStack` is invoked as a plain function call, not as JSX**
    (`page.tsx:10931` does `{PrintingPanelStack({...})}`), with `'use no memo'`
    at the top, so `FloatingPanelStack` can flatten the keyed panels. A port must
    not treat the component boundary as a box boundary.

---

## Open questions

1. **Which image does the settled canvas show while a new layer decodes?** The
   aspect box holds the previous URL via `printingPreviewPngUrlForDisplay`, but
   `usePrintingSettledHiResCanvas` is torn down whenever
   `selectedPrintingLayerPreviewUrl` changes (`isPrintingSettledCanvasReady`
   goes false), so a zoomed view blanks its hi-res layer for one decode. Whether
   that is intended is not stated.
2. **`printingPreviewTargetResolution` requires `printingArtifact.outputName`
   to be non-empty.** A slice written with an empty output name would lose the
   SVG path and the exact aspect ratio; the source does not treat that as an
   error state.
3. **`models`' `visible` flag** is honoured by the GPU cap, but printing mode
   itself zeroes tint and highlight (`page.tsx:8392-8431`); hidden models are
   hidden in the cap only.
4. **The multi-tank composite reads the artifact zip from disk**
   (`readPrintArtifactBytesFromPath` + JSZip). The GPUI port has no equivalent
   yet; this is the one place the preview depends on native file access.
5. **`reveal_in_file_manager`** is a Tauri command; whether the port exposes it
   through `gpui/backend` is not established here.
6. **`useFloatingPanelCollapse(true)`** always opens expanded; nothing in the
   graph read persists the collapsed state, so a port that restores it would
   differ.

---

## Port deviations already recorded (for continuity)

`gpui/src/features/printing_preview.rs` documents these — restated because they
are the current contract of the Rust module:

- GPU scrub preview: absent (three.js). With layers and no PNG the preview box
  is empty.
- PNG/settled canvas: one resolved local path drawn `object-fit: contain`; no
  fade transitions; drawn settled.
- Aspect box: not laid out; the dashed border is painted onto a canvas child at
  the fitted rect (`paint_aspect_border`, 3px dashes / 3px gaps) and the corners
  are square.
- Transforms (pan/zoom/de-mirror/upscale): not applied.
- Wheel / pointer pan-zoom: not ported; the platform cursor is used.
- `z-20`, `tracking-wide`, `containerType`, ARIA: no counterparts.
- `printingCurrentHeightMm` / `slicingHeightMm`: not taken by the port (so the
  GPU `clipZ` and the slider mm readout are missing).

`gpui/src/chrome/overlays.rs` documents the `layer_slider` gaps: right-click
thumb editor, shift arcs, shift latch during drag, `scale-105`, three-stop
gradient, `tabular-nums`, cursor-following tooltip, deferred scrub values, and
the non-minimal variants.
