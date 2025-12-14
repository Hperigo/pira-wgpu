use crate::state::State;
use wgpu::{BlendState, DepthStencilState, PipelineCompilationOptions, PrimitiveTopology, ShaderModule, TextureFormat};

#[derive(Debug)]
pub enum DepthConfig {
    None,
    Custom(DepthStencilState),
    DefaultWrite,
    DefaultDontWrite,
}

impl DepthConfig {
    pub fn get(&self) -> Option<DepthStencilState> {
        match &self {
            DepthConfig::None => None,
            DepthConfig::Custom(depth_state) => Some(depth_state.clone()),
            DepthConfig::DefaultWrite => {
                Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24Plus,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less, // 1.
                    stencil: wgpu::StencilState::default(),     // 2.
                    bias: wgpu::DepthBiasState::default(),
                })
            }
            DepthConfig::DefaultDontWrite => {
                Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24Plus,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Less, // 1.
                    stencil: wgpu::StencilState::default(),     // 2.
                    bias: wgpu::DepthBiasState::default(),
                })
            }
        }
    }
}

#[derive(Debug)]
pub enum BlendConfig {
    None,
    Custom(BlendState),
    Default,
}

impl BlendConfig {
    pub fn get(&self) -> Option<BlendState> {
        match self {
            BlendConfig::Default => Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent::OVER,
            }),
            BlendConfig::Custom(custom) => Some(*custom),
            BlendConfig::None => None,
        }
    }
}

pub struct RenderPipelineFactory<'a> {
    vertex_buffer_layouts: Vec<wgpu::VertexBufferLayout<'a>>,
    // depth_config: Option<wgpu::DepthStencilState>,
    depth_config: DepthConfig,
    blend_config: BlendConfig,

    vert_shader_entry: Option<&'a str>,
    frag_shader_entry: Option<&'a str>,

    color_target_format: Option<TextureFormat>,

    sample_count: Option<u32>,

    topology: PrimitiveTopology,

    cull_mode: Option<wgpu::Face>,

    label: Option<&'static str>,
}

impl<'a> RenderPipelineFactory<'a> {
    pub fn new() -> Self {
        RenderPipelineFactory {
            vertex_buffer_layouts: Vec::new(),
            depth_config: DepthConfig::None,
            blend_config: BlendConfig::Default,

            vert_shader_entry: Self::default_vert_entry_point(),
            frag_shader_entry: Self::default_frag_entry_point(),

            color_target_format: None,

            sample_count: None,

            topology: PrimitiveTopology::TriangleList,

            cull_mode: None,
            label: Some("Pipeline from helper"),
        }
    }

    pub fn set_label(&mut self, label: &'static str) {
        self.label = Some(label);
    }

    pub fn set_cull_mode(&mut self, mode: Option<wgpu::Face>) {
        self.cull_mode = mode;
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

    //TODO: rename to set_depth_config
    pub fn add_depth_stencil(&mut self, config: DepthConfig) {
        self.depth_config = config;
    }

    pub fn set_blend_config(&mut self, config: BlendConfig) {
        self.blend_config = config;
    }

    pub fn set_sample_count(&mut self, sample_count: Option<u32>) {
        self.sample_count = sample_count;
    }

    pub fn set_color_target_format(&mut self, format: Option<TextureFormat>) {
        self.color_target_format = format;
    }

    pub fn set_vert_entry(&mut self, name: Option<&'a str>) {
        self.vert_shader_entry = name;
    }

    pub fn set_frag_entry(&mut self, name: Option<&'a str>) {
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
        let depth_config = self.depth_config.get();

        let blend_config = self.blend_config.get();

        let sample_count = match &self.sample_count {
            Some(val) => *val,
            None => state.get_sample_count(),
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
            compilation_options: PipelineCompilationOptions::default(),
        };

        let color_target_format = match self.color_target_format {
            Some(format) => format,
            None => state
                .window_surface
                .get_capabilities(&state.adapter)
                .formats[0]
                .clone(),
        };

        let frag_state = wgpu::FragmentState {
            module: shader_module,
            entry_point: self.frag_shader_entry,
            compilation_options : PipelineCompilationOptions::default(),

            targets: &[Some(wgpu::ColorTargetState {
                format: color_target_format,
                blend: blend_config,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        };

        let r_pipeline: wgpu::RenderPipelineDescriptor<'_> = wgpu::RenderPipelineDescriptor {
            label: self.label,
            layout: Some(&pipeline_layout),
            vertex: vertex_state,
            fragment: Some(frag_state),
            primitive: wgpu::PrimitiveState {
                cull_mode: self.cull_mode,
                topology: self.topology,
                ..Default::default()
            },
            depth_stencil: depth_config,
            multisample: wgpu::MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            multiview: None,
            cache : None,
        };

        state.device.create_render_pipeline(&r_pipeline)
    }

    fn default_vert_entry_point() -> Option<&'a str> {
        return Some("vs_main");
    }

    fn default_frag_entry_point() -> Option<&'a str> {
        return Some("fs_main");
    }
}
