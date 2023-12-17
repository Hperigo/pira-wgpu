use wgpu;
use wgpu::util::DeviceExt;
use wgpu::RenderPass;
use wgpu_app_lib::factories;
use wgpu_app_lib::factories::texture::{SamplerOptions, Texture2dOptions};
use wgpu_app_lib::framework;
use wgpu_app_lib::framework::Application;
use wgpu_app_lib::pipelines::{self, shadeless, ModelUniform};
use wgpu_app_lib::state::State;

use winit::dpi::PhysicalSize;
use {glam::Mat4, glam::Vec4Swizzles};

use image::EncodableLayout;

#[derive(Debug)]
pub struct CameraController2D {
    // mouse state
    is_left_mouse_dragging: bool,
    is_middle_mouse_dragging: bool,
    last_mouse_position: Option<glam::Vec2>,

    // anchor_point: glam::Vec3,
    // anchor_point_scale: f32,
    position: glam::Vec3,
    scale: f32,
}

impl CameraController2D {
    pub fn new() -> Self {
        CameraController2D {
            is_left_mouse_dragging: false,
            is_middle_mouse_dragging: false,
            last_mouse_position: None,
            // anchor_point: glam::vec3(0.0, 0.0, 0.0),
            // anchor_point_scale: 1.0,
            position: glam::Vec3::ZERO,
            scale: 1.0,
        }
    }

    pub fn handle_events(&mut self, _state: &State, event: &winit::event::WindowEvent) {
        use winit::event;

        if let event::WindowEvent::MouseInput { state, button, .. } = event {
            if matches!(state, event::ElementState::Pressed) {
                self.handle_mouse_press(
                    *button == event::MouseButton::Left,
                    *button == event::MouseButton::Middle || *button == event::MouseButton::Right,
                );
            } else if matches!(state, event::ElementState::Released) {
                self.handle_mouse_press(false, false);
            }
        }

        if let event::WindowEvent::MouseWheel { delta, .. } = event {
            let v = self.last_mouse_position.unwrap();
            match delta {
                event::MouseScrollDelta::LineDelta(_x, y) => {
                    self.handle_zoom(*y, glam::vec3(v.x, v.y, 0.0));
                }
                event::MouseScrollDelta::PixelDelta(position) => {
                    self.handle_zoom(position.y as f32 * 0.5, glam::vec3(v.x, v.y, 0.0));
                }
            }
        }

        if let event::WindowEvent::CursorMoved { position, .. } = event {
            self.handle_mouse_move(glam::vec2(position.x as f32, position.y as f32));
        }
    }

    fn handle_mouse_press(&mut self, value: bool, middle_mouse: bool) {
        self.is_left_mouse_dragging = value;
        self.is_middle_mouse_dragging = middle_mouse;

        if value == true || middle_mouse == true {
            self.last_mouse_position = None;
        }
    }

    fn handle_zoom(&mut self, value: f32, pos: glam::Vec3) {
        let mut anchor_point_scale = 1.0 + value * 0.01;
        anchor_point_scale = anchor_point_scale.max(std::f32::EPSILON);

        self.set_anchor_point_scale(pos, anchor_point_scale);
    }

    // on mouse click or touch drag
    fn handle_mouse_move(&mut self, position: glam::Vec2) {
        match self.last_mouse_position {
            None => self.last_mouse_position = Some(position), // if last mouse position is None, we need to skip this logic and set the position
            Some(last_mouse_position) => {
                if self.is_left_mouse_dragging {
                    let delta = position - last_mouse_position; // * self.sensitivity;
                    self.position -= glam::vec3(delta.x, delta.y, 0.0) * self.scale;
                }
                if self.is_middle_mouse_dragging {}

                self.last_mouse_position = Some(position);
            }
        }
    }

    pub fn set_anchor_point_scale(&mut self, anchor_point: glam::Vec3, scale: f32) {
        //transform the anchor point to world cordinate
        let world_mat = self.get_camera_matrix();
        let world_anchor_point =
            world_mat * glam::Vec4::new(anchor_point.x, anchor_point.y, anchor_point.z, 1.0);

        // create a matrix for the anchor point and multiply the by the inverse of the anchor point matrix so main point is a child of anchor point transofor
        let initial_anchor_point = glam::Mat4::from_scale_rotation_translation(
            glam::Vec3::ONE,
            glam::Quat::IDENTITY,
            world_anchor_point.xyz(),
        );
        let mut local_mat = initial_anchor_point.inverse() * world_mat;

        //create another one with the scale transform
        let scale_anchor_point = glam::Mat4::from_scale_rotation_translation(
            glam::Vec3::ONE * scale,
            glam::Quat::IDENTITY,
            world_anchor_point.xyz(),
        );
        local_mat = scale_anchor_point * local_mat;

        let decomposed = local_mat.to_scale_rotation_translation();
        self.scale = decomposed.0.x;
        self.position = decomposed.2;
    }

