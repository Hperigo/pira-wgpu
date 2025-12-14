use wgpu::util::{DeviceExt, TextureDataOrder};

use crate::state::State;

pub struct Texture2dFactory<'a> {
    sampler_descriptor: wgpu::SamplerDescriptor<'a>,
    texture_descriptor: wgpu::TextureDescriptor<'a>,
}


#[derive(Debug)]
pub struct TextureBundle {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

pub struct SamplerOptions {
    pub address_mode: wgpu::AddressMode,
    pub filter: wgpu::FilterMode,
    pub mipmap_filter: wgpu::FilterMode,
}

impl Default for SamplerOptions {
    fn default() -> Self {
        Self {
            address_mode: wgpu::AddressMode::ClampToEdge,
            filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
        }
    }
}

pub struct Texture2dOptions {
    pub mip_level_count: u32,
    pub sample_count: u32,
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
    pub label: Option<&'static str>,
}

impl Default for Texture2dOptions {
    fn default() -> Self {
        Self {
            mip_level_count: 1,
            sample_count: 1,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: None,
        }
    }
}

impl<'a> Texture2dFactory<'a> {
    pub fn new(width: u32, height: u32) -> Self {
        let texture_size = wgpu::Extent3d {
            width: width,
            height: height,
            depth_or_array_layers: 1,
        };

        Self {
            sampler_descriptor: wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,

                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,

                ..Default::default()
            },
            texture_descriptor: wgpu::TextureDescriptor {
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                view_formats: &[],
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                label: Some("texture"),
            },
        }
    }

    pub fn new_with_options(
        state: &State,
        size: [u32; 2],
        texture_options: Texture2dOptions,
        sampler_options: SamplerOptions,
        data: &[u8],
    ) -> TextureBundle {
        let texture_size = wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        };

        let sampler_descriptor = wgpu::SamplerDescriptor {
            address_mode_u: sampler_options.address_mode,
            address_mode_v: sampler_options.address_mode,
            address_mode_w: sampler_options.address_mode,

            mag_filter: sampler_options.filter,
            min_filter: sampler_options.filter,
            mipmap_filter: sampler_options.mipmap_filter,

            ..Default::default()
        };

        let texture_descriptor = wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: texture_options.mip_level_count,
            sample_count: texture_options.sample_count,
            view_formats: &[],
            dimension: wgpu::TextureDimension::D2,
            format: texture_options.format,
            usage: texture_options.usage,
            label: texture_options.label,
        };

        let texture = if data.len() == 0 {
            state.device.create_texture(&texture_descriptor)
        } else {
            state.device.create_texture_with_data(
                &state.queue,
                &texture_descriptor,
                TextureDataOrder::default(),
                &data,
            )
        };

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = state.device.create_sampler(&sampler_descriptor);

        TextureBundle {
            texture,
            view,
            sampler,
        }
    }

    pub fn new_ktx(
        state: &State,
        size: [u32; 2],
        texture_options: Texture2dOptions,
        sampler_options: SamplerOptions,
        data: &[u8],
    ) -> TextureBundle {
        let texture_size = wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        };

        let sampler_descriptor = wgpu::SamplerDescriptor {
            address_mode_u: sampler_options.address_mode,
            address_mode_v: sampler_options.address_mode,
            address_mode_w: sampler_options.address_mode,

            mag_filter: sampler_options.filter,
            min_filter: sampler_options.filter,
            mipmap_filter: sampler_options.mipmap_filter,

            ..Default::default()
        };

        let texture_descriptor = wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: texture_options.mip_level_count,
            sample_count: texture_options.sample_count,
            view_formats: &[],
            dimension: wgpu::TextureDimension::D2,
            format: texture_options.format,
            usage: texture_options.usage,
            label: texture_options.label,
        };

        let texture = if data.len() == 0 {
            state.device.create_texture(&texture_descriptor)
        } else {
            state.device.create_texture_with_data(
                &state.queue,
                &texture_descriptor,
                TextureDataOrder::default(),
                &data,
            )
        };

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = state.device.create_sampler(&sampler_descriptor);

        TextureBundle {
            texture,
            view,
            sampler,
        }
    }

    pub fn set_sampler_descriptor<'b>(
        &'b mut self,
        sampler: wgpu::SamplerDescriptor<'a>,
    ) -> &'b Self {
        self.sampler_descriptor = sampler;
        self
    }

    pub fn set_texture_descriptor<'b>(
        &'b mut self,
        texture_descriptor: wgpu::TextureDescriptor<'a>,
    ) -> &'b Self {
        self.texture_descriptor = texture_descriptor;
        self
    }

    pub fn get_texture_and_sampler(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[u8],
    ) -> TextureBundle {
        let texture = if data.len() != 0 {
            device.create_texture_with_data(
                &queue,
                &self.texture_descriptor,
                TextureDataOrder::default(),
                &data,
            )
        } else {
            device.create_texture(&self.texture_descriptor)
        };

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&self.sampler_descriptor);

        TextureBundle {
            texture,
            view,
            sampler,
        }
    }
}

pub struct DepthTextureFactory {}

impl DepthTextureFactory {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        sample_count: u32,
        label: &str,
    ) -> TextureBundle {

        println!("Creating depth texture with size: {}, {}", config.width, config.height);

        let size = wgpu::Extent3d {
            // 2.
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };

        pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus; // 1.

        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            view_formats: &[],
            sample_count: sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT // 3.
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST,
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: None, //Some(wgpu::CompareFunction::LessEqual), // 5.
            ..Default::default()
        });

        TextureBundle {
            texture,
            view,
            sampler,
        }
    }
}
