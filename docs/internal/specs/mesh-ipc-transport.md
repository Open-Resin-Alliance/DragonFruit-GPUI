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