    pub fn get_camera_matrix(&self) -> glam::Mat4 {
        glam::Mat4::from_scale_rotation_translation(
            glam::Vec3::ONE * self.scale,
            glam::Quat::IDENTITY,
            self.position,
        )
    }

    pub fn get_view_projection_matrix(&self, state: &State) -> glam::Mat4 {
        let window_size = state.window_size;
        let ortho_perspective_matrix = glam::Mat4::orthographic_lh(
            0.0,
            window_size.width_f32(),
            window_size.height_f32(),
            0.0,
            -1.0,
            1.0,
        );

        let view_matrix = self.get_camera_matrix();
        return ortho_perspective_matrix * view_matrix.inverse();
    }
}

struct MyExample {
    clear_color: [f32; 4],
    buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    pipeline: shadeless::ShadelessPipeline,

    position: glam::Vec3,
    camera_controller: CameraController2D,
}

impl Application for MyExample {
    fn init(state: &State) -> Self {
        let image = image::open("./assets/rusty.png").unwrap().to_rgba8();
        let rect_size: f32 = 500.0;

        let aspect_ratio = image.height() as f32 / image.width() as f32;

        let vertices = vec![
            shadeless::Vertex::new(
                [-rect_size, -rect_size * aspect_ratio, -0.1],
                [0.0, 0.0],
                [1.0, 0.0, 0.0, 1.0],
            ),
            shadeless::Vertex::new(
                [rect_size, rect_size * aspect_ratio, -0.1],
                [1.0, 1.0],
                [0.0, 1.0, 0.0, 1.0],
            ),
            shadeless::Vertex::new(
                [rect_size, -rect_size * aspect_ratio, -0.1],
                [1.0, 0.0],
                [0.0, 0.0, 1.0, 1.0],
            ),
            shadeless::Vertex::new(
                [-rect_size, rect_size * aspect_ratio, -0.1],
                [0.0, 1.0],
                [1.0, 1.0, 1.0, 1.0],
            ),
        ];

        let indices: [u16; 6] = [0, 1, 2, 0, 3, 1];
        // indices.reverse();

        let texture_bundle = factories::Texture2dFactory::new_with_options(
            &state,
            [image.width(), image.height()],
            Texture2dOptions {
                ..Default::default()
            },
            SamplerOptions {
                filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            },
            image.as_bytes(),
        );

        let pipeline = shadeless::ShadelessPipeline::new_with_texture(
            state,
            &texture_bundle,
            wgpu::PrimitiveTopology::TriangleList,
            true,
        );

        MyExample {
            position: glam::Vec3::ZERO, //glam::vec3(1000.0, 800.0, 0.0),
            camera_controller: CameraController2D::new(),

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

    fn event(&mut self, state: &State, event: &winit::event::WindowEvent) {
        self.camera_controller.handle_events(state, event);
    }

    fn render<'rpass>(&'rpass self, state: &State, render_pass: &mut RenderPass<'rpass>) {
        let mat = self.camera_controller.get_view_projection_matrix(state);

        pipelines::write_global_uniform_buffer(
            mat,
            self.pipeline.global_uniform_buffer.as_ref().unwrap(),
            &state.queue,
        );

        let matrices = [ModelUniform {
            model_matrix: Mat4::from_translation(self.position),
        }];

        pipelines::write_uniform_buffer(
            &matrices,
            &self.pipeline.model_uniform_buffer.as_ref().unwrap(),
            &state.queue,
            &state.device,
        );

        render_pass.set_bind_group(0, &self.pipeline.bind_group, &[0, 0 as u32]);
        render_pass.set_bind_group(1, self.pipeline.texture_bind_group.as_ref().unwrap(), &[]);
        render_pass.set_pipeline(&self.pipeline.pipeline);
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_vertex_buffer(0, self.buffer.slice(..));
        render_pass.draw_indexed(0..6, 0, 0..1);
    }
}

fn main() {
    let dpi = 2;

    framework::run::<MyExample>(
        "simple_app",
        PhysicalSize {
            width: 1920 * dpi,
            height: 1080 * dpi,
        },
        4,
    );
}
