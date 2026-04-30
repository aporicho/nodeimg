#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NodeMutationMeta {
    pub rect_move: RectMoveInvalidation,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RectMoveInvalidation {
    #[default]
    Layout,
    Repaint,
    BoundaryPlacement,
    LayoutAndBoundaryPlacement,
}
