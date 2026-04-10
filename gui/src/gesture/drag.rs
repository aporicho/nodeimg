use super::recognizer::{GestureDisposition, GestureRecognizer};
use crate::widget::action::Action;

const MOVE_THRESHOLD: f32 = 3.0;

pub struct DragRecognizer {
    target_id: String,
    down_x: f32,
    down_y: f32,
    current_x: f32,
    current_y: f32,
    dragging: bool,
    done: bool,
}

impl DragRecognizer {
    pub fn new(target_id: String) -> Self {
        Self { target_id, down_x: 0.0, down_y: 0.0, current_x: 0.0, current_y: 0.0, dragging: false, done: false }
    }
}

impl GestureRecognizer for DragRecognizer {
    fn on_pointer_down(&mut self, x: f32, y: f32) -> bool {
        self.down_x = x; self.down_y = y; self.current_x = x; self.current_y = y; true
    }
    fn on_pointer_move(&mut self, x: f32, y: f32) -> GestureDisposition {
        self.current_x = x; self.current_y = y;
        if self.dragging { return GestureDisposition::Accepted; }
        let dx = x - self.down_x; let dy = y - self.down_y;
        if dx * dx + dy * dy > MOVE_THRESHOLD * MOVE_THRESHOLD {
            self.dragging = true; GestureDisposition::Accepted
        } else { GestureDisposition::Pending }
    }
    fn on_pointer_up(&mut self, x: f32, y: f32) -> GestureDisposition {
        self.current_x = x; self.current_y = y;
        if self.dragging { self.done = true; GestureDisposition::Accepted }
        else { GestureDisposition::Rejected }
    }
    fn accept(&mut self) -> Action {
        if self.done {
            Action::DragEnd { id: self.target_id.clone(), x: self.current_x, y: self.current_y }
        } else if self.dragging {
            Action::DragMove { id: self.target_id.clone(), x: self.current_x, y: self.current_y }
        } else {
            self.dragging = true;
            Action::DragStart { id: self.target_id.clone(), x: self.current_x, y: self.current_y }
        }
    }
    fn reject(&mut self) {}
}
