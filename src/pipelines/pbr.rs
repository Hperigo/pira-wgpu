use std::borrow::Cow;

use crate::helpers::geometry::{self, attribute_names, GeometryData};
use crate::state::State;

use crate::factories::{BindGroupFactory, RenderPipelineFactory};

use wgpu::PrimitiveTopology;

use wgpu::util::DeviceExt;

use super::{ModelUniform, ViewUniform};
const SHADER_SRC: &'static str = " 

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec3<f32>,
    @location(3) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec3<f32>,
    @location(3) normal: vec3<f32>,
};

struct CameraUniform {
    view_matrix : mat4x4<f32>,
    perspective_matrix : mat4x4<f32>,
    view_proj_matrix : mat4x4<f32>,
    position : vec3<f32>,
}

struct ModelUniform {
    model_matrix : mat4x4<f32>,

    albedo : vec3<f32>,
    metallic : f32,
    roughness : f32,
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


fn fresnelSchlick(cosTheta : f32, F0 : vec3<f32>) -> vec3<f32>
{
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}  

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


@fragment
fn fs_main(in : VertexOutput) -> @location(0) vec4<f32> {

    //UNIFORMS 
    var light_position = vec3(5.0, 5.0, 10.0);

    var N = normalize(in.normal);
    var V = normalize( camera.position - in.world_position );

    // Light contribution
    var L = normalize(light_position - in.world_position);
    var H = normalize(V + L);

    var distance = length(light_position - in.world_position);
    var attenuation = 1.0 / (distance * distance);
    var radiance = vec3(1.0, 1.0, 1.0);

    var F0 = vec3(0.04); 
    F0     = mix(F0, modelUniform.albedo, modelUniform.metallic);

    var NDF = DistributionGGX(N, H, modelUniform.roughness);   
    var G   = GeometrySmith(N, V, L, modelUniform.roughness);      
    var F  = fresnelSchlick(max(dot(H, V), 0.0), F0);


    var numerator    = NDF * G * F; 
    var denominator = 4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0) + 0.0001; // + 0.0001 to prevent divide by zero
    var specular = numerator / denominator;
    
    // kS is equal to Fresnel
    var kS = F;
    // for energy conservation, the diffuse and specular light can't
    // be above 1.0 (unless the surface emits light); to preserve this
    // relationship the diffuse component (kD) should equal 1.0 - kS.
    var kD = vec3(1.0) - kS;
    // multiply kD by the inverse metalness such that only non-metals 
    // have diffuse lighting, or a linear blend if partly metal (pure metals
    // have no diffuse light).
    kD *= 1.0 - modelUniform.metallic;	  

    // scale light by NdotL
    var NdotL = max(dot(N, L), 0.0);        

    // add to outgoing radiance Lo
    var Lo = (kD * modelUniform.albedo / PI + specular) * radiance * NdotL;


    // HDR tonemapping
    var color = Lo / (Lo + vec3(1.0));
    // gamma correct
    color = pow(color, vec3(1.0/2.2)); 

    return  vec4<f32>(Lo, 1.0);// textureSample(t_diffuse, s_diffuse, in.uv) * vec4(in.normal, 1.0) * vec4(in.color, 1.0);
}
";

#[repr(C, align(256))]
#[derive(Clone, Copy)]
pub struct PbrMaterialModelUniform {
    pub model_matrix: glam::Mat4,
    pub albedo: glam::Vec3,
    pub metallic: f32,
    pub roughness: f32,
}

impl PbrMaterialModelUniform {
    pub fn new(mat: glam::Mat4) -> Self {
        Self {
            model_matrix: mat,
            albedo: glam::Vec3::ONE,
            metallic: 0.0,
            roughness: 1.0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 3],
    pub normal: [f32; 3],
}

pub struct PbrPipeline {
    pub shader_module: wgpu::ShaderModule,
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,

    pub texture_bind_group_layout: Option<wgpu::BindGroupLayout>,
    pub texture_bind_group: Option<wgpu::BindGroup>,
}

impl PbrPipeline {
    pub fn new_with_texture(
        ctx: &State,
        global_uniform_buffer: &wgpu::Buffer,
        model_uniform_buffer: &wgpu::Buffer,
        texture: (wgpu::ShaderStages, &wgpu::Sampler, &wgpu::TextureView),
        topology: PrimitiveTopology,
    ) -> Self {
        let shader_module = ctx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER_SRC)),
            });

        let attribs = wgpu::vertex_attr_array![ 0 => Float32x3, 1 => Float32x2, 2 => Float32x3 ,3 => Float32x3 ];
        let stride = std::mem::size_of::<Vertex>() as u64;

        let mut bind_factory = BindGroupFactory::new();
        bind_factory.add_uniform(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            &global_uniform_buffer,
            wgpu::BufferSize::new(std::mem::size_of::<ViewUniform>() as _),
        );
        bind_factory.add_uniform(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            &model_uniform_buffer,
            wgpu::BufferSize::new(std::mem::size_of::<ModelUniform>() as _),
        );
        let (bind_group_layout, bind_group) = bind_factory.build(&ctx.device);

        let mut texture_bind_group_factory = BindGroupFactory::new();
        texture_bind_group_factory.add_texture_and_sampler(texture.0, texture.2, texture.1);
        let (texture_bind_group_layout, texture_bind_group) =
            texture_bind_group_factory.build(&ctx.device);

        let mut pipeline_factory = RenderPipelineFactory::new();
        pipeline_factory.set_label("PBR pipeline");
        pipeline_factory.add_vertex_attributes(&attribs, stride);
        // .add_instance_attributes(&instance_attribs, std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress)
        pipeline_factory.add_depth_stencil();
        pipeline_factory.set_topology(topology);

        let pipeline = pipeline_factory.create_render_pipeline(
            &ctx,
            &shader_module,
            &[&bind_group_layout, &texture_bind_group_layout],
        );

        Self {
            pipeline,
            shader_module,
            bind_group_layout,
            bind_group,

            texture_bind_group_layout: Some(texture_bind_group_layout),
            texture_bind_group: Some(texture_bind_group),
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
                color: [1.0, 1.0, 1.0],
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
                vertices[i].color = [vcolor[index], vcolor[index + 1], vcolor[index + 2]];

                println!("Color: {:?}", vertices[i]);
                index += 3;
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
