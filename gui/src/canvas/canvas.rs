use super::camera::Camera;
use super::pan::PanState;

pub struct Canvas {
    pub camera: Camera,
    pub(super) pan: PanState,
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            camera: Camera::new(),
            pan: PanState::new(),
        }
    }
}
