//! Overhang region classification — the stage the whole run stands on.
//!
//! This is a port of `src-tauri/src/overhang.rs` (`classify_overhangs`,
//! `build_region`, `build_footprint_mask`, `build_perimeter_loops`,
//! `classify_steep_flats`). The React app does not classify overhangs itself:
//! `useIslands.ts` calls the `scan_overhangs` command, and
//! `overhangRegionToIsland` hands the regions to `autoPlace.ts`. A mesh-only
//! `place` needs the same regions, so the classifier is ported here rather than
//! assumed.
//!
//! # PORT DEVIATIONS
//!
//! - **Loop order is deterministic.** `build_perimeter_loops` walks an
//!   `HashMap<(u32, u32), _>` to find boundary edges, so its loop order (and
//!   which loop a shared vertex starts from) follows hash iteration order. The
//!   port sorts the boundary edges by vertex key before tracing: the same
//!   loops, in a fixed order. That matters because ring contacts are emitted in
//!   loop order, and `place` must be reproducible.
//! - **The topple report is not ported.** `compute_stability_report` and the
//!   `drag_moment_mm3`/`drag_dir_deg` fields are analysis for the panel; nothing
//!   in contact placement reads them.

use std::collections::{HashMap, HashSet, VecDeque};

use glam::Vec3;

use super::constants::{
    PERIMETER_CONTACT_INSET_MM, STEEP_FLAT_MAX_ANGLE_DEG, STEEP_FLAT_MIN_AREA_MM2,
    STEEP_FLAT_NORMAL_TOL_DEG,
};
use super::soup::WeldedMesh;

/// Binary raster of a region's XY-projected footprint (`FootprintMask`).
#[derive(Clone, Debug, Default)]
pub(crate) struct FootprintMask {
    pub width: u32,
    pub height: u32,
    /// World XY of the mask's top-left pixel centre (mm).
    pub origin_x: f32,
    pub origin_y: f32,
    pub px_mm: f32,
    /// Row-major pixels, 1 = inside the projected region.
    pub data: Vec<u8>,
    /// Row-major surface Z on the region's own triangles, parallel to `data`.
    pub surface_z: Vec<f32>,
}

impl FootprintMask {
    /// Every inside pixel as `(x, y, z)`, row-major — the same ordering
    /// `overhangRegionToIsland` pushes into `contactVoxels`.
    pub fn points(&self) -> Vec<[f32; 3]> {
        let mut points = Vec::new();
        for row in 0..self.height {
            for col in 0..self.width {
                let idx = (row * self.width + col) as usize;
                if self.data[idx] == 0 {
                    continue;
                }
                points.push([
                    self.origin_x + (col as f32 + 0.5) * self.px_mm,
                    self.origin_y + (row as f32 + 0.5) * self.px_mm,
                    self.surface_z[idx],
                ]);
            }
        }
        points
    }
}

/// A connected patch of overhang triangles (`OverhangRegion`, minus the topple
/// fields).
#[derive(Clone, Debug)]
pub(crate) struct Region {
    /// Triangle indices into the welded mesh.
    pub triangle_ids: Vec<u32>,
    /// Sum of the 3D triangle areas (mm²).
    pub area_mm2: f32,
    /// Sum of the XY-projected triangle areas (mm²).
    pub projected_area_mm2: f32,
    /// Area-weighted mean surface angle from horizontal (degrees).
    pub angle_deg: f32,
    /// Area-weighted mean face normal (world space).
    pub normal: [f32; 3],
    pub min_z: f32,
    pub max_z: f32,
    pub footprint: FootprintMask,
    /// Triangle-accurate perimeter loops, inset by
    /// [`PERIMETER_CONTACT_INSET_MM`], each loop closed.
    pub perimeter_loops: Vec<Vec<[f32; 3]>>,
    /// True for a `classify_steep_flats` patch: a large planar face past the
    /// self-support angle, a topple lever rather than a formation overhang.
    pub steep_flat: bool,
}

/// One undirected mesh edge, tagged with the triangle that owns it.
#[derive(Clone, Copy)]
struct EdgeRef {
    /// `(min_vertex << 32) | max_vertex`.
    key: u64,
    tri: u32,
}

#[inline]
fn edge_key(a: u32, b: u32) -> u64 {
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    ((lo as u64) << 32) | hi as u64
}

