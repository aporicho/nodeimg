use super::context::PaintCx;
use super::node::paint_to_target;
use super::traversal::PaintTraversal;
use crate::geometry::{Affine2D, Point, Rect};
use crate::paint::{BoundaryPaintRecorder, PaintBuildError, PaintFragment, TextStyle};
use crate::tree::paint_target::PaintTarget;
use crate::tree::{RepaintBoundaryId, Revision, Tree};

pub(crate) fn build_paint_fragment(
    tree: &Tree,
    boundary: RepaintBoundaryId,
    cx: PaintCx<'_>,
    measure_text: impl FnMut(&str, &TextStyle) -> (f32, f32),
) -> Result<PaintFragment, PaintBuildError> {
    let Some(node) = tree.get(boundary.0) else {
        return Ok(empty_fragment(boundary));
    };
    let origin = Point {
        x: node.rect.x,
        y: node.rect.y,
    };
    let boundary_to_screen = Affine2D::translation(node.rect.x, node.rect.y);
    let local_bounds = Rect {
        x: 0.0,
        y: 0.0,
        w: node.rect.w,
        h: node.rect.h,
    };
    let revision = node.paint_meta.fragment_revision;
    let mut recorder = BoundaryPaintRecorder::with_measure(measure_text);
    recorder
        .target()
        .push_transform(Affine2D::translation(-origin.x, -origin.y));
    let mut child_boundaries = Vec::new();
    let mut traversal = PaintTraversal::Boundary {
        root: boundary.0,
        boundary_to_screen,
        child_boundaries: &mut child_boundaries,
    };
    paint_to_target(
        tree,
        boundary.0,
        recorder.target(),
        cx.interaction,
        cx.text_boxes,
        cx.animations,
        cx.theme,
        &mut traversal,
    );
    recorder.target().pop_transform();
    recorder.finish(boundary, revision, local_bounds, child_boundaries)
}

fn empty_fragment(boundary: RepaintBoundaryId) -> PaintFragment {
    PaintFragment {
        boundary,
        revision: Revision::ZERO,
        local_bounds: Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        clips: Vec::new(),
        commands: Vec::new(),
        child_boundaries: Vec::new(),
    }
}
