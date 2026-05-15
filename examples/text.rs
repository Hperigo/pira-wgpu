#![allow(dead_code)]
#![allow(unused_variables)]

use pira_wgpu::factories::render_pipeline::DepthConfig;
use pira_wgpu::framework::{self, Application};
use pira_wgpu::state::State;
use wgpu_text::glyph_brush;
use wgpu_text::glyph_brush::ab_glyph::{FontRef};
use winit::dpi::PhysicalSize;

use wgpu_text::{glyph_brush::{Section as TextSection, Text}, BrushBuilder, TextBrush};


struct MyExample {
    brush : TextBrush<FontRef<'static>>,
    med_id : glyph_brush::FontId,
    reg_id : glyph_brush::FontId,
}

impl Application for MyExample {
    fn init(state: &State) -> Self {
        
        let firacode_medium = include_bytes!("../assets/FiraCode-Medium.ttf");
        let firacode_regular = include_bytes!("../assets/FiraCode-Regular.ttf");
        let depth_config = DepthConfig::DefaultWrite;
        let depth_config_result = depth_config.get();

        
        let regular_font = FontRef::try_from_slice(firacode_regular).unwrap();
        let medium_font = FontRef::try_from_slice(firacode_medium).unwrap();

        let mut builder = BrushBuilder::using_fonts(vec![])
            .with_multisample(wgpu::MultisampleState {
                count: state.sample_count,
                ..Default::default()
            })
            .with_depth_stencil(depth_config_result);

            let reg_id = builder.add_font(regular_font);
            let med_id = builder.add_font(medium_font);

            let brush = builder.build(&state.device, state.window_size.width, state.window_size.height, state.config.format);

        println!("Fonts: {:?}", brush.fonts());
        
        
        Self { 
            brush,
            med_id,
            reg_id,
         }
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
            .add_text(Text::new("Hello World").with_scale(100.0).with_font_id(self.med_id).with_color([1.0, 1.0, 1.0, 1.0]))
            .add_text(Text::new("\nThis is another text\ntesting multi line").with_font_id(self.reg_id).with_scale(50.0).with_color([0.03, 0.03, 0.03, 1.0]));
        
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
            width: 1920,
            height: 1080,
        },
        2,
    );
}
