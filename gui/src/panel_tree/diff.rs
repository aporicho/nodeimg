use super::desc::Desc;
use super::node::{NodeId, NodeKind, PanelNode};
use super::tree::PanelTree;
use crate::renderer::Rect;

/// 对比视图描述和旧树，直接更新树。
pub fn reconcile(tree: &mut PanelTree, desc: &Desc) {
    if let Some(root_id) = tree.root() {
        reconcile_node(tree, root_id, desc);
    } else {
        // 首次：从描述创建整棵树
        let root_id = create_from_desc(tree, desc);
        tree.set_root(root_id);
    }
}

fn reconcile_node(tree: &mut PanelTree, node_id: NodeId, desc: &Desc) {
    let new_hash = desc.props_hash();

    // 检查属性是否变化
    let old_hash = tree.get(node_id).map(|n| n.props_hash).unwrap_or(0);
    if old_hash != new_hash {
        // 属性变了，更新节点
        if let Some(node) = tree.get_mut(node_id) {
            node.props_hash = new_hash;
            node.kind = kind_from_desc(desc);
        }
    }

    // 递归处理子节点
    let desc_children = match desc {
        Desc::Column { children } => children.as_slice(),
        _ => &[],
    };

    let old_children: Vec<NodeId> = tree.get(node_id)
        .map(|n| n.children.clone())
        .unwrap_or_default();

    // 按位置 + id 匹配
    let mut new_children = Vec::new();
    let mut old_map: Vec<(NodeId, &'static str)> = old_children.iter()
        .filter_map(|&id| tree.get(id).map(|n| (id, n.id)))
        .collect();

    for child_desc in desc_children {
        let child_id_str = child_desc.id();
        // 在旧节点中查找同 id 的节点
        if let Some(pos) = old_map.iter().position(|(_, id)| *id == child_id_str) {
            let (existing_id, _) = old_map.remove(pos);
            reconcile_node(tree, existing_id, child_desc);
            new_children.push(existing_id);
        } else {
            // 新节点
            let new_id = create_from_desc(tree, child_desc);
            new_children.push(new_id);
        }
    }

    // 删除旧树中多余的节点
    for (old_id, _) in old_map {
        tree.remove(old_id);
    }

    // 更新子节点列表
    if let Some(node) = tree.get_mut(node_id) {
        node.children = new_children;
    }
}

fn create_from_desc(tree: &mut PanelTree, desc: &Desc) -> NodeId {
    let kind = kind_from_desc(desc);
    let id = desc.id();
    let props_hash = desc.props_hash();

    let node_id = tree.insert(PanelNode {
        id,
        kind,
        rect: Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 },
        children: Vec::new(),
        props_hash,
    });

    // 递归创建子节点
    let child_descs = match desc {
        Desc::Column { children } => children.as_slice(),
        _ => &[],
    };

    let child_ids: Vec<NodeId> = child_descs.iter()
        .map(|d| create_from_desc(tree, d))
        .collect();

    if let Some(node) = tree.get_mut(node_id) {
        node.children = child_ids;
    }

    node_id
}

fn kind_from_desc(desc: &Desc) -> NodeKind {
    match desc {
        Desc::Column { .. } => NodeKind::Column,
        Desc::Button { label, color, .. } => NodeKind::Button { label, color: *color },
    }
}
