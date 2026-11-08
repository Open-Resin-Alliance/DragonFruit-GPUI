//! Decoding the `SUPP` chunk into the draw batches.
//!
//! `codec-v2.ts` writes the chunk as `JSON.stringify(input.supports)`, where
//! `supports` is `DragonfruitImportFormat` (`src/supports/types.ts`): one array
//! per support type (`roots`, `trunks`, `branches`, `leaves`, `twigs`, `sticks`,
//! `braces`, `stumps`, `kickstands`) plus the shared `knots`.
//!
//! The walk below follows `collectProxyPrimitives`
//! (`src/supports/SupportProxyMeshLayer.tsx`) and the per-type recipes it calls
//! (`src/supports/SupportTypes/*/…ProxyGeometry.ts`), because those are the
//! recipes that describe what one support set draws as instanced primitives.
//! Each recipe pushes shafts, joints (spheres), roots (a disk, a frustum and a
//! top ball) and contact cones; this module turns the same records into the four
//! batches [`SupportBatches`] draws. A root is not a fifth batch: its disk and
//! frustum are shaft instances and its top ball is a node, so a root costs no
//! extra draw call.
//!
//! # Fidelity notes
//!
//! - A curved (`bezier`) segment becomes its straight chord. The source draws a
//!   `TubeGeometry` warped along the curve; the port has no tube generator yet,
//!   so the endpoints and diameter are kept and the control points are ignored.
//! - The contact cone's nib is drawn as the flat disk [`DiskInstance`] models.
//!   The source's disk has a small cylinder body; at 0.1–0.35 mm thick it is
//!   sub-pixel at plate zoom and the port draws the pad only. Its
//!   `penetrationMm` pull-back is ignored: the pad sits at
//!   `pos + surfaceNormal * thickness / 2`.
//! - The knot spheres come from the `knots` array itself, at `KnotRenderer`'s
//!   blend (`diameter - JOINT_DIAMETER_OFFSET_MM`). Branch and leaf recipes
//!   re-push their parent knot at the joint blend in the source; doing that here
//!   would draw the same sphere twice, so the parent knot is skipped.
//! - A brace with a `curve` is drawn between its two knots, not along its bezier.
//!
//! Every constant below cites the source line it comes from.

use std::collections::{HashMap, HashSet};

use glam::Vec3;
use serde_json::{Map, Value};

use crate::instance::{DiskInstance, NodeInstance, ShaftInstance, SupportBatches};

/// `ROOT_RENDER_LIFT_MM` in `SupportPrimitives/Roots/InstancedRootsGroup.tsx`:
/// the whole root stack sits this far above the plate so its bottom face does
/// not z-fight with the plate.
pub const ROOT_RENDER_LIFT_MM: f32 = 0.01;

/// `JOINT_DIAMETER_OFFSET_MM` in `src/supports/constants.ts`.
pub const JOINT_DIAMETER_OFFSET_MM: f32 = 0.1;

/// `PROXY_JOINT_DIAMETER_BLEND_MM` in `SupportProxyMeshLayer.tsx` — the sphere
/// is this much narrower than the joint it draws.
const PROXY_JOINT_DIAMETER_BLEND_MM: f32 = JOINT_DIAMETER_OFFSET_MM * 0.75;

/// `KnotRenderer.tsx`: `knot.diameter ?? 1.2`.
const KNOT_FALLBACK_DIAMETER_MM: f32 = 1.2;

/// `StumpProxyGeometry`: a stump's inline root always has a 0.1 mm disk.
const STUMP_DISK_HEIGHT_MM: f32 = 0.1;

/// The source's fallback end for a segment with no top joint and no contact
/// cone: `currentStart.z + 5`.
const SEGMENT_FALLBACK_MM: f32 = 5.0;

/// `DEFAULT_MIN_DISK_THICKNESS_MM` in `ContactDisk/contactDiskUtils.ts`.
const DISK_DEFAULT_THICKNESS_MM: f32 = 0.1;

/// `DEFAULT_STANDOFF_ANGLE_THRESHOLD_RAD` — 45°.
const DISK_DEFAULT_STANDOFF_THRESHOLD_RAD: f32 = std::f32::consts::FRAC_PI_4;

/// `DEFAULT_LEGACY_MAX_STANDOFF_MM`.
const DISK_DEFAULT_MAX_STANDOFF_MM: f32 = 1.5;

/// `DEFAULT_LEGACY_CLAMPED_MAX_STANDOFF_MM` — the legacy 1.5 is read as 0.35.
const DISK_LEGACY_CLAMPED_MAX_STANDOFF_MM: f32 = 0.35;

