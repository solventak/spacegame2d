struct Scene {
    viewport: vec4<f32>,
    ship: vec4<f32>,
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
    let world = rotated + scene.ship.xy;
    output.clip_position = vec4<f32>(world * scene.viewport.xy, 0.0, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
