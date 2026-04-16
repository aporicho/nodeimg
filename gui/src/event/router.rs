use std::time::Instant;

use crate::context::Context;
use crate::event::gesture_adapter;
use crate::gesture::{self, GestureSignal};
use crate::output::FrameworkOutput;
use crate::shell::{AppEvent, MouseButton};
use crate::tree::layout::Overflow;
use crate::tree::{hit_test, NodeId, Tree};

pub(crate) fn handle_event(ctx: &mut Context, event: &AppEvent) -> FrameworkOutput {
    ctx.interaction_state.handle_event(&ctx.tree, event);
    let scroll_consumed = handle_scroll_event(&mut ctx.tree, event);
    if ctx
        .popup_system
        .handle_event(&ctx.tree, &mut ctx.interaction_state, event)
    {
        return finalize_output(FrameworkOutput::consumed());
    }

    let dropdown_output = ctx.dropdown_system.handle_event(
        &ctx.tree,
        &mut ctx.interaction_state,
        &mut ctx.popup_system,
        event,
    );
    let text_output =
        ctx.text_input_system
            .handle_event(&ctx.tree, &mut ctx.interaction_state, event);

    if dropdown_output.consumed
        || !dropdown_output.events.is_empty()
        || !dropdown_output.effects.is_empty()
    {
        ctx.gesture_arena = None;
        return finalize_output(
            dropdown_output
                .merge(text_output)
                .with_consumed(scroll_consumed),
        );
    }

    let gesture_output = handle_gesture_event(ctx, event);
    finalize_output(
        dropdown_output
            .merge(text_output)
            .merge(gesture_output)
            .with_consumed(scroll_consumed),
    )
}

fn handle_gesture_event(ctx: &mut Context, event: &AppEvent) -> FrameworkOutput {
    match *event {
        AppEvent::MousePress { x, y, button }
            if button == MouseButton::Left && ctx.gesture_arena.is_none() =>
        {
            let chain = hit_chain(&ctx.tree, x, y);
            if let Some(arena) =
                gesture::arena_from_hit_chain(&ctx.tree, &chain, x, y, ctx.last_tap_time)
            {
                ctx.gesture_arena = Some(arena);
                return FrameworkOutput::consumed();
            }
            FrameworkOutput::default()
        }
        AppEvent::MouseMove { x, y } => {
            let Some(arena) = ctx.gesture_arena.as_mut() else {
                return FrameworkOutput::default();
            };
            arena
                .pointer_move(x, y)
                .map(|signal| record_gesture_signal(ctx, signal))
                .unwrap_or_else(FrameworkOutput::consumed)
        }
        AppEvent::MouseRelease { x, y, button } if button == MouseButton::Left => {
            let Some(mut arena) = ctx.gesture_arena.take() else {
                return FrameworkOutput::default();
            };
            arena
                .pointer_up(x, y)
                .map(|signal| record_gesture_signal(ctx, signal))
                .unwrap_or_else(FrameworkOutput::consumed)
        }
        AppEvent::Unfocused => {
            ctx.gesture_arena = None;
            FrameworkOutput::default()
        }
        _ => FrameworkOutput::default(),
    }
}

fn record_gesture_signal(ctx: &mut Context, signal: GestureSignal) -> FrameworkOutput {
    match &signal {
        GestureSignal::Click(_) => {
            ctx.last_tap_time = Some(Instant::now());
        }
        GestureSignal::DoubleClick(_) => {
            ctx.last_tap_time = None;
        }
        _ => {}
    }
    gesture_adapter::gesture_signal_output(&ctx.tree, &signal)
}

fn finalize_output(mut output: FrameworkOutput) -> FrameworkOutput {
    output.consumed |= !output.events.is_empty() || !output.effects.is_empty();
    output
}

fn handle_scroll_event(tree: &mut Tree, event: &AppEvent) -> bool {
    let Some((x, y, delta)) = scroll_event_delta(event) else {
        return false;
    };
    let Some(node_id) = scroll_target_at(tree, x, y) else {
        return false;
    };
    tree.scroll(node_id, delta);
    true
}

fn scroll_target_at(tree: &Tree, x: f32, y: f32) -> Option<NodeId> {
    hit_chain(tree, x, y).iter().find(|&node_id| {
        tree.get(node_id)
            .map(|node| node.style.overflow == Overflow::Scroll)
            .unwrap_or(false)
    })
}

fn hit_chain(tree: &Tree, x: f32, y: f32) -> crate::tree::HitChain {
    let Some(root) = tree.root() else {
        return crate::tree::HitChain::empty();
    };
    hit_test(tree, root, x, y)
}

fn scroll_event_delta(event: &AppEvent) -> Option<(f32, f32, f32)> {
    match *event {
        AppEvent::ScrollLine { x, y, delta_y, .. } => Some((x, y, -delta_y * 32.0)),
        AppEvent::ScrollPixel { x, y, delta_y, .. } => Some((x, y, -delta_y)),
        _ => None,
    }
}
