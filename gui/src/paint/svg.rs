use crate::geometry::Rect;

use super::style::Color;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SvgSourceKey {
    pub id: String,
}

impl SvgSourceKey {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SvgFit {
    Stretch,
    Contain,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SvgPaintOverride {
    Preserve,
    ReplaceCurrent(Color),
    Force(Color),
    None,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SvgStrokeWidth {
    /// Preserve the SVG source stroke width in SVG/local units.
    Preserve,
    /// Override the SVG source stroke width in SVG/local units.
    ///
    /// The resolved stroke is part of the SVG geometry and scales with the
    /// DisplayList transform stack.
    SvgUnits(f32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SvgStyle {
    pub color: Color,
    pub fill: SvgPaintOverride,
    pub stroke: SvgPaintOverride,
    pub stroke_width: SvgStrokeWidth,
    pub opacity: f32,
    pub fit: SvgFit,
}

impl SvgStyle {
    pub fn new(color: Color) -> Self {
        Self {
            color,
            fill: SvgPaintOverride::ReplaceCurrent(color),
            stroke: SvgPaintOverride::ReplaceCurrent(color),
            stroke_width: SvgStrokeWidth::Preserve,
            opacity: 1.0,
            fit: SvgFit::Contain,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        if matches!(self.fill, SvgPaintOverride::ReplaceCurrent(_)) {
            self.fill = SvgPaintOverride::ReplaceCurrent(color);
        }
        if matches!(self.stroke, SvgPaintOverride::ReplaceCurrent(_)) {
            self.stroke = SvgPaintOverride::ReplaceCurrent(color);
        }
        self
    }

    pub fn with_fill(mut self, fill: SvgPaintOverride) -> Self {
        self.fill = fill;
        self
    }

    pub fn with_stroke(mut self, stroke: SvgPaintOverride) -> Self {
        self.stroke = stroke;
        self
    }

    pub fn with_stroke_width(mut self, stroke_width: SvgStrokeWidth) -> Self {
        self.stroke_width = stroke_width;
        self
    }

    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = if opacity.is_finite() {
            opacity.clamp(0.0, 1.0)
        } else {
            1.0
        };
        self
    }

    pub fn with_fit(mut self, fit: SvgFit) -> Self {
        self.fit = fit;
        self
    }
}

impl Default for SvgStyle {
    fn default() -> Self {
        Self::new(Color::WHITE)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SvgPaint {
    pub rect: Rect,
    pub source: SvgSourceKey,
    pub style: SvgStyle,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_source_key_preserves_id() {
        assert_eq!(SvgSourceKey::new("plus").id, "plus");
    }

    #[test]
    fn svg_style_new_matches_monochrome_defaults() {
        let style = SvgStyle::new(Color::BLACK);

        assert_eq!(style.color, Color::BLACK);
        assert_eq!(style.fill, SvgPaintOverride::ReplaceCurrent(Color::BLACK));
        assert_eq!(style.stroke, SvgPaintOverride::ReplaceCurrent(Color::BLACK));
        assert_eq!(style.stroke_width, SvgStrokeWidth::Preserve);
        assert_eq!(style.opacity, 1.0);
        assert_eq!(style.fit, SvgFit::Contain);
    }

    #[test]
    fn svg_style_clamps_opacity() {
        assert_eq!(SvgStyle::default().with_opacity(-1.0).opacity, 0.0);
        assert_eq!(SvgStyle::default().with_opacity(2.0).opacity, 1.0);
        assert_eq!(SvgStyle::default().with_opacity(f32::NAN).opacity, 1.0);
    }

    #[test]
    fn svg_style_recolors_current_paint_overrides() {
        let style = SvgStyle::new(Color::BLACK).with_color(Color::WHITE);

        assert_eq!(style.color, Color::WHITE);
        assert_eq!(style.fill, SvgPaintOverride::ReplaceCurrent(Color::WHITE));
        assert_eq!(style.stroke, SvgPaintOverride::ReplaceCurrent(Color::WHITE));
    }
}
