use super::{
    InstanceId, SlotTarget, SlotValue, SlotValues, TemplateError, TemplateId, TemplateInstance,
    TemplateRevision, TemplateSlots,
};
use crate::renderer::Rect;
use crate::tree::layout::{
    BoxStyle, Decoration, LayoutDependencyKind, LayoutDependencyScope, LeafKind,
    RelayoutBoundaryReason,
};
use crate::tree::style_patch::apply_style_patch;
use crate::tree::{
    NodeId, NodeKind, RectMoveInvalidation, RepaintBoundaryReason, Tree, TreeNode, TreeNodeBuilder,
};
use std::borrow::Cow;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CompiledTemplate {
    pub id: TemplateId,
    pub revision: TemplateRevision,
    pub root: CompiledNode,
    pub slots: TemplateSlots,
    pub boundaries: BoundaryDeclarations,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct BoundaryDeclarations {
    pub relayout: Vec<&'static str>,
    pub repaint: Vec<&'static str>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CompiledNode {
    pub id_suffix: &'static str,
    pub absolute_id: Option<&'static str>,
    pub kind: CompiledNodeKind,
    pub style: BoxStyle,
    pub decoration: Option<Decoration>,
    pub rect: Rect,
    pub layout_boundary: Option<RelayoutBoundaryReason>,
    pub paint_boundary: Option<RepaintBoundaryReason>,
    pub rect_move_invalidation: RectMoveInvalidation,
    pub children: Vec<CompiledNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum CompiledNodeKind {
    Container,
    Leaf(LeafKind),
}

impl CompiledTemplate {
    pub(crate) fn new(
        id: impl Into<TemplateId>,
        revision: TemplateRevision,
        root: CompiledNode,
    ) -> Self {
        Self {
            id: id.into(),
            revision,
            root,
            slots: TemplateSlots::default(),
            boundaries: BoundaryDeclarations::default(),
        }
    }

    pub(crate) fn with_slots(mut self, slots: TemplateSlots) -> Self {
        self.slots = slots;
        self
    }

    pub(crate) fn with_boundaries(mut self, boundaries: BoundaryDeclarations) -> Self {
        self.boundaries = boundaries;
        self
    }

    pub(crate) fn instantiate(
        &self,
        tree: &mut Tree,
        instance: InstanceId,
        parent: NodeId,
        slots: SlotValues,
    ) -> Result<TemplateInstance, TemplateError> {
        if tree.get(parent).is_none() {
            return Err(TemplateError::MissingParent(parent));
        }

        let mut nodes_by_suffix = HashMap::new();
        let root_node = match mount_compiled_node(
            tree,
            Some(parent),
            instance.as_str(),
            &self.root,
            &mut nodes_by_suffix,
        ) {
            Ok(root_node) => root_node,
            Err(error) => {
                rollback_instance_mount(tree, &nodes_by_suffix);
                return Err(error);
            }
        };

        if let Err(error) = self.apply_slots(tree, &nodes_by_suffix, &slots) {
            rollback_instance_mount(tree, &nodes_by_suffix);
            return Err(error);
        }

        Ok(TemplateInstance {
            template: self.id.clone(),
            template_revision: self.revision,
            instance,
            root_node,
        })
    }

    pub(crate) fn instantiate_root(
        &self,
        tree: &mut Tree,
        instance: InstanceId,
        slots: SlotValues,
    ) -> Result<TemplateInstance, TemplateError> {
        let mut nodes_by_suffix = HashMap::new();
        let root_node = match mount_compiled_node(
            tree,
            None,
            instance.as_str(),
            &self.root,
            &mut nodes_by_suffix,
        ) {
            Ok(root_node) => root_node,
            Err(error) => {
                rollback_instance_mount(tree, &nodes_by_suffix);
                return Err(error);
            }
        };

        if let Err(error) = self.apply_slots(tree, &nodes_by_suffix, &slots) {
            rollback_instance_mount(tree, &nodes_by_suffix);
            return Err(error);
        }

        tree.set_root(root_node);
        Ok(TemplateInstance {
            template: self.id.clone(),
            template_revision: self.revision,
            instance,
            root_node,
        })
    }

    fn apply_slots(
        &self,
        tree: &mut Tree,
        nodes_by_suffix: &HashMap<&'static str, NodeId>,
        slots: &SlotValues,
    ) -> Result<(), TemplateError> {
        for (slot, _) in slots.iter() {
            if self
                .slots
                .bindings()
                .iter()
                .all(|binding| binding.name != slot)
            {
                return Err(TemplateError::UnsupportedSlot {
                    slot: slot.to_string(),
                    reason: "slot is not declared by the template",
                });
            }
        }

        for binding in self.slots.bindings() {
            let Some(value) = slots.get(binding.name) else {
                continue;
            };
            let Some(node_id) = nodes_by_suffix.get(binding.node_suffix).copied() else {
                return Err(TemplateError::UnsupportedSlot {
                    slot: binding.name.to_string(),
                    reason: "slot target node is not part of the template",
                });
            };
            let result = {
                let Some(node) = tree.get_mut(node_id) else {
                    return Err(TemplateError::UnsupportedSlot {
                        slot: binding.name.to_string(),
                        reason: "slot target node is not live",
                    });
                };
                apply_slot_value(binding.name, binding.target, node, value)?
            };
            if let Some((kind, scope)) = result.layout_dependency {
                tree.record_layout_dependency_change(node_id, kind, scope);
            }
        }
        Ok(())
    }
}

impl CompiledNode {
    pub(crate) fn container(id_suffix: &'static str, style: BoxStyle) -> Self {
        Self::new(id_suffix, CompiledNodeKind::Container, style)
    }

    pub(crate) fn leaf(id_suffix: &'static str, leaf: LeafKind, style: BoxStyle) -> Self {
        Self::new(id_suffix, CompiledNodeKind::Leaf(leaf), style)
    }

    pub(crate) fn new(id_suffix: &'static str, kind: CompiledNodeKind, style: BoxStyle) -> Self {
        Self {
            id_suffix,
            absolute_id: None,
            kind,
            style,
            decoration: None,
            rect: zero_rect(),
            layout_boundary: None,
            paint_boundary: None,
            rect_move_invalidation: RectMoveInvalidation::Layout,
            children: Vec::new(),
        }
    }

    pub(crate) fn with_decoration(mut self, decoration: Decoration) -> Self {
        self.decoration = Some(decoration);
        self
    }

    pub(crate) fn with_layout_boundary(mut self, boundary: RelayoutBoundaryReason) -> Self {
        self.layout_boundary = Some(boundary);
        self
    }

    pub(crate) fn with_paint_boundary(mut self, boundary: RepaintBoundaryReason) -> Self {
        self.paint_boundary = Some(boundary);
        self
    }

    pub(crate) fn with_rect_move_invalidation(
        mut self,
        invalidation: RectMoveInvalidation,
    ) -> Self {
        self.rect_move_invalidation = invalidation;
        self
    }

    pub(crate) fn with_children(mut self, children: Vec<CompiledNode>) -> Self {
        self.children = children;
        self
    }

    pub(crate) fn with_absolute_id(mut self, id: &'static str) -> Self {
        self.absolute_id = Some(id);
        self
    }

    fn to_tree_node(&self, stable_id: String) -> TreeNode {
        let builder = match &self.kind {
            CompiledNodeKind::Container => {
                TreeNodeBuilder::container(stable_id, self.style.clone())
            }
            CompiledNodeKind::Leaf(leaf) => {
                TreeNodeBuilder::leaf(stable_id, leaf.clone(), self.style.clone())
            }
        }
        .maybe_decoration(self.decoration.clone())
        .rect(self.rect)
        .rect_move_invalidation(self.rect_move_invalidation);
        let builder = match self.layout_boundary {
            Some(boundary) => builder.layout_boundary(boundary),
            None => builder,
        };
        let builder = match self.paint_boundary {
            Some(boundary) => builder.paint_boundary(boundary),
            None => builder,
        };
        builder.build()
    }
}

fn mount_compiled_node(
    tree: &mut Tree,
    parent: Option<NodeId>,
    instance_id: &str,
    node: &CompiledNode,
    nodes_by_suffix: &mut HashMap<&'static str, NodeId>,
) -> Result<NodeId, TemplateError> {
    let stable_id = node
        .absolute_id
        .map(str::to_string)
        .unwrap_or_else(|| format!("{instance_id}{}", node.id_suffix));
    let node_id = tree.insert_checked(node.to_tree_node(stable_id))?;
    if let Some(parent) = parent {
        if !tree.append_child(parent, node_id) {
            return Err(TemplateError::MissingParent(parent));
        }
    }
    nodes_by_suffix.insert(node.id_suffix, node_id);
    for child in &node.children {
        mount_compiled_node(tree, Some(node_id), instance_id, child, nodes_by_suffix)?;
    }
    Ok(node_id)
}

fn rollback_instance_mount(tree: &mut Tree, nodes_by_suffix: &HashMap<&'static str, NodeId>) {
    let Some(root) = nodes_by_suffix.get("").copied() else {
        return;
    };
    tree.detach_from_parent(root);
    tree.remove(root);
}

fn apply_slot_value(
    slot: &'static str,
    target: SlotTarget,
    node: &mut TreeNode,
    value: &SlotValue,
) -> Result<SlotApplyResult, TemplateError> {
    match (target, value) {
        (SlotTarget::TextContent, SlotValue::Text(value)) => {
            let NodeKind::Leaf(LeafKind::Text { content, .. }) = &mut node.kind else {
                return Err(TemplateError::UnsupportedSlot {
                    slot: slot.to_string(),
                    reason: "text slot target is not a text leaf",
                });
            };
            let mut result = SlotApplyResult::default();
            if content != value {
                *content = value.clone();
                node.paint_meta.bump_text();
                result.layout_dependency = Some((
                    LayoutDependencyKind::Text,
                    LayoutDependencyScope::RelayoutBoundary,
                ));
            }
            Ok(result)
        }
        (SlotTarget::Rect, SlotValue::Rect(rect)) => {
            node.rect = *rect;
            Ok(SlotApplyResult {
                layout_dependency: Some((
                    LayoutDependencyKind::ExplicitRect,
                    LayoutDependencyScope::RelayoutBoundary,
                )),
            })
        }
        (SlotTarget::Style, SlotValue::Style(patch)) => {
            let effect = apply_style_patch(node, patch.clone());
            Ok(SlotApplyResult {
                layout_dependency: effect.layout_dependency,
            })
        }
        (SlotTarget::Visible, SlotValue::Visible(visible)) => {
            node.local_runtime.visible = *visible;
            Ok(SlotApplyResult::default())
        }
        (SlotTarget::ZIndex, SlotValue::ZIndex(z_index)) => {
            node.style.z_index = *z_index;
            node.paint_meta.bump_paint_order();
            Ok(SlotApplyResult {
                layout_dependency: Some((
                    LayoutDependencyKind::Style,
                    LayoutDependencyScope::LocalNode,
                )),
            })
        }
        (SlotTarget::ConnectionEndpoints, SlotValue::Connection { from_port, to_port }) => {
            let NodeKind::Leaf(LeafKind::Connection {
                from_port: target_from,
                to_port: target_to,
            }) = &mut node.kind
            else {
                return Err(TemplateError::UnsupportedSlot {
                    slot: slot.to_string(),
                    reason: "connection slot target is not a connection leaf",
                });
            };
            *target_from = Cow::Owned(from_port.clone());
            *target_to = Cow::Owned(to_port.clone());
            node.paint_meta.bump_visual();
            Ok(SlotApplyResult::default())
        }
        (
            SlotTarget::PendingConnection,
            SlotValue::PendingConnection {
                from_port,
                cursor_canvas,
            },
        ) => {
            let NodeKind::Leaf(LeafKind::PendingConnection {
                from_port: target_from,
                cursor_canvas: target_cursor,
            }) = &mut node.kind
            else {
                return Err(TemplateError::UnsupportedSlot {
                    slot: slot.to_string(),
                    reason: "pending connection slot target is not a pending connection leaf",
                });
            };
            *target_from = Cow::Owned(from_port.clone());
            *target_cursor = *cursor_canvas;
            node.paint_meta.bump_visual();
            Ok(SlotApplyResult::default())
        }
        _ => Err(TemplateError::UnsupportedSlot {
            slot: slot.to_string(),
            reason: "slot value type does not match slot target",
        }),
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SlotApplyResult {
    layout_dependency: Option<(LayoutDependencyKind, LayoutDependencyScope)>,
}

fn zero_rect() -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
    }
}
