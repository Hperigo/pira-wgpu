use wgpu::PrimitiveTopology;
use wgpu_app_lib::{
    cameras::{self, CameraTrait, OrbitControls},
    framework::{self, Application},
    geometry::{axis, GeometryFactory},
    pipelines::{self, shadeless::GpuMesh, shadeless::ShadelessPipeline},
};
use winit::dpi::PhysicalSize;

struct MyExample {
    mesh: GpuMesh,
    pipeline_batch: ShadelessPipeline,
    // camera: cameras::PespectiveCamera,
    orbit_control: cameras::OrbitControls,
}

impl Application for MyExample {
    fn init(state: &wgpu_app_lib::wgpu_helper::State) -> Self {
        let mut axis_geo = axis::Axis::new(10.0); //sphere::Sphere::new(10.0, 32, 16);
        axis_geo.texture_coords();
        axis_geo.vertex_colors();

        let pipeline_batch = ShadelessPipeline::new_with_texture(
            state,
            &state.default_white_texture_bundle,
            PrimitiveTopology::LineStrip,
            true,
        );

        let mesh = ShadelessPipeline::get_buffers_from_geometry(state, &axis_geo.geometry);

        let size = state.window_size;
        let mut camera = cameras::PespectiveCamera::new(45.0, size[0] / size[1], 0.001, 10000.0);
        camera.position = glam::vec3(100.0, 70.0, -100.0);
        camera.look_at(glam::Vec3::ZERO);

        Self {
            mesh,
            pipeline_batch,
            orbit_control: OrbitControls::new(size[0] / size[1]),
        }
    }

    fn clear_color(&self) -> wgpu::Color {
        wgpu::Color {
            r: 0.3,
            g: 0.2,
            b: 0.24,
            a: 1.0,
        }
    }
    fn event(
        &mut self,
        state: &wgpu_app_lib::wgpu_helper::State,
        event: &winit::event::WindowEvent,
    ) {
        self.orbit_control.handle_events(state, event);
    }

    fn update(
        &mut self,
        _state: &wgpu_app_lib::wgpu_helper::State,
        _ui: &mut imgui::Ui,
        _frame_count: u64,
        _delta_time: f64,
    ) {
        _ui.show_about_window(&mut true);

        self.orbit_control.update();
    }

    fn render<'rpass>(
        &'rpass self,
        state: &wgpu_app_lib::wgpu_helper::State,
        render_pass: &mut wgpu::RenderPass<'rpass>,
    ) {
        render_pass.set_pipeline(&self.pipeline_batch.pipeline);

        // SET up mesh self -----
        render_pass.set_index_buffer(self.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_vertex_buffer(0, self.mesh.vertex_buffer.slice(..));

        let mut matrices = Vec::new();
        for i in 0..10 {
            let offset = (i as f32 * 10.0) - 0.0;

            let mut m = glam::Mat4::from_translation([0.0, 0.0, offset].into());

            let s = 1.0; // ((i as f32 / 10.0) + _app.frame_number as f32 * 0.01).sin();
            m = m * glam::Mat4::from_scale([s, s, s].into());
            let uniform = pipelines::ModelUniform::new(m);

            matrices.push(uniform);
        }

        //target
        matrices[0].model_matrix =
            glam::Mat4::from_translation(self.orbit_control.get_target_position())
                * self.orbit_control.get_pan_matrix();

        matrices[1].model_matrix = self.orbit_control.get_model_matrix();

        pipelines::write_global_uniform_buffer(
            self.orbit_control.get_perspective_view_matrix(),
            self.pipeline_batch.global_uniform_buffer.as_ref().unwrap(),
            &state.queue,
        );

        pipelines::write_uniform_buffer(
            &matrices,
            self.pipeline_batch.model_uniform_buffer.as_ref().unwrap(),
            &state.queue,
            &state.device,
        );

        render_pass.set_bind_group(
            1,
            &self.pipeline_batch.texture_bind_group.as_ref().unwrap(),
            &[],
        );

        let uniform_alignment = state.device.limits().min_uniform_buffer_offset_alignment as usize;
        for i in 0..matrices.len() {
            render_pass.set_bind_group(
                0,
                &self.pipeline_batch.bind_group,
                &[0, uniform_alignment as u32 * i as u32],
            );
            render_pass.draw_indexed(0..self.mesh.vertex_count, 0, 0..1);
        }
    }
}

fn main() {
    framework::run::<MyExample>(
        "camera_controller",
        PhysicalSize {
            width: 1920 * 2,
            height: 1080 * 2,
        },
        4,
    );
}
