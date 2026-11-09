# Mesh transport across Tauri v2 IPC

Status: design note, nothing here is implemented.

Question: the GUI is fast everywhere except moving mesh data between the webview
and the Rust core. Can Tauri v2 give us shared memory?

Short answer: **not shared memory, but we can get most of the way there**, and
the biggest wins do not need it.

## Where the bytes go today

Both directions already use raw binary rather than JSON:

- frontend → Rust: `invoke('stage_mesh_binary_set', bytes, { headers:
  { 'Content-Type': 'application/octet-stream' } })`
  (`src/utils/tauriMeshBridge.ts`), received as `InvokeBody::Raw`
  (`src-tauri/src/main.rs`);
- Rust → frontend: `tauri::ipc::Response::new(Vec<u8>)`
  (`mesh_repair_read_positions`, `mesh_punch_read_positions`, …).

So "make it binary" is done. What remains is **copies and encoding**, and the
two are platform-dependent — verified in `tauri-2.11.6`:

| Path | Windows / Linux | macOS / iOS |
|---|---|---|
| `Response` with raw bytes | `Channel::send(Raw)` → JS fetches it back through `plugin:__TAURI_CHANNEL__\|fetch` (`tauri-2.11.6/src/ipc/protocol.rs`, the `Raw` arm; `tauri-2.11.6/src/ipc/channel.rs`, `FETCH_CHANNEL_DATA_COMMAND`) — binary, chunked | `format_result(...)` → the `Vec<u8>` is **serialised as a JSON array of numbers inside an `eval` string** (`tauri-2.11.6/src/ipc/protocol.rs`, the `Raw` fallback; `tauri-2.11.6/src/ipc/format_callback.rs`) |

That asymmetry matters: a 170k-triangle mesh is ~6.1 MB of positions as `f32`;
on the macOS path it becomes ~20 MB of JSON text parsed by the webview, with a
`MAX_JSON_STR_LEN` ceiling of `2^30 - 2` characters (`format_callback.rs`, `MAX_JSON_STR_LEN`).
If DragonFruit ships on macOS, binary mesh reads are already the worst case
there and no amount of Windows tuning will show it.

The copies on Windows today, for one `stage_mesh_binary_set` of 6 MB:

1. JS builds the body (the `Uint8Array` view itself is free);
2. WebView2 copies the request body into the WebView2 process;
3. Tauri copies it into `InvokeBody::Raw(Vec<u8>)`;
4. our handler copies it into the staging mesh.

And any result that comes back repeats 2–4 in reverse.

## Why "shared memory" is not directly available

- WebView2 and WKWebView run the page in a **separate process**. Their JS heaps
  are not mapped into our process, so there is no pointer we can hand out.
- `SharedArrayBuffer` is created by JS and cannot be addressed from Rust: there
  is no wry/tauri API that exposes the backing memory of a JS `SAB`. Passing one
  over IPC is not possible either — the IPC transports move copies.
- `SAB` also needs cross-origin isolation (`COOP: same-origin` +
  `COEP: require-corp`) on the app origin. That is achievable in Tauri v2
  (custom protocol response headers / `on_web_resource_request`), but it buys
  **JS ↔ Worker** sharing, not Rust ↔ JS.

The only way to get true zero-copy both ways would be a custom native addon loaded
into the webview process (N-API/WASM that owns its memory), which is a much
bigger architectural change than the transport itself.

## What to do instead, in the order that pays

### P0 — measure, per platform

No optimisation without numbers. Instrument one round trip each way with a fixed
mesh, and record: bytes on the wire, wall time, and peak RSS in both processes.
On Windows also run the same measurement with the mesh 10× smaller to separate
fixed overhead from per-byte cost. This is the only way to know whether the cost
is copies (fix P1/P3) or payload size (fix P2).

### P1 — serve the mesh from a memory-mapped file over a custom protocol

Replace the invoke-with-bytes handoff with a **buffer handle**:

- Rust keeps meshes in an arena and exposes `mesh://local/<buffer-id>` through
  `register_uri_scheme_protocol` (or `register_asynchronous_uri_scheme_protocol`
  when the body must be produced off the main thread);
- the body is written straight from the mmap / arena slice with
  `Content-Length`, and `Range` is honoured for partial reads;
- JS does `const buf = await (await fetch('mesh://local/42')).arrayBuffer()`
  and uploads it with `gl.bufferData`.

Why this is the biggest structural win:

- one copy instead of three (webview → JS), and none of it through an `eval`
  string on macOS;
- no 2 GB JSON ceiling, no per-call `invoke` overhead, and streaming for free;
- the Rust side can keep the authoritative copy in a memmap and never re-encode
  it.

Cost: roughly one command (`mesh_open` → id), one protocol handler, and a
`Vec<u8>`-free response path. Keep `invoke` for control-plane calls only.

### P2 — make the payload smaller (independent of P1, stacking)

Current transport is an expanded triangle soup of `f32` positions (3× the vertex
count of indexed geometry), and it also carries nothing the shader could derive:

