use wgpu::PrimitiveTopology;
use wgpu_app_lib::{
    cameras::{self, CameraTrait, OrbitControls},
    framework::{self, Application},
    geometry::{axis, GeometryFactory},
    pipelines::{self, shadeless::GpuMesh, shadeless::ShadelessPipeline, ModelUniform},
    wgpu_helper::factories,
};
use winit::dpi::PhysicalSize;

struct MyExample {
    mesh: GpuMesh,
    pipeline_batch: ShadelessPipeline,

    global_uniform_buffer: wgpu::Buffer,
    model_uniform_buffer: wgpu::Buffer,

    camera: cameras::PespectiveCamera,
    orbit_control: cameras::OrbitControls,
}

impl Application for MyExample {
    fn init(state: &wgpu_app_lib::wgpu_helper::State) -> Self {
        let mut axis_geo = axis::Axis::new(10.0); //sphere::Sphere::new(10.0, 32, 16);
        axis_geo.texture_coords();
        axis_geo.vertex_colors();

        // let (buffer, index_buffer) = cube.geometry.get_vertex_index_buffer(&state);

        let global_uniform_buffer = pipelines::create_global_uniform(&state.device);

        let camera_matrix = glam::Mat4::IDENTITY;
        pipelines::write_global_uniform_buffer(camera_matrix, &global_uniform_buffer, &state.queue);

        let model_matrix = glam::Mat4::IDENTITY;
        let matrices = [
            pipelines::ModelUniform::new(model_matrix),
            pipelines::ModelUniform::new(model_matrix),
            pipelines::ModelUniform::new(model_matrix),
        ];

        let model_uniform_buffer =
            pipelines::create_uniform_buffer::<ModelUniform>(matrices.len(), &state.device);

        let tf = factories::Texture2dFactory::new(2, 2);
        let data: [u8; 16] = [
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        ];
        let (_texture, view, sampler) =
            tf.get_texture_and_sampler(&state.device, &state.queue, &data);

        let pipeline_batch = ShadelessPipeline::new_with_texture(
            state,
            &global_uniform_buffer,
            &model_uniform_buffer,
            (wgpu::ShaderStages::FRAGMENT, &sampler, &view),
            PrimitiveTopology::LineStrip,
        );

        let mesh = ShadelessPipeline::get_buffers_from_geometry(state, &axis_geo.geometry);

        let size = [1920.0, 1080.0];
        let mut camera = cameras::PespectiveCamera::new(45.0, size[0] / size[1], 0.001, 10000.0);
        camera.position = glam::vec3(100.0, 70.0, -100.0);
        camera.look_at(glam::Vec3::ZERO);

        Self {
            camera,
            model_uniform_buffer,
            global_uniform_buffer,
            mesh,
            pipeline_batch,
            orbit_control: OrbitControls::new(),
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
        _state: &wgpu_app_lib::wgpu_helper::State,
        event: &winit::event::WindowEvent,
    ) {
        self.orbit_control.handle_events(event);
    }

    fn update(
        &mut self,
        _state: &wgpu_app_lib::wgpu_helper::State,
        _ui: &mut imgui::Ui,
        _frame_count: u64,
        _delta_time: f64,
    ) {
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
            self.camera.get_perspective_matrix() * self.orbit_control.get_view_matrix(),
            &self.global_uniform_buffer,
            &state.queue,
        );

        pipelines::write_uniform_buffer(
            &matrices,
            &self.model_uniform_buffer,
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

        println!("Rendering");
    }
}

fn main() {
    framework::run::<MyExample>(
        "camera_controller",
        PhysicalSize {
            width: 1920 * 2,
            height: 1080 * 2,
        },
    );
}
