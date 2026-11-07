# GPUI port spike

Throwaway worktree for the GPUI feasibility question: can DragonFruit's UI and
3D viewport be rebuilt on GPUI in Rust?

Branch `spike/gpui-feasibility`, based on `origin/dev` @ `3775742fb`
(2026-11-07).

## Probe 1 — shell + GPU bridge

`spike/gpui-probe` is a standalone crate (no workspace root in this repo).

```sh
cd spike/gpui-probe
cargo run
```

It opens a GPUI window, then on the first paint calls
`gpui_interop::attach(gpu.try_device::<WgpuRenderer>())` and prints what it
found: the window renderer, and whether a wgpu device is reachable on it.

What it proves:

1. GPUI builds and runs on this machine.
2. `gpui_interop` attaches inside `gpu_canvas`.
3. A wgpu producer can reach a device on the window's GPU.

## Ecosystem note

The GPUI ecosystem is split across incompatible lines. This is the main
non-technical risk:

| Line | Component library | Zero-copy viewport | Notes |
| --- | --- | --- | --- |
| `gpui-pre` 0.3.11 (Zed snapshot) | `gpui-component` 0.9.0 / gpui-kit | **No seam** | `gpui-pre-seams` fork (0.3.7) adds `RenderImage::external` / `Window::with_renderer` |
| `bite-gpui` 1.25 | none (port your own) | `gpui_interop` 0.2.1: Linux dma-buf, Windows D3D12→D3D11, **macOS not implemented** | What this probe uses |
| `gpui` 0.2.2 (crates.io) | — | — | Stale (2025-10-22) |
| `gpui-ce` / WGPUI forks | — | wgpu escape hatch | Fork maintenance |

No single line has both the mature component library and the external-texture
seam. A real port must either fork `gpui-pre` to add the seam, port
`gpui_interop` to `gpui-pre`, or build the component library on `bite-gpui`.

The macOS gap is the one to resolve before committing: DragonFruit ships a
universal macOS build, `gpui_wgpu`'s window renderer is `cfg`-excluded on macOS,
and `gpui_interop`'s IOSurface backend is a stub.

## Probe 2 — viewport (next)

The Windows path is a Direct3D 12 producer handing a shared texture to GPUI's
Direct3D 11 renderer (`SharedSurface` + `Fence`); the reference is
`gpui_interop`'s `tests/cross_device_surface.rs`. The Linux path is a Vulkan
dma-buf (`WgpuSharedBuffer`), reference `examples/interop_wgpu`.

Acceptance for the spike: a 2.8M-triangle mesh at 60 fps, zero CPU copies,
picking and a gizmo, on Windows, macOS and Linux.
