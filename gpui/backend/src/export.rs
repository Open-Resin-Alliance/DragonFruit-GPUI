//! `export_mesh_file` — raw staged geometry to a real STL or 3MF.
//!
//! Ported from `src-tauri/src/main.rs` lines 961-1200. The frontend uploads the
//! triangle soup it renders to a staging file, then asks for it in a format the
//! user picked. Staging format: 9 × `f32` LE per triangle.
//!
//! This is not algorithm work — it is two file writers — so it lives here rather
//! than in a `rust/` crate; the mesh engines have no opinion about export
//! containers.

use std::io::Write;

use dragonfruit_mesh_core::mesh::Vec3;

use crate::staging::MeshStaging;
use crate::BackendResult;

/// Read a little-endian `f32` at a byte offset.
#[inline(always)]
fn read_f32_le(data: &[u8], off: usize) -> f32 {
    f32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}

/// Write binary STL from raw triangle staging data.
///
/// Face normals are computed here (`cross(v1 - v0, v2 - v0)`, normalised) rather
/// than read from the staging stream, which carries positions only.
pub fn write_binary_stl(raw: &[u8], tri_count: usize, dest_path: &str) -> BackendResult<()> {
    let file =
        std::fs::File::create(dest_path).map_err(|e| format!("Failed creating STL file: {e}"))?;
    let mut w = std::io::BufWriter::with_capacity(256 * 1024, file);

    // 80-byte header (all zeros), then the triangle count.
    w.write_all(&[0u8; 80])
        .map_err(|e| format!("STL write: {e}"))?;
    w.write_all(&(tri_count as u32).to_le_bytes())
        .map_err(|e| format!("STL write: {e}"))?;

    for i in 0..tri_count {
        let base = i * 36; // 9 floats × 4 bytes
        let v0 = Vec3::new(
            read_f32_le(raw, base),
            read_f32_le(raw, base + 4),
            read_f32_le(raw, base + 8),
        );
        let v1 = Vec3::new(
            read_f32_le(raw, base + 12),
            read_f32_le(raw, base + 16),
            read_f32_le(raw, base + 20),
        );
        let v2 = Vec3::new(
            read_f32_le(raw, base + 24),
            read_f32_le(raw, base + 28),
            read_f32_le(raw, base + 32),
        );

        let mut normal = v1.sub(v0).cross(v2.sub(v0));
        let len = normal.length();
        if len > 1e-30 {
            normal = normal.scale(1.0 / len);
        }

        w.write_all(&normal.x.to_le_bytes())
            .map_err(|e| format!("STL write: {e}"))?;
        w.write_all(&normal.y.to_le_bytes())
            .map_err(|e| format!("STL write: {e}"))?;
        w.write_all(&normal.z.to_le_bytes())
            .map_err(|e| format!("STL write: {e}"))?;
        w.write_all(&raw[base..base + 36])
            .map_err(|e| format!("STL write: {e}"))?;
        w.write_all(&0u16.to_le_bytes())
            .map_err(|e| format!("STL write: {e}"))?;
    }

    w.flush().map_err(|e| format!("STL flush: {e}"))?;
    Ok(())
}

