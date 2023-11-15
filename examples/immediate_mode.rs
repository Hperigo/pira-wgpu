use wgpu_app_lib::{
    framework::{self, Application},
    state::State,
};
use winit::dpi::PhysicalSize;

use wgpu_app_lib::immediate_mode::DrawContext;

struct MyExample {
    im_draw: wgpu_app_lib::immediate_mode::DrawContext,

    spacing: f32,
    freq: f32,
}

impl Application for MyExample {
    fn init(state: &State) -> Self {
        Self {
            im_draw: DrawContext::new(state),
            spacing: 25.0,
            freq: 0.01,
        }
    }

    fn event(&mut self, _state: &State, _event: &winit::event::WindowEvent) {}

    fn clear_color(&self) -> wgpu::Color {
        wgpu::Color {
            r: 0.3,
            g: 0.2,
            b: 0.4,
            a: 1.0,
        }
    }

    fn update(&mut self, state: &State, ui: &mut imgui::Ui, frame_count: u64, _delta_time: f64) {
        let w = ui
            .window("debug")
            .size([200.0, 300.0], imgui::Condition::FirstUseEver)
            .collapsed(false, imgui::Condition::FirstUseEver)
            .position([0.0, 500.0], imgui::Condition::FirstUseEver)
            .begin();
        if let Some(w) = w {
            imgui::Drag::new("spacing")
                .speed(0.1)
                .build(ui, &mut self.spacing);
            imgui::Drag::new("freq")
                .speed(0.01)
                .build(ui, &mut self.freq);
            w.end();
        }

        self.im_draw.start();

        self.im_draw.push_color_alpha(0.1, 0.2, 0.3, 0.5);
        self.im_draw.push_rect(100.0, 100.0, 200.0, 100.0);

        // self.im_draw.push_color(1.0, 1.0, 0.0);
        // self.im_draw.push_rect(100.0, 350.0, 200.0, 100.0);

        // self.im_draw.push_color(0.3, 0.4, 0.2);

        // let mut points = Vec::new();
        // for i in 0..1000 {
        //     let x = (i as f32) * self.spacing;
        //     let y = (frame_count as f32 * 0.05 + (i as f32 * self.freq)).sin() * 25.0 + 350.0;

        //     points.push(glam::vec2(x, y + 100.0));

        //     self.im_draw.push_circle(x, y, 10.0);
        // }

        // self.im_draw.push_line(&points, 10.0);

        self.im_draw.end(state);
    }

    fn render<'rpass>(&'rpass self, _state: &State, render_pass: &mut wgpu::RenderPass<'rpass>) {
        self.im_draw.draw(render_pass);
    }
}

fn main() {
    framework::run::<MyExample>(
        "imidiate mode",
        PhysicalSize {
            width: 1920 * 2,
            height: 1080 * 2,
        },
        4,
    );
}
