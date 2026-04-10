pub mod background;
pub mod connection;
pub mod node;

use iced::Color;

pub const BG: Color = Color::from_rgb(0.98, 0.98, 0.98);
pub const BORDER: Color = Color::from_rgb(0.894, 0.894, 0.906);
pub const BORDER_SELECTED: Color = Color::from_rgb(0.094, 0.094, 0.106);
pub const TEXT_PRIMARY: Color = Color::from_rgb(0.094, 0.094, 0.106);
pub const TEXT_SECONDARY: Color = Color::from_rgb(0.443, 0.443, 0.478);
pub const TEXT_MUTED: Color = Color::from_rgb(0.631, 0.631, 0.663);
pub const NODE_BG: Color = Color::WHITE;
pub const GRID_DOT: Color = Color::from_rgb(0.894, 0.894, 0.906);

pub fn pin_color(data_type: &str) -> Color {
    match data_type {
        "image" => Color::from_rgb(0.231, 0.510, 0.965),
        "float" => Color::from_rgb(0.976, 0.451, 0.086),
        "int"   => Color::from_rgb(0.024, 0.714, 0.831),
        "bool"  => Color::from_rgb(0.937, 0.267, 0.267),
        "string"=> Color::from_rgb(0.631, 0.631, 0.663),
        "color" => Color::from_rgb(0.659, 0.333, 0.969),
        _       => Color::from_rgb(0.631, 0.631, 0.663),
    }
}

pub fn category_color(category: &str) -> Color {
    match category {
        "data"   => Color::from_rgb(0.231, 0.510, 0.965),
        "color"  => Color::from_rgb(0.976, 0.451, 0.086),
        "filter" => Color::from_rgb(0.976, 0.659, 0.831),
        "ai"     => Color::from_rgb(0.745, 0.094, 0.365),
        _        => Color::from_rgb(0.631, 0.631, 0.663),
    }
}
