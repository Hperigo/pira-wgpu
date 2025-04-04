use crate::egui::ahash::{HashMap, HashMapExt};
use crate::pipelines::ModelUniform;
use crate::wgpu;
use crate::{
    factories::{self},
    pipelines::{self, shadeless },
    state::State,
};

use crate::glam;

#[derive(Debug, Clone, Copy)]
struct DrawCommand {
    start_vertex: usize,
    end_vertex: usize,

    texture_id: Option<wgpu::Id<wgpu::TextureView>>,
    transform_id : usize,
    
    pipeline_index: usize,
}

pub struct DrawContext {
    pipelines: [shadeless::ShadelessPipeline; 2],

    vertex_buffer: wgpu::Buffer,

    commands: Vec<DrawCommand>,
    vertices: Vec<shadeless::Vertex>,

    textures: HashMap<wgpu::Id<wgpu::TextureView>, wgpu::BindGroup>,

    last_draw_command: DrawCommand,

    last_transform_index : usize,
    transform_matrices : [ModelUniform; 1024],

    last_color: [f32; 4],
    last_uv: [f32; 2],

    pub perspective_matrix: glam::Mat4,
    pub view_matrix: glam::Mat4,

}

impl DrawContext {
    pub fn new(state: &State) -> Self {
        let window_size = state.window_size;

        let tri_list_pipeline = shadeless::ShadelessPipeline::new_with_texture(
            state,
            &state.default_white_texture_bundle,
            wgpu::PrimitiveTopology::TriangleList,
            false,
        );

        let tri_strip_pipeline = shadeless::ShadelessPipeline::new_with_texture(
            state,
            &state.default_white_texture_bundle,
            wgpu::PrimitiveTopology::TriangleStrip,
            false,
        );

        let transform_matrices = [ ModelUniform::default(); 1024];
        // for i in 0..128  {
        //     transform_matrices[i] = glam::Mat4::from_translation(glam::vec3(i as f32 * 100.0, 0.0, 0.0));
        // }

        Self {
            commands: Vec::new(),
            vertices: Vec::new(),
            textures: HashMap::new(),

            last_draw_command: DrawCommand {
                start_vertex: 0,
                end_vertex: 0,
                texture_id: None,
                pipeline_index: 0,
                transform_id : 0
            },

            last_color: *glam::Vec4::ONE.as_ref(),
            last_uv: *glam::Vec2::ZERO.as_ref(),

            view_matrix: glam::Mat4::IDENTITY,
            perspective_matrix: glam::Mat4::orthographic_lh(
                0.0,
                window_size.width_f32(),
                window_size.height_f32(),
                0.0,
                -1.0,
                1.0,
            ),

            transform_matrices,
            last_transform_index: 0,

            pipelines: [tri_list_pipeline, tri_strip_pipeline],

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
        self.last_transform_index = 0;
        self.last_draw_command = DrawCommand {
            start_vertex: 0,
            end_vertex: 0,

            texture_id: None,
            pipeline_index: 0,
            transform_id: 0,
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



        for pip in &self.pipelines {
            pipelines::write_global_uniform_buffer(
                self.perspective_matrix * self.view_matrix.inverse(),
                pip.global_uniform_buffer.as_ref().unwrap(),
                &state.queue,
            );

            pipelines::write_uniform_buffer(
                &self.transform_matrices,
                &pip.model_uniform_buffer.as_ref().unwrap(),
                &state.queue,
                &state.device,
            );
        }
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
    pub fn push_color_alpha(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.last_color[0] = r;
        self.last_color[1] = g;
        self.last_color[2] = b;
        self.last_color[3] = a;
    }

    pub fn push_uv_slice(&mut self, uv: &[f32; 2]) {
        self.last_uv = *uv;
    }

    pub fn push_color_slice(&mut self, color: &[f32; 4]) {
        self.last_color = *color;
    }

    pub fn push_vertex_slice(&mut self, pos: &[f32; 3]) {
        self.vertices
            .push(shadeless::Vertex::new(*pos, self.last_uv, self.last_color))
    }
    pub fn push_vertex(&mut self, x: f32, y: f32, z: f32) {
        self.push_vertex_slice(&[x, y, z]);
    }

    pub fn push_texture(
        &mut self,
        device: &wgpu::Device,
        texture: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) {
        let id = match self.textures.get(&texture.global_id()) {
            Some(_) => texture.global_id(),
            None => {
                let (_, bind_group) = factories::BindGroupFactory::new()
                    .add_texture_and_sampler(wgpu::ShaderStages::VERTEX_FRAGMENT, texture, sampler)
                    .build(device);

                let id = texture.global_id(); // id 0 is the default white texture
                self.textures.insert(id, bind_group);
                id
            }
        };
        self.last_draw_command.texture_id = Some(id);
    }


    pub fn pop_texture(&mut self) {
        self.last_draw_command.texture_id = None;
    }

    pub fn set_transform(&mut self, t : glam::Mat4){ 
        
        self.last_transform_index += 1;
        self.transform_matrices[self.last_transform_index] = ModelUniform::new(t);
        self.last_draw_command.transform_id = self.last_transform_index;
    }

    pub fn clear_transform(&mut self){
        self.last_draw_command.transform_id = 0;
    }

    pub fn push_circle(&mut self, x: f32, y: f32, radius: f32) {
        let vert_count = 16.0f32;
        let center = glam::vec2(x, y);

        let step = (1.0 / vert_count) * std::f32::consts::PI * 2.0;
        let mut current_step = 0.0;

        self.begin_shape();
        self.last_draw_command.pipeline_index = 0;
        // self.last_draw_command.pi
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

    pub fn push_circle_stroke(&mut self, x: f32, y: f32, radius: f32) {
        // let vert_count = 16.0f32;
        const VERT_COUNT: usize = 17;
        self.last_draw_command.pipeline_index = 0;

        // let step = (1.0 / vert_count) * std::f32::consts::PI * 2.0;
        // let mut current_step = 0.0;

        let mut points: [glam::Vec2; VERT_COUNT] = [glam::Vec2::ZERO; VERT_COUNT];

        for i in 0..VERT_COUNT {
            let t = (i as f32 / (VERT_COUNT - 1) as f32) * std::f32::consts::PI * 2.0;

            let x1 = (t.cos() * radius) + x;
            let y1 = (t.sin() * radius) + y;

            points[i] = glam::vec2(x1, y1);
        }

        self.push_line(&points, 2.0);
    }

    pub fn push_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.begin_shape();
        self.last_draw_command.pipeline_index = 0;

        // first triangle
        self.push_uv_slice(&[1.0, 1.0]);
        self.push_vertex(x + width, y + height, 0.0);

        self.push_uv_slice(&[1.0, 0.0]);
        self.push_vertex(x + width, y, 0.0);

        self.push_uv_slice(&[0.0, 0.0]);
        self.push_vertex(x, y, 0.0);

        // second triangle
        self.push_uv_slice(&[1.0, 1.0]);
        self.push_vertex(x + width, y + height, 0.0);

        self.push_uv_slice(&[0.0, 0.0]);
        self.push_vertex(x, y, 0.0);

        self.push_uv_slice(&[0.0, 1.0]);
        self.push_vertex(x, y + height, 0.0);

        self.end_shape();
    }

    pub fn set_draw_mode(&mut self, primitive: wgpu::PrimitiveTopology) {
        match primitive {
            wgpu::PrimitiveTopology::TriangleList => self.last_draw_command.pipeline_index = 0,
            wgpu::PrimitiveTopology::TriangleStrip => self.last_draw_command.pipeline_index = 1,
            _ => {
                todo!("Not implemented yet");
            }
        }
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
                next_up = point_normal + a + point;
                next_down = point_normal + b + point;
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
        self.last_draw_command.pipeline_index = 1;
        self.end_shape();
    }

    pub fn draw<'rpass>(&'rpass self, state: &State, render_pass: &mut wgpu::RenderPass<'rpass>) {
        let uniform_alignment = state.device.limits().min_uniform_buffer_offset_alignment as u32;
        
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        let mut prev_pipeline = None;
        for cmd in self.commands.iter() {
            let pip = &self.pipelines[cmd.pipeline_index];

            // Only set the pipeline if we actualy need to..
            if let Some(prev_index) = prev_pipeline{
                if cmd.pipeline_index !=  prev_index{
                    render_pass.set_pipeline(&pip.pipeline);                
                }
            }else{
                render_pass.set_pipeline(&pip.pipeline);
            }

            // pipelines::write_uniform_buffer(data, buffer, queue, device); pip.model_uniform_buffer
            let offset = cmd.transform_id as u32 * uniform_alignment as wgpu::DynamicOffset;
            render_pass.set_bind_group(0, &pip.bind_group, &[0, offset]);

            match cmd.texture_id {
                Some(id) => {
                    let bind_group = self.textures.get(&id).unwrap();
                    render_pass.set_bind_group(1, bind_group, &[]);
                }
                None => {
                   render_pass.set_bind_group(1, pip.texture_bind_group.as_ref().unwrap(), &[]);
                }
            }

            let start = cmd.start_vertex as u32;
            let end = cmd.end_vertex as u32;
            render_pass.draw(start..end, 0..1);

            prev_pipeline = Some(cmd.pipeline_index);
        }
    }
}
