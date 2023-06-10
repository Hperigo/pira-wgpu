#![allow(dead_code)]
#![allow(unused_variables)]

use wgpu_app_lib::framework::{self, Application};
use winit::dpi::PhysicalSize;

struct MyExample {}

impl Application for MyExample {
    fn init(state: &wgpu_app_lib::wgpu_helper::State) -> Self {
        Self {}
    }

    fn event(
        &mut self,
        state: &wgpu_app_lib::wgpu_helper::State,
        _event: &winit::event::WindowEvent,
    ) {
    }

    fn update(
        &mut self,
        state: &wgpu_app_lib::wgpu_helper::State,
        ui: &mut imgui::Ui,
        frame_count: u64,
        delta_time: f64,
    ) {
    }

    fn render<'rpass>(
        &'rpass self,
        state: &wgpu_app_lib::wgpu_helper::State,
        render_pass: &mut wgpu::RenderPass<'rpass>,
    ) {
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