| Change | Approx. bytes |
|---|---|
| indexed instead of soup | ÷3 |
| positions quantised to `u16` + per-mesh scale/offset | ÷2 |
| normals dropped, recomputed in the shader | −33 % on a soup that carries them |
| zstd on the buffer | ÷2–4 |

Together that is 10–20× on typical meshes, which changes what "slow IPC" even
means. Quantisation needs a float-safe decode path in JS (`u16 → f32` on the
GPU is cheapest: an integer attribute plus a uniform scale/offset).

### P3 — stop sending meshes back at all

The expensive pattern is a round trip per operation (repair, punch, hollow):
upload the whole mesh, run an op, download the whole mesh. Instead:

- the Rust core owns the mesh by id;
- an operation returns a **new id** (or the same id if mutated in place) plus
  statistics (triangle count, bounds, changed region);
- the frontend re-fetches only when it must draw, via P1, and can do so lazily.

This is also what makes progressive rendering possible: fetch the bounding box
and a coarse LOD first, the full buffer when the camera settles.

### P4 — `SharedArrayBuffer` only where it helps

With COOP/COEP headers on the app origin, move decode and de-quantisation into a
worker and hand the buffer over with `postMessage(…, [sab])` instead of copying
per chunk. This is a JavaScript-side optimisation; it does not remove the
webview boundary copy, so it comes last.

## Risks and unknowns

- Frame budget vs. big files: a 100 MB scene under `Range` streaming needs
  care in the renderer (upload in slices, keep the frame loop alive).
- Custom protocol responses are cached by the webview unless headers say
  otherwise; a stale `mesh://` body after a mutation would be a silent
  correctness bug — use a fresh id per version, and `Cache-Control: no-store`.
- Security: the handler must only serve ids from the arena, never a caller
  supplied path.
- macOS: P1 removes the JSON-array path entirely, which is the difference
  between "slow" and "broken" on large meshes.

## Recommendation

Do P0 and P2 first — both are local, testable, and P2 is the largest single
factor. Then P1 to remove the remaining copies and the platform asymmetry. P3
is the architectural change worth planning next; P4 is a refinement.

## The alternative architecture: native viewport, HTML shell

If the goal is that **no mesh ever crosses IPC**, the way to get it is to stop
drawing the scene in the webview: render it with wgpu into a surface owned by
the Rust process, and keep the Next.js UI as the shell around it.

Tauri v2 exposes what is needed to create that surface —
`WebviewWindow::hwnd()` (`tauri-2.11.6/src/webview/webview_window.rs`) and
`WebviewWindow::ns_window()` — so a `wgpu::Surface` over a child window or
`sibling` `NSView` is straightforward. The hard part is not the surface,
it is **composition and input**, because z-order is per window, not per region.

Two variants, and they are very different jobs:

| | Surface **beneath** a transparent webview | Surface **above** the webview |
|---|---|---|
| HTML overlays (view cube, chips, sliders) | stay HTML — they paint over the hole | must be drawn in wgpu, like the GPUI port |
| Webview | transparent (WebView2 `DefaultBackgroundColor` transparent, WKWebView `isOpaque = false`) | opaque |
| Input over the viewport | the webview owns the pixels, so DOM events must be forwarded to Rust (camera deltas, picking — small, no mesh) | the surface receives input directly |
| Hard part | transparent-webview compositing and resize/DPI sync; corruption on resize is a known failure mode | re-implementing every viewport overlay natively |

Platform reality: macOS is the friendliest (opaque-off WKWebView plus a Metal
layer sibling is a well-trodden arrangement); Windows generally works with
WebView2 transparency but is where resize artifacts show up; **Linux
(webkit2gtk) is the likely blocker** — transparent webviews there are not
reliable, so this architecture would need a fallback for Linux.

What it costs, honestly:

- the scene must be drawn by wgpu, not three.js — this branch already contains
  that renderer (`gpui/crates/gpui-core/src/viewport.rs`: mesh pass, plate,
  axes, view cube, camera), so the asset exists, but it must be decoupled from
  GPUI;
- one more surface to keep in sync with the DOM viewport rect: position, size,
  DPR, occlusion (a modal open must hide the surface or it paints over it);
- input forwarding for camera gestures, or native overlays if the surface sits
  on top;
- window-level transparency: the Tauri window itself must be transparent, which
  changes how every panel draws.

The payoff: **zero mesh traffic**, no copies, 60 fps on meshes that today
cannot cross at all, and slicing results stay where the engine is.

### Relationship to the other proposals

This does not replace P0–P2; it replaces P1 and P3. P2 (smaller payload) is
still worth doing because the *slicing preview* and layer views may stay in the
webview. And P0 decides whether this is worth it: if a 6 MB transfer costs 8 ms
after P2, a native surface buys little; if it costs 400 ms and stalls the frame
loop, it is the only real answer.

## Does WebGPU in three.js change this?

Partly — and it changes the **rendering** half, not the **transfer** half.

