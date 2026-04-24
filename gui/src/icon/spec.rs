use crate::renderer::Color;

use super::IconId;

#[derive(Debug, Clone, PartialEq)]
pub struct IconSpec {
    pub id: IconId,
    pub style: IconStyle,
}

impl IconSpec {
    pub fn new(id: impl Into<IconId>, color: Color) -> Self {
        Self {
            id: id.into(),
            style: IconStyle::monochrome(color),
        }
    }

    pub fn with_style(mut self, style: IconStyle) -> Self {
        self.style = style;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconStyle {
    pub color: Color,
    pub fill: IconPaintOverride,
    pub stroke: IconPaintOverride,
    pub stroke_width: IconStrokeWidth,
    pub opacity: IconOpacity,
    pub fit: IconFit,
}

impl IconStyle {
    pub fn monochrome(color: Color) -> Self {
        Self {
            color,
            fill: IconPaintOverride::ReplaceCurrent(color),
            stroke: IconPaintOverride::ReplaceCurrent(color),
            stroke_width: IconStrokeWidth::Preserve,
            opacity: IconOpacity::OPAQUE,
            fit: IconFit::Contain,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        if matches!(self.fill, IconPaintOverride::ReplaceCurrent(_)) {
            self.fill = IconPaintOverride::ReplaceCurrent(color);
        }
        if matches!(self.stroke, IconPaintOverride::ReplaceCurrent(_)) {
            self.stroke = IconPaintOverride::ReplaceCurrent(color);
        }
        self
    }

    pub fn with_fill(mut self, fill: IconPaintOverride) -> Self {
        self.fill = fill;
        self
    }

    pub fn with_stroke(mut self, stroke: IconPaintOverride) -> Self {
        self.stroke = stroke;
        self
    }

    pub fn with_stroke_width(mut self, stroke_width: IconStrokeWidth) -> Self {
        self.stroke_width = stroke_width;
        self
    }

    pub fn with_opacity(mut self, opacity: IconOpacity) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn with_fit(mut self, fit: IconFit) -> Self {
        self.fit = fit;
        self
    }

    pub(crate) fn raster_color(self) -> Color {
        let opacity = self.opacity.get();
        Color {
            r: self.color.r,
            g: self.color.g,
            b: self.color.b,
            a: self.color.a * opacity,
        }
    }
}

impl Default for IconStyle {
    fn default() -> Self {
        Self::monochrome(Color::WHITE)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IconPaintOverride {
    Preserve,
    ReplaceCurrent(Color),
    Force(Color),
    None,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IconStrokeWidth {
    Preserve,
    SvgUnits(f32),
    ScreenPx(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconFit {
    Stretch,
    Contain,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconOpacity(f32);

impl IconOpacity {
    pub const TRANSPARENT: Self = Self(0.0);
    pub const OPAQUE: Self = Self(1.0);

    pub fn new(value: f32) -> Self {
        if !value.is_finite() {
            return Self::OPAQUE;
        }
        Self(value.clamp(0.0, 1.0))
    }

    pub fn get(self) -> f32 {
        self.0
    }
}

impl Default for IconOpacity {
    fn default() -> Self {
        Self::OPAQUE
    }
}
