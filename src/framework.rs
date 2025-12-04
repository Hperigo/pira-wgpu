use std::{sync::Arc, time::Instant};

use egui::{FontDefinitions, ViewportId};
use egui_wgpu::ScreenDescriptor;
use winit::{dpi::PhysicalSize, event::WindowEvent, event_loop::EventLoop, keyboard::Key};

use crate::{
    factories::render_pass::RenderPassFactory,
    state::{PerFrameData, Size, State},
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

    fn on_gui(&mut self, _egui_ctx: &mut EguiLayer) {}

    fn event(&mut self, _state: &State, _event: &winit::event::WindowEvent) {}

    fn update(&mut self, state: &State, frame_count: u64, delta_time: f64);

    fn render<'rpass>(&'rpass self, state: &State, render_pass: &mut wgpu::RenderPass<'rpass>);
}

pub trait UILayer {
    fn setup(window: &winit::window::Window, device: &wgpu::Device) -> Self
    where
        Self: Sized;
    fn event(
        &mut self,
        _window: &winit::window::Window,
        _event: &winit::event::WindowEvent,
    ) -> bool {
        false
    }
    fn begin_gui(&mut self);
    fn end_gui(
        &mut self,
        window: &winit::window::Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
    );

    fn render<'rpass>(
        &'rpass self,
        render_pass: &mut wgpu::RenderPass<'rpass>,
        screen_descriptor: &ScreenDescriptor,
    );
}

pub struct EguiLayer {
    pub ctx: egui::Context,
    winit_state: egui_winit::State,
    renderer: egui_wgpu::Renderer,

    primitives: Vec<egui::ClippedPrimitive>,
}

impl UILayer for EguiLayer {
    fn setup(window: &winit::window::Window, device: &wgpu::Device) -> Self {
        let ctx = egui::Context::default();
        ctx.set_fonts(FontDefinitions::default());

        egui_extras::install_image_loaders(&ctx);

        // let winit_state = egui_winit::State::new(
        //     ViewportId::ROOT,
        //     &window,
        //     Some(window.scale_factor() as f32),
        //     Some(1024),
        // );

        let winit_state = egui_winit::State::new(
            ctx.clone(),
            ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            Some(1024),
        );

        let renderer = egui_wgpu::Renderer::new(
            device,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            Some(wgpu::TextureFormat::Depth24Plus),
            4,
        );

        Self {
            ctx,
            winit_state,
            renderer,
            primitives: Vec::new(),
        }
    }
    fn event(&mut self, window: &winit::window::Window, event: &winit::event::WindowEvent) -> bool {
        let _ = self.winit_state.on_window_event(window, event);
        self.ctx.is_pointer_over_area()
    }
    fn begin_gui(&mut self) {
        let raw_input = self.winit_state.egui_input_mut().take();
        self.ctx.begin_frame(raw_input);
    }

    fn end_gui(
        &mut self,
        window: &winit::window::Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let output = self.ctx.end_frame();

        self.winit_state
            .handle_platform_output(&window, output.platform_output);

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [window.inner_size().width, window.inner_size().height],
            pixels_per_point: window.scale_factor() as f32,
        };

        let primitives = self.ctx.tessellate(output.shapes, output.pixels_per_point);

        for (id, image_delta) in &output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }

        for id in output.textures_delta.free {
            self.renderer.free_texture(&id);
        }

        self.renderer
            .update_buffers(device, queue, encoder, &primitives, &screen_descriptor);

        self.primitives = primitives;
    }

    fn render<'rpass>(
        &'rpass self,
        render_pass: &mut wgpu::RenderPass<'rpass>,
        screen_descriptor: &ScreenDescriptor,
    ) {
        self.renderer
            .render(render_pass, &self.primitives, screen_descriptor)
    }
}

struct Setup {
    window: Arc<winit::window::Window>,
    size: winit::dpi::PhysicalSize<u32>,
    event_loop: EventLoop<()>,
    state: State,
}

