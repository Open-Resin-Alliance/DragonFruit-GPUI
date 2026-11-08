# Panels — the Models panel and the mode panel stacks

Sources: `src/components/controls/ModelManagerPanel.tsx` (1139 lines, read in
full) and `src/components/organisms/panels/{ModelsPanel,PreparePanelStack,
SharedPanelStack,ExportPanelStack,PrintingPanelStack,AnalysisPanelStack}.tsx`.
The atoms/widgets this area renders through (`Card`, `CardHeader`,
`IconButton`, `PanelCollapseToggle`, `ContextMenu`, `StructuredDialogModal`,
`Tooltip`, `Button`) are quoted here with the exact geometry this area depends
on; they are owned by their own specs.

The React directory is `src/components/organisms/panels/` — there is no
`src/components/panels/`.

## Components in scope

| Component | File |
|---|---|
| `ModelManagerPanel` | `src/components/controls/ModelManagerPanel.tsx` |
| `ModelsPanel` | `src/components/organisms/panels/ModelsPanel.tsx` |
| `PreparePanelStack` | `src/components/organisms/panels/PreparePanelStack.tsx` |
| `SharedPanelStack` | `src/components/organisms/panels/SharedPanelStack.tsx` |
| `ExportPanelStack` | `src/components/organisms/panels/ExportPanelStack.tsx` |
| `PrintingPanelStack` | `src/components/organisms/panels/PrintingPanelStack.tsx` |
| `AnalysisPanelStack` | `src/components/organisms/panels/AnalysisPanelStack.tsx` |

Support mode mounts `ModelsPanel` directly from `page.tsx:10831-10858` with the
key `support-models`; Prepare mounts it through `PreparePanelStack` with the key
`prepare-models`. Prepare also uses `PreparePanelStack`'s Transform / Auto
Orientation / Smoothing / Organic Cut / Arrange / Duplicate panels, which are
other areas.

---

## Surface: ModelManagerPanel — the model list card

- **Source**: `src/components/controls/ModelManagerPanel.tsx:202` (component),
  `:823` (return), `:594` (`renderModelRow`).
- **Renders**: a `Card` (panel shell) whose `CardHeader` carries the collapse
  chevron, the title "Models" and a plus button, followed (only while expanded)
  by one scrollable column of groups and rows. A right-edge resize strip and a
  `ContextMenu` are always rendered as the card's last children.

### Props (the port's contract; every one maps to the source)

```
models: LoadedModel[]
outsidePlateModelIds?: string[]
plates?: readonly ScenePlate[]
activePlateId?: string
onActivatePlate?: (plateId: string) => void
onRenamePlate?: (plateId: string, name: string) => void
onRemovePlate?: (plateId: string) => void
resolveModelPlateId?: (model: LoadedModel) => string
activeModelId: string | null
selectedModelIds: string[]
selectedBodyKeys?: string[]          // `${modelId}#${bodyIndex}`
onSelectBody?: (modelId, bodyIndex, mode: 'single' | 'toggle') => void
onCloseBodyDrawer?: (modelId) => void
onSelect: (id, mode?: 'single' | 'toggle' | 'add') => void
onSelectRange?: (ids, activeId, mode?: 'replace' | 'add') => void
onSelectGroup?: (groupId, mode?: 'single' | 'add') => void
onGroupModels?: (modelIds) => void
onUngroupModels?: (modelIds) => void
onUngroupGroup?: (groupId) => void
onSplitImportGroup?: (modelId) => void
onRenameGroup?: (groupId, nextName) => void
onRenameModel?: (id, nextName) => void
onModelContextMenu?: (id, position: {x,y}) => void
onRepairModel?: (id) => void
onOpenSupportsInfo?: (id) => void
onAddModels?: () => void
onDelete: (id) => void
onVisibilityChange: (id, visible) => void
dimmed?: boolean          // default false
hidden?: boolean          // default false
bottomClearancePx?: number // default 220
collapsible?: boolean      // default true
```

`LoadedModel` (`useSceneCollectionManager.ts:1381`) contributes only
`id, name, groupId?, groupName?, visible, polygonCount, fileSizeBytes?,
splitBodies?: GeometryWithBounds[], bodyColors?: Array<string|undefined>`. A
body's triangle count is `body.geometry.index ? body.geometry.index.count / 3 :
(body.geometry.getAttribute('position')?.count ?? 0) / 3`
(`ModelManagerPanel.tsx:803-806`).

### Card shell — exact classes and styles

`Card` base (`atoms/Card.tsx`): `ui-panel rounded-lg shadow-lg border`, with
`style={{ borderColor: 'var(--border-subtle)', ...style }}`.

- `.ui-panel` (`globals.css:444`): `background: var(--surface-0)`,
  `border: 1px solid var(--border-subtle)`, `font-size: var(--font-size-sm)`
  (13px), `line-height: var(--line-height-normal)` (1.45).
- `rounded-lg` = `--radius-lg` = 14px (Tailwind's `--radius-*` are overridden in
  `:root`; see `gpui-ui/src/widgets/mod.rs` note).
- `className` (`:370`): dimmed →
  `'opacity-60 pointer-events-none transition-opacity duration-150 flex flex-col relative'`,
  otherwise `'transition-opacity duration-150 flex flex-col relative'`.
- `style` (`:373`):
  - `filter: 'grayscale(0.25)'` when dimmed;
  - `maxHeight: 'calc(100vh - var(--topbar-height) - ${computedBottomClearance}px)'`
    **only while expanded**, where
    `computedBottomClearance = Math.max(140, Math.round(bottomClearancePx))`;
  - `display: 'none'` when hidden. Comment: *"Hidden rather than unmounted: the
    window layout profiles resolve against the set of mounted panels, so
    unmounting this one would move every panel that is anchored to it."*

### Header (`:828-855`)

`CardHeader` base: `relative px-2.5 py-2.5 flex items-center`, plus
`data-panel-drag-handle="true"`; `left` container is
`flex items-center gap-2.5 pr-8`; `right` is
`absolute right-2.5 top-1/2 -translate-y-1/2 flex items-center`.
When `collapsible` is false the header gets `className="py-4"`. Comment: *"Only
the column layout is missing the collapse chevron, so there the title alone sets
the header height and the 32px plus sat nearly flush with the top edge. `py-4`
gives it the room the chevron gives the bar's header."*

Left slot, in order:
1. `PanelCollapseToggle` **only when `collapsible`**, given
   `expanded={expanded}` and `onToggle={() => setCollapseExpanded((prev) => !prev)}`.
   Atom: `IconButton` (default `solid` variant, `md` size) → classes
   `ui-button ui-button-secondary`, `.ui-button` padding `5px 11px 7px 11px`,
   radius `--radius-sm` (6px), font-size `--font-size-md` (14px), weight 600;
   title/aria-label `Collapse card` / `Expand card`, `aria-expanded={expanded}`;
   glyph `chevron-down` 12px tinted `var(--accent)` when expanded,
   `chevron-right` 12px tinted `var(--text-muted)` when collapsed. The atom's
   doc comment is the reason the tile is 30×30 and must not be "shrunk by
   dropping the padding".
2. `<h3 class="text-sm font-semibold" style="color: var(--text-strong)">Models</h3>`.
   `.ui-panel h3` forces `font-size: var(--font-size-md) !important` (14px),
   `line-height: 1.2`.

Right slot: the add button, **only when `onAddModels` exists** (`:843`):
`IconButton` with default `solid` variant and `size="sm"` → `ui-button
ui-button-secondary h-8 w-8 !p-0`, inside a panel so `.ui-panel .ui-button`
gives it 14px text (the padding rule is defeated by `!p-0`), overridden with
`!text-[var(--text-muted)] hover:!text-[var(--text-strong)] hover:!bg-[var(--surface-2)]`.
Glyph `Plus` at `h-3.5 w-3.5` (14px). `title`/tooltip text:
"Add models to the plate" (comment: *"Tooltip on the plus in the Models panel
header, which opens the mesh picker."*). Comment above the slot: *"The count
that used to sit here repeated what the list below already says, line by line.
The slot earns its keep as the way to add more."*

### Body (`:894-1119`)

Only while `expanded` (when collapsed the body is absent; the card is just the
header, no `maxHeight` applied):

```
<div class="px-2.5 pt-1 pb-2.5 space-y-2 flex flex-col flex-1 min-h-0">
  <div class="space-y-1 overflow-y-auto custom-scrollbar pr-0.5 flex-1 min-h-0">
    ...
