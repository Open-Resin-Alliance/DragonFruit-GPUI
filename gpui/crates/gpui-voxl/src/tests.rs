//! In-crate tests: synthetic containers built byte by byte, plus a real
//! locally-saved scene behind `#[ignore]`.

use std::io::Write;
use std::path::Path;

use super::*;

/// One chunk for [`container`].
struct Chunk {
    kind: [u8; 4],
    payload: Vec<u8>,
    /// Directory compression word: `0` stores raw, anything else is zlib.
    /// The writer emits `65536`; the spec says `1`.
    compression: u32,
    zlib: bool,
}

impl Chunk {
    fn raw(kind: [u8; 4], payload: impl Into<Vec<u8>>) -> Self {
        Self {
            kind,
            payload: payload.into(),
            compression: 0,
            zlib: false,
        }
    }

    fn zlib(kind: [u8; 4], payload: impl Into<Vec<u8>>, compression: u32) -> Self {
        Self {
            kind,
            payload: payload.into(),
            compression,
            zlib: true,
        }
    }
}

fn zlib_bytes(payload: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(payload).expect("compress chunk");
    encoder.finish().expect("finish zlib stream")
}

/// Assembles a container exactly the way `codec-v2.ts` lays one out.
fn container(version: u32, chunks: &[Chunk]) -> Vec<u8> {
    let mut directory = Vec::with_capacity(chunks.len() * DIR_ENTRY_LEN);
    let mut bodies = Vec::new();
    let mut offset = HEADER_LEN + chunks.len() * DIR_ENTRY_LEN;

    for chunk in chunks {
        let stored = if chunk.zlib {
            zlib_bytes(&chunk.payload)
        } else {
            chunk.payload.clone()
        };
        directory.extend_from_slice(&chunk.kind);
        directory.extend_from_slice(&chunk.compression.to_le_bytes());
        directory.extend_from_slice(&(offset as u32).to_le_bytes());
        directory.extend_from_slice(&(stored.len() as u32).to_le_bytes());
        directory.extend_from_slice(&(chunk.payload.len() as u32).to_le_bytes());
        offset += stored.len();
        bodies.extend_from_slice(&stored);
    }

    let mut bytes = Vec::with_capacity(offset);
    bytes.extend_from_slice(&MAGIC);
    bytes.extend_from_slice(&version.to_le_bytes());
    bytes.extend_from_slice(&(chunks.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&directory);
    bytes.extend_from_slice(&bodies);
    bytes
}

#[test]
fn reads_synthetic_container() {
    let mesh = b"not really a mesh, just bytes".to_vec();
    let bytes = container(
        2,
        &[
            Chunk::zlib(*b"META", br#"{"name":"cube scene","revision":2.1}"#, 65536),
            Chunk::zlib(*b"SCNE", br#"{"activeModelId":"m1"}"#, 1),
            Chunk::zlib(
                *b"MODL",
                br#"[{"id":"m1","name":"cube","mesh":{"chunkIndex":0}}]"#,
                65536,
            ),
            Chunk::zlib(*b"MESH", mesh.clone(), 65536),
            Chunk::zlib(*b"SUPP", br#"{"anchors":[]}"#, 65536),
            Chunk::raw(*b"EXTD", br#"{"plugin":{"enabled":true}}"#),
        ],
    );

    let scene = read_bytes(&bytes).expect("synthetic container parses");

    assert_eq!(scene.version, 2);
    assert_eq!(
        scene.meta.as_ref().and_then(|m| m["name"].as_str()),
        Some("cube scene")
    );
    assert_eq!(
        scene
            .scene
            .as_ref()
            .and_then(|s| s["activeModelId"].as_str()),
        Some("m1")
    );
    assert_eq!(scene.models.len(), 1);
    assert_eq!(scene.models[0]["id"], "m1");
    assert_eq!(scene.meshes, vec![mesh]);
    assert!(scene.supports.is_some());
    assert_eq!(
        scene
            .extensions
            .as_ref()
            .and_then(|e| e["plugin"]["enabled"].as_bool()),
        Some(true)
    );
}

#[test]
fn ignores_unknown_and_snapshot_chunks() {
    let bytes = container(
        3,
        &[
            Chunk::zlib(*b"META", br#"{"name":"additive"}"#, 65536),
            Chunk::zlib(*b"MODL", br#"[{"id":"m1"}]"#, 65536),
            Chunk::raw(*b"ZZZZ", b"whatever this is".to_vec()),
            Chunk::raw(*b"HSRC", b"float bytes".to_vec()),
            Chunk::raw(*b"MATP", br#"[{"id":"mat"}]"#),
        ],
    );

    let scene = read_bytes(&bytes).expect("unknown chunks do not reject the file");

    assert_eq!(scene.version, 3);
    assert_eq!(scene.models.len(), 1);
    assert!(scene.meshes.is_empty());
    assert!(scene.extensions.is_none());
}

#[test]
fn raw_chunk_with_writer_index_is_not_inflated() {
    // The writer's directory word is `index | compression << 16`, so a raw
    // chunk that is not the first of its type reads as its ordinal.
    let bytes = container(
        2,
        &[Chunk {
            kind: *b"MESH",
            payload: b"raw mesh bytes".to_vec(),
            compression: 2,
            zlib: false,
        }],
    );

    let scene = read_bytes(&bytes).expect("raw indexed chunk reads as-is");
    assert_eq!(scene.meshes, vec![b"raw mesh bytes".to_vec()]);
}

#[test]
fn truncated_directory_is_an_error() {
    let bytes = container(2, &[Chunk::zlib(*b"META", br#"{"name":"cut"}"#, 65536)]);
    // Header claims one 20-byte entry; keep only the first ten bytes of it.
    let truncated = &bytes[..HEADER_LEN + DIR_ENTRY_LEN / 2];

    match read_bytes(truncated) {
        Err(VoxlError::Truncated) => {}
        other => panic!("expected Truncated, got {other:?}"),
    }
}

#[test]
fn bad_magic_is_an_error() {
    let mut bytes = container(2, &[Chunk::raw(*b"META", b"{}".to_vec())]);
    bytes[..4].copy_from_slice(b"NOPE");

    match read_bytes(&bytes) {
        Err(VoxlError::NotVoxl) => {}
        other => panic!("expected NotVoxl, got {other:?}"),
    }
}

#[test]
fn unsupported_version_is_an_error() {
    let bytes = container(4, &[Chunk::raw(*b"META", b"{}".to_vec())]);

    match read_bytes(&bytes) {
        Err(VoxlError::UnsupportedVersion(4)) => {}
        other => panic!("expected UnsupportedVersion(4), got {other:?}"),
    }
}

#[test]
fn corrupt_zlib_payload_is_an_error() {
    let mut bytes = container(2, &[Chunk::zlib(*b"META", b"{}".to_vec(), 65536)]);
    // Corrupt the stored zlib trailer while leaving the directory claim
    // (non-zero code, zlib length) intact: the reader has to report a typed
    // inflate failure rather than hand back garbage.
    let last = bytes.len() - 1;
    bytes[last] ^= 0xFF;

    match read_bytes(&bytes) {
        Err(VoxlError::Inflate(_)) => {}
        other => panic!("expected Inflate, got {other:?}"),
    }
}

#[test]
#[ignore = "opens a real scene from C:/Users/Paul/Downloads when present"]
fn opens_local_supported_scene() {
    let path = Path::new(r"C:\Users\Paul\Downloads\NOM_majoraMask_Supported.voxl");
    let Ok(bytes) = std::fs::read(path) else {
        // The fixture is local-only; an ignored test run on another machine
        // must not fail on its absence.
        return;
    };
    assert!(bytes.len() >= HEADER_LEN, "file is smaller than a header");
    let chunk_count = u32::from_le_bytes(read_u32(&bytes, 8));

    let scene = read_bytes(&bytes).expect("real scene parses");

    assert_eq!(scene.version, 2);
    assert_eq!(chunk_count, 6);
    assert!(!scene.models.is_empty());
    assert!(!scene.meshes.is_empty());
    assert!(scene.supports.is_some());
}