/// `MAX_STANDOFF_ANGLE_RAD` — `PI * 0.5 * 0.9`.
const DISK_MAX_STANDOFF_ANGLE_RAD: f32 = std::f32::consts::PI * 0.5 * 0.9;

/// The only way a `SUPP` document can fail to decode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// The chunk's JSON is not an object and is not `null`. A null or absent
    /// document is an empty support set, not a malformed one.
    NotAnObject,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAnObject => formatter.write_str("the SUPP chunk is not a JSON object"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Decodes a `SUPP` document into the batches the viewport draws.
///
/// Missing or empty collections are an empty support set: a scene without
/// supports, a chunk that only carries `meta`, and a `null` document all decode
/// to `Ok` with no instances. Only a non-object, non-null document is an error.
pub fn parse(json: &Value) -> Result<SupportBatches, ParseError> {
    let mut batches = SupportBatches::default();
    if json.is_null() {
        return Ok(batches);
    }
    let Some(document) = json.as_object() else {
        return Err(ParseError::NotAnObject);
    };

    let roots: HashMap<String, RootAnchor> = index(document, "roots", anchor_root);
    let knots: HashMap<String, KnotAnchor> = index(document, "knots", anchor_knot);
    let mut seen_joints: HashSet<String> = HashSet::new();

    // `KnotRenderer` draws every knot in the store; the proxy recipes only
    // re-push the knots a branch or leaf hangs from. Drawing the collection is
    // both the detailed view and the one place a knot sphere comes from.
    for knot in entities(document, "knots") {
        if let Some(anchor) = anchor_knot(knot) {
            push_knot_node(&mut batches, &anchor);
        }
    }

    // A trunk owns the root it stands on; without that root the recipe returns
    // before pushing anything, so the whole trunk is skipped.
    for trunk in entities(document, "trunks") {
        let model_id = model_id_of(trunk);
        let Some(root) = trunk
            .get("rootId")
            .and_then(Value::as_str)
            .and_then(|id| roots.get(id))
        else {
            continue;
        };
        let segments = trunk.get("segments");
        let first_diameter = segments
            .and_then(Value::as_array)
            .and_then(|segments| segments.first())
            .and_then(|segment| number(segment.get("diameter")));
        push_root_stack(
            &mut batches,
            root,
            first_diameter.unwrap_or(root.diameter),
        );
        let cone = trunk.get("contactCone");
        push_contact_cone(&mut batches, cone);
        let mut start = root_top(root);
        push_segments(
            &mut batches,
            segments,
            &mut start,
            cone.and_then(final_socket_position),
            None,
            model_id,
            &mut seen_joints,
            false,
        );
    }

    for branch in entities(document, "branches") {
        let model_id = model_id_of(branch);
        let Some(parent) = branch
            .get("parentKnotId")
            .and_then(Value::as_str)
            .and_then(|id| knots.get(id))
        else {
            continue;
        };
        let cone = branch.get("contactCone");
        push_contact_cone(&mut batches, cone);
        let mut start = parent.position;
        push_segments(
            &mut batches,
            branch.get("segments"),
            &mut start,
            cone.and_then(final_socket_position),
            None,
            model_id,
            &mut seen_joints,
            false,
        );
    }

    for leaf in entities(document, "leaves") {
        let cone = leaf.get("contactCone");
        push_contact_cone(&mut batches, cone);
        let Some(parent) = leaf
            .get("parentKnotId")
            .and_then(Value::as_str)
            .and_then(|id| knots.get(id))
        else {
            continue;
        };
        // The leaf's rod: the cone's socket back to the knot it hangs from.
        if let Some(socket) = cone.and_then(final_socket_position) {
            let diameter = cone
                .and_then(|cone| cone.get("profile"))
                .and_then(|profile| number(profile.get("bodyDiameterMm")))
                .unwrap_or(0.5);
            push_shaft(&mut batches, socket, parent.position, diameter, None);
        }
    }

    for twig in entities(document, "twigs") {
        let model_id = model_id_of(twig);
        let disk_a = twig.get("contactDiskA");
        let disk_b = twig.get("contactDiskB");
        push_contact_disk(&mut batches, disk_a);
        push_contact_disk(&mut batches, disk_b);
        let Some(mut start) = disk_a.and_then(contact_disk_tip) else {
            continue;
        };
        push_segments(
            &mut batches,
            twig.get("segments"),
            &mut start,
            disk_b.and_then(contact_disk_tip),
            None,
            model_id,
            &mut seen_joints,
            true,
        );
    }

    for stick in entities(document, "sticks") {
        let model_id = model_id_of(stick);
        let cone_a = stick.get("contactConeA");
        let cone_b = stick.get("contactConeB");
        push_contact_cone(&mut batches, cone_a);
        push_contact_cone(&mut batches, cone_b);
        let Some(mut start) = cone_a.and_then(final_socket_position) else {
            continue;
        };
        push_segments(
            &mut batches,
            stick.get("segments"),
            &mut start,
            cone_b.and_then(final_socket_position),
            None,
            model_id,
            &mut seen_joints,
            true,
        );
    }

    for stump in entities(document, "stumps") {
        let Some(position) = stump.get("rootPos").and_then(vec3) else {
            continue;
        };
        let base = number(stump.get("rootBaseDiameter")).unwrap_or(0.0);
        let top = number(stump.get("rootTopDiameter")).unwrap_or(base);
        let root = RootAnchor {
            position,
            diameter: base,
            disk_height: STUMP_DISK_HEIGHT_MM,
            cone_height: number(stump.get("rootHeight")).unwrap_or(0.0).max(0.0),
        };
        push_root_stack(&mut batches, &root, top);
        push_contact_cone(&mut batches, stump.get("contactCone"));
    }

    for kickstand in entities(document, "kickstands") {
        let model_id = model_id_of(kickstand);
        let Some(root) = kickstand
            .get("rootId")
            .and_then(Value::as_str)
            .and_then(|id| roots.get(id))
        else {
            continue;
        };
        let Some(host) = kickstand
            .get("hostKnotId")
            .and_then(Value::as_str)
            .and_then(|id| knots.get(id))
        else {
            continue;
        };
        let segments = kickstand.get("segments");
        let first_diameter = segments
            .and_then(Value::as_array)
            .and_then(|segments| segments.first())
            .and_then(|segment| number(segment.get("diameter")));
        push_root_stack(
            &mut batches,
            root,
            first_diameter.unwrap_or(root.diameter),
        );
        // The last segment narrows into the host: `terminalEndDiameterMm`.
        let terminal = kickstand
            .get("profile")
            .and_then(|profile| number(profile.get("terminalEndDiameterMm")));
        let mut start = root_top(root);
        push_segments(
            &mut batches,
            segments,
            &mut start,
            Some(host.position),
            terminal,
            model_id,
            &mut seen_joints,
            false,
        );
    }

    for brace in entities(document, "braces") {
        let Some(start) = brace
            .get("startKnotId")
            .and_then(Value::as_str)
            .and_then(|id| knots.get(id))
        else {
            continue;
        };
        let Some(end) = brace
            .get("endKnotId")
            .and_then(Value::as_str)
            .and_then(|id| knots.get(id))
        else {
            continue;
        };
        let profile_diameter = brace
            .get("profile")
            .and_then(|profile| number(profile.get("diameter")))
            .unwrap_or(1.0);
        push_shaft(
            &mut batches,
            start.position,
            end.position,
            brace_end_diameter(profile_diameter, start),
            Some(brace_end_diameter(profile_diameter, end)),
        );
    }

    Ok(batches)
}

/// A `Roots` entity's geometry: where it stands and how tall each part is.
struct RootAnchor {
    position: Vec3,
    /// `Roots.diameter` — the base of the frustum.
    diameter: f32,
    disk_height: f32,
    cone_height: f32,
}

/// A `Knot`: a point on a host shaft other supports hang from.
struct KnotAnchor {
    position: Vec3,
    /// `Knot.diameter`; absent means the host shaft's width + 0.1 mm.
    diameter: Option<f32>,
}

fn anchor_root(entity: &Value) -> Option<RootAnchor> {
    let transform = entity.get("transform")?;
    let position = vec3_at(transform, "pos")?;
    Some(RootAnchor {
        position,
        diameter: number(entity.get("diameter"))?,
        disk_height: number(entity.get("diskHeight")).unwrap_or(0.0).max(0.0),
        cone_height: number(entity.get("coneHeight")).unwrap_or(0.0).max(0.0),
    })
}

fn anchor_knot(entity: &Value) -> Option<KnotAnchor> {
    Some(KnotAnchor {
        position: vec3_at(entity, "pos")?,
        diameter: number(entity.get("diameter")),
    })
}

/// Index a collection by entity id, skipping records without one.
fn index<T>(
    document: &Map<String, Value>,
    key: &str,
    build: impl Fn(&Value) -> Option<T>,
) -> HashMap<String, T> {
    let mut entries = HashMap::new();
    for entity in entities(document, key) {
        let Some(id) = entity.get("id").and_then(Value::as_str) else {
            continue;
        };
        if let Some(value) = build(entity) {
            entries.insert(id.to_owned(), value);
        }
    }
    entries
}

/// The records of one collection, or nothing when it is absent.
fn entities<'a>(document: &'a Map<String, Value>, key: &str) -> std::slice::Iter<'a, Value> {
    const EMPTY: &[Value] = &[];
    match document.get(key).and_then(Value::as_array) {
        Some(entries) => entries.iter(),
        None => EMPTY.iter(),
    }
}

