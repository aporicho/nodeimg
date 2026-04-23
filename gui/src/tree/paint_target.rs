pub use crate::paint::PaintTarget;

use crate::geometry::{Affine2D, Rect};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CustomPaintCx {
    pub local_rect: Rect,
    pub transform: Affine2D,
    pub screen_bounds: Rect,
}
