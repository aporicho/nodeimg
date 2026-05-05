use crate::tree::{HitChain, NodeId, Tree};

use super::TargetDescriptor;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TargetChain {
    targets_from_leaf_to_root: Vec<TargetDescriptor>,
}

impl TargetChain {
    pub(crate) fn from_hit_chain(tree: &Tree, chain: &HitChain) -> Self {
        Self {
            targets_from_leaf_to_root: chain
                .iter()
                .filter_map(|node_id| TargetDescriptor::from_node(tree, node_id))
                .collect(),
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &TargetDescriptor> {
        self.targets_from_leaf_to_root.iter()
    }

    pub(crate) fn input_target(&self) -> Option<NodeId> {
        self.select_input_target()
    }

    pub(crate) fn focus_target(&self) -> Option<NodeId> {
        self.select_input_target()
    }

    pub(crate) fn allows_semantic_drag(&self, index: usize) -> bool {
        self.first_semantic_index()
            .map(|control_index| index <= control_index)
            .unwrap_or(true)
    }

    fn select_input_target(&self) -> Option<NodeId> {
        let mut gesture_target = None;
        let mut focusable_control = None;

        for target in &self.targets_from_leaf_to_root {
            if !target.is_enabled() {
                continue;
            }

            if gesture_target.is_none() && target.has_any_gesture() {
                gesture_target = Some(target.node_id());
            }

            if focusable_control.is_none() && target.is_focusable() {
                focusable_control = Some(target.node_id());
            }
        }

        focusable_control.or(gesture_target)
    }

    fn first_semantic_index(&self) -> Option<usize> {
        self.targets_from_leaf_to_root
            .iter()
            .position(TargetDescriptor::has_own_semantic_role)
    }
}