fn find(parent: &mut [u32], x: u32) -> u32 {
    let mut root = x;
    while parent[root as usize] != root {
        root = parent[root as usize];
    }
    let mut cur = x;
    while parent[cur as usize] != root {
        let next = parent[cur as usize];
        parent[cur as usize] = root;
        cur = next;
    }
    root
}

fn union(parent: &mut [u32], a: u32, b: u32) {
    let ra = find(parent, a);
    let rb = find(parent, b);
    if ra != rb {
        // Deterministic: smaller id wins as root.
        let (keep, drop) = if ra < rb { (ra, rb) } else { (rb, ra) };
        parent[drop as usize] = keep;
    }
}

/// `classify_overhangs`: every connected overhang patch on the mesh, plus the
/// steep flats.
pub(crate) fn classify_overhangs(
    mesh: &WeldedMesh,
    self_support_angle_deg: f32,
    px_mm: f32,
) -> Vec<Region> {
    let tri_count = mesh.triangle_count();
    if tri_count == 0 {
        return Vec::new();
    }

    // A down-facing surface at angle θ from horizontal has normal.z = -cos(θ).
    // Overhang iff θ < threshold ⟺ normal.z < -cos(threshold).
    let threshold = -self_support_angle_deg.to_radians().cos();

    let mut normals = Vec::with_capacity(tri_count);
    let mut is_overhang = Vec::with_capacity(tri_count);
    for fi in 0..tri_count as u32 {
        let n = mesh.tri_normal(fi);
        normals.push(n);
        is_overhang.push(n.z < threshold);
    }

    // Triangle adjacency through undirected edges, as one flat array sorted by
    // edge key.
    let mut edges: Vec<EdgeRef> = Vec::with_capacity(tri_count * 3);
    for (fi, tri) in mesh.triangles.iter().enumerate() {
        let owner = fi as u32;
        edges.push(EdgeRef {
            key: edge_key(tri[0], tri[1]),
            tri: owner,
        });
        edges.push(EdgeRef {
            key: edge_key(tri[1], tri[2]),
            tri: owner,
        });
        edges.push(EdgeRef {
            key: edge_key(tri[2], tri[0]),
            tri: owner,
        });
    }
    edges.sort_unstable_by_key(|edge| edge.key);

    let mut parent: Vec<u32> = (0..tri_count as u32).collect();
    for pair in edges.windows(2) {
        if pair[0].key == pair[1].key {
            union(&mut parent, pair[0].tri, pair[1].tri);
        }
    }

    let mut by_root: HashMap<u32, Vec<u32>> = HashMap::new();
    for fi in 0..tri_count as u32 {
        if !is_overhang[fi as usize] {
            continue;
        }
        let root = find(&mut parent, fi);
        by_root.entry(root).or_default().push(fi);
    }
    let mut groups: Vec<(u32, Vec<u32>)> = by_root.into_iter().collect();
    groups.sort_by_key(|(root, _)| *root);

    // The part's own base plane: the drag moment is measured from it.
    let base_z = mesh.positions.iter().map(|p| p.z).fold(f32::INFINITY, f32::min);
    if !base_z.is_finite() {
        return Vec::new();
    }

    let mut regions: Vec<Region> = Vec::with_capacity(groups.len());
    for (_, triangle_ids) in groups {
        regions.push(build_region(
            mesh,
            &normals,
            triangle_ids,
            px_mm,
            base_z,
            false,
        ));
    }
    regions.extend(classify_steep_flats(
        mesh,
        &normals,
        &edges,
        threshold,
        px_mm,
        base_z,
    ));
    regions
}

