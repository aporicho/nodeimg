use crate::widget::desc::Desc;
use crate::widget::node::{NodeId, NodeKind, PanelNode};
use super::tree::PanelTree;
use crate::renderer::Rect;

pub fn reconcile(tree: &mut PanelTree, desc: Desc) {
    if let Some(root_id) = tree.root() {
        reconcile_node(tree, root_id, desc);
    } else {
        let root_id = create_from_desc(tree, desc);
        tree.set_root(root_id);
    }
}

fn reconcile_node(tree: &mut PanelTree, node_id: NodeId, desc: Desc) {
    match desc {
        Desc::Container { id: _, style, decoration, children } => {
            if let Some(node) = tree.get_mut(node_id) {
                if !node.props_match(&style, &decoration, &NodeKind::Container) {
                    node.style = style;
                    node.decoration = decoration;
                    node.kind = NodeKind::Container;
                }
            }
            reconcile_children(tree, node_id, children);
        }
        Desc::Leaf { id: _, style, kind } => {
            let new_kind = NodeKind::Leaf(kind);
            if let Some(node) = tree.get_mut(node_id) {
                if !node.props_match(&style, &None, &new_kind) {
                    node.style = style;
                    node.decoration = None;
                    node.kind = new_kind;
                }
            }
        }
        Desc::Widget { id, props } => {
            let props_changed = tree.get(node_id)
                .map(|n| match &n.kind {
                    NodeKind::Widget(old) => !old.props_eq(props.as_ref()),
                    _ => true,
                })
                .unwrap_or(true);

            if props_changed {
                let wb = props.build(&id);
                if let Some(node) = tree.get_mut(node_id) {
                    node.style = wb.style;
                    node.decoration = wb.decoration;
                    node.kind = NodeKind::Widget(props);
                }
                reconcile_children(tree, node_id, wb.children);
            }
        }
    }
}

fn reconcile_children(tree: &mut PanelTree, node_id: NodeId, desc_children: Vec<Desc>) {
    let old_children: Vec<NodeId> = tree
        .get(node_id)
        .map(|n| n.children.clone())
        .unwrap_or_default();

    let mut new_children = Vec::new();
    let mut old_map: Vec<(NodeId, String)> = old_children
        .iter()
        .filter_map(|&id| tree.get(id).map(|n| (id, n.id.as_ref().to_owned())))
        .collect();

    for child_desc in desc_children {
        let child_id_str = child_desc.id();
        if let Some(pos) = old_map.iter().position(|(_, id)| id == child_id_str) {
            let (existing_id, _) = old_map.remove(pos);
            reconcile_node(tree, existing_id, child_desc);
            new_children.push(existing_id);
        } else {
            let new_id = create_from_desc(tree, child_desc);
            new_children.push(new_id);
        }
    }

    for (old_id, _) in old_map {
        tree.remove(old_id);
    }

    if let Some(node) = tree.get_mut(node_id) {
        node.children = new_children;
    }
}

fn create_from_desc(tree: &mut PanelTree, desc: Desc) -> NodeId {
    let (id, style, decoration, kind, child_descs) = match desc {
        Desc::Container { id, style, decoration, children } => {
            (id, style, decoration, NodeKind::Container, children)
        }
        Desc::Leaf { id, style, kind } => {
            (id, style, None, NodeKind::Leaf(kind), Vec::new())
        }
        Desc::Widget { id, props } => {
            let wb = props.build(&id);
            (id, wb.style, wb.decoration, NodeKind::Widget(props), wb.children)
        }
    };

    let node_id = tree.insert(PanelNode {
        id,
        style,
        decoration,
        kind,
        rect: Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 },
        children: Vec::new(),
        scroll_offset: 0.0,
        content_height: 0.0,
    });

    let child_ids: Vec<NodeId> = child_descs
        .into_iter()
        .map(|d| create_from_desc(tree, d))
        .collect();

    if let Some(node) = tree.get_mut(node_id) {
        node.children = child_ids;
    }

    node_id
}
