use crate::geometry::{Affine2D, Rect};
use crate::icon::{IconFit, IconOpacity, IconPaintOverride, IconStrokeWidth, IconStyle};
use crate::paint::{
    CirclePaint, ClipId, ClipShape, Color, DisplayList, ImagePaint, PaintCommand, PathPaint,
    PathStyle, RectPaint, RectStyle, ResolvedClip, ResolvedPaintCommand, Shadow, ShadowPaint,
    Stroke, SvgFit, SvgPaint, SvgPaintOverride, SvgRasterPaint, SvgSourceKey, SvgStrokeWidth,
    SvgStyle, TextPaint, TextStyle, TextureHandle,
};

use super::command::BackendCommand;
use super::display_resources::DisplayResourceResolver;
use super::path::PathRequest;
use super::pipeline::circle::CircleRequest;
use super::pipeline::quad::QuadRequest;
use super::pipeline::shadow::ShadowRequest;
use super::pipeline::text::TextRequest;
use super::svg::{resolve_svg_icon_paths, SvgRasterDraw, SvgVectorCache};

const BACKEND_TRANSFORM_EPSILON: f32 = 1e-5;

#[derive(Default)]
pub(super) struct DisplayBackendOutput {
    pub(super) commands: Vec<BackendCommand>,
    pub(super) report: DisplayRenderReport,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct DisplayRenderReport {
    pub(crate) unsupported: Vec<UnsupportedDisplayCommand>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UnsupportedDisplayCommand {
    pub(crate) index: usize,
    pub(crate) reason: UnsupportedDisplayReason,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum UnsupportedDisplayReason {
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

struct LoweringContext<'a, R> {
    resources: &'a R,
    svg_vector_cache: &'a mut SvgVectorCache,
    active_clips: Vec<ClipId>,
    commands: Vec<BackendCommand>,
    report: DisplayRenderReport,
}

pub(super) fn lower_display_list<R: DisplayResourceResolver>(
    list: &DisplayList,
    resources: &R,
    svg_vector_cache: &mut SvgVectorCache,
) -> DisplayBackendOutput {
    let mut cx = LoweringContext {
        resources,
        svg_vector_cache,
        active_clips: Vec::new(),
        commands: Vec::new(),
        report: DisplayRenderReport::default(),
    };

    for (index, command) in list.commands.iter().enumerate() {
        if !cx.sync_clips(index, &command.clips, &list.clips) {
            continue;
        }
        cx.lower_command(index, command);
    }

    cx.pop_all_clips();

    DisplayBackendOutput {
        commands: cx.commands,
        report: cx.report,
    }
}

impl<R: DisplayResourceResolver> LoweringContext<'_, R> {
    fn lower_command(&mut self, index: usize, resolved: &ResolvedPaintCommand) {
        match &resolved.command {
            PaintCommand::Rect(paint) => self.lower_rect(index, resolved.transform, paint),
            PaintCommand::Path(paint) => self.lower_path(index, resolved.transform, paint),
            PaintCommand::Circle(paint) => self.lower_circle(index, resolved.transform, *paint),
            PaintCommand::Image(paint) => self.lower_image(index, resolved.transform, *paint),
            PaintCommand::Text(paint) => self.lower_text(index, resolved.transform, paint),
            PaintCommand::Shadow(paint) => self.lower_shadow(index, resolved.transform, *paint),
            PaintCommand::Svg(paint) => self.lower_svg(index, resolved.transform, paint),
            PaintCommand::SvgRaster(paint) => {
                self.lower_svg_raster(index, resolved.transform, paint);
            }
            PaintCommand::Layer(_) => {
                self.report
                    .record(index, UnsupportedDisplayReason::UnsupportedCommand("layer"));
            }
        }
    }

    fn lower_rect(&mut self, index: usize, transform: Affine2D, paint: &RectPaint) {
        let Some(legacy) = legacy_translate_uniform_scale(transform) else {
            self.report
                .record(index, UnsupportedDisplayReason::NonUniformTransform);
            return;
        };
        let rect = legacy.rect(paint.rect);
        let style = scale_rect_style(&paint.style, legacy.scale);
        if let Some(shadow) = style.shadow {
            self.commands.push(BackendCommand::Shadow(ShadowRequest {
                rect,
                radius: style.radius,
                shadow,
            }));
        }
        self.commands
            .push(BackendCommand::Rect(QuadRequest::from_style(rect, &style)));
    }

    fn lower_path(&mut self, index: usize, transform: Affine2D, paint: &PathPaint) {
        let Some(legacy) = legacy_translate_uniform_scale(transform) else {
            self.report
                .record(index, UnsupportedDisplayReason::NonUniformTransform);
            return;
        };
        self.commands.push(BackendCommand::Path(PathRequest {
            data: paint.data.transformed(transform),
            style: scale_path_style(paint.style, legacy.scale),
        }));
    }

    fn lower_circle(&mut self, index: usize, transform: Affine2D, paint: CirclePaint) {
        let Some(legacy) = legacy_translate_uniform_scale(transform) else {
            self.report
                .record(index, UnsupportedDisplayReason::NonSimilarityTransform);
            return;
        };
        let center = transform.transform_point(paint.center);
        let radius = paint.radius * legacy.scale;
        if let Some(stroke) = paint.stroke {
            self.commands.push(BackendCommand::Circle(CircleRequest {
                center,
                radius,
                color: stroke.color,
            }));
        }
        if let Some(fill) = paint.fill {
            let fill_radius = paint
                .stroke
                .map(|stroke| radius - stroke.width * legacy.scale)
                .unwrap_or(radius)
                .max(0.0);
            self.commands.push(BackendCommand::Circle(CircleRequest {
                center,
                radius: fill_radius,
                color: fill,
            }));
        }
    }

    fn lower_image(&mut self, index: usize, transform: Affine2D, paint: ImagePaint) {
        let Some(legacy) = legacy_translate_uniform_scale(transform) else {
            self.report
                .record(index, UnsupportedDisplayReason::NonUniformTransform);
            return;
        };
        let Some(resource) = self.resources.texture(paint.texture) else {
            self.report.record(
                index,
                UnsupportedDisplayReason::MissingTexture(paint.texture),
            );
            return;
        };
        self.commands.push(BackendCommand::Image {
            rect: legacy.rect(paint.rect),
            view: resource.view,
            size: resource.size,
            style: paint.style,
        });
    }

    fn lower_text(&mut self, index: usize, transform: Affine2D, paint: &TextPaint) {
        let Some(legacy) = legacy_translate_uniform_scale(transform) else {
            self.report
                .record(index, UnsupportedDisplayReason::NonUniformTransform);
            return;
        };
        self.commands.push(BackendCommand::Text(TextRequest {
            pos: transform.transform_point(paint.pos),
            text: paint.text.clone(),
            style: scale_text_style(paint.style, legacy.scale),
            bounds: paint.bounds.map(|bounds| legacy.rect(bounds)),
        }));
    }

    fn lower_shadow(&mut self, index: usize, transform: Affine2D, paint: ShadowPaint) {
        let Some(legacy) = legacy_translate_uniform_scale(transform) else {
            self.report
                .record(index, UnsupportedDisplayReason::NonUniformTransform);
            return;
        };
        self.commands.push(BackendCommand::Shadow(ShadowRequest {
            rect: legacy.rect(paint.rect),
            radius: paint.radius.map(|radius| radius * legacy.scale),
            shadow: scale_shadow(paint.shadow, legacy.scale),
        }));
    }

    fn lower_svg(&mut self, index: usize, transform: Affine2D, paint: &SvgPaint) {
        let Some(legacy) = legacy_translate_uniform_scale(transform) else {
            self.report
                .record(index, UnsupportedDisplayReason::NonUniformTransform);
            return;
        };
        let Some(source) = self.resolve_svg_source(index, &paint.source) else {
            return;
        };
        match self.svg_vector_cache.get_or_parse(&source) {
            Ok(document) => {
                for request in resolve_svg_icon_paths(
                    &document,
                    legacy.rect(paint.rect),
                    icon_style_from_svg(paint.style),
                ) {
                    self.commands.push(BackendCommand::Path(request));
                }
            }
            Err(err) => {
                if !err.is_unsupported() {
                    tracing::warn!(
                        "SVG icon '{}' could not be parsed as vector: {:?}",
                        source.key().id(),
                        err
                    );
                }
                self.commands.push(BackendCommand::SvgRaster(SvgRasterDraw {
                    rect: legacy.rect(paint.rect),
                    source,
                    style: icon_style_from_svg(paint.style),
                }));
            }
        }
    }

    fn lower_svg_raster(&mut self, index: usize, transform: Affine2D, paint: &SvgRasterPaint) {
        let Some(legacy) = legacy_translate_uniform_scale(transform) else {
            self.report
                .record(index, UnsupportedDisplayReason::NonUniformTransform);
            return;
        };
        let Some(source) = self.resolve_svg_source(index, &paint.source) else {
            return;
        };
        self.commands.push(BackendCommand::SvgRaster(SvgRasterDraw {
            rect: legacy.rect(paint.rect),
            source,
            style: IconStyle::monochrome(paint.color.unwrap_or(Color::WHITE)),
        }));
    }

    fn resolve_svg_source(
        &mut self,
        index: usize,
        key: &SvgSourceKey,
    ) -> Option<super::svg::SvgSource> {
        match self.resources.svg_source(key) {
            Some(source) => Some(source),
            None => {
                self.report.record(
                    index,
                    UnsupportedDisplayReason::MissingSvgSource(key.id.clone()),
                );
                None
            }
        }
    }

    fn sync_clips(&mut self, index: usize, desired: &[ClipId], clips: &[ResolvedClip]) -> bool {
        let common = self
            .active_clips
            .iter()
            .zip(desired)
            .take_while(|(active, wanted)| active == wanted)
            .count();

        for _ in common..self.active_clips.len() {
            self.commands.push(BackendCommand::PopClip);
        }
        self.active_clips.truncate(common);

        let mut pending = Vec::new();
        for clip_id in &desired[common..] {
            let Some(clip) = clips.iter().find(|clip| clip.id == *clip_id) else {
                self.report.record(
                    index,
                    UnsupportedDisplayReason::UnsupportedClip("missing clip"),
                );
                return false;
            };
            let Some(command) = self.lower_clip(index, clip) else {
                return false;
            };
            pending.push((*clip_id, command));
        }

        for (clip_id, command) in pending {
            self.commands.push(command);
            self.active_clips.push(clip_id);
        }

        true
    }

    fn lower_clip(&mut self, index: usize, clip: &ResolvedClip) -> Option<BackendCommand> {
        let Some(legacy) = legacy_translate_uniform_scale(clip.transform) else {
            self.report.record(
                index,
                UnsupportedDisplayReason::UnsupportedClip("non-uniform transform"),
            );
            return None;
        };

        match &clip.shape {
            ClipShape::Rect(rect) => Some(BackendCommand::PushClip {
                rect: legacy.rect(*rect),
                radius: 0.0,
            }),
            ClipShape::RoundedRect { rect, radius } => {
                if !all_same_radius(*radius) {
                    self.report.record(
                        index,
                        UnsupportedDisplayReason::UnsupportedClip("per-corner radius"),
                    );
                    return None;
                }
                Some(BackendCommand::PushClip {
                    rect: legacy.rect(*rect),
                    radius: radius[0] * legacy.scale,
                })
            }
            ClipShape::Path { .. } => {
                self.report
                    .record(index, UnsupportedDisplayReason::UnsupportedClip("path"));
                None
            }
        }
    }

    fn pop_all_clips(&mut self) {
        for _ in 0..self.active_clips.len() {
            self.commands.push(BackendCommand::PopClip);
        }
        self.active_clips.clear();
    }
}

impl DisplayRenderReport {
    fn record(&mut self, index: usize, reason: UnsupportedDisplayReason) {
        tracing::warn!(
            "DisplayList command {index} is unsupported by the renderer display backend: {:?}",
            reason
        );
        self.unsupported
            .push(UnsupportedDisplayCommand { index, reason });
    }
}

impl LegacyTransform {
    fn rect(self, rect: Rect) -> Rect {
        Rect {
            x: self.tx + rect.x * self.scale,
            y: self.ty + rect.y * self.scale,
            w: rect.w * self.scale,
            h: rect.h * self.scale,
        }
    }
}

fn legacy_translate_uniform_scale(transform: Affine2D) -> Option<LegacyTransform> {
    if !transform.is_finite()
        || transform.xy.abs() > BACKEND_TRANSFORM_EPSILON
        || transform.yx.abs() > BACKEND_TRANSFORM_EPSILON
        || (transform.xx - transform.yy).abs() > BACKEND_TRANSFORM_EPSILON
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
        .all(|value| (*value - radius[0]).abs() <= BACKEND_TRANSFORM_EPSILON)
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

fn icon_style_from_svg(style: SvgStyle) -> IconStyle {
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
    use crate::paint::{
        ClipShape, DisplayListBuilder, Fill, ImagePaint, PathData, RectPaint, SvgPaint,
        SvgSourceKey, TextureHandle,
    };
    use crate::renderer::display_resources::EmptyDisplayResources;

    fn rect() -> Rect {
        Rect {
            x: 1.0,
            y: 2.0,
            w: 3.0,
            h: 4.0,
        }
    }

    fn rect_style() -> RectStyle {
        RectStyle {
            color: Color::WHITE,
            border: None,
            radius: [0.0; 4],
            shadow: None,
        }
    }

    fn lower(list: &DisplayList) -> DisplayBackendOutput {
        let mut svg_cache = SvgVectorCache::new();
        lower_display_list(list, &EmptyDisplayResources, &mut svg_cache)
    }

    #[test]
    fn lowers_rect_with_translate_scale() {
        let mut builder = DisplayListBuilder::new();
        builder.push_transform(Affine2D::compose(
            Affine2D::translation(10.0, 20.0),
            Affine2D::scale(2.0),
        ));
        builder.draw(PaintCommand::Rect(RectPaint {
            rect: rect(),
            style: rect_style(),
        }));
        builder.pop_transform();
        let list = builder.finish().unwrap();

        let output = lower(&list);

        assert!(output.report.unsupported.is_empty());
        match &output.commands[0] {
            BackendCommand::Rect(req) => {
                assert_eq!(
                    req.rect,
                    Rect {
                        x: 12.0,
                        y: 24.0,
                        w: 6.0,
                        h: 8.0,
                    }
                );
            }
            _ => panic!("expected rect command"),
        }
    }

    #[test]
    fn reports_rotated_rect_unsupported() {
        let mut builder = DisplayListBuilder::new();
        builder.push_transform(Affine2D::rotation_radians(0.5));
        builder.draw(PaintCommand::Rect(RectPaint {
            rect: rect(),
            style: rect_style(),
        }));
        builder.pop_transform();
        let list = builder.finish().unwrap();

        let output = lower(&list);

        assert_eq!(
            output.report.unsupported,
            vec![UnsupportedDisplayCommand {
                index: 0,
                reason: UnsupportedDisplayReason::NonUniformTransform,
            }]
        );
        assert!(output.commands.is_empty());
    }

    #[test]
    fn lowers_path_and_scales_stroke() {
        let mut builder = DisplayListBuilder::new();
        builder.push_transform(Affine2D::scale(3.0));
        builder.draw(PaintCommand::Path(PathPaint {
            data: PathData::line(Point { x: 1.0, y: 1.0 }, Point { x: 2.0, y: 1.0 }),
            style: PathStyle::stroke(Stroke::new(2.0, Color::WHITE)),
        }));
        builder.pop_transform();
        let list = builder.finish().unwrap();

        let output = lower(&list);

        match &output.commands[0] {
            BackendCommand::Path(req) => {
                assert_eq!(req.style.stroke.unwrap().width, 6.0);
                assert_eq!(
                    req.data.commands[0],
                    crate::paint::PathCommand::MoveTo(Point { x: 3.0, y: 3.0 })
                );
            }
            _ => panic!("expected path command"),
        }
    }

    #[test]
    fn lowers_clip_stack_to_push_and_pop_commands() {
        let mut builder = DisplayListBuilder::new();
        builder.push_clip(ClipShape::Rect(rect()));
        builder.draw(PaintCommand::Rect(RectPaint {
            rect: rect(),
            style: rect_style(),
        }));
        builder.pop_clip();
        let list = builder.finish().unwrap();

        let output = lower(&list);

        assert!(matches!(
            output.commands[0],
            BackendCommand::PushClip { .. }
        ));
        assert!(matches!(output.commands[1], BackendCommand::Rect(_)));
        assert!(matches!(output.commands[2], BackendCommand::PopClip));
    }

    #[test]
    fn reports_missing_texture() {
        let mut builder = DisplayListBuilder::new();
        builder.draw(PaintCommand::Image(ImagePaint {
            rect: rect(),
            texture: TextureHandle(99),
            style: Default::default(),
        }));
        let list = builder.finish().unwrap();

        let output = lower(&list);

        assert_eq!(
            output.report.unsupported,
            vec![UnsupportedDisplayCommand {
                index: 0,
                reason: UnsupportedDisplayReason::MissingTexture(TextureHandle(99)),
            }]
        );
    }

    #[test]
    fn reports_missing_svg_source() {
        let mut builder = DisplayListBuilder::new();
        builder.draw(PaintCommand::Svg(SvgPaint {
            rect: rect(),
            source: SvgSourceKey::new("missing"),
            style: SvgStyle::default(),
        }));
        let list = builder.finish().unwrap();

        let output = lower(&list);

        assert_eq!(
            output.report.unsupported,
            vec![UnsupportedDisplayCommand {
                index: 0,
                reason: UnsupportedDisplayReason::MissingSvgSource("missing".to_string()),
            }]
        );
    }

    #[test]
    fn reports_layer_unsupported() {
        let mut builder = DisplayListBuilder::new();
        builder.draw(PaintCommand::Layer(crate::paint::LayerPaint::new(
            rect(),
            1.0,
            DisplayList::default(),
        )));
        let list = builder.finish().unwrap();

        let output = lower(&list);

        assert_eq!(
            output.report.unsupported,
            vec![UnsupportedDisplayCommand {
                index: 0,
                reason: UnsupportedDisplayReason::UnsupportedCommand("layer"),
            }]
        );
    }

    #[test]
    fn lowers_filled_path_to_backend_command() {
        let mut builder = DisplayListBuilder::new();
        builder.draw(PaintCommand::Path(PathPaint {
            data: PathData::new()
                .move_to(Point { x: 0.0, y: 0.0 })
                .line_to(Point { x: 10.0, y: 0.0 })
                .line_to(Point { x: 10.0, y: 10.0 })
                .close(),
            style: PathStyle::fill(Fill::non_zero(Color::WHITE)),
        }));
        let list = builder.finish().unwrap();

        let output = lower(&list);

        assert!(matches!(output.commands[0], BackendCommand::Path(_)));
    }
}