fn model_id_of(entity: &Value) -> &str {
    entity.get("modelId").and_then(Value::as_str).unwrap_or_default()
}

fn number(value: Option<&Value>) -> Option<f32> {
    let value = value?.as_f64()? as f32;
    value.is_finite().then_some(value)
}

fn vec3(value: &Value) -> Option<Vec3> {
    let object = value.as_object()?;
    Some(Vec3::new(
        number(object.get("x"))?,
        number(object.get("y"))?,
        number(object.get("z"))?,
    ))
}

fn vec3_at(value: &Value, key: &str) -> Option<Vec3> {
    value.get(key).and_then(vec3)
}

/// The world point the trunk's shaft grows from: the top of its root stack.
///
/// The trunk recipes build this from the authored heights and do **not** add
/// `ROOT_RENDER_LIFT_MM` — the lift is only the root stack's own render offset,
/// so the shaft starts a hundredth of a millimetre inside the root's top ball,
/// exactly as the source does.
fn root_top(root: &RootAnchor) -> Vec3 {
    root.position + Vec3::Z * (root.disk_height + root.cone_height)
}

/// The root's three pieces: a disk, a frustum and a top ball.
fn push_root_stack(batches: &mut SupportBatches, root: &RootAnchor, top_diameter: f32) {
    let bottom_radius = (root.diameter * 0.5).max(0.001);
    let top_radius = (top_diameter * 0.5).max(0.001);
    let base = root.position + Vec3::Z * ROOT_RENDER_LIFT_MM;
    let disk_top = base + Vec3::Z * root.disk_height;
    if root.disk_height > 0.0 {
        push_shaft(batches, base, disk_top, bottom_radius * 2.0, None);
    }
    if root.cone_height > 0.0 {
        let cone_top = disk_top + Vec3::Z * root.cone_height;
        push_shaft(
            batches,
            disk_top,
            cone_top,
            bottom_radius * 2.0,
            Some(top_radius * 2.0),
        );
        push_node(batches, cone_top, top_radius);
    }
}

