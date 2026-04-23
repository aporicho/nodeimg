use super::{Color, Rect};

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

    let resolved = match style.fit {
        ImageFit::Stretch => (rect, source),
        ImageFit::Contain => (contain_rect(rect, source_aspect), source),
        ImageFit::Cover => (rect, cover_source(rect, source, source_aspect)),
    };

    ResolvedImageDraw {
        rect: resolved.0,
        uv_rect: resolved.1,
        modulate: style.modulate(),
        filter: style.filter,
    }
}

fn contain_rect(rect: Rect, source_aspect: f32) -> Rect {
    if rect.w <= 0.0 || rect.h <= 0.0 || !source_aspect.is_finite() || source_aspect <= 0.0 {
        return rect;
    }

    let target_aspect = rect.w / rect.h;
    if target_aspect > source_aspect {
        let width = rect.h * source_aspect;
        Rect {
            x: rect.x + (rect.w - width) * 0.5,
            y: rect.y,
            w: width,
            h: rect.h,
        }
    } else {
        let height = rect.w / source_aspect;
        Rect {
            x: rect.x,
            y: rect.y + (rect.h - height) * 0.5,
            w: rect.w,
            h: height,
        }
    }
}

fn cover_source(rect: Rect, source: ImageSourceRect, source_aspect: f32) -> ImageSourceRect {
    if rect.w <= 0.0 || rect.h <= 0.0 || !source_aspect.is_finite() || source_aspect <= 0.0 {
        return source;
    }

    let target_aspect = rect.w / rect.h;
    if target_aspect > source_aspect {
        let visible_h = source.h * (source_aspect / target_aspect);
        ImageSourceRect::new(
            source.x,
            source.y + (source.h - visible_h) * 0.5,
            source.w,
            visible_h,
        )
    } else {
        let visible_w = source.w * (target_aspect / source_aspect);
        ImageSourceRect::new(
            source.x + (source.w - visible_w) * 0.5,
            source.y,
            visible_w,
            source.h,
        )
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

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.001,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn image_style_defaults_to_full_linear_stretch() {
        let style = ImageStyle::default();

        assert_eq!(style.source, ImageSourceRect::FULL);
        assert_eq!(style.fit, ImageFit::Stretch);
        assert_eq!(style.filter, ImageFilter::Linear);
        assert_eq!(style.tint, None);
        assert_eq!(style.opacity, ImageOpacity::OPAQUE);
        assert_eq!(style.modulate(), [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn image_source_rect_clamps_to_normalized_bounds() {
        let source = ImageSourceRect::new(-1.0, 0.75, 2.0, 0.5);

        assert_eq!(
            source,
            ImageSourceRect {
                x: 0.0,
                y: 0.75,
                w: 1.0,
                h: 0.25,
            }
        );
    }

    #[test]
    fn image_opacity_clamps_invalid_values() {
        assert_eq!(ImageOpacity::new(-1.0).get(), 0.0);
        assert_eq!(ImageOpacity::new(2.0).get(), 1.0);
        assert_eq!(ImageOpacity::new(f32::NAN), ImageOpacity::OPAQUE);
    }

    #[test]
    fn resolve_stretch_keeps_rect_and_source() {
        let style = ImageStyle::default().with_source(ImageSourceRect::new(0.2, 0.1, 0.5, 0.25));
        let resolved = resolve_image_draw(
            rect(10.0, 20.0, 300.0, 100.0),
            TextureSize::new(200, 100),
            style,
        );

        assert_eq!(resolved.rect, rect(10.0, 20.0, 300.0, 100.0));
        assert_eq!(resolved.uv_rect, ImageSourceRect::new(0.2, 0.1, 0.5, 0.25));
    }

    #[test]
    fn resolve_contain_centers_image_inside_target() {
        let style = ImageStyle::default().with_fit(ImageFit::Contain);
        let resolved = resolve_image_draw(
            rect(0.0, 0.0, 200.0, 200.0),
            TextureSize::new(200, 100),
            style,
        );

        assert_close(resolved.rect.x, 0.0);
        assert_close(resolved.rect.y, 50.0);
        assert_close(resolved.rect.w, 200.0);
        assert_close(resolved.rect.h, 100.0);
        assert_eq!(resolved.uv_rect, ImageSourceRect::FULL);
    }

    #[test]
    fn resolve_cover_crops_source_to_fill_target() {
        let style = ImageStyle::default().with_fit(ImageFit::Cover);
        let resolved = resolve_image_draw(
            rect(0.0, 0.0, 200.0, 200.0),
            TextureSize::new(200, 100),
            style,
        );

        assert_eq!(resolved.rect, rect(0.0, 0.0, 200.0, 200.0));
        assert_close(resolved.uv_rect.x, 0.25);
        assert_close(resolved.uv_rect.y, 0.0);
        assert_close(resolved.uv_rect.w, 0.5);
        assert_close(resolved.uv_rect.h, 1.0);
    }

    #[test]
    fn resolve_modulates_tint_alpha_with_opacity() {
        let tint = Color {
            r: 0.2,
            g: 0.4,
            b: 0.6,
            a: 0.5,
        };
        let style = ImageStyle::default()
            .with_tint(tint)
            .with_opacity(ImageOpacity::new(0.25));
        let resolved =
            resolve_image_draw(rect(0.0, 0.0, 10.0, 10.0), TextureSize::new(1, 1), style);

        assert_eq!(resolved.modulate, [0.2, 0.4, 0.6, 0.125]);
    }
}
