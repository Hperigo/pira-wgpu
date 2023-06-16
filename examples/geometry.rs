use wgpu_app_lib::{
    cameras::OrbitControls,
    framework::{self, Application},
    geometry::{self, GeometryFactory},
    pipelines::{self, shadeless, ModelUniform},
};
use winit::dpi::PhysicalSize;

struct Object {
    mesh: pipelines::shadeless::GpuMesh,
    model_uniform: pipelines::ModelUniform,
}

struct MyExample {
    batch: wgpu_app_lib::pipelines::shadeless::ShadelessPipeline,
    objects: Vec<Object>,
    orbit_controls: OrbitControls,
}

impl Application for MyExample {
    fn init(state: &wgpu_app_lib::wgpu_helper::State) -> Self {
        let batch = pipelines::shadeless::ShadelessPipeline::new_with_texture(
            state,
            (
                wgpu::ShaderStages::FRAGMENT,
                &state.default_white_texture_bundle.sampler,
                &state.default_white_texture_bundle.view,
            ),
            wgpu::PrimitiveTopology::TriangleList,
        );

        let size = [1920.0, 1080.0];

        let mut cube = geometry::Cube::new(5.0);
        cube.texture_coords();
        cube.vertex_colors();
        let mesh = shadeless::ShadelessPipeline::get_buffers_from_geometry(state, &cube.geometry);

        let mut sphere = geometry::Sphere::new(5.0, 16, 16);
        sphere.texture_coords();
        sphere.vertex_colors();
        let axis_mesh =
            shadeless::ShadelessPipeline::get_buffers_from_geometry(state, &sphere.geometry);

        Self {
            batch,
            objects: vec![
                Object {
                    mesh,
                    model_uniform: ModelUniform::new(glam::Mat4::IDENTITY),
                },
                Object {
                    mesh: axis_mesh,
                    model_uniform: ModelUniform::new(glam::Mat4::from_translation(glam::vec3(
                        12.0, 0.0, 0.0,
                    ))),
                },
            ],

            orbit_controls: OrbitControls::new(size[0] / size[1]),
        }
    }

    fn clear_color(&self) -> wgpu::Color {
        wgpu::Color {
            r: 0.7,
            g: 0.2,
            b: 0.2,
            a: 1.0,
        }
    }

    fn event(
        &mut self,
        _state: &wgpu_app_lib::wgpu_helper::State,
        event: &winit::event::WindowEvent,
    ) {
        self.orbit_controls.handle_events(event);
    }

    fn update(
        &mut self,
        _state: &wgpu_app_lib::wgpu_helper::State,
        _ui: &mut imgui::Ui,
        _frame_count: u64,
        _delta_time: f64,
    ) {
        self.orbit_controls.update();
    }

    fn render<'rpass>(
        &'rpass self,
        state: &wgpu_app_lib::wgpu_helper::State,
        render_pass: &mut wgpu::RenderPass<'rpass>,
    ) {
        let shadeless::ShadelessPipeline { pipeline, .. } = &self.batch;

        render_pass.set_pipeline(&pipeline);

        let mut matrices = Vec::new();
        for i in 0..self.objects.len() {
            matrices.push(self.objects[i].model_uniform);
        }

        pipelines::write_global_uniform_buffer(
            self.orbit_controls.get_perspective_view_matrix(),
            self.batch.global_uniform_buffer.as_ref().unwrap(),
            &state.queue,
        );

        pipelines::write_uniform_buffer(
            &matrices,
            &self.batch.model_uniform_buffer.as_ref().unwrap(),
            &state.queue,
            &state.device,
        );

        render_pass.set_bind_group(1, &self.batch.texture_bind_group.as_ref().unwrap(), &[]);

        let uniform_alignment = state.device.limits().min_uniform_buffer_offset_alignment as usize;
        for i in 0..self.objects.len() {
            let obj = &self.objects[i];

            render_pass.set_bind_group(
                0,
                &self.batch.bind_group,
                &[0, uniform_alignment as u32 * i as u32],
            );
            // render_pass.draw_indexed(0..self.mesh.vertex_count, 0,
            render_pass.set_vertex_buffer(0, obj.mesh.vertex_buffer.slice(..));
            render_pass
                .set_index_buffer(obj.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..obj.mesh.vertex_count, 0, 0..1);
        }

        println!("Rendering");
    }
}

fn main() {
    framework::run::<MyExample>(
        "simple_app",
        PhysicalSize {
            width: 1920 * 2,
            height: 1080 * 2,
        },
    );
}
