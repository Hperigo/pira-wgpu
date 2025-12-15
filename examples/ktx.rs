#![allow(dead_code)]
#![allow(unused_variables)]


use pira_wgpu::factories::texture::{SamplerOptions, Texture2dOptions, TextureBundle};
use pira_wgpu::factories::{self, BindGroupFactory};
use pira_wgpu::framework::{self, Application};
use pira_wgpu::pipelines::{self, shadeless, ModelUniform};
use pira_wgpu::state::State;
use wgpu::BindGroup;
use winit::dpi::PhysicalSize;

use ktx2;
use wgpu::util::DeviceExt;

struct KtxExample {
    clear_color: [f32; 4],
    buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    pipeline: shadeless::ShadelessPipeline,

    use_toronto_photo: bool,
    texture_bundle: TextureBundle,
    texture_bind_group: BindGroup,


    save_frame : bool,
}

impl Application for KtxExample {
    fn init(state: &State) -> Self {
        let bytes = include_bytes!("../assets/toronto-skyline.ktx2");
        let reader = ktx2::Reader::new(bytes).expect("Can't create reader"); // Crate instance of reader.
        let header = reader.header();
        println!("Header: {:#?}", header);

        let levels = reader.levels().collect::<Vec<_>>();

        let data = levels[0];

        let texture_bundle = factories::Texture2dFactory::new_ktx(
            &state,
            [header.pixel_width, header.pixel_height],
            Texture2dOptions {
                format: wgpu::TextureFormat::Astc {
                    block: wgpu::AstcBlock::B4x4,
                    channel: wgpu::AstcChannel::UnormSrgb,
                },
                ..Default::default()
            },
            SamplerOptions {
                filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            },
            data,
        );

        let vertices = vec![
            shadeless::Vertex::new([-0.8, -0.8, 0.0], [0.0, 0.0], [1.0, 1.0, 1.0, 1.0]),
            shadeless::Vertex::new([0.8, 0.8, 0.0], [1.0, 1.0], [1.0, 1.0, 1.0, 1.0]),
            shadeless::Vertex::new([0.8, -0.8, 0.0], [1.0, 0.0], [1.0, 1.0, 1.0, 1.0]),
            shadeless::Vertex::new([-0.8, 0.8, 0.0], [0.0, 1.0], [1.0, 1.0, 1.0, 1.0]),
        ];

        let mut indices: [u16; 6] = [0, 1, 2, 0, 3, 1];
        indices.reverse();

        let mut texture_bind_group_factory = BindGroupFactory::new();
        texture_bind_group_factory.add_texture_and_sampler(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            &texture_bundle.view,
            &texture_bundle.sampler,
        );
        let (_toronto_bind_group_layout, texture_bind_group) =
            texture_bind_group_factory.build(&state.device);

        let pipeline = shadeless::ShadelessPipeline::new_with_texture(
            state,
            &texture_bundle,
            wgpu::PrimitiveTopology::TriangleList,
            true,
            None,
        );

        Self {
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
            texture_bundle,
            texture_bind_group,
            save_frame: false,
        }
    }

    fn event(&mut self, state: &State, _event: &winit::event::WindowEvent) {}

    fn update(&mut self, state: &State, frame_count: u64, delta_time: f64) {


        if self.save_frame {
            self.save_frame = false;
            state.save_window_surface_to_file("window.png");
            // framework::utils::save_texture_to_file(
            //     &state.device,
            //     &state.queue,
            //     &self.texture_bundle.view,
            //     "ktx_texture_output.png",
            // );
        }
    }

    fn on_gui(&mut self, egui_ctx: &mut framework::EguiLayer) {
        egui::Window::new("Settings").show(&egui_ctx.ctx, |ui| {
            if ui.button("Save texture").clicked() {
                println!("Saving texture...");
                self.save_frame = true;
            }
        });
    }

    fn render<'rpass>(&'rpass self, state: &State, render_pass: &mut wgpu::RenderPass<'rpass>) {
        let ortho_perspective_matrix = glam::Mat4::IDENTITY;
        pipelines::write_global_uniform_buffer(
            ortho_perspective_matrix,
            self.pipeline.global_uniform_buffer.as_ref().unwrap(),
            &state.queue,
        );

        let matrices = [ModelUniform {
            model_matrix: glam::Mat4::IDENTITY,
        }];

        pipelines::write_uniform_buffer(
            &matrices,
            &self.pipeline.model_uniform_buffer.as_ref().unwrap(),
            &state.queue,
            &state.device,
        );

        render_pass.set_bind_group(0, &self.pipeline.bind_group, &[0, 0 as u32]);
        render_pass.set_bind_group(1, &self.texture_bind_group, &[]);

        render_pass.set_pipeline(&self.pipeline.pipeline);
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_vertex_buffer(0, self.buffer.slice(..));
        render_pass.draw_indexed(0..6, 0, 0..1);
    }
}

fn main() {
    framework::run::<KtxExample>(
        "simple_app",
        PhysicalSize {
            width: 1920,
            height: 1080,
        },
        4,
    );
}
