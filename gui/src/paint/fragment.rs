use crate::geometry::{Affine2D, Rect};
use crate::tree::{RepaintBoundaryId, Revision};

use super::{ClipId, DisplayList, ResolvedClip, ResolvedPaintCommand};

#[derive(Clone, Debug, PartialEq)]
pub struct PaintFragment {
    pub boundary: RepaintBoundaryId,
    pub revision: Revision,
    pub local_bounds: Rect,
    pub clips: Vec<ResolvedClip>,
    pub commands: Vec<ResolvedPaintCommand>,
    pub child_boundaries: Vec<FragmentChildRef>,
}

impl PaintFragment {
    pub fn display_list(&self) -> DisplayList {
        DisplayList {
            commands: self.commands.clone(),
            clips: self.clips.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FragmentChildRef {
    pub boundary: RepaintBoundaryId,
    pub local_transform: Affine2D,
    pub clip_stack: Vec<ClipId>,
    pub z_index: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PaintFlushStats {
    pub fragments_rebuilt: usize,
    pub fragments_reused: usize,
    pub fragments_evicted: usize,
    pub boundaries_dirty: usize,
    pub commands_recorded: usize,
    pub fragments_flattened: usize,
}
