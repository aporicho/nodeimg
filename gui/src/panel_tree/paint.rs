use super::node::{NodeId, NodeKind};
use super::tree::PanelTree;
use crate::renderer::{Color, Point, RectStyle, Renderer, TextStyle};

/// 遍历面板树，为每个节点生成 draw 调用。
pub fn paint(tree: &PanelTree, root: NodeId, renderer: &mut Renderer) {
    paint_node(tree, root, renderer);
}

fn paint_node(tree: &PanelTree, node_id: NodeId, renderer: &mut Renderer) {
    let Some(node) = tree.get(node_id) else { return };

    match &node.kind {
        NodeKind::Column => {
            // Column 无自身绘制，递归子节点
        }
        NodeKind::Button { label, color } => {
            // 按钮背景
            renderer.draw_rect(node.rect, &RectStyle {
                color: *color,
                border: None,
                radius: [8.0; 4],
                shadow: None,
            });

            // 按钮文字（居中）
            let text_x = node.rect.x + 16.0;
            let text_y = node.rect.y + (node.rect.h - 16.0) / 2.0;
            renderer.draw_text(
                Point { x: text_x, y: text_y },
                label,
                &TextStyle { color: Color::WHITE, size: 16.0 },
            );
        }
    }

    // 递归子节点
    let children: Vec<NodeId> = tree.get(node_id)
        .map(|n| n.children.clone())
        .unwrap_or_default();
    for child_id in children {
        paint_node(tree, child_id, renderer);
    }
}

/// 在面板树中查找包含指定坐标的 Button 节点，返回其 id。
pub fn hit_test(tree: &PanelTree, root: NodeId, x: f32, y: f32) -> Option<&'static str> {
    hit_test_node(tree, root, x, y)
}

fn hit_test_node(tree: &PanelTree, node_id: NodeId, x: f32, y: f32) -> Option<&'static str> {
    let Some(node) = tree.get(node_id) else { return None };

    // 检查是否在 rect 内
    let r = &node.rect;
    if x < r.x || x > r.x + r.w || y < r.y || y > r.y + r.h {
        return None;
    }

    // 先递归子节点（子节点在上层）
    for &child_id in &node.children {
        if let Some(id) = hit_test_node(tree, child_id, x, y) {
            return Some(id);
        }
    }

    // 自身是 Button 才返回
    if matches!(node.kind, NodeKind::Button { .. }) {
        return Some(node.id);
    }

    None
}
