#![allow(dead_code)]
#![allow(unused_variables)]

use image::EncodableLayout;
use pira_wgpu::factories::texture::{SamplerOptions, Texture2dOptions, TextureBundle};
use pira_wgpu::factories::{self, bind_group};
use pira_wgpu::framework::{self, Application};
use pira_wgpu::pipelines::{self, shadeless, ModelUniform};
use pira_wgpu::state::State;
use wgpu::util::DeviceExt;
use wgpu::{
    BindGroup, BindGroupLayoutDescriptor, PipelineLayout, PipelineLayoutDescriptor,
    SamplerDescriptor,
};
use winit::dpi::PhysicalSize;

const TEXTURE_DIMS: (usize, usize) = (512, 512);

const SHADER_SRC: &'static str = " 
struct VertexOutput {
    @builtin(position) clip_position  : vec4<f32>,
    @location(0) cube_coords : vec3<f32>,
}
@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var vertices = array<vec4<f32>, 6>(
        vec4<f32>(-1.0, 1.0, 0.0, 1.0),
        vec4<f32>(-1.0, -1.0, 0.0, 1.0),
        vec4<f32>(1.0, -1.0, 0.0, 1.0),

        vec4<f32>(1.0, 1.0, 0.0, 1.0),
        vec4<f32>(-1.0, 1.0, 0.0, 1.0),
        vec4<f32>(1.0, -1.0, 0.0, 1.0)
    );

    var cube_coords = array<vec3<f32>, 6>(
        vec3<f32>(-1.0, 1.0, -1.0 ),
        vec3<f32>(-1.0, -1.0, -1.0 ),
        vec3<f32>(1.0, -1.0, -1.0 ),

        vec3<f32>(1.0, 1.0, -1.0 ),
        vec3<f32>(-1.0, 1.0, -1.0),
        vec3<f32>(1.0, -1.0, -1.0 )
    );

    var out : VertexOutput;
    out.clip_position = vertices[in_vertex_index];
    out.cube_coords = cube_coords[in_vertex_index];

    return out;
}

@group(0) @binding(0)
var hdr_texture: texture_2d<f32>;
@group(0) @binding(1)
var hdr_sampler: sampler;


// Rotation matrix around the X axis.
fn rotateX(theta : f32) -> mat3x3<f32> {
    var c = cos(theta);
    var s = sin(theta);
    return mat3x3<f32>(
        vec3(1.0, 0.0, 0.0),
        vec3(0.0, c, -s),
        vec3(0.0, s, c)
    );
}

const invAtan : vec2<f32> = vec2<f32>(0.1591, 0.3183);
fn SampleSphericalMap(v : vec3<f32>) -> vec2<f32>
{
    var uv = vec2<f32>(atan2(v.z, v.x), asin(v.y));
    uv *= invAtan;
    uv += 0.5;
    return uv;
}

@fragment
fn fs_main(in : VertexOutput) -> @location(0) vec4<f32> {

    var spherical_coord = normalize( rotateX(3.14) * in.cube_coords);
    var cube_uv = SampleSphericalMap(spherical_coord);

    var texture_color = textureLoad(hdr_texture, vec2<i32>(cube_uv * vec2<f32>(1024.0, 512.0)), 0);
    return texture_color;
 }
";

struct MyExample {
    buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    pipeline: shadeless::ShadelessPipeline,
}

