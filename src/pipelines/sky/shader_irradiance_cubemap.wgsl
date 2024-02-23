struct VertexOutput {
    @builtin(position) clip_position  : vec4<f32>,
    @location(0) cube_coords : vec3<f32>,
}
@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var vertices = array<vec4<f32>, 6>(
        vec4<f32>(-1.0, 1.0, 0.9, 1.0),
        vec4<f32>(-1.0, -1.0, 0.9, 1.0),
        vec4<f32>(1.0, -1.0, 0.9, 1.0),

        vec4<f32>(1.0, 1.0, 0.9, 1.0),
        vec4<f32>(-1.0, 1.0, 0.9, 1.0),
        vec4<f32>(1.0, -1.0, 0.9, 1.0)
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
var env_map: texture_cube<f32>;
@group(0) @binding(1)
var env_sampler: sampler;


const PI : f32 = 3.14159265359;

@group(0) @binding(2)
var<uniform> rotation_matrix : mat4x4<f32>;


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

    var spherical_coord = normalize(  in.cube_coords);
    //var cube_uv = SampleSphericalMap(spherical_coord);


    var coords = normalize(rotation_matrix * vec4(in.cube_coords, 1.0));

    var normal = coords.xyz;
    var irradiance = vec3(0.0);
    var up = vec3(0.0, 0.0, 1.0);
    var right = normalize(cross(up, normal));
    //up = cross(normal, right);


    var sampleDelta = 0.05;
    var nSamples = 0.0;

    // var uv = sphericalToEquirectangular(normal) * src_dimensions;
    // irradiance += textureLoad(src, vec2<i32>(uv.xy), 0).rgb;

    for(var phi = 0.0; phi < PI * 2.0; phi += sampleDelta){
        for(var theta = 0.0; theta < PI * 0.5; theta += sampleDelta){   
            var tangentSample = vec3<f32>(sin(theta) * cos(phi), sin(theta) * sin(phi), cos(theta));
            var sampleVec = tangentSample.x * right + tangentSample.y + up * tangentSample.z * normal;

            var uv = sampleVec;

            irradiance += textureSample(env_map, env_sampler, uv.xyz).rgb * cos(theta) * sin(theta);
            nSamples += 1.0;
        }
    }
    irradiance =  PI * irradiance * (1.0 / f32(nSamples));


    var texture_color = vec4(irradiance.xyz, 1.0);// textureSample(env_map, env_sampler, coords.xyz);
    return texture_color;
 }