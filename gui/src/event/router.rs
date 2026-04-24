use crate::context::Context;
use crate::output::FrameworkOutput;
use crate::shell::AppEvent;
use crate::tree::layout::Overflow;
use crate::tree::NodeId;

pub(crate) fn handle_event(ctx: &mut Context, event: &AppEvent) -> FrameworkOutput {
    ctx.handle_interaction_event(event);
    let scroll_consumed = handle_scroll_event(ctx, event);
    let runtime_result = ctx.handle_runtime_pre_gesture_event(event);
    if runtime_result.cancel_gesture {
        ctx.cancel_gesture();
        return finalize_output(runtime_result.output.with_consumed(scroll_consumed));
    }

    let gesture_output = ctx.handle_gesture_event(event);
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

fn handle_scroll_event(ctx: &mut Context, event: &AppEvent) -> bool {
    let Some((x, y, delta)) = scroll_event_delta(event) else {
        return false;
    };
    let Some(node_id) = scroll_target_at(ctx, x, y) else {
        return false;
    };
    ctx.tree.scroll(node_id, delta);
    true
}

fn scroll_target_at(ctx: &Context, x: f32, y: f32) -> Option<NodeId> {
    ctx.hit_test(x, y).iter().find(|&node_id| {
        ctx.tree
            .get(node_id)
            .map(|node| node.style.overflow == Overflow::Scroll)
            .unwrap_or(false)
    })
}

fn scroll_event_delta(event: &AppEvent) -> Option<(f32, f32, f32)> {
    match *event {
        AppEvent::ScrollLine { x, y, delta_y, .. } => Some((x, y, -delta_y * 32.0)),
        AppEvent::ScrollPixel { x, y, delta_y, .. } => Some((x, y, -delta_y)),
        _ => None,
    }
}
