# Controls & design-system primitives

Scope: `src/components/atoms/*` and `src/components/ui/*` — the primitives every other
area leans on — plus the `.ui-*` / control-related rules in `src/app/globals.css`.
Every file listed below was read in full.

Existing prose contracts this spec expands on (both present in the repo):
`docs/dev/ui-atoms.md` (the atom inventory and its constraints) and
`docs/dev/context-menu.md` (the menu widget). This file is the exhaustive version: it
adds the exact class maps, the resolved geometry, the cascade result and the per-file
behaviour.

Two things to know before using it:

1. **The GPUI port does not exist in this worktree.** `gpui/` is absent (verified:
   `gpui/*` and `gpui/src/**/*.rs` both return "Path not found"), and
   `docs/internal/gpui-port-handoff.md` is not present either. The only port document
   that exists is `docs/internal/gpui-backend-map.md`, which describes
   `gpui/crates/gpui-backend/` (the Tauri command surface, 131 commands) and says
   nothing about a UI crate. Every "Port target" below therefore names a *proposed*
   module (a sibling of the source file's name) and states that nothing exists there
   yet. Treat the target names as a naming proposal, not as a fact about the repo.
2. **Tailwind utilities lose to `.ui-*` CSS.** See "The cascade" below. It changes what
   actually renders, and several class lists read as if they win when they do not.

---

## Components in scope

| Component | File |
|---|---|
| Button | `src/components/atoms/Button.tsx` |
| IconButton | `src/components/atoms/IconButton.tsx` |
| Card, CardHeader | `src/components/atoms/Card.tsx` |
| Toggle (switch) | `src/components/atoms/Toggle.tsx` |
| SegmentedControl | `src/components/atoms/SegmentedControl.tsx` |
| Input | `src/components/atoms/Input.tsx` |
| Select (native-shaped wrapper) | `src/components/atoms/Select.tsx` |
| SelectDropdown | `src/components/ui/SelectDropdown.tsx` |
| ContextMenu | `src/components/ui/ContextMenu.tsx` |
| EditorContextMenu | `src/components/ui/EditorContextMenu.tsx` |
| Tooltip | `src/components/ui/Tooltip.tsx` |
| MouseTooltip | `src/components/ui/MouseTooltip.tsx` |
| StructuredDialogModal (modal shell) | `src/components/ui/StructuredDialogModal.tsx` |
| BlockingOverlay (modal shell, non-dismissible) | `src/components/atoms/BlockingOverlay.tsx` |
| NumberInput | `src/components/ui/NumberInput.tsx` |
| ScrollableNumberField | `src/components/ui/scrollableNumberField.tsx` |
| MiniStepperField | `src/components/ui/miniStepperField.tsx` |
| CompactNumberField | `src/components/ui/compactNumberField.tsx` |
| wheelStepDirection (helper) | `src/components/ui/wheelStepDirection.ts` |
| ScrollLockedInputs (global behaviour) | `src/components/ui/ScrollLockedInputs.tsx` |
| LanguageSwitcher | `src/components/ui/LanguageSwitcher.tsx` |
| Toast, ToastViewport | `src/components/atoms/Toast.tsx` |
| ProgressBar | `src/components/atoms/ProgressBar.tsx` |
| Spinner | `src/components/atoms/Spinner.tsx` |
| IconChip + ICON_TONE_STYLES (badges) | `src/components/atoms/IconChip.tsx`, `src/components/atoms/iconTone.ts` |
| SettingRow | `src/components/atoms/SettingRow.tsx` |
| ColorSwatchInput | `src/components/atoms/ColorSwatchInput.tsx` |
| FadeText | `src/components/atoms/FadeText.tsx` |
| AlertDiamondIcon | `src/components/atoms/AlertDiamondIcon.tsx` |
| PanelCollapseToggle | `src/components/atoms/PanelCollapseToggle.tsx` |
| cn (class joiner) | `src/components/atoms/cn.ts` |
| Barrel | `src/components/atoms/index.ts` |
| `.ui-*` CSS + control CSS | `src/app/globals.css:1-987` |

The slider is **not a component**: it is `<input type="range" className="ui-range">`
styled entirely by `globals.css` (`.ui-range`, `.slider-vertical`), with ~30 hand-written
call sites. There is no `Slider.tsx`.

Scale of the consolidation, from `docs/dev/ui-atoms.md`: "the same dialog shell appeared
43 times, the same button shape 353 times, and the same 20px icon slot 46 times". The
component comments add: `SettingRow` "was re-declared ~55 times", `SegmentedControl`
"around 50 call sites", tinted buttons "~60 call sites", `ICON_TONE_STYLES` "~35 files
(143 spans)".

---

## Token vocabulary

`src/app/globals.css:1` declares these on `:root` (dark is the default; light is
`:root[data-theme='light']` and also applied by the `prefers-color-scheme: light`
media query when no `data-theme` is set).

| Token | Dark (default) | Light |
|---|---|---|
| `--background` | `#0b0f14` | `#b4b6c2` |
| `--foreground` | `#e6ebf2` | `#191a20` |
| `--surface-0` | `#111216` | `#bcbfd0` |
| `--surface-1` | `#1a1b21` | `#b2b5c4` |
| `--surface-2` | `#23252e` | `#a6a9b8` |
| `--border-subtle` | `#272a33` | `#9498a8` |
| `--border-strong` | `#353944` | `#818596` |
| `--text-strong` | `#f8f8fb` | `#191a20` |
| `--text-muted` | `#c3c7cf` | `#3c4050` |
| `--indicator` | `#c3c7cf` | `#4a4e62` |
| `--accent` | `#ec2a77` | `#ec2a77` |
| `--primary-button-surface` | `#c11f61` | `#c11f61` |
| `--accent-hover` | `#d81d67` | `#d81d67` |
| `--accent-contrast` | `#fff6ff` | `#fff0f7` |
| `--accent-secondary` | `#baf72e` | `#6ab80a` |
| `--secondary-button-surface` | `#bdf435` | `#4e8900` |
| `--accent-secondary-hover` | `#a6df29` | `#5fa309` |
| `--accent-secondary-contrast` | `#182106` | `#f0fff4` |
| `--topbar-accent` | `#ec2a77` | `#ec2a77` |
| `--danger` | `#e45454` | `#c9302c` |
| `--success` | `#2eb67d` | `#2eb67d` (not overridden) |
| `--accent-secondary-action-color` | `var(--accent-secondary)` | `color-mix(in srgb, #4f8a08, var(--text-strong) 30%)` |
| `--accent-secondary-action-border` | `color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 42%)` | `color-mix(in srgb, #6aa20d, var(--border-subtle) 34%)` |
| `--accent-secondary-action-bg-92` | `color-mix(in srgb, var(--accent-secondary), var(--surface-1) 92%)` | `color-mix(in srgb, #6aa20d, var(--surface-1) 80%)` |
| `--accent-secondary-action-bg-90` | `color-mix(in srgb, var(--accent-secondary), var(--surface-1) 90%)` | `color-mix(in srgb, #6aa20d, var(--surface-1) 78%)` |
| `--radius-sm` | `6px` | same |
| `--radius-md` | `10px` | same |
| `--radius-lg` | `14px` | same |
| `--space-1..6` | `4px, 8px, 12px, 16px, 20px, 24px` | same |
| `--font-size-xs/sm/md/lg/xl` | `12px, 13px, 14px, 17px, 22px` | same |
| `--line-height-tight` / `--line-height-normal` | `1.2` / `1.45` | same |
| `--topbar-height` | `52px` | same |

`:root` also sets `user-select: none; -webkit-user-select: none;` globally.

Source comment on density (`globals.css:63-66`):

> `/* The base UI density is fixed and DPI-aware (CSS px are physical-size constant in the webview). A screen-size factor + user UI scale are applied via native webview zoom (see useUiScale) so the UI is slightly more compact on smaller monitors instead of crowded. */`

`body` sets `font-family: var(--font-geist-sans), Arial, Helvetica, sans-serif;
font-size: var(--font-size-md); line-height: var(--line-height-normal);` and the
`--background`/`--foreground` pair.

### The zoom layer (`src/hooks/useUiScale.ts`)

`useUiScale()` (called once at `src/app/page.tsx:805`) applies
`effectiveZoom = userScale × screenSizeFactor` through
`getCurrentWebview().setZoom(effectiveZoom)` (Tauri only; a no-op in a browser). The
screen factor is:

| `window.screen.availWidth` | factor |
|---|---|
| `>= 2000` | `1` ("1440p & larger → reference size") |
| `>= 1600` | `0.9` ("1080p → 90%") |
| `>= 1200` | `0.82` ("small laptops") |
| else | `0.72` ("tiny") |

A user scale of 100% means "adapt automatically"; anything else multiplies on top.
Comment: "Native page zoom scales the whole interface uniformly and keeps all
`getBoundingClientRect()` / pointer math consistent, so the 3D canvas, gizmos, and
picking stay correct." This is why every geometry below is in CSS px with no rem
scaling.

---

## The cascade (read before porting any class list)

`.ui-*` rules are ordinary unlayered declarations. Tailwind utilities come from
`@import "tailwindcss"` (layered, emitted before these rules), and a plain utility has
the same specificity (0,1,0) as `.ui-button` / `.ui-input`. So for any property those
rules declare, a non-`!` utility never wins. `docs/dev/ui-atoms.md` states it as:

> "**Size means geometry, and geometry only wins when it is important.** The `.ui-button` rule sets padding and font size, and a plain utility class ties with it on specificity, so `.ui-button` wins: a size's `px-*` and `text-*` are decorative, and the `!h-*` height is what actually sizes a button. The same is true of the panel rule (`.ui-panel .ui-button`), which is a two-class selector and beats any non-important utility. If a call site really needs its own padding, it must write `!p-*`/`!px-*`…"

`LanguageSwitcher.tsx` names the same failure for inputs:

> `// .ui-input (unlayered CSS) overrides the trigger's Tailwind text-sm/pr-10, // making the font too large and the chevron overlap the text. The ! // important utilities beat the unlayered rule.`

Consequences the porter must reproduce (these are what the app actually renders):

* every `ui-button` renders at `font-size: 14px` (`.ui-button`) regardless of
  `text-xs` / `text-[11px]` / `text-sm` in its size class; only the `!h-*` height and
  the `!`-prefixed utilities in the size map take effect;
* every `ui-button` keeps `padding: 5px 11px 7px 11px`; `px-3`, `px-2.5`, `px-2`,
  `px-4` in the size maps are dead unless `!`-prefixed (they are not);
* every `ui-input` keeps `padding: 6px 10px`, `font-size: 13px`, `line-height: 1.3`,
  `border-radius: 6px`, `border: 1px solid var(--border-subtle)`,
  `background: var(--surface-0)`, `color: var(--text-strong)`; call sites write
  `!bg-[var(--surface-0)]`, `!w-4 !h-4`, `!text-[13px]`, `!pr-9` when they mean it;
* `h-8`, `h-9`, `h-10`, `w-full`, `text-center`, `tabular-nums` still apply because
  `.ui-*` does not declare those properties;
* inside `.ui-panel`, `.ui-panel .ui-input` (0,2,0) forces `font-size: 14px` and
  `padding: 6px 10px` on any input/select/button inside a `Card`.

If the Rust port wants a design system rather than a cascade bug, resolve each
component to the *rendered* value listed in its section, and keep the class maps only as
the source of the height/width numbers.

---

## The `.ui-*` CSS layer (verbatim)

All from `src/app/globals.css`. Rules marked **other area** are listed for completeness
but belong to the topbar/onboarding specs.

### `.ui-topbar` — other area (`:157-267`)

```css
.ui-topbar {
  position: relative;
  isolation: isolate;
  height: var(--topbar-height);
  background: transparent;
  padding: 3px 4px 9px 4px;
  gap: var(--space-3);
}
```

`.ui-topbar::before` (the frosted layer) carries:
`position: absolute; inset: 0; z-index: -1; pointer-events: none;` a
`background` of two stacked gradients —
`linear-gradient(90deg, color-mix(in srgb, var(--topbar-accent), transparent 93%) 0%, color-mix(in srgb, var(--topbar-accent), transparent 97%) 45%, color-mix(in srgb, var(--topbar-accent), transparent 94%) 100%)`
over
`linear-gradient(to bottom, color-mix(in srgb, var(--surface-0), transparent 6%) 0%, color-mix(in srgb, var(--surface-0), transparent 10%) 50%, color-mix(in srgb, var(--surface-0), transparent 24%) 100%)`
— plus `backdrop-filter: blur(24px)`, `-webkit-backdrop-filter: blur(24px)` and a
`mask-image`/`-webkit-mask-image` of
`linear-gradient(to bottom, rgba(0,0,0,1) 0%, rgba(0,0,0,1) 86%, rgba(0,0,0,0.52) 92%, rgba(0,0,0,0.12) 97%, transparent 100%)`.
Source comment:

> `/* Pseudo-element carries the frosted glass surface + bottom-edge fade-off. A mask on a real element clips its children (including modals); putting it on ::before keeps the visual effect without breaking any child rendering. */`

`.ui-topbar-blur` (other area): `position: fixed; top: 0; left: 0; right: 0;
height: var(--topbar-height); z-index: 30; pointer-events: none;` +
`backdrop-filter: blur(24px)` + mask
`linear-gradient(to bottom, rgba(0,0,0,1) 0%, rgba(0,0,0,1) 60%, rgba(0,0,0,0.82) 75%, rgba(0,0,0,0.44) 88%, rgba(0,0,0,0.14) 96%, transparent 100%)`.
Comment: "Backdrop blur layer behind the topbar (z-30) so the Transform Toolbar (z-40) and modals (z-50+) float crisp above the frosted glass."

`.ui-topbar::before, .ui-topbar-blur { transition: opacity 150ms ease; }`
`.ui-topbar-transparent::before` / `.ui-topbar-blur-transparent`:
`opacity: 0; backdrop-filter: none; -webkit-backdrop-filter: none;`
`.ui-topbar-mode-idle { border-color: color-mix(in srgb, var(--topbar-accent), var(--border-subtle) 62%); }`
`.ui-topbar-mode-active { box-shadow: 0 0 0 1px color-mix(in srgb, var(--topbar-accent), white 40%) inset; }`
`.ui-topbar-monitor-active { color: #22c55e; animation: uiTopbarMonitorActivePulse 3.8s ease-in-out infinite; }`
`.ui-topbar-monitor-paused { color: #facc15; animation: uiTopbarMonitorPausedBlink 1s ease-in-out infinite; }`
with `@media (prefers-reduced-motion: reduce) { … animation: none !important; }`.
Keyframes `uiTopbarMonitorActivePulse` (0/100%: `color:#22c55e; opacity:.84`, 50%:
`color:#85c59d; opacity:1`) and `uiTopbarMonitorPausedBlink` (0/100%: `color:#facc15;
opacity:.92; filter:drop-shadow(0 0 4px rgba(250,204,21,.34))`, 50%: `color:#fb923c;
opacity:1; filter:drop-shadow(0 0 12px rgba(251,146,60,.82))`).

### `.ui-divider`

```css
.ui-divider {
  width: 1px;
  height: 32px;
  background: var(--border-strong);
}
```

### `.ui-input`, `.ui-select`

```css
.ui-input,
.ui-select {
  border: 1px solid var(--border-subtle);
  background: var(--surface-0);
  color: var(--text-strong);
  border-radius: var(--radius-sm);
  padding: 6px 10px;
  font-size: var(--font-size-sm);
  line-height: 1.3;
}

.ui-select {
  height: 32px;
  min-height: 32px;
  padding-top: 0;
  padding-bottom: 0;
  padding-left: 10px;
  padding-right: 28px;
  line-height: 1;
  vertical-align: middle;
}

.ui-input:focus,
.ui-select:focus {
  outline: none;
  border-color: var(--accent);
}
```

### `.ui-button` and its variants

```css
.ui-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid transparent;
  border-radius: var(--radius-sm);
  padding: 5px 11px 7px 11px;
  font-size: var(--font-size-md);
  font-weight: 600;
  line-height: 1;
  transition: background-color 140ms ease, border-color 140ms ease, color 140ms ease;
  cursor: pointer;
}

.ui-button-danger   { background: #dc2626; color: #fff4f4; border-color: color-mix(in srgb, #dc2626, white 15%); }
.ui-button-danger:hover   { background: #b91c1c; }
.ui-button-primary  { background: var(--primary-button-surface); color: var(--accent-contrast); border-color: color-mix(in srgb, var(--primary-button-surface), white 14%); }
.ui-button-primary:hover  { background: color-mix(in srgb, var(--primary-button-surface), black 12%); }
.ui-button-secondary{ background: var(--surface-1); color: var(--text-strong); border-color: var(--border-subtle); }
.ui-button-secondary:hover{ background: var(--surface-2); }
.ui-button-accent   { background: var(--secondary-button-surface); color: var(--accent-secondary-contrast); border-color: color-mix(in srgb, var(--secondary-button-surface), white 14%); }
.ui-button-accent:hover   { background: color-mix(in srgb, var(--secondary-button-surface), black 9%); }
```

Tinted family:

```css
/* Tinted action: the tone mixed into the surface rather than filled solid.
   These three lines replace the inline style triples that ~60 call sites used to
   hand-write (border 45%, surface 86%, text = tone). */
.ui-button-tint {
  background: color-mix(in srgb, var(--button-tint), var(--surface-1) 86%);
  border-color: color-mix(in srgb, var(--button-tint), var(--border-subtle) 45%);
  color: var(--button-tint);
}
.ui-button-tint:hover { background: color-mix(in srgb, var(--button-tint), var(--surface-1) 76%); }
.ui-button-tint-accent  { --button-tint: var(--accent); }
.ui-button-tint-danger  { --button-tint: var(--danger); }
.ui-button-tint-warning { --button-tint: #f59e0b; }
.ui-button-tint-success { --button-tint: var(--success); }
```

### `.ui-segmented-option`

```css
/* Segmented option pill. `--segmented-tone` is set by the control, so one class
   covers the accent and accent-secondary families that were hand-written ~50
   times with a four-line inline active style block. */
.ui-segmented-option {
  background: var(--surface-1);
  border-color: var(--border-subtle);
  color: var(--text-muted);
}
.ui-segmented-option:hover:not(:disabled) { background: var(--surface-2); }
.ui-segmented-option[data-active="true"] {
  border-color: color-mix(in srgb, var(--segmented-tone, var(--accent)), white 10%);
  background: color-mix(in srgb, var(--segmented-tone, var(--accent)), var(--surface-0) 76%);
  color: color-mix(in srgb, var(--segmented-tone, var(--accent)), var(--text-strong) 25%);
}
```

### `.ui-shell`, `.ui-panel`

```css
.ui-shell { background: var(--background); color: var(--foreground); }

.ui-panel {
  background: var(--surface-0);
  border: 1px solid var(--border-subtle);
  font-size: var(--font-size-sm);
  line-height: var(--line-height-normal);
}
.ui-panel h3 { font-size: var(--font-size-md) !important; line-height: var(--line-height-tight); }
.ui-panel .ui-input, .ui-panel .ui-select, .ui-panel .ui-button { font-size: var(--font-size-md); }
.ui-panel .ui-input, .ui-panel .ui-select { padding: 6px 10px; }
.ui-panel .ui-select { height: 32px; min-height: 32px; padding-top: 0; padding-bottom: 0; padding-left: 10px; padding-right: 28px; line-height: 1; }
.ui-panel .ui-button { padding: 5px 11px 7px 11px; }
```

### Cursor + disabled (global, applies to every `<button>`)

```css
button:not(:disabled),
[role="button"]:not([aria-disabled="true"]),
.ui-button:not(:disabled) { cursor: pointer; }

button:disabled,
.ui-button:disabled {
  cursor: not-allowed;
  background: color-mix(in srgb, var(--surface-1), black 8%);
  border-color: color-mix(in srgb, var(--border-subtle), black 10%);
  color: color-mix(in srgb, var(--text-muted), var(--surface-2) 18%);
  box-shadow: none;
  opacity: 1;
}
```

### `.ui-label`, `.ui-meta`, `.ui-checkbox`, `.ui-range`

```css
.ui-label { color: var(--indicator); font-size: var(--font-size-sm); }
.ui-meta  { color: var(--indicator); font-size: var(--font-size-sm); }

.ui-checkbox {
  width: 15px; height: 15px; border-radius: 4px;
  border: 1px solid var(--border-subtle);
  background: var(--surface-1);
  appearance: none; -webkit-appearance: none;
  cursor: pointer;
  display: inline-grid; place-items: center;
  transition: background-color 140ms ease, border-color 140ms ease, box-shadow 140ms ease;
}
.ui-checkbox:hover {
  border-color: color-mix(in srgb, var(--accent), var(--border-subtle) 55%);
  background: color-mix(in srgb, var(--surface-2), transparent 6%);
}
.ui-checkbox:checked {
  border-color: color-mix(in srgb, var(--accent), white 12%);
  background: color-mix(in srgb, var(--accent), var(--surface-0) 18%);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent), transparent 82%);
}
.ui-checkbox:checked::after {
  content: ''; width: 8px; height: 5px;
  border: 2px solid var(--accent-contrast);
  border-top: 0; border-right: 0;
  transform: translateY(-1px) rotate(-45deg);
}
.ui-checkbox:disabled { opacity: 0.5; cursor: not-allowed; }

.ui-range {
  width: 100%; height: 8px;
  appearance: none;
  cursor: pointer;
  border-radius: 9999px;
  background: var(--surface-2);
  accent-color: var(--accent);
}
.ui-range:disabled, .ui-checkbox:disabled { opacity: 0.5; cursor: not-allowed; }
```

### `.ui-empty-state`

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
.ui-empty-title {
  color: var(--text-strong);
  font-size: clamp(20px, 3vw, 28px);
  line-height: var(--line-height-tight);
  font-weight: 600;
  max-width: 460px;
  margin-left: auto; margin-right: auto; margin-bottom: var(--space-3);
}
.ui-empty-text {
  color: var(--text-muted);
  font-size: var(--font-size-md);
  margin-bottom: var(--space-5);
}
```

Used only by `src/components/layout/EmptySceneState.tsx` (other area).

### Loading bar

```css
@keyframes uiLoadingLoop { 0% { transform: translateX(-130%); } 100% { transform: translateX(330%); } }
.ui-loading-track { position: relative; overflow: hidden; }
.ui-loading-indicator {
  position: absolute; top: 0; bottom: 0; left: 0;
  width: 32%;
  border-radius: 9999px;
  animation: uiLoadingLoop 1.15s linear infinite;
}
```

### Modal entry

```css
@keyframes uiModalBackdropIn { from { opacity: 0; } to { opacity: 1; } }
@keyframes uiModalPanelIn { from { opacity: 0; transform: translateY(8px) scale(0.985); } to { opacity: 1; transform: translateY(0) scale(1); } }
.ui-modal-backdrop-enter { animation: uiModalBackdropIn 140ms ease-out; }
.ui-modal-panel-enter { animation: uiModalPanelIn 180ms ease-out; transform-origin: center; }
```

### Onboarding (`:788-877`) — other area, listed for completeness

`.ui-onboarding-backdrop` (180ms ease-out), `.ui-onboarding-panel` (240ms ease-out),
`.ui-onboarding-step-forward` / `-backward` (280ms `cubic-bezier(0.22, 1, 0.36, 1)`,
±32px), `.ui-onboarding-fade` (140ms), `.ui-onboarding-success-circle` (460ms
`cubic-bezier(0.34, 1.56, 0.64, 1)`), `.ui-onboarding-success-title/subtitle/action`
(420ms ease-out at 140/240/340ms, `both`), `.ui-onboarding-reveal` (750ms forwards,
radial mask driven by the registered `@property --onboarding-reveal` 0%→135%). All are
disabled under `prefers-reduced-motion: reduce`.

### Adjacent global rules that primitives depend on

```css
/* Custom vertical slider styling */
.slider-vertical { -webkit-appearance: none; appearance: none; background: transparent; outline: none; }
.slider-vertical::-webkit-slider-runnable-track { background: #ef4444; height: 8px; border-radius: 9999px; }
.slider-vertical::-webkit-slider-thumb {
  -webkit-appearance: none; appearance: none;
  width: 18px; height: 18px; border-radius: 9999px;
  background: #e5e7eb; border: 2px solid #ffffff;
  box-shadow: 0 0 0 2px rgba(0,0,0,0.15);
  margin-top: calc((8px - 18px) / 2);
}
/* Firefox + Edge/IE blocks mirror the same colours/sizes (track 8px, thumb 18px). */

/* Hide number input spinners */
input[type="number"].no-spinners::-webkit-outer-spin-button,
input[type="number"].no-spinners::-webkit-inner-spin-button { -webkit-appearance: none; margin: 0; }
input[type="number"].no-spinners { appearance: textfield; -moz-appearance: textfield; }

/* Themed custom scrollbar */
.custom-scrollbar { scrollbar-width: thin; scrollbar-color: color-mix(in srgb, var(--accent), var(--surface-2) 58%) color-mix(in srgb, var(--surface-2), black 18%); }
.custom-scrollbar::-webkit-scrollbar { width: 10px; height: 10px; }
.custom-scrollbar::-webkit-scrollbar-track { background: color-mix(in srgb, var(--surface-2), black 18%); border-radius: 999px; }
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: linear-gradient(180deg, color-mix(in srgb, var(--accent), white 8%), color-mix(in srgb, var(--accent), var(--surface-2) 42%));
  border-radius: 999px;
  border: 2px solid color-mix(in srgb, var(--surface-2), black 18%);
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: linear-gradient(180deg, color-mix(in srgb, var(--accent), white 16%), color-mix(in srgb, var(--accent), var(--surface-2) 34%));
}

/* Themed number stepper controls (global) */
input[type="number"]:not(.no-spinners), .ui-number-input { accent-color: var(--accent); color-scheme: dark; }
input[type="number"]:not(.no-spinners)::-webkit-inner-spin-button, … , .ui-number-input::-webkit-outer-spin-button {
  opacity: 1; width: 14px; margin: 0;
  background: color-mix(in srgb, var(--surface-2), var(--accent) 10%);
  border-left: 1px solid color-mix(in srgb, var(--border-subtle), var(--accent) 24%);
  border-radius: 4px; cursor: pointer;
}
/* hover: background: color-mix(in srgb, var(--surface-2), var(--accent) 16%) */

@keyframes spinOnceCCW { 0% { transform: rotate(0deg); } 100% { transform: rotate(-360deg); } }
.animate-spin-once { animation: spinOnceCCW 500ms ease-in-out; }

/* Legacy keyframes with no `.ui-` consumer */
@keyframes fadeIn { from { opacity: 0; transform: translate(-50%, -20px); } to { opacity: 1; transform: translate(-50%, 0); } }
@keyframes settingsTabIn { from { opacity: 0; transform: translateY(6px); } to { opacity: 1; transform: translateY(0); } }
```

`body.preset-drag-delete, body.preset-drag-delete *` sets a trash-can SVG cursor with
`8 4` hotspot and `not-allowed` fallback `!important` (Support Studio drag; other area).

### Dormant / dead selectors

Verified by grep across `src/`:

| Selector | Status |
|---|---|
| `.slider-vertical` (+ its pseudo-elements) | **No call site anywhere in `src/`.** Only the CSS exists. |
| `.ui-number-input` | **No call site.** Only reachable through the global spinner rules at `globals.css:962-986`. |
| `.ui-button-ghost` | **No CSS rule at all.** Used by `src/components/settings/PluginStudioModal.tsx:1200,1208,2137,2990` as `"ui-button ui-button-ghost !h-7 !px-2.5 …"`; it inherits only `.ui-button` (transparent border, no background), so those are plain buttons with inline accent/danger styles for the active state. `docs/dev/ui-atoms.md` still describes "a bordered capsule around `ui-button-ghost` buttons" as one of the app's segmented looks — that capsule is produced by the call sites' own inline styles, not by the class. |
| `@keyframes fadeIn`, `@keyframes settingsTabIn` | No `.ui-` consumer. |
| `.ui-onboarding-*`, `.ui-topbar-*`, `.ui-empty-*` | Real, but other areas. |

---

## Surface: Button — the action pill

- **Source**: `src/components/atoms/Button.tsx:1-62`
- **Renders**: a single `<button type="button">` with `className` = `ui-button` +
  variant class + size class + caller `className`.
- **Layout & styling**:
  - Base (from `.ui-button`): `display: inline-flex; align-items: center;
    justify-content: center; border: 1px solid transparent;
    border-radius: var(--radius-sm); padding: 5px 11px 7px 11px;
    font-size: var(--font-size-md); font-weight: 600; line-height: 1;
    transition: background-color 140ms ease, border-color 140ms ease, color 140ms ease;
    cursor: pointer;`
  - Variant map (exact):
    ```
    primary          -> 'ui-button-primary'
    secondary        -> 'ui-button-secondary'      // default
    accent           -> 'ui-button-accent'
    danger           -> 'ui-button-danger'
    'tinted-accent'  -> 'ui-button-tint ui-button-tint-accent'
    'tinted-danger'  -> 'ui-button-tint ui-button-tint-danger'
    'tinted-warning' -> 'ui-button-tint ui-button-tint-warning'
    'tinted-success' -> 'ui-button-tint ui-button-tint-success'
    ```
  - Size map (exact) and resolved geometry:
    | size | classes | height | render font-size | render h-padding |
    |---|---|---|---|---|
    | `xs` | `'!h-7 px-2 text-[11px]'` | 28px | 14px (CSS wins) | 11px |
    | `sm` | `'!h-8 px-2.5 text-[11px]'` | 32px | 14px | 11px |
    | `md` (default) | `'!h-9 px-3 text-xs'` | 36px | 14px | 11px |
    | `lg` | `'h-10 px-4 text-sm'` | 40px | 14px | 11px |
    | `auto` | `''` | from caller | 14px | 11px |
  - Source comment on `md`:
    > `/** `md` is the app's default action shape (the one hand-written as `!h-9 px-3 text-xs`). `auto` emits no geometry at all, for a call site that still carries its own height/padding classes; the variant is the part that gets unified today. */`
- **States**:
  - default: variant fill (table above);
  - hover: `.ui-button-*:hover` per variant (`secondary` → `--surface-2`,
    `primary` → primary mix black 12%, `accent` → secondary mix black 9%,
    `danger` → `#b91c1c`, tint → tint mixed into surface-1 at 76%);
  - active/pressed: no rule (browser default suppressed by the background being set);
  - focus: no rule (browser default outline);
  - disabled: the global `button:disabled` rule wins —
    `background: color-mix(in srgb, var(--surface-1), black 8%); border-color:
    color-mix(in srgb, var(--border-subtle), black 10%); color:
    color-mix(in srgb, var(--text-muted), var(--surface-2) 18%); box-shadow: none;
    opacity: 1; cursor: not-allowed;` — i.e. every variant collapses to the same grey;
  - loading/empty/error: none at this level.
- **Data**: `variant` (default `'secondary'`), `size` (default `'md'`), `type`
  (default `'button'`), everything else spread onto the `<button>`.
- **Callbacks**: none of its own; `onClick` etc. pass through `...props`.
- **Text**: no strings; children.
- **Port target**: proposed `ui/button.rs` (a `Button` view + `ButtonVariant` /
  `ButtonSize` enums). Nothing exists: no `gpui/` tree in this worktree.

## Deliberate decisions and gotchas (Button)

1. `md` is the default, not `secondary`'s biggest neighbour — a bare `<Button>` is a
   grey 36px pill at 14px type.
2. The `!` on `!h-7/!h-8/!h-9` matters: `.ui-button` declares padding and font-size but
   not height, so heights need `!` only to stay ahead of any caller utility; `lg`'s
   plain `h-10` works. Do not "normalise" the `!` away without checking call sites.
3. `px-*` in every size class is inert (see "The cascade"). Horizontal padding is
   always 11px; the real widths of `xs…md` differ only by the glyph inside until a
   caller adds `!px-*`.
4. `font-size` is always 14px; `text-[11px]`/`text-xs`/`text-sm` in the size map never
   render. Any port that honours them will be visibly smaller than the app.
5. `type` defaults to `"button"`, deliberately preventing accidental form submits; a
   ported `Button` must default the same way.
6. `docs/dev/ui-atoms.md`: "**A size must not be repurposed.** `Button`'s `sm`/`md` used
   to mean `px-3 py-2 text-sm` / `px-3.5 py-2.5 text-sm`; redefining them to the app's
   action scale silently resized the 21 call sites that had already used them, so those
   were moved to `size="auto"`… When a size's value changes, grep `<Button` for the old
   size first."
7. `cn()` is a plain `filter(Boolean).join(' ')` — no de-duplication, no Tailwind merge.
   Later classes do not reliably win: CSS order does.

---

## Surface: IconButton — square / ghost icon action

- **Source**: `src/components/atoms/IconButton.tsx:1-107`
- **Renders**: one of three `<button>` shapes depending on `variant`:
  1. `ghost` — bare `<button className="inline-flex items-center justify-center rounded
     transition-colors hover:bg-white/10">` + size padding class;
  2. `surface` — `<button className="inline-flex shrink-0 items-center justify-center
     rounded-md border transition-colors">` + box size class, inline style from
     `ICON_TONE_STYLES`;
  3. `solid` (default) — `<button className="ui-button">` + padding class +
     `ui-button-primary` (active) or `ui-button-secondary`. **This is the default.**
- **Layout & styling**:
  | variant | size | class | resolved box |
  |---|---|---|---|
  | solid | `xs` | `'!p-1'` | padding 4px, content-sized |
  | solid | `sm` | `'h-8 w-8 !p-0'` | 32×32 |
  | solid | `md` (default) | `'!p-2'` | padding 8px, content-sized |
  | solid | `lg` | `'h-10 w-10 !p-0'` | 40×40 |
  | solid | `auto` | `''` | caller |
  | surface | `xs` | `'h-6 w-6'` | 24×24 |
  | surface | `sm` | `'h-8 w-8'` | 32×32 |
  | surface | `md` | `'h-9 w-9'` | 36×36 |
  | surface | `lg` | `'h-10 w-10'` | 40×40 |
  | surface | `auto` | `''` | caller |
  | ghost | `xs` | `'!p-0.5'` | padding 2px |
  | ghost | `sm` | `'!p-1'` | padding 4px |
  | ghost | `md` | `'!p-1.5'` | padding 6px |
  | ghost | `lg` | `'!p-2'` | padding 8px |
  | ghost | `auto` | `''` | caller |
  - `surface` inline style: `active ? { ...ICON_TONE_STYLES.accent, color: 'var(--accent-secondary)' } : ICON_TONE_STYLES[tone]`, then `...style`.
    `ICON_TONE_STYLES` (see IconChip section) supplies `borderColor`, `background`,
    `color`; `tone` defaults to `'neutral'`.
  - `ghost` inline style: `{ color: 'var(--text-muted)', ...style }`.
  - `solid` has no inline style unless the caller passes one.
  - Source comments:
    > `/** Padding is owned by the size so a caller's `!p-*` never has to fight it. */`
    > `` `solid` is the `ui-button` tile (the default, and what the app's small square buttons were); `surface` the bordered surface tile a modal or panel uses; `ghost` a bare hit target that shows a surface on hover. ``
- **States**:
  - `solid`: `ui-button-secondary` normally, `ui-button-primary` when `active` —
    hover comes from `.ui-button-secondary:hover`; disabled from `button:disabled`
    (grey, `opacity: 1`, `cursor: not-allowed`);
  - `surface`: no hover rule at all (border/background/color stay; only the browser
    cursor from `button:not(:disabled)`); `active` recolours to the accent tone with
    `color: var(--accent-secondary)`; disabled: the global rule sets
    `background`/`color`, but the **inline** tone style wins for `borderColor`,
    `background`, `color`, so a disabled surface button keeps its tone and only loses
    the cursor (opacity stays 1 because the global rule forces it);
  - `ghost`: hover `hover:bg-white/10` (a white 10% wash — see gotchas); disabled: the
    global `background` rule applies (ghost has no inline background), so a disabled
    ghost turns grey (`--surface-1` −8% black).
- **Data**: `active` (default `false`), `variant` (default `'solid'`), `size`
  (default `'md'`), `tone` (default `'neutral'`, **only read by `surface`**),
  `type` (default `'button'`).
- **Callbacks**: pass-through `onClick` etc.
- **Text**: none; icon children. Accessible name must be supplied by the caller
  (`aria-label`/`title`).
- **Port target**: proposed `ui/icon_button.rs`. Nothing exists.

### Deliberate decisions and gotchas (IconButton)

1. The default is `solid`, i.e. the `ui-button` look — not a bordered tile. A porter
   who makes `surface` the default changes 100+ buttons.
2. `tone` is silently ignored by `solid` and `ghost`; only `surface` reads it.
3. `!p-*` on solid/ghost is `!` because `.ui-button`'s `padding` (solid) or the
   caller's own utilities would otherwise win; `sm`/`lg` replace padding with an
   explicit square box instead (`!p-0`).
4. `surface` disabled does **not** dim (inline style beats the global disabled rule;
   the rule's `opacity: 1` explicitly defeats the UA's). Several call sites rely on
   that staying flat.
5. `ghost`'s `hover:bg-white/10` is theme-blind: it is the only variant that uses a raw
   white wash, so in the light theme the hover reads as a light haze.

---

## Surface: Card / CardHeader — panel shell and drag handle

- **Source**: `src/components/atoms/Card.tsx:1-44`
- **Renders**: `Card` → `<div className="ui-panel rounded-lg shadow-lg border">` with
  inline `borderColor: 'var(--border-subtle)'` + caller style. `CardHeader` → a
  `<div data-panel-drag-handle="true">` containing a left cluster and an optional
  absolutely-positioned right cluster.
- **Layout & styling**:
  - Card base: `ui-panel` (`background: var(--surface-0); border: 1px solid
    var(--border-subtle); font-size: 13px; line-height: 1.45`) + `rounded-lg` (8px —
    the Tailwind utility, **not** `var(--radius-lg)`, which is 14px) + `shadow-lg` +
    `border` (1px; `.ui-panel` already sets the same) + inline
    `borderColor: var(--border-subtle)`.
  - CardHeader: `'relative px-2.5 py-2.5 flex items-center'` (10px padding, flex).
    Left cluster: `'flex items-center gap-2.5 pr-8'` (10px gap, 32px right padding so
    the right cluster never overlaps). Right cluster: `'absolute right-2.5
    top-1/2 -translate-y-1/2 flex items-center'`.
- **States**: none (static shells).
- **Data**: `children`, `className`, `style`; CardHeader: `left`, `right?`,
  `className`.
- **Callbacks**: none.
- **Text**: none.
- **Port target**: proposed `ui/card.rs`. Nothing exists. The header's
  `data-panel-drag-handle="true"` is consumed by
  `src/components/layout/FloatingPanelStack.tsx:220`
  (`target.closest('[data-panel-drag-handle="true"]')`) — the floating-panel area
  depends on this exact attribute being on the draggable header; keep the concept (a
  `DragHandle` marker) even if the DOM attribute goes.

### Deliberate decisions and gotchas (Card)

1. `ui-panel` is what makes a Card a *panel*: it forces 13px body type and, inside it,
   14px `h3`s and 14px inputs/buttons. Dropping the class silently changes every
   control inside the card.
2. The header is the drag handle. `relative` + `pr-8` exist so the absolute right
   cluster cannot cover the title.
3. `rounded-lg` here is the Tailwind utility (8px), not `var(--radius-lg)` (14px).
   `.ui-empty-state` uses the token; Card does not.
4. Three different drag opt-outs exist and are not interchangeable
   (`docs/dev/ui-atoms.md`, plus the comment at `src/components/layout/TopBar.tsx:379`):
   `[data-panel-drag-handle="true"]` starts a panel drag;
   `[data-no-drag]` / `[data-no-drag="true"]` opts out of the floating-panel stack
   (`FloatingPanelStack.isDragBlockedByTarget`);
   `[data-no-window-drag="true"]` opts out of the desktop window drag. The shell
   `EditorLayout` puts the window one on its root. Comment:
   > `// The desktop window drag keeps its own opt-out. `data-no-drag` belongs to // the floating panel stack (see FloatingPanelStack.isDragBlockedByTarget); // the two are not interchangeable.`

---

## Surface: Toggle — the switch

- **Source**: `src/components/atoms/Toggle.tsx:1-82`
- **Renders**: `<button role="switch" aria-checked={checked} aria-label={label}>`
  containing one `<span>` knob.
- **Layout & styling**:
  - track: `'flex shrink-0 items-center rounded-full transition-colors'` +
    `sm: 'h-5 w-9 px-0.5'` (20×36, 2px inset) / `md: 'h-6 w-10 px-0.5'` (24×40);
  - track inline: `background: checked ? 'var(--accent)' : 'var(--surface-2)'`;
  - knob: `'rounded-full bg-white shadow transition-transform'` + `sm: 'h-4 w-4'`
    (16×16) / `md: 'h-5 w-5'` (20×20), plus `checked ? 'translate-x-4' : 'translate-x-0'`;
  - travel is 16px in both sizes: `36 − 2·2 − 16 = 16` (sm), `40 − 2·2 − 20 = 16` (md).
  - Source comment:
    > `/** The pill switch. Two geometries were hand-rolled before this (`w-9 h-5` and `w-10 h-6`) and only one of the copies was reachable as a switch, so the `role` and `aria-checked` live here rather than at each call site. */`
- **States**: on = `--accent` track, knob translated 16px; off = `--surface-2` track,
  knob at 0; hover = none; focus = browser default (no `outline` override); disabled =
  global `button:disabled` (inline background wins, so the track keeps its state
  colour; cursor becomes `not-allowed`, opacity stays 1).
- **Data**: `checked` (required), `onChange(checked: boolean)` (required),
  `size` (default `'md'`), `label?` (aria-label), `type` (default `'button'`),
  rest spread onto the button. `children` and `onChange` are omitted from the HTML
  button props type.
- **Callbacks**: `onClick={() => onChange(!checked)}`.
- **Text**: none; `label` is an accessible name only.
- **Port target**: proposed `ui/toggle.rs`. Nothing exists.

### Deliberate decisions and gotchas (Toggle)

1. It's a `<button role="switch">`, not `<input type=checkbox>`: no form value, no
   `:checked` CSS, and Space/Enter activate it as a button.
2. The state lives *only* on the inline background + knob transform; there is no
   `[data-checked]` selector to port.
3. Both sizes translate exactly 16px; `sm` and `md` differ in track and knob size but
   not in travel. Don't derive travel from track width.
4. `onChange` fires with the *next* value, computed in the component.
5. `docs/dev/ui-atoms.md`: "`Toggle`, `PanelCollapseToggle` and `BlockingOverlay` set
   their own roles and states; a call site that adds `role="switch"` to a `Toggle` is
   double-declaring it."

---

## Surface: SegmentedControl — exclusive pill row

- **Source**: `src/components/atoms/SegmentedControl.tsx:1-99`
- **Renders**: `<div role="group" aria-label={label}>` with one
  `<button aria-pressed data-active>` per option.
- **Layout & styling**:
  - container: `'inline-flex items-center gap-1'` + `fullWidth && 'w-full'` +
    caller `className`; inline `style={{ '--segmented-tone': tone === 'accent' ?
    'var(--accent)' : 'var(--accent-secondary)' }}`;
  - option: `'ui-segmented-option inline-flex items-center justify-center rounded-sm
    border font-semibold uppercase tracking-wide transition-colors'` + size class +
    `fullWidth && 'flex-1'`;
  - sizes: `sm: 'h-7 px-2.5 text-xs'` (28px high, 12px), `md` (default):
    `'h-10 min-w-[92px] px-3 text-[12px]'` (40px high, min 92px, 12px);
  - `rounded-sm` = 4px; `border` = 1px, colour from `.ui-segmented-option`;
  - `data-active={active ? 'true' : undefined}` drives
    `.ui-segmented-option[data-active="true"]`:
    border `color-mix(in srgb, var(--segmented-tone, var(--accent)), white 10%)`,
    background `color-mix(…, var(--surface-0) 76%)`,
    text `color-mix(…, var(--text-strong) 25%)`;
  - inactive: `background: var(--surface-1); border-color: var(--border-subtle);
    color: var(--text-muted)`; hover (not disabled): `background: var(--surface-2)`.
  - Source comment:
    > `/** A row of mutually exclusive pills: ON/OFF switches, tab strips and 2 to 4 way pickers. Around 50 call sites hand-wrote this before, most of them repeating the same four-line active style block. */`
- **States**: active (accent-tinted or `accent-secondary`-tinted per `tone`), inactive,
  hover, disabled (`option.disabled`; `.ui-segmented-option:hover:not(:disabled)`
  suppresses hover, and the global `button:disabled` paints the grey disabled fill).
- **Data**: `options: Array<{ value: T; label: ReactNode; title?; disabled? }>`,
  `value: T`, `onChange(value: T)`, `label` (group aria-label), `size`
  (default `'md'`), `tone` (default `'accent'`; `'accent-secondary'` is what the
  sidebar tab strips use), `className`, `fullWidth` (default `false`).
- **Callbacks**: `onClick={() => onChange(option.value)}` on every option, including
  the already-active one (no guard) — a `disabled` button cannot fire.
- **Text**: all from `option.label`; uppercased by CSS.
- **Port target**: proposed `ui/segmented.rs`. Nothing exists.

### Deliberate decisions and gotchas (SegmentedControl)

1. Selection is a *CSS variable on the container* (`--segmented-tone`), not a colour
   per option: one class covers both accent families. A port should carry a `tone`
   parameter down, not two duplicate rule sets.
2. `md` is 40px tall with `min-w-[92px]`; `sm` is 28px with no minimum. Both are
   uppercase via CSS, so the label text is **not** pre-uppercased.
3. Clicking the active pill re-fires `onChange` with the same value. Several callers
   rely on the idempotence; do not add a guard without checking them.
4. `fullWidth` sets both `w-full` on the group and `flex-1` per option; without it the
   group is `inline-flex`.
5. `aria-pressed` (not `role=tab`/`radio`) is the accessibility contract.
6. **This atom covers only one of four segmented looks in the app** (`docs/dev/ui-atoms.md`):
   > "The app has four different segmented looks: the uppercase ON/OFF pill (`SegmentedControl`), the underline tab strip (`ui-button ui-button-secondary !h-7 !px-2.5 !py-0 text-xs` with a colour-only active state), the compact grid strip (`grid grid-cols-N gap-1` of `rounded border px-1.5 py-1 text-xs font-medium`), and a bordered capsule around `ui-button-ghost` buttons. The other three are still written at their call sites; do not map them onto `SegmentedControl` without first giving it that variant, or their design changes."

---

## Surface: Input — the one-line text field

- **Source**: `src/components/atoms/Input.tsx:1-7`
- **Renders**: `<input className="ui-input …" {...props} />`.
- **Layout & styling**: `.ui-input` only (see CSS). Renders 13px type, `6px 10px`
  padding, 6px radius, `--surface-0` fill, `--border-subtle` border; focus swaps the
  border to `--accent` and removes the outline.
- **States**: default, focus (`border-color: var(--accent)`), disabled (UA styling —
  no `.ui-input:disabled` rule exists; the browser greys it), invalid (none).
- **Data**: standard `InputHTMLAttributes`; `className` is appended.
- **Callbacks**: all pass-through.
- **Text**: none.
- **Port target**: proposed `ui/input.rs`. Nothing exists.
- **Gotcha**: there is no placeholder styling, no `:disabled` rule and no error state.
  `.ui-input` carries no height, so call sites set `h-8`/`!h-*`; it is also applied by
  hand to raw `<textarea>`s and other fields outside this atom.

---

## Surface: Select — native `<option>` API over SelectDropdown

- **Source**: `src/components/atoms/Select.tsx:1-122`
- **Renders**: `SelectDropdown` (not a native `<select>`), configured with
  `selectClassName={cn('ui-select', className)}` and `selectStyle={style}`.
- **Layout & styling**: the trigger therefore carries **both** `ui-input` (rendered by
  SelectDropdown) and `ui-select`. Both are unlayered and `.ui-select` appears later in
  the file, so it wins on shared properties: height 32px, `padding: 0 28px 0 10px`,
  `line-height: 1`, `vertical-align: middle`, 13px type, 6px radius.
- **States**: as SelectDropdown; `disabled` is forwarded.
- **Data**: children are parsed:
  - `React.Fragment` and `<optgroup>` are flattened recursively;
  - each `<option>` becomes `{ value: String(option.value ?? label), label:
    extractOptionLabel(children) || value, disabled }`, where the label is the
    recursive text of the option's children (strings, numbers, arrays, elements'
    `children`);
  - controlled when `value !== undefined`; otherwise internal state, seeded from
    `defaultValue` or **the first non-disabled option**;
  - a `useEffect` re-seeds the uncontrolled value when `defaultValue` changes or the
    current value is no longer in `options`.
- **Callbacks**: `onChange` is called with a **synthetic event**:
  `{ target: { value }, currentTarget: { value } } as React.ChangeEvent<HTMLSelectElement>`
  — it has no `preventDefault`, no `nativeEvent`, no `type`. Callers only read
  `e.target.value`.
- **Text**: all from `<option>` children.
- **Port target**: proposed `ui/select_compat.rs` (only if the port keeps the
  `<option>`-children API; otherwise port `SelectDropdown` and delete this wrapper).
  Nothing exists.

### Deliberate decisions and gotchas (Select)

1. It is **not** a `<select>`. There is no native dropdown, no iOS picker, no form
   participation, and no `multiple`.
2. `defaultValue` is re-applied by an effect whenever it changes — not the React
   "uncontrolled after mount" semantics.
3. The default selection is the first *enabled* option, not the first option.
4. The synthetic change event is a plain object literal. `e.preventDefault()`,
   `e.stopPropagation()`, `e.persist()` are undefined or throw.
5. `<optgroup label>` is ignored (its label is not rendered anywhere); only its
   children are flattened.

---

## Surface: SelectDropdown — the custom listbox

- **Source**: `src/components/ui/SelectDropdown.tsx:1-556`
- **Renders**: `<label class="space-y-1 block {className}">` → optional
  `<span class="ui-label font-medium inline-flex items-center">` → `<div class="relative">`
  → `<button role="combobox" aria-expanded aria-haspopup="listbox">` (the trigger) →
  a portalled `<div role="listbox">` when open.
- **Layout & styling**:
  - Trigger:
    ```
    ui-input rounded-[4px] relative w-full h-[36px] px-2.5 py-0 pr-10 leading-none text-sm
    disabled:pointer-events-none disabled:opacity-55 disabled:cursor-not-allowed
    inline-flex items-center text-left {selectClassName}
    ```
    Rendered result (cascade): the `ui-input` base wins → radius 6px (`rounded-[4px]`
    loses), `padding: 6px 10px` (`px-2.5`/`pr-10` lose; call sites that need the room
    use `!pr-9`), 13px type (`text-sm` loses), `line-height: 1.3` (`leading-none`
    loses), height 36px (`h-[36px]` applies), `inline-flex items-center text-left`,
    `w-full`, `relative`.
  - Trigger open style (inline): `borderColor: color-mix(in srgb, var(--accent),
    var(--border-subtle) 18%)` and `boxShadow: 0 0 0 1px color-mix(in srgb, var(--accent),
    transparent 72%) inset`.
  - Leading slot: `<span class="mr-1.5 inline-flex h-[18px] w-[18px] shrink-0
    items-center justify-center">` (18px + 6px gap).
  - Selected label: `<span class="min-w-0 flex-1 truncate">` (+ `text-center` when
    `selectedTextAlign === 'center'`; `text-transparent` when
    `hideSelectedText && selectedDisplay`), with
    `marginLeft: '-1.5rem'` when a leading display and centred text are both present
    (comment: "A leading icon is a fixed 18px plus 6px gap; centring inside the space
    left over from it would centre this text to the right of the control. Reclaim that
    width so the label centres on the control, dot aside.").
  - Overlay selected display (when `selectedDisplay && hideSelectedText`):
    `<span class="pointer-events-none absolute top-1/2 inline-flex -translate-y-1/2
    items-center {alignment === 'center' ? 'left-1/2 -translate-x-1/2' : 'left-2.5'}">`
    + `marginLeft: selectedDisplayOffsetX` when non-zero.
  - `endAdornment`: `<span class="pointer-events-none absolute right-8 top-1/2
    inline-flex -translate-y-1/2 items-center justify-center">`.
  - Chevron: `lucide-react` `ChevronDown`, class
    `'pointer-events-none absolute {chevronClassName} top-1/2 h-3.5 w-3.5 -translate-y-1/2
    transition-transform'` + `rotate-180` when open; `chevronClassName` default
    `'right-3'` (12px); inline `color: 'var(--text-muted)'` in **both** branches of
    `disabled ? … : …`.
  - Menu (portal to `document.body`):
    ```
    fixed z-[9999] rounded-[4px] border shadow-xl {menuClassName}
    ```
    inline: `top`/`left` from measurement (0 until measured), `width` and `minWidth` =
    trigger width, `maxHeight` = available space, `visibility: 'hidden' | 'visible'`,
    `borderColor: color-mix(in srgb, var(--accent), var(--border-subtle) 82%)`,
    `background: color-mix(in srgb, var(--surface-0), black 6%)`,
    `backdropFilter/WebkitBackdropFilter: 'blur(6px) saturate(112%)'`,
    `boxShadow: '0 14px 34px rgba(0, 0, 0, 0.46), 0 4px 12px rgba(0, 0, 0, 0.32)'`.
  - Menu inner: `<div class="flex max-h-full flex-col">` → scroll area
    `<div class="overflow-y-auto custom-scrollbar" style="max-height">` → footer slot.
  - Heading option: `<div class="px-3 pb-1 pt-2 text-[10px] font-semibold uppercase
    tracking-wide">` with `color: var(--accent)` and
    `borderBottom: 1px solid color-mix(in srgb, var(--border-subtle), transparent 35%)`.
  - Divider option: `<div class="border-t">` with `borderColor: var(--border-subtle)`.
  - Row: `<button role="option" class="group w-full px-3 py-2 text-left text-sm
    transition-colors inline-flex items-center gap-2 border-b last:border-b-0 {optionClassName}">`.
    Row border colour: `transparent` when it is the last row **or** the next option is a
    heading/divider; otherwise
    `color-mix(in srgb, var(--border-subtle), transparent 16%)`.
    Row background: selected → `color-mix(in srgb, var(--accent), var(--surface-0) 86%)`;
    else hovered → danger `color-mix(in srgb, var(--danger), transparent 88%)`,
    accent `color-mix(in srgb, var(--accent-secondary), var(--surface-0) 92%)`,
    default `color-mix(in srgb, var(--surface-2), transparent 18%)`; else `transparent`.
    Row text: danger → `var(--danger)`; accent → `var(--accent-secondary)`; else
    `(selected || hovered) ? var(--text-strong) : var(--text-muted)`.
    Disabled rows: inline `opacity: 0.55; cursor: not-allowed; borderBottomColor:
    var(--border-subtle)`.
  - Row icon slot: `<span class="inline-flex h-3.5 w-3.5 shrink-0 items-center
    justify-center">`; icon colour danger/accent tones, else selected → `var(--accent)`,
    hovered → `var(--text-strong)`, else `var(--text-muted)`.
  - Row label: `<span class="truncate text-[0.95em]">`.
  - `rightContent`: `<span class="ml-auto inline-flex shrink-0 items-center
    justify-end text-[10px]">` with `color: var(--text-muted)`.
  - Footer action: container `<div class="py-0">` with
    `borderTop: menuFooterDivider ? '1px solid color-mix(in srgb, var(--border-subtle),
    transparent 14%)' : 'none'` and `background: transparent`; button
    `<button class="w-full px-3 py-2 text-left text-sm inline-flex items-center gap-2
    transition-colors">`; colours by `tone`:
    danger → `var(--danger)`, hover `color-mix(in srgb, var(--danger), var(--surface-0) 92%)`;
    accent → `var(--accent-secondary-action-color)`, hover
    `color-mix(in srgb, var(--accent-secondary), var(--surface-0) 92%)`;
    default → text `(hovered ? var(--text-strong) : var(--text-muted))`, hover
    `color-mix(in srgb, var(--surface-2), transparent 18%)`. Icon slot
    `class="inline-flex h-3.5 w-3.5 shrink-0 items-center justify-center"`; label
    `<span class="truncate">`.
  - `CENTERED_SELECT_PADDING` export: `{ paddingLeft: 0, paddingRight: 26, textAlign:
    'center' }`, documented as:
    > "The chevron's footprint is `right-3` (12px) plus its `h-3.5 w-3.5` icon (14px) — 26px in from the right edge — hence no left inset and 26px on the right. The label then centres on `(width - 26) / 2`, which is 13px left of the control's centre…"
- **Placement algorithm** (`updateMenuPosition`): `margin = 8`, `gap = 6`;
  `left = rect.left` (or `rect.right - measuredMenuWidth` for `menuAlign: 'right'`),
  then `clampToViewport({ x: left, y: rect.bottom + gap }, { width, height }, { margin,
  viewport })` for the horizontal clamp only;
  `belowSpace = innerHeight - (rect.bottom + gap) - margin`,
  `aboveSpace = rect.top - gap - margin`;
  opens upward when `measureMenu && belowSpace < Math.min(180, measuredMenuHeight) &&
  aboveSpace > belowSpace`, in which case `top = Math.max(margin, rect.top -
  measuredMenuHeight - gap)` and `maxHeight = Math.max(120, aboveSpace)`; otherwise
  `top = rect.bottom + gap`, `maxHeight = Math.max(120, belowSpace)`.
  `minWidth`/`width` = `rect.width` (the menu re-measures after writing that width so
  `!w-64`-style overrides are reflected — see the source comment).
- **States**: closed; open (trigger border/shadow, chevron rotated 180°); hovered row;
  selected row; disabled trigger (`disabled:pointer-events-none disabled:opacity-55`);
  disabled row; focused (container `onFocusCapture` sets `isFocused`, but **nothing
  renders from `isFocused` except being cleared on blur**).
- **Data**: full prop list at `SelectDropdownProps`; internal state `isOpen`,
  `isFocused`, `hoveredOptionKey`, `isFooterActionHovered`, `menuPosition`.
  `selectedOption` = first option with matching `String(value)` that is not a heading
  and not a divider; `selectedLabel = selectedOption?.label ?? String(value)`;
  `triggerLeading = leadingDisplay ?? selectedOption?.icon ?? null` (comment: "The
  trigger carries the selected option's icon too, so a colour swatch reads in the
  closed control and not only in the menu.").
- **Callbacks**:
  - trigger click: `setIsOpen(prev => !prev)` (guarded by `disabled`);
  - trigger keys: `Enter` / `' '` / `ArrowDown` → `preventDefault` + open;
    `Escape` → `preventDefault` + close. No other keys.
  - option click: `onChange(option.value)` → close → `setIsFocused(false)` →
    `clearActiveFocus()` (blurs `document.activeElement` and the trigger) → the same
    again inside `requestAnimationFrame`;
  - footer click: `menuFooterAction.onClick()` → close → same focus clearing;
  - container `onFocusCapture` → `onFocus?.(event)` once; `onBlurCapture` → if the new
    focus target is outside both the container and the menu, close + `onBlur?.(event)`;
  - dismissal: `useOutsideDismiss(isOpen, closeMenu, { ignoreRef: containerRef,
    dismissOnResize: false, dismissOnScroll: false })` — Escape and outside
    pointerdown only; resize/scroll reposition instead.
- **Text**: no built-in strings (all labels/headings come from props). The chevron is
  decorative (`pointer-events-none`); the trigger carries `ariaLabel`/`title`.
- **Port target**: proposed `ui/select_dropdown.rs`. Nothing exists.

### Deliberate decisions and gotchas (SelectDropdown)

1. The menu is portalled and measured before paint (`useIsomorphicLayoutEffect`) so it
   never shows at a guessed position; `visibility: 'hidden'` until measured. Any port
   must place-then-show, not show-then-place.
2. Resize/scroll do **not** dismiss (unlike ContextMenu) — they reposition. That is
   deliberate and the source says so.
3. Clicking an option blur()s the active element twice (once now, once next frame) to
   kill the focus ring; without it the trigger keeps a focus outline after the menu
   closes.
4. `option.action: true` means "a command, not a value": it still renders as a row and
   still calls `onChange`, but it is never shown as the selected value. `heading` and
   `divider` rows never match `selectedOption`; `divider` ignores `label`.
5. Keyboard support stops at "open/close": no arrow-key navigation, no
   `aria-activedescendant`, no roving focus inside the listbox. A GPUI port can do
   better, but it changes behaviour callers never had.
6. The chevron's `disabled ? 'var(--text-muted)' : 'var(--text-muted)'` is a no-op
   conditional — do not "fix" it into a different disabled colour; the app has none.
7. The menu width is forced to the trigger width (`minWidth` + `width`), and a
   `!`-important width in `menuClassName` is measured *after* being written.
8. `className` lands on the outer `<label>`, `selectClassName` on the trigger,
   `selectStyle` on the trigger. LanguageSwitcher relies on this (`className="w-36 …"`).
9. The `h-[36px]` trigger height is taller than the 32px `.ui-select` wrapper — two
   control heights coexist.

---

## Surface: ContextMenu — the one menu widget

Contract document: `docs/dev/context-menu.md` (exists; this section is the exhaustive
reading of the code and matches it). It summarises: "Placement… clamps into the viewport
before the paint… Dismissal… Selection. Choosing a row calls `onSelect(id)` and then
`onClose()`. The caller does not close the menu itself."

- **Source**: `src/components/ui/ContextMenu.tsx:1-386`
- **Renders**: portal → `<div role="menu" aria-label>` positioned at a clamped point,
  containing an optional title row, a `space-y-0.5` column of `<button role="menuitem">`
  rows with optional separators, and optionally a nested flyout `<div role="menu">`.
- **Layout & styling**:
  - constants: `EDGE_MARGIN_PX = 8`, `SUBMENU_CLOSE_DELAY_MS = 120`,
    `MENU_SURFACE = 'rounded-lg border p-1.5 shadow-xl backdrop-blur-sm'`,
    `MENU_SURFACE_STYLE = { borderColor: 'var(--border-subtle)', background:
    'color-mix(in srgb, var(--surface-0), #000 10%)' }`;
  - root: `fixed {zIndexClassName} {widthClassName} {MENU_SURFACE}` — defaults
    `z-[130]`, `w-48` (192px); inline `left`/`top` from
    `clampToViewport(position, { width, height }, { margin: 8 })`, falling back to the
    raw `position` before the first measurement;
  - title: `<div class="mb-1 px-2 py-1 text-[10px] font-semibold uppercase
    tracking-wide" style="color: var(--text-muted)">`;
  - separator: `<div role="separator" class="my-1 h-px" style="background:
    var(--border-subtle)">`, inserted before any entry with `startsGroup` that is not
    the first;
  - row (`MenuRow`): `<button role="menuitem" class="flex w-full items-center gap-2
    rounded-md px-2 py-1.5 text-left text-[13px] font-medium transition-colors">`
    with inline `color` = disabled `var(--text-muted)` / danger `var(--danger)` /
    `var(--text-strong)`; `opacity: disabled ? 0.55 : 1`; `cursor: disabled ?
    'not-allowed' : 'pointer'`. Hover is **imperative**: `onMouseEnter` sets
    `event.currentTarget.style.background = hoverFill`, `onMouseLeave` resets it to
    `'transparent'`; `hoverFill` = danger
    `color-mix(in srgb, var(--danger), var(--surface-1) 90%)` else
    `color-mix(in srgb, var(--accent), var(--surface-1) 82%)`;
  - icon slot: `<span class="inline-flex h-5 w-5 items-center justify-center
    {iconBare && !checked ? '' : 'rounded border'}">` with inline
    `borderColor: var(--border-subtle)`, `background: iconBare && !checked ?
    'transparent' : 'var(--surface-1)'`, `opacity: disabled ? 0.8 : 1`,
    `color: checked && !disabled ? 'var(--accent)' : undefined`; content is
    `checked ? <Check class="h-3.5 w-3.5"> : iconNode ?? <Icon class="h-3.5 w-3.5">`;
  - label span: `class={trailing ? 'flex-1' : undefined}`; trailing node follows;
  - submenu row: same `MenuRow` with `hasPopup`, `aria-expanded={expanded}`, trailing
    `<ChevronRight class="h-4 w-4 shrink-0 opacity-60 transition-transform
    {submenuPlacement.left && expanded ? 'rotate-180' : ''}">`;
  - flyout: `<div role="menu" class="absolute {up ? 'bottom-0' : 'top-0'}
    {left ? 'right-full mr-1' : 'left-full ml-1'} z-[131] {widthClassName}
    {MENU_SURFACE}">` with the same surface style.
- **Placement / flips**:
  - root: measure the rendered box, `clampToViewport(position, size, { margin: 8 })`;
    state is written only when the clamped point actually changed.
  - flyout: measured in `useLayoutEffect`; flips up when
    `flyout.bottom > innerHeight - 8 && row.top - flyout.height >= 8`; flips left when
    `flyout.right > innerWidth - 8 && row.left - flyout.width >= 8`; placement is
    reported back so the parent chevron can mirror.
- **States**: closed (`position === null` → renders `null`); open; row hover (imperative
  inline background); row disabled (dimmed text, `opacity: 0.55`, no hover fill, click
  swallowed); danger row; checked row; submenu row expanded/collapsed; submenu
  auto-disabled when **every** child is disabled.
- **Data**: `position: {x,y} | null`, `entries: ContextMenuEntry[]`, `title?`,
  `ariaLabel?`, `dismissIgnoreRef?`, `widthClassName` (default `'w-48'`),
  `zIndexClassName` (default `'z-[130]'`). Entry fields: `id`, `label` (node),
  `icon?`, `iconNode?`, `iconBare?`, `trailing?`, `checked?`, `disabled?`, `danger?`,
  `startsGroup?`, `children?`. Internal: `placement`, `openSubmenuId`,
  `submenuPlacement`, `submenuCloseTimerRef`.
- **Callbacks**:
  - leaf `onClick` → `onSelect(entry.id); onClose();` (the widget closes itself);
  - submenu row hover **or** click → `openSubmenu(id, disabled)` (cancels the pending
    close, resets placement, sets the open id);
  - `onMouseLeave` on the submenu wrapper (and on the flyout) → `scheduleSubmenuClose`
    (120ms timer); entering the flyout cancels it;
  - `useOutsideDismiss(open, onClose, { ignoreRef: dismissIgnoreRef })` → Escape,
    outside pointerdown, **resize and scroll** all close;
  - `onPointerDown` on the root calls `stopPropagation()` so clicks inside never reach
    the dismiss listener;
  - reopening (`position` identity changes) resets the open submenu and its placement
    in an effect.
- **Text**: none built in; `title`/`label`/`trailing` come from the caller.
- **Port target**: proposed `ui/context_menu.rs`. Nothing exists.
- **Call-site owners** (from `docs/dev/context-menu.md`): `ModelManagerPanel.tsx`,
  `TopBar.tsx`, `FloatingPanelStack.tsx`, `PresetSelector.tsx`,
  `SupportAnatomyPreviewSlot.tsx`, plus `EditorContextMenu`.

### Deliberate decisions and gotchas (ContextMenu)

1. Hover is applied by mutating `style.background` in JS, not by a CSS `:hover` rule.
   That is why a disabled row can suppress the fill and why it resets to
   `'transparent'` rather than a token.
2. The root's `onPointerDown` `stopPropagation()` is load-bearing: without it, every
   click inside the menu would hit the outside-dismiss listener.
3. A submenu row never emits its own id; only leaves do. `onSelect` receives leaf ids.
4. Submenu auto-disable is "all children disabled", not a caller's own notion of
   enablement — the widget only knows `entry.disabled`.
5. `iconBare` removes the frame only while `!checked`; a checked bare row still gets the
   bordered slot to hold the tick.
6. The flyout sits at `z-[131]`, one above the root's `z-[130]`; both are fixed numbers,
   overridable only as a whole via `zIndexClassName` on the root.
7. The root renders `null` when `position === null` and also guards
   `typeof document === 'undefined'` (SSR).
8. No keyboard navigation (no arrow keys, no Home/End) and no focus trap; rows are
   `<button>`s so Tab works.
9. One level of nesting only: "A submenu's rows are leaves; a flyout inside a flyout is
   not supported" (`docs/dev/context-menu.md`).

---

## Surface: EditorContextMenu — the canvas menu vocabulary

- **Source**: `src/components/ui/EditorContextMenu.tsx:1-224`
- **Renders**: `ContextMenu` with entries built from `items` (default `MENU_ITEMS`),
  `ariaLabel={_(msg`Editor context menu`)}`, `title` passed through.
- **Layout & styling**: inherited entirely from ContextMenu.
- **States**: each entry disabled iff its id is in `disabledActions`; a submenu entry is
  disabled iff **all** of its children are. `checked`/`iconNode`/`iconBare` are copied
  for leaves only.
- **Data**: `items: MenuItemDef[]` (default `MENU_ITEMS`), `disabledActions:
  EditorMenuAction[]` (default `[]`), `position`, `title`. Labels are
  `MessageDescriptor | string`; strings pass through unresolved (used for data-built
  rows such as a material name).
- **Callbacks**: `onSelect(id)` → `onAction(id as EditorMenuAction)`; `onClose`
  forwarded.
- **Text** (messages and order in `MENU_ITEMS`):
  | id | label | icon | notes |
  |---|---|---|---|
  | `delete` | ``msg`Delete` `` | `Trash2` | first group |
  | `cut` | ``msg`Cut` `` | `Scissors` | |
  | `copy` | ``msg`Copy` `` | `Copy` | |
  | `paste` | ``msg`Paste` `` | `ClipboardPaste` | |
  | submenu `repair-menu` | ``msg`Repair` `` | `Wrench` | `startsGroup: true` |
  | ↳ `repair` | ``msg`Auto-Repair` `` | `Wrench` | |
  | ↳ `remesh` | ``msg`Remesh` `` | `Pyramid` | |
  | ↳ `shrinkwrap` | ``msg`Shrink-Wrap` `` | `RefreshCw` | |
  | submenu `supports-menu` | ``msg`Supports` `` | `Blocks` | `startsGroup: true` |
  | ↳ `split-supports` | `msg({ message: 'Split supports', comment: 'Context-menu command that detaches the generated support scaffolding at the clicked point. "Supports" = the temporary print scaffolding structures, not customer support.' })` | `Split` | |
  | ↳ `merge-supports` | `msg({ message: 'Merge supports', comment: 'Context-menu command that re-attaches the support scaffolding back to the model.' })` | `Link` | |
  | ↳ `scan-for-supports` | ``msg`Scan for Supports` `` | `Search` | |
  | submenu `geometry-menu` | ``msg`Geometry` `` | `Shapes` | |
  | ↳ `mark-as-support-geometry` | ``msg`Mark as Support` `` | `LifeBuoy` | |
  | ↳ `mark-as-model-geometry` | ``msg`Mark as Model` `` | `Box` | |

  Commented out in the list (but still in the `EditorMenuAction` union and with
  imported icons): `link-models` / `unlink-models` (`Link`/`Unlink`).
  Exported convenience items: `ORGANIC_CUT_ADD_WAYPOINT_ITEM`
  (``msg`Add waypoint here` ``, `Plus`) and `ORGANIC_CUT_DELETE_WAYPOINT_ITEM`
  (``msg`Delete waypoint` ``, `Trash2`).
  Source comment on the Repair submenu:
  > `// Mesh repair, fanned out like supports and geometry. The first entry reads "Repair Again" and the report entry only appears once the model has one, so the caller builds this submenu from these items; this is the shape it starts from.`
  The action union also carries `show-repair-report`, `undo-repairs`,
  `supports-toggle-curve`, `supports-add-joint`, various `assign-material:<materialId>`,
  and the two `organic-cut-*` ids — none of which appear in `MENU_ITEMS`.
- **Port target**: proposed `ui/editor_context_menu.rs`. Nothing exists.

### Deliberate decisions and gotchas (EditorContextMenu)

1. `MENU_ITEMS` is a *starting shape*, not the product menu: the repair submenu's
   contents (and "Repair Again" wording) are built by the caller.
2. `label: string` bypasses the catalog on purpose — data-built rows (material names)
   are already localized elsewhere.
3. Only leaf ids reach `onAction`; `repair-menu`/`supports-menu`/`geometry-menu` are
   never emitted.
4. A submenu is disabled only when **all** children are in `disabledActions`.
5. `assign-material:<id>` is encoded in the id string; parsing is the caller's job.
6. `docs/dev/context-menu.md`: "the Organic Cut tool supplies its own one-row lists
   through it (`src/features/organicCut/OrganicCutMounts.tsx`). New menus should use
   `ContextMenu` directly."

---

## Surface: Tooltip — cursor- or trigger-anchored popover

- **Source**: `src/components/ui/Tooltip.tsx:1-280`
- **Renders**: `<span class="inline-flex">` wrapper around the single child, plus a
  portalled `<div>` when shown. If `content` is falsy the child is returned unwrapped.
- **Layout & styling**: popover class
  `"fixed pointer-events-none z-[9999] rounded px-2 py-1.5 text-[11px] leading-tight font-medium shadow-lg"`,
  inline:
  ```
  left: 0, top: 0, visibility: 'hidden',
  background: 'rgba(24, 24, 24, 0.98)',
  color: 'var(--text-strong, #e0e0e0)',
  border: '1px solid var(--accent, #baf72e)',
  width: width ?? 'max-content',
  maxWidth,
  whiteSpace: 'normal',
  textAlign: 'left',
  boxShadow: '0 6px 32px 0 rgba(0,0,0,0.44), 0 1.5px 8px 0 rgba(0,0,0,0.28)'
  ```
  Wrapper: `inline-flex` (+ `wrapperClassName` appended as `inline-flex ${…}`), or
  `{ display: 'flex', width: '100%', minWidth: 0 }` when `fullWidth`.
- **Placement**:
  - `anchor='cursor'` (default): box centred under the pointer
    (`x = cursor.x - width/2`, `y = cursor.y + offsetY`), flipped above
    (`y = cursor.y - offsetY - height`) when the preferred spot overflows the bottom;
  - `anchor='trigger'`: `triggerPlacement` — `below` centres on the trigger's bottom
    (`y = rect.bottom + gap`), `right` centres vertically on the trigger and sits at
    `rect.right + gap`; the mirror position is the flip;
  - clamp via `clampToViewport(…, { margin: 4 })`; flip is chosen by
    `placement === 'right' ? placed.overflowRight : placed.overflowBottom`; a final
    `settled.top < 4 ? 4 : settled.top` keeps it off the top edge. Positioning is
    written directly to `el.style.left/top` and then `visibility = 'visible'`, in a
    `useLayoutEffect` before paint.
- **States**: hidden (no content → child unwrapped), waiting (delay timer), shown;
  cursor mode re-places on every `mousemove`; trigger mode ignores moves; focus shows
  (`onFocus`) and blur hides (`onBlur`) the same popover (that is why trigger mode
  exists: keyboard focus has no cursor).
- **Data**: props `content`, `offsetY` (default `28`), `maxWidth` (default `260`),
  `width?`, `anchor` (default `'cursor'`), `placement` (default `'below'`),
  `delayMs` (default `0`), `wrapperClassName?`, `fullWidth` (default `false`),
  `children` (exactly one element). Internal `hovered`, `pos`, `delayTimerRef`.
- **Callbacks**: pure presentation; no store writes. Mouse enter/move/leave and
  focus/blur on the wrapper only.
- **Text**: `content` only.
- **Port target**: proposed `ui/tooltip.rs`. Nothing exists.

### Deliberate decisions and gotchas (Tooltip)

1. It is portalled to `<body>` with a long source comment explaining why — quote it
   when deciding whether GPUI needs an overlay layer:
   > `// Portalled to <body> rather than left where it sits in the tree. `position: fixed` // resolves against the nearest ancestor that carries a transform, filter or // backdrop-filter — and inside the settings modal that is the panel itself, which // also clips with overflow:hidden. Left in place the popover is positioned against // the panel and then clipped away: present in the DOM, reported visible, and // painted nowhere.`
2. Placement mutates the DOM in a layout effect rather than storing the position in
   state — "measuring it and moving it is work on the DOM … without spending a render
   on a position only the browser needs."
3. `delayMs` only delays the *first* appearance; movement after it is instant.
4. `clampToViewport` margin is **4** here, not the 8 used by menus.
5. The colours have fallbacks (`var(--text-strong, #e0e0e0)`, `var(--accent,
   #baf72e)`) and the accent border is a hard 1px.
6. Empty `content` skips the wrapper entirely, so a conditional tooltip does not add a
   layout box.
7. `fullWidth` exists because an `inline-flex` wrapper sizes to the trigger's
   *untruncated* content and defeats `truncate`; the trigger still needs `min-w-0`.

---

## Surface: MouseTooltip — free-floating cursor follower

- **Source**: `src/components/ui/MouseTooltip.tsx:1-80`
- **Renders**: `null` unless `visible && pos`, else
  `<div class="fixed pointer-events-none z-[9999] {className}">`.
- **Layout & styling**: no surface styling at all — the caller supplies the look via
  `className`/children. Position and clamped width are written to the element in a
  layout effect that runs on **every render** (no dependency array).
- **Placement**: on each `mousemove` (window listener) it stores the client point; the
  layout effect clears `width/left/top`, measures at natural width, freezes that width,
  reads how far a caller's `translate-*` class shifted the box (`rect.left/rect.top`),
  then clamps `{ x: pos.x + offset.x + shiftX, y: pos.y + offset.y + shiftY }` with
  `clampToViewport(…, { margin: 0 })` and flips by `overflowRight`/`overflowBottom` to
  the other side of the cursor, never covering it. Source comment:
  > `// Placed after it renders, from the size it actually has. Measured at the cursor, a box // near the right edge wraps to whatever width is left there; a position worked out from // that width puts it somewhere it then grows out of, and it took three mouse moves to // settle. At the viewport's corner nothing squeezes it.`
- **States**: hidden (`visible === false` or no pointer position yet), shown.
- **Data**: `children`, `visible` (default `true`), `offset` (default `{ x: 15, y: 15 }`),
  `className`.
- **Callbacks**: none.
- **Port target**: proposed `ui/mouse_tooltip.rs`. Nothing exists.

### Deliberate decisions and gotchas (MouseTooltip)

1. It does **not** portal; it relies on `fixed` and is vulnerable to the same
   transformed-ancestor problem Tooltip's comment describes.
2. The width is frozen after the first measurement so the box cannot reflow as it moves.
3. `offset` is a *cursor* offset, not a viewport margin; the clamp uses margin 0.
4. The layout effect runs every render by design (no dep array) — that is how it
   re-measures after a content change.

---

## Surface: StructuredDialogModal — the dismissible modal shell

- **Source**: `src/components/ui/StructuredDialogModal.tsx:1-160`
- **Renders**: `createPortal` → backdrop `<div class="fixed inset-0 {z} flex
  items-center justify-center bg-black/55 backdrop-blur-sm px-3">` → panel
  `<div class="w-full {maxWidthClassName} overflow-hidden rounded-xl border shadow-2xl
  {panelClassName}">` → header (`flex items-center justify-between gap-4 border-b px-5
  py-4`) → body `<div class={bodyClassName}>` with children and optional actions.
- **Layout & styling** (defaults, all overridable):
  - `zIndexClassName = 'z-[100]'`, `maxWidthClassName = 'max-w-lg'` (32rem),
    `bodyClassName = 'space-y-4 p-5'`, `actionsClassName = 'grid grid-flow-col
    auto-cols-fr gap-2 pt-1'`;
  - panel inline: `background: 'var(--surface-0)'`, `borderColor:
    'var(--border-subtle)'`, `boxShadow: '0 24px 46px rgba(0,0,0,0.42)'`;
    header border `borderColor: var(--border-subtle)`;
  - icon slot when `icon` is passed:
    `class="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-md border"`,
    `style={ICON_TONE_STYLES[iconTone]}` (`iconTone` default `'warning'`);
  - title `<h2 class="text-base font-semibold leading-tight" style="color:
    var(--text-strong)">`; subtitle `<p class="mt-0.5 text-[11px] leading-snug"
    style="color: var(--text-muted)">`;
  - close button: `<IconButton variant="surface" size="md" aria-label={closeAriaLabel}
    disabled={closeDisabled}><X class="w-4 h-4"/></IconButton>` — a 36×36 bordered
    tile with the neutral tone.
- **States**: closed → `null`; open; close disabled (`closeDisabled` also disables the
  Escape handler); no `onClose` → no close button at all; backdrop press (only when the
  event target is the backdrop itself) → `onBackdropClick() ?? onClose()`.
- **Data**: `open`, `ariaLabel`, `title`, `subtitle?`, `icon?`, `iconTone`
  (default `'warning'`), `zIndexClassName`, `maxWidthClassName`, `panelClassName`,
  `bodyClassName`, `actionsClassName`, `closeAriaLabel` (default `'Close dialog'`),
  `closeDisabled` (default `false`), `onClose?`, `onBackdropClick?`, `children?`,
  `actions?`. Panel has `role="dialog" aria-modal="true" aria-label={ariaLabel}`.
- **Callbacks**: close button → `onClose`; backdrop `onMouseDown` → `onBackdropClick`
  else `onClose`; Escape → `useEscapeToClose(open, closeDisabled ? undefined :
  (onClose ?? onBackdropClick))` — registered on the shared escape stack, top-most
  dialog only.
- **Text**: the default `closeAriaLabel = 'Close dialog'` is a **plain string**, not a
  `msg` descriptor. Everything else comes from props.
- **Port target**: proposed `ui/dialog.rs`. Nothing exists.

### Deliberate decisions and gotchas (StructuredDialogModal)

1. Dismissal has **two** paths: Escape and backdrop `onMouseDown`. Escape is registered
   *disabled* (a handler-less registration that swallows the key) when
   `closeDisabled`, so a modal that cannot be closed still blocks the canvas behind it.
   Source comment:
   > `// Escape closes the dialog by default; a dialog whose close button is // disabled (or that has no close affordance) swallows the key instead.`
2. Backdrop dismissal uses `onMouseDown`, not `click`, and only when
   `event.target === event.currentTarget` — a drag that starts inside the panel and
   ends on the backdrop does not close it.
3. `closeAriaLabel` defaults to the untranslated English `'Close dialog'` — one of the
   few user-visible primitive strings outside the catalog; port it as translatable.
4. The panel has no enter animation class; `ui-modal-panel-enter` exists in CSS but this
   component does not apply it.
5. `shadow-2xl` is overridden by the inline `boxShadow` — the utility is dead here.
6. There is no focus trap and no initial focus call.
7. `docs/dev/ui-atoms.md` calls it "every dismissible dialog: title, optional subtitle
   and icon (`iconTone` reads the shared tone table), close affordance, actions."

---

## Surface: BlockingOverlay — the non-dismissible busy sheet

- **Source**: `src/components/atoms/BlockingOverlay.tsx:1-104`
- **Renders**: `<div class="{position} inset-0 {z} flex items-center justify-center
  {backdrop}">` → `<div class="{width} rounded-xl border px-5 py-4 shadow-xl {className}"
  role="status" aria-live="polite" aria-busy="true">` → title, details, elapsed,
  footnote, optional ProgressBar, children.
- **Layout & styling**:
  - defaults: `zIndexClassName = 'z-[120]'`, `widthClassName = 'w-[min(520px,92vw)]'`,
    `backdropClassName = 'bg-black/45 backdrop-blur-[1px]'`, `position = 'absolute'`;
  - panel inline: `background: 'color-mix(in srgb, var(--surface-0), black 10%)'`,
    `borderColor: 'var(--border-subtle)'`;
  - title: `class="text-sm font-semibold"` + `color: var(--text-strong)`;
  - details: `class="mt-1 space-y-0.5 text-xs leading-relaxed"` + `color:
    var(--text-muted)`, one `<p>` per line;
  - elapsed: `class="mt-2 text-[11px] font-medium tracking-wide"` + `color:
    var(--accent)`;
  - footnote: `class="mt-1 text-[11px]"` + `color: var(--text-muted)`;
  - bar wrapper: `class="mt-3"` → `ProgressBar value={progress ?? 0}
    indeterminate={progress === null} ariaLabel={progressLabel}`;
  - children rendered raw after the bar.
- **States**: no progress (`progress === undefined` hides the bar); determinate;
  indeterminate (`progress === null`); optional lines present/absent. Never dismissible.
- **Data**: `title`, `details?: ReactNode[]`, `elapsed?`, `footnote?`,
  `progress?: number | null`, `progressLabel?`, `children?`, `zIndexClassName`,
  `widthClassName`, `backdropClassName`, `position`, `className`.
- **Callbacks**: none.
- **Text**: all from props.
- **Port target**: proposed `ui/blocking_overlay.rs`. Nothing exists.

### Deliberate decisions and gotchas (BlockingOverlay)

1. Source comment on why it is not a dialog:
   > `/** * The non-dismissible busy sheet: dim backdrop, one panel, no close affordance. * * `StructuredDialogModal` is the wrong shell for this (it is dismissible and it * makes the caller supply a title, icon and close button). Seven copies of this * shape existed, each with its own width, backdrop opacity and z-index. * * The copies announced themselves with `role="dialog"` and `aria-modal="true"` * while refusing every interaction; this is a status surface, so it uses * `role="status"` with `aria-busy`. */`
2. `progress === undefined` means "no bar"; `null` means indeterminate. Three states, not
   two — do not collapse them.
3. The default position is `absolute`, not `fixed`; it is an inset of the nearest
   positioned ancestor (usually the canvas container).
4. Default `z-[120]` sits above any dialog using `StructuredDialogModal`'s default
   `z-[100]`, and below the toasts (`z-[125]`–`z-[127]`), so toasts float over a blocking
   overlay while dialogs do not.
5. `className` lands on the *panel*, not the backdrop; the backdrop has no per-call
   class beyond `backdropClassName`.

---

## Surface: NumberInput — locale-aware numeric field with steppers

- **Source**: `src/components/ui/NumberInput.tsx:1-224`
- **Renders**: `showStepper` (default true) →
  `<div class="relative min-w-0"><input type="text" inputMode="decimal" class={className}/>
  <div class="absolute inset-y-0 right-0.5 flex w-4 flex-col items-center justify-center
  gap-0.5">↑ ↓</div></div>`; `showStepper={false}` → the bare `<input>`.
- **Layout & styling**:
  - the input gets **only** the caller's `className` (no `ui-input`) — call sites pass
    `ui-input …`;
  - stepper wrapper: `absolute inset-y-0 right-0.5 flex w-4 flex-col items-center
    justify-center gap-0.5` (16px wide, 2px from the right);
  - each stepper: `inline-flex h-3 w-3 items-center justify-center rounded
    hover:bg-white/10 disabled:opacity-50` (12×12), icon
    `ChevronUp`/`ChevronDown` at `h-2.5 w-2.5` (10×10), `tabIndex={-1}`,
    ``aria-label={_(msg`Increase value`)}`` / ``_(msg`Decrease value`)``.
- **Editing rules** (the core of the component):
  - `type="text" inputMode="decimal"` — "Use text to allow full control over input
    (like '0,', '', '-')";
  - display string = `fmt.field(value, 2)` (locale decimal separator, no grouping,
    max 2 fraction digits, `-0` normalised to `0`);
  - every keystroke is tested against `inputPattern(fmt.decimalSeparator, minBound)`:
    `^${sign}(?:\d+)?(?:[\.${sep}]\d{0,2})?$` where `sign` is `''` when
    `min != null && min >= 0`, else `'-?'`; a failing string is **ignored entirely**
    (the field keeps the previous text);
  - accepted text is parsed with `fmt.parse` (locale separator, `.` also accepted) and
    `onChange(parsed)` fires only when the parse is finite (so `'-'` and `'1,'` are
    legal intermediate states that do not call back);
  - focus sets `isEditing`; while editing, external `value` changes do **not** rewrite
    the text;
  - blur clears `isEditing`, snaps the text back to `formatValue(safeValue)` — always,
    even if the committed value did not change — then calls `onBlur`;
  - non-finite `value` renders as `0`.
  - Source comment on blur: `// Always snap back to the latest committed prop value on blur. // This avoids stale UI when an attempted edit is rejected upstream // (e.g. destructive-transform modal canceled).`
- **Stepping** (`applyStepDelta(direction)`): base = parsed display text (fallback
  `safeValue`), `next = base + stepSize * direction`, clamped to `[min, max]` when
  present, then `Number(next.toFixed(stepPrecision))` where
  `stepPrecision = Math.min(6, countStepDecimals(stepSize))`; `stepSize` defaults to 1
  when `step` is not a finite positive number. Writes both the display and `onChange`.
- **Wheel**: `handleWheel` first calls the caller's `props.onWheel`; if
  `defaultPrevented`, returns; if `disabled || readOnly`, returns; else
  `wheelStepDirection(e)`; on non-zero it `preventDefault()`s and steps. Source comment:
  > `// Dominant axis, not deltaY: Shift+wheel arrives as horizontal scroll, and // Shift is what ScrollLockedInputs requires to step an unfocused field.`
- **States**: default; focused/editing; stepper hover (`hover:bg-white/10`); stepper
  disabled when `props.disabled || (max != null && current >= max)` (increment) or
  `props.disabled || (min != null && current <= min)` (decrement) → `opacity-50`;
  invalid text (refused, no visual feedback); empty text (allowed while editing, parsed
  NaN, no callback).
- **Data**: `value: number`, `onChange(value: number)`, `showStepper` (default `true`),
  plus any input props (`min`, `max`, `step`, `disabled`, `readOnly`, `onFocus`,
  `onBlur`, `onWheel`, `className`, `aria-label`, …). `onChange` and `value` are
  replaced in the type. Internal `displayValue` state + `isEditing` ref.
- **Callbacks**: typing → `onChange` per accepted parse; steppers → `onChange`; wheel →
  `onChange`; focus/blur → caller's handlers.
- **Text**: ``msg`Increase value` ``, ``msg`Decrease value` `` (aria-labels).
- **Port target**: proposed `ui/number_input.rs`. Nothing exists.
- **Locale behaviour is test-locked**: `src/components/ui/__tests__/localeNumberFields.dom-test.tsx`
  (Spanish) asserts `0,5` renders, `1,25` → `1.25`, `1.5` (dot) still parses, `1,2,3`
  is refused, `-` is allowed as an intermediate state without a callback, and an
  emptied ScrollableNumberField is not zero.

### Deliberate decisions and gotchas (NumberInput)

1. It is a **text** input, not `type="number"`. A number input would use the *system*
   locale and reject the app's decimal comma (same reasoning repeated verbatim in
   `ScrollableNumberField`):
   > `// Text, not "number": a number input writes and reads the decimals of the system's // locale, not the app's, and rejects a comma where the app shows one.`
2. The regex allows only **two** decimals while typing, regardless of `step`; a step of
   `0.001` cannot be typed, only stepped. (The display is also capped at 2.)
3. The pattern's sign is derived from `min >= 0`: a field whose minimum is 0 refuses
   `-` entirely; a field with no `min` allows it.
4. An invalid keystroke is silently dropped — the caret does not move because the DOM
   value never changes.
5. Blur always rewrites the text from the prop, even when nothing changed — that is how
   an upstream rejection is undone.
6. Steppers are `tabIndex={-1}`: keyboard users cannot focus them; wheel is the only
   non-pointer stepping path here, and it is gated only by `disabled`/`readOnly`, *not*
   by focus (unlike ScrollableNumberField).
7. `applyStepDelta` snaps with `toFixed(stepPrecision)` — a step of `0.1` avoids
   `0.30000000000000004`, a step of `1` leaves integers.
8. Caller `className` is the only input styling; there is no default `ui-input`, so a
   call site that forgets it renders an unstyled text box inside a relative wrapper.

---

## Surface: ScrollableNumberField — stepper button + field, wheel-gated

- **Source**: `src/components/ui/scrollableNumberField.tsx:1-260`
- **Renders**: `<div class="flex min-w-0 items-center gap-1|gap-0.5">` →
  `<IconButton>` (Minus) → `<div class="relative w-0 min-w-0 flex-1"><input type="text"
  inputMode="decimal" class="ui-input …"/> optional unit span</div>` → `<IconButton>` (Plus).
- **Layout & styling**:
  - container: `'flex min-w-0 items-center'` + `compact ? 'gap-0.5' : 'gap-1'` +
    `className`;
  - steppers: `IconButton` (solid, default size) with
    `compact ? '!h-6 !w-6' : '!h-8 !w-8'` +
    `'shrink-0 !p-0 !bg-[var(--surface-0)] hover:!bg-[color-mix(in_srgb,var(--surface-0),white_4%)] disabled:!bg-[color-mix(in_srgb,var(--surface-1),black_8%)]'`;
    icon `compact ? 'h-3 w-3' : 'h-3.5 w-3.5'` (12px / 14px);
  - input: `'ui-input w-full text-center tabular-nums font-semibold no-spinners
    !bg-[var(--surface-0)]'` + `compact ? 'h-7 px-1 text-[11px]' : 'h-8 px-1.5 text-xs
    sm:text-sm'` + `unit ? (compact ? 'pr-4' : 'pr-8') : undefined` + `inputClassName`.
    Rendered: 13px (from `.ui-input`; `text-[11px]`/`text-xs`/`sm:text-sm` lose),
    `padding: 6px 10px`, height 28/32px, centred, tabular, semibold;
  - unit: `'pointer-events-none absolute top-1/2 -translate-y-1/2 font-medium'` +
    `compact ? 'right-1 text-[8px]' : 'right-2 text-[10px]'`, `color:
    var(--text-muted)`.
- **Behaviour**:
  - display = `fmt.field(clampedValue, 6)` (**6** fraction digits, unlike NumberInput's 2);
  - typing: state-only draft; when `commitOnBlur === false` (default) each keystroke
    that parses fires `onChange(clamp(parsed))`; empty/partial text fires nothing;
  - blur: always parses and commits (or restores when unparseable); with
    `commitOnBlur` the commit is *deferred* to blur/Enter;
  - Enter: commits (when `commitOnBlur`), then blurs;
  - ArrowUp/ArrowDown: `preventDefault` + step ±1;
  - Escape: restores from the prop and blurs;
  - wheel: a **native, non-passive** listener on the input:
    ```js
    if (document.activeElement !== el && !event.shiftKey) return;
    const direction = wheelStepDirection(event); … preventDefault(); stopPropagation(); stepBy(direction);
    ```
    Source comment:
    > `// React attaches its `wheel` listener at the root as PASSIVE, so // preventDefault() inside an onWheel handler is silently ignored and the wheel // both steps the value AND scrolls whatever container the field sits in. A // native non-passive listener is the only way to consume the event.`
  - stepper disable: `disabled || currentValue <= min` / `disabled || currentValue >= max`.
- **States**: default, focused, stepper hover, stepper disabled, unit present/absent,
  `compact` density.
- **Data**: `value`, `onChange`, `min`, `max`, `step`, `ariaLabel`, `unit?`,
  `disabled` (default `false`), `decreaseTitle` (default `'Decrease value'`),
  `increaseTitle` (default `'Increase value'`), `commitOnBlur` (default `false`),
  `className`, `inputClassName`, `compact` (default `false`). Internal `draftValue`,
  `isEditingRef`, `inputRef`.
- **Callbacks**: as above; also `onChange` during stepping/typing.
- **Text**: no translated strings; the two `title`s default to literal English
  `'Decrease value'` / `'Increase value'` (titles, not aria-labels — the input has
  `ariaLabel`).
- **Port target**: proposed `ui/scrollable_number_field.rs`. Nothing exists.

### Deliberate decisions and gotchas (ScrollableNumberField)

1. The wheel gate is "focused **or** Shift held". `ScrollLockedInputs` exempts
   Shift+wheel for exactly this reason — the two components are a matched pair.
2. `commitOnBlur: true` means the store lags the text until blur/Enter; the default
   (`false`) commits per keystroke. Some call sites depend on each.
3. `fmt.field(…, 6)` here vs `fmt.field(…, 2)` in NumberInput — two different display
   precisions for numerically similar fields.
4. `px-1`/`px-1.5`/`pr-4`/`pr-8` are inert under the cascade; the unit still needs the
   right padding because `.ui-input` gives 10px and the unit sits at `right-1`/`right-2`
   (4/8px), so a long value can run under it.
5. The steppers are `IconButton`s — their `!bg-*`/`hover:!bg-*`/`disabled:!bg-*`
   overrides exist because `.ui-button-secondary` (and the global disabled rule) would
   otherwise paint them.
6. The input wrapper is `w-0 min-w-0 flex-1` so the pair can shrink inside a grid
   column; without `w-0` the input's intrinsic width overflows.

---

## Surface: MiniStepperField — integer field with tiny chevrons

- **Source**: `src/components/ui/miniStepperField.tsx:1-60`
- **Renders**: `<div class="min-w-0" onWheel>` → `NumberInput` with
  `className="ui-input h-8 w-full min-w-0 pl-1.5 pr-5 text-xs text-center no-spinners
  !bg-[var(--surface-0)]"`, `min/max/step={1}`, `showStepper` default (true) → the
  NumberInput's own 12px chevrons.
- **Layout & styling**: 32px high, `.ui-input` padding (so `pl-1.5`/`pr-5` lose; the
  right 20px is where NumberInput's stepper column sits, at `right-0.5`, 16px wide).
- **States**: clamped display `clamp(round(value), min, max)`; wheel on the wrapper steps
  when not disabled; NumberInput's own chevrons are disabled at the bounds.
- **Data**: `value`, `onChange`, `min`, `max`, `disabled` (default `false`),
  `ariaLabel`.
- **Callbacks**: wheel (dominant axis, `preventDefault`) → `apply(clamped ± 1)`;
  typing/chevrons go through `NumberInput` → `apply` (round + clamp).
- **Text**: none of its own.
- **Port target**: proposed `ui/mini_stepper.rs`. Nothing exists.
- **Gotcha**: rounding happens in `apply`, so a typed `2.7` becomes `3`; the display is
  the *clamped* value, so typing `999` snaps to `max` on the next render. Source
  comment: "Integer-clamped and wheel-scrolls to step — off the dominant axis, since Shift+wheel comes in horizontally."

---

## Surface: CompactNumberField — labelled bare number box

- **Source**: `src/components/ui/compactNumberField.tsx:1-75`
- **Renders**: `<div class="min-w-0 {className}">` → `<label class="ui-meta block
  text-center" style="color: var(--text-muted)">{label}</label>` →
  `<div class="relative mt-1">` → `NumberInput showStepper={false}
  className="ui-input h-8 w-full min-w-0 pl-1.5 {unit ? 'pr-6' : 'pr-1.5'} text-xs
  text-center no-spinners !bg-[var(--surface-0)]"` → optional unit span
  `class="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-[10px]
  font-medium" style="color: var(--text-muted)"`.
- **Layout & styling**: label is `.ui-meta` (13px, `--indicator`) overridden to
  `--text-muted` and centred; field is 32px; unit at `right-2` (8px), 10px.
- **States**: default; `disabled` forwarded; label `title` becomes a native tooltip.
- **Data**: `label: string`, `value`, `onChange`, `min`, `max`, `step`, `unit?`,
  `disabled`, `ariaLabel` (required), `title?`, `className?`.
- **Callbacks**: `onChange` from NumberInput only.
- **Text**: `label` and `unit` from props; no catalog strings.
- **Port target**: proposed `ui/compact_number_field.rs`. Nothing exists.
- **Gotcha**: the visible label is **not** the accessible name (the `aria-label` prop
  is), so a port must not derive the a11y label from `label` — call sites pass
  distinct values.

---

## Surface: wheelStepDirection — the shared wheel-axis helper

- **Source**: `src/components/ui/wheelStepDirection.ts:1-31`
- **Renders**: nothing (function). Returns `1` (scroll up/left), `-1` (down/right), or
  `0` for no movement.
- **Implementation**: `const delta = Math.abs(event.deltaY) >= Math.abs(event.deltaX) ?
  event.deltaY : event.deltaX; if (!Number.isFinite(delta) || delta === 0) return 0;
  return delta < 0 ? 1 : -1;` — the tie (`>=`) goes to `deltaY`.
- **Why** (quote):
  > `* With Shift held, a mouse wheel is delivered as horizontal scroll — `deltaY` * arrives as 0 and the movement lands in `deltaX`. A trackpad keeps reporting * both axes, so a `deltaY`-only reading works there and silently ignores the * wheel, which is exactly the Shift+scroll gate in `ScrollLockedInputs`. * * This is not a macOS quirk: the swap happens inside the browser engine, so * Chrome, Edge and Firefox do it on Windows and Linux too.`
- **Covered by** `src/components/ui/__tests__/wheelStepDirection.test.ts`.
- **Port target**: proposed `ui/wheel.rs` (a free function). Nothing exists.

---

## Surface: ScrollLockedInputs — global wheel guard

- **Source**: `src/components/ui/ScrollLockedInputs.tsx:1-56`
- **Renders**: `null`; installs a `document`-level `wheel` listener in the capture phase
  with `{ passive: false }`.
- **Behaviour**: for targets matching
  `input[type="text"], input[type="search"], input[type="url"], input[type="email"],
  input[type="password"], input:not([type]), textarea, [contenteditable="true"]`,
  it calls `preventDefault()` **unless** `event.shiftKey` or
  `document.activeElement === event.target`.
- **States**: none.
- **Data**: none (mount once at the app root).
- **Callbacks**: none.
- **Text**: none. Source comment: "Number inputs are intentionally skipped; the ScrollableNumberField component uses its own non-passive wheel listener to step values and already calls preventDefault()."
- **Port target**: proposed `ui/scroll_lock.rs`. Nothing exists. In GPUI this becomes a
  policy on text fields' scroll handling rather than a DOM listener.

---

## Surface: LanguageSwitcher — locale picker

- **Source**: `src/components/ui/LanguageSwitcher.tsx:1-60`
- **Renders**: `SelectDropdown<Locale>` with options from `SUPPORTED_LOCALES` /
  `LOCALE_LABELS` (from `@/i18n`).
- **Layout & styling**: outer `className={`w-36 ${className}`}` (144px, on the
  `<label>`), `menuAlign="right"`, `leadingDisplay={<Languages className="w-4 h-4"/>}`
  (16px globe), `selectClassName="!text-[13px] !pr-9"` (the `!` pair is required to beat
  `.ui-input`).
- **States**: controlled when `onChange` is supplied; otherwise live-switches the app
  locale via `setLocale`.
- **Data**: `value?: Locale`, `onChange?: (locale) => void`, `className`. Falls back to
  `useLocale()`'s `locale`/`setLocale`.
- **Callbacks**: `onChange(locale)` when provided, else `setLocale(locale)`.
- **Text**: `ariaLabel`/`title` = ``_(msg`Language`)``; option labels are
  `LOCALE_LABELS` (endonyms, untranslated).
- **Port target**: proposed `ui/language_switcher.rs`. Nothing exists.
- **Gotcha**: the controlled form exists for the Settings modal's draft state — it must
  not switch the live locale until the draft is applied.

---

## Surface: Toast / ToastViewport — transient notices

- **Source**: `src/components/atoms/Toast.tsx:1-176` (doc: `docs/dev/notifications.md`)
- **Renders**: `Toast` → `<div class="border px-4 py-2 text-sm font-semibold shadow-lg
  rounded-full|rounded-lg">`; `ToastViewport` → `<div class="pointer-events-none fixed
  inset-x-0 flex justify-center px-3">` holding children.
- **Layout & styling**:
  - base: `border px-4 py-2 text-sm font-semibold shadow-lg` (16px/8px padding, 14px
    type, 600);
  - `shape`: `'pill'` → `rounded-full` (**default**), `'rounded'` → `rounded-lg`;
  - `pulse` → `motion-safe:animate-pulse`;
  - tone inline styles:
    | tone | borderColor | background | color |
    |---|---|---|---|
    | neutral | `var(--border-subtle)` | `var(--surface-0)` | `var(--text-strong)` |
    | info (default) | `color-mix(in srgb, #60a5fa, var(--border-subtle) 50%)` | `color-mix(in srgb, #60a5fa, var(--surface-0) 90%)` | `var(--text-strong)` |
    | success | `color-mix(in srgb, #22c55e, var(--border-subtle) 50%)` | `color-mix(in srgb, #22c55e, var(--surface-0) 90%)` | `var(--text-strong)` |
    | warning | `color-mix(in srgb, #f59e0b, var(--border-subtle) 50%)` | `color-mix(in srgb, #f59e0b, var(--surface-0) 90%)` | `var(--text-strong)` |
    | error | `color-mix(in srgb, #ef4444, var(--border-subtle) 50%)` | `color-mix(in srgb, #ef4444, var(--surface-0) 90%)` | `var(--text-strong)` |
  - `animated` inline: `opacity: shown ? 1 : 0`, `transform:
    translateY(${shown ? '0px' : `${enterOffsetPx}px`})`,
    `transition: 'opacity 220ms ease, transform 220ms ease'`;
  - viewport inline: `top: offset` for `top-center`, `bottom: offset` for
    `bottom-center`; `zIndex` when given.
- **States**: `visible` (default `true`) and, when `animated`, an `entered` latch set on
  the next animation frame — `shown = animated ? entered && visible : visible`. Source
  comment:
  > `* A transition needs a starting value to move away from, and a toast that is * mounted with `visible` already true has none: the browser paints it in its final * state, so `transition` has nothing to run. Running that toggle on the next * frame instead gives the element one paint in the off state first, which is what * makes the fade-and-rise happen on the way in.`
- **Data**: `tone` (default `'info'`), `shape` (default `'pill'`), `animated`
  (default `false`), `visible` (default `true`), `enterOffsetPx` (default `8`),
  `pulse` (default `false`), plus div props. Viewport: `position` (default
  `'bottom-center'`), `offset` (number → `Npx`; string used if non-empty; fallback
  `'1rem'` for top, `'1.25rem'` for bottom), `zIndex?`.
- **Callbacks**: none (`pointer-events-none` viewport; no dismiss affordance).
- **Text**: children only.
- **Port target**: proposed `ui/toast.rs`. Nothing exists.
- **Call sites** (for stacking reality): `NotificationStack.tsx` uses z 125/126 and
  offsets `'1.25rem'` / `'4.5rem'` to stack; `page.tsx` uses 125/127.

### Deliberate decisions and gotchas (Toast)

1. There is **no auto-dismiss timer and no close button** in this component — visibility
   is entirely the caller's state machine.
2. The viewport is `pointer-events-none`; a toast cannot be clicked unless a child
   re-enables pointer events.
3. Toasts are `position: fixed` and centred; there is no queue. Two visible toasts
   overlap unless the caller offsets them (see the `4.5rem` offsets).
4. The `animated` entrance needs the one-frame `entered` latch; a port that starts
   `visible` will show no animation.
5. `shape` defaults to `pill`; most call sites keep it, a few pass `rounded`.
6. Tone colours are hard-coded hexes mixed into `--surface-0`, not tokens — they do not
   adapt to the light theme beyond the base surface mix.

---

## Surface: ProgressBar

- **Source**: `src/components/atoms/ProgressBar.tsx:1-64`
- **Renders**: `<div role="progressbar" class="w-full overflow-hidden rounded-full
  {h-1.5|h-2|h-2.5}">` with either a sweeping `ui-loading-indicator` or a width-driven
  inner div.
- **Layout & styling**:
  - track: `w-full overflow-hidden rounded-full` + `xs: 'h-1.5'` (6px) /
    `sm: 'h-2'` (8px) / `md: 'h-2.5'` (10px, default); inline
    `background: 'color-mix(in srgb, var(--surface-2), black 20%)'`;
  - determinate fill: `class="h-full rounded-full transition-[width] duration-200"`,
    `width: ${percent}%`, `background: 'linear-gradient(90deg, var(--accent), #ff79c6)'`;
  - indeterminate: `indeterminate && 'ui-loading-track'` on the track, child
    `<div class="ui-loading-indicator" style="background: linear-gradient(90deg,
    var(--accent), #ff79c6)">` (32% wide, `1.15s linear infinite`,
    translateX −130% → 330%).
- **States**: determinate (`value`), indeterminate (`indeterminate`), unknown
  (`value === null` → renders 0% and omits `aria-valuenow`).
- **Data**: `value?: number | null` (default `0`), `size` (default `'md'`),
  `indeterminate` (default `false`), `ariaLabel?`. `percent = value === null ? 0 :
  Math.min(100, Math.max(0, value))`. ARIA: `role="progressbar"`, `aria-valuemin={0}`,
  `aria-valuemax={100}`, `aria-valuenow={indeterminate || value === null ? undefined :
  Math.round(percent)}`, `aria-label={ariaLabel}`.
- **Callbacks**: none.
- **Text**: none.
- **Port target**: proposed `ui/progress.rs`. Nothing exists.
- **Gotcha**: the gradient's second stop is a hard-coded `#ff79c6` (pink), not
  `--accent-hover`; it is the same in every theme.

---

## Surface: Spinner — the busy glyph

- **Source**: `src/components/atoms/Spinner.tsx:1-59`
- **Renders**: `<svg viewBox="0 0 24 24" class="shrink-0 {size}">` containing
  `<g><animateTransform type="rotate" from="0 12 12" to="360 12 12" dur="1s"
  repeatCount="indefinite"/><Icon width={24} height={24} aria-hidden/></g>`.
- **Layout & styling**: sizes `xs: 'h-3 w-3'` (12px), `sm: 'h-3.5 w-3.5'` (14px,
  default), `md: 'h-4 w-4'` (16px), `lg: 'h-5 w-5'` (20px), `xl: 'h-6 w-6'` (24px);
  `shrink-0`; default icon `Loader2` from lucide; colour inherits (`currentColor`).
- **States**: spinning only; `aria-label={label}` + `role={label ? 'status' : undefined}`
  + `aria-hidden={label ? undefined : true}`.
- **Data**: `size` (default `'sm'`), `label?`, `icon?` (a `LucideIcon`), plus SVG props.
- **Callbacks**: none.
- **Text**: `label` only.
- **Port target**: proposed `ui/spinner.rs`. Nothing exists.
- **Why SMIL** (quote):
  > `* The glyph turns inside its own SVG (SMIL) rather than with `animate-spin`. Under the * fractional webview zoom of the UI scale, the compositor rotates a CSS-spun layer about * a point that's off the glyph's centre, and the spinner wobbles.`

---

## Surface: IconChip + ICON_TONE_STYLES — icon tiles and status badges

- **Source**: `src/components/atoms/IconChip.tsx:1-72`, `src/components/atoms/iconTone.ts:1-36`
- **Renders**: `<span class="inline-flex shrink-0 items-center justify-center border
  {size}">` containing `children ?? <Icon class={iconSize} />`.
- **Layout & styling**:
  | size | classes | resolved |
  |---|---|---|
  | `xs` | `'h-4 w-4 rounded-[3px] text-[10px] font-bold leading-none'` | 16×16 badge |
  | `sm` (default) | `'h-5 w-5 rounded'` | 20×20 |
  | `md` | `'h-8 w-8 rounded-md'` | 32×32 |
  | `lg` | `'h-9 w-9 rounded-md'` | 36×36 |
  Icon sizes: `xs: ''` (icon keeps its own), `sm: 'h-3.5 w-3.5'`,
  `md: 'h-4 w-4'`, `lg: 'h-4 w-4'`.
  Tone style: `tone === 'inherit'` (default) →
  `{ borderColor: 'var(--border-subtle)', background: 'var(--surface-1)' }` (and **no**
  `color`, so the icon inherits); otherwise `ICON_TONE_STYLES[tone]`.
  `ICON_TONE_STYLES`:
  ```
  warning: { borderColor: 'color-mix(in srgb, #d97706, var(--border-subtle) 50%)',
             background:  'color-mix(in srgb, #d97706, var(--surface-1) 85%)',
             color: '#d97706' }
  danger:  { borderColor: 'color-mix(in srgb, #ef4444, var(--border-subtle) 55%)',
             background:  'color-mix(in srgb, #ef4444, var(--surface-1) 88%)',
             color: 'var(--danger)' }
  accent:  { borderColor: 'color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 45%)',
             background:  'color-mix(in srgb, var(--accent-secondary), var(--surface-1) 90%)',
             color: 'var(--accent-secondary)' }
  neutral: { borderColor: 'var(--border-subtle)', background: 'var(--surface-1)',
             color: 'var(--text-muted)' }
  ```
- **States**: static; `children` replaces the icon entirely (tick, number, glyph).
- **Data**: `icon?: LucideIcon`, `iconClassName?`, `size` (default `'sm'`), `tone`
  (default `'inherit'`), `className`, `style`, children.
- **Callbacks**: none (span).
- **Text**: none; `xs` carries `font-bold`/`leading-none` so a one-character child reads
  as a badge.
- **Port target**: proposed `ui/icon_chip.rs`. Nothing exists.
- **Source comments**:
  > `/** * `inherit` is the default and leaves the icon's colour alone: a chip in a row * inherits the row's colour, which is how the hand-written slots behaved. Use * `neutral` when the icon should be muted regardless of its surroundings. */`
  > `// These were inline in `StructuredDialogModal` and re-typed by hand in ~35 files (143 spans) before they moved here.`
- **Gotchas**: `rounded` = 4px and `rounded-md` = 6px; `xs` uses the odd
  `rounded-[3px]`; `border` (1px) is always on, including `inherit`.
  `ICON_TONE_STYLES` is also used directly by `IconButton surface`,
  `StructuredDialogModal`'s icon slot, and settings/modal icons — a change there is
  app-wide.

---

## Surface: SettingRow — label/description + trailing control

- **Source**: `src/components/atoms/SettingRow.tsx:1-140`
- **Renders**: `<Element>` (`div` or `label`) class
  `'flex items-center justify-between gap-3'` (+ bordered card) →
  left `<div class="min-w-0" (+'pl-1')>` with label and optional description →
  right `<div class="inline-flex items-center gap-2">`.
- **Layout & styling**:
  - root: `flex items-center justify-between gap-3`;
  - bordered: `'rounded-md border'` + `p-1.5` (compact, 6px) / `p-2` (comfortable,
    8px); inline `borderColor: 'var(--border-subtle)'` and
    `background: surface === 'raised' ? 'var(--surface-1)' : 'var(--surface-0)'`;
  - non-bordered: no border/background;
  - `disabled` → inline `opacity: 0.68`;
  - label: `'text-xs font-semibold'` (+ `'leading-tight'` when compact), inline
    `color: 'var(--text-strong)'`;
  - description: compact `'text-[11px] leading-tight'` / comfortable `'text-xs'`,
    inline `color: 'var(--text-muted)'`;
  - trailing: `inline-flex items-center gap-2`.
  - Source comment:
    > `* Both lines stay visible: the description carries the behaviour, so hiding it * behind a hover was worse than the space it saved. The compact density gets the * height back from the padding and the line-heights instead (50px to 40px for a * bordered row), and the row layout lives on the root element because rendered * as a `<label>` a wrapper inside it would leave the root inline, laying out * roughly twice as tall as the hand-written rows it replaces.`
- **States**: default; `bordered` sunken/raised; `disabled` (0.68 opacity, controls not
  disabled by the row — the caller disables them); description present/absent.
- **Data**: `label`, `description?`, `density` (default `'compact'`),
  `children` (control), `bordered` (default `false`), `surface` (default `'sunken'`),
  `as` (default `'div'`), `disabled` (default `false`), `className`, `style`.
- **Callbacks**: none (a `label` wrapper gives native label activation; the row itself
  is inert).
- **Text**: from props only.
- **Port target**: proposed `ui/setting_row.rs`. Nothing exists.
- **Gotchas**:
  1. `disabled` only dims — it does **not** disable the child control.
  2. `surface` is only meaningful with `bordered`; `sunken` on a `--surface-0` panel is
     invisible, which is why `raised` exists. `docs/dev/ui-atoms.md` records the
     measured failure: "`SettingRow` rendered as a `<label>` stayed inline while the flex
     row lived on a wrapper inside it, and an inline box holding a block child laid out
     78px tall against the 42px of the hand-written row it replaced."
  3. The label column's extra `pl-1` exists because text against the border reads cramped.
  4. `density` changes both padding and type sizes; "50px to 40px for a bordered row" is
     the stated intent.

---

## Surface: ColorSwatchInput — native colour picker, flattened

- **Source**: `src/components/atoms/ColorSwatchInput.tsx:1-45`
- **Renders**: `<input type="color">` with
  `className="shrink-0 cursor-pointer appearance-none overflow-hidden rounded border p-0
  [&::-webkit-color-swatch]:border-0 [&::-webkit-color-swatch-wrapper]:p-0
  [&::-moz-color-swatch]:border-0"` + caller `className`.
- **Layout & styling**: inline `borderColor: 'var(--border-subtle)'` and
  `boxShadow: 'inset 0 0 0 1px rgba(0, 0, 0, 0.28)'`. Size comes entirely from the
  caller (settings rows and overlay panels use different sizes).
- **States**: default; no hover/focus/disabled rules.
- **Data**: `value: string`, `onChange(value)`, `onBlur?`, `className?`, `title?`,
  `ariaLabel?`.
- **Callbacks**: `onChange(event.target.value)`; `onBlur` forwarded.
- **Text**: none.
- **Port target**: proposed `ui/color_swatch.rs`. Nothing exists.
- **Gotchas** (quote):
  > `* The native `input[type=color]` draws its own swatch inside a padded, bordered * wrapper, which reads as a box inside our rounded border. Every appearance * override here exists to flatten that: no UA appearance, no padding, no inner * swatch border.`
  > `// The swatch is the colour itself, so a pale resin on a pale surface would read // as a larger box with no edge. An inset hairline keeps every colour the same // apparent size.`
  The picker itself is a native OS dialog — not portable to GPUI without a native
  colour panel.

---

## Surface: FadeText — truncation with a soft edge

- **Source**: `src/components/atoms/FadeText.tsx:1-74`
- **Renders**: `<span>` with inline
  `display: block; minWidth: 0; overflow: hidden; whiteSpace: nowrap; textOverflow:
  clip` (+ mask only when overflowing).
- **Layout & styling**: mask =
  `linear-gradient(to right, #000 calc(100% - ${fadeWidth}px), transparent 100%)`,
  applied as `maskImage` and `WebkitMaskImage`; `fadeWidth` default `18`.
- **States**: fits (no mask), overflows (`scrollWidth > clientWidth + 1` → mask).
- **Data**: `children`, `className`, `style`, `fadeWidth`.
- **Callbacks**: none.
- **Port target**: proposed `ui/fade_text.rs`. Nothing exists.
- **Gotchas** (quote):
  > `* CSS has no `text-overflow: fade`, and a plain `mask-image` fades the last pixels * even when nothing overflowed, which dims short labels. So the truncation is inline * (no stylesheet rule to lose) and the mask is applied only once a measurement says * `scrollWidth` exceeds `clientWidth`. It re-measures on resize, on text change and * when web fonts settle, since a font swap changes the text width without changing * the box.`
  It re-measures via `ResizeObserver`, on `children` change, and on
  `document.fonts.ready`.

---

## Surface: AlertDiamondIcon — the one bespoke glyph

- **Source**: `src/components/atoms/AlertDiamondIcon.tsx:1-30`
- **Renders**: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor"
  strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">`
  with three paths: the diamond outline, `M12 8v5` (stem), `M12 16.5h.01` (dot).
- **Layout & styling**: only `className` (sizing/colour from the caller).
- **States**: none.
- **Data**: `className?`.
- **Callbacks**: none.
- **Text**: none.
- **Port target**: proposed `ui/alert_diamond.rs`. Nothing exists.
- **Why** (quote): "Lucide 0.563 has no diamond alert (neither `DiamondAlert` nor `AlertDiamond`), so this is the set's own `Diamond` outline with the alert stem and dot added: same 24 unit grid, 2px stroke, currentColor. Swap it for `DiamondAlert` if lucide ships one."

---

## Surface: PanelCollapseToggle — the chevron

- **Source**: `src/components/atoms/PanelCollapseToggle.tsx:1-45`
- **Renders**: `IconButton` (solid, default sizes) with a 12px chevron.
- **Layout & styling**: default IconButton (`.ui-button` + `ui-button-secondary`, no
  size class) containing `<Icon className="h-3 w-3" style={{ color: expanded ?
  'var(--accent)' : 'var(--text-muted)' }} />`; `ChevronDown` when expanded,
  `ChevronRight` when collapsed.
- **States**: expanded (down-chevron, accent), collapsed (right-chevron, muted), plus
  the IconButton's hover and `aria-expanded`.
- **Data**: `expanded`, `onToggle`, `className?`.
- **Callbacks**: `onClick={onToggle}`; `title` and `aria-label` both set to the label.
- **Text**: ``_(msg`Collapse card`)`` when expanded, ``_(msg`Expand card`)`` when
  collapsed.
- **Port target**: proposed `ui/panel_collapse_toggle.rs`. Nothing exists.
- **Gotchas** (quote):
  > `* The hand-written copies were `ui-button ui-button-secondary` with `!p-0.5` * alongside the `!p-2` the button atom already carried. Both are `!important`, * so the atom's `!p-2` won and the tile rendered 30x30 around a 12px chevron — * dropping the padding here would shrink it to 18x18. What the copies did not * have is the accessible name and `aria-expanded`, which live here.`
  It is **not** exported from `atoms/index.ts` because it reaches the Lingui catalog
  (macro) and would break the macro-free barrel test; import it directly. Comment in
  `index.ts`:
  > `// `PanelCollapseToggle` is deliberately NOT re-exported here. It resolves its // accessible name through the Lingui catalog, so it imports `@lingui/core/macro`, // and that module throws by design when it runs untransformed. The barrel is the // one atom module graph that unit tests import …`

---

## Surface: cn — the class joiner

- **Source**: `src/components/atoms/cn.ts:1-3`
- **Behaviour**: `parts.filter(Boolean).join(' ')`. No dedupe, no conflict resolution.
- **Port target**: nothing; in Rust this is a `Vec<&str>` rendered into a class string,
  or (better) a style struct. Do not port it as a "merge" function — it never merged.

---

## Surface: Slider — `.ui-range`, no component

- **Source**: `src/app/globals.css:547-561`; ~30 call sites, e.g.
  `src/components/controls/IslandsPanel.tsx:345`,
  `src/components/settings/CameraSettingsTab.tsx:151`,
  `src/components/settings/MeshSettingsTab.tsx:381`.
- **Renders**: `<input type="range" min=… max=… step=… value={draft} onChange={e =>
  set(parseFloat(e.target.value))} disabled={…} className="ui-range w-full" />`, usually
  under a label row.
- **Layout & styling**: `.ui-range` — 100% width, 8px height, `appearance: none`,
  `border-radius: 9999px`, `background: var(--surface-2)`, `accent-color: var(--accent)`,
  `cursor: pointer`. Thumb/track otherwise come from the UA (no `::-webkit-slider-thumb`
  rule for `.ui-range` — only the unused `.slider-vertical` family styles a thumb).
- **States**: default; disabled (`opacity: 0.5; cursor: not-allowed`); the filled portion
  is UA-drawn using `accent-color`.
- **Data**: native range props only.
- **Callbacks**: `onChange` parses with `parseFloat`; no debounce, no
  commit-on-release. Several call sites keep a `draft` state and apply on release
  elsewhere (area-specific).
- **Text**: none.
- **Port target**: proposed `ui/slider.rs`. Nothing exists.
- **Gotchas**:
  1. The two colour families in the file disagree: `.ui-range` is accent-pink and
     UA-thumbed; `.slider-vertical` is a `#ef4444` track with an 18px `#e5e7eb` thumb and
     **is referenced by nothing** in `src/`.
  2. `accent-color` alone means the visual thumb is whatever the webview draws; a GPUI
     slider must pick its own geometry. The closest in-app precedent is the
     `.slider-vertical` thumb: 18px circle, 2px white border,
     `0 0 0 2px rgba(0,0,0,0.15)` shadow, over an 8px rounded track.
  3. `parseFloat` on an empty string yields `NaN`; no call site guards it, so the value
     chain must already be numeric.

---

## Cross-cutting gotchas

1. **Two control heights.** `.ui-select` is 32px; a `SelectDropdown` trigger is 36px
   (`h-[36px]`); `.ui-input` has no height at all (callers set `h-8`). Panels force
   `.ui-select` to 32px inside `.ui-panel`. There is no single "control height" token.
2. **Overlay stacking** (all hard-coded): topbar blur `z-30`, transform toolbar `z-40`,
   modals `z-50+` per the CSS comment, `StructuredDialogModal` `z-[100]`,
   `BlockingOverlay` `z-[120]`, toasts `z-[125]`/`126`/`127`, `ContextMenu` `z-[130]`,
   flyout `z-[131]`, `SelectDropdown` menu and both tooltips `z-[9999]`.
3. **Escape ownership** goes through `useEscapeToClose`'s dialog stack
   (`src/hotkeys/useEscapeToClose.ts`): the top-most registration consumes the press; a
   registration with `undefined` swallows it. Modals register on open; popovers register
   only while open (through `useOutsideDismiss`), so a popover opened over a modal is the
   top-most target.
4. **`color-mix` is used heavily** (mix ratios quoted above). A Rust port must implement
   `color-mix(in srgb, A, B p%)` semantics (A at `p%` + B at `100−p%`, with `transparent`
   mixing affecting alpha) or precompute the resolved values per theme.
5. **i18n surface of the primitives** is tiny and specific: `NumberInput`
   (`Increase value`, `Decrease value`), `PanelCollapseToggle` (`Collapse card`,
   `Expand card`), `EditorContextMenu` (`Editor context menu` + the menu labels),
   `LanguageSwitcher` (`Language`). `StructuredDialogModal`'s `'Close dialog'` and
   `ScrollableNumberField`'s two default titles are **untranslated literals**.
   `docs/dev/ui-atoms.md`: "**Atoms take content, not translations.** A label, title or
   `aria-label` is the caller's".
6. **Reduced motion**: the CSS disables the topbar monitor, modal entry and the whole
   onboarding family. Toasts (`motion-safe:animate-pulse`), `Spinner` (SMIL) and
   `ProgressBar`'s sweep have **no** reduced-motion handling.
7. **No component uses a focus-visible ring.** Focus styling exists only as
   `.ui-input:focus`/`.ui-select:focus` border colour; buttons, toggles, segmented
   options, chips and icon buttons fall back to the UA outline (which `appearance: none`
   removes on the checkbox and colour swatch).
8. **The atoms barrel is macro-free by contract**
   (`src/components/atoms/__tests__/barrel.test.ts`): `PanelCollapseToggle` must stay out
   of it. Any ported module that needs translations at import time inherits that
   constraint.
9. **Dead surface area**: `.ui-button-ghost`, `.slider-vertical` and `.ui-number-input`
   have no live styling path (see "Dormant / dead selectors"). Do not port them as if
   they were a design system layer.

---

## Open questions

1. **Where does the GPUI UI crate live?** `gpui/` does not exist in this worktree;
   `docs/internal/gpui-backend-map.md` only proves `gpui/crates/gpui-backend`. Target
   module names above are proposals. The port's handoff doc
   (`docs/internal/gpui-port-handoff.md`) is referenced by the task but absent.
2. **Do we reproduce the cascade or the intended geometry?** The rendered values are
   listed per surface. If a later "fix" honours the inert `px-*`/`text-*` utilities, every
   button and field changes size. This needs a decision recorded before the first
   component lands.
3. **Native colour pickers.** `ColorSwatchInput` depends on the OS colour dialog. GPUI
   has no equivalent; the overlay/settings specs will need a custom picker or an `rfd`
   dialog.
4. **Webview-only styling hooks.** `::-webkit-color-swatch`,
   `::-webkit-slider-thumb`/`::-moz-range-thumb`, `::-webkit-inner-spin-button`,
   `accent-color`, `appearance: none` and `mask-image` are all webview-specific; the port
   must re-derive the visuals from the token values, not from these selectors.
5. **Zoom policy in GPUI.** The web app divides density between a fixed CSS base and
   `useUiScale`'s `userScale × screenFactor` native zoom (1 / 0.9 / 0.82 / 0.72 by
   `screen.availWidth`). Whether the GPUI window reproduces that factor table or uses a
   real `rem`-scale is undecided; every px in this spec assumes the zoom applied *after*
   layout.
6. **`ui-button-ghost` call sites.** Four `PluginStudioModal` buttons use a class with no
   rule and rely on inline styles for their active state. If the port needs that shape,
   the variant has to be defined (fill, border and active state) rather than copied.
