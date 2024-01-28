use std::borrow::Cow;

use crate::factories::texture::TextureBundle;
use crate::helpers::geometry::{self, attribute_names, GeometryData};
use crate::state::State;

use crate::factories::{BindGroupFactory, RenderPipelineFactory};

use wgpu::PrimitiveTopology;

use wgpu::util::DeviceExt;

use super::sky::SkyRenderer;
use super::{create_global_uniform, create_uniform_buffer, ModelUniform, ViewUniform};
const SHADER_SRC: &'static str = " 

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
    var ambient = modelUniform.ambient;

    var albedo =  modelUniform.albedo * c_albedo;
    var roughness = saturate((modelUniform.roughness * c_roughness.r));
    var metallic = modelUniform.metallic * c_metallic;

    var roughness4 : f32 = pow(roughness, 4.0); //roughness * roughness * roughness * roughness;

    // Vectors ---

    var N = normalize(in.normal);
    var L = normalize(light_position - in.world_position);
    var V = normalize( camera.position - in.world_position ) ;
    var H = normalize(V + L);

    //Dot products ----

    var NoH = saturate(dot(N, H));
    var VoH = saturate(dot(V, H));
    var NoV = saturate(dot(N, V));
    var NoL = saturate(dot(N, L));
    
	var diffuseColor		= albedo - albedo * metallic;
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

    //color = pow( color, vec3( 1.0f / 2.2 ) );


    //return  vec4<f32>( vec3(color), 1.0);// textureSample(t_diffuse, s_diffuse, in.uv) * vec4(in.normal, 1.0) * vec4(in.color, 1.0);


    return vec4<f32>(reflection, 1.0);
}
";

#[repr(C, align(256))]
#[derive(Clone, Copy)]

pub struct PbrMaterialModelUniform {
    pub model_matrix: glam::Mat4,
    pub light_position: glam::Vec3,
    pub light_intensity: f32,

    pub ambient: glam::Vec3,
    pub roughness: f32,

    pub albedo: glam::Vec3,
    pub metallic: f32,
}

impl PbrMaterialModelUniform {
    pub fn new(mat: glam::Mat4) -> Self {
        Self {
            model_matrix: mat,
            light_position: glam::Vec3::new(5.0, 5.0, 10.0),
            light_intensity: 1.0,
            ambient: glam::Vec3::ONE * 0.005,
            albedo: glam::Vec3::ONE,
            metallic: 1.0,
            roughness: 1.0,
        }
    }
}

#[repr(C, align(16))]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub normal: [f32; 3],
}

pub struct PbrPipeline {
    pub shader_module: wgpu::ShaderModule,
    pub pipeline: wgpu::RenderPipeline,
    // pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,

    // pub texture_bind_group_layout: Option<wgpu::BindGroupLayout>,
    pub texture_bind_group: Option<wgpu::BindGroup>,

    pub global_uniform_buffer: Option<wgpu::Buffer>,
    pub model_uniform_buffer: Option<wgpu::Buffer>,
}

