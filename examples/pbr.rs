#![allow(dead_code)]
#![allow(unused_variables)]

use image::EncodableLayout;
use pira_wgpu::{
    factories::{
        self,
        texture::{SamplerOptions, Texture2dOptions},
    },
    framework::{self, Application},
    helpers::cameras::OrbitControls,
    helpers::geometry::{sphere, GeometryFactory},
    image,
    pipelines::{
        self, pbr,
        sky::{self, SkyRendererOptions},
    },
    state::State,
};
use wgpu::TextureFormat;
use winit::dpi::PhysicalSize;

struct MyExample {
    pipeline: pira_wgpu::pipelines::pbr::PbrPipeline,
    mesh: pbr::GpuMesh,
    orbit_controls: OrbitControls,

    uniform: pipelines::pbr::PbrMaterialModelUniform,

    sky_renderer: pipelines::sky::SkyRenderer,
}

impl Application for MyExample {
    fn init(state: &State) -> Self {
        let sphere_mesh = {
            puffin::profile_scope!("Creating Sphere geometry");
            let mut sphere = sphere::Sphere::new(5.0, 16, 32);
            sphere.texture_coords();
            sphere.normals();
            sphere.vertex_colors_from_normal();
            pbr::PbrPipeline::get_buffers_from_geometry(state, &sphere.geometry)
        };

        let base_path =
            std::path::Path::new("./assets/");

        let abs_path = std::fs::canonicalize(base_path).unwrap().to_str();

        println!("Base path: {}", std::fs::canonicalize(base_path).unwrap().to_str().unwrap_or_default());
        let roughness_bundle = {
            puffin::profile_scope!("Loading roughness map");

            let roughness_image = image::open(base_path.join("rustediron2_roughness.png"))
                .unwrap()
                .to_rgba8();

            factories::Texture2dFactory::new_with_options(
                &state,
                [roughness_image.width(), roughness_image.height()],
                Texture2dOptions {
                    label: Some("RoughnessTexture"),
                    format: TextureFormat::Rgba8UnormSrgb,
                    ..Default::default()
                },
                SamplerOptions {
                    filter: wgpu::FilterMode::Nearest,
                    ..Default::default()
                },
                roughness_image.as_bytes(),
            )
        };

        let albedo_image = image::open(base_path.join("rustediron2_basecolor.png"))
            .unwrap()
            .to_rgba8();

        let albedo_bundle = factories::Texture2dFactory::new_with_options(
            &state,
            [albedo_image.width(), albedo_image.height()],
            Texture2dOptions {
                label: Some("Albedo Texture"),
                format: TextureFormat::Rgba8UnormSrgb,
                ..Default::default()
            },
            SamplerOptions {
                filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            },
            albedo_image.as_bytes(),
        );

        let metallic_image = image::open(base_path.join("rustediron2_metallic.png"))
            .unwrap()
            .to_rgba8();

        let metallic_bundle = factories::Texture2dFactory::new_with_options(
            &state,
            [metallic_image.width(), albedo_image.height()],
            Texture2dOptions {
                label: Some("metallic Texture"),
                format: TextureFormat::Rgba8Unorm,
                ..Default::default()
            },
            SamplerOptions {
                filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            },
            metallic_image.as_bytes(),
        );

        let image = {
            puffin::profile_scope!("Loading HDR");
            image::open(
                base_path.join("buikslotermeerplein_1k.exr"),
                // "/Users/henrique/Documents/dev/rust/pira-wgpu/assets/cubemap-equi.png",
            )
            .unwrap()
        };

        let sky_renderer = sky::SkyRenderer::new(
            state,
            &image,
            SkyRendererOptions {
                dst_size: 512,
                ..Default::default()
            },
        );

        let pipeline = pipelines::pbr::PbrPipeline::new_with_texture(
            state,
            &roughness_bundle,
            &albedo_bundle,
            &metallic_bundle,
            &sky_renderer,
            wgpu::PrimitiveTopology::TriangleList,
            true,
        );

        let mut uniform = pipelines::pbr::PbrMaterialModelUniform::new(glam::Mat4::IDENTITY);
        uniform.ambient = glam::vec3(0.4, 0.4, 0.4);
        uniform.light_intensity = 5.0;

        Self {
            pipeline,
            mesh: sphere_mesh,
            orbit_controls: OrbitControls::new(state.window_size.aspect_ratio()),
            uniform,

            sky_renderer,
        }
    }

