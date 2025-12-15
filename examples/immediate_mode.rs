use pira_wgpu::factories::texture::TextureBundle;
use pira_wgpu::framework::EguiLayer;
use pira_wgpu::immediate_mode::DrawContext;
use pira_wgpu::{
    factories::{
        self,
        texture::{SamplerOptions, Texture2dOptions},
    },
    framework::{self, Application},
    state::State,
};
use winit::dpi::PhysicalSize;

use image::EncodableLayout;

struct MyExample {
    im_draw: pira_wgpu::immediate_mode::DrawContext,

    spacing: f32,
    freq: f32,

    texture_bundle: TextureBundle,
    toronto_texture_bundle: TextureBundle,
}

impl Application for MyExample {
    fn init(state: &State) -> Self {

        let base_path = std::env::current_exe().unwrap().join( "../../../../").canonicalize().unwrap();
        println!("{:?}", base_path);


        let image = image::open(base_path.join("./assets/rusty.png")).unwrap().to_rgba8();
        let rust_texture_bundle = factories::Texture2dFactory::new_with_options(
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

        let image = image::open(base_path.join("./assets/toronto-skyline.jpeg"))
            .unwrap()
            .to_rgba8();
        let toronto_texture_bundle = factories::Texture2dFactory::new_with_options(
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

        Self {
            im_draw: DrawContext::new(state),
            spacing: 25.0,
            freq: 0.01,

            texture_bundle: rust_texture_bundle,
            toronto_texture_bundle,
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

    fn on_gui(&mut self, _ui_layer: &mut EguiLayer) {}

    fn update(&mut self, state: &mut State, frame_count: u64, _delta_time: f64) {
        self.im_draw.start();

        self.im_draw.push_color_alpha(0.1, 0.2, 0.3, 1.0);
        self.im_draw.push_rect(100.0, 100.0, 200.0, 100.0);

        self.im_draw.push_color(1.0, 1.0, 1.0);
        self.im_draw.push_circle_stroke(250.0, 250.0, 50.0);


        self.im_draw.push_texture(
            &state.device,
            &self.toronto_texture_bundle.view,
            &self.toronto_texture_bundle.sampler,
        );
        self.im_draw.push_color_alpha(1.0, 1.0, 1.0, 1.0);
        self.im_draw.push_rect(300.0, 100.0, 200.0, 100.0);

        self.im_draw.pop_texture();
        self.im_draw.push_color(1.0, 0.2, 0.2);
        let mut points = Vec::new();

        self.im_draw.set_transform( glam::Mat4::from_translation(  glam::vec3(0.0, 1000.0, 0.0) ) );

        for i in 0..500 {
            let x = (i as f32) * self.spacing;
            let y = (frame_count as f32 * 0.05 + (i as f32 * self.freq)).sin() * 25.0 + 350.0;

            points.push(glam::vec2(x + 500.0, y + 100.0));
            self.im_draw.push_circle(x, y, 10.0);
        }
        self.im_draw.clear_transform();

        self.im_draw.push_color(1.0, 1.0, 1.0);
        self.im_draw.push_line(&points, 10.0);

        self.im_draw.push_texture(
            &state.device,
            &self.texture_bundle.view,
            &self.texture_bundle.sampler,
        );

        let x = (frame_count as f32 * 0.05).sin() * 25.0 + 350.0;
        self.im_draw.set_transform( glam::Mat4::from_translation(  glam::vec3(x, 200.0, 0.0) ) );
        self.im_draw.push_rect(100.0, 350.0, 200.0, 100.0);
        self.im_draw.pop_texture();
        self.im_draw.clear_transform();

        self.im_draw.end(state);
    }

    fn render<'rpass>(&'rpass self, state: &State, render_pass: &mut wgpu::RenderPass<'rpass>) {
        self.im_draw.draw(state, render_pass);
    }
}

fn main() {
    framework::run::<MyExample>(
        "imidiate mode",
        PhysicalSize {
            width: 1920 * 2,
            height: 1080 * 2,
        },
        1,
    );
}
