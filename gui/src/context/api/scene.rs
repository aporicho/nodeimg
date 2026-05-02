use super::super::{Context, RetainedRootIds};
use crate::geometry::TransformSpec;
use crate::renderer::Rect;
use crate::scene::SceneMutation;
use crate::tree::{Invalidation, MutationError};

pub struct SceneApi<'a> {
    pub(in crate::context) ctx: &'a mut Context,
}

impl SceneApi<'_> {
    pub fn ensure_retained_root(
        &mut self,
        viewport: Rect,
    ) -> Result<RetainedRootIds, MutationError> {
        self.ctx.ensure_retained_root(viewport)
    }

    pub fn apply(&mut self, mutation: SceneMutation) -> Result<Invalidation, MutationError> {
        self.ctx.apply_mutation(mutation)
    }

    pub fn apply_many(
        &mut self,
        mutations: impl IntoIterator<Item = SceneMutation>,
    ) -> Result<Vec<Invalidation>, MutationError> {
        self.ctx.apply_mutations(mutations)
    }

    pub fn update_canvas_transform(
        &mut self,
        transform: TransformSpec,
    ) -> Result<(), MutationError> {
        self.ctx.update_canvas_transform(transform)
    }
}
