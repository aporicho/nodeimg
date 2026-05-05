use super::arrange;
use crate::renderer::Rect;
use crate::tree::layout::{BoxStyle, Size};
use crate::tree::{Tree, TreeNode, TreeNodeBuilder};

/// 构造一个基础 Container 节点（decoration 无、子节点无）
pub(super) fn container(style: BoxStyle) -> TreeNode {
    TreeNodeBuilder::container("test", style).build()
}

/// 不依赖字体的 measure 回调
pub(super) fn no_measure(_text: &str, _style: &crate::renderer::TextStyle) -> (f32, f32) {
    (0.0, 0.0)
}

pub(super) fn auto_wrapper_with_fixed_child(
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

pub(super) fn arrange_root(tree: &mut Tree, root_id: crate::tree::NodeId, w: f32, h: f32) {
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
