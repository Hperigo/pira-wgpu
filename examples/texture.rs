use glam::Mat4;
use wgpu::util::DeviceExt;
use wgpu::{self, BindGroup};

use pira_wgpu::factories::texture::{SamplerOptions, Texture2dOptions, TextureBundle};
use pira_wgpu::factories::{self, BindGroupFactory};
use pira_wgpu::framework;
use pira_wgpu::framework::Application;
use pira_wgpu::pipelines::{self, shadeless, ModelUniform};
use pira_wgpu::state::State;
use wgpu::RenderPass;

use winit::dpi::PhysicalSize;

use image::EncodableLayout;
use winit::event::ElementState;
use winit::keyboard::KeyCode;

struct MyExample {
    clear_color: [f32; 4],
    buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    pipeline: shadeless::ShadelessPipeline,

    use_toronto_photo: bool,
    _texture_bundle_toronto: TextureBundle,
    toronto_bind_group: BindGroup,
}

impl Application for MyExample {
    fn init(state: &State) -> Self {
        let vertices = vec![
            shadeless::Vertex::new([-0.8, -0.8, 0.0], [0.0, 0.0], [1.0, 1.0, 1.0, 1.0]),
            shadeless::Vertex::new([0.8, 0.8, 0.0], [1.0, 1.0], [1.0, 1.0, 1.0, 1.0]),
            shadeless::Vertex::new([0.8, -0.8, 0.0], [1.0, 0.0], [1.0, 1.0, 1.0, 1.0]),
            shadeless::Vertex::new([-0.8, 0.8, 0.0], [0.0, 1.0], [1.0, 1.0, 1.0, 1.0]),
        ];

        let mut indices: [u16; 6] = [0, 1, 2, 0, 3, 1];
        indices.reverse();

        let image = image::open("./assets/rusty.png").unwrap().to_rgba8();
        let texture_bundle = factories::Texture2dFactory::new_with_options(
            &state,
            [image.width(), image.height()],
            Texture2dOptions {
                ..Default::default()
            },
            SamplerOptions {
                filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            },
            image.as_bytes(),
        );

        let image_toronto = image::open("./assets/toronto-skyline.jpeg")
            .unwrap()
            .to_rgba8();
        let texture_bundle_toronto = factories::Texture2dFactory::new_with_options(
            &state,
            [image_toronto.width(), image_toronto.height()],
            Texture2dOptions {
                ..Default::default()
            },
            SamplerOptions {
                filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            },
            image_toronto.as_bytes(),
        );

        let mut texture_bind_group_factory = BindGroupFactory::new();
        texture_bind_group_factory.add_texture_and_sampler(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            &texture_bundle_toronto.view,
            &texture_bundle_toronto.sampler,
        );
        let (_toronto_bind_group_layout, toronto_bind_group) =
            texture_bind_group_factory.build(&state.device);

        let pipeline = shadeless::ShadelessPipeline::new_with_texture(
            state,
            &texture_bundle,
            wgpu::PrimitiveTopology::TriangleList,
            true,
            None,
        );

        MyExample {
            clear_color: [0.5, 0.1, 0.1, 1.0],
            buffer: state
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex buffer"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                }),
            index_buffer: state
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Index Buffer"),
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsages::INDEX,
                }),

            pipeline,

            use_toronto_photo: false,
            _texture_bundle_toronto: texture_bundle_toronto,
            toronto_bind_group,
        }
    }

    fn clear_color(&self) -> wgpu::Color {
        wgpu::Color {
            r: self.clear_color[0] as f64,
            g: self.clear_color[1] as f64,
            b: self.clear_color[2] as f64,
            a: self.clear_color[3] as f64,
        }
    }

    fn update(&mut self, _state: &mut State, _frame_count: u64, _delta_time: f64) {}

    fn resize(
        &mut self,
        _config: &wgpu::SurfaceConfiguration,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) {
    }

    fn event(&mut self, _state: &State, _event: &winit::event::WindowEvent) {
        match _event {
            winit::event::WindowEvent::KeyboardInput { event, .. } => match event.physical_key {
                winit::keyboard::PhysicalKey::Code(KeyCode::KeyT) => match event.state {
                    ElementState::Released => {
                        self.use_toronto_photo = !self.use_toronto_photo;
                    }
                    ElementState::Pressed => {}
                },
                _ => (),
            },
            _ => {}
        }
    }

    fn render<'rpass>(&'rpass self, state: &State, render_pass: &mut RenderPass<'rpass>) {
        let ortho_perspective_matrix = glam::Mat4::IDENTITY;
        pipelines::write_global_uniform_buffer(
            ortho_perspective_matrix,
            self.pipeline.global_uniform_buffer.as_ref().unwrap(),
            &state.queue,
        );

        let matrices = [ModelUniform {
            model_matrix: Mat4::IDENTITY,
        }];

        pipelines::write_uniform_buffer(
            &matrices,
            &self.pipeline.model_uniform_buffer.as_ref().unwrap(),
            &state.queue,
            &state.device,
        );

        render_pass.set_bind_group(0, &self.pipeline.bind_group, &[0, 0 as u32]);
        if self.use_toronto_photo {
            render_pass.set_bind_group(1, &self.toronto_bind_group, &[]);
        } else {
            render_pass.set_bind_group(1, self.pipeline.texture_bind_group.as_ref().unwrap(), &[]);
        }

        render_pass.set_pipeline(&self.pipeline.pipeline);
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_vertex_buffer(0, self.buffer.slice(..));
        render_pass.draw_indexed(0..6, 0, 0..1);
    }
}

fn main() {
    let dpi = 1;

    framework::run::<MyExample>(
        "simple_app",
        PhysicalSize {
            width: 460 * dpi,
            height: 307 * dpi,
        },
        4,
    );
}
