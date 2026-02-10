// Grass billboard shader with wind animation and unlit rendering.
// Avoids PBR normal-based lighting which causes black faces on vertical billboards.

#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_world}
#import bevy_pbr::view_transformations::position_world_to_clip
#import bevy_pbr::mesh_view_bindings::globals

// Custom vertex input matching our grass mesh attributes (position, normal, uv).
struct GrassVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct GrassVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Material bindings (set by AsBindGroup derive)
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var grass_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var grass_sampler: sampler;

struct GrassParams {
    alpha_cutoff: f32,
    wind_strength: f32,
    wind_speed: f32,
    _padding: f32,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> params: GrassParams;

@vertex
fn vertex(vertex: GrassVertex) -> GrassVertexOutput {
    var out: GrassVertexOutput;

    var pos = vertex.position;

    // Wind displacement: top vertices (uv.y == 0.0) sway, bottom (uv.y == 1.0) stay anchored.
    // The sway_factor is 1.0 at the top and 0.0 at the bottom for a natural bending look.
    let sway_factor = 1.0 - vertex.uv.y;
    let t = globals.time * params.wind_speed;

    // Primary wave: large slow sway along X
    let wind_x = sin(t + pos.x * 0.05 + pos.z * 0.07) * params.wind_strength * sway_factor;
    // Secondary wave: smaller offset along Z for organic feel
    let wind_z = sin(t * 0.7 + pos.x * 0.03 + pos.z * 0.09) * params.wind_strength * 0.5 * sway_factor;

    pos.x += wind_x;
    pos.z += wind_z;

    var model = get_world_from_local(vertex.instance_index);
    var world_pos = mesh_position_local_to_world(model, vec4<f32>(pos, 1.0));
    out.position = position_world_to_clip(world_pos.xyz);
    out.uv = vertex.uv;

    return out;
}

@fragment
fn fragment(in: GrassVertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(grass_texture, grass_sampler, in.uv);

    // Alpha masking — discard transparent pixels
    if color.a < params.alpha_cutoff {
        discard;
    }

    // Unlit rendering with soft height gradient:
    // Top of grass (uv.y=0) is full brightness, bottom (uv.y=1) is slightly darker.
    // This gives a natural ambient occlusion feel without PBR lighting artifacts.
    let light = 0.82 + (1.0 - in.uv.y) * 0.18;

    return vec4<f32>(color.rgb * light, 1.0);
}
