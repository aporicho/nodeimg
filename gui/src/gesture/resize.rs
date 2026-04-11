use super::recognizer::{GestureDisposition, GestureRecognizer};
use crate::renderer::Rect;
use crate::widget::action::Action;
use crate::widget::resize_edge::ResizeEdge;

const EDGE_THRESHOLD: f32 = 6.0;
const MOVE_THRESHOLD: f32 = 3.0;

pub struct ResizeRecognizer {
    target_id: String,
    node_rect: Rect,
    edge: Option<ResizeEdge>,
    down_x: f32,
    down_y: f32,
    current_x: f32,
    current_y: f32,
    resizing: bool,
    started: bool,
    done: bool,
}

impl ResizeRecognizer {
    pub fn new(target_id: String, node_rect: Rect) -> Self {
        Self {
            target_id,
            node_rect,
            edge: None,
            down_x: 0.0,
            down_y: 0.0,
            current_x: 0.0,
            current_y: 0.0,
            resizing: false,
            started: false,
            done: false,
        }
    }
}

/// 在 rect 的 8 个边/角附近（6px 阈值）内检测具体命中的是哪个。
/// 返回 None 表示点击在内部（非边缘）。
fn detect_edge(rect: &Rect, x: f32, y: f32) -> Option<ResizeEdge> {
    let near_left = (x - rect.x).abs() < EDGE_THRESHOLD;
    let near_right = (x - (rect.x + rect.w)).abs() < EDGE_THRESHOLD;
    let near_top = (y - rect.y).abs() < EDGE_THRESHOLD;
    let near_bottom = (y - (rect.y + rect.h)).abs() < EDGE_THRESHOLD;

    match (near_left, near_right, near_top, near_bottom) {
        (true, _, true, _) => Some(ResizeEdge::TopLeft),
        (true, _, _, true) => Some(ResizeEdge::BottomLeft),
        (_, true, true, _) => Some(ResizeEdge::TopRight),
        (_, true, _, true) => Some(ResizeEdge::BottomRight),
        (true, _, _, _) => Some(ResizeEdge::Left),
        (_, true, _, _) => Some(ResizeEdge::Right),
        (_, _, true, _) => Some(ResizeEdge::Top),
        (_, _, _, true) => Some(ResizeEdge::Bottom),
        _ => None,
    }
}

impl GestureRecognizer for ResizeRecognizer {
    fn on_pointer_down(&mut self, x: f32, y: f32) -> bool {
        self.down_x = x;
        self.down_y = y;
        self.current_x = x;
        self.current_y = y;
        self.edge = detect_edge(&self.node_rect, x, y);
        self.edge.is_some()
    }

    fn on_pointer_move(&mut self, x: f32, y: f32) -> GestureDisposition {
        self.current_x = x;
        self.current_y = y;
        if self.edge.is_none() {
            return GestureDisposition::Rejected;
        }
        if self.resizing {
            return GestureDisposition::Accepted;
        }
        let dx = x - self.down_x;
        let dy = y - self.down_y;
        if dx * dx + dy * dy > MOVE_THRESHOLD * MOVE_THRESHOLD {
            self.resizing = true;
            GestureDisposition::Accepted
        } else {
            GestureDisposition::Pending
        }
    }

    fn on_pointer_up(&mut self, x: f32, y: f32) -> GestureDisposition {
        self.current_x = x;
        self.current_y = y;
        if self.resizing {
            self.done = true;
            GestureDisposition::Accepted
        } else {
            GestureDisposition::Rejected
        }
    }

    fn accept(&mut self) -> Action {
        let edge = self.edge.expect("accept called before on_pointer_down succeeded");
        if self.done {
            Action::ResizeEnd {
                id: self.target_id.clone(),
                edge,
                x: self.current_x,
                y: self.current_y,
            }
        } else if self.started {
            Action::ResizeMove {
                id: self.target_id.clone(),
                edge,
                x: self.current_x,
                y: self.current_y,
            }
        } else {
            self.started = true;
            Action::ResizeStart {
                id: self.target_id.clone(),
                edge,
                x: self.current_x,
                y: self.current_y,
            }
        }
    }

    fn reject(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect_100() -> Rect {
        Rect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 }
    }

    #[test]
    fn detect_edge_corners() {
        let r = rect_100();
        assert_eq!(detect_edge(&r, 2.0, 2.0), Some(ResizeEdge::TopLeft));
        assert_eq!(detect_edge(&r, 98.0, 2.0), Some(ResizeEdge::TopRight));
        assert_eq!(detect_edge(&r, 2.0, 98.0), Some(ResizeEdge::BottomLeft));
        assert_eq!(detect_edge(&r, 98.0, 98.0), Some(ResizeEdge::BottomRight));
    }

    #[test]
    fn detect_edge_sides() {
        let r = rect_100();
        assert_eq!(detect_edge(&r, 50.0, 2.0), Some(ResizeEdge::Top));
        assert_eq!(detect_edge(&r, 50.0, 98.0), Some(ResizeEdge::Bottom));
        assert_eq!(detect_edge(&r, 2.0, 50.0), Some(ResizeEdge::Left));
        assert_eq!(detect_edge(&r, 98.0, 50.0), Some(ResizeEdge::Right));
    }

