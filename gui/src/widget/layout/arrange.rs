use crate::renderer::Rect;

use super::measure::measure;
use super::types::*;

/// 自顶向下分配位置，收集结果到 rects 和 scroll_areas。
pub(crate) fn arrange(
    node: &LayoutBox,
    available: Rect,
    rects: &mut Vec<(&'static str, Rect)>,
    scroll_areas: &mut Vec<ScrollArea>,
) {
    let style = &node.style;

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
        },
        h: match style.height {
            Size::Fixed(h) => h,
            _ => after_margin.h,
        },
    };

    // 记录有 id 的节点
    if let Some(id) = node.id {
        rects.push((id, node_rect));
    }

    if node.children.is_empty() {
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
        // scroll_areas 由外部管理 offset，这里查找已有的
        let offset = scroll_areas
            .iter()
            .find(|s| node.id.is_some() && Some(s.id) == node.id)
            .map(|s| s.offset)
            .unwrap_or(0.0);

        // 计算内容总高度用于滚动
        let child_sizes: Vec<DesiredSize> = node.children.iter().map(measure).collect();
        let n = child_sizes.len();
        let total_gap = if n > 1 { style.gap * (n as f32 - 1.0) } else { 0.0 };
        let content_height = match style.direction {
            Direction::Column => child_sizes.iter().map(|s| s.height).sum::<f32>() + total_gap,
            Direction::Row => child_sizes.iter().map(|s| s.height).fold(0.0f32, f32::max),
        };

        if let Some(id) = node.id {
            // 更新或插入 scroll area
            if let Some(sa) = scroll_areas.iter_mut().find(|s| s.id == id) {
                sa.viewport = content;
                sa.content_height = content_height;
            } else {
                scroll_areas.push(ScrollArea {
                    id,
                    viewport: content,
                    content_height,
                    offset: 0.0,
                });
            }
        }

        offset
    } else {
        0.0
    };

    // 度量子节点
    let child_sizes: Vec<DesiredSize> = node.children.iter().map(measure).collect();
    let n = child_sizes.len();
    let total_gap = if n > 1 { style.gap * (n as f32 - 1.0) } else { 0.0 };

    let is_column = style.direction == Direction::Column;

    // 主轴可用空间
    let main_available = if is_column { content.h } else { content.w };

    // 计算固定子节点占用 + 收集 flex_grow
    let mut fixed_main: f32 = total_gap;
    let mut total_grow: f32 = 0.0;
    for (i, child) in node.children.iter().enumerate() {
        let child_main = if is_column { child_sizes[i].height } else { child_sizes[i].width };
        let grow = child.style.flex_grow;
        let is_fill = if is_column {
            matches!(child.style.height, Size::Fill)
        } else {
            matches!(child.style.width, Size::Fill)
        };

        if grow > 0.0 || is_fill {
            total_grow += if grow > 0.0 { grow } else { 1.0 };
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
    for (i, child) in node.children.iter().enumerate() {
        let child_desired = &child_sizes[i];
        let grow = child.style.flex_grow;
        let is_fill = if is_column {
            matches!(child.style.height, Size::Fill)
        } else {
            matches!(child.style.width, Size::Fill)
        };
        let effective_grow = if grow > 0.0 {
            grow
        } else if is_fill {
            1.0
        } else {
            0.0
        };

        // 主轴尺寸
        let child_main = if effective_grow > 0.0 {
            remaining * effective_grow / total_grow
        } else if is_column {
            child_desired.height
        } else {
            child_desired.width
        };

        // 交叉轴尺寸和偏移
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

        arrange(child, child_rect, rects, scroll_areas);

        main_offset += child_main + style.gap + extra_gap;
    }
}
