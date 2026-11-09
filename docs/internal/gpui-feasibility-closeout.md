# GPUI feasibility: closing out

**Verdict: not ready for DragonFruit's GUI today.** Not because GPUI is bad —
the viewport, camera and theme work are good — but because the port's contract
was "indistinguishable from the Tauri/React build", and the React build's
identity is Tailwind layout plus WebGL. Every one of those two mediums is a
hand-rebuild here, and five missing or surprising primitives turned each
rebuild into a bug that only surfaced under a user's eyes.

Branch: `spike/gpui-feasibility` (pushed to
`Open-Resin-Alliance/DragonFruit-GPUI`). 200 commits.

## What was built (and is worth keeping)

- **`gpui/` workspace**: `gpui-core` (theme, icons, camera, stl, viewport,
  scene, cube, state, …) and `gpui-ui` (ui primitives, panel fields); feature
  modules stay in the binary because `Render` is a foreign trait.
- **The 3D scene works**: wgpu mesh pass with the source's lighting
  (ambient/key/fill/hemisphere/headlight, roughness 0.55, ACES), plate, axis
  arrows, view cube as a real texture, orbit/pan/zoom with the source's
  time-based smoothing, camera intro/home/focus tweens.
- **2,130 UI surfaces extracted** from `src/**` into
  `docs/internal/gpui-coverage.json` with a machine-checked ledger:
  **603 verified (28.3 %)**, 1,508 absent, 19 dead.
- **A behaviour harness that is toolkit-independent**: `gpui/src/automation.rs`
  + `gpui/scripts/scenario.mjs` drive the real binary through clicks/keys/wheel
  and assert *state*, plus `rects` and per-card geometry
  (`height_ratio`, `center_error`, `parts_error`). Five scenarios, 64 steps.
  This caught a real Möller–Trumbore bug (barycentric `v` used as the ray
  parameter, so hits behind the origin counted) and proved the multi-model
  import, selection and modal-height fixes.
- **Generators**: `scripts/generate-gpui-theme.py` (theme from `globals.css`),
  `scripts/generate-gpui-icons.mjs` (Lucide icons), and the surface extractor
  with 12 tests.

## Why the port does not converge

Measured against the React source, not against expectations:

| Missing or surprising primitive | Consequence observed |
|---|---|
| Hit-testing collects **every** hitbox under the pointer and only stops at `BlockMouse` | A full-window canvas is "hovered" under panels, so **any** button click deselected the model until every layer called `occlude()` |
| A scroller's content is the union of its **direct children's** boxes | The flex-1/`min-h-full` body reported its own height, `scroll_max` collapsed to 0, and the material pane was **unreachable** |
| No portal | Row tooltips had to hang *upward*; `deferred` could not work because `group_hover` resolves before the group hitbox registers |
| `paint_image` is axis-aligned; `RenderImage::new` needs `Frame`, which is `pub(crate)` | The view cube needed a bespoke wgpu pass |
| No CSS transitions, `backdrop-blur`, or styled scrollbars | Modal enter animation, blur and scrollbars are permanent deviations |
| `svg().external_path` is a silent no-op without `text_color`, and renders a **monochrome alpha mask** | The About page's wordmark/icon painted nothing; fixed by `img()` + `SvgRenderer`, at the cost of a false claim sitting in the code for two rounds |
| No hover callback (`on_hover` is a `gpui-pre` API) | Marquee pause implemented by hit-testing the pointer against prepainted bounds |
| `App::http_client` is `NullHttpClient` | Sponsor avatars needed a new HTTP dependency (`ureq`) |
| Edition-2024 RPIT captures borrows; rustfmt on a fixed entry point reflows whole modules | Working around both costs a review cycle each time |

The rate matters more than any single row: after the whole effort, **28.3 %** of
surfaces are verified, and the ones that remain (modals 281, settings 924 with
504 absent, panels 98 absent) are the *same* kind of hand-porting — each row a
Tailwind class list mapped by hand, each needing a scenario to be trusted.

## What would change the verdict

Any of these would move the port from "rebuild everything" toward "translate":

1. A CSS-like layout story with real `min-height`/scroll-overflow semantics, or
   an upstream fix for scroller content measurement.
2. Portal/deferred painting for popovers, or a stable workaround documented
   upstream.
3. Arbitrary-transform image painting, or `RenderImage` constructible outside
   the crate.
4. A hover/focus callback API on the stable line, not only `gpui-pre`.
5. An HTTP client seam in `App` (or `img(uri)` fetching).

Until then, the honest comparison is: a GPUI port is a from-scratch rewrite of
the UI *and* the scene, with a permanently different feel in motion, blur and
scrollbars.

## Re-evaluation path: GPUIX

[GPUIX](https://gpuix.dev) (React/Solid bindings for GPUI, Node-API native
bindings, no webview) was assessed after this closure and is the most promising
follow-up, because it removes most of the blockers above rather than the
symptoms:

| Blocker in this port | GPUIX (`@gpuix/react` 0.11.0) |
|---|---|
| no portal; tooltips inverted, `deferred` broken | `Dialog.Portal`, `createPortal`, `<anchored>` layers, per-window Escape layer stack |
| scroller content = direct children's boxes | `overflow: "scroll"`, `onScrollChange`, native `scrollbarColor`/`scrollbarWidth` |
| no hover callback on the stable line | nested `hover` / `active` / `focusVisible` styles |
| hit-testing marks a full-window canvas hovered | `getElementsAt(x, y)`, `pointerEvents: "none"`, modal dialogs block clicks behind |
| axis-aligned images; `RenderImage` needs `pub(crate) Frame` | `setImagePixels(w, h, buffer, { format: 'bgra' })` |

It also keeps the existing React tree, hooks and state stores, and this
research's artefacts become its inputs: `docs/internal/gpui-coverage.json`
already carries 2,130 surfaces *with their className strings*, so a
Tailwind-to-style-object codemod starts from a real corpus with a checker.

The blocking unknown is the **3D scene**: three.js/R3F needs WebGL and a DOM
canvas, GPUIX has neither, and `setImagePixels` is a CPU upload — workable for
content, not for a 60 fps orbit viewport at resolution. The fallback is the
wgpu pass this branch already contains, bridged through Node-API. Other risks:
pre-1.0 API churn (adapter and native must be pinned to the same exact
version) and Bun (~82 MB) or Hermes (~34 MB) plus a `.node` sidecar to ship.

Re-evaluate when GPUIX exposes a GPU surface or a three.js-compatible canvas.

## What to do with this branch

Keep it as research: the coverage ledger, the extractor, the scenario harness
and the viewport maths are the reusable parts and are independent of the
final toolkit choice. Do not merge it into `dev`.
