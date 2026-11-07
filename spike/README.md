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
| 240 frames, GPU-waited | mean 1.1–1.3 ms (**790–890 fps**), p50 0.9–1.0 ms, p95 2.9–3.4 ms |

Caveats: one draw call, static mesh, procedural sphere. A real scene has
several models, instanced supports, overlays and a stencil pass, so treat this
as the ceiling, not the product number.

## Probe 4 — scene (models, instancing, stencil cap)

```sh
cargo run --bin scene
```

Four models × 720k triangles (2.88M total) from one vertex/index buffer with
per-instance matrices, 20,000 instanced support cylinders, and the
cross-section implemented the way the app does it: two stencil passes over the
clipped models, then a cap quad where the stencil is non-zero.

Measured on this Windows box (dev profile), 2026-11-07:

| Metric | Result |
| --- | --- |
| Setup (mesh + buffers + pipelines) | 50 ms |
| 240 frames, GPU-waited | mean 1.83 ms (**547 fps**), p50 1.69 ms, p95 3.27 ms |
| Readback | cap 76,792 px · model 154,767 px · supports 30,161 px |

The readback is the correctness check: a frame time says nothing about whether
the cap drew, so the probe renders into a `COPY_SRC` texture and counts the
pixels each layer should have produced.

## Probe 5 — interactive overlay

```sh
cargo run --bin scene -- --interactive
```

The same scene, plus the hybrid UI the app needs: a GPUI label positioned at a
projected 3D point, a screen-space axis gizmo drawn as GPUI elements over the
canvas, and drag-to-rotate routed from the canvas to the scene. The overlay is
ordinary GPUI layout; only the projection math is shared with the renderer.

Runs without crashing; the overlay's pixels are not part of the readback, so
this one is a smoke test, not a measured result.

## macOS: a small patch, not a redesign

`gpui_apple`'s `paint_surface` already imports a `CVPixelBuffer` through
`CVMetalTextureCache::create_texture_from_image` — that is zero-copy — but it
asserts the format is NV12 (`kCVPixelFormatType_420YpCbCr8BiPlanarFullRange`).
A BGRA path is a known, bounded patch: issue #64926 has a working fork diff of
+92/−39 across `metal_renderer.rs` and `shaders.metal`. So macOS needs a GPUI
fork or an upstream change, not a new renderer.

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

- macOS on real hardware: the path is a bounded `paint_surface` BGRA patch, but
  it has not been built or run here.
- HTML-style labels positioned in 3D beyond the smoke test, the full
  screen-space gizmo, and the 273-component UI port.
- Accessibility and the 2,504-string i18n catalog.
