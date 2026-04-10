use std::time::Instant;
use super::recognizer::{GestureDisposition, GestureRecognizer};
use crate::widget::action::Action;

const MOVE_THRESHOLD: f32 = 3.0;
const DOUBLE_TAP_TIMEOUT_MS: u128 = 300;

pub struct TapRecognizer {
    target_id: String,
    down_x: f32,
    down_y: f32,
    down_time: Instant,
    last_tap_time: Option<Instant>,
    rejected: bool,
}

impl TapRecognizer {
    pub fn new(target_id: String, last_tap_time: Option<Instant>) -> Self {
        Self { target_id, down_x: 0.0, down_y: 0.0, down_time: Instant::now(), last_tap_time, rejected: false }
    }
}

impl GestureRecognizer for TapRecognizer {
    fn on_pointer_down(&mut self, x: f32, y: f32) -> bool {
        self.down_x = x; self.down_y = y; self.down_time = Instant::now(); true
    }
    fn on_pointer_move(&mut self, x: f32, y: f32) -> GestureDisposition {
        let dx = x - self.down_x; let dy = y - self.down_y;
        if dx * dx + dy * dy > MOVE_THRESHOLD * MOVE_THRESHOLD {
            self.rejected = true; GestureDisposition::Rejected
        } else { GestureDisposition::Pending }
    }
    fn on_pointer_up(&mut self, _x: f32, _y: f32) -> GestureDisposition {
        if self.rejected { GestureDisposition::Rejected } else { GestureDisposition::Accepted }
    }
    fn accept(&mut self) -> Action {
        if let Some(last) = self.last_tap_time {
            if self.down_time.duration_since(last).as_millis() < DOUBLE_TAP_TIMEOUT_MS {
                return Action::DoubleClick(self.target_id.clone());
            }
        }
        Action::Click(self.target_id.clone())
    }
    fn reject(&mut self) { self.rejected = true; }
}
