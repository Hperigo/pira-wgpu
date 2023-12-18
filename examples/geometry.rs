use pira_wgpu::{
    framework::{self, Application},
    helpers::cameras::OrbitControls,
    helpers::geometry::{cube, sphere, GeometryFactory},
    pipelines::{self, shadeless, ModelUniform},
    state::State,
};
use winit::dpi::PhysicalSize;

struct Object {
    _name: &'static str,
    mesh: pipelines::shadeless::GpuMesh,
    position: glam::Vec3,
    rotation: glam::Quat,
    scale: glam::Vec3,
}

struct MyExample {
    batch: pira_wgpu::pipelines::shadeless::ShadelessPipeline,
    wire_pipeline: pira_wgpu::pipelines::shadeless::ShadelessPipeline,
    objects: Vec<Object>,
    orbit_controls: OrbitControls,
}

impl Application for MyExample {
    fn init(state: &State) -> Self {
        let batch = pipelines::shadeless::ShadelessPipeline::new_with_texture(
            state,
            &state.default_white_texture_bundle,
            wgpu::PrimitiveTopology::TriangleList,
            true,
        );

        let mut cube = cube::Cube::new(5.0);
        cube.texture_coords();
        let mesh = shadeless::ShadelessPipeline::get_buffers_from_geometry(state, &cube.geometry);

        let mut sphere = sphere::Sphere::new(5.0, 16, 32);
        sphere.texture_coords();
        sphere.normals();
        sphere.vertex_colors_from_normal();
        let axis_mesh =
            shadeless::ShadelessPipeline::get_buffers_from_geometry(state, &sphere.geometry);

        let wire_pipeline = pipelines::shadeless::ShadelessPipeline::new_with_texture(
            state,
            &state.default_white_texture_bundle,
            wgpu::PrimitiveTopology::LineList,
            true,
        );

        Self {
            batch,
            wire_pipeline,
            objects: vec![
                Object {
                    _name: "Cube",
                    mesh,
                    position: glam::Vec3::ZERO,
                    rotation: glam::Quat::IDENTITY,
                    scale: glam::Vec3::ONE,
                },
                Object {
                    _name: "Sphere",
                    mesh: axis_mesh,
                    position: glam::vec3(12.0, 0.0, 0.0),
                    rotation: glam::Quat::IDENTITY,
                    scale: glam::Vec3::ONE,
                },
            ],

            orbit_controls: OrbitControls::new(state.window_size.aspect_ratio()),
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

    fn event(&mut self, state: &State, event: &winit::event::WindowEvent) {
        self.orbit_controls.handle_events(state, event);
    }

    fn update(&mut self, _state: &State, _frame_count: u64, _delta_time: f64) {
        self.orbit_controls.update();
    }

    fn render<'rpass>(&'rpass self, state: &State, render_pass: &mut wgpu::RenderPass<'rpass>) {
        let shadeless::ShadelessPipeline { pipeline, .. } = &self.batch;

        render_pass.set_pipeline(&pipeline);

        let mut matrices = Vec::new();
        for i in 0..self.objects.len() {
            let obj = &self.objects[i];
            let m =
                glam::Mat4::from_scale_rotation_translation(obj.scale, obj.rotation, obj.position);
            matrices.push(ModelUniform { model_matrix: m });
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

        // WIREFRAME PASS ----
        matrices.clear();
        for i in 0..self.objects.len() {
            let obj = &self.objects[i];
            let m = glam::Mat4::from_scale_rotation_translation(
                obj.scale,
                obj.rotation,
                obj.position + glam::vec3(25.0, 0.0, 0.0),
            );
            matrices.push(ModelUniform { model_matrix: m });
        }

        pipelines::write_global_uniform_buffer(
            self.orbit_controls.get_perspective_view_matrix(),
            self.wire_pipeline.global_uniform_buffer.as_ref().unwrap(),
            &state.queue,
        );

        pipelines::write_uniform_buffer(
            &matrices,
            &self.wire_pipeline.model_uniform_buffer.as_ref().unwrap(),
            &state.queue,
            &state.device,
        );

        render_pass.set_pipeline(&self.wire_pipeline.pipeline);

        for i in 0..self.objects.len() {
            let obj = &self.objects[i];

            render_pass.set_bind_group(
                0,
                &self.wire_pipeline.bind_group,
                &[0, uniform_alignment as u32 * i as u32],
            );
            // render_pass.draw_indexed(0..self.mesh.vertex_count, 0,
            render_pass.set_vertex_buffer(0, obj.mesh.vertex_buffer.slice(..));
            render_pass
                .set_index_buffer(obj.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            render_pass.draw_indexed(0..obj.mesh.vertex_count, 0, 0..1);
        }

        // println!("Rendering");
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
