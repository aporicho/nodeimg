use crate::widget::desc::Desc;
use crate::widget::node::{NodeId, PanelNode};
use super::tree::PanelTree;
use crate::renderer::Rect;

/// 对比视图描述和旧树，直接更新树。消费 desc 的所有权。
pub fn reconcile(tree: &mut PanelTree, desc: Desc) {
    if let Some(root_id) = tree.root() {
        reconcile_node(tree, root_id, desc);
    } else {
        let root_id = create_from_desc(tree, desc);
        tree.set_root(root_id);
    }
}

fn reconcile_node(tree: &mut PanelTree, node_id: NodeId, desc: Desc) {
    // 提取子描述和节点类型
    let (_id, desc_children, new_style, new_decoration, new_kind) = desc.into_parts();

    // 属性变化时更新（直接比较，不用 hash）
    if let Some(node) = tree.get_mut(node_id) {
        if !node.props_match(&new_style, &new_decoration, &new_kind) {
            node.style = new_style;
            node.decoration = new_decoration;
            node.kind = new_kind;
        }
    }

    // 递归处理子节点
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
    let (id, child_descs, style, decoration, kind) = desc.into_parts();

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
