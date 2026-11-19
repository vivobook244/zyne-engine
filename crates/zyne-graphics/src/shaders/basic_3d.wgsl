struct Camera {
    view_projection: mat4x4<f32>,
    camera_position: vec4<f32>,
};

struct DirectionalLight {
    direction: vec4<f32>,
    color_intensity: vec4<f32>,
};

struct PointLight {
    position_range: vec4<f32>,
    color_intensity: vec4<f32>,
};

struct SpotLight {
    position_range: vec4<f32>,
    direction_inner_cos: vec4<f32>,
    color_intensity: vec4<f32>,
    outer_cos_padding: vec4<f32>,
};

struct Lighting {
    ambient_color: vec4<f32>,
    counts: vec4<u32>,
    directional_lights: array<DirectionalLight, 4>,
    point_lights: array<PointLight, 16>,
    spot_lights: array<SpotLight, 8>,
};

struct Material {
    ambient: vec4<f32>,
    diffuse: vec4<f32>,
    specular: vec4<f32>,
};

struct ShadowCamera {
    view_projection: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var checker_texture: texture_2d<f32>;

@group(1) @binding(1)
var checker_sampler: sampler;

@group(2) @binding(0)
var<uniform> lighting: Lighting;

@group(3) @binding(0)
var<uniform> material: Material;

@group(4) @binding(0)
var shadow_map: texture_depth_2d;

@group(4) @binding(1)
var shadow_sampler: sampler_comparison;

@group(4) @binding(2)
var<uniform> shadow_camera: ShadowCamera;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) normal: vec3<f32>,
    @location(4) model_0: vec4<f32>,
    @location(5) model_1: vec4<f32>,
    @location(6) model_2: vec4<f32>,
    @location(7) model_3: vec4<f32>,
    @location(8) normal_0: vec4<f32>,
    @location(9) normal_1: vec4<f32>,
    @location(10) normal_2: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) world_normal: vec3<f32>,
    @location(3) world_position: vec3<f32>,
    @location(4) shadow_position: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let model = mat4x4<f32>(input.model_0, input.model_1, input.model_2, input.model_3);
    let world_position = model * vec4<f32>(input.position, 1.0);
    output.clip_position = camera.view_projection * world_position;
    let normal_matrix = mat3x3<f32>(input.normal_0.xyz, input.normal_1.xyz, input.normal_2.xyz);
    output.color = input.color;
    output.uv = input.uv;
    output.world_normal = normalize(normal_matrix * input.normal);
    output.world_position = world_position.xyz;
    output.shadow_position = shadow_camera.view_projection * world_position;
    return output;
}

@vertex
fn vs_shadow(input: VertexInput) -> @builtin(position) vec4<f32> {
    let model = mat4x4<f32>(input.model_0, input.model_1, input.model_2, input.model_3);
    let world_position = model * vec4<f32>(input.position, 1.0);
    return shadow_camera.view_projection * world_position;
}

fn safe_normalize(value: vec3<f32>, fallback: vec3<f32>) -> vec3<f32> {
    let length_squared = dot(value, value);
    if length_squared > 1.0e-8 {
        return value * inverseSqrt(length_squared);
    }
    return fallback;
}

fn shadow_visibility(shadow_position: vec4<f32>) -> f32 {
    if shadow_position.w <= 0.0 {
        return 1.0;
    }
    let projected = shadow_position.xyz / shadow_position.w;
    let uv = vec2<f32>(projected.x * 0.5 + 0.5, 0.5 - projected.y * 0.5);
    if projected.z <= 0.0 || projected.z >= 1.0
        || uv.x <= 0.0 || uv.x >= 1.0
        || uv.y <= 0.0 || uv.y >= 1.0 {
        return 1.0;
    }
    let dimensions = vec2<f32>(textureDimensions(shadow_map));
    let texel = 1.0 / dimensions;
    let compare_depth = projected.z - 0.0015;
    var visibility = 0.0;
    for (var y = -1; y <= 1; y = y + 1) {
        for (var x = -1; x <= 1; x = x + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel;
            visibility += textureSampleCompare(
                shadow_map,
                shadow_sampler,
                uv + offset,
                compare_depth,
            );
        }
    }
    return visibility / 9.0;
}

