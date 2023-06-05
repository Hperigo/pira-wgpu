use crate::wgpu_helper::State;
use wgpu::{PrimitiveTopology, ShaderModule};

pub struct RenderPipelineFactory<'a> {
    vertex_buffer_layouts: Vec<wgpu::VertexBufferLayout<'a>>,
    depth_config: Option<wgpu::DepthStencilState>,

    vert_shader_entry: &'a str,
    frag_shader_entry: &'a str,

    topology: PrimitiveTopology,

    label: Option<&'static str>,
}

impl<'a> RenderPipelineFactory<'a> {
    pub fn new() -> Self {
        RenderPipelineFactory {
            vertex_buffer_layouts: Vec::new(),
            depth_config: None,

            vert_shader_entry: Self::default_vert_entry_point(),
            frag_shader_entry: Self::default_frag_entry_point(),
            topology: PrimitiveTopology::TriangleList,
            label: Some("Pipeline from helper"),
        }
    }

    pub fn set_label(&mut self, label: &'static str) {
        self.label = Some(label);
    }

    pub fn add_vertex_attributes(
        &mut self,
        attribs: &'a [wgpu::VertexAttribute],
        stride: wgpu::BufferAddress,
    ) {
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: stride,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: attribs,
        };
        self.vertex_buffer_layouts.push(vertex_layout);
    }

    pub fn add_instance_attributes(
        &mut self,
        attribs: &'a [wgpu::VertexAttribute],
        stride: wgpu::BufferAddress,
    ) {
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: stride,
            attributes: attribs,
            step_mode: wgpu::VertexStepMode::Instance,
        };
        self.vertex_buffer_layouts.push(vertex_layout);
    }

    pub fn add_depth_stencil(&mut self) {
        self.depth_config = Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less, // 1.
            stencil: wgpu::StencilState::default(),     // 2.
            bias: wgpu::DepthBiasState::default(),
        });
    }

    pub fn set_vert_entry(&mut self, name: &'a str) {
        self.vert_shader_entry = name;
    }

    pub fn set_frag_entry(&mut self, name: &'a str) {
        self.frag_shader_entry = name;
    }

    pub fn set_topology(&mut self, value: PrimitiveTopology) {
        self.topology = value;
    }

    pub fn create_render_pipeline(
        &self,
        state: &State,
        shader_module: &ShaderModule,
        bind_group_layout: &[&wgpu::BindGroupLayout],
    ) -> wgpu::RenderPipeline {
        let depth_config = if let Some(_) = self.depth_config {
            Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less, // 1.
                stencil: wgpu::StencilState::default(),     // 2.
                bias: wgpu::DepthBiasState::default(),
            })
        } else {
            None
        };

        let pipeline_layout_desc = wgpu::PipelineLayoutDescriptor {
            label: Some("PipelineLayout"),
            bind_group_layouts: bind_group_layout,
            push_constant_ranges: &[],
        };
        let pipeline_layout = state.device.create_pipeline_layout(&pipeline_layout_desc);

        let vertex_state = wgpu::VertexState {
            module: shader_module,
            entry_point: self.vert_shader_entry,
            buffers: &self.vertex_buffer_layouts[..],
        };

        let swap_chain_format = state
            .window_surface
            .get_capabilities(&state.adapter)
            .formats[0]
            .clone();

        let frag_state = wgpu::FragmentState {
            module: shader_module,
            entry_point: self.frag_shader_entry,
            targets: &[Some(swap_chain_format.into())],
        };

        let r_pipeline = wgpu::RenderPipelineDescriptor {
            label: self.label,
            layout: Some(&pipeline_layout),
            vertex: vertex_state,
            fragment: Some(frag_state),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                topology: self.topology,
                ..Default::default()
            },
            depth_stencil: depth_config,
            multisample: wgpu::MultisampleState {
                count: State::get_sample_count(),
                ..Default::default()
            },
            multiview: None,
        };

        state.device.create_render_pipeline(&r_pipeline)
    }

    fn default_vert_entry_point() -> &'a str {
        return "vs_main";
    }

    fn default_frag_entry_point() -> &'a str {
        return "fs_main";
    }
}
