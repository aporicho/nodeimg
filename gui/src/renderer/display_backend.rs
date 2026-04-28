use crate::geometry::Affine2D;
use crate::icon::{IconFit, IconOpacity, IconPaintOverride, IconStrokeWidth, IconStyle};
use crate::paint::{
    CirclePaint, ClipId, Color, DisplayList, GridPaint, ImageOpacity, ImagePaint, LayerPaint,
    PaintCommand, PathPaint, RectPaint, RectStyle, ResolvedClip, ResolvedPaintCommand, ShadowPaint,
    SvgFit, SvgPaint, SvgPaintOverride, SvgRasterPaint, SvgSourceKey, SvgStrokeWidth, SvgStyle,
    TextPaint, TextureHandle,
};

use super::command::{
    AffineCircleRequest, AffineClipRequest, AffineGridRequest, AffineImageRequest,
    AffinePathRequest, AffineRectRequest, AffineShadowRequest, AffineSvgRasterRequest,
    AffineTextRequest, BackendCommand,
};
use super::display_resources::DisplayResourceResolver;
use super::svg::{resolve_svg_icon_paths, SvgVectorCache};

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
    MissingTexture(TextureHandle),
    MissingSvgSource(String),
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
            PaintCommand::Grid(paint) => self.lower_grid(index, resolved.transform, *paint),
            PaintCommand::Image(paint) => self.lower_image(index, resolved.transform, *paint),
            PaintCommand::Text(paint) => self.lower_text(index, resolved.transform, paint),
            PaintCommand::Shadow(paint) => self.lower_shadow(index, resolved.transform, *paint),
            PaintCommand::Svg(paint) => self.lower_svg(index, resolved.transform, paint),
            PaintCommand::SvgRaster(paint) => {
                self.lower_svg_raster(index, resolved.transform, paint);
            }
            PaintCommand::Layer(paint) => self.lower_layer(index, resolved.transform, paint),
        }
    }

    fn lower_rect(&mut self, _index: usize, transform: Affine2D, paint: &RectPaint) {
        if let Some(shadow) = paint.style.shadow {
            self.commands
                .push(BackendCommand::Shadow(AffineShadowRequest {
                    rect: paint.rect,
                    radius: paint.style.radius,
                    shadow,
                    transform,
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

    fn lower_grid(&mut self, _index: usize, transform: Affine2D, paint: GridPaint) {
        if paint.spacing <= 0.0 || paint.dot_size <= 0.0 {
            return;
        }
        self.commands
            .push(BackendCommand::Grid(AffineGridRequest { paint, transform }));
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

    fn lower_text(&mut self, _index: usize, transform: Affine2D, paint: &TextPaint) {
        self.commands.push(BackendCommand::Text(AffineTextRequest {
            pos: paint.pos,
            text: paint.text.clone(),
            style: paint.style,
            bounds: paint.bounds,
            transform,
        }));
    }

    fn lower_shadow(&mut self, _index: usize, transform: Affine2D, paint: ShadowPaint) {
        self.commands
            .push(BackendCommand::Shadow(AffineShadowRequest {
                rect: paint.rect,
                radius: paint.radius,
                shadow: paint.shadow,
                transform,
            }));
    }

    fn lower_svg(&mut self, index: usize, transform: Affine2D, paint: &SvgPaint) {
        let Some(source) = self.resolve_svg_source(index, &paint.source) else {
            return;
        };
        let icon_style = icon_style_from_svg(paint.style);
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
                self.commands
                    .push(BackendCommand::SvgRaster(AffineSvgRasterRequest {
                        rect: paint.rect,
                        source,
                        style: icon_style_from_svg(paint.style),
                        transform,
                    }));
            }
        }
    }

    fn lower_svg_raster(&mut self, index: usize, transform: Affine2D, paint: &SvgRasterPaint) {
        let Some(source) = self.resolve_svg_source(index, &paint.source) else {
            return;
        };
        self.commands
            .push(BackendCommand::SvgRaster(AffineSvgRasterRequest {
                rect: paint.rect,
                source,
                style: IconStyle::monochrome(paint.color.unwrap_or(Color::WHITE)),
                transform,
            }));
    }

    fn lower_layer(&mut self, _index: usize, transform: Affine2D, paint: &LayerPaint) {
        if paint.opacity <= 0.0 {
            return;
        }

        let mut nested = LoweringContext {
            resources: self.resources,
            svg_vector_cache: self.svg_vector_cache,
            active_clips: Vec::new(),
            commands: Vec::new(),
            report: DisplayRenderReport::default(),
        };

        for (index, command) in paint.content.commands.iter().enumerate() {
            if !nested.sync_clips(index, &command.clips, &paint.content.clips) {
                continue;
            }
            nested.lower_command(index, command);
        }
        nested.pop_all_clips();

        apply_layer_to_commands(&mut nested.commands, transform, paint.opacity);
        self.report.unsupported.extend(nested.report.unsupported);
        self.commands.extend(nested.commands);
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

fn rect_style_without_shadow(style: &RectStyle) -> RectStyle {
    RectStyle {
        color: style.color,
        border: style.border,
        radius: style.radius,
        shadow: None,
    }
}

fn apply_layer_to_commands(commands: &mut [BackendCommand], transform: Affine2D, opacity: f32) {
    for command in commands {
        compose_command_transform(command, transform);
        apply_command_opacity(command, opacity);
    }
}

fn compose_command_transform(command: &mut BackendCommand, transform: Affine2D) {
    match command {
        BackendCommand::Shadow(req) => {
            req.transform = Affine2D::compose(transform, req.transform);
        }
        BackendCommand::Rect(req) => {
            req.transform = Affine2D::compose(transform, req.transform);
        }
        BackendCommand::Circle(req) => {
            req.transform = Affine2D::compose(transform, req.transform);
        }
        BackendCommand::Grid(req) => {
            req.transform = Affine2D::compose(transform, req.transform);
        }
        BackendCommand::Text(req) => {
            req.transform = Affine2D::compose(transform, req.transform);
        }
        BackendCommand::Image(req) => {
            req.transform = Affine2D::compose(transform, req.transform);
        }
        BackendCommand::SvgRaster(req) => {
            req.transform = Affine2D::compose(transform, req.transform);
        }
        BackendCommand::Path(req) => {
            req.transform = Affine2D::compose(transform, req.transform);
        }
        BackendCommand::PushClip(req) => {
            req.transform = Affine2D::compose(transform, req.transform);
        }
        BackendCommand::PopClip => {}
    }
}

fn apply_command_opacity(command: &mut BackendCommand, opacity: f32) {
    match command {
        BackendCommand::Shadow(req) => {
            req.shadow.color = color_with_opacity(req.shadow.color, opacity);
        }
        BackendCommand::Rect(req) => {
            apply_rect_style_opacity(&mut req.style, opacity);
        }
        BackendCommand::Circle(req) => {
            req.paint.fill = req
                .paint
                .fill
                .map(|color| color_with_opacity(color, opacity));
            req.paint.stroke = req.paint.stroke.map(|mut stroke| {
                stroke.color = color_with_opacity(stroke.color, opacity);
                stroke
            });
        }
        BackendCommand::Grid(req) => {
            req.paint.dot_color = color_with_opacity(req.paint.dot_color, opacity);
        }
        BackendCommand::Text(req) => {
            req.style.color = color_with_opacity(req.style.color, opacity);
        }
        BackendCommand::Image(req) => {
            req.style = req
                .style
                .with_opacity(ImageOpacity::new(req.style.opacity.get() * opacity));
        }
        BackendCommand::SvgRaster(req) => {
            req.style = icon_style_with_opacity(req.style, opacity);
        }
        BackendCommand::Path(req) => {
            if let Some(mut fill) = req.style.fill {
                fill.color = color_with_opacity(fill.color, opacity);
                req.style.fill = Some(fill);
            }
            if let Some(mut stroke) = req.style.stroke {
                stroke.color = color_with_opacity(stroke.color, opacity);
                req.style.stroke = Some(stroke);
            }
        }
        BackendCommand::PushClip(_) | BackendCommand::PopClip => {}
    }
}

fn apply_rect_style_opacity(style: &mut RectStyle, opacity: f32) {
    style.color = color_with_opacity(style.color, opacity);
    if let Some(mut border) = style.border {
        border.color = color_with_opacity(border.color, opacity);
        style.border = Some(border);
    }
    if let Some(mut shadow) = style.shadow {
        shadow.color = color_with_opacity(shadow.color, opacity);
        style.shadow = Some(shadow);
    }
}

fn icon_style_with_opacity(mut style: IconStyle, opacity: f32) -> IconStyle {
    style.opacity = IconOpacity::new(style.opacity.get() * opacity);
    style
}

fn color_with_opacity(mut color: Color, opacity: f32) -> Color {
    color.a *= opacity.clamp(0.0, 1.0);
    color
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
        ClipShape, DisplayListBuilder, Fill, GridPaint, ImagePaint, PathData, RectPaint,
        ShadowPaint, SvgPaint, SvgRasterPaint, SvgSourceKey, TextPaint, TextureHandle,
    };
    use crate::renderer::display_resources::{DisplayResourceResolver, EmptyDisplayResources};
    use crate::renderer::svg::SvgSource;
    use crate::renderer::{PathStyle, Rect, Shadow, Stroke, TextStyle};
    use std::sync::Arc;

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

    struct SvgOnlyResources {
        source: SvgSource,
    }

    impl DisplayResourceResolver for SvgOnlyResources {
        fn texture(&self, _handle: TextureHandle) -> Option<super::super::TextureResource> {
            None
        }

        fn svg_source(&self, _key: &SvgSourceKey) -> Option<SvgSource> {
            Some(self.source.clone())
        }
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
    fn lowers_grid_to_single_backend_command() {
        let mut builder = DisplayListBuilder::new();
        let transform = Affine2D::translation(10.0, 20.0);
        let paint = GridPaint {
            rect: rect(),
            spacing: 8.0,
            dot_color: Color::WHITE,
            dot_size: 1.0,
        };
        builder.push_transform(transform);
        builder.draw(PaintCommand::Grid(paint));
        builder.pop_transform();
        let list = builder.finish().unwrap();

        let output = lower(&list);

        assert_eq!(output.commands.len(), 1);
        match &output.commands[0] {
            BackendCommand::Grid(req) => {
                assert_eq!(req.paint, paint);
                assert_eq!(req.transform, transform);
            }
            _ => panic!("expected grid command"),
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
    fn lowers_layer_content_with_composed_transform_and_opacity() {
        let mut inner = DisplayListBuilder::new();
        inner.push_transform(Affine2D::translation(3.0, 4.0));
        inner.draw(PaintCommand::Rect(RectPaint {
            rect: rect(),
            style: rect_style(),
        }));
        inner.pop_transform();

        let mut builder = DisplayListBuilder::new();
        let transform = Affine2D::rotation_radians(0.25);
        builder.push_transform(transform);
        builder.draw(PaintCommand::Layer(LayerPaint::new(
            rect(),
            0.5,
            inner.finish().unwrap(),
        )));
        builder.pop_transform();
        let list = builder.finish().unwrap();

        let output = lower(&list);

        assert!(output.report.unsupported.is_empty());
        match &output.commands[0] {
            BackendCommand::Rect(req) => {
                assert_eq!(
                    req.transform,
                    Affine2D::compose(transform, Affine2D::translation(3.0, 4.0))
                );
                assert_eq!(req.style.color.a, 0.5);
            }
            _ => panic!("expected rect command"),
        }
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

    #[test]
    fn lowers_rotated_text_to_affine_backend_command() {
        let mut builder = DisplayListBuilder::new();
        let transform = Affine2D::rotation_radians(0.25);
        builder.push_transform(transform);
        builder.draw(PaintCommand::Text(TextPaint {
            pos: Point { x: 1.0, y: 2.0 },
            text: "hello".to_string(),
            style: TextStyle::new(Color::WHITE, 12.0),
            bounds: Some(rect()),
        }));
        builder.pop_transform();
        let list = builder.finish().unwrap();

        let output = lower(&list);

        assert!(output.report.unsupported.is_empty());
        match &output.commands[0] {
            BackendCommand::Text(req) => {
                assert_eq!(req.pos, Point { x: 1.0, y: 2.0 });
                assert_eq!(req.transform, transform);
                assert_eq!(req.style.size, 12.0);
            }
            _ => panic!("expected text command"),
        }
    }

    #[test]
    fn lowers_rotated_shadow_to_affine_backend_command() {
        let mut builder = DisplayListBuilder::new();
        let transform = Affine2D::rotation_radians(0.25);
        builder.push_transform(transform);
        builder.draw(PaintCommand::Shadow(ShadowPaint {
            rect: rect(),
            radius: [2.0; 4],
            shadow: Shadow {
                color: Color::BLACK,
                offset: [1.0, 2.0],
                blur: 3.0,
                spread: 4.0,
            },
        }));
        builder.pop_transform();
        let list = builder.finish().unwrap();

        let output = lower(&list);

        assert!(output.report.unsupported.is_empty());
        match &output.commands[0] {
            BackendCommand::Shadow(req) => {
                assert_eq!(req.rect, rect());
                assert_eq!(req.radius, [2.0; 4]);
                assert_eq!(req.transform, transform);
            }
            _ => panic!("expected shadow command"),
        }
    }

    #[test]
    fn lowers_rotated_svg_raster_to_affine_backend_command() {
        let mut builder = DisplayListBuilder::new();
        let transform = Affine2D::rotation_radians(0.25);
        builder.push_transform(transform);
        builder.draw(PaintCommand::SvgRaster(SvgRasterPaint {
            rect: rect(),
            source: SvgSourceKey::new("test-icon"),
            color: Some(Color::WHITE),
        }));
        builder.pop_transform();
        let list = builder.finish().unwrap();
        let mut svg_cache = SvgVectorCache::new();
        let resources = SvgOnlyResources {
            source: SvgSource::new("test-icon", Arc::<[u8]>::from(&b"<svg/>"[..])),
        };

        let output = lower_display_list(&list, &resources, &mut svg_cache);

        assert!(output.report.unsupported.is_empty());
        match &output.commands[0] {
            BackendCommand::SvgRaster(req) => {
                assert_eq!(req.rect, rect());
                assert_eq!(req.transform, transform);
            }
            _ => panic!("expected svg raster command"),
        }
    }
}
