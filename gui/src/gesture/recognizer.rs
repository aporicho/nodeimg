use crate::widget::action::Action;

/// 识别器的裁决结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestureDisposition {
    Pending,
    Accepted,
    Rejected,
}

/// 手势识别器 trait。
pub trait GestureRecognizer {
    fn on_pointer_down(&mut self, x: f32, y: f32) -> bool;
    fn on_pointer_move(&mut self, x: f32, y: f32) -> GestureDisposition;
    fn on_pointer_up(&mut self, x: f32, y: f32) -> GestureDisposition;
    fn accept(&mut self) -> Action;
    fn reject(&mut self);
}
