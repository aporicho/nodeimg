use super::layer::PanelLayer;

/// 拖拽移动状态。
pub struct DragState {
    active_panel: Option<&'static str>,
    last_x: f32,
    last_y: f32,
}

impl DragState {
    pub fn new() -> Self {
        Self { active_panel: None, last_x: 0.0, last_y: 0.0 }
    }

    pub fn start(&mut self, panel_id: &'static str, x: f32, y: f32) {
        self.active_panel = Some(panel_id);
        self.last_x = x;
        self.last_y = y;
    }

    pub fn update(&mut self, x: f32, y: f32, layer: &mut PanelLayer) {
        let Some(id) = self.active_panel else { return };
        let dx = x - self.last_x;
        let dy = y - self.last_y;
        if let Some(frame) = layer.get_mut(id) {
            frame.x += dx;
            frame.y += dy;
        }
        self.last_x = x;
        self.last_y = y;
    }

    pub fn end(&mut self) {
        self.active_panel = None;
    }

    pub fn is_active(&self) -> bool {
        self.active_panel.is_some()
    }
}
