use crate::tree::Tree;

use super::TargetDescriptor;

pub(crate) struct TargetOwnerResolver<'a> {
    tree: &'a Tree,
}

impl<'a> TargetOwnerResolver<'a> {
    pub(crate) fn new(tree: &'a Tree) -> Self {
        Self { tree }
    }

    pub(crate) fn owner_id(&self, id: &str) -> String {
        let exact = self
            .tree
            .node_by_str(id)
            .and_then(|node_id| TargetDescriptor::from_node(self.tree, node_id));
        if let Some(target) = exact.as_ref() {
            if let Some(owner_id) = target.owner_id() {
                return owner_id.to_string();
            }
            if target.has_own_semantic_role() {
                return target.stable_id().to_string();
            }
        }

        for prefix in stable_id_prefixes(id) {
            if let Some(owner) = self
                .tree
                .node_by_str(prefix)
                .and_then(|node_id| TargetDescriptor::from_node(self.tree, node_id))
            {
                if owner.has_own_semantic_role() {
                    return owner.stable_id().to_string();
                }
            }
        }

        if exact.is_some_and(|target| target.is_retained_interaction_target()) {
            return id.to_string();
        }

        id.to_string()
    }
}

fn stable_id_prefixes(id: &str) -> impl Iterator<Item = &str> {
    id.match_indices("::")
        .map(|(index, _)| &id[..index])
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
}
