use std::ops::Range;
use winit::dpi::PhysicalSize;

use super::buffer::SharedViewport;
use super::command::DrawCommand;
use super::pipeline::blit::BlitPipeline;
use super::pipeline::circle::CirclePipeline;
use super::pipeline::image::ImagePipeline;
use super::pipeline::quad::QuadPipeline;
use super::pipeline::shadow::ShadowPipeline;
use super::pipeline::stencil::StencilState;
use super::pipeline::text::TextPipeline;
use super::pipeline::vector::VectorPipeline;
use super::prepare::{prepare_frame, DrawOp};
use super::text_measurer::TextMeasurer;
use super::vector_tessellator::VectorTessellator;

pub fn dispatch(
    commands: &[DrawCommand],
    frame_view: &wgpu::TextureView,
    msaa_view: &wgpu::TextureView,
    resolve_view: &wgpu::TextureView,
    internal_size: PhysicalSize<u32>,
    scale_factor: f64,
    render_scale: f32,
    clear_color: super::types::Color,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    blit: &BlitPipeline,
    shared_viewport: &mut SharedViewport,
    quad_pipeline: &mut QuadPipeline,
    text_pipeline: &mut TextPipeline,
    image_pipeline: &mut ImagePipeline,
    circle_pipeline: &mut CirclePipeline,
    vector_pipeline: &mut VectorPipeline,
    vector_tessellator: &mut VectorTessellator,
    shadow_pipeline: &mut ShadowPipeline,
    stencil: &mut StencilState,
    text_measurer: &mut TextMeasurer,
) {
    let logical_w = internal_size.width as f64 / scale_factor / render_scale as f64;
    let logical_h = internal_size.height as f64 / scale_factor / render_scale as f64;
    let viewport_size = [logical_w as f32, logical_h as f32];

    shared_viewport.upload(device, queue, viewport_size);
    let viewport_buf = shared_viewport.buffer();

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("renderer"),
    });

    for cmd in commands {
        if let DrawCommand::Shadow(req) = cmd {
            shadow_pipeline.prepare(&mut encoder, device, req);
        }
    }

    let prepared = prepare_frame(commands, vector_tessellator);

    quad_pipeline.upload(
        device,
        queue,
        &prepared.quad_vertices,
        &prepared.quad_indices,
    );
    circle_pipeline.upload(
        device,
        queue,
        &prepared.circle_vertices,
        &prepared.circle_indices,
    );
    vector_pipeline.upload(
        device,
        queue,
        &prepared.vector_vertices,
        &prepared.vector_indices,
    );
    stencil.upload(
        device,
        queue,
        &prepared.stencil_vertices,
        &prepared.stencil_indices,
    );

    quad_pipeline.update_bind_group(device, viewport_buf);
    circle_pipeline.update_bind_group(device, viewport_buf);
    vector_pipeline.update_bind_group(device, viewport_buf);
    stencil.update_bind_group(device, viewport_buf);

    let has_quads = !prepared.quad_vertices.is_empty();
    let has_stencils = !prepared.stencil_vertices.is_empty();
    let render_steps = plan_render_steps(&prepared.ops);
    text_pipeline.begin_frame();

    {
        let total_steps = render_steps.len();
        let mut first_pass = true;
        for (step_index, step) in render_steps.iter().enumerate() {
            let resolve_target =
                should_resolve_step(step_index, total_steps).then_some(resolve_view);
            match step {
                RenderStep::Ops {
                    range,
                    starting_clip_depth,
                } => {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("main_ops"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: msaa_view,
                            resolve_target,
                            ops: wgpu::Operations {
                                load: color_load_op(first_pass, clear_color),
                                store: color_store_op(),
                            },
                            depth_slice: None,
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: stencil.depth_stencil_view(),
                            depth_ops: None,
                            stencil_ops: Some(wgpu::Operations {
                                load: stencil_load_op(first_pass),
                                store: wgpu::StoreOp::Store,
                            }),
                        }),
                        ..Default::default()
                    });

                    let mut last_bound = PipelineKind::None;
                    let mut clip_depth = *starting_clip_depth;

                    if has_quads {
                        quad_pipeline.bind(&mut pass);
                    }

                    for op in &prepared.ops[range.clone()] {
                        match op {
                            DrawOp::Quad {
                                index_start,
                                index_count,
                            } => {
                                if last_bound != PipelineKind::Quad {
                                    quad_pipeline.bind(&mut pass);
                                    last_bound = PipelineKind::Quad;
                                }
                                pass.set_stencil_reference(clip_depth);
                                QuadPipeline::draw_batch(&mut pass, *index_start, *index_count);
                            }
                            DrawOp::Circle {
                                index_start,
                                index_count,
                            } => {
                                if last_bound != PipelineKind::Circle {
                                    circle_pipeline.bind(&mut pass);
                                    last_bound = PipelineKind::Circle;
                                }
                                pass.set_stencil_reference(clip_depth);
                                CirclePipeline::draw_batch(&mut pass, *index_start, *index_count);
                            }
                            DrawOp::Vector {
                                index_start,
                                index_count,
                            } => {
                                if last_bound != PipelineKind::Vector {
                                    vector_pipeline.bind(&mut pass);
                                    last_bound = PipelineKind::Vector;
                                }
                                pass.set_stencil_reference(clip_depth);
                                VectorPipeline::draw_batch(&mut pass, *index_start, *index_count);
                            }
                            DrawOp::Shadow(req) => {
                                last_bound = PipelineKind::Other;
                                shadow_pipeline.draw(
                                    &mut pass,
                                    device,
                                    req,
                                    viewport_buf,
                                    clip_depth,
                                );
                            }
                            DrawOp::Image { view, draw } => {
                                last_bound = PipelineKind::Other;
                                pass.set_stencil_reference(clip_depth);
                                image_pipeline.draw(&mut pass, device, view, *draw, viewport_buf);
                            }
                            DrawOp::Text { .. } => {
                                unreachable!("text ops are split into dedicated render steps")
                            }
                            DrawOp::StencilWrite {
                                index_start,
                                index_count,
                            } => {
                                if has_stencils && last_bound != PipelineKind::Stencil {
                                    stencil.bind_stencil(&mut pass);
                                    last_bound = PipelineKind::Stencil;
                                }
                                stencil.draw_write(
                                    &mut pass,
                                    clip_depth,
                                    *index_start,
                                    *index_count,
                                );
                                clip_depth += 1;
                            }
                            DrawOp::StencilClear {
                                index_start,
                                index_count,
                            } => {
                                if has_stencils && last_bound != PipelineKind::Stencil {
                                    stencil.bind_stencil(&mut pass);
                                    last_bound = PipelineKind::Stencil;
                                }
                                stencil.draw_clear(
                                    &mut pass,
                                    clip_depth,
                                    *index_start,
                                    *index_count,
                                );
                                clip_depth = clip_depth.saturating_sub(1);
                            }
                        }
                    }
                }
                RenderStep::Text { index, clip_depth } => {
                    let batch_index = text_pipeline.prepare(
                        device,
                        queue,
                        std::slice::from_ref(&prepared.text_requests[*index]),
                        internal_size,
                        scale_factor * render_scale as f64,
                        text_measurer,
                    );

                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("main_text"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: msaa_view,
                            resolve_target,
                            ops: wgpu::Operations {
                                load: color_load_op(first_pass, clear_color),
                                store: color_store_op(),
                            },
                            depth_slice: None,
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: stencil.depth_stencil_view(),
                            depth_ops: None,
                            stencil_ops: Some(wgpu::Operations {
                                load: stencil_load_op(first_pass),
                                store: wgpu::StoreOp::Store,
                            }),
                        }),
                        ..Default::default()
                    });

                    pass.set_stencil_reference(*clip_depth);
                    text_pipeline.render_batch(batch_index, &mut pass);
                }
            }
            first_pass = false;
        }
    }

    {
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

    queue.submit(std::iter::once(encoder.finish()));
}

