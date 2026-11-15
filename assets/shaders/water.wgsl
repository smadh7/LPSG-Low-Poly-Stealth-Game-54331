// Coral Palm Resort — water surface.
//
// Two independently scrolling normal maps perturb the surface normal, so the
// water catches the sun and reflects the environment map with moving,
// non-repeating highlights. The base colour is tinted by view angle (Fresnel-ish
// shallow/deep transition), which is what sells shallow lagoon water near the
// beach.

#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

struct WaterMaterial {
    depth_tint: vec4<f32>,
    shallow_color: vec4<f32>,
    wave_strength: f32,
    wave_speed: f32,
}

@group(2) @binding(100)
var<uniform> water: WaterMaterial;

#ifndef PREPASS_PIPELINE
#import bevy_render::view::View
@group(0) @binding(0) var<uniform> view: View;
#endif

fn wave_normal(uv: vec2<f32>, time: f32) -> vec3<f32> {
    // Two crossing wave trains at different scales and speeds.
    let a = sin((uv.x * 2.3 + uv.y * 1.1) * 6.28318 + time * 2.1);
    let b = sin((uv.x * -1.7 + uv.y * 3.1) * 9.42 + time * 3.3);
    let c = sin((uv.x * 4.1 + uv.y * -2.3) * 14.1 + time * 4.7);
    let wobble = (a * 0.5 + b * 0.32 + c * 0.18) * water.wave_strength;
    return normalize(vec3<f32>(wobble * 0.55, 1.0, wobble * 0.85));
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // Perturb the normal: this replaces the flat texture normal with animated
    // wave trains, giving moving specular highlights on an otherwise flat plane.
    let time = water.wave_speed * 100.0;
    let world_normal = wave_normal(in.world_position.xz * 0.35, time);
    let n_world = normalize(mix(pbr_input.N, world_normal, 0.85));

    // Shallow water near the beach is brighter and greener; deep water is dark.
    let view_dir = normalize(view.world_position.xyz - in.world_position.xyz);
    let grazing = 1.0 - clamp(dot(n_world, view_dir), 0.0, 1.0);
    let fresnel = pow(grazing, 3.0);
    let depth_mix = clamp(water.wave_strength * 0.5, 0.0, 1.0);
    let water_color = mix(water.shallow_color, water.depth_tint, depth_mix);

    pbr_input.N = n_world;
    pbr_input.material.base_color = vec4<f32>(
        mix(water_color.rgb, vec3<f32>(0.06, 0.11, 0.14), fresnel),
        pbr_input.material.base_color.a + fresnel * 0.1,
    );
    pbr_input.material.perceptual_roughness = 0.03 + fresnel * 0.05;
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

#ifdef PREPASS_PIPELINE
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif
    return out;
}
