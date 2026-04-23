use crate::geometry::Rect;

use super::DisplayList;

#[derive(Debug, Clone, PartialEq)]
pub struct LayerPaint {
    pub bounds: Rect,
    pub opacity: f32,
    pub content: Box<DisplayList>,
}

impl LayerPaint {
    pub fn new(bounds: Rect, opacity: f32, content: DisplayList) -> Self {
        Self {
            bounds,
            opacity: normalized_opacity(opacity),
            content: Box::new(content),
        }
    }
}

fn normalized_opacity(opacity: f32) -> f32 {
    if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        }
    }

    #[test]
    fn layer_opacity_is_normalized() {
        assert_eq!(
            LayerPaint::new(rect(), -1.0, DisplayList::default()).opacity,
            0.0
        );
        assert_eq!(
            LayerPaint::new(rect(), 2.0, DisplayList::default()).opacity,
            1.0
        );
        assert_eq!(
            LayerPaint::new(rect(), f32::NAN, DisplayList::default()).opacity,
            1.0
        );
    }
}
