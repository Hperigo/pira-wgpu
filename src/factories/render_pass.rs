use crate::state::State;

pub struct RenderPassFactory<'a> {
    color_attachments: Vec<Option<wgpu::RenderPassColorAttachment<'a>>>,
    depth_stencil: Option<wgpu::RenderPassDepthStencilAttachment<'a>>,
}

impl<'a> RenderPassFactory<'a> {
    pub fn new() -> Self {
        Self {
            color_attachments: Vec::new(),
            depth_stencil: None,
        }
    }

    pub fn add_color_atachment(
        &mut self,
        clear_color: wgpu::Color,
        source: &'a wgpu::TextureView,
        target: Option<&'a wgpu::TextureView>,
    ) {
        self.color_attachments
            .push(Some(wgpu::RenderPassColorAttachment {
                view: &source,
                resolve_target: target,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
                    store: wgpu::StoreOp::Discard,
                },
            }));
    }

    pub fn add_depth_stencil(&mut self, depth_texture: &'a wgpu::TextureView) {
        self.depth_stencil = Some(wgpu::RenderPassDepthStencilAttachment {
            view: &depth_texture,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Discard,
            }),
            stencil_ops: None,
        });
    }

    pub fn get_render_pass(
        &self,
        ctx: &'a State,
        encoder: &'a mut wgpu::CommandEncoder,
        enable_depth: bool,
    ) -> wgpu::RenderPass<'a> {
        let depth_stencil = if enable_depth && ctx.depth_texture.is_some() {
            let texture_bundle = ctx.depth_texture.as_ref().unwrap();

            Some(wgpu::RenderPassDepthStencilAttachment {
                view: &texture_bundle.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            })
        } else {
            println!("NONE");
            None
        };

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &self.color_attachments,
            depth_stencil_attachment: depth_stencil,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass
    }
}
