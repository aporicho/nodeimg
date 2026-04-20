use super::signal::GestureSignal;

/// 识别器的裁决结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GestureDisposition {
    Pending,
    Accepted,
    Rejected,
}

/// 手势识别器 trait。
pub(crate) trait GestureRecognizer {
    fn on_pointer_down(&mut self, x: f32, y: f32) -> bool;
    fn on_pointer_move(&mut self, x: f32, y: f32) -> GestureDisposition;
    fn on_pointer_up(&mut self, x: f32, y: f32) -> GestureDisposition;
    fn accept(&mut self) -> GestureSignal;
    fn reject(&mut self);
}