/// Walk one entity's segments, lifting the shared shaft start as joints appear.
///
/// `terminal_taper` is the width at the far end of a segment with no top joint
/// (`KickstandFields.profile.terminalEndDiameterMm`). `joint_diameter_from_segment`
/// is the twig/stick rule: their joints carry the segment's width, not the
/// joint record's.
#[allow(clippy::too_many_arguments)]
fn push_segments(
    batches: &mut SupportBatches,
    segments: Option<&Value>,
    start: &mut Vec3,
    fallback_end: Option<Vec3>,
    terminal_taper: Option<f32>,
    model_id: &str,
    seen_joints: &mut HashSet<String>,
    joint_diameter_from_segment: bool,
) {
    let Some(segments) = segments.and_then(Value::as_array) else {
        return;
    };
    for segment in segments {
        let diameter = number(segment.get("diameter")).unwrap_or(0.0);
        let joint_diameter = joint_diameter_from_segment.then_some(diameter);
        if let Some(bottom) = segment.get("bottomJoint") {
            if let Some(position) = vec3_at(bottom, "pos") {
                *start = position;
            }
            push_joint(batches, seen_joints, model_id, bottom, joint_diameter);
        }
        let top = segment.get("topJoint");
        let end = top
            .and_then(|joint| vec3_at(joint, "pos"))
            .or(fallback_end)
            .unwrap_or(*start + Vec3::Z * SEGMENT_FALLBACK_MM);
        push_shaft(
            batches,
            *start,
            end,
            diameter,
            if top.is_some() { None } else { terminal_taper },
        );
        if let Some(top) = top {
            push_joint(batches, seen_joints, model_id, top, joint_diameter);
        }
        *start = end;
    }
}

