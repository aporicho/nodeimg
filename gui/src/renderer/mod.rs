mod buffer;
mod command;
mod dispatch;
mod pipeline;
mod prepare;
pub(crate) mod text_measurer;
mod types;

pub mod style;

pub use pipeline::svg;
pub use renderer::Renderer;
pub use style::{Border, RectStyle, Shadow, TextStyle};
pub use text_measurer::TextMeasurer;
pub use types::{Color, Point, Rect};

mod renderer;
