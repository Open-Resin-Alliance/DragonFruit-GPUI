# GPUI port spike

Throwaway worktree for the GPUI feasibility question: can DragonFruit's UI and
3D viewport be rebuilt on GPUI in Rust?

Branch `spike/gpui-feasibility`, based on `origin/dev` @ `3775742fb`
(2026-11-07).

`spike/gpui-probe` is a standalone crate (this repo has no workspace root).
All three probes run from it:

```sh
cd spike/gpui-probe
cargo run --bin gpui-probe   # shell + GPU bridge
cargo run --bin viewport     # zero-copy wgpu viewport
cargo run --bin bench --release
```

## Probe 1 — shell + GPU bridge

Opens a GPUI window, then on the first paint calls
`gpui_interop::attach(gpu.try_device::<WgpuRenderer>())` and prints what it
found: the window renderer, and whether a wgpu device is reachable on it.

Result on Windows: GPUI builds and runs; the default Direct3D 11 renderer
reports `NoDevice` to `gpui_interop` (expected — the cross-device hand-off is
the D3D12 path).

## Probe 2 — zero-copy viewport

Installs `WgpuRenderer` through `WindowOptions::with_renderer_factory`, renders
a wgpu triangle into a `WgpuSurface` on the renderer's own device, and
composites it with `paint_texture` — no CPU copy.

Result on Windows: **works**, 640×480, frames composited continuously. The
factory is ~60 lines (`src/lib.rs`). Linux gets this path by default. macOS
cannot: `gpui_wgpu`'s window renderer is `cfg`-excluded there, and
`gpui_interop`'s IOSurface backend is a stub.

## Probe 3 — throughput

A procedural UV sphere at print scale (2,877,600 triangles, 1,442,401
vertices), depth-tested, with an always-on-top axis gizmo, plus the
`dragonfruit-mesh-core` BVH over the same mesh for picking.

Measured on this Windows box (dev profile, dependencies at `opt-level = 2`),
2026-11-07:

| Metric | Result |
| --- | --- |
| Mesh generation | 47 ms |
| Vertex + index upload | 21 ms (34.6 MB + 34.5 MB) |
| BVH build | 636 ms |
| 10,000 ray queries | 28.9 ms (2.9 µs/ray) |
| 240 frames, GPU-waited | mean 1.27 ms (**787 fps**), p50 1.00 ms, p95 3.40 ms |

Caveats: one draw call, static mesh, procedural sphere. A real scene has
several models, instanced supports, overlays and a stencil pass, so treat this
as the ceiling, not the product number.

## Ecosystem note

The GPUI ecosystem is split across incompatible lines. This is the main
non-technical risk:

| Line | Component library | Zero-copy viewport | Notes |
| --- | --- | --- | --- |
| `gpui-pre` 0.3.11 (Zed snapshot) | `gpui-component` 0.9.0 / gpui-kit | **No seam** | `gpui-pre-seams` fork (0.3.7) adds `RenderImage::external` / `Window::with_renderer` |
| `bite-gpui` 1.25 | none (port your own) | `gpui_interop` 0.2.1: Linux dma-buf, Windows D3D12→D3D11, **macOS not implemented** | What these probes use |
| `gpui` 0.2.2 (crates.io) | — | — | Stale (2025-10-22) |
| `gpui-ce` / WGPUI forks | — | wgpu escape hatch | Fork maintenance |

No single line has both the mature component library and the external-texture
seam. A real port must either fork `gpui-pre` to add the seam, port
`gpui_interop` to `gpui-pre`, or build the component library on `bite-gpui`.

## What the spike does not answer

- macOS: no zero-copy path exists today; needs a GPUI Metal fork or upstream
  `paint_surface` work.
- Multi-model scenes, instanced supports, stencil cross-sections, HTML-style
  labels, and the 273-component UI port.
- Accessibility and the 2,504-string i18n catalog.
