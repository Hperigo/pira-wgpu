#![allow(dead_code)]
#![allow(unused_variables)]

use glam::Vec3;
use image;

use pira_wgpu::framework::{self, Application};

use pira_wgpu::pipelines;
use pira_wgpu::pipelines::sky::{self, SkyRenderer, SkyRendererOptions};

use pira_wgpu::state::State;
use winit::dpi::PhysicalSize;

const TEXTURE_DIMS: (usize, usize) = (512, 512);

struct MyExample {
    rotation: glam::Vec3,
    exposure: f32,

    sky_renderer: SkyRenderer,
}

impl Application for MyExample {
    fn init(state: &State) -> Self {
        let State {
            instance,
            adapter,
            device,
            queue,
            config,
            window_surface,
            depth_texture,
            default_white_texture_bundle,
            window_size,
            delta_time,
            sample_count,
        } = state;

        let image = image::open(
            "/Users/henrique/Documents/dev/rust/pira-wgpu/assets/buikslotermeerplein_1k.exr",
            // "/Users/henrique/Documents/dev/rust/pira-wgpu/assets/cubemap-equi.png",
        )
        .unwrap();

        //-----------------------------------------------
        // let vertices = vec![
        //     shadeless::Vertex::new([-1.0, -1.0, 0.0], [0.0, 0.0], [1.0, 1.0, 1.0, 1.0]),
        //     shadeless::Vertex::new([1.0, 1.0, 0.0], [1.0, 1.0], [1.0, 1.0, 1.0, 1.0]),
        //     shadeless::Vertex::new([1.0, -1.0, 0.0], [1.0, 0.0], [1.0, 1.0, 1.0, 1.0]),
        //     shadeless::Vertex::new([-1.0, 1.0, 0.0], [0.0, 1.0], [1.0, 1.0, 1.0, 1.0]),
        // ];

        // let indices: [u16; 6] = [0, 1, 2, 0, 3, 1];

        // let uniform_buffer = pipelines::create_uniform_buffer::<Uniform>(1, device);

        // let environment_layout =
        //     device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        //         label: Some("environment_layout"),
        //         entries: &[
        //             wgpu::BindGroupLayoutEntry {
        //                 binding: 0,
        //                 visibility: wgpu::ShaderStages::FRAGMENT,
        //                 ty: wgpu::BindingType::Texture {
        //                     sample_type: wgpu::TextureSampleType::Float { filterable: false },
        //                     view_dimension: wgpu::TextureViewDimension::Cube,
        //                     multisampled: false,
        //                 },
        //                 count: None,
        //             },
        //             wgpu::BindGroupLayoutEntry {
        //                 binding: 1,
        //                 visibility: wgpu::ShaderStages::FRAGMENT,
        //                 ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
        //                 count: None,
        //             },
        //             wgpu::BindGroupLayoutEntry {
        //                 binding: 2,
        //                 visibility: wgpu::ShaderStages::FRAGMENT,
        //                 ty: wgpu::BindingType::Buffer {
        //                     ty: wgpu::BufferBindingType::Uniform,
        //                     has_dynamic_offset: false,
        //                     min_binding_size: None,
        //                 },
        //                 count: None,
        //             },
        //         ],
        //     });

        // let environment_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     label: Some("environment_bind_group"),
        //     layout: &environment_layout,
        //     entries: &[
        //         wgpu::BindGroupEntry {
        //             binding: 0,
        //             resource: wgpu::BindingResource::TextureView(&sky_texture_bundle.view),
        //         },
        //         wgpu::BindGroupEntry {
        //             binding: 1,
        //             resource: wgpu::BindingResource::Sampler(&device.create_sampler(
        //                 &SamplerDescriptor {
        //                     ..Default::default()
        //                 },
        //             )),
        //         },
        //         wgpu::BindGroupEntry {
        //             binding: 2,
        //             resource: wgpu::BindingResource::Buffer(BufferBinding {
        //                 buffer: &uniform_buffer,
        //                 offset: 0,
        //                 size: wgpu::BufferSize::new(std::mem::size_of::<Uniform>() as _),
        //             }),
        //         },
        //     ],
        // });

        // let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        //     label: Some("Sky Pipeline Layout"),
        //     bind_group_layouts: &[&environment_layout],
        //     push_constant_ranges: &[],
        // });

        // let shader = device.create_shader_module(ShaderModuleDescriptor {
        //     label: Some("Sky"),
        //     source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SHADER_SRC)),
        // });

        // let vertex_attribs = shadeless::ShadelessPipeline::get_vertex_attrib_layout_array();

        // let mut sky_render_pipeline = factories::RenderPipelineFactory::new();
        // // sky_render_pipeline.add_vertex_attributes(
        // //     &vertex_attribs,
        // //     shadeless::ShadelessPipeline::get_array_stride(),
        // // );
        // sky_render_pipeline.set_cull_mode(Some(wgpu::Face::Back));

        // let pipeline =
        //     sky_render_pipeline.create_render_pipeline(state, &shader, &[&environment_layout]);

        let sky_renderer = SkyRenderer::new(state, &image, SkyRendererOptions::default());
        Self {
            sky_renderer,
            rotation: Vec3::ZERO,
            exposure: 1.0,
        }
    }

