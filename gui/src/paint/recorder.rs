use crate::geometry::Rect;
use crate::tree::{RepaintBoundaryId, Revision};

use super::{DisplayList, PaintBuildError, PaintFragment, RecordingPaintTarget, TextStyle};

pub struct BoundaryPaintRecorder<'a> {
    target: RecordingPaintTarget<'a>,
}

impl BoundaryPaintRecorder<'static> {
    pub fn new() -> Self {
        Self {
            target: RecordingPaintTarget::new(),
        }
    }
}

impl<'a> BoundaryPaintRecorder<'a> {
    pub fn with_measure(measure: impl FnMut(&str, &TextStyle) -> (f32, f32) + 'a) -> Self {
        Self {
            target: RecordingPaintTarget::with_measure(measure),
        }
    }

    pub fn target(&mut self) -> &mut RecordingPaintTarget<'a> {
        &mut self.target
    }

    pub fn finish(
        self,
        boundary: RepaintBoundaryId,
        revision: Revision,
        local_bounds: Rect,
        child_boundaries: Vec<super::FragmentChildRef>,
    ) -> Result<PaintFragment, PaintBuildError> {
        let DisplayList { commands, clips } = self.target.display_list()?;
        Ok(PaintFragment {
            boundary,
            revision,
            local_bounds,
            clips,
            commands,
            child_boundaries,
        })
    }
}
