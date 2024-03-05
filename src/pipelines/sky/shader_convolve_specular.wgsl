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

@group(0) @binding(2)
var<uniform> roughnessUniform : f32;

@group(1) @binding(0)
var env_map: texture_cube<f32>;
@group(1) @binding(1)
var env_sampler: sampler;

@vertex
fn vs_main( model : VertexInput ) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position =  modelUniform.model_matrix * vec4(model.position, 1.0);
    out.uv = model.uv;
    out.color = model.position;
    return out;
}


const PI : f32 =3.14159265359;

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

fn RadicalInverse_VdC(in_bits : u32) -> f32
{
    var bits = (in_bits << 16u) | (in_bits >> 16u);
    bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u);
    bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u);
    bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u);
    bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u);
    return f32(bits) * 2.3283064365386963e-10; // / 0x100000000
}
// ----------------------------------------------------------------------------
fn Hammersley(i : u32, N : u32) -> vec2<f32>
{
    return vec2<f32>(f32(i)/f32(N), RadicalInverse_VdC(i));
}  

fn ImportanceSampleGGX( Xi : vec2<f32>, N : vec3<f32>, roughness : f32)-> vec3<f32>{
	let a = roughness * roughness;
	let phi = 2.0 * PI * Xi.x;
	let cosTheta = sqrt((1.0 - Xi.y) / (1.0 + (a*a - 1.0) * Xi.y));
	let sinTheta = sqrt(1.0 - cosTheta*cosTheta);

	var H = vec3(0.0);
	H.x = cos(phi) * sinTheta;
	H.y = sin(phi) * sinTheta;
	H.z = cosTheta;

	var up        = vec3<f32>(1.0, 0.0, 0.0);
	if abs(N.z) < 0.999 {
		up = vec3(0.0, 0.0, 1.0);
	}

	let tangent   = normalize(cross(up, N));
	let bitangent = cross(N, tangent);
	

	let sampleVec = tangent * H.x + bitangent * H.y + N * H.z;
	return normalize(sampleVec);
}

@fragment
fn fs_main(in : VertexOutput) -> @location(0) vec4<f32> {

    var coords = normalize(in.color);

    var normal = coords.xyz;
    let N = normal;
    let V = normal;

	let SAMPLE_COUNT : u32 = 4096u;

	var total_weight = 0.0;
	var f_color = vec3<f32>(0.0);

	for(var i : u32 = 0u; i < SAMPLE_COUNT; i++){
		var x_i = Hammersley(i, SAMPLE_COUNT);
		var H = ImportanceSampleGGX(x_i, normal, camera.view_proj[0][0]);
		var L = normalize(2.0 * dot(V,H) * H - V);

		var n_dot_l = max(dot(N, L), 0.0);

		if(n_dot_l > 0.0){
			f_color += textureSample(env_map, env_sampler, L).rgb * n_dot_l;
			total_weight += n_dot_l;
		}
	}

	f_color = f_color / total_weight;
	
    return vec4<f32>(f_color, 1.0);
 }
