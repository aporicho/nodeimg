use super::layer::PanelLayer;

/// 从前到后遍历面板，返回第一个包含坐标的可见面板 id。
pub fn hit_test_panel(layer: &PanelLayer, x: f32, y: f32) -> Option<&'static str> {
    for frame in layer.iter_front_to_back() {
        if !frame.visible {
            continue;
        }
        let r = frame.rect();
        if x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h {
            return Some(frame.id);
        }
    }
    None
}
