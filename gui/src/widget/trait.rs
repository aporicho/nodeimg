use super::layout::LayoutBox;
use crate::renderer::{Rect, Renderer};

/// 所有控件的统一接口。
pub trait Widget {
    fn id(&self) -> &'static str;

    /// 返回布局信息（尺寸、padding 等）
    fn layout(&self, renderer: &mut Renderer) -> LayoutBox;

    /// 渲染控件
    fn paint(&self, rect: Rect, renderer: &mut Renderer);

    /// 点击检测（默认：矩形范围内即命中）
    fn hit_test(&self, rect: Rect, x: f32, y: f32) -> bool {
        x >= rect.x && x <= rect.x + rect.w && y >= rect.y && y <= rect.y + rect.h
    }

    /// 属性哈希（用于 diff 判断是否需要更新）
    fn props_hash(&self) -> u64;
}
