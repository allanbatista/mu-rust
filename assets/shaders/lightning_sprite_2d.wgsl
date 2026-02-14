#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var color_texture: texture_2d<f32>;
@group(2) @binding(1) var color_sampler: sampler;

struct LightningSprite2dParams {
    intensity: f32,
    _padding: vec3<f32>,
};

@group(2) @binding(2) var<uniform> params: LightningSprite2dParams;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let sampled = textureSample(color_texture, color_sampler, in.uv);
    let rgb = sampled.rgb * params.intensity;
    return vec4<f32>(rgb, 0.0);
}
