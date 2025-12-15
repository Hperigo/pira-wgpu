use std::{process::exit, sync::Arc, time::Instant};

use egui::{FontDefinitions, ViewportId};
use egui_wgpu::ScreenDescriptor;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use crate::{
    factories::{DepthTextureFactory, render_pass::RenderPassFactory},
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
    fn setup(window: &Window, device: &wgpu::Device) -> Self
    where
        Self: Sized;
    fn event(&mut self, _window: &Window, _event: &WindowEvent) -> bool {
        false
    }
    fn begin_gui(&mut self);
    fn end_gui(
        &mut self,
        window: &Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
    );

    fn render<'rpass>(
        &'rpass self,
        render_pass: wgpu::RenderPass<'rpass>,
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
    fn setup(window: &Window, device: &wgpu::Device) -> Self {
        let ctx = egui::Context::default();
        ctx.set_fonts(FontDefinitions::default());

        egui_extras::install_image_loaders(&ctx);

        let winit_state = egui_winit::State::new(
            ctx.clone(),
            ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            Some(1024),
        );

        let renderer = egui_wgpu::Renderer::new(
            device,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            egui_wgpu::RendererOptions {
                msaa_samples: 4,
                depth_stencil_format: Some( DepthTextureFactory::get_default_depth_format() ),
                dithering: true,
                predictable_texture_filtering: false,
            },
        );

        Self {
            ctx,
            winit_state,
            renderer,
            primitives: Vec::new(),
        }
    }

    fn event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        let _ = self.winit_state.on_window_event(window, event);
        self.ctx.is_pointer_over_area()
    }

    fn begin_gui(&mut self) {
        let raw_input = self.winit_state.egui_input_mut().take();
        self.ctx.begin_pass(raw_input);
    }

    fn end_gui(
        &mut self,
        window: &Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let output = self.ctx.end_pass();

        self.winit_state
            .handle_platform_output(window, output.platform_output);

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

    fn render(
        &self,
        render_pass: wgpu::RenderPass,
        screen_descriptor: &ScreenDescriptor,
    ) {

        self.renderer.render(&mut render_pass.forget_lifetime(), &self.primitives, screen_descriptor);
        // self.renderer
        //     .render(render_pass, &self.primitives, screen_descriptor)
    }
}

struct AppHandler<E: Application> {
    window: Option<Arc<Window>>,
    state: Option<State>,
    application: Option<E>,
    ui: Option<EguiLayer>,
    last_frame_inst: Instant,
    frame_count: u64,
    title: String,
    size: PhysicalSize<u32>,
    sample_count: u32,
}

impl<E: Application> AppHandler<E> {
    fn new(title: String, size: PhysicalSize<u32>, sample_count: u32) -> Self {
        Self {
            window: None,
            state: None,
            application: None,
            ui: None,
            last_frame_inst: Instant::now(),
            frame_count: 0,
            title,
            size,
            sample_count,
        }
    }
}

impl<E: Application> ApplicationHandler for AppHandler<E> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title(&self.title)
            .with_inner_size(self.size);

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        println!("Window scale factor: {}", window.scale_factor());

        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let window_surface = instance.create_surface(window.clone()).unwrap();

        let state = pollster::block_on(State::new(
            self.sample_count,
            instance,
            window_surface,
            Size::new(size.width, size.height),
        ));

        // let mut config = state
        //     .window_surface
        //     .get_default_config(&state.adapter, size.width, size.height)
        //     .expect("Surface isn't supported by the adapter.");
        
        let caps = state.window_surface.get_capabilities(&state.adapter);

        let mut config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            format: *caps.formats.first().unwrap(),
            width: size.width,
            height: size.height,
            desired_maximum_frame_latency: 2,
            present_mode: *caps.present_modes.first().unwrap(),
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };
        
        let surface_view_format = config.format.add_srgb_suffix();
        config.view_formats.push(surface_view_format);

        state.window_surface.configure(&state.device, &config);

        puffin::set_scopes_on(true);

        let application = {
            puffin::profile_scope!("Application init");
            E::init(&state)
        };

        let ui = EguiLayer::setup(&window, &state.device);

        puffin::GlobalProfiler::lock().new_frame();

        self.window = Some(window);
        self.state = Some(state);
        self.application = Some(application);
        self.ui = Some(ui);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = &self.window else {
            return;
        };
        let Some(state) = &mut self.state else {
            return;
        };
        let Some(application) = &mut self.application else {
            return;
        };
        let Some(ui) = &mut self.ui else {
            return;
        };

        let is_ui_using_event = ui.event(window, &event);
        application.event(state, &event);

        match event {
            WindowEvent::RedrawRequested => {
                let delta_time = Instant::now() - self.last_frame_inst;
                self.last_frame_inst = Instant::now();

                {
                    puffin::profile_scope!("update");
                    application.update(&state, self.frame_count, delta_time.as_secs_f64());
                }
                self.frame_count += 1;

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

                    application.on_gui(ui);

                    ui.end_gui(&window, &state.device, &state.queue, encoder);

                    {
                        if state.sample_count > 1 {
                            render_pass_factory.add_color_atachment(
                                application.clear_color(),
                                &multisampled_view,
                                Some(&view),
                            );
                        } else {
                            render_pass_factory
                                .add_color_atachment(application.clear_color(), &view, None);
                        }

                        let mut render_pass =
                            render_pass_factory.get_render_pass(ctx, encoder, true);

                        application.render(&state, &mut render_pass);

                        let screen_descriptor = ScreenDescriptor {
                            size_in_pixels: [
                                window.inner_size().width,
                                window.inner_size().height,
                            ],
                            pixels_per_point: window.scale_factor() as f32,
                        };
                        ui.render(render_pass, &screen_descriptor)
                    }
                });

                puffin::GlobalProfiler::lock().new_frame();

                window.request_redraw();
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.physical_key == PhysicalKey::Code(KeyCode::Escape) {
                    event_loop.exit();
                }
            }
            _ => {}
        }

        if !is_ui_using_event {
            puffin::profile_scope!("Event");
            // application.event(&state, &event);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run<E: Application>(title: &str, size: PhysicalSize<u32>, sample_count: u32) {
    let event_loop = EventLoop::new().unwrap();
    let mut app = AppHandler::<E>::new(title.to_string(), size, sample_count);
    let _ = event_loop.run_app(&mut app);
}