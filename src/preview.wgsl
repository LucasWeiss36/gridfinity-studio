struct Camera {
    matrix: mat4x4<f32>,
    rotation: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> camera: Camera;

struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) colour: vec3<f32>,
    @location(2) world_normal: vec3<f32>,
};

fn project(position: vec3<f32>, normal: vec3<f32>, colour: vec3<f32>) -> Output {
    var result: Output;
    result.position = camera.matrix * vec4<f32>(position, 1.0);
    result.normal = (camera.rotation * vec4<f32>(normal, 0.0)).xyz;
    result.world_normal = normal;
    result.colour = colour;
    return result;
}

@vertex fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) colour: vec3<f32>,
) -> Output {
    return project(position, normal, colour);
}

@vertex fn vs_edge(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) colour: vec3<f32>,
) -> Output {
    var result = project(position, normal, colour);
    // Resolve coincident surface/line samples while retaining solid depth occlusion.
    result.position.z -= 0.000003 * result.position.w;
    return result;
}

@fragment fn fs_main(input: Output) -> @location(0) vec4<f32> {
    if dot(input.normal, input.normal) < 0.01 {
        return vec4<f32>(input.colour, 1.0);
    }
    let normal = normalize(input.normal);
    let key = max(dot(normal, normalize(vec3<f32>(-0.35, 0.65, 0.68))), 0.0);
    let fill = max(dot(normal, normalize(vec3<f32>(0.65, 0.15, 0.65))), 0.0);
    let light = min(0.78 + 0.19 * key + 0.09 * fill, 1.02);
    let top_highlight = 0.18 * max(normalize(input.world_normal).z, 0.0);
    let colour = mix(input.colour, vec3<f32>(0.98, 0.98, 0.95), top_highlight);
    return vec4<f32>(colour * light, 1.0);
}

@fragment fn fs_edge() -> @location(0) vec4<f32> {
    return vec4<f32>(0.12, 0.25, 0.21, 1.0);
}
