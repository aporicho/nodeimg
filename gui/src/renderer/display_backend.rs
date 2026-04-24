use crate::geometry::Affine2D;
use crate::icon::{IconFit, IconOpacity, IconPaintOverride, IconStrokeWidth, IconStyle};
use crate::paint::{
    CirclePaint, ClipId, Color, DisplayList, ImagePaint, PaintCommand, PathPaint, RectPaint,
    RectStyle, ResolvedClip, ResolvedPaintCommand, Shadow, ShadowPaint, SvgFit, SvgPaint,
    SvgPaintOverride, SvgRasterPaint, SvgSourceKey, SvgStrokeWidth, SvgStyle, TextPaint, TextStyle,
    TextureHandle,
};

use super::affine::{similarity_scale, translate_uniform_scale};
use super::command::{
    AffineCircleRequest, AffineClipRequest, AffineImageRequest, AffinePathRequest,
    AffineRectRequest, BackendCommand,
};
use super::display_resources::DisplayResourceResolver;
use super::pipeline::shadow::ShadowRequest;
use super::pipeline::text::TextRequest;
use super::svg::{resolve_svg_icon_paths, SvgRasterDraw, SvgVectorCache};

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
    MissingTexture(TextureHandle),
    MissingSvgSource(String),
    UnsupportedCommand(&'static str),
    UnsupportedClip(&'static str),
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
        if let Some(shadow) = paint.style.shadow {
            let Some(legacy) = translate_uniform_scale(transform) else {
                self.report.record(
                    index,
                    UnsupportedDisplayReason::UnsupportedCommand("shadow affine"),
                );
                self.commands.push(BackendCommand::Rect(AffineRectRequest {
                    rect: paint.rect,
                    style: rect_style_without_shadow(&paint.style),
                    transform,
                }));
                return;
            };
            let shadow_rect = paint.rect;
            self.commands.push(BackendCommand::Shadow(ShadowRequest {
                rect: legacy.rect(shadow_rect),
                radius: paint.style.radius.map(|radius| radius * legacy.scale),
                shadow: scale_shadow(shadow, legacy.scale),
            }));
        }
        self.commands.push(BackendCommand::Rect(AffineRectRequest {
            rect: paint.rect,
            style: rect_style_without_shadow(&paint.style),
            transform,
        }));
    }

    fn lower_path(&mut self, _index: usize, transform: Affine2D, paint: &PathPaint) {
        self.commands.push(BackendCommand::Path(AffinePathRequest {
            data: paint.data.clone(),
            style: paint.style,
            transform,
        }));
    }

    fn lower_circle(&mut self, _index: usize, transform: Affine2D, paint: CirclePaint) {
        self.commands
            .push(BackendCommand::Circle(AffineCircleRequest {
                paint,
                transform,
            }));
    }

    fn lower_image(&mut self, index: usize, transform: Affine2D, paint: ImagePaint) {
        let Some(resource) = self.resources.texture(paint.texture) else {
            self.report.record(
                index,
                UnsupportedDisplayReason::MissingTexture(paint.texture),
            );
            return;
        };
        self.commands
            .push(BackendCommand::Image(AffineImageRequest {
                rect: paint.rect,
                transform,
                view: resource.view,
                size: resource.size,
                style: paint.style,
            }));
    }

    fn lower_text(&mut self, index: usize, transform: Affine2D, paint: &TextPaint) {
        let Some(legacy) = translate_uniform_scale(transform) else {
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
        let Some(legacy) = translate_uniform_scale(transform) else {
            self.report
                .record(index, UnsupportedDisplayReason::NonUniformTransform);
            return;
        };
        let shadow_rect = paint.rect;
        self.commands.push(BackendCommand::Shadow(ShadowRequest {
            rect: legacy.rect(shadow_rect),
            radius: paint.radius.map(|radius| radius * legacy.scale),
            shadow: scale_shadow(paint.shadow, legacy.scale),
        }));
    }

    fn lower_svg(&mut self, index: usize, transform: Affine2D, paint: &SvgPaint) {
        let Some(source) = self.resolve_svg_source(index, &paint.source) else {
            return;
        };
        let icon_style = match icon_style_for_svg_affine(paint.style, transform) {
            Ok(style) => style,
            Err(reason) => {
                self.report.record(index, reason);
                return;
            }
        };
        match self.svg_vector_cache.get_or_parse(&source) {
            Ok(document) => {
                for request in resolve_svg_icon_paths(&document, paint.rect, icon_style) {
                    self.commands.push(BackendCommand::Path(AffinePathRequest {
                        data: request.data,
                        style: request.style,
                        transform,
                    }));
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
                let Some(legacy) = translate_uniform_scale(transform) else {
                    self.report.record(
                        index,
                        UnsupportedDisplayReason::UnsupportedCommand("svg raster affine"),
                    );
                    return;
                };
                let raster_rect = paint.rect;
                self.commands.push(BackendCommand::SvgRaster(SvgRasterDraw {
                    rect: legacy.rect(raster_rect),
                    source,
                    style: icon_style_from_svg(paint.style),
                }));
            }
        }
    }

    fn lower_svg_raster(&mut self, index: usize, transform: Affine2D, paint: &SvgRasterPaint) {
        let Some(legacy) = translate_uniform_scale(transform) else {
            self.report
                .record(index, UnsupportedDisplayReason::NonUniformTransform);
            return;
        };
        let Some(source) = self.resolve_svg_source(index, &paint.source) else {
            return;
        };
        let raster_rect = paint.rect;
        self.commands.push(BackendCommand::SvgRaster(SvgRasterDraw {
            rect: legacy.rect(raster_rect),
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

    fn lower_clip(&mut self, _index: usize, clip: &ResolvedClip) -> Option<BackendCommand> {
        Some(BackendCommand::PushClip(AffineClipRequest {
            shape: clip.shape.clone(),
            transform: clip.transform,
        }))
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

fn scale_text_style(mut style: TextStyle, scale: f32) -> TextStyle {
    style.size *= scale;
    style
}

fn rect_style_without_shadow(style: &RectStyle) -> RectStyle {
    RectStyle {
        color: style.color,
        border: style.border,
        radius: style.radius,
        shadow: None,
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

fn icon_style_for_svg_affine(
    style: SvgStyle,
    transform: Affine2D,
) -> Result<IconStyle, UnsupportedDisplayReason> {
    let mut icon_style = icon_style_from_svg(style);
    if let SvgStrokeWidth::ScreenPx(width) = style.stroke_width {
        let Some(scale) = similarity_scale(transform) else {
            return Err(UnsupportedDisplayReason::UnsupportedCommand(
                "svg screen-px stroke affine",
            ));
        };
        icon_style.stroke_width = IconStrokeWidth::ScreenPx(width / scale);
    }
    Ok(icon_style)
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
    use crate::renderer::{PathStyle, Rect, Stroke};

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
        let transform = Affine2D::compose(Affine2D::translation(10.0, 20.0), Affine2D::scale(2.0));
        builder.push_transform(transform);
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
                assert_eq!(req.rect, rect());
                assert_eq!(req.transform, transform);
            }
            _ => panic!("expected rect command"),
        }
    }

    #[test]
    fn lowers_rotated_rect_as_affine_command() {
        let mut builder = DisplayListBuilder::new();
        let transform = Affine2D::rotation_radians(0.5);
        builder.push_transform(transform);
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
                assert_eq!(req.rect, rect());
                assert_eq!(req.transform, transform);
            }
            _ => panic!("expected rect command"),
        }
    }

    #[test]
    fn lowers_path_with_local_geometry_and_affine_transform() {
        let mut builder = DisplayListBuilder::new();
        let transform = Affine2D::scale(3.0);
        builder.push_transform(transform);
        builder.draw(PaintCommand::Path(PathPaint {
            data: PathData::line(Point { x: 1.0, y: 1.0 }, Point { x: 2.0, y: 1.0 }),
            style: PathStyle::stroke(Stroke::new(2.0, Color::WHITE)),
        }));
        builder.pop_transform();
        let list = builder.finish().unwrap();

        let output = lower(&list);

        match &output.commands[0] {
            BackendCommand::Path(req) => {
                assert_eq!(req.style.stroke.unwrap().width, 2.0);
                assert_eq!(req.transform, transform);
                assert_eq!(
                    req.data.commands[0],
                    crate::paint::PathCommand::MoveTo(Point { x: 1.0, y: 1.0 })
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

        match &output.commands[0] {
            BackendCommand::PushClip(req) => {
                assert_eq!(req.shape, ClipShape::Rect(rect()));
                assert_eq!(req.transform, Affine2D::IDENTITY);
            }
            _ => panic!("expected push clip command"),
        }
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
