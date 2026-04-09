use crate::widget::node::{NodeId, NodeKind};
use super::tree::PanelTree;
use crate::widget::layout::LeafKind;
use crate::renderer::{Color, Point, Renderer, RectStyle, TextStyle};

pub fn paint(tree: &PanelTree, root: NodeId, renderer: &mut Renderer) {
    paint_node(tree, root, renderer);
}

fn paint_node(tree: &PanelTree, node_id: NodeId, renderer: &mut Renderer) {
    let Some(node) = tree.get(node_id) else { return };
    let rect = node.rect;

    if let Some(dec) = &node.decoration {
        renderer.draw_rect(rect, &RectStyle {
            color: dec.background.unwrap_or(Color::TRANSPARENT),
            border: dec.border,
            radius: dec.radius,
            shadow: None,
        });
    }

    if let NodeKind::Leaf(LeafKind::Text { content, font_size, color }) = &node.kind {
        renderer.draw_text(
            Point { x: rect.x, y: rect.y },
            content,
            &TextStyle { color: *color, size: *font_size },
        );
    }

    let children: Vec<NodeId> = tree
        .get(node_id)
        .map(|n| n.children.clone())
        .unwrap_or_default();
    for child_id in children {
        paint_node(tree, child_id, renderer);
    }
}