    fn event(&mut self, state: &State, event: &winit::event::WindowEvent) {
        self.orbit_controls.handle_events(state, event);
    }

    fn update(&mut self, state: &State, frame_count: u64, delta_time: f64) {
        let State { device, queue, .. } = state;

        self.orbit_controls.update();

        let view_pos = glam::Vec4::from((self.orbit_controls.get_local_position(), 1.0));
        let view_mat = self.orbit_controls.get_view_matrix();
        let view_proj_mat = self.orbit_controls.get_perspective_view_matrix(); // self.orbit_controls.get_view_matrix();

        let uniform: sky::Uniform = sky::Uniform {
            view_pos: view_pos.to_array(),
            view: view_mat.to_cols_array(),
            view_proj: view_proj_mat.to_cols_array(),
            inv_proj: self
                .orbit_controls
                .get_perspective_matrix()
                .inverse()
                .to_cols_array(),
            inv_view: view_mat.inverse().to_cols_array(),
        };

        pipelines::write_uniform_buffer(
            &[uniform],
            &self.sky_renderer.uniform_buffer,
            queue,
            device,
        );
    }

    fn on_gui(&mut self, egui_ctx: &mut framework::EguiLayer) {
        egui::Window::new("Settings").show(&egui_ctx.ctx, |ui| {
            ui.color_edit_button_rgb(self.uniform.albedo.as_mut());

            ui.label("Ambient");

            ui.color_edit_button_rgb(self.uniform.ambient.as_mut());

            ui.label("Light Position");

            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(&mut self.uniform.light_position.x)
                        .prefix("x: ")
                        .speed(0.01),
                );
                ui.add(
                    egui::DragValue::new(&mut self.uniform.light_position.y)
                        .prefix("y: ")
                        .speed(0.01),
                );
                ui.add(
                    egui::DragValue::new(&mut self.uniform.light_position.z)
                        .prefix("z: ")
                        .speed(0.01),
                );
            });

            ui.label("Light");
            ui.add(
                egui::DragValue::new(&mut self.uniform.light_intensity)
                    .clamp_range(0.0..=20.0)
                    .speed(0.01),
            );

            ui.label("Roughness");
            ui.add(
                egui::DragValue::new(&mut self.uniform.roughness)
                    .clamp_range(0.0..=1.0)
                    .speed(0.01),
            );

            ui.label("Metallic");
            ui.add(
                egui::DragValue::new(&mut self.uniform.metallic)
                    .clamp_range(0.0..=1.0)
                    .speed(0.01),
            );
        });
    }

    fn clear_color(&self) -> wgpu::Color {
        wgpu::Color {
            r: self.uniform.ambient.x as f64,
            g: self.uniform.ambient.y as f64,
            b: self.uniform.ambient.z as f64,
            a: 1.0,
        }
    }

    fn render<'rpass>(&'rpass self, state: &State, render_pass: &mut wgpu::RenderPass<'rpass>) {
        let MyExample {
            pipeline,
            mesh,
            sky_renderer,
            ..
        } = &self;

        sky_renderer.draw(render_pass);

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

        pipelines::write_uniform_buffer(
            &[self.uniform],
            &pipeline.model_uniform_buffer.as_ref().unwrap(),
            &state.queue,
            &state.device,
        );

        render_pass.set_bind_group(0, &pipeline.bind_group, &[0, 0]);
        render_pass.set_bind_group(1, pipeline.texture_bind_group.as_ref().unwrap(), &[]);

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