fn blinn_phong(
    base_color: vec3<f32>,
    normal: vec3<f32>,
    view_direction: vec3<f32>,
    light_direction: vec3<f32>,
    light_color: vec3<f32>,
    intensity: f32,
    specular_color: vec3<f32>,
    shininess: f32,
) -> vec3<f32> {
    let diffuse_factor = max(dot(normal, light_direction), 0.0);
    var specular_factor = 0.0;
    if diffuse_factor > 0.0 {
        let halfway = light_direction + view_direction;
        let halfway_length_squared = dot(halfway, halfway);
        if halfway_length_squared > 1.0e-8 {
            let halfway_direction = halfway * inverseSqrt(halfway_length_squared);
            specular_factor = pow(max(dot(normal, halfway_direction), 0.0), shininess);
        }
    }
    let diffuse = base_color * diffuse_factor;
    let specular = specular_color * specular_factor;
    return (diffuse + specular) * light_color * intensity;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let sampled_color = textureSample(checker_texture, checker_sampler, input.uv);
    let base_color = sampled_color.rgb * input.color * material.diffuse.rgb;
    let normal = safe_normalize(input.world_normal, vec3<f32>(0.0, 1.0, 0.0));
    let view_direction = safe_normalize(
        camera.camera_position.xyz - input.world_position,
        normal,
    );
    var result = base_color * material.ambient.rgb * lighting.ambient_color.rgb;

    for (var i = 0u; i < lighting.counts.x; i = i + 1u) {
        let light = lighting.directional_lights[i];
        let light_direction = safe_normalize(-light.direction.xyz, normal);
        let direct_light = blinn_phong(
            base_color,
            normal,
            view_direction,
            light_direction,
            light.color_intensity.rgb,
            light.color_intensity.w,
            material.specular.rgb,
            material.specular.w,
        );
        if i == 0u {
            result += direct_light * shadow_visibility(input.shadow_position);
        } else {
            result += direct_light;
        }
    }

    for (var i = 0u; i < lighting.counts.y; i = i + 1u) {
        let light = lighting.point_lights[i];
        let to_light = light.position_range.xyz - input.world_position;
        let distance_squared = max(dot(to_light, to_light), 0.0001);
        let distance = sqrt(distance_squared);
        let light_direction = safe_normalize(to_light, normal);
        let range_factor = max(1.0 - distance / light.position_range.w, 0.0);
        let attenuation = range_factor * range_factor / max(distance_squared, 0.01);
        result += blinn_phong(
            base_color,
            normal,
            view_direction,
            light_direction,
            light.color_intensity.rgb,
            light.color_intensity.w * attenuation,
            material.specular.rgb,
            material.specular.w,
        );
    }

    for (var i = 0u; i < lighting.counts.z; i = i + 1u) {
        let light = lighting.spot_lights[i];
        let from_light = input.world_position - light.position_range.xyz;
        let distance_squared = max(dot(from_light, from_light), 0.0001);
        let distance = sqrt(distance_squared);
        let from_light_direction = safe_normalize(from_light, -light.direction_inner_cos.xyz);
        let cone_cosine = dot(
            safe_normalize(light.direction_inner_cos.xyz, vec3<f32>(0.0, 0.0, -1.0)),
            from_light_direction,
        );
        var cone_factor = 0.0;
        let inner_cosine = light.direction_inner_cos.w;
        let outer_cosine = light.outer_cos_padding.x;
        if inner_cosine - outer_cosine <= 1.0e-6 {
            if cone_cosine >= inner_cosine {
                cone_factor = 1.0;
            }
        } else {
            cone_factor = smoothstep(outer_cosine, inner_cosine, cone_cosine);
        }
        let range_factor = max(1.0 - distance / light.position_range.w, 0.0);
        let attenuation = range_factor * range_factor / max(distance_squared, 0.01);
        result += blinn_phong(
            base_color,
            normal,
            view_direction,
            -from_light_direction,
            light.color_intensity.rgb,
            light.color_intensity.w * attenuation * cone_factor,
            material.specular.rgb,
            material.specular.w,
        );
    }

    return vec4<f32>(result, sampled_color.a);
}
