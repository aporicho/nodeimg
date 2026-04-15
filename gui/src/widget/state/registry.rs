use crate::tree::{HitChain, NodeId, NodeKind, Tree};
use crate::widget::atoms::button::ButtonProps;
use crate::widget::atoms::checkbox::CheckboxProps;
use crate::widget::atoms::dropdown::DropdownProps;
use crate::widget::atoms::number_input::NumberInputProps;
use crate::widget::atoms::radio::RadioProps;
use crate::widget::atoms::slider::SliderProps;
use crate::widget::atoms::text_input::TextInputProps;
use crate::widget::atoms::toggle::ToggleProps;
use crate::widget::frameworks::collapsible::CollapsibleProps;

pub fn focusable_nodes(tree: &Tree) -> Vec<NodeId> {
    tree.iter()
        .filter_map(|(id, node)| match &node.kind {
            NodeKind::Widget(props)
                if is_focusable_widget_type(props.widget_type())
                    && !is_disabled(props.as_ref()) =>
            {
                Some(id)
            }
            _ => None,
        })
        .collect()
}

pub fn interactive_target(tree: &Tree, chain: &HitChain) -> Option<NodeId> {
    let mut gesture_target = None;
    let mut focusable_widget = None;

    for node_id in chain.iter() {
        let Some(node) = tree.get(node_id) else {
            continue;
        };

        let widget_enabled = match &node.kind {
            NodeKind::Widget(props) => !is_disabled(props.as_ref()),
            _ => true,
        };
        if !widget_enabled {
            continue;
        }

        if gesture_target.is_none() && !node.style.gestures.is_empty() {
            gesture_target = Some(node_id);
        }

        if focusable_widget.is_none() && is_focusable_node_kind(&node.kind) {
            focusable_widget = Some(node_id);
        }
    }

    focusable_widget.or(gesture_target)
}

pub fn input_target(tree: &Tree, chain: &HitChain) -> Option<NodeId> {
    let mut gesture_target = None;
    let mut focusable_widget = None;

    for node_id in chain.iter() {
        let Some(node) = tree.get(node_id) else {
            continue;
        };

        let widget_enabled = match &node.kind {
            NodeKind::Widget(props) => !is_disabled(props.as_ref()),
            _ => true,
        };
        if !widget_enabled {
            continue;
        }

        if gesture_target.is_none() && !node.style.gestures.is_empty() {
            gesture_target = Some(node_id);
        }

        if focusable_widget.is_none() && is_focusable_node_kind(&node.kind) {
            focusable_widget = Some(node_id);
        }
    }

    focusable_widget.or(gesture_target)
}

fn is_focusable_node_kind(kind: &NodeKind) -> bool {
    match kind {
        NodeKind::Widget(props) => is_focusable_widget_type(props.widget_type()),
        _ => false,
    }
}

fn is_focusable_widget_type(widget_type: &str) -> bool {
    matches!(
        widget_type,
        "Button"
            | "Checkbox"
            | "Collapsible"
            | "Dropdown"
            | "NumberInput"
            | "Radio"
            | "Slider"
            | "TextInput"
            | "Toggle"
    )
}

fn is_disabled(props: &dyn crate::widget::props::WidgetProps) -> bool {
    props
        .as_any()
        .downcast_ref::<ButtonProps>()
        .map(|p| p.disabled)
        .or_else(|| {
            props
                .as_any()
                .downcast_ref::<CheckboxProps>()
                .map(|p| p.disabled)
        })
        .or_else(|| {
            props
                .as_any()
                .downcast_ref::<SliderProps>()
                .map(|p| p.disabled)
        })
        .or_else(|| {
            props
                .as_any()
                .downcast_ref::<ToggleProps>()
                .map(|p| p.disabled)
        })
        .or_else(|| {
            props
                .as_any()
                .downcast_ref::<NumberInputProps>()
                .map(|p| p.disabled)
        })
        .or_else(|| {
            props
                .as_any()
                .downcast_ref::<RadioProps>()
                .map(|p| p.disabled)
        })
        .or_else(|| {
            props
                .as_any()
                .downcast_ref::<TextInputProps>()
                .map(|p| p.disabled)
        })
        .or_else(|| {
            props
                .as_any()
                .downcast_ref::<DropdownProps>()
                .map(|p| p.disabled)
        })
        .or_else(|| {
            props
                .as_any()
                .downcast_ref::<CollapsibleProps>()
                .map(|p| p.disabled)
        })
        .unwrap_or(false)
}
