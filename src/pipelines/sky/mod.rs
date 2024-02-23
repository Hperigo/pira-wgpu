use std::default;

use crate::factories::texture::{self, SamplerOptions, Texture2dOptions, TextureBundle};

use crate::helpers::geometry::{cube, GeometryFactory};
use crate::helpers::{self, cameras};
use crate::pipelines::shadeless;
use crate::state::State;
use crate::{factories, pipelines};
use image::EncodableLayout;
use wgpu::{
    BufferBinding, PipelineLayoutDescriptor, RenderPipelineDescriptor, SamplerDescriptor,
    ShaderModuleDescriptor, ShaderStages, TextureViewDescriptor, TextureViewDimension,
};

/*
Notes:
1. Finish rendering the 6 layers of the cube map texture
2. Correct rotation on cube map (prob will have to use a camera matrix)
3. Cleanup texture creation code


*/

#[repr(C, align(16))]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniform {
    pub view_pos: [f32; 4],
    pub view: [f32; 16],
    pub view_proj: [f32; 16],
    pub inv_proj: [f32; 16],
    pub inv_view: [f32; 16],
}

pub struct SkyRenderer {
    pub textures: TextureBundle,
    pub iradiance_texture: TextureBundle,

    pub pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,
    pub uniform_buffer: wgpu::Buffer,
}

pub struct SkyRendererOptions<'a> {
    pub label: Option<&'a str>,
    pub dst_size: u32,
}

impl<'a> Default for SkyRendererOptions<'a> {
    fn default() -> Self {
        Self {
            label: Some("Sky Renderer"),
            dst_size: 512,
        }
    }
}

