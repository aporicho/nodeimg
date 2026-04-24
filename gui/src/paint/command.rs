use crate::geometry::{Point, Rect};

use super::style::{Color, RectStyle};
use super::{
    ImageStyle, LayerPaint, PathData, PathStyle, Shadow, Stroke, SvgPaint, SvgSourceKey, TextStyle,
    TextureHandle,
};

#[derive(Debug, Clone, PartialEq)]
pub enum PaintCommand {
    Rect(RectPaint),
    Path(PathPaint),
    Circle(CirclePaint),
    Image(ImagePaint),
    Text(TextPaint),
    Shadow(ShadowPaint),
    Svg(SvgPaint),
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
    /// Circle radius in local paint units.
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
    /// Per-corner radii in local paint units.
    pub radius: [f32; 4],
    pub shadow: Shadow,
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
