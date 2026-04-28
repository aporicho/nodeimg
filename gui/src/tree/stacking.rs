use super::node::NodeId;
use super::tree::Tree;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StackingChild {
    source_index: usize,
    node_id: NodeId,
    z_index: i32,
}

pub(crate) fn children_in_paint_order(tree: &Tree, children: &[NodeId]) -> Vec<NodeId> {
    sorted_children(tree, children, false)
}

pub(crate) fn children_in_hit_order(tree: &Tree, children: &[NodeId]) -> Vec<NodeId> {
    sorted_children(tree, children, true)
}

fn sorted_children(tree: &Tree, children: &[NodeId], reverse: bool) -> Vec<NodeId> {
    let mut indexed = children
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(source_index, node_id)| {
            tree.get(node_id).map(|node| StackingChild {
                source_index,
                node_id,
                z_index: node.style.z_index,
            })
        })
        .collect::<Vec<_>>();

    indexed.sort_by(|a, b| {
        a.z_index
            .cmp(&b.z_index)
            .then_with(|| a.source_index.cmp(&b.source_index))
    });

    if reverse {
        indexed.reverse();
    }

    indexed.into_iter().map(|child| child.node_id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::Rect;
    use crate::tree::layout::BoxStyle;
    use crate::tree::node::{NodeKind, NodeLocalRuntime, TreeNode};
    use crate::tree::{NodeProps, RuntimeSlots};
    use std::borrow::Cow;

    fn container(z_index: i32) -> TreeNode {
        TreeNode {
            id: Cow::Borrowed("test").into(),
            props: NodeProps::default(),
            style: BoxStyle {
                z_index,
                ..Default::default()
            },
            decoration: None,
            kind: NodeKind::Container,
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            },
            children: Vec::new(),
            local_runtime: NodeLocalRuntime::default(),
            layout_meta: Default::default(),
            paint_meta: Default::default(),
            runtime_slots: RuntimeSlots::default(),
        }
    }

    #[test]
    fn paint_order_sorts_by_z_index_then_source_order() {
        let mut tree = Tree::new();
        let first = tree.insert(container(2));
        let second = tree.insert(container(1));
        let third = tree.insert(container(2));

        let actual = children_in_paint_order(&tree, &[first, second, third]);

        assert_eq!(actual, vec![second, first, third]);
    }

    #[test]
    fn hit_order_is_reverse_paint_order() {
        let mut tree = Tree::new();
        let first = tree.insert(container(2));
        let second = tree.insert(container(1));
        let third = tree.insert(container(2));

        let actual = children_in_hit_order(&tree, &[first, second, third]);

        assert_eq!(actual, vec![third, first, second]);
    }
}
