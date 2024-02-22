struct VertexOutput {
    @builtin(position) clip_position  : vec4<f32>,
    @location(0) cube_coords : vec3<f32>,
}
@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var vertices = array<vec4<f32>, 6>(
        vec4<f32>(-1.0, 1.0, 0.0, 1.0),
        vec4<f32>(-1.0, -1.0, 0.0, 1.0),
        vec4<f32>(1.0, -1.0, 0.0, 1.0),

        vec4<f32>(1.0, 1.0, 0.0, 1.0),
        vec4<f32>(-1.0, 1.0, 0.0, 1.0),
        vec4<f32>(1.0, -1.0, 0.0, 1.0)
    );

    var cube_coords = array<vec3<f32>, 6>(
        vec3<f32>(-1.0, 1.0, -1.0 ),
        vec3<f32>(-1.0, -1.0, -1.0 ),
        vec3<f32>(1.0, -1.0, -1.0 ),

        vec3<f32>(1.0, 1.0, -1.0 ),
        vec3<f32>(-1.0, 1.0, -1.0),
        vec3<f32>(1.0, -1.0, -1.0 )
    );

    var out : VertexOutput;
    out.clip_position = vertices[in_vertex_index];
    out.cube_coords = cube_coords[in_vertex_index];

    return out;
}

@group(0) @binding(0)
var hdr_texture: texture_2d<f32>;
@group(0) @binding(1)
var hdr_sampler: sampler;


// Rotation matrix around the X axis.
fn rotateX(theta : f32) -> mat3x3<f32> {
    var c = cos(theta);
    var s = sin(theta);
    return mat3x3<f32>(
        vec3(1.0, 0.0, 0.0),
        vec3(0.0, c, -s),
        vec3(0.0, s, c)
    );
}

const invAtan : vec2<f32> = vec2<f32>(0.1591, 0.3183);
fn SampleSphericalMap(v : vec3<f32>) -> vec2<f32>
{
    var uv = vec2<f32>(atan2(v.z, v.x), asin(v.y));
    uv *= invAtan;
    uv += 0.5;
    return uv;
}

@fragment
fn fs_main(in : VertexOutput) -> @location(0) vec4<f32> {

    var spherical_coord = normalize( rotateX(3.14) * in.cube_coords);
    var cube_uv = SampleSphericalMap(spherical_coord);

    var texture_color = textureLoad(hdr_texture, vec2<i32>(cube_uv * vec2<f32>(1024.0, 512.0)), 0);
    return texture_color;
 }