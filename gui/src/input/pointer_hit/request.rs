use crate::shell::{AppEvent, MouseButton};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PointerHitRequest {
    x: f32,
    y: f32,
    include_resize_hit: bool,
}

impl PointerHitRequest {
    pub(crate) fn from_event(event: &AppEvent) -> Option<Self> {
        match *event {
            AppEvent::MousePress {
                x,
                y,
                button: MouseButton::Left,
            } => Some(Self::new(x, y, true)),
            AppEvent::MouseMove { x, y }
            | AppEvent::MouseRelease { x, y, .. }
            | AppEvent::ScrollLine { x, y, .. }
            | AppEvent::ScrollPixel { x, y, .. }
            | AppEvent::PinchZoom { x, y, .. } => Some(Self::new(x, y, false)),
            AppEvent::MousePress { .. } => None,
            _ => None,
        }
    }

    pub(crate) fn new(x: f32, y: f32, include_resize_hit: bool) -> Self {
        Self {
            x,
            y,
            include_resize_hit,
        }
    }

    pub(crate) fn x(self) -> f32 {
        self.x
    }

    pub(crate) fn y(self) -> f32 {
        self.y
    }

    pub(crate) fn include_resize_hit(self) -> bool {
        self.include_resize_hit
    }
}
