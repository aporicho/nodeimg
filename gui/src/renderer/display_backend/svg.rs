use crate::diagnostics::render_trace::{self, TARGET_RENDER};
use crate::geometry::Affine2D;
use crate::icon::{IconFit, IconOpacity, IconPaintOverride, IconStrokeWidth, IconStyle};
use crate::paint::{
    Color, SvgFit, SvgPaint, SvgPaintOverride, SvgRasterPaint, SvgSourceKey, SvgStrokeWidth,
    SvgStyle,
};

use super::super::command::{AffinePathRequest, AffineSvgRasterRequest, BackendCommand};
use super::super::display_resources::DisplayResourceResolver;
use super::super::svg::{resolve_svg_icon_paths, SvgSource};
use super::api::UnsupportedDisplayReason;
use super::lowering::LoweringContext;

impl<R: DisplayResourceResolver> LoweringContext<'_, R> {
    pub(super) fn lower_svg(&mut self, index: usize, transform: Affine2D, paint: &SvgPaint) {
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
                        target: TARGET_RENDER,
                        frame_id = render_trace::current_render_trace_frame().id,
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

    pub(super) fn lower_svg_raster(
        &mut self,
        index: usize,
        transform: Affine2D,
        paint: &SvgRasterPaint,
    ) {
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

    fn resolve_svg_source(&mut self, index: usize, key: &SvgSourceKey) -> Option<SvgSource> {
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
