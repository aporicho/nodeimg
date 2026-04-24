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

    const EPS: f32 = 0.0001;

    fn assert_near(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < EPS,
            "actual={actual}, expected={expected}"
        );
    }

    #[test]
    fn zoom_at_keeps_cursor_canvas_point_stable() {
        let mut camera = Camera::new();
        camera.x = 120.0;
        camera.y = 80.0;
        camera.zoom = 1.5;

        let (before_x, before_y) = camera.screen_to_canvas(320.0, 240.0);
        camera.zoom_at(320.0, 240.0, 0.25);
        let (after_x, after_y) = camera.screen_to_canvas(320.0, 240.0);

        assert_near(before_x, after_x);
        assert_near(before_y, after_y);
    }

    #[test]
    fn screen_canvas_roundtrips_after_pan_and_zoom() {
        let mut camera = Camera::new();
        camera.pan(120.0, -40.0);
        camera.zoom_at(320.0, 240.0, 0.75);

        let (canvas_x, canvas_y) = camera.screen_to_canvas(400.0, 300.0);
        let (screen_x, screen_y) = camera.canvas_to_screen(canvas_x, canvas_y);

        assert_near(screen_x, 400.0);
        assert_near(screen_y, 300.0);
    }

    #[test]
    fn pan_preserves_canvas_point_under_shifted_screen_position() {
        let mut camera = Camera::new();
        camera.x = 10.0;
        camera.y = 20.0;
        camera.zoom = 2.0;

        let before = camera.screen_to_canvas(100.0, 100.0);
        camera.pan(20.0, -10.0);
        let after = camera.screen_to_canvas(120.0, 90.0);

        assert_near(before.0, after.0);
        assert_near(before.1, after.1);
    }

    #[test]
    fn zoom_at_clamp_keeps_cursor_canvas_point_stable() {
        let mut min_camera = Camera::new();
        min_camera.zoom = 0.11;
        let before_min = min_camera.screen_to_canvas(100.0, 80.0);
        min_camera.zoom_at(100.0, 80.0, -0.9);
        let after_min = min_camera.screen_to_canvas(100.0, 80.0);
        assert_near(min_camera.zoom, 0.1);
        assert_near(before_min.0, after_min.0);
        assert_near(before_min.1, after_min.1);

        let mut max_camera = Camera::new();
        max_camera.zoom = 9.5;
        let before_max = max_camera.screen_to_canvas(100.0, 80.0);
        max_camera.zoom_at(100.0, 80.0, 1.0);
        let after_max = max_camera.screen_to_canvas(100.0, 80.0);
        assert_near(max_camera.zoom, 10.0);
        assert_near(before_max.0, after_max.0);
        assert_near(before_max.1, after_max.1);
    }
}
