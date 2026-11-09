# GUI renderer options while staying on Tauri

Status: design note, nothing implemented.

Question: can we keep Tauri but stop using the WebView to render the React GUI?

## What Tauri actually is

Tauri v2 is `tao` (windows) + `wry` (a thin wrapper over the *system* webview:
WebView2 on Windows, WKWebView on macOS, webkit2gtk on Linux) + an IPC layer and
plugin host. There is no "choose your renderer" switch. Replacing the webview
means replacing `wry`, and at that point the remaining value is the windowing
layer and the Rust command surface — i.e. it is no longer Tauri, it is a native
app with an embedded browser engine, or a native app with no browser at all.

That distinction decides the whole answer, because the options below are not
"Tauri settings", they are different applications.

## The options, honestly

| Option | Keeps React? | Keeps CSS/Tailwind? | Keeps three.js? | Real cost |
|---|---|---|---|---|
| **System webview** (today) | yes | yes | yes | none — and the reported slowness is transport, not DOM |
| **Embedded Chromium** (CEF/Electron-style) | yes | yes | yes | strictly more memory and a bigger install; not faster than WebView2, which is already Chromium on Windows |
| **GPUIX** (React/Solid on GPUI) | yes | no — inline style objects | no | a JSX transform plus a class-strings → style-object codemod; the scene must move to a native surface |
| **React Native Desktop** (AppKit / WinAppSDK) | yes | no | no | a different component model, no DOM, and a 3D view needs native interop |
| **Servo/Verso in a Tauri-like shell** | yes | mostly | maybe | experimental; not a shipping dependency for a product |

Only two of these are worth considering for this app: **GPUIX** and **the hybrid
below**. The Chromium options are not faster, and React Native Desktop is the
largest rewrite of the set.

## The hybrid: keep React + CSS, move only the heavy painting

The webview is slow at exactly one thing here: drawing a scene with millions of
triangles and per-frame instanced buffers. It is not slow at layouts, panels,
forms or text. So the useful split is:

- **React + Tailwind stays in the webview** — every panel, modal, dialog and
  form keeps its current code;
- **the 3D viewport moves to a wgpu surface** owned by the Rust process, composited
  under a transparent webview region (see `mesh-ipc-transport.md`, "The
  alternative architecture").

This is the only arrangement that removes mesh transfer *and* keeps the GUI
codebase. Its price is compositing, not porting: see the scene-graph cost in the
same document (149 scene files, 56 instanced usages, 6 custom shaders) — the
hybrid still has to port those, because they live in the scene.

## What to measure before choosing anything

"WebView is slow" is currently an inference, not a measurement. Three numbers
separate the cases:

1. **Transport**: bytes and milliseconds for one mesh round trip (see P0 in
   `mesh-ipc-transport.md`).
2. **DOM cost**: a React profile of the heaviest interaction (opening the
   printer profiles modal, switching tabs in settings) — long tasks, layout
   count, style recalc time.
3. **GPU cost**: frame time of the scene with supports and the hollow preview
   visible, with the mesh already loaded.

If (1) dominates, no renderer change helps: the bytes still have to cross. If
(3) dominates, the hybrid is the answer. If (2) dominates, the fix is inside the
webview — production build rather than `devUrl`, virtualised lists, fewer
per-frame React updates — and swapping the engine would be a rewrite bought for
a problem CSS scheduling caused.

## Levers inside the current webview, for completeness

- ship the exported build, never the Next dev server (`devUrl` is 3005 in
  `tauri.conf.json`) — dev-mode React and unbundled modules are several times
  slower;
- keep the window non-transparent (`transparent: false` today) — transparent
  windows give up compositing fast paths on some platforms;
- run the scene in an `OffscreenCanvas` inside a worker so React re-renders
  never contend with frame production;
- avoid re-rendering the whole tree on pointer moves; the scene should own its
  own animation loop, and panel state should not feed it.

## Recommendation

Do not replace the webview. The cheapest order that actually addresses the
reported problem is:

1. measure the three numbers above;
2. shrink the mesh payload (indexed, `u16`, zstd) — one order of magnitude for
   a week of work;
3. serve meshes over a `mesh://` handle instead of `invoke`;
4. only then consider the wgpu surface, and only for the viewport.

GPUIX stays the interesting option if the goal ever becomes "no webview at
all" — it keeps React, which is the expensive part of any rewrite here.
