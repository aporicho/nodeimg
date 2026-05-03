use super::icon_svg::svg_style_from_icon;
use crate::control::paint_text_leaf_override;
use crate::control::TextBoxStore;
use crate::geometry::{Point, Rect};
use crate::interaction::InteractionState;
use crate::paint::{
    CirclePaint, Color, GridPaint, PathData, PathStyle, Stroke, SvgSourceKey, TextStyle,
};
use crate::theme::Theme;
use crate::tree::connection_endpoint::node_screen_center;
use crate::tree::layout::LeafKind;
use crate::tree::node::NodeId;
use crate::tree::paint_helpers::{connection_path, CONNECTION_WIDTH};
use crate::tree::paint_space::{NodePaintSpace, PaintSpace};
use crate::tree::paint_target::{CustomPaintCx, PaintTarget};
use crate::tree::text_layout::resolve_text_paint;
use crate::tree::Tree;

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_leaf(
    tree: &Tree,
    node_id: NodeId,
    leaf: &LeafKind,
    target: &mut dyn PaintTarget,
    node_rect: Rect,
    node_space: NodePaintSpace,
    current_space: PaintSpace,
    interaction: Option<&InteractionState>,
    text_boxes: Option<&TextBoxStore>,
    theme: &Theme,
    inherited_text_color: Option<Color>,
) {
    let local_rect = node_space.local_rect;

    match leaf {
        LeafKind::Text {
            content,
            style,
            layout,
        } => {
            let text_style = with_inherited_text_color(*style, inherited_text_color);
            if !paint_text_leaf_override(
                tree,
                node_id,
                target,
                node_rect,
                interaction,
                text_boxes,
                theme,
                &text_style,
            ) {
                let resolved =
                    resolve_text_paint(content, &text_style, *layout, local_rect, |text, style| {
                        target.measure_text(text, style)
                    });
                if let Some(bounds) = resolved.bounds {
                    target.draw_text_clipped(resolved.pos, &resolved.content, text_style, bounds);
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
            target.draw_grid(GridPaint {
                rect: local_rect,
                spacing: *spacing,
                dot_color: *dot_color,
                dot_size: *dot_size,
            });
        }
        LeafKind::Image { texture, style } => {
            target.draw_image(local_rect, *texture, *style);
        }
        LeafKind::Icon { spec } => {
            target.draw_svg(
                local_rect,
                SvgSourceKey::new(spec.id.as_str()),
                svg_style_from_icon(spec.style),
            );
        }
        LeafKind::Circle {
            radius,
            fill,
            stroke,
        } => {
            target.draw_circle_paint(CirclePaint {
                center: Point {
                    x: local_rect.w * 0.5,
                    y: local_rect.h * 0.5,
                },
                radius: *radius,
                fill: *fill,
                stroke: stroke.map(|border| Stroke::new(border.width, border.color)),
            });
        }
        LeafKind::Connection { from_port, to_port } => {
            let Some(from_screen) = node_screen_center(tree, from_port.as_ref()) else {
                return;
            };
            let Some(to_screen) = node_screen_center(tree, to_port.as_ref()) else {
                return;
            };
            let Some(inverse) = node_space.local_to_screen.inverse() else {
                return;
            };
            let from_local = inverse.transform_point(from_screen);
            let to_local = inverse.transform_point(to_screen);
            target.draw_path(
                connection_path(from_local, to_local),
                PathStyle::stroke(Stroke::new(CONNECTION_WIDTH, theme.colors.connection)),
            );
        }
        LeafKind::PendingConnection {
            from_port,
            cursor_canvas,
        } => {
            let Some(from_screen) = node_screen_center(tree, from_port.as_ref()) else {
                return;
            };
            let Some(inverse) = node_space.local_to_screen.inverse() else {
                return;
            };
            let to_screen = current_space.to_screen.transform_point(*cursor_canvas);
            target.draw_path(
                connection_path(
                    inverse.transform_point(from_screen),
                    inverse.transform_point(to_screen),
                ),
                PathStyle::stroke(Stroke::new(CONNECTION_WIDTH, theme.colors.accent)),
            );
        }
        LeafKind::Line { start, end, stroke } => {
            target.draw_path(PathData::line(*start, *end), PathStyle::stroke(*stroke));
        }
        LeafKind::Curve { points, stroke } => {
            target.draw_path(PathData::cubic(*points), PathStyle::stroke(*stroke));
        }
        LeafKind::Path { data, style } => {
            target.draw_path(data.clone(), *style);
        }
        LeafKind::CustomPaint(custom) => {
            custom.0.paint(
                target,
                CustomPaintCx {
                    local_rect,
                    transform: node_space.local_to_screen,
                    screen_bounds: node_space.screen_bounds(),
                },
            );
        }
    }
}

fn with_inherited_text_color(mut style: TextStyle, inherited: Option<Color>) -> TextStyle {
    if let Some(color) = inherited {
        style.color = color;
    }
    style
}
