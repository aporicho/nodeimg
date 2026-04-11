use super::layout::LeafKind;
use super::node::{NodeId, NodeKind};
use super::paint_helpers::{
    bezier_control_points, find_node_by_str_id, grid_cells, rect_center_left,
    rect_center_right, PaintTransform,
};
use super::tree::Tree;
use crate::renderer::{Color, Point, Renderer, RectStyle, TextStyle};

/// 连线宽度（local 空间像素，paint 时按 scale 缩放）
const CONNECTION_WIDTH: f32 = 2.0;
/// 连线颜色（中性灰）
const CONNECTION_COLOR: Color = Color { r: 0.55, g: 0.58, b: 0.65, a: 1.0 };

pub fn paint(tree: &Tree, root: NodeId, renderer: &mut Renderer) {
    paint_node(tree, root, renderer, PaintTransform::identity());
}

fn paint_node(tree: &Tree, node_id: NodeId, renderer: &mut Renderer, tf: PaintTransform) {
    let Some(node) = tree.get(node_id) else { return };

    let screen_rect = tf.apply_rect(node.rect);

    // 1. Container decoration（使用 screen_rect）
    if let Some(dec) = &node.decoration {
        renderer.draw_rect(
            screen_rect,
            &RectStyle {
                color: dec.background.unwrap_or(Color::TRANSPARENT),
                border: dec.border,
                radius: dec.radius,
                shadow: dec.shadow,
            },
        );
    }

    // 2. Leaf 分发
    if let NodeKind::Leaf(leaf) = &node.kind {
        match leaf {
            LeafKind::Text { content, font_size, color } => {
                renderer.draw_text(
                    Point { x: screen_rect.x, y: screen_rect.y },
                    content,
                    &TextStyle { color: *color, size: *font_size * tf.scale },
                );
            }
            LeafKind::Grid { spacing, dot_color, dot_size } => {
                for p in grid_cells(node.rect, *spacing) {
                    let sp = tf.apply_point(p);
                    renderer.draw_circle(sp, *dot_size * tf.scale, *dot_color);
                }
            }
            LeafKind::Connection { from_port, to_port } => {
                let Some(from_rect) = find_node_by_str_id(tree, from_port.as_ref()) else {
                    return;
                };
                let Some(to_rect) = find_node_by_str_id(tree, to_port.as_ref()) else {
                    return;
                };
                let from_p = tf.apply_point(rect_center_right(from_rect));
                let to_p = tf.apply_point(rect_center_left(to_rect));
                let ctrl = bezier_control_points(from_p, to_p);
                renderer.draw_curve(ctrl, CONNECTION_WIDTH * tf.scale, CONNECTION_COLOR);
            }
            _ => {}
        }
    }

    // 3. 复合 Transform 并递归子节点
    let transform_opt = node.style.transform;
    let children: Vec<NodeId> = node.children.clone();
    // node 借用在此处结束（NLL），后面可以重新借 tree
    let child_tf = match transform_opt {
        Some(ref tf_decl) => tf.compose(tf_decl),
        None => tf,
    };
    for child_id in children {
        paint_node(tree, child_id, renderer, child_tf);
    }
}
