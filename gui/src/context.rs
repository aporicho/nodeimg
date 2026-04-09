use crate::canvas::CanvasRenderer;
use crate::panel::{PanelFrame, PanelLayer, PanelRenderer};
use crate::renderer::Renderer;

/// GUI 中心对象。持有整棵 GUI 树，统一渲染。
/// Canvas 和 Panel 各自处理自己的子树。
pub struct Context {
    pub canvas: CanvasRenderer,
    pub layer: PanelLayer,
    pub panel: PanelRenderer,
}

impl Context {
    pub fn new() -> Self {
        Self {
            canvas: CanvasRenderer::new(),
            layer: PanelLayer::new(),
            panel: PanelRenderer::new(),
        }
    }

    /// 渲染整棵树：先画布，再面板层。
    pub fn render(&self, renderer: &mut Renderer, viewport_w: f32, viewport_h: f32) {
        self.canvas.render(renderer, viewport_w, viewport_h);
        let panel = &self.panel;
        self.layer.render(renderer, |_frame, renderer| {
            panel.paint(renderer);
        });
    }
}