impl PbrPipeline {
    pub fn new_with_texture(
        ctx: &State,
        // global_uniform_buffer: &wgpu::Buffer,
        // model_uniform_buffer: &wgpu::Buffer,
        // texture: (wgpu::ShaderStages, &wgpu::Sampler, &wgpu::TextureView),
        texture: &TextureBundle,
        albedo: &TextureBundle,
        metallic: &TextureBundle,

        sky: &SkyRenderer,

        topology: PrimitiveTopology,
        enable_depth: bool,
    ) -> Self {
        let shader_module = ctx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER_SRC)),
            });

        let attribs = wgpu::vertex_attr_array![ 0 => Float32x3, 1 => Float32x2, 2 => Float32x4 ,3 => Float32x3];
        let stride = std::mem::size_of::<Vertex>() as u64;

        let global_uniform_buffer = create_global_uniform(&ctx.device);
        let model_uniform_buffer = create_uniform_buffer::<ModelUniform>(1, &ctx.device);

        let mut bind_factory = BindGroupFactory::new();
        bind_factory.add_uniform(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            &global_uniform_buffer,
            wgpu::BufferSize::new(std::mem::size_of::<ViewUniform>() as _),
        );
        bind_factory.add_uniform(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            &model_uniform_buffer,
            wgpu::BufferSize::new(std::mem::size_of::<PbrMaterialModelUniform>() as _),
        );
        let (bind_group_layout, bind_group) = bind_factory.build(&ctx.device);

        let mut texture_bind_group_factory = BindGroupFactory::new();
        texture_bind_group_factory.add_texture_and_sampler(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            &texture.view,
            &texture.sampler,
        );

        texture_bind_group_factory.add_texture_and_sampler(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            &albedo.view,
            &albedo.sampler,
        );

        texture_bind_group_factory.add_texture_and_sampler(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            &metallic.view,
            &metallic.sampler,
        );

        texture_bind_group_factory.add_texture_sky_sampler(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            &sky.iradiance_texture.view,
            &sky.iradiance_texture.sampler,
        );

        let (texture_bind_group_layout, texture_bind_group) =
            texture_bind_group_factory.build(&ctx.device);

        let mut pipeline_factory = RenderPipelineFactory::new();
        pipeline_factory.set_label("PBR pipeline");
        pipeline_factory.add_vertex_attributes(&attribs, stride);
        // .add_instance_attributes(&instance_attribs, std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress)
        if enable_depth {
            pipeline_factory.add_depth_stencil();
        }
        pipeline_factory.set_topology(topology);

        let pipeline = pipeline_factory.create_render_pipeline(
            &ctx,
            &shader_module,
            &[&bind_group_layout, &texture_bind_group_layout],
        );

        Self {
            pipeline,
            shader_module,
            // bind_group_layout,
            bind_group,

            // texture_bind_group_layout: Some(texture_bind_group_layout),
            texture_bind_group: Some(texture_bind_group),

            global_uniform_buffer: Some(global_uniform_buffer),
            model_uniform_buffer: Some(model_uniform_buffer),
        }
    }

    pub fn get_buffers_from_geometry(ctx: &State, geo_data: &GeometryData) -> GpuMesh {
        let mut vertices = Vec::new();
        let position_attrib = geo_data
            .attributes
            .get(&geometry::attribute_names::POSITION)
            .unwrap();

        for i in (0..position_attrib.len()).step_by(3) {
            let position = [
                position_attrib[i],
                position_attrib[i + 1],
                position_attrib[i + 2],
            ];

            vertices.push(Vertex {
                position,
                uv: [0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
                normal: [0.0, 0.0, 0.0],
            });
        }

        // UVS -------
        let uvs_option = geo_data.attributes.get(&attribute_names::UV);
        if let Some(uv) = uvs_option {
            let mut uv_index = 0;
            for i in 0..vertices.len() {
                vertices[i].uv = [uv[uv_index], uv[uv_index + 1]];
                uv_index += 2;
            }
        }

        //Vertex Colors ---
        let vcolors_option: Option<&Vec<f32>> = geo_data.attributes.get(&attribute_names::COLOR);
        println!("{:?}", vcolors_option);
        if let Some(vcolor) = vcolors_option {
            let mut index = 0;
            for i in 0..vertices.len() {
                vertices[i].color = [
                    vcolor[index],
                    vcolor[index + 1],
                    vcolor[index + 2],
                    vcolor[index + 3],
                ];

                println!("Color: {:?}", vertices[i]);
                index += 4;
            }
        }

        // Normals -----

        let normals_option = geo_data.attributes.get(&attribute_names::NORMALS);
        if let Some(normals) = normals_option {
            println!("NORMALS!");
            let mut normal_index = 0;
            for i in 0..vertices.len() {
                vertices[i].normal = [
                    normals[normal_index],
                    normals[normal_index + 1],
                    normals[normal_index + 2],
                ];
                normal_index += 3;
            }
        }

        let vertex_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

        let index_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&geo_data.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        GpuMesh {
            vertex_buffer,
            index_buffer,
            vertex_count: geo_data.indices.len() as u32,
        }
    }
}

pub struct GpuMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub vertex_count: u32,
}
