# GPUI port — session handoff and coverage plan

State after this cycle, the decisions that must not be re-litigated, and the
systematic plan for finishing the 2D port at 1:1.

## Where the port stands

- `gpui/crates/gpui-core`: theme, icons, camera, stl, viewport/ (construct,
  render, cube, shaders, uniforms, targets, plate, pipeline, mesh), cube, state,
  support_state, empty_state, panel_layout.
- `gpui/crates/gpui-ui`: `widgets/` (button with `ButtonSpec::icon_tile`, card,
  controls, menu with flyouts, dialog, feedback, number_field, select, tooltip,
  icon, layout, color, input, text_field), `text_input.rs` (`EntityInputHandler`),
  `panel_fields`.
- `gpui/crates/gpui-supports`: primitives, instanced batches, `parse` (SUPP),
  `generate` (overhang classifier, candidates, density grid — stage 1).
- `gpui/crates/gpui-voxl`, `gpui-store` (settings/session/presets, atomic +
  debounced + versioned), `gpui-locale` (unit formatter verified against
  `Intl.NumberFormat`, `t!` with English msgid, PO extractor).
- `gpui/backend`: 131 Tauri commands mapped, 109 ported; excluded from the
  workspace until the `plugins/lumen` submodule is checked out.
- Binary: `src/app.rs` (entity, persistence, gizmo wiring, recents, camera),
  `src/chrome/` (app bar, rail, overlays, editor menu), `src/panels/`,
  `src/features/` (settings per tab, support panels, auto-support, profiles
  modal, picker, scene modals), `src/widgets/`, `src/gizmo/`.

Working end to end: VOXL and STL load with centred placement, wgpu viewport
with scene lighting and ACES, camera with real frame deltas and the source's
speeds, grab on the plate plane, transform gizmo (XYZ rotation, uniform scale,
diamond-only ring pick, draft/commit drags), view cube (atlas, hover, arrows,
home chip, ortho, MSAA), out-of-bounds stripes inside the mesh pass, support
batching and instanced draw, settings/presets/session persistence, printer and
material profiles modal, About, auto-support panel, text input.

## Decisions (do not reopen without evidence)

1. **Rust port, not a JS bridge.** GPUIX and incajs both run on upstream gpui,
   whose only GPU surface is macOS CoreVideo and whose Windows renderer is
   D3D11 — no way to present our wgpu texture. Our fork (`bite-gpui` +
   `gpui_interop`) is what makes the viewport possible.
2. **Values trace to `../src`.** Every radius, colour, spacing, duration and
   threshold cites a utility class, a CSS variable or a component prop. A
   literal that duplicates a token is a defect.
3. **`mix(a, b, w)` is `a*w + b*(1-w)`; CSS `color-mix(in srgb, A, B N%)` is
   `A*(1-N) + B*N`.** Call sites must be written in the CSS direction. This
   inversion has produced at least four visible bugs (hero gradient, name
   cards, footer card, mode pills).
4. **Assets live in two places.** Icons/fonts in `gpui/assets`; branding in
   `public/dragonfruit_assets`. A packaged binary must ship `public/`.
5. **Persistence is ours.** `%APPDATA%\org.openresinalliance.dragonfruit` with
   `settings.json`, `session.json`, `support-presets.json`,
   `auto-support-presets.json`. The last file is not reopened at boot; the camera
   is not restored (the source has neither).
6. **One owner per shared file per wave.** `app.rs` and `mod.rs` cause every
   collision we have had. Agents report the lines to add; the parent applies.
7. **Agents never run cargo.** One gate per wave: `cargo check`, `cargo test`,
   build, `--smoke`, `check:docs`, commit.

## The coverage plan

The port is now broad but uneven, and the remaining work is *coverage*, not
capability. Finish it with a ledger rather than by noticing things.

### Step 1 — extract the surface (mechanical)

`gpui/scripts/extract-surface.mjs` parses every `.tsx` (184 files under
`src/components`, plus `src/app/page.tsx`) with the TypeScript AST and emits one
row per user-facing thing:

| extracted | signal |
|---|---|
| buttons, menu entries, tabs | `onClick`/`onSelect` on JSX with text or `aria-label` |
| modals, panels, popovers | components inside a conditional branch |
| gated states | `disabled`, `hidden`, empty-state returns |
| tabs and routes | `activeTab === 'x'` |

Output `gpui/scripts/surface.json`: `{ id, source, kind, label, action }`.

### Step 2 — the ledger

`docs/internal/gpui-coverage.json`, one row per surface id:

```json
{ "id": "auto-support/preset-trigger",
  "source": "AutoSupportPanel.tsx:360",
  "rust": "panels/content.rs:supports_body",
  "status": "wired",
  "pixels": "pass 0.4%",
  "behaviour": "click → menu opens, selection changes" }
```

`absent` → `rendered` → `wired` → `verified`. Only `verified` counts, and it
needs **both** gates: a parity region under threshold *and* a scripted
interaction that changes entity state.

### Step 3 — the behaviour driver

The gate the port lacks, and the one that catches this session's recurring bug
class (dead `div`, gear with no destination, a field written and never read).
Run the app with an automation flag, feed a scripted event sequence, dump the
entity-state diff, assert on it. GPUI supplies synthetic keys and mouse.

### Step 4 — the army

One agent per area, disjoint Rust files, each closing ledger rows and citing
both gates:

| Wave | Agents | Areas |
|---|---|---|
| A | 1 | extractor + ledger + checker |
| B | 2 | driver + scenario format |
| C | 6 | `features/settings` tabs; `panels/`; `chrome/` (bar, rail, menus); `features/scene_modals`; `features/{transform,arrange,smoothing,organic,auto_support}`; profiles + picker + printing |
| D | 2 | printing preview and export; final parity sweep |

Every wave: one gate, one commit, ledger rows updated with evidence.

## Known open items

- `tool_panels::PanelStore.transform.auto_lift` should delegate to
  `auto_support_settings::{auto_lift, set_auto_lift}` — one flag, two surfaces
  today.
- `model::duplicate_material` names `{name} Copy` and keeps the colour; the
  source uses `{name} Custom` and drops it.
- `model::update_material` has no official/locked guard; only the UI enforces it.
- Support picking needs per-instance ids and a ray test before the studio can
  select what it draws.
- `--smoke` samples the first frame, so the pixel count is the pre-intro pose;
  framing checks need a post-intro sample.
- The auto-support document lives in the store root; promote it to a fourth
  `gpui-store` document when that crate is next touched.
