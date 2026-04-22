use crate::renderer::Rect;

use super::box_model::{
    absolute_available_rect, border_box_from_available, layout_boxes, relative_offset_rect,
    ChildCoordinateSpace,
};
use super::measure::measure;
use super::types::*;

#[derive(Debug, Clone, Copy)]
struct FlexChildStyle {
    grow: f32,
    shrink: f32,
    width: Size,
    height: Size,
    position: Position,
    align_self: Option<Align>,
    min_width: f32,
    max_width: f32,
    min_height: f32,
    max_height: f32,
    main_margin: f32,
}

/// 自顶向下分配位置，直接写入树节点。
pub(crate) fn arrange<T: LayoutTree>(
    tree: &mut T,
    node: T::NodeId,
    available: Rect,
    measure_text: &mut dyn FnMut(&str, &crate::renderer::TextStyle) -> (f32, f32),
) {
    arrange_in_containing_block(tree, node, available, available, measure_text);
}

fn arrange_in_containing_block<T: LayoutTree>(
    tree: &mut T,
    node: T::NodeId,
    available: Rect,
    absolute_containing_block: Rect,
    measure_text: &mut dyn FnMut(&str, &crate::renderer::TextStyle) -> (f32, f32),
) {
    let style = tree.style(node).clone();
    let desired_size = style
        .position
        .is_absolute()
        .then(|| measure(&*tree, node, measure_text));

    let border_available = border_box_from_available(available, style.margin);

    // 节点最终 rect（margin 内的区域）
    let node_rect = Rect {
        x: border_available.x,
        y: border_available.y,
        w: match style.width {
            Size::Fixed(w) => w,
            Size::Fill => border_available.w,
            Size::Auto
                if matches!(
                    style.position,
                    Position::Absolute(pos) if pos.has_horizontal_stretch()
                ) =>
            {
                border_available.w
            }
            Size::Auto if matches!(style.position, Position::Absolute(_)) => desired_size
                .map(|size| size.width)
                .unwrap_or(border_available.w),
            _ => border_available.w,
        }
        .clamp(style.min_width, style.max_width),
        h: match style.height {
            Size::Fixed(h) => h,
            Size::Fill => border_available.h,
            Size::Auto
                if matches!(
                    style.position,
                    Position::Absolute(pos) if pos.has_vertical_stretch()
                ) =>
            {
                border_available.h
            }
            Size::Auto if matches!(style.position, Position::Absolute(_)) => desired_size
                .map(|size| size.height)
                .unwrap_or(border_available.h),
            _ => border_available.h,
        }
        .clamp(style.min_height, style.max_height),
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
        .partition(|&c| !tree.style(c).position.is_absolute());

    // Transform 节点的子节点在 local 空间，起点相对 (0, 0) + padding；
    // 普通节点的子节点在父坐标空间，起点相对 border box + padding。
    let child_space = if style.transform.is_some() {
        ChildCoordinateSpace::Local
    } else {
        ChildCoordinateSpace::Parent
    };
    let boxes = layout_boxes(available, node_rect, style.padding, child_space);
    let _ = (boxes.margin_box, boxes.border_box);
    let content = boxes.content_box;
    let child_absolute_containing_block =
        if style.position.is_positioned() || style.transform.is_some() {
            content
        } else {
            absolute_containing_block
        };

    // 度量子节点（提前，Scroll 分支和 Flex 分支共用）
    let is_column = style.direction == Direction::Column;
    let child_sizes: Vec<DesiredSize> = flow_children
        .iter()
        .map(|&c| measure(&*tree, c, measure_text))
        .collect();

    // 处理滚动
    let scroll_offset = if style.overflow == Overflow::Scroll {
        let offset = tree.scroll_offset(node);

        let n = child_sizes.len();
        let total_gap = if n > 1 {
            style.gap * (n as f32 - 1.0)
        } else {
            0.0
        };
        let content_height = match style.direction {
            Direction::Column => child_sizes.iter().map(|s| s.height).sum::<f32>() + total_gap,
            Direction::Row => child_sizes.iter().map(|s| s.height).fold(0.0f32, f32::max),
        };

        tree.set_content_height(node, content_height);
        offset
    } else {
        0.0
    };
    let child_styles: Vec<FlexChildStyle> = flow_children
        .iter()
        .map(|&c| {
            let s = tree.style(c);
            let main_margin = if is_column {
                s.margin.vertical()
            } else {
                s.margin.horizontal()
            };
            FlexChildStyle {
                grow: s.flex_grow,
                shrink: s.flex_shrink,
                width: s.width,
                height: s.height,
                position: s.position,
                align_self: s.align_self,
                min_width: s.min_width,
                max_width: s.max_width,
                min_height: s.min_height,
                max_height: s.max_height,
                main_margin,
            }
        })
        .collect();

    let n = child_sizes.len();
    let total_gap = if n > 1 {
        style.gap * (n as f32 - 1.0)
    } else {
        0.0
    };

    // 主轴可用空间
    let main_available = if is_column { content.h } else { content.w };
    let base_main_sizes: Vec<f32> = child_sizes
        .iter()
        .map(|size| if is_column { size.height } else { size.width })
        .collect();
    let total_base_main = base_main_sizes.iter().sum::<f32>() + total_gap;
    let should_shrink = total_base_main > main_available;

    // 计算固定子节点占用 + 收集 flex_grow
    let mut fixed_main: f32 = total_gap;
    let mut total_grow: f32 = 0.0;
    for (i, child_style) in child_styles.iter().enumerate() {
        let child_main = base_main_sizes[i];
        let is_fill = if is_column {
            matches!(child_style.height, Size::Fill)
        } else {
            matches!(child_style.width, Size::Fill)
        };

        if !should_shrink && (child_style.grow > 0.0 || is_fill) {
            total_grow += if child_style.grow > 0.0 {
                child_style.grow
            } else {
                1.0
            };
            fixed_main += child_style.main_margin; // flex 子节点的 margin 也要从剩余空间扣除
        } else {
            fixed_main += child_main;
        }
    }

    let remaining = (main_available - fixed_main).max(0.0);
    let shrink_main_sizes = if should_shrink {
        Some(resolve_shrink_main_sizes(
            &base_main_sizes,
            &child_styles,
            is_column,
            main_available,
            total_gap,
        ))
    } else {
        None
    };

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
        let child_style = child_styles[i];
        let is_fill = if is_column {
            matches!(child_style.height, Size::Fill)
        } else {
            matches!(child_style.width, Size::Fill)
        };
        let effective_grow = if should_shrink {
            0.0
        } else if child_style.grow > 0.0 {
            child_style.grow
        } else if is_fill {
            1.0
        } else {
            0.0
        };

        let child_main = if let Some(shrink_sizes) = &shrink_main_sizes {
            shrink_sizes[i]
        } else if effective_grow > 0.0 {
            remaining * effective_grow / total_grow
        } else if is_column {
            child_desired.height
        } else {
            child_desired.width
        };

        let cross_available = if is_column { content.w } else { content.h };
        let child_cross_desired = if is_column {
            child_desired.width
        } else {
            child_desired.height
        };
        let child_align = child_style.align_self.unwrap_or(style.align_items);
        let (cross_offset, child_cross) = match child_align {
            Align::Stretch => (0.0, cross_available),
            Align::Start => (0.0, child_cross_desired),
            Align::End => (cross_available - child_cross_desired, child_cross_desired),
            Align::Center => (
                (cross_available - child_cross_desired) / 2.0,
                child_cross_desired,
            ),
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

        let child_rect = match child_style.position {
            Position::Relative(position) => relative_offset_rect(child_rect, position.inset),
            Position::Flow | Position::Absolute(_) => child_rect,
        };

        arrange_in_containing_block(
            tree,
            child,
            child_rect,
            child_absolute_containing_block,
            measure_text,
        );

        main_offset += child_main + style.gap + extra_gap;
    }

    // ── Absolute 阶段：每个 Absolute 子节点独立 arrange ──
    for child in abs_children {
        let child_style = tree.style(child).clone();
        let position = match child_style.position {
            Position::Absolute(position) => position,
            Position::Flow | Position::Relative(_) => {
                unreachable!("partition 已保证这里只有 Absolute")
            }
        };
        let desired = measure(&*tree, child, measure_text);

        let child_available = absolute_available_rect(
            child_absolute_containing_block,
            position,
            child_style.width,
            child_style.height,
            (desired.width, desired.height),
        );

        arrange_in_containing_block(
            tree,
            child,
            child_available,
            child_absolute_containing_block,
            measure_text,
        );
    }
}

fn resolve_shrink_main_sizes(
    base_sizes: &[f32],
    styles: &[FlexChildStyle],
    is_column: bool,
    main_available: f32,
    total_gap: f32,
) -> Vec<f32> {
    let target_children_main = (main_available - total_gap).max(0.0);
    let base_children_main = base_sizes.iter().sum::<f32>();
    let overflow = (base_children_main - target_children_main).max(0.0);
    if overflow <= 0.0 {
        return base_sizes.to_vec();
    }

    let shrink_factors: Vec<f32> = base_sizes
        .iter()
        .zip(styles.iter())
        .map(|(base, style)| (style.shrink.max(0.0)) * *base)
        .collect();
    let total_shrink_factor = shrink_factors.iter().sum::<f32>();
    if total_shrink_factor <= 0.0 {
        return base_sizes.to_vec();
    }

    base_sizes
        .iter()
        .zip(styles.iter())
        .zip(shrink_factors.iter())
        .map(|((base, style), factor)| {
            let shrink = overflow * *factor / total_shrink_factor;
            let min_main = if is_column {
                style.min_height
            } else {
                style.min_width
            };
            let max_main = if is_column {
                style.max_height
            } else {
                style.max_width
            };
            (*base - shrink).clamp(min_main, max_main)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::node::{NodeKind, NodeLocalRuntime, TreeNode};
    use crate::tree::tree::Tree;
    use crate::tree::{NodeProps, RuntimeSlots};
    use std::borrow::Cow;

    /// 构造一个基础 Container 节点（decoration 无、子节点无）
    fn container(style: BoxStyle) -> TreeNode {
        TreeNode {
            id: Cow::Borrowed("test").into(),
            props: NodeProps::default(),
            style,
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
            runtime_slots: RuntimeSlots::default(),
        }
    }

    /// 不依赖字体的 measure 回调
    fn no_measure(_text: &str, _style: &crate::renderer::TextStyle) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn auto_wrapper_with_fixed_child(
        tree: &mut Tree,
        fixed_width: f32,
        fixed_height: f32,
        style: BoxStyle,
    ) -> crate::tree::NodeId {
        let inner_id = tree.insert(container(BoxStyle {
            width: Size::Fixed(fixed_width),
            height: Size::Fixed(fixed_height),
            ..Default::default()
        }));
        let mut wrapper = container(style);
        wrapper.children = vec![inner_id];
        tree.insert(wrapper)
    }

    fn arrange_root(tree: &mut Tree, root_id: crate::tree::NodeId, w: f32, h: f32) {
        let mut measure = no_measure;
        arrange(
            tree,
            root_id,
            Rect {
                x: 0.0,
                y: 0.0,
                w,
                h,
            },
            &mut measure,
        );
    }

    #[test]
    fn absolute_simple() {
        let mut tree = Tree::new();
        let child_id = tree.insert(container(BoxStyle {
            position: Position::absolute_xy(100.0, 50.0),
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
        arrange(
            &mut tree,
            root_id,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 800.0,
                h: 600.0,
            },
            &mut measure,
        );

        let child = tree.get(child_id).unwrap();
        assert_eq!(child.rect.x, 100.0);
        assert_eq!(child.rect.y, 50.0);
        assert_eq!(child.rect.w, 100.0);
        assert_eq!(child.rect.h, 80.0);
    }

    #[test]
    fn absolute_left_right_auto_width_stretches_between_insets() {
        let mut tree = Tree::new();
        let child_id = tree.insert(container(BoxStyle {
            position: Position::absolute_inset(Inset {
                top: Some(10.0),
                right: Some(25.0),
                bottom: None,
                left: Some(15.0),
            }),
            width: Size::Auto,
            height: Size::Fixed(30.0),
            ..Default::default()
        }));
        let mut root = container(BoxStyle {
            width: Size::Fixed(200.0),
            height: Size::Fixed(100.0),
            ..Default::default()
        });
        root.children = vec![child_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        arrange_root(&mut tree, root_id, 200.0, 100.0);

        let child = tree.get(child_id).unwrap();
        assert_eq!(child.rect.x, 15.0);
        assert_eq!(child.rect.y, 10.0);
        assert_eq!(child.rect.w, 160.0);
        assert_eq!(child.rect.h, 30.0);
    }

    #[test]
    fn absolute_right_bottom_positions_from_containing_block_end() {
        let mut tree = Tree::new();
        let child_id = tree.insert(container(BoxStyle {
            position: Position::absolute_inset(Inset {
                top: None,
                right: Some(25.0),
                bottom: Some(10.0),
                left: None,
            }),
            width: Size::Fixed(50.0),
            height: Size::Fixed(30.0),
            ..Default::default()
        }));
        let mut root = container(BoxStyle {
            width: Size::Fixed(200.0),
            height: Size::Fixed(100.0),
            ..Default::default()
        });
        root.children = vec![child_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        arrange_root(&mut tree, root_id, 200.0, 100.0);

        let child = tree.get(child_id).unwrap();
        assert_eq!(child.rect.x, 125.0);
        assert_eq!(child.rect.y, 60.0);
        assert_eq!(child.rect.w, 50.0);
        assert_eq!(child.rect.h, 30.0);
    }

    #[test]
    fn relative_child_offsets_rect_but_keeps_flow_slot() {
        let mut tree = Tree::new();
        let relative_id = tree.insert(container(BoxStyle {
            position: Position::relative_inset(Inset::xy(10.0, 5.0)),
            width: Size::Fixed(40.0),
            height: Size::Fixed(10.0),
            ..Default::default()
        }));
        let sibling_id = tree.insert(container(BoxStyle {
            width: Size::Fixed(40.0),
            height: Size::Fixed(10.0),
            ..Default::default()
        }));
        let mut root = container(BoxStyle {
            width: Size::Fixed(100.0),
            height: Size::Fixed(50.0),
            direction: Direction::Column,
            ..Default::default()
        });
        root.children = vec![relative_id, sibling_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        arrange_root(&mut tree, root_id, 100.0, 50.0);

        let relative = tree.get(relative_id).unwrap();
        let sibling = tree.get(sibling_id).unwrap();
        assert_eq!(relative.rect.x, 10.0);
        assert_eq!(relative.rect.y, 5.0);
        assert_eq!(sibling.rect.y, 10.0);
    }

    #[test]
    fn absolute_descendant_uses_nearest_relative_containing_block() {
        let mut tree = Tree::new();
        let abs_id = tree.insert(container(BoxStyle {
            position: Position::absolute_xy(10.0, 15.0),
            width: Size::Fixed(20.0),
            height: Size::Fixed(10.0),
            ..Default::default()
        }));
        let mut static_wrapper = container(BoxStyle {
            width: Size::Fixed(80.0),
            height: Size::Fixed(50.0),
            padding: Edges::all(5.0),
            ..Default::default()
        });
        static_wrapper.children = vec![abs_id];
        let static_wrapper_id = tree.insert(static_wrapper);
        let mut relative_wrapper = container(BoxStyle {
            position: Position::relative(),
            width: Size::Fixed(100.0),
            height: Size::Fixed(70.0),
            padding: Edges::all(20.0),
            ..Default::default()
        });
        relative_wrapper.children = vec![static_wrapper_id];
        let relative_wrapper_id = tree.insert(relative_wrapper);
        let mut root = container(BoxStyle {
            width: Size::Fixed(200.0),
            height: Size::Fixed(120.0),
            ..Default::default()
        });
        root.children = vec![relative_wrapper_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        arrange_root(&mut tree, root_id, 200.0, 120.0);

        let abs = tree.get(abs_id).unwrap();
        assert_eq!(abs.rect.x, 30.0);
        assert_eq!(abs.rect.y, 35.0);
    }

    #[test]
    fn align_self_stretches_child_when_parent_centers_items() {
        let mut tree = Tree::new();
        let child_id = auto_wrapper_with_fixed_child(
            &mut tree,
            20.0,
            10.0,
            BoxStyle {
                width: Size::Fixed(20.0),
                align_self: Some(Align::Stretch),
                ..Default::default()
            },
        );
        let mut root = container(BoxStyle {
            width: Size::Fixed(100.0),
            height: Size::Fixed(60.0),
            direction: Direction::Row,
            align_items: Align::Center,
            ..Default::default()
        });
        root.children = vec![child_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        arrange_root(&mut tree, root_id, 100.0, 60.0);

        let child = tree.get(child_id).unwrap();
        assert_eq!(child.rect.y, 0.0);
        assert_eq!(child.rect.h, 60.0);
    }

    #[test]
    fn align_self_start_overrides_parent_stretch() {
        let mut tree = Tree::new();
        let child_id = tree.insert(container(BoxStyle {
            width: Size::Fixed(20.0),
            height: Size::Fixed(10.0),
            align_self: Some(Align::Start),
            ..Default::default()
        }));
        let mut root = container(BoxStyle {
            width: Size::Fixed(100.0),
            height: Size::Fixed(60.0),
            direction: Direction::Row,
            align_items: Align::Stretch,
            ..Default::default()
        });
        root.children = vec![child_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        arrange_root(&mut tree, root_id, 100.0, 60.0);

        let child = tree.get(child_id).unwrap();
        assert_eq!(child.rect.y, 0.0);
        assert_eq!(child.rect.h, 10.0);
    }

    #[test]
    fn row_shrinks_flexible_child_when_content_overflows() {
        let mut tree = Tree::new();
        let shrink_id = auto_wrapper_with_fixed_child(
            &mut tree,
            120.0,
            10.0,
            BoxStyle {
                flex_shrink: 1.0,
                height: Size::Fixed(10.0),
                ..Default::default()
            },
        );
        let fixed_id = tree.insert(container(BoxStyle {
            width: Size::Fixed(40.0),
            height: Size::Fixed(10.0),
            ..Default::default()
        }));
        let mut root = container(BoxStyle {
            width: Size::Fixed(100.0),
            height: Size::Fixed(20.0),
            direction: Direction::Row,
            ..Default::default()
        });
        root.children = vec![shrink_id, fixed_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let mut measure = no_measure;
        arrange(
            &mut tree,
            root_id,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 20.0,
            },
            &mut measure,
        );

        assert_eq!(tree.get(shrink_id).unwrap().rect.w, 60.0);
        assert_eq!(tree.get(fixed_id).unwrap().rect.w, 40.0);
    }

    #[test]
    fn row_keeps_default_non_shrinking_child_width() {
        let mut tree = Tree::new();
        let first_id = auto_wrapper_with_fixed_child(
            &mut tree,
            120.0,
            10.0,
            BoxStyle {
                height: Size::Fixed(10.0),
                ..Default::default()
            },
        );
        let second_id = tree.insert(container(BoxStyle {
            width: Size::Fixed(40.0),
            height: Size::Fixed(10.0),
            ..Default::default()
        }));
        let mut root = container(BoxStyle {
            width: Size::Fixed(100.0),
            height: Size::Fixed(20.0),
            direction: Direction::Row,
            ..Default::default()
        });
        root.children = vec![first_id, second_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let mut measure = no_measure;
        arrange(
            &mut tree,
            root_id,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 20.0,
            },
            &mut measure,
        );

        assert_eq!(tree.get(first_id).unwrap().rect.w, 120.0);
        assert_eq!(tree.get(second_id).unwrap().rect.x, 120.0);
    }

    #[test]
    fn row_distributes_shrink_by_weighted_base_size() {
        let mut tree = Tree::new();
        let first_id = auto_wrapper_with_fixed_child(
            &mut tree,
            100.0,
            10.0,
            BoxStyle {
                flex_shrink: 2.0,
                height: Size::Fixed(10.0),
                ..Default::default()
            },
        );
        let second_id = auto_wrapper_with_fixed_child(
            &mut tree,
            100.0,
            10.0,
            BoxStyle {
                flex_shrink: 1.0,
                height: Size::Fixed(10.0),
                ..Default::default()
            },
        );
        let mut root = container(BoxStyle {
            width: Size::Fixed(150.0),
            height: Size::Fixed(20.0),
            direction: Direction::Row,
            ..Default::default()
        });
        root.children = vec![first_id, second_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let mut measure = no_measure;
        arrange(
            &mut tree,
            root_id,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 150.0,
                h: 20.0,
            },
            &mut measure,
        );

        assert!((tree.get(first_id).unwrap().rect.w - 66.66667).abs() < 0.001);
        assert!((tree.get(second_id).unwrap().rect.w - 83.33333).abs() < 0.001);
    }

    #[test]
    fn row_shrink_respects_min_width() {
        let mut tree = Tree::new();
        let shrink_id = auto_wrapper_with_fixed_child(
            &mut tree,
            120.0,
            10.0,
            BoxStyle {
                flex_shrink: 1.0,
                min_width: 80.0,
                height: Size::Fixed(10.0),
                ..Default::default()
            },
        );
        let fixed_id = tree.insert(container(BoxStyle {
            width: Size::Fixed(40.0),
            height: Size::Fixed(10.0),
            ..Default::default()
        }));
        let mut root = container(BoxStyle {
            width: Size::Fixed(100.0),
            height: Size::Fixed(20.0),
            direction: Direction::Row,
            ..Default::default()
        });
        root.children = vec![shrink_id, fixed_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let mut measure = no_measure;
        arrange(
            &mut tree,
            root_id,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 20.0,
            },
            &mut measure,
        );

        assert_eq!(tree.get(shrink_id).unwrap().rect.w, 80.0);
    }

    #[test]
    fn column_shrinks_height_and_respects_min_height() {
        let mut tree = Tree::new();
        let shrink_id = auto_wrapper_with_fixed_child(
            &mut tree,
            10.0,
            120.0,
            BoxStyle {
                flex_shrink: 1.0,
                min_height: 70.0,
                width: Size::Fixed(10.0),
                ..Default::default()
            },
        );
        let fixed_id = tree.insert(container(BoxStyle {
            width: Size::Fixed(10.0),
            height: Size::Fixed(40.0),
            ..Default::default()
        }));
        let mut root = container(BoxStyle {
            width: Size::Fixed(20.0),
            height: Size::Fixed(100.0),
            direction: Direction::Column,
            ..Default::default()
        });
        root.children = vec![shrink_id, fixed_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let mut measure = no_measure;
        arrange(
            &mut tree,
            root_id,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 20.0,
                h: 100.0,
            },
            &mut measure,
        );

        assert_eq!(tree.get(shrink_id).unwrap().rect.h, 70.0);
    }

    #[test]
    fn absolute_with_padding() {
        let mut tree = Tree::new();
        let child_id = tree.insert(container(BoxStyle {
            position: Position::absolute_xy(100.0, 50.0),
            width: Size::Fixed(100.0),
            height: Size::Fixed(80.0),
            ..Default::default()
        }));
        let mut root = container(BoxStyle {
            position: Position::relative(),
            width: Size::Fixed(800.0),
            height: Size::Fixed(600.0),
            padding: Edges::all(20.0),
            ..Default::default()
        });
        root.children = vec![child_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let mut measure = no_measure;
        arrange(
            &mut tree,
            root_id,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 800.0,
                h: 600.0,
            },
            &mut measure,
        );

        let child = tree.get(child_id).unwrap();
        assert_eq!(
            child.rect.x, 120.0,
            "应该是 padding.left (20) + abs.x (100)"
        );
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
            position: Position::absolute_xy(200.0, 200.0),
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
        arrange(
            &mut tree,
            root_id,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 800.0,
                h: 600.0,
            },
            &mut measure,
        );

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
    fn absolute_child_does_not_contribute_to_auto_parent_size() {
        let mut tree = Tree::new();
        let flow_id = tree.insert(container(BoxStyle {
            width: Size::Fixed(30.0),
            height: Size::Fixed(20.0),
            ..Default::default()
        }));
        let abs_id = tree.insert(container(BoxStyle {
            position: Position::absolute_xy(0.0, 0.0),
            width: Size::Fixed(200.0),
            height: Size::Fixed(100.0),
            ..Default::default()
        }));
        let mut auto_parent = container(BoxStyle {
            width: Size::Auto,
            height: Size::Auto,
            direction: Direction::Row,
            ..Default::default()
        });
        auto_parent.children = vec![flow_id, abs_id];
        let auto_parent_id = tree.insert(auto_parent);
        let sibling_id = tree.insert(container(BoxStyle {
            width: Size::Fixed(10.0),
            height: Size::Fixed(20.0),
            ..Default::default()
        }));
        let mut root = container(BoxStyle {
            width: Size::Fixed(300.0),
            height: Size::Fixed(100.0),
            direction: Direction::Row,
            align_items: Align::Start,
            ..Default::default()
        });
        root.children = vec![auto_parent_id, sibling_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        arrange_root(&mut tree, root_id, 300.0, 100.0);

        let auto_parent = tree.get(auto_parent_id).unwrap();
        let sibling = tree.get(sibling_id).unwrap();
        assert_eq!(auto_parent.rect.w, 30.0);
        assert_eq!(auto_parent.rect.h, 20.0);
        assert_eq!(sibling.rect.x, 30.0);
    }

    #[test]
    fn absolute_fixed_size() {
        let mut tree = Tree::new();
        let child_id = tree.insert(container(BoxStyle {
            position: Position::absolute_xy(50.0, 50.0),
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
        arrange(
            &mut tree,
            root_id,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 400.0,
                h: 300.0,
            },
            &mut measure,
        );

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
            position: Position::absolute_xy(10.0, 10.0),
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
        arrange(
            &mut tree,
            root_id,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 400.0,
                h: 300.0,
            },
            &mut measure,
        );

        let root = tree.get(root_id).unwrap();
        assert_eq!(
            root.content_height(),
            400.0,
            "scroll content_height 应只累计 Flow 子节点"
        );
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
            transform: Some(Transform {
                translate: [10.0, 20.0],
                scale: 2.0,
                rotate: 0.0,
            }),
            ..Default::default()
        });
        root.children = vec![child_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let mut measure = no_measure;
        arrange(
            &mut tree,
            root_id,
            Rect {
                x: 50.0,
                y: 60.0,
                w: 400.0,
                h: 300.0,
            },
            &mut measure,
        );

        let child = tree.get(child_id).unwrap();
        assert_eq!(
            child.rect.x, 0.0,
            "Transform 父的 Flow 子应从 local origin (0,0) 开始"
        );
        assert_eq!(child.rect.y, 0.0);
        assert_eq!(child.rect.w, 100.0);
        assert_eq!(child.rect.h, 50.0);
    }

    #[test]
    fn transform_node_absolute_children_use_local_content_box() {
        let mut tree = Tree::new();
        let child_id = tree.insert(container(BoxStyle {
            position: Position::absolute_xy(10.0, 15.0),
            width: Size::Fixed(100.0),
            height: Size::Fixed(50.0),
            ..Default::default()
        }));
        let mut root = container(BoxStyle {
            width: Size::Fixed(400.0),
            height: Size::Fixed(300.0),
            padding: Edges::all(20.0),
            transform: Some(Transform {
                translate: [10.0, 20.0],
                scale: 2.0,
                rotate: 0.0,
            }),
            ..Default::default()
        });
        root.children = vec![child_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let mut measure = no_measure;
        arrange(
            &mut tree,
            root_id,
            Rect {
                x: 50.0,
                y: 60.0,
                w: 400.0,
                h: 300.0,
            },
            &mut measure,
        );

        let child = tree.get(child_id).unwrap();
        assert_eq!(
            child.rect.x, 30.0,
            "Transform 父的 Absolute 子应从 local padding + abs.x 开始"
        );
        assert_eq!(child.rect.y, 35.0);
        assert_eq!(child.rect.w, 100.0);
        assert_eq!(child.rect.h, 50.0);
    }
}
