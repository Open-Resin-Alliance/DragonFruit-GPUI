# Transforms

Transform panel, transform gizmo handles, mesh-shader preview, and the Arrange /
Smoothing / Organic Cut (Split) tool panels.

All paths below are relative to the worktree root
(`E:/Open-Resin-Alliance/DragonFruit.worktrees/gpui-feasibility`). The TypeScript
sources live under `src/`; the GPUI port lives under `gpui/`.

## Components in scope

| Component | File |
|---|---|
| `TransformControls` | `src/components/controls/TransformControls.tsx` |
| `GizmoMove` | `src/components/gizmo/move/GizmoMove.tsx` |
| `GizmoCenter` | `src/components/gizmo/move/GizmoCenter.tsx` |
| `GizmoRotation` | `src/components/gizmo/rotate/GizmoRotation.tsx` |
| `RotationDial` | `src/components/gizmo/rotate/RotationDial.tsx` |
| `rotationDialModel` (pure model) | `src/components/gizmo/rotate/rotationDialModel.ts` |
| `SnapAngleReadout` | `src/components/gizmo/rotate/SnapAngleReadout.tsx` |
| `RotationHintTooltip` | `src/components/gizmo/rotate/RotationHintTooltip.tsx` |
| `GizmoScale` | `src/components/gizmo/scale/GizmoScale.tsx` |
| `applyScaleFactor` (pure) | `src/components/gizmo/scale/applyScaleFactor.ts` |
| `MeshShaderPreviewSlot` | `src/components/settings/meshSettings/MeshShaderPreviewSlot.tsx` |
| `MeshShaderPreviewCanvas` | `src/components/settings/meshSettings/MeshShaderPreviewCanvas.tsx` |
| `ArrangePanel` | `src/components/controls/ArrangePanel.tsx` |
| `AutoRotationPanel` | `src/components/controls/AutoRotationPanel.tsx` |
| `MeshSmoothingSettingsPanel` | `src/features/mesh-smoothing/MeshSmoothingSettingsPanel.tsx` |
| mesh-smoothing settings store | `src/features/mesh-smoothing/settings.ts` |
| `OrganicCutPanel` | `src/features/organicCut/OrganicCutPanel.tsx` |
| organic-cut panel defaults | `src/features/organicCut/useOrganicCutSession.ts` (`DEFAULT_PANEL_STATE`, `:411`) |
| organic-cut shared types | `src/features/organicCut/types.ts` |

Shared constants used by every gizmo surface: `src/components/gizmo/constants.ts`.
Shared assembly point (which panel renders for which tool mode):
`src/components/organisms/panels/PreparePanelStack.tsx:220-345`.

---

## Surface: `TransformControls` — the numeric transform card

- **Source**: `src/components/controls/TransformControls.tsx:65` (component),
  card shell at `:203`, sections at `:218` (Move), `:283` (Lift), `:361`
  (Rotate), `:448` (Scale).
- **Renders**: one collapsible `Card`. Header = collapse chevron + `h3`
  "Transform". Body = four stacked sub-cards: Move, Lift, Rotate, Scale, each a
  coloured `rounded-md border p-2` with a centred section heading and a toggle on
  the right of that heading row.

### Card shell

```
<Card className="w-full overflow-x-hidden shadow-xl">
<CardHeader left={ <PanelCollapseToggle/> <h3 className="text-sm font-semibold" style={{color:'var(--text-strong)'}}>Transform</h3> } />
```

Body (only when `expanded`):

```
className="px-2 pb-2 space-y-2 sm:px-2.5 sm:pb-2.5 max-h-[calc(100vh-180px)] overflow-y-auto custom-scrollbar"
```

`expanded` comes from `useFloatingPanelCollapse(true)`
(`src/components/layout/FloatingPanelStack.tsx:75`): a local `useState(true)`,
and the setter also asks the floating stack to expand the panel
(`stack.requestPanelExpand(panelId)`) when going collapsed → expanded.

`PanelCollapseToggle` (`src/components/atoms/PanelCollapseToggle.tsx`): an
`IconButton` whose icon is `ChevronDown` when expanded / `ChevronRight` when not,
`className="h-3 w-3"`, colour `var(--accent)` expanded / `var(--text-muted)`
collapsed, accessible name `Collapse card` / `Expand card`, plus
`aria-expanded`.

`SectionHeader` (`:17-21`):

```
className="py-0.5 text-center text-xs font-semibold uppercase tracking-wide"
style={{ color: 'var(--text-strong)' }}
```

`SectionHeaderProps` declares `accentColor?: string` which is **never read**
(`:10-13`).

### Section card styles (quoted)

```js
const sectionCardStyle = { borderColor: 'var(--border-subtle)', background: 'var(--surface-1)' };        // declared, unused
const moveCardStyle   = { borderColor: 'color-mix(in srgb, #4f8cff, var(--border-subtle) 78%)',
                          background:  'color-mix(in srgb, #4f8cff, var(--surface-1) 94%)' };
const rotateCardStyle = { borderColor: 'color-mix(in srgb, #8f6cff, var(--border-subtle) 80%)',
                          background:  'color-mix(in srgb, #8f6cff, var(--surface-1) 95%)' };
const scaleCardStyle  = { borderColor: 'color-mix(in srgb, #2eb67d, var(--border-subtle) 80%)',
                          background:  'color-mix(in srgb, #2eb67d, var(--surface-1) 95%)' };
const liftCardStyle   = { borderColor: 'color-mix(in srgb, #f59e0b, var(--border-subtle) 76%)',
                          background:  'color-mix(in srgb, #f59e0b, var(--surface-1) 93%)' };
const outlinedConfigEmphasisStyle = { borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 36%)',
                                      background: 'transparent', color: 'var(--text-strong)' };          // declared, unused
```

`color-mix(in srgb, A, B N%)` = `A*(1-N) + B*N` (handoff decision 3). So
`moveCardStyle` is `#4f8cff` at 22% over `--border-subtle` at 78%, and
`#4f8cff` at 6% over `--surface-1` at 94%.

### Shared button/input recipes

```js
const compactButtonClass = '!h-8 whitespace-nowrap px-1.5 text-[10px] sm:text-[11px]';
const valueInputClass    = 'ui-input h-8 w-full px-1.5 text-xs sm:text-sm text-left tabular-nums no-spinners';
const toggleButtonClass  = 'h-7 min-w-[64px] rounded-md border px-2 text-[10px] font-semibold uppercase tracking-wide transition-colors';
const toggleButtonStyle = (active) => active
  ? { borderColor: 'color-mix(in srgb, var(--accent), white 10%)',
      background:  'color-mix(in srgb, var(--accent), var(--surface-0) 76%)',
      color:       'var(--accent-contrast)' }
  : { borderColor: 'var(--border-subtle)', background: 'var(--surface-1)', color: 'var(--text-muted)' };
```

Active toggle = accent 90% + white 10% border, accent 24% + `--surface-0` 76%
fill, `--accent-contrast` text.

Every numeric field is `NumberInput` with `showStepper={false}`
(`src/components/ui/NumberInput.tsx`): a text input, `inputMode="decimal"`,
locale-formatted to 2 decimals (`fmt.field(n, 2)`), accepts `-`, a locale
separator and up to two decimals (`inputPattern`), writes on every keystroke,
**snaps back to the committed prop value on blur**, and steps by `step` (default
1) on wheel (dominant axis, `wheelStepDirection`).

### Move section (`:218`)

Heading row: `flex items-center` with an empty `flex-1` left, `SectionHeader`
"Move" centre, and `flex-1 justify-end` holding the local-space toggle.

Local-space toggle (`renderLocalSpaceToggle`, `:146`):

- Label `Local`, `context="Gizmo space toggle"`.
- `onClick={() => onLocalSpaceChange(!localSpace)}`.
- `title={t`Move and rotate along the model's own axes instead of the build plate's`}`.
- Rendered **twice** — Move heading and Rotate heading. Source comment:
  `// Move and rotate share the gizmo's frame, so they share one button: pressing
  either one turns both to the model's own axes.`

Fields: `grid grid-cols-3 gap-1 min-w-0`, one `min-w-0` column per axis.

| Axis | Label class/style | Unit span |
|---|---|---|
| X | `ui-meta mb-1 block text-center`, `color: '#f87171'` | `mm` |
| Y | same, `color: '#4ade80'` | `mm` |
| Z | same, `color: '#60a5fa'` | `mm` |

Unit span: `pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-[11px] font-semibold`,
`color: 'var(--text-muted)'`.

Values: `parseFloat(position.x.toFixed(2))` etc. `onChange` →
`handlePositionChange(axis, value)` (`:177`), which clones `position`, sets the
one component, and calls `onPositionChange(x, y, z)` for **all three** values.
`onBlur` → `handlePositionBlur` (`:183`):

```js
if (onPositionCommit) { onPositionCommit(); return; }
onTransformCommit?.();
```

Button (full width, below the grid):
`<Trans>Center Selection</Trans>`, `variant="secondary"`, `size="auto"`,
`className={compactButtonClass + ' w-full'}`, `onClick={onCenter}`.

### Lift section (`:283`)

Heading row: empty `flex-1`, `SectionHeader` "Lift", right slot = the Auto toggle.

Auto toggle: `toggleButtonClass` + `toggleButtonStyle(autoLift)`, label
`Auto` with `context="Auto-lift toggle"`. Title (verbatim):

> `On, the selected models are lifted to this distance and keep it through moves, rotations and scaling. Off, they return to the height they were lifted from; Drop puts them on the plate.`

`onClick={() => onAutoLiftChange(!autoLift)}`.

Distance field: `NumberInput`, `value={liftDistance}`,
`className="ui-input h-8 w-full px-2 text-xs sm:text-sm text-center no-spinners"`,
`showStepper={false}`, unit span `mm` at `text-[10px]`,
`onBlur={() => onTransformCommit?.()}`.

Buttons: `grid grid-cols-2 gap-1`, both `variant="secondary"`, `size="auto"`,
`className="!h-8 px-1.5 text-[10px] sm:text-[11px] disabled:opacity-50 disabled:cursor-not-allowed"`,
both `disabled={!modelBBox}`.

| Button | Label | onClick | Inline style |
|---|---|---|---|
| Lift | `<Trans context="Lift button">Lift</Trans>` | `onLift(); onTransformCommit?.()` | `borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 38%)'`, `color: 'color-mix(in srgb, var(--accent), var(--text-strong) 30%)'`, `background: 'color-mix(in srgb, var(--accent), var(--surface-1) 90%)'` |
| Drop | `<Trans context="Drop button">Drop</Trans>` | `onDrop(); onTransformCommit?.()` | `borderColor: 'color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 42%)'`, `color: 'color-mix(in srgb, var(--accent-secondary), var(--text-strong) 30%)'`, `background: 'color-mix(in srgb, var(--accent-secondary), var(--surface-1) 92%)'` |

### Rotate section (`:361`)

Heading row: `flex-1 flex justify-start` (left), `SectionHeader` "Rotate",
`flex-1 flex justify-end` (local-space toggle).

Left slot renders the Group toggle **only when `selectionRotationMode` is set**
(the parent passes it only for a multi-selection):

- `toggleButtonClass`, `toggleButtonStyle(selectionRotationMode === 'group')`.
- Label `<Trans context="Group rotation toggle">Group</Trans>`.
- `onClick={() => onSelectionRotationModeChange?.(selectionRotationMode === 'group' ? 'individual' : 'group')}`.
- Title: `Rotate the selection as one block around its centre; turn off to rotate each model in place`.

Fields: same 3-column grid and axis colours as Move. Values:

```js
parseFloat(wrapRotationDegrees(toDegrees(rotation.x)).toFixed(2))
```

`wrapRotationDegrees` (`:161`):

```js
if (!Number.isFinite(deg)) return 0;
const wrapped = ((((deg + 180) % 360) + 360) % 360) - 180;
return Object.is(wrapped, -0) ? 0 : wrapped;
```

The field therefore shows `(-180, 180]`: `-180` is reachable, `+180` is not.

Unit span: `°` at `text-[14px]`.

`onChange` → `handleRotationChange(axis, value)` (`:192`): wraps, converts to
radians, clones `rotation`, sets one component, calls `onRotationChange(x,y,z)`.

`onBlur` → `{ onRotationComplete?.(); onTransformCommit?.(); }` — in that order,
on all three axes.

Button: `Reset Rotation`, `variant="secondary"`, `size="auto"`,
`className="w-full !h-8 px-1.5 text-[10px] sm:text-[11px]"`,
`onClick={() => { onResetRotation(); onTransformCommit?.(); }}`.

### Scale section (`:448`)

Heading row: `flex items-center justify-between`; empty `flex-1`,
`SectionHeader` "Scale", right slot = the Uniform toggle (same
`toggleButtonClass` / `toggleButtonStyle(uniformScaling)`, label
`<Trans context="Uniform scaling toggle">Uniform</Trans>`,
`onClick={() => onUniformScalingChange(!uniformScaling)}`; **no title**).

Body: `pt-1.5 space-y-1.5`, two 3-column rows.

**Percentage row** (`grid grid-cols-3 gap-1 min-w-0 items-start`): labelled
columns, labels `ui-meta mb-1 block text-center text-[10px]` with the axis colours
(`#f87171` / `#4ade80` / `#60a5fa`). Value `parseFloat((scale.x * 100).toFixed(2))`,
unit `%` at `text-[10px]`.

```js
onChange: (val) => {
  const newScale = val / 100;
  if (uniformScaling) onScaleChange(newScale, newScale, newScale);
  else onScaleChange(newScale, scale.y, scale.z);   // Y/Z rows swap the written axis
}
onBlur: onScaleCommit
```

