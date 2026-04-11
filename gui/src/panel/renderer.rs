use crate::tree::{reconcile, layout, paint, hit_test, Desc, NodeId, Tree};
use crate::renderer::{Rect, Renderer, TextMeasurer};

/// 面板渲染器。封装 reconcile → layout → paint 流水线。
pub struct PanelRenderer {
    tree: Tree,
}

impl PanelRenderer {
    pub fn new() -> Self {
        Self { tree: Tree::new() }
    }

    /// 更新面板内容并重新布局。
    pub fn update(&mut self, desc: Desc, content_rect: Rect, measurer: &mut TextMeasurer) {
        reconcile(&mut self.tree, desc);
        if let Some(root) = self.tree.root() {
            layout(
                &mut self.tree,
                root,
                content_rect,
                &mut |text, size| measurer.measure(text, size),
            );
        }
    }

    /// 绘制面板内容。
    pub fn paint(&self, renderer: &mut Renderer) {
        if let Some(root) = self.tree.root() {
            paint(&self.tree, root, renderer);
        }
    }

    /// 命中测试，返回被点击的 widget id。
    pub fn hit_test(&self, x: f32, y: f32) -> Option<&str> {
        let root = self.tree.root()?;
        let chain = hit_test(&self.tree, root, x, y);
        chain.leaf()
            .and_then(|id| self.tree.get(id))
            .map(|n| n.id.as_ref())
    }

    /// 滚动指定节点。
    pub fn scroll(&mut self, node_id: NodeId, delta: f32) {
        self.tree.scroll(node_id, delta);
    }

    pub fn root(&self) -> Option<NodeId> {
        self.tree.root()
    }

    pub fn tree(&self) -> &Tree {
        &self.tree
    }
}
