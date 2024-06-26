struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) pad : vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec3<f32>,
};

struct CameraUniform {
    view_proj : mat4x4<f32>,
}

struct ModelUniform {
    model_matrix : mat4x4<f32>,
    //TODO: add tint color
}

@group(0) @binding(0) // 1.
var<uniform> camera: CameraUniform;

@group(0) @binding(1) // 1.
var<uniform> modelUniform: ModelUniform;

@group(1) @binding(0)
var env_map: texture_cube<f32>;
@group(1) @binding(1)
var env_sampler: sampler;


// @group(0) @binding(1) // 1.
// var<uniform> modelUniform: ModelUniform;



@vertex
fn vs_main( model : VertexInput ) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position =  camera.view_proj * modelUniform.model_matrix * vec4(model.position, 1.0);
    out.uv = model.uv;
    out.color = model.position;
    return out;
}


const PI : f32 =3.14159265359;

// @group(0) @binding(2)
// var<uniform> rotation_matrix : mat4x4<f32>;


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

   // var spherical_coord = normalize(  in.cube_coords);
    //var cube_uv = SampleSphericalMap(spherical_coord);


    var coords = normalize(in.color);
    coords.x = -coords.x;

    var normal = coords.xyz;
    var irradiance = vec3(0.0);
    
    var up = vec3(0.0, 1.0, 0.0);
    var right = normalize(cross(up, normal));
    up = normalize(cross(normal, right));


    var sampleDelta = 0.05;
    var nSamples = 0.0;

    for(var phi = 0.0; phi < PI * 2.0; phi += sampleDelta){
        for(var theta = 0.0; theta < PI * 0.5; theta += sampleDelta){   
            var tangent_sample = vec3<f32>(sin(theta) * cos(phi), sin(theta) * sin(phi), cos(theta));
            var sample_vec = tangent_sample.x * right + tangent_sample.y * up + tangent_sample.z * normal;

            irradiance += textureSample(env_map, env_sampler, sample_vec.xyz).rgb * cos(theta) * sin(theta);
            nSamples += 1.0;
        }
    }
    irradiance =  PI * irradiance * (1.0 / f32(nSamples));

    var texture_color = vec4(irradiance, 1.0);
    return texture_color;
 }
