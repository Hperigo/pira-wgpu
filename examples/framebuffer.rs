#![allow(dead_code)]
#![allow(unused_variables)]

use std::num::NonZeroU64;
use std::os::macos::raw::stat;

use egui::epaint::text;
use glam::Vec3;
use image::{EncodableLayout, ImageBuffer};
use pira_wgpu::factories::texture::{SamplerOptions, Texture2dOptions, TextureBundle};
use pira_wgpu::framework::{self, Application};
use pira_wgpu::helpers::geometry::attribute_names;
use pira_wgpu::pipelines::{shadeless, ModelUniform};
use pira_wgpu::state::State;
use pira_wgpu::{factories, pipelines};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BufferBinding, BufferUsages, MultisampleState, PipelineLayoutDescriptor, PrimitiveState,
    RenderPipelineDescriptor, SamplerDescriptor, ShaderModuleDescriptor, ShaderStages, Texture,
    TextureViewDescriptor,
};
use winit::dpi::PhysicalSize;

const TEXTURE_DIMS: (usize, usize) = (512, 512);

const SHADER_SRC: &'static str = " 
struct VertexOutput {
    @builtin(position) clip_position  : vec4<f32>,
    @location(0) cube_coords : vec3<f32>,
}
@vertex
fn vs_main(@location(0) vertex_position: vec3<f32>) -> VertexOutput {
    var out : VertexOutput;
    out.clip_position = vec4(vertex_position, 1.0);
    out.cube_coords = vertex_position;

    return out;
}

struct Uniforms {
    @location(2) view_direction: vec3<f32>,
    @location(3) exposure: f32,
};


@group(0) @binding(0)
var env_map: texture_cube<f32>;
@group(0) @binding(1)
var env_sampler: sampler;

@group(0) @binding(2)
var<uniform> uniform : Uniforms;

@fragment
fn fs_main(in : VertexOutput) -> @location(0) vec4<f32> {

    var ray_direction = normalize(uniform.view_direction);
    var sample = textureSample(env_map, env_sampler, ray_direction);
    return sample;
 }
";

const COMPUTE_SHADER: &'static str = "

const PI: f32 = 3.1415926535897932384626433832795;

struct Face {
    forward: vec3<f32>,
    up: vec3<f32>,
    right: vec3<f32>,
}

@group(0)
@binding(0)
var src: texture_2d<f32>;

@group(0)
@binding(1)
var dst: texture_storage_2d_array<rgba32float, write>;


@compute
@workgroup_size(16, 16, 1)
fn compute_equirect_to_cubemap(
    @builtin(global_invocation_id)
    gid: vec3<u32>,
) {
    // If texture size is not divisible by 32, we
    // need to make sure we don't try to write to
    // pixels that don't exist.
    if gid.x >= u32(textureDimensions(dst).x) {
        return;
    }

    var FACES: array<Face, 6> = array(
        // FACES +X
        Face(
            vec3(1.0, 0.0, 0.0),  // forward
            vec3(0.0, 1.0, 0.0),  // up
            vec3(0.0, 0.0, -1.0), // right
        ),
        // FACES -X
        Face (
            vec3(-1.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
            vec3(0.0, 0.0, 1.0),
        ),
        // FACES +Y
        Face (
            vec3(0.0, -1.0, 0.0),
            vec3(0.0, 0.0, 1.0),
            vec3(1.0, 0.0, 0.0),
        ),
        // FACES -Y
        Face (
            vec3(0.0, 1.0, 0.0),
            vec3(0.0, 0.0, -1.0),
            vec3(1.0, 0.0, 0.0),
        ),
        // FACES +Z
        Face (
            vec3(0.0, 0.0, 1.0),
            vec3(0.0, 1.0, 0.0),
            vec3(1.0, 0.0, 0.0),
        ),
        // FACES -Z
        Face (
            vec3(0.0, 0.0, -1.0),
            vec3(0.0, 1.0, 0.0),
            vec3(-1.0, 0.0, 0.0),
        ),
    );

    // Get texture coords relative to cubemap face
    let dst_dimensions = vec2<f32>(textureDimensions(dst));
    let cube_uv = vec2<f32>(gid.xy) / dst_dimensions * 2.0 - 1.0;

    // Get spherical coordinate from cube_uv
    let face = FACES[gid.z];
    let spherical = normalize(face.forward + face.right * cube_uv.x + face.up * cube_uv.y);

    // Get coordinate on the equirectangular texture
    let inv_atan = vec2(0.1591, 0.3183);
    let eq_uv = vec2(atan2(spherical.z, spherical.x), asin(spherical.y)) * inv_atan + 0.5;
    let eq_pixel = vec2<i32>(eq_uv * vec2<f32>(textureDimensions(src)));

    // We use textureLoad() as textureSample() is not allowed in compute shaders
    var sample = textureLoad(src, eq_pixel, 0);

    textureStore(dst, gid.xy, gid.z, sample);
}
";

#[repr(C, align(16))]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniform {
    pub ray_direction: [f32; 3],
    pub exposure: f32,
}

