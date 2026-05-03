use crate::renderer::{Color, FillRule, LineCap, LineJoin, PathData};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SvgSize {
    pub(crate) width: f32,
    pub(crate) height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SvgVectorDocument {
    pub(crate) source_size: SvgSize,
    pub(crate) paths: Vec<SvgVectorPath>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SvgVectorPath {
    pub(crate) data: PathData,
    pub(crate) fill: Option<Color>,
    pub(crate) fill_rule: FillRule,
    pub(crate) stroke: Option<SvgVectorStroke>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SvgVectorStroke {
    pub(crate) color: Color,
    pub(crate) width: f32,
    pub(crate) cap: LineCap,
    pub(crate) join: LineJoin,
    pub(crate) miter_limit: f32,
}
