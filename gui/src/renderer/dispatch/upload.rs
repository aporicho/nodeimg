use super::super::pipeline::circle::CirclePipeline;
use super::super::pipeline::grid::GridPipeline;
use super::super::pipeline::quad::QuadPipeline;
use super::super::pipeline::stencil::StencilState;
use super::super::pipeline::vector::VectorPipeline;
use super::super::prepare::PreparedFrame;
use super::super::upload_arena::UploadStats;

pub(super) struct UploadPreparedBuffers<'a> {
    pub(super) upload_stats: &'a mut UploadStats,
    pub(super) prepared: &'a mut PreparedFrame,
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
    pub(super) viewport_buf: &'a wgpu::Buffer,
    pub(super) quad_pipeline: &'a mut QuadPipeline,
    pub(super) circle_pipeline: &'a mut CirclePipeline,
    pub(super) grid_pipeline: &'a mut GridPipeline,
    pub(super) vector_pipeline: &'a mut VectorPipeline,
    pub(super) stencil: &'a mut StencilState,
}

pub(super) fn upload_prepared_buffers(ctx: UploadPreparedBuffers<'_>) {
    let UploadPreparedBuffers {
        upload_stats,
        prepared,
        device,
        queue,
        viewport_buf,
        quad_pipeline,
        circle_pipeline,
        grid_pipeline,
        vector_pipeline,
        stencil,
    } = ctx;

    upload_stats.add(quad_pipeline.upload(
        device,
        queue,
        &prepared.quad_vertices,
        &prepared.quad_indices,
    ));
    upload_stats.add(circle_pipeline.upload(
        device,
        queue,
        &prepared.circle_vertices,
        &prepared.circle_indices,
    ));
    upload_stats.add(grid_pipeline.upload(
        device,
        queue,
        &prepared.grid_vertices,
        &prepared.grid_indices,
    ));
    upload_stats.add(vector_pipeline.upload(
        device,
        queue,
        &prepared.vector_vertices,
        &prepared.vector_indices,
    ));
    upload_stats.add(stencil.upload(
        device,
        queue,
        &prepared.stencil_vertices,
        &prepared.stencil_indices,
    ));
    prepared.stats.upload_bytes = upload_stats.bytes;
    prepared.stats.upload_buffer_grows = upload_stats.buffer_grows;

    quad_pipeline.update_bind_group(device, viewport_buf);
    circle_pipeline.update_bind_group(device, viewport_buf);
    grid_pipeline.update_bind_group(device, viewport_buf);
    vector_pipeline.update_bind_group(device, viewport_buf);
    stencil.update_bind_group(device, viewport_buf);
}
