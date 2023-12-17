use std::time::Instant;

use winit::{
    dpi::PhysicalSize,
    event::{self, WindowEvent},
    event_loop::EventLoop,
};

//use crate::wgpu::{self, factories::render_pass::RenderPassFactory, State};
use crate::{
    factories::render_pass::RenderPassFactory,
    state::{Size, State},
};

pub trait Application: 'static + Sized {
    fn optional_features() -> wgpu::Features {
        wgpu::Features::empty()
    }
    fn required_features() -> wgpu::Features {
        wgpu::Features::empty()
    }

    fn clear_color(&self) -> wgpu::Color {
        wgpu::Color::BLACK
    }

    fn required_downlevel_capabilities() -> wgpu::DownlevelCapabilities {
        wgpu::DownlevelCapabilities {
            flags: wgpu::DownlevelFlags::empty(),
            shader_model: wgpu::ShaderModel::Sm5,
            ..wgpu::DownlevelCapabilities::default()
        }
    }

    fn required_limits() -> wgpu::Limits {
        wgpu::Limits::downlevel_webgl2_defaults() // These downlevel limits will allow the code to run on all possible hardware
    }

    fn init(state: &State) -> Self;

    fn resize(
        &mut self,
        _config: &wgpu::SurfaceConfiguration,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) {
    }

    fn event(&mut self, _state: &State, _event: &winit::event::WindowEvent) {}

    fn update(&mut self, state: &State, frame_count: u64, delta_time: f64);

    fn render<'rpass>(&'rpass self, state: &State, render_pass: &mut wgpu::RenderPass<'rpass>);
}

struct Setup {
    window: winit::window::Window,
    size: winit::dpi::PhysicalSize<u32>,
    event_loop: EventLoop<()>,
    state: State,
}

async fn setup<E: Application>(title: &str, size: PhysicalSize<u32>, sample_count: u32) -> Setup {
    let event_loop = EventLoop::new();
    let mut builder = winit::window::WindowBuilder::new();
    println!("1. Size: {:?}", size);
    builder = builder.with_title(title).with_inner_size(size);

    let window = builder.build(&event_loop).unwrap();
    let size = window.inner_size();
    println!("2. Size: {:?}", size);
    let instance = wgpu::Instance::default();
    let window_surface = unsafe { instance.create_surface(&window).unwrap() };

    let state = State::new(
        sample_count,
        instance,
        window_surface,
        Size::new(size.width, size.height),
    )
    .await;

    Setup {
        window,
        event_loop,
        size,
        state,
    }
}

fn start<E: Application>(
    Setup {
        window,
        event_loop,
        size,
        mut state,
    }: Setup,
) {
    let mut config = state
        .window_surface
        .get_default_config(&state.adapter, size.width, size.height)
        .expect("Surface isn't supported by the adapter.");

    let surface_view_format = config.format.add_srgb_suffix();
    config.view_formats.push(surface_view_format);

    state.window_surface.configure(&state.device, &config);

    let mut last_frame_inst = Instant::now();
    let mut frame_count = 0;

    // INIT APPLICATION
    let mut application = E::init(&state);

    event_loop.run(move |event, _, control_flow| {
        match event {
            winit::event::Event::RedrawRequested(_) => {
                let delta_time = Instant::now() - last_frame_inst;
                last_frame_inst = Instant::now();

                // let ui: &mut imgui::Ui = imgui.frame();
                application.update(&state, frame_count, delta_time.as_secs_f64());
                frame_count += 1;

                state.delta_time = delta_time.as_millis() as f32;

                state.render(|ctx, frame_data| {
                    let mut render_pass_factory = RenderPassFactory::new();
                    render_pass_factory.add_color_atachment(
                        application.clear_color(),
                        &frame_data.multisampled_view,
                        Some(&frame_data.view),
                    );

                    let mut render_pass =
                        render_pass_factory.get_render_pass(ctx, &mut frame_data.encoder, true);

                    application.render(&state, &mut render_pass);
                });
            }
            winit::event::Event::MainEventsCleared => {
                window.request_redraw();
            }
            winit::event::Event::WindowEvent { ref event, .. } => {
                if matches!(event, WindowEvent::CloseRequested | WindowEvent::Destroyed) {
                    *control_flow = winit::event_loop::ControlFlow::Exit;
                }

                if let winit::event::WindowEvent::Resized(physical_size) = event {
                    // state.window_size = Size::new(physical_size.width, physical_size.height);

                    println!("Size: {:?}", physical_size);
                    // state.resize(*physical_size);
                }

                if let winit::event::WindowEvent::Focused(_focused) = event {}

                if let winit::event::WindowEvent::KeyboardInput { input, .. } = event {
                    if input.virtual_keycode == Some(event::VirtualKeyCode::Escape) {
                        *control_flow = winit::event_loop::ControlFlow::Exit;
                    }
                }
            }
            _ => {}
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run<E: Application>(title: &str, size: PhysicalSize<u32>, sample_count: u32) {
    let setup = pollster::block_on(setup::<E>(title, size, sample_count));
    start::<E>(setup);
}
