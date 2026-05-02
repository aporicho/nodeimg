use crate::tree::{HitChain, NodeId, Tree};

pub(crate) fn interactive_target(tree: &Tree, chain: &HitChain) -> Option<NodeId> {
    let mut gesture_target = None;
    let mut focusable_control = None;

    for node_id in chain.iter() {
        let Some(node) = tree.get(node_id) else {
            continue;
        };

        if !node.props.enabled {
            continue;
        }

        if gesture_target.is_none() && !node.style.gestures.is_empty() {
            gesture_target = Some(node_id);
        }

        if focusable_control.is_none()
            && node
                .props
                .semantic_role
                .is_some_and(|role| role.is_focusable())
        {
            focusable_control = Some(node_id);
        }
    }

    focusable_control.or(gesture_target)
}

pub(crate) fn input_target(tree: &Tree, chain: &HitChain) -> Option<NodeId> {
    let mut gesture_target = None;
    let mut focusable_control = None;

    for node_id in chain.iter() {
        let Some(node) = tree.get(node_id) else {
            continue;
        };

        if !node.props.enabled {
            continue;
        }

        if gesture_target.is_none() && !node.style.gestures.is_empty() {
            gesture_target = Some(node_id);
        }

        if focusable_control.is_none()
            && node
                .props
                .semantic_role
                .is_some_and(|role| role.is_focusable())
        {
            focusable_control = Some(node_id);
        }
    }

    focusable_control.or(gesture_target)
}
