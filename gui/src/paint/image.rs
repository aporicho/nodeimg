use crate::geometry::Rect;

use super::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFit {
    Stretch,
    Contain,
    Cover,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFilter {
    Linear,
    Nearest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureSize {
    pub width: u32,
    pub height: u32,
}

impl TextureSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn aspect_ratio(self) -> Option<f32> {
        if self.width == 0 || self.height == 0 {
            return None;
        }
        Some(self.width as f32 / self.height as f32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageOpacity(f32);

impl ImageOpacity {
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

impl Default for ImageOpacity {
    fn default() -> Self {
        Self::OPAQUE
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageSourceRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl ImageSourceRect {
    pub const FULL: Self = Self {
        x: 0.0,
        y: 0.0,
        w: 1.0,
        h: 1.0,
    };

    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        let x = finite_or_zero(x).clamp(0.0, 1.0);
        let y = finite_or_zero(y).clamp(0.0, 1.0);
        let max_w = 1.0 - x;
        let max_h = 1.0 - y;

        Self {
            x,
            y,
            w: finite_or_zero(w).clamp(0.0, max_w),
            h: finite_or_zero(h).clamp(0.0, max_h),
        }
    }

    pub fn aspect_ratio(self, texture_size: TextureSize) -> Option<f32> {
        if self.w <= 0.0 || self.h <= 0.0 {
            return None;
        }
        texture_size
            .aspect_ratio()
            .map(|texture_aspect| texture_aspect * (self.w / self.h))
    }
}

impl Default for ImageSourceRect {
    fn default() -> Self {
        Self::FULL
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageStyle {
    pub source: ImageSourceRect,
    pub fit: ImageFit,
    pub filter: ImageFilter,
    pub tint: Option<Color>,
    pub opacity: ImageOpacity,
}

impl ImageStyle {
    pub fn with_tint(mut self, tint: Color) -> Self {
        self.tint = Some(tint);
        self
    }

    pub fn with_opacity(mut self, opacity: ImageOpacity) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn with_source(mut self, source: ImageSourceRect) -> Self {
        self.source = source;
        self
    }

    pub fn with_fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self
    }

    pub fn with_filter(mut self, filter: ImageFilter) -> Self {
        self.filter = filter;
        self
    }

    pub fn modulate(self) -> [f32; 4] {
        let color = self.tint.unwrap_or(Color::WHITE);
        [color.r, color.g, color.b, color.a * self.opacity.get()]
    }
}

impl Default for ImageStyle {
    fn default() -> Self {
        Self {
            source: ImageSourceRect::FULL,
            fit: ImageFit::Stretch,
            filter: ImageFilter::Linear,
            tint: None,
            opacity: ImageOpacity::OPAQUE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedImageDraw {
    pub rect: Rect,
    pub uv_rect: ImageSourceRect,
    pub modulate: [f32; 4],
    pub filter: ImageFilter,
}

pub fn resolve_image_draw(
    rect: Rect,
    texture_size: TextureSize,
    style: ImageStyle,
) -> ResolvedImageDraw {
    let source = style.source;
    let Some(source_aspect) = source.aspect_ratio(texture_size) else {
        return ResolvedImageDraw {
            rect,
            uv_rect: source,
            modulate: style.modulate(),
            filter: style.filter,
        };
    };
    let target_aspect = if rect.h.abs() <= f32::EPSILON {
        None
    } else {
        Some(rect.w.abs() / rect.h.abs())
    };

    match (style.fit, target_aspect) {
        (ImageFit::Stretch, _) | (_, None) => ResolvedImageDraw {
            rect,
            uv_rect: source,
            modulate: style.modulate(),
            filter: style.filter,
        },
        (ImageFit::Contain, Some(target_aspect)) => {
            let mut draw = rect;
            if target_aspect > source_aspect {
                let width = rect.h.abs() * source_aspect;
                draw.x += (rect.w - width.copysign(rect.w)) * 0.5;
                draw.w = width.copysign(rect.w);
            } else {
                let height = rect.w.abs() / source_aspect;
                draw.y += (rect.h - height.copysign(rect.h)) * 0.5;
                draw.h = height.copysign(rect.h);
            }
            ResolvedImageDraw {
                rect: draw,
                uv_rect: source,
                modulate: style.modulate(),
                filter: style.filter,
            }
        }
        (ImageFit::Cover, Some(target_aspect)) => {
            let mut uv = source;
            if target_aspect > source_aspect {
                let wanted_h =
                    source.w / target_aspect * texture_size.aspect_ratio().unwrap_or(1.0);
                let delta = (source.h - wanted_h).max(0.0);
                uv.y += delta * 0.5;
                uv.h -= delta;
            } else {
                let wanted_w =
                    source.h * target_aspect / texture_size.aspect_ratio().unwrap_or(1.0);
                let delta = (source.w - wanted_w).max(0.0);
                uv.x += delta * 0.5;
                uv.w -= delta;
            }
            ResolvedImageDraw {
                rect,
                uv_rect: uv,
                modulate: style.modulate(),
                filter: style.filter,
            }
        }
    }
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(w: f32, h: f32) -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            w,
            h,
        }
    }

    #[test]
    fn image_style_defaults_to_full_linear_stretch() {
        let style = ImageStyle::default();

        assert_eq!(style.source, ImageSourceRect::FULL);
        assert_eq!(style.fit, ImageFit::Stretch);
        assert_eq!(style.filter, ImageFilter::Linear);
        assert_eq!(style.opacity, ImageOpacity::OPAQUE);
    }

    #[test]
    fn image_opacity_clamps_invalid_values() {
        assert_eq!(ImageOpacity::new(-1.0).get(), 0.0);
        assert_eq!(ImageOpacity::new(2.0).get(), 1.0);
        assert_eq!(ImageOpacity::new(f32::NAN).get(), 1.0);
    }

    #[test]
    fn image_source_rect_clamps_to_normalized_bounds() {
        assert_eq!(
            ImageSourceRect::new(-1.0, 0.25, 2.0, f32::NAN),
            ImageSourceRect {
                x: 0.0,
                y: 0.25,
                w: 1.0,
                h: 0.0,
            }
        );
    }

    #[test]
    fn resolve_stretch_keeps_rect_and_source() {
        let draw = resolve_image_draw(
            rect(100.0, 50.0),
            TextureSize::new(200, 100),
            ImageStyle::default(),
        );

        assert_eq!(draw.rect, rect(100.0, 50.0));
        assert_eq!(draw.uv_rect, ImageSourceRect::FULL);
    }

    #[test]
    fn resolve_contain_centers_image_inside_target() {
        let draw = resolve_image_draw(
            rect(100.0, 100.0),
            TextureSize::new(200, 100),
            ImageStyle::default().with_fit(ImageFit::Contain),
        );

        assert_eq!(
            draw.rect,
            Rect {
                x: 0.0,
                y: 25.0,
                w: 100.0,
                h: 50.0,
            }
        );
    }

    #[test]
    fn resolve_cover_crops_source_to_fill_target() {
        let draw = resolve_image_draw(
            rect(100.0, 100.0),
            TextureSize::new(200, 100),
            ImageStyle::default().with_fit(ImageFit::Cover),
        );

        assert_eq!(
            draw.uv_rect,
            ImageSourceRect {
                x: 0.25,
                y: 0.0,
                w: 0.5,
                h: 1.0,
            }
        );
    }

    #[test]
    fn resolve_modulates_tint_alpha_with_opacity() {
        let draw = resolve_image_draw(
            rect(100.0, 100.0),
            TextureSize::new(100, 100),
            ImageStyle::default()
                .with_tint(Color {
                    r: 0.2,
                    g: 0.3,
                    b: 0.4,
                    a: 0.5,
                })
                .with_opacity(ImageOpacity::new(0.5)),
        );

        assert_eq!(draw.modulate, [0.2, 0.3, 0.4, 0.25]);
    }
}