    fn clear_color(&self) -> wgpu::Color {
        wgpu::Color::GREEN
    }

    fn event(&mut self, state: &State, _event: &winit::event::WindowEvent) {}

    fn update(&mut self, state: &State, frame_count: u64, delta_time: f64) {
        let State { device, queue, .. } = state;

        let rotation_matrix_buffer = glam::Mat4::from_euler(
            glam::EulerRot::XYZ,
            self.rotation.x,
            self.rotation.y,
            self.rotation.z,
        );

        let uniform: sky::Uniform = sky::Uniform {
            view_pos: [0.0, 0.0, 0.0, 0.0],
            view: *glam::Mat4::IDENTITY.as_ref(),
            view_proj: *glam::Mat4::IDENTITY.as_ref(),
            inv_proj: *glam::Mat4::IDENTITY.as_ref(),
            inv_view: *glam::Mat4::IDENTITY.as_ref(),
        };

        pipelines::write_uniform_buffer(
            &[uniform],
            &self.sky_renderer.uniform_buffer,
            queue,
            device,
        );

        // queue.write_buffer(
        //     &self.uniform_buffer,
        //     0,
        //     bytemuck::cast_slice(rotation_matrix_buffer.as_ref()),
        // );
    }

    fn on_gui(&mut self, egui_ctx: &mut framework::EguiLayer) {
        egui::SidePanel::new(egui::panel::Side::Left, "Debug").show(&egui_ctx.ctx, |ui| {
            ui.drag_angle(&mut self.rotation.x);
            ui.drag_angle(&mut self.rotation.y);
            ui.drag_angle(&mut self.rotation.z);

            ui.spacing();

            ui.add(egui::DragValue::new(&mut self.exposure).speed(0.01));
        });
    }

    fn render<'rpass>(&'rpass self, state: &State, render_pass: &mut wgpu::RenderPass<'rpass>) {
        render_pass.set_bind_group(0, &self.sky_renderer.bind_group, &[]);
        render_pass.set_pipeline(&self.sky_renderer.pipeline);
        render_pass.draw(0..3, 0..1);

        // render_pass.draw_indexed(0..6, 0, 0..1);

        // let ortho_perspective_matrix = glam::Mat4::IDENTITY;
        // pipelines::write_global_uniform_buffer(
        //     ortho_perspective_matrix,
        //     self.pipeline.global_uniform_buffer.as_ref().unwrap(),
        //     &state.queue,
        // );

        // let matrices = [ModelUniform {
        //     model_matrix: glam::Mat4::IDENTITY,
        // }];

        // pipelines::write_uniform_buffer(
        //     &matrices,
        //     &self.pipeline.model_uniform_buffer.as_ref().unwrap(),
        //     &state.queue,
        //     &state.device,
        // );

        // render_pass.set_bind_group(0, &self.pipeline.bind_group, &[0, 0 as u32]);
        // render_pass.set_bind_group(1, self.pipeline.texture_bind_group.as_ref().unwrap(), &[]);
        // render_pass.set_pipeline(&self.pipeline.pipeline);
        // render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        // render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        // render_pass.draw_indexed(0..6, 0, 0..1);
    }
}

fn main() {
    framework::run::<MyExample>(
        "framebuffer",
        PhysicalSize {
            width: 1000,
            height: 1000,
        },
        4,
    );
}
