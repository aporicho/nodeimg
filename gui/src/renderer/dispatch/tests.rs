use std::ops::Range;

use super::render_steps::{plan_render_steps, should_resolve_step, RenderStep};
use crate::renderer::prepare::DrawOp;

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
            RenderStep::TextBatch {
                indices: vec![0],
                clip_depth: 1,
            },
            RenderStep::Ops {
                range: Range { start: 2, end: 3 },
                starting_clip_depth: 1,
            },
            RenderStep::TextBatch {
                indices: vec![1],
                clip_depth: 0,
            },
        ]
    );
}

#[test]
fn plan_render_steps_batches_adjacent_text_ops_at_same_clip_depth() {
    let steps = plan_render_steps(&[
        DrawOp::Text { index: 0 },
        DrawOp::Text { index: 1 },
        DrawOp::StencilWrite {
            index_start: 0,
            index_count: 6,
        },
        DrawOp::Text { index: 2 },
        DrawOp::Text { index: 3 },
    ]);

    assert_eq!(
        steps,
        vec![
            RenderStep::TextBatch {
                indices: vec![0, 1],
                clip_depth: 0,
            },
            RenderStep::Ops {
                range: Range { start: 2, end: 3 },
                starting_clip_depth: 0,
            },
            RenderStep::TextBatch {
                indices: vec![2, 3],
                clip_depth: 1,
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
