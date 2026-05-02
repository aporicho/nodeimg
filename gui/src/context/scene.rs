use super::Context;
use crate::geometry::TransformSpec;
use crate::tree::{Invalidation, MutationError, StylePatch, TreeMutation};

impl Context {
    pub(crate) fn apply_mutation(
        &mut self,
        mutation: TreeMutation,
    ) -> Result<Invalidation, MutationError> {
        self.tree.apply_mutation(&self.template_registry, mutation)
    }

    pub(crate) fn apply_mutations(
        &mut self,
        mutations: impl IntoIterator<Item = TreeMutation>,
    ) -> Result<Vec<Invalidation>, MutationError> {
        self.tree
            .apply_mutations(&self.template_registry, mutations)
    }

    pub(crate) fn update_canvas_transform(
        &mut self,
        transform: TransformSpec,
    ) -> Result<(), MutationError> {
        let Some(canvas_root) = self.tree.node_by_str("canvas_root") else {
            return Err(MutationError::MissingNode(usize::MAX));
        };
        self.apply_mutation(TreeMutation::SetStyle {
            node: canvas_root,
            patch: StylePatch {
                transform: Some(Some(transform)),
                ..StylePatch::default()
            },
        })?;
        Ok(())
    }
}