/// Large planar faces just past the self-support angle — the topple levers.
/// `classify_steep_flats` in `overhang.rs`.
fn classify_steep_flats(
    mesh: &WeldedMesh,
    normals: &[Vec3],
    edges: &[EdgeRef],
    self_support_threshold: f32,
    px_mm: f32,
    base_z: f32,
) -> Vec<Region> {
    let tri_count = mesh.triangle_count();
    if tri_count == 0 {
        return Vec::new();
    }
    // Band: θ ∈ [self-support, STEEP_FLAT_MAX_ANGLE_DEG]. normal.z = -cos(θ),
    // so θ ≥ self-support ⟺ nz ≥ threshold, and θ ≤ max ⟺ nz ≤ -cos(max).
    let steep_ceiling = -STEEP_FLAT_MAX_ANGLE_DEG.to_radians().cos();
    let cos_tol = STEEP_FLAT_NORMAL_TOL_DEG.to_radians().cos();
    let in_band = |fi: usize| {
        let nz = normals[fi].z;
        nz >= self_support_threshold && nz <= steep_ceiling
    };

    let mut claimed = vec![false; tri_count];
    let mut out: Vec<Region> = Vec::new();
    let mut neighbors: Vec<u32> = Vec::with_capacity(8);
    let mut queue: VecDeque<u32> = VecDeque::new();

    for seed in 0..tri_count as u32 {
        if claimed[seed as usize] || !in_band(seed as usize) {
            continue;
        }
        claimed[seed as usize] = true;
        let mut acc = [0f32; 3];
        let mut area = 0f32;
        let mut members: Vec<u32> = vec![seed];
        queue.clear();
        queue.push_back(seed);
        let seed_area = mesh.tri_area(seed);
        accumulate_normal(&mut acc, normals[seed as usize], seed_area);
        area += seed_area;

        while let Some(fi) = queue.pop_front() {
            // Area-weighted mean so far — the plane the patch is fitting.
            let len = (acc[0] * acc[0] + acc[1] * acc[1] + acc[2] * acc[2]).sqrt();
            let (mx, my, mz) = if len > 1e-9 {
                (acc[0] / len, acc[1] / len, acc[2] / len)
            } else {
                (0.0, 0.0, -1.0)
            };

            neighbors.clear();
            let tri = mesh.triangle(fi);
            for key in [
                edge_key(tri[0], tri[1]),
                edge_key(tri[1], tri[2]),
                edge_key(tri[2], tri[0]),
            ] {
                let start = edges.partition_point(|edge| edge.key < key);
                let end = edges.partition_point(|edge| edge.key <= key);
                neighbors.extend(edges[start..end].iter().map(|edge| edge.tri));
            }
            neighbors.sort_unstable();
            neighbors.dedup();

            for &nb in neighbors.iter() {
                if nb == fi || claimed[nb as usize] || !in_band(nb as usize) {
                    continue;
                }
                let n = normals[nb as usize];
                if mx * n.x + my * n.y + mz * n.z < cos_tol {
                    continue;
                }
                claimed[nb as usize] = true;
                members.push(nb);
                let tri_area = mesh.tri_area(nb);
                area += tri_area;
                accumulate_normal(&mut acc, n, tri_area);
                queue.push_back(nb);
            }
        }

        if area < STEEP_FLAT_MIN_AREA_MM2 {
            continue;
        }
        // The footprint raster keeps the first triangle per pixel, so a stable
        // triangle order keeps the mask stable too.
        members.sort_unstable();
        out.push(build_region(mesh, normals, members, px_mm, base_z, true));
    }
    out
}

fn accumulate_normal(acc: &mut [f32; 3], n: Vec3, area: f32) {
    acc[0] += n.x * area;
    acc[1] += n.y * area;
    acc[2] += n.z * area;
}

fn build_region(
    mesh: &WeldedMesh,
    normal: &[Vec3],
    triangle_ids: Vec<u32>,
    px_mm: f32,
    _base_z: f32,
    steep_flat: bool,
) -> Region {
    let mut area_mm2 = 0.0f32;
    let mut projected_area_mm2 = 0.0f32;
    let mut angle_weighted = 0.0f32;
    let mut min_z = f32::INFINITY;
    let mut max_z = f32::NEG_INFINITY;
    let mut xy_min = [f32::INFINITY; 2];
    let mut xy_max = [f32::NEG_INFINITY; 2];

    let mut normal_acc = [0f32; 3];
    for &fi in &triangle_ids {
        let [a, b, c] = mesh.tri_positions(fi);
        let area = mesh.tri_area(fi);
        area_mm2 += area;

        let n = normal[fi as usize];
        normal_acc[0] += n.x * area;
        normal_acc[1] += n.y * area;
        normal_acc[2] += n.z * area;

        // XY-projected area of the triangle.
        let cross2d = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
        projected_area_mm2 += cross2d.abs() * 0.5;

        let nz = normal[fi as usize].z.clamp(-1.0, 0.0);
        // Surface angle from horizontal: normal.z = -cos(θ) → θ = acos(-nz).
        let angle = (-nz).acos().to_degrees();
        angle_weighted += area * angle;

        for v in [a, b, c] {
            min_z = min_z.min(v.z);
            max_z = max_z.max(v.z);
            xy_min[0] = xy_min[0].min(v.x);
            xy_min[1] = xy_min[1].min(v.y);
            xy_max[0] = xy_max[0].max(v.x);
            xy_max[1] = xy_max[1].max(v.y);
        }
    }

    let footprint = build_footprint_mask(mesh, &triangle_ids, xy_min, xy_max, px_mm);
    let perimeter_loops = build_perimeter_loops(mesh, &triangle_ids, PERIMETER_CONTACT_INSET_MM);

    let normal_len =
        (normal_acc[0] * normal_acc[0] + normal_acc[1] * normal_acc[1] + normal_acc[2] * normal_acc[2])
            .sqrt();
    let normal = if normal_len > 1e-9 {
        [
            normal_acc[0] / normal_len,
            normal_acc[1] / normal_len,
            normal_acc[2] / normal_len,
        ]
    } else {
        [0.0, 0.0, -1.0]
    };

    Region {
        triangle_ids,
        area_mm2,
        projected_area_mm2,
        angle_deg: if area_mm2 > 1e-9 {
            angle_weighted / area_mm2
        } else {
            0.0
        },
        normal,
        min_z,
        max_z,
        footprint,
        perimeter_loops,
        steep_flat,
    }
}

