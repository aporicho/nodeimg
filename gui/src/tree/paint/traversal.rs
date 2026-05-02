use crate::geometry::Affine2D;
use crate::paint::FragmentChildRef;
use crate::tree::NodeId;

pub(crate) enum PaintTraversal<'a> {
    #[cfg(test)]
    Full,
    Boundary {
        root: NodeId,
        boundary_to_screen: Affine2D,
        child_boundaries: &'a mut Vec<FragmentChildRef>,
    },
}