async fn setup<E: Application>(title: &str, size: PhysicalSize<u32>, sample_count: u32) -> Setup {
    let event_loop = EventLoop::new().unwrap();
    let mut builder = winit::window::WindowBuilder::new();
    builder = builder.with_title(title).with_inner_size(size);

    let window = Arc::new(builder.build(&event_loop).unwrap());

    println!("Window scale factor: {}", window.scale_factor());

    let size = window.inner_size();
    let instance = wgpu::Instance::default();
    let window_surface = { instance.create_surface(window.clone()).unwrap() };

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
    let _server_addr = format!("127.0.0.1:{}", puffin_http::DEFAULT_PORT);
    // let _puffin_server = puffin_http::Server::new(&server_addr).or(None);
    // eprintln!("Run this to view profiling data:  puffin_viewer {server_addr}");
    puffin::set_scopes_on(true);

    let mut config = state
        .window_surface
        .get_default_config(&state.adapter, size.width, size.height)
        .expect("Surface isn't supported by the adapter.");

    let surface_view_format = config.format.add_srgb_suffix();
    config.view_formats.push(surface_view_format);

    state.window_surface.configure(&state.device, &config);

    let mut last_frame_inst = Instant::now();
    let mut frame_count = 0;

    let mut application = {
        puffin::profile_scope!("Application init");
        E::init(&state)
    };

    let mut ui = EguiLayer::setup(&window, &state.device);

    puffin::GlobalProfiler::lock().new_frame();
    let _ = event_loop.run(move |event, control_flow| {
        match event {
            winit::event::Event::AboutToWait => {
                window.request_redraw();
            }
            winit::event::Event::WindowEvent { ref event, .. } => {
                let is_ui_using_event = ui.event(&window, event);

                match event {
                    WindowEvent::RedrawRequested => {
                        let delta_time = Instant::now() - last_frame_inst;
                        last_frame_inst = Instant::now();

                        {
                            puffin::profile_scope!("update");
                            application.update(&state, frame_count, delta_time.as_secs_f64());
                        }
                        frame_count += 1;

                        state.delta_time = delta_time.as_millis() as f32;

                        state.render(|ctx, frame_data| {
                            puffin::profile_scope!("Render");
                            let mut render_pass_factory = RenderPassFactory::new();

                            let PerFrameData {
                                encoder,
                                view,
                                multisampled_view,
                            } = frame_data;

                            ui.begin_gui();

                            application.on_gui(&mut ui);

                            ui.end_gui(&window, &state.device, &state.queue, encoder);

                            {

                                if state.sample_count > 1 {
                                    render_pass_factory.add_color_atachment(
                                        application.clear_color(),
                                        &multisampled_view,
                                        Some(&view),
                                    );
                                }else{
                                    render_pass_factory.add_color_atachment(application.clear_color(), &view, None);
                                }

                                let mut render_pass =
                                    render_pass_factory.get_render_pass(ctx, encoder, true);

                                application.render(&state, &mut render_pass);

                                let screen_descriptor = ScreenDescriptor {
                                    size_in_pixels: [
                                        window.inner_size().width,
                                        window.inner_size().height,
                                    ],
                                    pixels_per_point: window.scale_factor() as f32, //window.scale_factor() as f32,
                                };
                                ui.render(&mut render_pass, &screen_descriptor)
                            }
                        });
                        puffin::GlobalProfiler::lock().new_frame();
                    }
                    WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                        control_flow.exit();
                    }
                    WindowEvent::KeyboardInput { event, .. } => match event.logical_key {
                        Key::Named(winit::keyboard::NamedKey::Escape) => {
                            control_flow.exit();
                        }
                        _ => (),
                    },
                    _ => (),
                }

                if !is_ui_using_event {
                    puffin::profile_scope!("Event");
                    application.event(&state, event);
                }
                // if matches!(event, WindowEvent::CloseRequested | WindowEvent::Destroyed) {
                //
                // }

                //

                // if let winit::event::WindowEvent::Resized(physical_size) = event {
                //     // state.window_size = Size::new(physical_size.width, physical_size.height);

                //     println!("Size: {:?}", physical_size);
                //     // state.resize(*physical_size);
                // }

                // if let winit::event::WindowEvent::Focused(_focused) = event {}

                // if let winit::event::WindowEvent::KeyboardInput { input, .. } = event {
                //     if input.virtual_keycode == Some(event::VirtualKeyCode::Escape) {
                //         *control_flow = winit::event_loop::ControlFlow::Exit;
                //     }
                // }
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
