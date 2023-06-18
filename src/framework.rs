use std::time::Instant;

use winit::{
    dpi::PhysicalSize,
    event::{self, WindowEvent},
    event_loop::EventLoop,
};

use crate::wgpu_helper::{self, factories::render_pass::RenderPassFactory, State};

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

    fn init(state: &wgpu_helper::State) -> Self;

    fn resize(
        &mut self,
        _config: &wgpu::SurfaceConfiguration,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) {
    }

    fn event(&mut self, _state: &State, _event: &winit::event::WindowEvent) {}

    fn update(&mut self, state: &State, ui: &mut imgui::Ui, frame_count: u64, delta_time: f64);

    fn render<'rpass>(&'rpass self, state: &State, render_pass: &mut wgpu::RenderPass<'rpass>);
}

struct Setup {
    window: winit::window::Window,
    size: winit::dpi::PhysicalSize<u32>,
    event_loop: EventLoop<()>,
    state: State,
}

async fn setup<E: Application>(title: &str, size: PhysicalSize<u32>) -> Setup {
    let event_loop = EventLoop::new();
    let mut builder = winit::window::WindowBuilder::new();

    builder = builder.with_title(title).with_inner_size(size);

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

    // Set up dear imgui
    let mut imgui = imgui::Context::create();
    let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
    platform.attach_window(
        imgui.io_mut(),
        &window,
        imgui_winit_support::HiDpiMode::Default,
    );
    imgui.set_ini_filename(None);

    let hidpi_factor = window.scale_factor();

    let font_size = (13.0 * hidpi_factor) as f32;
    imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

    imgui
        .fonts()
        .add_font(&[imgui::FontSource::DefaultFontData {
            config: Some(imgui::FontConfig {
                oversample_h: 1,
                pixel_snap_h: true,
                size_pixels: font_size,
                ..Default::default()
            }),
        }]);

    let renderer_config = imgui_wgpu::RendererConfig {
        texture_format: state
            .window_surface
            .get_capabilities(&state.adapter)
            .formats[0],
        sample_count: 4,
        depth_format: Some(wgpu::TextureFormat::Depth24Plus),
        ..Default::default()
    };

    let mut imgui_renderer =
        imgui_wgpu::Renderer::new(&mut imgui, &state.device, &state.queue, renderer_config);

    event_loop.run(move |event, _, control_flow| {
        match event {
            winit::event::Event::RedrawRequested(_) => {
                let delta_time = Instant::now() - last_frame_inst;
                last_frame_inst = Instant::now();

                let ui: &mut imgui::Ui = imgui.frame();
                application.update(&state, ui, frame_count, delta_time.as_secs_f64());
                frame_count += 1;

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

                    imgui_renderer
                        .render(
                            imgui.render(),
                            &state.queue,
                            &state.device,
                            &mut render_pass,
                        )
                        .expect("Rendering failed");
                });
            }
            winit::event::Event::MainEventsCleared => {
                window.request_redraw();
            }
            winit::event::Event::WindowEvent { ref event, .. } => {
                let ui_active = unsafe {
                    imgui::sys::igIsWindowHovered(imgui::sys::ImGuiHoveredFlags_AnyWindow as i32)
                };

                if !ui_active {
                    application.event(&mut state, event);
                } else {
                    println!("{} Active: {}", frame_count, ui_active);
                }

                if matches!(event, WindowEvent::CloseRequested | WindowEvent::Destroyed) {
                    *control_flow = winit::event_loop::ControlFlow::Exit;
                }

                if let winit::event::WindowEvent::Resized(physical_size) = event {
                    state.window_size = [physical_size.width as f32, physical_size.height as f32];
                    state.resize(*physical_size);
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
        platform.handle_event(imgui.io_mut(), &window, &event);
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run<E: Application>(title: &str, size: PhysicalSize<u32>) {
    let setup = pollster::block_on(setup::<E>(title, size));
    start::<E>(setup);
}
