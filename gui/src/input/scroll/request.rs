use crate::shell::AppEvent;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ScrollRequest {
    x: f32,
    y: f32,
    delta_y: f32,
}

impl ScrollRequest {
    pub(crate) fn from_event(event: &AppEvent) -> Option<Self> {
        match *event {
            AppEvent::ScrollLine { x, y, delta_y, .. } => Some(Self::new(x, y, -delta_y * 32.0)),
            AppEvent::ScrollPixel { x, y, delta_y, .. } => Some(Self::new(x, y, -delta_y)),
            _ => None,
        }
    }

    pub(crate) fn new(x: f32, y: f32, delta_y: f32) -> Self {
        Self { x, y, delta_y }
    }

    pub(crate) fn x(self) -> f32 {
        self.x
    }

    pub(crate) fn y(self) -> f32 {
        self.y
    }

    pub(crate) fn delta_y(self) -> f32 {
        self.delta_y
    }
}
