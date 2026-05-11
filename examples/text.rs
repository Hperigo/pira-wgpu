#![allow(dead_code)]
#![allow(unused_variables)]

use pira_wgpu::factories::render_pipeline::DepthConfig;
use pira_wgpu::framework::{self, Application};
use pira_wgpu::state::State;
use wgpu::DepthStencilState;
use wgpu_text::glyph_brush::ab_glyph::FontRef;
use winit::dpi::PhysicalSize;

use wgpu_text::{glyph_brush::{Section as TextSection, Text}, BrushBuilder, TextBrush};


struct MyExample {
    brush : TextBrush<FontRef<'static>>,
}

impl Application for MyExample {
    fn init(state: &State) -> Self {
        
        let font_bytes = include_bytes!("../assets/DejaVuSans.ttf");
        let depth_config = DepthConfig::DefaultWrite;
        let depth_config_result = depth_config.get();

        let brush = BrushBuilder::using_font_bytes(font_bytes)
            .unwrap()
            .with_multisample(wgpu::MultisampleState {
                count: state.sample_count,
                ..Default::default()
            })
            .with_depth_stencil(depth_config_result)
            .build(&state.device, state.window_size.width, state.window_size.height, state.config.format);

        Self { brush }
    }

    fn event(&mut self, state: &State, _event: &winit::event::WindowEvent) {}

    fn clear_color(&self) -> wgpu::Color {
        wgpu::Color {
            r: 0.2,
            g: 0.2,
            b: 0.2,
            a: 1.0,
        }
    }
    fn update(&mut self, state: &mut State, frame_count: u64, delta_time: f64) {
        let section = TextSection::default()
            .add_text(Text::new("Hello World").with_scale(100.0).with_color([1.0, 1.0, 1.0, 1.0]))
            .add_text(Text::new("\nThis is another text\ntesting multi line").with_scale(50.0).with_color([1.0, 0.0, 0.0, 1.0]));
        
        self.brush.queue(&state.device, &state.queue, [&section]).unwrap();

    }

    fn render<'rpass>(&'rpass self, state: &State, render_pass: &mut wgpu::RenderPass<'rpass>) {
        
        self.brush.draw(render_pass);
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
