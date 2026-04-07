use iced::widget::canvas::Frame;
use iced::Size;

use super::GRID_DOT;

pub fn draw_grid(frame: &mut Frame, offset_x: f32, offset_y: f32, zoom: f32, bounds: Size) {
    let spacing = 24.0 * zoom;
    if spacing < 4.0 { return; }

    let ox = (offset_x * zoom) % spacing;
    let oy = (offset_y * zoom) % spacing;

    let mut x = ox;
    while x < bounds.width {
        let mut y = oy;
        while y < bounds.height {
            let dot = iced::widget::canvas::Path::circle(iced::Point::new(x, y), 1.0);
            frame.fill(&dot, GRID_DOT);
            y += spacing;
        }
        x += spacing;
    }
}
