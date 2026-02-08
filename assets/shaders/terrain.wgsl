#import bevy_pbr::forward_io::VertexOutput

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
    // Simple green color for testing
    return vec4<f32>(0.2, 0.6, 0.2, 1.0);
}
