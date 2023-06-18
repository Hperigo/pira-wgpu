use std::ops::Mul;

use glam::{Mat4, Vec3, Vec4, Vec4Swizzles};

use crate::wgpu_helper;

pub trait CameraTrait {
    fn get_perspective_matrix(&self) -> glam::Mat4;
    fn get_view_matrix(&self) -> glam::Mat4;
    fn look_at(&mut self, target: glam::Vec3);
}

#[derive(Debug)]
pub struct PespectiveCamera {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,

    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

impl PespectiveCamera {
    pub fn new(fov: f32, aspect_ratio: f32, near: f32, far: f32) -> Self {
        Self {
            position: glam::Vec3::ZERO,
            rotation: glam::Quat::from_euler(glam::EulerRot::XYZ, 0.0, 0.0, 0.0),
            fov,
            aspect_ratio,
            near,
            far,
        }
    }
}

impl CameraTrait for PespectiveCamera {
    fn get_view_matrix(&self) -> glam::Mat4 {
        let m = glam::Mat4::from_rotation_translation(self.rotation, self.position);
        m.inverse()
    }

    fn get_perspective_matrix(&self) -> glam::Mat4 {
        glam::Mat4::perspective_lh(self.fov, self.aspect_ratio, self.near, self.far)
    }

    fn look_at(&mut self, target: glam::Vec3) {
        let mat = glam::Mat4::look_at_lh(self.position, target, glam::Vec3::Y);
        let rotation = glam::Quat::from_mat4(&mat).inverse();
        self.rotation = rotation;
    }
}

#[derive(Debug)]
pub struct OrbitControls {
    // mouse state
    is_left_mouse_dragging: bool,
    is_middle_mouse_dragging: bool,
    last_mouse_position: Option<glam::Vec2>,

    // delta_mouse_position: glam::Vec2, /d
    lat: f32,
    long: f32,
    zoom: f32,

    sensitivity: f32,

    orbit_local_position: glam::Vec3,
    target_world_position: glam::Vec3,

    pub camera: PespectiveCamera,
}

impl OrbitControls {
    pub fn new(aspect_ratio: f32) -> Self {
        Self {
            is_left_mouse_dragging: false,
            is_middle_mouse_dragging: false,
            last_mouse_position: None,
            // delta_mouse_position: glam::Vec2::ZERO,
            lat: 45.0,
            long: 45.0,
            zoom: 50.0,

            sensitivity: 0.1,

            orbit_local_position: glam::Vec3::ONE.mul(4.0),
            target_world_position: glam::Vec3::ZERO,

            camera: self::PespectiveCamera::new(45.0, aspect_ratio, 0.01, 1000.0),
        }
    }

    pub fn handle_events(&mut self, state: &wgpu_helper::State, event: &winit::event::WindowEvent) {
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
            match delta {
                event::MouseScrollDelta::LineDelta(_x, y) => {
                    self.handle_zoom(*y);
                }
                _ => (),
            }
        }

        if let event::WindowEvent::CursorMoved { position, .. } = event {
            self.handle_mouse_move(state, glam::vec2(position.x as f32, position.y as f32));

            // self.mouse_input(
            //     *position,
            //     [
            //         // app.input_state.window_size.0 as f32,
            //         // app.input_state.window_size.1 as f32,
            //     ],
            // );
        }
    }

    fn handle_mouse_press(&mut self, value: bool, middle_mouse: bool) {
        self.is_left_mouse_dragging = value;
        self.is_middle_mouse_dragging = middle_mouse;

        if value == true || middle_mouse == true {
            self.last_mouse_position = None;
        }
    }

    fn handle_zoom(&mut self, value: f32) {
        self.zoom += value;
        self.zoom = self.zoom.max(std::f32::EPSILON);
    }

    // on mouse click or touch drag
    fn handle_mouse_move(&mut self, state: &wgpu_helper::State, position: glam::Vec2) {
        match self.last_mouse_position {
            None => self.last_mouse_position = Some(position), // if last mouse position is None, we need to skip this logic and set the position
            Some(last_mouse_position) => {
                if self.is_left_mouse_dragging {
                    let delta = (position - last_mouse_position) * self.sensitivity;

                    self.lat += delta.y;
                    self.lat = self.lat.clamp(-85.0, 85.0);

                    self.long += delta.x;

                    if self.long < 0.0 {
                        self.long += 360.0;
                    }

                    if self.long > 360.0 {
                        self.long -= 360.0;
                    }
                }

                if self.is_middle_mouse_dragging {
                    let delta = position - last_mouse_position; // * self.sensitivity;
                    let length = delta.length();

                    if length < 0.01 {
                        return;
                    }
                    let aspect_ratio = state.window_size[0] / state.window_size[1];
                    let delta_normalized =
                        delta / glam::vec2(state.window_size[0], state.window_size[1]);

                    let ctd = self.zoom * 1.3; // not sure how but this number helps the camera lock on target.
                    let image_plane_height = 2.0 * ctd * (self.camera.fov.to_radians() * 0.5).tan();
                    let image_plane_width = image_plane_height * aspect_ratio;

                    let mut x_axis: Vec3 = (self.get_pan_matrix() * Vec4::X).xyz();
                    let mut y_axis: Vec3 = (self.get_pan_matrix() * Vec4::Y).xyz();

                    x_axis *= delta_normalized.x * image_plane_width;
                    y_axis *= delta_normalized.y * image_plane_height;

                    self.target_world_position += x_axis + y_axis;
                }

                self.last_mouse_position = Some(position);
            }
        }
    }

    fn update_local_pos(&mut self) {
        let lat_r = self.lat.to_radians();
        let long_r = self.long.to_radians();

        let pos = glam::vec3(
            lat_r.cos() * long_r.sin(),
            lat_r.sin(),
            lat_r.cos() * long_r.cos(),
        ) * self.zoom;

        self.orbit_local_position = pos;
    }

    pub fn get_target_position(&self) -> Vec3 {
        self.target_world_position
    }

    pub fn get_pan_matrix(&self) -> Mat4 {
        glam::Mat4::look_to_lh(glam::Vec3::ZERO, self.orbit_local_position, glam::Vec3::Y).inverse()
    }

    pub fn get_local_position(&self) -> Vec3 {
        self.orbit_local_position
    }

    pub fn update(&mut self) {
        self.update_local_pos();
    }

    pub fn get_model_matrix(&self) -> glam::Mat4 {
        glam::Mat4::look_at_lh(
            self.orbit_local_position + self.target_world_position,
            self.target_world_position,
            glam::Vec3::Y,
        )
        .inverse()
    }

    pub fn get_view_matrix(&self) -> glam::Mat4 {
        glam::Mat4::look_at_lh(
            self.orbit_local_position + self.target_world_position,
            self.target_world_position,
            glam::Vec3::Y,
        )
    }

    pub fn get_perspective_view_matrix(&self) -> glam::Mat4 {
        self.camera.get_perspective_matrix() * self.get_view_matrix()
    }
}
