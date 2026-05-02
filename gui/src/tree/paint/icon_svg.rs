use crate::icon::{IconFit, IconPaintOverride, IconStrokeWidth, IconStyle};
use crate::paint::{SvgFit, SvgPaintOverride, SvgStrokeWidth, SvgStyle};

pub(super) fn svg_style_from_icon(style: IconStyle) -> SvgStyle {
    SvgStyle {
        color: style.color,
        fill: svg_paint_override_from_icon(style.fill),
        stroke: svg_paint_override_from_icon(style.stroke),
        stroke_width: svg_stroke_width_from_icon(style.stroke_width),
        opacity: style.opacity.get(),
        fit: svg_fit_from_icon(style.fit),
    }
}

fn svg_paint_override_from_icon(override_paint: IconPaintOverride) -> SvgPaintOverride {
    match override_paint {
        IconPaintOverride::Preserve => SvgPaintOverride::Preserve,
        IconPaintOverride::ReplaceCurrent(color) => SvgPaintOverride::ReplaceCurrent(color),
        IconPaintOverride::Force(color) => SvgPaintOverride::Force(color),
        IconPaintOverride::None => SvgPaintOverride::None,
    }
}

fn svg_stroke_width_from_icon(stroke_width: IconStrokeWidth) -> SvgStrokeWidth {
    match stroke_width {
        IconStrokeWidth::Preserve => SvgStrokeWidth::Preserve,
        IconStrokeWidth::SvgUnits(width) => SvgStrokeWidth::SvgUnits(width),
    }
}

fn svg_fit_from_icon(fit: IconFit) -> SvgFit {
    match fit {
        IconFit::Stretch => SvgFit::Stretch,
        IconFit::Contain => SvgFit::Contain,
    }
}
