/// 无限画布相机：平移 + 缩放。
pub struct Camera {
    /// 画布原点在屏幕上的 x 坐标
    pub x: f32,
    /// 画布原点在屏幕上的 y 坐标
    pub y: f32,
    /// 缩放倍率，1.0 = 100%
    pub zoom: f32,
}

const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 10.0;

impl Camera {
    pub fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }
    }

    /// 屏幕坐标 → 画布坐标
    pub fn screen_to_canvas(&self, sx: f32, sy: f32) -> (f32, f32) {
        ((sx - self.x) / self.zoom, (sy - self.y) / self.zoom)
    }

    /// 画布坐标 → 屏幕坐标
    pub fn canvas_to_screen(&self, cx: f32, cy: f32) -> (f32, f32) {
        (cx * self.zoom + self.x, cy * self.zoom + self.y)
    }

    /// 以屏幕坐标 (sx, sy) 为中心缩放
    pub fn zoom_at(&mut self, sx: f32, sy: f32, delta: f32) {
        let (canvas_x, canvas_y) = self.screen_to_canvas(sx, sy);
        self.zoom = (self.zoom * (1.0 + delta)).clamp(MIN_ZOOM, MAX_ZOOM);
        let (screen_x, screen_y) = self.canvas_to_screen(canvas_x, canvas_y);
        self.x += sx - screen_x;
        self.y += sy - screen_y;
    }

    /// 平移（屏幕像素偏移量）
    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.x += dx;
        self.y += dy;
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::Camera;

    #[test]
    fn zoom_at_keeps_cursor_canvas_point_stable() {
        let mut camera = Camera::new();
        camera.x = 120.0;
        camera.y = 80.0;
        camera.zoom = 1.5;

        let (before_x, before_y) = camera.screen_to_canvas(320.0, 240.0);
        camera.zoom_at(320.0, 240.0, 0.25);
        let (after_x, after_y) = camera.screen_to_canvas(320.0, 240.0);

        assert!((before_x - after_x).abs() < 0.0001);
        assert!((before_y - after_y).abs() < 0.0001);
    }
}
