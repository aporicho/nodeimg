use crate::geometry::{Affine2D, Point, Rect};

use super::{
    CirclePaint, ClipShape, Color, ImagePaint, ImageStyle, PaintCommand, PathData, PathPaint,
    PathStyle, RectPaint, RectStyle, Shadow, ShadowPaint, Stroke, SvgRasterPaint, SvgSourceKey,
    TextPaint, TextStyle, TextureHandle,
};

pub trait PaintTarget {
    fn push_transform(&mut self, transform: Affine2D);
    fn pop_transform(&mut self);

    fn push_clip(&mut self, clip: ClipShape);
    fn pop_clip(&mut self);

    fn draw(&mut self, command: PaintCommand);

    fn measure_text(&mut self, text: &str, style: &TextStyle) -> (f32, f32);

    fn draw_rect(&mut self, rect: Rect, style: RectStyle) {
        self.draw(PaintCommand::Rect(RectPaint { rect, style }));
    }

    fn draw_path(&mut self, data: PathData, style: PathStyle) {
        self.draw(PaintCommand::Path(PathPaint { data, style }));
    }

    fn draw_circle(&mut self, center: Point, radius: f32, color: Color) {
        self.draw_circle_paint(CirclePaint {
            center,
            radius,
            fill: Some(color),
            stroke: None,
        });
    }

    fn draw_circle_paint(&mut self, circle: CirclePaint) {
        self.draw(PaintCommand::Circle(circle));
    }

    fn draw_image(&mut self, rect: Rect, texture: TextureHandle, style: ImageStyle) {
        self.draw(PaintCommand::Image(ImagePaint {
            rect,
            texture,
            style,
        }));
    }

    fn draw_text(&mut self, pos: Point, text: &str, style: TextStyle) {
        self.draw(PaintCommand::Text(TextPaint {
            pos,
            text: text.to_string(),
            style,
            bounds: None,
        }));
    }

    fn draw_text_clipped(&mut self, pos: Point, text: &str, style: TextStyle, bounds: Rect) {
        self.draw(PaintCommand::Text(TextPaint {
            pos,
            text: text.to_string(),
            style,
            bounds: Some(bounds),
        }));
    }

    fn draw_shadow(&mut self, rect: Rect, radius: [f32; 4], shadow: Shadow) {
        self.draw(PaintCommand::Shadow(ShadowPaint {
            rect,
            radius,
            shadow,
        }));
    }

    fn draw_stroked_circle(&mut self, center: Point, radius: f32, stroke: Stroke) {
        self.draw_circle_paint(CirclePaint {
            center,
            radius,
            fill: None,
            stroke: Some(stroke),
        });
    }

    fn draw_svg_raster(&mut self, rect: Rect, source: SvgSourceKey, color: Option<Color>) {
        self.draw(PaintCommand::SvgRaster(SvgRasterPaint {
            rect,
            source,
            color,
        }));
    }
}
