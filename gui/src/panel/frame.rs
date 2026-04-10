use crate::renderer::Rect;

/// 单个面板的框架状态。
pub struct PanelFrame {
    pub id: &'static str,
    pub visible: bool,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub min_w: f32,
    pub min_h: f32,
    pub radius: f32,
}

impl PanelFrame {
    pub fn new(id: &'static str, x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            id,
            visible: true,
            x, y, w, h,
            min_w: 120.0,
            min_h: 80.0,
            radius: 12.0,
        }
    }

    pub fn rect(&self) -> Rect {
        Rect { x: self.x, y: self.y, w: self.w, h: self.h }
    }
}