fn push_contact_cone(batches: &mut SupportBatches, cone: Option<&Value>) {
    let Some(cone) = cone else {
        return;
    };
    let Some(position) = vec3_at(cone, "pos") else {
        return;
    };
    let Some(normal) = vec3_at(cone, "normal") else {
        return;
    };
    let surface_normal = vec3_at(cone, "surfaceNormal").unwrap_or(normal);
    let profile = cone.get("profile");
    let profile_type = profile
        .and_then(|profile| profile.get("type"))
        .and_then(Value::as_str);
    let contact_diameter = profile
        .and_then(|profile| number(profile.get("contactDiameterMm")))
        .unwrap_or(0.0);
    let body_diameter = profile
        .and_then(|profile| number(profile.get("bodyDiameterMm")))
        .unwrap_or(contact_diameter);
    let thickness = if profile_type == Some("disk") {
        disk_thickness_of(cone, profile, surface_normal, normal)
    } else {
        0.0
    };
    let start = position + surface_normal * thickness;
    if let Some(socket) = final_socket_position(cone) {
        // The body frustum: contact width at the model, body width at the
        // socket (`conePrimitiveScales`' `body` scale, taper in the mesh).
        push_shaft(batches, start, socket, contact_diameter, Some(body_diameter));
    }
    match profile_type {
        // The nib: `getDiskCenter(pos, surfaceNormal, thickness)`.
        Some("disk") => push_disk(
            batches,
            position + surface_normal * (thickness * 0.5),
            surface_normal,
            contact_diameter * 0.5,
        ),
        // A sphere profile's contact primitive replaces the nib.
        Some("sphere") => push_node(batches, start, (contact_diameter * 0.5).max(0.001)),
        _ => {}
    }
}

/// The socket the last shaft of a member ends at.
fn final_socket_position(cone: &Value) -> Option<Vec3> {
    let position = vec3_at(cone, "pos")?;
    let normal = vec3_at(cone, "normal")?;
    let surface_normal = vec3_at(cone, "surfaceNormal").unwrap_or(normal);
    let profile = cone.get("profile");
    let profile_type = profile
        .and_then(|profile| profile.get("type"))
        .and_then(Value::as_str);
    let thickness = if profile_type == Some("disk") {
        disk_thickness_of(cone, profile, surface_normal, normal)
    } else {
        0.0
    };
    let length = profile
        .and_then(|profile| number(profile.get("lengthMm")))
        .unwrap_or(0.0);
    Some(position + surface_normal * thickness + normal * length)
}

/// `calculateDiskThickness`: the nib's thickness from the cone's angle to the
/// surface, floored by `max(diskThicknessMm, contactRadius)`.
fn disk_thickness_of(
    cone: &Value,
    profile: Option<&Value>,
    surface_normal: Vec3,
    cone_axis: Vec3,
) -> f32 {
    if let Some(override_mm) = number(cone.get("diskLengthOverride")) {
        return override_mm;
    }
    let Some(profile) = profile else {
        return DISK_DEFAULT_THICKNESS_MM;
    };
    disk_thickness(surface_normal, cone_axis, profile)
}

fn disk_thickness(surface_normal: Vec3, cone_axis: Vec3, profile: &Value) -> f32 {
    let min_thickness = number(profile.get("diskThicknessMm"))
        .unwrap_or(DISK_DEFAULT_THICKNESS_MM)
        .max(number(profile.get("contactDiameterMm")).unwrap_or(0.0).max(0.0) * 0.5);
    let raw_max = number(profile.get("maxStandoffMm")).unwrap_or(DISK_DEFAULT_MAX_STANDOFF_MM);
    let max_standoff = if (raw_max - DISK_DEFAULT_MAX_STANDOFF_MM).abs() <= f32::EPSILON {
        DISK_LEGACY_CLAMPED_MAX_STANDOFF_MM
    } else {
        raw_max
    };
    let max_thickness = min_thickness.max(max_standoff);
    let threshold = number(profile.get("standoffAngleThreshold"))
        .unwrap_or(DISK_DEFAULT_STANDOFF_THRESHOLD_RAD);

    let (normal_len_sq, axis_len_sq) =
        (surface_normal.length_squared(), cone_axis.length_squared());
    if normal_len_sq < 1e-8 || axis_len_sq < 1e-8 {
        return min_thickness;
    }
    let dot = surface_normal.dot(cone_axis) / (normal_len_sq * axis_len_sq).sqrt();
    let angle = dot.clamp(-1.0, 1.0).acos();
    if angle <= threshold {
        return min_thickness;
    }
    let effective_angle = angle.min(DISK_MAX_STANDOFF_ANGLE_RAD);
    let denominator = (DISK_MAX_STANDOFF_ANGLE_RAD - threshold).max(1e-8);
    let factor = ((effective_angle - threshold) / denominator).clamp(0.0, 1.0);
    min_thickness + factor * (max_thickness - min_thickness)
}