```

`.custom-scrollbar` (`globals.css:928`): thin scrollbar; thumb
`linear-gradient(180deg, color-mix(in srgb, var(--accent), white 8%),
color-mix(in srgb, var(--accent), var(--surface-2) 42%))`, 10px wide, radius
999px, 2px border of `color-mix(in srgb, var(--surface-2), black 18%)`; track
`color-mix(in srgb, var(--surface-2), black 18%)`.

**Empty state** (`:897-901`): when `models.length === 0`,
`<div class="text-xs text-center py-2 italic" style="color: var(--text-muted)">No
models loaded</div>`. See gotcha 2: in the running app the panel is hidden
outright while the scene is empty.

### Grouping — `grouped` (`:275-320`)

Computed from `models`, `outsidePlateModelIds`, `plates`, `resolveModelPlateId`.

```
outsidePlateSet = new Set(outsidePlateModelIds)
outsideModels   = models.filter(m => set.has(m.id))
inPlateModels   = models.filter(m => !set.has(m.id))
```

Branch A — `plates && plates.length > 1 && resolveModelPlateId` (comment:
*"More than one plate makes the plate the grouping: one folder per plate, in the
cascade's own order, holding the models that stand on it. A single plate is not
worth a folder around everything."*):

1. One entry per plate, **in `plates` array order**:
   `{ id: plate.id, name: plate.name.trim() || plateNumberPlaceholder(index+1, _),
   models: [], isGrouped: true, isPlateGroup: true }` where
   `plateNumberPlaceholder(i) = translate(msg\`Plate ${i}\`)` → msgid "Plate {0}".
2. Each `inPlateModels` entry goes to its `resolveModelPlateId(model)` group, or
   into `strays` when that id is not one of the plates.
3. Each plate entry then gets `models` sorted by `a.name.localeCompare(b.name)`
   and `children: buildModelGroups(group.models)`.
4. If `strays.length > 0 || outsideModels.length > 0`, append one entry
   `{ id: '__system_outside_plate__', name: _('Outside plate'), models:
   [...strays, ...outsideModels].sort(by name), isGrouped: true,
   isSystemGroup: true }`. No `children`.
5. Result: plate entries then the trailing outside entry.

Branch B — otherwise: `buildModelGroups(inPlateModels)`, with the outside entry
(first, when `outsideModels.length > 0`) as the reduce's initial accumulator, so
**outside plate comes first**, then the folders/rows.

`buildModelGroups` (`:159-186`):
- Group key = `model.groupId ?? \`single-${model.id}\``; group name =
  `model.groupName ?? model.name`; `isGrouped = !!model.groupId`.
- Members keep insertion order into the map, then are sorted by `name`.
- Final sort: `if (a.isGrouped !== b.isGrouped) return a.isGrouped ? -1 : 1;` then
  `a.name.localeCompare(b.name)` — **folders first, then one row per ungrouped
  model, each alphabetical**.

`orderedModelIds` (`:361-366`): flatten `grouped`, using `group.children` when
present (`children.flatMap(child => child.models…)`), else `group.models`. This
is the order Shift-range selection walks (so a range crosses plate boundaries and
folder boundaries in on-screen order).

### Group rendering (`:902-1118`)

For each top-level `group`:

- `isCollapsed = group.isGrouped ? (collapsedGroupIds[group.id] ??
  (!!group.isPlateGroup && group.id !== activePlateId)) : false` — comment:
  *"Plates start with the one you are working on open and the rest closed; an
  explicit toggle still wins."*
- `selectedCount = group.models.filter(m => selectedSet.has(m.id)).length`;
  `isGroupFullySelected = selectedCount > 0 && selectedCount ===
  group.models.length`; `isGroupPartiallySelected = selectedCount > 0 &&
  !isGroupFullySelected`.
- `showHeader = group.isGrouped` (standalone rows have no header).
- `isActivePlateGroup = !!group.isPlateGroup && group.id === activePlateId`.
- `showChildren = !showHeader || !isCollapsed`.

Wrapper: `className={showHeader && !group.isPlateGroup ? 'space-y-1 rounded-md
border p-1' : 'space-y-1'}`; style only in the folder case:
`borderColor: 'color-mix(in srgb, var(--border-subtle), var(--accent) 14%)'`,
`background: 'color-mix(in srgb, var(--surface-1), var(--accent) 3%)'`. A plate
group and a standalone row get no card. Comment: *"A plate group is its own row
and needs no card around it; a folder of models keeps the wrapper that groups it
visually."*

**Header row** (`:905-1030`): `px-1.5 py-1 rounded border flex items-center
gap-1.5 cursor-pointer transition-colors`; style by selection:

| state | background | borderColor |
|---|---|---|
| fully selected **or** active plate | `color-mix(in srgb, var(--accent), var(--surface-2) 90%)` | `color-mix(in srgb, var(--accent), var(--border-subtle) 45%)` |
| partially selected | `color-mix(in srgb, var(--accent), var(--surface-2) 94%)` | `color-mix(in srgb, var(--accent), var(--border-subtle) 30%)` |
| otherwise | `var(--surface-2)` | `var(--border-subtle)` |

Header contents:
1. Collapse button: bare `<button type="button" class="inline-flex items-center
   justify-center rounded p-0.5 hover:bg-black/20">`, `onClick` →
   `e.stopPropagation(); toggleGroupCollapsed(group.id, isCollapsed)`, title
   `Expand folder` / `Collapse folder`; glyph `chevron-right`/`chevron-down`
   `w-3 h-3` in `var(--text-muted)`.
2. Glyph, 14px:
   - `group.isSystemGroup` → `AlertTriangle`, colour `#ff7c88`;
   - `group.isPlateGroup` → `LayoutGrid`, `var(--accent)` (comment: *"A plate is
     not a folder of models, it is a bed they stand on, so it wears a plate
     rather than a folder."*);
   - collapsed folder → `Folder`; expanded → `FolderOpen`, both `var(--accent)`.
3. Name cell, one of three, in this precedence:
   - plate being renamed (`renamingPlateId === group.id`): `<input autoFocus
     value={plateNameDraft} … class="flex-1 min-w-0 rounded border px-1.5 py-0.5
     text-[11px]">`, style `borderColor: var(--border-subtle)`, `background:
     var(--surface-0)`, `color: var(--text-strong)`, `aria-label="Plate name"`;
     `onClick` stopPropagation; `onBlur` → `onRenamePlate(group.id, draft.trim());
     setRenamingPlateId(null)`; Enter → same + close; Escape → close (no write).
   - group being renamed (`renamingGroupId === group.id`): same input classes,
     `aria-label="Rename folder"`, `autoFocus`, Enter → `commitRenameGroup()`,
     Escape → `cancelRenameGroup()`, `onBlur` → `commitRenameGroup()`.
   - otherwise `<span class="text-[10px] font-semibold uppercase tracking-wide
     truncate" style="color: var(--text-muted)">{group.name}</span>`.
4. Count: `<span class="ml-auto text-[10px] tabular-nums"
   style="color: var(--text-muted)">{group.models.length}</span>`.

Header `onClick` (`:1000`): comment *"A plate group is the plate: clicking its
header works on it, and then selects the models standing on it like any
folder."* → `if (group.isPlateGroup && onActivatePlate) onActivatePlate(group.id);`
then `mode = (ctrlKey || metaKey || shiftKey) ? 'add' : 'single';
selectFolder(group, mode)`.

Header `onContextMenu`: `preventDefault(); stopPropagation();` then
`isPlateGroup` → `setContextMenu({x, y, plateId: group.id})`; otherwise
`setContextMenu({x, y, groupId: group.id, groupName: group.name,
isSystemGroup: group.isSystemGroup})`.

**Children container** (`:1044-1117`): `className={showHeader ? 'ml-1.5 space-y-1
pl-1' : 'space-y-1'}`. If `group.children` exists (plate branch), map each child
folder:
- Child folder header (only when `child.isGrouped`):
  `px-1.5 py-1 rounded border flex items-center gap-1.5 cursor-pointer
  transition-colors`, style **always** `borderColor: var(--border-subtle)`,
  `background: var(--surface-2)` (child headers never tint with selection);
  `onClick` (no stopPropagation) → `selectFolder(child, (ctrl||meta||shift) ?
  'add' : 'single')`; **no `onContextMenu`**.
  Contents: bare collapse button (`toggleGroupCollapsed(child.id)` — no second
  argument, so it flips the stored entry), chevron 12px muted, `Folder`/`FolderOpen`
  14px accent (chosen from `collapsedGroupIds[child.id]` directly), name span
  identical to the top-level name span, count span identical.
- Rows: `{(!child.isGrouped || !collapsedGroupIds[child.id]) && <div
  className={child.isGrouped ? 'ml-1.5 space-y-1 pl-1' : 'space-y-1'}>…rows…</div>}`.
  A standalone child (`isGrouped` false) renders its row without a header.
If no `children`, render `group.models.map(renderModelRow)`.

### Model row — `renderModelRow` (`:594-821`)

Outer wrapper `div className="space-y-1"`.

Row (`:600-760`): `px-2 py-1 rounded border transition-colors flex items-center
gap-2 cursor-pointer` (`rounded` = 4px). Style:

- selected (`selectedSet.has(model.id)`):
  `background: 'color-mix(in srgb, var(--accent), var(--surface-1) 92%)'`,
  `borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 40%)'`;
- otherwise `background: 'var(--surface-1)'`,
  `borderColor: 'var(--border-subtle)'`.

Row `onClick` (`:610`):
1. If `e.shiftKey`:
   `anchorId = activeModelId ?? selectedModelIds[selectedModelIds.length-1] ??
   model.id`; `anchorIndex = orderedModelIds.indexOf(anchorId)`;
   `clickedIndex = orderedModelIds.indexOf(model.id)`. When both `>= 0`:
   `rangeIds = orderedModelIds.slice(min, max+1)`; `additive = e.ctrlKey ||
   e.metaKey`; if `onSelectRange` → `onSelectRange(rangeIds, model.id, additive
   ? 'add' : 'replace')`; else additive → each `onSelect(id,'add')`, non-additive
   → `onSelect(model.id,'single')` then every other range id `'add'`. Then return.
2. Otherwise `onSelect(model.id, (e.ctrlKey || e.metaKey) ? 'toggle' : 'single')`.

Row `onContextMenu`: `preventDefault(); stopPropagation(); setContextMenu({x:
e.clientX, y: e.clientY, modelId: model.id, groupId: model.groupId, groupName:
model.groupName})`.

Row contents, left to right:

1. **Bodies chevron**, only when `model.splitBodies && model.splitBodies.length >
   1` (`splitBodies` variable is null otherwise). `IconButton` `variant="ghost"`
   `size="sm"` `className="!-ml-1 shrink-0"` → ghost recipe
   `inline-flex items-center justify-center rounded transition-colors
   hover:bg-white/10`, `!p-1`, colour `var(--text-muted)`; `onClick` stops
   propagation, flips `expandedBodyModelIds[model.id]`, and on collapse calls
   `onCloseBodyDrawer?.(model.id)` (comment: *"Closing the drawer hides the rows,
   so a body selected in them would stay selected with nothing to show it."*);
   `title` and `aria-label` = body count plural (`# body` / `# bodies`), `aria-expanded`;
   glyph `ChevronDown`/`ChevronRight` 14px.
2. **Name cell**: `div class="flex-1 min-w-0"`.
   - While renaming (`renamingModelId === model.id`): `div class="flex w-full
     min-w-0 items-center gap-1"` containing `<input value={renamingModelName}
     autoFocus class="min-w-0 flex-1 rounded border px-1.5 py-0.5 text-xs
     font-medium">` with style `borderColor: var(--border-subtle)`, `background:
     var(--surface-0)`, `color: var(--text-strong)`, `aria-label="Rename model
     base name"`; `onClick`/`onPointerDown` stopPropagation; Enter →
     `commitRenameModel()`, Escape → `cancelRenameModel()`, `onBlur` →
     `commitRenameModel()`. Then, if `renamingModelSuffix` is non-empty, a
     non-editable suffix `<span class="shrink-0 text-[11px] font-medium"
     style="color: var(--text-muted)">{suffix}</span>`.
   - Otherwise `Tooltip content={model.name} fullWidth` wrapping `<div
     class="min-w-0 text-sm font-medium truncate" style="color:
     var(--text-strong)">{model.name}</div>`.
3. **Actions** `div class="flex items-center gap-0.5"`:
   - Info button, only when `onOpenSupportsInfo`: wrapped in
     `Tooltip content={formatModelInfoTooltip(_, fmt, model)}`; `IconButton`
     ghost sm (`!p-1`, muted, hover `hover:bg-white/10`), `aria-label="Model
     details"` (comment about the tooltip listing size and polygons), glyph
     `Info` `w-3.5 h-3.5`; `onClick` stops propagation → `onOpenSupportsInfo(model.id)`.
   - Visibility toggle: ghost sm; `title={model.visible ? 'Hide' : 'Show'}`;
     glyph `Eye`/`EyeOff` at `w-4 h-4`; `onClick` stops propagation →
     `onVisibilityChange(model.id, !model.visible)`.
   - Delete: ghost sm; `title="Delete model"` (comment: *"Deleting is undoable,
     so it asks nothing first."*); glyph `Trash2` `w-3.5 h-3.5`; `onClick` stops
     propagation → `onDelete(model.id)`.

**Body drawer** (`:762-819`), only when `splitBodies && bodiesExpanded`:
container `div class="ml-1.5 space-y-1 pl-1"` (comment: *"Indented like a
folder's models rather than ruled: the bodies are what this model is made of,
not a separate column."*). Each body:

- key `${model.id}#${index}`; selected = `selectedBodySet.has(key)`; colour =
  `model.bodyColors?.[index]`.
- Row: `flex items-center justify-between gap-2 rounded border px-2 py-1 text-sm
  cursor-pointer transition-colors`; style selected →
  `borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 40%)'`,
  `background: 'color-mix(in srgb, var(--accent), var(--surface-1) 92%)'`;
  otherwise `borderColor: var(--border-subtle)`, `background: var(--surface-2)`.
- `onClick`: `stopPropagation()`;
  `onSelectBody?.(model.id, index, (ctrlKey || metaKey) ? 'toggle' : 'single')`.
- `onContextMenu`: `preventDefault(); stopPropagation();` then comment *"The same
  menu a model row opens, targeting this body: selecting it first is what makes
  Material a body assignment."* → `onSelectBody?.(model.id, index, 'single');
  onModelContextMenu?.(model.id, {x: clientX, y: clientY})`. **This opens the
  editor's context menu, not this panel's menu** (see gotcha 12).
- Left: `span class="flex min-w-0 items-center gap-1.5"`; when a body colour
  exists, `<span class="h-3.5 w-3.5 shrink-0 rounded-[4px]" style="background:
  {bodyColor}; box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.28)"
  aria-hidden="true">`; then label `bodyNameLabel(_, index)` = msgid
  `Body ${index + 1}` → "Body {0}", class `truncate font-medium`, colour
  `var(--text-strong)` when selected else `var(--text-muted)`.
- Right: `span class="shrink-0 text-[11px] tabular-nums" style="color:
  var(--text-muted)">{formatPolygonCountCompact(triangles, fmt)}</span>`.

### Selection helpers

`selectFolder(group, mode)` (`:447-461`):
```
if (group.models.length === 0) return;
if (onSelectGroup && group.isGrouped && !group.isSystemGroup) { onSelectGroup(group.id, mode); return; }
group.models.forEach((model, index) => {
  if (mode === 'single') { onSelect(model.id, index === 0 ? 'single' : 'add'); return; }
  onSelect(model.id, 'add');
});
```
So a system group ("Outside plate") and a standalone row fall back to
per-model selection, and only a real folder uses the group API.

### Context menu

State `PanelContextMenuState` (`:141`): `x, y, modelId?, groupId?, groupName?,
isSystemGroup?, plateId?`. Opened by row/header context menus.

Visibility rules (`:354-356`, comment quoted):
```
// Smart context menu visibility:
// showGroupSection — show Group/Ungroup Selected only when there are grouped/multi-selected models or a folder is involved
// showFolderSection — show folder actions only when the right-clicked item has a group context
const showGroupSection = !!(contextMenu?.groupId || selectedModelIds.length >= 2 || selectedGroupedCount > 0 || !contextMenu?.modelId);
const showFolderSection = !!contextMenu?.groupId;
```
`selectedGroupedCount` = number of `selectedModelIds` whose model has a
`groupId`.

Entries are rebuilt every render, pushed in this order (`:466-520`):

1. `group-selected` — label `Group selected`, icon `FolderPlus`,
   `disabled: selectedModelIds.length < 2`.
2. `ungroup-selected` — label `Ungroup selected`, icon `FolderMinus`,
   `disabled: selectedGroupedCount === 0`.
3. When `contextMenu?.plateId`:
   `rename-plate` (`Rename plate`, `Pencil`, `startsGroup: true`);
   `delete-plate` (`Delete plate`, `Trash2`, `disabled: (plates?.length ?? 0) <= 1
   || plates?.[0]?.id === contextMenu.plateId`, comment: *"The first plate is the
   scene's floor; the last one has nowhere to go."*).
4. When `showFolderSection`:
   `select-folder` (`Select folder`, `PanelsTopLeft`, `startsGroup: true`);
   `rename-folder` (`Rename folder`, `Pencil`, `disabled: !!isSystemGroup`);
   `ungroup-folder` (`Ungroup folder`, `FolderMinus`, `disabled: !!isSystemGroup`).
5. When `contextModel && (onRenameModel || onModelContextMenu || onRepairModel)`:
   `rename-model` (`Rename model`, `Pencil`, `startsGroup: true`, only when
   `onRenameModel`); `repair-model` (`Repair mesh…`, `Wrench`, only when
   `onRepairModel`); `split-bodies` (`Split to bodies`, `Scissors`,
   `startsGroup: true`, only when `contextModel.splitBodies &&
   onSplitImportGroup`; the label carries a translator comment); `model-actions`
   (`Model actions…`, `Box`, only when `onModelContextMenu`).

`handleContextMenuSelect` (`:522-591`) and its observable result:

| id | behaviour |
|---|---|
| `group-selected` | `onGroupModels(selectedModelIds)` when `onGroupModels && length >= 2`. |
| `ungroup-selected` | `onUngroupModels(selectedModelIds)` when `onUngroupModels && selectedGroupedCount > 0`. |
| `select-folder` | `target = contextGroup ?? grouped.find(g => g.id === contextMenu?.groupId)`; `selectFolder(target, 'single')`. |
| `rename-folder` | when `groupId && onRenameGroup && !isSystemGroup` → `beginRenameGroup(groupId, groupName ?? contextGroup?.name ?? 'Group')`. |
| `ungroup-folder` | when `groupId && onUngroupGroup && !isSystemGroup` → `onUngroupGroup(groupId)`. |
| `rename-plate` | when `plateId && onRenamePlate` → `setPlateNameDraft(plates.find(...)?.name ?? '')`, `setRenamingPlateId(plateId)`. |
| `delete-plate` | see below. |
| `rename-model` | `beginRenameModel(contextModel.id, contextModel.name ?? 'Model')`. |
| `repair-model` | `onRepairModel(contextModel.id)`. |
| `split-bodies` | `onSplitImportGroup(contextModel.id)` — replaces the one model with N independent models, instant (pre-computed bodies). |
| `model-actions` | `onModelContextMenu(contextModel.id, {x, y})` — the **editor** menu at the same coordinates. |

Every branch ends with `closeContextMenu()`.

`delete-plate` (`:550-576`): when `plateId && onRemovePlate`: comment *"An empty
bed has nothing to lose, so it goes straight away. The confirmation is for the
models a delete takes with it."*
`holdsModels = resolveModelPlateId ? models.some(m => resolveModelPlateId(m) ===
plateId) : true`. If `!holdsModels` → `onRemovePlate(plateId)` immediately.
Otherwise find `index = plates.findIndex(plate.id)`, `named = plates[index].name.trim()`,
and `setPlatePendingDelete({ id: plateId, name: named || plateNumberPlaceholder(index + 1, _) })`
— comment: *"The wording is settled here, where the plate is known, rather than
in the modal, where the list may already have moved on."*

The rendered `ContextMenu` (`:1114-1120`): `position={contextMenu}`,
`entries={contextMenuEntries}`, `onSelect={handleContextMenuSelect}`,
`onClose={closeContextMenu}`, `title={<Trans>Models</Trans>}`,
`ariaLabel={_(msg\`Models context menu\`)}`. Defaults apply: `widthClassName =
'w-48'`, `zIndexClassName = 'z-[130]'`.

`ContextMenu` widget contract (`ui/ContextMenu.tsx`): portalled to `document.body`;
surface `rounded-lg border p-1.5 shadow-xl backdrop-blur-sm`,
`borderColor: var(--border-subtle)`, `background: color-mix(in srgb, var(--surface-0),
#000 10%)`; title row `mb-1 px-2 py-1 text-[10px] font-semibold uppercase
tracking-wide` in `var(--text-muted)`; rows `flex w-full items-center gap-2
rounded-md px-2 py-1.5 text-left text-[13px] font-medium`, 20px framed icon slot
(`rounded border`, `var(--surface-1)`, border-subtle, icon 14px), row colour
`var(--text-muted)` when disabled else `var(--text-strong)`; disabled opacity
0.55, `cursor: not-allowed`; hover fill
`color-mix(in srgb, var(--accent), var(--surface-1) 82%)` (danger rows: tone
`var(--danger)`, fill `color-mix(in srgb, var(--danger), var(--surface-1) 90%)`);
group separators `my-1 h-px` of `var(--border-subtle)`, suppressed on index 0;
placement measured then clamped with an 8px margin; dismissal is outside
pointer-down, Escape, resize and scroll; selecting a row calls `onSelect` then
`onClose`. No `children` (flyout) entries are used by this panel.

### Plate delete dialog (`:857-892`)

Rendered when `platePendingDelete` is set:

`StructuredDialogModal` `open`, `ariaLabel="Delete plate"`, `title="Delete this
plate?"`, `subtitle={platePendingDelete.name}`, `icon=<Trash2 h-4 w-4>`,
`iconTone="danger"`, `zIndexClassName="z-[130]"`, `closeAriaLabel="Close modal"`,
`onClose` and `onBackdropClick` → `setPlatePendingDelete(null)`.

- Icon slot style (`iconTone` danger): `inline-flex h-9 w-9 shrink-0 items-center
  justify-center rounded-md border`, border
  `color-mix(in srgb, #ef4444, var(--border-subtle) 55%)`, background
  `color-mix(in srgb, #ef4444, var(--surface-1) 88%)`, colour `var(--danger)`.
- Panel: `w-full max-w-lg overflow-hidden rounded-xl border shadow-2xl`,
  `background: var(--surface-0)`, `borderColor: var(--border-subtle)`,
  `boxShadow: 0 24px 46px rgba(0,0,0,0.42)`; backdrop `fixed inset-0 flex
  items-center justify-center bg-black/55 backdrop-blur-sm px-3`.
- Header: `flex items-center justify-between gap-4 border-b px-5 py-4`; title
  `text-base font-semibold leading-tight` in `var(--text-strong)`; subtitle
  `mt-0.5 text-[11px] leading-snug` in `var(--text-muted)`; close `IconButton`
  surface md with `X` 16px.
- Body: `space-y-4 p-5`; text `<p class="text-sm leading-relaxed" style="color:
  var(--text-muted)">The models on this plate are deleted with it. Undo brings
  them back.</p>` with translator comment *"The count is how many models stand on
  the plate being deleted."*
- Actions: `grid grid-flow-col auto-cols-fr gap-2 pt-1`: `Button
  variant="secondary"` → "Cancel" (closes); `Button variant="tinted-danger"` →
  "Delete plate" (`onRemovePlate(platePendingDelete.id)` then close).
  `tinted-danger` = `.ui-button-tint .ui-button-tint-danger`: background
  `color-mix(in srgb, var(--danger), var(--surface-1) 86%)`, border
  `color-mix(in srgb, var(--danger), var(--border-subtle) 45%)`, text
  `var(--danger)`; hover 76%.
- Escape closes (unless the close button is disabled — it is not).

### Rename flows

- `beginRenameGroup(groupId, currentName)` (`:399`): clears model rename state,
  sets group id + name, closes the menu.
- `commitRenameGroup` (`:412`): no id or no `onRenameGroup` → cancel;
  `trimmed = trim()`; if non-empty call `onRenameGroup(id, trimmed)`; always
  cancel. Empty → silent no-op.
- `beginRenameModel(modelId, currentName)` (`:427`): uses
  `splitModelNameSuffix(currentName, _('Model'))` (`:188`) = trim, then match
  `/^(.*?)(\.[^.\s]+)$/`; no match → `{base: trimmed, suffix: ''}`; match with an
  empty base → base becomes the fallback `Model`; suffix is the last
  dot-segment. Clears group rename state, sets model id/base/suffix, closes menu.
- `commitRenameModel` (`:437`): no id or no `onRenameModel` → cancel; `trimmedBase
  = trim()`; if non-empty → `onRenameModel(id, `${trimmedBase}${suffix}`)`; always
  cancel.
- Commit also happens on `onBlur` of the inline inputs, so clicking elsewhere
  commits (there is no cancel-on-blur). The suffix span is display-only; typing
  cannot change it and it is appended back on commit.

### Right-edge resize handle (`:1122-1135`)

Always rendered as the card's last child:
`<div class="absolute right-0 top-0 bottom-0 w-1.5 cursor-col-resize
hover:opacity-100 opacity-0 transition-opacity" style="background: transparent"
onPointerDown={handleResizePointerDown}>` containing a
`<div class="absolute right-0 top-0 bottom-0 w-[3px] rounded-full
transition-colors" style="background: color-mix(in srgb, var(--accent),
transparent 60%)">`.

`handleResizePointerDown` (`:257-265`) finds `closest('[data-dock]')`, reads
`data-dock`, and only for `'left'`/`'right'` calls
`e.preventDefault(); e.stopPropagation(); beginDockResize(side, e.clientX)`.
`beginDockResize` comes from `useFloatingPanelDockResize()` →
`FloatingPanelStackContext.beginDockResize`, which measures
`[data-dock="{side}"]` width and starts a dock resize (comment:
*"The panel's edge handle resizes the dock it sits in; the dock owns the width,
so writing the panel's own style here would be undone by the next render."*).
If the hook is absent (`null`), the handle does nothing.

Layout-system facts that apply (`FloatingPanelStack.tsx`): `RESIZABLE_PANELS =
new Set(['prepare-models', 'support-models'])`; `LOCKED_PANELS =
{'prepare-models': 'rail-column', 'support-models': 'rail-column'}` — the panel
cannot be dragged while the rail is a column, and its width is dragged from the
dock edge. `DEFAULT_PANEL_WIDTH = 320`, `MIN_DOCK_WIDTH = 200`,
`MAX_DOCK_WIDTH = 520`, `COLLAPSED_PANEL_HEIGHT = 48` (arithmetic only).

### State (local, resets on unmount)

`collapseExpanded` (persisted per panel id by `useFloatingPanelCollapse(true)`,
`autoCollapse` defaults true), `collapsedGroupIds: Record<string, boolean>`,
`expandedBodyModelIds: Record<string, boolean>`, `renamingPlateId` + `plateNameDraft`,
`platePendingDelete`, `renamingGroupId` + `renamingGroupName`,
`renamingModelId` + `renamingModelName` + `renamingModelSuffix`, `contextMenu`.

`expanded = collapsible ? collapseExpanded : true` — comment:
*"Where collapse is not offered the panel is pinned expanded. This has to be a
fallback, not a conjunction: `collapsible && collapseExpanded` reads false in the
column layout and hides the whole body, which is an empty model list."*

`toggleGroupCollapsed(groupId, collapsedNow = !!collapsedGroupIds[groupId])`
(`:379`) writes `[groupId]: !collapsedNow`. Comment: *"Flips what the row shows.
A plate the user has not touched has no entry yet and is shown folded unless it
is the active one, so flipping the bare entry folded it again."*

### Text and msgids (verbatim)

| where | text | msgid / note |
|---|---|---|
| title | `Models` | `Trans` comment "Title of the panel listing every model loaded into the scene." |
| add tooltip | `Add models to the plate` | msgid, comment about the mesh picker |
| collapse toggle | `Collapse card` / `Expand card` | `PanelCollapseToggle` |
| empty | `No models loaded` | `Trans` |
| info aria | `Model details` | msgid with comment |
| info tooltip | `{size} · {polygons}` / polygons alone | descriptor ``msg`${size} · ${polygons}` `` |
| polygons | `# polygon` / `# polygons` | plural |
| visibility | `Hide` / `Show` | `title` |
| delete | `Delete model` | msgid with comment |
| body chevron | `# body` / `# bodies` | plural |
| body label | `Body {0}` | ``msg`Body ${index + 1}` `` |
| rename model aria | `Rename model base name` | msgid |
| placeholder fallback | `Model` | `msg({message:'Model', context:'Default model name'})` |
| plate name aria | `Plate name` | msgid |
| folder name aria | `Rename folder` | msgid |
| folder chevron | `Expand folder` / `Collapse folder` | msgid |
| system group | `Outside plate` | msgid with comment |
| unnamed plate | `Plate {0}` | `plateNumberPlaceholder` |
| menu title | `Models` | `Trans` comment "…Rendered uppercase." |
| menu aria | `Models context menu` | msgid |
| menu rows | `Group selected`, `Ungroup selected`, `Rename plate`, `Delete plate`, `Select folder`, `Rename folder`, `Ungroup folder`, `Rename model`, `Repair mesh…`, `Split to bodies`, `Model actions…` | `Trans` (msgid = text); `Split to bodies` has a comment |
| dialog | `Delete plate` (aria), `Delete this plate?`, `Close modal`, `Cancel`, `Delete plate`, `The models on this plate are deleted with it. Undo brings them back.` | msgids / `Trans` |
| rename fallback in menu | `Group` | hardcoded English fallback, not translated |

### Port target

`gpui/src/panels/content.rs::models_body` — the body only; the shell (card,
header, collapse chevron, drag, right slot, context-menu host) is
`gpui/src/panels/list.rs` (`PanelDef`, `PanelKind::Models`, `header_right`,
`collapsed`, `auto_collapse`). The panel is mounted in `gpui/src/app.rs:1840`
with id `"models"`, dock Left, `header_right(models_add_button)` and
`content(models_body(...))`.

What exists today that a porter must build on or replace (all in `content.rs`,
module docs items 6-15): `models_body(mesh_name, triangles, selected, entity)`
renders only one ungrouped row or the empty text; `model_row` exists with inert
info/eye/trash buttons and no rename; `body_row`, `folder_row` and
`models_context_menu()` exist but are `#[allow(dead_code)]` and unreachable
because the app holds a single `DragonFruit::mesh` (no model list, no plates, no
import groups, no `splitBodies`); the selection is one `Rc<Cell<bool>>`
(`selected`) so `'single' | 'toggle' | 'add'`, Shift-range and group selection
have no equivalent; `models_add_button` calls `this.open_stl(cx)` (native file
dialog) rather than the source's mesh picker; the `overflow-y-auto` list is
deliberately not applied (note 14).

`gpui/src/features/...` has no models-panel module; the Prepare-stack flags live
on `gpui-ui/src/widgets/prepare_stack.rs::PrepareStackState`
(`models_panel_visible`, `models_panel_collapsible`, `models_dimmed`,
`models_bottom_clearance_px`) and the panel machinery in
`gpui/src/features/tool_panels.rs`.

---

## Surface: ModelsPanel — the Prepare/Support wrapper

- **Source**: `src/components/organisms/panels/ModelsPanel.tsx`.
- **Renders**: `ModelManagerPanel` fed entirely from the `scene` manager plus
  props threaded through. No DOM of its own.
- **Doc comment**: *"It exists so the two modes can render the same panel with the
  same wiring without either of them carrying the other's copy of it: the window
  layout system keys panels by the React key, so each mode passes its own —
  `prepare-models` in Prepare, `support-models` in Support."*
- **Data mapping** (every prop is `React.ComponentProps<typeof ModelManagerPanel>`
  passthrough): `scene.models`, `scene.plates`, `scene.activePlateId`,
  `scene.activatePlate`, `scene.renamePlate`, `scene.removePlate`,
  `scene.resolveModelPlateId`, `scene.activeModelId`, `scene.selectedModelIds`,
  `scene.deleteModel`, `scene.setModelVisibility`; the callbacks are the caller's
  handlers (`handleModelSelection`, `handleSelectBody`, `handleCloseBodyDrawer`,
  `handleModelRangeSelection`, `handleGroupSelection`, `handleGroupSelectedModels`,
  `handleUngroupSelectedModels`, `handleUngroupFolder`, `handleSplitImportGroup`,
  `handleRenameFolder`, `handleRenameModel`, `handleModelListContextMenu`,
  `handleRepairModel`, `handleOpenModelSupportsInfo`, `handleAddModels`) plus
  `dimmed`, `hidden`, `collapsible`, `bottomClearancePx`.
- **Port target**: no independent behaviour; in Rust it is the caller passing
  `panel_content::models_body` into a `PanelDef`. No new module.

---

## Surface: PreparePanelStack — Prepare-mode panel composition

- **Source**: `src/components/organisms/panels/PreparePanelStack.tsx`.
- **Renders**: a keyed fragment passed inline by `page.tsx`; comment: *"Invoked
  inline by Home (not as `<JSX/>`) so FloatingPanelStack can flatten these keyed
  panels as direct children for its layout-profile positioning. 'use no memo'
  keeps React Compiler from injecting a useMemoCache hook (the conditional inline
  call must stay hook-free)."* Contains `'use no memo';`.
- **Children, in order, with exact conditions:**

| key | component | condition |
|---|---|---|
| `prepare-models` | `ModelsPanel` | always (`dimmed={showEmptySceneDialog || importOverlayState.active}`, `hidden={!modelsPanelVisible}`, `collapsible={modelsPanelCollapsible}`, `bottomClearancePx={modelStatsBottomClearancePx}`) |
| `prepare-transform-controls` | `TransformControls` | `scene.geom && transformMgr.transformMode === 'transform'` |
| `prepare-orientation` | `AutoRotationPanel` | `scene.geom && transformMode === 'transform' && orientationPanel` (`{...orientationPanel}`; `null` when the experiment is off) |
| `prepare-smoothing-settings` | `MeshSmoothingSettingsPanel` | `scene.geom && transformMode === 'smoothing'` |
| `prepare-organic-cut-panel` | `OrganicCutPanel` | `scene.geom && transformMode === 'organicCut'` |
| `prepare-arrange-panel` | `ArrangePanel` | `scene.models.length > 0 && transformMode === 'arrange'` |
| `prepare-duplicate-panel` | `DuplicatePanel` | `scene.models.length > 0 && transformMode === 'duplicate'` |

- `isMultiSelection = scene.selectedModelIds.length > 1` switches every
  `TransformControls` callback between the single-model hook and the
  multi-selection handler (position/rotation/scale/reset/commit).
- `ArrangePanel` `onApplyAll`/`onApplySelected` dispatch
  `arrangeLayoutMode === 'array' ? handleManualArrayArrangeModels(target) :
  (arrangePrecisionMode === 'high_precision' ? handleHighPrecisionArrangeModels(target)
  : handleAutoArrangeModels(target))`; counts are visible-only
  (`modelCount`, `selectedModelCount`).
- **Port target**: `gpui/src/features/tool_panels.rs::tool_panel` already maps the
  rail index to these bodies; `gpui-ui/src/widgets/prepare_stack.rs` holds the
  panel ids (`prepare-models`, …) and flags. The other bodies are owned by
  `features/transform_panel.rs`, `smoothing_panel.rs`, `organic_cut_panel.rs`,
  `arrange_panels.rs`; the Auto Orientation body is
  `features::transform_panel::orientation_body` and is *not* stacked in the port
  (no experiment flag).

---

## Surface: SharedPanelStack — the developer debug overlay

- **Source**: `src/components/organisms/panels/SharedPanelStack.tsx` (426 lines).
- **Renders**: one `Card` (`key="transform-debug-overlay"`) only while
  `isTransformDebugOverlayOpen`; the card is `font-mono text-[10px] leading-tight`
  with `style={{ color: 'var(--text-strong)', fontSize: '10px' }}`. Everything
  inside is wrapped in `{/* i18n-ignore-start: developer debug overlay
  (Ctrl+Shift+X), kept in English */ … {/* i18n-ignore-end */}`. This is a
  developer surface: it is disposable for parity, but its toggling contract
  (Ctrl+Shift+X) is user-reachable.
- **Header**: title by mode — `Printing Debug Overlay` / `Support Debug Overlay`
  / `Transform Debug Overlay` (`h3 text-xs font-semibold`,
  `fontFamily: var(--font-geist-mono)`); right slot a bare button
  `class="rounded border px-2 py-0.5 text-[10px]"`,
  `borderColor: var(--border-subtle)`, `color: var(--text-muted)`, label
  `Close`, `onClick={() => setIsTransformDebugOverlayOpen(false)}`.
- **Body**: `div class="px-2.5 pb-2.5"`.
- **Per mode**:
  - `printing`: `grid grid-cols-2 gap-x-3 gap-y-1` with rows `Mode`, `Total
    layers`, `Selected layer`, `Displayed layer`, `Is scrubbing`, `Show scrub
    preview`, `Send progress` (`(p*100).toFixed(1)+'%'`), `Send busy`, `Stage
    text` (truncated, `title`, `'none'` fallback); then a bordered section
    `Preview State` (border-t, border-subtle) with
    `Preview URLs loaded: {count} / {total}`, `Selected URL exists`,
    `Displayed URL exists`, `Artifact ready`, `Artifact name`,
    `Upload dialog open`, `Upload stage`, `Display progress`, `Ready plate ID`,
    `Print now busy`, `Status text`; then, when a benchmark exists, `Slicing
    Metrics` with `Total time / Mesh prep / Core slicing / Total layers /
    Layers/sec` (each optional row rendered only when its field is non-null).
  - `support`: grid with `Mode`, `Active model`, `Hovered category`,
    `Hovered id`; then `Support Debug` section: `hover:`, `scene hover:`,
    `segment hover:`, `hover model:`, `visual hover:`, an accent-red
    (`#ff8a8a`) `suppressed: …` line listing armed flags joined by `' · '`
    (`interaction`, `selection+hover`, `gizmo lock`, `knot drag`, `joint drag`),
    a red `hover/snap segment mismatch` line, and a `<details>` whose summary is
    `Details` containing ~22 more rows (shaft hover point, brace state/preview,
    guard times, hover ownership, raw hover, …).
  - other modes (`prepare`/`export`/`printing` false): grid with `Mode`,
    `Transform mode`, `Active model`, `Display model`, `isTransforming`,
    `Drag group auto`; then `Transform Delta (live vs store)` (`Δpos` mm, `Δrot
    max` rad, `Δscale`), `Active Model Transform` (store/live pos, drag deltas),
    `Transform Timeline` (last op and event stamps plus `Release → …` latencies),
    `Transform History Commit` (pending/commit/nonce fields and undo counts).
- `SupportInspectorPanel` (`src/supports/SupportInspectorPanel.tsx`, another
  area) is rendered unconditionally inside the card with `activeModelId` and
  `hoveredSupportId={supportDebugStats.hoveredIdForVisual ?? null}`. Comment:
  *"Selection, connections and counts, read from the support store. The page
  snapshot these used to come from is deliberately empty in support mode, so
  every count read zero exactly here."*
- Footer `Toggle: Ctrl+Shift+X` at `mt-2 text-[10px]`, muted.
- **Port target**: none today. `gpui/src/panels/` has no debug overlay; the
  prepare stack's tool panels do not render one. If ported it should be a panel
  with id `transform-debug-overlay` (the layout keeps its 420px
  `PANEL_WIDTH_OVERRIDES` entry in `panels/list.rs`).

---

## Surface: ExportPanelStack — Export-mode composition

- **Source**: `src/components/organisms/panels/ExportPanelStack.tsx`.
- **Renders**: keyed fragment, `'use no memo';`, two panels:
  `ExportPanel` (`key="export-main"`) and `SlicingPanel` (`key="export-slicing"`).
- **Derived data (computed inline because the component is deliberately
  hook-free — quoted comment):**
  - `plateSliceScopes`: when `scene.plateFrames.length > 0`, per frame
    `{...plateSliceScope(frame, scene.models, scene.plateFrames), plateName:
    scene.plates.find(p => p.id === frame.id)?.name ?? ''}`, else `undefined`.
  - `activePlateSliceIndex = Math.max(0, scene.plateFrames.findIndex(f => f.id ===
    scene.activePlateId))`.
  - `plateGroups = scene.plates.map(plate => ({id: plate.id, name: plate.name,
    modelIds: modelsOnPlate(scene.models, plate.id, scene.plateFrames).map(m => m.id)}))
    .filter(plate => plate.modelIds.length > 0)`. Comment: *"Membership is
    resolved by the scene, so a model that was dragged to another bed counts as
    being there rather than where it was imported; a bed left empty is not
    offered… Names come through as they are: an unnamed plate's placeholder is
    the panel's to phrase, and this component is deliberately hook-free so it
    cannot translate one."*
- `ExportPanel` props: `models`, `activeModel`, `activeModelId`,
  `selectedModelIds`, `plateGroups`, `activePlateId`,
  `onActiveModelChange={scene.setActiveModelId}`, `supportsRef`,
  `captureSceneThumbnailPng`, `onExportSuccess`, `onExportError`,
  `onExportProgress={setIsExporting}`.
- `SlicingPanel` props: models, plate scopes, active index (above),
  `excludedModelIds`, `activeModel`, the three `…Override` estimate props,
  thumbnail capture, `onSliceRunStarted`, `onSlicingFinished`,
  `onSliceArtifactReady`, `onSlicePlateSliced`, `onBenchmarkComplete`,
  `onSliceTriggerRef`, `shouldAutoSlice`, `skipThumbnailCapture`,
  `onSlicingBusyChange`, `canUpload`, `canPrint`, `onSliceIntentChanged`,
  `onBeforeSliceStart`, `onBeforeSlicingRun`,
  `resolveOutputPathForIntent={(intent) => (intent === 'file' || intent ===
  'uvtools') ? (preSliceFileDestinationPathRef.current?.trim() || null) : null}`.
- **Port target**: `ExportPanel` / `SlicingPanel` are other areas
  (`src/features/export`, `src/features/slicing`); in Rust Export mode mounts a
  single `PanelDef` with id `"export-main"` / kind `Export` in
  `gpui/src/app.rs:1932`, and `SlicingPanel` is not ported. This stack's
  contribution is the plate-scope derivation above.

---

## Surface: PrintingPanelStack — Printing-mode composition

- **Source**: `src/components/organisms/panels/PrintingPanelStack.tsx`.
- **Renders**: `'use no memo';` + one `PrintingPanel` (no key).
- **Props**: `outputName = printingArtifact?.outputName ?? null`;
  `outputFormat` derived from the name (`'.' + last extension`, else null);
  `outputSizeLabel`; `printerName = activePrinterProfile?.name ?? 'No printer
  selected'` (**hardcoded English, not translated**); `resinName`;
  `estimatedPrintTimeLabel`; `estimatedVolumeLabel`; `canDownload`;
  `canSendToPrinter`; `sendBusy`; `sendStatusText`; `sendButtonLabel`;
  `showSendTargetPicker={printableConnectedPrinterFleet.length > 1}`;
  `onOpenSendTargetPicker` → `setPrintingTargetPickerMode('post-slice')` +
  `setPrintingTargetPickerOpen(true)`; `onDownload`; `onSendToPrinter`;
  `onCancelSendToPrinter`; `canSendToUvTools = getSavedUvToolsSettings().enabled`;
  `onSendToUvTools` → no-op when `completedSaveDestinationPath` is empty, else
  `launchExternalProcess(resolveUvToolsExecutablePath(settings),
  completedSaveDestinationPath)` with a `console.warn('[UVTools] Failed to
  launch from printing panel:', err)` on rejection; `sliceIntent =
  completedSliceIntent`; `savedFilePath = completedSaveDestinationPath`.
- **Port target**: `PrintingPanel` is another area; `gpui/src/app.rs:1941` mounts
  a single panel with id `"printing"`, kind `Printing`. UVTools launch is
  `backend::uvtools` / `gpui-ui` settings (`features/settings/uvtools.rs`).

---

## Surface: AnalysisPanelStack — Analysis-mode composition

- **Source**: `src/components/organisms/panels/AnalysisPanelStack.tsx`.
- **Renders**: `'use no memo';` + a keyed fragment of six analysis cards, all fed
  from `islands: useIslandManager()` (plus `slicing.layerHeightMm` for two):

| key | component | data |
|---|---|---|
| `analysis-scan-card` | `IslandScanCard` | `islands`, `hasGeometry={!!scene.geom}`, `onLoadSupportJson={scene.handleLoadSupportJson}`, `onImportSupportFile={scene.importSupportDataFile}`, `pluginImportPhase`, `pluginImportError`, `onPluginJsonFile`, `onPluginStlFile`, `onCancelPluginImport` |
| `analysis-workflow` | `IslandScanWorkflowCard` | `islands`, `hasGeometry` |
| `analysis-volumes` | `IslandVolumesHierarchyCard` | `islands`, `layerHeightMm={slicing.layerHeightMm}` |
| `analysis-island-list` | `IslandListCard` | `islands.scanData?.islands ?? []`, `selectedIslandId`, `onSelectIsland=setSelectedIslandId`, `showMerged`, `onShowMergedChange=setShowMerged`, `layerHeightMm`, `zOffsetMm={0}` |
| `analysis-overlay-controls` | `IslandOverlayControls` | enabled/brush/colour/opacity/taper pairs, `islandCount={islands.scanData?.islands.length ?? 0}` |
| `analysis-island-voxel` | `IslandVoxelControls` | `enabled={voxelEnabled && !voxelShowTerritory}`; `onEnabledChange(e)`: on → `setVoxelEnabled(true); setVoxelShowTerritory(false)`; off → `setVoxelEnabled(false)`; plus opacity/colour scheme/show-merged/count |
| `analysis-territory-voxel` | `TerritoryVoxelControls` | `enabled={voxelEnabled && voxelShowTerritory}`; on → both true; off → `setVoxelEnabled(false)`; `islandCount` is the same expression on both branches (no-op conditional, see gotcha 18); `useSurfaceContiguity` pair; `onRescan={islands.onRunScanlineScan}` |

- **Port target**: the analysis cards are another area; the port has none of
  them. No module owns this stack in `gpui/src` yet.

---

## Deliberate decisions and gotchas

1. **`expanded` must be a fallback, not a conjunction.** Quoted:
   *"Where collapse is not offered the panel is pinned expanded. This has to be a
   fallback, not a conjunction: `collapsible && collapseExpanded` reads false in the
   column layout and hides the whole body, which is an empty model list."*
2. **The empty state is unreachable in the app.** `page.tsx:8597` computes
   `modelsPanelShown = !showEmptySceneDialog && (modelsPanelVisible ||
   toolLayout === 'horizontal')` and `showEmptySceneDialog = scene.models.length
   === 0` (`:8586`), so the panel is `display: none` exactly when the list would
   be empty. The port's `models_body` renders the text when `triangles == 0`, but
   `app.rs` only mounts the panel `when has_models`. Keep the string, do not
   treat "No models loaded" as a visible parity region.
3. **Collapse persists per panel id.** `useFloatingPanelCollapse(true)` registers
   with the stack under the panel's React key (`prepare-models` /
   `support-models`), `autoCollapse` defaults `true`. Collapsing the list is not
   the same as `hidden` (the rail's Models toggle): collapsed keeps a header-only
   card, hidden is `display: none` and keeps the panel mounted for the layout
   profile.
4. **`hidden` is a style, never an unmount.** Quoted comment at `:376-378`.
5. **Plate groups default collapsed unless active; folders default expanded.**
   `collapsedGroupIds[group.id] ?? (!!group.isPlateGroup && group.id !==
   activePlateId)`. A first click on an unset plate header passes the resolved
   `isCollapsed` into `toggleGroupCollapsed`, which is why the resolved value is
   a parameter. Child folder headers instead call `toggleGroupCollapsed(child.id)`
   with no second argument and therefore read the *stored* map (undefined → flip
   to `true`, i.e. collapse), which is the correct default because an untouched
   child folder is expanded.
6. **Ordering is three separate sorts, not one.** Models inside a group are
   sorted by name; groups are sorted "folders first, then standalone rows, each
   alphabetical"; plate groups stay in the `plates` array order and are never
   sorted among themselves. The outside-plate group is appended last in the
   multi-plate branch but is the accumulator (first) in the single-plate branch.
7. **One group entry per ungrouped model**, with synthetic id `single-${model.id}`
   and `isGrouped: false`. Two models with the same name stay separate (ids
   differ) and sort adjacent.
8. **`strays` are not an error state.** In the multi-plate branch a model whose
   `resolveModelPlateId` returns an id not in `plates` is folded into "Outside
   plate" along with `outsidePlateModelIds`, silently.
9. **Group header selection tint uses `group.models`, so a plate header tints
   only when every model on the plate is selected** — the plate itself has no
   selection state of its own. Active plate (`isActivePlateGroup`) uses the same
   fill as fully-selected.
10. **Clicking a plate header activates the plate and then tries to select its
    models "like any folder" — the selection call is a no-op against today's
    scene API.** `onActivatePlate(plate.id)` clears the selection
    (`activatePlate` sets `selectedModelIds=[]`, `activeModelId=null`), then
    `selectFolder(plateGroup,'single')` calls `onSelectGroup(plate.id)`, and
    `scene.selectGroup` selects `models.filter(m => m.groupId === groupId)` — a
    plate id is not a model `groupId`, so it selects nothing. Port the calls as
    written and record the outcome; decide with evidence whether "select the
    plate's models" is intended (see Open questions).
11. **The context menu for a model inside a plate-side folder loses its folder.**
    `contextGroup` is looked up only in the top-level `grouped` array
    (`grouped.find(g => g.id === contextGroupId)`), so a model's `groupName`
    still renames the folder (it comes from `contextMenu.groupName`), but
    `select-folder` finds no target and does nothing. Child folder headers
    themselves have **no `onContextMenu`** at all.
12. **Right-clicking a 3MF body row opens the *editor* menu, not the panel's.**
    The body row calls `onSelectBody(...,'single')` then
    `onModelContextMenu(model.id, {x,y})`; in `page.tsx:5670` that selects the
    model (if not already selected) and sets `editorContextMenuPos` — the scene
    canvas context menu. It never sets `contextMenu` in this panel.
13. **`startsGroup` on the first entry is a no-op** (`index > 0` guard in
    `ContextMenu`). The first section therefore has no rule above it, and the
    separator appears only between sections that both exist. `rename-plate`,
    `select-folder`, `rename-model` and `split-bodies` all request
    `startsGroup`, so when sections stack the meaning is "new group", not "rule".
14. **Menu sections stack rather than replace.** Right-clicking a model that
    belongs to a folder gives Group/Ungroup **then** folder actions **then**
    model actions, in that fixed push order; only the *presence* conditions vary.
    The section order does not follow the click target.
15. **`showGroupSection` shows up to two disabled rows.** On a plate header
    (`!contextMenu.modelId` makes it true) both `Group selected` and `Ungroup
    selected` render, normally disabled by the selection predicates.
16. **Plate delete has two paths with different confirmation.** Empty plate →
    `onRemovePlate` immediately. Plate holding models → modal. `removePlate`
    itself also refuses the first plate and the last plate; the menu disablement
    mirrors that (`plates.length <= 1 || plates[0].id === plateId`).
17. **The modal's subtitle is resolved at menu-open time**, from
    `plates[index].name.trim() || plateNumberPlaceholder(index+1)`. Quoted
    comment: *"The wording is settled here, where the plate is known, rather than
    in the modal, where the list may already have moved on."*
18. **`AnalysisPanelStack`'s `TerritoryVoxelControls.islandCount` is
    `islands.voxelEnabled ? count : count`** — a conditional with the same
    expression on both sides. Preserve as a constant; do not "fix" it into 0 when
    disabled without checking the card.
19. **Rename asymmetry.** Model and folder renames trim and refuse empty input
    (silent cancel); plate rename **writes whatever is in the field on blur/Enter**,
    including `''`, and the display then falls back to the plate placeholder.
    Both plate and folder/model inputs commit on blur; there is no
    cancel-on-blur, and Escape only closes the editor.
20. **Model rename keeps the extension out of the editable field.**
    `splitModelNameSuffix` separates `.stl`-style suffixes with
    `/^(.*?)(\.[^.\s]+)$/`, so the field edits the base and the suffix span is
    shown read-only and re-appended at commit. A name that is *only* an extension
    (`.stl`) gets the translatable fallback base `Model`.
21. **Message descriptors live at module scope on purpose.** Quoted from
    `formatModelInfoTooltip`: *"Built at module scope because React Compiler
    renames locals ahead of the Lingui macro, which desyncs an interpolated
    message id and leaves the placeholders raw in production — the same reason
    page.tsx keeps its formatters out here."* Same reason for `bodyNameLabel`,
    `bodyCountLabel` and `plateNumberPlaceholder`. The Rust port uses literal
    msgids, so it must reproduce the *id* the macro generates
    (`Body {0}`, `Plate {0}`, `{0} polygon`/plural), not re-derive one at the call
    site.
22. **Tooltip text is itself a translated composite:**
    `translate(msg\`${size} · ${polygons}\`)` — the separator and order are
    translator-visible, and when `formatFileSize` returns `null` (size unknown,
    ≤ 0 or non-finite) the tooltip is polygons alone.
23. **`formatFileSize` ladder:** `GB` at ≥ GB, `MB` at ≥ MB, `KB` at ≥ KB, else
    `B`, all `quantity(value, 2|2|1|0)`, binary multiples (1024). Zero means "not
    known", not "empty" — quoted comment in `meshStatsFormatting.ts`.
24. **`formatPolygonCountCompact`:** ≥ 1 000 000 → millions with 2 decimals
    below 10 (`0` at ≥ 10); ≥ 1 000 → thousands with 1 decimal below 10 (`0` at
    ≥ 10); else `fixed(count, 0)`. Numbers are locale-formatted
    (`Intl.NumberFormat`, fixed digits).
25. **Row action buttons stop propagation** (info/eye/trash/chevron all
    `e.stopPropagation()`), so pressing one never changes the selection. The row
    body itself does not stop propagation.
26. **Shift-range is inclusive of both anchors and spans `orderedModelIds`**
    (folders flattened). Without `onSelectRange` the fallback issues one
    `onSelect` per id (first `'single'`, rest `'add'`), which is N store writes
    instead of one.
27. **Ctrl/Meta is toggle, Shift is range** — and Shift + Ctrl/Meta makes the
    range additive (`'add'`), otherwise `'replace'`. On group headers Shift is
    treated as additive too (`ctrl || meta || shift`), a different meaning from
    the row.
28. **`selectFolder` bypasses the group API for the system group** so "Outside
    plate" selects its models individually, and for standalone rows
    (`isGrouped: false`) which have no header but can still be a target.
29. **The info button only exists when `onOpenSupportsInfo` is provided**
    (`page.tsx:5689` sets `supportsInfoModelId`, which opens `ModelSupportsModal`,
    another area); the body drawer chevron only when `splitBodies.length > 1`;
    the menu's model section only when at least one of
    `onRenameModel/onModelContextMenu/onRepairModel` exists.
30. **Panel max height is viewport-derived, not content-derived:**
    `100vh - var(--topbar-height) - max(140, round(bottomClearancePx))`, applied
    only while expanded. `bottomClearancePx` comes from
    `modelStatsBottomClearancePx` (default 220, measured in `page.tsx:2184`).
31. **The dock-resize strip is opacity-0 until hover and only works when the
    panel is inside a `[data-dock="left|right"]`.** It resizes the *dock*, never
    the panel: the comment explains that writing the panel's own width would be
    undone by the next render.
32. **The header is the drag handle** (`data-panel-drag-handle`), which is why
    the plus button sits absolutely positioned in the right slot and the left
    cluster reserves `pr-8`; a port must not make the header a plain row.
33. **The plus replaced a model count.** Do not re-add a count to the header.
34. **Support mode mounts `ModelsPanel` directly with key `support-models`**
    and the same props as Prepare; Export/Printing/Analysis do not mount it at
    all.
35. **`SharedPanelStack` is developer-only and `i18n-ignore`d.** Its strings are
    English by policy; the only contract is the toggle (`Ctrl+Shift+X`) and that
    it closes through its own `Close` button. `SupportInspectorPanel` is mounted
    inside it unconditionally, not only in support mode.
36. **`PrintingPanelStack`'s `'No printer selected'` fallback is hardcoded
    English** — it is not a `msg` id. `showSendTargetPicker` is strictly
    `fleet.length > 1`, so a one-printer fleet sends immediately.
37. **`ExportPanelStack` is hook-free by design** and therefore cannot
    translate; plate names pass through raw and the nameless case is the panel's
    business.

## Open questions

1. **Plate-header selection intent.** Does clicking a plate header need to
   select the models standing on it? The comment says so; `scene.selectGroup`
   keys on `model.groupId`, so today only `activatePlate` (which clears the
   selection) has an effect. Decide against a running build.
2. **`select-folder` for a model inside a plate-side folder.** `contextGroup`
   only searches the top-level list, so the entry silently does nothing for
   nested folders. Intended?
3. **Child folder context menu.** Nested folder headers have no
   `onContextMenu`; right-click there cannot rename/ungroup a nested folder, even
   though `rename-folder`/`ungroup-folder` handlers would work from a model's
   menu. Is the omission deliberate?
4. **Escape on the plate rename input.** Escape only calls
   `setRenamingPlateId(null)`; the input unmounts while focused, and the
   `onBlur` commit is attached. Whether React fires that blur on unmount (a
   spurious rename) needs a DOM check.
5. **`onModelContextMenu` position reuse.** The panel passes the *panel* menu
   coordinates to the editor menu, so the editor menu opens at the same x/y. Is
   that intended to be the same gesture, or should the second menu be positioned
   from the body row?
6. **Data the port lacks.** The panel needs a model list (id/name/group/plate/
   visible/polygonCount/fileSize/splitBodies/bodyColors), plate list with active
   plate and rename/remove, per-model selection with `single|toggle|add` and
   range, body selection keys, and an undo-capable scene. `DragonFruit` today has
   one `mesh` and one `selected: Rc<Cell<bool>>`; the spec's behaviour cannot be
   finished until the scene grows those fields. Also missing: a translated-
   descriptor path (`gpui-locale` `t!` works from literals, so the msgids must be
   written literally), the details modal, the mesh-repair flow, the
   split-to-bodies flow and the visibility semantics beyond the single mesh.
7. **`formatFileSize`'s "cannot be known" state** in the port: `MeshData` has no
   file size, so the info tooltip is polygons-only today. If the port gains a
   size, the `{size} · {polygons}` message must be the translated composite, not
   a concatenation.
8. **Group name fallback `'Group'`** in the rename handler is untranslated in
   the source; whether the port keeps that or introduces a msgid is undecided.
9. **Analysis-mode mounting.** No Rust module owns the six analysis cards or the
   stack order; whether Analysis becomes a fourth `Mode` (it is not one today in
   `app.rs`) is a shell decision, not this spec's.
