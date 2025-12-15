use wgpu::{
    self, AddressMode, BlendState, CommandEncoder, Features, TexelCopyTextureInfoBase, TextureFormat, TextureView, util::TextureBlitterBuilder
};
use winit::dpi::PhysicalSize;

use crate::factories;

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
            .request_device(&wgpu::DeviceDescriptor {
                required_features: Features::TEXTURE_COMPRESSION_ASTC
                    | Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::default(),
                label: None,
            })
            .await
            .unwrap();

        let surface_caps = window_surface.get_capabilities(&adapter);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
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
            sample_count: self.get_sample_count(),
            view_formats: &[TextureFormat::Bgra8UnormSrgb],
            dimension: wgpu::TextureDimension::D2,
            format: self.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            label: None,
        };

        let multisampled_view = self
            .device
            .create_texture(&multisampled_frame_descriptor)
            .create_view(&wgpu::TextureViewDescriptor{
                usage : Some(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING),
                ..Default::default()
            });

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

    pub fn save_window_surface_to_file(&self, _path: &str) {
        let output_surface = self.window_surface.get_current_texture().unwrap();
        let width = output_surface.texture.width();
        let height = output_surface.texture.height();


        let format = wgpu::TextureFormat::Bgra8UnormSrgb; // Ensure this matches your texture's format

        let mid_texture = factories::Texture2dFactory::new_with_options(
            self,
            [width, height],
            factories::texture::Texture2dOptions {
                format,
                usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                ..Default::default()
            },
            factories::texture::SamplerOptions {
                ..Default::default()
            },
            &[],
        );

        let u8_size = std::mem::size_of::<u8>() as u32;
        let bytes_per_pixel = u8_size * 4; // RGBA has 4 channels
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = (unpadded_bytes_per_row + align - 1) & !(align - 1);
        let buffer_size = padded_bytes_per_row * height;

        // println!("Saving texture to file: {}, {}, {}", path, width, height);

        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: buffer_size as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // 3. Encode the copy command
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Copy Encoder"),
            });

            // let source_view = &output_surface.texture.create_view(&wgpu::TextureViewDescriptor{ 
            //     usage : Some( wgpu::TextureUsages::RENDER_ATTACHMENT),
            //     ..Default::default()                    
            // });
            // let blitter =  TextureBlitterBuilder::new(&self.device, format).blend_state(BlendState::ALPHA_BLENDING).build();
            // // blitter.copy(&self.device, &mut encoder, , mid_texture);
            // blitter.copy(&self.device, &mut encoder, source_view, &mid_texture.view);

            let texture_size = wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            };

        encoder.copy_texture_to_texture(
            TexelCopyTextureInfoBase {
                texture: & output_surface.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            TexelCopyTextureInfoBase {
                texture: &mid_texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            texture_size
        );

        println!(
            "Encoding copy command... {:?} - {:?}",
            output_surface.texture.usage(),
            self.config
        );

            encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &output_surface.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &output_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            texture_size,
        );

        self.queue.submit(Some(encoder.finish()));
        // // 5. Map the buffer and wait
        let buffer_slice = output_buffer.slice(..);


        let (tx, rx) = futures::channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });

        self.device.poll(wgpu::wgt::PollType::Wait { submission_index: None, timeout: None }).unwrap();
        // rx.await??;

        pollster::block_on( rx ).unwrap().unwrap();

        // Get the mapped data
        let data = buffer_slice.get_mapped_range();

        println!("Creating image buffer... {}", data.len());

        let mut img_data = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height {
            let start = (row * padded_bytes_per_row) as usize;
            let end = start + (width * bytes_per_pixel) as usize;
            img_data.extend_from_slice(&data[start..end]);
        }

        // Save using the image crate
        image::save_buffer(
            "Test.jpg",
            &img_data,
            width,
            height,
            image::ColorType::Rgba8,
        ).unwrap();

        drop(data);
        output_buffer.unmap();

    }

    pub fn get_sample_count(&self) -> u32 {
        return self.sample_count;
    }
}
