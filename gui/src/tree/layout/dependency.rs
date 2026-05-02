#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutDependencyKind {
    Style,
    Text,
    Children,
    ExplicitRect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutDependencyScope {
    LocalNode,
    RelayoutBoundary,
}
