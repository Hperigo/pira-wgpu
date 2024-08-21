use wgpu::{self, AddressMode, CommandEncoder, Features, TextureFormat, TextureView};
use winit::dpi::PhysicalSize;

use super::factories::texture::{DepthTextureFactory, Texture2dFactory, TextureBundle};

#[derive(Copy, Clone)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn width_f32(&self) -> f32 {
        self.width as f32
    }

    pub fn height_f32(&self) -> f32 {
        self.height as f32
    }

    pub fn into_array(&self) -> [f32; 2] {
        [self.width_f32(), self.height_f32()]
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.width_f32() / self.height_f32()
    }
}

pub struct State {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub config: wgpu::SurfaceConfiguration,
    pub window_surface: wgpu::Surface<'static>,

    pub depth_texture: Option<TextureBundle>,

    pub default_white_texture_bundle: TextureBundle,

    pub window_size: Size,

    pub delta_time: f32,

    pub sample_count: u32,
}

pub struct PerFrameData {
    pub encoder: CommandEncoder,
    pub view: TextureView,
    pub multisampled_view: TextureView,
}

impl State {
    pub async fn new(
        sample_count: u32,
        instance: wgpu::Instance,
        window_surface: wgpu::Surface<'static>,
        window_size: Size,
    ) -> State {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&window_surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: Features::TEXTURE_COMPRESSION_ASTC
                        | Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let surface_caps = window_surface.get_capabilities(&adapter);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_caps.formats[0],
            view_formats: Vec::new(),
            width: window_size.width,
            height: window_size.height,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            present_mode: surface_caps.present_modes[0],
            desired_maximum_frame_latency: 2,
        };

        window_surface.configure(&device, &config);

        let depth_texture =
            DepthTextureFactory::new(&device, &config, sample_count, "Default Depth texture");

        let mut tf = Texture2dFactory::new(2, 2);
        tf.set_sampler_descriptor(wgpu::SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            ..Default::default()
        });

        // #[rustfmt::skip]
        // let data: [u8; 16] = [
        //     255, 255, 255, 255,
        //     0, 0, 0, 255,
        //     0, 0, 0, 255,
        //     255, 255, 255, 255,
        // ]; // checkerboard

        #[rustfmt::skip]
        let data: [u8; 16] = [
            255, 255, 255, 255,
            255, 255, 255, 255,
            255, 255, 255, 255,
            255, 255, 255, 255,
        ]; // checkerboard

        let texture_bundle = tf.get_texture_and_sampler(&device, &queue, &data);

        State {
            instance,
            adapter,

            device,
            queue,

            window_surface,
            config,
            depth_texture: Some(depth_texture),

            default_white_texture_bundle: texture_bundle,

            delta_time: 0.0,
            window_size,
            sample_count,
        }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.window_surface.configure(&self.device, &self.config);

            if self.depth_texture.is_some() {
                self.depth_texture = Some(DepthTextureFactory::new(
                    &self.device,
                    &self.config,
                    self.get_sample_count(),
                    "Default Depth texture",
                ));
            }
        }
    }

    pub fn render<'a, F: 'a>(&self, render_callback: F)
    where
        F: FnOnce(&State, &mut PerFrameData),
    {
        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let output_surface = self.window_surface.get_current_texture().unwrap();

        let view = output_surface
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let multisampled_texture_extent = wgpu::Extent3d {
            width: self.config.width,
            height: self.config.height,
            depth_or_array_layers: 1,
        };

        let multisampled_frame_descriptor = wgpu::TextureDescriptor {
            size: multisampled_texture_extent,
            mip_level_count: 1,
            sample_count: 4,
            view_formats: &[TextureFormat::Bgra8UnormSrgb],
            dimension: wgpu::TextureDimension::D2,
            format: self.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
        };

        let multisampled_view = self
            .device
            .create_texture(&multisampled_frame_descriptor)
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut per_frame_data = PerFrameData {
            view,
            encoder,
            multisampled_view,
        };

        {
            render_callback(self, &mut per_frame_data);
        }

        self.queue
            .submit(std::iter::once(per_frame_data.encoder.finish()));
        output_surface.present();
    }

    pub fn get_sample_count(&self) -> u32 {
        return self.sample_count;
    }
}
