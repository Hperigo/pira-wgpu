use crate::factories::texture::TextureBundle;
use crate::helpers::geometry::{self, attribute_names, GeometryData};
use crate::state::State;

use crate::factories::{self, BindGroupFactory, RenderPipelineFactory};

use wgpu::PrimitiveTopology;

use wgpu::util::DeviceExt;

use super::sky::SkyRenderer;
use super::{create_global_uniform, create_uniform_buffer, ModelUniform, ViewUniform};

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
                source: wgpu::ShaderSource::Wgsl(include_str!("shader_pbr.wgsl").into()),
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
            pipeline_factory
                .add_depth_stencil(factories::render_pipeline::DepthConfig::DefaultWrite);
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
        if let Some(vcolor) = vcolors_option {
            let mut index = 0;
            for i in 0..vertices.len() {
                vertices[i].color = [
                    vcolor[index],
                    vcolor[index + 1],
                    vcolor[index + 2],
                    vcolor[index + 3],
                ];
                index += 4;
            }
        }

        // Normals -----

        let normals_option = geo_data.attributes.get(&attribute_names::NORMALS);
        if let Some(normals) = normals_option {
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
