use std::ops::Range;

use super::super::prepare::DrawOp;

#[derive(Debug, PartialEq)]
pub(super) enum RenderStep {
    Ops {
        range: Range<usize>,
        starting_clip_depth: u32,
    },
    TextBatch {
        indices: Vec<usize>,
        clip_depth: u32,
    },
}

impl RenderStep {
    pub(super) fn kind(&self) -> &'static str {
        match self {
            Self::Ops { .. } => "ops",
            Self::TextBatch { .. } => "text_batch",
        }
    }
}

pub(super) fn plan_render_steps(ops: &[DrawOp]) -> Vec<RenderStep> {
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
                match steps.last_mut() {
                    Some(RenderStep::TextBatch {
                        indices,
                        clip_depth: existing_clip_depth,
                    }) if *existing_clip_depth == clip_depth => {
                        indices.push(*index);
                    }
                    _ => steps.push(RenderStep::TextBatch {
                        indices: vec![*index],
                        clip_depth,
                    }),
                }
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

pub(super) fn should_resolve_step(step_index: usize, total_steps: usize) -> bool {
    total_steps > 0 && step_index + 1 == total_steps
}