/// Rasterize the region's XY-projected triangles into a containment mask.
/// The mask is expanded by half a pixel on each side so edge pixels sample the
/// region interior rather than falling just outside it.
fn build_footprint_mask(
    mesh: &WeldedMesh,
    triangle_ids: &[u32],
    xy_min: [f32; 2],
    xy_max: [f32; 2],
    px_mm: f32,
) -> FootprintMask {
    let px = px_mm.max(1e-4);
    let width = (((xy_max[0] - xy_min[0]) / px).ceil() as u32).max(1);
    let height = (((xy_max[1] - xy_min[1]) / px).ceil() as u32).max(1);

    let mut data = vec![0u8; (width * height) as usize];
    let mut surface_z = vec![0f32; (width * height) as usize];
    // Pixel centres, expanded by half a pixel outward from the bbox.
    let origin_x = xy_min[0] - px * 0.5;
    let origin_y = xy_min[1] - px * 0.5;

    // Rasterize per triangle: each triangle fills the pixels inside its own XY
    // bbox. First triangle in list order wins.
    for &fi in triangle_ids {
        let [a, b, c] = mesh.tri_positions(fi);
        let tmin_x = a.x.min(b.x).min(c.x);
        let tmax_x = a.x.max(b.x).max(c.x);
        let tmin_y = a.y.min(b.y).min(c.y);
        let tmax_y = a.y.max(b.y).max(c.y);

        let px0 = (((tmin_x - origin_x) / px).floor() as i64).max(0) as u32;
        let px1 = (((tmax_x - origin_x) / px).floor() as i64).min(width as i64 - 1) as u32;
        let py0 = (((tmin_y - origin_y) / px).floor() as i64).max(0) as u32;
        let py1 = (((tmax_y - origin_y) / px).floor() as i64).min(height as i64 - 1) as u32;

        for py in py0..=py1 {
            let y = origin_y + (py as f32 + 0.5) * px;
            for px_idx in px0..=px1 {
                let idx = (py * width + px_idx) as usize;
                if data[idx] == 1 {
                    continue;
                }
                let x = origin_x + (px_idx as f32 + 0.5) * px;
                if point_in_triangle_2d(x, y, (a.x, a.y), (b.x, b.y), (c.x, c.y)) {
                    data[idx] = 1;
                    surface_z[idx] = barycentric_z(x, y, a, b, c);
                }
            }
        }
    }

    FootprintMask {
        width,
        height,
        origin_x,
        origin_y,
        px_mm: px,
        data,
        surface_z,
    }
}

