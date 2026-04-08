mod blit;
mod buffer;
mod circle;
mod command;
mod curve;
mod dispatch;
mod image;
mod prepare;
mod quad;
pub(crate) mod shadow;
pub(crate) mod stencil;
mod text;
mod types;

pub mod style;
pub mod svg;

pub use renderer::Renderer;
pub use style::{Border, RectStyle, Shadow, TextStyle};
pub use types::{Color, Point, Rect};

mod renderer;
