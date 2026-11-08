//! The WGSL programs the viewport builds its pipelines from.

pub(super) const SPRITE_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
    headlight_dir: vec3<f32>,
};
struct Sprite {
    right: vec3<f32>,
    size: f32,
    up: vec3<f32>,
    _pad: f32,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var<uniform> sprite: Sprite;
@group(2) @binding(0) var tex: texture_2d<f32>;
@group(2) @binding(1) var samp: sampler;

struct SpriteOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@location(0) center: vec3<f32>, @location(1) uv: vec2<f32>) -> SpriteOut {
    var out: SpriteOut;
    let world = center
        + sprite.right * (uv.x - 0.5) * sprite.size
        + sprite.up * (0.5 - uv.y) * sprite.size;
    out.position = uniforms.view_proj * vec4<f32>(world, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: SpriteOut) -> @location(0) vec4<f32> {
    return textureSample(tex, samp, in.uv);
}
"#;

pub(super) const MIP_SHADER: &str = r#"
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

struct MipOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> MipOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let p = positions[index];
    var out: MipOut;
    out.position = vec4<f32>(p, 0.0, 1.0);
    out.uv = vec2<f32>((p.x + 1.0) * 0.5, (1.0 - p.y) * 0.5);
    return out;
}

@fragment
fn fs_main(in: MipOut) -> @location(0) vec4<f32> {
    return textureSample(tex, samp, in.uv);
}
"#;

pub(super) const DECAL_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
    headlight_dir: vec3<f32>,
};
struct Decal {
    flip: f32,
    opacity: f32,
    _pad0: f32,
    _pad1: f32,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var<uniform> decal: Decal;
@group(2) @binding(0) var tex: texture_2d<f32>;
@group(2) @binding(1) var samp: sampler;

struct DecalOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>) -> DecalOut {
    var out: DecalOut;
    out.position = uniforms.view_proj * vec4<f32>(position, 1.0);
    var u = uv;
    if (decal.flip > 0.5) {
        u = vec2<f32>(1.0 - u.x, 1.0 - u.y);
    }
    out.uv = u;
    return out;
}

@fragment
fn fs_main(in: DecalOut) -> @location(0) vec4<f32> {
    let texel = textureSample(tex, samp, in.uv);
    return vec4<f32>(texel.rgb, texel.a * decal.opacity);
}
"#;

/// The supports' flat material.
///
/// `SupportProxyMeshLayer` draws with `meshBasicMaterial` — no lighting, one
/// colour. The vertex stage carries each instance's transform and taper: the
/// unit cylinder spans `y = -0.5..0.5` at radius `0.5`, so `taper` narrows the
/// `+y` (the `to`) end without a second mesh. Nodes and disks share the material
/// and pass `1.0`.
pub(super) const SUPPORT_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
    headlight_dir: vec3<f32>,
};
struct SupportColor {
    color: vec3<f32>,
    opacity: f32,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var<uniform> support: SupportColor;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) column0: vec4<f32>,
    @location(2) column1: vec4<f32>,
    @location(3) column2: vec4<f32>,
    @location(4) column3: vec4<f32>,
    @location(5) taper: f32,
) -> VertexOut {
    let instance = mat4x4<f32>(column0, column1, column2, column3);
    let height = clamp(position.y + 0.5, 0.0, 1.0);
    let width = mix(1.0, taper, height);
    let local = vec3<f32>(position.x * width, position.y, position.z * width);
    var out: VertexOut;
    out.position = uniforms.view_proj * instance * vec4<f32>(local, 1.0);
    return out;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(support.color, support.opacity);
}
"#;

pub(super) const MESH_SHADER: &str = r#"

struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
    headlight_dir: vec3<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct Model {
    matrix: mat4x4<f32>,
    clip: vec4<f32>,
    bounds_min: vec4<f32>,
    bounds_max: vec4<f32>,
    stripe: vec4<f32>,
};
@group(1) @binding(0) var<uniform> model: Model;

