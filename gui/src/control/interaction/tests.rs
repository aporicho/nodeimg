use super::{ControlInteractionSpec, ControlInteractionSystem};
use crate::control::ControlValue;
use crate::control::SystemCx;
use crate::interaction::InteractionState;
use crate::output::{ControlEvent, GuiEvent};
use crate::renderer::Rect;
use crate::runtime::RuntimeEventResult;
use crate::shell::{AppEvent, MouseButton};
use crate::tree::layout::BoxStyle;
use crate::tree::{Tree, TreeNode, TreeNodeBuilder};

fn control_node(id: &'static str, rect: Rect, spec: ControlInteractionSpec) -> TreeNode {
    TreeNodeBuilder::container(
        id,
        BoxStyle {
            hittable: true,
            ..Default::default()
        },
    )
    .rect(rect)
    .runtime_slot(spec)
    .build()
}

fn root_with_child(child: TreeNode) -> Tree {
    let mut tree = Tree::new();
    let child_id = tree.insert(child);
    let mut root = TreeNodeBuilder::container(
        "root",
        BoxStyle {
            hittable: true,
            ..Default::default()
        },
    )
    .rect(Rect {
        x: 0.0,
        y: 0.0,
        w: 300.0,
        h: 200.0,
    })
    .runtime_slot(ControlInteractionSpec::None)
    .build();
    root.children = vec![child_id];
    let root_id = tree.insert(root);
    tree.set_root(root_id);
    tree
}

fn handle(
    system: &mut ControlInteractionSystem,
    tree: &Tree,
    interaction: &mut InteractionState,
    event: AppEvent,
) -> RuntimeEventResult {
    system.handle_pre_gesture_event(SystemCx::new(tree, None, interaction, None), &event)
}

#[test]
fn toggle_outputs_bool_on_release_inside_same_control() {
    let tree = root_with_child(control_node(
        "toggle",
        Rect {
            x: 10.0,
            y: 10.0,
            w: 60.0,
            h: 24.0,
        },
        ControlInteractionSpec::toggle(false),
    ));
    let mut system = ControlInteractionSystem::new();
    let mut interaction = InteractionState::new();

    let press = handle(
        &mut system,
        &tree,
        &mut interaction,
        AppEvent::MousePress {
            x: 20.0,
            y: 20.0,
            button: MouseButton::Left,
        },
    );
    assert!(press.cancel_gesture);
    assert!(press.output.events.is_empty());

    let release = handle(
        &mut system,
        &tree,
        &mut interaction,
        AppEvent::MouseRelease {
            x: 20.0,
            y: 20.0,
            button: MouseButton::Left,
        },
    );

    assert!(release.cancel_gesture);
    assert_eq!(release.output.events.len(), 1);
    assert!(matches!(
        &release.output.events[0],
        GuiEvent::Control(ControlEvent::ValueChanged {
            id,
            value: ControlValue::Bool(true)
        }) if id == "toggle"
    ));
}

#[test]
fn toggle_release_outside_same_control_is_cancelled_without_value() {
    let tree = root_with_child(control_node(
        "toggle",
        Rect {
            x: 10.0,
            y: 10.0,
            w: 60.0,
            h: 24.0,
        },
        ControlInteractionSpec::toggle(false),
    ));
    let mut system = ControlInteractionSystem::new();
    let mut interaction = InteractionState::new();

    handle(
        &mut system,
        &tree,
        &mut interaction,
        AppEvent::MousePress {
            x: 20.0,
            y: 20.0,
            button: MouseButton::Left,
        },
    );
    let release = handle(
        &mut system,
        &tree,
        &mut interaction,
        AppEvent::MouseRelease {
            x: 120.0,
            y: 120.0,
            button: MouseButton::Left,
        },
    );

    assert!(release.cancel_gesture);
    assert!(release.output.events.is_empty());
}

