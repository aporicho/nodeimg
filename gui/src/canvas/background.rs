use crate::renderer::{Color, Point, Renderer};

use super::camera::Camera;

// zinc 色系（shadcn 风格）
const GRID_SPACING: f32 = 24.0;
const DOT_RADIUS: f32 = 1.0;
const DOT_COLOR: Color = Color { r: 0.831, g: 0.831, b: 0.847, a: 1.0 }; // zinc-300 #d4d4d8
const BG_COLOR: Color = Color { r: 0.980, g: 0.980, b: 0.980, a: 1.0 }; // zinc-50 #fafafa
const MIN_SCREEN_SPACING: f32 = 4.0;

pub fn render(renderer: &mut Renderer, camera: &Camera, viewport_w: f32, viewport_h: f32) {
    renderer.set_clear_color(BG_COLOR);

    let screen_spacing = GRID_SPACING * camera.zoom;
    if screen_spacing < MIN_SCREEN_SPACING {
        return;
    }

    let dot_radius = (DOT_RADIUS * camera.zoom).max(0.5);

    let (canvas_left, canvas_top) = camera.screen_to_canvas(0.0, 0.0);
    let (canvas_right, canvas_bottom) = camera.screen_to_canvas(viewport_w, viewport_h);

    let start_x = (canvas_left / GRID_SPACING).floor() as i32;
    let end_x = (canvas_right / GRID_SPACING).ceil() as i32;
    let start_y = (canvas_top / GRID_SPACING).floor() as i32;
    let end_y = (canvas_bottom / GRID_SPACING).ceil() as i32;

    for gx in start_x..=end_x {
        for gy in start_y..=end_y {
            let cx = gx as f32 * GRID_SPACING;
            let cy = gy as f32 * GRID_SPACING;
            let (sx, sy) = camera.canvas_to_screen(cx, cy);
            renderer.draw_circle(Point { x: sx, y: sy }, dot_radius, DOT_COLOR);
        }
    }
}
