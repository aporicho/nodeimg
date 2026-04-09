use crate::renderer::Rect;

use super::measure::measure;
use super::types::*;

/// 自顶向下分配位置，直接写入树节点。
pub(crate) fn arrange<T: LayoutTree>(tree: &mut T, node: T::NodeId, available: Rect) {
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

    // 扣除 padding → 内容区域
    let content = Rect {
        x: node_rect.x + style.padding.left,
        y: node_rect.y + style.padding.top,
        w: (node_rect.w - style.padding.horizontal()).max(0.0),
        h: (node_rect.h - style.padding.vertical()).max(0.0),
    };

    // 处理滚动
    let scroll_offset = if style.overflow == Overflow::Scroll {
        let offset = tree.scroll_offset(node);

        let child_sizes: Vec<DesiredSize> = children.iter().map(|&c| measure(&*tree, c)).collect();
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

    // 度量子节点 + 预读子节点样式字段
    let is_column = style.direction == Direction::Column;
    let child_sizes: Vec<DesiredSize> = children.iter().map(|&c| measure(&*tree, c)).collect();
    let child_styles: Vec<(f32, Size, Size, f32)> = children.iter().map(|&c| {
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
    for (i, &child) in children.iter().enumerate() {
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

        arrange(tree, child, child_rect);

        main_offset += child_main + style.gap + extra_gap;
    }
}
