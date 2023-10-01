use crate::{
    pipelines::{self, shadeless, ModelUniform},
    state::State,
};
use wgpu;

#[derive(Debug, Clone, Copy)]
struct DrawCommand {
    start_vertex: usize,
    end_vertex: usize,
}

pub struct DrawContext {
    pipeline: shadeless::ShadelessPipeline,
    vertex_buffer: wgpu::Buffer,

    commands: Vec<DrawCommand>,
    vertices: Vec<shadeless::Vertex>,

    last_draw_command: DrawCommand,
    last_color: [f32; 3],

    pub perspective_matrix: glam::Mat4,
    pub view_matrix: glam::Mat4,
}

impl DrawContext {
    pub fn new(state: &State) -> Self {
        let window_size = state.window_size;

        let pipeline = shadeless::ShadelessPipeline::new_with_texture(
            state,
            &state.default_white_texture_bundle,
            wgpu::PrimitiveTopology::TriangleList,
            false,
        );

        Self {
            commands: Vec::new(),
            vertices: Vec::new(),
            last_draw_command: DrawCommand {
                start_vertex: 0,
                end_vertex: 0,
            },
            last_color: *glam::Vec3::ONE.as_ref(),
            view_matrix: glam::Mat4::IDENTITY,
            perspective_matrix: glam::Mat4::orthographic_lh(
                0.0,
                window_size.width_f32(),
                window_size.height_f32(),
                0.0,
                -1.0,
                1.0,
            ),

            pipeline,

            vertex_buffer: state.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Im mode vertex buffer"),
                size: 1024,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        }
    }

    pub fn start(&mut self) {
        self.commands = Vec::new();
        self.vertices = Vec::new();
        self.last_draw_command = DrawCommand {
            start_vertex: 0,
            end_vertex: 0,
        };
    }

    pub fn end(&mut self, state: &State) {
        let data = bytemuck::cast_slice(&self.vertices);

        let buffer_size = self.vertex_buffer.size() as usize;
        let data_size = std::mem::size_of::<shadeless::Vertex>() * self.vertices.len();
        if data_size > buffer_size {
            self.vertex_buffer.destroy();

            self.vertex_buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Im mode vertex buffer"),
                size: data_size.next_power_of_two() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        state.queue.write_buffer(&self.vertex_buffer, 0, data);

        pipelines::write_global_uniform_buffer(
            self.perspective_matrix * self.view_matrix.inverse(),
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
    }

    pub fn begin_shape(&mut self) {
        if self.vertices.len() != 0 {
            self.last_draw_command.start_vertex = self.vertices.len();
        }
    }
    pub fn end_shape(&mut self) {
        self.last_draw_command.end_vertex = self.vertices.len();
        self.commands.push(self.last_draw_command)
    }

    pub fn push_color(&mut self, r: f32, g: f32, b: f32) {
        self.last_color[0] = r;
        self.last_color[1] = g;
        self.last_color[2] = b;
    }

    pub fn push_color_slice(&mut self, color: &[f32; 3]) {
        self.last_color = *color;
    }

    pub fn push_vertex_slice(&mut self, pos: &[f32; 3]) {
        self.vertices.push(shadeless::Vertex {
            position: *pos,
            uv: [0.0, 0.0],
            color: self.last_color,
        })
    }
    pub fn push_vertex(&mut self, x: f32, y: f32, z: f32) {
        self.push_vertex_slice(&[x, y, z]);
    }

    pub fn push_circle(&mut self, x: f32, y: f32, radius: f32) {
        let vert_count = 16.0f32;
        let center = glam::vec2(x, y);

        let step = (1.0 / vert_count) * std::f32::consts::PI * 2.0;
        let mut current_step = 0.0;

        self.begin_shape();
        while current_step < std::f32::consts::PI * 2.0 {
            let x1 = (current_step.cos() * radius) + x;
            let y1 = (current_step.sin() * radius) + y;

            let next_step = current_step + step;
            let x2 = (next_step.cos() * radius) + x;
            let y2 = (next_step.sin() * radius) + y;

            self.push_vertex(x1, y1, 0.0);
            self.push_vertex(center.x, center.y, 0.0);
            self.push_vertex(x2, y2, 0.0);

            current_step += step;
        }

        self.end_shape();
    }

    pub fn push_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.begin_shape();
        self.push_vertex(x + width, y + height, 0.0);
        self.push_vertex(x + width, y, 0.0);
        self.push_vertex(x, y, 0.0);

        self.push_vertex(x + width, y + height, 0.0);
        self.push_vertex(x, y, 0.0);
        self.push_vertex(x, y + height, 0.0);

        self.end_shape();
    }

    pub fn push_line(&mut self, points: &[glam::Vec2], stroke_size: f32) {
        if points.len() < 2 {
            return;
        }

        self.begin_shape();

        let mut next_up = glam::Vec3::ZERO;
        let mut next_down = glam::Vec3::ZERO;

        for i in 0..points.len() - 1 {
            //TODO: can use a smarter algo here with vec2
            let point = glam::Vec3::from((points[i], 0.0));
            let next_point = glam::Vec3::from((points[i + 1], 0.0));
            let point_normal = (point - next_point).normalize();

            let a = point_normal.cross(glam::vec3(0.0, 0.0, 1.0)) * stroke_size;
            let b = -a;

            if i == 0 {
                next_up = point_normal + a;
                next_down = point_normal + b;
            }

            self.push_vertex(next_up.x, next_up.y, 0.0);
            self.push_vertex(next_point.x + b.x, next_point.y + b.y, 0.0);
            self.push_vertex(next_down.x, next_down.y, 0.0);

            self.push_vertex(next_point.x + b.x, next_point.y + b.y, 0.0);
            self.push_vertex(next_up.x, next_up.y, 0.0);
            self.push_vertex(next_point.x + a.x, next_point.y + a.y, 0.0);

            next_up = next_point + a;
            next_down = next_point + b;
        }
        self.end_shape();
    }

    pub fn draw<'rpass>(&'rpass self, render_pass: &mut wgpu::RenderPass<'rpass>) {
        render_pass.set_bind_group(0, &self.pipeline.bind_group, &[0, 0 as u32]);
        render_pass.set_bind_group(1, self.pipeline.texture_bind_group.as_ref().unwrap(), &[]);
        render_pass.set_pipeline(&self.pipeline.pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        for cmd in &self.commands {
            let start = cmd.start_vertex as u32;
            let end = cmd.end_vertex as u32;
            render_pass.draw(start..end, 0..1);
        }
    }
}
