// Legacy additive material shader for MU-style emissive effects.
// It intentionally ignores sampled alpha for color accumulation and
// outputs alpha=0.0 so premultiplied-alpha blending resolves to:
//   dst + src_rgb

#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_world}
#import bevy_pbr::view_transformations::position_world_to_clip

struct LegacyVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct LegacyVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var color_sampler: sampler;

struct LegacyAdditiveParams {
    intensity: f32,
    _padding: vec3<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> params: LegacyAdditiveParams;

@vertex
fn vertex(vertex: LegacyVertex) -> LegacyVertexOutput {
    var out: LegacyVertexOutput;
    let model = get_world_from_local(vertex.instance_index);
    let world_pos = mesh_position_local_to_world(model, vec4<f32>(vertex.position, 1.0));
    out.position = position_world_to_clip(world_pos.xyz);
    out.uv = vertex.uv;
    return out;
}

@fragment
fn fragment(in: LegacyVertexOutput) -> @location(0) vec4<f32> {
    let sampled = textureSample(color_texture, color_sampler, in.uv);
    let rgb = sampled.rgb * params.intensity;
    return vec4<f32>(rgb, 0.0);
}
