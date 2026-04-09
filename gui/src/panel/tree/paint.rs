use super::node::{NodeId, NodeKind};
use super::tree::PanelTree;
use crate::widget::layout::LeafKind;
use crate::renderer::{Color, Point, Renderer, RectStyle, TextStyle};

pub fn paint(tree: &PanelTree, root: NodeId, renderer: &mut Renderer) {
    paint_node(tree, root, renderer);
}

fn paint_node(tree: &PanelTree, node_id: NodeId, renderer: &mut Renderer) {
    let Some(node) = tree.get(node_id) else { return };
    let rect = node.rect;

    match &node.kind {
        NodeKind::Container { decoration, .. } => {
            if let Some(dec) = decoration {
                renderer.draw_rect(rect, &RectStyle {
                    color: dec.background.unwrap_or(Color::TRANSPARENT),
                    border: dec.border,
                    radius: dec.radius,
                    shadow: None,
                });
            }
        }
        NodeKind::Leaf { kind, .. } => {
            match kind {
                LeafKind::Text { content, font_size, color } => {
                    renderer.draw_text(
                        Point { x: rect.x, y: rect.y },
                        content,
                        &TextStyle { color: *color, size: *font_size },
                    );
                }
            }
        }
    }

    let children: Vec<NodeId> = tree
        .get(node_id)
        .map(|n| n.children.clone())
        .unwrap_or_default();
    for child_id in children {
        paint_node(tree, child_id, renderer);
    }
}