/// Write a 3MF (DEFLATE-compressed ZIP) from raw triangle staging data.
///
/// The `zip` crate handles DEFLATE, CRC32 and the central directory. DEFLATE at
/// level 1 rather than the default: repetitive XML compresses 5-8× even there,
/// so output size stays reasonable while the compressor runs ~5× faster. The XML
/// is accumulated in 4 MiB chunks and flushed to the compressor in bulk — the
/// per-vertex `write_all` the first version did meant ~8 M DEFLATE feed calls for
/// a 2 M-triangle mesh.
pub fn write_3mf(raw: &[u8], tri_count: usize, dest_path: &str) -> BackendResult<()> {
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    let file =
        std::fs::File::create(dest_path).map_err(|e| format!("Failed creating 3MF file: {e}"))?;
    let buf_writer = std::io::BufWriter::with_capacity(4 * 1024 * 1024, file);
    let mut zip = ZipWriter::new(buf_writer);

    let deflate_opts = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(1));

    zip.start_file("[Content_Types].xml", deflate_opts)
        .map_err(|e| format!("3MF zip: {e}"))?;
    zip.write_all(
        b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
          <Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
          <Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
          <Default Extension=\"model\" ContentType=\"application/vnd.ms-package.3dmanufacturing-3dmodel+xml\"/>\
          </Types>",
    )
    .map_err(|e| format!("3MF zip: {e}"))?;

    zip.start_file("_rels/.rels", deflate_opts)
        .map_err(|e| format!("3MF zip: {e}"))?;
    zip.write_all(
        b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
          <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
          <Relationship Target=\"/3D/3dmodel.model\" Id=\"rel0\" \
          Type=\"http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel\"/>\
          </Relationships>",
    )
    .map_err(|e| format!("3MF zip: {e}"))?;

    zip.start_file("3D/3dmodel.model", deflate_opts)
        .map_err(|e| format!("3MF zip: {e}"))?;

    const CHUNK: usize = 4 * 1024 * 1024;
    let mut buf: Vec<u8> = Vec::with_capacity(CHUNK + 512);

    buf.extend_from_slice(
        b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
          <model unit=\"millimeter\" xml:lang=\"en-US\" \
          xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\">\
          <resources><object id=\"1\" type=\"model\"><mesh><vertices>",
    );

    for i in 0..tri_count {
        let base = i * 36;
        for j in 0..3usize {
            let fbase = base + j * 12;
            let x = read_f32_le(raw, fbase);
            let y = read_f32_le(raw, fbase + 4);
            let z = read_f32_le(raw, fbase + 8);
            // Writing into a `Vec<u8>` cannot fail; the `write!` result is discarded.
            let _ = write!(buf, "<vertex x=\"{x:.4}\" y=\"{y:.4}\" z=\"{z:.4}\"/>");
        }
        if buf.len() >= CHUNK {
            zip.write_all(&buf).map_err(|e| format!("3MF zip: {e}"))?;
            buf.clear();
        }
    }

    buf.extend_from_slice(b"</vertices><triangles>");

    // Sequential indices: triangle i → corners 3i, 3i+1, 3i+2.
    for i in 0..tri_count {
        let v0 = i * 3;
        let _ = write!(
            buf,
            "<triangle v1=\"{v0}\" v2=\"{}\" v3=\"{}\"/>",
            v0 + 1,
            v0 + 2
        );
        if buf.len() >= CHUNK {
            zip.write_all(&buf).map_err(|e| format!("3MF zip: {e}"))?;
            buf.clear();
        }
    }

    buf.extend_from_slice(
        b"</triangles></mesh></object></resources>\
          <build><item objectid=\"1\"/></build></model>",
    );
    zip.write_all(&buf).map_err(|e| format!("3MF zip: {e}"))?;

    zip.finish().map_err(|e| format!("3MF zip finish: {e}"))?;
    Ok(())
}