#[test]
fn select_outputs_next_selection_on_release_inside_same_control() {
    let tree = root_with_child(control_node(
        "select",
        Rect {
            x: 10.0,
            y: 10.0,
            w: 100.0,
            h: 24.0,
        },
        ControlInteractionSpec::select(1, 3),
    ));
    let mut system = ControlInteractionSystem::new();
    let mut interaction = InteractionState::new();

    let press = handle(
        &mut system,
        &tree,
        &mut interaction,
        AppEvent::MousePress {
            x: 20.0,
            y: 20.0,
            button: MouseButton::Left,
        },
    );
    assert!(press.cancel_gesture);
    assert!(press.output.events.is_empty());

    let release = handle(
        &mut system,
        &tree,
        &mut interaction,
        AppEvent::MouseRelease {
            x: 20.0,
            y: 20.0,
            button: MouseButton::Left,
        },
    );

    assert!(release.cancel_gesture);
    assert!(matches!(
        &release.output.events[0],
        GuiEvent::Control(ControlEvent::ValueChanged {
            id,
            value: ControlValue::Selection(2)
        }) if id == "select"
    ));
}

#[test]
fn select_with_no_options_consumes_without_value() {
    let tree = root_with_child(control_node(
        "select",
        Rect {
            x: 10.0,
            y: 10.0,
            w: 100.0,
            h: 24.0,
        },
        ControlInteractionSpec::select(0, 0),
    ));
    let mut system = ControlInteractionSystem::new();
    let mut interaction = InteractionState::new();

    handle(
        &mut system,
        &tree,
        &mut interaction,
        AppEvent::MousePress {
            x: 20.0,
            y: 20.0,
            button: MouseButton::Left,
        },
    );
    let release = handle(
        &mut system,
        &tree,
        &mut interaction,
        AppEvent::MouseRelease {
            x: 20.0,
            y: 20.0,
            button: MouseButton::Left,
        },
    );

    assert!(release.cancel_gesture);
    assert!(release.output.events.is_empty());
}

#[test]
fn slider_press_outputs_snapped_number() {
    let tree = root_with_child(control_node(
        "slider",
        Rect {
            x: 10.0,
            y: 10.0,
            w: 100.0,
            h: 24.0,
        },
        ControlInteractionSpec::slider(0.0, 0.0, 10.0, 1.0),
    ));
    let mut system = ControlInteractionSystem::new();
    let mut interaction = InteractionState::new();

    let output = handle(
        &mut system,
        &tree,
        &mut interaction,
        AppEvent::MousePress {
            x: 66.0,
            y: 20.0,
            button: MouseButton::Left,
        },
    );

    assert!(output.cancel_gesture);
    assert!(matches!(
        &output.output.events[0],
        GuiEvent::Control(ControlEvent::ValueChanged {
            id,
            value: ControlValue::Number(value)
        }) if id == "slider" && (*value - 6.0).abs() < 0.0001
    ));
}

#[test]
fn slider_drag_clamps_to_range() {
    let tree = root_with_child(control_node(
        "slider",
        Rect {
            x: 10.0,
            y: 10.0,
            w: 100.0,
            h: 24.0,
        },
        ControlInteractionSpec::slider(0.0, 0.0, 10.0, 0.0),
    ));
    let mut system = ControlInteractionSystem::new();
    let mut interaction = InteractionState::new();

    handle(
        &mut system,
        &tree,
        &mut interaction,
        AppEvent::MousePress {
            x: 60.0,
            y: 20.0,
            button: MouseButton::Left,
        },
    );
    let output = handle(
        &mut system,
        &tree,
        &mut interaction,
        AppEvent::MouseMove { x: 500.0, y: 20.0 },
    );

    assert!(output.cancel_gesture);
    assert!(matches!(
        &output.output.events[0],
        GuiEvent::Control(ControlEvent::ValueChanged {
            id,
            value: ControlValue::Number(value)
        }) if id == "slider" && (*value - 10.0).abs() < 0.0001
    ));
}
