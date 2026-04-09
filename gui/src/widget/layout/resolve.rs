use crate::widget::node::NodeKind;
use super::{LeafKind, Size};
use crate::renderer::TextMeasurer;
use crate::panel::tree::PanelTree;

/// 解析叶子节点的内容尺寸。遍历树，用 TextMeasurer 将 Auto 尺寸的 Text 叶子量好。
pub fn resolve(tree: &mut PanelTree, measurer: &mut TextMeasurer) {
    for node in tree.iter_mut() {
        if let NodeKind::Leaf(LeafKind::Text { content, font_size, .. }) = &node.kind {
            if node.style.width == Size::Auto {
                let (w, h) = measurer.measure(content, *font_size);
                node.style.width = Size::Fixed(w);
                node.style.height = Size::Fixed(h);
            }
        }
    }
}
