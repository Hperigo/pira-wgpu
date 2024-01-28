use crate::factories::texture::{SamplerOptions, Texture2dOptions, TextureBundle};

use crate::helpers::cameras;
use crate::state::State;
use crate::{factories, pipelines};
use image::EncodableLayout;
use wgpu::{BufferBinding, SamplerDescriptor, ShaderModuleDescriptor, TextureViewDescriptor};

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
        options: SkyRendererOptions,
    ) -> TextureBundle {
        let State { device, queue, .. } = state;

        let image = image.to_rgba32f();
        let SkyRendererOptions { label, dst_size } = options;

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

    pub fn create_iradiance_map(state: &State, input: &TextureBundle) -> TextureBundle {
        let State { device, queue, .. } = state;
        let label = Some("create_iradiance_map");

        let dst_size = input.texture.width();

        // This will be the result bundle image
        // TODO: create this texture via the texture factory
        let dst_texture = device.create_texture(&wgpu::TextureDescriptor {
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

        let dst_cube_view = dst_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("SKy texture view"),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            array_layer_count: Some(6),
            ..Default::default()
        });

        let dst_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("shader_iradiance compute"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader_iradiance.wgsl").into()),
        });

        let compute_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("HDR: equirect layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
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

        let iradiance_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("iradiance_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: "calc_iradiance",
        });

        let src_array_view = input.texture.create_view(&TextureViewDescriptor {
            label: Some("Src array view"),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });

        let dst_array_view = dst_texture.create_view(&TextureViewDescriptor {
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
                    resource: wgpu::BindingResource::TextureView(&src_array_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&dst_array_view),
                },
            ],
        });

        let mut encoder = device.create_command_encoder(&Default::default());
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label,
            timestamp_writes: None,
        });

        let num_workgroups = (dst_size + 15) / 16;
        pass.set_pipeline(&iradiance_pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(num_workgroups, num_workgroups, 6);

        drop(pass);

        queue.submit([encoder.finish()]);

        TextureBundle {
            view: dst_cube_view,
            texture: dst_texture,
            sampler: dst_sampler,
        }
    }

    pub fn new(state: &State, image: &image::DynamicImage, options: SkyRendererOptions) -> Self {
        let State { device, .. } = state;

        let textures = SkyRenderer::create_cube_map_textures_from_equi(state, image, options);

        let iradiance_texture = SkyRenderer::create_iradiance_map(state, &textures);

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
