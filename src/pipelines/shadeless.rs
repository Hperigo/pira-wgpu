use std::borrow::Cow;

use crate::helpers::geometry::{self, attribute_names, GeometryData};
use crate::state::State;

use crate::factories::{texture::TextureBundle, BindGroupFactory, RenderPipelineFactory};

use wgpu::PrimitiveTopology;

use wgpu::util::DeviceExt;

use super::{create_global_uniform, create_uniform_buffer, ModelUniform};

const SHADER_SRC: &'static str = " 

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec3<f32>,
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
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

@vertex
fn vs_main( model : VertexInput ) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position =  camera.view_proj * modelUniform.model_matrix * vec4(model.position, 1.0);
    out.uv = model.uv;
    out.color = model.color;
    return out;
}


@fragment
fn fs_main(in : VertexOutput) -> @location(0) vec4<f32> {
    // let flipped_uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
    return  textureSample(t_diffuse, s_diffuse, in.uv) * vec4(in.color, 1.0);
}
";

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 3],
}

pub struct ShadelessPipeline {
    pub shader_module: wgpu::ShaderModule,
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,

    pub texture_bind_group_layout: Option<wgpu::BindGroupLayout>,
    pub texture_bind_group: Option<wgpu::BindGroup>,

    pub global_uniform_buffer: Option<wgpu::Buffer>,
    pub model_uniform_buffer: Option<wgpu::Buffer>,
}

impl ShadelessPipeline {
    pub fn new(ctx: &State, topology: PrimitiveTopology) -> Self {
        let shader_module = ctx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER_SRC)),
            });

        let attribs = wgpu::vertex_attr_array![ 0 => Float32x3, 1 => Float32x2, 2 => Float32x3 ];
        let stride = std::mem::size_of::<Vertex>() as u64;

        let global_uniform_buffer = create_global_uniform(&ctx.device);
        let model_uniform_buffer = create_uniform_buffer::<ModelUniform>(1, &ctx.device);

        let mut bind_factory = BindGroupFactory::new();
        bind_factory.add_uniform(
            wgpu::ShaderStages::VERTEX,
            &global_uniform_buffer,
            wgpu::BufferSize::new(std::mem::size_of::<glam::Mat4>() as _),
        );

        bind_factory.add_uniform(
            wgpu::ShaderStages::VERTEX,
            &model_uniform_buffer,
            wgpu::BufferSize::new(std::mem::size_of::<glam::Mat4>() as _),
        );

        let (bind_group_layout, bind_group) = bind_factory.build(&ctx.device);
        let mut pipeline_factory = RenderPipelineFactory::new();

        pipeline_factory.add_vertex_attributes(&attribs, stride);
        // .add_instance_attributes(&instance_attribs, std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress)
        pipeline_factory.add_depth_stencil();
        pipeline_factory.set_topology(topology);

        let pipeline =
            pipeline_factory.create_render_pipeline(&ctx, &shader_module, &[&bind_group_layout]);

        Self {
            pipeline,
            shader_module,
            bind_group_layout,
            bind_group,

            texture_bind_group_layout: None,
            texture_bind_group: None,

            model_uniform_buffer: None,
            global_uniform_buffer: None,
        }
    }

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

        let attribs = wgpu::vertex_attr_array![ 0 => Float32x3, 1 => Float32x2, 2 => Float32x3 ];
        let stride = std::mem::size_of::<Vertex>() as u64;

        let global_uniform_buffer = create_global_uniform(&ctx.device);
        let model_uniform_buffer = create_uniform_buffer::<ModelUniform>(1, &ctx.device);

        let mut bind_factory = BindGroupFactory::new();
        bind_factory.add_uniform(
            wgpu::ShaderStages::VERTEX,
            &global_uniform_buffer,
            wgpu::BufferSize::new(std::mem::size_of::<glam::Mat4>() as _),
        );
        bind_factory.add_uniform(
            wgpu::ShaderStages::VERTEX,
            &model_uniform_buffer,
            wgpu::BufferSize::new(std::mem::size_of::<glam::Mat4>() as _),
        );
        let (bind_group_layout, bind_group) = bind_factory.build(&ctx.device);

        let mut texture_bind_group_factory = BindGroupFactory::new();
        texture_bind_group_factory.add_texture_and_sampler(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            &texture.view,
            &texture.sampler,
        );
        let (texture_bind_group_layout, texture_bind_group) =
            texture_bind_group_factory.build(&ctx.device);

        let mut pipeline_factory = RenderPipelineFactory::new();

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
            bind_group_layout,
            bind_group,

            texture_bind_group_layout: Some(texture_bind_group_layout),
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
                color: [1.0, 1.0, 1.0],
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