**Millimetre row**: no labels, same 3 columns. Value
`originalSize ? parseFloat((scale.x * originalSize.x).toFixed(2)) : 0`; unit `mm`.
`onChange`: `if (!originalSize) return;` then `newScale = val / originalSize.x`
and the same uniform/axis logic. `onBlur: onScaleCommit`.

`originalSize` (`:168`) is `modelBBox ? (max - min per axis) : new THREE.Vector3(1,1,1)`
— **always truthy**, so the `originalSize ? … : 0` guards are dead code; with a
null `modelBBox` the mm row shows `scale * 1`.

Button: `Reset Scale`, `variant="secondary"`, `size="auto"`,
`className="w-full !h-8 px-1.5 text-[10px] sm:text-[11px]"`,
`onClick={() => { onResetScale(); onScaleCommit(); }}`.

### Data

Props only — no store/context reads. Source of truth is the parent
(`PreparePanelStack.tsx:221-280`): `transformMgr.transform.{position,rotation,scale}`,
`scene.geom.bbox`, `autoLift`, `transformMgr.liftDistance`, plus the hook's own
setters. In a multi-selection the position/rotation callbacks are swapped for the
selection-wide handlers and `selectionRotationMode` is passed.

### Callbacks (full list)

`onPositionChange(x,y,z)`, `onPositionCommit()`, `onCenter()`, `onPlatform(bbox)`
(**declared and destructured at `:70` but never called** — no Platform control is
rendered), `onRotationChange(x,y,z)`, `onResetRotation()`, `onRotationComplete()`,
`onSelectionRotationModeChange(mode)`, `onScaleChange(x,y,z)`, `onResetScale()`,
`onScaleCommit()`, `onUniformScalingChange(bool)`, `onLocalSpaceChange(bool)`,
`onAutoLiftChange(bool)`, `onLiftDistanceChange(n)`, `onLift()`, `onDrop()`,
`onTransformCommit()`.

### Text

| String | i18n |
|---|---|
| `Transform` | `<Trans>` |
| `Move` | `t({message:'Move', context:'Transform panel section'})` |
| `Lift` | `t({message:'Lift', context:'Transform panel section'})` |
| `Rotate` | `t({message:'Rotate', context:'Transform panel section'})` |
| `Scale` | `t({message:'Scale', context:'Transform panel section'})` |
| `Local` | `<Trans context="Gizmo space toggle">` |
| `Center Selection` | `<Trans>` |
| `Auto` | `<Trans context="Auto-lift toggle">` |
| `Lift` (button) | `<Trans context="Lift button">` |
| `Drop` | `<Trans context="Drop button">` |
| `Group` | `<Trans context="Group rotation toggle">` |
| `Reset Rotation` | `<Trans>` |
| `Uniform` | `<Trans context="Uniform scaling toggle">` |
| `Reset Scale` | `<Trans>` |
| `X`, `Y`, `Z` | literal, not translated |
| `mm`, `°`, `%` | literal unit strings |

Tooltips (`t\`…\`` msgids, no context): the Local title, the Auto-lift title and
the Group title, quoted above verbatim.

### Port target

`gpui/src/features/transform_panel.rs` — `TransformState` / `TransformActions` /
`transform_body(state, actions)` with `move_section`, `lift_section`,
`rotate_section`, `scale_section`, `local_space_toggle`. Already ported, with
these recorded deviations (file header): no text entry (stepped chevrons instead
of typing), no `vh` height cap (the panel stack owns the height), no
`transition-colors`, no `tracking-wide`; the Auto Orientation body lives in the
same file (`orientation_body`). The port does not render `onPlatform` — neither
does the source.

---

## Surface: `GizmoMove` — one axis arrow of the move gizmo

- **Source**: `src/components/gizmo/move/GizmoMove.tsx:47`.
- **Renders**: an R3F group rotated onto one axis, containing an invisible pick
  sphere on the tip (optionally a second on the opposite tip), a gradient shaft
  mesh, a double cone head (outline + solid), and a point light.
- **Layout & styling** — gizmo units, later scaled by `ScreenSpaceGizmo`
  (`distance * 0.04`):

```js
const shaftLength = Math.max(0.3, GIZMO_SIZES.arrowShaftLength * moveHandleLengthScale); // 6.0
const shaftRadius = Math.max(0.008, 0.02 * moveHandleThicknessScale);
const headRadius  = Math.max(0.03, GIZMO_SIZES.arrowHeadRadius * moveHandleThicknessScale); // 0.24
const headLength  = Math.max(0.08, GIZMO_SIZES.arrowHeadLength * moveHandleLengthScale);   // 0.6
```

Arrow side ("which side of the model the camera is on", `:132-148`):

```js
const pointsTowardCamera = axisDirection.dot(camera.position - gizmoPosition) > 0;
// x → [0,0, shouldFlipX ? -PI/2 : PI/2]
// y → [0,0, shouldFlipY ? 0 : PI]
// z → shouldFlipZ ? [PI/2,0,0] : [-PI/2,0,0]
// disableArrowFlip → positiveAxisRotation (x:[0,0,-PI/2], y:[0,0,0], z:[PI/2,0,0])
```

Pick spheres: radius `Math.max(0.14, headRadius * 2.35)`, 12×12 segments, at
`[0, ±shaftLength, 0]`, `scale={handleScale}`,
`<meshBasicMaterial visible={false} depthTest={false} />`. `visible` lives on the
**mesh**, not the group — source comment:

> `Invisible sphere pick targets. visible={!isHidden} is on the MESH, not the parent group, so Three.js raycasting is actually suppressed on hidden handles (R3F registers leaf meshes in internal.interaction, and Three.js checks each object's OWN .visible, not its parent).`

Shaft: `position={[0, shaftLength / 2, 0]}`, `renderOrder={-10}`,
`vertexColors={!isDimmed}`,
`color={isDimmed ? dimmedColor : isActive ? '#ffffff' : '#f2f2f2'}`,
`opacity`, `transparent`, `depthTest={false}`, `toneMapped={false}`. The
bidirectional copy sits at `[0, -shaftLength/2, 0]` with `rotation={[0,0,PI]}`.

Head: `<group position arrowTipPosition scale={handleScale * hoverScale}>` with

- outline mesh `scale={1.15}`, colour `dimmed ? dimmed*0.7 : endColor*0.3`,
- solid mesh, colour `dimmed ? dimmed : endColorHex`.

`hoverScale = isActive ? 1.18 : effectiveHovered ? 1.1 : 1.0`.

Point light (only `enableLighting && !isDimmed`) at the tip, colour
`isActive ? GIZMO_COLORS.active : effectiveHovered ? GIZMO_COLORS.hover : axisColors.end`,
`intensity={lightIntensity}`, `distance={GIZMO_LIGHTING.pointLightDistance}` (6.2),
`decay={GIZMO_LIGHTING.pointLightDecay}` (1.15).

- **States**:

```js
const effectiveHovered = !suppressHover && (isPickingHovered || isHovered);
const isHighlighted = !!isActive;
const baseOpacity = isHidden ? 0 : isDimmed ? 0.15 : isHighlighted ? 1.0 : 0.9;
const opacity = baseOpacity * opacityScale;
const endColorHex = isActive ? GIZMO_COLORS.active : effectiveHovered ? GIZMO_COLORS.hover : axisColors.end;
```

`isPickingHovered` = the GPU pick buffer reports `category==='gizmo' &&
gizmoHandle===`move-${axis}``.

- **Data**: `usePicking().register({category:'gizmo', objectId: null, gizmoHandle: 'move-<axis>', object: pickMeshRef.current})`; hover from the same context's `hit`. `axisDirection` = `worldAxisDir` when given, else world X/Y/Z.
- **Callbacks**:
  - `onPointerDown` (`:197`): ignores button 2, `isHidden`, `!interactionsEnabled`;
    `e.stopPropagation(); e.stopped = true`; snapshots
    `dragAxisOriginRef = gizmoPosition.clone()`; reads `initialAxisParam` (abort
    if null); `onDragStart()` (abort when it returns `false`); starts the drag.
  - `onDrag(delta)` per pointer move; `onDragEnd()` on window `pointerup`.
  - `onPointerEnter` / `onPointerLeave` — stop propagation, forward to props.

### Drag maths (`getAxisParamFromMouse`, `:150`)

NDC → `setPickRayFromCamera`, then closest points between the pointer ray and the
infinite axis line:

```js
axisToRay = origin - ray.origin                       // origin = press-captured gizmo centre
denom = 1 - (axisDir·rayDir)^2
if (|denom| < 1e-6) return axisDir·axisToRay          // degenerate: ray parallel to axis
return ((axisDir·rayDir) * (rayDir·axisToRay) - axisDir·axisToRay) / denom
```

Per move (`:234`):

```js
const axisMagnitude = nextAxisParam - lastAxisParam;
if (Math.abs(axisMagnitude) < MIN_AXIS_DELTA) return;   // MIN_AXIS_DELTA = 1e-10
const delta = axisDirection.multiplyScalar(axisMagnitude);
onDrag(delta);
lastAxisParamRef.current = nextAxisParam;
```

Source comment on the fixed origin:

> `Closest points between the infinite drag axis line and pointer ray. Uses the FIXED drag origin (captured at pointer-down) as the axis reference point, so t-values are comparable across frames even as gizmoPosition updates in response to onDrag calls.`

- **Text**: none.
- **Port target**: `gpui/src/gizmo/` — `drag.rs` (`Handle::Axis`), `layout.rs`
  (`axis_param`, `arrow_tip`, `axis_sign`), `geometry.rs` (shaft/head triangles),
  `pick.rs`. The port picks by pixel distance instead of a GPU buffer, and
  hover/active colouring is a vertex attribute in one gizmo triangle stream.

---

## Surface: `GizmoCenter` — the free-move nub at the gizmo origin

- **Source**: `src/components/gizmo/move/GizmoCenter.tsx:31`.
- **Renders**: invisible pick sphere, filled disc, white border ring, and a
  hover/active halo ring; deliberately no point light.
- **Layout & styling**:

```js
pickGeometry       = SphereGeometry(GIZMO_SIZES.centerRadius * 1.08, 24, 24)
discGeometry       = CircleGeometry(GIZMO_SIZES.centerRadius * 1.05, 32)
borderRingGeometry = RingGeometry(GIZMO_SIZES.centerRadius * 1.0, GIZMO_SIZES.centerRadius * 1.05, 32)
haloRingGeometry   = RingGeometry(GIZMO_SIZES.centerRadius * 1.1, GIZMO_SIZES.centerRadius * 1.32, 32)
```

`centerRadius = 0.6`. All materials `depthTest={false}`,
`side={THREE.DoubleSide}` (disc, ring, halo), `transparent opacity={opacity}`.

- **States**:

```js
const effectiveHovered = !suppressHover && (isPickingHovered || isHovered || isScreenHovered);
const isHighlighted = !!(effectiveHovered || isActive);
const centerColor = isDimmed ? '#cccccc' : isActive ? GIZMO_COLORS.active
                  : effectiveHovered ? GIZMO_COLORS.hover : GIZMO_COLORS.center;
const baseOpacity = isHidden ? 0 : isDimmed ? 0.15 : isHighlighted ? 1.0 : 0.55;
```

Border ring colour `isDimmed ? dimmedColor : "#ffffff"`. Halo (only
`isHighlighted && !isDimmed && !isHidden`): colour
`isActive ? GIZMO_COLORS.active : GIZMO_COLORS.hover`, opacity
`isActive ? 0.5 : 0.38`.

Extra hover source: a per-frame **screen-space** proximity test. `useFrame`
(`:175`) projects `gizmoPosition` and compares against `pointer` in canvas pixels
with `const hoverRadiusPx = 24;`. It caches the canvas rect (`canvasRectRef`,
refreshed by a `ResizeObserver` plus `resize`/`scroll` listeners) because —
source comment — `getBoundingClientRect` inside `useFrame` forces a style/layout
read. While `isDragging || isHidden || suppressHover` it clears the hover state.
Entering fires `onPointerEnter()`, leaving `onPointerLeave()`.

- **Data**: `usePicking()` as `'move-center'`; `useThree()` for `camera`, `gl`,
  `pointer`.
- **Callbacks / drag**: drag plane pinned to the gizmo's current Z —
  `new THREE.Plane(new THREE.Vector3(0, 0, 1), -gizmoPosition.z)`. Press → ray ∩
  plane → `onDragStart()` → store the point. Move →
  `delta = worldPoint - lastPoint`, **`delta.z = 0`**, skip when
  `lengthSq() < 1e-12`, then `onDrag(delta)`. Window `pointerup` and the
  component's own `handlePointerUp` both end the drag with `onDragEnd()`.
- **Text**: none.
- **Port target**: `gpui/src/gizmo/drag.rs` (`Handle::Center`, drag-plane hit,
  Z pinned) + `geometry.rs` (`centre()`), `layout.rs`. Fully ported.

---

## Surface: `GizmoRotation` — one ring + double-cone diamond handle

- **Source**: `src/components/gizmo/rotate/GizmoRotation.tsx:135`.
- **Renders**: a group rotated by `ringGroupEuler(axis)`, containing an invisible
  pick sphere at the handle, a back-arc `Line`, the protractor `RotationDial`
  (only while a drag is live), a `rotatingArcRef` group with the gradient arc mesh
  + front-arc `Line` (+ a white highlight `Line` when active), the double cone
  head inside a billboard group, and an optional point light.

### Constants (`:44-60`)

```js
const ROTATION_CENTER_SINGULARITY_PX = 8;   // pointer angle is noise this close to the centre
const DIAL_MOVE_SLOP_PX = 3;                // travel below this keeps the sweep at zero
```

