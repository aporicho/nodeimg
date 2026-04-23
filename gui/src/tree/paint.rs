use super::layout::{LeafKind, Overflow};
use super::node::{NodeId, NodeKind};
use super::paint_helpers::{
    connection_path, find_node_by_str_id, grid_cells, leaf_path_to_screen, rect_center,
    scaled_path_style, PaintTransform,
};
use super::paint_target::{CustomPaintCx, PaintTarget, RendererPaintTarget};
use super::stacking::children_in_paint_order;
use super::text_layout::resolve_text_paint;
use super::tree::Tree;
use crate::interaction::InteractionState;
use crate::renderer::{
    Border, Color, PathData, PathStyle, Point, Rect, RectStyle, Renderer, Shadow, Stroke, TextStyle,
};
use crate::runtime::TextureResource;
use crate::theme::Theme;
use crate::widget::painters::{
    paint_text_leaf_override as paint_widget_text_leaf_override,
    widget_visual_override as paint_widget_visual_override,
};
use crate::widget::state::TextInputStore;
use std::collections::HashMap;

/// 连线宽度（local 空间像素，paint 时按 scale 缩放）
const CONNECTION_WIDTH: f32 = 2.0;
pub(crate) fn paint(
    tree: &Tree,
    root: NodeId,
    renderer: &mut Renderer,
    interaction: Option<&InteractionState>,
    text_inputs: Option<&TextInputStore>,
    textures: Option<&HashMap<crate::tree::layout::TextureHandle, TextureResource>>,
    theme: &Theme,
) {
    let mut target = RendererPaintTarget::new(renderer, textures);
    paint_to_target(tree, root, &mut target, interaction, text_inputs, theme);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_to_target(
    tree: &Tree,
    root: NodeId,
    target: &mut dyn PaintTarget,
    interaction: Option<&InteractionState>,
    text_inputs: Option<&TextInputStore>,
    theme: &Theme,
) {
    paint_node(
        tree,
        root,
        target,
        PaintTransform::identity(),
        interaction,
        text_inputs,
        theme,
        None,
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_node(
    tree: &Tree,
    node_id: NodeId,
    target: &mut dyn PaintTarget,
    tf: PaintTransform,
    interaction: Option<&InteractionState>,
    text_inputs: Option<&TextInputStore>,
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
        .map(|dec| dec.radius[0] * tf.scale)
        .unwrap_or(0.0);
    let should_clip_children = matches!(node.style.overflow, Overflow::Hidden | Overflow::Scroll);

    if let Some((style, text_color)) =
        paint_widget_visual_override(tree, node_id, interaction, theme)
    {
        target.draw_rect(screen_rect, scaled_rect_style(&style, tf.scale));
        child_text_color = Some(text_color);
    } else if let Some(dec) = &node.decoration {
        let style = RectStyle {
            color: dec.background.unwrap_or(Color::TRANSPARENT),
            border: dec.border,
            radius: dec.radius,
            shadow: dec.shadow,
        };
        target.draw_rect(screen_rect, scaled_rect_style(&style, tf.scale));
    }

    if should_clip_children {
        target.push_clip(screen_rect, clip_radius);
    }

    // 2. Leaf 分发
    if let NodeKind::Leaf(leaf) = &node.kind {
        match leaf {
            LeafKind::Text {
                content,
                style,
                layout,
            } => {
                if !paint_widget_text_leaf_override(
                    tree,
                    node_id,
                    target,
                    tf,
                    interaction,
                    text_inputs,
                    theme,
                    content,
                    &with_inherited_text_color(*style, child_text_color),
                ) {
                    let text_style = scaled_text_style(
                        with_inherited_text_color(*style, child_text_color),
                        tf.scale,
                    );
                    let resolved = resolve_text_paint(
                        content,
                        &text_style,
                        *layout,
                        screen_rect,
                        |text, style| target.measure_text(text, style),
                    );
                    if let Some(bounds) = resolved.bounds {
                        target.draw_text_clipped(
                            resolved.pos,
                            &resolved.content,
                            text_style,
                            bounds,
                        );
                    } else {
                        target.draw_text(resolved.pos, &resolved.content, text_style);
                    }
                }
            }
            LeafKind::Grid {
                spacing,
                dot_color,
                dot_size,
            } => {
                for p in grid_cells(node.rect, *spacing) {
                    let sp = tf.apply_point(p);
                    target.draw_circle(sp, *dot_size * tf.scale, *dot_color);
                }
            }
            LeafKind::Image { texture, style } => {
                target.draw_image(screen_rect, *texture, *style);
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
                    target.draw_circle(center, scaled_radius, stroke.color);
                }
                if let Some(fill) = fill {
                    let fill_radius = stroke
                        .map(|stroke| scaled_radius - stroke.width * tf.scale)
                        .unwrap_or(scaled_radius)
                        .max(0.0);
                    target.draw_circle(center, fill_radius, *fill);
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
                target.draw_path(
                    connection_path(from_p, to_p),
                    PathStyle::stroke(Stroke::new(
                        CONNECTION_WIDTH * tf.scale,
                        theme.colors.connection,
                    )),
                );
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
                target.draw_path(
                    connection_path(from_p, to_p),
                    PathStyle::stroke(Stroke::new(
                        CONNECTION_WIDTH * tf.scale,
                        theme.colors.accent,
                    )),
                );
            }
            LeafKind::Line { start, end, stroke } => draw_local_path(
                target,
                &PathData::line(*start, *end),
                PathStyle::stroke(*stroke),
                node.rect,
                tf,
            ),
            LeafKind::Curve { points, stroke } => draw_local_path(
                target,
                &PathData::cubic(*points),
                PathStyle::stroke(*stroke),
                node.rect,
                tf,
            ),
            LeafKind::Path { data, style } => {
                draw_local_path(target, data, *style, node.rect, tf);
            }
            LeafKind::CustomPaint(custom) => {
                custom.0.paint(target, CustomPaintCx { rect: screen_rect });
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
            target,
            child_tf,
            interaction,
            text_inputs,
            theme,
            child_text_color,
        );
    }

    if should_clip_children {
        target.pop_clip();
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

fn draw_local_path(
    target: &mut dyn PaintTarget,
    data: &PathData,
    style: PathStyle,
    rect: Rect,
    tf: PaintTransform,
) {
    target.draw_path(
        leaf_path_to_screen(data, rect, tf),
        scaled_path_style(style, tf.scale),
    );
}

fn scaled_rect_style(style: &RectStyle, scale: f32) -> RectStyle {
    RectStyle {
        color: style.color,
        border: style.border.map(|border| Border {
            width: border.width * scale,
            color: border.color,
        }),
        radius: style.radius.map(|radius| radius * scale),
        shadow: style.shadow.map(|shadow| scaled_shadow(shadow, scale)),
    }
}

fn scaled_shadow(shadow: Shadow, scale: f32) -> Shadow {
    Shadow {
        color: shadow.color,
        offset: [shadow.offset[0] * scale, shadow.offset[1] * scale],
        blur: shadow.blur * scale,
        spread: shadow.spread * scale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::{Border, Color, ImageFit, ImageOpacity, ImageStyle, PathCommand, Shadow};
    use crate::tree::layout::{
        BoxStyle, CustomPaintFn, CustomPainter, LeafKind, Overflow, TextLayout, TextOverflow,
        TextureHandle,
    };
    use crate::tree::node::{NodeLocalRuntime, TreeNode};
    use crate::tree::paint_ops::{PaintOp, RecordingPaintTarget};
    use crate::tree::paint_target::{CustomPaintCx, PaintTarget};
    use crate::tree::{NodeProps, RuntimeSlots};
    use std::borrow::Cow;
    use std::sync::Arc;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    fn point(x: f32, y: f32) -> Point {
        Point { x, y }
    }

    fn leaf_node(id: &'static str, kind: LeafKind, rect: Rect) -> TreeNode {
        TreeNode {
            id: Cow::Borrowed(id).into(),
            props: NodeProps::default(),
            style: BoxStyle::default(),
            decoration: None,
            kind: NodeKind::Leaf(kind),
            rect,
            children: Vec::new(),
            local_runtime: NodeLocalRuntime::default(),
            runtime_slots: RuntimeSlots::default(),
        }
    }

    fn container_node(id: &'static str, rect: Rect, children: Vec<NodeId>) -> TreeNode {
        TreeNode {
            id: Cow::Borrowed(id).into(),
            props: NodeProps::default(),
            style: BoxStyle::default(),
            decoration: None,
            kind: NodeKind::Container,
            rect,
            children,
            local_runtime: NodeLocalRuntime::default(),
            runtime_slots: RuntimeSlots::default(),
        }
    }

    fn paint_single_leaf(kind: LeafKind, rect: Rect) -> Vec<PaintOp> {
        let mut tree = Tree::new();
        let root = tree.insert(leaf_node("leaf", kind, rect));
        tree.set_root(root);
        let mut target = RecordingPaintTarget::new();

        paint_to_target(&tree, root, &mut target, None, None, &Theme::default());

        target.into_ops()
    }

    #[test]
    fn scaled_rect_style_scales_radius_border_and_shadow_metrics() {
        let style = RectStyle {
            color: Color::WHITE,
            border: Some(Border {
                width: 1.5,
                color: Color::BLACK,
            }),
            radius: [2.0, 4.0, 6.0, 8.0],
            shadow: Some(Shadow {
                color: Color::BLACK,
                offset: [1.0, 2.0],
                blur: 3.0,
                spread: 4.0,
            }),
        };

        let scaled = scaled_rect_style(&style, 2.0);

        assert_eq!(scaled.radius, [4.0, 8.0, 12.0, 16.0]);
        assert_eq!(scaled.border.unwrap().width, 3.0);
        let shadow = scaled.shadow.unwrap();
        assert_eq!(shadow.offset, [2.0, 4.0]);
        assert_eq!(shadow.blur, 6.0);
        assert_eq!(shadow.spread, 8.0);
    }

    #[test]
    fn line_leaf_records_path_op() {
        let stroke = Stroke::new(2.0, Color::WHITE);
        let ops = paint_single_leaf(
            LeafKind::Line {
                start: point(1.0, 2.0),
                end: point(3.0, 4.0),
                stroke,
            },
            rect(10.0, 20.0, 100.0, 50.0),
        );

        assert_eq!(
            ops,
            vec![PaintOp::Path {
                data: PathData::line(point(11.0, 22.0), point(13.0, 24.0)),
                style: PathStyle::stroke(stroke),
            }]
        );
    }

    #[test]
    fn curve_leaf_records_path_op() {
        let stroke = Stroke::new(2.0, Color::WHITE);
        let points = [
            point(0.0, 0.0),
            point(10.0, 0.0),
            point(20.0, 10.0),
            point(30.0, 10.0),
        ];
        let ops = paint_single_leaf(
            LeafKind::Curve { points, stroke },
            rect(5.0, 7.0, 100.0, 50.0),
        );

        assert_eq!(
            ops,
            vec![PaintOp::Path {
                data: PathData::cubic([
                    point(5.0, 7.0),
                    point(15.0, 7.0),
                    point(25.0, 17.0),
                    point(35.0, 17.0),
                ]),
                style: PathStyle::stroke(stroke),
            }]
        );
    }

    #[test]
    fn explicit_path_leaf_records_path_op() {
        let style = PathStyle::stroke(Stroke::new(1.0, Color::WHITE));
        let ops = paint_single_leaf(
            LeafKind::Path {
                data: PathData::line(point(0.0, 0.0), point(5.0, 0.0)),
                style,
            },
            rect(2.0, 3.0, 100.0, 50.0),
        );

        assert_eq!(
            ops,
            vec![PaintOp::Path {
                data: PathData::line(point(2.0, 3.0), point(7.0, 3.0)),
                style,
            }]
        );
    }

    #[test]
    fn image_leaf_records_style_without_gpu_texture() {
        let texture = TextureHandle(42);
        let style = ImageStyle::default()
            .with_fit(ImageFit::Contain)
            .with_opacity(ImageOpacity::new(0.5));
        let ops = paint_single_leaf(
            LeafKind::Image { texture, style },
            rect(10.0, 20.0, 30.0, 40.0),
        );

        assert_eq!(
            ops,
            vec![PaintOp::Image {
                rect: rect(10.0, 20.0, 30.0, 40.0),
                texture,
                style,
            }]
        );
    }

    #[test]
    fn connection_leaf_records_path_from_port_centers() {
        let mut tree = Tree::new();
        let from = tree.insert(leaf_node(
            "from_port",
            LeafKind::Circle {
                radius: 4.0,
                fill: Some(Color::WHITE),
                stroke: None,
            },
            rect(10.0, 20.0, 8.0, 8.0),
        ));
        let to = tree.insert(leaf_node(
            "to_port",
            LeafKind::Circle {
                radius: 4.0,
                fill: Some(Color::WHITE),
                stroke: None,
            },
            rect(50.0, 40.0, 8.0, 8.0),
        ));
        let connection = tree.insert(leaf_node(
            "connection",
            LeafKind::Connection {
                from_port: Cow::Borrowed("from_port"),
                to_port: Cow::Borrowed("to_port"),
            },
            rect(0.0, 0.0, 0.0, 0.0),
        ));
        let root = tree.insert(container_node(
            "root",
            rect(0.0, 0.0, 100.0, 100.0),
            vec![from, to, connection],
        ));
        tree.set_root(root);
        let mut target = RecordingPaintTarget::new();

        paint_to_target(&tree, root, &mut target, None, None, &Theme::default());

        let path_op = target
            .ops()
            .iter()
            .find_map(|op| match op {
                PaintOp::Path { data, style } => Some((data, style)),
                _ => None,
            })
            .expect("connection should record a path op");
        assert_eq!(
            path_op.0.commands,
            vec![
                PathCommand::MoveTo(point(14.0, 24.0)),
                PathCommand::CubicTo(point(34.0, 24.0), point(34.0, 44.0), point(54.0, 44.0)),
            ]
        );
        assert_eq!(path_op.1.stroke.unwrap().width, CONNECTION_WIDTH);
    }

    #[test]
    fn overflow_hidden_records_clip_around_children() {
        let mut tree = Tree::new();
        let child = tree.insert(leaf_node(
            "child",
            LeafKind::Line {
                start: point(0.0, 0.0),
                end: point(10.0, 0.0),
                stroke: Stroke::new(1.0, Color::WHITE),
            },
            rect(0.0, 0.0, 10.0, 10.0),
        ));
        let mut root_node = container_node("root", rect(1.0, 2.0, 30.0, 40.0), vec![child]);
        root_node.style.overflow = Overflow::Hidden;
        let root = tree.insert(root_node);
        tree.set_root(root);
        let mut target = RecordingPaintTarget::new();

        paint_to_target(&tree, root, &mut target, None, None, &Theme::default());

        assert!(matches!(
            target.ops().first(),
            Some(PaintOp::PushClip {
                rect: clip_rect,
                radius: 0.0
            }) if *clip_rect == rect(1.0, 2.0, 30.0, 40.0)
        ));
        assert!(matches!(target.ops().get(1), Some(PaintOp::Path { .. })));
        assert!(matches!(target.ops().last(), Some(PaintOp::PopClip)));
    }

    #[test]
    fn text_leaf_records_visible_and_clipped_text_ops() {
        let visible = paint_single_leaf(
            LeafKind::Text {
                content: "hello".to_string(),
                style: TextStyle::new(Color::WHITE, 12.0),
                layout: TextLayout::default(),
            },
            rect(10.0, 20.0, 100.0, 20.0),
        );
        assert!(matches!(
            visible.as_slice(),
            [PaintOp::Text { bounds: None, .. }]
        ));

        let clipped = paint_single_leaf(
            LeafKind::Text {
                content: "hello".to_string(),
                style: TextStyle::new(Color::WHITE, 12.0),
                layout: TextLayout {
                    overflow: TextOverflow::Clip,
                    ..TextLayout::default()
                },
            },
            rect(10.0, 20.0, 100.0, 20.0),
        );
        assert!(matches!(
            clipped.as_slice(),
            [PaintOp::Text {
                bounds: Some(bounds),
                ..
            }] if *bounds == rect(10.0, 20.0, 100.0, 20.0)
        ));
    }

    #[derive(Debug)]
    struct TestCustomPainter;

    impl CustomPainter for TestCustomPainter {
        fn paint(&self, target: &mut dyn PaintTarget, cx: CustomPaintCx) {
            target.draw_circle(
                Point {
                    x: cx.rect.x + cx.rect.w * 0.5,
                    y: cx.rect.y + cx.rect.h * 0.5,
                },
                3.0,
                Color::WHITE,
            );
        }
    }

    #[test]
    fn custom_paint_leaf_invokes_painter_with_screen_rect() {
        let ops = paint_single_leaf(
            LeafKind::CustomPaint(CustomPaintFn(Arc::new(TestCustomPainter))),
            rect(10.0, 20.0, 8.0, 10.0),
        );

        assert_eq!(
            ops,
            vec![PaintOp::Circle {
                center: point(14.0, 25.0),
                radius: 3.0,
                color: Color::WHITE,
            }]
        );
    }
}
