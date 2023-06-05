use wgpu::{self, CommandEncoder, TextureFormat, TextureView};

use super::{factories, TextureBundle};

pub struct State {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub config: wgpu::SurfaceConfiguration,
    pub window_surface: wgpu::Surface,

    pub depth_texture: Option<TextureBundle>,

    pub default_white_texture: wgpu::Texture,
}

pub struct PerFrameData {
    pub encoder: CommandEncoder,
    pub view: TextureView,
    pub multisampled_view: TextureView,
}

impl State {
    pub async fn new(window: &winit::window::Window) -> State {
        let instance = wgpu::Instance::default();
        let window_surface = unsafe { instance.create_surface(&window).unwrap() };

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
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let window_surface = unsafe { instance.create_surface(&window).unwrap() };

        let formats = window_surface.get_capabilities(&adapter).formats;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: formats[0],
            view_formats: Vec::new(),
            width: 1920,
            height: 1080,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            present_mode: wgpu::PresentMode::Immediate,
        };

        window_surface.configure(&device, &config);

        let depth_texture =
            factories::texture::DepthTextureFactory::new(&device, &config, "Default Depth texture");

        let tf = factories::Texture2dFactory::new(2, 2);
        let data: [u8; 16] = [
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        ];
        let (_texture, _view, _sampler) = tf.get_texture_and_sampler(&device, &queue, &data);

        State {
            instance,
            adapter,

            device,
            queue,

            window_surface,
            config,
            depth_texture: Some(depth_texture),

            default_white_texture: _texture,
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
            sample_count: Self::get_sample_count(),
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

    pub fn get_sample_count() -> u32 {
        return 4;
    }
}