use crate::renderer::Rect;

use super::measure::measure;
use super::types::*;

/// 自顶向下分配位置，直接写入树节点。
pub(crate) fn arrange<T: LayoutTree>(
    tree: &mut T,
    node: T::NodeId,
    available: Rect,
    measure_text: &mut dyn FnMut(&str, f32) -> (f32, f32),
) {
    let style = tree.style(node).clone();

    // 扣除 margin
    let after_margin = Rect {
        x: available.x + style.margin.left,
        y: available.y + style.margin.top,
        w: (available.w - style.margin.horizontal()).max(0.0),
        h: (available.h - style.margin.vertical()).max(0.0),
    };

    // 节点最终 rect（margin 内的区域）
    let node_rect = Rect {
        x: after_margin.x,
        y: after_margin.y,
        w: match style.width {
            Size::Fixed(w) => w,
            _ => after_margin.w,
        }.clamp(style.min_width, style.max_width),
        h: match style.height {
            Size::Fixed(h) => h,
            _ => after_margin.h,
        }.clamp(style.min_height, style.max_height),
    };

    tree.set_rect(node, node_rect);

    let children = tree.children(node);
    if children.is_empty() {
        return;
    }

    // 按 position 分组：Flow 子节点走 Flex 流程，Absolute 子节点单独一趟
    let (flow_children, abs_children): (Vec<_>, Vec<_>) = children
        .iter()
        .copied()
        .partition(|&c| matches!(tree.style(c).position, Position::Flow));

    // 扣除 padding → 内容区域
    // Transform 节点的子节点在 local 空间，起点相对 (0, 0) + padding；
    // 普通节点的子节点在绝对空间，起点相对 node_rect + padding。
    let content = if style.transform.is_some() {
        Rect {
            x: style.padding.left,
            y: style.padding.top,
            w: (node_rect.w - style.padding.horizontal()).max(0.0),
            h: (node_rect.h - style.padding.vertical()).max(0.0),
        }
    } else {
        Rect {
            x: node_rect.x + style.padding.left,
            y: node_rect.y + style.padding.top,
            w: (node_rect.w - style.padding.horizontal()).max(0.0),
            h: (node_rect.h - style.padding.vertical()).max(0.0),
        }
    };

    // 度量子节点（提前，Scroll 分支和 Flex 分支共用）
    let is_column = style.direction == Direction::Column;
    let child_sizes: Vec<DesiredSize> = flow_children.iter().map(|&c| measure(&*tree, c, measure_text)).collect();

    // 处理滚动
    let scroll_offset = if style.overflow == Overflow::Scroll {
        let offset = tree.scroll_offset(node);

        let n = child_sizes.len();
        let total_gap = if n > 1 { style.gap * (n as f32 - 1.0) } else { 0.0 };
        let content_height = match style.direction {
            Direction::Column => child_sizes.iter().map(|s| s.height).sum::<f32>() + total_gap,
            Direction::Row => child_sizes.iter().map(|s| s.height).fold(0.0f32, f32::max),
        };

        tree.set_content_height(node, content_height);
        offset
    } else {
        0.0
    };
    let child_styles: Vec<(f32, Size, Size, f32)> = flow_children.iter().map(|&c| {
        let s = tree.style(c);
        let main_margin = if is_column { s.margin.vertical() } else { s.margin.horizontal() };
        (s.flex_grow, s.width, s.height, main_margin)
    }).collect();

    let n = child_sizes.len();
    let total_gap = if n > 1 { style.gap * (n as f32 - 1.0) } else { 0.0 };

    // 主轴可用空间
    let main_available = if is_column { content.h } else { content.w };

    // 计算固定子节点占用 + 收集 flex_grow
    let mut fixed_main: f32 = total_gap;
    let mut total_grow: f32 = 0.0;
    for (i, &(grow, width, height, main_margin)) in child_styles.iter().enumerate() {
        let child_main = if is_column { child_sizes[i].height } else { child_sizes[i].width };
        let is_fill = if is_column {
            matches!(height, Size::Fill)
        } else {
            matches!(width, Size::Fill)
        };

        if grow > 0.0 || is_fill {
            total_grow += if grow > 0.0 { grow } else { 1.0 };
            fixed_main += main_margin; // flex 子节点的 margin 也要从剩余空间扣除
        } else {
            fixed_main += child_main;
        }
    }

    let remaining = (main_available - fixed_main).max(0.0);

    // justify_content 偏移
    let (mut main_offset, extra_gap) = match style.justify_content {
        Justify::Start => (0.0, 0.0),
        Justify::End => (remaining.max(0.0), 0.0),
        Justify::Center => (remaining.max(0.0) / 2.0, 0.0),
        Justify::SpaceBetween => {
            if n > 1 && total_grow == 0.0 {
                (0.0, remaining / (n as f32 - 1.0))
            } else {
                (0.0, 0.0)
            }
        }
    };

    main_offset -= scroll_offset;

    // 排列子节点
    for (i, &child) in flow_children.iter().enumerate() {
        let child_desired = &child_sizes[i];
        let (grow, width, height, _) = child_styles[i];
        let is_fill = if is_column {
            matches!(height, Size::Fill)
        } else {
            matches!(width, Size::Fill)
        };
        let effective_grow = if grow > 0.0 {
            grow
        } else if is_fill {
            1.0
        } else {
            0.0
        };

        let child_main = if effective_grow > 0.0 {
            remaining * effective_grow / total_grow
        } else if is_column {
            child_desired.height
        } else {
            child_desired.width
        };

        let cross_available = if is_column { content.w } else { content.h };
        let child_cross_desired = if is_column { child_desired.width } else { child_desired.height };
        let (cross_offset, child_cross) = match style.align_items {
            Align::Stretch => (0.0, cross_available),
            Align::Start => (0.0, child_cross_desired),
            Align::End => (cross_available - child_cross_desired, child_cross_desired),
            Align::Center => ((cross_available - child_cross_desired) / 2.0, child_cross_desired),
        };

        let child_rect = if is_column {
            Rect {
                x: content.x + cross_offset,
                y: content.y + main_offset,
                w: child_cross,
                h: child_main,
            }
        } else {
            Rect {
                x: content.x + main_offset,
                y: content.y + cross_offset,
                w: child_main,
                h: child_cross,
            }
        };

        arrange(tree, child, child_rect, measure_text);

        main_offset += child_main + style.gap + extra_gap;
    }

    // ── Absolute 阶段：每个 Absolute 子节点独立 arrange ──
    for child in abs_children {
        let child_style = tree.style(child).clone();
        let (abs_x, abs_y) = match child_style.position {
            Position::Absolute { x, y } => (x, y),
            Position::Flow => unreachable!("partition 已保证这里只有 Absolute"),
        };

        // Absolute 子节点的"可用空间"：从 content 偏移 abs_x/abs_y 后的区域
        let child_available = Rect {
            x: content.x + abs_x,
            y: content.y + abs_y,
            w: (content.w - abs_x).max(0.0),
            h: (content.h - abs_y).max(0.0),
        };

        arrange(tree, child, child_available, measure_text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::tree::Tree;
    use crate::tree::node::{NodeKind, PanelNode};
    use std::borrow::Cow;

    /// 构造一个基础 Container 节点（decoration 无、子节点无）
    fn container(style: BoxStyle) -> PanelNode {
        PanelNode {
            id: Cow::Borrowed("test"),
            style,
            decoration: None,
            kind: NodeKind::Container,
            rect: Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 },
            children: Vec::new(),
            scroll_offset: 0.0,
            content_height: 0.0,
        }
    }

    /// 不依赖字体的 measure 回调
    fn no_measure(_text: &str, _size: f32) -> (f32, f32) { (0.0, 0.0) }

    #[test]
    fn absolute_simple() {
        let mut tree = Tree::new();
        let child_id = tree.insert(container(BoxStyle {
            position: Position::Absolute { x: 100.0, y: 50.0 },
            width: Size::Fixed(100.0),
            height: Size::Fixed(80.0),
            ..Default::default()
        }));
        let mut root = container(BoxStyle {
            width: Size::Fixed(800.0),
            height: Size::Fixed(600.0),
            ..Default::default()
        });
        root.children = vec![child_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let mut measure = no_measure;
        arrange(&mut tree, root_id, Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 }, &mut measure);

        let child = tree.get(child_id).unwrap();
        assert_eq!(child.rect.x, 100.0);
        assert_eq!(child.rect.y, 50.0);
        assert_eq!(child.rect.w, 100.0);
        assert_eq!(child.rect.h, 80.0);
    }

    #[test]
    fn absolute_with_padding() {
        let mut tree = Tree::new();
        let child_id = tree.insert(container(BoxStyle {
            position: Position::Absolute { x: 100.0, y: 50.0 },
            width: Size::Fixed(100.0),
            height: Size::Fixed(80.0),
            ..Default::default()
        }));
        let mut root = container(BoxStyle {
            width: Size::Fixed(800.0),
            height: Size::Fixed(600.0),
            padding: Edges::all(20.0),
            ..Default::default()
        });
        root.children = vec![child_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let mut measure = no_measure;
        arrange(&mut tree, root_id, Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 }, &mut measure);

        let child = tree.get(child_id).unwrap();
        assert_eq!(child.rect.x, 120.0, "应该是 padding.left (20) + abs.x (100)");
        assert_eq!(child.rect.y, 70.0, "应该是 padding.top (20) + abs.y (50)");
    }

    #[test]
    fn absolute_does_not_take_flex_space() {
        let mut tree = Tree::new();

        let flex1_id = tree.insert(container(BoxStyle {
            flex_grow: 1.0,
            height: Size::Fixed(100.0),
            ..Default::default()
        }));
        let flex2_id = tree.insert(container(BoxStyle {
            flex_grow: 1.0,
            height: Size::Fixed(100.0),
            ..Default::default()
        }));
        let abs_id = tree.insert(container(BoxStyle {
            position: Position::Absolute { x: 200.0, y: 200.0 },
            width: Size::Fixed(50.0),
            height: Size::Fixed(50.0),
            ..Default::default()
        }));

        let mut root = container(BoxStyle {
            width: Size::Fixed(800.0),
            height: Size::Fixed(600.0),
            direction: Direction::Row,
            ..Default::default()
        });
        root.children = vec![flex1_id, flex2_id, abs_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let mut measure = no_measure;
        arrange(&mut tree, root_id, Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 }, &mut measure);

        let flex1 = tree.get(flex1_id).unwrap();
        let flex2 = tree.get(flex2_id).unwrap();
        assert_eq!(flex1.rect.w, 400.0, "Flex1 应占一半");
        assert_eq!(flex2.rect.w, 400.0, "Flex2 应占一半");
        assert_eq!(flex1.rect.x, 0.0);
        assert_eq!(flex2.rect.x, 400.0);

        let abs = tree.get(abs_id).unwrap();
        assert_eq!(abs.rect.x, 200.0);
        assert_eq!(abs.rect.y, 200.0);
    }

    #[test]
    fn absolute_fixed_size() {
        let mut tree = Tree::new();
        let child_id = tree.insert(container(BoxStyle {
            position: Position::Absolute { x: 50.0, y: 50.0 },
            width: Size::Fixed(200.0),
            height: Size::Fixed(150.0),
            ..Default::default()
        }));
        let mut root = container(BoxStyle {
            width: Size::Fixed(400.0),
            height: Size::Fixed(300.0),
            ..Default::default()
        });
        root.children = vec![child_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let mut measure = no_measure;
        arrange(&mut tree, root_id, Rect { x: 0.0, y: 0.0, w: 400.0, h: 300.0 }, &mut measure);

        let child = tree.get(child_id).unwrap();
        assert_eq!(child.rect.x, 50.0);
        assert_eq!(child.rect.y, 50.0);
        assert_eq!(child.rect.w, 200.0);
        assert_eq!(child.rect.h, 150.0);
    }

    #[test]
    fn absolute_in_scroll_container() {
        let mut tree = Tree::new();
        let flow1_id = tree.insert(container(BoxStyle {
            height: Size::Fixed(200.0),
            ..Default::default()
        }));
        let flow2_id = tree.insert(container(BoxStyle {
            height: Size::Fixed(200.0),
            ..Default::default()
        }));
        let abs_id = tree.insert(container(BoxStyle {
            position: Position::Absolute { x: 10.0, y: 10.0 },
            width: Size::Fixed(100.0),
            height: Size::Fixed(999.0),
            ..Default::default()
        }));

        let mut root = container(BoxStyle {
            width: Size::Fixed(400.0),
            height: Size::Fixed(300.0),
            direction: Direction::Column,
            overflow: Overflow::Scroll,
            ..Default::default()
        });
        root.children = vec![flow1_id, flow2_id, abs_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let mut measure = no_measure;
        arrange(&mut tree, root_id, Rect { x: 0.0, y: 0.0, w: 400.0, h: 300.0 }, &mut measure);

        let root = tree.get(root_id).unwrap();
        assert_eq!(root.content_height, 400.0, "scroll content_height 应只累计 Flow 子节点");
    }

    #[test]
    fn transform_node_children_in_local_space() {
        let mut tree = Tree::new();
        let child_id = tree.insert(container(BoxStyle {
            width: Size::Fixed(100.0),
            height: Size::Fixed(50.0),
            ..Default::default()
        }));
        let mut root = container(BoxStyle {
            width: Size::Fixed(400.0),
            height: Size::Fixed(300.0),
            transform: Some(Transform { translate: [10.0, 20.0], scale: 2.0, rotate: 0.0 }),
            ..Default::default()
        });
        root.children = vec![child_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let mut measure = no_measure;
        arrange(&mut tree, root_id, Rect { x: 50.0, y: 60.0, w: 400.0, h: 300.0 }, &mut measure);

        let child = tree.get(child_id).unwrap();
        assert_eq!(child.rect.x, 0.0, "Transform 父的 Flow 子应从 local origin (0,0) 开始");
        assert_eq!(child.rect.y, 0.0);
        assert_eq!(child.rect.w, 100.0);
        assert_eq!(child.rect.h, 50.0);
    }
}
