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

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var checker_texture: texture_2d<f32>;

@group(1) @binding(1)
var checker_sampler: sampler;

@group(2) @binding(0)
var<uniform> lighting: Lighting;

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
    return output;
}

fn blinn_phong(
    base_color: vec3<f32>,
    normal: vec3<f32>,
    view_direction: vec3<f32>,
    light_direction: vec3<f32>,
    light_color: vec3<f32>,
    intensity: f32,
) -> vec3<f32> {
    let diffuse_factor = max(dot(normal, light_direction), 0.0);
    let halfway_direction = normalize(light_direction + view_direction);
    let specular_factor = pow(max(dot(normal, halfway_direction), 0.0), 32.0);
    let diffuse = base_color * diffuse_factor;
    let specular = vec3<f32>(0.22) * specular_factor;
    return (diffuse + specular) * light_color * intensity;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let sampled_color = textureSample(checker_texture, checker_sampler, input.uv);
    let base_color = sampled_color.rgb * input.color;
    let normal = normalize(input.world_normal);
    let view_direction = normalize(camera.camera_position.xyz - input.world_position);
    var result = base_color * lighting.ambient_color.rgb;

    for (var i = 0u; i < lighting.counts.x; i = i + 1u) {
        let light = lighting.directional_lights[i];
        let light_direction = normalize(-light.direction.xyz);
        result += blinn_phong(
            base_color,
            normal,
            view_direction,
            light_direction,
            light.color_intensity.rgb,
            light.color_intensity.w,
        );
    }

    for (var i = 0u; i < lighting.counts.y; i = i + 1u) {
        let light = lighting.point_lights[i];
        let to_light = light.position_range.xyz - input.world_position;
        let distance_squared = max(dot(to_light, to_light), 0.0001);
        let distance = sqrt(distance_squared);
        let light_direction = to_light / distance;
        let range_factor = max(1.0 - distance / light.position_range.w, 0.0);
        let attenuation = range_factor * range_factor / max(distance_squared, 0.01);
        result += blinn_phong(
            base_color,
            normal,
            view_direction,
            light_direction,
            light.color_intensity.rgb,
            light.color_intensity.w * attenuation,
        );
    }

    for (var i = 0u; i < lighting.counts.z; i = i + 1u) {
        let light = lighting.spot_lights[i];
        let from_light = input.world_position - light.position_range.xyz;
        let distance_squared = max(dot(from_light, from_light), 0.0001);
        let distance = sqrt(distance_squared);
        let from_light_direction = from_light / distance;
        let cone_cosine = dot(normalize(light.direction_inner_cos.xyz), from_light_direction);
        let cone_factor = smoothstep(
            light.outer_cos_padding.x,
            light.direction_inner_cos.w,
            cone_cosine,
        );
        let range_factor = max(1.0 - distance / light.position_range.w, 0.0);
        let attenuation = range_factor * range_factor / max(distance_squared, 0.01);
        result += blinn_phong(
            base_color,
            normal,
            view_direction,
            -from_light_direction,
            light.color_intensity.rgb,
            light.color_intensity.w * attenuation * cone_factor,
        );
    }

    return vec4<f32>(result, sampled_color.a);
}
