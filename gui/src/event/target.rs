use crate::control::ControlRole;
use crate::tree::{NodeId, Tree};

pub(crate) struct TargetResolver<'a> {
    tree: &'a Tree,
}

impl<'a> TargetResolver<'a> {
    pub(crate) fn new(tree: &'a Tree) -> Self {
        Self { tree }
    }

    pub(crate) fn owner_id(&self, id: &str) -> String {
        let exact = self
            .tree
            .node_by_str(id)
            .and_then(|node_id| self.tree.get(node_id));
        if let Some(node_id) = self.tree.node_by_str(id) {
            if let Some(node) = self.tree.get(node_id) {
                if let Some(owner_id) = node.props.owner_id.as_ref() {
                    return owner_id.to_string();
                }
                if node.props.semantic_role.is_some() {
                    return node.id.to_string();
                }
            }
        }

        for prefix in stable_id_prefixes(id) {
            if let Some(owner) = self
                .tree
                .node_by_str(prefix)
                .and_then(|node_id| self.tree.get(node_id))
            {
                if owner.props.semantic_role.is_some() {
                    return owner.id.to_string();
                }
            }
        }

        if exact.is_some_and(is_retained_interaction_target) {
            return id.to_string();
        }

        id.to_string()
    }

    pub(crate) fn control_role(&self, id: &str) -> Option<ControlRole> {
        let node = self.tree.node_by_str(id)?;
        self.control_role_for_resolved_node(node)
    }

    fn control_role_for_resolved_node(&self, node_id: NodeId) -> Option<ControlRole> {
        let node = self.tree.get(node_id)?;
        node.props.semantic_role
    }
}

fn is_retained_interaction_target(node: &crate::tree::TreeNode) -> bool {
    node.style.hittable
        || node.style.draggable
        || node.style.resizable
        || !node.style.gestures.is_empty()
        || node.props.action_id.is_some()
}

fn stable_id_prefixes(id: &str) -> impl Iterator<Item = &str> {
    id.match_indices("::")
        .map(|(index, _)| &id[..index])
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
}