impl Application for MyExample {
    fn init(state: &State) -> Self {
        let State {
            instance,
            adapter,
            device,
            queue,
            config,
            window_surface,
            depth_texture,
            default_white_texture_bundle,
            window_size,
            delta_time,
            sample_count,
        } = state;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SHADER_SRC)),
        });

        let render_target = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: TEXTURE_DIMS.0 as u32,
                height: TEXTURE_DIMS.1 as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
        });

        let image = image::open("./assets/studio_small_03_1k.hdr")
            .unwrap()
            .to_rgba32f();

        let env_texture_bundle = factories::Texture2dFactory::new_with_options(
            state,
            [image.width(), image.height()],
            Texture2dOptions {
                format: wgpu::TextureFormat::Rgba32Float,
                ..Default::default()
            },
            SamplerOptions {
                filter: wgpu::FilterMode::Linear,
                ..Default::default()
            },
            &image.as_bytes(),
        );

        let (bind_group_layout, bind_group) = factories::BindGroupFactory::new()
            .add_texture_hdr_and_sampler(
                wgpu::ShaderStages::FRAGMENT,
                &env_texture_bundle.view,
                &env_texture_bundle.sampler,
            )
            .build(device);

        let pipeline_layout = PipelineLayoutDescriptor {
            label: Some("Equirectangular-pipeline"),
            bind_group_layouts: &[&bind_group_layout],
            ..Default::default()
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&device.create_pipeline_layout(&pipeline_layout)),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::TextureFormat::Rgba8UnormSrgb.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        //-----------------------------------------------

        let texture_view = render_target.create_view(&wgpu::TextureViewDescriptor::default());

        let mut command_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }
        queue.submit(Some(command_encoder.finish()));

        //-----------------------------------------------

        let vertices = vec![
            shadeless::Vertex::new([-0.8, -0.8, 0.0], [0.0, 0.0], [1.0, 1.0, 1.0, 1.0]),
            shadeless::Vertex::new([0.8, 0.8, 0.0], [1.0, 1.0], [1.0, 1.0, 1.0, 1.0]),
            shadeless::Vertex::new([0.8, -0.8, 0.0], [1.0, 0.0], [1.0, 1.0, 1.0, 1.0]),
            shadeless::Vertex::new([-0.8, 0.8, 0.0], [0.0, 1.0], [1.0, 1.0, 1.0, 1.0]),
        ];

        let mut indices: [u16; 6] = [0, 1, 2, 0, 3, 1];
        indices.reverse();

        let texture_bundle = TextureBundle {
            texture: render_target,
            view: texture_view,
            sampler: device.create_sampler(&SamplerDescriptor {
                ..Default::default()
            }),
        };

        let pipeline = shadeless::ShadelessPipeline::new_with_texture(
            state,
            &texture_bundle,
            wgpu::PrimitiveTopology::TriangleList,
            true,
        );

        Self {
            pipeline,
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
        }
    }

    fn clear_color(&self) -> wgpu::Color {
        wgpu::Color::GREEN
    }

    fn event(&mut self, state: &State, _event: &winit::event::WindowEvent) {}

    fn update(&mut self, state: &State, frame_count: u64, delta_time: f64) {}

    fn render<'rpass>(&'rpass self, state: &State, render_pass: &mut wgpu::RenderPass<'rpass>) {
        let ortho_perspective_matrix = glam::Mat4::IDENTITY;
        pipelines::write_global_uniform_buffer(
            ortho_perspective_matrix,
            self.pipeline.global_uniform_buffer.as_ref().unwrap(),
            &state.queue,
        );

        let matrices = [ModelUniform {
            model_matrix: glam::Mat4::IDENTITY,
        }];

        pipelines::write_uniform_buffer(
            &matrices,
            &self.pipeline.model_uniform_buffer.as_ref().unwrap(),
            &state.queue,
            &state.device,
        );

        render_pass.set_bind_group(0, &self.pipeline.bind_group, &[0, 0 as u32]);
        render_pass.set_bind_group(1, self.pipeline.texture_bind_group.as_ref().unwrap(), &[]);
        render_pass.set_pipeline(&self.pipeline.pipeline);
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_vertex_buffer(0, self.buffer.slice(..));
        render_pass.draw_indexed(0..6, 0, 0..1);
    }
}

fn main() {
    framework::run::<MyExample>(
        "framebuffer",
        PhysicalSize {
            width: 1000,
            height: 1000,
        },
        4,
    );
}