/// `StlMesh.tsx`'s out-of-bounds material: diagonal bands of the stripe colour
/// over white, at `stripeFreq 0.22`.
fn hazard_stripe(world: vec3<f32>, color: vec3<f32>) -> vec3<f32> {
    let band = step(0.5, fract((world.x + world.y + world.z) * 0.22));
    return mix(color, vec3<f32>(1.0), band);
}

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) world: vec3<f32>,
    @location(2) ao: f32,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) ao: f32,
) -> VertexOut {
    var out: VertexOut;
    let world = model.matrix * vec4<f32>(position, 1.0);
    out.position = uniforms.view_proj * world;
    out.normal = (model.matrix * vec4<f32>(normal, 0.0)).xyz;
    out.world = world.xyz;
    out.ao = ao;
    return out;
}

// three.js BRDF_GGX: D_GGX * V_GGX_SmithCorrelated * F_Schlick.
fn brdf_ggx(light_dir: vec3<f32>, view_dir: vec3<f32>, n: vec3<f32>, roughness: f32, f0: vec3<f32>) -> vec3<f32> {
    let h = normalize(light_dir + view_dir);
    let n_dot_l = saturate(dot(n, light_dir));
    let n_dot_v = saturate(dot(n, view_dir));
    let n_dot_h = saturate(dot(n, h));
    let v_dot_h = saturate(dot(view_dir, h));

    let alpha = roughness * roughness;
    let a2 = alpha * alpha;
    let d = (n_dot_h * a2 - n_dot_h) * n_dot_h + 1.0;
    let distribution = a2 / (3.14159265 * d * d);

    let gv = n_dot_l * sqrt(n_dot_v * n_dot_v * (1.0 - a2) + a2);
    let gl = n_dot_v * sqrt(n_dot_l * n_dot_l * (1.0 - a2) + a2);
    let visibility = 0.5 / max(gv + gl, 0.0001);

    let f = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - v_dot_h, 5.0);
    return distribution * visibility * f;
}

// `ACESFilmicToneMapping`, exposure 1.
fn aces_filmic(color: vec3<f32>) -> vec3<f32> {
    let input_mat = mat3x3<f32>(
        0.59719, 0.07600, 0.02840,
        0.35458, 0.90834, 0.13383,
        0.04823, 0.01566, 0.83777,
    );
    let output_mat = mat3x3<f32>(
         1.60475, -0.10208, -0.00327,
        -0.53108,  1.10813, -0.07276,
        -0.07367, -0.00605,  1.07602,
    );
    var c = color * (1.0 / 0.6);
    c = input_mat * c;
    let a = c * (c + 0.0245786) - 0.000090537;
    let b = c * (0.983729 * c + 0.4329510) + 0.238081;
    c = a / b;
    c = output_mat * c;
    return saturate(c);
}

