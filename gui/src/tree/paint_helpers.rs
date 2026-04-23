//! Paint 子系统的纯函数辅助 + PaintTransform 类型。

use crate::renderer::{PathData, PathStyle, Point, Rect, Stroke};
use crate::tree::layout::Transform;
use crate::tree::node::NodeId;
use crate::tree::tree::Tree;

/// 从 paint 入口向下递归时累积的变换。只处理 translate + scale，不处理 rotate。
#[derive(Debug, Clone, Copy)]
pub struct PaintTransform {
    pub tx: f32,
    pub ty: f32,
    pub scale: f32,
}

impl PaintTransform {
    /// 恒等变换。paint 入口使用。
    pub fn identity() -> Self {
        Self {
            tx: 0.0,
            ty: 0.0,
            scale: 1.0,
        }
    }

    /// 复合：new.tx = self.tx + self.scale * child.translate[0], 以此类推。rotate 忽略。
    pub fn compose(&self, child: &Transform) -> Self {
        Self {
            tx: self.tx + self.scale * child.translate[0],
            ty: self.ty + self.scale * child.translate[1],
            scale: self.scale * child.scale,
        }
    }

    /// local 点 → screen 点：screen = (tx + scale * p.x, ty + scale * p.y)
    pub fn apply_point(&self, p: Point) -> Point {
        Point {
            x: self.tx + self.scale * p.x,
            y: self.ty + self.scale * p.y,
        }
    }

    /// local rect → screen rect（x/y 按 apply_point，w/h 按 scale 缩放）
    pub fn apply_rect(&self, r: Rect) -> Rect {
        Rect {
            x: self.tx + self.scale * r.x,
            y: self.ty + self.scale * r.y,
            w: self.scale * r.w,
            h: self.scale * r.h,
        }
    }
}

/// leaf local path → screen path。PathData 的点先落到 leaf border-box，再套 paint transform。
pub fn leaf_path_to_screen(data: &PathData, leaf_rect: Rect, tf: PaintTransform) -> PathData {
    data.map_points(|point| {
        tf.apply_point(Point {
            x: leaf_rect.x + point.x,
            y: leaf_rect.y + point.y,
        })
    })
}

pub fn scaled_stroke(mut stroke: Stroke, scale: f32) -> Stroke {
    stroke.width *= scale;
    stroke
}

pub fn scaled_path_style(mut style: PathStyle, scale: f32) -> PathStyle {
    style.stroke = style.stroke.map(|stroke| scaled_stroke(stroke, scale));
    style
}

pub fn connection_path(from: Point, to: Point) -> PathData {
    PathData::cubic(bezier_control_points(from, to))
}

/// 遍历 rect 内 spacing 为间距的格点。spacing <= 0 时返回空 Vec。
pub fn grid_cells(rect: Rect, spacing: f32) -> Vec<Point> {
    if spacing <= 0.0 {
        return Vec::new();
    }
    let cols = (rect.w / spacing).floor() as usize + 1;
    let rows = (rect.h / spacing).floor() as usize + 1;
    let mut cells = Vec::with_capacity(cols * rows);
    for row in 0..rows {
        for col in 0..cols {
            cells.push(Point {
                x: rect.x + col as f32 * spacing,
                y: rect.y + row as f32 * spacing,
            });
        }
    }
    cells
}

/// 三次贝塞尔 4 个控制点（水平偏移启发式）。
/// p0=from, p1=(from.x+|dx|/2, from.y), p2=(to.x-|dx|/2, to.y), p3=to
pub fn bezier_control_points(from: Point, to: Point) -> [Point; 4] {
    let dx_abs = (to.x - from.x).abs();
    let offset = dx_abs * 0.5;
    [
        from,
        Point {
            x: from.x + offset,
            y: from.y,
        },
        Point {
            x: to.x - offset,
            y: to.y,
        },
        to,
    ]
}

/// 深度优先遍历 tree，查找 id 匹配的节点，返回其 rect。找不到返回 None。
pub fn find_node_by_str_id(tree: &Tree, id: &str) -> Option<Rect> {
    let root = tree.root()?;
    find_recursive(tree, root, id)
}

fn find_recursive(tree: &Tree, node_id: NodeId, target_id: &str) -> Option<Rect> {
    let node = tree.get(node_id)?;
    if node.id.as_ref() == target_id {
        return Some(node.rect);
    }
    let children: Vec<NodeId> = node.children.clone();
    for child_id in children {
        if let Some(rect) = find_recursive(tree, child_id, target_id) {
            return Some(rect);
        }
    }
    None
}

