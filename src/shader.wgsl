struct Scene {
    viewport: vec4<f32>,
    ship: vec4<f32>,
    marker: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> scene: Scene;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let sine = scene.ship.z;
    let cosine = scene.ship.w;
    let local = input.position;
    let rotated = vec2<f32>(
        local.x * cosine - local.y * sine,
        local.x * sine + local.y * cosine,
    );
    let is_marker = input.color.r > 0.9 && input.color.g < 0.1;
    let world = select(rotated + scene.ship.xy, input.position + scene.marker.xy, is_marker);
    output.clip_position = vec4<f32>(world * scene.viewport.xy, 0.0, 1.0);
    output.color = vec4<f32>(input.color.rgb, select(1.0, scene.marker.z, is_marker));
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    if input.color.a < 0.5 { discard; }
    return input.color;
}
