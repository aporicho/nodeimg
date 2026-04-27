use super::recognizer::{GestureDisposition, GestureRecognizer};
use super::signal::GestureSignal;
use crate::widget::resize_edge::ResizeEdge;

const MOVE_THRESHOLD: f32 = 3.0;

pub struct ResizeRecognizer {
    target_id: String,
    edge: ResizeEdge,
    down_x: f32,
    down_y: f32,
    current_x: f32,
    current_y: f32,
    resizing: bool,
    started: bool,
    done: bool,
}

impl ResizeRecognizer {
    pub fn new(target_id: String, edge: ResizeEdge) -> Self {
        Self {
            target_id,
            edge,
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

impl GestureRecognizer for ResizeRecognizer {
    fn on_pointer_down(&mut self, x: f32, y: f32) -> bool {
        self.down_x = x;
        self.down_y = y;
        self.current_x = x;
        self.current_y = y;
        true
    }

    fn on_pointer_move(&mut self, x: f32, y: f32) -> GestureDisposition {
        self.current_x = x;
        self.current_y = y;
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

    fn accept(&mut self) -> GestureSignal {
        if self.done {
            GestureSignal::ResizeEnd {
                id: self.target_id.clone(),
                edge: self.edge,
                x: self.current_x,
                y: self.current_y,
            }
        } else if self.started {
            GestureSignal::ResizeMove {
                id: self.target_id.clone(),
                edge: self.edge,
                x: self.current_x,
                y: self.current_y,
            }
        } else {
            self.started = true;
            GestureSignal::ResizeStart {
                id: self.target_id.clone(),
                edge: self.edge,
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
    use crate::renderer::Rect;
    use crate::widget::resize_edge::{detect_resize_edge, DEFAULT_RESIZE_EDGE_THRESHOLD};

    fn rect_100() -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        }
    }

    #[test]
    fn detect_edge_corners() {
        let r = rect_100();
        assert_eq!(
            detect_resize_edge(r, 2.0, 2.0, DEFAULT_RESIZE_EDGE_THRESHOLD),
            Some(ResizeEdge::TopLeft)
        );
        assert_eq!(
            detect_resize_edge(r, 98.0, 2.0, DEFAULT_RESIZE_EDGE_THRESHOLD),
            Some(ResizeEdge::TopRight)
        );
        assert_eq!(
            detect_resize_edge(r, 2.0, 98.0, DEFAULT_RESIZE_EDGE_THRESHOLD),
            Some(ResizeEdge::BottomLeft)
        );
        assert_eq!(
            detect_resize_edge(r, 98.0, 98.0, DEFAULT_RESIZE_EDGE_THRESHOLD),
            Some(ResizeEdge::BottomRight)
        );
    }

    #[test]
    fn detect_edge_sides() {
        let r = rect_100();
        assert_eq!(
            detect_resize_edge(r, 50.0, 2.0, DEFAULT_RESIZE_EDGE_THRESHOLD),
            Some(ResizeEdge::Top)
        );
        assert_eq!(
            detect_resize_edge(r, 50.0, 98.0, DEFAULT_RESIZE_EDGE_THRESHOLD),
            Some(ResizeEdge::Bottom)
        );
        assert_eq!(
            detect_resize_edge(r, 2.0, 50.0, DEFAULT_RESIZE_EDGE_THRESHOLD),
            Some(ResizeEdge::Left)
        );
        assert_eq!(
            detect_resize_edge(r, 98.0, 50.0, DEFAULT_RESIZE_EDGE_THRESHOLD),
            Some(ResizeEdge::Right)
        );
    }

    #[test]
    fn detect_edge_inside() {
        let r = rect_100();
        assert_eq!(
            detect_resize_edge(r, 50.0, 50.0, DEFAULT_RESIZE_EDGE_THRESHOLD),
            None
        );
        assert_eq!(
            detect_resize_edge(r, 20.0, 30.0, DEFAULT_RESIZE_EDGE_THRESHOLD),
            None
        );
    }

    #[test]
    fn recognizer_arms_after_edge_resolution() {
        let mut rec = ResizeRecognizer::new("test".to_string(), ResizeEdge::Right);
        assert!(rec.on_pointer_down(50.0, 50.0));
    }

    #[test]
    fn recognizer_accepts_any_down_after_edge_resolution() {
        let mut rec = ResizeRecognizer::new("test".to_string(), ResizeEdge::TopLeft);
        assert!(rec.on_pointer_down(2.0, 2.0));
        assert!(rec.on_pointer_down(50.0, 2.0));
        assert!(rec.on_pointer_down(2.0, 50.0));
    }

    #[test]
    fn recognizer_pending_until_move_threshold() {
        let mut rec = ResizeRecognizer::new("test".to_string(), ResizeEdge::TopLeft);
        rec.on_pointer_down(2.0, 2.0);
        assert_eq!(rec.on_pointer_move(2.0, 2.0), GestureDisposition::Pending);
        assert_eq!(rec.on_pointer_move(3.0, 3.0), GestureDisposition::Pending);
    }

    #[test]
    fn recognizer_accept_on_sufficient_move() {
        let mut rec = ResizeRecognizer::new("test".to_string(), ResizeEdge::TopLeft);
        rec.on_pointer_down(2.0, 2.0);
        assert_eq!(
            rec.on_pointer_move(12.0, 12.0),
            GestureDisposition::Accepted
        );
    }

    #[test]
    fn recognizer_reject_on_up_without_move() {
        let mut rec = ResizeRecognizer::new("test".to_string(), ResizeEdge::TopLeft);
        rec.on_pointer_down(2.0, 2.0);
        assert_eq!(rec.on_pointer_up(2.0, 2.0), GestureDisposition::Rejected);
    }

    #[test]
    fn resize_start_signal() {
        let mut rec = ResizeRecognizer::new("panel_1".to_string(), ResizeEdge::TopLeft);
        rec.on_pointer_down(2.0, 2.0);
        rec.on_pointer_move(12.0, 12.0);
        match rec.accept() {
            GestureSignal::ResizeStart { id, edge, .. } => {
                assert_eq!(id, "panel_1");
                assert_eq!(edge, ResizeEdge::TopLeft);
            }
            other => panic!("期望 ResizeStart，实际 {:?}", other),
        }
    }

    #[test]
    fn resize_move_signal() {
        let mut rec = ResizeRecognizer::new("panel_1".to_string(), ResizeEdge::TopLeft);
        rec.on_pointer_down(2.0, 2.0);
        rec.on_pointer_move(12.0, 12.0);
        let _ = rec.accept();
        match rec.accept() {
            GestureSignal::ResizeMove { id, edge, .. } => {
                assert_eq!(id, "panel_1");
                assert_eq!(edge, ResizeEdge::TopLeft);
            }
            other => panic!("期望 ResizeMove，实际 {:?}", other),
        }
    }

    #[test]
    fn resize_end_signal() {
        let mut rec = ResizeRecognizer::new("panel_1".to_string(), ResizeEdge::TopLeft);
        rec.on_pointer_down(2.0, 2.0);
        rec.on_pointer_move(12.0, 12.0);
        rec.on_pointer_up(12.0, 12.0);
        match rec.accept() {
            GestureSignal::ResizeEnd { id, edge, x, y } => {
                assert_eq!(id, "panel_1");
                assert_eq!(edge, ResizeEdge::TopLeft);
                assert_eq!(x, 12.0);
                assert_eq!(y, 12.0);
            }
            other => panic!("期望 ResizeEnd，实际 {:?}", other),
        }
    }

    #[test]
    fn all_eight_edges_signal_variants() {
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
            let mut rec = ResizeRecognizer::new("p".to_string(), expected_edge);
            assert!(
                rec.on_pointer_down(x, y),
                "down at ({}, {}) should succeed",
                x,
                y
            );
            rec.on_pointer_move(x + 20.0, y + 20.0);
            let signal = rec.accept();
            match signal {
                GestureSignal::ResizeStart { edge, .. } => {
                    assert_eq!(edge, expected_edge, "edge mismatch at ({}, {})", x, y);
                }
                other => panic!("期望 ResizeStart，实际 {:?}", other),
            }
        }
    }
}
