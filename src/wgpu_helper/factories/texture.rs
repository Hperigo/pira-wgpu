use wgpu::util::DeviceExt;

pub struct Texture2dFactory<'a> {
    sampler_descriptor: wgpu::SamplerDescriptor<'a>,
    texture_descriptor: wgpu::TextureDescriptor<'a>,
}

pub struct TextureBundle {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
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

                mag_filter: wgpu::FilterMode::Linear,
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
    ) -> (wgpu::Texture, wgpu::TextureView, wgpu::Sampler) {
        let texture = device.create_texture_with_data(&queue, &self.texture_descriptor, &data);

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&self.sampler_descriptor);

        (texture, texture_view, sampler)
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
