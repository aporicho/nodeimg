use crate::paint::TextureHandle;

use super::super::command::BackendCommand;

#[derive(Default)]
pub(in crate::renderer) struct DisplayBackendOutput {
    pub(in crate::renderer) commands: Vec<BackendCommand>,
    pub(in crate::renderer) report: DisplayRenderReport,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct DisplayRenderReport {
    pub(crate) unsupported: Vec<UnsupportedDisplayCommand>,
    pub(crate) stats: DisplayLoweringStats,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct DisplayLoweringStats {
    pub(crate) input_commands: usize,
    pub(crate) input_clips: usize,
    pub(crate) backend_commands: usize,
    pub(crate) unsupported: usize,
    pub(crate) max_clip_depth: usize,
    pub(crate) command_kinds: DisplayCommandKindCounts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct DisplayCommandKindCounts {
    pub(crate) rect: usize,
    pub(crate) path: usize,
    pub(crate) circle: usize,
    pub(crate) grid: usize,
    pub(crate) image: usize,
    pub(crate) text: usize,
    pub(crate) shadow: usize,
    pub(crate) svg: usize,
    pub(crate) svg_raster: usize,
    pub(crate) layer: usize,
}