/// A `ContactDisk` (a twig's endpoint) as the flat pad it draws.
fn push_contact_disk(batches: &mut SupportBatches, disk: Option<&Value>) {
    let Some(disk) = disk else {
        return;
    };
    let Some(position) = vec3_at(disk, "pos") else {
        return;
    };
    let Some(surface_normal) = vec3_at(disk, "surfaceNormal") else {
        return;
    };
    let thickness = contact_disk_thickness(disk, surface_normal);
    push_disk(
        batches,
        position + surface_normal * (thickness * 0.5),
        surface_normal,
        number(disk.get("contactDiameterMm")).unwrap_or(0.0) * 0.5,
    );
}

/// Where a contact disk's tip sits: its full standoff along the surface normal.
fn contact_disk_tip(disk: &Value) -> Option<Vec3> {
    let position = vec3_at(disk, "pos")?;
    let surface_normal = vec3_at(disk, "surfaceNormal")?;
    let thickness = contact_disk_thickness(disk, surface_normal);
    Some(position + surface_normal * thickness)
}

fn contact_disk_thickness(disk: &Value, surface_normal: Vec3) -> f32 {
    if let Some(override_mm) = number(disk.get("diskLengthOverride")) {
        return override_mm;
    }
    let Some(profile) = disk.get("profile") else {
        return DISK_DEFAULT_THICKNESS_MM;
    };
    let cone_axis = vec3_at(disk, "coneAxis").unwrap_or(surface_normal);
    disk_thickness(surface_normal, cone_axis, profile)
}

/// `braceSpanDiameters`: the knot's width less the joint offset, capped by the
/// brace's profile width.
fn brace_end_diameter(profile_diameter: f32, knot: &KnotAnchor) -> f32 {
    let profile = profile_diameter.max(0.001);
    let knot_diameter = knot
        .diameter
        .unwrap_or(profile + JOINT_DIAMETER_OFFSET_MM);
    (knot_diameter - JOINT_DIAMETER_OFFSET_MM)
        .max(0.001)
        .min(profile)
}

fn push_knot_node(batches: &mut SupportBatches, knot: &KnotAnchor) {
    let diameter = knot.diameter.unwrap_or(KNOT_FALLBACK_DIAMETER_MM);
    // `KnotRenderer`: `resolvedDiameter - JOINT_DIAMETER_OFFSET_MM`.
    let radius = (diameter - JOINT_DIAMETER_OFFSET_MM).max(0.001) * 0.5;
    push_node(batches, knot.position, radius);
}

fn push_joint(
    batches: &mut SupportBatches,
    seen: &mut HashSet<String>,
    model_id: &str,
    joint: &Value,
    diameter_override: Option<f32>,
) {
    let Some(position) = vec3_at(joint, "pos") else {
        return;
    };
    // `seenJointKeysByModel`: the same joint is the top of one segment and the
    // bottom of the next, and is drawn once.
    if let Some(id) = joint.get("id").and_then(Value::as_str) {
        if !seen.insert(format!("{model_id}\u{0}{id}")) {
            return;
        }
    }
    let diameter = diameter_override
        .or_else(|| number(joint.get("diameter")))
        .unwrap_or(0.0);
    let radius = (diameter - PROXY_JOINT_DIAMETER_BLEND_MM).max(0.001) * 0.5;
    push_node(batches, position, radius);
}

fn push_node(batches: &mut SupportBatches, position: Vec3, radius: f32) {
    if !position.is_finite() || !radius.is_finite() || radius <= 0.0 {
        return;
    }
    batches.nodes.push(NodeInstance { position, radius });
}

fn push_disk(batches: &mut SupportBatches, position: Vec3, normal: Vec3, radius: f32) {
    if !position.is_finite()
        || !normal.is_finite()
        || normal.length_squared() <= 1e-12
        || !radius.is_finite()
        || radius <= 0.0
    {
        return;
    }
    batches.disks.push(DiskInstance {
        position,
        normal,
        radius,
    });
}

