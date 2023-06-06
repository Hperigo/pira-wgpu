use std::borrow::Cow;

use wgpu::util::DeviceExt;
use wgpu::RenderPass;
use wgpu_app_lib::framework::Application;
use wgpu_app_lib::wgpu_helper::factories::RenderPipelineFactory;
use wgpu_app_lib::wgpu_helper::{BindGroupFactory, State};
use wgpu_app_lib::{framework, wgpu_helper};

const SHADER_SRC: &'static str = " 

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

/*
@group(0) @binding(2)
var t_diffuse: texture_2d<f32>;
@group(0)@binding(3)
var s_diffuse: sampler;
*/

@vertex
fn vs_main( model : VertexInput ) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4(model.position, 1.0);
    out.color = model.color;
    return out;
}

@fragment
fn fs_main(in : VertexOutput) -> @location(0) vec4<f32> {
    return vec4(1.0, 1.0, 1.0, 1.0) * vec4(in.color, 1.0);
}
";

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

struct MyExample {
    buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
}

impl Application for MyExample {
    fn init(state: &wgpu_helper::State) -> Self {
        let vertices = vec![
            Vertex {
                position: [0.0, 0.0, 0.0],
                color: [1.0, 0.0, 0.0],
            },
            Vertex {
                position: [1.0, 1.0, 0.0],
                color: [0.0, 1.0, 0.0],
            },
            Vertex {
                position: [1.0, 0.0, 0.0],
                color: [0.0, 0.0, 1.0],
            },
            Vertex {
                position: [0.0, 1.0, 0.0],
                color: [1.0, 1.0, 1.0],
            },
        ];
        let mut indices: [u16; 6] = [0, 1, 2, 0, 3, 1];
        indices.reverse();

        let shader_module = state
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER_SRC)),
            });

        let (bind_group_layout, bind_group) = BindGroupFactory::new().build(&state.device);

        let attribs = wgpu::vertex_attr_array![ 0 => Float32x3, 1 => Float32x3 ];
        let stride = std::mem::size_of::<Vertex>() as u64;
        let mut pipeline_factory = RenderPipelineFactory::new();
        pipeline_factory.add_vertex_attributes(&attribs, stride);
        // // .add_instance_attributes(&instance_attribs, std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress)
        pipeline_factory.add_depth_stencil();

        let pipeline =
            pipeline_factory.create_render_pipeline(&state, &shader_module, &[&bind_group_layout]);

        MyExample {
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

            bind_group,
            pipeline,
        }
    }

    fn clear_color(&self) -> wgpu::Color {
        wgpu::Color {
            r: 0.5,
            g: 0.15,
            b: 0.3,
            a: 1.0,
        }
    }

    fn update(&mut self, _state: &State, _frame_count: u64, _delta_time: f64) {}

    fn resize(
        &mut self,
        _config: &wgpu::SurfaceConfiguration,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) {
    }

    fn render<'rpass>(
        &'rpass self,
        _state: &wgpu_helper::State,
        render_pass: &mut RenderPass<'rpass>,
    ) {
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_vertex_buffer(0, self.buffer.slice(..));
        render_pass.draw_indexed(0..6, 0, 0..1);
    }
}

fn main() {
    framework::run::<MyExample>("title");
}
