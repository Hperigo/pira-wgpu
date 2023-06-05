use std::future::Future;
use std::time::Instant;

use winit::{
    dpi::PhysicalSize,
    event::{self, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

use crate::wgpu_helper::{
    self,
    factories::render_pass::{self, RenderPassFactory},
    State,
};

pub trait Example: 'static + Sized {
    fn optional_features() -> wgpu::Features {
        wgpu::Features::empty()
    }
    fn required_features() -> wgpu::Features {
        wgpu::Features::empty()
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
    fn init(config: &wgpu::SurfaceConfiguration, state: &wgpu_helper::State) -> Self;
    fn resize(
        &mut self,
        config: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    );
    fn update(&mut self, event: WindowEvent);
    fn render<'rpass>(&'rpass self, state: &State, render_pass: &mut wgpu::RenderPass<'rpass>);
}

struct Setup {
    window: winit::window::Window,
    size: winit::dpi::PhysicalSize<u32>,
    event_loop: EventLoop<()>,
    // instance: wgpu::Instance,
    // surface: wgpu::Surface,
    // adapter: wgpu::Adapter,
    // device: wgpu::Device,
    // queue: wgpu::Queue,
    state: State,
}

async fn setup<E: Example>(title: &str) -> Setup {
    let event_loop = EventLoop::new();
    let mut builder = winit::window::WindowBuilder::new();

    builder = builder.with_title(title).with_inner_size(PhysicalSize {
        width: 1920,
        height: 1080,
    });

    let window = builder.build(&event_loop).unwrap();
    let size = window.inner_size();

    let state = wgpu_helper::State::new(&window).await;

    Setup {
        window,
        event_loop,
        size,
        state,
    }
}

fn start<E: Example>(
    Setup {
        window,
        event_loop,
        size,
        state,
    }: Setup,
) {
    let mut config = state
        .window_surface
        .get_default_config(&state.adapter, size.width, size.height)
        .expect("Surface isn't supported by the adapter.");
    let surface_view_format = config.format.add_srgb_suffix();
    config.view_formats.push(surface_view_format);
    state.window_surface.configure(&state.device, &config);

    let mut example = E::init(&config, &state);

    let mut last_frame_inst = Instant::now();
    let (mut frame_count, mut accum_time) = (0, 0.0);

    event_loop.run(move |event, _, control_flow| match event {
        winit::event::Event::RedrawRequested(_) => {
            state.render(|ctx, frame_data| {
                let mut render_pass_factory = RenderPassFactory::new();
                render_pass_factory.add_color_atachment(
                    wgpu::Color::RED,
                    &frame_data.multisampled_view,
                    Some(&frame_data.view),
                );

                let mut render_pass =
                    render_pass_factory.get_render_pass(ctx, &mut frame_data.encoder, true);

                example.render(&state, &mut render_pass);
            });
        }
        winit::event::Event::MainEventsCleared => {
            window.request_redraw();
        }
        winit::event::Event::WindowEvent { ref event, .. } => {
            use winit::event::WindowEvent;

            if matches!(event, WindowEvent::CloseRequested | WindowEvent::Destroyed) {
                *control_flow = winit::event_loop::ControlFlow::Exit;
            }

            if let winit::event::WindowEvent::Resized(_physical_size) = event {}

            if let winit::event::WindowEvent::Moved(_position) = event {
                *control_flow = winit::event_loop::ControlFlow::Wait;
            }

            if let winit::event::WindowEvent::CursorMoved { position, .. } = event {
                // app.input_state.mouse_pos = (position.x as f32, position.y as f32);
            }

            if let winit::event::WindowEvent::Focused(focused) = event {
                // app.window_visible = *focused;
            }

            if let winit::event::WindowEvent::KeyboardInput { input, .. } = event {
                if input.virtual_keycode == Some(event::VirtualKeyCode::Escape) {
                    *control_flow = winit::event_loop::ControlFlow::Exit;
                }
            }

            // if let Some(event_fn) = builder.event_fn {
            //     event_fn(&mut app, &mut data, event);
            // }
            //window.window().request_redraw(); // TODO: ask egui if the events warrants a repaint instead
        }
        _ => {}
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run<E: Example>(title: &str) {
    let setup = pollster::block_on(setup::<E>(title));
    start::<E>(setup);
}
