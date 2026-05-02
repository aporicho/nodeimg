use crate::tree::{NodeId, NodeKind, Tree};
use crate::widget::WidgetRole;

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
                if matches!(&node.kind, NodeKind::Widget(_)) || node.props.semantic_role.is_some() {
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
                if matches!(&owner.kind, NodeKind::Widget(_)) || owner.props.semantic_role.is_some()
                {
                    return owner.id.to_string();
                }
            }
        }

        if exact.is_some_and(is_retained_interaction_target) {
            return id.to_string();
        }

        id.to_string()
    }

    #[allow(dead_code)]
    pub(crate) fn owner_node(&self, node_id: NodeId) -> Option<NodeId> {
        let node = self.tree.get(node_id)?;
        if matches!(&node.kind, NodeKind::Widget(_)) {
            return Some(node_id);
        }

        let owner_id = self.owner_id(node.id.as_ref());
        self.tree.node_by_str(&owner_id)
    }

    pub(crate) fn widget_role(&self, id: &str) -> Option<WidgetRole> {
        let node = self.tree.node_by_str(id)?;
        self.widget_role_for_resolved_node(node)
    }

    #[cfg(test)]
    pub(crate) fn widget_role_for_node(&self, node_id: NodeId) -> Option<WidgetRole> {
        let owner = self.owner_node(node_id)?;
        self.widget_role_for_resolved_node(owner)
    }

    fn widget_role_for_resolved_node(&self, node_id: NodeId) -> Option<WidgetRole> {
        let node = self.tree.get(node_id)?;
        match &node.kind {
            NodeKind::Widget(props) => Some(props.role()),
            _ => node.props.semantic_role,
        }
    }
}

fn is_retained_interaction_target(node: &crate::tree::TreeNode) -> bool {
    node.style.hittable == Some(true)
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
