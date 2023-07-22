use wgpu_app_lib::{
    cameras::OrbitControls,
    framework::{self, Application},
    geometry::{self, GeometryFactory},
    pipelines::{self, shadeless, ModelUniform},
};
use winit::dpi::PhysicalSize;

struct Object {
    name: &'static str,
    mesh: pipelines::shadeless::GpuMesh,
    position: glam::Vec3,
    rotation: glam::Quat,
    scale: glam::Vec3,
}

struct MyExample {
    batch: wgpu_app_lib::pipelines::shadeless::ShadelessPipeline,
    wire_pipeline: wgpu_app_lib::pipelines::shadeless::ShadelessPipeline,
    objects: Vec<Object>,
    orbit_controls: OrbitControls,
}

impl Application for MyExample {
    fn init(state: &wgpu_app_lib::wgpu_helper::State) -> Self {
        let batch = pipelines::shadeless::ShadelessPipeline::new_with_texture(
            state,
            &state.default_white_texture_bundle,
            wgpu::PrimitiveTopology::TriangleList,
        );

        let mut cube = geometry::Cube::new(5.0);
        cube.texture_coords();
        let mesh = shadeless::ShadelessPipeline::get_buffers_from_geometry(state, &cube.geometry);

        let mut sphere = geometry::Sphere::new(5.0, 16, 32);
        sphere.texture_coords();
        sphere.normals();
        sphere.vertex_colors_from_normal();
        let axis_mesh =
            shadeless::ShadelessPipeline::get_buffers_from_geometry(state, &sphere.geometry);

        let wire_pipeline = pipelines::shadeless::ShadelessPipeline::new_with_texture(
            state,
            &state.default_white_texture_bundle,
            wgpu::PrimitiveTopology::LineList,
        );

        Self {
            batch,
            wire_pipeline,
            objects: vec![
                Object {
                    name: "Cube",
                    mesh,
                    position: glam::Vec3::ZERO,
                    rotation: glam::Quat::IDENTITY,
                    scale: glam::Vec3::ONE,
                },
                Object {
                    name: "Sphere",
                    mesh: axis_mesh,
                    position: glam::vec3(12.0, 0.0, 0.0),
                    rotation: glam::Quat::IDENTITY,
                    scale: glam::Vec3::ONE,
                },
            ],

            orbit_controls: OrbitControls::new(state.window_size[0] / state.window_size[1]),
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
        state: &wgpu_app_lib::wgpu_helper::State,
        event: &winit::event::WindowEvent,
    ) {
        self.orbit_controls.handle_events(state, event);
    }

    fn update(
        &mut self,
        _state: &wgpu_app_lib::wgpu_helper::State,
        ui: &mut imgui::Ui,
        _frame_count: u64,
        _delta_time: f64,
    ) {
        self.orbit_controls.update();

        ui.window("Objects")
            .size([400.0, 200.0], imgui::Condition::Always)
            .build(|| {
                for i in 0..self.objects.len() {
                    ui.spacing();

                    let obj = &mut self.objects[i];

                    ui.label_text(obj.name, "");

                    let _id = ui.push_id(format!("{}", i).as_str());
                    imgui::Drag::new("Position").build_array(ui, obj.position.as_mut());
                    imgui::Drag::new("Scale").build_array(ui, obj.scale.as_mut());

                    let euler_rot = obj.rotation.to_euler(glam::EulerRot::XYZ);
                    let mut euler_rot = [euler_rot.0, euler_rot.1, euler_rot.2];
                    if imgui::Drag::new("Rotation")
                        .speed(0.01)
                        .build_array(ui, &mut euler_rot)
                    {
                        obj.rotation = glam::Quat::from_euler(
                            glam::EulerRot::XYZ,
                            euler_rot[0],
                            euler_rot[1],
                            euler_rot[2],
                        );
                    }
                }
            });
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