/// Extract triangle-accurate perimeter loops for a region and inset them by
/// `inset_mm` so a support contact disc sits fully on the surface.
///
/// Boundary edges are those with exactly one incident region triangle; loops are
/// traced via vertex adjacency and offset per-vertex toward the interior
/// (edge-mid → opposite vertex, averaged at vertices).
fn build_perimeter_loops(
    mesh: &WeldedMesh,
    triangle_ids: &[u32],
    inset_mm: f32,
) -> Vec<Vec<[f32; 3]>> {
    if triangle_ids.is_empty() {
        return Vec::new();
    }
    // Edge → [(tri_id, opposite_vertex)]
    let mut edge_map: HashMap<(u32, u32), Vec<(u32, u32)>> = HashMap::new();
    for &tid in triangle_ids {
        let tri = mesh.triangle(tid);
        let (v0, v1, v2) = (tri[0], tri[1], tri[2]);
        for (a, b, c) in [(v0, v1, v2), (v1, v2, v0), (v2, v0, v1)] {
            let key = if a < b { (a, b) } else { (b, a) };
            edge_map.entry(key).or_default().push((tid, c));
        }
    }

    // Boundary edges: exactly one region triangle. Sorted by edge key so the
    // trace order (and therefore the emitted loop order) is deterministic.
    let mut keys: Vec<(u32, u32)> = edge_map.keys().copied().collect();
    keys.sort_unstable();
    let mut boundary: Vec<(u32, u32, u32, u32)> = Vec::new(); // a, b, opp, tri
    let mut edge_opp: HashMap<(u32, u32), (u32, u32)> = HashMap::new(); // key -> (tri, opp)
    let mut adj: HashMap<u32, Vec<u32>> = HashMap::new();
    for key in keys {
        let vec = &edge_map[&key];
        if vec.len() == 1 {
            let (tri, opp) = vec[0];
            let (a, b) = key;
            boundary.push((a, b, opp, tri));
            adj.entry(a).or_default().push(b);
            adj.entry(b).or_default().push(a);
            edge_opp.insert(key, (tri, opp));
        }
    }
    if boundary.is_empty() {
        return Vec::new();
    }

    // Trace loops via adjacency.
    let mut visited: HashSet<(u32, u32)> = HashSet::new();
    let mut loops: Vec<Vec<u32>> = Vec::new();
    for &(a, b, _, _) in &boundary {
        let key = if a < b { (a, b) } else { (b, a) };
        if visited.contains(&key) {
            continue;
        }
        let mut loop_vs: Vec<u32> = vec![a, b];
        visited.insert(key);
        let mut prev = a;
        let mut cur = b;
        loop {
            if cur == a {
                break;
            }
            let neighbors = match adj.get(&cur) {
                Some(n) => n,
                None => break,
            };
            let mut next_opt: Option<u32> = None;
            for &nb in neighbors {
                if nb == prev {
                    continue;
                }
                let k = if cur < nb { (cur, nb) } else { (nb, cur) };
                if !visited.contains(&k) {
                    next_opt = Some(nb);
                    break;
                }
            }
            if next_opt.is_none() {
                for &nb in neighbors {
                    let k = if cur < nb { (cur, nb) } else { (nb, cur) };
                    if !visited.contains(&k) {
                        next_opt = Some(nb);
                        break;
                    }
                }
            }
            if let Some(next) = next_opt {
                let k = if cur < next { (cur, next) } else { (next, cur) };
                visited.insert(k);
                loop_vs.push(next);
                prev = cur;
                cur = next;
                if cur == a {
                    break;
                }
                if loop_vs.len() > boundary.len() + 2 {
                    break;
                }
            } else {
                break;
            }
        }
        if loop_vs.len() > 1 && loop_vs[0] == *loop_vs.last().unwrap() {
            loop_vs.pop();
        }
        if loop_vs.len() >= 3 {
            loops.push(loop_vs);
        }
    }
    if loops.is_empty() {
        return Vec::new();
    }

    // Compute inset loops: per-vertex average of incident edge interior dirs.
    let mut out: Vec<Vec<[f32; 3]>> = Vec::new();
    for vs in loops {
        let n = vs.len();
        let mut edge_dirs: Vec<(f32, f32)> = Vec::with_capacity(n);
        for i in 0..n {
            let a = vs[i];
            let b = vs[(i + 1) % n];
            let key = if a < b { (a, b) } else { (b, a) };
            if let Some((_, opp)) = edge_opp.get(&key) {
                let pa = mesh.positions[a as usize];
                let pb = mesh.positions[b as usize];
                let pc = mesh.positions[*opp as usize];
                let mid_x = (pa.x + pb.x) * 0.5;
                let mid_y = (pa.y + pb.y) * 0.5;
                let dx = pc.x - mid_x;
                let dy = pc.y - mid_y;
                let len = (dx * dx + dy * dy).sqrt();
                if len > 1e-6 {
                    edge_dirs.push((dx / len, dy / len));
                } else {
                    edge_dirs.push((0.0, 0.0));
                }
            } else {
                edge_dirs.push((0.0, 0.0));
            }
        }
        let mut inset_loop: Vec<[f32; 3]> = Vec::with_capacity(n);
        for i in 0..n {
            let vid = vs[i];
            let p = mesh.positions[vid as usize];
            let dir_prev = edge_dirs[(i + n - 1) % n];
            let dir_next = edge_dirs[i];
            let avg_x = dir_prev.0 + dir_next.0;
            let avg_y = dir_prev.1 + dir_next.1;
            let len = (avg_x * avg_x + avg_y * avg_y).sqrt();
            let (off_x, off_y) = if len > 1e-6 {
                (avg_x / len * inset_mm, avg_y / len * inset_mm)
            } else if dir_next.0 != 0.0 || dir_next.1 != 0.0 {
                (dir_next.0 * inset_mm, dir_next.1 * inset_mm)
            } else if dir_prev.0 != 0.0 || dir_prev.1 != 0.0 {
                (dir_prev.0 * inset_mm, dir_prev.1 * inset_mm)
            } else {
                (0.0, 0.0)
            };
            let new_x = p.x + off_x;
            let new_y = p.y + off_y;
            // Project Z onto the incident triangle plane so the inset point
            // stays on the surface.
            let new_z = if inset_mm.abs() > 1e-6 {
                let a = vs[i];
                let b = vs[(i + 1) % n];
                let key = if a < b { (a, b) } else { (b, a) };
                if let Some((tri, _opp)) = edge_opp.get(&key) {
                    let tri_verts = mesh.triangle(*tri);
                    let v_a = mesh.positions[tri_verts[0] as usize];
                    let v_b = mesh.positions[tri_verts[1] as usize];
                    let v_c = mesh.positions[tri_verts[2] as usize];
                    let n = (v_b - v_a).cross(v_c - v_a);
                    if n.z.abs() > 1e-6 {
                        v_a.z - (n.x * (new_x - v_a.x) + n.y * (new_y - v_a.y)) / n.z
                    } else {
                        p.z
                    }
                } else {
                    p.z
                }
            } else {
                p.z
            };
            inset_loop.push([new_x, new_y, new_z]);
        }

        // Validate: the inset loop must still have area and must not collapse.
        let mut use_raw = false;
        if inset_mm > 1e-6 {
            let mut area2: f64 = 0.0;
            for i in 0..n {
                let a = inset_loop[i];
                let b = inset_loop[(i + 1) % n];
                area2 += (a[0] as f64) * (b[1] as f64) - (b[0] as f64) * (a[1] as f64);
            }
            if area2.abs() < 1e-6 {
                use_raw = true;
            } else {
                // Inset should not push vertices more than 3× inset away from
                // the original — catches inversion on tight concavities.
                let mut max_dist2: f32 = 0.0;
                for i in 0..n {
                    let dx = inset_loop[i][0] - mesh.positions[vs[i] as usize].x;
                    let dy = inset_loop[i][1] - mesh.positions[vs[i] as usize].y;
                    max_dist2 = max_dist2.max(dx * dx + dy * dy);
                }
                if max_dist2 > (inset_mm * 3.0) * (inset_mm * 3.0) {
                    use_raw = true;
                }
            }
        }
        if use_raw {
            let raw: Vec<[f32; 3]> = vs
                .iter()
                .map(|id| {
                    let p = mesh.positions[*id as usize];
                    [p.x, p.y, p.z]
                })
                .collect();
            out.push(raw);
        } else {
            out.push(inset_loop);
        }
    }
    out
}