    #[test]
    fn detect_edge_inside() {
        let r = rect_100();
        assert_eq!(detect_edge(&r, 50.0, 50.0), None);
        assert_eq!(detect_edge(&r, 20.0, 30.0), None);
    }

    #[test]
    fn recognizer_rejects_interior_down() {
        let mut rec = ResizeRecognizer::new("test".to_string(), rect_100());
        assert!(!rec.on_pointer_down(50.0, 50.0));
    }

    #[test]
    fn recognizer_accepts_edge_down() {
        let mut rec = ResizeRecognizer::new("test".to_string(), rect_100());
        assert!(rec.on_pointer_down(2.0, 2.0));
        assert!(rec.on_pointer_down(50.0, 2.0));
        assert!(rec.on_pointer_down(2.0, 50.0));
    }

    #[test]
    fn recognizer_pending_until_move_threshold() {
        let mut rec = ResizeRecognizer::new("test".to_string(), rect_100());
        rec.on_pointer_down(2.0, 2.0);
        assert_eq!(rec.on_pointer_move(2.0, 2.0), GestureDisposition::Pending);
        assert_eq!(rec.on_pointer_move(3.0, 3.0), GestureDisposition::Pending);
    }

    #[test]
    fn recognizer_accept_on_sufficient_move() {
        let mut rec = ResizeRecognizer::new("test".to_string(), rect_100());
        rec.on_pointer_down(2.0, 2.0);
        assert_eq!(rec.on_pointer_move(12.0, 12.0), GestureDisposition::Accepted);
    }

    #[test]
    fn recognizer_reject_on_up_without_move() {
        let mut rec = ResizeRecognizer::new("test".to_string(), rect_100());
        rec.on_pointer_down(2.0, 2.0);
        assert_eq!(rec.on_pointer_up(2.0, 2.0), GestureDisposition::Rejected);
    }

    #[test]
    fn resize_start_action() {
        let mut rec = ResizeRecognizer::new("panel_1".to_string(), rect_100());
        rec.on_pointer_down(2.0, 2.0);
        rec.on_pointer_move(12.0, 12.0);
        match rec.accept() {
            Action::ResizeStart { id, edge, .. } => {
                assert_eq!(id, "panel_1");
                assert_eq!(edge, ResizeEdge::TopLeft);
            }
            other => panic!("期望 ResizeStart，实际 {:?}", other),
        }
    }

    #[test]
    fn resize_move_action() {
        let mut rec = ResizeRecognizer::new("panel_1".to_string(), rect_100());
        rec.on_pointer_down(2.0, 2.0);
        rec.on_pointer_move(12.0, 12.0);
        let _ = rec.accept();
        match rec.accept() {
            Action::ResizeMove { id, edge, .. } => {
                assert_eq!(id, "panel_1");
                assert_eq!(edge, ResizeEdge::TopLeft);
            }
            other => panic!("期望 ResizeMove，实际 {:?}", other),
        }
    }

    #[test]
    fn resize_end_action() {
        let mut rec = ResizeRecognizer::new("panel_1".to_string(), rect_100());
        rec.on_pointer_down(2.0, 2.0);
        rec.on_pointer_move(12.0, 12.0);
        rec.on_pointer_up(12.0, 12.0);
        match rec.accept() {
            Action::ResizeEnd { id, edge, x, y } => {
                assert_eq!(id, "panel_1");
                assert_eq!(edge, ResizeEdge::TopLeft);
                assert_eq!(x, 12.0);
                assert_eq!(y, 12.0);
            }
            other => panic!("期望 ResizeEnd，实际 {:?}", other),
        }
    }

    #[test]
    fn all_eight_edges_action_variants() {
        let cases = [
            (2.0, 2.0, ResizeEdge::TopLeft),
            (98.0, 2.0, ResizeEdge::TopRight),
            (2.0, 98.0, ResizeEdge::BottomLeft),
            (98.0, 98.0, ResizeEdge::BottomRight),
            (50.0, 2.0, ResizeEdge::Top),
            (50.0, 98.0, ResizeEdge::Bottom),
            (2.0, 50.0, ResizeEdge::Left),
            (98.0, 50.0, ResizeEdge::Right),
        ];
        for (x, y, expected_edge) in cases {
            let mut rec = ResizeRecognizer::new("p".to_string(), rect_100());
            assert!(rec.on_pointer_down(x, y), "down at ({}, {}) should succeed", x, y);
            rec.on_pointer_move(x + 20.0, y + 20.0);
            let action = rec.accept();
            match action {
                Action::ResizeStart { edge, .. } => {
                    assert_eq!(edge, expected_edge, "edge mismatch at ({}, {})", x, y);
                }
                other => panic!("期望 ResizeStart，实际 {:?}", other),
            }
        }
    }
}
