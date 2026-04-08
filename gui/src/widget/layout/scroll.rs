use super::types::ScrollArea;

/// 更新滚动偏移。delta 为正数向下滚。
pub fn scroll(areas: &mut [ScrollArea], id: &str, delta: f32) {
    if let Some(area) = areas.iter_mut().find(|a| a.id == id) {
        let max_offset = (area.content_height - area.viewport.h).max(0.0);
        area.offset = (area.offset + delta).clamp(0.0, max_offset);
    }
}
