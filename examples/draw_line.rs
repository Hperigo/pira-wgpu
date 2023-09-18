use std::borrow::Cow;

use glam::Vec2;
use wgpu;
use wgpu::util::DeviceExt;

use wgpu::RenderPass;
use wgpu_app_lib::factories::{BindGroupFactory, RenderPipelineFactory};
use wgpu_app_lib::framework::{self, Application};
use wgpu_app_lib::state::State;
use winit::dpi::PhysicalSize;
use winit::event::{self, ElementState};

const SHADER_SRC: &'static str = " 

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct InstanceInput {
    @location(2) instance_position : vec2<f32>,
    @location(3) next_position : vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> camera: mat4x4<f32>;

@vertex
fn vs_main( model : VertexInput,  instance : InstanceInput) -> VertexOutput {

    var x_basis = instance.next_position - instance.instance_position ;
    var y_basis = normalize(vec2(-x_basis.y, x_basis.x));

    var point = instance.instance_position + x_basis * model.position.x + y_basis * 1.0 * model.position.y;

    var out: VertexOutput;
    out.clip_position = camera * vec4( vec3(point, 0.0), 1.0 );
    out.color = model.color;
    return out;
}

@fragment
fn fs_main(in : VertexOutput) -> @location(0) vec4<f32> {
    return vec4(1.0, 1.0, 1.0, 1.0); // * vec4(in.color, 1.0);
}
";

const CIRCLE_SHADER_SRC: &'static str = " 

struct VertexInput {
    @location(0) position: vec2<f32>,
};

struct InstanceInput {
    @location(1) pos : vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: mat4x4<f32>;

@vertex
fn vs_main( model : VertexInput, instance : InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera * vec4( vec3(model.position + instance.pos, 0.0), 1.0 ); 
    return out;
}

@fragment
fn fs_main(in : VertexOutput) -> @location(0) vec4<f32> {
    return vec4(1.0, 1.0, 1.0, 1.0);
}
";

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

struct PathDrawCmd {
    start_index: i32,
    end_index: i32,
}

struct MyExample {
    clear_color: [f32; 4],
    buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    circle_buffer: wgpu::Buffer,

    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,

    c_pipeline: wgpu::RenderPipeline,

    draw_commands: Vec<PathDrawCmd>,

    //data
    draw_next_buffer: bool,
    instance_points: Vec<glam::Vec2>,

    // input
    is_mouse_down: bool,
    range: i32,
}

impl Application for MyExample {
    fn init(state: &State) -> Self {
        let rect_size = 1.0;
        let rect_height = 5.0;
        let vertices = vec![
            Vertex {
                position: [-0.0, -rect_height, 0.0],
                color: [1.0, 0.0, 0.0],
            },
            Vertex {
                position: [rect_size, rect_height, 0.0],
                color: [0.0, 1.0, 0.0],
            },
            Vertex {
                position: [rect_size, -rect_height, 0.0],
                color: [0.0, 0.0, 1.0],
            },
            Vertex {
                position: [-0.0, rect_height, 0.0],
                color: [1.0, 1.0, 1.0],
            },
        ];
        let indices: [u16; 6] = [0, 1, 2, 0, 3, 1];
        // indices.reverse();

        let vert_count = 16.0f32;

        let step = (1.0 / vert_count) * std::f32::consts::PI * 2.0;
        let mut current_step = 0.0;

        let mut circle_vertices: Vec<Vec2> = Vec::new();
        let radius = 5.0;
        while current_step < std::f32::consts::PI * 2.0 {
            let x1 = current_step.cos() * radius;
            let y1 = current_step.sin() * radius;

            let next_step = current_step + step;
            let x2 = next_step.cos() * radius;
            let y2 = next_step.sin() * radius;

            circle_vertices.push(glam::vec2(x1, y1));
            circle_vertices.push(glam::vec2(0.0, 0.0));
            circle_vertices.push(glam::vec2(x2, y2));

            current_step += step;
        }

        // circle_vertices.reverse();

        let c_buffer_size = circle_vertices.len() as u64 * std::mem::size_of::<Vec2>() as u64;
        println!("{}", c_buffer_size);
        let circle_buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("circle buffer"),
            size: c_buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        state
            .queue
            .write_buffer(&circle_buffer, 0, bytemuck::cast_slice(&circle_vertices));

        let perspective_matrix = glam::Mat4::orthographic_lh(
            0.0,
            state.window_size.width_f32(),
            state.window_size.height_f32(),
            0.0,
            -1.0,
            1.0,
        );

        // let perspective_matrix = glam::Mat4::IDENTITY;

        let uniform_buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("CameraMatrix"),
            size: std::mem::size_of::<glam::Mat4>() as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        state.queue.write_buffer(
            &uniform_buffer,
            0,
            bytemuck::cast_slice(perspective_matrix.as_ref()),
        );

        let mut bind_group_factory = BindGroupFactory::new();
        bind_group_factory.add_uniform(
            wgpu::ShaderStages::VERTEX,
            &uniform_buffer,
            wgpu::BufferSize::new(std::mem::size_of::<glam::Mat4>() as _),
        );
        let (bind_group_layout, bind_group) = bind_group_factory.build(&state.device);

        let circle_attribs = wgpu::vertex_attr_array![0 => Float32x2];
        let c_instance_attribs = wgpu::vertex_attr_array![1 => Float32x2];
        let mut pipeline_factory = RenderPipelineFactory::new();
        pipeline_factory.set_topology(wgpu::PrimitiveTopology::TriangleList);
        pipeline_factory
            .add_vertex_attributes(&circle_attribs, std::mem::size_of::<glam::Vec2>() as u64);

        pipeline_factory.add_instance_attributes(
            &c_instance_attribs,
            std::mem::size_of::<glam::Vec2>() as u64,
        );

        pipeline_factory.add_depth_stencil();

        let c_shader_module = state
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(CIRCLE_SHADER_SRC)),
            });

        let c_pipeline = pipeline_factory.create_render_pipeline(
            &state,
            &c_shader_module,
            &[&bind_group_layout],
        );

        let instance_data: Vec<Vec2> = Vec::new();
        let shader_module = state
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER_SRC)),
            });

        let attribs = wgpu::vertex_attr_array![ 0 => Float32x3, 1 => Float32x3 ];
        let instance_attribs = wgpu::vertex_attr_array![ 2 => Float32x2, 3 => Float32x2];
        // let instance_attribs2 = wgpu::vertex_attr_array![ ];
        let stride = std::mem::size_of::<Vertex>() as u64;
        let mut pipeline_factory = RenderPipelineFactory::new();
        pipeline_factory.add_vertex_attributes(&attribs, stride);
        pipeline_factory
            .add_instance_attributes(&instance_attribs, std::mem::size_of::<glam::Vec2>() as u64);

        pipeline_factory.add_depth_stencil();

        let pipeline =
            pipeline_factory.create_render_pipeline(&state, &shader_module, &[&bind_group_layout]);

        MyExample {
            clear_color: [0.0, 0.0, 0.0, 0.0],
            buffer: state
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex buffer"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                }),
            index_buffer: state
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Index Buffer"),
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsages::INDEX,
                }),

            instance_buffer: state
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Instance buffer"),
                    contents: bytemuck::cast_slice(&instance_data),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                }),

            circle_buffer,
            c_pipeline,
            bind_group,
            pipeline,
            range: 0,

            // uniform_buffer,
            draw_commands: Vec::new(),

            instance_points: Vec::new(),
            draw_next_buffer: false,
            is_mouse_down: false,
        }
    }

    fn clear_color(&self) -> wgpu::Color {
        wgpu::Color {
            r: self.clear_color[0] as f64,
            g: self.clear_color[1] as f64,
            b: self.clear_color[2] as f64,
            a: self.clear_color[3] as f64,
        }
    }

    fn update(&mut self, _state: &State, ui: &mut imgui::Ui, _frame_count: u64, _delta_time: f64) {
        let w = ui
            .window("debug")
            .size([200.0, 300.0], imgui::Condition::FirstUseEver)
            .begin();
        if let Some(w) = w {
            ui.label_text("Delta time: ", format!("{}", _state.delta_time));
            ui.label_text(
                "number of instances: ",
                format!("{}", self.instance_points.len()),
            );

            ui.label_text("number of cmds: ", format!("{}", self.draw_commands.len()));
            imgui::Drag::new("clear color")
                .speed(0.01)
                .range(0.0, 1.0)
                .build_array(ui, &mut self.clear_color);

            ui.checkbox("draw next buffer", &mut self.draw_next_buffer);

            // imgui::InputInt::new(ui, "range", &mut self.range);
            ui.input_int("range", &mut self.range).build();

            w.end();
        }
    }

    fn resize(
        &mut self,
        _config: &wgpu::SurfaceConfiguration,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) {
    }

    fn event(&mut self, ctx: &State, event: &winit::event::WindowEvent) {
        match event {
            event::WindowEvent::MouseInput { state, .. } => match state {
                ElementState::Pressed {} => {
                    self.is_mouse_down = true;

                    let current_len = self.instance_points.len() as i32;

                    self.draw_commands.push(PathDrawCmd {
                        start_index: std::cmp::max(current_len, 0),
                        end_index: -1,
                    })
                }
                ElementState::Released {} => {
                    self.is_mouse_down = false;

                    let current_len = self.instance_points.len() as i32;

                    let last_cmd = self.draw_commands.last_mut().unwrap();
                    last_cmd.end_index = current_len - 1;
                }
            },
            event::WindowEvent::CursorMoved { position, .. } => {
                if self.is_mouse_down {
                    let mut ndc = glam::Vec2::new(0.0, 0.0);
                    ndc.x = (position.x as f32 / ctx.window_size.width_f32()) * 2.0 - 1.0;
                    ndc.y = ((ctx.window_size.height_f32() - position.y as f32)
                        / ctx.window_size.height_f32())
                        * 2.0
                        - 1.0;

                    let ndc = glam::Vec2::new(position.x as f32 / 2.0, position.y as f32 / 2.0);

                    println!("Moving: {:?}", ndc);
                    self.instance_points.push(ndc);

                    self.instance_buffer.destroy();
                    self.instance_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("instance point buffer"),
                        size: (std::mem::size_of::<glam::Vec2>() * self.instance_points.len())
                            as u64,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });

                    ctx.queue.write_buffer(
                        &self.instance_buffer,
                        0,
                        bytemuck::cast_slice(&self.instance_points),
                    )
                }
            }
            _ => (),
        }
    }

    fn render<'rpass>(&'rpass self, _state: &State, render_pass: &mut RenderPass<'rpass>) {
        let buffer_size = self.instance_buffer.size();
        let step_size = std::mem::size_of::<glam::Vec2>() as u64;

        if self.instance_points.len() <= 2 {
            // println!("Skiping");
            return;
        }

        render_pass.set_bind_group(0, &self.bind_group, &[0]);
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_vertex_buffer(0, self.buffer.slice(..));

        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(0..buffer_size - step_size));
        render_pass.set_vertex_buffer(2, self.instance_buffer.slice(step_size..buffer_size));
        for cmd in &self.draw_commands {
            // render_pass.draw_indexed(0..6, 0, 0..self.instance_points.len() as u32 - 1);

            let end_index = if cmd.end_index < 0 {
                self.instance_points.len() - 1
            } else {
                cmd.end_index as usize
            } as i32;

            println!("cmd: {} - {}", cmd.start_index, end_index);
            if cmd.start_index > end_index {
                continue;
            }

            render_pass.draw_indexed(0..6, 0, cmd.start_index as u32..end_index as u32);
        }

        //DRAW CIRCLE
        render_pass.set_bind_group(0, &self.bind_group, &[0]);
        render_pass.set_pipeline(&self.c_pipeline);
        render_pass.set_vertex_buffer(0, self.circle_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.draw(0..48, 0..self.instance_points.len() as u32 - 1);
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