fn push_shaft(
    batches: &mut SupportBatches,
    from: Vec3,
    to: Vec3,
    diameter: f32,
    diameter_end: Option<f32>,
) {
    if !from.is_finite() || !to.is_finite() || !diameter.is_finite() || diameter <= 0.0 {
        return;
    }
    // `InstancedShaftGroup` does not draw a zero-length shaft.
    if (to - from).length_squared() <= 1e-8 {
        return;
    }
    batches.push(ShaftInstance {
        from,
        to,
        diameter,
        diameter_end: diameter_end.filter(|end| end.is_finite() && *end >= 0.0),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real excerpt from `NOM_majoraMask_Supported.voxl`'s `SUPP` chunk
    /// (zlib, 24 013 stored bytes → 109 961 inflated). Kept verbatim: the root,
    /// the trunk on it, the two knots a brace spans, and that brace.
    const EXCERPT: &str = r#"{
      "version": 1,
      "meta": {"source": "dragonfruit-voxl-export", "objectCenter": {"x": 0, "y": 0, "z": 0}},
      "roots": [
        {
          "id": "b76bf421-c059-43bd-be88-f8dfa131d6f2",
          "modelId": "258b162f-c9f9-4b11-9e76-92b4201f73cf",
          "transform": {
            "pos": {"x": 7.79546991681309, "y": -13.54782853472119, "z": 1.7763568394002505e-15},
            "rot": {"x": 0, "y": 0, "z": 0, "w": 1}
          },
          "diameter": 3,
          "diskHeight": 0.5,
          "coneHeight": 1.5
        }
      ],
      "trunks": [
        {
          "id": "55a9c65d-f0d3-48c0-bfaa-befbfc915426",
          "modelId": "258b162f-c9f9-4b11-9e76-92b4201f73cf",
          "rootId": "b76bf421-c059-43bd-be88-f8dfa131d6f2",
          "segments": [
            {
              "id": "d9ec231e-0e95-4928-9c99-92292a4c15ba",
              "type": "straight",
              "diameter": 1.15,
              "topJoint": {
                "id": "f786413c-4dd4-4d13-94da-b6be2eea07e1",
                "pos": {"x": 7.79546991681309, "y": -13.54782853472119, "z": 2.0500000000000007},
                "diameter": 1.25
              }
            },
            {
              "id": "95e2258f-5a48-41ee-b576-e02d8c50a61f",
              "type": "straight",
              "diameter": 1.15,
              "bottomJoint": {
                "id": "f786413c-4dd4-4d13-94da-b6be2eea07e1",
                "pos": {"x": 7.79546991681309, "y": -13.54782853472119, "z": 2.0500000000000007},
                "diameter": 1.25
              },
              "topJoint": {
                "id": "0db32be9-f4e0-4bca-af72-f2dc61eef202",
                "pos": {"x": 7.79546991681309, "y": -13.54782853472119, "z": 16.731375268653316},
                "diameter": 1.25
              }
            }
          ],
          "contactCone": {
            "id": "909b62bc-2b7b-4baa-b35c-18e87221ad9b",
            "pos": {"x": 7.795482158660889, "y": -10.814831499620112, "z": 19.340470560402323},
            "normal": {"x": -3.239916395408976e-06, "y": -0.7233125299652803, "z": -0.6905208063373098},
            "surfaceNormal": {"x": -3.239916395408976e-06, "y": -0.7233125299652803, "z": -0.6905208063373098},
            "socketJointId": "0db32be9-f4e0-4bca-af72-f2dc61eef202",
            "profile": {
              "type": "disk",
              "lengthMm": 3.7784455845556346,
              "contactDiameterMm": 0.5,
              "bodyDiameterMm": 1.15,
              "diskThicknessMm": 0.1,
              "maxStandoffMm": 1.5,
              "standoffAngleThreshold": 0.7853981633974483,
              "penetrationMm": 0.1
            }
          },
          "typeId": "trunk"
        }
      ],
      "knots": [
        {
          "id": "df480978-1c36-47aa-bda1-ce32196f9519",
          "parentShaftId": "95e2258f-5a48-41ee-b576-e02d8c50a61f",
          "t": 0.26904839869460995,
          "pos": {"x": 7.79546991681309, "y": -13.54782853472119, "z": 6.000000506665824},
          "diameter": 1.25,
          "normalizationHint": "braceImported"
        },
        {
          "id": "2a549064-5004-40ac-9482-5913e7be559b",
          "parentShaftId": "7eb7930f-f771-4de3-8f94-6d7e9a87c1df",
          "t": 0,
          "pos": {"x": 8.417119945109297, "y": -11.041951172793741, "z": 1.000000506665824},
          "diameter": 1.25,
          "normalizationHint": "braceImported"
        }
      ],
      "braces": [
        {
          "id": "e87e7fcd-4233-494e-bc2f-f0ed431c9933",
          "modelId": "258b162f-c9f9-4b11-9e76-92b4201f73cf",
          "startKnotId": "df480978-1c36-47aa-bda1-ce32196f9519",
          "endKnotId": "2a549064-5004-40ac-9482-5913e7be559b",
          "profile": {"diameter": 0.9},
          "typeId": "brace"
        }
      ]
    }"#;

    fn excerpt() -> SupportBatches {
        parse(&serde_json::from_str(EXCERPT).expect("the excerpt is valid JSON"))
            .expect("the excerpt decodes")
    }

    #[test]
    fn a_real_trunk_decodes_into_the_four_batches() {
        let batches = excerpt();
        // Root disk + the trunk's two segments + the brace.
        assert_eq!(batches.straight_shafts.len(), 4);
        // Root frustum + the contact-cone body.
        assert_eq!(batches.tapered_shafts.len(), 2);
        // Root top ball + the two segment joints + the two knots.
        assert_eq!(batches.nodes.len(), 5);
        // The contact cone's nib.
        assert_eq!(batches.disks.len(), 1);
        assert_eq!(batches.draw_calls(), 4);
    }

    #[test]
    fn the_root_is_a_disk_a_frustum_and_a_ball() {
        let batches = excerpt();
        let disk = batches.straight_shafts[0];
        // `diskHeight` 0.5 from the lifted plate, at the authored 3 mm base.
        assert!((disk.from.z - 0.01).abs() < 1e-4);
        assert!((disk.to.z - 0.51).abs() < 1e-4);
        assert!((disk.diameter - 3.0).abs() < 1e-5);
        assert!(disk.diameter_end.is_none());

        let cone = batches.tapered_shafts[0];
        assert!((cone.from.z - 0.51).abs() < 1e-4);
        assert!((cone.to.z - 2.01).abs() < 1e-4);
        // The frustum narrows to the trunk's first segment, 1.15 mm.
        assert!((cone.diameter - 3.0).abs() < 1e-5);
        assert!((cone.diameter_end.unwrap() - 1.15).abs() < 1e-5);
        assert!((cone.taper() - 1.15 / 3.0).abs() < 1e-5);
    }

    #[test]
    fn the_contact_cone_body_runs_to_the_socket_and_the_nib_sits_at_the_surface() {
        let batches = excerpt();
        let body = batches.tapered_shafts[1];
        // lengthMm 3.7784 along the normal, contact 0.5 → body 1.15.
        let axis = body.to - body.from;
        assert!((axis.length() - 3.7784455845556346).abs() < 1e-4);
        assert!((body.diameter - 0.5).abs() < 1e-6);
        assert!((body.diameter_end.unwrap() - 1.15).abs() < 1e-5);

        // The nib is 0.25 mm thick at the shared normal, so its centre is
        // 0.125 mm off the surface point.
        let disk = batches.disks[0];
        assert!((disk.radius - 0.25).abs() < 1e-6);
        let offset = disk.position
            - Vec3::new(
                7.795482158660889,
                -10.814831499620112,
                19.340470560402323,
            );
        assert!((offset.length() - 0.125).abs() < 1e-5);
    }

    #[test]
    fn the_brace_spans_its_two_knots_at_the_capped_width() {
        let batches = excerpt();
        let brace = *batches.straight_shafts.last().unwrap();
        assert!((brace.from.z - 6.000000506665824).abs() < 1e-4);
        assert!((brace.to.z - 1.000000506665824).abs() < 1e-4);
        // Knot 1.25 less 0.1 = 1.15, capped by the 0.9 profile.
        assert!((brace.diameter - 0.9).abs() < 1e-6);
        assert!((brace.diameter_end.unwrap() - 0.9).abs() < 1e-6);
    }

    #[test]
    fn an_empty_document_is_an_empty_support_set() {
        for document in [serde_json::json!({}), serde_json::json!({"roots": [], "trunks": []})] {
            let batches = parse(&document).expect("empty documents decode");
            assert_eq!(batches.draw_calls(), 0);
            assert!(batches.straight_shafts.is_empty());
            assert!(batches.tapered_shafts.is_empty());
            assert!(batches.nodes.is_empty());
            assert!(batches.disks.is_empty());
        }
        assert_eq!(
            parse(&Value::Null).expect("a null chunk decodes").draw_calls(),
            0
        );
    }

    #[test]
    fn a_trunk_without_its_root_is_skipped() {
        let document = serde_json::json!({
            "trunks": [{
                "id": "t",
                "rootId": "missing",
                "segments": [{"id": "s", "diameter": 1.0}]
            }]
        });
        assert_eq!(parse(&document).expect("decodes").draw_calls(), 0);
    }

    #[test]
    fn a_non_object_document_is_the_one_error() {
        assert_eq!(
            parse(&serde_json::json!([])).unwrap_err(),
            ParseError::NotAnObject
        );
    }
}