// three.js BRDF_GGX, the dc9dacee calibration.
fn specular(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, roughness: f32, f0: vec3<f32>) -> vec3<f32> {
    let h = normalize(v + l);
    let n_dot_l = max(dot(n, l), 0.0);
    let n_dot_v = max(dot(n, v), 0.0);
    let n_dot_h = max(dot(n, h), 0.0);
    let v_dot_h = max(dot(v, h), 0.0);

    let alpha = roughness * roughness;
    let a2 = alpha * alpha;
    let d = (n_dot_h * a2 - n_dot_h) * n_dot_h + 1.0;
    let distribution = a2 / (3.14159265 * d * d);

    let gv = n_dot_l * sqrt(n_dot_v * n_dot_v * (1.0 - a2) + a2);
    let gl = n_dot_v * sqrt(n_dot_l * n_dot_l * (1.0 - a2) + a2);
    let visibility = 0.5 / max(gv + gl, 0.0001);

    let f = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - v_dot_h, 5.0);
    return distribution * visibility * f;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    if (in.world.z > model.clip.x) {
        discard;
    }
    let n = normalize(in.normal);
    let view_dir = normalize(uniforms.camera_pos - in.world);
    // The headlight is a directional light along the view axis (`ViewHeadlight`
    // targets `position + viewDir * 100`); `headlight_dir` is the scene-to-
    // camera vector, i.e. the surface-to-light direction. Using the uniform
    // keeps the lighting stable when the ortho dolly carries the camera
    // through the model, where `camera_pos - world` flips sign per fragment.
    let head_dir = normalize(uniforms.headlight_dir);

    // `Lights` in SceneEnvironment.tsx with `useSceneCollectionManager`'s
    // defaults: ambient 0.28, directional 1.12 from +Z, its 0.15 fill from -Z
    // in #90a7ff, a #f6e8ff/#3e415c hemisphere at 0.6x ambient, and the
    // camera-forward headlight at 1.0.
    let key_dir = normalize(vec3<f32>(0.0, 0.0, 1.0));
    let fill_dir = normalize(vec3<f32>(0.0, 0.0, -1.0));
    let sky = vec3<f32>(0.965, 0.910, 1.0);
    let ground = vec3<f32>(0.243, 0.255, 0.361);
    let hemi = mix(ground, sky, n.z * 0.5 + 0.5) * 0.168;

    let ambient = vec3<f32>(0.28);
    let key = vec3<f32>(1.0) * 1.12 * max(dot(n, key_dir), 0.0);
    let fill = vec3<f32>(0.565, 0.655, 1.0) * 0.168 * max(dot(n, fill_dir), 0.0);
    let head = vec3<f32>(1.0) * 1.0 * max(dot(n, head_dir), 0.0);
    let irradiance = ambient + hemi + key + fill + head;

    let base = uniforms.base_color;
    let f0 = mix(vec3<f32>(0.04), base, 0.02);
    // `DEFAULT_MATERIAL_ROUGHNESS` is 0.55 (the 0.9 is only the prop fallback),
    // metalness 0.02. No extra multiplier: the old 1.5 boost was the hotspot.
    let spec = specular(n, view_dir, key_dir, 0.55, f0) * 1.12 * max(dot(n, key_dir), 0.0)
        + specular(n, view_dir, fill_dir, 0.55, f0) * 0.168 * max(dot(n, fill_dir), 0.0)
        + specular(n, view_dir, head_dir, 0.55, f0) * 1.0 * max(dot(n, head_dir), 0.0);
    let n_dot_l = max(dot(n, key_dir), 0.0);
    let cavity = pow(1.0 - n_dot_l, 1.35);
    let fake_ao = 1.0 - cavity * 0.2;
    let baked = mix(1.0, clamp(in.ao, 0.0, 1.0), 0.85);
    var color = base * irradiance / 3.14159265 * fake_ao * baked + spec;
    // `StlMesh`'s out-of-bounds overlay, folded into this pass instead of a
    // second draw: the source composites `stripeColor` at 42% over the surface,
    // which for an opaque model is a 42% mix. The volume is a uniform compare,
    // so a model that fits pays one branch.
    if (model.stripe.w > 0.5) {
        let outside = any(in.world < model.bounds_min.xyz) || any(in.world > model.bounds_max.xyz);
        if (outside) {
            color = mix(color, hazard_stripe(in.world, model.stripe.rgb), 0.42);
        }
    }
    return vec4<f32>(aces_filmic(color), 1.0);
}
"#;

pub(super) const AXIS_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
    headlight_dir: vec3<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct AxisOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) color: vec3<f32>) -> AxisOut {
    var out: AxisOut;
    out.position = uniforms.view_proj * vec4<f32>(position, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: AxisOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
"#;

/// The gizmo's vertex-colour shader with an alpha channel. The handles carry the
/// source's own opacities per vertex (`0.55` for the centre disc, `0.72` for a
/// ring at rest, `0.26` of that for the back arc, `0.15` for a handle dimmed by
/// a running gesture), and the pipeline blends them over the model and the grid
/// exactly where three.js blends its `transparent` `meshBasicMaterial`s.
pub(super) const GIZMO_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
    headlight_dir: vec3<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct GizmoOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) color: vec4<f32>) -> GizmoOut {
    var out: GizmoOut;
    out.position = uniforms.view_proj * vec4<f32>(position, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: GizmoOut) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

pub(super) const VOLUME_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
    headlight_dir: vec3<f32>,
};
struct Fill {
    color: vec3<f32>,
    opacity: f32,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var<uniform> fill: Fill;

fn aces_filmic(color: vec3<f32>) -> vec3<f32> {
    let input_mat = mat3x3<f32>(
        0.59719, 0.07600, 0.02840,
        0.35458, 0.90834, 0.13383,
        0.04823, 0.01566, 0.83777,
    );
    let output_mat = mat3x3<f32>(
         1.60475, -0.10208, -0.00327,
        -0.53108,  1.10813, -0.07276,
        -0.07367, -0.00605,  1.07602,
    );
    var c = color;
    c = input_mat * c;
    let a = c * (c + 0.0245786) - 0.000090537;
    let b = c * (0.983729 * c + 0.4329510) + 0.238081;
    c = a / b;
    c = output_mat * c;
    return saturate(c);
}

fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
    let cutoff = step(c, vec3<f32>(0.0031308));
    let low = c * 12.92;
    let high = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055;
    return mix(high, low, cutoff);
}

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return uniforms.view_proj * vec4<f32>(position, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    // This pass renders into the non-sRGB view of the MSAA samples so the
    // transparent box blends in sRGB space, as three.js does on its RGBA8
    // canvas; the shader therefore writes sRGB-encoded values.
    return vec4<f32>(linear_to_srgb(aces_filmic(fill.color)), fill.opacity);
}
"#;

