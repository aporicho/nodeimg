use crate::diagnostics::render_trace::{self, RenderTraceStage};
use crate::paint::{ClipId, DisplayList, PaintCommand, ResolvedPaintCommand};

use super::super::command::BackendCommand;
use super::super::display_resources::DisplayResourceResolver;
use super::super::svg::SvgVectorCache;
use super::api::{
    DisplayBackendOutput, DisplayCommandKindCounts, DisplayLoweringStats, DisplayRenderReport,
};

pub(super) struct LoweringContext<'a, R> {
    pub(super) resources: &'a R,
    pub(super) svg_vector_cache: &'a mut SvgVectorCache,
    pub(super) active_clips: Vec<ClipId>,
    pub(super) commands: Vec<BackendCommand>,
    pub(super) report: DisplayRenderReport,
    pub(super) command_kinds: DisplayCommandKindCounts,
    pub(super) max_clip_depth: usize,
}

pub(in crate::renderer) fn lower_display_list<R: DisplayResourceResolver>(
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
        command_kinds: DisplayCommandKindCounts::default(),
        max_clip_depth: 0,
    };

    for (index, command) in list.commands.iter().enumerate() {
        if !cx.sync_clips(index, &command.clips, &list.clips) {
            continue;
        }
        cx.lower_command(index, command);
    }

    cx.pop_all_clips();
    cx.report.stats = DisplayLoweringStats {
        input_commands: list.commands.len(),
        input_clips: list.clips.len(),
        backend_commands: cx.commands.len(),
        unsupported: cx.report.unsupported.len(),
        max_clip_depth: cx.max_clip_depth,
        command_kinds: cx.command_kinds,
    };

    render_trace::debug_stage(RenderTraceStage::DisplayListLowering, cx.report.stats);

    DisplayBackendOutput {
        commands: cx.commands,
        report: cx.report,
    }
}

impl<R: DisplayResourceResolver> LoweringContext<'_, R> {
    pub(super) fn lower_command(&mut self, index: usize, resolved: &ResolvedPaintCommand) {
        match &resolved.command {
            PaintCommand::Rect(paint) => {
                self.command_kinds.rect += 1;
                self.lower_rect(index, resolved.transform, paint);
            }
            PaintCommand::Path(paint) => {
                self.command_kinds.path += 1;
                self.lower_path(index, resolved.transform, paint);
            }
            PaintCommand::Circle(paint) => {
                self.command_kinds.circle += 1;
                self.lower_circle(index, resolved.transform, *paint);
            }
            PaintCommand::Grid(paint) => {
                self.command_kinds.grid += 1;
                self.lower_grid(index, resolved.transform, *paint);
            }
            PaintCommand::Image(paint) => {
                self.command_kinds.image += 1;
                self.lower_image(index, resolved.transform, *paint);
            }
            PaintCommand::Text(paint) => {
                self.command_kinds.text += 1;
                self.lower_text(index, resolved.transform, paint);
            }
            PaintCommand::Shadow(paint) => {
                self.command_kinds.shadow += 1;
                self.lower_shadow(index, resolved.transform, *paint);
            }
            PaintCommand::Svg(paint) => {
                self.command_kinds.svg += 1;
                self.lower_svg(index, resolved.transform, paint);
            }
            PaintCommand::SvgRaster(paint) => {
                self.command_kinds.svg_raster += 1;
                self.lower_svg_raster(index, resolved.transform, paint);
            }
            PaintCommand::Layer(paint) => {
                self.command_kinds.layer += 1;
                self.lower_layer(index, resolved.transform, paint);
            }
        }
    }
}
