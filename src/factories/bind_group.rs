use std::num::NonZeroU64;

pub struct BindGroupFactory<'a> {
    resources: Vec<wgpu::BindGroupEntry<'a>>,
    // buffers : Vec<wgpu::Buffer>,
    pub binding_types: Vec<(wgpu::ShaderStages, wgpu::BindingType)>,
}

impl<'a> BindGroupFactory<'a> {
    pub fn new() -> Self {
        Self {
            resources: Vec::new(),
            // buffers : Vec::new(),
            binding_types: Vec::new(),
        }
    }

    pub fn add_uniform<'b>(
        &'b mut self,
        stage: wgpu::ShaderStages,
        data: &'a wgpu::Buffer,
        min_binding_size: Option<NonZeroU64>,
    ) -> &'b mut Self {
        {
            let binding_type = wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size,
            };
            self.binding_types.push((stage, binding_type));

            self.resources.push(wgpu::BindGroupEntry {
                binding: self.resources.len() as u32,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &data,
                    offset: 0,
                    size: min_binding_size,
                }),
            });
        }

        self
    }

    pub fn add_texture_and_sampler<'b>(
        &'b mut self,
        stage: wgpu::ShaderStages,
        texture_view: &'a wgpu::TextureView,
        sampler: &'a wgpu::Sampler,
    ) -> &'b mut Self {
        self.resources.push(wgpu::BindGroupEntry {
            binding: self.resources.len() as u32,
            resource: wgpu::BindingResource::TextureView(&texture_view),
        });
        self.resources.push(wgpu::BindGroupEntry {
            binding: self.resources.len() as u32,
            resource: wgpu::BindingResource::Sampler(&sampler),
        });

        let texture_binding_type = wgpu::BindingType::Texture {
            multisampled: false,
            view_dimension: wgpu::TextureViewDimension::D2,
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
        };
        let sampler_binding_type = wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering);

        self.binding_types.push((stage, texture_binding_type));
        self.binding_types.push((stage, sampler_binding_type));

        self
    }

    pub fn add_texture_sky_sampler<'b>(
        &'b mut self,
        stage: wgpu::ShaderStages,
        texture_view: &'a wgpu::TextureView,
        sampler: &'a wgpu::Sampler,
    ) -> &'b mut Self {
        self.resources.push(wgpu::BindGroupEntry {
            binding: self.resources.len() as u32,
            resource: wgpu::BindingResource::TextureView(&texture_view),
        });
        self.resources.push(wgpu::BindGroupEntry {
            binding: self.resources.len() as u32,
            resource: wgpu::BindingResource::Sampler(&sampler),
        });

        let texture_binding_type = wgpu::BindingType::Texture {
            multisampled: false,
            view_dimension: wgpu::TextureViewDimension::Cube,
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
        };
        let sampler_binding_type =
            wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering);

        self.binding_types.push((stage, texture_binding_type));
        self.binding_types.push((stage, sampler_binding_type));

        self
    }

    pub fn add_texture_hdr_and_sampler<'b>(
        &'b mut self,
        stage: wgpu::ShaderStages,
        texture_view: &'a wgpu::TextureView,
        sampler: &'a wgpu::Sampler,
    ) -> &'b mut Self {
        self.resources.push(wgpu::BindGroupEntry {
            binding: self.resources.len() as u32,
            resource: wgpu::BindingResource::TextureView(&texture_view),
        });
        self.resources.push(wgpu::BindGroupEntry {
            binding: self.resources.len() as u32,
            resource: wgpu::BindingResource::Sampler(&sampler),
        });

        let texture_binding_type = wgpu::BindingType::Texture {
            multisampled: false,
            view_dimension: wgpu::TextureViewDimension::D2,
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
        };
        let sampler_binding_type = wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering);

        self.binding_types.push((stage, texture_binding_type));
        self.binding_types.push((stage, sampler_binding_type));

        self
    }

    pub fn build(&self, device: &wgpu::Device) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let mut layout_entries = Vec::new();
        for index in 0..self.binding_types.len() {
            println!("{} Binding {:?}", index, self.binding_types[index]);

            let layout_entry = wgpu::BindGroupLayoutEntry {
                binding: index as u32,
                visibility: self.binding_types[index].0,
                ty: self.binding_types[index].1,
                count: None,
            };
            layout_entries.push(layout_entry);
        }

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &layout_entries.as_slice(),
            label: Some("Bind group layout from helper"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: self.resources.as_slice(),
            label: Some("Bind group from helper"),
        });

        (bind_group_layout, bind_group)
    }
}
