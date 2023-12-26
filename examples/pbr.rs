#![allow(dead_code)]
#![allow(unused_variables)]

use pira_wgpu::{
    framework::{self, Application},
    helpers::cameras::OrbitControls,
    helpers::geometry::{sphere, GeometryFactory},
    pipelines::{self, pbr},
    state::State,
};
use winit::dpi::PhysicalSize;

struct MyExample {
    pipeline: pira_wgpu::pipelines::pbr::PbrPipeline,
    mesh: pbr::GpuMesh,
    orbit_controls: OrbitControls,

    uniform: pipelines::pbr::PbrMaterialModelUniform,
}

impl Application for MyExample {
    fn init(state: &State) -> Self {
        let mut sphere = sphere::Sphere::new(5.0, 16, 32);
        sphere.texture_coords();
        sphere.normals();
        sphere.vertex_colors_from_normal();
        let sphere_mesh = pbr::PbrPipeline::get_buffers_from_geometry(state, &sphere.geometry);

        // ctx: &State,
        // global_uniform_buffer: &wgpu::Buffer,
        // model_uniform_buffer: &wgpu::Buffer,
        // texture: (wgpu::ShaderStages, &wgpu::Sampler, &wgpu::TextureView),
        // topology: PrimitiveTopology,

        let pipeline = pipelines::pbr::PbrPipeline::new_with_texture(
            state,
            &state.default_white_texture_bundle,
            wgpu::PrimitiveTopology::TriangleList,
            true,
        );

        Self {
            pipeline,
            mesh: sphere_mesh,
            orbit_controls: OrbitControls::new(state.window_size.aspect_ratio()),
            uniform: pipelines::pbr::PbrMaterialModelUniform::new(glam::Mat4::IDENTITY),
        }
    }

    fn event(&mut self, state: &State, event: &winit::event::WindowEvent) {
        self.orbit_controls.handle_events(state, event);
    }

    fn update(&mut self, state: &State, frame_count: u64, delta_time: f64) {
        self.orbit_controls.update();
    }

    fn on_gui(&mut self, egui_ctx: &mut framework::EguiLayer) {
        egui::Window::new("Settings").show(&egui_ctx.ctx, |ui| {
            ui.color_edit_button_rgb(self.uniform.albedo.as_mut());

            ui.add(
                egui::DragValue::new(&mut self.uniform.roughness)
                    .clamp_range(0.0..=1.0)
                    .speed(0.01),
            );
        });
    }

    fn clear_color(&self) -> wgpu::Color {
        // wgpu::Color {
        //     r: 0.3,
        //     g: 0.5,
        //     b: 0.1,
        //     a: 1.0,
        // };

        wgpu::Color::BLACK
    }

    fn render<'rpass>(&'rpass self, state: &State, render_pass: &mut wgpu::RenderPass<'rpass>) {
        let MyExample { pipeline, mesh, .. } = &self;

        render_pass.set_pipeline(&pipeline.pipeline);
        self.orbit_controls.get_perspective_view_matrix();

        let view_uniform = pipelines::ViewUniform {
            view_pespective_matrix: self.orbit_controls.get_perspective_view_matrix(),
            view_matrix: self.orbit_controls.get_view_matrix(),
            perspective_matrix: self.orbit_controls.get_perspective_matrix(),
            camera_position: self.orbit_controls.get_local_position(),
        };

        pipelines::write_uniform_buffer(
            &[view_uniform],
            self.pipeline.global_uniform_buffer.as_ref().unwrap(),
            &state.queue,
            &state.device,
        );

        let mut pbr_uniform = pipelines::pbr::PbrMaterialModelUniform::new(glam::Mat4::IDENTITY);
        pbr_uniform.albedo = glam::Vec3::ONE;
        pbr_uniform.roughness = 1.0;
        pbr_uniform.metallic = 1.0;

        pipelines::write_uniform_buffer(
            &[self.uniform],
            &pipeline.model_uniform_buffer.as_ref().unwrap(),
            &state.queue,
            &state.device,
        );

        render_pass.set_bind_group(0, &pipeline.bind_group, &[0, 0]);
        render_pass.set_bind_group(1, &pipeline.texture_bind_group.as_ref().unwrap(), &[]);

        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..mesh.vertex_count, 0, 0..1);
    }
}

fn main() {
    framework::run::<MyExample>(
        "simple_app",
        PhysicalSize {
            width: 1920 * 2,
            height: 1080 * 2,
        },
        4,
    );
}
