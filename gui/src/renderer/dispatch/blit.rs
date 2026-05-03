use super::super::pipeline::blit::BlitPipeline;

pub(super) struct BlitPassContext<'a> {
    pub(super) encoder: &'a mut wgpu::CommandEncoder,
    pub(super) device: &'a wgpu::Device,
    pub(super) frame_view: &'a wgpu::TextureView,
    pub(super) resolve_view: &'a wgpu::TextureView,
    pub(super) blit: &'a BlitPipeline,
}

pub(super) fn render_blit_pass(ctx: BlitPassContext<'_>) {
    let BlitPassContext {
        encoder,
        device,
        frame_view,
        resolve_view,
        blit,
    } = ctx;

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("blit_bind_group"),
        layout: &blit.bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(resolve_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&blit.sampler),
            },
        ],
    });

    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("blit"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: frame_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        })],
        depth_stencil_attachment: None,
        ..Default::default()
    });

    pass.set_pipeline(&blit.pipeline);
    pass.set_bind_group(0, &bind_group, &[]);
    pass.draw(0..3, 0..1);
}
