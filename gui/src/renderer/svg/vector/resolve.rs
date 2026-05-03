use crate::icon::{IconFit, IconPaintOverride, IconStrokeWidth, IconStyle};
use crate::renderer::{
    Color, Fill, LineCap, LineJoin, PathRequest, PathStyle, Point, Rect, Stroke,
};

use super::model::{SvgSize, SvgVectorDocument, SvgVectorPath};

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