Long source comments; the slop one:

> `Without it a press that never meant to rotate — a click, or the pixel of jitter a mouse produces while a button goes down — would resolve to an angle a fraction off zero and commit it, since the sweep tracks the pointer's absolute angle rather than accumulating deltas.`

### Handle rest angle

```js
function getPositiveAxisMidpointAngle(axis) {
  if (axis === 'x') return (3 * Math.PI) / 4;
  if (axis === 'y') return -Math.PI / 4;
  return Math.PI / 4;
}
```

### Per-frame animation (`useFrame`, priority `-1`, `:389`)

- Not dragging: `targetHandleAngleRef = getCameraAlignedAngle()`.
- Wrap the delta into `(-PI, PI]`;
  `smoothing = isDragging || suppressAxisAnimations ? 1 : 0.2`.
- Handle position `(cos(a)*4.8, sin(a)*4.8, 0)`;
  `handleRootRef.rotation.set(0, 0, a + PI/2)`; pick mesh and point light follow.
- Billboard: `billboardGroupRef.rotation.x` eases toward
  `atan2(cameraDir.y, cameraDir.x)` at `0.2` per frame (snapped when
  `suppressAxisAnimations`).

`getCameraAlignedAngle` (`:331`) reads the camera direction **in the ring's own
frame** when `ringGroupRef` exists
(`Matrix4().extractRotation(ringGroup.matrixWorld).invert()` → `atan2(y, x)`),
with per-axis `atan2` fallbacks otherwise. Comment:

> `Where the camera is IN THE RING'S OWN FRAME. The per-axis formulas below say the same thing for a world-aligned gizmo, but only this reading survives a gizmo whose frame is the model's own (local space), where the ring's plane has nothing to do with the world axes.`

### Visual states (`:682-706`)

```js
const baseOpacity = isHidden ? 0 : isDimmed ? 0.15 : ringIsActive ? 0.95 : 0.72;
const dialVisible = dialZero !== null && !isHidden && !isDimmed;
const arcOpacity  = dialVisible ? opacity * 0.4 : opacity;
const dimmedColor = '#cccccc';
const diamondPrimaryColor = isDimmed ? dimmedColor : isActive ? GIZMO_COLORS.active
                          : effectiveHovered ? GIZMO_COLORS.hover : axisColors.end;
const diamondSecondaryColor = isDimmed ? dimmedColor : isActive ? GIZMO_COLORS.active
  : effectiveHovered ? new THREE.Color(GIZMO_COLORS.hover).lerp(new THREE.Color(axisColors.start), 0.35).getStyle()
  : axisColors.start;
const ringColor = isDimmed ? dimmedColor : isActive ? GIZMO_COLORS.active : ringColors.ring;
```

Back arc `Line`: `lineWidth={0.8}`, `opacity={Math.max(0, opacity * 0.26)}`.
Front arc mesh: `scale={ringIsActive ? 1.02 : 1.0}`, `vertexColors={!isDimmed}`.
Front arc `Line`: `lineWidth={0.92}`, `opacity={Math.max(0, arcOpacity * 0.38)}`.
Active highlight `Line`: `ringColor.lerp('#ffffff', 0.35)`, `lineWidth={1.34}`,
`opacity={0.22}`.

Pick sphere:
`getCachedSphereGeometry(Math.max(0.18, GIZMO_SIZES.ringDiamondRadius * 0.9 * handleScale), 16, 16)`.

Diamond cones:
`getCachedConeGeometry(GIZMO_SIZES.ringDiamondRadius * 0.36, GIZMO_SIZES.ringDiamondRadius, 16)`,
placed at `[±ringDiamondRadius*0.52, 0, 0]` with rotations `[0,0,-PI/2]` /
`[0,0,PI/2]`; outline mesh `scale={1.08}` with colour `×0.3` / `×0.32`. Head
group `scale={(isHighlighted ? 1.08 : 1.0) * handleScale}`.

Point light: colour
`isActive ? active : effectiveHovered ? hover : diamondPrimaryColor`, same
distance/decay as the move handle.

### States beyond colour

- `isHidden` → `visible={false}` on the pick mesh, opacity 0, no dial, no light.
- `isDimmed` → 0.15, grey, no dial, no light.
- `suppressHover` → picking hover ignored.
- `interactionsEnabled === false` → pointer handlers withheld from the head group.
- Pressing (any button except 2) while `interactionsEnabled`:
  `dialZeroRef = handleAngleRef`, `prevTarget = 0`, `sweepAccum = 0`, magnet
  seeded with `{ angleRad: 0, tier: 'long' }`, `pressPoint = {clientX, clientY}`,
  `beginGizmoDrag(() => finishDrag('cancel'))`, rotation hint hidden,
  `setIsDragging(true)`.

### Drag flow

Global listeners installed while `isDragging` (`:521`):

- `pointermove`: ignore foreign pointers; ignore within `DIAL_MOVE_SLOP_PX` of the
  press; ignore within `ROTATION_CENTER_SINGULARITY_PX` of the projected ring
  centre; NDC → ray → `rayToRingLocal(ray.origin, ray.direction,
  ringGroup.matrixWorld)`; a `null` hit keeps the previous angle. Then
  `shortestAngleDelta(dialZero, hit.angleRad)` → `resolveDialAngle({cursorAngleRad,
  cursorRadius, held, gapPxForAngle})`, where `gapPxForAngle` projects the mark's
  line at `DIAL_ANATOMY.rimRadius` and measures `distancePointToLine` in pixels.
  `heldMark` is updated only when the held mark actually changed. Then
  `stepDialSweep` emits through `onDragRef.current(asked)`.
- `pointerup`: only the drag's own pointer **and button** → `finishDrag('commit')`.
- `pointercancel` → `finishDrag('cancel')`.
- `contextmenu` (capture): `preventDefault` + `stopPropagation`.
- `pointerdown` (capture): a foreign button is swallowed, so a right-drag cannot
  orbit under a live rotation.

`finishDrag('commit' | 'cancel')` (`:262`): if the button is still down it calls
`swallowRestOfGesture(pointerId, button)` (capture-phase `pointerup` /
`pointercancel` / `click` swallowers, removed on a `setTimeout(…, 0)` after the
release, so a gesture that never produces a click cannot leave a swallower
behind). It then detaches the listeners, releases the drag-registry claim, and
for `'cancel'` with a non-zero sweep emits
`emittedDeltaForSweep(-sweepAccum, axisVisualFlip)`. Finally it clears the
dragging state, dial, held mark and press point, calls `onDragEndRef.current()`,
and dispatches `dragonfruit:snap-angle` with `{active:false}`.

Unmount under a live gesture → `useEffect(() => () => finishDrag('commit'), …)`.

### Events the rotation emits

| Event | Detail | Consumer |
|---|---|---|
| `dragonfruit:snap-angle` | `{ active: true, angle: sweepAccumRef, axis }` / `{active:false}` | `SnapAngleReadout` |
| `dragonfruit:rotation-hint` | `{visible:false}` on press; `{visible:true, axis}` on enter while not dragging; `{visible:false}` on leave | `RotationHintTooltip` |

Pointer-enter comment: `// The handle rides the sweep, so it keeps arriving under
the pointer and re-firing this while the gesture is running.`

- **Data**: `usePicking()` as `rotate-<axis>`; `useThree()` camera + gl.
- **Callbacks**: `onDragStart() → boolean | void` (`false` refuses),
  `onDrag(angle) → number | void` (the consumer returns how much it actually
  took, for a hard end), `onDragEnd()`, `onPointerEnter()`, `onPointerLeave()`.
