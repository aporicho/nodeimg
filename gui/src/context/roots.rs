use super::Context;
use crate::renderer::Rect;
use crate::template::{InstanceId, SlotValue, SlotValues, TemplateId, WORKSPACE_ROOT_TEMPLATE};
use crate::tree::{MutationError, NodeId, TreeMutation};

pub struct RetainedRootIds {
    pub root: NodeId,
    pub canvas_root: NodeId,
    pub canvas_grid: NodeId,
    pub canvas_connections: NodeId,
    pub panel_root: NodeId,
    pub overlay_root: NodeId,
}

#[allow(dead_code)]

impl Context {
    pub(crate) fn ensure_retained_root(
        &mut self,
        viewport: Rect,
    ) -> Result<RetainedRootIds, MutationError> {
        if self.tree.node_by_str("root").is_none() {
            self.template_registry.instantiate_root(
                &mut self.tree,
                TemplateId::from(WORKSPACE_ROOT_TEMPLATE),
                InstanceId::from("root"),
                retained_root_slots(viewport),
            )?;
            self.tree.mark_dirty(
                self.tree.root().expect("retained root must be set"),
                crate::tree::DirtyFlags::STRUCTURE
                    | crate::tree::DirtyFlags::LAYOUT
                    | crate::tree::DirtyFlags::HIT
                    | crate::tree::DirtyFlags::PAINT,
            );
        } else {
            let root = self.tree.node_by_str("root").expect("root");
            let canvas = self.tree.node_by_str("canvas_root").expect("canvas root");
            let panel = self.tree.node_by_str("panel_root").expect("panel root");
            let overlay = self
                .tree
                .node_by_str("__overlay_root")
                .expect("overlay root");
            for mutation in [
                TreeMutation::SetRect {
                    node: root,
                    rect: viewport,
                },
                TreeMutation::SetRect {
                    node: canvas,
                    rect: viewport,
                },
                TreeMutation::SetRect {
                    node: panel,
                    rect: viewport,
                },
                TreeMutation::SetRect {
                    node: overlay,
                    rect: viewport,
                },
            ] {
                self.tree
                    .apply_mutation(&self.template_registry, mutation)?;
            }
        }
        self.sync_overlay_tree();
        self.retained_root_ids()
            .ok_or(MutationError::MissingNode(usize::MAX))
    }

    pub(crate) fn retained_root_ids(&self) -> Option<RetainedRootIds> {
        Some(RetainedRootIds {
            root: self.tree.node_by_str("root")?,
            canvas_root: self.tree.node_by_str("canvas_root")?,
            canvas_grid: self.tree.node_by_str("canvas_grid")?,
            canvas_connections: self.tree.node_by_str("canvas_connections")?,
            panel_root: self.tree.node_by_str("panel_root")?,
            overlay_root: self.tree.node_by_str("__overlay_root")?,
        })
    }
}

fn retained_root_slots(viewport: Rect) -> SlotValues {
    SlotValues::new()
        .with("root_rect", SlotValue::Rect(viewport))
        .with("canvas_rect", SlotValue::Rect(viewport))
        .with("grid_rect", SlotValue::Rect(viewport))
        .with("panel_rect", SlotValue::Rect(viewport))
        .with("overlay_rect", SlotValue::Rect(viewport))
}
