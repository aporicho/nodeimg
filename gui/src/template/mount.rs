use super::{InstanceId, TemplateId};
use super::{TemplateError, TemplateInstance, TemplateRevision};
use crate::tree::{NodeId, Tree, TreeNode};

pub(crate) struct TemplateMountCx<'a> {
    tree: &'a mut Tree,
    mounted: Vec<NodeId>,
    committed: bool,
}

impl<'a> TemplateMountCx<'a> {
    pub(crate) fn new(tree: &'a mut Tree) -> Self {
        Self {
            tree,
            mounted: Vec::new(),
            committed: false,
        }
    }

    pub(crate) fn child(
        &mut self,
        parent: NodeId,
        node: impl Into<TreeNode>,
    ) -> Result<NodeId, TemplateError> {
        let id = self.tree.insert_checked(node.into())?;
        self.mounted.push(id);
        if !self.tree.append_child(parent, id) {
            return Err(TemplateError::MissingParent(parent));
        }
        Ok(id)
    }

    pub(crate) fn commit_instance(
        mut self,
        template: TemplateId,
        template_revision: TemplateRevision,
        instance: InstanceId,
        root_node: NodeId,
    ) -> TemplateInstance {
        self.committed = true;
        TemplateInstance {
            template,
            template_revision,
            instance,
            root_node,
        }
    }

    fn rollback(&mut self) {
        for id in self.mounted.iter().rev().copied() {
            self.tree.detach_from_parent(id);
            self.tree.remove(id);
        }
        self.mounted.clear();
    }
}

impl Drop for TemplateMountCx<'_> {
    fn drop(&mut self) {
        if !self.committed {
            self.rollback();
        }
    }
}