/// Surface Z of a triangle at a projected XY via barycentric interpolation.
pub(crate) fn barycentric_z(px: f32, py: f32, a: Vec3, b: Vec3, c: Vec3) -> f32 {
    let denom = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
    if denom.abs() < 1e-9 {
        return (a.z + b.z + c.z) / 3.0;
    }
    let w_a = ((b.y - c.y) * (px - c.x) + (c.x - b.x) * (py - c.y)) / denom;
    let w_b = ((c.y - a.y) * (px - c.x) + (a.x - c.x) * (py - c.y)) / denom;
    let w_c = 1.0 - w_a - w_b;
    w_a * a.z + w_b * b.z + w_c * c.z
}

/// Point-in-triangle test (2D, half-plane method); boundary counts as inside.
pub(crate) fn point_in_triangle_2d(
    px: f32,
    py: f32,
    a: (f32, f32),
    b: (f32, f32),
    c: (f32, f32),
) -> bool {
    let sign = |x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32| {
        (x1 - x3) * (y2 - y3) - (x2 - x3) * (y1 - y3)
    };
    let d1 = sign(px, py, a.0, a.1, b.0, b.1);
    let d2 = sign(px, py, b.0, b.1, c.0, c.1);
    let d3 = sign(px, py, c.0, c.1, a.0, a.1);
    let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(has_neg && has_pos)
}
