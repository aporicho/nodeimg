use crate::renderer::{Color, FillRule, LineCap, LineJoin, PathData, Point};

use super::super::{SvgError, SvgUnsupportedFeature};
use super::model::SvgVectorStroke;

pub(super) fn convert_path_data(
    data: &resvg::tiny_skia::Path,
    transform: resvg::tiny_skia::Transform,
) -> PathData {
    let mut out = PathData::new();
    for segment in data.segments() {
        out = match segment {
            resvg::tiny_skia::PathSegment::MoveTo(point) => {
                out.move_to(convert_point(point, transform))
            }
            resvg::tiny_skia::PathSegment::LineTo(point) => {
                out.line_to(convert_point(point, transform))
            }
            resvg::tiny_skia::PathSegment::QuadTo(ctrl, to) => {
                out.quad_to(convert_point(ctrl, transform), convert_point(to, transform))
            }
            resvg::tiny_skia::PathSegment::CubicTo(ctrl1, ctrl2, to) => out.cubic_to(
                convert_point(ctrl1, transform),
                convert_point(ctrl2, transform),
                convert_point(to, transform),
            ),
            resvg::tiny_skia::PathSegment::Close => out.close(),
        };
    }
    out
}

pub(super) fn convert_fill(fill: &resvg::usvg::Fill) -> Result<Color, SvgError> {
    convert_paint(fill.paint(), fill.opacity().get())
}

pub(super) fn convert_fill_rule(fill: &resvg::usvg::Fill) -> FillRule {
    match fill.rule() {
        resvg::usvg::FillRule::NonZero => FillRule::NonZero,
        resvg::usvg::FillRule::EvenOdd => FillRule::EvenOdd,
    }
}

pub(super) fn convert_stroke(stroke: &resvg::usvg::Stroke) -> Result<SvgVectorStroke, SvgError> {
    if stroke.dasharray().is_some() {
        return Err(SvgError::unsupported(SvgUnsupportedFeature::DashArray));
    }
    Ok(SvgVectorStroke {
        color: convert_paint(stroke.paint(), stroke.opacity().get())?,
        width: stroke.width().get(),
        cap: match stroke.linecap() {
            resvg::usvg::LineCap::Butt => LineCap::Butt,
            resvg::usvg::LineCap::Round => LineCap::Round,
            resvg::usvg::LineCap::Square => LineCap::Square,
        },
        join: match stroke.linejoin() {
            resvg::usvg::LineJoin::Miter | resvg::usvg::LineJoin::MiterClip => LineJoin::Miter,
            resvg::usvg::LineJoin::Round => LineJoin::Round,
            resvg::usvg::LineJoin::Bevel => LineJoin::Bevel,
        },
        miter_limit: stroke.miterlimit().get(),
    })
}

fn convert_point(point: resvg::tiny_skia::Point, transform: resvg::tiny_skia::Transform) -> Point {
    let mut point = point;
    transform.map_point(&mut point);
    Point {
        x: point.x,
        y: point.y,
    }
}

fn convert_paint(paint: &resvg::usvg::Paint, opacity: f32) -> Result<Color, SvgError> {
    match paint {
        resvg::usvg::Paint::Color(color) => Ok(Color {
            r: color.red as f32 / 255.0,
            g: color.green as f32 / 255.0,
            b: color.blue as f32 / 255.0,
            a: opacity,
        }),
        resvg::usvg::Paint::LinearGradient(_) | resvg::usvg::Paint::RadialGradient(_) => {
            Err(SvgError::unsupported(SvgUnsupportedFeature::GradientPaint))
        }
        resvg::usvg::Paint::Pattern(_) => {
            Err(SvgError::unsupported(SvgUnsupportedFeature::PatternPaint))
        }
    }
}
