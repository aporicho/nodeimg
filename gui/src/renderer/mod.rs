mod buffer;
mod command;
mod dispatch;
mod path;
mod pipeline;
mod prepare;
pub(crate) mod text_measurer;
mod types;

pub mod style;

pub use path::{PathCommand, PathData, PathRequest, PathStyle};
pub use pipeline::svg;
pub use renderer::Renderer;
pub use style::{
    Border, Fill, FillRule, LineCap, LineJoin, RectStyle, Shadow, Stroke, TextFamily, TextStyle,
    TextWeight,
};
pub use text_measurer::TextMeasurer;
pub use types::{Color, Point, Rect};

mod renderer;

#[cfg(test)]
pub(crate) mod test_support;
