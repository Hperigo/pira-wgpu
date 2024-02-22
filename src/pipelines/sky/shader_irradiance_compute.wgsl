const PI: f32 = 3.1415926535897932384626433832795;

struct Face {
    forward: vec3<f32>,
    up: vec3<f32>,
    right: vec3<f32>,
}

@group(0)
@binding(0)
var src: texture_2d<f32>;

@group(0)
@binding(1)
var dst: texture_storage_2d_array<rgba32float, write>;


fn sphericalToEquirectangular(spherical : vec3f) -> vec2f
{
    let inv_atan = vec2(0.1591, 0.3183);
    let eq_uv = vec2(atan2(spherical.z, spherical.x), asin(spherical.y)) * inv_atan + 0.5;
    return eq_uv;
}  

@compute
@workgroup_size(16, 16, 1)
fn calc_iradiance(
    @builtin(global_invocation_id)
    gid: vec3<u32>,
) {

  var FACES: array<Face, 6> = array(
        // FACES +X
        Face(
            vec3(1.0, 0.0, 0.0),  // forward
            vec3(0.0, 1.0, 0.0),  // up
            vec3(0.0, 0.0, -1.0), // right
        ),
        // FACES -X
        Face (
            vec3(-1.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
            vec3(0.0, 0.0, 1.0),
        ),
        // FACES +Y
        Face (
            vec3(0.0, -1.0, 0.0),
            vec3(0.0, 0.0, 1.0),
            vec3(1.0, 0.0, 0.0),
        ),
        // FACES -Y
        Face (
            vec3(0.0, 1.0, 0.0),
            vec3(0.0, 0.0, -1.0),
            vec3(1.0, 0.0, 0.0),
        ),
        // FACES +Z
        Face (
            vec3(0.0, 0.0, 1.0),
            vec3(0.0, 1.0, 0.0),
            vec3(1.0, 0.0, 0.0),
        ),
        // FACES -Z
        Face (
            vec3(0.0, 0.0, -1.0),
            vec3(0.0, 1.0, 0.0),
            vec3(-1.0, 0.0, 0.0),
        ),
    );


    // Get texture coords relative to cubemap face
    let src_dimensions = vec2<f32>(textureDimensions(src));
    let dst_dimensions = vec2<f32>(textureDimensions(dst));
    let cube_uv = (vec2<f32>(gid.xy) / dst_dimensions) * 2.0 - 1.0;

    // Get spherical coordinate from cube_uv
    let face = FACES[gid.z];
    let normal = normalize(face.forward + face.right * cube_uv.x + face.up * cube_uv.y) * -1.0;


    // Compute iradiance
    var irradiance = vec3(0.0);
    var up = vec3(0.0, 1.0, 0.0);
    var right = normalize(cross(up, normal));

    var sampleDelta = 0.025;
    var nSamples = 0.0;

    // var uv = sphericalToEquirectangular(normal) * src_dimensions;
    // irradiance += textureLoad(src, vec2<i32>(uv.xy), 0).rgb;

    for(var phi = 0.0; phi < PI * 2.0; phi += sampleDelta){
        for(var theta = 0.0; theta < PI * 0.5; theta += sampleDelta){   
            var tangentSample = vec3<f32>(sin(theta) * cos(phi), sin(theta) * sin(phi), cos(theta));
            var sampleVec = tangentSample.x * right + tangentSample.y + up * tangentSample.z * normal;

            var uv = sphericalToEquirectangular(normalize(sampleVec)) * src_dimensions;

            irradiance += textureLoad(src, vec2<i32>(uv.xy), 0).rgb * cos(theta) * sin(theta);
            nSamples += 1.0;
        }
    }
    irradiance =  PI * irradiance * (1.0 / f32(nSamples));

    textureStore(dst, gid.xy, gid.z, vec4(irradiance, 1.0));
}