/// 取 rect 中心点。端口圆点使用自身中心作为连接锚点。
pub fn rect_center(r: Rect) -> Point {
    Point {
        x: r.x + r.w * 0.5,
        y: r.y + r.h * 0.5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::{Color, LineCap, PathCommand, PathStyle, Stroke};
    use crate::tree::layout::{BoxStyle, Transform};
    use crate::tree::node::{NodeKind, NodeLocalRuntime, TreeNode};
    use crate::tree::tree::Tree;
    use crate::tree::{NodeProps, RuntimeSlots};
    use std::borrow::Cow;

    fn container_at(id: &'static str, rect: Rect) -> TreeNode {
        TreeNode {
            id: Cow::Borrowed(id).into(),
            props: NodeProps::default(),
            style: BoxStyle::default(),
            decoration: None,
            kind: NodeKind::Container,
            rect,
            children: Vec::new(),
            local_runtime: NodeLocalRuntime::default(),
            runtime_slots: RuntimeSlots::default(),
        }
    }

    // ── PaintTransform ──

    #[test]
    fn transform_identity_compose() {
        let id = PaintTransform::identity();
        let child = Transform {
            translate: [10.0, 20.0],
            scale: 2.0,
            rotate: 0.0,
        };
        let result = id.compose(&child);
        assert_eq!(result.tx, 10.0);
        assert_eq!(result.ty, 20.0);
        assert_eq!(result.scale, 2.0);
    }

    #[test]
    fn transform_compose_nested() {
        let first = PaintTransform::identity().compose(&Transform {
            translate: [10.0, 20.0],
            scale: 2.0,
            rotate: 0.0,
        });
        let second = first.compose(&Transform {
            translate: [5.0, 5.0],
            scale: 3.0,
            rotate: 0.0,
        });
        assert_eq!(second.tx, 20.0, "tx = 10 + 2*5 = 20");
        assert_eq!(second.ty, 30.0, "ty = 20 + 2*5 = 30");
        assert_eq!(second.scale, 6.0, "scale = 2*3 = 6");
    }

    #[test]
    fn transform_apply_point() {
        let tf = PaintTransform {
            tx: 10.0,
            ty: 20.0,
            scale: 2.0,
        };
        let result = tf.apply_point(Point { x: 3.0, y: 4.0 });
        assert_eq!(result.x, 16.0, "x = 10 + 2*3 = 16");
        assert_eq!(result.y, 28.0, "y = 20 + 2*4 = 28");
    }

    #[test]
    fn transform_apply_rect() {
        let tf = PaintTransform {
            tx: 10.0,
            ty: 20.0,
            scale: 2.0,
        };
        let result = tf.apply_rect(Rect {
            x: 3.0,
            y: 4.0,
            w: 5.0,
            h: 6.0,
        });
        assert_eq!(result.x, 16.0);
        assert_eq!(result.y, 28.0);
        assert_eq!(result.w, 10.0, "w = 2*5 = 10");
        assert_eq!(result.h, 12.0, "h = 2*6 = 12");
    }

    #[test]
    fn leaf_path_to_screen_offsets_by_leaf_rect_before_transform() {
        let data = PathData::line(Point { x: 1.0, y: 2.0 }, Point { x: 3.0, y: 4.0 });
        let leaf_rect = Rect {
            x: 10.0,
            y: 20.0,
            w: 100.0,
            h: 50.0,
        };
        let tf = PaintTransform {
            tx: 5.0,
            ty: 7.0,
            scale: 2.0,
        };

        let screen = leaf_path_to_screen(&data, leaf_rect, tf);

        assert_eq!(
            screen.commands,
            vec![
                PathCommand::MoveTo(Point { x: 27.0, y: 51.0 }),
                PathCommand::LineTo(Point { x: 31.0, y: 55.0 }),
            ]
        );
    }

    #[test]
    fn scaled_path_style_scales_stroke_width_only() {
        let style = PathStyle::stroke(Stroke::new(2.0, Color::WHITE).with_cap(LineCap::Round));

        let scaled = scaled_path_style(style, 3.0);

        let stroke = scaled.stroke.unwrap();
        assert_eq!(stroke.width, 6.0);
        assert_eq!(stroke.cap, LineCap::Round);
        assert_eq!(stroke.miter_limit, 4.0);
    }

    #[test]
    fn paint_transform_policy_ignores_rotate_until_renderer_supports_it() {
        let id = PaintTransform::identity();
        let child = Transform {
            translate: [10.0, 20.0],
            scale: 2.0,
            rotate: 1.5,
        };
        let result = id.compose(&child);
        assert_eq!(result.tx, 10.0);
        assert_eq!(result.ty, 20.0);
        assert_eq!(result.scale, 2.0);
    }

    // ── grid_cells ──

    #[test]
    fn grid_cells_single_point() {
        let rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        };
        let cells = grid_cells(rect, 10.0);
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0], Point { x: 0.0, y: 0.0 });
    }

    #[test]
    fn grid_cells_multi() {
        let rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 30.0,
            h: 30.0,
        };
        let cells = grid_cells(rect, 10.0);
        assert_eq!(cells.len(), 16, "4×4 = 16 个点");
        assert!(cells.contains(&Point { x: 0.0, y: 0.0 }));
        assert!(cells.contains(&Point { x: 30.0, y: 30.0 }));
    }

    #[test]
    fn grid_cells_zero_spacing() {
        let rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        };
        assert!(grid_cells(rect, 0.0).is_empty(), "spacing=0 应返回空");
        assert!(grid_cells(rect, -5.0).is_empty(), "负 spacing 应返回空");
    }

    // ── bezier_control_points ──

    #[test]
    fn bezier_horizontal() {
        let result = bezier_control_points(Point { x: 0.0, y: 0.0 }, Point { x: 100.0, y: 0.0 });
        assert_eq!(result[0], Point { x: 0.0, y: 0.0 });
        assert_eq!(result[1], Point { x: 50.0, y: 0.0 });
        assert_eq!(result[2], Point { x: 50.0, y: 0.0 });
        assert_eq!(result[3], Point { x: 100.0, y: 0.0 });
    }

    #[test]
    fn bezier_diagonal() {
        let result = bezier_control_points(Point { x: 0.0, y: 0.0 }, Point { x: 100.0, y: 50.0 });
        assert_eq!(result[0], Point { x: 0.0, y: 0.0 });
        assert_eq!(result[1], Point { x: 50.0, y: 0.0 });
        assert_eq!(result[2], Point { x: 50.0, y: 50.0 });
        assert_eq!(result[3], Point { x: 100.0, y: 50.0 });
    }

    #[test]
    fn bezier_reverse() {
        let result = bezier_control_points(Point { x: 100.0, y: 0.0 }, Point { x: 0.0, y: 0.0 });
        assert_eq!(result[0], Point { x: 100.0, y: 0.0 });
        assert_eq!(
            result[1],
            Point { x: 150.0, y: 0.0 },
            "p1.x = 100 + |100|/2 = 150"
        );
        assert_eq!(
            result[2],
            Point { x: -50.0, y: 0.0 },
            "p2.x = 0 - |100|/2 = -50"
        );
        assert_eq!(result[3], Point { x: 0.0, y: 0.0 });
    }

    // ── find_node_by_str_id ──

    #[test]
    fn find_node_hit_root() {
        let mut tree = Tree::new();
        let root_id = tree.insert(container_at(
            "root",
            Rect {
                x: 10.0,
                y: 20.0,
                w: 100.0,
                h: 50.0,
            },
        ));
        tree.set_root(root_id);
        let result = find_node_by_str_id(&tree, "root");
        assert!(result.is_some());
        let rect = result.unwrap();
        assert_eq!(rect.x, 10.0);
        assert_eq!(rect.y, 20.0);
    }

    #[test]
    fn find_node_hit_nested() {
        let mut tree = Tree::new();
        let target_id = tree.insert(container_at(
            "target",
            Rect {
                x: 30.0,
                y: 40.0,
                w: 20.0,
                h: 10.0,
            },
        ));
        let middle = {
            let mut n = container_at(
                "middle",
                Rect {
                    x: 5.0,
                    y: 5.0,
                    w: 100.0,
                    h: 100.0,
                },
            );
            n.children = vec![target_id];
            n
        };
        let middle_id = tree.insert(middle);
        let root = {
            let mut n = container_at(
                "root",
                Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 200.0,
                    h: 200.0,
                },
            );
            n.children = vec![middle_id];
            n
        };
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let result = find_node_by_str_id(&tree, "target");
        assert!(result.is_some());
        assert_eq!(result.unwrap().x, 30.0);
    }

    #[test]
    fn find_node_miss() {
        let mut tree = Tree::new();
        let root_id = tree.insert(container_at(
            "root",
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
            },
        ));
        tree.set_root(root_id);
        assert!(find_node_by_str_id(&tree, "nonexistent").is_none());
    }

    #[test]
    fn rect_center_returns_midpoint() {
        assert_eq!(
            rect_center(Rect {
                x: 10.0,
                y: 20.0,
                w: 32.0,
                h: 32.0,
            }),
            Point { x: 26.0, y: 36.0 }
        );
    }

    // ── dry run（Renderer 需 wgpu Device，暂无单测 stub）──

    #[test]
    #[ignore = "Renderer 需 wgpu Device，暂无单测 stub"]
    fn paint_dry_run_variety() {}
}
