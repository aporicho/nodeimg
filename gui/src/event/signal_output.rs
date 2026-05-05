use crate::action::dispatch_control_click;
use crate::geometry::ResizeEdge;
use crate::gesture::GestureSignal;
use crate::output::{ControlEvent, FrameworkOutput, GuiEvent, OutputBuilder};
use crate::tree::{TargetOwnerResolver, Tree};

#[derive(Clone, Copy)]
enum DragPhase {
    Start,
    Move,
    End,
}

#[derive(Clone, Copy)]
enum ResizePhase {
    Start,
    Move,
    End,
}

pub(crate) fn gesture_signal_output(tree: &Tree, signal: &GestureSignal) -> FrameworkOutput {
    let event = gesture_signal_event(tree, signal);
    let builder = OutputBuilder::new().event(event.clone());

    match event {
        GuiEvent::Control(ControlEvent::Click { id }) => {
            builder.action(dispatch_control_click(&id)).finish()
        }
        _ => builder.finish(),
    }
}

fn gesture_signal_event(tree: &Tree, signal: &GestureSignal) -> GuiEvent {
    let resolver = TargetOwnerResolver::new(tree);
    match signal {
        GestureSignal::Click(id) => GuiEvent::Control(ControlEvent::Click {
            id: resolver.owner_id(id),
        }),
        GestureSignal::DoubleClick(id) => GuiEvent::Control(ControlEvent::DoubleClick {
            id: resolver.owner_id(id),
        }),
        GestureSignal::DragStart { id, x, y } => {
            drag_event(&resolver, id, *x, *y, DragPhase::Start)
        }
        GestureSignal::DragMove { id, x, y } => drag_event(&resolver, id, *x, *y, DragPhase::Move),
        GestureSignal::DragEnd { id, x, y } => drag_event(&resolver, id, *x, *y, DragPhase::End),
        GestureSignal::LongPress(id) => GuiEvent::Control(ControlEvent::LongPress {
            id: resolver.owner_id(id),
        }),
        GestureSignal::ResizeStart { id, edge, x, y } => {
            resize_event(&resolver, id, *edge, *x, *y, ResizePhase::Start)
        }
        GestureSignal::ResizeMove { id, edge, x, y } => {
            resize_event(&resolver, id, *edge, *x, *y, ResizePhase::Move)
        }
        GestureSignal::ResizeEnd { id, edge, x, y } => {
            resize_event(&resolver, id, *edge, *x, *y, ResizePhase::End)
        }
    }
}

fn drag_event(
    resolver: &TargetOwnerResolver<'_>,
    id: &str,
    x: f32,
    y: f32,
    phase: DragPhase,
) -> GuiEvent {
    let owner_id = resolver.owner_id(id);
    GuiEvent::Control(match phase {
        DragPhase::Start => ControlEvent::DragStart { id: owner_id, x, y },
        DragPhase::Move => ControlEvent::DragMove { id: owner_id, x, y },
        DragPhase::End => ControlEvent::DragEnd { id: owner_id, x, y },
    })
}

fn resize_event(
    resolver: &TargetOwnerResolver<'_>,
    id: &str,
    edge: ResizeEdge,
    x: f32,
    y: f32,
    phase: ResizePhase,
) -> GuiEvent {
    let owner_id = resolver.owner_id(id);
    tracing::debug!(
        target: "gui::gesture",
        raw_id = %id,
        owner_id = %owner_id,
        edge = ?edge,
        x,
        y,
        "map resize gesture signal"
    );
    GuiEvent::Control(match phase {
        ResizePhase::Start => ControlEvent::ResizeStart {
            id: owner_id,
            edge,
            x,
            y,
        },
        ResizePhase::Move => ControlEvent::ResizeMove {
            id: owner_id,
            edge,
            x,
            y,
        },
        ResizePhase::End => ControlEvent::ResizeEnd {
            id: owner_id,
            edge,
            x,
            y,
        },
    })
}
