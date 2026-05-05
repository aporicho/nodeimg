use crate::context::Context;
use crate::input::{PointerHitSnapshot, ScrollRequest};
use crate::output::FrameworkOutput;
use crate::shell::AppEvent;

pub(crate) fn handle_event(ctx: &mut Context, event: &AppEvent) -> FrameworkOutput {
    let pointer_hit = ctx.pointer_hit_snapshot(event);
    let pointer_hit = pointer_hit.as_ref();
    ctx.handle_interaction_event(event, pointer_hit);
    let scroll_consumed = handle_scroll_event(ctx, event, pointer_hit);
    let runtime_result = ctx.handle_runtime_pre_gesture_event(event, pointer_hit);
    if runtime_result.cancel_gesture {
        ctx.cancel_gesture();
        return finalize_output(runtime_result.output.with_consumed(scroll_consumed));
    }

    let gesture_output = ctx.handle_gesture_event(event, pointer_hit);
    finalize_output(
        runtime_result
            .output
            .merge(gesture_output)
            .with_consumed(scroll_consumed),
    )
}

fn finalize_output(mut output: FrameworkOutput) -> FrameworkOutput {
    output.consumed |= !output.events.is_empty() || !output.effects.is_empty();
    output
}

fn handle_scroll_event(
    ctx: &mut Context,
    event: &AppEvent,
    hit: Option<&PointerHitSnapshot>,
) -> bool {
    let Some(request) = ScrollRequest::from_event(event) else {
        return false;
    };
    let Some(node_id) = ctx.scroll_target_for_request(hit, request) else {
        return false;
    };
    ctx.tree.scroll(node_id, request.delta_y());
    true
}
