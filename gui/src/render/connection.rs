use iced::widget::canvas::{Frame, Path, Stroke};
use iced::Point;
use super::{pin_color, TEXT_MUTED};

pub fn draw_connection(frame: &mut Frame, from: Point, to: Point, data_type: &str) {
    let dx = (to.x - from.x).abs() * 0.4;
    let path = Path::new(|builder| {
        builder.move_to(from);
        builder.bezier_curve_to(
            Point::new(from.x + dx, from.y),
            Point::new(to.x - dx, to.y),
            to,
        );
    });
    let stroke = Stroke::default().with_color(pin_color(data_type)).with_width(1.5);
    frame.stroke(&path, stroke);
}

pub fn draw_temp_connection(frame: &mut Frame, from: Point, to: Point) {
    let dx = (to.x - from.x).abs() * 0.4;
    let path = Path::new(|builder| {
        builder.move_to(from);
        builder.bezier_curve_to(
            Point::new(from.x + dx, from.y),
            Point::new(to.x - dx, to.y),
            to,
        );
    });
    let stroke = Stroke::default().with_color(TEXT_MUTED).with_width(1.5);
    frame.stroke(&path, stroke);
}