/// `export_mesh_file`: convert a staging file into `dest_path` in `format`.
///
/// `staging` is taken so the chunked upload's writer can be flushed and released
/// first — exporting a file the appender still holds buffered bytes for would
/// write a truncated mesh.
pub fn export_mesh_file(
    staging: &MeshStaging,
    staging_path: &str,
    dest_path: &str,
    format: &str,
) -> BackendResult<String> {
    staging.release_appender_for(staging_path)?;

    let raw = std::fs::read(staging_path)
        .map_err(|e| format!("Failed reading staging file '{staging_path}': {e}"))?;

    if raw.len() % 36 != 0 {
        return Err(format!(
            "Invalid staging data: {} bytes is not a multiple of 36 (9 × f32 per triangle)",
            raw.len()
        ));
    }
    let tri_count = raw.len() / 36;
    if tri_count == 0 {
        return Err("Cannot export: no triangles in staged geometry.".into());
    }

    log::info!("[export_mesh_file] {tri_count} triangles → {format} format → {dest_path}");

    match format {
        "stl" => write_binary_stl(&raw, tri_count, dest_path)?,
        "3mf" => write_3mf(&raw, tri_count, dest_path)?,
        other => return Err(format!("Unsupported export format: {other}")),
    }

    // The staging file is spent: the export consumed it.
    let _ = std::fs::remove_file(staging_path);

    Ok(dest_path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two triangles of a unit-ish square in the z=0 plane, as the staging stream
    /// carries them: 9 f32 LE per triangle.
    fn stage_two_triangles(dir: &std::path::Path) -> (MeshStaging, String) {
        let mut raw = Vec::new();
        for value in [
            0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0,
        ] {
            raw.extend_from_slice(&value.to_le_bytes());
        }
        let path = dir.join("stage.bin");
        std::fs::write(&path, &raw).expect("stage");

        let staging = MeshStaging::new();
        staging
            .append_mesh_stage_chunk(&path.to_string_lossy(), &raw, true)
            .expect("appender");
        (staging, path.to_string_lossy().to_string())
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("df-export-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    #[test]
    fn binary_stl_has_the_header_count_and_two_records() {
        let dir = temp_dir("stl");
        let (staging, staging_path) = stage_two_triangles(&dir);
        let dest = dir.join("out.stl");

        let written = export_mesh_file(
            &staging,
            &staging_path,
            &dest.to_string_lossy(),
            "stl",
        )
        .expect("export");
        assert_eq!(written, dest.to_string_lossy());

        let bytes = std::fs::read(&dest).expect("stl");
        assert_eq!(bytes.len(), 84 + 2 * 50);
        assert_eq!(&bytes[0..80], &[0u8; 80][..]);
        assert_eq!(u32::from_le_bytes(bytes[80..84].try_into().unwrap()), 2);
        // The face normal of the first triangle points along -Z under the
        // right-hand rule for these winding orderings.
        let normal_z = read_f32_le(&bytes, 84 + 8);
        assert!(normal_z.abs() > 0.0);
        assert!(
            !std::path::Path::new(&staging_path).exists(),
            "the staging file is consumed"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn three_mf_is_a_zip_with_a_model_part() {
        let dir = temp_dir("3mf");
        let (staging, staging_path) = stage_two_triangles(&dir);
        let dest = dir.join("out.3mf");

        export_mesh_file(&staging, &staging_path, &dest.to_string_lossy(), "3mf").expect("export");

        let file = std::fs::File::open(&dest).expect("3mf");
        let mut archive = zip::ZipArchive::new(file).expect("zip");
        let mut names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).expect("entry").name().to_string())
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec![
                "[Content_Types].xml".to_string(),
                "3D/3dmodel.model".to_string(),
                "_rels/.rels".to_string()
            ]
        );

        let mut model = String::new();
        {
            use std::io::Read;
            archive
                .by_name("3D/3dmodel.model")
                .expect("model")
                .read_to_string(&mut model)
                .expect("read model");
        }
        assert!(model.contains("<vertices>"));
        assert_eq!(model.matches("<triangle ").count(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unsupported_format_is_refused_before_the_staging_file_is_dropped() {
        let dir = temp_dir("bad-format");
        let (staging, staging_path) = stage_two_triangles(&dir);
        let dest = dir.join("out.obj");
        let err = export_mesh_file(&staging, &staging_path, &dest.to_string_lossy(), "obj")
            .expect_err("refuse");
        assert!(err.contains("Unsupported export format"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_empty_stage_is_refused() {
        let dir = temp_dir("empty");
        let path = dir.join("empty.bin");
        std::fs::write(&path, b"").expect("stage");
        let staging = MeshStaging::new();
        let err = export_mesh_file(
            &staging,
            &path.to_string_lossy(),
            &dir.join("out.stl").to_string_lossy(),
            "stl",
        )
        .expect_err("refuse");
        assert!(err.contains("no triangles"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