pub(super) const PLATE_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
    headlight_dir: vec3<f32>,
};
struct Plate {
    color: vec3<f32>,
    opacity: f32,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var<uniform> plate: Plate;

// three.js BRDF_GGX: D_GGX * V_GGX_SmithCorrelated * F_Schlick.
fn brdf_ggx(light_dir: vec3<f32>, view_dir: vec3<f32>, n: vec3<f32>, roughness: f32, f0: vec3<f32>) -> vec3<f32> {
    let h = normalize(light_dir + view_dir);
    let n_dot_l = saturate(dot(n, light_dir));
    let n_dot_v = saturate(dot(n, view_dir));
    let n_dot_h = saturate(dot(n, h));
    let v_dot_h = saturate(dot(view_dir, h));

    let alpha = roughness * roughness;
    let a2 = alpha * alpha;
    let d = (n_dot_h * a2 - n_dot_h) * n_dot_h + 1.0;
    let distribution = a2 / (3.14159265 * d * d);

    let gv = n_dot_l * sqrt(n_dot_v * n_dot_v * (1.0 - a2) + a2);
    let gl = n_dot_v * sqrt(n_dot_l * n_dot_l * (1.0 - a2) + a2);
    let visibility = 0.5 / max(gv + gl, 0.0001);

    let f = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - v_dot_h, 5.0);
    return distribution * visibility * f;
}

// `ACESFilmicToneMapping`, exposure 1.
fn aces_filmic(color: vec3<f32>) -> vec3<f32> {
    let input_mat = mat3x3<f32>(
        0.59719, 0.07600, 0.02840,
        0.35458, 0.90834, 0.13383,
        0.04823, 0.01566, 0.83777,
    );
    let output_mat = mat3x3<f32>(
         1.60475, -0.10208, -0.00327,
        -0.53108,  1.10813, -0.07276,
        -0.07367, -0.00605,  1.07602,
    );
    var c = color * (1.0 / 0.6);
    c = input_mat * c;
    let a = c * (c + 0.0245786) - 0.000090537;
    let b = c * (0.983729 * c + 0.4329510) + 0.238081;
    c = a / b;
    c = output_mat * c;
    return saturate(c);
}

struct PlateOut {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) world: vec3<f32>,
};

@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) normal: vec3<f32>) -> PlateOut {
    var out: PlateOut;
    out.position = uniforms.view_proj * vec4<f32>(position, 1.0);
    out.normal = normal;
    out.world = position;
    return out;
}

@fragment
fn fs_main(in: PlateOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let view_dir = normalize(uniforms.camera_pos - in.world);

    // `meshStandardMaterial` with the defaults: roughness 1, metalness 0, so F0
    // is the dielectric 0.04 and the diffuse term carries the whole albedo.
    let base = plate.color;
    let f0 = vec3<f32>(0.04);
    let roughness = 1.0;

    let key_dir = vec3<f32>(0.0, 0.0, 1.0);
    let fill_dir = vec3<f32>(0.0, 0.0, -1.0);
    let head_dir = uniforms.headlight_dir;
    // `useSceneCollectionManager`'s defaults: ambient 0.28, directional 1.12,
    // headlight 1.0 (`Lights`'s `?? 1.0`). The hemisphere is ambient * 0.6 and
    // the fill is directional * 0.15; the colours are linear.
    let key_color = vec3<f32>(1.0) * 1.12;
    let fill_color = vec3<f32>(0.2789, 0.3864, 1.0) * 0.168;
    let head_color = vec3<f32>(1.0);
    let sky = vec3<f32>(0.9216, 0.8070, 1.0) * 0.168;
    let ground = vec3<f32>(0.0482, 0.0529, 0.1070) * 0.168;
    let hemi = mix(ground, sky, n.y * 0.5 + 0.5);
    let ambient = vec3<f32>(1.0) * 0.28;

    let irradiance = ambient
        + key_color * max(dot(n, key_dir), 0.0)
        + fill_color * max(dot(n, fill_dir), 0.0)
        + hemi
        + head_color * max(dot(n, head_dir), 0.0);
    var color = base * irradiance / 3.14159265;
    color += brdf_ggx(key_dir, view_dir, n, roughness, f0) * key_color * max(dot(n, key_dir), 0.0);
    color += brdf_ggx(fill_dir, view_dir, n, roughness, f0) * fill_color * max(dot(n, fill_dir), 0.0);
    color += brdf_ggx(head_dir, view_dir, n, roughness, f0) * head_color * max(dot(n, head_dir), 0.0);
    return vec4<f32>(aces_filmic(color), plate.opacity);
}
"#;