What it does not do: WebGPU cannot import memory owned by our process. A
`GPUBuffer` is still filled from a JS `ArrayBuffer`, so the chain stays
IPC copy → JS heap → GPU upload. The native surface remains the only
architecture that removes the boundary copies.

What it improves:

- uploads no longer stall the frame the way `WebGL2.gl.bufferData` can; WebGPU
  buffers are staged asynchronously, so a 6 MB upload overlaps rendering
  instead of blocking a frame;
- compute shaders let the **payload shrink further**: send `u16` positions and
  normals the GPU reconstructs, instead of decoding them in JS (this multiplies
  with P2's quantisation and zstd gains);
- multi-draw and instancing overhead is lower, which matters for a scene that
  draws many models and supports.

Platform support (`navigator.gpu`, per the W3C implementation-status page):

| Tauri target | WebView | WebGPU |
|---|---|---|
| Windows | WebView2 (Chromium, evergreen) | available (Chromium 113+) |
| macOS | system WKWebView | Safari 26+ only, i.e. macOS 26+; older macOS falls back |
| Linux | webkit2gtk | not shipped, falls back |

three.js's `WebGPURenderer` falls back to WebGL 2, so the gap is a performance
tier rather than a broken build — but the porting cost is real for this app:
`src/features/shaders/mesh/*` are custom GLSL `ShaderMaterial`s, which have to
become TSL to run on the WebGPU backend.

Verdict: worth a spike **after P0/P2**, because it is cheap to evaluate and it
makes the cheap payloads cheaper to consume. It is not a substitute for the
transport work, and it does not remove the reason to consider a native surface.

## What the native surface actually costs: the whole scene graph

The transfer is only the *mesh*. Everything else in the scene is also three.js,
and a wgpu surface has to draw it too. Counted in this tree:

| | Count |
|---|---|
| files under `src/components/scene` + `src/features/scene` | 149 |
| files carrying R3F objects or `useFrame` | 59 |
| instanced-mesh usages | 56 |
| custom `ShaderMaterial`s | 6 |

The instancing is not incidental — it is the support system:

- `src/supports/SupportPrimitives/Roots/InstancedRootsGroup.tsx`
- `.../ContactCone/InstancedContactConeGroup.tsx`
- `.../Shaft/InstancedShaftGroup.tsx`
- `.../Joint/InstancedJointGroup.tsx`

plus `HollowVoxelPreview.tsx` (volume preview), `IslandOverlay.tsx`,
`MeshSmoothingBrushCursor.tsx` and the canvas overlays. Each instanced group
keeps its own per-instance attribute buffers, so moving to wgpu re-creates the
same "how do I get this data into the GPU" problem *inside* the renderer, on a
per-frame update path rather than a one-off load.

For calibration, the GPUI spike's renderer — mesh, plate, axes, view cube,
gizmo, camera — is 4,599 lines across `gpui/crates/gpui-core/src/viewport/`,
and it does not include supports, hollowing preview, islands, or the smoothing
brush.

So the native surface is not "move the viewport", it is "re-implement the scene
subsystem". That is the right call only if the webview's rendering is itself
the limit, not just the transport.

## Relationship to PR #972 (`sin/lazy-load-ipc`)

That branch implements the "Rust owns the mesh" half of P3, and its own smoke
table is the evidence: after import finalises, preparation and slicing upload
**zero raw mesh chunks** (`Mesh upload sessions 1`, `Raw mesh chunk bytes
uploaded 0`), because imported sources are retained natively and later sends are
references plus transforms. It adds content-addressed 4 MiB chunk dedupe (only
missing chunks cross), a retained last-slice snapshot, a prepared-modifier cache
and a leased cache for large job values, refreshed by a scene coordinator after
15 s of quiet plus idle.

What that does and does not settle, measured against the analysis above:

- **Fixed**: repeated whole-mesh uploads per slice — the largest *repeat* cost,
  and the reason "slice, tweak, slice" felt slow.
- **Not fixed: the first download.** Import still returns the full mesh through
  `Response::new(bytes)` (`load_mesh_bytes` in `src-tauri/src/mesh_repair.rs`).
  In `tauri-2.11.6` that path is the channel/fetch on Windows and Linux but the
  `eval` + JSON-array path on macOS/iOS, so the one transfer that must happen
  every import is still the platform-asymmetric one.
- **Not fixed: copies.** Chunk transport is still IPC bytes; the webview
  boundary copy remains.
- **Not fixed: payload size.** No indexing/quantisation/zstd, so a single cold
  transfer is the same number of bytes as before.
- **Risk**: draft PR, 68 files, desktop `cargo check` unverified, no Windows
  timings, no real-scene export, no final-layer parity. The macOS build check
  passed.

So it removes the repeated cost and leaves the cold path alone. The remaining
work in this document is not in conflict with it: P0 still needed to know what a
cold transfer costs, and the macOS response encoding is worth its own look
because it is the direction that branch did not touch.
