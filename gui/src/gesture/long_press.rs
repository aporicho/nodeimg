use std::time::Instant;
use super::recognizer::{GestureDisposition, GestureRecognizer};
use crate::widget::action::Action;

const MOVE_THRESHOLD: f32 = 3.0;
const LONG_PRESS_DURATION_MS: u128 = 500;

pub struct LongPressRecognizer {
    target_id: String,
    down_x: f32,
    down_y: f32,
    down_time: Instant,
    triggered: bool,
    rejected: bool,
}

impl LongPressRecognizer {
    pub fn new(target_id: String) -> Self {
        Self { target_id, down_x: 0.0, down_y: 0.0, down_time: Instant::now(), triggered: false, rejected: false }
    }
}

impl GestureRecognizer for LongPressRecognizer {
    fn on_pointer_down(&mut self, x: f32, y: f32) -> bool {
        self.down_x = x; self.down_y = y; self.down_time = Instant::now(); true
    }
    fn on_pointer_move(&mut self, x: f32, y: f32) -> GestureDisposition {
        let dx = x - self.down_x; let dy = y - self.down_y;
        if dx * dx + dy * dy > MOVE_THRESHOLD * MOVE_THRESHOLD {
            self.rejected = true; return GestureDisposition::Rejected;
        }
        if Instant::now().duration_since(self.down_time).as_millis() >= LONG_PRESS_DURATION_MS {
            self.triggered = true; return GestureDisposition::Accepted;
        }
        GestureDisposition::Pending
    }
    fn on_pointer_up(&mut self, _x: f32, _y: f32) -> GestureDisposition {
        if self.triggered { GestureDisposition::Accepted } else { GestureDisposition::Rejected }
    }
    fn accept(&mut self) -> Action { Action::LongPress(self.target_id.clone()) }
    fn reject(&mut self) { self.rejected = true; }
}