#[derive(Debug, PartialEq)]
enum RenderStep {
    Ops {
        range: Range<usize>,
        starting_clip_depth: u32,
    },
    Text {
        index: usize,
        clip_depth: u32,
    },
}

fn plan_render_steps(ops: &[DrawOp]) -> Vec<RenderStep> {
    let mut steps = Vec::new();
    let mut clip_depth = 0u32;
    let mut ops_start: Option<usize> = None;
    let mut ops_clip_depth = 0u32;

    for (i, op) in ops.iter().enumerate() {
        match op {
            DrawOp::Text { index } => {
                if let Some(start) = ops_start.take() {
                    steps.push(RenderStep::Ops {
                        range: start..i,
                        starting_clip_depth: ops_clip_depth,
                    });
                }
                steps.push(RenderStep::Text {
                    index: *index,
                    clip_depth,
                });
            }
            DrawOp::StencilWrite { .. } => {
                if ops_start.is_none() {
                    ops_start = Some(i);
                    ops_clip_depth = clip_depth;
                }
                clip_depth += 1;
            }
            DrawOp::StencilClear { .. } => {
                if ops_start.is_none() {
                    ops_start = Some(i);
                    ops_clip_depth = clip_depth;
                }
                clip_depth = clip_depth.saturating_sub(1);
            }
            _ => {
                if ops_start.is_none() {
                    ops_start = Some(i);
                    ops_clip_depth = clip_depth;
                }
            }
        }
    }

    if let Some(start) = ops_start {
        steps.push(RenderStep::Ops {
            range: start..ops.len(),
            starting_clip_depth: ops_clip_depth,
        });
    }

    if steps.is_empty() {
        steps.push(RenderStep::Ops {
            range: 0..0,
            starting_clip_depth: 0,
        });
    }

    steps
}

