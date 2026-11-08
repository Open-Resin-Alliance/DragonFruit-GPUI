//! Contact-point placement — stage 1 of the auto-support run.
//!
//! The React app's run is `computeAutoSupportPlan` in
//! `src/supports/autoSupport/autoPlace.ts`: overhang regions → candidates →
//! dedupe → `placeOneCandidate` through the placement engine → coverage,
//! fanning and bracing. This module is the first half of that — everything up
//! to (and including) the deduplicated candidate list, resolved to the contact
//! a support would attach at.
//!
//! ```text
//! mesh ──classify_overhangs──► regions ──overhangRegionToIsland──► islands
//!   ──generateCandidates──► single contacts ──generateGridCandidates──► lattice+ring
//!   ──deduplicateCandidates──► contacts (position, normal, type)
//! ```
//!
//! [`place`] is pure and deterministic: same mesh, same settings, same output
//! in the same order.
//!
//! # What is NOT here
//!
//! The placement engine itself (`decideGridPlacement`, `PlacementLogicV3`,
//! `buildTrunkData`, pathfinding, collision) decides *how* a contact is carried
//! to the plate; it needs a mutable support draft and an SDF, neither of which a
//! pure contact-placement function has. It is the next stage. Sizing
//! (`sizeParameters`) and the coverage/gap-fill/fanning/bracing passes are
//! likewise downstream.
//!
//! # PORT DEVIATIONS
//!
//! - **Voxel/minima/stabilization passes are absent.** Their islands come from
//!   the slice-growth and minima scanners and from `poseStability`, not from the
//!   mesh alone; the contacts they add are separate candidate sources.
//! - **The topple verdict is assumed to need coverage.** `computeAutoSupportPlan`
//!   drops a steep flat under 150 mm² only when `needsToppleCoverage` says the
//!   pose is stable. The classifier already drops steep flats under 150 mm², so
//!   assuming the verdict is true keeps every flat the classifier emitted —
//!   the conservative choice for a function with no stability report.
//! - **An already-supported filter is a no-op.** `filterAlreadySupported` removes
//!   candidates within 3 mm of an existing tip; a pure run starts from an empty
//!   draft, so nothing can be already supported.

mod candidates;
mod constants;
mod grid;
mod scan;
mod settings;
mod soup;

use glam::Vec3;

pub use settings::SupportSettings;
pub use soup::MeshTriangles;

use candidates::Candidate;
use constants::ANCHOR_HEIGHT_THRESHOLD_MM;
use scan::Region;
use soup::WeldedMesh;

/// The support type a contact attaches as.
///
/// `supportTypeRegistry.ts` declares exactly two `tipHeight` placement rules:
/// `trunk` with `{ minMm: ANCHOR_HEIGHT_THRESHOLD_MM, boundary: 'upper' }` and
/// `stump` with `{ maxMm: ANCHOR_HEIGHT_THRESHOLD_MM, boundary: 'upper' }`.
/// Bounds are half-open, so a tip at or above the threshold is a trunk and one
/// below it is a stump (the near-plate anchor). `selectTypeForPlacement` reads
/// them in registry order, which is trunk first; the two ranges do not overlap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContactSupportType {
    /// `trunk` — a shaft from the plate, `tipHeight >= ANCHOR_HEIGHT_THRESHOLD_MM`.
    Trunk,
    /// `stump` — the near-plate anchor, `tipHeight < ANCHOR_HEIGHT_THRESHOLD_MM`.
    Stump,
}

/// Which type serves a tip at `z_mm` above the plate — `selectTypeForPlacement('tipHeight', z)`.
pub fn contact_support_type_for_tip_height(z_mm: f32) -> ContactSupportType {
    // `boundary: 'upper'`: the trunk's rule skips values below its `minMm` and
    // the stump's skips values at or above its `maxMm`.
    if z_mm < ANCHOR_HEIGHT_THRESHOLD_MM {
        ContactSupportType::Stump
    } else {
        ContactSupportType::Trunk
    }
}

/// One contact point the auto-support run would attach a support at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contact {
    /// Contact position on the model surface, world mm.
    pub position: Vec3,
    /// Surface normal at the contact, world space, unit length.
    pub normal: Vec3,
    /// The registry type this tip height selects.
    pub support_type: ContactSupportType,
    /// Index of the overhang region the contact belongs to — the `o<index>` id
    /// the classifier's regions were numbered with.
    pub region_index: usize,
    /// True for a density-grid contact (lattice cell or boundary-ring sample),
    /// false for the single contact of a sub-threshold region.
    pub grid_point: bool,
}

