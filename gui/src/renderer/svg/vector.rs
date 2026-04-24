use std::collections::HashMap;
use std::sync::Arc;

use crate::icon::{IconFit, IconPaintOverride, IconStrokeWidth, IconStyle};
use crate::renderer::{
    Color, Fill, FillRule, LineCap, LineJoin, PathData, PathRequest, PathStyle, Point, Rect, Stroke,
};

use super::{SvgError, SvgSource, SvgSourceKey, SvgUnsupportedFeature};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SvgSize {
    pub(crate) width: f32,
    pub(crate) height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SvgVectorDocument {
    pub(crate) source_size: SvgSize,
    pub(crate) paths: Vec<SvgVectorPath>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SvgVectorPath {
    pub(crate) data: PathData,
    pub(crate) fill: Option<Color>,
    pub(crate) fill_rule: FillRule,
    pub(crate) stroke: Option<SvgVectorStroke>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SvgVectorStroke {
    pub(crate) color: Color,
    pub(crate) width: f32,
    pub(crate) cap: LineCap,
    pub(crate) join: LineJoin,
    pub(crate) miter_limit: f32,
}

#[derive(Default)]
pub(crate) struct SvgVectorCache {
    cache: HashMap<SvgSourceKey, Result<Arc<SvgVectorDocument>, SvgError>>,
}

impl SvgVectorCache {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn get_or_parse(
        &mut self,
        source: &SvgSource,
    ) -> Result<Arc<SvgVectorDocument>, SvgError> {
        if let Some(entry) = self.cache.get(source.key()) {
            return entry.clone();
        }

        let parsed = parse_svg_vector(source).map(Arc::new);
        self.cache.insert(source.key().clone(), parsed.clone());
        parsed
    }
}

pub(crate) fn resolve_svg_icon_paths(
    document: &SvgVectorDocument,
    rect: Rect,
    style: IconStyle,
) -> Vec<PathRequest> {
    let layout = IconLayout::resolve(document.source_size, rect, style.fit);
    document
        .paths
        .iter()
        .filter_map(|path| resolve_path(path, layout, style))
        .collect()
}

fn parse_svg_vector(source: &SvgSource) -> Result<SvgVectorDocument, SvgError> {
    let tree = resvg::usvg::Tree::from_data(source.bytes(), &resvg::usvg::Options::default())
        .map_err(|err| SvgError::Parse(err.to_string()))?;
    if !tree.linear_gradients().is_empty() || !tree.radial_gradients().is_empty() {
        return Err(SvgError::unsupported(SvgUnsupportedFeature::GradientPaint));
    }
    if !tree.patterns().is_empty() {
        return Err(SvgError::unsupported(SvgUnsupportedFeature::PatternPaint));
    }
    if !tree.clip_paths().is_empty() {
        return Err(SvgError::unsupported(SvgUnsupportedFeature::GroupEffect(
            "clip_path",
        )));
    }
    if !tree.masks().is_empty() {
        return Err(SvgError::unsupported(SvgUnsupportedFeature::GroupEffect(
            "mask",
        )));
    }
    if !tree.filters().is_empty() {
        return Err(SvgError::unsupported(SvgUnsupportedFeature::GroupEffect(
            "filter",
        )));
    }
    let size = tree.size();
    let mut paths = Vec::new();
    collect_group(tree.root(), &mut paths)?;
    Ok(SvgVectorDocument {
        source_size: SvgSize {
            width: size.width(),
            height: size.height(),
        },
        paths,
    })
}

fn collect_group(
    group: &resvg::usvg::Group,
    paths: &mut Vec<SvgVectorPath>,
) -> Result<(), SvgError> {
    if group.opacity().get() < 0.999 {
        return Err(SvgError::unsupported(SvgUnsupportedFeature::GroupEffect(
            "opacity",
        )));
    }
    if group.blend_mode() != resvg::usvg::BlendMode::Normal {
        return Err(SvgError::unsupported(SvgUnsupportedFeature::GroupEffect(
            "blend_mode",
        )));
    }
    if group.isolate() {
        return Err(SvgError::unsupported(SvgUnsupportedFeature::GroupEffect(
            "isolation",
        )));
    }
    if group.clip_path().is_some() {
        return Err(SvgError::unsupported(SvgUnsupportedFeature::GroupEffect(
            "clip_path",
        )));
    }
    if group.mask().is_some() {
        return Err(SvgError::unsupported(SvgUnsupportedFeature::GroupEffect(
            "mask",
        )));
    }
    if !group.filters().is_empty() {
        return Err(SvgError::unsupported(SvgUnsupportedFeature::GroupEffect(
            "filter",
        )));
    }

    for child in group.children() {
        match child {
            resvg::usvg::Node::Group(group) => collect_group(group, paths)?,
            resvg::usvg::Node::Path(path) => {
                if let Some(path) = convert_path(path)? {
                    paths.push(path);
                }
            }
            resvg::usvg::Node::Image(_) => {
                return Err(SvgError::unsupported(SvgUnsupportedFeature::NonPathNode(
                    "image",
                )));
            }
            resvg::usvg::Node::Text(_) => {
                return Err(SvgError::unsupported(SvgUnsupportedFeature::NonPathNode(
                    "text",
                )));
            }
        }
    }

    Ok(())
}

fn convert_path(path: &resvg::usvg::Path) -> Result<Option<SvgVectorPath>, SvgError> {
    if !path.is_visible() {
        return Ok(None);
    }
    if path.paint_order() == resvg::usvg::PaintOrder::StrokeAndFill {
        return Err(SvgError::unsupported(
            SvgUnsupportedFeature::StrokePaintOrder,
        ));
    }

    Ok(Some(SvgVectorPath {
        data: convert_path_data(path.data(), path.abs_transform()),
        fill: match path.fill() {
            Some(fill) => Some(convert_fill(fill)?),
            None => None,
        },
        fill_rule: path
            .fill()
            .map(convert_fill_rule)
            .unwrap_or(FillRule::NonZero),
        stroke: match path.stroke() {
            Some(stroke) => Some(convert_stroke(stroke)?),
            None => None,
        },
    }))
}

fn convert_path_data(
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

fn convert_point(point: resvg::tiny_skia::Point, transform: resvg::tiny_skia::Transform) -> Point {
    let mut point = point;
    transform.map_point(&mut point);
    Point {
        x: point.x,
        y: point.y,
    }
}

fn convert_fill(fill: &resvg::usvg::Fill) -> Result<Color, SvgError> {
    convert_paint(fill.paint(), fill.opacity().get())
}

fn convert_fill_rule(fill: &resvg::usvg::Fill) -> FillRule {
    match fill.rule() {
        resvg::usvg::FillRule::NonZero => FillRule::NonZero,
        resvg::usvg::FillRule::EvenOdd => FillRule::EvenOdd,
    }
}

fn convert_stroke(stroke: &resvg::usvg::Stroke) -> Result<SvgVectorStroke, SvgError> {
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

#[derive(Debug, Clone, Copy)]
struct IconLayout {
    x: f32,
    y: f32,
    sx: f32,
    sy: f32,
    stroke_scale: f32,
}

impl IconLayout {
    fn resolve(size: SvgSize, rect: Rect, fit: IconFit) -> Self {
        let source_w = size.width.max(1.0);
        let source_h = size.height.max(1.0);
        let sx = rect.w / source_w;
        let sy = rect.h / source_h;
        match fit {
            IconFit::Stretch => Self {
                x: rect.x,
                y: rect.y,
                sx,
                sy,
                stroke_scale: (sx.abs() + sy.abs()) * 0.5,
            },
            IconFit::Contain => {
                let scale = sx.min(sy);
                let width = source_w * scale;
                let height = source_h * scale;
                Self {
                    x: rect.x + (rect.w - width) * 0.5,
                    y: rect.y + (rect.h - height) * 0.5,
                    sx: scale,
                    sy: scale,
                    stroke_scale: scale.abs(),
                }
            }
        }
    }
}

fn resolve_path(path: &SvgVectorPath, layout: IconLayout, style: IconStyle) -> Option<PathRequest> {
    let data = path.data.map_points(|point| Point {
        x: layout.x + point.x * layout.sx,
        y: layout.y + point.y * layout.sy,
    });
    let fill = resolve_fill(path, style);
    let stroke = resolve_stroke(path, layout.stroke_scale, style);

    if fill.is_none() && stroke.is_none() {
        return None;
    }

    Some(PathRequest {
        data,
        style: PathStyle { fill, stroke },
    })
}

fn resolve_fill(path: &SvgVectorPath, style: IconStyle) -> Option<Fill> {
    resolve_paint(path.fill, style.fill, style).map(|color| Fill {
        color,
        rule: path.fill_rule,
    })
}

fn resolve_stroke(path: &SvgVectorPath, stroke_scale: f32, style: IconStyle) -> Option<Stroke> {
    let source_stroke = path.stroke;
    let color = resolve_paint(
        source_stroke.map(|stroke| stroke.color),
        style.stroke,
        style,
    )?;
    let source_width = source_stroke
        .map(|stroke| stroke.width)
        .unwrap_or(1.0)
        .max(0.0);
    let width = match style.stroke_width {
        IconStrokeWidth::Preserve => source_width * stroke_scale,
        IconStrokeWidth::SvgUnits(width) => width.max(0.0) * stroke_scale,
    };
    let cap = source_stroke
        .map(|stroke| stroke.cap)
        .unwrap_or(LineCap::Butt);
    let join = source_stroke
        .map(|stroke| stroke.join)
        .unwrap_or(LineJoin::Miter);
    let miter_limit = source_stroke
        .map(|stroke| stroke.miter_limit)
        .unwrap_or(4.0);

    Some(
        Stroke::new(width, color)
            .with_cap(cap)
            .with_join(join)
            .with_miter_limit(miter_limit),
    )
}

fn resolve_paint(
    source: Option<Color>,
    override_paint: IconPaintOverride,
    style: IconStyle,
) -> Option<Color> {
    let opacity = style.opacity.get();
    match override_paint {
        IconPaintOverride::Preserve => source.map(|color| multiply_alpha(color, opacity)),
        IconPaintOverride::ReplaceCurrent(color) => source.map(|source| Color {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a * source.a * opacity,
        }),
        IconPaintOverride::Force(color) => Some(multiply_alpha(color, opacity)),
        IconPaintOverride::None => None,
    }
}

fn multiply_alpha(color: Color, alpha: f32) -> Color {
    Color {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a * alpha,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::icon::{IconOpacity, IconSpec};
    use crate::renderer::PathCommand;

    fn source(svg: &str) -> SvgSource {
        SvgSource::new("test", Arc::<[u8]>::from(svg.as_bytes()))
    }

    fn rect() -> Rect {
        Rect {
            x: 10.0,
            y: 20.0,
            w: 48.0,
            h: 48.0,
        }
    }

    #[test]
    fn svg_vector_cache_parses_path_segments() {
        let mut cache = SvgVectorCache::new();
        let source = source(
            r##"<svg width="24" height="24" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path d="M2 3L4 5Q6 7 8 9C10 11 12 13 14 15Z" fill="#ff0000"/></svg>"##,
        );

        let document = cache.get_or_parse(&source).expect("svg should parse");

        assert_eq!(
            document.source_size,
            SvgSize {
                width: 24.0,
                height: 24.0
            }
        );
        assert_eq!(
            document.paths[0].data.commands,
            vec![
                PathCommand::MoveTo(Point { x: 2.0, y: 3.0 }),
                PathCommand::LineTo(Point { x: 4.0, y: 5.0 }),
                PathCommand::QuadTo(Point { x: 6.0, y: 7.0 }, Point { x: 8.0, y: 9.0 }),
                PathCommand::CubicTo(
                    Point { x: 10.0, y: 11.0 },
                    Point { x: 12.0, y: 13.0 },
                    Point { x: 14.0, y: 15.0 },
                ),
                PathCommand::Close,
            ]
        );
    }

    #[test]
    fn svg_icon_resolve_replaces_stroke_color_and_scales_width() {
        let mut cache = SvgVectorCache::new();
        let source = source(
            r##"<svg width="24" height="24" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path d="M6 12H18" fill="none" stroke="#000000" stroke-width="2" stroke-linecap="round"/></svg>"##,
        );
        let document = cache.get_or_parse(&source).expect("svg should parse");
        let style = IconSpec::new(
            "plus",
            Color {
                r: 0.2,
                g: 0.4,
                b: 0.6,
                a: 1.0,
            },
        )
        .style
        .with_opacity(IconOpacity::new(0.5));

        let paths = resolve_svg_icon_paths(&document, rect(), style);

        assert_eq!(paths.len(), 1);
        let stroke = paths[0].style.stroke.expect("stroke should resolve");
        assert_eq!(stroke.cap, LineCap::Round);
        assert_eq!(stroke.width, 4.0);
        assert_eq!(
            stroke.color,
            Color {
                r: 0.2,
                g: 0.4,
                b: 0.6,
                a: 0.5,
            }
        );
    }

    #[test]
    fn unsupported_dasharray_returns_explicit_error() {
        let mut cache = SvgVectorCache::new();
        let source = source(
            r##"<svg width="24" height="24" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path d="M0 0H24" fill="none" stroke="#fff" stroke-width="2" stroke-dasharray="2 2"/></svg>"##,
        );

        let err = cache
            .get_or_parse(&source)
            .expect_err("dasharray should not vectorize");

        assert!(matches!(
            err,
            SvgError::Unsupported(SvgUnsupportedFeature::DashArray)
        ));
    }
}
