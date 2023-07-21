use glam::Mat4;
use wgpu::util::DeviceExt;
use wgpu::{RenderPass, TextureFormat};
use wgpu_app_lib::framework::Application;
use wgpu_app_lib::pipelines::{self, shadeless, ModelUniform};
use wgpu_app_lib::wgpu_helper::factories::texture::{SamplerOptions, Texture2dOptions};
use wgpu_app_lib::wgpu_helper::factories::{self};
use wgpu_app_lib::wgpu_helper::State;
use wgpu_app_lib::{framework, wgpu_helper};
use winit::dpi::PhysicalSize;

use image::{self, EncodableLayout, ImageFormat};

struct MyExample {
    clear_color: [f32; 4],
    buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    pipeline: shadeless::ShadelessPipeline,
}

impl Application for MyExample {
    fn init(state: &wgpu_helper::State) -> Self {
        let vertices = vec![
            shadeless::Vertex {
                position: [-0.8, -0.8, 0.0],
                uv: [0.0, 0.0],
                color: [1.0, 0.0, 0.0],
            },
            shadeless::Vertex {
                position: [0.8, 0.8, 0.0],
                uv: [1.0, 1.0],
                color: [0.0, 1.0, 0.0],
            },
            shadeless::Vertex {
                position: [0.8, -0.8, 0.0],
                uv: [1.0, 0.0],
                color: [0.0, 0.0, 1.0],
            },
            shadeless::Vertex {
                position: [-0.8, 0.8, 0.0],
                uv: [0.0, 1.0],
                color: [1.0, 1.0, 1.0],
            },
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

        let pipeline = shadeless::ShadelessPipeline::new_with_texture(
            state,
            &texture_bundle,
            wgpu::PrimitiveTopology::TriangleList,
        );

        // let attribs = wgpu::vertex_attr_array![ 0 => Float32x3, 1 => Float32x3 ];
        // let stride = std::mem::size_of::<Vertex>() as u64;
        // let mut pipeline_factory = RenderPipelineFactory::new();
        // pipeline_factory.add_vertex_attributes(&attribs, stride);
        // pipeline_factory.add_depth_stencil();

        // let pipeline =
        //     pipeline_factory.create_render_pipeline(&state, &shader_module, &[&bind_group_layout]);

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

    fn update(&mut self, _state: &State, ui: &mut imgui::Ui, _frame_count: u64, _delta_time: f64) {
        let w = ui
            .window("debug")
            .size([200.0, 300.0], imgui::Condition::FirstUseEver)
            .begin();
        if let Some(w) = w {
            imgui::Drag::new("clear color")
                .speed(0.01)
                .range(0.0, 1.0)
                .build_array(ui, &mut self.clear_color);
            w.end();
        }
    }

    fn resize(
        &mut self,
        _config: &wgpu::SurfaceConfiguration,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) {
    }

    fn event(&mut self, _state: &State, _event: &winit::event::WindowEvent) {}

    fn render<'rpass>(
        &'rpass self,
        state: &wgpu_helper::State,
        render_pass: &mut RenderPass<'rpass>,
    ) {
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
        render_pass.set_bind_group(1, self.pipeline.texture_bind_group.as_ref().unwrap(), &[]);
        render_pass.set_pipeline(&self.pipeline.pipeline);
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_vertex_buffer(0, self.buffer.slice(..));
        render_pass.draw_indexed(0..6, 0, 0..1);
    }
}

fn main() {
    framework::run::<MyExample>(
        "simple_app",
        PhysicalSize {
            width: 460,
            height: 307,
        },
        4,
    );
}
