use super::camera::Camera;

pub struct PanState {
    active: bool,
    last_x: f32,
    last_y: f32,
}

impl PanState {
    pub fn new() -> Self {
        Self {
            active: false,
            last_x: 0.0,
            last_y: 0.0,
        }
    }

    pub fn start(&mut self, x: f32, y: f32) {
        self.active = true;
        self.last_x = x;
        self.last_y = y;
    }

    pub fn update(&mut self, x: f32, y: f32, camera: &mut Camera) {
        if !self.active {
            return;
        }
        let dx = x - self.last_x;
        let dy = y - self.last_y;
        camera.pan(dx, dy);
        self.last_x = x;
        self.last_y = y;
    }

    pub fn end(&mut self) {
        self.active = false;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}
