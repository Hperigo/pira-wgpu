use std::borrow::Cow;

use glam::Mat4;
use pira_wgpu::factories::texture::{SamplerOptions, Texture2dOptions, TextureBundle};
use pira_wgpu::factories::BindGroupFactory;
use wgpu::util::DeviceExt;
use wgpu::{self, BindGroup, ComputePipelineDescriptor, ShaderModuleDescriptor};

use pira_wgpu::framework::Application;
use pira_wgpu::pipelines::{self, shadeless, ModelUniform};
use pira_wgpu::state::State;
use pira_wgpu::{factories, framework};
use wgpu::RenderPass;

use winit::dpi::PhysicalSize;
use winit::event::ElementState;
use winit::keyboard::KeyCode;

struct ComputeExample {
    clear_color: [f32; 4],
    buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    pipeline: shadeless::ShadelessPipeline,

    _texture_bundle: TextureBundle,
}

impl Application for ComputeExample {
    fn init(state: &State) -> Self {
        let compute_shader = state.device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Compute shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shaders/compute.wgsl"))),
        });

        let compute_pipeline = state
            .device
            .create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some("Compute pipeline"),
                layout: None,
                entry_point: Some("main"),
                module: &compute_shader,
                compilation_options : wgpu::PipelineCompilationOptions::default(),
                cache : None,
            });

        let output_texture = factories::Texture2dFactory::new_with_options(
            &state,
            [500, 500],
            Texture2dOptions {
                mip_level_count: 1,
                sample_count: 1,
                format: wgpu::TextureFormat::Rgba32Float,
                usage: wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::STORAGE_BINDING,
                label: Some("Output texture"),
            },
            SamplerOptions {
                address_mode: wgpu::AddressMode::Repeat,
                filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
            },
            &[],
        );

        let bind_group_layout = compute_pipeline.get_bind_group_layout(0);
        let bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&output_texture.view),
            }],
        });

        let mut encoder = state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
            cpass.set_pipeline(&compute_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.insert_debug_marker("compute collatz iterations");
            cpass.dispatch_workgroups(500, 500, 1); // Number of cells to run, the (x,y,z) size of item being processed
        }
        state.queue.on_submitted_work_done(|| {
            println!("Done!");
        });
        let idx = state.queue.submit([encoder.finish()]);

        loop {
            let result = state
                .device
                .poll(wgpu::PollType::Wait { submission_index: Some(idx.clone()), timeout: None });

            if result.is_ok() {
                break;
            }
        }

        println!("Compute pass complete!");

        // ------------ DRAW PIPELINE ------------
        let vertices = vec![
            shadeless::Vertex::new([-0.8, -0.8, 0.0], [0.0, 0.0], [1.0, 1.0, 1.0, 1.0]),
            shadeless::Vertex::new([0.8, 0.8, 0.0], [1.0, 1.0], [1.0, 1.0, 1.0, 1.0]),
            shadeless::Vertex::new([0.8, -0.8, 0.0], [1.0, 0.0], [1.0, 1.0, 1.0, 1.0]),
            shadeless::Vertex::new([-0.8, 0.8, 0.0], [0.0, 1.0], [1.0, 1.0, 1.0, 1.0]),
        ];

        let mut indices: [u16; 6] = [0, 1, 2, 0, 3, 1];
        indices.reverse();



        let mut texture_bind_group_factory = BindGroupFactory::new();
        texture_bind_group_factory.set_labels("Compute texture layout label", "Compute texture bind group label");
        texture_bind_group_factory.add_texture_hdr_and_sampler(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            &output_texture.view,
            &output_texture.sampler,
            wgpu::SamplerBindingType::NonFiltering,
        );
        let (draw_bind_group_layout, draw_bind_group) =
            texture_bind_group_factory.build(&state.device);

        let mut pipeline = shadeless::ShadelessPipeline::new_with_texture(
            state,
            &output_texture,
            wgpu::PrimitiveTopology::TriangleList,
            true,
            Some( (draw_bind_group_layout, draw_bind_group))
        );

        //pipeline.set_texture_bind_group(draw_bind_group.clone(), _draw_bind_group_layout);


        ComputeExample {
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
            // bind_group: draw_bind_group,
            _texture_bundle: output_texture,
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

    fn update(&mut self, _state: &State, _frame_count: u64, _delta_time: f64) {}

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
                    ElementState::Released => {}
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
        render_pass.set_bind_group(1, &self.pipeline.texture_bind_group, &[]);
        render_pass.set_pipeline(&self.pipeline.pipeline);
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_vertex_buffer(0, self.buffer.slice(..));
        render_pass.draw_indexed(0..6, 0, 0..1);
    }
}

fn main() {
    let dpi = 2;

    framework::run::<ComputeExample>(
        "simple_app",
        PhysicalSize {
            width: 460 * dpi,
            height: 307 * dpi,
        },
        4,
    );
}
