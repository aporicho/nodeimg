use std::collections::HashMap;

use crate::geometry::{Affine2D, Rect};
use crate::icon::{
    IconFit, IconId, IconOpacity, IconPaintOverride, IconRegistry, IconStrokeWidth, IconStyle,
};
use crate::paint::{
    CirclePaint, ClipId, ClipShape, DisplayList, ImagePaint, PaintCommand, PathPaint, PathStyle,
    RectPaint, RectStyle, ResolvedClip, ResolvedPaintCommand, Shadow, ShadowPaint, Stroke, SvgFit,
    SvgPaint, SvgPaintOverride, SvgRasterPaint, SvgStrokeWidth, TextPaint, TextStyle,
    TextureHandle,
};
use crate::renderer::{Renderer, TextureResource};

const LEGACY_TRANSFORM_EPSILON: f32 = 1e-5;

// Temporary Phase 2 bridge. Remove in Phase 3 when renderer consumes DisplayList directly.
pub(crate) struct LegacyDisplayListRenderer<'a> {
    renderer: &'a mut Renderer,
    textures: Option<&'a HashMap<TextureHandle, TextureResource>>,
    icons: Option<&'a IconRegistry>,
    active_clips: Vec<ClipId>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct LegacyReplayReport {
    pub unsupported: Vec<UnsupportedPaintCommand>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UnsupportedPaintCommand {
    pub index: usize,
    pub reason: UnsupportedPaintReason,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum UnsupportedPaintReason {
    NonUniformTransform,
    NonSimilarityTransform,
    MissingTexture(TextureHandle),
    MissingSvgSource(String),
    UnsupportedCommand(&'static str),
    UnsupportedClip(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct LegacyTransform {
    tx: f32,
    ty: f32,
    scale: f32,
}

impl LegacyDisplayListRenderer<'_> {
    pub(crate) fn new<'a>(
        renderer: &'a mut Renderer,
        textures: Option<&'a HashMap<TextureHandle, TextureResource>>,
        icons: Option<&'a IconRegistry>,
    ) -> LegacyDisplayListRenderer<'a> {
        LegacyDisplayListRenderer {
            renderer,
            textures,
            icons,
            active_clips: Vec::new(),
        }
    }

    pub(crate) fn render(&mut self, list: &DisplayList) -> LegacyReplayReport {
        let mut report = LegacyReplayReport::default();

        for (index, command) in list.commands.iter().enumerate() {
            if !self.sync_clips(index, &command.clips, &list.clips, &mut report) {
                continue;
            }
            self.render_command(index, command, &mut report);
        }

        self.pop_all_clips();
        report
    }

    fn render_command(
        &mut self,
        index: usize,
        resolved: &ResolvedPaintCommand,
        report: &mut LegacyReplayReport,
    ) {
        match &resolved.command {
            PaintCommand::Rect(paint) => self.render_rect(index, resolved.transform, paint, report),
            PaintCommand::Path(paint) => self.render_path(index, resolved.transform, paint, report),
            PaintCommand::Circle(paint) => {
                self.render_circle(index, resolved.transform, *paint, report);
            }
            PaintCommand::Image(paint) => {
                self.render_image(index, resolved.transform, *paint, report);
            }
            PaintCommand::Text(paint) => self.render_text(index, resolved.transform, paint, report),
            PaintCommand::Shadow(paint) => {
                self.render_shadow(index, resolved.transform, *paint, report);
            }
            PaintCommand::Svg(paint) => self.render_svg(index, resolved.transform, paint, report),
            PaintCommand::SvgRaster(paint) => {
                self.render_svg_raster(index, resolved.transform, paint, report);
            }
            PaintCommand::Layer(_) => {
                report.record(index, UnsupportedPaintReason::UnsupportedCommand("layer"))
            }
        }
    }

    fn render_rect(
        &mut self,
        index: usize,
        transform: Affine2D,
        paint: &RectPaint,
        report: &mut LegacyReplayReport,
    ) {
        let Some(legacy) = legacy_translate_scale(transform) else {
            report.record(index, UnsupportedPaintReason::NonUniformTransform);
            return;
        };
        let rect = legacy.transform_rect(paint.rect);
        let style = scale_rect_style(&paint.style, legacy.scale);
        self.renderer.draw_rect(rect, &style);
    }

    fn render_path(
        &mut self,
        index: usize,
        transform: Affine2D,
        paint: &PathPaint,
        report: &mut LegacyReplayReport,
    ) {
        let Some(legacy) = legacy_translate_scale(transform) else {
            report.record(index, UnsupportedPaintReason::NonUniformTransform);
            return;
        };
        self.renderer.draw_path(
            paint.data.transformed(transform),
            scale_path_style(paint.style, legacy.scale),
        );
    }

    fn render_circle(
        &mut self,
        index: usize,
        transform: Affine2D,
        paint: CirclePaint,
        report: &mut LegacyReplayReport,
    ) {
        let Some(legacy) = legacy_translate_scale(transform) else {
            report.record(index, UnsupportedPaintReason::NonSimilarityTransform);
            return;
        };
        let center = transform.transform_point(paint.center);
        let radius = paint.radius * legacy.scale;
        if let Some(stroke) = paint.stroke {
            self.renderer.draw_circle(center, radius, stroke.color);
        }
        if let Some(fill) = paint.fill {
            let fill_radius = paint
                .stroke
                .map(|stroke| radius - stroke.width * legacy.scale)
                .unwrap_or(radius)
                .max(0.0);
            self.renderer.draw_circle(center, fill_radius, fill);
        }
    }

    fn render_image(
        &mut self,
        index: usize,
        transform: Affine2D,
        paint: ImagePaint,
        report: &mut LegacyReplayReport,
    ) {
        let Some(legacy) = legacy_translate_scale(transform) else {
            report.record(index, UnsupportedPaintReason::NonUniformTransform);
            return;
        };
        let Some(resource) = self
            .textures
            .and_then(|textures| textures.get(&paint.texture))
            .cloned()
        else {
            report.record(index, UnsupportedPaintReason::MissingTexture(paint.texture));
            return;
        };
        self.renderer.draw_image(
            legacy.transform_rect(paint.rect),
            resource.view,
            resource.size,
            paint.style,
        );
    }

    fn render_text(
        &mut self,
        index: usize,
        transform: Affine2D,
        paint: &TextPaint,
        report: &mut LegacyReplayReport,
    ) {
        let Some(legacy) = legacy_translate_scale(transform) else {
            report.record(index, UnsupportedPaintReason::NonUniformTransform);
            return;
        };
        let pos = transform.transform_point(paint.pos);
        let style = scale_text_style(paint.style, legacy.scale);
        if let Some(bounds) = paint.bounds {
            self.renderer.draw_text_clipped(
                pos,
                &paint.text,
                &style,
                legacy.transform_rect(bounds),
            );
        } else {
            self.renderer.draw_text(pos, &paint.text, &style);
        }
    }

    fn render_shadow(
        &mut self,
        index: usize,
        transform: Affine2D,
        paint: ShadowPaint,
        report: &mut LegacyReplayReport,
    ) {
        let Some(legacy) = legacy_translate_scale(transform) else {
            report.record(index, UnsupportedPaintReason::NonUniformTransform);
            return;
        };
        let style = RectStyle {
            color: crate::paint::Color::TRANSPARENT,
            border: None,
            radius: paint.radius.map(|radius| radius * legacy.scale),
            shadow: Some(scale_shadow(paint.shadow, legacy.scale)),
        };
        self.renderer
            .draw_rect(legacy.transform_rect(paint.rect), &style);
    }

    fn render_svg(
        &mut self,
        index: usize,
        transform: Affine2D,
        paint: &SvgPaint,
        report: &mut LegacyReplayReport,
    ) {
        let Some(legacy) = legacy_translate_scale(transform) else {
            report.record(index, UnsupportedPaintReason::NonUniformTransform);
            return;
        };
        let Some(asset) = self.resolve_svg(index, paint.source.id.as_str(), report) else {
            return;
        };
        self.renderer.draw_svg_icon(
            legacy.transform_rect(paint.rect),
            asset.source.clone(),
            icon_style_from_svg(paint.style),
        );
    }

    fn render_svg_raster(
        &mut self,
        index: usize,
        transform: Affine2D,
        paint: &SvgRasterPaint,
        report: &mut LegacyReplayReport,
    ) {
        let Some(legacy) = legacy_translate_scale(transform) else {
            report.record(index, UnsupportedPaintReason::NonUniformTransform);
            return;
        };
        let Some(asset) = self.resolve_svg(index, paint.source.id.as_str(), report) else {
            return;
        };
        let style = IconStyle::monochrome(paint.color.unwrap_or(crate::paint::Color::WHITE));
        self.renderer.draw_svg_icon(
            legacy.transform_rect(paint.rect),
            asset.source.clone(),
            style,
        );
    }

    fn resolve_svg(
        &mut self,
        index: usize,
        source_id: &str,
        report: &mut LegacyReplayReport,
    ) -> Option<crate::icon::IconAsset> {
        let asset = self
            .icons
            .and_then(|icons| icons.resolve(&IconId::from(source_id)));
        match asset {
            Some(asset) => Some(asset.clone()),
            None => {
                report.record(
                    index,
                    UnsupportedPaintReason::MissingSvgSource(source_id.to_string()),
                );
                None
            }
        }
    }

    fn sync_clips(
        &mut self,
        index: usize,
        desired: &[ClipId],
        clips: &[ResolvedClip],
        report: &mut LegacyReplayReport,
    ) -> bool {
        let common = self
            .active_clips
            .iter()
            .zip(desired)
            .take_while(|(active, wanted)| active == wanted)
            .count();

        for _ in common..self.active_clips.len() {
            self.renderer.pop_clip();
        }
        self.active_clips.truncate(common);

        for clip_id in &desired[common..] {
            let Some(clip) = clips.iter().find(|clip| clip.id == *clip_id) else {
                report.record(
                    index,
                    UnsupportedPaintReason::UnsupportedClip("missing clip"),
                );
                return false;
            };
            if !self.push_clip(index, clip, report) {
                return false;
            }
            self.active_clips.push(*clip_id);
        }

        true
    }

    fn push_clip(
        &mut self,
        index: usize,
        clip: &ResolvedClip,
        report: &mut LegacyReplayReport,
    ) -> bool {
        let Some(legacy) = legacy_translate_scale(clip.transform) else {
            report.record(
                index,
                UnsupportedPaintReason::UnsupportedClip("non-uniform transform"),
            );
            return false;
        };

        match &clip.shape {
            ClipShape::Rect(rect) => {
                self.renderer.push_clip(legacy.transform_rect(*rect), 0.0);
                true
            }
            ClipShape::RoundedRect { rect, radius } => {
                if !all_same_radius(*radius) {
                    report.record(
                        index,
                        UnsupportedPaintReason::UnsupportedClip("per-corner radius"),
                    );
                    return false;
                }
                self.renderer
                    .push_clip(legacy.transform_rect(*rect), radius[0] * legacy.scale);
                true
            }
            ClipShape::Path { .. } => {
                report.record(index, UnsupportedPaintReason::UnsupportedClip("path"));
                false
            }
        }
    }

    fn pop_all_clips(&mut self) {
        for _ in 0..self.active_clips.len() {
            self.renderer.pop_clip();
        }
        self.active_clips.clear();
    }
}

impl LegacyTransform {
    fn transform_rect(self, rect: Rect) -> Rect {
        Rect {
            x: self.tx + rect.x * self.scale,
            y: self.ty + rect.y * self.scale,
            w: rect.w * self.scale,
            h: rect.h * self.scale,
        }
    }
}

impl LegacyReplayReport {
    fn record(&mut self, index: usize, reason: UnsupportedPaintReason) {
        tracing::warn!(
            "DisplayList command {index} is unsupported by the temporary legacy replay: {:?}",
            reason
        );
        self.unsupported
            .push(UnsupportedPaintCommand { index, reason });
    }
}

fn legacy_translate_scale(transform: Affine2D) -> Option<LegacyTransform> {
    if !transform.is_finite()
        || transform.xy.abs() > LEGACY_TRANSFORM_EPSILON
        || transform.yx.abs() > LEGACY_TRANSFORM_EPSILON
        || (transform.xx - transform.yy).abs() > LEGACY_TRANSFORM_EPSILON
        || transform.xx < 0.0
    {
        return None;
    }
    Some(LegacyTransform {
        tx: transform.tx,
        ty: transform.ty,
        scale: transform.xx,
    })
}

fn all_same_radius(radius: [f32; 4]) -> bool {
    radius
        .iter()
        .all(|value| (*value - radius[0]).abs() <= LEGACY_TRANSFORM_EPSILON)
}

fn scale_stroke(mut stroke: Stroke, scale: f32) -> Stroke {
    stroke.width *= scale;
    stroke
}

fn scale_path_style(mut style: PathStyle, scale: f32) -> PathStyle {
    style.stroke = style.stroke.map(|stroke| scale_stroke(stroke, scale));
    style
}

fn scale_text_style(mut style: TextStyle, scale: f32) -> TextStyle {
    style.size *= scale;
    style
}

fn scale_rect_style(style: &RectStyle, scale: f32) -> RectStyle {
    RectStyle {
        color: style.color,
        border: style.border.map(|border| crate::paint::Border {
            width: border.width * scale,
            color: border.color,
        }),
        radius: style.radius.map(|radius| radius * scale),
        shadow: style.shadow.map(|shadow| scale_shadow(shadow, scale)),
    }
}

fn scale_shadow(shadow: Shadow, scale: f32) -> Shadow {
    Shadow {
        color: shadow.color,
        offset: [shadow.offset[0] * scale, shadow.offset[1] * scale],
        blur: shadow.blur * scale,
        spread: shadow.spread * scale,
    }
}

fn icon_style_from_svg(style: crate::paint::SvgStyle) -> IconStyle {
    IconStyle {
        color: style.color,
        fill: icon_paint_override(style.fill),
        stroke: icon_paint_override(style.stroke),
        stroke_width: icon_stroke_width(style.stroke_width),
        opacity: IconOpacity::new(style.opacity),
        fit: icon_fit(style.fit),
    }
}

fn icon_paint_override(override_paint: SvgPaintOverride) -> IconPaintOverride {
    match override_paint {
        SvgPaintOverride::Preserve => IconPaintOverride::Preserve,
        SvgPaintOverride::ReplaceCurrent(color) => IconPaintOverride::ReplaceCurrent(color),
        SvgPaintOverride::Force(color) => IconPaintOverride::Force(color),
        SvgPaintOverride::None => IconPaintOverride::None,
    }
}

fn icon_stroke_width(stroke_width: SvgStrokeWidth) -> IconStrokeWidth {
    match stroke_width {
        SvgStrokeWidth::Preserve => IconStrokeWidth::Preserve,
        SvgStrokeWidth::SvgUnits(width) => IconStrokeWidth::SvgUnits(width),
        SvgStrokeWidth::ScreenPx(width) => IconStrokeWidth::ScreenPx(width),
    }
}

fn icon_fit(fit: SvgFit) -> IconFit {
    match fit {
        SvgFit::Stretch => IconFit::Stretch,
        SvgFit::Contain => IconFit::Contain,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;
    use crate::paint::{Color, Fill, PathData};

    #[test]
    fn legacy_translate_scale_accepts_uniform_translate_scale() {
        let transform = Affine2D::compose(Affine2D::translation(10.0, 20.0), Affine2D::scale(2.0));
        let legacy = legacy_translate_scale(transform).unwrap();

        assert_eq!(
            legacy.transform_rect(Rect {
                x: 1.0,
                y: 2.0,
                w: 3.0,
                h: 4.0,
            }),
            Rect {
                x: 12.0,
                y: 24.0,
                w: 6.0,
                h: 8.0,
            }
        );
    }

    #[test]
    fn legacy_translate_scale_rejects_rotate() {
        assert!(legacy_translate_scale(Affine2D::rotation_radians(0.25)).is_none());
    }

    #[test]
    fn scale_path_style_scales_stroke_only() {
        let style = PathStyle {
            fill: Some(Fill::non_zero(Color::WHITE)),
            stroke: Some(Stroke::new(2.0, Color::BLACK)),
        };

        let scaled = scale_path_style(style, 3.0);

        assert_eq!(scaled.fill, style.fill);
        assert_eq!(scaled.stroke.unwrap().width, 6.0);
    }

    #[test]
    fn icon_style_from_svg_preserves_contract_fields() {
        let style = crate::paint::SvgStyle::new(Color::WHITE)
            .with_fill(SvgPaintOverride::Preserve)
            .with_stroke(SvgPaintOverride::None)
            .with_stroke_width(SvgStrokeWidth::ScreenPx(2.0))
            .with_fit(SvgFit::Stretch)
            .with_opacity(0.5);
        let icon = icon_style_from_svg(style);

        assert_eq!(icon.fill, IconPaintOverride::Preserve);
        assert_eq!(icon.stroke, IconPaintOverride::None);
        assert_eq!(icon.stroke_width, IconStrokeWidth::ScreenPx(2.0));
        assert_eq!(icon.fit, IconFit::Stretch);
        assert_eq!(icon.opacity.get(), 0.5);
    }

    #[test]
    fn unsupported_rotated_rect_is_reported_by_replay_path() {
        let command = ResolvedPaintCommand {
            command: PaintCommand::Rect(RectPaint {
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 10.0,
                    h: 10.0,
                },
                style: RectStyle {
                    color: Color::WHITE,
                    border: None,
                    radius: [0.0; 4],
                    shadow: None,
                },
            }),
            transform: Affine2D::rotation_radians(0.25),
            clips: Vec::new(),
        };
        let mut report = LegacyReplayReport::default();
        report.record(0, UnsupportedPaintReason::NonUniformTransform);

        assert_eq!(command.clips, Vec::<ClipId>::new());
        assert_eq!(
            report.unsupported[0].reason,
            UnsupportedPaintReason::NonUniformTransform
        );
    }

    #[test]
    fn path_transform_uses_affine_points_when_legacy_transform_is_supported() {
        let transform = Affine2D::compose(Affine2D::translation(10.0, 20.0), Affine2D::scale(2.0));
        let path = PathData::line(Point { x: 1.0, y: 2.0 }, Point { x: 3.0, y: 4.0 })
            .transformed(transform);

        assert_eq!(
            path,
            PathData::line(Point { x: 12.0, y: 24.0 }, Point { x: 16.0, y: 28.0 })
        );
    }
}
