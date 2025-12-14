#![allow(dead_code)]
#![allow(unused_variables)]

use image::EncodableLayout;
use pira_wgpu::factories::texture::{SamplerOptions, Texture2dOptions};
use pira_wgpu::framework::{self, Application};
use pira_wgpu::helpers::geometry::attribute_names;
use pira_wgpu::state::State;
use pira_wgpu::{factories, pipelines};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BufferUsages, MultisampleState, PipelineLayoutDescriptor, PrimitiveState, ShaderStages,
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
    out.clip_position = vec4(vertex_position - vec3<f32>(0.0, 0.0, 0.5), 1.0);
    out.cube_coords = vertex_position;

    return out;
}

struct Uniforms {
    @location(2) rotation_matrix: mat4x4<f32>,
    @location(3) exposure: f32,
};


@group(0) @binding(0)
var hdr_texture: texture_2d<f32>;
@group(0) @binding(1)
var hdr_sampler: sampler;

@group(0) @binding(2)
var<uniform> uniform : Uniforms;


// Rotation matrix around the X axis.
fn rotate_x(theta : f32) -> mat3x3<f32> {
    var c = cos(theta);
    var s = sin(theta);
    return mat3x3<f32>(
        vec3(1.0, 0.0, 0.0),
        vec3(0.0, c, -s),
        vec3(0.0, s, c)
    );
}

const invAtan : vec2<f32> = vec2<f32>(0.1591, 0.3183);
fn sample_spherical_map(v : vec3<f32>) -> vec2<f32>
{
    var uv = vec2<f32>(atan2(v.z, v.x), asin(v.y));
    uv *= invAtan;
    uv += 0.5;
    return uv;
}

// Maps HDR values to linear values
// Based on http://www.oscars.org/science-technology/sci-tech-projects/aces
fn aces_tone_map(hdr: vec3<f32>) -> vec3<f32> {
    let m1 = mat3x3(
        0.59719, 0.07600, 0.02840,
        0.35458, 0.90834, 0.13383,
        0.04823, 0.01566, 0.83777,
    );
    let m2 = mat3x3(
        1.60475, -0.10208, -0.00327,
        -0.53108,  1.10813, -0.07276,
        -0.07367, -0.00605,  1.07602,
    );
    let v = m1 * hdr;
    let a = v * (v + 0.0245786) - 0.000090537;
    let b = v * (0.983729 * v + 0.4329510) + 0.238081;
    return clamp(m2 * (a / b), vec3(0.0), vec3(1.0));
}


@fragment
fn fs_main(in : VertexOutput) -> @location(0) vec4<f32> {

    
    var spherical_coord = normalize(uniform.rotation_matrix * vec4(in.cube_coords * vec3(1.0, -1.0, 1.0), 0.0));
    var cube_uv = sample_spherical_map(spherical_coord.xyz);

    var texture_color = textureLoad(hdr_texture, vec2<i32>(cube_uv * vec2<f32>(1024.0, 512.0)), 0).rgb;

    var gamma = 2.2;
    var mapped = texture_color / (texture_color + vec3(1.0));
    mapped = vec3(1.0) - exp(-texture_color * uniform.exposure); //pow(mapped, vec3(1.0 / gamma));
    return vec4(mapped, 1.0);  //vec4(aces_tone_map(texture_color + uniform.exposure) , 1.0);
 }
";

#[repr(C, align(16))]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniform {
    pub rotation_matrix: [f32; 16],
    pub exposure: f32,
    _pad: [f32; 3],
}

struct Sky {
    textures: [pira_wgpu::factories::texture::TextureBundle; 6],
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
}

fn create_cube_map_from_equi() {}

struct MyExample {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    // pipeline: shadeless::ShadelessPipeline,
    pipeline: wgpu::RenderPipeline,

    bind_group: wgpu::BindGroup,

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

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SHADER_SRC)),
        });

        // let render_target = device.create_texture(&wgpu::TextureDescriptor {
        //     label: None,
        //     size: wgpu::Extent3d {
        //         width: TEXTURE_DIMS.0 as u32,
        //         height: TEXTURE_DIMS.1 as u32,
        //         depth_or_array_layers: 1,
        //     },
        //     mip_level_count: 1,
        //     sample_count: 1,
        //     dimension: wgpu::TextureDimension::D2,
        //     format: wgpu::TextureFormat::Bgra8UnormSrgb,
        //     usage: wgpu::TextureUsages::RENDER_ATTACHMENT
        //         | wgpu::TextureUsages::COPY_SRC
        //         | wgpu::TextureUsages::TEXTURE_BINDING,
        //     view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
        // });

        let image = image::open(
            "assets/buikslotermeerplein_1k.exr"
            // "assets/rusty.png",
            // "/Users/henrique/Documents/dev/rust/pira-wgpu/assets/cubemap-equi.png",
        )
        .unwrap()
        .to_rgba32f();

        let px = image.get_pixel(200, 200);
        println!("{:?}", px);

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

        let rotation_matrix_buffer = glam::Mat4::IDENTITY;
        let uniform: Uniform = Uniform {
            rotation_matrix: *rotation_matrix_buffer.as_ref(),
            exposure: 1.0,
            _pad: [0.0, 0.0, 0.0],
        };

        let uniform_buffer = pipelines::create_uniform_buffer::<Uniform>(1, device);
        pipelines::write_uniform_buffer(&[uniform], &uniform_buffer, queue, device);

        let (bind_group_layout, bind_group) = factories::BindGroupFactory::new()
            .add_texture_hdr_and_sampler(
                wgpu::ShaderStages::FRAGMENT,
                &env_texture_bundle.view,
                &env_texture_bundle.sampler,
                wgpu::SamplerBindingType::Filtering,
            )
            .add_uniform(ShaderStages::FRAGMENT, &uniform_buffer, None)
            .build(device);

        let pipeline_layout = PipelineLayoutDescriptor {
            label: Some("Equirectangular-pipeline"),
            bind_group_layouts: &[&bind_group_layout],
            ..Default::default()
        };

        let mut cube = pira_wgpu::helpers::geometry::cube::Cube::new(1.0);

        let vertices = cube
            .geometry
            .attributes
            .get(&attribute_names::POSITION)
            .unwrap();

        let indices = &mut cube.geometry.indices;
        // indices.reverse();
        println!("{}", indices.len());

        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Cube index buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
        });

        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Cube vertex buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        let vertex_attrib = wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 0,
            shader_location: 0,
        };

        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: 12,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![ 0 => Float32x3],
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&device.create_pipeline_layout(&pipeline_layout)),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_buffer_layout],
                compilation_options : wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::TextureFormat::Bgra8UnormSrgb.into())],
                // compilation_options : ,
                compilation_options : wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState {
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Less, // 1.
                stencil: wgpu::StencilState::default(),     // 2.
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: 4,
                ..Default::default()
            },
            multiview: None,
            cache : None,
        });

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            bind_group,
            rotation: glam::Vec3::ZERO,
            exposure: 1.0,
            uniform_buffer,
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
            rotation_matrix: *rotation_matrix_buffer.as_ref(),
            exposure: self.exposure,
            _pad: [0.0, 0.0, 0.0],
        };

        pipelines::write_uniform_buffer(&[uniform], &self.uniform_buffer, queue, device);

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
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[0]);
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw_indexed(0..36, 0, 0..1);

        return;

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
