use crate::geometry::{Point, Rect};

use super::style::{Color, RectStyle};
use super::{
    ImageStyle, LayerPaint, PathData, PathStyle, Shadow, Stroke, TextStyle, TextureHandle,
};

#[derive(Debug, Clone, PartialEq)]
pub enum PaintCommand {
    Rect(RectPaint),
    Path(PathPaint),
    Circle(CirclePaint),
    Image(ImagePaint),
    Text(TextPaint),
    Shadow(ShadowPaint),
    SvgRaster(SvgRasterPaint),
    Layer(LayerPaint),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RectPaint {
    pub rect: Rect,
    pub style: RectStyle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathPaint {
    pub data: PathData,
    pub style: PathStyle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CirclePaint {
    pub center: Point,
    pub radius: f32,
    pub fill: Option<Color>,
    pub stroke: Option<Stroke>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImagePaint {
    pub rect: Rect,
    pub texture: TextureHandle,
    pub style: ImageStyle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextPaint {
    pub pos: Point,
    pub text: String,
    pub style: TextStyle,
    pub bounds: Option<Rect>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowPaint {
    pub rect: Rect,
    pub radius: [f32; 4],
    pub shadow: Shadow,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SvgSourceKey {
    pub id: String,
}

impl SvgSourceKey {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SvgRasterPaint {
    pub rect: Rect,
    pub source: SvgSourceKey,
    pub color: Option<Color>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_source_key_preserves_id() {
        assert_eq!(SvgSourceKey::new("plus").id, "plus");
    }

    #[test]
    fn paint_command_wraps_rect() {
        let rect = Rect {
            x: 1.0,
            y: 2.0,
            w: 3.0,
            h: 4.0,
        };
        let style = RectStyle {
            color: Color::WHITE,
            border: None,
            radius: [0.0; 4],
            shadow: None,
        };

        assert_eq!(
            PaintCommand::Rect(RectPaint {
                rect,
                style: style.clone(),
            }),
            PaintCommand::Rect(RectPaint { rect, style })
        );
    }
}
