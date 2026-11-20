struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@group(0) @binding(0)
var hdr_scene: texture_2d<f32>;

struct PresentationExposure {
    exposure_ev: f32,
    _padding: vec3<f32>,
};

@group(0) @binding(1)
var<uniform> presentation_exposure: PresentationExposure;

// Narkowicz's fitted ACES approximation. The input and output remain in
// linear-light space; an sRGB surface performs the display transfer.
fn tone_map_aces_fitted(color: vec3<f32>) -> vec3<f32> {
    let x = max(color, vec3<f32>(0.0));
    let numerator = x * (2.51 * x + vec3<f32>(0.03));
    let denominator = x * (2.43 * x + vec3<f32>(0.59)) + vec3<f32>(0.14);
    return clamp(numerator / denominator, vec3<f32>(0.0), vec3<f32>(1.0));
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );

    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(input.position.xy);
    let hdr_color = textureLoad(hdr_scene, pixel, 0);
    let exposure_multiplier = exp2(presentation_exposure.exposure_ev);
    let exposed_color = hdr_color.rgb * exposure_multiplier;
    let display_color = tone_map_aces_fitted(exposed_color);
    return vec4<f32>(display_color, 1.0);
}