- **Text**: none on the ring; the tooltip text lives in `RotationHintTooltip`.
- **Port target**: `gpui/src/gizmo/` — `layout.rs::ring_hit`,
  `types::Axis::ring_basis`, `drag.rs` (`Handle::Ring`, `shortest_angle_delta`,
  press slop and centre singularity from `constants`), `geometry.rs` (arc +
  diamond). Deliberately absent: the dial and magnet (`drag.rs` header: "The
  protractor dial's magnet is not ported: there is no dial overlay to display it
  against, and an invisible magnet would read as the ring sticking."), billboard
  easing, and per-handle lights.

---

## Surface: `RotationDial` — the protractor drawn under a held ring

- **Source**: `src/components/gizmo/rotate/RotationDial.tsx:42`.
- **Renders**: three `Line` batches (one per mark tier) at dial-relative angles,
  plus a dashed zero radius, an optional held-mark highlight, and a moving-radius
  group.
- **Layout & styling**:

```js
const TIER_LINE_WIDTH = { spoke: 1.3, long: 1.7, short: 0.9 };
const TIER_ORDER = ['spoke', 'long', 'short'];
```

Every `Line`: `transparent`, `depthTest={false}`, `toneMapped={false}`.
Tier opacity: `tier === 'short' ? opacity * 0.75 : opacity`.

Zero reference: `lineWidth={1.0}`, `dashed`, `dashSize={0.22}`,
`gapSize={0.16}`, `opacity={opacity * 0.7}` — comment: `Zero reference: where the
handle was grabbed. Dashed so it reads as the origin of the sweep rather than as
another mark.`

Held mark: `lineWidth={2.8}`, `opacity={Math.min(1, opacity + 0.1)}`.
Moving radius: inside `<group ref={sweepGroupRef}>`, `lineWidth={2.2}`,
`opacity={Math.min(1, opacity + 0.1)}`.

Live colour:

```js
const liveColor = new THREE.Color(color).lerp(new THREE.Color('#ffffff'), 0.55).getStyle();
```

Comment: `The live radius and the held mark are the two elements that have to win
the read against the marks behind them, so they get the same hue lifted toward
white rather than a second colour.`

- **States**: `held === null` → no highlight line. `sweepGroupRef.rotation.z` is
  written imperatively by the drag, once per pointer sample.
- **Data**: `getDialMarks()` (memoised once), `markRadiiForTier`.
- **Callbacks**: none (pure draw; the parent owns the ref).
- **Text**: none.
- **Port target**: absent. Nothing in `gpui/src/gizmo/` draws the dial.

---

## Surface: `rotationDialModel` — the dial's pure model

- **Source**: `src/components/gizmo/rotate/rotationDialModel.ts`.
- **Renders**: nothing. This is the contract the dial, the magnet and the readout
  must agree on.

### Anatomy (quoted)

```
 *   0.0 R .. 0.3 R    no magnet
 *   0.3 R .. 0.6 R    45-degree spokes; the magnet holds these
 *   0.6 R .. 0.9 R    no magnet
 *   0.9 R .. 1.0 R    5- and 10-degree rim marks; the magnet holds these
 *   past 1.0 R        no magnet
```

`R = GIZMO_SIZES.ringMajorRadius = 4.8`. Derived values:

```js
DIAL_STEP_DEG = { spoke: 45, long: 10, short: 5 }
DIAL_ANATOMY = {
  rimRadius: 4.8,
  longTickInnerRadius: 4.8 * 0.9,   // 4.32
  shortTickInnerRadius: 4.8 * 0.95, // 4.56
  spokeInnerRadius: 4.8 * 0.3,      // 1.44
  spokeOuterRadius: 4.8 * 0.6,      // 2.88
}
MAGNET_ENTER_PX = 10
MAGNET_EXIT_PX  = 10
```

### Functions

| Function | Contract |
|---|---|
| `wrapAngle(rad)` | wrap into `(-PI, PI]` modularly (`rad % TWO_PI`, then ±); "an angle already in range comes back bit-for-bit unchanged"; `-0` → `0`. |
| `shortestAngleDelta(from, to)` | `wrapAngle(to - from)`. |
| `formatSweepDegrees(sweepRad, fmt)` | wraps, converts to degrees, `fmt.number(deg, {minimumFractionDigits:2, maximumFractionDigits:2, signDisplay:'exceptZero'})`; non-finite → `0`. |
| `polarToLocal(angleRad, radius)` | `[cos*r, sin*r, 0]`. |
| `ringGroupEuler(axis)` | `x:[0, PI/2, 0]`, `y:[-PI/2, 0, 0]`, `z:[0,0,0]` — the single source of the ring frame. |
| `markRadiiForTier(tier)` | spoke → `(1.44, 2.88)`; long → `(4.32, 4.8)`; short → `(4.56, 4.8)`. |
| `getDialMarks()` | 72 rim marks every 5° (`long` when `deg % 10 === 0`, else `short`) **plus** 8 spokes every 45°. Multiples of 45 are drawn twice by design. |
| `magnetBandForRadius(r)` | `spokes` when `1.44 ≤ r ≤ 2.88`; `rim` when `4.32 ≤ r ≤ 4.8`; else `none`. Radial, not angular. |
| `bandHoldsTier(band, tier)` | `spokes`→`spoke`; `rim`→`long` or `short`; `none`→false. |
| `dialSnapCandidate(angleRad, radius)` | band `none` → null; else snap to the `45°` (spokes) or `5°` (rim) grid; tier `spoke`, or `long` if the snapped degree is a multiple of 10 else `short`; angle wrapped. |
| `resolveDialAngle(input)` | hysteresis (below). |
| `distancePointToLine(p, a, b)` | perpendicular distance to the **infinite line**; `length < 1e-6` → distance to `a`. |
| `rayToRingLocal(origin, dir, matrixWorld)` | below. |
| `emittedDeltaForSweep(sweepRad, flip)` | `-sweepRad * flip`, `-0` → `0`. |
| `stepDialSweep(prev, step)` | below. |

`resolveDialAngle` (`:296`):

```js
if (held && bandHoldsTier(band, held.tier) && gapPxForAngle(held.angleRad) <= exitPx)
  return { angleRad: held.angleRad, held };
const candidate = dialSnapCandidate(cursorAngleRad, cursorRadius);
if (candidate && gapPxForAngle(candidate.angleRad) <= enterPx)
  return { angleRad: candidate.angleRad, held: candidate };
return { angleRad: cursorAngleRad, held: null };
```

Hysteresis comment: "a mark that already has the radius keeps it until the
pointer pulls `exitPx` away… Leaving the corona releases immediately, no matter
how small the gap — the corona gate comes first".

`rayToRingLocal` uses the ring group's **world matrix**:

> `Takes the ring group's WORLD MATRIX, not its rotation and position: the gizmo root is scaled every frame to keep a constant size on screen (ScreenSpaceGizmo, scale = camera distance * 0.04), so the returned radius has to be divided by that scale to be comparable with DIAL_ANATOMY.`

Normal `= (0,0,1).transformDirection(matrixWorld)`; `|rayDir·n| < 1e-6` → null;
`t <= 0` → null; hit point `applyMatrix4(matrixWorld.clone().invert())`; returns
`{ radius: hypot(x,y), angleRad: wrapAngle(atan2(y,x)) }`.

`emittedDeltaForSweep` comment (the sign rule):

> `A positive sweep in the ring's local frame IS a positive rotation about that ring's axis …, but the consumer applies emitted deltas NEGATED — SceneCanvas does setFromAxisAngle(axis, -angle) — so the emission has to negate the sweep or the model turns against the dial.`

`stepDialSweep` (`:441`):

```js
const delta = shortestAngleDelta(previous.targetRad, cursorAngleRad);
if (delta === 0) return previous;
const asked = emittedDeltaForSweep(delta, axisVisualFlip);
const answer = emit(asked);
const applied = typeof answer === 'number' ? answer : asked;
const appliedSweep = axisVisualFlip === 0 || applied === asked ? delta : -applied / axisVisualFlip;
return {
  sweepRad: previous.sweepRad + appliedSweep,
  targetRad: frameCarriesRotation ? cursorAngleRad : previous.targetRad + appliedSweep,
};
```

- **Port target**: absent except `layout.rs::{wrap_angle, shortest_angle_delta}`
  and `drag.rs`'s ring maths. `DIAL_ANATOMY`, the tier grid, the magnet and the
  readout formatter are unported.

---

## Surface: `SnapAngleReadout` — the degree chip that follows the cursor

- **Source**: `src/components/gizmo/rotate/SnapAngleReadout.tsx:14`.
- **Renders**: `null` unless a rotation drag is live; otherwise a
  cursor-following chip with the sweep.
- **Layout & styling**:

```
chip: className="rounded px-1.5 py-0.5 text-[11px] font-mono font-semibold tabular-nums"
      color: AXIS_COLORS[axis]   // x '#ff3120', y '#00ff00', z '#1596ff', fallback '#ffffff'
      background: 'rgba(0, 0, 0, 0.75)'
      border: `1px solid ${color}33`
      textShadow: `0 0 6px ${color}88`
<MouseTooltip visible offset={{ x: 20, y: -30 }}>
```

- **States**: hidden until the first `dragonfruit:snap-angle` event with
  `active === true`; hidden again on `{active:false}` and while
  `angle === undefined`.
- **Data**: `useLocaleFormat()`; text is
  `fmt.unit('degree', formatSweepDegrees(snap.angle, fmt))` — the unit chosen by
  the active locale, value always two decimals with `exceptZero` sign display.
- **Callbacks**: none.
- **Text**: the formatted number + locale degree unit. No msgid.
- **Port target**: absent (`gpui/src/gizmo/` has no overlay text).

---

## Surface: `RotationHintTooltip` — "Drag to rotate" hint over a ring

- **Source**: `src/components/gizmo/rotate/RotationHintTooltip.tsx:7`.
- **Renders**: `MouseTooltip` (`offset={{ x: 20, y: -40 }}`) with two lines.
- **Layout & styling**:

```
container: className="rounded px-2 py-1.5 text-[11px] leading-tight font-medium"
           background: 'rgba(0, 0, 0, 0.8)'
           color: 'var(--text-strong, #e0e0e0)'
           border: '1px solid rgba(255, 255, 255, 0.1)'
           whiteSpace: 'nowrap'
line 2:    className="mt-0.5 opacity-70"
```

- **States**: visible while `dragonfruit:rotation-hint` reports `visible: true`.
- **Callbacks**: none.
- **Text** (both `<Trans>`, msgids verbatim):
  1. `Drag to rotate`
  2. `Dial marks pull at 5° / 10° / 45°`
- **Port target**: absent.

---

## Surface: `GizmoScale` — one scale cube

- **Source**: `src/components/gizmo/scale/GizmoScale.tsx:39`.
- **Renders**: an invisible pick box, a solid cube at the handle position, a wire
  edge overlay of that cube, and a point light. The commented-out connection line
  (`:143-152`, "DISABLED to avoid overlap with move arrows") is dead code.
- **Layout & styling**:

```js
const length = GIZMO_SIZES.scaleLineLength;                 // 2.3
const offset = pointsTowardCamera ? length : -length;
position = x ? [offset,0,0] : y ? [0,offset,0] : [0,0,offset];
rotation = x ? [0,0,PI/2] : y ? [0,0,0] : [PI/2,0,0];        // "Rotate hexagon to face perpendicular to axis"
pickBoxGeometry = BoxGeometry(scaleHexagonRadius*2.3, …, …)  // 1.15 cube
handleBoxGeometry = BoxGeometry(1,1,1)
```

Handle meshes scale by `GIZMO_SIZES.scaleHexagonRadius * highlightScale`
(`0.5 × {1.14 active | 1.08 hovered | 1.0}`). Edge colour
`new THREE.Color(handleColor).multiplyScalar(0.3)`.

```js
const baseOpacity = isHidden ? 0 : isDimmed ? 0.15 : isHighlighted ? 1.0 : 0.9;
const handleColor = isDimmed ? '#cccccc' : isActive ? GIZMO_COLORS.active
                  : effectiveHovered ? GIZMO_COLORS.hover : axisColors.end;
```

Pick box: `renderOrder={1000}`,
`<meshBasicMaterial visible={false} depthTest={false} />`; `onContextMenu` stops
propagation **and** calls `nativeEvent.preventDefault()`.

Point light rendered whenever `!isDimmed`.

- **Data**: `usePicking()` as `scale-<axis>`; `useThree()` camera + gl.
- **Callbacks / drag**:
  - Press: ignore button 2 / `!interactionsEnabled`; `isUniform = isUniformProp`;
    `startDistance = max(px distance from projected gizmo centre, 1e-6)`;
    `startDirection = (deltaX, deltaY) / startDistance`; then
    `onDragStart(isUniform)` (abort on `false`).
  - Move: recompute the projected centre each event, then

```js
const projectedDistance = currentDeltaX*startDirection.x + currentDeltaY*startDirection.y;
const ratio = projectedDistance / Math.max(startDistance.current, 1e-6);
return Math.max(0.01, Math.min(100.0, ratio));
```

  then `onDrag(factor, isUniformScale)`. `isUniformScale` is the state captured
  at press.
  - Window `pointerup` → `onDragEnd()`.
- **Text**: none.
- **Port target**: `gpui/src/gizmo/` — `drag.rs` scale branch ("`GizmoScale`'s
  screen-distance ratio along the press direction, clamped to 0.01…100 and
  multiplied into the scale the drag found (`applyScaleFactor`)") and
  `geometry.rs` (cube + edge prisms). Uniform-only.

---

## Surface: `applyScaleFactor` — pure scale helper

- **Source**: `src/components/gizmo/scale/applyScaleFactor.ts:5`.

```js
const AXIS_INDEX = { x: 0, y: 1, z: 2 };
export function applyScaleFactor(initial, axis, factor) {
  const clamped = Math.max(0.0001, factor);          // floor only, no ceiling here
  if (axis === 'uniform') return initial.clone().multiplyScalar(clamped);
  const i = AXIS_INDEX[axis];
  return initial.clone().setComponent(i, initial.getComponent(i) * clamped);
}
```

Covered by `src/components/gizmo/scale/__tests__/applyScaleFactor.test.ts`.

- **Port target**: the clamp and multiply live inline in `gpui/src/gizmo/drag.rs`.

---

## Surface: `MeshShaderPreviewSlot` — settings preview placeholder wrapper

- **Source**: `src/components/settings/meshSettings/MeshShaderPreviewSlot.tsx:6`.
- **Renders**: `<div className="w-full h-full relative">` wrapping
  `MeshShaderPreviewCanvas`, forwarding 15 props verbatim: `shaderType`,
  `matcapVariant`, `flatUseVertexColors`, `meshColor`, `materialRoughness`,
  `previewModel`, `ambientIntensity`, `directionalIntensity`, `xrayOpacity`,
  `heatmapMinAngle`, `heatmapMaxAngle`, `heatmapColors?`, `hoverTintStrength`,
  `selectedTintStrength`.
- **States / Callbacks / Text**: none of its own.
- **Port target**: `gpui/src/features/settings/mesh.rs` — the shader section's
  preview slot is currently a filled box labelled with the shader name
  (`// Preview slot: the source embeds a wgpu canvas; here the slot is filled with
  the mesh colour and labelled with the shader it would render.`).

---

## Surface: `MeshShaderPreviewCanvas` — the live shader preview

- **Source**: `src/components/settings/meshSettings/MeshShaderPreviewCanvas.tsx:479`.
- **Renders**: an R3F `Canvas` with a Z-up orbiting preview of one built-in
  primitive or an STL, lit by four lights plus a camera headlight.

### Canvas

```jsx
<Canvas
  gl={{ alpha: true, antialias: true }}
  camera={{ position: [0, -cameraDistance, 0], fov: 35 }}
  dpr={[1, 2]}
  onPointerMissed={() => { onHoverChange?.(false); onCanvasPress?.(); }}
>
```

`const cameraDistance = previewModel === 'knot' ? 7.2 : 5.6;`

`ZUpPreviewCamera` sets `camera.up = (0,0,1)`, `position (0,-distance,0)`,
`lookAt(0,0,0)` and `updateProjectionMatrix()` on mount / distance change.

`OrbitControls`:

```jsx
enablePan={false} enableZoom={false} enableRotate autoRotate autoRotateSpeed={0.6}
enableDamping dampingFactor={0.08} rotateSpeed={0.9}
```

### Lights (`PreviewContent`)

```jsx
<ambientLight intensity={ambientIntensity} />
<directionalLight position={[0, 0, 12]} intensity={directionalIntensity} />
<directionalLight position={[0, 0, -12]} intensity={directionalIntensity * 0.15} />
<hemisphereLight args={['#ffffff', '#444444', ambientIntensity * 0.6]} />
<CameraHeadlight intensity={1.0} />   // pointLight at camera.position, decay 0, distance 0, #ffffff
```

### Preview target

```js
const isStl = previewModel.startsWith('stl:');
const stlUrl = isStl ? previewModel.slice('stl:'.length) : null;
const builtinShape = previewModel === 'sphere' ? 'sphere' : previewModel === 'knot' ? 'knot' : 'cube';
```

| shape | geometry |
|---|---|
| `cube` | `new THREE.BoxGeometry(1.6, 1.6, 1.6, 1, 1, 1)` |
| `knot` | `new THREE.TorusKnotGeometry(1, 0.35, 160, 24)` |
| `sphere` (default) | `new THREE.SphereGeometry(1.1, 48, 32)` |

then `geometry.computeVertexNormals()` and
`applyUniformVertexColor(g, new THREE.Color(meshColor))` — a per-vertex colour
attribute filled with the mesh colour's r/g/b, so the `useVertexColors` shader
path shows the mesh colour.

STL: `useLoader(STLLoader, url)` → `clone()` → `computeVertexNormals()` →
`normalizeGeometryToUnitSize`:

```js
geometry.computeBoundingBox(); maxDim = max(size.x,size.y,size.z);
if (!Number.isFinite(maxDim) || maxDim <= 0) return;
geometry.center();
const scale = 1.5 / maxDim;
geometry.scale(scale, scale, scale);
```

Both variants recompute the colour attribute in an effect when `meshColor`
changes and set `geom.attributes.color.needsUpdate = true`.

### Shader branch

`shaderType === 'opaque_wire_mesh'` renders a `<group>` with two meshes: the
`MeshShaderMaterial` forced to `shaderType={'soft_clay'}` and a second mesh with
`<OpaqueWireOverlayMaterial clippingPlanes={[]} />` at `renderOrder={1}`.
Otherwise a single `MeshShaderMaterial` with the real `shaderType`.

Every material receives: `isSelected`, `isHovered`, `useVertexColors`,
`hoverTintColor`, `selectedTintColor`, `meshColor`, `matcapVariant`,
`flatUseVertexColors`, `materialRoughness`, `clippingPlanes={[]}`, `xrayOpacity`,
`heatmapMinAngle`, `heatmapMaxAngle`, `heatmapColors`, `hoverTintStrength`,
`selectedTintStrength`.

- **States**: hover — `onPointerOver` → `onHoverChange(true)`, `onPointerOut` →
  `onHoverChange(false)` (both `stopPropagation()`); click — `onClick` →
  `onPress()`; empty canvas — `onPointerMissed` → `onHoverChange(false)` +
  `onCanvasPress()`. Defaults at the canvas boundary: `useVertexColors = true`,
  `isSelected = false`, `isHovered = false`.
- **Data**: props only. `MeshShaderMaterial`, `MatcapVariant`, `MeshShaderType`
  from `@/features/shaders/mesh`; `OpaqueWireOverlayMaterial` from
  `@/features/shaders/mesh/opaqueWireMesh`.
- **Callbacks**: `onHoverChange(hovered)`, `onPress()`, `onCanvasPress()`.
- **Text**: none.
- **Port target**: `gpui/src/features/settings/mesh.rs` owns the shader tab today;
  the preview slot is a static stand-in. The same file already carries `shader`,
  `preview_model`, `SHADER_OPTIONS`, `PREVIEW_MODEL_OPTIONS` and the
  selection/hover preview card.

---

## Surface: `ArrangePanel` — the Arrange tool card

- **Source**: `src/components/controls/ArrangePanel.tsx:58`.
- **Renders**: `Card` → header (collapse chevron, title `Arrange`, right badge
  `LayoutGrid` icon + model count) → body of stacked sub-cards: Layout Mode,
  Plate Mode (auto only), Arrange Mode (auto only), distance *or* array grid,
  Placement Anchor, Allow Z-rotation (auto only), and the two action buttons.
- **Layout & styling**:

```js
const panelCardStyle   = { borderColor: 'var(--border-subtle)', background: 'var(--surface-1)' };
const disabledActionStyle = {
  background:   'color-mix(in srgb, var(--surface-1), black 8%)',
  borderColor:  'color-mix(in srgb, var(--border-subtle), black 10%)',
  color:        'color-mix(in srgb, var(--text-muted), var(--surface-2) 18%)',
};
const accentCardStyle  = { borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 76%)',
                           background:  'color-mix(in srgb, var(--accent), var(--surface-1) 95%)' };
const activeModeStyle  = { borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 30%)',
                           background:  'color-mix(in srgb, var(--accent), var(--surface-1) 85%)',
                           color:       'var(--text-strong)' };
const disabledModeStyle = {
  borderColor: 'color-mix(in srgb, var(--border-subtle), black 10%)',
  background:  'color-mix(in srgb, var(--surface-1), black 8%)',
  color:       'color-mix(in srgb, var(--text-muted), var(--surface-2) 18%)',
};
```

Header badge: `inline-flex items-center gap-1 rounded-full border px-2 py-0.5`,
`borderColor: 'var(--border-subtle)'`; icon `className="w-3 h-3"` with
`color: 'var(--accent)'`; count `text-[10px]` with `color: 'var(--text-muted)'`.

Body: `px-2 pb-2 space-y-2 sm:px-2.5 sm:pb-2.5`.

Section labels (`Layout Mode` / `Plate Mode` / `Arrange Mode` /
`Placement Anchor`): `ui-meta mb-1` (or `ui-meta`) with
`color: 'var(--text-muted)'`.

Mode buttons (all of them):

```
variant="secondary" size="auto"
className="!h-8 whitespace-nowrap px-1.5 text-[10px] sm:text-[11px]"
disabled={isApplying}
style={isApplying ? disabledModeStyle : (selected ? activeModeStyle : undefined)}
```

Distance field: `ScrollableNumberField` (`className="mt-1"`, `min={-50}`,
`max={5}`, `step={0.1}`, `unit="mm"`, `disabled={isApplying}`,
`ariaLabel="Arrange Distance"`, `decreaseTitle="Decrease spacing"`,
`increaseTitle="Increase spacing"`). That component renders
`[−] [value box with mm suffix] [+]` with 32px `--surface-0` steppers, commits on
blur/Enter/arrow keys, reverts on Escape, and steps on wheel only when focused or
Shift-held.

Array grid: header row
`grid grid-cols-[24px_minmax(0,1fr)_minmax(0,1fr)] gap-1 items-center text-[10px] uppercase tracking-wide mb-1`
with `Count` / `Gap` (`context="Arrange array spacing"`). Per axis (`X:`, `Y:`,
`Z:` at `text-[11px] font-semibold`), two `MiniStepperField`s (32px,
integer-clamped, `--surface-0`).

Anchor `Select`: `className="mt-1 w-full !h-8 px-2 text-xs"`, options `Center`
(`context="Placement anchor"`), `Front Left`, `Front Right`, `Back Left`,
`Back Right` → `ArrangeAnchorMode`.

Allow Z-rotation row: a full-width `<button>` —
`className="w-full rounded-md border px-2 py-2 text-left transition-colors disabled:opacity-60"`,
`style={allowRotateOnZ ? accentCardStyle : panelCardStyle}`,
`disabled={isApplying || precisionMode === 'high_precision'}`. Inside: `RotateCw`
`h-3.5 w-3.5` coloured
`precisionMode === 'high_precision' ? 'var(--text-muted)' : 'var(--accent)'`,
label `text-xs font-medium` `--text-strong`, and a pill
`rounded-full border px-2 py-0.5 text-[10px]` with
`borderColor: 'var(--border-subtle)'`,
`color: allowRotateOnZ ? 'var(--accent)' : 'var(--text-muted)'`,
`background: allowRotateOnZ ? 'color-mix(in srgb, var(--accent), transparent 88%)' : 'transparent'`,
text `ON (Required)` / `ON` / `OFF`.

Action buttons:
`className="w-full !min-h-8 px-1.5 py-1 text-[10px] sm:text-[11px] whitespace-normal text-center leading-tight"`,
`size="sm"`, `style={disabled ? disabledActionStyle : undefined}`. Busy label
replaces the text with
`<span className="inline-flex items-center gap-1.5"><Spinner /><Trans>Arranging…</Trans></span>`.

- **States**:

```js
const isArrangeAllDisabled      = modelCount <= 0 || isApplying || disableArrangeActions;
const isArrangeSelectedDisabled = selectedModelCount === 0 || isApplying || disableArrangeActions;
```

`Arrange All` is `primary` when enabled / `secondary` when disabled;
`Arrange Selected` is `accent` when enabled / `secondary` when disabled.

`Layout Mode` buttons are `disabled={isApplying}` only (not
`disableArrangeActions`). Plate Mode and Arrange Mode cards render only when
`layoutMode === 'auto'`; the array grid replaces the distance field when
`layoutMode === 'array'`.

- **Data**: fully controlled — every value is a prop; no store reads. The parent
  (`PreparePanelStack.tsx:314`) holds `arrangePrecisionMode`,
  `arrangePlateFillMode`, `arrangeLayoutMode`, `arrangeSpacingMm`,
  `arrangeAllowRotateOnZ`, `arrangeArrayCount{X,Y,Z}`, `arrangeArrayGap{X,Y,Z}`,
  `arrangeAnchorMode`.
- **Callbacks** and observable results:
  - `onLayoutModeChange('auto'|'array')` — swaps the body between distance and
    array grid.
  - `onFillModeChange('plates'|'plate')`.
  - `onPrecisionModeChange('standard')`; `'high_precision'` also calls
    `onAllowRotateOnZChange(true)` in the same click (`:232-235`).
  - `onSpacingMmChange` — sanitised: non-finite → `0.5`, rounded to 0.1,
    clamped to `[-50, 5]`.
  - `onArrayCount*Change(next)` — clamped `1..64`, rounded.
  - `onArrayGap*Change(next)` — clamped `-120..120`, rounded (negative gaps nest
    copies).
  - `onAnchorModeChange(mode)`.
  - `onAllowRotateOnZChange(!allowRotateOnZ)`.
  - `onApplyAll()` / `onApplySelected()` → the parent picks the engine by
    `layoutMode`/`precisionMode` (`handleManualArrayArrangeModels` /
    `handleHighPrecisionArrangeModels` / `handleAutoArrangeModels`).
- **Text** (msgids verbatim):

  - `Arrange` — `_(msg\`Arrange\`)`
  - `{count, plural, one {# model} other {# models}}` — module-scope
    `formatModelCount` using
    `msg\`${plural(count, { one: '# model', other: '# models' })}\``. Comment:
    `A module-scope helper because it interpolates: React Compiler renames inline msg locals in production.`
  - `Layout Mode`, `Auto`, `Manual`
  - `Plate Mode`, `Multi-Plate`, `This Plate`
  - `Arrange Mode`, `Standard`, `High-Precision`
  - `Arrange Distance`
  - `Count`, `Gap`
  - `Placement Anchor`, `Center`, `Front Left`, `Front Right`, `Back Left`,
    `Back Right`
  - `Allow Z-rotation`, `ON (Required)`, `ON`, `OFF`
  - `Arrange All`, `Arrange Selected`, `Arranging…`
  - Tooltips: `Use the plates the scene already has before adding more` (comment:
    `Tooltip on the Arrange panel's plate mode choice. It fills partly used plates
    around the models already on them.`), `Arrange only the plate being worked on`
    (comment: `… Overflow goes onto new plates, and no other plate is touched.`),
    `Current arrange algorithm`, `Hull-based SAT packing for tighter fit`,
    `High-Precision mode requires Z-rotation and keeps it enabled`,
    `Allow auto-arrange to rotate models by 90° on Z when beneficial`,
    `Reduce Total Copies to 1 before arranging`,
    `Need at least 2 visible models to arrange`,
    `Arrange all visible models`, `Select a model to arrange selected`,
    `Arrange selected models only`, `Decrease spacing`, `Increase spacing`.

### Port target

`gpui/src/features/arrange_panels.rs` — `ArrangeAnchorMode`,
`ArrangeLayoutMode`, `ArrangePrecisionMode`, `ArrangePlateFillMode`,
`ArrangeState`, `ArrangeActions`, `arrange_body`, `array_grid`,
`allow_z_rotation`, `format_model_count`, `header_badge`, `mode_button`,
`mode_grid`. Recorded deviations: steppers instead of typing, no
`transition-colors`, no `tracking-wide`, `title` tooltips recorded on each control
but not rendered, `select_field` instead of the OS `<select>`, `uppercase`
applied to the strings. `DuplicatePanel` shares the same module.

---

## Surface: `AutoRotationPanel` — Auto Orientation (Beta)

- **Source**: `src/components/controls/AutoRotationPanel.tsx:104`.
- **Renders**: `Card` → header (chevron + title `Auto Orientation (Beta)`) → body
  that flips between two shapes: normal mode (Orient + Blockers buttons, objective
  select, error line) and blocker-paint mode (instruction card, brush-size slider,
  Clear/Done row, error line).
- **Layout & styling**:

```
body:        className="px-2.5 pb-3 space-y-2.5"
paint card:  className="rounded-md border p-2 space-y-1.5 text-center min-h-[4.5rem] box-border"
             borderColor: 'var(--accent-secondary-action-border)'
             background:  'var(--accent-secondary-action-bg-92)'
paint title: className="ui-meta font-semibold", style BLOCKER_ACTION_TEXT
             = { color: 'var(--accent-secondary-action-color)', fontWeight: 600 }
paint copy:  className="text-[10px] leading-snug line-clamp-2", color 'var(--text-muted)'
button row:  className="flex gap-1.5"   (each button "flex-1 !h-8 text-[11px] disabled:opacity-50")
```

Orient button style: `{ borderColor: 'var(--accent)',
background: 'color-mix(in srgb, var(--accent), var(--surface-0) 86%)',
color: 'var(--accent)' }`. Blockers button is `variant="secondary"`.
Brush row: label `ui-meta flex justify-between` (name left,
`fmt.quantity(brushSizeMm, 1, 'mm')` right), `<input type="range"
className="ui-range" step={0.5}>` between `SUPPORT_BLOCKER_BRUSH_MIN_MM` and
`SUPPORT_BLOCKER_BRUSH_MAX_MM`. Objective `Select`:
`style={{ textAlign: 'center', background: 'var(--surface-1)' }}`. Error line:
`className="text-[11px]"`, `color: '#f87171'`.

- **States**:
  - `blockersActive` → paint layout; otherwise the two-button + select layout.
  - `busy` → both buttons, the slider and the select disabled; Orient label
    becomes `Analyzing…`.
  - `!activeModelId` → buttons and controls disabled.
  - `blockedCount === 0` → Clear disabled (`getSupportBlockedCount(activeModelId)`).
  - `error` set → the red paragraph.
  - A stale error is cleared by an effect on `[activeModelId, objective]`
    (`// A new model or goal clears a stale error.`).
- **Data**: `useLocaleFormat()`; the support-blocker store
  (`getSupportBlockersVersion` via `useSyncExternalStore`,
  `getSupportBlockerBrushSizeMm`, `setSupportBlockerBrushSizeMm`); geometry via
  `getModelMesh`; suggestions via `suggestOrientationForGeometry` /
  `composeOrientationDelta`; brush bounds from `supportBlockers`. A module-level
  busy flag with `getOrientationBusy()` / `subscribeOrientationBusy()` lets the
  page-level "Orienting Model" modal read it; the setter notifies only on change.
- **Callbacks**:
  - `handleOrient()` (`:125`): clears the error; `!activeModelId` → error
    `Load a model to get an orientation suggestion.`; `!onApplyRotation` → return;
    sets busy; `requestAnimationFrame(() => setTimeout(…, 0))` so the modal paints
    first. Inside: bake the live `currentRotation` into a **copy** of the position
    array (`new Float32Array(position.array …)`, comment `never mutate the live
    geometry`), read `index` and `getSupportBlockedTriangles`, then
    `suggestOrientationForGeometry({attributes:{position:{array:baked}}, index},
    {objective, …(blocked.size > 0 ? {blockedTriangleIndices: blocked} : {})})`.
    No result → error `Active model has no readable geometry.`. Improvement test:
    `objective === 'height' ? result.suggested.heightMm < result.baseline.heightMm
    : result.suggested.cost < result.baseline.cost`. Not improved →
    `onOrientationReport({status:'already-optimal', modelName})` with no apply
    (comment: `The scene apply path records history unconditionally, so only apply
    a strict improvement — never a no-op rotation.`). Improved → build `doApply`
    (`onApplyRotation(modelId, composeOrientationDelta(currentRotation,
    result.rotXDeg, result.rotYDeg))` then report `status:'applied'`), gated by
    `onBeforeOrientApply(doApply)` when provided (`true`/absent → apply now). Busy
    is cleared in `finally`.
  - `onToggleBlockers()` — Blockers and Done buttons.
  - `clearSupportBlockers(activeModelId)` — Clear.
  - `setSupportBlockerBrushSizeMm(parseFloat(e.target.value))` — slider.
  - Objective change → local `setObjective`.
- **Exports besides the component**: `getOrientationBusy`,
  `subscribeOrientationBusy`, `OrientElapsed` (a 250 ms-interval `m:ss` line
  `Elapsed: {elapsed}` for the modal), `AutoRotationPanelProps`.
- **Text** (module-level msgids): `Auto Orientation (Beta)`, `Orient Model`,
  `Fewest Supports`, `Shortest Print Time`, `Least Scarring`,
  `Load a model to get an orientation suggestion.`,
  `Active model has no readable geometry.`, `Blockers`, `Clear`, `Done`,
  `Paint areas to keep support-free. Orientation avoids them.`,
  `Paint nogo areas for supports. Blocked contact is refused when generating
  supports and avoided when orienting.`, `Blocker Painting Mode`, `Brush Size`,
  `Analyzing…`. The Trans with placeholders (comment: `Sits under the Blocker
  Painting Mode heading. {clearLabel} and {doneLabel} are the Clear and Done
  button labels; keep them as placeholders so the two always match the buttons.`):
  `Press {clearLabel} to reset all, or {doneLabel} to apply.`
- **Port target**: `gpui/src/features/transform_panel.rs` (`OrientationObjective`,
  `OrientationState`, `OrientationActions`, `orientation_body`,
  `SUPPORT_BLOCKER_BRUSH_MIN_MM`/`MAX_MM`). Recorded deviations: the orientation
  sweep is scene work and the panel only reports intent; `busy`/`error` are
  caller-owned; the range input renders as a `.ui-range` track with a knob at the
  current value but is not draggable.

---

## Surface: `MeshSmoothingSettingsPanel` — the Smooth tool card

- **Source**: `src/features/mesh-smoothing/MeshSmoothingSettingsPanel.tsx:40`.
- **Renders**: `Card` → header (chevron + `Mesh Smoothing`) → scrolling body with
  a Brush sub-card, a Highlight sub-card, and a full-width "Restore defaults"
  button.
- **Layout & styling**:

```js
const brushCardStyle     = { borderColor: 'color-mix(in srgb, #4f8cff, var(--border-subtle) 78%)',
                             background:  'color-mix(in srgb, #4f8cff, var(--surface-1) 94%)' };
const highlightCardStyle = { borderColor: 'color-mix(in srgb, #8f6cff, var(--border-subtle) 80%)',
                             background:  'color-mix(in srgb, #8f6cff, var(--surface-1) 95%)' };
const valueInputClass    = 'ui-input h-8 w-full px-2 text-xs sm:text-sm text-center tabular-nums no-spinners';
```

Card `className="w-full overflow-x-hidden shadow-xl"`. Body
`className="px-2.5 pb-2.5 space-y-2 max-h-[calc(100vh-var(--topbar-height)-88px)] overflow-y-auto custom-scrollbar"`.
`SectionHeader` identical to the Transform panel's helper.

Brush fields (all `NumberInput`, `showStepper={false}`, suffix spans
`pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-[10px] font-semibold`
`color: 'var(--text-muted)'`):

| Field | Value | onChange | Suffix |
|---|---|---|---|
| brush size | `settings.brushSizeMm` | `clampMeshSmoothingBrushSizeMm(val)` | `mm` |
| strength | `settings.strength` | `Math.min(1, Math.max(0, val))`, `step={0.01}` | `Strength` |
| iterations | `settings.iterations` | `Math.round(Math.min(20, Math.max(1, val)))`, `step={1}` | `Iterations` |

The suffix slot doubles as the field's label for strength/iterations.

Falloff row: `grid grid-cols-3 gap-1.5`, one button per tier:

```
className="min-h-[36px] rounded-md border px-3 text-[12px] font-semibold uppercase tracking-wide transition-colors"
active:   { borderColor: 'color-mix(in srgb, var(--accent), white 10%)',
            background:  'color-mix(in srgb, var(--accent), var(--surface-1) 84%)',
            color:       'color-mix(in srgb, var(--accent), var(--text-strong) 25%)' }
inactive: { borderColor: 'var(--border-subtle)', background: 'var(--surface-1)', color: 'var(--text-muted)' }
```

Highlight sub-card: swatch `className="h-8 w-8 shrink-0 rounded-md border"` with
`background: settings.highlightColor`,
`borderColor: 'color-mix(in srgb, var(--border-subtle), white 8%)'`; hex input
`className="ui-input flex-1 min-w-0 h-8 text-xs uppercase"` with
`placeholder="#269EFF"`, showing `clampedColorInput` (`raw.startsWith('#') ?
raw.toUpperCase() : '#' + raw.toUpperCase()`); then the picker container
`className="h-28 rounded-md border p-1 overflow-hidden"` `data-no-drag="true"`
with `borderColor: 'var(--border-subtle)'`,
`background: 'color-mix(in srgb, var(--surface-0), transparent 6%)'`, holding
`<HexColorPicker data-no-drag="true" … style={{width:'100%', height:'100%'}} />`.

Restore button: `variant="secondary"`, `size="auto"`,
`className="w-full !h-8 px-3 text-xs inline-flex items-center justify-center gap-1.5"`,
`<RotateCcw className="w-3 h-3" />` + `Restore defaults`,
`onClick={() => setMeshSmoothingSettings({ ...DEFAULT_MESH_SMOOTHING_SETTINGS })}`.

- **States**: `expanded` from a plain `useState(true)` here (not
  `useFloatingPanelCollapse` — the only panel in this area that does its own). No
  disabled state; every control is always live.
- **Data**: module store `./settings`:

```ts
MESH_SMOOTHING_BRUSH_SIZE_MM = { min: 0.1, max: 0.5, step: 0.01 }
DEFAULT_MESH_SMOOTHING_SETTINGS = { brushSizeMm: 0.5, strength: 0.5,
  highlightColor: '#269eff', falloff: 'smooth', iterations: 1 }
STORAGE_KEY = 'mesh-smoothing-settings'
```

Every write goes through `setMeshSmoothingSettings`, which runs `clampSettings`:
brush to `[0.1, 0.5]`, strength to `[0, 1]`, iterations `1..20` rounded, colour
lower-cased `#rrggbb` or the default unless it matches `/^#?([0-9a-fA-F]{6})$/`.
Mount loads from `localStorage` and subscribes; an effect saves on every change of
`[brushSizeMm, strength, highlightColor, falloff, iterations]`.

- **Callbacks**: `updateMeshSmoothingSettings(patch)` per field,
  `setMeshSmoothingSettings(defaults)` for the reset.
- **Text**: `Mesh Smoothing` (`<Trans>`), `Brush` (`msg`), `Highlight` (comment:
  `Section heading: the colour that marks the area the smoothing brush touches.`),
  `Strength`, `Iterations`, `Restore defaults`, and the falloff labels with context
  `Brush falloff`: `Linear` (comment `Smoothing brush falloff: strength fades
  evenly from the centre to the edge. Rendered uppercase on a narrow button.`),
  `Smooth` (comment `… strength fades along a soft curve. Adjective, not the verb.
  …`), `Sharp` (comment `… strength drops quickly away from the centre. …`).
- **Port target**: `gpui/src/features/smoothing_panel.rs` —
  `MeshSmoothingFalloff`, `SmoothingState`, `SmoothingActions`, `smoothing_body`,
  `falloff_button`, `clamped_color_input`, `hsv`/`hue_color`, and
  `MESH_SMOOTHING_BRUSH_MIN_MM`/`MAX_MM`/`STEP_MM`. Deviations: chevrons instead
  of typing, the hex field is a value box, the picker is a painted 112px gradient
  container with a knob (not draggable), `vh` cap dropped, persistence is the
  caller's job.

---

## Surface: `OrganicCutPanel` — the Split tool card

- **Source**: `src/features/organicCut/OrganicCutPanel.tsx:247`.
- **Renders**: `Card className="flex flex-col"` → header (chevron, title `Split`,
  help tooltip button, and on the right the preview eye + animated reset) → a
  scrollable body (Cut Mode card, cut-settings grid, registration-tenon card,
  Loops card, Snap to Edges) → a pinned action row (error banner + Clear All /
  Cut).

### Constants (`:16-52`)

```js
TENON_DIM_MAX_MM = 20;
FRUSTUM_MIN_WIDTH_MM = 1;
DOME_MIN_WIDTH_MM = 1.5;
TENON_TOP_SCALE = 0.5;
TENON_BASE_OVERLAP_MM = 0.3;
TENON_FILLET_STEP_MM = 0.1;
TENON_TOLERANCE_MAX_MM = 1;
```

`maxTenonFilletMm(widthMm, depthMm)`:

```js
const topHalfWidth = widthMm * TENON_TOP_SCALE * 0.5;
const height = depthMm + TENON_BASE_OVERLAP_MM;
const limit = Math.min(topHalfWidth * 0.999, height / 3);
const stepped = Math.floor(limit / TENON_FILLET_STEP_MM) * TENON_FILLET_STEP_MM;
return Math.max(0, Number(stepped.toFixed(2)));
```

Comments to keep: the frustum comment (`build_frustum clamps the radius to the
smallest half-extent … and to a third of the height … Offering the full 0–5mm
regardless meant that on a 2mm-wide tenon everything from 0.5mm up produced the
SAME clamped geometry: the field looked dead in both directions.`), the frustum
floor comment (`This field IS the floor now: Rust no longer has one of its own —
it builds what it is asked for and only says whether it fits`), and the dome floor
comment (`width is a diameter here: Rust stores a dome as semi-axes, so 1.5mm of
width is a 0.75mm radius — below that a hemisphere stops locating anything.`).

### State shape (`OrganicCutPanelState`)

`cutMode`, `jointClearanceMm`, `smoothing`, `membraneSmoothing`, `density`,
`generateTenon`, `tenonWidthMm`, `tenonDepthMm`, `tenonShape`
(`'frustum'|'dome'`), `tenonFilletMm`, `tenonToleranceMm`,
`tenonAnchor: [x,y,z] | null`, `tenonUniformScale`, `tenonSwapSides`,
`tenonTiltRad`, `tenonRollRad`, `showPreview`.

Defaults (`useOrganicCutSession.ts:411`): `cutMode: 'contour'`,
`jointClearanceMm: 0.0`, `smoothing: 1.0`, `membraneSmoothing: 1.0`,
`density: 4.0`, `generateTenon: false`, `tenonWidthMm: 2.0`,
`tenonDepthMm: 2.5`, `tenonShape: 'frustum'`, `tenonFilletMm: 0.2`,
`tenonToleranceMm: 0.1`, `tenonAnchor: null`, `tenonUniformScale: true`,
`tenonSwapSides: false`, `tenonTiltRad: 0`, `tenonRollRad: 0`,
`showPreview: true`.

### Styles

```js
const cardStyle       = { borderColor: 'var(--border-subtle)', background: 'var(--surface-1)' };
const accentCardStyle = { borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 76%)',
                          background:  'color-mix(in srgb, var(--accent), var(--surface-1) 95%)' };
const activeModeStyle = { borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 30%)',
                          background:  'color-mix(in srgb, var(--accent), var(--surface-1) 85%)',
                          color:       'var(--text-strong)' };
const disabledStyle   = disabled ? { opacity: 0.45, filter: 'grayscale(0.7)' } : undefined;
const cardShellStyle  = { ...disabledStyle,
                          ...(expanded ? { maxHeight: 'calc(100vh - var(--topbar-height) - 140px)' } : {}) };
```

Header help: `Tooltip maxWidth={280}` wrapping a button
`className="inline-flex cursor-help items-center justify-center p-0.5 transition-colors"`
`style={{ color: 'var(--text-muted)' }}`, `aria-label="How to use Split"`
(comment: `Accessible name of the help toggle in the Split panel; the tool is
called Split in the tool rail.`), `<CircleHelp className="h-4 w-4" />`.

Header-right (only when expanded): `div className="flex items-center gap-1"` with

- preview eye: `IconButton className="!p-0.5 transition-colors disabled:opacity-40"`,
  `disabled={disabled || isApplying}`, title `Hide the cut preview` /
  `Show the cut preview`, aria-label `Toggle cut preview`; icon `Eye`
  `h-3.5 w-3.5` `color: var(--accent)` or `EyeOff` with `var(--text-muted)`.
- `AnimatedResetButton` (`:172`): `IconButton` with
  `className="!p-0.5 transition-colors disabled:opacity-40"` plus
  `!text-red-400/70 hover:!text-red-400 hover:!bg-red-600/15` when not animating;
  the icon gets `animate-spin-once text-orange-400` for 600 ms after a click;
  disabled while `!cutSettingsDirty`, `disabled`, or `isApplying`; aria-label
  `Reset cut settings to defaults`; onClick `setState({ ...DEFAULT_CUT_SETTINGS })`.

Body: `className="px-2 space-y-2 sm:px-2.5 overflow-y-auto custom-scrollbar flex-1 min-h-0"`.
Action row: `className="px-2 pb-2 pt-2 sm:px-2.5 sm:pb-2.5"`.

Cut Mode card: `rounded-md border p-2 space-y-1.5` + `accentCardStyle`; label
`ui-meta` in `--text-muted`; two buttons `Contour` / `Flat` with the standard
`!h-8 whitespace-nowrap px-1.5 text-[10px] sm:text-[11px]` recipe and
`style={state.cutMode === X ? activeModeStyle : undefined}`.

Cut settings card: `rounded-md border p-2` + `cardStyle`, inside
`grid grid-cols-2 gap-x-2 gap-y-1.5` of `CompactNumberField`s:

| Field | Value | Range / step | Notes |
|---|---|---|---|
| `Seam Smoothing` | `smoothing` | 0–2, 0.05, 2 decimals | contour only |
| `Joint Clearance` | `jointClearanceMm` | 0–1.5, 0.05, `unit="mm"` | `className={isContour ? undefined : 'col-span-2'}` |
| `Cut Smoothing` | `membraneSmoothing` | 0–2, 0.05 | contour only |
| `Cut Resolution` | `density` | 1–4, 0.5, `unit="×"` | contour only |

`CompactNumberField` = centred `ui-meta` label + bare `NumberInput`
(`h-8 w-full min-w-0 pl-1.5 pr-6 text-xs text-center no-spinners
!bg-[var(--surface-0)]`) with the unit as a `text-[10px]` suffix; it wheel-steps.

Tenon card: `rounded-md border p-2 space-y-1.5` + `cardStyle`; a
`grid grid-cols-3 gap-1` selector:

| Button | onClick payload | Disabled |
|---|---|---|
| `No Tenon` | `{ generateTenon: false }` | `disabled \|\| isApplying` |
| `Frustum` | `{ generateTenon: true, tenonShape: 'frustum', showPreview: true, tenonFilletMm: Math.min(state.tenonFilletMm, maxTenonFilletMm(width, depth)) }` | `… \|\| !canApply` |
| `Dome` | `{ generateTenon: true, tenonShape: 'dome', showPreview: true, tenonWidthMm: Math.max(w, 1.5), tenonDepthMm: Math.max(d, 1.5) }` | `… \|\| !canApply` |

Active when `state.generateTenon && state.tenonShape === X`. The onClick bodies
carry comments explaining the fillet re-clamp (`Only the frustum is filleted, so
the radius can be stale … after a detour through the dome`) and the dome floor
lift; both shape buttons also carry `// Turning a tenon on brings the preview
back: the user is about to configure a tenon they need to see.`

Recentring row (only when `state.tenonAnchor` is non-null):
`flex items-center justify-between gap-2` with a `Moved off centre` label and a
`Center` button (`!h-6 … text-[10px]`) → `setState({ tenonAnchor: null })`,
title `Put the tenon back in the middle of the cut.`

Sizes: `grid grid-cols-2 gap-x-2 gap-y-1.5`; the width/depth pair is a
`col-span-2` grid that is `grid-cols-[1fr_auto_1fr]` for a dome and
`grid-cols-2` for a frustum. The dome adds the link button between them: an
invisible `ui-meta` label (`·`, `aria-hidden`) mirroring the fields' label height,
then a `mt-1 flex h-8 items-center justify-center` wrapper with
`Link2`/`Link2Off` at `h-4.5 w-4.5`, colour `var(--accent)` when
`tenonUniformScale` else `var(--text-muted)`; aria-label
`Toggle linked width and depth`; titles `Unlink width and depth — resize them
independently.` / `Link width and depth together so the dome keeps its shape.`

| Field | Range | Step | Notes |
|---|---|---|---|
| `Tenon Width` | `tenonDimMinMm`–20 | 0.5, mm | `tenonDimMinMm = tenonShape === 'dome' ? 1.5 : 1` |
| `Tenon Depth` | same | 0.5, mm | |
| `Edge Fillet` (frustum only) | 0–`tenonFilletMaxMm` | 0.1, mm | title from `formatTenonFilletTitle` |
| `Fit Tolerance` | 0–1 | 0.05, mm | `className={frustum ? undefined : 'col-span-2'}` |

Flip/reset row: `flex gap-1 pt-1`, both
`flex-1 !min-h-8 whitespace-nowrap px-1.5 text-[10px] sm:text-[11px] disabled:opacity-60`:

- `Flip Tenon` → `setState({ tenonSwapSides: !state.tenonSwapSides })`, title
  `Swap which cut half receives the tenon and which receives the mortise.`,
  enabled style `borderColor: 'color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 62%)'`,
  `color: 'color-mix(in srgb, var(--accent-secondary), var(--text-strong) 55%)'`,
  `background: 'color-mix(in srgb, var(--accent-secondary), var(--surface-1) 95%)'`.
- `Reset` → `setState({ ...DEFAULT_TENON_SETTINGS, generateTenon: state.generateTenon })`
  (comment: `The reset leaves the No Tenon/on choice alone.`), disabled while
  `!tenonSettingsDirty`; enabled style with `#f87171` at `45%` / `30%` / `90%`;
  title `Put every tenon setting back to its default: shape, width, depth, fillet,
  fit tolerance, uniform scale, side and aim.`

Won't-fit alert (only `state.generateTenon && !tenonFits && tenonDetail`):
`rounded border px-2 py-1.5 text-[10px] leading-snug`, border
`color-mix(in srgb, #b3121b, var(--border-subtle) 40%)`, background
`color-mix(in srgb, #b3121b, var(--surface-1) 88%)`, `--text-strong` text.

Loops card (contour only): `rounded-md border p-2 space-y-1.5` + `cardStyle`.
Heading `flex items-center justify-between gap-2`: label
`Loops{loopCount > 1 ? \` (${loopCount})\` : ''}`; trash button
`className="inline-flex h-5 w-5 shrink-0 items-center justify-center rounded transition-colors hover:bg-[var(--surface-2)] disabled:opacity-40"`
with `Trash2 h-3 w-3`, title `Remove the loop you're editing.` /
`A cut needs at least one loop.`, disabled `!canRemoveLoop`. Chips:
`flex flex-wrap items-center gap-1`; each chip is a `Button` with
`className="!h-7 !min-w-7 whitespace-nowrap px-1.5 text-[10px] disabled:opacity-60"`,
`style` = `activeModeStyle` when active,
`{ borderStyle: 'dashed', color: 'var(--text-muted)' }` when `pointCount < 3`,
else none; label `s.index + 1`; title from `formatLoopTitle`. Then a `+` button
(`!h-7 !min-w-7 … text-[11px]`) → `onAddLoop`, disabled `!canAddLoop`.

Snap to Edges: full-width secondary button,
`className="w-full !min-h-8 px-1.5 py-1 text-[10px] sm:text-[11px] whitespace-normal text-center leading-tight disabled:opacity-60"`,
disabled `disabled || isApplying || !canSnapToEdges`, with the long title quoted
below.

Action row: error banner `role="alert"`
`className="mb-2 flex items-start gap-2 rounded border px-2.5 py-2 text-[11px] leading-snug"`,
border `color-mix(in srgb, #b3121b, var(--border-subtle) 25%)`, background
`color-mix(in srgb, #b3121b, var(--surface-1) 80%)`,
`TriangleAlert mt-px h-3.5 w-3.5 shrink-0` in `#ff6b6b`; then `flex gap-2` with
`Clear All` (secondary) and `Cut` (`variant="accent"`), both
`flex-1 !min-h-8 px-1.5 py-1 … disabled:opacity-60`.

### States

```js
const tenonBlocksCut  = state.generateTenon && !tenonFits;
const tenonSettingsDirty = (Object.keys(DEFAULT_TENON_SETTINGS) as …)
  .some((k) => k !== 'generateTenon' && state[k] !== DEFAULT_TENON_SETTINGS[k]);
const cutSettingsDirty   = (Object.keys(DEFAULT_CUT_SETTINGS) as …)
  .some((k) => state[k] !== DEFAULT_CUT_SETTINGS[k]);
```

- `Clear All` disabled unless some loop has points:
  `!loopSummaries.some((s) => s.pointCount > 0)`.
- `Cut` disabled `disabled || isApplying || !canApply || tenonBlocksCut`; its
  `title` is `tenonDetail` when `tenonBlocksCut`; the label switches to
  `Cutting...` with `<Spinner size="xs" />`.
- The collapse toggle is inert while `disabled` (early return in `onToggle`).
- `disabled` also dims the whole card to `opacity: 0.45` (grayscale dropped in the
  port).

### Callbacks

`onStateChange(next)` receives the **whole** next state (the panel's `setState`
helper spreads `state`), `onClearLoop()`, `onSnapToEdges()`, `onSelectLoop(i)`,
`onAddLoop()`, `onRemoveLoop(i)`, `onApply()`.

Dimension bookkeeping inside the panel:

- `setFrustumDim(axis, next)`: clamp `[tenonDimMinMm, 20]` at 1 decimal, then
  `tenonFilletMm: Math.min(state.tenonFilletMm, maxTenonFilletMm(width, depth))`
  (comment: `Shrinking either lowers the fillet ceiling, so the stored radius comes
  down with it — otherwise it stays parked above the new limit and the field is
  dead until the user drags back under it.`).
- `setDomeDim(axis, next)`: unlinked → set just that axis. Linked → clamp the
  **factor**, not each axis:

```js
let factor = clamped / cur;
if (other > 0) {
  factor = Math.min(factor, TENON_DIM_MAX_MM / other);
  factor = Math.max(factor, tenonDimMinMm / other);
}
// then cur*factor and other*factor, each re-clamped
```

  Comment: `Clamping them separately lets the dragged axis keep moving after the
  other has hit 1mm or 20mm, which silently destroys the proportion the lock
  exists to preserve.`

- `clampFloat(value, min, max, decimals = 1)`: non-finite → `min`, then
  `toFixed(decimals)` round, then clamp.

### Text (verbatim msgids)

`Split` (`<Trans comment="Title of the panel that splits one model along a drawn
seam. The tool rail calls this tool Split; keep the two identical.">`),
`How to use Split`, `Hide the cut preview`, `Show the cut preview`,
`Toggle cut preview`, `Reset cut settings to defaults`,
`Put the cut settings back to their defaults: cut mode, thickness, both
smoothings and resolution. Your drawn loops and tenon settings are untouched.`,
`Cut Mode`, `Contour`, `Flat`,
`Split along a curved seam that follows your drawn loop (zero-thickness mate).`,
`Slice along a single flat plane derived from your points.`,
`Seam Smoothing`, `Seam line smoothing strength`,
`Rounds the seam line through the waypoints. A flat cut's seam is the plane ∩
mesh, so this only affects contour cuts.`,
`Joint Clearance`, `Joint clearance in millimeters`,
`Cut Smoothing`, `Cut surface smoothing strength`,
`How smooth/taut the curved cutter surface is — contour cut only.`,
`Cut Resolution`, `Cut mesh resolution multiplier (applied at cut)`,
`Cutter poly count — higher is a denser cut mesh. Contour cut only.`,
`No Tenon`, `Frustum`, `Dome`,
`Don't add a registration tenon — the halves are cut apart with no way to index
them back together.`, `Tapered rectangular tenon — locks the parts against
rotation.`, `Half-sphere tenon — locates the parts but allows rotation.`,
`Moved off centre`, `Center`, `Put the tenon back in the middle of the cut.`,
`Tenon Width`, `Tenon width in millimeters`, `Tenon Depth`,
`Tenon depth in millimeters`, `Edge Fillet`,
`Tenon edge fillet radius in millimeters (0 = sharp)`,
`Rounds the tenon's corners and tip. On this tenon the geometry accepts up to
{maxMm}mm — a wider or deeper tenon raises that ceiling.`,
`Fit Tolerance`, `Tenon to mortise fit tolerance in millimeters (0 = press fit)`,
`Slack between tenon and mortise, on every face. 0 = press fit (needs force).
0.1mm is a slide fit on a well-calibrated printer; raise it if the halves won't go
together.`, `Flip Tenon`, `Swap which cut half receives the tenon and which
receives the mortise.`, `Reset`,
`Put every tenon setting back to its default: shape, width, depth, fillet, fit
tolerance, uniform scale, side and aim.`, `+`, `Snap to Edges`,
`Nudge every waypoint onto the model's nearest sharp edge (crease or boundary),
preferring a corner where several edges meet — for points placed roughly in a
crease or corner. Does nothing on a smooth model with no sharp edges. Double-click
a waypoint to lock it (white cage) so snap leaves it where it is.`,
`Clear All`, `Discard every loop in this cut, not just the active one.`, `Cut`,
`Cutting...`,
`Add another loop and start drawing it. On Cut, every loop is cut together — use
it to free a part attached in several places (e.g. a tail joined at two posts).`,
`Remove the loop you're editing.`, `A cut needs at least one loop.`

Loop chip titles (`formatLoopTitle`):

```
Loop N — P point(s) (needs 3+ to cut) — editing
Loop N — P point(s) (needs 3+ to cut) — click to edit
Loop N — P point(s) — editing
Loop N — P point(s) — click to edit
```

Header help text by mode:

- contour: `Click points around the model to trace a curved seam that splits it in
  two — the cut follows your loop (3+ points). Turn on a Tenon to add a
  tenon/mortise so the halves index together.`
- plane: `Click 2 points across the model to set a flat cut that slices it in two
  along a single plane. Turn on a Tenon to add a tenon/mortise so the halves index
  together.`

### Port target

`gpui/src/features/organic_cut_panel.rs` — `OrganicCutMode`, `TenonShape`,
`OrganicCutState`, `LoopSummary`, `OrganicCutView`, `OrganicCutActions`,
`organic_cut_body`, `clamp_float`, `max_tenon_fillet_mm`, `tenon_fillet_title`,
`loop_title`, `cut_card`/`cut_accent_card`, plus `TENON_DIM_MAX_MM`,
`FRUSTUM_MIN_WIDTH_MM`, `DOME_MIN_WIDTH_MM`, `TENON_TOLERANCE_MAX_MM`.
Recorded deviations: chevrons instead of typing, `grayscale(0.7)` dropped, the
one-shot reset spin and transitions dropped, `vh` cap dropped, `title` tooltips
recorded but not rendered, and the viewport tenon gizmo
(`tenonTiltRad`/`tenonRollRad`/`tenonAnchor`) is scene work — the panel only
offers the recentre control.

---

## Deliberate decisions and gotchas

1. **`color-mix(in srgb, A, B N%)` is `A*(1-N) + B*N`.** The sub-card tints,
   toggle fills, `disabledModeStyle`, the Z-rotation pill and the tenon alert all
   depend on reading the percentage as belonging to the **second** colour. The
   handoff lists this inversion as having produced four visible bugs already.
2. **The Local toggle is one control rendered twice.** Move and Rotate each draw
   `renderLocalSpaceToggle()`; either button flips the same `localSpace` flag. Do
   not give them separate state.
3. **Rotation fields wrap into `(-180, 180]`, not `[-180, 180)`.** The formula is
   `((((deg + 180) % 360) + 360) % 360) - 180`, then `-0` is normalised to `0`. A
   model at exactly `-180°` shows `-180`; at `+180°` it also shows `-180`.
4. **Rotation typing and rotation history are separate events.** `onChange`
   writes the store on every keystroke; `onBlur` is where `onRotationComplete()`
   fires, then `onTransformCommit()`. Committing from `onChange` would create one
   history entry per character.
5. **Position blur is optional-chain-ordered**: `onPositionCommit` if present,
   else `onTransformCommit`. Lift-distance blur uses `onTransformCommit` only.
6. **Scale has two rows that must stay in sync.** The `%` row and the `mm` row
   both write the same `scale`; the mm row divides by `originalSize`, which is
   `(1,1,1)` when `modelBBox` is null — so the `originalSize ? … : 0` ternaries
   never take the zero branch. Do not "fix" them into a disabled state.
7. **Uniform scaling rewrites all three axes from whichever field was edited.**
   In non-uniform mode the other two axes are passed through from the current
   `scale` (not from the edited value), so typing into Y leaves X and Z alone.
8. **`onPlatform` is dead.** Declared, destructured, never invoked and never
   rendered. The Rust port correctly omits it; do not invent a Platform button.
9. **`sectionCardStyle` and `outlinedConfigEmphasisStyle` are dead constants** in
   `TransformControls`; `SectionHeaderProps.accentColor` is also unread.
10. **Arrow and scale handles flip to the camera's side** using the dot product
    with `axisDirection` (`worldAxisDir` when the gizmo is rotated), **not** by
    comparing world components. `GizmoMove` and `GizmoScale` use the same rule so
    the arrow and the cube never sit on opposite sides.
11. **The move drag's axis origin is captured at press.** `gizmoPosition` is
    deliberately excluded from `getAxisParamFromMouse`'s dependencies
    (`// gizmoPosition intentionally excluded — we use dragAxisOriginRef
    (stable).`). Reading the live position makes t-values drift as the model
    follows the pointer.
12. **The centre nub pins Z and reports only XY deltas** (`delta.z = 0`) on a
    plane fixed at the grab's `gizmoPosition.z`. It also has a 24px screen-space
    hover radius computed in `useFrame`, independent of GPU picking — either
    source can highlight it.
13. **The centre disc's opacity is 0.55 at rest**, not the 0.9 the arrows and
    cubes use. Its point light is deliberately absent (`// Point light disabled for
    center disc`, `emissiveIntensity = 0`).
14. **The ring's pick target is the diamond, not the ring.** One invisible sphere
    of `max(0.18, ringDiamondRadius * 0.9 * handleScale)`; the arc has no pointer
    handler. The port repeats this rule and documents why.
15. **The dial is anchored where the handle was grabbed**, and that anchor is both
    the dial's zero and the sweep's zero. The magnet is seeded with the zero mark
    on press so the first pixels do not jump to a neighbour.
16. **`DIAL_MOVE_SLOP_PX = 3` and `ROTATION_CENTER_SINGULARITY_PX = 8` are not the
    same guard.** Slop is a dead zone from the press point (so a click commits
    nothing); the singularity is a radius around the projected centre (so angular
    jitter there is discarded). Both must exist; the source comments explain the
    failure each prevents.
17. **The magnet's hysteresis is symmetric today** (`MAGNET_ENTER_PX ===
    MAGNET_EXIT_PX === 10`), but the two constants exist separately on purpose:
    "a release radius LARGER than the enter radius is what makes the magnet feel
    like a magnet".
18. **The dial's radius shows the *applied* rotation, not the pointer**, except
    when `frameCarriesRotation` is set — then the ring already carries it and the
    radius follows the raw pointer angle (`resolved.angleRad`), because otherwise
    the handle would travel at twice the pointer's speed.
19. **`stepDialSweep` advances the target by what the object *took*.** Past a hard
    end the refused travel stays as a gap so backing off holds the limit. Tracking
    the pointer instead re-engages from the limit — quoted: "the lean's 45-degree
    ceiling, backed off, landing on 15".
20. **`emittedDeltaForSweep` negates.** The consumer (`SceneCanvas`) applies
    `setFromAxisAngle(axis, -angle)`; without the negation the model turns against
    the dial. `axisVisualFlip === -1` inverts it again for consumers whose display
    axis is mirrored (`HolePunchGizmo`'s `displayY = -cutterY`).
21. **`rayToRingLocal` must take the world matrix**, not rotation + position. The
    gizmo root is scaled every frame (`distance * 0.04`); ignoring the scale makes
    the magnet never engage (or engage in the wrong corona).
22. **A gesture that ends with the button still down must swallow the rest of it**
    (`swallowRestOfGesture`). Otherwise the release lands on the model behind the
    gizmo and the Cut tool drops a stray waypoint. The swallowers are capture-phase
    and remove themselves one task later.
23. **The rotation's window listeners filter by pointer *and* button.** A second
    button pressed and released mid-drag used to end the rotation on the spot
    (`// The gesture is this pointer and this button, and nothing else.`). Foreign
    `pointerdown` events are swallowed so a right-drag cannot orbit under a live
    rotation.
24. **`onDrag` may return a number** — "how much the object actually took". Only
    `GizmoRotation` honours it, and only through `stepDialSweep`.
25. **`finishDrag('cancel')` reverses the whole sweep**, and a hard end clamps the
    way back exactly as it clamped the way out, so the object lands on its start
    angle. Escape and `pointercancel` both take this path; unmount takes `commit`.
26. **The dial is mounted in the ring's own frame, outside the camera-following
    arc group.** Comment: `fixed angular positions inside a camera-following frame
    would drift away from the marks they are supposed to measure against.`
27. **`RotationDial`'s sweep group is written imperatively** (`rotation.z`) once
    per pointer sample — "the radius follows the pointer at pointer rate, which is
    no place for a React state update".
28. **`GizmoScale`'s factor is a *ratio*, not a delta.** It projects the current
    cursor offset onto the press direction and divides by the press distance, so
    crossing the centre cannot invert growth. Clamped `0.01 … 100`.
29. **`applyScaleFactor` floors at `0.0001`; the drag clamps at `0.01`.** Two
    different floors at two layers; keep both.
30. **The three scale cubes are three grab points for one uniform operation.**
    With `uniformScaling` on (the source default) each multiplies all three axes —
    the port exposes only that mode but keeps a vector for a future per-axis mode.
31. **The mesh preview's lights are part of the spec.** Ambient + two
    directionals (the back one at `* 0.15`) + hemisphere (`ambient * 0.6`) + a
    camera-pointing headlight at intensity `1.0` with `decay 0`, `distance 0`. Its
    `useVertexColors` is `true` at the canvas boundary and the slot never overrides
    it.
32. **`opaque_wire_mesh` renders two meshes and forces `soft_clay`** on the base
    material; the wire overlay is a second mesh at `renderOrder={1}`.
33. **STL previews are normalised to unit size**: `center()` then
    `scale(1.5 / maxDim)`, and the mesh colour is baked into a per-vertex colour
    attribute so the vertex-colour shader path shows it.
34. **Arrange's spacing is clamped to `[-50, 5]` in 0.1 steps and array gaps to
    `[-120, 120]`** — negative values are intentional (nesting/interlocking), and
    the source says so in a comment. Counts are `1..64` integers.
35. **Choosing `High-Precision` also turns `allowRotateOnZ` on** in the same
    click, and the Z-rotation row becomes disabled while high precision is active
    with the label `ON (Required)`. That ordering is why the two live in one
    handler.
36. **Arrange's action disabling has two layers**: `isApplying` disables the mode
    buttons; `disableArrangeActions` (the parent's "reduce Total Copies to 1" gate)
    disables only the two apply buttons, with a different tooltip.
37. **The "Arranging…" label replaces the button text** with a `Spinner` in an
    `inline-flex` span; it does not merely disable the button.
38. **Orientation applies only a strict improvement.** The scene apply path
    records history unconditionally, so a no-op rotation would pollute undo; the
    panel reports `already-optimal` instead.
39. **Orientation bakes the live rotation into a throwaway copy** of the position
    array and never mutates the live geometry.
40. **The orientation sweep is deferred twice** (`requestAnimationFrame` then
    `setTimeout(0)`) so the busy modal paints before the synchronous sweep blocks
    the thread.
41. **The orientation busy flag is module-level, not component state**, so the
    page-level modal can subscribe through `useSyncExternalStore`.
42. **Smoothing iterations are 1..20 and rounded; the brush is 0.1..0.5 mm.**
    Those clamps live in the settings store (`clampSettings`), not only in the
    panel — the panel clamps `brushSizeMm` explicitly and relies on the store for
    the rest.
43. **The smoothing colour is normalised to lower-case `#rrggbb`** on write, while
    the hex input displays upper case. Both are intentional; the field text is
    derived, not stored.
44. **The Split panel's cut reset deliberately does not touch loops or tenon
    settings** ("losing a seam to a stray click on a reset button would be a cruel
    way to find this button"), and the tenon reset deliberately preserves
    `generateTenon`.
45. **`maxTenonFilletMm` is recomputed from live width/depth** and used as the
    field's `max`; any stored fillet above it is re-clamped whenever a dimension
    changes. A static `0..5` range makes the field look dead.
46. **The dome's dim lock clamps the *factor*, not the axes**, so both ends stop
    together and the proportion survives.
47. **The dome's minimum width (1.5) is above the frustum's (1.0)** because the
    dome's width is a diameter and Rust stores semi-axes.
48. **Split's `Cut` button is blocked by a tenon that will not fit**
    (`tenonBlocksCut`), and the button's tooltip *is* `tenonDetail`. Rust would
    refuse the tenon anyway; the panel stops it at the button next to the red
    tenon.
49. **Split's body scrolls but the actions are pinned outside the scroll area** —
    `Cut` must be reachable without scrolling.
50. **Both Split reset affordances disable themselves when nothing differs from
    the default**, derived from the defaults objects rather than a hand-written
    list (`// Derived from the defaults rather than listed by hand, so a setting
    added later is covered on its own.`).
51. **`OrganicCutPanel` uses a plain `useState(true)` for expansion**, unlike
    every other panel here which uses `useFloatingPanelCollapse`.

## Open questions

1. **`onPlatform` / `setPlatformZ`.** The Transform panel declares it and never
   renders a control, yet `PreparePanelStack` still passes
   `transformMgr.transformHook.setPlatformZ`. Is there an intended "sit on plate"
   action that was removed, or is the prop a leftover? The port ignores it.
2. **Local space is not wired in the GPUI port.** `gpui/src/gizmo/mod.rs` records
   "the entity's store flag is not read here, so the port's gizmo is always world
   aligned". The Transform panel's `Local` toggle therefore writes a flag nothing
   consumes. Whoever ports the toggle must also wire `gizmoFrameRotation`.
3. **No dial, magnet or readout in the port.** Whether the protractor should be a
   2D overlay (pixel-space drawing over the viewport) or triangle geometry is
   undecided; the module header calls it "2D overlay work with no GPUI counterpart
   yet".
4. **`GIZMO_LIGHTING` point lights.** The source washes coloured light onto the
   model from every hovered handle; the port's gizmo pass is unlit and the
   viewport has one headlight. Whether to emulate the wash is open.
5. **Retarget damping.** `ScreenSpaceGizmo`'s `SWITCH_DAMP_LAMBDA` /
   `SWITCH_SNAP_EPSILON` / `AXIS_SUPPRESS_MS` easing is unported; the port snaps.
6. **`GizmoRotation`'s per-axis `atan2` fallbacks** in `getCameraAlignedAngle` are
   unreachable whenever `ringGroupRef` is set (it always is inside the component),
   so they are dead paths of unknown intent.
7. **`GizmoCenter`'s own `handlePointerUp`** is defined but never attached to any
   mesh; only the window `pointerup` ends the drag.
8. **Mesh preview parity.** The GPUI slot is a static stand-in; whether a real
   shader preview must be rendered (and with which renderer) is unanswered.
   `matcap` and `flat_unlit` are not offered by the port's `MESH_SHADER_OPTIONS`.
9. **Split's preview cache and debounce** (`PREVIEW_SETTLE_MS = 80`,
   `PREVIEW_CACHE_MAX = 8`, the geodesic recompute) are scene services, not panel
   state; where they live in the port is undecided.
10. **Orientation's `onBeforeOrientApply` gate** is a destructive-apply confirm
    hook supplied by the page; the port's `orientation_body` takes the intent only
    and leaves the gate to the caller.
