use super::layout::{LeafKind, Overflow};
use super::node::{NodeId, NodeKind};
use super::paint_helpers::{
    bezier_control_points, find_node_by_str_id, grid_cells, rect_center, PaintTransform,
};
use super::stacking::children_in_paint_order;
use super::text_layout::resolve_text_paint;
use super::tree::Tree;
use crate::interaction::InteractionState;
use crate::renderer::{Color, Point, RectStyle, Renderer, TextStyle};
use crate::theme::Theme;
use crate::widget::painters::{
    paint_text_leaf_override as paint_widget_text_leaf_override,
    widget_visual_override as paint_widget_visual_override,
};
use crate::widget::state::TextInputStore;
use std::collections::HashMap;
use std::sync::Arc;

/// 连线宽度（local 空间像素，paint 时按 scale 缩放）
const CONNECTION_WIDTH: f32 = 2.0;
pub fn paint(
    tree: &Tree,
    root: NodeId,
    renderer: &mut Renderer,
    interaction: Option<&InteractionState>,
    text_inputs: Option<&TextInputStore>,
    textures: Option<&HashMap<crate::tree::layout::TextureHandle, Arc<wgpu::TextureView>>>,
    theme: &Theme,
) {
    paint_node(
        tree,
        root,
        renderer,
        PaintTransform::identity(),
        interaction,
        text_inputs,
        textures,
        theme,
        None,
    );
}

fn paint_node(
    tree: &Tree,
    node_id: NodeId,
    renderer: &mut Renderer,
    tf: PaintTransform,
    interaction: Option<&InteractionState>,
    text_inputs: Option<&TextInputStore>,
    textures: Option<&HashMap<crate::tree::layout::TextureHandle, Arc<wgpu::TextureView>>>,
    theme: &Theme,
    inherited_text_color: Option<Color>,
) {
    let Some(node) = tree.get(node_id) else {
        return;
    };

    let screen_rect = tf.apply_rect(node.rect);
    let mut child_text_color = inherited_text_color;
    let clip_radius = node
        .decoration
        .as_ref()
        .map(|dec| dec.radius[0])
        .unwrap_or(0.0);
    let should_clip_children = matches!(node.style.overflow, Overflow::Hidden | Overflow::Scroll);

    if let Some((style, text_color)) =
        paint_widget_visual_override(tree, node_id, interaction, theme)
    {
        renderer.draw_rect(screen_rect, &style);
        child_text_color = Some(text_color);
    } else if let Some(dec) = &node.decoration {
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

    if should_clip_children {
        renderer.push_clip(screen_rect, clip_radius);
    }

    // 2. Leaf 分发
    if let NodeKind::Leaf(leaf) = &node.kind {
        match leaf {
            LeafKind::Text {
                content,
                style,
                layout,
            } => {
                if paint_widget_text_leaf_override(
                    tree,
                    node_id,
                    renderer,
                    tf,
                    interaction,
                    text_inputs,
                    theme,
                    content,
                    &with_inherited_text_color(*style, child_text_color),
                ) {
                    return;
                }
                let text_style = scaled_text_style(
                    with_inherited_text_color(*style, child_text_color),
                    tf.scale,
                );
                let resolved = resolve_text_paint(
                    content,
                    &text_style,
                    *layout,
                    screen_rect,
                    |text, style| renderer.text_measurer().measure_with_style(text, style),
                );
                if let Some(bounds) = resolved.bounds {
                    renderer.draw_text_clipped(
                        resolved.pos,
                        &resolved.content,
                        &text_style,
                        bounds,
                    );
                } else {
                    renderer.draw_text(resolved.pos, &resolved.content, &text_style);
                }
            }
            LeafKind::Grid {
                spacing,
                dot_color,
                dot_size,
            } => {
                for p in grid_cells(node.rect, *spacing) {
                    let sp = tf.apply_point(p);
                    renderer.draw_circle(sp, *dot_size * tf.scale, *dot_color);
                }
            }
            LeafKind::Image { texture, .. } => {
                if let Some(view) = textures.and_then(|registry| registry.get(texture)).cloned() {
                    renderer.draw_image(screen_rect, view);
                }
            }
            LeafKind::Circle {
                radius,
                fill,
                stroke,
            } => {
                let center = Point {
                    x: screen_rect.x + screen_rect.w * 0.5,
                    y: screen_rect.y + screen_rect.h * 0.5,
                };
                let scaled_radius = *radius * tf.scale;
                if let Some(stroke) = stroke {
                    renderer.draw_circle(center, scaled_radius, stroke.color);
                }
                if let Some(fill) = fill {
                    let fill_radius = stroke
                        .map(|stroke| scaled_radius - stroke.width * tf.scale)
                        .unwrap_or(scaled_radius)
                        .max(0.0);
                    renderer.draw_circle(center, fill_radius, *fill);
                }
            }
            LeafKind::Connection { from_port, to_port } => {
                let Some(from_rect) = find_node_by_str_id(tree, from_port.as_ref()) else {
                    return;
                };
                let Some(to_rect) = find_node_by_str_id(tree, to_port.as_ref()) else {
                    return;
                };
                let from_p = tf.apply_point(rect_center(from_rect));
                let to_p = tf.apply_point(rect_center(to_rect));
                let ctrl = bezier_control_points(from_p, to_p);
                renderer.draw_curve(ctrl, CONNECTION_WIDTH * tf.scale, theme.colors.connection);
            }
            LeafKind::PendingConnection {
                from_port,
                cursor_canvas,
            } => {
                let Some(from_rect) = find_node_by_str_id(tree, from_port.as_ref()) else {
                    return;
                };
                let from_p = tf.apply_point(rect_center(from_rect));
                let to_p = tf.apply_point(*cursor_canvas);
                let ctrl = bezier_control_points(from_p, to_p);
                renderer.draw_curve(ctrl, CONNECTION_WIDTH * tf.scale, theme.colors.accent);
            }
            _ => {}
        }
    }

    // 3. 复合 Transform 并递归子节点
    let transform_opt = node.style.transform;
    let children = children_in_paint_order(tree, &node.children);
    // node 借用在此处结束（NLL），后面可以重新借 tree
    let child_tf = match transform_opt {
        Some(ref tf_decl) => tf.compose(tf_decl),
        None => tf,
    };
    for child_id in children {
        paint_node(
            tree,
            child_id,
            renderer,
            child_tf,
            interaction,
            text_inputs,
            textures,
            theme,
            child_text_color,
        );
    }

    if should_clip_children {
        renderer.pop_clip();
    }
}

fn with_inherited_text_color(mut style: TextStyle, inherited: Option<Color>) -> TextStyle {
    if let Some(color) = inherited {
        style.color = color;
    }
    style
}

fn scaled_text_style(mut style: TextStyle, scale: f32) -> TextStyle {
    style.size *= scale;
    style
}