impl Contact {
    fn from_candidate(candidate: Candidate) -> Self {
        Self {
            position: Vec3::new(
                candidate.tip_pos.x as f32,
                candidate.tip_pos.y as f32,
                candidate.tip_pos.z as f32,
            ),
            normal: Vec3::new(
                candidate.tip_normal.x as f32,
                candidate.tip_normal.y as f32,
                candidate.tip_normal.z as f32,
            ),
            support_type: contact_support_type_for_tip_height(candidate.tip_pos.z as f32),
            region_index: candidate.region_index,
            grid_point: candidate.grid_point,
        }
    }
}

/// Place contact points on `mesh` for `settings`.
///
/// The run is `computeAutoSupportPlan`'s steps 0–2b (`autoPlace.ts`):
///
/// 1. classify overhang regions on the mesh (`scan_overhangs`, ported in
///    [`scan`]);
/// 2. turn them into islands (`overhangRegionToIsland`);
/// 3. generate the single-island candidates (`generateCandidates`), replace the
///    grid-eligible regions' candidates with `generateGridCandidates`' lattice
///    and ring, and deduplicate (`deduplicateCandidates`);
/// 4. resolve each candidate's type from its tip height.
///
/// Order is the TS's: grid contacts in region order (lattice then ring), then
/// the remaining single contacts in priority order, then the dedupe's stable
/// priority sort.
pub fn place(mesh: &MeshTriangles, settings: &SupportSettings) -> Vec<Contact> {
    if !settings.enabled || mesh.is_empty() {
        return Vec::new();
    }

    let welded = mesh.weld();
    let regions: Vec<Region> = scan::classify_overhangs(
        &welded,
        settings.overhang_self_support_angle_deg,
        constants::OVERHANG_FOOTPRINT_PX_MM,
    );
    if regions.is_empty() {
        return Vec::new();
    }

    let islands: Vec<candidates::Island> = regions
        .iter()
        .enumerate()
        .map(|(index, region)| candidates::Island::from_region(index, region))
        // `islandsToCover`: a steep flat is covered when the pose needs it or
        // when it is big enough to anchor the part. The pose verdict needs the
        // stability report, which this stage does not measure, so the
        // conservative verdict — coverage needed — keeps every flat the
        // classifier emitted.
        .filter(|island| {
            !island.steep_flat
                || candidates::steep_flat_needs_coverage(island.surface_area_mm2, true)
        })
        .collect();

    let mut contacts = candidates::generate_candidates(&islands, settings);

    // Step 1b: the overhang regions big enough for the density grid are
    // regenerated as ring + infill, and their single-candidate output dropped.
    let eligible: Vec<&candidates::Island> = islands
        .iter()
        .filter(|island| grid::should_use_density_grid(island, settings))
        .collect();
    if !eligible.is_empty() {
        // A run with no stability report passes `slenderPart = false` and
        // `partThicknessMm = 0` to `generateGridCandidates`, so a steep flat
        // keeps its sparse field and its low anchoring band.
        let generated = grid::generate_grid_candidates(&eligible, settings, Some(&welded), false, 0.0);
        if !generated.is_empty() {
            let eligible_ids: Vec<&str> = eligible.iter().map(|island| island.id.as_str()).collect();
            let mut merged = generated;
            merged.extend(
                contacts
                    .into_iter()
                    .filter(|candidate| !eligible_ids.contains(&candidate.id.as_str())),
            );
            contacts = merged;
        }
    }

    // Step 2: deduplicate.
    let mut contacts = candidates::deduplicate_candidates(contacts, settings);

    // Step 3: the tip's real surface normal. `placeOneCandidate` skips this for
    // a grid point, which carries the region's exact surface position and
    // normal from the classifier's own triangles, and resolves it for every
    // other candidate.
    for candidate in &mut contacts {
        if candidate.grid_point {
            continue;
        }
        let ids = &regions[candidate.region_index].triangle_ids;
        let (point, normal) = grid::resolve_surface_normal(&welded, ids, candidate.tip_pos);
        candidate.tip_pos = point;
        candidate.tip_normal = normal;
    }

    contacts.into_iter().map(Contact::from_candidate).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A horizontal quad facing down at `z` — a flat ceiling (0° from
    /// horizontal, normal −Z).
    fn ceiling(size: f32, z: f32) -> MeshTriangles {
        let h = size / 2.0;
        let mut mesh = MeshTriangles::new();
        mesh.push(
            [-h, -h, z],
            [h, h, z],
            [h, -h, z],
        );
        mesh.push(
            [-h, -h, z],
            [-h, h, z],
            [h, h, z],
        );
        mesh
    }

    /// A slope of `angle_deg` from horizontal, `size` wide in X and `size` in
    /// Y-run, facing down.
    fn slope(size: f32, angle_deg: f32, base_z: f32) -> MeshTriangles {
        let h = size / 2.0;
        let rise = size * angle_deg.to_radians().tan();
        let mut mesh = MeshTriangles::new();
        mesh.push([-h, -h, base_z], [-h, h, base_z + rise], [h, -h, base_z]);
        mesh.push([h, -h, base_z], [-h, h, base_z + rise], [h, h, base_z + rise]);
        mesh
    }

    /// A vertical wall facing +X.
    fn wall(height: f32, width: f32) -> MeshTriangles {
        let mut mesh = MeshTriangles::new();
        mesh.push([0.0, -width / 2.0, 5.0], [0.0, width / 2.0, 5.0], [0.0, width / 2.0, 5.0 + height]);
        mesh.push([0.0, -width / 2.0, 5.0], [0.0, width / 2.0, 5.0 + height], [0.0, -width / 2.0, 5.0 + height]);
        mesh
    }

    #[test]
    fn a_flat_ceiling_is_gridded() {
        let settings = SupportSettings::default();
        let contacts = place(&ceiling(10.0, 10.0), &settings);

        assert!(!contacts.is_empty(), "a 100 mm² flat ceiling needs contacts");
        assert!(
            contacts.iter().filter(|c| c.grid_point).count() >= 36,
            "the lattice covers the region: {} contacts",
            contacts.len()
        );
        for contact in &contacts {
            assert!(
                (contact.position.z - 10.0).abs() < 1e-3,
                "contact sits on the ceiling: {}",
                contact.position.z
            );
            assert!(
                (contact.normal - Vec3::NEG_Z).length() < 1e-3,
                "flat ceiling normal is −Z: {}",
                contact.normal
            );
            assert_eq!(contact.support_type, ContactSupportType::Trunk);
            assert_eq!(contact.region_index, 0);
        }
    }

    /// A flat at exactly the self-support angle is not an overhang: the
    /// classifier's test is `normal.z < -cos(θ)`, so the boundary itself is
    /// self-supporting. It is also under the 150 mm² steep-flat floor.
    #[test]
    fn a_ceiling_at_the_self_support_angle_gets_nothing() {
        let settings = SupportSettings::default();
        // 8 × 8 mm: 64 mm² projected, so a misclassification would show up as
        // grid contacts, but 90.5 mm² of surface — well under the 150 mm²
        // steep-flat floor, so the topple path cannot mask the answer.
        let contacts = place(&slope(8.0, 45.0, 10.0), &settings);
        assert!(
            contacts.is_empty(),
            "45° is the threshold, not past it: {} contacts",
            contacts.len()
        );
    }

    #[test]
    fn a_shallow_overhang_is_gridded_with_its_own_face_normal() {
        let settings = SupportSettings::default();
        let contacts = place(&slope(10.0, 30.0, 10.0), &settings);

        assert!(!contacts.is_empty(), "a 30° overhang needs contacts");
        let expected_z = 30.0f32.to_radians().cos();
        for contact in &contacts {
            assert!(
                (contact.normal.z + expected_z).abs() < 1e-3,
                "the contact leans with the face: {}",
                contact.normal
            );
            // The fixture rises 1/√3 in Z per mm of Y.
            let expected = 10.0 + (contact.position.y + 5.0) * 30.0f32.to_radians().tan();
            assert!(
                (contact.position.z - expected).abs() < 0.05,
                "contact lands on the slope: {} vs {expected}",
                contact.position.z
            );
        }
    }

    #[test]
    fn a_vertical_wall_gets_nothing() {
        let settings = SupportSettings::default();
        let contacts = place(&wall(20.0, 20.0), &settings);
        assert!(contacts.is_empty(), "a vertical wall is self-supporting");
    }

    /// A 60° face is past the self-support angle, but it forms on its own; the
    /// classifier only keeps one as a topple lever at 150 mm² or more, and this
    /// patch is 25 mm² of surface.
    #[test]
    fn a_small_steep_overhang_gets_nothing() {
        let settings = SupportSettings::default();
        let contacts = place(&slope(5.0, 60.0, 10.0), &settings);
        assert!(
            contacts.is_empty(),
            "a small steep face is self-supporting: {} contacts",
            contacts.len()
        );
    }

    #[test]
    fn a_near_plate_contact_is_a_stump() {
        let settings = SupportSettings::default();
        let contacts = place(&ceiling(10.0, 2.0), &settings);
        assert!(!contacts.is_empty());
        for contact in &contacts {
            assert_eq!(
                contact.support_type,
                ContactSupportType::Stump,
                "tip height {} is under ANCHOR_HEIGHT_THRESHOLD_MM",
                contact.position.z
            );
        }
    }

    #[test]
    fn placement_is_deterministic() {
        let settings = SupportSettings::default();
        let mesh = slope(10.0, 30.0, 10.0);
        let first = place(&mesh, &settings);
        let second = place(&mesh, &settings);
        assert_eq!(first.len(), second.len());
        for (a, b) in first.iter().zip(second.iter()) {
            assert_eq!(a.position, b.position);
            assert_eq!(a.normal, b.normal);
            assert_eq!(a.support_type, b.support_type);
        }
    }

    #[test]
    fn a_disabled_block_places_nothing() {
        let settings = SupportSettings {
            enabled: false,
            ..SupportSettings::default()
        };
        assert!(place(&ceiling(10.0, 10.0), &settings).is_empty());
    }

    #[test]
    fn an_empty_mesh_places_nothing() {
        assert!(place(&MeshTriangles::new(), &SupportSettings::default()).is_empty());
    }

    /// The real-mesh smoke run. Not part of the default suite: the file is a
    /// developer download, not a fixture.
    #[test]
    #[ignore = "needs C:/Users/Paul/Downloads/poussin.stl"]
    fn poussin_places_contacts() {
        let path = "C:/Users/Paul/Downloads/poussin.stl";
        let Ok(bytes) = std::fs::read(path) else {
            eprintln!("skipping: {path} is not present");
            return;
        };
        let mesh = parse_stl(&bytes);
        assert!(!mesh.is_empty(), "{path} holds no triangles");

        let settings = SupportSettings::default();
        let contacts = place(&mesh, &settings);
        eprintln!(
            "{} triangles → {} contacts ({} grid)",
            mesh.len(),
            contacts.len(),
            contacts.iter().filter(|c| c.grid_point).count()
        );
        assert!(!contacts.is_empty(), "a real model needs supports");

        // Same mesh, same settings, same contacts.
        let again = place(&mesh, &settings);
        assert_eq!(contacts.len(), again.len());
        for (a, b) in contacts.iter().zip(again.iter()) {
            assert_eq!(a.position, b.position);
        }
    }

    /// Binary STL, or ASCII when the byte count does not fit the binary shape.
    fn parse_stl(bytes: &[u8]) -> MeshTriangles {
        if bytes.len() >= 84 {
            let count = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
            if 84 + count * 50 == bytes.len() {
                let mut mesh = MeshTriangles::new();
                for i in 0..count {
                    let base = 84 + i * 50 + 12; // skip the stored normal
                    let mut corners = [[0f32; 3]; 3];
                    for (corner, slot) in corners.iter_mut().enumerate() {
                        for axis in 0..3 {
                            let at = base + corner * 12 + axis * 4;
                            slot[axis] = f32::from_le_bytes([
                                bytes[at],
                                bytes[at + 1],
                                bytes[at + 2],
                                bytes[at + 3],
                            ]);
                        }
                    }
                    mesh.push(corners[0], corners[1], corners[2]);
                }
                return mesh;
            }
        }

        let text = String::from_utf8_lossy(bytes);
        let mut mesh = MeshTriangles::new();
        let mut corners: Vec<[f32; 3]> = Vec::new();
        for line in text.lines() {
            let mut parts = line.split_whitespace();
            if parts.next() != Some("vertex") {
                continue;
            }
            let mut read = || {
                parts
                    .next()
                    .and_then(|v| v.parse::<f32>().ok())
                    .unwrap_or(0.0)
            };
            corners.push([read(), read(), read()]);
            if corners.len() == 3 {
                mesh.push(corners[0], corners[1], corners[2]);
                corners.clear();
            }
        }
        mesh
    }
}