struct Sky {
    textures: [pira_wgpu::factories::texture::TextureBundle; 6],
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
}

fn create_cube_map_from_equi(
    state: &pira_wgpu::state::State,
    label: Option<&str>,
    image: &image::ImageBuffer<image::Rgba<f32>, Vec<f32>>,
) -> TextureBundle {
    let State { device, queue, .. } = state;

    let dst_size = 512;
    // This will be the result bundle image
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label,
        size: wgpu::Extent3d {
            width: 512,
            height: 512,
            depth_or_array_layers: 6,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    });

    let cube_view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("SKy texture view"),
        dimension: Some(wgpu::TextureViewDimension::Cube),
        array_layer_count: Some(6),
        ..Default::default()
    });

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("SKy texture sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    // Run compute shader -----------
    let shader_module = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Equirectangular to cubemap compute"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(COMPUTE_SHADER)),
    });

    //create equirectangular texture
    let env_texture_bundle = factories::Texture2dFactory::new_with_options(
        state,
        [image.width(), image.height()],
        Texture2dOptions {
            format: wgpu::TextureFormat::Rgba32Float,
            ..Default::default()
        },
        SamplerOptions {
            filter: wgpu::FilterMode::Nearest,
            address_mode: wgpu::AddressMode::Repeat,
            mipmap_filter: wgpu::FilterMode::Nearest,
        },
        &image.as_bytes(),
    );

    let compute_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("HDR: equirect layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: wgpu::TextureFormat::Rgba32Float,
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                },
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&compute_layout],
        push_constant_ranges: &[],
    });

    let equirect_to_cubemap = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("equirect_to_cubemap"),
        layout: Some(&pipeline_layout),
        module: &shader_module,
        entry_point: "compute_equirect_to_cubemap",
    });

    let dst_view = texture.create_view(&TextureViewDescriptor {
        label,
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label,
        layout: &compute_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&env_texture_bundle.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&dst_view),
            },
        ],
    });

    let mut encoder = device.create_command_encoder(&Default::default());
    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label,
        timestamp_writes: None,
    });

    let num_workgroups = (dst_size + 15) / 16;
    pass.set_pipeline(&equirect_to_cubemap);
    pass.set_bind_group(0, &bind_group, &[]);
    pass.dispatch_workgroups(num_workgroups, num_workgroups, 6);

    drop(pass);

    queue.submit([encoder.finish()]);

    return TextureBundle {
        sampler,
        view: cube_view,
        texture,
    };
}

struct MyExample {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    // pipeline: shadeless::ShadelessPipeline,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    // bind_group: wgpu::BindGroup,
    rotation: glam::Vec3,
    exposure: f32,
    uniform_buffer: wgpu::Buffer,
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

        let image = image::open(
            "/Users/henrique/Documents/dev/rust/pira-wgpu/assets/buikslotermeerplein_1k.exr",
            // "/Users/henrique/Documents/dev/rust/pira-wgpu/assets/cubemap-equi.png",
        )
        .unwrap()
        .to_rgba32f();

        let sky_texture_bundle = create_cube_map_from_equi(state, Some("Sky"), &image);

        //-----------------------------------------------
        let vertices = vec![
            shadeless::Vertex::new([-1.0, -1.0, 0.0], [0.0, 0.0], [1.0, 1.0, 1.0, 1.0]),
            shadeless::Vertex::new([1.0, 1.0, 0.0], [1.0, 1.0], [1.0, 1.0, 1.0, 1.0]),
            shadeless::Vertex::new([1.0, -1.0, 0.0], [1.0, 0.0], [1.0, 1.0, 1.0, 1.0]),
            shadeless::Vertex::new([-1.0, 1.0, 0.0], [0.0, 1.0], [1.0, 1.0, 1.0, 1.0]),
        ];