impl SkyRenderer {
    pub fn create_cube_map_textures_from_equi(
        state: &State,
        image: &image::DynamicImage,
        options: &SkyRendererOptions,
    ) -> TextureBundle {
        let State { device, queue, .. } = state;

        let image = image.to_rgba32f();
        let SkyRendererOptions { label, dst_size } = *options;

        // This will be the result bundle image
        // TODO: create this texture via the texture factory
        let cube_texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width: dst_size,
                height: dst_size,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        let cube_view = cube_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("SKy texture view"),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            array_layer_count: Some(6),
            ..Default::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // -------- Compute shader pipeline -----------
        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Equirectangular to cubemap compute"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader_env_to_cubemap.wgsl").into()),
        });

        //create equirectangular texture
        let env_texture_bundle = factories::Texture2dFactory::new_with_options(
            state,
            [image.width(), image.height()],
            Texture2dOptions {
                format: wgpu::TextureFormat::Rgba32Float,
                ..Default::default()
            },
            SamplerOptions {
                filter: wgpu::FilterMode::Nearest,
                address_mode: wgpu::AddressMode::Repeat,
                mipmap_filter: wgpu::FilterMode::Nearest,
            },
            &image.as_bytes(),
        );

        let compute_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("HDR: equirect layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba32Float,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                    },
                    count: None,
                },
            ],
        });

        // TODO: Create pipeline layout via factory functions -----
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&compute_layout],
            push_constant_ranges: &[],
        });

        let equirect_to_cubemap =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("equirect_to_cubemap"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: "compute_equirect_to_cubemap",
            });

        let dst_view = cube_texture.create_view(&TextureViewDescriptor {
            label,
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });

        // TODO: Create bind groups via factory functions -----
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label,
            layout: &compute_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&env_texture_bundle.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&dst_view),
                },
            ],
        });

        let mut encoder = device.create_command_encoder(&Default::default());
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label,
            timestamp_writes: None,
        });

        let num_workgroups = (dst_size + 15) / 16;
        pass.set_pipeline(&equirect_to_cubemap);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(num_workgroups, num_workgroups, 6);

        drop(pass);

        queue.submit([encoder.finish()]);

        return TextureBundle {
            sampler,
            view: cube_view,
            texture: cube_texture,
        };
    }

    pub fn create_irradiance_map_from_env(
        state: &State,
        image: &image::DynamicImage,
        options: &SkyRendererOptions, //make options clonable instead of reference,
    ) -> TextureBundle {
        todo!("Doesnt work!");
        let State { device, queue, .. } = state;

        let image = image.to_rgba32f();
        let SkyRendererOptions { label, dst_size } = *options;

        // This will be the result bundle image
        // TODO: create this texture via the texture factory
        let cube_texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width: 128,
                height: 128,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        let cube_view = cube_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("SKy texture view"),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            array_layer_count: Some(6),
            ..Default::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // -------- Compute shader pipeline -----------
        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Equirectangular to cubemap compute"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader_irradiance_compute.wgsl").into()),
        });

        //create equirectangular texture
        let env_texture_bundle = factories::Texture2dFactory::new_with_options(
            state,
            [image.width(), image.height()],
            Texture2dOptions {
                format: wgpu::TextureFormat::Rgba32Float,
                ..Default::default()
            },
            SamplerOptions {
                filter: wgpu::FilterMode::Nearest,
                address_mode: wgpu::AddressMode::Repeat,
                mipmap_filter: wgpu::FilterMode::Nearest,
            },
            &image.as_bytes(),
        );

        let compute_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("HDR: equirect layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba32Float,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                    },
                    count: None,
                },
            ],
        });

        // TODO: Create pipeline layout via factory functions -----
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&compute_layout],
            push_constant_ranges: &[],
        });

        let equirect_to_cubemap =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("equirect_to_cubemap"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: "calc_iradiance",
            });

        let dst_view = cube_texture.create_view(&TextureViewDescriptor {
            label,
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });

        // TODO: Create bind groups via factory functions -----
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label,
            layout: &compute_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&env_texture_bundle.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&dst_view),
                },
            ],
        });

        let mut encoder = device.create_command_encoder(&Default::default());
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label,
            timestamp_writes: None,
        });

        let num_workgroups = (dst_size + 15) / 16;
        pass.set_pipeline(&equirect_to_cubemap);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(num_workgroups, num_workgroups, 6);

        drop(pass);

        queue.submit([encoder.finish()]);

        return TextureBundle {
            sampler,
            view: cube_view,
            texture: cube_texture,
        };
    }

    pub fn create_iradiance_map(state: &State, input: &TextureBundle) -> TextureBundle {
        let State {
            instance,
            adapter,
            device,
            queue,
            config,
            window_surface,
            depth_texture,
            default_white_texture_bundle,
            window_size,
            delta_time,
            sample_count,
        } = state;

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Equirectangular to cubemap compute"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader_irradiance_cubemap.wgsl").into()),
        });

        let mut cube_geo = helpers::geometry::cube::Cube::new(1.0);
        cube_geo.texture_coords();
        let mesh =
            shadeless::ShadelessPipeline::get_buffers_from_geometry(state, &cube_geo.geometry);

        let uniform_buffer = pipelines::create_uniform_buffer::<glam::Mat4>(1, device);

        let rotation_matrix: glam::Mat4 = glam::Mat4::IDENTITY;
        pipelines::write_uniform_buffer(rotation_matrix.as_ref(), &uniform_buffer, queue, device);

        let (bind_group_layout, bind_group) = factories::BindGroupFactory::new()
            .add_texture_sky_sampler(wgpu::ShaderStages::FRAGMENT, &input.view, &input.sampler)
            .add_uniform(ShaderStages::FRAGMENT, &uniform_buffer, None)
            .build(device);

        let pipeline_layout = PipelineLayoutDescriptor {
            label: Some("Equirectangular-pipeline"),
            bind_group_layouts: &[&bind_group_layout],
            ..Default::default()
        };
        println!("=======");

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&device.create_pipeline_layout(&pipeline_layout)),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::TextureFormat::Rgba8UnormSrgb.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        //-----------------------------------------------
        let render_target = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: input.texture.width() as u32,
                height: input.texture.height() as u32,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
        });

        let matrices = [
            glam::Mat4::look_at_rh(
                glam::Vec3::ZERO,
                glam::vec3(1.0, 0.0, 0.0),
                glam::vec3(0.0, 1.0, 0.0),
            ),
            glam::Mat4::look_at_rh(
                glam::Vec3::ZERO,
                glam::vec3(-1.0, 0.0, 0.0),
                glam::vec3(0.0, 1.0, 0.0),
            ),
            glam::Mat4::look_at_rh(
                glam::Vec3::ZERO,
                glam::vec3(0.0, 1.0, 0.0),
                glam::vec3(0.0, 0.0, -1.0),
            ),
            glam::Mat4::look_at_rh(
                glam::Vec3::ZERO,
                glam::vec3(0.0, -1.0, 0.0),
                glam::vec3(0.0, 0.0, 1.0),
            ),
            glam::Mat4::look_at_rh(
                glam::Vec3::ZERO,
                glam::vec3(0.0, 0.0, 1.0),
                glam::vec3(0.0, 1.0, 0.0),
            ),
            glam::Mat4::look_at_rh(
                glam::Vec3::ZERO,
                glam::vec3(0.0, 0.0, -1.0),
                glam::vec3(0.0, 1.0, 0.0),
            ),
        ];

        for i in 0..6 {
            pipelines::write_uniform_buffer(matrices[i].as_ref(), &uniform_buffer, queue, device);

            let texture_view = render_target.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(TextureViewDimension::D2),
                base_array_layer: i as u32,
                array_layer_count: Some(1),
                label: Some("Temp view"),
                ..Default::default()
            });

            let mut command_encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            {
                let mut render_pass =
                    command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &texture_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        occlusion_query_set: None,
                        timestamp_writes: None,
                    });
                render_pass.set_pipeline(&pipeline);
                render_pass.set_bind_group(0, &bind_group, &[0]);
                render_pass.draw(0..6, 0..1);
            }
            queue.submit(Some(command_encoder.finish()));
        }

        let dst_cube_view = render_target.create_view(&TextureViewDescriptor {
            array_layer_count: Some(6),
            mip_level_count: Some(1),
            dimension: Some(TextureViewDimension::Cube),
            label: Some("Cube View"),
            ..Default::default()
        });

        let dst_cube_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Cube Sampler"),
            ..Default::default()
        });

        println!("Done generating cube map");
        TextureBundle {
            view: dst_cube_view,
            texture: render_target,
            sampler: dst_cube_sampler,
        }
    }

    pub fn new(state: &State, image: &image::DynamicImage, options: SkyRendererOptions) -> Self {
        let State { device, .. } = state;

        let textures = SkyRenderer::create_cube_map_textures_from_equi(state, image, &options);

        let iradiance_texture = SkyRenderer::create_iradiance_map(state, &textures); //SkyRenderer::create_irradiance_map_from_env(state, image, &options);

        let uniform_buffer = pipelines::create_uniform_buffer::<Uniform>(1, device);

        let environment_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("environment_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::Cube,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let environment_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("environment_bind_group"),
            layout: &environment_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&textures.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&device.create_sampler(
                        &SamplerDescriptor {
                            ..Default::default()
                        },
                    )),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(BufferBinding {
                        buffer: &uniform_buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new(std::mem::size_of::<Uniform>() as _),
                    }),
                },
            ],
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Sky"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader_sky_render.wgsl").into()),
        });

        let mut sky_render_pipeline = factories::RenderPipelineFactory::new();
        sky_render_pipeline.set_cull_mode(Some(wgpu::Face::Back));

        let pipeline =
            sky_render_pipeline.create_render_pipeline(state, &shader, &[&environment_layout]);

        Self {
            pipeline,
            textures,
            iradiance_texture,
            bind_group: environment_bind_group,
            uniform_buffer,
        }
    }

    pub fn set_uniform_buffer(&mut self, _camera: cameras::PespectiveCamera) {}

    pub fn draw<'rpass>(&'rpass self, render_pass: &mut wgpu::RenderPass<'rpass>) {
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_pipeline(&self.pipeline);
        render_pass.draw(0..3, 0..1);
    }
}
