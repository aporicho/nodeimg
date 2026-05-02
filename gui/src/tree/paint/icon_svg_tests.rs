use super::icon_svg::svg_style_from_icon;
use crate::icon::{IconFit, IconPaintOverride, IconStrokeWidth, IconStyle};
use crate::paint::{Color, SvgFit, SvgPaintOverride, SvgStrokeWidth};

#[test]
fn svg_style_from_icon_preserves_icon_style_fields() {
    let style = IconStyle::monochrome(Color::WHITE)
        .with_fill(IconPaintOverride::Preserve)
        .with_stroke(IconPaintOverride::None)
        .with_stroke_width(IconStrokeWidth::SvgUnits(2.0))
        .with_fit(IconFit::Stretch);

    let svg = svg_style_from_icon(style);

    assert_eq!(svg.fill, SvgPaintOverride::Preserve);
    assert_eq!(svg.stroke, SvgPaintOverride::None);
    assert_eq!(svg.stroke_width, SvgStrokeWidth::SvgUnits(2.0));
    assert_eq!(svg.fit, SvgFit::Stretch);
}
