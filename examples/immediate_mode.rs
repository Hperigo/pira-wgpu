use pira_wgpu::{
    framework::{self, Application},
    state::State,
};
use winit::dpi::PhysicalSize;

use pira_wgpu::immediate_mode::DrawContext;

use pira_wgpu::framework::EguiLayer;

struct MyExample {
    im_draw: pira_wgpu::immediate_mode::DrawContext,

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

    fn on_gui(&mut self, ui_layer: &mut EguiLayer) {
        egui::SidePanel::new(egui::panel::Side::Left, egui::Id::new("Side pannel")).show(
            &ui_layer.ctx,
            |ui| {
                ui.label("text");
            },
        );
    }

    fn update(&mut self, state: &State, frame_count: u64, _delta_time: f64) {
        self.im_draw.start();

        self.im_draw.push_color_alpha(0.1, 0.2, 0.3, 0.5);
        self.im_draw.push_rect(100.0, 100.0, 200.0, 100.0);

        self.im_draw.push_color(1.0, 1.0, 0.0);
        self.im_draw.push_rect(100.0, 350.0, 200.0, 100.0);

        self.im_draw.push_color(0.3, 0.4, 0.2);

        let mut points = Vec::new();
        for i in 0..1000 {
            let x = (i as f32) * self.spacing;
            let y = (frame_count as f32 * 0.05 + (i as f32 * self.freq)).sin() * 25.0 + 350.0;

            points.push(glam::vec2(x + 500.0, y + 100.0));

            self.im_draw.push_circle(x, y, 10.0);
        }

        self.im_draw.push_line(&points, 10.0);

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
