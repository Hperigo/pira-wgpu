#![allow(dead_code)]
#![allow(unused_variables)]

use pira_wgpu::framework::{self, Application};
use pira_wgpu::state::State;
use winit::dpi::PhysicalSize;

struct MyExample {}

impl Application for MyExample {
    fn init(state: &State) -> Self {
        Self {}
    }

    fn event(&mut self, state: &State, _event: &winit::event::WindowEvent) {}

    fn update(&mut self, state: &State, frame_count: u64, delta_time: f64) {}

    fn render<'rpass>(&'rpass self, state: &State, render_pass: &mut wgpu::RenderPass<'rpass>) {}
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
