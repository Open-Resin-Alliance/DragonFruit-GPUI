# GPUI VOXL reader

`gpui/crates/gpui-voxl` reads the scene container the React app writes. It is a
container reader, not a scene loader: it splits a `.voxl` file into chunks,
decompresses the payloads and hands the JSON back as written. Resolving mesh
references, modifier snapshots and material assignments stays with the caller.

The ported source is the binary half of `src/features/scene/voxl/codec-v2.ts`
(`parseVoxlBinaryV2`). The format contract is
[`voxl-format-spec.md`](voxl-format-spec.md); this page describes how the Rust
crate implements it.

## Layout

```text
[16-byte header] [directory: chunkCount * 20 bytes] [chunk payloads]
```

### Header

| Bytes | Field | Encoding |
|---|---|---|
| 0..4 | magic `VOXL` | 4 ASCII bytes (`0x56 0x4F 0x58 0x4C`) |
| 4..6 | version | `u16` LE — the compat floor: `2`, or `3` when MESH chunks are shared |
| 6..8 | flags | `u16` LE — reserved, always `0` |
| 8..12 | chunk count | `u32` LE |
| 12..16 | reserved | `u32` LE — always `0` |

The writer stores the version as a `u16` followed by a zero `u16` flags word.
The crate reads the four bytes at offset 4 as one `u32` version and the four at
offset 8 as the chunk count; the readings are byte-identical because the flags
word is always zero. A version other than `2` or `3` is refused, not guessed at.

### Directory entry

Every entry is 20 bytes, little-endian.

| Bytes | Field | Meaning |
|---|---|---|
| 0..4 | type | 4 ASCII characters (`META`, `MESH`, …) |
| 4..6 | index | `u16` — ordinal when a type repeats, e.g. one `MESH` per model |
| 6..8 | compression | `u16` — `0` raw, `1` zlib on a spec-conforming file |
| 8..12 | offset | `u32` — payload start from the beginning of the file |
| 12..16 | compressed size | `u32` — bytes stored |
| 16..20 | raw size | `u32` — bytes after decompression |

Every `offset + compressed size` is checked against the file length before a
payload is sliced, and the directory itself must fit in the file. A decompressed
payload whose length is not the entry's raw size is an error, not a truncated
read.

The crate caps the chunk count at 10 000 before doing directory arithmetic, so
a corrupted count fails cleanly instead of trying to reserve for it.

## The compression-word quirk

The spec's table says `0` is raw and `1` is zlib. The real writer emits `65536`
for zlib chunks and the payload starts with `78 9C`. Both are true because they
describe different words.

The writer lays the entry out as two `u16`s: `index` at 4..6 and `compression`
at 6..8. A reader that takes the four bytes at 4..8 as one `u32` gets

```text
u32 = index | (compression_code << 16)
```

so a zlib chunk at index `0` reads as `65536` (`0x0001_0000`, bytes
`00 00 01 00`), and one at index `1` reads as `65537`. `78 9C` is the ordinary
zlib header on the payload; it is not a second format.

The crate therefore classifies the whole word:

| Word | Meaning |
|---|---|
| `0` | raw payload |
| `1` | zlib (the spec's code) |
| high half `1` (`code >> 16 == 1`) | zlib; the low half is the chunk's index — `65536`, `65537`, … |
| high half `0` with a non-zero low half | raw; the writer stored this chunk raw because zlib did not shrink it, and the value is the chunk index |
| anything else | unsupported compression, reported as an error |

The reader does not sniff `78 9C`. It trusts the classification and reports an
inflate failure rather than passing corrupt bytes through. The synthetic
containers in the tests cover both `1` and `65536`, the raw-index case, and a
corrupt zlib payload.

## Chunk types

| Type | Payload | Destination |
|---|---|---|
| `META` | scene metadata JSON | `meta` |
| `SCNE` | scene state JSON | `scene` |
| `MODL` | model entries JSON array | `models`, one element per entry |
| `MESH` | opaque mesh bytes | `meshes`, in directory order |
| `SUPP` | support document JSON | `supports` |
| `EXTD` | extensions JSON | `extensions` |
| `HSRC` `CAVT` `PSRC` | hollowing / hole-punch snapshot bytes | ignored |
| `BODY` `MATP` `ORIG` | split bodies, material profiles, original-file bytes | ignored |
| anything else | unknown | ignored |

`MODL` is normally an array; a single JSON object is accepted as a one-model
list so a hand-made file still opens. `MESH` chunks are collected in directory
order; with container version 3 the writer has already removed shared chunks, so
that order is the deduplicated model order and a duplicate model points at its
owner through `mesh.chunkIndex` inside the `MODL` entry.

Ignored does not mean deleted. A caller that needs modifier snapshots or body
geometry must read those chunks itself; the container reader deliberately leaves
document-level semantics alone. Unknown types are ignored because the format
allows additive chunks, and the React reader tolerated them too.

## API

```rust
pub fn read(path: &Path) -> Result<Scene, VoxlError>;
pub fn read_bytes(bytes: &[u8]) -> Result<Scene, VoxlError>;
```

`read` reads the file and delegates to `read_bytes`, which does all parsing. Use
the byte form when the scene came from a zip, a network response or a test
fixture; use the path form when the caller already has a file.

The returned scene record holds:

| Field | Type | Holds |
|---|---|---|
| `version` | `u32` | container floor (`2` or `3`) |
| `meta` | optional JSON | `META` object |
| `scene` | optional JSON | `SCNE` object |
| `models` | JSON list | `MODL` entries |
| `meshes` | byte vectors | decompressed `MESH` payloads |
| `supports` | optional JSON | `SUPP` document |
| `extensions` | optional JSON | `EXTD` object |

Parsing failures are typed: bad magic, unsupported version, truncation, a bad
chunk header, invalid JSON, an inflate failure, and I/O for the path form. There
is no partial scene on error.

## Minimal example

```rust
use std::path::Path;

let scene = gpui_voxl::read(Path::new("MyPrint.voxl"))?;
println!(
    "version {}, {} models, {} meshes",
    scene.version,
    scene.models.len(),
    scene.meshes.len(),
);
```

The reader returns complete payloads, not file ranges, so every visited `MESH`
chunk is decompressed even when the caller only wants the JSON chunks. A caller
that needs to keep meshes on disk should read the directory itself.

## Constraints

- **No V1.** A JSON scene (first byte `{`) fails the magic check. V1 was refused
  by the React reader too.
- **No dedup reconstruction.** Version 3 files are accepted, but the reader does
  not copy a shared mesh to the models that point at it.
- **No size guessing.** Offsets, lengths and chunk count are validated before
  allocation or slicing.
- **No silent corruption.** Raw-size mismatch and inflate failure are errors.

## Sources

- `gpui/crates/gpui-voxl/src/lib.rs` — container layout, compression
  classification, chunk decode.
- `gpui/crates/gpui-voxl/src/tests.rs` — synthetic `1`/`65536` containers, the
  raw-index case and the real-writer fixture.
- `src/features/scene/voxl/codec-v2.ts` — the writer and its header comment.
- `src/features/scene/voxl/types.ts` — `VoxlMeshRef` and the model/scene shapes.
- [`voxl-format-spec.md`](voxl-format-spec.md) — the format contract.