fn color_load_op(first_pass: bool, clear_color: super::types::Color) -> wgpu::LoadOp<wgpu::Color> {
    if first_pass {
        wgpu::LoadOp::Clear(wgpu::Color {
            r: clear_color.r as f64,
            g: clear_color.g as f64,
            b: clear_color.b as f64,
            a: clear_color.a as f64,
        })
    } else {
        wgpu::LoadOp::Load
    }
}

fn color_store_op() -> wgpu::StoreOp {
    wgpu::StoreOp::Store
}

fn should_resolve_step(step_index: usize, total_steps: usize) -> bool {
    total_steps > 0 && step_index + 1 == total_steps
}

fn stencil_load_op(first_pass: bool) -> wgpu::LoadOp<u32> {
    if first_pass {
        wgpu::LoadOp::Clear(0)
    } else {
        wgpu::LoadOp::Load
    }
}

#[derive(PartialEq)]
enum PipelineKind {
    None,
    Quad,
    Circle,
    Vector,
    Stencil,
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Range;

    #[test]
    fn plan_render_steps_tracks_clip_depth_per_text_op() {
        let steps = plan_render_steps(&[
            DrawOp::StencilWrite {
                index_start: 0,
                index_count: 6,
            },
            DrawOp::Text { index: 0 },
            DrawOp::StencilClear {
                index_start: 6,
                index_count: 6,
            },
            DrawOp::Text { index: 1 },
        ]);

        assert_eq!(
            steps,
            vec![
                RenderStep::Ops {
                    range: Range { start: 0, end: 1 },
                    starting_clip_depth: 0,
                },
                RenderStep::Text {
                    index: 0,
                    clip_depth: 1,
                },
                RenderStep::Ops {
                    range: Range { start: 2, end: 3 },
                    starting_clip_depth: 1,
                },
                RenderStep::Text {
                    index: 1,
                    clip_depth: 0,
                },
            ]
        );
    }

    #[test]
    fn plan_render_steps_emits_clear_step_for_empty_ops() {
        assert_eq!(
            plan_render_steps(&[]),
            vec![RenderStep::Ops {
                range: Range { start: 0, end: 0 },
                starting_clip_depth: 0,
            }]
        );
    }

    #[test]
    fn resolves_only_on_final_render_step() {
        assert!(!should_resolve_step(0, 3));
        assert!(!should_resolve_step(1, 3));
        assert!(should_resolve_step(2, 3));
    }

    #[test]
    fn single_render_step_still_resolves() {
        assert!(should_resolve_step(0, 1));
    }
}
