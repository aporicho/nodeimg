use crate::renderer::{Border, Color, RectStyle, Renderer, Shadow};

use super::frame::PanelFrame;

/// 管理所有面板的集合，维护 z-order。索引越大越靠前。
pub struct PanelLayer {
    panels: Vec<PanelFrame>,
}

impl PanelLayer {
    pub fn new() -> Self {
        Self { panels: Vec::new() }
    }

    pub fn add(&mut self, frame: PanelFrame) {
        self.panels.push(frame);
    }

    pub fn get(&self, id: &str) -> Option<&PanelFrame> {
        self.panels.iter().find(|p| p.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut PanelFrame> {
        self.panels.iter_mut().find(|p| p.id == id)
    }

    /// 将指定面板提到最前面。
    pub fn bring_to_front(&mut self, id: &str) {
        if let Some(pos) = self.panels.iter().position(|p| p.id == id) {
            let frame = self.panels.remove(pos);
            self.panels.push(frame);
        }
    }

    /// 从前到后遍历（最前面的先返回）。用于 hit test。
    pub fn iter_front_to_back(&self) -> impl Iterator<Item = &PanelFrame> {
        self.panels.iter().rev()
    }

    /// 渲染所有可见面板。content_fn 绘制每个面板的内部内容。
    pub fn render(
        &self,
        renderer: &mut Renderer,
        content_fn: impl Fn(&PanelFrame, &mut Renderer),
    ) {
        for frame in &self.panels {
            if !frame.visible {
                continue;
            }

            let rect = frame.rect();

            // 阴影 + 白色背景 + 描边
            renderer.draw_rect(rect, &RectStyle {
                color: Color::WHITE,
                border: Some(Border {
                    width: 1.0,
                    color: Color { r: 0.894, g: 0.894, b: 0.906, a: 1.0 }, // #e4e4e7
                }),
                radius: [frame.radius; 4],
                shadow: Some(Shadow {
                    color: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.06 },
                    offset: [0.0, 2.0],
                    blur: 8.0,
                    spread: 0.0,
                }),
            });

            // 圆角裁剪 + 内容
            renderer.push_clip(rect, frame.radius);
            content_fn(frame, renderer);
            renderer.pop_clip();
        }
    }
}
