# Settings — General, Camera, Workspace, Multi-Plate

Scope: `src/components/settings/GeneralSettingsTab.tsx`, `CameraSettingsTab.tsx`,
`WorkspacesSettingsTab.tsx`, `MultiPlateSettingsTab.tsx`, every shared component they
render (`atoms/SettingRow`, `atoms/SegmentedControl`, `atoms/Button`, `atoms/IconButton`,
`ui/SelectDropdown`, `ui/LanguageSwitcher`, `ui/ScrollableNumberField`, `ui/NumberInput`),
the preference modules behind them, and the host-shell contract of `SettingsModal`
(the tab strip, the draft/Apply/Cancel model, and the four tabs' metadata).

The worktree has **no `gpui/` tree** — `gpui/crates/gpui-backend/` is referenced by
`docs/internal/gpui-backend-map.md` but the directory does not exist here. Every
"Port target" row below is therefore a proposal for where the module should live;
**nothing exists there yet, so there is no Rust to reuse.** Proposed layout follows
the one crate name the docs actually use:

```
gpui/crates/gpui-app/src/features/settings/mod.rs        (tab enum + host shell)
gpui/crates/gpui-app/src/features/settings/general.rs
gpui/crates/gpui-app/src/features/settings/camera.rs
gpui/crates/gpui-app/src/features/settings/workspaces.rs
gpui/crates/gpui-app/src/features/settings/multi_plate.rs
gpui/crates/gpui-app/src/ui/setting_row.rs               (SettingRow)
gpui/crates/gpui-app/src/ui/segmented_control.rs         (SegmentedControl)
gpui/crates/gpui-app/src/ui/select_dropdown.rs           (SelectDropdown)
gpui/crates/gpui-app/src/ui/number_field.rs              (NumberInput + ScrollableNumberField)
gpui/crates/gpui-app/src/features/settings/prefs/*.rs    (one module per preference file)
```

## Components in scope

| Component | File |
|---|---|
| `GeneralSettingsTab` | `src/components/settings/GeneralSettingsTab.tsx` |
| `CameraSettingsTab` | `src/components/settings/CameraSettingsTab.tsx` |
| `WorkspacesSettingsTab` | `src/components/settings/WorkspacesSettingsTab.tsx` |
| `MultiPlateSettingsTab` | `src/components/settings/MultiPlateSettingsTab.tsx` |
| `SettingRow` | `src/components/atoms/SettingRow.tsx` |
| `SegmentedControl` | `src/components/atoms/SegmentedControl.tsx` |
| `Button` | `src/components/atoms/Button.tsx` |
| `IconButton` (steppers in `ScrollableNumberField`) | `src/components/atoms/IconButton.tsx` |
| `SelectDropdown` | `src/components/ui/SelectDropdown.tsx` |
| `LanguageSwitcher` | `src/components/ui/LanguageSwitcher.tsx` |
| `ScrollableNumberField` | `src/components/ui/scrollableNumberField.tsx` |
| `NumberInput` | `src/components/ui/NumberInput.tsx` |
| UI-scale preference | `src/components/settings/uiScalePreference.ts` |
| Support-tooltip preference | `src/components/settings/supportPlacementPreferences.ts` |
| Import-defaults preference | `src/features/scene/importDefaultsPreferences.ts` |
| Floating-layout / tool-layout preference | `src/components/layout/floatingLayoutPreferences.ts` |
| Camera projection preference | `src/components/settings/cameraProjectionPreferences.ts` |
| Camera feel preference | `src/components/settings/cameraFeelPreferences.ts` |
| Camera FOV preference | `src/components/settings/cameraFovPreferences.ts` |
| Camera trackpad preference | `src/components/settings/cameraTrackpadPreferences.ts` |
| Workspace camera preference | `src/components/settings/workspaceCameraPreferences.ts` |
| 3D view preference | `src/components/settings/view3dPreferences.ts` |
| Multi-plate preference | `src/components/settings/multiPlatePreferences.ts` |
| Plate cascade constants | `src/features/scene/plates/plateCascade.ts` |
| Host shell (`SettingsModal`, `SettingsSidebarTab`, tab metadata, footer) | `src/components/settings/SettingsModal.tsx:215-315`, `:1209-1315`, `:1355-1450`, `:1773-1810` |

---

## Surface: the settings host shell — the frame all four tabs render inside

- **Source**: `src/components/settings/SettingsModal.tsx:1355` (dialog), `:1372-1395`
  (header), `:1397-1428` (sidebar), `:1430-1478` (content + these four tabs),
  `:1773-1810` (footer). Sidebar item: `SettingsSidebarTab` at `:228-270`;
  `TabDescription` at `:280-315`.
- **Renders**: fixed-size modal panel: header row (icon tile, title "Settings", X
  close), a two-column body (72-wide-unit sidebar of tab cards, scrollable content),
  and a footer with Restore Defaults / Cancel / Apply.
- **Layout & styling**:
  - Dialog: `className="w-full max-w-[72rem] h-full flex flex-col rounded-2xl shadow-2xl border overflow-hidden ui-modal-panel-enter"`,
    `background: 'var(--surface-0)'`, `borderColor: 'var(--border-strong)'`,
    `boxShadow: '0 26px 64px rgba(0, 0, 0, 0.46)'`.
  - Header: `className="flex items-center justify-between px-4 py-3"`,
    `borderBottom: '1px solid var(--border-subtle)'`. Icon tile:
    `className="inline-flex h-9 w-9 items-center justify-center rounded-lg border"`,
    `background: 'linear-gradient(135deg, color-mix(in srgb, var(--accent), var(--surface-1) 84%), color-mix(in srgb, var(--accent-secondary), var(--surface-1) 90%))'`,
    `Settings2` icon `className="h-4.5 w-4.5"` `color: 'var(--accent)'`.
  - Title: `h2 className="text-base font-semibold"` `color: 'var(--text-strong)'`;
    subtitle `p className="text-xs"` `color: 'var(--text-muted)'`.
  - Close: `IconButton variant="solid" size="sm" aria-label="Close"` with `<X className="h-4 w-4" />`.
  - Body: `className="flex-1 min-h-0 flex"`.
  - Sidebar column: `className="w-72 min-h-0 p-2.5"`,
    `borderRight: '1px solid var(--border-subtle)'`,
    `background: 'linear-gradient(180deg, color-mix(in srgb, var(--surface-1), transparent 6%), color-mix(in srgb, var(--accent-secondary), var(--surface-1) 96%))'`.
    Its scroller: `className="h-full min-h-0 overflow-y-auto custom-scrollbar flex flex-col"`;
    top group `className="space-y-1"`, bottom group `className="mt-auto space-y-1 pt-3"`.
    Deliberately no `pr-1` (comment at `:1398-1402`: "the column's 2.5 already insets
    the tab cards, and a second inset here stacked on top of the 10px scrollbar gutter").
  - Sidebar tab (`:245-265`): `className="w-full rounded-sm border px-2.5 py-1.5 text-left transition-all duration-150"`.
    Active style: `borderColor: color-mix(in srgb, ${tabColor}, var(--border-subtle) 35%)`,
    `background: color-mix(in srgb, ${tabColor}, var(--surface-0) 84%)`,
    `boxShadow: 0 0 0 1px color-mix(in srgb, ${tabColor}, transparent 76%) inset`;
    idle: `borderColor: 'var(--border-subtle)'`, `background: 'var(--surface-1)'`.
    `tabColor` = `var(--accent-secondary)` for `tone: 'secondary'`, else `var(--accent)`
    (all four tabs in scope are `tone: 'primary'`). Inner row `className="flex items-center gap-2.5"`,
    icon `className="h-5 w-5 shrink-0"` coloured `active ? tabColor : 'var(--text-muted)'`,
    label `className="block text-[15px] font-semibold leading-tight"` `color: 'var(--text-strong)'`.
  - Content column (`:1430`): `'flex-1 min-h-0 overflow-y-auto custom-scrollbar p-4'`
    (`usesInternalTabScrollLayout` — true only for `about`/`hotkeys`/`updates` — is
    false for all four tabs here, so plain page scroll applies).
  - Inner wrapper (`:1432`): `key={activeTab}`, `className="animate-[settingsTabIn_180ms_ease-out]"`.
  - Footer (`:1776`): `className="px-3 py-3 flex items-center justify-between gap-2"`,
    `borderTop: '1px solid var(--border-subtle)'`,
    `background: 'color-mix(in srgb, var(--surface-1), transparent 10%)'`.
- **States**: one tab active at a time; the content remounts on tab change (animation).
  Modal stays mounted while closed (`isOpen` toggles visibility) and `activeTab`
  resets to the caller's `initialTab ?? 'general'` on each open.
- **Data**: `SettingsTabKey` union at `:215`:
  `'general' | 'camera' | 'workspaces' | 'multiPlate' | 'mesh' | 'performance' | 'spacemouse' | 'plugins' | 'experiments' | 'sceneAutosave' | 'backups' | 'uvtools' | 'ui' | 'hotkeys' | 'logging' | 'updates' | 'about'`.
  Sidebar order (`:1314-1315`):
  top `['general', 'camera', 'workspaces', 'multiPlate', 'mesh', 'performance', 'spacemouse', 'ui', 'hotkeys']`,
  bottom `['plugins', 'experiments', 'sceneAutosave', 'backups', 'uvtools', 'logging', 'updates', 'about']`.
- **Callbacks**: `handleSelectTab` (`:419`) records `previousTabRef` then sets the tab.
  `handleApply` (`:968`) writes every draft (incl. `applyLocale(draftLocale)`,
  `setFloatingLayoutPersistenceEnabled`, `setToolLayout`, `saveImportDefaultsSettings`,
  `saveCameraProjectionSettings`, `saveCameraFovSettings`, `saveCameraFeelSettings`,
  `saveCameraTrackpadSettings`, `saveWorkspaceCameraSettings` — the last one preserves
  the saved `selectionHighlightDefaults` untouched). `handleCancel` (`:876`) discards
  drafts. `handleRestoreDefaults` (`:928`) opens a confirm dialog
  (`StructuredDialogModal`, title "Restore Defaults?", body ends
  "Nothing is written until you click **Apply**.") and
  `applyRestoreDefaultsToDraft` (`:891`) resets all drafts.
- **Text**: "Settings" / "Customize DragonFruit behavior, visuals, and controls." /
  "Restore Defaults" / "Cancel" / "Apply" — all plain strings, untranslated.
- **Port target**: `gpui/crates/gpui-app/src/features/settings/mod.rs` — nothing exists.
  The four tabs in scope are hosted here; other tabs are other specs' business.
- Tab metadata for the four tabs in scope (`:1209-1250`), `msg`-translated:

  | key | label (msgid) | description (msgid) | icon |
  |---|---|---|---|
  | `general` | `General` | `Workspace behavior and panel layout` | `Settings2` |
  | `camera` | `Camera` | `Projection and navigation behavior` | `Camera` |
  | `workspaces` | `Workspace` | `Build volume bounds, origin, and 3D view resolution` | `MonitorCog` |
  | `multiPlate` | `Multi-Plate` | `Plate behaviour across several beds` | `SquareStack` |

---

## Surface: `SettingRow` — the label/description-left, control-right row

- **Source**: `src/components/atoms/SettingRow.tsx:48-118`.
- **Renders**: a `div` (or `label` when `as="label"`) with two columns: a `min-w-0`
  text block (label line + optional description line) and an `inline-flex items-center gap-2`
  control slot.
- **Layout & styling**:
  - Root: `className={cn('flex items-center justify-between gap-3', bordered && cn('rounded-md border', compact ? 'p-1.5' : 'p-2'), className)}`.
  - When `bordered`: `borderColor: 'var(--border-subtle)'`,
    `background: surface === 'raised' ? 'var(--surface-1)' : 'var(--surface-0)'`.
  - `opacity: disabled ? 0.68 : undefined`. Caller `style` is spread last and wins.
  - Text block: `cn('min-w-0', bordered && 'pl-1')` — the extra left pad exists because
    "text against the border reads cramped in a way a pill or a field does not".
  - Label: `className={cn('text-xs font-semibold', compact && 'leading-tight')}`,
    `color: 'var(--text-strong)'`.
  - Description: `className={compact ? 'text-[11px] leading-tight' : 'text-xs'}`,
    `color: 'var(--text-muted)'`. Always visible (comment: "Both lines stay visible:
    the description carries the behaviour, so hiding it behind a hover was worse than
    the space it saved.").
- **States**: `density` `'compact'` (default, 40px bordered row) vs `'comfortable'`
  (`p-2`, `text-xs` description, 50px); `bordered` on/off; `surface` `'sunken'`
  (`--surface-0`, default) vs `'raised'`; `disabled` dims to 0.68 (it does **not**
  disable the child control — the caller must do that).
- **Data**: props `label`, `description`, `density`, `children`, `bordered`, `surface`,
  `as`, `disabled`, `className`, `style` plus passthrough `HTMLAttributes`.
- **Callbacks**: none.
- **Text**: none of its own.
- **Port target**: `gpui/crates/gpui-app/src/ui/setting_row.rs` — nothing exists.

## Surface: `SegmentedControl` — mutually exclusive pills (every ON/OFF in these tabs)

- **Source**: `src/components/atoms/SegmentedControl.tsx:36-93`.
- **Renders**: `role="group"` container with one `<button role>` per option.
- **Layout & styling**:
  - Container: `cn('inline-flex items-center gap-1', fullWidth && 'w-full', className)`,
    style `{'--segmented-tone': tone === 'accent' ? 'var(--accent)' : 'var(--accent-secondary)'}`.
  - Option: `'ui-segmented-option inline-flex items-center justify-center rounded-sm border font-semibold uppercase tracking-wide transition-colors'`
    plus size class `md` (default, used by every control in scope):
    `'h-10 min-w-[92px] px-3 text-[12px]'`; `sm` is `'h-7 px-2.5 text-xs'`.
    `fullWidth && 'flex-1'`. `data-active={active ? 'true' : undefined}`.
  - `.ui-segmented-option` (globals.css:423-437): idle `background: var(--surface-1)`,
    `border-color: var(--border-subtle)`, `color: var(--text-muted)`;
    hover `background: var(--surface-2)`; active
    `border-color: color-mix(in srgb, var(--segmented-tone, var(--accent)), white 10%)`,
    `background: color-mix(in srgb, var(--segmented-tone, var(--accent)), var(--surface-0) 76%)`,
    `color: color-mix(in srgb, var(--segmented-tone, var(--accent)), var(--text-strong) 25%)`.
- **States**: default / hover / active (`data-active`) / `disabled` per option
  (`disabled` attr) / `title` tooltip when given.
- **Data**: `options: {value, label, title?, disabled?}[]`, `value`, `label` (accessible
  group name), `size`, `tone`, `className`, `fullWidth`.
- **Callbacks**: `onChange(option.value)` on click; the control is fully controlled —
  it never stores the value itself.
- **Text**: none of its own; all labels come from the caller.
- **Port target**: `gpui/crates/gpui-app/src/ui/segmented_control.rs` — nothing exists.

## Surface: `Button` / `IconButton` — the Reset button and the numeric steppers

- **Source**: `src/components/atoms/Button.tsx:45-59`; `src/components/atoms/IconButton.tsx:50-95`.
- **Renders**: `Button` → `<button type="button" class="ui-button <variant> <size> <className>">`;
  `IconButton` (default `variant="solid"`) → `ui-button` + `ui-button-primary` when
  `active` else `ui-button-secondary`, with a size padding class.
- **Layout & styling** (from globals.css:347-410): `.ui-button` is
  `display:inline-flex; align-items:center; justify-content:center; border:1px solid transparent;
  border-radius: var(--radius-sm); padding: 5px 11px 7px 11px; font-size: var(--font-size-md);
  font-weight:600; line-height:1; transition: background-color 140ms ease, border-color 140ms ease,
  color 140ms ease; cursor:pointer`. `size="auto"` emits **no** geometry
  (`sizeClassMap.auto = ''`), which is why the Reset button carries its own
  `!h-10 !px-3 !py-0 text-sm`. `ui-button-secondary`: `background: var(--surface-1)`,
  `color: var(--text-strong)`, `border-color: var(--border-subtle)`, hover
  `background: var(--surface-2)`.
- **States**: default / hover / disabled (`disabled` attr) / `active` (IconButton only).
- **Callbacks**: native `onClick`.
- **Text**: caller-supplied.
- **Port target**: shared `gpui/crates/gpui-app/src/ui/button.rs` — nothing exists.

## Surface: `SelectDropdown` — the toolbar, raft-base, camera-feel and language pickers

- **Source**: `src/components/ui/SelectDropdown.tsx:94-556`.
- **Renders**: an outer `<label class="space-y-1 block ${className}">`; optional label
  span (never used by these tabs — they pass no `label`, so the wrapper is a bare
  `<label>` around the trigger); a `relative` container; a `<button role="combobox">`
  trigger; and, while open, a **portal to `document.body`** with
  `role="listbox"` and one `role="option"` button per entry.
- **Layout & styling**:
  - Trigger: `"ui-input rounded-[4px] relative w-full h-[36px] px-2.5 py-0 pr-10 leading-none text-sm disabled:pointer-events-none disabled:opacity-55 disabled:cursor-not-allowed inline-flex items-center text-left " + selectClassName`.
    Open state adds `borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 18%)'`
    and `boxShadow: '0 0 0 1px color-mix(in srgb, var(--accent), transparent 72%) inset'`.
  - Leading slot: `"mr-1.5 inline-flex h-[18px] w-[18px] shrink-0 items-center justify-center"`.
  - Value text: `"min-w-0 flex-1 truncate"` (+ `text-center` when `selectedTextAlign`).
  - Chevron: `ChevronDown` `"pointer-events-none absolute right-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 transition-transform"`,
    `rotate-180` when open, colour `var(--text-muted)`.
  - Menu: `"fixed z-[9999] rounded-[4px] border shadow-xl"` +
    `borderColor: 'color-mix(in srgb, var(--accent), var(--border-subtle) 82%)'`,
    `background: 'color-mix(in srgb, var(--surface-0), black 6%)'`,
    `backdropFilter: 'blur(6px) saturate(112%)'` (and `WebkitBackdropFilter`),
    `boxShadow: '0 14px 34px rgba(0, 0, 0, 0.46), 0 4px 12px rgba(0, 0, 0, 0.32)'`,
    width/min-width forced to the trigger width, `maxHeight` clamped to the viewport side.
  - Option rows: `"group w-full px-3 py-2 text-left text-sm transition-colors inline-flex items-center gap-2 border-b last:border-b-0"`.
    Idle text `var(--text-muted)`; selected/hovered `var(--text-strong)`; selected
    background `color-mix(in srgb, var(--accent), var(--surface-0) 86%)`; hover
    `color-mix(in srgb, var(--surface-2), transparent 18%)`; the last row and any row
    followed by a heading/divider get `borderBottomColor: 'transparent'`.
- **States**: closed / open / hovered option / selected option / `disabled` option
  (`opacity: 0.55`, `cursor: not-allowed`) / `disabled` trigger / focus ring (accent
  border) / "no room below" → menu flips above when `belowSpace < min(180, menuHeight)`
  and there is more room above.
- **Data**: fully controlled by `value` + `options`; open/hover/focus/menu-position are
  internal state. `selectedOption` lookup skips `heading`/`divider` entries.
- **Callbacks**: `onChange(value)` then close; `onFocus`/`onBlur`; Escape and
  outside-pointer-down close the menu **without** changing the value; resize/scroll
  reposition rather than close (`dismissOnResize: false, dismissOnScroll: false`).
- **Text**: caller-supplied option labels; no chrome text.
- **Port target**: `gpui/crates/gpui-app/src/ui/select_dropdown.rs` — nothing exists.
  GPUI replaces the portal + `getBoundingClientRect` maths with an
  anchored/deferred overlay; the *behaviour* to preserve is: width-matched menu,
  right-aligned variant, viewport clamp, flip upwards, close on Escape/outside click.
- Callers in scope pass `className="w-36"`, `menuAlign="right"`, and (language +
  UI scale) `selectClassName="!text-[13px] !pr-9"` because `.ui-input` (unlayered CSS)
  otherwise overrides the trigger's `text-sm`/`pr-10`.

## Surface: `LanguageSwitcher` — interface-language picker

- **Source**: `src/components/ui/LanguageSwitcher.tsx:16-49`.
- **Renders**: a `SelectDropdown<Locale>` with a leading `Languages` icon.
- **Layout & styling**: `className={`w-36 ${className}`}` (settings passes none),
  `menuAlign="right"`, `leadingDisplay={<Languages className="w-4 h-4" />}`,
  `selectClassName="!text-[13px] !pr-9"`.
- **States**: controlled when `value`/`onChange` are provided (the settings path),
  otherwise live-switches the app locale through `useLocale()`.
- **Data**: `SUPPORTED_LOCALES = ["en", "es", "de", "fr", "sk", "cs"]` and
  `LOCALE_LABELS` (English, Español, Deutsch, …) from `src/i18n.ts:4-19`.
- **Callbacks**: `handleChange = onChange ?? setLocale`.
- **Text**: option labels are the native language names; `ariaLabel`/`title` are
  `_(msg`Language`)`.
- **Port target**: `gpui/crates/gpui-app/src/features/settings/general.rs` (or a shared
  `ui/language_switcher.rs`) — nothing exists. The locale enum and labels need a GPUI
  i18n story first (see Open questions).

## Surface: `ScrollableNumberField` — custom UI scale field

- **Source**: `src/components/ui/scrollableNumberField.tsx:31-260`.
- **Renders**: `[−]` IconButton, a text `<input inputMode="decimal">` with an optional
  unit suffix, `[+]` IconButton, in one flex row.
- **Layout & styling**:
  - Row: `cn('flex min-w-0 items-center', compact ? 'gap-0.5' : 'gap-1', className)`.
  - Steppers (`IconButton`, default solid): `cn(compact ? '!h-6 !w-6' : '!h-8 !w-8', 'shrink-0 !p-0 !bg-[var(--surface-0)] hover:!bg-[color-mix(in_srgb,var(--surface-0),white_4%)] disabled:!bg-[color-mix(in_srgb,var(--surface-1),black_8%)]')`
    with `Minus`/`Plus` at `h-3.5 w-3.5` (compact `h-3 w-3`).
  - Input wrapper `"relative w-0 min-w-0 flex-1"`; input
    `'ui-input w-full text-center tabular-nums font-semibold no-spinners !bg-[var(--surface-0)]'`
    + `h-8 px-1.5 text-xs sm:text-sm`, plus `pr-8` when `unit` (compact: `h-7 px-1 text-[11px]`, `pr-4`).
  - Unit span: `'pointer-events-none absolute top-1/2 -translate-y-1/2 font-medium right-2 text-[10px]'`,
    colour `var(--text-muted)`.
- **States**: disabled (whole field); decrement disabled at `<= min`, increment at `>= max`;
  editing (focused) suppresses external-value writes; `commitOnBlur` defers the write.
- **Data**: `value`, `min`, `max`, `step`, `unit`, `ariaLabel`, `commitOnBlur`, `compact`.
  Internal `draftValue` string; `fmt.parse`/`fmt.field` from `useLocaleFormat()`.
- **Callbacks**: `onChange(clampedNumber)` on step click, on typed input without
  `commitOnBlur`, on Enter/blur commit, and on wheel (only when focused or Shift held;
  a native non-passive listener is required — comment at `:88-92`: "React attaches its
  `wheel` listener at the root as PASSIVE, so `preventDefault()` inside an onWheel handler
  is silently ignored"). ArrowUp/ArrowDown step; Escape reverts and blurs.
- **Text**: stepper `title`s "Decrease value"/"Increase value"; `ariaLabel` supplied;
  no other text.
- **Port target**: `gpui/crates/gpui-app/src/ui/number_field.rs` — nothing exists.

## Surface: `NumberInput` — build-volume and beds-per-row fields

- **Source**: `src/components/ui/NumberInput.tsx:47-215`.
- **Renders**: a `relative min-w-0` wrapper around a text input plus a 4-wide stepper
  stack (`ChevronUp` / `ChevronDown`, each `h-3 w-3` hit box, `h-2.5 w-2.5` icon,
  `hover:bg-white/10 disabled:opacity-50`), positioned
  `absolute inset-y-0 right-0.5 flex w-4 flex-col items-center justify-center gap-0.5`.
- **Layout & styling**: caller owns the input's class entirely (Workspaces passes
  `mt-1 h-9 w-full rounded-md border pl-2 pr-5 text-[12px]` with inline
  `borderColor: var(--border-subtle)`, `background: var(--surface-1)`,
  `color: var(--text-strong)`; MultiPlate passes
  `ui-input h-[34px] w-[120px] py-1.5 pl-2.5 pr-5 text-sm`).
- **States**: editing (keeps the raw string) / blur (snaps back to the committed prop,
  comment: "Always snap back to the latest committed prop value on blur") /
  increment-disabled at `max`, decrement-disabled at `min` / `disabled` / `readOnly`
  (wheel ignored).
- **Data**: `value: number` (non-finite renders as `0`), `min`, `max`, `step`,
  `showStepper` (default true), plus native input props.
- **Callbacks**: `onChange(parsedNumber)` per keystroke while the typed string matches
  the locale-aware pattern `^sign?(?:\d+)?(?:[.,]\d{0,2})?$` (a `min >= 0` field
  rejects a leading `-`); steppers and wheel call the clamped step; `onBlur`/`onFocus`
  forwarded.
- **Text**: stepper aria-labels `_(msg`Increase value`)` / `_(msg`Decrease value`)`.
- **Port target**: same `ui/number_field.rs` — nothing exists.

---

## Surface: `GeneralSettingsTab` — General tab

- **Source**: `src/components/settings/GeneralSettingsTab.tsx:48-534`.
- **Renders**: `<div className="space-y-3">` with six sibling `<section>` cards:
  Language, UI Scale, Toolbar, Floating Windows, Support Tooltips, Import Defaults.
- **Layout & styling** (repeated verbatim six times):
  ```jsx
  <section className="rounded-md border p-3"
    style={{ background: 'var(--surface-1)', borderColor: 'var(--border-subtle)' }}>
    <div className="flex items-start gap-2">
      <span className="inline-flex h-8 w-8 items-center justify-center rounded-md border"
        style={{ borderColor: 'var(--border-subtle)',
                 background: 'color-mix(in srgb, var(--surface-2), transparent 8%)' }}>
        <Languages className="h-4 w-4" style={{ color: 'var(--accent)' }} />
      </span>
      <div className="flex-1">
        <h3 className="text-xs font-semibold" style={{ color: 'var(--text-strong)' }}>Language</h3>
        <p className="text-xs mt-0.5" style={{ color: 'var(--text-muted)' }}>…</p>
      </div>
    </div>
  ```
  Only the Lucide icon, the `h3` text and the `p` text change per section. All other
  rows are `<SettingRow bordered …>` with `className="mt-3"` for the first row of a
  section and `className="mt-2"` for the rest.

### Section 1 — Language (`:140-175`)
- **Icon**: `Languages`. **Heading**: "Language". **Description**: "Choose the interface language. Detected from your system on first run; your choice is remembered."
- **Row**: `bordered className="mt-3"`, label `_(msg`Interface language`)`,
  description `_(msg`Applied immediately across the app.`)`.
- **Control**: `<LanguageSwitcher value={language} onChange={onLanguageChange} />`
  (`w-36`, `menuAlign="right"`, leading `Languages` icon, `!text-[13px] !pr-9`).
- **States**: none conditional. The description lies slightly: the modal keeps
  language as a draft and only calls `applyLocale` in `handleApply`
  (`SettingsModal.tsx:435-439` comment: "Language is a draft like every other setting:
  changing the switcher only updates draftLocale; the actual loadLocale happens in handleApply.").

### Section 2 — UI Scale (`:177-243`)
- **Icon**: `ZoomIn`. **Heading**: "UI Scale". **Description**: "Adjusts the size of the entire interface. 100% adapts to your screen automatically."
- **Inner panel**: `className="mt-3 rounded-md border p-2.5"`,
  `borderColor: 'var(--border-subtle)'`, `background: 'var(--surface-0)'`.
- **Row (not bordered)**: label `_(msg`Interface scale`)`,
  description `_(msg`Larger percentages magnify the whole UI.`)`.
- **Control**: `SelectDropdown<string>` with
  `value={isCustomScale ? 'custom' : String(uiScale)}`,
  options `UI_SCALE_PRESETS.map(p => ({ value: String(p), label: fmt.quantity(p * 100, 0, 'percent') }))`
  then `{ value: 'custom', label: 'Custom' }` — "Custom" is a **raw string**, not
  translated. `ariaLabel="Interface scale"`, `title="Interface scale"`,
  `className="w-36"`, `menuAlign="right"`,
  `leadingDisplay={<ZoomIn className="w-4 h-4" />}`, `selectClassName="!text-[13px] !pr-9"`.
- **Custom row**, rendered only when `isCustomScale` (`:221-240`):
  `<div className="mt-2.5 flex items-center justify-between gap-3">`, left text
  `className="text-xs"` colour `var(--text-muted)` reading "Custom scale";
  right `ScrollableNumberField` `className="w-36"`,
  `value={Math.round(uiScale * 100)}`, `min={MIN_UI_SCALE * 100}` (25),
  `max={MAX_UI_SCALE * 100}` (400), `step={1}`, `unit="%"`,
  `ariaLabel="Custom interface scale"`, `commitOnBlur`.
- **States**:
  - `isCustomScale = customScaleArmed || !UI_SCALE_PRESETS.includes(uiScale)` (`:67`).
  - Selecting a preset: `setCustomScaleArmed(false)`, `normalizeUiScale(Number(v))`,
    `setUiScale`, `saveUiScale` — writes localStorage immediately (`ui-scale`).
  - Selecting "custom": only `setCustomScaleArmed(true)`; **saves nothing** — source
    comment `:72-73`: "Selecting \"Custom\" arms the numeric input but saves nothing —
    the current scale stays until the user commits a value in the field."
  - Presets: `[0.75, 0.9, 1, 1.1, 1.25, 1.5]`; default `1`; clamp `[0.25, 4]`.
- **Data**: own state `uiScale` seeded from `getSavedUiScale()`; not part of the modal
  draft, not restored by Restore Defaults, not saved by Apply.
- **Callbacks**: `handleUiScaleChange`, `handleCustomUiScaleChange(percent)` →
  `normalizeUiScale(percent / 100)` → save.
- **Text**: "Custom", "Custom scale", percent labels via `fmt.quantity(…, 0, 'percent')`
  (`{value}%`), msgids listed above.
- **Port target**: `features/settings/general.rs` + `prefs/ui_scale.rs` — nothing exists.

### Section 3 — Toolbar (`:245-294`)
- **Icon**: `PanelTop`. **Heading**: "Toolbar". **Description**: "Park the tools in a column on the left edge, or in a bar centred under the app bar."
- **Row**: `bordered className="mt-3"`, label `_(msg`Toolbar position`)`,
  description `_(msg`A left column or a bar centred under the app bar.`)`.
- **Control**: `SelectDropdown<ToolLayout>` `value={toolLayout}`,
  options `[{ value: 'horizontal', label: _(msg`Top`) }, { value: 'vertical', label: _(msg`Left`) }]`,
  `ariaLabel`/`title` "Toolbar position", `className="w-36"`, `menuAlign="right"`,
  `leadingDisplay={toolLayout === 'horizontal' ? <ArrowUp className="w-4 h-4" /> : <ArrowLeft className="w-4 h-4" />}`.
- **States**: draft only — written by `setToolLayout` on Apply.
- **Text**: "Top", "Left".
- **Port target**: `features/settings/general.rs` + `prefs/floating_layout.rs`.

### Section 4 — Floating Windows (`:296-356`)
- **Icon**: `LayoutGrid`. **Heading**: "Floating Windows". **Description**: "Keep moved panel positions between sessions, or always start from the default workspace layout."
- **Row A**: `bordered className="mt-3"`, label `_(msg`Remember window positions`)`,
  description `_(msg`Persist dragged panel positions in local storage.`)`;
  `SegmentedControl` options OFF/ON (`_(msg`OFF`)`, `_(msg`ON`)`),
  `value={floatingLayoutPersistence ? 'on' : 'off'}`,
  `onChange={(next) => onFloatingLayoutPersistenceChange(next === 'on')}`.
- **Row B**: `bordered className="mt-2"`, label `_(msg`Reset saved window layout`)`,
  description `_(msg`Forget all stored panel positions and return to seeded layout.`)`;
  control is
  `<Button variant="secondary" size="auto" className="!h-10 !px-3 !py-0 text-sm inline-flex items-center gap-1.5 whitespace-nowrap">`
  with `<RotateCcw className="h-4 w-4 shrink-0" />` + "Reset" (raw string).
- **States**: persistence default is `true` when the key is absent
  (`isFloatingLayoutPersistenceEnabled`, `!== 'false'`).
- **Callbacks**: persist toggle → draft (applied by `setFloatingLayoutPersistenceEnabled`
  on Apply). Reset → `handleResetFloatingLayout` → `clearSavedFloatingLayout()` which
  removes **both** `lumenslicer:floating-panel-layout:v6` and the legacy `…:v5` key —
  it is immediate, not draft, and there is no confirmation.
- **Text**: "Reset", "OFF", "ON", plus the msgids above.
- **Port target**: `features/settings/general.rs` + `prefs/floating_layout.rs`.

### Section 5 — Support Tooltips (`:358-405`)
- **Icon**: `HelpCircle`. **Heading**: "Support Tooltips". **Description**: "Show placement tooltips next to the cursor while adding supports."
- **Row**: `bordered className="mt-3"`, label `_(msg`Show support tooltips`)`,
  description `_(msg`Shows "Cannot Place Support" and "Stability Warning" tooltips.`)`
  — note the nested double quotes inside the template literal.
- **Control**: `SegmentedControl` OFF/ON, `value={supportHelpEnabled ? 'on' : 'off'}`,
  `onChange`: `setSupportPlacementHelpEnabled(enabled)` **and** `setSupportHelpEnabled(enabled)`
  (immediate write to `app-support-placement-help-enabled`, plus a re-read subscription
  kept in sync by `subscribeSupportPlacementHelp` at `:69`).
- **States**: default `true` (`DEFAULT_SUPPORT_PLACEMENT_HELP_ENABLED`); stored as the
  literal string `'true'`/`'false'`; absent key → enabled.
- **Port target**: `features/settings/general.rs` + `prefs/support_placement.rs`.

### Section 6 — Import Defaults (`:407-530`)
- **Icon**: `Database`. **Heading**: "Import Defaults". **Description**: "Applied automatically when importing Scene Files."
- **Row 1**: label `_(msg`Default Raft Base`)`,
  description `_(msg`Chooses raft bottom mode for imported supports.`)`;
  `SelectDropdown<RaftBottomMode>` options
  `off → _(msg`Off`)`, `line → _(msg`Line`)`, `solid → _(msg`Solid`)`,
  `ariaLabel`/`title` "Default Raft Base", `w-36`, `menuAlign="right"`, no leading icon.
  `onChange` writes `{...importDefaults, raftBottomMode: next, rootsEnabled: next === 'line' ? true : importDefaults.rootsEnabled}`
  — choosing **Line** force-enables roots.
- **Row 2** (only when `raftBottomMode === 'solid'`): label `_(msg`Default Raft Wall`)`,
  description `_(msg`Enable perimeter wall for imported solid rafts.`)`; OFF/ON
  SegmentedControl bound to `raftWallEnabled`.
- **Row 3** (only when `!rootsLockedByLineRaft`, i.e. mode ≠ `'line'`): label
  `_(msg`Roots Enabled on Import`)`,
  description `_(msg`OFF makes imported root diameter match trunk diameter.`)`;
  OFF/ON bound to `rootsEnabled`.
- **Row 4**: label `_(msg`Auto-Repair`)`,
  description `_(msg`Automatically runs native mesh auto-repair for standard mesh imports.`)`;
  OFF/ON bound to `autoRepair`.
- **Row 5**: label `_(msg`Auto-Repair Scenes`)`,
  description `_(msg`Automatically runs native mesh auto-repair for scene-file imports.`)`;
  OFF/ON bound to `autoRepairScenes`.
- **States**: defaults `{ raftBottomMode: 'solid', raftWallEnabled: true, rootsEnabled: true, autoRepair: false, autoRepairScenes: false }`;
  normalization forces `rootsEnabled: true` whenever the stored mode is `line`.
- **Callbacks**: every row calls `onImportDefaultsChange({...importDefaults, <field>})`;
  the parent stores the draft and `saveImportDefaultsSettings` runs on Apply.
- **Text**: msgids above.
- **Port target**: `features/settings/general.rs` + `prefs/import_defaults.rs`.

### Dead code in this file (do **not** port)
`layoutDump`/`dumpStatus` state (`:61-62`), `handleDumpCurrentLayout` (`:90`),
`handleDumpSavedLayout` (`:103`), `handleCopyDump` (`:123`), and the `Bug`,
`ClipboardCopy` icon imports exist but **no JSX references them** — a floating-layout
dump UI was removed. Porting them would be inventing a feature. `FLOATING_LAYOUT_DEBUG_REQUEST_EVENT`
is imported solely by that dead handler.

---

## Surface: `CameraSettingsTab` — Camera tab

- **Source**: `src/components/settings/CameraSettingsTab.tsx:49-405`.
- **Renders**: `<div className="space-y-3">` with, in order: the "Camera Defaults"
  section, an optional "Higher Contrast Model Edges" section, and the "Trackpad
  Navigation" section.
- **Translation state**: this whole tab is **untranslated**. It imports `useLingui`
  and destructures `_` at `:73` but never calls it; every label, description, option
  and tooltip is a raw English string. The only locale-aware formatting is
  `fmt.quantity(x, 2, 'times')` for the acceleration read-outs.

### Section 1 — Camera Defaults (`:81-225`)
- **Icon**: `Camera`. **Heading**: "Camera Defaults". **Description**: "Global camera projection mode and navigation behavior."
- **Row A — scope**: `bordered className="mt-3"`, label "Camera scope",
  description "Choose one global projection mode for every workspace, or set projection defaults per workspace."
  `SegmentedControl label="Camera scope" value={cameraScope}` options
  `[{value:'global',label:'Global'},{value:'workspace',label:'Workspace'}]`.
- **Global branch** (`usingGlobalScope`, `:125-163`): `<div className="mt-2 space-y-1.5">` with
  - Row: `bordered`, label "Projection mode",
    description "Use one projection mode everywhere when global scope is active.";
    options `orthographic → 'Ortho'`, `perspective → 'Perspective'`.
  - If `cameraProjectionMode === 'perspective'` (`:143`), a panel
    `className="rounded-md border px-2.5 py-1.5"` (`borderColor: var(--border-subtle)`,
    `background: var(--surface-0)`) containing `className="space-y-0.5"`, a
    `<label className="text-xs flex justify-between" style={{color:'var(--text-muted)'}}>`
    with `<span>Field of view</span>` and
    `<span style={{color:'var(--text-strong)'}}>{perspectiveFov}°</span>` (raw degree
    sign, **not** `fmt`), then
    `<input type="range" min={FOV_MIN} max={FOV_MAX} step={1} value={perspectiveFov} onChange={e => onPerspectiveFovChange(parseInt(e.target.value, 10))} className="ui-range w-full" />`.
    `FOV_MIN = 15`, `FOV_MAX = 120`, default `50`.
- **Workspace branch** (`usingWorkspaceScope`, `:165-202`): `<div className="mt-2 space-y-1.5">` with
  - A panel (`px-2.5 py-1.5`, surface-0) titled "Workspace camera defaults"
    (`text-xs font-semibold`, `--text-strong`), body "Pick the default projection mode
    used when you enter each workspace." (`text-xs`, `--text-muted`), then
    `<div className="mt-2">` holding `SegmentedControl<SupportMode> label="Workspace camera defaults" fullWidth`
    over `workspaceMeta`, `value={activeWorkspace}`, `onChange={setActiveWorkspace}`.
  - A `SettingRow bordered` labelled `` `${label} default camera` `` with
    `` description={hint} `` resolved from `workspaceMeta`, control = segmented
    `orthographic/'Ortho'`, `perspective/'Perspective'`, bound to
    `workspaceCameraDefaults[activeWorkspace]` and
    `onWorkspaceCameraModeChange(activeWorkspace, next)`.
  - `workspaceMeta` (`:40-45`) contains **four** entries:
    `prepare`/"Prepare"/"Model prep and transform workflows",
    `analysis`/"Analysis"/"Island diagnostics and inspection tools",
    `support`/"Support"/"Support placement and editing workspace",
    `export`/"Export"/"Final output and export pipeline".
    `SupportMode` is `'prepare' | 'analysis' | 'support' | 'export' | 'printing'`
    (`src/supports/types.ts:9`), so **`printing` has a default in the store but no
    selectable tab**.
- **Row B — feel**: `bordered className="mt-2"`, label "Camera feel",
  description "Controls smoothing and movement acceleration while orbiting, panning, and zooming.";
  `SelectDropdown<CameraFeelPreset>` options `raw→'Raw'`, `precise→'Precise'`,
  `balanced→'Balanced'`, `fast→'Fast'`; `ariaLabel`/`title` "Camera feel",
  `className="w-36"`, `menuAlign="right"`, no leading icon.
- **States**: `activeWorkspace` is local state defaulting to `'prepare'` — it is not
  persisted and resets on remount (tab switch). Global and workspace branches are
  mutually exclusive (`cameraScope === 'global'` / `'workspace'`; the normalizer maps
  anything that is not `'global'` to `'workspace'`, while the store default is `'global'`).

### Section 2 — Higher Contrast Model Edges (`:227-273`) — normally invisible
- **Icon**: `Camera` (same icon as section 1). **Heading**: "Higher Contrast Model Edges".
  **Description**: "Overlay black edge lines on model geometry to improve shape definition."
- **Row**: `bordered className="mt-3"`, label "Edge lines",
  description "Draws black outlines along hard edges of the model for better visual clarity.";
  SegmentedControl `off → 'OFF'`, `on → 'ON'`,
  `value={higherContrastModelEdges ? 'on' : 'off'}`,
  `onChange={next => onHigherContrastModelEdgesChange?.(next === 'on')}`.
- **States**: the whole section is wrapped in `{MODEL_EDGE_OVERLAY_ENABLED && …}` and
  `MODEL_EDGE_OVERLAY_ENABLED = false` (`src/hooks/useStlGeometry.ts:361`), with the
  source comment: `{/* Rendering — Higher Contrast Model Edges (disabled while its build path is reworked) */}`.
  The prop defaults to `false` and the callback is optional.
- **Port target**: keep the setting and the prop, but render nothing while the flag is
  false. The flag itself belongs to the 3D-view area, not this tab.

### Section 3 — Trackpad Navigation (`:275-402`)
- **Icon**: `Hand`. **Heading**: "Trackpad Navigation". **Description**: "Configure two-finger gestures and alternate modifier behavior."
- **Container**: `<div className="mt-3 space-y-1.5">`.
- **Row A**: `bordered`, label "Trackpad navigation mode",
  description "What a two-finger drag does. Pinch-to-zoom still works either way.";
  `SelectDropdown<CameraTrackpadPrimaryAction>` options `off→'Off'`, `pan→'Pan'`,
  `orbit→'Orbit'`; `ariaLabel`/`title` "Trackpad navigation mode", `w-36`, `menuAlign="right"`.
- **Conditional block** (`cameraTrackpadPrimaryAction !== 'off'`, `:324-401`):
  - Row B: `bordered`, label "Alternate gesture modifier",
    description `` `Hold this key to temporarily switch two-finger drag to ${cameraTrackpadPrimaryAction === 'pan' ? 'orbit' : 'pan'}.` ``
    (so it reads "…to orbit." when the primary action is pan, "…to pan." when orbit);
    SegmentedControl `alt→'Option'`, `shift→'Shift'`.
  - Panel (`px-2.5 py-1.5`, surface-0) titled "Trackpad acceleration"
    (`text-xs font-semibold`), body "Tune movement speed for two-finger pan/orbit gestures."
    (`mt-0.5 text-xs`, `--text-muted`), then
    `<div className="mt-2 grid grid-cols-1 gap-2 md:grid-cols-2">` with three
    slider groups (`className="space-y-0.5"`), each: label
    `className="text-xs flex justify-between"` `--text-muted` with left `<span>` and
    right `<span style={{color:'var(--text-strong)'}}>{fmt.quantity(value, 2, 'times')}</span>`
    (renders e.g. `2.00x`), then
    `<input type="range" min="0.4" max="4" step="0.05" … className="ui-range w-full" />`.
    Sliders in order: "Pan acceleration", "Orbit acceleration", "Zoom acceleration".
- **States**: with primary action `off`, only Row A is shown. Acceleration range
  `[0.4, 4]` (clamped in `normalizeCameraTrackpadSettings` too), defaults `2`, `2`, `2`.
  Normalization force-migrates a persisted `orbit + shift` pair to `orbit + alt`
  (comment `:62-63`: "Defensive fallback: a persisted orbit+shift combo has caused
  startup instability on some macOS trackpad setups.").
- **Data**: all props; order of the defaults object is `{primaryAction:'pan', modifierKey:'shift', …}`
  which normalization then rewrites to `alt` on load (see Decisions).
- **Port target**: `gpui/crates/gpui-app/src/features/settings/camera.rs` +
  `prefs/camera_*.rs`.

---

## Surface: `WorkspacesSettingsTab` — Workspace tab

- **Source**: `src/components/settings/WorkspacesSettingsTab.tsx:20-292`.
- **Renders**: `<div className="space-y-3">` containing exactly **one**
  `<section className="rounded-md border p-3">` ("3D View"); everything is inside it.
  Entirely **untranslated** (no Lingui import at all).
- **Section header** (`:34-59`): same icon-tile markup as General but icon `Layers3`;
  heading "3D View"; description "Build volume boundaries and display resolution hints
  used across the workspace."
- **Printer-managed branch** (`:62-67`, when `activePrinterProfile` exists):
  ```jsx
  <div className="rounded-md border px-2 py-1.5"
    style={{ borderColor: 'color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 45%)',
             background: 'color-mix(in srgb, var(--accent-secondary), var(--surface-1) 94%)' }}>
    <div className="text-xs" style={{ color: 'var(--text-muted)' }}>
      Bounding box dimensions are handled by the selected printer profile
      (<span style={{ color: 'var(--text-strong)' }}>{activePrinterProfile?.name}</span>).
    </div>
  </div>
  ```
  In this branch the **dimension fields and the bounds toggle are not rendered**, but
  the four debug/warning rows below are still rendered.
- **Manual branch** (`:69-176`), when no active printer profile:
  - Row: `bordered`, label "Enable build volume bounds",
    description "Shows a faint printer volume outline and enables out-of-bounds checks.";
    OFF/ON bound to `view3dSettings.enabled`.
  - When `enabled` (`:86-175`): `<div className="mt-2 grid grid-cols-2 gap-2">` with:
    - three `label className="text-xs"` `--text-muted` blocks — "Build Width (mm)",
      "Build Depth (mm)", "Max Z Height (mm)" — each containing a `NumberInput`
      with `min={10} step={1}`, `className="mt-1 h-9 w-full rounded-md border pl-2 pr-5 text-[12px]"`,
      inline `borderColor: 'var(--border-subtle)'`, `background: 'var(--surface-1)'`,
      `color: 'var(--text-strong)'`, and
      `onChange={(next) => patchView3dSettings({ <field>: Math.max(10, Math.round(next)) })}`.
    - a summary card `className="rounded-md border px-2 py-1.5"` (`--border-subtle`,
      `--surface-1`) titled "Build Volume" (`text-xs`, `--text-muted`) whose value line
      `className="mt-0.5 text-xs font-semibold"` (`--text-strong`) reads
      `{Math.round(widthMm)} × {Math.round(depthMm)} × {Math.round(maxZMm)} mm`.
    - `SettingRow bordered className="col-span-2"` label "Build volume origin",
      description "Choose where XYZ 0,0,0 is located for the printer volume.";
      options `center→'Center'`, `front_left→'Front-left corner'`.
    - two more labelled `NumberInput`s — "Screen Width (px)" `min={320}`,
      "Screen Height (px)" `min={200}` — with `Math.max(320|200, Math.round(next))`.
- **Always-rendered rows** (both branches):
  - `bordered className="mt-2"` label "Show out-of-bounds warnings",
    description "Warn when any visible model extends beyond the configured build volume.";
    OFF/ON → `showViolationWarning`.
  - label "Show model bounding boxes", description "Debug overlay: draws world-space
    bounds for each visible model (red if out-of-bounds)."; OFF/ON → `showModelBoundingBoxes`.
  - label "Show slice SAT bounding mesh", description "SAT debug overlay for nesting and
    diagnostics."; OFF/ON → `showSliceSatBoundingMesh`.
  - When `showSliceSatBoundingMesh`:
    label "SAT debug scope", description "Show SAT mesh on the active model only, or on
    all visible models."; options `active→'ACTIVE ONLY'`, `all→'ALL MODELS'` bound to
    `showSliceSatBoundingMeshForAllModels`.
  - When `showSliceSatBoundingMesh`:
    label "SAT mode", description "Choose accurate convex-hull SAT for nesting, or
    experimental slice-derived SAT for diagnostics."; options
    `accurate_hull→'Accurate Hull-based SAT'`, `experimental_slice→'Experimental Slice-based SAT'`.
  - When `showSliceSatBoundingMesh && sliceSatBoundingMeshMode === 'experimental_slice'`:
    label "Experimental slice display", description "Pick how the experimental
    slice-derived SAT is visualized."; options `shaded→'Shaded'`, `wireframe→'Wireframe'`.
- **States**: printer-managed (profile exists) vs manual; `enabled` gate on the
  dimension grid; SAT rows nested three deep.
- **Data**: `view3dSettings` prop + `onView3dSettingsChange`; local profile store via
  `React.useSyncExternalStore(subscribeToProfileStore, getProfileStoreSnapshot, getProfileStoreServerSnapshot)`
  and `getActivePrinterProfile(profileState)`. Every edit rebuilds the whole object in
  `patchView3dSettings` (`{ ...view3dSettings, ...patch }`).
- **Defaults** (`view3dPreferences.ts`): `enabled: true`, `widthMm: 218`, `depthMm: 123`,
  `maxZMm: 250`, `originMode: 'center'`, `screenWidthPx: 2560`, `screenHeightPx: 1440`,
  `showViolationWarning: true`, `showModelBoundingBoxes: false`,
  `showSliceSatBoundingMesh: false`, `showSliceSatBoundingMeshForAllModels: false`,
  `sliceSatBoundingMeshMode: 'accurate_hull'`,
  `experimentalSliceSatBoundingMeshRenderMode: 'shaded'`.
  Normalization clamps mm to `[10,2000]`/`[10,2000]`/`[10,3000]`, px to
  `[320,16384]`/`[200,16384]`, and migrates the legacy
  `sliceSatBoundingMeshRenderMode` field (`hull`→accurate_hull; `shaded`/`wireframe`→experimental_slice).
- **Port target**: `gpui/crates/gpui-app/src/features/settings/workspaces.rs` +
  `prefs/view3d.rs`; printer-profile lookup belongs to the profiles area.

---

## Surface: `MultiPlateSettingsTab` — Multi-Plate tab

- **Source**: `src/components/settings/MultiPlateSettingsTab.tsx:17-118`.
- **Renders**: a single `<section className="rounded-lg border p-3">` (note **`rounded-lg`**,
  unlike the `rounded-md` used by the other three tabs) with the standard icon tile,
  heading "Plates" (`_(msg`Plates`)`) and description
  `_(msg`Behaviour across the beds of a scene.`)`, then three rows.
- **Section style**: `background: 'var(--surface-1)'`, `borderColor: 'var(--border-subtle)'`.
  Icon `SquareStack` (`h-4 w-4`, `--accent`) in the same `h-8 w-8 rounded-md border` tile.
- **Row 1**: `bordered className="mt-3"`, label
  `_(msg`Follow the plate a model lands on`)`, description
  `_(msg`A model dropped on another bed makes that bed the one you work on.`)`;
  SegmentedControl options **`ON` first, then `OFF`** (reversed relative to every other
  ON/OFF in this area — it is authored this way):
  `[{value:'on',label:_(msg`ON`)},{value:'off',label:_(msg`OFF`)}]`,
  `value={settings.followLandedPlate ? 'on' : 'off'}`,
  `onChange={(next) => setSettings(saveMultiPlateSettings({ followLandedPlate: next === 'on' }))}`.
- **Row 2**: `bordered className="mt-3"`, label `_(msg`Plate Ordering`)`, description
  `_(msg`Growing grid, or a fixed row length.`)`; options
  `fixed → _(msg({ message: 'Fixed', context: 'Plate ordering mode: a fixed number of beds per row.' }))`,
  `dynamic → _(msg({ message: 'Dynamic', context: 'Plate ordering mode: a grid that grows to stay square.' }))`;
  `value={settings.plateOrdering}`,
  `onChange={(next) => setSettings(saveMultiPlateSettings({ plateOrdering: next === 'fixed' ? 'fixed' : 'dynamic' }))}`.
- **Row 3** (only when `settings.plateOrdering === 'fixed'`, `:91-115`):
  `bordered className="mt-3"`, label `_(msg`Beds per row`)`, description
  `_(msg`1-2-3-4 across, then the next row. Adding a bed never moves the ones already placed.`)`;
  control `NumberInput` `min={1}`, `max={MAX_FIXED_PLATE_COLUMNS}` (12), `step={1}`,
  `value={settings.fixedPlateColumns}`,
  `className="ui-input h-[34px] w-[120px] py-1.5 pl-2.5 pr-5 text-sm"`;
  `onChange`: ignore non-finite, else
  `setSettings(saveMultiPlateSettings({ fixedPlateColumns: next }))`
  (the store's `fixedPlateColumns()` clamps to `[1,12]` and rounds).
- **States**: `fixedPlateColumns` row hidden for `dynamic`. All three controls write
  through immediately (`saveMultiPlateSettings`), bypassing the modal draft — see Decisions.
- **Data**: local state seeded from `getMultiPlateSettingsSnapshot()` and re-read on
  `subscribeToMultiPlateSettings` (listens to `dragonfruit://multi-plate-settings-changed`
  and `storage` for key `dragonfruit-multi-plate:settings-v1`). Store defaults:
  `{ followLandedPlate: true, plateOrdering: 'fixed', fixedPlateColumns: 3 }`;
  `normalizeMultiPlateSettings` treats `followLandedPlate !== false` as `true`
  (so a missing field means ON).
- **Port target**: `gpui/crates/gpui-app/src/features/settings/multi_plate.rs` +
  `prefs/multi_plate.rs`; the grid maths (`plateCascadeCell`, `plateCascadeOffsetMm`,
  `PLATE_CASCADE_GAP_MM = 24`) belongs to the scene/plates area, not here.

---

## Deliberate decisions and gotchas

1. **`MultiPlateSettingsTab` is not a draft.** It reads/writes localStorage directly and
   has no props. Opening the settings modal, toggling a plate option, then pressing
   **Cancel** leaves the change applied. The other three tabs are draft-only. A port must
   either preserve this or deliberately convert it — do not "fix" it silently.
2. **UI scale and support tooltips are also immediate**, despite living in the General
   tab: `saveUiScale` / `setSupportPlacementHelpEnabled` fire on interaction. So the
   General tab has three persistence timings in one card stack (draft for language,
   toolbar, floating persistence, import defaults; immediate for scale and tooltips;
   immediate-and-destructive for "Reset saved window layout").
3. **`applyRestoreDefaultsToDraft` does not reset `draftToolLayout` or `draftLocale`.**
   Restore Defaults resets floating-layout persistence to `true` and import defaults, but
   "Toolbar position" and "Interface language" keep their current draft values
   (`SettingsModal.tsx:891-926`). This looks like an oversight; port it as-is unless the
   product owner says otherwise.
4. **UI scale "Custom" arms without saving.** Comment at `GeneralSettingsTab.tsx:72-73`:
   "Selecting \"Custom\" arms the numeric input but saves nothing — the current scale
   stays until the user commits a value in the field." `customScaleArmed` is local state,
   so it resets every time the tab remounts, and a saved non-preset value (e.g. 0.83)
   re-derives `isCustomScale = true` from `!UI_SCALE_PRESETS.includes(uiScale)`.
5. **`UI_SCALE_PRESETS` are fractions, the UI shows percents.** The dropdown value is
   `String(uiScale)` (e.g. `"0.9"`); labels are `fmt.quantity(preset * 100, 0, 'percent')`
   → `"90%"`; the custom field shows `Math.round(uiScale * 100)` and divides by 100 on
   commit. Clamp is `[0.25, 4]`, not `[0, 1]`.
6. **"Line" raft mode force-enables roots, and locks the row.** `onChange` sets
   `rootsEnabled: true` when switching to `line`, and the "Roots Enabled on Import" row is
   hidden while `raftBottomMode === 'line'` (`rootsLockedByLineRaft`). The store's
   normalizer applies the same rule on read, so the invariant survives a stale file.
7. **"Default Raft Wall" only exists for `solid`.** It is hidden for `off` and `line`;
   `getImportDefaultsRaftPatch` also forces `wallEnabled: false` for any non-solid mode.
8. **Camera tab: options are `Ortho`/`Perspective`, not "Orthographic".** The stored
   values are `'orthographic' | 'perspective'`.
9. **Camera scope normalizer differs from the store default.** Default is `'global'`,
   but `normalizeScope` returns `'global'` only for the exact string, otherwise
   `'workspace'`. Same pattern for `primaryAction` (default `'pan'`, anything but
   `'off'`/`'orbit'` becomes `'pan'`) and `modifierKey` (default `'alt'`, only `'shift'`
   is shift).
10. **Trackpad `orbit + shift` is rewritten to `orbit + alt` on every load.** Comment at
    `cameraTrackpadPreferences.ts:62-63`: "Defensive fallback: a persisted orbit+shift
    combo has caused startup instability on some macOS trackpad setups." The default
    constants are `{ primaryAction: 'pan', modifierKey: 'shift' }`, so the *default* is
    itself a shift pairing (legal because the action is `pan`).
11. **FOV is only rendered for global + perspective.** In workspace scope the FOV slider
    disappears entirely (the per-workspace mode has no FOV); the value is still saved on
    Apply and shared by every perspective workspace.
12. **`printing` has no UI.** `WorkspaceCameraDefaults` is a `Record<SupportMode, …>` with
    five keys, but `workspaceMeta` lists four. The printing default is preserved by
    `handleWorkspaceCameraModeChange` (spread of `prev`) and never edited here.
13. **The Camera tab's `activeWorkspace` is transient.** It resets to `'prepare'` on
    remount, so switching tabs and coming back loses the selected workspace.
14. **Translations are inconsistent by file.** General and Multi-Plate use `msg` for row
    text but leave **section headings and descriptions** ("Language", "UI Scale",
    "Toolbar", "Floating Windows", "Support Tooltips", "Import Defaults") and the
    "Custom" option as raw English. Camera and Workspace tabs are fully raw. Before
    porting, decide whether the port adds msgids for these (adding them is safe — the
    English string becomes the msgid) or mirrors the omission.
15. **The "Higher Contrast Model Edges" section is dead by flag**, not by deletion:
    `MODEL_EDGE_OVERLAY_ENABLED = false` in `src/hooks/useStlGeometry.ts:361`, with the
    author's comment in the tab: "Rendering — Higher Contrast Model Edges (disabled while
    its build path is reworked)". Render nothing; keep the persisted field.
16. **`GeneralSettingsTab` contains unreachable code** (`layoutDump`, `dumpStatus`, three
    dump/copy handlers, and the `Bug`/`ClipboardCopy` imports). Nothing renders them. Do
    not port; do not resurrect the floating-layout debug event from this tab.
17. **Two 36px control heights.** `SelectDropdown` hard-codes `h-[36px]` while
    `.ui-input` (globals) says `padding: 6px 10px` and `.ui-select` says `height: 32px`.
    The trigger is not `.ui-select`; it is a `<button class="ui-input …">` with an
    explicit height. Port the *rendered* 36px trigger, not the 32px `.ui-select`.
18. **`SettingRow` `disabled` only fades.** It never disables the child; the caller must.
    No usage in these four tabs passes it.
19. **`SettingRow` control column is not shrinkable;** the text column is
    `min-w-0`/`truncate`-friendly but the row itself is `justify-between`. Long translated
    descriptions wrap under the label, they do not push the control.
20. **Segmented pills are `min-w-[92px]` at `md` size**, i.e. "OFF"/"ON" render far wider
    than their text. Three-option groups are still 92px each. `fullWidth` (used once, for
    the workspace picker) switches to `flex-1` so the four pills share the panel width.
21. **The Multi-Plate ON/OFF order is inverted** relative to every other toggle in this
    area (`ON` then `OFF`). It is the only control with that ordering.
22. **`ui-range` has no thumb styling** — globals.css only styles the 8px track
    (`appearance: none`, `border-radius: 9999px`, `background: var(--surface-2)`,
    `accent-color: var(--accent)`). The FOV and acceleration sliders therefore inherit
    the engine's default thumb. In GPUI both the track and the thumb must be drawn
    explicitly; do not try to reproduce "whatever the browser did".
23. **The acceleration read-out uses `fmt.quantity(x, 2, 'times')`** → `2.00x`
    (message `{value}x`, no space), while FOV uses a raw template `{perspectiveFov}°`.
    Two different conventions for a number+unit in the same tab.
24. **`SelectDropdown` option labels are `string`, not `ReactNode`** — the UI-scale
    options must be pre-formatted strings, so they are formatted once per render with the
    active locale.
25. **`NumberInput` clamps are applied by the caller, not the component**, for these tabs:
    Workspaces wraps `Math.max(10|320|200, Math.round(next))`; MultiPlate relies on the
    store's `fixedPlateColumns`. The component itself only clamps on stepping
    (`applyStepDelta`), not on typing.
26. **`NumberInput` shows a 2-decimal locale format** (`fmt.field(n, 2)`) but these tabs
    pass step `1`, so a typed `12.34` momentarily commits `12.34` before the caller's
    `Math.round` normalises it. Port the round-trip, not just the rendered value.
27. **`ScrollableNumberField` consumes wheel events with a native non-passive listener**
    and only when focused or Shift is held. In GPUI there is no passive/active split;
    the behaviour to keep is "wheel over the field steps it only when focused or Shift".
28. **`SettingRow` surface default is `sunken` (`--surface-0`)**, and the UI-scale group
    already sits on a `--surface-0` panel; its inner row is *not* bordered precisely to
    avoid a second sunken plate inside a sunken plate (`surface="raised"` exists for the
    inverse case).
29. **The sections are `rounded-md` … except Multi-Plate, which is `rounded-lg`.**
    Quoted class lists differ by exactly that token; do not normalise it away.
30. **`max-w-[72rem]` and `w-72` sidebar** are the modal's hard geometry. The four tabs
    themselves add no width constraints beyond `w-36` controls and `w-[120px]` for the
    beds-per-row field.

## Open questions

1. **There is no `gpui/` tree in this worktree.** All port targets above are proposals.
   Before writing code, confirm the crate/module naming with whoever owns the port
   (`gpui/crates/gpui-app/...` is inferred from `docs/internal/gpui-backend-map.md`,
   which only names `gpui/crates/gpui-backend/`). The referenced
   `docs/internal/gpui-port-handoff.md` does not exist here.
2. **UI scale in GPUI.** The web implementation is a native webview zoom
   (`Webview::setZoom`, per the module comment in `uiScalePreference.ts`). GPUI has no
   webview; the equivalent is a global text/space scale factor. Whether the port keeps
   the same six presets and 25–400% clamp, and how it applies them across every widget,
   is a whole-app decision this tab cannot make alone.
3. **i18n in GPUI.** Language switching requires a catalog mechanism equivalent to
   Lingui plus re-render on locale change. The four tabs are partly translated (see
   decision 14); the port needs a target format for the msgids in this area
   (`Interface language`, `Interface scale`, `Toolbar position`, `Remember window
   positions`, `Reset saved window layout`, `Show support tooltips`, `Default Raft
   Base`, `Default Raft Wall`, `Roots Enabled on Import`, `Auto-Repair`, `Auto-Repair
   Scenes`, `Follow the plate a model lands on`, `Plate Ordering`, `Beds per row`,
   `Plates`, the unit messages `{value}%` and `{value}x`, and the row descriptions).
4. **Clipboard.** Only the dead `handleCopyDump` uses `navigator.clipboard`; nothing live
   in these tabs needs a clipboard API. If that dump UI is ever restored, GPUI needs its
   own clipboard path — out of scope now.
5. **Printer-profile store.** Workspaces reads the *app* profile store
   (`src/features/profiles/profileStore`, which is plugin-populated). The GPUI profiles
   implementation must expose an equivalent "active printer or none" selector, plus the
   profile's display `name`, before the printer-managed branch can be ported.
6. **`--accent-secondary-action-*` tokens** used by the modal footer come from the theme
   system (`themeCustomizations.ts`) and switch on `isLightTheme`. Confirm the GPUI theme
   exposes the derived tokens rather than only the base palette.
7. **`Restore Defaults` scope.** It resets settings from *all* tabs in one press; whether
   the GPUI port keeps that global behaviour or scopes it per tab is unspecified.
8. **`MultiPlate` bypassing Apply/Cancel** (decision 1) may be a bug rather than intent;
   the source has no comment either way.
