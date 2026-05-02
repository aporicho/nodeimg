use crate::action::dispatch_widget_click;
use crate::event::target::TargetResolver;
use crate::gesture::GestureSignal;
use crate::output::{FrameworkOutput, GuiEvent, OutputBuilder, PanelEvent, WidgetEvent};
use crate::tree::Tree;
use crate::widget::resize_edge::ResizeEdge;

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
        GuiEvent::Widget(WidgetEvent::Click { id }) => {
            builder.action(dispatch_widget_click(&id)).finish()
        }
        _ => builder.finish(),
    }
}

fn gesture_signal_event(tree: &Tree, signal: &GestureSignal) -> GuiEvent {
    let resolver = TargetResolver::new(tree);
    match signal {
        GestureSignal::Click(id) => GuiEvent::Widget(WidgetEvent::Click {
            id: resolver.owner_id(id),
        }),
        GestureSignal::DoubleClick(id) => GuiEvent::Widget(WidgetEvent::DoubleClick {
            id: resolver.owner_id(id),
        }),
        GestureSignal::DragStart { id, x, y } => {
            drag_event(&resolver, id, *x, *y, DragPhase::Start)
        }
        GestureSignal::DragMove { id, x, y } => drag_event(&resolver, id, *x, *y, DragPhase::Move),
        GestureSignal::DragEnd { id, x, y } => drag_event(&resolver, id, *x, *y, DragPhase::End),
        GestureSignal::LongPress(id) => GuiEvent::Widget(WidgetEvent::LongPress {
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
    resolver: &TargetResolver<'_>,
    id: &str,
    x: f32,
    y: f32,
    phase: DragPhase,
) -> GuiEvent {
    let owner_id = resolver.owner_id(id);
    if resolver
        .widget_role(&owner_id)
        .is_some_and(|role| role.is_panel())
    {
        return GuiEvent::Panel(match phase {
            DragPhase::Start => PanelEvent::DragStart { id: owner_id, x, y },
            DragPhase::Move => PanelEvent::DragMove { id: owner_id, x, y },
            DragPhase::End => PanelEvent::DragEnd { id: owner_id, x, y },
        });
    }

    GuiEvent::Widget(match phase {
        DragPhase::Start => WidgetEvent::DragStart { id: owner_id, x, y },
        DragPhase::Move => WidgetEvent::DragMove { id: owner_id, x, y },
        DragPhase::End => WidgetEvent::DragEnd { id: owner_id, x, y },
    })
}

fn resize_event(
    resolver: &TargetResolver<'_>,
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
        is_panel = resolver.widget_role(&owner_id).is_some_and(|role| role.is_panel()),
        "map resize gesture signal"
    );
    if !resolver
        .widget_role(&owner_id)
        .is_some_and(|role| role.is_panel())
    {
        return GuiEvent::Widget(match phase {
            ResizePhase::Start => WidgetEvent::ResizeStart {
                id: owner_id,
                edge,
                x,
                y,
            },
            ResizePhase::Move => WidgetEvent::ResizeMove {
                id: owner_id,
                edge,
                x,
                y,
            },
            ResizePhase::End => WidgetEvent::ResizeEnd {
                id: owner_id,
                edge,
                x,
                y,
            },
        });
    }

    GuiEvent::Panel(match phase {
        ResizePhase::Start => PanelEvent::ResizeStart {
            id: owner_id,
            edge,
            x,
            y,
        },
        ResizePhase::Move => PanelEvent::ResizeMove {
            id: owner_id,
            edge,
            x,
            y,
        },
        ResizePhase::End => PanelEvent::ResizeEnd {
            id: owner_id,
            edge,
            x,
            y,
        },
    })
}
