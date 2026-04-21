use crate::renderer::Rect;

use super::arrange;
use super::types::{LayoutTree, Position};

/// 两遍布局：measure → arrange。直接写入树节点的 rect。
///
/// 如果根节点自身声明了 `Position::absolute_xy(x, y)`，
/// 则以该 left/top 作为 available 的起点，使根节点 rect 从指定坐标开始。
/// 这支持 Panel 等顶层 widget 把绝对位置编码在 WidgetBuild.style 里。
pub fn layout<T: LayoutTree>(
    tree: &mut T,
    root: T::NodeId,
    available: Rect,
    measure_text: &mut dyn FnMut(&str, &crate::renderer::TextStyle) -> (f32, f32),
) {
    let effective_available = match tree.style(root).position {
        // 注意：仅调整起点 x/y，w/h 仍以调用方传入为准。
        // PanelProps 使用 Size::Fixed，arrange() 会用 Fixed 值覆盖 available.w/h，行为正确。
        // 若根节点使用 Size::Auto/Fill，available.w/h 会透传，布局结果视调用方传入而定。
        Position::Absolute(position) => Rect {
            x: position.inset.left.unwrap_or(available.x),
            y: position.inset.top.unwrap_or(available.y),
            ..available
        },
        Position::Flow | Position::Relative(_) => available,
    };
    arrange::arrange(tree, root, effective_available, measure_text);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::node::{NodeKind, NodeLocalRuntime, TreeNode};
    use crate::tree::{NodeProps, RuntimeSlots, Tree};
    use std::borrow::Cow;

    fn no_measure(_text: &str, _style: &crate::renderer::TextStyle) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn container(style: super::super::types::BoxStyle) -> TreeNode {
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

    #[test]
    fn root_absolute_xy_still_offsets_root_available_origin() {
        let mut tree = Tree::new();
        let root_id = tree.insert(container(super::super::types::BoxStyle {
            position: Position::absolute_xy(10.0, 15.0),
            width: super::super::types::Size::Fixed(100.0),
            height: super::super::types::Size::Fixed(50.0),
            ..Default::default()
        }));
        tree.set_root(root_id);

        let mut measure = no_measure;
        layout(
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
        assert_eq!(root.rect.x, 10.0);
        assert_eq!(root.rect.y, 15.0);
        assert_eq!(root.rect.w, 100.0);
        assert_eq!(root.rect.h, 50.0);
    }
}
