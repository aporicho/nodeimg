use super::types::*;

/// 自底向上计算每个节点的期望尺寸。
pub(crate) fn measure<T: LayoutTree>(tree: &T, node: T::NodeId) -> DesiredSize {
    let children = tree.children(node);
    let style = tree.style(node);

    if children.is_empty() {
        return DesiredSize {
            width: match style.width {
                Size::Fixed(w) => w,
                _ => 0.0,
            },
            height: match style.height {
                Size::Fixed(h) => h,
                _ => 0.0,
            },
        };
    }

    let child_sizes: Vec<DesiredSize> = children.iter().map(|&c| measure(tree, c)).collect();
    let n = child_sizes.len();
    let total_gap = if n > 1 { style.gap * (n as f32 - 1.0) } else { 0.0 };

    let (content_w, content_h) = match style.direction {
        Direction::Column => {
            let w = child_sizes.iter().map(|s| s.width).fold(0.0f32, f32::max);
            let h: f32 = child_sizes.iter().map(|s| s.height).sum::<f32>() + total_gap;
            (w, h)
        }
        Direction::Row => {
            let w: f32 = child_sizes.iter().map(|s| s.width).sum::<f32>() + total_gap;
            let h = child_sizes.iter().map(|s| s.height).fold(0.0f32, f32::max);
            (w, h)
        }
    };

    DesiredSize {
        width: match style.width {
            Size::Fixed(w) => w,
            _ => content_w + style.padding.horizontal() + style.margin.horizontal(),
        },
        height: match style.height {
            Size::Fixed(h) => h,
            _ => content_h + style.padding.vertical() + style.margin.vertical(),
        },
    }
}