        let indices: [u16; 6] = [0, 1, 2, 0, 3, 1];

        let uniform_buffer = pipelines::create_uniform_buffer::<Uniform>(1, device);

        let environment_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("environment_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::Cube,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let environment_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("environment_bind_group"),
            layout: &environment_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&sky_texture_bundle.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&device.create_sampler(
                        &SamplerDescriptor {
                            ..Default::default()
                        },
                    )),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(BufferBinding {
                        buffer: &uniform_buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new(std::mem::size_of::<Uniform>() as _),
                    }),
                },
            ],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sky Pipeline Layout"),
            bind_group_layouts: &[&environment_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Sky"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SHADER_SRC)),
        });

        let vertex_attribs = shadeless::ShadelessPipeline::get_vertex_attrib_layout_array();

        let mut sky_render_pipeline = factories::RenderPipelineFactory::new();
        sky_render_pipeline.add_vertex_attributes(
            &vertex_attribs,
            shadeless::ShadelessPipeline::get_array_stride(),
        );
        sky_render_pipeline.set_cull_mode(None);

        let pipeline =
            sky_render_pipeline.create_render_pipeline(state, &shader, &[&environment_layout]);

        Self {
            pipeline,
            bind_group: environment_bind_group,
            vertex_buffer: state
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
            uniform_buffer,
            rotation: Vec3::ZERO,
            exposure: 0.0,
        }
    }

    fn clear_color(&self) -> wgpu::Color {
        wgpu::Color::GREEN
    }

    fn event(&mut self, state: &State, _event: &winit::event::WindowEvent) {}

    fn update(&mut self, state: &State, frame_count: u64, delta_time: f64) {
        let State { device, queue, .. } = state;

        let rotation_matrix_buffer = glam::Mat4::from_euler(
            glam::EulerRot::XYZ,
            self.rotation.x,
            self.rotation.y,
            self.rotation.z,
        );

        let uniform: Uniform = Uniform {
            ray_direction: *self.rotation.as_ref(),
            exposure: self.exposure,
        };

        pipelines::write_uniform_buffer(&[uniform], &self.uniform_buffer, queue, device);

        // queue.write_buffer(
        //     &self.uniform_buffer,
        //     0,
        //     bytemuck::cast_slice(rotation_matrix_buffer.as_ref()),
        // );
    }

    fn on_gui(&mut self, egui_ctx: &mut framework::EguiLayer) {
        egui::SidePanel::new(egui::panel::Side::Left, "Debug").show(&egui_ctx.ctx, |ui| {
            ui.drag_angle(&mut self.rotation.x);
            ui.drag_angle(&mut self.rotation.y);
            ui.drag_angle(&mut self.rotation.z);

            ui.spacing();

            ui.add(egui::DragValue::new(&mut self.exposure).speed(0.01));
        });
    }

    fn render<'rpass>(&'rpass self, state: &State, render_pass: &mut wgpu::RenderPass<'rpass>) {
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        //render_pass.draw(0..6, 0..1);

        render_pass.draw_indexed(0..6, 0, 0..1);

        // let ortho_perspective_matrix = glam::Mat4::IDENTITY;
        // pipelines::write_global_uniform_buffer(
        //     ortho_perspective_matrix,
        //     self.pipeline.global_uniform_buffer.as_ref().unwrap(),
        //     &state.queue,
        // );

        // let matrices = [ModelUniform {
        //     model_matrix: glam::Mat4::IDENTITY,
        // }];

        // pipelines::write_uniform_buffer(
        //     &matrices,
        //     &self.pipeline.model_uniform_buffer.as_ref().unwrap(),
        //     &state.queue,
        //     &state.device,
        // );

        // render_pass.set_bind_group(0, &self.pipeline.bind_group, &[0, 0 as u32]);
        // render_pass.set_bind_group(1, self.pipeline.texture_bind_group.as_ref().unwrap(), &[]);
        // render_pass.set_pipeline(&self.pipeline.pipeline);
        // render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        // render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        // render_pass.draw_indexed(0..6, 0, 0..1);
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
