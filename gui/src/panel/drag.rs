use super::layer::PanelLayer;

/// 根据 Action 中的坐标更新面板位置。
pub fn apply_drag_move(layer: &mut PanelLayer, panel_id: &str, x: f32, y: f32, last_x: f32, last_y: f32) {
    let dx = x - last_x;
    let dy = y - last_y;
    if let Some(frame) = layer.get_mut(panel_id) {
        frame.x += dx;
        frame.y += dy;
    }
}