pub(super) const FOOTPRINT_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
    headlight_dir: vec3<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(@location(0) position: vec2<f32>) -> @builtin(position) vec4<f32> {
    return uniforms.view_proj * vec4<f32>(position, 0.05, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.58, 0.18, 0.024, 1.0);
}
"#;

pub(super) const BACKGROUND_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
    headlight_dir: vec3<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0),
    );
    return vec4<f32>(corners[index], 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    // The canvas is transparent over `backgroundColor: var(--surface-0)`;
    // #111216 in linear light.
    return vec4<f32>(0.0056, 0.0061, 0.0080, 1.0);
}
"#;

pub(super) const COMPOSITE_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    base_color: vec3<f32>,
    resolution: vec2<f32>,
    headlight_dir: vec3<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var scene_tex: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0),
    );
    return vec4<f32>(corners[index], 0.0, 1.0);
}

fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
    let cutoff = step(c, vec3<f32>(0.0031308));
    let low = c * 12.92;
    let high = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055;
    return mix(high, low, cutoff);
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let cutoff = step(c, vec3<f32>(0.04045));
    let low = c / 12.92;
    let high = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return mix(high, low, cutoff);
}

@fragment
fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = frag.xy / uniforms.resolution;
    let scene = textureSampleLevel(scene_tex, samp, uv, 0.0).rgb;

    // `SceneMoodOverlay`: two `mix-blend-mode: screen` layers over the canvas,
    // composited in sRGB the way CSS does it.
    let pink = vec3<f32>(1.0, 0.216, 0.667);   // #ff37aa
    let violet = vec3<f32>(0.435, 0.2, 1.0);   // #6f33ff
    var color = linear_to_srgb(scene);

    // radial-gradient(120% 95% at 50% 46%, transparent 56%, #ff37aa/18% 100%),
    // opacity 0.75.
    let radial = length((uv - vec2<f32>(0.5, 0.46)) / vec2<f32>(1.2, 0.95));
    let radial_alpha = clamp((radial - 0.56) / (1.0 - 0.56), 0.0, 1.0) * 0.18 * 0.75;
    color = mix(color, vec3<f32>(1.0) - (vec3<f32>(1.0) - color) * (vec3<f32>(1.0) - pink), radial_alpha);

    // linear-gradient(180deg, #ff37aa/10% 0%, #6f33ff/6% 40%, transparent 100%),
    // opacity 0.8. CSS interpolates gradients premultiplied, so the colour is
    // recovered by dividing the premultiplied mix back out.
    var layer_color = violet;
    var layer_alpha = 0.0;
    if (uv.y < 0.4) {
        let k = uv.y / 0.4;
        let alpha = 0.10 + (0.06 - 0.10) * k;
        let premultiplied = pink * 0.10 * (1.0 - k) + violet * 0.06 * k;
        layer_color = premultiplied / alpha;
        layer_alpha = alpha;
    } else {
        let k = (uv.y - 0.4) / 0.6;
        layer_color = violet;
        layer_alpha = 0.06 * (1.0 - k);
    }
    layer_alpha *= 0.8;
    color = mix(color, vec3<f32>(1.0) - (vec3<f32>(1.0) - color) * (vec3<f32>(1.0) - layer_color), layer_alpha);

    return vec4<f32>(srgb_to_linear(color), 1.0);
}
"#;
