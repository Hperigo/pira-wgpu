use std::borrow::Cow;

use crate::factories::texture::TextureBundle;
use crate::helpers::geometry::{self, attribute_names, GeometryData};
use crate::state::State;

use crate::factories::{BindGroupFactory, RenderPipelineFactory};

use wgpu::PrimitiveTopology;

use wgpu::util::DeviceExt;

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

    _pad1 : f32, 

    albedo : vec3<f32>,
    roughness : f32,
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


fn DistributionGGX(N : vec3<f32>, H : vec3<f32>, roughness : f32) -> f32
{
    var a      = roughness*roughness;
    var a2     = a*a;
    var NdotH  = max(dot(N, H), 0.0);
    var NdotH2 = NdotH*NdotH;
	
    var num   = a2;
    var denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;
	
    return num / denom;
}
fn GeometrySchlickGGX(NdotV : f32, roughness : f32) -> f32
{
    var r = (roughness + 1.0);
    var k = (r*r) / 8.0;
     
    var nom   = NdotV;
    var denom = NdotV * (1.0 - k) + k;
    
    return nom / denom;
}

fn GeometrySmith(N : vec3<f32>, V : vec3<f32>, L : vec3<f32>, roughness : f32) -> f32
{
    var NdotV = max(dot(N, V), 0.0);
    var NdotL = max(dot(N, L), 0.0);
    var ggx2 = GeometrySchlickGGX(NdotV, roughness);
    var ggx1 = GeometrySchlickGGX(NdotL, roughness);

    return ggx1 * ggx2;
}

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
    if(cosri >= 0.0) {
        f = min(1.0, NoL / NoV);
    }

    var c2 = 0.45 * roughness4 / (roughness4 + 0.09) * cosri * f ;
    return albedo / PI * ( NoL * c1 + c2 );
}


@fragment
fn fs_main(in : VertexOutput) -> @location(0) vec4<f32> {

    // //UNIFORMS 
    var light_position = modelUniform.light_position; // vec3(5.0, 5.0, 10.0);
    var albedo =  modelUniform.albedo;
    var roughness = modelUniform.roughness;
    var metallic = 0.0;
    //var specular = 0.04;

    var roughness4 : f32 = roughness * roughness * roughness * roughness;


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
    var F0 = vec3f(0.04);
    F0 = mix(F0, albedo, vec3f(metallic));


    var normal_distrib = getNormalDistribution(roughness4, NoH);
	var fresnel = fresnelSchlick(max(dot(H, V), 0.0), F0) ;
    var geom = getGeometricShadowing( roughness4, NoV, NoL, VoH, L, V );

    var diffuse = getDiffuse(albedo, roughness, NoV, NoL, VoH);
    var specular = NoL * normal_distrib * fresnel * geom;

    var color = diffuse + specular; // TODO: add light color

    // color = color / (color + vec3(1.0));
    // color = pow(color, vec3(1.0/2.2));  

    return  vec4<f32>( vec3(color), 1.0);// textureSample(t_diffuse, s_diffuse, in.uv) * vec4(in.normal, 1.0) * vec4(in.color, 1.0);
}
";

#[repr(C, align(256))]
#[derive(Clone, Copy)]

pub struct PbrMaterialModelUniform {
    pub model_matrix: glam::Mat4,
    pub light_position: glam::Vec3,

    _pad1: f32,

    pub albedo: glam::Vec3,

    pub roughness: f32,
    pub metallic: f32,
}

impl PbrMaterialModelUniform {
    pub fn new(mat: glam::Mat4) -> Self {
        Self {
            model_matrix: mat,
            light_position: glam::Vec3::new(5.0, 5.0, 10.0),
            albedo: glam::Vec3::ONE,
            metallic: 0.0,
            roughness: 1.0,

            _pad1: 0.0,
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
            wgpu::ShaderStages::VERTEX,
            &texture.view,
            &texture.sampler,
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
