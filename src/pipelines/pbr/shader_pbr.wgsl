struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) normal: vec3<f32>,
};

struct CameraUniform {
    view_proj_matrix : mat4x4<f32>,
    view_matrix : mat4x4<f32>,
    perspective_matrix : mat4x4<f32>,
    position : vec3<f32>,
}

struct ModelUniform {
    model_matrix : mat4x4<f32>,
    
    light_position : vec3<f32>,
    light_intensity : f32,

    ambient : vec3<f32>,
    roughness : f32,

    albedo : vec3<f32>,
    metallic : f32,
}


@group(0) @binding(0) // 1.
var<uniform> camera: CameraUniform;

@group(0) @binding(1) // 1.
var<uniform> modelUniform: ModelUniform;

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

@group(1) @binding(2)
var t_albedo: texture_2d<f32>;
@group(1) @binding(3)
var s_albedo: sampler;

@group(1) @binding(4)
var t_metallic: texture_2d<f32>;
@group(1) @binding(5)
var s_metallic: sampler;

@group(1) @binding(6)
var env_map: texture_cube<f32>;
@group(1) @binding(7)
var env_sampler: sampler;

@vertex
fn vs_main( model : VertexInput ) -> VertexOutput {
    var out: VertexOutput;

    var world_position = modelUniform.model_matrix * vec4(model.position, 1.0);

    out.clip_position =  camera.view_proj_matrix * world_position;
    out.world_position = world_position.xyz;
    out.uv = model.uv;
    out.color = model.color;
    out.normal = model.normal;
    return out;
}

const PI : f32 = 3.14159265359;


// GGX Normal distribution
fn getNormalDistribution(roughness : f32, NoH : f32 ) -> f32{
    var d = ( NoH * roughness - NoH ) * NoH + 1.0;
	return roughness / ( d*d );
}

fn getFresnel( specular_color : vec3f, VoH : f32 ) -> vec3f {
    var specular_color_sqrt = sqrt( clamp( vec3f(), vec3f(0.99, 0.99, 0.99), specular_color ) );
    var n = ( 1.0 + specular_color_sqrt ) / ( 1.0 - specular_color_sqrt );
    var g = sqrt( n * n + VoH * VoH - 1.0 );
	return 0.5 * pow( (g - VoH) / (g + VoH), vec3f(2.0) ) * ( 1.0 + pow( ((g+VoH)*VoH - 1.0) / ((g-VoH)*VoH + 1.0), vec3f(2.0) ) );
}

fn fresnelSchlick(cosTheta : f32, F0 : vec3f) -> vec3f
{
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}  

fn getGeometricShadowing( roughness4 : f32,  NoV : f32, NoL : f32, VoH : f32, L : vec3f, V : vec3f )  -> f32 {
    var gSmithV = NoV + sqrt( NoV * (NoV - NoV * roughness4) + roughness4 );
	var gSmithL = NoL + sqrt( NoL * (NoL - NoL * roughness4) + roughness4 );

    return 1.0 / (gSmithV * gSmithL);
}

fn getDiffuse( albedo : vec3f, roughness4 : f32, NoV : f32, NoL : f32, VoH : f32 ) -> vec3f {
    var VoL = 2.0 * VoH - 1.0;
    var c1 = 1.0 - 0.5 * roughness4 / (roughness4 + 0.33);
    var cosri = VoL - NoV * NoL;

    var f = NoL; 
    // if(cosri >= 0.0) {
    //     f = min(1.0, NoL / NoV);
    // }

    var c2 = 0.45 * roughness4 / (roughness4 + 0.09) * cosri * f ;
    return albedo / PI * ( NoL * c1 + c2 );
}

// http://imdoingitwrong.wordpress.com/2011/01/31/light-attenuation/
fn getAttenuation( lightPosition :  vec3f, vertexPosition : vec3f, lightRadius : f32 ) -> f32
{
	var r				= lightRadius;
	var L				= lightPosition - vertexPosition;
	var dist			= length(L);
	var d				= max( dist - r, 0.0 );
	L					/= dist;
	var denom			= d / r + 1.0f;
	var attenuation	    = 1.0f / (denom*denom);
	var cutoff  		= 0.0052f;
	attenuation			= (attenuation - cutoff) / (1.0 - cutoff);
	attenuation			= max(attenuation, 0.0);
	
	return attenuation;
}

const A = 0.15;
const B = 0.50;
const C = 0.10;
const D = 0.20;
const E = 0.02;
const F = 0.30;

fn Uncharted2Tonemap( x : vec3f ) -> vec3f
{
	return ((x*(A*x+C*B)+D*E)/(x*(A*x+B)+D*F))-E/F;
}


@fragment
fn fs_main(in : VertexOutput) -> @location(0) vec4<f32> {

    // Textures --- 

    var c_roughness = textureSample(t_diffuse, s_diffuse, in.uv * vec2(1.0)).rgb;
    var c_albedo = textureSample(t_albedo, s_albedo, in.uv * vec2(1.0)).rgb;
    var c_metallic = textureSample(t_metallic, s_metallic, in.uv * vec2(1.0)).rgb;


    // UNIFORMS 
    var light_position = modelUniform.light_position; // vec3(5.0, 5.0, 10.0);
    var light_intensity = modelUniform.light_intensity;
    

    // Vectors ---
    var N = normalize(in.normal);
    var L = normalize(light_position - in.world_position);
    var V = normalize( camera.position - in.world_position ) ;
    var H = normalize(V + L);


    // Textures -----
    var irradiance = textureSample(env_map, env_sampler, N ).rgb; // modelUniform.ambient;
    var albedo =  irradiance * modelUniform.albedo * c_albedo;
    var roughness = saturate((modelUniform.roughness * c_roughness.r));
    var metallic = modelUniform.metallic * c_metallic;
    var ambient = modelUniform.ambient;
    var roughness4 : f32 = pow(roughness, 4.0); //roughness * roughness * roughness * roughness;



    //Dot products ----

    var NoH = saturate(dot(N, H));
    var VoH = saturate(dot(V, H));
    var NoV = saturate(dot(N, V));
    var NoL = saturate(dot(N, L));
    

	//var diffuseColo	= albedo - albedo * metallic;
    // var F0 = vec3f(0.0001);
    // F0 = mix(F0, albedo, vec3f(metallic));
    
    

    let world_reflect = reflect(-V, N);
    let reflection = textureSample(env_map, env_sampler, world_reflect).rgb;

    var specularColor = mix( vec3( 0.04 ), albedo, metallic );


    var normal_distrib = getNormalDistribution(roughness4, NoH);
	var fresnel = fresnelSchlick(max(dot(H, V), 0.0), specularColor) ;
    var geom = getGeometricShadowing( roughness4, NoV, NoL, VoH, L, V );

    var diffuse = getDiffuse(albedo, roughness4, NoV, NoL, VoH) * (1.0 - metallic);
    var specular = NoL * normal_distrib * fresnel * geom;

    var attenuation = getAttenuation(light_position, in.world_position, light_intensity);

    var color = (diffuse + specular)* attenuation; // TODO: add light color

    color += ambient * albedo;

    color = Uncharted2Tonemap(color * 10.0);

    // white balance
	var whiteInputLevel = 10.0f;
	var whiteScale			= 1.0f / Uncharted2Tonemap( vec3( whiteInputLevel ) );
	color					= color * whiteScale;
    return vec4<f32>(color, 1.0);
}